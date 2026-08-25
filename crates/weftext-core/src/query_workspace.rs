use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;

use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use crate::query::{
    QUERY_EXPRESSION_CAPABILITY_ID, QUERY_MAX_CONTEXT_TEXT_BYTES, QueryBlockContext,
    QueryContextReference, QueryDocumentContext, QueryHeadingContext, QueryHeadingReference,
    QueryLexicalContext, QueryNodeContext, analyze_query_document_context,
    default_projection_output_name, is_document_property_field, query_field,
    query_group_output_name, valid_query_alias, valid_query_output_name,
};
use crate::task_projection::TaskProjectionAccessMode;
use crate::{
    CalendarDate, DocumentRevision, InventoryIssueCode, NodeId, QueryComparisonOperator,
    QueryDirection, QueryExpression, QueryExpressionKind, QueryField, QueryFieldReference,
    QueryLiteral, QueryNullPlacement, QueryPlan, QueryScope, QuerySource, QueryValueExpression,
    QueryValueExpressionKind, QueryValueType, TaskDateTime, TaskNodePriority, TaskNodeState,
    TaskNodeTemporal, TaskRow, TaskRowEvidence, TaskRowKind, TaskWorkspaceProjection,
    TaskWorkspaceProjectionDiagnostic, TaskWorkspaceProjectionError, WorkspaceDocumentGeneration,
    analyze_query_source, scan_workspace,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum QueryDisclosureMode {
    Complete,
    Filtered,
}

/// Explicit permission-filtered set of managed nodes available to one query execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryAccessScope {
    node_ids: BTreeSet<NodeId>,
    disclosure: QueryDisclosureMode,
}

impl QueryAccessScope {
    /// Creates a restricted scope whose unavailable subtree diagnostics never distinguish missing
    /// from hidden nodes.
    #[must_use]
    pub fn filtered(node_ids: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            node_ids: node_ids.into_iter().collect(),
            disclosure: QueryDisclosureMode::Filtered,
        }
    }

    /// Creates a complete local or Owner-equivalent workspace scope.
    #[must_use]
    pub fn complete(node_ids: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            node_ids: node_ids.into_iter().collect(),
            disclosure: QueryDisclosureMode::Complete,
        }
    }

    #[must_use]
    pub fn allows(&self, node_id: NodeId) -> bool {
        self.node_ids.contains(&node_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryExecutionBinding {
    #[serde(deserialize_with = "deserialize_optional_query_node_id")]
    pub node_id: Option<NodeId>,
    pub heading: Option<QueryHeadingContext>,
}

fn deserialize_optional_query_node_id<'de, D>(deserializer: D) -> Result<Option<NodeId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|value| NodeId::from_str(&value).map_err(serde::de::Error::custom))
        .transpose()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryEvaluationContext {
    pub today: CalendarDate,
    pub now: TaskNodeTemporal,
    pub timezone: String,
    pub locale: String,
    pub binding: QueryExecutionBinding,
    #[serde(skip)]
    lexical_context: Option<QueryLexicalContext>,
}

impl QueryEvaluationContext {
    /// Creates an explicit deterministic temporal, locale, and saved-query binding context.
    ///
    /// # Errors
    ///
    /// Returns an error when any accepted context field is invalid.
    pub fn new(
        today: CalendarDate,
        now: TaskNodeTemporal,
        timezone: String,
        locale: String,
        binding: QueryExecutionBinding,
    ) -> Result<Self, QueryEvaluationContextError> {
        let context = Self {
            today,
            now,
            timezone,
            locale,
            binding,
            lexical_context: None,
        };
        validate_evaluation_context(&context)?;
        Ok(context)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryEvaluationContextError {
    InvalidToday,
    InvalidNow,
    InvalidTimezone,
    InvalidLocale,
}

impl fmt::Display for QueryEvaluationContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToday => formatter.write_str("query today value is not a valid date"),
            Self::InvalidNow => {
                formatter.write_str("query now value must be an explicit-offset instant")
            }
            Self::InvalidTimezone => formatter.write_str("query timezone is invalid or too large"),
            Self::InvalidLocale => formatter.write_str("query locale is invalid or too large"),
        }
    }
}

impl std::error::Error for QueryEvaluationContextError {}

fn validate_evaluation_context(
    context: &QueryEvaluationContext,
) -> Result<(), QueryEvaluationContextError> {
    CalendarDate::new(context.today.year, context.today.month, context.today.day)
        .map_err(|_| QueryEvaluationContextError::InvalidToday)?;
    if !matches!(context.now, TaskNodeTemporal::Instant(_)) {
        return Err(QueryEvaluationContextError::InvalidNow);
    }
    if context.timezone.is_empty()
        || context.timezone.len() > 128
        || context.timezone.contains(['\r', '\n', '\0'])
        || TimeZone::get(&context.timezone).is_err()
    {
        return Err(QueryEvaluationContextError::InvalidTimezone);
    }
    if context.locale.is_empty()
        || context.locale.len() > 64
        || context.locale.contains(['\r', '\n', '\0'])
    {
        return Err(QueryEvaluationContextError::InvalidLocale);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum QueryRowIdentity {
    Node {
        node_id: NodeId,
        revision: DocumentRevision,
    },
    Task {
        evidence: TaskRowEvidence,
    },
    Heading {
        node_id: NodeId,
        revision: DocumentRevision,
        range: std::ops::Range<u64>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum QueryCellValue {
    Null,
    Text(String),
    Boolean(bool),
    Integer(i64),
    Uuid(String),
    Temporal(TaskNodeTemporal),
    TaskKind(TaskRowKind),
    TaskState(TaskNodeState),
    Priority(TaskNodePriority),
    List(Vec<QueryCellValue>),
    Record(BTreeMap<String, QueryCellValue>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryColumnIdentity {
    pub output_name: String,
    pub path: String,
    pub field: QueryField,
    pub property_key: Option<String>,
    pub value_type: QueryValueType,
    pub nullable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResultCell {
    pub column: QueryColumnIdentity,
    pub value: QueryCellValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResultRow {
    pub identity: QueryRowIdentity,
    pub cells: Vec<QueryResultCell>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResultGroup {
    pub column: QueryColumnIdentity,
    pub value: QueryCellValue,
    pub row_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub source: QuerySource,
    pub columns: Vec<QueryColumnIdentity>,
    pub rows: Vec<QueryResultRow>,
    pub groups: Vec<QueryResultGroup>,
    pub total_before_limit: usize,
    pub truncated: bool,
}

/// Exact-source analysis plus the optional result for one selected canonical query block.
///
/// Invalid or missing blocks return their complete analysis with no result. This lets product
/// callers render Core diagnostics without selecting or interpreting a plan themselves.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySourceExecution {
    pub block_index: usize,
    pub analysis: crate::QuerySourceAnalysis,
    pub result: Option<QueryResult>,
    pub csv: Option<String>,
}

#[derive(Clone, Debug)]
struct QueryNodeRecord {
    node_id: NodeId,
    parent_id: Option<NodeId>,
    name: String,
    path: String,
    depth: u16,
    revision: DocumentRevision,
    document: QueryDocumentContext,
    headings: Vec<QueryHeadingContext>,
}

#[derive(Clone, Debug)]
pub struct QueryWorkspaceIndex {
    generation: WorkspaceDocumentGeneration,
    nodes: BTreeMap<NodeId, QueryNodeRecord>,
    tasks: TaskWorkspaceProjection,
}

impl QueryWorkspaceIndex {
    /// Rebuilds node and task query inputs from exact managed `AsciiDoc` authority.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid workspace, unsupported generation, unreadable document, or
    /// task-index failure.
    pub fn rebuild(root: impl AsRef<Path>) -> Result<Self, QueryWorkspaceError> {
        Self::rebuild_internal(root.as_ref(), None)
    }

    /// Rebuilds query inputs from only an already-authorized logical node
    /// projection. Hidden document bodies are never opened, and node
    /// path/parent/depth fields use the supplied visible topology.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid workspace/scope or an unreadable
    /// authorized document.
    pub fn rebuild_scoped(
        root: impl AsRef<Path>,
        scope: &crate::WorkspaceReadScope,
    ) -> Result<Self, QueryWorkspaceError> {
        Self::rebuild_internal(root.as_ref(), Some(scope))
    }

    fn rebuild_internal(
        root: &Path,
        scope: Option<&crate::WorkspaceReadScope>,
    ) -> Result<Self, QueryWorkspaceError> {
        let inventory = scan_workspace(root);
        if let Some(scope) = scope {
            scope
                .validate_inventory(&inventory)
                .map_err(|_| QueryWorkspaceError::InvalidScope)?;
        } else if !inventory.is_valid() {
            return Err(QueryWorkspaceError::InvalidWorkspace(
                inventory
                    .issues
                    .first()
                    .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
            ));
        }
        if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
            return Err(QueryWorkspaceError::UnsupportedGeneration(
                inventory.generation,
            ));
        }
        let tasks =
            TaskWorkspaceProjection::rebuild_from_validated_inventory(root, &inventory, scope)
                .map_err(QueryWorkspaceError::TaskProjection)?;
        let mut nodes = BTreeMap::new();
        for node in &inventory.nodes {
            if crate::workspace_trash::is_trash_storage_path(root, &node.path) {
                continue;
            }
            let Some(node_id) = node.id else {
                if scope.is_some() {
                    continue;
                }
                return Err(QueryWorkspaceError::InvalidWorkspace(
                    InventoryIssueCode::MissingIdentity,
                ));
            };
            if scope.is_some_and(|scope| !scope.allows(node_id)) {
                continue;
            }
            let (parent_id, path, depth) = if let Some(scope) = scope {
                let locator = scope
                    .locator(node_id)
                    .ok_or(QueryWorkspaceError::InvalidScope)?;
                (
                    scope.parent_node_id(node_id),
                    if locator.is_empty() {
                        "/".to_owned()
                    } else {
                        format!("/{locator}")
                    },
                    scope
                        .depth(node_id)
                        .ok_or(QueryWorkspaceError::InvalidScope)?,
                )
            } else {
                let path = logical_node_path(root, &node.path)?;
                let depth = if path == "/" {
                    0
                } else {
                    u16::try_from(path.bytes().filter(|byte| *byte == b'/').count())
                        .unwrap_or(u16::MAX)
                };
                (node.parent_id, path, depth)
            };
            let source = fs::read_to_string(&node.document_path).map_err(|error| {
                QueryWorkspaceError::DocumentRead {
                    node_id,
                    message: error.to_string(),
                }
            })?;
            let (mut document, headings) = analyze_query_document_context(&source);
            if document.display_title.is_none() {
                document.display_title = Some(node.name.clone());
            }
            nodes.insert(
                node_id,
                QueryNodeRecord {
                    node_id,
                    parent_id,
                    name: node.name.clone(),
                    path,
                    depth,
                    revision: tasks.document_revision(node_id).cloned().ok_or_else(|| {
                        QueryWorkspaceError::DocumentRead {
                            node_id,
                            message:
                                "canonical task projection omitted an authorized document revision"
                                    .to_owned(),
                        }
                    })?,
                    document,
                    headings,
                },
            );
        }
        Ok(Self {
            generation: inventory.generation,
            nodes,
            tasks,
        })
    }

    #[must_use]
    pub const fn generation(&self) -> WorkspaceDocumentGeneration {
        self.generation
    }

    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Returns task diagnostics re-derived for the supplied authorization scope. Filtered access
    /// treats every unavailable dependency uniformly and omits hidden dependency identities.
    #[must_use]
    pub fn task_diagnostics(
        &self,
        access: &QueryAccessScope,
    ) -> Vec<TaskWorkspaceProjectionDiagnostic> {
        self.tasks
            .derive_for_access(task_projection_access_mode(access), |node_id| {
                access.allows(node_id)
            })
            .diagnostics
    }

    /// Executes one Core-typed plan against only explicitly authorized nodes.
    ///
    /// Permission filtering is applied before field evaluation, matching counts, grouping,
    /// ordering, projection, and limit. A filtered scope never distinguishes a hidden subtree root
    /// from a missing UUID.
    ///
    /// # Errors
    ///
    /// Returns an error for a forged/invalid plan, missing owning-node context, or unavailable
    /// scope root.
    pub fn execute(
        &self,
        plan: &QueryPlan,
        access: &QueryAccessScope,
        context: &QueryEvaluationContext,
    ) -> Result<QueryResult, QueryExecutionError> {
        self.execute_internal(plan, access, context, None)
    }

    fn execute_internal(
        &self,
        plan: &QueryPlan,
        access: &QueryAccessScope,
        context: &QueryEvaluationContext,
        lexical_context: Option<QueryLexicalContext>,
    ) -> Result<QueryResult, QueryExecutionError> {
        let context = self.prepare_execution_context(context, access, lexical_context)?;
        if !valid_query_plan(plan) {
            return Err(QueryExecutionError::InvalidPlan);
        }
        if plan.source == QuerySource::Templates {
            return Err(QueryExecutionError::DomainUnavailable(
                QuerySource::Templates,
            ));
        }
        validate_plan_context(plan, &context)?;
        let scope = self.resolve_scope(&plan.scope, access, &context)?;
        let mut rows = match plan.source {
            QuerySource::Nodes => self.node_rows(access, &scope),
            QuerySource::Tasks => self.task_rows(access, &scope),
            QuerySource::Headings => self.heading_rows(access, &scope),
            QuerySource::Templates => unreachable!("handled unavailable domain"),
        };
        validate_source_rows(&rows)?;
        let mut budget = QueryEvaluationBudget::default();
        if let Some(filter) = &plan.filter {
            let mut filtered = Vec::with_capacity(rows.len());
            for row in rows {
                if evaluate_expression(filter, &row, &context, &mut budget)? {
                    filtered.push(row);
                }
            }
            rows = filtered;
        }
        let mut prepared_rows = Vec::with_capacity(rows.len());
        for row in rows {
            let group_value = plan
                .group
                .as_ref()
                .map(|group| {
                    evaluate_value(&group.expression, &row, &context, &mut budget)
                        .map(|value| value.unwrap_or(QueryCellValue::Null))
                })
                .transpose()?;
            let mut sort_values = Vec::with_capacity(plan.sort.len());
            for sort in &plan.sort {
                sort_values.push(
                    evaluate_value(&sort.expression, &row, &context, &mut budget)?
                        .unwrap_or(QueryCellValue::Null),
                );
            }
            prepared_rows.push(PreparedQueryRow {
                row,
                group_value,
                sort_values,
            });
        }
        prepared_rows.sort_by(|left, right| compare_prepared_rows(left, right, plan, &context));
        let groups =
            collect_prepared_groups(&prepared_rows, plan.source, plan.group.as_ref(), &context);
        let total_before_limit = prepared_rows.len();
        prepared_rows.truncate(usize::from(plan.limit));
        let columns = plan
            .projection
            .iter()
            .map(|projection| query_expression_column_identity(plan.source, projection))
            .collect::<Vec<_>>();
        validate_result_output_strings(&columns, &groups)?;
        let mut result_bytes = checked_serialized_size(&columns, 0, crate::QUERY_MAX_RESULT_BYTES)?;
        result_bytes =
            checked_serialized_size(&groups, result_bytes, crate::QUERY_MAX_RESULT_BYTES)?;
        let mut result_rows = Vec::with_capacity(prepared_rows.len());
        for prepared in prepared_rows {
            let mut cells = Vec::with_capacity(plan.projection.len());
            for projection in &plan.projection {
                cells.push(QueryResultCell {
                    column: query_expression_column_identity(plan.source, projection),
                    value: evaluate_value(
                        &projection.expression,
                        &prepared.row,
                        &context,
                        &mut budget,
                    )?
                    .unwrap_or(QueryCellValue::Null),
                });
            }
            let result_row = QueryResultRow {
                identity: prepared.row.identity,
                cells,
            };
            result_bytes =
                checked_serialized_size(&result_row, result_bytes, crate::QUERY_MAX_RESULT_BYTES)?;
            result_rows.push(result_row);
        }
        let result = QueryResult {
            source: plan.source,
            columns,
            rows: result_rows,
            groups,
            total_before_limit,
            truncated: total_before_limit > usize::from(plan.limit),
        };
        checked_serialized_size(&result, 0, crate::QUERY_MAX_RESULT_BYTES)?;
        Ok(result)
    }

    fn prepare_execution_context(
        &self,
        context: &QueryEvaluationContext,
        access: &QueryAccessScope,
        lexical_context: Option<QueryLexicalContext>,
    ) -> Result<QueryEvaluationContext, QueryExecutionError> {
        validate_evaluation_context(context).map_err(|_| QueryExecutionError::InvalidContext)?;
        let mut prepared = context.clone();
        if let Some(node_id) = context.binding.node_id {
            self.require_available_scope_node(node_id, access)?;
            let node = self
                .nodes
                .get(&node_id)
                .ok_or(QueryExecutionError::InvalidContext)?;
            prepared.lexical_context = if let Some(mut lexical) = lexical_context {
                if lexical.document.display_title.is_none() {
                    lexical.document.display_title = Some(node.name.clone());
                }
                if context.binding.heading.is_some()
                    && context.binding.heading.as_ref() != lexical.heading.as_ref()
                {
                    return Err(QueryExecutionError::InvalidContext);
                }
                lexical.node = Some(query_node_context(node));
                Some(lexical)
            } else {
                let heading = if let Some(heading) = &context.binding.heading {
                    if !node.headings.contains(heading) {
                        return Err(QueryExecutionError::InvalidContext);
                    }
                    Some(heading.clone())
                } else {
                    None
                };
                Some(QueryLexicalContext {
                    node: Some(query_node_context(node)),
                    document: node.document.clone(),
                    heading,
                    query: QueryBlockContext { title: None },
                })
            };
        } else {
            if context.binding.heading.is_some() {
                return Err(QueryExecutionError::MissingContext("this.node"));
            }
            prepared.lexical_context = lexical_context;
        }
        Ok(prepared)
    }

    /// Analyzes exact `AsciiDoc` source and executes one valid canonical query block.
    ///
    /// A missing or invalid block is not an execution failure: the returned analysis contains the
    /// source diagnostics and `result` is `None`. Authorization, scope, and context failures from a
    /// valid plan remain errors.
    ///
    /// # Errors
    ///
    /// Returns an error when a valid selected plan fails permission, scope, or context checks.
    pub fn execute_source(
        &self,
        source: &str,
        block_index: usize,
        access: &QueryAccessScope,
        context: &QueryEvaluationContext,
    ) -> Result<QuerySourceExecution, QueryExecutionError> {
        validate_evaluation_context(context).map_err(|_| QueryExecutionError::InvalidContext)?;
        if let Some(node_id) = context.binding.node_id {
            self.require_available_scope_node(node_id, access)?;
        }
        let analysis = analyze_query_source(source);
        let result = analysis
            .blocks
            .get(block_index)
            .and_then(|block| {
                block
                    .plan
                    .as_ref()
                    .map(|plan| (plan, block.lexical_context.clone()))
            })
            .map(|(plan, lexical)| self.execute_internal(plan, access, context, Some(lexical)))
            .transpose()?;
        let csv = result.as_ref().map(query_result_csv);
        Ok(QuerySourceExecution {
            block_index,
            analysis,
            result,
            csv,
        })
    }

    fn resolve_scope(
        &self,
        scope: &QueryScope,
        access: &QueryAccessScope,
        context: &QueryEvaluationContext,
    ) -> Result<ResolvedQueryScope, QueryExecutionError> {
        match scope {
            QueryScope::Workspace => Ok(ResolvedQueryScope::Workspace),
            QueryScope::SubtreeThisNode | QueryScope::DescendantsThisNode => {
                let current = context
                    .binding
                    .node_id
                    .ok_or(QueryExecutionError::MissingContext("this.node"))?;
                self.require_available_scope_node(current, access)?;
                Ok(if matches!(scope, QueryScope::DescendantsThisNode) {
                    ResolvedQueryScope::Descendants(current)
                } else {
                    ResolvedQueryScope::Subtree(current)
                })
            }
            QueryScope::SectionThisHeading => {
                let node_id = context
                    .binding
                    .node_id
                    .ok_or(QueryExecutionError::MissingContext("this.node"))?;
                let heading = context
                    .lexical_context
                    .as_ref()
                    .and_then(|lexical| lexical.heading.as_ref())
                    .ok_or(QueryExecutionError::MissingHeadingContext)?;
                Ok(ResolvedQueryScope::Section {
                    node_id,
                    range: heading.section_range.clone(),
                })
            }
        }
    }

    fn require_available_scope_node(
        &self,
        node_id: NodeId,
        access: &QueryAccessScope,
    ) -> Result<(), QueryExecutionError> {
        if self.nodes.contains_key(&node_id) && access.allows(node_id) {
            return Ok(());
        }
        match access.disclosure {
            QueryDisclosureMode::Complete => Err(QueryExecutionError::MissingScopeNode(node_id)),
            QueryDisclosureMode::Filtered => Err(QueryExecutionError::UnavailableScope),
        }
    }

    fn node_rows(
        &self,
        access: &QueryAccessScope,
        scope: &ResolvedQueryScope,
    ) -> Vec<WorkingQueryRow> {
        self.nodes
            .values()
            .filter(|node| {
                access.allows(node.node_id) && self.row_in_scope(node.node_id, None, scope)
            })
            .map(WorkingQueryRow::from_node)
            .collect()
    }

    fn task_rows(
        &self,
        access: &QueryAccessScope,
        scope: &ResolvedQueryScope,
    ) -> Vec<WorkingQueryRow> {
        self.tasks
            .derive_for_access(task_projection_access_mode(access), |node_id| {
                access.allows(node_id)
            })
            .rows
            .into_iter()
            .filter(|row| {
                let range = match &row.evidence {
                    TaskRowEvidence::Checklist { item_range, .. } => Some(item_range),
                    TaskRowEvidence::Node { .. } => None,
                };
                self.row_in_scope(row.owner_node_id, range, scope)
            })
            .filter_map(|row| {
                self.nodes
                    .get(&row.owner_node_id)
                    .map(|owner| WorkingQueryRow::from_task(&row, owner))
            })
            .collect()
    }

    fn heading_rows(
        &self,
        access: &QueryAccessScope,
        scope: &ResolvedQueryScope,
    ) -> Vec<WorkingQueryRow> {
        self.nodes
            .values()
            .filter(|node| access.allows(node.node_id))
            .flat_map(|node| {
                node.headings
                    .iter()
                    .filter(move |heading| {
                        self.row_in_scope(node.node_id, Some(&heading.range), scope)
                    })
                    .map(|heading| WorkingQueryRow::from_heading(node, heading))
            })
            .collect()
    }

    fn row_in_scope(
        &self,
        node_id: NodeId,
        range: Option<&std::ops::Range<u64>>,
        scope: &ResolvedQueryScope,
    ) -> bool {
        match scope {
            ResolvedQueryScope::Workspace => true,
            ResolvedQueryScope::Descendants(current) => {
                node_id != *current && self.is_descendant_of(node_id, *current)
            }
            ResolvedQueryScope::Subtree(root) => {
                node_id == *root || self.is_descendant_of(node_id, *root)
            }
            ResolvedQueryScope::Section {
                node_id: owner,
                range: section,
            } => {
                node_id == *owner
                    && range.is_some_and(|range| {
                        section.start <= range.start && range.end <= section.end
                    })
            }
        }
    }

    fn is_descendant_of(&self, mut node_id: NodeId, ancestor: NodeId) -> bool {
        let mut steps = 0_usize;
        while let Some(parent) = self.nodes.get(&node_id).and_then(|node| node.parent_id) {
            if parent == ancestor {
                return true;
            }
            node_id = parent;
            steps += 1;
            if steps > self.nodes.len() {
                return false;
            }
        }
        false
    }
}

const fn task_projection_access_mode(access: &QueryAccessScope) -> TaskProjectionAccessMode {
    match access.disclosure {
        QueryDisclosureMode::Complete => TaskProjectionAccessMode::Complete,
        QueryDisclosureMode::Filtered => TaskProjectionAccessMode::Filtered,
    }
}

/// Serializes only the already-authorized projected query result as deterministic UTF-8 CSV.
/// Headers use canonical projection output names, records use CRLF, null is empty, and RFC 4180
/// quoting is applied to text and every other rendered cell when required.
#[must_use]
pub fn query_result_csv(result: &QueryResult) -> String {
    let mut csv = String::new();
    csv.push_str(
        &result
            .columns
            .iter()
            .map(|column| csv_cell(&column.output_name))
            .collect::<Vec<_>>()
            .join(","),
    );
    csv.push_str("\r\n");
    for row in &result.rows {
        for (index, column) in result.columns.iter().enumerate() {
            if index > 0 {
                csv.push(',');
            }
            let value = row
                .cells
                .iter()
                .find(|cell| cell.column == *column)
                .map_or_else(String::new, |cell| query_cell_text(&cell.value));
            csv.push_str(&csv_cell(&value));
        }
        csv.push_str("\r\n");
    }
    csv
}

fn query_cell_text(value: &QueryCellValue) -> String {
    match value {
        QueryCellValue::Null => String::new(),
        QueryCellValue::Text(value) | QueryCellValue::Uuid(value) => value.clone(),
        QueryCellValue::Boolean(value) => value.to_string(),
        QueryCellValue::Integer(value) => value.to_string(),
        QueryCellValue::Temporal(value) => value.as_str().to_owned(),
        QueryCellValue::TaskKind(value) => match value {
            TaskRowKind::Checklist => "checklist",
            TaskRowKind::Node => "node",
        }
        .to_owned(),
        QueryCellValue::TaskState(value) => match value {
            TaskNodeState::Todo => "todo",
            TaskNodeState::InProgress => "in-progress",
            TaskNodeState::OnHold => "on-hold",
            TaskNodeState::Completed => "completed",
            TaskNodeState::Cancelled => "cancelled",
        }
        .to_owned(),
        QueryCellValue::Priority(value) => match value {
            TaskNodePriority::Lowest => "lowest",
            TaskNodePriority::Low => "low",
            TaskNodePriority::Normal => "normal",
            TaskNodePriority::Medium => "medium",
            TaskNodePriority::High => "high",
            TaskNodePriority::Highest => "highest",
        }
        .to_owned(),
        QueryCellValue::List(value) => serde_json::to_string(value).unwrap_or_default(),
        QueryCellValue::Record(value) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod csv_tests {
    use std::collections::BTreeMap;

    use super::{QueryCellValue, QueryEvaluationBudget, QueryExecutionError, csv_cell};

    #[test]
    fn csv_cells_apply_rfc_4180_quoting() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("comma,value"), "\"comma,value\"");
        assert_eq!(csv_cell("quoted \"value\""), "\"quoted \"\"value\"\"\"");
        assert_eq!(csv_cell("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn nested_result_values_enforce_string_and_collection_limits() {
        let oversized = "x".repeat(crate::QUERY_MAX_STRING_LITERAL_BYTES + 1);
        for value in [
            QueryCellValue::List(vec![QueryCellValue::Text(oversized.clone())]),
            QueryCellValue::Record(BTreeMap::from([(
                "nested".to_owned(),
                QueryCellValue::Text(oversized.clone()),
            )])),
            QueryCellValue::Record(BTreeMap::from([(oversized, QueryCellValue::Null)])),
        ] {
            assert_eq!(
                QueryEvaluationBudget::default().charge_value(&value),
                Err(QueryExecutionError::ResourceLimit)
            );
        }
        for value in [
            QueryCellValue::List(vec![QueryCellValue::Null; 65]),
            QueryCellValue::Record(
                (0..65)
                    .map(|index| (format!("field_{index}"), QueryCellValue::Null))
                    .collect(),
            ),
        ] {
            assert_eq!(
                QueryEvaluationBudget::default().charge_value(&value),
                Err(QueryExecutionError::ResourceLimit)
            );
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResolvedQueryScope {
    Workspace,
    Descendants(NodeId),
    Subtree(NodeId),
    Section {
        node_id: NodeId,
        range: std::ops::Range<u64>,
    },
}

#[derive(Clone, Debug)]
struct WorkingQueryRow {
    identity: QueryRowIdentity,
    values: BTreeMap<QueryField, QueryCellValue>,
    tie_path: String,
    tie_kind: Option<TaskRowKind>,
    tie_range_start: u64,
    tie_id: String,
    properties: BTreeMap<String, String>,
}

impl WorkingQueryRow {
    fn from_node(node: &QueryNodeRecord) -> Self {
        let mut values = BTreeMap::new();
        values.insert(QueryField::Id, uuid_cell(node.node_id));
        values.insert(QueryField::Name, QueryCellValue::Text(node.name.clone()));
        values.insert(QueryField::Path, QueryCellValue::Text(node.path.clone()));
        values.insert(
            QueryField::ParentId,
            node.parent_id.map_or(QueryCellValue::Null, uuid_cell),
        );
        values.insert(
            QueryField::Depth,
            QueryCellValue::Integer(i64::from(node.depth)),
        );
        values.insert(
            QueryField::NodeDisplayTitle,
            QueryCellValue::Text(
                node.document
                    .display_title
                    .clone()
                    .unwrap_or_else(|| node.name.clone()),
            ),
        );
        insert_document_values(&mut values, &node.document);
        Self {
            identity: QueryRowIdentity::Node {
                node_id: node.node_id,
                revision: node.revision.clone(),
            },
            values,
            tie_path: node.path.clone(),
            tie_kind: None,
            tie_range_start: 0,
            tie_id: node.node_id.to_string(),
            properties: node.document.properties.clone(),
        }
    }

    fn from_task(row: &TaskRow, owner: &QueryNodeRecord) -> Self {
        let mut values = BTreeMap::new();
        values.insert(QueryField::Kind, QueryCellValue::TaskKind(row.kind));
        values.insert(
            QueryField::Id,
            row.id.map_or(QueryCellValue::Null, |node_id| {
                QueryCellValue::Uuid(node_id.to_string())
            }),
        );
        values.insert(QueryField::OwnerNodeId, uuid_cell(row.owner_node_id));
        values.insert(
            QueryField::OwnerNodeName,
            QueryCellValue::Text(row.owner_node_name.clone()),
        );
        values.insert(
            QueryField::OwnerNodePath,
            QueryCellValue::Text(row.owner_node_path.clone()),
        );
        insert_owner_node_values(&mut values, owner);
        values.insert(
            QueryField::Title,
            QueryCellValue::Text(row.description.clone()),
        );
        values.insert(QueryField::Closed, QueryCellValue::Boolean(row.closed));
        values.insert(QueryField::State, QueryCellValue::TaskState(row.state));
        values.insert(
            QueryField::ChecklistDepth,
            row.checklist_depth.map_or(QueryCellValue::Null, |depth| {
                QueryCellValue::Integer(i64::from(depth))
            }),
        );
        insert_document_values(&mut values, &owner.document);
        values.insert(
            QueryField::Priority,
            row.priority
                .map_or(QueryCellValue::Null, QueryCellValue::Priority),
        );
        for (field, value) in [
            (QueryField::Created, row.created.clone()),
            (QueryField::Start, row.start.clone()),
            (QueryField::Scheduled, row.scheduled.clone()),
            (QueryField::Due, row.due.clone()),
            (QueryField::ClosedAt, row.closed_at.clone()),
        ] {
            values.insert(
                field,
                value.map_or(QueryCellValue::Null, QueryCellValue::Temporal),
            );
        }
        values.insert(
            QueryField::Blocked,
            row.blocked
                .map_or(QueryCellValue::Null, QueryCellValue::Boolean),
        );
        let tie_range_start = match &row.evidence {
            TaskRowEvidence::Checklist { item_range, .. } => item_range.start,
            TaskRowEvidence::Node { .. } => u64::MAX,
        };
        Self {
            identity: QueryRowIdentity::Task {
                evidence: row.evidence.clone(),
            },
            values,
            tie_path: row.owner_node_path.clone(),
            tie_kind: Some(row.kind),
            tie_range_start,
            tie_id: row
                .id
                .map_or_else(String::new, |node_id| node_id.to_string()),
            properties: owner.document.properties.clone(),
        }
    }

    fn from_heading(node: &QueryNodeRecord, heading: &QueryHeadingContext) -> Self {
        let mut values = BTreeMap::new();
        values.insert(
            QueryField::Title,
            QueryCellValue::Text(heading.title.clone()),
        );
        values.insert(
            QueryField::Level,
            QueryCellValue::Integer(i64::from(heading.level)),
        );
        values.insert(
            QueryField::Anchor,
            heading
                .anchor
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        );
        values.insert(
            QueryField::HeadingParent,
            heading
                .parent
                .as_ref()
                .map_or(QueryCellValue::Null, |parent| {
                    QueryCellValue::Record(BTreeMap::from([
                        (
                            "title".to_owned(),
                            QueryCellValue::Text(parent.title.clone()),
                        ),
                        (
                            "level".to_owned(),
                            QueryCellValue::Integer(i64::from(parent.level)),
                        ),
                        (
                            "anchor".to_owned(),
                            parent
                                .anchor
                                .clone()
                                .map_or(QueryCellValue::Null, QueryCellValue::Text),
                        ),
                        (
                            "path".to_owned(),
                            QueryCellValue::List(
                                parent
                                    .path
                                    .iter()
                                    .cloned()
                                    .map(QueryCellValue::Text)
                                    .collect(),
                            ),
                        ),
                    ]))
                }),
        );
        values.insert(
            QueryField::HeadingPath,
            QueryCellValue::List(
                heading
                    .path
                    .iter()
                    .cloned()
                    .map(QueryCellValue::Text)
                    .collect(),
            ),
        );
        values.insert(QueryField::OwnerNodeId, uuid_cell(node.node_id));
        values.insert(
            QueryField::OwnerNodeName,
            QueryCellValue::Text(node.name.clone()),
        );
        values.insert(
            QueryField::OwnerNodePath,
            QueryCellValue::Text(node.path.clone()),
        );
        insert_owner_node_values(&mut values, node);
        insert_document_values(&mut values, &node.document);
        insert_heading_document_values(&mut values, &node.document);
        Self {
            identity: QueryRowIdentity::Heading {
                node_id: node.node_id,
                revision: node.revision.clone(),
                range: heading.range.clone(),
            },
            values,
            tie_path: node.path.clone(),
            tie_kind: None,
            tie_range_start: heading.range.start,
            tie_id: heading.anchor.clone().unwrap_or_default(),
            properties: node.document.properties.clone(),
        }
    }

    fn value(&self, field: QueryField) -> QueryCellValue {
        self.values
            .get(&field)
            .cloned()
            .unwrap_or(QueryCellValue::Null)
    }

    fn value_reference(&self, field: &QueryFieldReference) -> QueryCellValue {
        if is_document_property_field(field.field) {
            return field
                .custom_property
                .as_ref()
                .and_then(|key| self.properties.get(key))
                .cloned()
                .map_or(QueryCellValue::Null, QueryCellValue::Text);
        }
        self.value(field.field)
    }

    fn validate_source_strings(&self) -> Result<(), QueryExecutionError> {
        for value in self.values.values() {
            validate_query_cell_value(value)?;
        }
        validate_query_string(&self.tie_path)?;
        validate_query_string(&self.tie_id)?;
        for (key, value) in &self.properties {
            validate_query_string(key)?;
            validate_query_string(value)?;
        }
        Ok(())
    }
}

fn validate_source_rows(rows: &[WorkingQueryRow]) -> Result<(), QueryExecutionError> {
    rows.iter()
        .try_for_each(WorkingQueryRow::validate_source_strings)
}

fn validate_result_output_strings(
    columns: &[QueryColumnIdentity],
    groups: &[QueryResultGroup],
) -> Result<(), QueryExecutionError> {
    for column in columns {
        validate_query_column_strings(column)?;
    }
    for group in groups {
        validate_query_column_strings(&group.column)?;
        validate_query_cell_value(&group.value)?;
    }
    Ok(())
}

fn insert_heading_document_values(
    values: &mut BTreeMap<QueryField, QueryCellValue>,
    document: &QueryDocumentContext,
) {
    values.insert(
        QueryField::HeadingDocumentTitle,
        document
            .title
            .clone()
            .map_or(QueryCellValue::Null, QueryCellValue::Text),
    );
    values.insert(
        QueryField::HeadingDocumentSubtitle,
        document
            .subtitle
            .clone()
            .map_or(QueryCellValue::Null, QueryCellValue::Text),
    );
    values.insert(
        QueryField::HeadingDocumentDisplayTitle,
        document
            .display_title
            .clone()
            .map_or(QueryCellValue::Null, QueryCellValue::Text),
    );
}

fn insert_document_values(
    values: &mut BTreeMap<QueryField, QueryCellValue>,
    document: &QueryDocumentContext,
) {
    values.insert(
        QueryField::DocumentTitle,
        document
            .title
            .clone()
            .map_or(QueryCellValue::Null, QueryCellValue::Text),
    );
    values.insert(
        QueryField::DocumentSubtitle,
        document
            .subtitle
            .clone()
            .map_or(QueryCellValue::Null, QueryCellValue::Text),
    );
    values.insert(
        QueryField::DocumentDisplayTitle,
        document
            .display_title
            .clone()
            .map_or(QueryCellValue::Null, QueryCellValue::Text),
    );
}

fn insert_owner_node_values(
    values: &mut BTreeMap<QueryField, QueryCellValue>,
    node: &QueryNodeRecord,
) {
    values.insert(
        QueryField::OwnerNodeParentId,
        node.parent_id.map_or(QueryCellValue::Null, uuid_cell),
    );
    values.insert(
        QueryField::OwnerNodeDepth,
        QueryCellValue::Integer(i64::from(node.depth)),
    );
    values.insert(
        QueryField::OwnerNodeDisplayTitle,
        QueryCellValue::Text(
            node.document
                .display_title
                .clone()
                .unwrap_or_else(|| node.name.clone()),
        ),
    );
}

fn query_node_context(node: &QueryNodeRecord) -> QueryNodeContext {
    QueryNodeContext {
        id: node.node_id,
        name: node.name.clone(),
        path: node.path.clone(),
        depth: node.depth,
        display_title: node
            .document
            .display_title
            .clone()
            .unwrap_or_else(|| node.name.clone()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryWorkspaceError {
    InvalidWorkspace(InventoryIssueCode),
    UnsupportedGeneration(WorkspaceDocumentGeneration),
    DocumentRead { node_id: NodeId, message: String },
    InvalidNodePath,
    InvalidScope,
    TaskProjection(TaskWorkspaceProjectionError),
}

impl fmt::Display for QueryWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace(code) => {
                write!(formatter, "workspace inventory is invalid: {code:?}")
            }
            Self::UnsupportedGeneration(generation) => {
                write!(
                    formatter,
                    "query runtime requires AsciiDoc v1, got {generation:?}"
                )
            }
            Self::DocumentRead { node_id, message } => {
                write!(formatter, "could not read node {node_id}: {message}")
            }
            Self::InvalidNodePath => {
                formatter.write_str("workspace node path is not portable UTF-8")
            }
            Self::InvalidScope => formatter.write_str("query read scope is invalid"),
            Self::TaskProjection(error) => {
                write!(formatter, "could not build task query input: {error}")
            }
        }
    }
}

impl std::error::Error for QueryWorkspaceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryExecutionError {
    InvalidPlan,
    InvalidContext,
    MissingContext(&'static str),
    MissingHeadingContext,
    NullComparison,
    ResourceLimit,
    DomainUnavailable(QuerySource),
    MissingScopeNode(NodeId),
    UnavailableScope,
}

impl fmt::Display for QueryExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPlan => formatter.write_str("query plan is not a valid Core v1 plan"),
            Self::InvalidContext => formatter.write_str("query evaluation context is not valid"),
            Self::MissingContext(root) => write!(formatter, "missing_context: {root}"),
            Self::MissingHeadingContext => formatter.write_str("missing_heading_context"),
            Self::NullComparison => formatter.write_str("null_comparison"),
            Self::ResourceLimit => formatter.write_str("resource_limit"),
            Self::DomainUnavailable(source) => {
                write!(formatter, "domain_unavailable: {source:?}")
            }
            Self::MissingScopeNode(node_id) => {
                write!(formatter, "query scope node {node_id} does not exist")
            }
            Self::UnavailableScope => formatter.write_str("query scope is unavailable"),
        }
    }
}

impl std::error::Error for QueryExecutionError {}

#[derive(Default)]
struct QueryEvaluationBudget {
    steps: usize,
    materialized_bytes: usize,
}

impl QueryEvaluationBudget {
    fn step(&mut self) -> Result<(), QueryExecutionError> {
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(QueryExecutionError::ResourceLimit)?;
        if self.steps > crate::QUERY_MAX_EVALUATION_STEPS {
            return Err(QueryExecutionError::ResourceLimit);
        }
        Ok(())
    }

    fn charge_value(&mut self, value: &QueryCellValue) -> Result<(), QueryExecutionError> {
        validate_query_cell_value(value)?;
        self.materialized_bytes = checked_serialized_size(
            value,
            self.materialized_bytes,
            crate::QUERY_MAX_RESULT_BYTES,
        )?;
        Ok(())
    }
}

fn validate_query_string(value: &str) -> Result<(), QueryExecutionError> {
    if value.len() > crate::QUERY_MAX_STRING_LITERAL_BYTES {
        return Err(QueryExecutionError::ResourceLimit);
    }
    Ok(())
}

const QUERY_MAX_TYPED_COLLECTION_ITEMS: usize = 64;

fn validate_query_cell_value(value: &QueryCellValue) -> Result<(), QueryExecutionError> {
    match value {
        QueryCellValue::Text(value) | QueryCellValue::Uuid(value) => validate_query_string(value),
        QueryCellValue::Temporal(value) => validate_query_string(value.as_str()),
        QueryCellValue::List(values) => {
            if values.len() > QUERY_MAX_TYPED_COLLECTION_ITEMS {
                return Err(QueryExecutionError::ResourceLimit);
            }
            for value in values {
                validate_query_cell_value(value)?;
            }
            Ok(())
        }
        QueryCellValue::Record(values) => {
            if values.len() > QUERY_MAX_TYPED_COLLECTION_ITEMS {
                return Err(QueryExecutionError::ResourceLimit);
            }
            for (key, value) in values {
                validate_query_string(key)?;
                validate_query_cell_value(value)?;
            }
            Ok(())
        }
        QueryCellValue::Null
        | QueryCellValue::Boolean(_)
        | QueryCellValue::Integer(_)
        | QueryCellValue::TaskKind(_)
        | QueryCellValue::TaskState(_)
        | QueryCellValue::Priority(_) => Ok(()),
    }
}

fn validate_query_column_strings(column: &QueryColumnIdentity) -> Result<(), QueryExecutionError> {
    validate_query_string(&column.output_name)?;
    validate_query_string(&column.path)?;
    if let Some(property_key) = &column.property_key {
        validate_query_string(property_key)?;
    }
    Ok(())
}

struct SerializedSizeWriter {
    written: usize,
    limit: usize,
}

impl Write for SerializedSizeWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .written
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("serialized size overflow"))?;
        if next > self.limit {
            return Err(io::Error::other("serialized size limit exceeded"));
        }
        self.written = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn checked_serialized_size<T: Serialize>(
    value: &T,
    already_written: usize,
    limit: usize,
) -> Result<usize, QueryExecutionError> {
    if already_written > limit {
        return Err(QueryExecutionError::ResourceLimit);
    }
    let mut writer = SerializedSizeWriter {
        written: already_written,
        limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| QueryExecutionError::ResourceLimit)?;
    Ok(writer.written)
}

fn logical_node_path(root: &Path, path: &Path) -> Result<String, QueryWorkspaceError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| QueryWorkspaceError::InvalidNodePath)?;
    let components = relative
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(str::to_owned)
                .ok_or(QueryWorkspaceError::InvalidNodePath)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(if components.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", components.join("/"))
    })
}

fn uuid_cell(node_id: NodeId) -> QueryCellValue {
    QueryCellValue::Uuid(node_id.to_string())
}

fn evaluate_expression(
    expression: &QueryExpression,
    row: &WorkingQueryRow,
    context: &QueryEvaluationContext,
    budget: &mut QueryEvaluationBudget,
) -> Result<bool, QueryExecutionError> {
    budget.step()?;
    match &expression.kind {
        QueryExpressionKind::Boolean { value } => Ok(matches!(
            evaluate_value(value, row, context, budget)?,
            Some(QueryCellValue::Boolean(true))
        )),
        QueryExpressionKind::Comparison {
            left,
            operator,
            right,
        } => compare_predicate(left, *operator, right, row, context, budget),
        QueryExpressionKind::In { left, values } => {
            let cell = evaluate_value(left, row, context, budget)?;
            if matches!(cell, None | Some(QueryCellValue::Null)) {
                return Ok(false);
            }
            for value in values {
                if matches!(
                    &value.kind,
                    QueryValueExpressionKind::Literal {
                        literal: QueryLiteral::Null
                    }
                ) {
                    continue;
                }
                if compare_predicate(
                    left,
                    QueryComparisonOperator::Equal,
                    value,
                    row,
                    context,
                    budget,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        QueryExpressionKind::IsNull { value, negated } => Ok(matches!(
            evaluate_value(value, row, context, budget)?,
            None | Some(QueryCellValue::Null)
        ) != *negated),
        QueryExpressionKind::Not { expression } => {
            Ok(!evaluate_expression(expression, row, context, budget)?)
        }
        QueryExpressionKind::And { left, right } => {
            if !evaluate_expression(left, row, context, budget)? {
                return Ok(false);
            }
            evaluate_expression(right, row, context, budget)
        }
        QueryExpressionKind::Or { left, right } => {
            if evaluate_expression(left, row, context, budget)? {
                return Ok(true);
            }
            evaluate_expression(right, row, context, budget)
        }
    }
}

fn compare_predicate(
    left: &QueryValueExpression,
    operator: QueryComparisonOperator,
    right: &QueryValueExpression,
    row: &WorkingQueryRow,
    context: &QueryEvaluationContext,
    budget: &mut QueryEvaluationBudget,
) -> Result<bool, QueryExecutionError> {
    let Some(left_cell) = evaluate_value(left, row, context, budget)? else {
        return Err(QueryExecutionError::NullComparison);
    };
    if matches!(left_cell, QueryCellValue::Null) {
        return Err(QueryExecutionError::NullComparison);
    }
    let Some(right_cell) = evaluate_value(right, row, context, budget)? else {
        return Err(QueryExecutionError::NullComparison);
    };
    if matches!(right_cell, QueryCellValue::Null) {
        return Err(QueryExecutionError::NullComparison);
    }
    match operator {
        QueryComparisonOperator::Contains => Ok(match (left_cell, right_cell) {
            (QueryCellValue::Text(left), QueryCellValue::Text(right)) => left.contains(&right),
            _ => false,
        }),
        QueryComparisonOperator::StartsWith => Ok(match (left_cell, right_cell) {
            (QueryCellValue::Text(left), QueryCellValue::Text(right)) => left.starts_with(&right),
            _ => false,
        }),
        _ => Ok(
            compare_cell_values(&left_cell, &right_cell, left.value_type, context).is_some_and(
                |ordering| match operator {
                    QueryComparisonOperator::Equal => ordering == Ordering::Equal,
                    QueryComparisonOperator::NotEqual => ordering != Ordering::Equal,
                    QueryComparisonOperator::LessThan => ordering == Ordering::Less,
                    QueryComparisonOperator::LessThanOrEqual => ordering != Ordering::Greater,
                    QueryComparisonOperator::GreaterThan => ordering == Ordering::Greater,
                    QueryComparisonOperator::GreaterThanOrEqual => ordering != Ordering::Less,
                    QueryComparisonOperator::Contains | QueryComparisonOperator::StartsWith => {
                        false
                    }
                },
            ),
        ),
    }
}

fn evaluate_value(
    expression: &QueryValueExpression,
    row: &WorkingQueryRow,
    context: &QueryEvaluationContext,
    budget: &mut QueryEvaluationBudget,
) -> Result<Option<QueryCellValue>, QueryExecutionError> {
    budget.step()?;
    let value = match &expression.kind {
        QueryValueExpressionKind::SourceField { reference } => Some(row.value_reference(reference)),
        QueryValueExpressionKind::Literal { literal } => match literal {
            QueryLiteral::String(text) => match expression.value_type {
                QueryValueType::TaskKind => parse_task_kind(text).map(QueryCellValue::TaskKind),
                QueryValueType::TaskState => parse_task_state(text).map(QueryCellValue::TaskState),
                QueryValueType::Priority => parse_priority(text).map(QueryCellValue::Priority),
                _ => Some(QueryCellValue::Text(text.clone())),
            },
            QueryLiteral::Boolean(value) => Some(QueryCellValue::Boolean(*value)),
            QueryLiteral::Number(value) => Some(QueryCellValue::Integer(*value)),
            QueryLiteral::Uuid(value) => Some(QueryCellValue::Uuid(value.clone())),
            QueryLiteral::Temporal(value) => Some(QueryCellValue::Temporal(value.clone())),
            QueryLiteral::DurationDays(value) => Some(QueryCellValue::Text(format!("P{value}D"))),
            QueryLiteral::Null => Some(QueryCellValue::Null),
        },
        QueryValueExpressionKind::Context { reference } => {
            evaluate_context_reference(reference, context)
        }
        QueryValueExpressionKind::DateOffset { base, days } => {
            let Some(QueryCellValue::Temporal(base)) = evaluate_value(base, row, context, budget)?
            else {
                return Ok(None);
            };
            offset_task_node_temporal(&base, *days).map(QueryCellValue::Temporal)
        }
    };
    if let Some(value) = &value {
        budget.charge_value(value)?;
    }
    Ok(value)
}

#[allow(clippy::too_many_lines)]
fn evaluate_context_reference(
    reference: &QueryContextReference,
    context: &QueryEvaluationContext,
) -> Option<QueryCellValue> {
    let lexical = context.lexical_context.as_ref();
    match reference {
        QueryContextReference::ContextToday => {
            Some(QueryCellValue::Temporal(TaskNodeTemporal::Date(format!(
                "{:04}-{:02}-{:02}",
                context.today.year, context.today.month, context.today.day
            ))))
        }
        QueryContextReference::ContextNow => Some(QueryCellValue::Temporal(context.now.clone())),
        QueryContextReference::ContextTimezone => {
            Some(QueryCellValue::Text(context.timezone.clone()))
        }
        QueryContextReference::ContextLocale => Some(QueryCellValue::Text(context.locale.clone())),
        QueryContextReference::ThisNodeId => lexical?.node.as_ref().map(|node| uuid_cell(node.id)),
        QueryContextReference::ThisNodeName => lexical?
            .node
            .as_ref()
            .map(|node| QueryCellValue::Text(node.name.clone())),
        QueryContextReference::ThisNodePath => lexical?
            .node
            .as_ref()
            .map(|node| QueryCellValue::Text(node.path.clone())),
        QueryContextReference::ThisNodeDepth => lexical?
            .node
            .as_ref()
            .map(|node| QueryCellValue::Integer(i64::from(node.depth))),
        QueryContextReference::ThisNodeDisplayTitle => lexical?
            .node
            .as_ref()
            .map(|node| QueryCellValue::Text(node.display_title.clone())),
        QueryContextReference::ThisDocumentTitle => Some(
            lexical?
                .document
                .title
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
        QueryContextReference::ThisDocumentSubtitle => Some(
            lexical?
                .document
                .subtitle
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
        QueryContextReference::ThisDocumentDisplayTitle => Some(
            lexical?
                .document
                .display_title
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
        QueryContextReference::ThisDocumentProperty(key) => Some(
            lexical?
                .document
                .properties
                .get(key)
                .cloned()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
        QueryContextReference::ThisHeadingTitle => lexical?
            .heading
            .as_ref()
            .map(|heading| QueryCellValue::Text(heading.title.clone())),
        QueryContextReference::ThisHeadingLevel => lexical?
            .heading
            .as_ref()
            .map(|heading| QueryCellValue::Integer(i64::from(heading.level))),
        QueryContextReference::ThisHeadingAnchor => Some(
            lexical?
                .heading
                .as_ref()?
                .anchor
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
        QueryContextReference::ThisHeadingParent => Some(
            lexical?
                .heading
                .as_ref()?
                .parent
                .as_ref()
                .map_or(QueryCellValue::Null, heading_parent_cell),
        ),
        QueryContextReference::ThisHeadingPath => lexical?.heading.as_ref().map(|heading| {
            QueryCellValue::List(
                heading
                    .path
                    .iter()
                    .cloned()
                    .map(QueryCellValue::Text)
                    .collect(),
            )
        }),
        QueryContextReference::ThisQueryTitle => Some(
            lexical?
                .query
                .title
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
    }
}

fn heading_parent_cell(parent: &QueryHeadingReference) -> QueryCellValue {
    QueryCellValue::Record(BTreeMap::from([
        (
            "title".to_owned(),
            QueryCellValue::Text(parent.title.clone()),
        ),
        (
            "level".to_owned(),
            QueryCellValue::Integer(i64::from(parent.level)),
        ),
        (
            "anchor".to_owned(),
            parent
                .anchor
                .clone()
                .map_or(QueryCellValue::Null, QueryCellValue::Text),
        ),
        (
            "path".to_owned(),
            QueryCellValue::List(
                parent
                    .path
                    .iter()
                    .cloned()
                    .map(QueryCellValue::Text)
                    .collect(),
            ),
        ),
    ]))
}

struct PreparedQueryRow {
    row: WorkingQueryRow,
    group_value: Option<QueryCellValue>,
    sort_values: Vec<QueryCellValue>,
}

fn compare_prepared_rows(
    left: &PreparedQueryRow,
    right: &PreparedQueryRow,
    plan: &QueryPlan,
    context: &QueryEvaluationContext,
) -> Ordering {
    if let Some(group) = &plan.group {
        let ordering = compare_sort_cell(
            left.group_value.as_ref().unwrap_or(&QueryCellValue::Null),
            right.group_value.as_ref().unwrap_or(&QueryCellValue::Null),
            group.expression.value_type,
            QueryDirection::Ascending,
            QueryNullPlacement::Last,
            context,
        );
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    for (index, sort) in plan.sort.iter().enumerate() {
        let ordering = compare_sort_cell(
            &left.sort_values[index],
            &right.sort_values[index],
            sort.expression.value_type,
            sort.direction,
            sort.nulls,
            context,
        );
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.row
        .tie_path
        .cmp(&right.row.tie_path)
        .then_with(|| left.row.tie_kind.cmp(&right.row.tie_kind))
        .then_with(|| left.row.tie_range_start.cmp(&right.row.tie_range_start))
        .then_with(|| left.row.tie_id.cmp(&right.row.tie_id))
}

fn compare_sort_cell(
    left: &QueryCellValue,
    right: &QueryCellValue,
    value_type: QueryValueType,
    direction: QueryDirection,
    nulls: QueryNullPlacement,
    context: &QueryEvaluationContext,
) -> Ordering {
    match (left, right) {
        (QueryCellValue::Null, QueryCellValue::Null) => Ordering::Equal,
        (QueryCellValue::Null, _) => match nulls {
            QueryNullPlacement::First => Ordering::Less,
            QueryNullPlacement::Last => Ordering::Greater,
        },
        (_, QueryCellValue::Null) => match nulls {
            QueryNullPlacement::First => Ordering::Greater,
            QueryNullPlacement::Last => Ordering::Less,
        },
        _ => {
            let ordering =
                compare_cell_values(left, right, value_type, context).unwrap_or(Ordering::Equal);
            if direction == QueryDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        }
    }
}

fn compare_cell_values(
    left: &QueryCellValue,
    right: &QueryCellValue,
    value_type: QueryValueType,
    context: &QueryEvaluationContext,
) -> Option<Ordering> {
    match (value_type, left, right) {
        (QueryValueType::String, QueryCellValue::Text(left), QueryCellValue::Text(right))
        | (QueryValueType::Uuid, QueryCellValue::Uuid(left), QueryCellValue::Uuid(right)) => {
            Some(left.cmp(right))
        }
        (
            QueryValueType::Boolean,
            QueryCellValue::Boolean(left),
            QueryCellValue::Boolean(right),
        ) => Some(left.cmp(right)),
        (QueryValueType::Number, QueryCellValue::Integer(left), QueryCellValue::Integer(right)) => {
            Some(left.cmp(right))
        }
        (
            QueryValueType::Temporal | QueryValueType::Date | QueryValueType::Instant,
            QueryCellValue::Temporal(left),
            QueryCellValue::Temporal(right),
        ) => compare_task_node_temporal(left, right, context_utc_offset_minutes(context)?),
        (
            QueryValueType::Priority,
            QueryCellValue::Priority(left),
            QueryCellValue::Priority(right),
        ) => Some(left.cmp(right)),
        (
            QueryValueType::TaskKind,
            QueryCellValue::TaskKind(left),
            QueryCellValue::TaskKind(right),
        ) => Some(left.cmp(right)),
        (
            QueryValueType::TaskState,
            QueryCellValue::TaskState(left),
            QueryCellValue::TaskState(right),
        ) => Some(task_state_rank(*left).cmp(&task_state_rank(*right))),
        (QueryValueType::List, QueryCellValue::List(left), QueryCellValue::List(right)) => Some(
            serde_json::to_string(left)
                .ok()?
                .cmp(&serde_json::to_string(right).ok()?),
        ),
        (QueryValueType::Record, QueryCellValue::Record(left), QueryCellValue::Record(right)) => {
            Some(
                serde_json::to_string(left)
                    .ok()?
                    .cmp(&serde_json::to_string(right).ok()?),
            )
        }
        _ => None,
    }
}

fn collect_prepared_groups(
    rows: &[PreparedQueryRow],
    source: QuerySource,
    group: Option<&crate::QueryGroup>,
    context: &QueryEvaluationContext,
) -> Vec<QueryResultGroup> {
    let Some(group) = group else {
        return Vec::new();
    };
    let column = query_group_column_identity(source, group);
    let mut groups = Vec::<QueryResultGroup>::new();
    for row in rows {
        let value = row.group_value.clone().unwrap_or(QueryCellValue::Null);
        if let Some(current) = groups.last_mut()
            && compare_sort_cell(
                &current.value,
                &value,
                group.expression.value_type,
                QueryDirection::Ascending,
                QueryNullPlacement::Last,
                context,
            ) == Ordering::Equal
        {
            current.row_count += 1;
            continue;
        }
        groups.push(QueryResultGroup {
            column: column.clone(),
            value,
            row_count: 1,
        });
    }
    groups
}

fn parse_task_kind(value: &str) -> Option<TaskRowKind> {
    match value {
        "checklist" => Some(TaskRowKind::Checklist),
        "node" => Some(TaskRowKind::Node),
        _ => None,
    }
}

fn parse_task_state(value: &str) -> Option<TaskNodeState> {
    match value {
        "todo" => Some(TaskNodeState::Todo),
        "in-progress" => Some(TaskNodeState::InProgress),
        "on-hold" => Some(TaskNodeState::OnHold),
        "completed" => Some(TaskNodeState::Completed),
        "cancelled" => Some(TaskNodeState::Cancelled),
        _ => None,
    }
}

fn parse_priority(value: &str) -> Option<TaskNodePriority> {
    match value {
        "lowest" => Some(TaskNodePriority::Lowest),
        "low" => Some(TaskNodePriority::Low),
        "normal" => Some(TaskNodePriority::Normal),
        "medium" => Some(TaskNodePriority::Medium),
        "high" => Some(TaskNodePriority::High),
        "highest" => Some(TaskNodePriority::Highest),
        _ => None,
    }
}

const fn task_state_rank(value: TaskNodeState) -> u8 {
    match value {
        TaskNodeState::Todo => 0,
        TaskNodeState::InProgress => 1,
        TaskNodeState::OnHold => 2,
        TaskNodeState::Completed => 3,
        TaskNodeState::Cancelled => 4,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ComparableTemporal {
    Date(i64),
    Instant { seconds: i64, fraction: String },
}

fn context_utc_offset_minutes(context: &QueryEvaluationContext) -> Option<i16> {
    let TaskNodeTemporal::Instant(value) = &context.now else {
        return None;
    };
    let time = value.get(11..)?;
    let offset_start = if time.as_bytes().get(8) == Some(&b'.') {
        9 + time[9..].bytes().take_while(u8::is_ascii_digit).count()
    } else {
        8
    };
    let offset = time.get(offset_start..)?;
    if offset == "Z" {
        return Some(0);
    }
    let sign = if offset.get(..1) == Some("+") {
        1_i16
    } else {
        -1_i16
    };
    let hour = offset.get(1..3)?.parse::<i16>().ok()?;
    let minute = offset.get(4..6)?.parse::<i16>().ok()?;
    Some(sign * (hour * 60 + minute))
}

fn compare_task_node_temporal(
    left: &TaskNodeTemporal,
    right: &TaskNodeTemporal,
    utc_offset_minutes: i16,
) -> Option<Ordering> {
    let left = comparable_task_node_temporal(left)?;
    let right = comparable_task_node_temporal(right)?;
    Some(compare_comparable_temporal(
        &left,
        &right,
        utc_offset_minutes,
    ))
}

pub(crate) fn compare_temporal(
    left: &TaskDateTime,
    right: &TaskDateTime,
    utc_offset_minutes: i16,
) -> Option<Ordering> {
    let left = comparable_temporal(left)?;
    let right = comparable_temporal(right)?;
    Some(compare_comparable_temporal(
        &left,
        &right,
        utc_offset_minutes,
    ))
}

fn compare_comparable_temporal(
    left: &ComparableTemporal,
    right: &ComparableTemporal,
    utc_offset_minutes: i16,
) -> Ordering {
    match (left, right) {
        (ComparableTemporal::Date(left), ComparableTemporal::Date(right)) => left.cmp(right),
        (
            ComparableTemporal::Instant {
                seconds: left_seconds,
                fraction: left_fraction,
            },
            ComparableTemporal::Instant {
                seconds: right_seconds,
                fraction: right_fraction,
            },
        ) => left_seconds
            .cmp(right_seconds)
            .then_with(|| compare_fraction(left_fraction, right_fraction)),
        (ComparableTemporal::Date(left), ComparableTemporal::Instant { seconds, .. }) => {
            left.cmp(&local_day(*seconds, utc_offset_minutes))
        }
        (ComparableTemporal::Instant { seconds, .. }, ComparableTemporal::Date(right)) => {
            local_day(*seconds, utc_offset_minutes).cmp(right)
        }
    }
}

fn comparable_task_node_temporal(value: &TaskNodeTemporal) -> Option<ComparableTemporal> {
    comparable_temporal_source(value.as_str(), matches!(value, TaskNodeTemporal::Date(_)))
}

fn comparable_temporal(value: &TaskDateTime) -> Option<ComparableTemporal> {
    match value {
        TaskDateTime::Date(value) => comparable_temporal_source(value, true),
        TaskDateTime::Instant(value) => comparable_temporal_source(value, false),
    }
}

fn comparable_temporal_source(value: &str, date_only: bool) -> Option<ComparableTemporal> {
    if date_only {
        Some(ComparableTemporal::Date(parse_date_ordinal(value)?))
    } else {
        let date = parse_date_ordinal(&value[..10])?;
        let time = &value[11..];
        let hour = time[0..2].parse::<i64>().ok()?;
        let minute = time[3..5].parse::<i64>().ok()?;
        let second = time[6..8].parse::<i64>().ok()?;
        let offset_start = if time.as_bytes().get(8) == Some(&b'.') {
            9 + time[9..].bytes().take_while(u8::is_ascii_digit).count()
        } else {
            8
        };
        let fraction = if offset_start > 8 {
            time[9..offset_start].trim_end_matches('0').to_owned()
        } else {
            String::new()
        };
        let offset = &time[offset_start..];
        let offset_minutes = if offset == "Z" {
            0_i64
        } else {
            let sign = if &offset[..1] == "+" { 1_i64 } else { -1_i64 };
            sign * (offset[1..3].parse::<i64>().ok()? * 60 + offset[4..6].parse::<i64>().ok()?)
        };
        Some(ComparableTemporal::Instant {
            seconds: date * 86_400 + hour * 3_600 + minute * 60 + second - offset_minutes * 60,
            fraction,
        })
    }
}

fn offset_task_node_temporal(value: &TaskNodeTemporal, days: i32) -> Option<TaskNodeTemporal> {
    match comparable_task_node_temporal(value)? {
        ComparableTemporal::Date(ordinal) => {
            let (year, month, day) = date_from_ordinal(ordinal + i64::from(days))?;
            Some(TaskNodeTemporal::Date(format!(
                "{year:04}-{month:02}-{day:02}"
            )))
        }
        ComparableTemporal::Instant { .. } => {
            let TaskNodeTemporal::Instant(source) = value else {
                unreachable!()
            };
            let date = parse_date_ordinal(&source[..10])? + i64::from(days);
            let (year, month, day) = date_from_ordinal(date)?;
            Some(TaskNodeTemporal::Instant(format!(
                "{year:04}-{month:02}-{day:02}{}",
                &source[10..]
            )))
        }
    }
}

pub(crate) fn offset_temporal(value: &TaskDateTime, days: i32) -> Option<TaskDateTime> {
    match comparable_temporal(value)? {
        ComparableTemporal::Date(ordinal) => {
            let (year, month, day) = date_from_ordinal(ordinal + i64::from(days))?;
            Some(TaskDateTime::Date(format!("{year:04}-{month:02}-{day:02}")))
        }
        ComparableTemporal::Instant { .. } => {
            let TaskDateTime::Instant(source) = value else {
                unreachable!()
            };
            let date = parse_date_ordinal(&source[..10])? + i64::from(days);
            let (year, month, day) = date_from_ordinal(date)?;
            Some(TaskDateTime::Instant(format!(
                "{year:04}-{month:02}-{day:02}{}",
                &source[10..]
            )))
        }
    }
}

pub(crate) fn temporal_day_ordinal(value: &TaskDateTime, utc_offset_minutes: i16) -> Option<i64> {
    match comparable_temporal(value)? {
        ComparableTemporal::Date(ordinal) => Some(ordinal),
        ComparableTemporal::Instant { seconds, .. } => Some(local_day(seconds, utc_offset_minutes)),
    }
}

fn compare_fraction(left: &str, right: &str) -> Ordering {
    let length = left.len().max(right.len());
    (0..length)
        .map(|index| {
            left.as_bytes()
                .get(index)
                .copied()
                .unwrap_or(b'0')
                .cmp(&right.as_bytes().get(index).copied().unwrap_or(b'0'))
        })
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

fn local_day(seconds: i64, utc_offset_minutes: i16) -> i64 {
    (seconds + i64::from(utc_offset_minutes) * 60).div_euclid(86_400)
}

pub(crate) fn parse_date_ordinal(value: &str) -> Option<i64> {
    let year = value.get(0..4)?.parse::<i32>().ok()?;
    let month = value.get(5..7)?.parse::<u8>().ok()?;
    let day = value.get(8..10)?.parse::<u8>().ok()?;
    CalendarDate::new(year, month, day).ok()?;
    Some(days_before_year(year) + i64::from(days_before_month(year, month)) + i64::from(day - 1))
}

fn days_before_year(year: i32) -> i64 {
    let years = i64::from(year - 1);
    years * 365 + years / 4 - years / 100 + years / 400
}

fn days_before_month(year: i32, month: u8) -> u16 {
    const STARTS: [u16; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let start = STARTS[usize::from(month - 1)];
    if month > 2 && leap_year(year) {
        start + 1
    } else {
        start
    }
}

fn date_from_ordinal(ordinal: i64) -> Option<(i32, u8, u8)> {
    if ordinal < 0 {
        return None;
    }
    let mut low = 1_i32;
    let mut high = 10_000_i32;
    while low < high {
        let middle = low + (high - low) / 2;
        if days_before_year(middle) <= ordinal {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    let year = low - 1;
    if !(1..=9_999).contains(&year) {
        return None;
    }
    let day_of_year = u16::try_from(ordinal - days_before_year(year)).ok()?;
    let mut month = 1_u8;
    while month < 12 && days_before_month(year, month + 1) <= day_of_year {
        month += 1;
    }
    let day = u8::try_from(day_of_year - days_before_month(year, month) + 1).ok()?;
    Some((year, month, day))
}

const fn leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn validate_plan_context(
    plan: &QueryPlan,
    context: &QueryEvaluationContext,
) -> Result<(), QueryExecutionError> {
    if let Some(filter) = &plan.filter {
        validate_expression_context(filter, context)?;
    }
    Ok(())
}

fn validate_expression_context(
    expression: &QueryExpression,
    context: &QueryEvaluationContext,
) -> Result<(), QueryExecutionError> {
    match &expression.kind {
        QueryExpressionKind::Boolean { value } | QueryExpressionKind::IsNull { value, .. } => {
            validate_value_context(value, context)
        }
        QueryExpressionKind::Comparison { left, right, .. } => {
            validate_value_context(left, context)?;
            validate_value_context(right, context)
        }
        QueryExpressionKind::In { left, values } => {
            validate_value_context(left, context)?;
            for value in values {
                validate_value_context(value, context)?;
            }
            Ok(())
        }
        QueryExpressionKind::Not { expression } => validate_expression_context(expression, context),
        QueryExpressionKind::And { left, right } | QueryExpressionKind::Or { left, right } => {
            validate_expression_context(left, context)?;
            validate_expression_context(right, context)
        }
    }
}

fn validate_value_context(
    value: &QueryValueExpression,
    context: &QueryEvaluationContext,
) -> Result<(), QueryExecutionError> {
    match &value.kind {
        QueryValueExpressionKind::SourceField { .. } | QueryValueExpressionKind::Literal { .. } => {
            Ok(())
        }
        QueryValueExpressionKind::DateOffset { base, .. } => validate_value_context(base, context),
        QueryValueExpressionKind::Context { reference } => {
            if matches!(
                reference,
                QueryContextReference::ThisNodeId
                    | QueryContextReference::ThisNodeName
                    | QueryContextReference::ThisNodePath
                    | QueryContextReference::ThisNodeDepth
                    | QueryContextReference::ThisNodeDisplayTitle
                    | QueryContextReference::ThisDocumentTitle
                    | QueryContextReference::ThisDocumentSubtitle
                    | QueryContextReference::ThisDocumentDisplayTitle
                    | QueryContextReference::ThisDocumentProperty(_)
                    | QueryContextReference::ThisHeadingTitle
                    | QueryContextReference::ThisHeadingLevel
                    | QueryContextReference::ThisHeadingAnchor
                    | QueryContextReference::ThisHeadingParent
                    | QueryContextReference::ThisHeadingPath
                    | QueryContextReference::ThisQueryTitle
            ) && context.binding.node_id.is_none()
            {
                return Err(QueryExecutionError::MissingContext("this.node"));
            }
            let lexical = context.lexical_context.as_ref();
            match reference {
                QueryContextReference::ThisNodeId
                | QueryContextReference::ThisNodeName
                | QueryContextReference::ThisNodePath
                | QueryContextReference::ThisNodeDepth
                | QueryContextReference::ThisNodeDisplayTitle
                    if lexical.and_then(|value| value.node.as_ref()).is_none() =>
                {
                    Err(QueryExecutionError::MissingContext("this.node"))
                }
                QueryContextReference::ThisDocumentTitle
                | QueryContextReference::ThisDocumentSubtitle
                | QueryContextReference::ThisDocumentDisplayTitle
                | QueryContextReference::ThisDocumentProperty(_)
                    if lexical.is_none() =>
                {
                    Err(QueryExecutionError::MissingContext("this.document"))
                }
                QueryContextReference::ThisHeadingTitle
                | QueryContextReference::ThisHeadingLevel
                | QueryContextReference::ThisHeadingAnchor
                | QueryContextReference::ThisHeadingParent
                | QueryContextReference::ThisHeadingPath
                    if lexical.and_then(|value| value.heading.as_ref()).is_none() =>
                {
                    Err(QueryExecutionError::MissingHeadingContext)
                }
                QueryContextReference::ThisQueryTitle if lexical.is_none() => {
                    Err(QueryExecutionError::MissingContext("this.query"))
                }
                _ => Ok(()),
            }
        }
    }
}

fn valid_query_plan(plan: &QueryPlan) -> bool {
    if plan.expression_capability != QUERY_EXPRESSION_CAPABILITY_ID
        || !valid_query_alias(&plan.alias)
        || plan.limit == 0
        || plan.limit > crate::QUERY_MAX_LIMIT
        || (plan.source == QuerySource::Templates && plan.scope != QueryScope::Workspace)
        || plan.filter.is_none()
        || plan.sort.is_empty()
        || plan.projection.is_empty()
    {
        return false;
    }
    if plan.sort.len() > crate::QUERY_MAX_SORT_FIELDS
        || plan.projection.len() > crate::QUERY_MAX_PROJECTION_FIELDS
    {
        return false;
    }
    if plan
        .sort
        .iter()
        .filter_map(|sort| source_field_reference(&sort.expression))
        .map(|field| (field.field, field.custom_property.as_deref()))
        .collect::<BTreeSet<_>>()
        .len()
        != plan.sort.len()
        || plan
            .projection
            .iter()
            .filter_map(|projection| source_field_reference(&projection.expression))
            .map(|field| (field.field, field.custom_property.as_deref()))
            .collect::<BTreeSet<_>>()
            .len()
            != plan.projection.len()
        || plan
            .projection
            .iter()
            .map(|projection| projection.output_name.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != plan.projection.len()
    {
        return false;
    }
    let fields_valid = plan
        .sort
        .iter()
        .map(|sort| &sort.expression)
        .chain(plan.group.iter().map(|group| &group.expression))
        .chain(
            plan.projection
                .iter()
                .map(|projection| &projection.expression),
        )
        .all(|expression| valid_source_field_expression(plan.source, &plan.alias, expression));
    let projections_valid = plan.projection.iter().all(|projection| {
        valid_query_output_name(&projection.output_name)
            && (projection.output_explicit
                || source_field_reference(&projection.expression).is_some_and(|field| {
                    !is_document_property_field(field.field)
                        && projection.output_name == default_projection_output_name(field.field)
                }))
            && projection.range.start <= projection.range.end
            && projection.range.start <= projection.expression.range.start
            && projection.expression.range.end <= projection.range.end
            && if projection.output_explicit {
                projection.range.end > projection.expression.range.end
            } else {
                projection.range == projection.expression.range
            }
    });
    let group_valid = plan.group.as_ref().is_none_or(|group| {
        query_group_output_name(group).is_some_and(|name| {
            valid_query_output_name(&name)
                && plan
                    .projection
                    .iter()
                    .all(|projection| projection.output_name != name)
        }) && group.range.start == group.expression.range.start
            && if group.output_name.is_some() {
                group.range.end > group.expression.range.end
            } else {
                group.range == group.expression.range
            }
    });
    fields_valid
        && projections_valid
        && group_valid
        && plan
            .filter
            .as_ref()
            .is_none_or(|filter| valid_expression(plan.source, &plan.alias, filter, 0, &mut 0))
}

fn valid_expression(
    source: QuerySource,
    alias: &str,
    expression: &QueryExpression,
    depth: usize,
    nodes: &mut usize,
) -> bool {
    *nodes += 1;
    if depth > crate::QUERY_MAX_NESTING || *nodes > crate::QUERY_MAX_EXPRESSION_NODES {
        return false;
    }
    match &expression.kind {
        QueryExpressionKind::Boolean { value } => {
            valid_value(source, alias, value, QueryValueType::Boolean)
        }
        QueryExpressionKind::Comparison {
            left,
            operator,
            right,
        } => {
            valid_source_field_expression(source, alias, left)
                && valid_value(source, alias, right, left.value_type)
                && valid_operator(left.value_type, *operator)
        }
        QueryExpressionKind::In { left, values } => {
            valid_source_field_expression(source, alias, left)
                && !values.is_empty()
                && values.len() <= crate::QUERY_MAX_IN_VALUES
                && values.iter().all(|value| {
                    if value.value_type == QueryValueType::Null {
                        valid_value(source, alias, value, QueryValueType::Null)
                    } else {
                        valid_value(source, alias, value, left.value_type)
                    }
                })
                && !values.iter().enumerate().any(|(index, value)| {
                    values[..index]
                        .iter()
                        .any(|existing| query_values_equal(existing, value))
                })
        }
        QueryExpressionKind::IsNull { value, .. } => source_field_reference(value)
            .is_some_and(|field| valid_field(source, alias, field) && field.nullable),
        QueryExpressionKind::Not { expression } => {
            valid_expression(source, alias, expression, depth + 1, nodes)
        }
        QueryExpressionKind::And { left, right } | QueryExpressionKind::Or { left, right } => {
            valid_expression(source, alias, left, depth + 1, nodes)
                && valid_expression(source, alias, right, depth + 1, nodes)
        }
    }
}

fn source_field_reference(expression: &QueryValueExpression) -> Option<&QueryFieldReference> {
    let QueryValueExpressionKind::SourceField { reference } = &expression.kind else {
        return None;
    };
    Some(reference)
}

fn valid_source_field_expression(
    source: QuerySource,
    alias: &str,
    expression: &QueryValueExpression,
) -> bool {
    source_field_reference(expression).is_some_and(|field| {
        field.value_type == expression.value_type
            && field.range == expression.range
            && valid_field(source, alias, field)
    })
}

fn valid_field(source: QuerySource, alias: &str, field: &QueryFieldReference) -> bool {
    if field.alias != alias
        || field.range.start > field.range.end
        || field.range.end.saturating_sub(field.range.start)
            > u64::try_from(crate::QUERY_MAX_BODY_BYTES).unwrap_or(u64::MAX)
    {
        return false;
    }
    if is_document_property_field(field.field) {
        let source_matches_field = match field.field {
            QueryField::DocumentProperty => {
                matches!(
                    source,
                    QuerySource::Nodes | QuerySource::Tasks | QuerySource::Headings
                )
            }
            QueryField::HeadingDocumentProperty => source == QuerySource::Headings,
            _ => false,
        };
        return source_matches_field
            && field.value_type == QueryValueType::String
            && field.nullable
            && field.custom_property.as_ref().is_some_and(|key| {
                !key.is_empty()
                    && key.len() <= QUERY_MAX_CONTEXT_TEXT_BYTES
                    && !key.contains(['\r', '\n', '\0'])
            });
    }
    if field.custom_property.is_some() {
        return false;
    }
    query_field(source, query_field_reference_name(source, field).as_str()).is_some_and(
        |(expected, value_type, nullable)| {
            expected == field.field && value_type == field.value_type && nullable == field.nullable
        },
    )
}

fn valid_operator(value_type: QueryValueType, operator: QueryComparisonOperator) -> bool {
    match operator {
        QueryComparisonOperator::Contains | QueryComparisonOperator::StartsWith => {
            value_type == QueryValueType::String
        }
        QueryComparisonOperator::LessThan
        | QueryComparisonOperator::LessThanOrEqual
        | QueryComparisonOperator::GreaterThan
        | QueryComparisonOperator::GreaterThanOrEqual => matches!(
            value_type,
            QueryValueType::Number | QueryValueType::Temporal | QueryValueType::Priority
        ),
        QueryComparisonOperator::Equal | QueryComparisonOperator::NotEqual => true,
    }
}

fn valid_value(
    source: QuerySource,
    alias: &str,
    value: &QueryValueExpression,
    expected: QueryValueType,
) -> bool {
    if value.value_type != expected
        && !(expected == QueryValueType::Temporal
            && matches!(
                value.value_type,
                QueryValueType::Date | QueryValueType::Instant
            ))
    {
        return false;
    }
    match &value.kind {
        QueryValueExpressionKind::SourceField { .. } => {
            valid_source_field_expression(source, alias, value)
        }
        QueryValueExpressionKind::Literal { literal } => match (expected, literal) {
            (QueryValueType::String, QueryLiteral::String(value)) => {
                value.len() <= crate::QUERY_MAX_STRING_LITERAL_BYTES
            }
            (QueryValueType::Boolean, QueryLiteral::Boolean(_))
            | (QueryValueType::Date, QueryLiteral::Temporal(TaskNodeTemporal::Date(_)))
            | (QueryValueType::Instant, QueryLiteral::Temporal(TaskNodeTemporal::Instant(_)))
            | (QueryValueType::Null, QueryLiteral::Null) => true,
            (QueryValueType::Number, QueryLiteral::Number(value)) => *value >= 0,
            (QueryValueType::Uuid, QueryLiteral::Uuid(value)) => NodeId::from_str(value).is_ok(),
            (QueryValueType::Temporal, QueryLiteral::Temporal(temporal)) => {
                TaskNodeTemporal::parse(temporal.as_str()).is_ok_and(|parsed| {
                    parsed == *temporal
                        && matches!(
                            (&parsed, value.value_type),
                            (TaskNodeTemporal::Date(_), QueryValueType::Date)
                                | (TaskNodeTemporal::Instant(_), QueryValueType::Instant)
                        )
                })
            }
            (QueryValueType::TaskKind, QueryLiteral::String(value)) => {
                parse_task_kind(value).is_some()
            }
            (QueryValueType::TaskState, QueryLiteral::String(value)) => {
                parse_task_state(value).is_some()
            }
            (QueryValueType::Priority, QueryLiteral::String(value)) => {
                parse_priority(value).is_some()
            }
            (QueryValueType::Duration, QueryLiteral::DurationDays(days)) => {
                (1..=36_500).contains(days)
            }
            _ => false,
        },
        QueryValueExpressionKind::Context { reference } => {
            context_reference_type(reference) == value.value_type
        }
        QueryValueExpressionKind::DateOffset { base, days } => {
            expected == QueryValueType::Temporal
                && days
                    .checked_abs()
                    .is_some_and(|days| (1..=36_500).contains(&days))
                && !matches!(&base.kind, QueryValueExpressionKind::DateOffset { .. })
                && matches!(
                    base.value_type,
                    QueryValueType::Date | QueryValueType::Instant
                )
                && valid_value(source, alias, base, base.value_type)
        }
    }
}

fn query_values_equal(left: &QueryValueExpression, right: &QueryValueExpression) -> bool {
    if left.value_type != right.value_type {
        return false;
    }
    match (&left.kind, &right.kind) {
        (
            QueryValueExpressionKind::SourceField { reference: left },
            QueryValueExpressionKind::SourceField { reference: right },
        ) => left == right,
        (
            QueryValueExpressionKind::Literal { literal: left },
            QueryValueExpressionKind::Literal { literal: right },
        ) => left == right,
        (
            QueryValueExpressionKind::Context { reference: left },
            QueryValueExpressionKind::Context { reference: right },
        ) => left == right,
        (
            QueryValueExpressionKind::DateOffset {
                base: left,
                days: left_days,
            },
            QueryValueExpressionKind::DateOffset {
                base: right,
                days: right_days,
            },
        ) => left_days == right_days && query_values_equal(left, right),
        _ => false,
    }
}

fn query_expression_column_identity(
    source: QuerySource,
    projection: &crate::QueryProjection,
) -> QueryColumnIdentity {
    query_value_column_identity(
        source,
        &projection.expression,
        projection.output_name.clone(),
    )
}

fn query_group_column_identity(
    source: QuerySource,
    group: &crate::QueryGroup,
) -> QueryColumnIdentity {
    query_value_column_identity(
        source,
        &group.expression,
        query_group_output_name(group).expect("validated group is a source-field expression"),
    )
}

fn query_value_column_identity(
    source: QuerySource,
    expression: &QueryValueExpression,
    output_name: String,
) -> QueryColumnIdentity {
    let field = source_field_reference(expression)
        .expect("validated projection is a source-field expression");
    QueryColumnIdentity {
        output_name,
        path: query_field_reference_name(source, field),
        field: field.field,
        property_key: field.custom_property.clone(),
        value_type: field.value_type,
        nullable: field.nullable,
    }
}

fn query_field_reference_name(source: QuerySource, field: &QueryFieldReference) -> String {
    if is_document_property_field(field.field) {
        let root = match (source, field.field) {
            (QuerySource::Tasks, QueryField::DocumentProperty) => "owner_node.document.properties",
            (QuerySource::Headings, QueryField::DocumentProperty) => {
                "owning_node.document.properties"
            }
            (QuerySource::Headings, QueryField::HeadingDocumentProperty)
            | (QuerySource::Nodes, QueryField::DocumentProperty) => "document.properties",
            _ => return String::new(),
        };
        return format!(
            "{root}[{}]",
            serde_json::to_string(field.custom_property.as_deref().unwrap_or_default())
                .unwrap_or_else(|_| "\"\"".to_owned())
        );
    }
    let name = match (source, field.field) {
        (QuerySource::Nodes | QuerySource::Tasks | QuerySource::Templates, QueryField::Id) => "id",
        (QuerySource::Nodes | QuerySource::Templates, QueryField::Name) => "name",
        (QuerySource::Nodes | QuerySource::Templates, QueryField::Path)
        | (QuerySource::Headings, QueryField::HeadingPath) => "path",
        (QuerySource::Nodes, QueryField::ParentId) => "parent_id",
        (QuerySource::Nodes, QueryField::Depth) => "depth",
        (QuerySource::Nodes | QuerySource::Templates, QueryField::NodeDisplayTitle) => {
            "display_title"
        }
        (QuerySource::Tasks, QueryField::Kind) => "kind",
        (QuerySource::Tasks | QuerySource::Headings, QueryField::Title) => "title",
        (QuerySource::Tasks, QueryField::OwnerNodeId) => "owner_node.id",
        (QuerySource::Tasks, QueryField::OwnerNodeName) => "owner_node.name",
        (QuerySource::Tasks, QueryField::OwnerNodePath) => "owner_node.path",
        (QuerySource::Tasks, QueryField::OwnerNodeParentId) => "owner_node.parent_id",
        (QuerySource::Tasks, QueryField::OwnerNodeDepth) => "owner_node.depth",
        (QuerySource::Tasks, QueryField::OwnerNodeDisplayTitle) => "owner_node.display_title",
        (QuerySource::Headings, QueryField::OwnerNodeId) => "owning_node.id",
        (QuerySource::Headings, QueryField::OwnerNodeName) => "owning_node.name",
        (QuerySource::Headings, QueryField::OwnerNodePath) => "owning_node.path",
        (QuerySource::Headings, QueryField::OwnerNodeParentId) => "owning_node.parent_id",
        (QuerySource::Headings, QueryField::OwnerNodeDepth) => "owning_node.depth",
        (QuerySource::Headings, QueryField::OwnerNodeDisplayTitle) => "owning_node.display_title",
        (QuerySource::Tasks, QueryField::Closed) => "closed",
        (QuerySource::Tasks, QueryField::State) => "state",
        (QuerySource::Tasks, QueryField::ChecklistDepth) => "checklist_depth",
        (QuerySource::Tasks, QueryField::Priority) => "priority",
        (QuerySource::Tasks, QueryField::Created) => "created",
        (QuerySource::Tasks, QueryField::Start) => "start",
        (QuerySource::Tasks, QueryField::Scheduled) => "scheduled",
        (QuerySource::Tasks, QueryField::Due) => "due",
        (QuerySource::Tasks, QueryField::ClosedAt) => "closed_at",
        (QuerySource::Tasks, QueryField::Blocked) => "blocked",
        (QuerySource::Headings, QueryField::Level) => "level",
        (QuerySource::Headings, QueryField::Anchor) => "anchor",
        (QuerySource::Headings, QueryField::HeadingParent) => "parent",
        (QuerySource::Nodes, QueryField::DocumentTitle)
        | (QuerySource::Headings, QueryField::HeadingDocumentTitle) => "document.title",
        (QuerySource::Nodes, QueryField::DocumentSubtitle)
        | (QuerySource::Headings, QueryField::HeadingDocumentSubtitle) => "document.subtitle",
        (QuerySource::Nodes, QueryField::DocumentDisplayTitle)
        | (QuerySource::Headings, QueryField::HeadingDocumentDisplayTitle) => {
            "document.display_title"
        }
        (QuerySource::Tasks, QueryField::DocumentTitle) => "owner_node.document.title",
        (QuerySource::Tasks, QueryField::DocumentSubtitle) => "owner_node.document.subtitle",
        (QuerySource::Tasks, QueryField::DocumentDisplayTitle) => {
            "owner_node.document.display_title"
        }
        (QuerySource::Headings, QueryField::DocumentTitle) => "owning_node.document.title",
        (QuerySource::Headings, QueryField::DocumentSubtitle) => "owning_node.document.subtitle",
        (QuerySource::Headings, QueryField::DocumentDisplayTitle) => {
            "owning_node.document.display_title"
        }
        (QuerySource::Templates, QueryField::PartCount) => "part_count",
        (QuerySource::Templates, QueryField::ParameterCount) => "parameter_count",
        _ => "",
    };
    name.to_owned()
}

fn context_reference_type(reference: &QueryContextReference) -> QueryValueType {
    match reference {
        QueryContextReference::ThisNodeId => QueryValueType::Uuid,
        QueryContextReference::ThisNodeDepth | QueryContextReference::ThisHeadingLevel => {
            QueryValueType::Number
        }
        QueryContextReference::ThisHeadingParent => QueryValueType::Record,
        QueryContextReference::ThisHeadingPath => QueryValueType::List,
        QueryContextReference::ContextToday => QueryValueType::Date,
        QueryContextReference::ContextNow => QueryValueType::Instant,
        QueryContextReference::ThisNodeName
        | QueryContextReference::ThisNodePath
        | QueryContextReference::ThisNodeDisplayTitle
        | QueryContextReference::ThisDocumentTitle
        | QueryContextReference::ThisDocumentSubtitle
        | QueryContextReference::ThisDocumentDisplayTitle
        | QueryContextReference::ThisDocumentProperty(_)
        | QueryContextReference::ThisHeadingTitle
        | QueryContextReference::ThisHeadingAnchor
        | QueryContextReference::ThisQueryTitle
        | QueryContextReference::ContextTimezone
        | QueryContextReference::ContextLocale => QueryValueType::String,
    }
}

#[cfg(test)]
mod performance_tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::str::FromStr;
    use std::time::Instant;

    use tempfile::tempdir;

    use super::{
        QueryAccessScope, QueryEvaluationContext, QueryExecutionBinding, QueryNodeRecord,
        QueryWorkspaceIndex, WorkingQueryRow, validate_source_rows,
    };
    use crate::{
        CalendarDate, DocumentRevision, NodeId, QueryCellValue, QueryDocumentContext,
        QueryExecutionError, QueryField, QueryHeadingContext, TaskNodeTemporal,
        analyze_query_source,
    };

    #[test]
    fn representative_large_node_tables_execute_deterministically() {
        for node_count in [10_001_usize, 50_000] {
            let mut index = empty_index();
            let root_id = synthetic_node_id(0);
            index.nodes.clear();
            for ordinal in 0..node_count {
                let node_id = synthetic_node_id(ordinal);
                let name = if ordinal == 0 {
                    "Workspace".to_owned()
                } else {
                    format!("Node {ordinal:05}")
                };
                let path = if ordinal == 0 {
                    "/".to_owned()
                } else {
                    format!("/Node-{ordinal:05}")
                };
                index.nodes.insert(
                    node_id,
                    QueryNodeRecord {
                        node_id,
                        parent_id: (ordinal != 0).then_some(root_id),
                        name,
                        path,
                        depth: u16::from(ordinal != 0),
                        revision: DocumentRevision::from_source(&format!("revision-{ordinal}")),
                        document: QueryDocumentContext {
                            title: Some(format!("Node {ordinal:05}")),
                            subtitle: None,
                            properties: BTreeMap::default(),
                            display_title: Some(format!("Node {ordinal:05}")),
                        },
                        headings: Vec::new(),
                    },
                );
            }

            let source = concat!(
                "[.weftext-query,version=1,view=table]\n",
                "....\n",
                "from nodes as node\n",
                "scope workspace\n",
                "where node.depth >= 0\n",
                "select node.name, node.path\n",
                "order by node.path desc\n",
                "limit 100\n",
                "....\n",
            );
            let analysis = analyze_query_source(source);
            assert!(
                analysis.diagnostics.is_empty(),
                "{:?}",
                analysis.diagnostics
            );
            let plan = analysis.blocks[0].plan.as_ref().expect("typed node query");
            let access = QueryAccessScope::complete(index.node_ids());
            let context = QueryEvaluationContext::new(
                CalendarDate::new(2026, 8, 24).expect("valid date"),
                TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("valid instant"),
                "Asia/Shanghai".to_owned(),
                "zh-CN".to_owned(),
                QueryExecutionBinding {
                    node_id: Some(root_id),
                    heading: None,
                },
            )
            .expect("valid context");

            let started = Instant::now();
            let first = index.execute(plan, &access, &context);
            let elapsed = started.elapsed();
            if node_count == 50_000 {
                assert_eq!(first, Err(QueryExecutionError::ResourceLimit));
                eprintln!(
                    "query node-table resource ceiling: nodes={node_count} elapsed_ms={}",
                    elapsed.as_millis()
                );
                continue;
            }
            let first = first.expect("bounded large node table");
            let second = index
                .execute(plan, &access, &context)
                .expect("repeat large node table");
            assert_eq!(first, second);
            assert_eq!(first.total_before_limit, node_count);
            assert_eq!(first.rows.len(), 100);
            assert!(first.truncated);
            assert_eq!(
                first.rows[0]
                    .cells
                    .iter()
                    .find(|cell| cell.column.field == QueryField::Path)
                    .map(|cell| &cell.value),
                Some(&QueryCellValue::Text(format!(
                    "/Node-{:05}",
                    node_count - 1
                )))
            );
            eprintln!(
                "query node-table baseline: nodes={node_count} elapsed_ms={}",
                elapsed.as_millis()
            );
        }
    }

    #[test]
    fn repeated_large_values_hit_the_canonical_result_materialization_ceiling() {
        let mut index = empty_index();
        let root_id = synthetic_node_id(0);
        index.nodes.clear();
        for ordinal in 0..1_025_usize {
            let node_id = synthetic_node_id(ordinal);
            let mut properties = BTreeMap::new();
            properties.insert("payload".to_owned(), "x".repeat(4_096));
            index.nodes.insert(
                node_id,
                QueryNodeRecord {
                    node_id,
                    parent_id: (ordinal != 0).then_some(root_id),
                    name: format!("Node {ordinal:05}"),
                    path: format!("/Node-{ordinal:05}"),
                    depth: u16::from(ordinal != 0),
                    revision: DocumentRevision::from_source(&format!("revision-{ordinal}")),
                    document: QueryDocumentContext {
                        title: None,
                        subtitle: None,
                        properties,
                        display_title: Some(format!("Node {ordinal:05}")),
                    },
                    headings: Vec::new(),
                },
            );
        }

        let analysis = analyze_query_source(concat!(
            "[.weftext-query,version=1,view=table]\n",
            "....\n",
            "from nodes as node\n",
            "scope workspace\n",
            "where true\n",
            "select node.document.properties[\"payload\"] as payload\n",
            "order by node.document.properties[\"payload\"] asc\n",
            "limit 1000\n",
            "....\n",
        ));
        assert!(
            analysis.diagnostics.is_empty(),
            "{:?}",
            analysis.diagnostics
        );
        let plan = analysis.blocks[0].plan.as_ref().expect("typed node query");
        let access = QueryAccessScope::complete(index.node_ids());
        let context = QueryEvaluationContext::new(
            CalendarDate::new(2026, 8, 24).expect("valid date"),
            TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("valid instant"),
            "Asia/Shanghai".to_owned(),
            "zh-CN".to_owned(),
            QueryExecutionBinding {
                node_id: Some(root_id),
                heading: None,
            },
        )
        .expect("valid context");

        assert_eq!(
            index.execute(plan, &access, &context),
            Err(QueryExecutionError::ResourceLimit)
        );
    }

    #[test]
    fn source_derived_list_and_record_values_reject_more_than_sixty_four_members() {
        let mut index = empty_index();
        let root_id = synthetic_node_id(0);
        let node = index.nodes.get_mut(&root_id).expect("root query node");
        node.headings.push(QueryHeadingContext {
            title: "Deep heading".to_owned(),
            level: 1,
            anchor: None,
            parent: None,
            path: (0..=64).map(|index| format!("Heading {index}")).collect(),
            range: 0..12,
            section_range: 0..12,
        });
        let analysis = analyze_query_source(concat!(
            "[.weftext-query,version=1,view=table]\n",
            "....\n",
            "from headings as heading\n",
            "scope workspace\n",
            "where true\n",
            "select heading.path\n",
            "order by heading.title asc\n",
            "limit 10\n",
            "....\n",
        ));
        assert!(
            analysis.diagnostics.is_empty(),
            "{:?}",
            analysis.diagnostics
        );
        let plan = analysis.blocks[0]
            .plan
            .as_ref()
            .expect("typed heading query");
        let access = QueryAccessScope::complete(index.node_ids());
        let context = QueryEvaluationContext::new(
            CalendarDate::new(2026, 8, 24).expect("valid date"),
            TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("valid instant"),
            "Asia/Shanghai".to_owned(),
            "zh-CN".to_owned(),
            QueryExecutionBinding {
                node_id: Some(root_id),
                heading: None,
            },
        )
        .expect("valid context");
        assert_eq!(
            index.execute(plan, &access, &context),
            Err(QueryExecutionError::ResourceLimit),
            "oversized source list must fail without returning a partial result",
        );

        let node = index.nodes.get(&root_id).expect("root query node");
        let normal_heading = QueryHeadingContext {
            title: "Normal heading".to_owned(),
            level: 1,
            anchor: None,
            parent: None,
            path: vec!["Normal heading".to_owned()],
            range: 0..14,
            section_range: 0..14,
        };
        let mut source_row = WorkingQueryRow::from_heading(node, &normal_heading);
        source_row.values.insert(
            QueryField::HeadingParent,
            QueryCellValue::Record(
                (0..=64)
                    .map(|index| (format!("field_{index}"), QueryCellValue::Null))
                    .collect(),
            ),
        );
        assert_eq!(
            validate_source_rows(&[source_row]),
            Err(QueryExecutionError::ResourceLimit),
        );
    }

    #[test]
    fn heading_document_unicode_property_uses_the_owning_document_value() {
        let mut index = empty_index();
        let root_id = synthetic_node_id(0);
        let node = index.nodes.get_mut(&root_id).expect("root query node");
        node.document
            .properties
            .insert("状态".to_owned(), "进行中".to_owned());
        node.headings.push(QueryHeadingContext {
            title: "阶段".to_owned(),
            level: 1,
            anchor: None,
            parent: None,
            path: vec!["阶段".to_owned()],
            range: 0..6,
            section_range: 0..6,
        });
        let analysis = analyze_query_source(concat!(
            "[.weftext-query,version=1,view=table]\n",
            "....\n",
            "from headings as heading\n",
            "scope workspace\n",
            "where true\n",
            "select heading.document.properties[\"状态\"] as document_status\n",
            "order by heading.title asc\n",
            "limit 10\n",
            "....\n",
        ));
        assert!(
            analysis.diagnostics.is_empty(),
            "{:?}",
            analysis.diagnostics
        );
        let plan = analysis.blocks[0]
            .plan
            .as_ref()
            .expect("typed heading query");
        let access = QueryAccessScope::complete(index.node_ids());
        let context = QueryEvaluationContext::new(
            CalendarDate::new(2026, 8, 24).expect("valid date"),
            TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("valid instant"),
            "Asia/Shanghai".to_owned(),
            "zh-CN".to_owned(),
            QueryExecutionBinding {
                node_id: Some(root_id),
                heading: None,
            },
        )
        .expect("valid context");

        let result = index
            .execute(plan, &access, &context)
            .expect("heading document property");
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.columns[0].path, "document.properties[\"状态\"]");
        assert_eq!(
            result.rows[0].cells[0].value,
            QueryCellValue::Text("进行中".to_owned())
        );
    }

    fn empty_index() -> QueryWorkspaceIndex {
        let temporary = tempdir().expect("temporary query workspace");
        let root = temporary.path().join("Workspace");
        fs::create_dir(&root).expect("workspace root");
        fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
        let root_id = synthetic_node_id(0);
        fs::write(
            root.join("Workspace.adoc"),
            format!("---\nweftext:\n  id: \"{root_id}\"\n---\n= Workspace\n"),
        )
        .expect("root document");
        QueryWorkspaceIndex::rebuild(&root).expect("empty query index")
    }

    fn synthetic_node_id(ordinal: usize) -> NodeId {
        NodeId::from_str(&format!("{ordinal:08x}-0000-4000-8000-{ordinal:012x}"))
            .expect("synthetic UUIDv4")
    }
}
