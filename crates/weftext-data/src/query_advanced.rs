//! Query producer nodes over one immutable input. CEL and typed equality remain
//! owned by `query`; this module performs no IO or ambient definition lookup.
use crate::{query::*, *};
use cel::{
    Program,
    common::ast::{Expr, IdedExpr},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod definition_preflight;
pub use definition_preflight::{
    QueryDefinitionInvocation, QueryDefinitionInvocationSource, QueryDefinitionPreflight,
    preflight_query_definition,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryAdvanced {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<QueryParameter>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "declared_union"
    )]
    pub union_all: Option<Vec<QueryPlan>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct: Option<Vec<QueryColumn>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related: Option<QueryRelated>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<QueryCall>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryParameter {
    pub name: String,
    pub shape: Shape,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<NativeValue>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryRelated {
    pub relation: String,
    pub from: Box<QueryPlan>,
    #[serde(default = "forward")]
    pub direction: String,
}
// A present null remains a normal serde sequence-type error; only an absent
// field is None. An explicit empty vector survives to typed product preflight.
fn declared_union<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Vec<QueryPlan>>, D::Error> {
    Vec::<QueryPlan>::deserialize(deserializer).map(Some)
}
impl QueryAdvanced {
    pub fn union_branches(&self) -> &[QueryPlan] {
        self.union_all.as_deref().unwrap_or_default()
    }
    pub fn union_branches_mut(&mut self) -> &mut [QueryPlan] {
        self.union_all.as_deref_mut().unwrap_or_default()
    }
    pub(crate) fn validate_union(&self) -> Result<()> {
        if self.union_all.as_ref().is_some_and(Vec::is_empty) {
            return Err(DataError::new(
                UiMessage::error_data_query_advanced_union_requires_producer(),
            ));
        }
        Ok(())
    }
}
fn forward() -> String {
    "forward".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryCall {
    pub owner: String,
    pub selection: DefinitionSelection,
    #[serde(default)]
    pub arguments: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenAdvancedInput {
    /// Trusted Core-qualified, same-cut structure. JSON cannot supply authority.
    #[serde(skip)]
    pub tag_structure: Option<FrozenTagStructure>,
    #[serde(default)]
    pub parameters: BTreeMap<String, NativeValue>,
    #[serde(default)]
    pub definitions: Vec<FrozenDefinitionOwner>,
    #[serde(default)]
    pub schemas: Vec<FrozenDefinitionSchema>,
    #[serde(default)]
    pub scope_bindings: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenDefinitionOwner {
    pub owner: String,
    pub history: Vec<DefinitionRevision>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenDefinitionSchema {
    pub schema: String,
    pub revision: String,
    pub projection: Schema,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryDefinitionDependency {
    pub owner: String,
    pub selection: DefinitionSelection,
}

const MAX_CALL_DEPTH: usize = 64;
const MAX_PLAN_NODES: usize = 1024;
type Bindings = (BTreeMap<String, Cell>, BTreeMap<String, Shape>);
type TerminalGroupBucket = (Vec<Cell>, Vec<Vec<Option<Cell>>>);

fn selected<'a>(
    call: &QueryCall,
    owners: &'a [FrozenDefinitionOwner],
) -> Result<(&'a DefinitionRevision, SavedQuery)> {
    let owners: Vec<_> = owners.iter().filter(|o| o.owner == call.owner).collect();
    if owners.len() != 1 {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_query_advanced_definition_owner_history_missing_or_ambiguous(),
        ));
    }
    let history = &owners[0].history;
    let revisions: Vec<_> = history
        .iter()
        .filter(|r| match &call.selection {
            DefinitionSelection::Current { .. } => r.current,
            DefinitionSelection::Pinned { revision, .. } => &r.revision == revision,
        })
        .collect();
    if revisions.len() != 1 {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_query_advanced_definition_revision_missing_or_ambiguous(),
        ));
    }
    Ok((revisions[0], select_definition(&call.selection, history)?))
}

/// Host dependency discovery is iterative: an unloaded owner is returned before
/// traversal stops at that call. Once loaded, all nested calls are expanded.
/// Loaded histories are exact; missing pinned revisions never select current.
pub fn definition_dependencies(
    plan: &QueryPlan,
    owners: &[FrozenDefinitionOwner],
) -> Result<Vec<QueryDefinitionDependency>> {
    fn visit(
        plan: &QueryPlan,
        owners: &[FrozenDefinitionOwner],
        stack: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
        out: &mut Vec<QueryDefinitionDependency>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_query_advanced_definition_call_expansion_budget_exhausted(),
            ));
        }
        let Some(a) = &plan.advanced else {
            return Ok(());
        };
        a.validate_union()?;
        if usize::from(a.union_all.is_some())
            + usize::from(a.related.is_some())
            + usize::from(a.call.is_some())
            > 1
        {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_query_requires_one_producer_node(),
            ));
        }
        for child in a.union_branches() {
            visit(child, owners, stack, seen, out, nodes, depth + 1)?;
        }
        if let Some(r) = &a.related {
            visit(&r.from, owners, stack, seen, out, nodes, depth + 1)?;
        }
        if let Some(call) = &a.call {
            let request_key = format!("{}:{}", call.owner, serde_json::to_string(&call.selection)?);
            if seen.insert(request_key) {
                out.push(QueryDefinitionDependency {
                    owner: call.owner.clone(),
                    selection: call.selection.clone(),
                });
            }
            if owners
                .iter()
                .filter(|o| o.owner == call.owner)
                .flat_map(|o| &o.history)
                .any(|r| match &call.selection {
                    DefinitionSelection::Current { .. } => r.current,
                    DefinitionSelection::Pinned { revision, .. } => &r.revision == revision,
                })
            {
                let (revision, definition) = selected(call, owners)?;
                let identity = format!("{}:{}:{}", call.owner, revision.revision, definition.key);
                if stack.contains(&identity) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_query_advanced_definition_call_cycle(),
                    ));
                }
                stack.push(identity);
                visit(&definition.plan, owners, stack, seen, out, nodes, depth + 1)?;
                stack.pop();
            }
        }
        Ok(())
    }
    let mut out = vec![];
    visit(
        plan,
        owners,
        &mut vec![],
        &mut BTreeSet::new(),
        &mut out,
        &mut 0,
        0,
    )?;
    Ok(out)
}

fn bind_parameters(
    plan: &QueryPlan,
    schema: &Schema,
    values: &BTreeMap<String, NativeValue>,
) -> Result<Bindings> {
    parameter_bindings(plan, schema, values, true)
}
fn parameter_bindings(
    plan: &QueryPlan,
    schema: &Schema,
    values: &BTreeMap<String, NativeValue>,
    required: bool,
) -> Result<Bindings> {
    let declared = plan
        .advanced
        .as_ref()
        .map_or(&[][..], |a| a.parameters.as_slice());
    let mut cells = BTreeMap::new();
    let mut shapes = BTreeMap::new();
    let reserved = symbols(schema)?;
    for p in declared {
        if p.name.is_empty()
            || !p
                .name
                .chars()
                .enumerate()
                .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || i > 0 && c.is_ascii_digit())
            || reserved.contains_key(&p.name)
            || shapes.contains_key(&p.name)
        {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_parameter_name_collides_or_invalid_identifier(),
            ));
        }
        // Reuse the installed native Shape validator, without pretending this
        // minimum Query projection is a complete schema to reinstall. Unknown
        // `use` must still fail before any adapter resolves its shape.
        schema.check_query_shape(&p.shape)?;
        if let Some(default) = &p.default {
            validate_parameter(schema, p, default)?;
        }
        let value = values.get(&p.name).or(p.default.as_ref());
        if required && value.is_none() {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_required_parameter_missing(
                    weftext_messages::params::ErrorDataQueryAdvancedRequiredParameterMissing {
                        parameter_name: (p.name).to_string(),
                    },
                ),
            ));
        }
        if let Some(value) = value {
            validate_parameter(schema, p, value)?;
        }
        cells.insert(
            p.name.clone(),
            match value {
                Some(NativeValue::None) => Cell::Null,
                Some(v) => Cell::value(v.clone()),
                None => Cell::Absent,
            },
        );
        shapes.insert(p.name.clone(), p.shape.clone());
    }
    if values.keys().any(|k| !shapes.contains_key(k)) {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_query_advanced_unknown_query_parameter(),
        ));
    }
    Ok((cells, shapes))
}

pub(crate) fn validate_advanced_plan(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
) -> Result<()> {
    validate_plan_static(input, plan, schema, None)
}
/// Save-time compiler validation of the actual frozen callee/baseline closure.
/// Required input parameters are typed without fabricating example values.
pub fn validate_query_plan_with_context(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<()> {
    definition_dependencies(plan, &context.definitions)?;
    preflight_tag_structure(plan, schema, context)?;
    let input = FrozenQueryInput {
        cut_id: "validation-only".into(),
        schema_revision: "validation-only".into(),
        coverage: "complete".into(),
        freshness: "stale".into(),
        notes: vec![],
        advanced: None,
    };
    validate_plan_static(&input, plan, schema, Some(context))
}
fn validate_plan_static(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    context: Option<&FrozenAdvancedInput>,
) -> Result<()> {
    fn visit(
        input: &FrozenQueryInput,
        plan: &QueryPlan,
        schema: &Schema,
        context: Option<&FrozenAdvancedInput>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<QueryResult> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_query_plan_expansion_budget_exhausted()));
        }
        let (parameters, shapes) = parameter_bindings(plan, schema, &BTreeMap::new(), false)?;
        let mut remaining = QueryBudget::default().max_steps;
        let a = plan.advanced.clone().unwrap_or_default();
        a.validate_union()?;
        if usize::from(a.union_all.is_some())
            + usize::from(a.related.is_some())
            + usize::from(a.call.is_some())
            > 1
        {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_query_requires_one_producer_node(),
            ));
        }
        if let Some(call) = &a.call {
            require_terminal_scope(plan)?;
            let vars = shapes
                .iter()
                .map(|(n, s)| (n.clone(), shape_type(schema, s)))
                .collect();
            for e in call.arguments.values() {
                compile(e, schema, &vars)?;
            }
            if let Some(context) = context {
                let (_, definition) = selected(call, &context.definitions)?;
                let baseline = exact_definition_schema(context, &definition, schema)?;
                let target = definition
                    .plan
                    .advanced
                    .as_ref()
                    .map_or(&[][..], |a| a.parameters.as_slice());
                for (name, e) in &call.arguments {
                    let p = target
                        .iter()
                        .find(|p| &p.name == name)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_query_advanced_unknown_callee_parameter()))?;
                    let compiled = compile(e, schema, &vars)?;
                    if !(p.nullable && compiled.ty == scalar("none"))
                        && !compatible(&compiled.ty, &shape_type(baseline, &p.shape))
                    {
                        return Err(DataError::new(
                            weftext_messages::UiMessage::error_data_query_advanced_saved_call_argument_shape_mismatch(),
                        ));
                    }
                }
                if target
                    .iter()
                    .any(|p| !call.arguments.contains_key(&p.name) && p.default.is_none())
                {
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_required_callee_parameter_missing()));
                }
                let result = visit(
                    input,
                    &definition.plan,
                    baseline,
                    Some(context),
                    nodes,
                    depth + 1,
                )?;
                return transform(
                    result,
                    plan,
                    schema,
                    &parameters,
                    &shapes,
                    &mut remaining,
                    &QueryBudget::default(),
                    None,
                    None,
                );
            }
            if !plan.select.is_empty()
                || plan.r#where.is_some()
                || !plan.order.is_empty()
                || plan.unnest.is_some()
                || plan.group.is_some()
                || a.distinct.is_some()
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_query_advanced_saved_call_terminal_edit_needs_definition_validation(),
                ));
            }
            let leaf = QueryPlan {
                select: vec![QueryColumn {
                    name: "__validation".into(),
                    expr: "true".into(),
                }],
                ..Default::default()
            };
            return execute_query_leaf(
                input,
                &leaf,
                schema,
                &QueryBudget::default(),
                &parameters,
                &shapes,
                &mut remaining,
                &BTreeMap::new(),
            );
        }
        if a.union_all.is_some() {
            require_terminal_scope(plan)?;
            let mut first = None;
            for branch in a.union_branches() {
                let result = visit(input, branch, schema, context, nodes, depth + 1)?;
                if first.as_ref().is_some_and(|first: &QueryResult| {
                    first.terminal_schema != result.terminal_schema
                }) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_query_advanced_union_all_terminal_types_or_columns_mismatch(),
                    ));
                }
                first = Some(result);
            }
            return transform(
                first.unwrap(),
                plan,
                schema,
                &parameters,
                &shapes,
                &mut remaining,
                &QueryBudget::default(),
                None,
                None,
            );
        }
        if let Some(related) = &a.related {
            if !schema.relations.iter().any(|r| r.id == related.relation)
                || !matches!(related.direction.as_str(), "forward" | "inverse")
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_query_advanced_unknown_relation_or_direction(),
                ));
            }
            visit(input, &related.from, schema, context, nodes, depth + 1)?;
        }
        let mut leaf = plan.clone();
        leaf.advanced = None;
        if leaf.group.is_some() || a.distinct.is_some() {
            leaf.order.clear();
        }
        let mut result = execute_query_leaf(
            input,
            &leaf,
            schema,
            &QueryBudget::default(),
            &parameters,
            &shapes,
            &mut remaining,
            &BTreeMap::new(),
        )?;
        if plan.group.is_some() {
            sort_terminal(
                &mut result,
                &plan.order,
                schema,
                &parameters,
                &shapes,
                &mut remaining,
                None,
                None,
            )?;
        }
        distinct_terminal(
            result,
            a.distinct.as_deref(),
            plan,
            schema,
            &parameters,
            &shapes,
            &mut remaining,
            None,
            None,
        )
    }
    visit(input, plan, schema, context, &mut 0, 0).map(|_| ())
}

/// Source scopes from all loaded same-cut callees, excluding terminal scopes.
pub fn required_query_lifecycles(
    plan: &QueryPlan,
    owners: &[FrozenDefinitionOwner],
) -> Result<Vec<String>> {
    fn visit(
        plan: &QueryPlan,
        owners: &[FrozenDefinitionOwner],
        out: &mut BTreeSet<String>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_query_scope_expansion_budget_exhausted()));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        a.validate_union()?;
        if let Some(call) = &a.call {
            let (_, definition) = selected(call, owners)?;
            visit(&definition.plan, owners, out, nodes, depth + 1)?;
        } else if a.union_all.is_some() {
            for p in a.union_branches() {
                visit(p, owners, out, nodes, depth + 1)?;
            }
        } else {
            let lifecycle = if plan.scope.lifecycle.is_empty() {
                "live"
            } else {
                &plan.scope.lifecycle
            };
            if !matches!(lifecycle, "live" | "trash") {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_query_advanced_unsupported_lifecycle_scope(),
                ));
            }
            out.insert(lifecycle.to_string());
            if let Some(related) = &a.related {
                visit(&related.from, owners, out, nodes, depth + 1)?;
            }
        }
        Ok(())
    }
    definition_dependencies(plan, owners)?;
    let mut out = BTreeSet::new();
    visit(plan, owners, &mut out, &mut 0, 0)?;
    Ok(out.into_iter().collect())
}
/// Typed or literal directory scopes read by leaf producers/callees.
pub fn required_query_scopes(
    plan: &QueryPlan,
    owners: &[FrozenDefinitionOwner],
) -> Result<Vec<String>> {
    fn visit(
        plan: &QueryPlan,
        owners: &[FrozenDefinitionOwner],
        out: &mut BTreeSet<String>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_query_scope_expansion_budget_exhausted()));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        a.validate_union()?;
        if let Some(call) = &a.call {
            let (_, definition) = selected(call, owners)?;
            visit(&definition.plan, owners, out, nodes, depth + 1)?;
        } else if a.union_all.is_some() {
            for p in a.union_branches() {
                visit(p, owners, out, nodes, depth + 1)?;
            }
        } else {
            if let Some(directory) = &plan.scope.directory {
                out.insert(directory.clone());
            }
            if let Some(r) = &a.related {
                visit(&r.from, owners, out, nodes, depth + 1)?;
            }
        }
        Ok(())
    }
    definition_dependencies(plan, owners)?;
    let mut out = BTreeSet::new();
    visit(plan, owners, &mut out, &mut 0, 0)?;
    Ok(out.into_iter().collect())
}
/// Native products demanded by actual CEL free symbols. Comprehension locals
/// and record member names never trigger body/title/headings compilation.
pub fn query_products(plan: &QueryPlan, owners: &[FrozenDefinitionOwner]) -> Result<Vec<String>> {
    use cel::{Program, common::ast::Expr};
    fn expression(source: &str, out: &mut BTreeSet<String>) -> Result<()> {
        fn walk(
            ast: &cel::common::ast::IdedExpr,
            bound: &BTreeSet<String>,
            out: &mut BTreeSet<String>,
        ) -> Result<()> {
            match &ast.expr {
                Expr::Ident(n) => {
                    if matches!(n.as_str(), "title" | "body" | "headings") && !bound.contains(n) {
                        out.insert(n.clone());
                    }
                }
                Expr::Comprehension(c) => {
                    walk(&c.iter_range, bound, out)?;
                    walk(&c.accu_init, bound, out)?;
                    let mut scope = bound.clone();
                    scope.insert(c.iter_var.clone());
                    scope.insert(c.accu_var.clone());
                    if let Some(n) = &c.iter_var2 {
                        scope.insert(n.clone());
                    }
                    walk(&c.loop_cond, &scope, out)?;
                    walk(&c.loop_step, &scope, out)?;
                    walk(&c.result, &scope, out)?;
                }
                _ => {
                    let mut clone = ast.clone();
                    visit_children_mut(&mut clone, |e| walk(e, bound, out))?;
                }
            }
            Ok(())
        }
        if source.len() > 16384 {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_query_advanced_cel_source_budget_exceeded(),
            ));
        }
        let program = Program::compile(source).map_err(|error| {
            DataError::new(UiMessage::error_data_query_cel_syntax_invalid())
                .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
        })?;
        walk(program.expression(), &BTreeSet::new(), out)
    }
    fn visit(
        plan: &QueryPlan,
        owners: &[FrozenDefinitionOwner],
        out: &mut BTreeSet<String>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_query_product_expansion_budget_exhausted()));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        a.validate_union()?;
        if plan.select.is_empty() && a.union_all.is_none() && a.call.is_none() {
            out.insert("title".into());
        }
        for e in plan
            .r#where
            .iter()
            .chain(plan.select.iter().map(|c| &c.expr))
            .chain(plan.order.iter().map(|o| &o.expr))
            .chain(plan.unnest.iter().map(|u| &u.array))
        {
            expression(e, out)?;
        }
        if let Some(g) = &plan.group {
            for k in &g.keys {
                expression(&k.expr, out)?;
            }
            for a in &g.aggregates {
                if let Some(e) = &a.expr {
                    expression(e, out)?;
                }
            }
        }
        if let Some(keys) = &a.distinct {
            for k in keys {
                expression(&k.expr, out)?;
            }
        }
        for p in a.union_branches() {
            visit(p, owners, out, nodes, depth + 1)?;
        }
        if let Some(r) = &a.related {
            visit(&r.from, owners, out, nodes, depth + 1)?;
        }
        if let Some(call) = &a.call {
            for e in call.arguments.values() {
                expression(e, out)?;
            }
            let (_, definition) = selected(call, owners)?;
            visit(&definition.plan, owners, out, nodes, depth + 1)?;
        }
        Ok(())
    }
    definition_dependencies(plan, owners)?;
    let mut out = BTreeSet::new();
    visit(plan, owners, &mut out, &mut 0, 0)?;
    Ok(out.into_iter().collect())
}

#[derive(Debug, Clone, Default)]
pub struct QueryReadRequirements {
    pub tag_structure: bool,
    pub fields: BTreeSet<String>,
    pub products: BTreeSet<String>,
    pub relations: BTreeSet<String>,
    pub types: BTreeSet<String>,
    pub type_filter: bool,
}

pub fn query_output_columns(
    plan: &QueryPlan,
    owners: &[FrozenDefinitionOwner],
) -> Result<Vec<String>> {
    fn visit(
        plan: &QueryPlan,
        owners: &[FrozenDefinitionOwner],
        depth: usize,
    ) -> Result<Vec<String>> {
        if depth > MAX_CALL_DEPTH {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_query_column_expansion_budget_exceeded()));
        }
        if let Some(group) = &plan.group {
            return Ok(group
                .keys
                .iter()
                .map(|k| k.name.clone())
                .chain(group.aggregates.iter().map(|a| a.name.clone()))
                .collect());
        }
        if !plan.select.is_empty() {
            return Ok(plan.select.iter().map(|c| c.name.clone()).collect());
        }
        if let Some(a) = &plan.advanced {
            a.validate_union()?;
            if let Some(call) = &a.call {
                let (_, definition) = selected(call, owners)?;
                return visit(&definition.plan, owners, depth + 1);
            }
            if let Some(branch) = a.union_branches().first() {
                return visit(branch, owners, depth + 1);
            }
        }
        Ok(vec!["title".into(), "types".into()])
    }
    definition_dependencies(plan, owners)?;
    visit(plan, owners, 0)
}

/// Static read effects use the same parsed CEL AST and each callee's exact
/// baseline. Local variables, parameters and terminal column names do not
/// acquire a field/product authority by resembling an author key.
pub fn query_read_requirements(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<QueryReadRequirements> {
    fn expression(
        source: &str,
        schema: &Schema,
        bound: &BTreeSet<String>,
        out: &mut QueryReadRequirements,
    ) -> Result<()> {
        fn walk(
            ast: &IdedExpr,
            schema: &Schema,
            bound: &BTreeSet<String>,
            out: &mut QueryReadRequirements,
        ) -> Result<()> {
            match &ast.expr {
                Expr::Ident(n) if !bound.contains(n) => {
                    if matches!(n.as_str(), "title" | "body" | "headings" | "self" | "types") {
                        out.products.insert(n.clone());
                    } else if let Some(f) = schema.fields.iter().find(|f| &f.key == n) {
                        out.fields.insert(f.id.clone());
                    }
                }
                Expr::Call(c) if c.func_name == "field" => {
                    let id = c
                        .args
                        .first()
                        .and_then(literal_text)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_query_advanced_literal_field_id_required()))?;
                    if c.args.len() != 1
                        || c.target.is_some()
                        || !schema.fields.iter().any(|f| f.id == id)
                    {
                        return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_unknown_canonical_field_id()));
                    }
                    out.fields.insert(id);
                }
                Expr::Call(c) if c.func_name == "tagInSubtree" => {
                    out.tag_structure = true;
                    let mut child = ast.clone();
                    visit_children_mut(&mut child, |e| walk(e, schema, bound, out))?;
                }
                Expr::Comprehension(c) => {
                    walk(&c.iter_range, schema, bound, out)?;
                    walk(&c.accu_init, schema, bound, out)?;
                    let mut locals = bound.clone();
                    locals.insert(c.iter_var.clone());
                    locals.insert(c.accu_var.clone());
                    if let Some(n) = &c.iter_var2 {
                        locals.insert(n.clone());
                    }
                    for e in [&c.loop_cond, &c.loop_step, &c.result] {
                        walk(e, schema, &locals, out)?;
                    }
                }
                _ => {
                    let mut child = ast.clone();
                    visit_children_mut(&mut child, |e| walk(e, schema, bound, out))?;
                }
            }
            Ok(())
        }
        if source.len() > 16384 {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_query_advanced_cel_source_budget_exceeded(),
            ));
        }
        let program = Program::compile(source).map_err(|error| {
            DataError::new(UiMessage::error_data_query_cel_syntax_invalid())
                .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
        })?;
        walk(program.expression(), schema, bound, out)
    }
    fn visit(
        plan: &QueryPlan,
        schema: &Schema,
        context: &FrozenAdvancedInput,
        out: &mut QueryReadRequirements,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_query_advanced_query_read_closure_budget_exceeded(),
            ));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        a.validate_union()?;
        let mut bound = a
            .parameters
            .iter()
            .map(|p| p.name.clone())
            .collect::<BTreeSet<_>>();
        let terminal = a.union_all.is_some() || a.call.is_some();
        if terminal {
            if let Some(branch) = a.union_branches().first() {
                bound.extend(query_output_columns(branch, &context.definitions)?);
            }
            if let Some(call) = &a.call {
                let (_, definition) = selected(call, &context.definitions)?;
                bound.extend(query_output_columns(
                    &definition.plan,
                    &context.definitions,
                )?);
            }
        }
        if let Some(u) = &plan.unnest {
            expression(&u.array, schema, &bound, out)?;
            bound.insert(u.binding.clone());
        }
        for e in plan
            .r#where
            .iter()
            .chain(plan.select.iter().map(|c| &c.expr))
        {
            expression(e, schema, &bound, out)?;
        }
        if let Some(g) = &plan.group {
            for e in g
                .keys
                .iter()
                .map(|c| &c.expr)
                .chain(g.aggregates.iter().filter_map(|a| a.expr.as_ref()))
            {
                expression(e, schema, &bound, out)?;
            }
        }
        let mut order_bound = bound.clone();
        if let Some(g) = &plan.group {
            order_bound.extend(g.keys.iter().map(|c| c.name.clone()));
            order_bound.extend(g.aggregates.iter().map(|a| a.name.clone()));
        }
        if a.distinct.is_some() {
            order_bound.extend(query_output_columns(plan, &context.definitions)?);
        }
        for order in &plan.order {
            expression(&order.expr, schema, &order_bound, out)?;
        }
        if let Some(keys) = &a.distinct {
            let mut terminal_bound = bound.clone();
            terminal_bound.extend(query_output_columns(plan, &context.definitions)?);
            for k in keys {
                expression(&k.expr, schema, &terminal_bound, out)?;
            }
        }
        if plan.select.is_empty() && !terminal && plan.group.is_none() {
            out.products.extend(["title".into(), "types".into()]);
        }
        for t in &plan.types {
            out.type_filter = true;
            out.types.insert(
                schema
                    .type_id(t)
                    .ok_or_else(|| {
                        DataError::new(
                            weftext_messages::UiMessage::error_data_query_advanced_unknown_query_type(),
                        )
                    })?
                    .into(),
            );
            out.products.insert("types".into());
        }
        for child in a.union_branches() {
            visit(child, schema, context, out, nodes, depth + 1)?;
        }
        if let Some(r) = &a.related {
            let relation = schema
                .relations
                .iter()
                .find(|d| d.id == r.relation)
                .ok_or_else(|| {
                    DataError::new(
                        weftext_messages::UiMessage::error_data_query_advanced_unknown_relation_id(
                        ),
                    )
                })?;
            out.relations.insert(relation.id.clone());
            out.fields.insert(relation.field.clone());
            out.types.insert(relation.source_type.clone());
            if let Some(ids) = relation.target_types["ids"].as_array() {
                out.types
                    .extend(ids.iter().filter_map(|id| id.as_str().map(str::to_owned)));
            }
            visit(&r.from, schema, context, out, nodes, depth + 1)?;
        }
        if let Some(call) = &a.call {
            for e in call.arguments.values() {
                expression(e, schema, &bound, out)?;
            }
            let (_, definition) = selected(call, &context.definitions)?;
            let baseline = context
                .schemas
                .iter()
                .find(|s| {
                    s.schema == definition.binding.schema
                        && s.revision == definition.binding.revision
                })
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_query_advanced_exact_definition_baseline_missing()))?;
            visit(
                &definition.plan,
                &baseline.projection,
                context,
                out,
                nodes,
                depth + 1,
            )?;
        }
        Ok(())
    }
    definition_dependencies(plan, &context.definitions)?;
    let mut result = QueryReadRequirements::default();
    visit(plan, schema, context, &mut result, &mut 0, 0)?;
    Ok(result)
}
fn validate_parameter(
    schema: &Schema,
    parameter: &QueryParameter,
    value: &NativeValue,
) -> Result<()> {
    if matches!(value, NativeValue::None) && parameter.nullable {
        return Ok(());
    }
    if !schema.validate(&parameter.shape, value).is_valid() {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_query_advanced_parameter_shape_mismatch(
                weftext_messages::params::ErrorDataQueryAdvancedParameterShapeMismatch {
                    parameter_name: (parameter.name).to_string(),
                },
            ),
        ));
    }
    Ok(())
}

/// Validate every structural root in the exact static call closure before rows
/// are scanned. Empty scans and short-circuit CEL cannot erase this read.
pub fn prepare_tag_structure(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<()> {
    check_tag_structure(plan, schema, context, true)
}

/// Definition-time structural proof uses actual supplied/default parameters.
/// An unbound required typed parameter stays deferred; it never supplies a
/// fabricated TagRef or suppresses the complete Tags read dependency.
pub fn preflight_tag_structure(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
) -> Result<()> {
    check_tag_structure(plan, schema, context, false)
}

fn check_tag_structure(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
    require_parameters: bool,
) -> Result<()> {
    fn needs(
        plan: &QueryPlan,
        owners: &[FrozenDefinitionOwner],
        depth: usize,
        nodes: &mut usize,
    ) -> Result<bool> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_tag_structure_closure_budget_exhausted()));
        }
        fn ast_needs(ast: &IdedExpr) -> Result<bool> {
            if matches!(&ast.expr, Expr::Call(c) if c.func_name == "tagInSubtree") {
                return Ok(true);
            }
            let mut child = ast.clone();
            let mut found = false;
            visit_children_mut(&mut child, |e| {
                found |= ast_needs(e)?;
                Ok(())
            })?;
            Ok(found)
        }
        let a = plan.advanced.clone().unwrap_or_default();
        a.validate_union()?;
        let mut found = false;
        for source in plan
            .r#where
            .iter()
            .map(String::as_str)
            .chain(plan.select.iter().map(|c| c.expr.as_str()))
            .chain(plan.order.iter().map(|c| c.expr.as_str()))
            .chain(plan.unnest.iter().map(|u| u.array.as_str()))
            .chain(plan.group.iter().flat_map(|g| {
                g.keys
                    .iter()
                    .map(|c| c.expr.as_str())
                    .chain(g.aggregates.iter().filter_map(|a| a.expr.as_deref()))
            }))
            .chain(a.distinct.iter().flatten().map(|c| c.expr.as_str()))
            .chain(
                a.call
                    .iter()
                    .flat_map(|c| c.arguments.values().map(String::as_str)),
            )
        {
            if source.len() > 16384 {
                return Err(DataError::budget(
                    weftext_messages::UiMessage::error_data_query_advanced_cel_source_budget_exceeded(),
                ));
            }
            found |= ast_needs(
                Program::compile(source)
                    .map_err(|error| {
                        DataError::new(UiMessage::error_data_query_cel_syntax_invalid())
                            .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
                    })?
                    .expression(),
            )?;
        }
        for child in a.union_branches() {
            found |= needs(child, owners, depth + 1, nodes)?;
        }
        if let Some(r) = &a.related {
            found |= needs(&r.from, owners, depth + 1, nodes)?;
        }
        if let Some(call) = &a.call {
            found |= needs(&selected(call, owners)?.1.plan, owners, depth + 1, nodes)?;
        }
        Ok(found)
    }
    if !needs(plan, &context.definitions, 0, &mut 0)? {
        return Ok(());
    }
    let tags = context
        .tag_structure
        .as_ref()
        .ok_or_else(|| DataError::unavailable(weftext_messages::UiMessage::error_data_query_advanced_frozen_qualified_tag_structure_unavailable()))?;
    fn roots(
        ast: &IdedExpr,
        parameters: &BTreeMap<String, Cell>,
        tags: &FrozenTagStructure,
        locals: &BTreeSet<String>,
        deferred: bool,
    ) -> Result<()> {
        if let Expr::Call(call) = &ast.expr
            && call.func_name == "tagInSubtree"
        {
            if call.target.is_some() || call.args.len() != 2 {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_query_advanced_tag_in_subtree_arity_mismatch(),
                ));
            }
            let reference = match &call.args[1].expr {
                Expr::Call(root)
                    if root.func_name == "tagRef"
                        && root.target.is_none()
                        && root.args.len() == 1 =>
                {
                    literal_text(&root.args[0])
                }
                Expr::Ident(name) if !locals.contains(name) => match parameters.get(name) {
                    Some(Cell::Absent) if deferred => None,
                    Some(Cell::Value {
                        value: NativeValue::Text(s),
                    }) => Some(s.clone()),
                    _ => {
                        return Err(DataError::new(
                            weftext_messages::UiMessage::error_data_query_advanced_tag_root_requires_frozen_typed_parameter(),
                        ));
                    }
                },
                _ => {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_query_advanced_tag_root_requires_frozen_parameter_or_literal(),
                    ));
                }
            };
            if let Some(reference) = reference {
                tags.validate_root(&reference)?;
            }
        }
        if let Expr::Comprehension(c) = &ast.expr {
            roots(&c.iter_range, parameters, tags, locals, deferred)?;
            roots(&c.accu_init, parameters, tags, locals, deferred)?;
            let mut bound = locals.clone();
            bound.insert(c.iter_var.clone());
            bound.insert(c.accu_var.clone());
            if let Some(name) = &c.iter_var2 {
                bound.insert(name.clone());
            }
            for child in [&c.loop_cond, &c.loop_step, &c.result] {
                roots(child, parameters, tags, &bound, deferred)?;
            }
            return Ok(());
        }
        let mut child = ast.clone();
        visit_children_mut(&mut child, |e| roots(e, parameters, tags, locals, deferred))
    }
    #[allow(clippy::too_many_arguments)]
    fn visit(
        plan: &QueryPlan,
        schema: &Schema,
        context: &FrozenAdvancedInput,
        values: &BTreeMap<String, NativeValue>,
        unknown: &BTreeSet<String>,
        tags: &FrozenTagStructure,
        require_parameters: bool,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_CALL_DEPTH || *nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(weftext_messages::UiMessage::error_data_query_advanced_tag_structure_closure_budget_exhausted()));
        }
        let (mut parameters, shapes) =
            parameter_bindings(plan, schema, values, require_parameters)?;
        for name in unknown {
            if let Some(value) = parameters.get_mut(name) {
                *value = Cell::Absent;
            }
        }
        let advanced = plan.advanced.clone().unwrap_or_default();
        advanced.validate_union()?;
        for expression in plan
            .r#where
            .iter()
            .map(String::as_str)
            .chain(plan.select.iter().map(|c| c.expr.as_str()))
            .chain(plan.order.iter().map(|c| c.expr.as_str()))
            .chain(plan.unnest.iter().map(|u| u.array.as_str()))
            .chain(plan.group.iter().flat_map(|g| {
                g.keys
                    .iter()
                    .map(|c| c.expr.as_str())
                    .chain(g.aggregates.iter().filter_map(|a| a.expr.as_deref()))
            }))
            .chain(advanced.distinct.iter().flatten().map(|c| c.expr.as_str()))
            .chain(
                advanced
                    .call
                    .iter()
                    .flat_map(|c| c.arguments.values().map(String::as_str)),
            )
        {
            if expression.len() > 16384 {
                return Err(DataError::budget(
                    weftext_messages::UiMessage::error_data_query_advanced_cel_source_budget_exceeded(),
                ));
            }
            let program = Program::compile(expression).map_err(|error| {
                DataError::new(UiMessage::error_data_query_cel_syntax_invalid())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })?;
            roots(
                program.expression(),
                &parameters,
                tags,
                &BTreeSet::new(),
                !require_parameters,
            )?;
        }
        let inherited_unknown = parameters
            .iter()
            .filter(|(_, cell)| matches!(cell, Cell::Absent))
            .map(|(name, _)| name.clone())
            .collect::<BTreeSet<_>>();
        for child in advanced.union_branches() {
            visit(
                child,
                schema,
                context,
                &parameter_subset(child, &parameters),
                &inherited_unknown,
                tags,
                require_parameters,
                depth + 1,
                nodes,
            )?;
        }
        if let Some(related) = &advanced.related {
            visit(
                &related.from,
                schema,
                context,
                &parameter_subset(&related.from, &parameters),
                &inherited_unknown,
                tags,
                require_parameters,
                depth + 1,
                nodes,
            )?;
        }
        if let Some(call) = &advanced.call {
            let (_, definition) = selected(call, &context.definitions)?;
            let baseline = exact_definition_schema(context, &definition, schema)?;
            let vars = shapes
                .iter()
                .map(|(n, s)| (n.clone(), shape_type(schema, s)))
                .collect();
            let mut env = query_env_traced(schema, None)?;
            env.parameter_shapes = shapes;
            env.tag_structure = Some(tags.clone());
            let mut budget = QueryBudget::default().max_steps;
            let mut arguments = BTreeMap::new();
            let mut undecided = BTreeSet::new();
            for (name, expression) in &call.arguments {
                let compiled = compile(expression, schema, &vars)?;
                if !require_parameters && expression_uses_unknown_parameters(&compiled, &parameters)
                {
                    undecided.insert(name.clone());
                    continue;
                }
                let cell = evaluate(&compiled, &parameters, schema, &mut budget, &env)?;
                let value = match cell {
                    Cell::Value { value } => value,
                    Cell::Null => NativeValue::None,
                    _ => return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_saved_call_argument_undecided())),
                };
                arguments.insert(name.clone(), value);
            }
            visit(
                &definition.plan,
                baseline,
                context,
                &arguments,
                &undecided,
                tags,
                require_parameters,
                depth + 1,
                nodes,
            )?;
        }
        Ok(())
    }
    visit(
        plan,
        schema,
        context,
        &context.parameters,
        &BTreeSet::new(),
        tags,
        require_parameters,
        0,
        &mut 0,
    )
}

/// Conservative semantic proof: aliases can change display tokens, while
/// authored field keys, shapes, relation contracts and provider data must agree.
pub fn baseline_contract_compatible(old: &Schema, current: &Schema) -> Result<bool> {
    let mut old = serde_json::to_value(old)?;
    let mut actual = serde_json::to_value(current)?;
    old.as_object_mut().unwrap().remove("aliases");
    actual.as_object_mut().unwrap().remove("aliases");
    Ok(old == actual)
}
fn exact_definition_schema<'a>(
    context: &'a FrozenAdvancedInput,
    definition: &SavedQuery,
    current: &Schema,
) -> Result<&'a Schema> {
    let matches = context
        .schemas
        .iter()
        .filter(|s| {
            s.schema == definition.binding.schema && s.revision == definition.binding.revision
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_query_advanced_definition_schema_baseline_missing_or_ambiguous(),
        ));
    }
    let baseline = &matches[0].projection;
    if !baseline_contract_compatible(baseline, current)? {
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_query_advanced_definition_schema_requires_explicit_migration(),
        ));
    }
    Ok(baseline)
}
pub(crate) fn execute_advanced_query(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> Result<QueryResult> {
    execute_advanced_query_traced(input, plan, schema, limits, metadata, None)
}
pub(crate) fn execute_advanced_query_traced(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
    trace: Option<&crate::query_providers::ProviderTrace>,
) -> Result<QueryResult> {
    let empty = FrozenAdvancedInput::default();
    let context = input.advanced.as_ref().unwrap_or(&empty);
    let definition_bytes = context
        .definitions
        .iter()
        .flat_map(|o| &o.history)
        .try_fold(0usize, |bytes, r| bytes.checked_add(r.source.len()))
        .ok_or_else(|| {
            DataError::budget(
                weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted(),
            )
        })?
        .saturating_add(
            context
                .tag_structure
                .as_ref()
                .map_or(0, FrozenTagStructure::retained_bytes),
        );
    if definition_bytes > limits.max_source_bytes {
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted(),
        ));
    }
    definition_dependencies(plan, &context.definitions)?;
    prepare_tag_structure(plan, schema, context)?;
    let mut runner = Runner {
        input,
        context,
        limits,
        remaining: limits.max_steps,
        nodes: 0,
        source_bytes: definition_bytes,
        metadata,
        trace: trace.cloned(),
    };
    runner.run(plan, schema, &context.parameters, 0, None)
}
struct Runner<'a> {
    input: &'a FrozenQueryInput,
    context: &'a FrozenAdvancedInput,
    limits: &'a QueryBudget,
    remaining: u64,
    nodes: usize,
    source_bytes: usize,
    metadata: &'a BTreeMap<String, AuthorizedQueryMetadata>,
    trace: Option<crate::query_providers::ProviderTrace>,
}
impl Runner<'_> {
    fn run(
        &mut self,
        plan: &QueryPlan,
        schema: &Schema,
        values: &BTreeMap<String, NativeValue>,
        depth: usize,
        baseline_key: Option<ProviderBaseline>,
    ) -> Result<QueryResult> {
        self.nodes += 1;
        if depth > MAX_CALL_DEPTH || self.nodes > MAX_PLAN_NODES {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_query_advanced_definition_call_expansion_budget_exhausted(),
            ));
        }
        let trace = self.trace.as_ref().map(|t| t.at(baseline_key.clone()));
        if let Some(trace) = &trace {
            crate::query_providers::trace_parameters(
                plan,
                schema,
                values,
                trace,
                &mut self.remaining,
            )?;
        }
        let (parameters, shapes) = bind_parameters(plan, schema, values)?;
        let advanced = plan.advanced.clone().unwrap_or_default();
        advanced.validate_union()?;
        let producer_count = usize::from(advanced.union_all.is_some())
            + usize::from(advanced.related.is_some())
            + usize::from(advanced.call.is_some());
        if producer_count > 1 {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_query_requires_one_producer_node(),
            ));
        }
        if let Some(call) = &advanced.call {
            let (_, definition) = selected(call, &self.context.definitions)?;
            let baseline = exact_definition_schema(self.context, &definition, schema)?;
            let vars = shapes
                .iter()
                .map(|(n, s)| (n.clone(), shape_type(schema, s)))
                .collect();
            let mut env = query_env_traced(schema, trace.as_ref())?;
            env.tag_structure = self.context.tag_structure.clone();
            env.parameter_shapes = shapes.clone();
            let mut args = BTreeMap::new();
            for (name, expression) in &call.arguments {
                let compiled = compile(expression, schema, &vars)?;
                if trace.is_some() {
                    preflight_provider_expression(
                        &compiled,
                        &parameters,
                        schema,
                        &mut self.remaining,
                        &env,
                    )?;
                }
                let cell = evaluate(&compiled, &parameters, schema, &mut self.remaining, &env)?;
                let value = match cell {
                    Cell::Value { value } => value,
                    Cell::Null => NativeValue::None,
                    _ => return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_saved_call_argument_undecided())),
                };
                args.insert(name.clone(), value);
            }
            let result = self.run(
                &definition.plan,
                baseline,
                &args,
                depth + 1,
                Some(ProviderBaseline {
                    schema: definition.binding.schema.clone(),
                    revision: definition.binding.revision.clone(),
                }),
            )?;
            require_terminal_scope(plan)?;
            return transform(
                result,
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
                self.limits,
                trace.as_ref(),
                self.context.tag_structure.as_ref(),
            );
        }
        if advanced.union_all.is_some() {
            require_terminal_scope(plan)?;
            let mut result: Option<QueryResult> = None;
            for (ordinal, branch) in advanced.union_branches().iter().enumerate() {
                let supplied = parameter_subset(branch, &parameters);
                let mut child =
                    self.run(branch, schema, &supplied, depth + 1, baseline_key.clone())?;
                for row in &mut child.rows {
                    row.key = format!("union/{ordinal}/{}", row.key);
                }
                if let Some(result) = &mut result {
                    if result.terminal_schema != child.terminal_schema {
                        return Err(DataError::new(
                            weftext_messages::UiMessage::error_data_query_advanced_union_all_terminal_types_or_columns_mismatch(),
                        ));
                    }
                    result.rows.extend(child.rows);
                    result.diagnostics.extend(child.diagnostics);
                    if child.coverage == "partial" {
                        result.coverage = "partial".into();
                    }
                    result.aggregate_exact &= child.aggregate_exact;
                    result.limited |= child.limited;
                    if result.rows.len() > self.limits.max_rows {
                        result.rows.truncate(self.limits.max_rows);
                        mark_partial(result, weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted().into());
                        break;
                    }
                } else {
                    result = Some(child);
                }
            }
            return transform(
                result.unwrap(),
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
                self.limits,
                trace.as_ref(),
                self.context.tag_structure.as_ref(),
            );
        }
        if let Some(related) = &advanced.related {
            let supplied = parameter_subset(&related.from, &parameters);
            let origin = self.run(
                &related.from,
                schema,
                &supplied,
                depth + 1,
                baseline_key.clone(),
            )?;
            let available = self
                .limits
                .max_source_bytes
                .saturating_sub(self.source_bytes);
            let (notes, gaps, consumed) = related_notes(
                self.input,
                related,
                &origin,
                schema,
                &mut self.remaining,
                available,
                self.metadata,
            )?;
            self.source_bytes = self.source_bytes.saturating_add(consumed);
            let mut filtered = self.input.clone();
            filtered.notes = notes;
            if origin.coverage == "partial" || !gaps.is_empty() {
                filtered.coverage = "partial".into();
            }
            let mut leaf = plan.clone();
            leaf.advanced = None;
            if advanced.distinct.is_some() {
                leaf.take = None;
                leaf.order.clear();
            }
            let mut result = self.leaf(
                &filtered,
                &leaf,
                schema,
                &parameters,
                &shapes,
                trace.as_ref(),
            )?;
            result.diagnostics.extend(origin.diagnostics);
            result.diagnostics.extend(gaps);
            if origin.rows.iter().any(|r| r.read_only) {
                for row in &mut result.rows {
                    row.read_only = true;
                }
            }
            return distinct_terminal(
                result,
                advanced.distinct.as_deref(),
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
                trace.as_ref(),
                self.context.tag_structure.as_ref(),
            );
        }
        let mut leaf = plan.clone();
        leaf.advanced = None;
        if advanced.distinct.is_some() {
            leaf.take = None;
            leaf.order.clear();
        }
        let mut result = self.leaf(
            self.input,
            &leaf,
            schema,
            &parameters,
            &shapes,
            trace.as_ref(),
        )?;
        if advanced.distinct.is_some() {
            result = distinct_terminal(
                result,
                advanced.distinct.as_deref(),
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
                trace.as_ref(),
                self.context.tag_structure.as_ref(),
            )?;
        }
        Ok(result)
    }
    fn leaf(
        &mut self,
        input: &FrozenQueryInput,
        plan: &QueryPlan,
        schema: &Schema,
        parameters: &BTreeMap<String, Cell>,
        shapes: &BTreeMap<String, Shape>,
        trace: Option<&crate::query_providers::ProviderTrace>,
    ) -> Result<QueryResult> {
        let bytes = input.notes.iter().try_fold(0usize, |bytes, n| {
            Ok::<_, DataError>(bytes.saturating_add(query_note_bytes(n, self.metadata)?))
        })?;
        let available = self
            .limits
            .max_source_bytes
            .saturating_sub(self.source_bytes);
        self.source_bytes = self.source_bytes.saturating_add(bytes.min(available));
        let mut limits = self.limits.clone();
        limits.max_source_bytes = available;
        let mut resolved = plan.clone();
        if let Some(directory) = &plan.scope.directory {
            if let Some(path) = self.context.scope_bindings.get(directory) {
                resolved.scope.directory = Some(path.clone());
            } else if directory.starts_with("weftext-") && directory.contains("://") {
                return Err(DataError::unavailable(
                    weftext_messages::UiMessage::error_data_query_advanced_frozen_directory_binding_missing(),
                ));
            }
        }
        let grouped = resolved.group.is_some();
        let order = resolved.order.clone();
        let grouped_take = resolved.take;
        if grouped {
            resolved.order.clear();
            resolved.take = None;
        }
        let mut result = execute_query_leaf_traced(
            input,
            &resolved,
            schema,
            &limits,
            parameters,
            shapes,
            &mut self.remaining,
            self.metadata,
            trace,
        )?;
        if grouped {
            sort_terminal(
                &mut result,
                &order,
                schema,
                parameters,
                shapes,
                &mut self.remaining,
                trace,
                self.context.tag_structure.as_ref(),
            )?;
            if let Some(t) = grouped_take {
                result.limited |= t < result.rows.len();
                result.rows.truncate(t);
            }
        }
        Ok(result)
    }
}
fn parameter_subset(
    plan: &QueryPlan,
    cells: &BTreeMap<String, Cell>,
) -> BTreeMap<String, NativeValue> {
    plan.advanced
        .as_ref()
        .into_iter()
        .flat_map(|a| &a.parameters)
        .filter_map(|p| {
            cells.get(&p.name).and_then(|c| match c {
                Cell::Value { value } => Some((p.name.clone(), value.clone())),
                Cell::Null => Some((p.name.clone(), NativeValue::None)),
                _ => None,
            })
        })
        .collect()
}
fn require_terminal_scope(plan: &QueryPlan) -> Result<()> {
    if plan.scope.directory.is_some()
        || plan.scope.lifecycle != "live"
        || !plan.scope.descendants
        || !plan.types.is_empty()
    {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_query_advanced_terminal_scope_must_be_in_scan_branches(),
        ));
    }
    Ok(())
}
fn mark_partial_error(result: &mut QueryResult, error: DataError) {
    result.coverage = "partial".into();
    result.aggregate_exact = false;
    result.diagnostics.extend(error.into_diagnostics());
}
fn mark_partial(result: &mut QueryResult, reason: ErrorDescription) {
    result.coverage = "partial".into();
    result.aggregate_exact = false;
    result.diagnostics.push(reason.into());
}

fn related_notes(
    input: &FrozenQueryInput,
    related: &QueryRelated,
    origin: &QueryResult,
    schema: &Schema,
    remaining: &mut u64,
    max_source_bytes: usize,
    metadata: &BTreeMap<String, AuthorizedQueryMetadata>,
) -> Result<(Vec<FrozenNote>, Vec<ErrorDescription>, usize)> {
    if !matches!(related.direction.as_str(), "forward" | "inverse") {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_query_advanced_related_direction_invalid(),
        ));
    }
    let relation = schema
        .relations
        .iter()
        .find(|r| r.id == related.relation)
        .ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_unknown_registered_relation_id(),
            )
        })?;
    let mut seeds = BTreeSet::new();
    for row in &origin.rows {
        let identity = row.entity_key.as_deref().or_else(|| {
            if origin.terminal_schema.get("self") == Some(&scalar("node_ref")) {
                row.cells
                    .get("self")
                    .and_then(Cell::native)
                    .and_then(NativeValue::text)
            } else {
                None
            }
        });
        seeds.insert(
            identity
                .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_query_advanced_related_requires_declared_node_identities()))?
                .to_string(),
        );
    }
    fn refs<'a>(value: &'a NativeValue, path: &[SchemaPath], out: &mut Vec<&'a str>) {
        if path.is_empty() {
            if let Some(s) = value.text() {
                out.push(s);
            }
            return;
        }
        match &path[0] {
            SchemaPath::Each => {
                if let Some(a) = value.array() {
                    for v in a {
                        refs(v, &path[1..], out);
                    }
                }
            }
            SchemaPath::Member { key } => {
                if let Some(v) = value.member(key) {
                    refs(v, &path[1..], out);
                }
            }
        }
    }
    let mut wanted = BTreeSet::new();
    let mut gaps = vec![];
    let mut consumed = 0usize;
    let mut targets: BTreeMap<String, bool> = BTreeMap::new();
    if seeds.is_empty() {
        return Ok((vec![], gaps, consumed));
    }
    'scan: for note in &input.notes {
        if !matches!(note.lifecycle.as_str(), "live" | "trash") {
            continue;
        }
        if related.direction == "forward"
            && relation.direction != "symmetric"
            && !seeds.contains(&note.entity_key)
        {
            continue;
        }
        let note_bytes = query_note_bytes(note, metadata)?;
        if *remaining == 0 || consumed.saturating_add(note_bytes) > max_source_bytes {
            gaps.push(
                weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted()
                    .into(),
            );
            break;
        }
        *remaining -= 1;
        consumed += note_bytes;
        let p = query_projection(note, schema, metadata);
        if p.qualification != "qualified" {
            gaps.push(weftext_messages::UiMessage::error_data_query_advanced_related_source_projection_unavailable().into());
            continue;
        }
        if !p.resolved_types.contains(&relation.source_type) {
            continue;
        }
        let cell = p
            .fields
            .iter()
            .find(|f| f.field_id == relation.field)
            .map(|f| &f.cell);
        match cell {
            Some(Cell::Value { value }) => {
                let mut endpoints = vec![];
                refs(value, &relation.target_path, &mut endpoints);
                for endpoint in endpoints {
                    if *remaining == 0 {
                        gaps.push(weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted().into());
                        break 'scan;
                    }
                    *remaining -= 1;
                    if !targets.contains_key(endpoint) {
                        let qualified = if let Some(target) =
                            input.notes.iter().find(|n| n.entity_key == endpoint)
                        {
                            let target_bytes = query_note_bytes(target, metadata)?;
                            if *remaining == 0
                                || consumed.saturating_add(target_bytes) > max_source_bytes
                            {
                                gaps.push(weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted().into());
                                break 'scan;
                            }
                            *remaining -= 1;
                            consumed += target_bytes;
                            let projection = query_projection(target, schema, metadata);
                            projection.qualification == "qualified"
                                && (relation.target_types["kind"].as_str() != Some("any_of")
                                    || relation.target_types["ids"].as_array().is_some_and(|ids| {
                                        ids.iter().any(|id| {
                                            id.as_str().is_some_and(|id| {
                                                projection.resolved_types.iter().any(|t| t == id)
                                            })
                                        })
                                    }))
                        } else {
                            false
                        };
                        targets.insert(endpoint.to_string(), qualified);
                    }
                    if !targets[endpoint] {
                        gaps.push(weftext_messages::UiMessage::error_data_query_advanced_related_endpoint_qualification_unavailable().into());
                        continue;
                    }
                    if (related.direction == "inverse" || relation.direction == "symmetric")
                        && seeds.contains(endpoint)
                    {
                        wanted.insert(note.entity_key.clone());
                    }
                    if (related.direction == "forward" || relation.direction == "symmetric")
                        && seeds.contains(&note.entity_key)
                    {
                        wanted.insert(endpoint.to_string());
                    }
                }
            }
            Some(Cell::Invalid { .. } | Cell::Unavailable { .. }) => gaps.push(
                weftext_messages::UiMessage::error_data_query_advanced_related_assertion_undecided(
                )
                .into(),
            ),
            _ => {}
        }
    }
    let notes = input
        .notes
        .iter()
        .filter(|n| wanted.contains(&n.entity_key))
        .cloned()
        .collect::<Vec<_>>();
    if notes.len() != wanted.len() {
        gaps.push(
            weftext_messages::UiMessage::error_data_query_advanced_related_endpoint_unavailable()
                .into(),
        );
    }
    if origin.coverage == "partial" || input.coverage == "partial" {
        gaps.push(
            weftext_messages::UiMessage::error_data_query_advanced_related_scan_coverage_partial()
                .into(),
        );
    }
    Ok((notes, gaps, consumed))
}

fn terminal_env(
    result: &QueryResult,
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    trace: Option<&crate::query_providers::ProviderTrace>,
    tags: Option<&FrozenTagStructure>,
) -> Result<(BTreeMap<String, QType>, QueryRuntime)> {
    let mut vars = result.terminal_schema.clone();
    for (name, shape) in shapes {
        if vars
            .insert(name.clone(), shape_type(schema, shape))
            .is_some()
        {
            return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_parameter_collides_with_terminal_column()));
        }
    }
    let mut env = query_env_traced(schema, trace)?;
    env.tag_structure = tags.cloned();
    env.parameter_shapes = shapes.clone();
    // Type adapters for terminal exact text products follow their declared type,
    // including nested list/record members, never a first row's representation.
    for (name, ty) in &result.terminal_schema {
        if let Some(shape) = type_shape(ty) {
            env.parameter_shapes.insert(name.clone(), shape);
        }
    }
    let _ = parameters;
    Ok((vars, env))
}
pub(crate) fn type_shape(ty: &QType) -> Option<Shape> {
    Some(match ty {
        QType::Scalar(s) => Shape::Scalar {
            semantic: s.clone(),
        },
        QType::List(t) => Shape::List {
            items: Box::new(type_shape(t)?),
            min: 0,
            max: None,
        },
        QType::Record(m) => Shape::Record {
            members: m
                .iter()
                .map(|(k, t)| {
                    Some((
                        k.clone(),
                        Member {
                            shape: type_shape(t)?,
                            required: false,
                            nullable: true,
                        },
                    ))
                })
                .collect::<Option<_>>()?,
            unknown: "preserve".into(),
            constraints: vec![],
        },
        QType::Union(v) => Shape::Union {
            variants: v.iter().map(type_shape).collect::<Option<_>>()?,
            discriminator: None,
        },
        _ => return None,
    })
}

#[allow(clippy::too_many_arguments)]
fn sort_terminal(
    result: &mut QueryResult,
    order: &[QueryOrder],
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
    trace: Option<&crate::query_providers::ProviderTrace>,
    tags: Option<&FrozenTagStructure>,
) -> Result<()> {
    let (vars, env) = terminal_env(result, schema, parameters, shapes, trace, tags)?;
    if !order.is_empty() {
        let compiled_order = order
            .iter()
            .map(|o| {
                if !matches!(o.nulls.as_str(), "first" | "last") {
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_sort_null_placement_invalid()));
                }
                let c = compile(&o.expr, schema, &vars)?;
                if !orderable(&c.ty) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_query_advanced_sort_requires_declared_scalar_comparator(),
                    ));
                }
                Ok(c)
            })
            .collect::<Result<Vec<_>>>()?;
        if trace.is_some() {
            for expression in &compiled_order {
                preflight_provider_expression(expression, parameters, schema, remaining, &env)?;
            }
        }
        let mut sorted = vec![];
        for row in std::mem::take(&mut result.rows) {
            let mut cells = row.cells.clone();
            cells.extend(parameters.clone());
            let keys = compiled_order
                .iter()
                .map(|c| evaluate(c, &cells, schema, remaining, &env))
                .collect::<Result<Vec<_>>>();
            match keys {
                Ok(keys) => {
                    if keys
                        .iter()
                        .any(|c| matches!(c, Cell::Invalid { .. } | Cell::Unavailable { .. }))
                    {
                        mark_partial(result, weftext_messages::UiMessage::error_data_query_advanced_sort_input_undecided().into());
                    }
                    sorted.push((row, keys));
                }
                Err(_) => {
                    mark_partial(
                        result,
                        weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted()
                            .into(),
                    );
                    break;
                }
            }
        }
        sorted.sort_by(|(a, ak), (b, bk)| {
            for (((a, b), o), c) in ak.iter().zip(bk).zip(order).zip(&compiled_order) {
                let mut cmp = compare_cell(a, b, &o.nulls, &c.ty);
                if o.descending && a.native().is_some() && b.native().is_some() {
                    cmp = cmp.reverse();
                }
                if !cmp.is_eq() {
                    return cmp;
                }
            }
            a.key.cmp(&b.key)
        });
        result.rows = sorted.into_iter().map(|(r, _)| r).collect();
    }
    if env
        .native_budget_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            result,
            weftext_messages::UiMessage::error_data_query_advanced_consumed_field_decoder_budget_exhausted().into(),
        );
    }
    if env.tag_partial.load(std::sync::atomic::Ordering::Relaxed) {
        mark_partial(
            result,
            weftext_messages::UiMessage::error_data_query_advanced_tag_structure_input_unavailable(
            )
            .into(),
        );
    }
    if env
        .provider_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            result,
            weftext_messages::UiMessage::error_data_query_advanced_temporal_expansion_partial()
                .into(),
        );
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn distinct_terminal(
    mut result: QueryResult,
    keys: Option<&[QueryColumn]>,
    plan: &QueryPlan,
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
    trace: Option<&crate::query_providers::ProviderTrace>,
    tags: Option<&FrozenTagStructure>,
) -> Result<QueryResult> {
    let take = plan.take;
    let order = &plan.order;
    let Some(keys) = keys else {
        return Ok(result);
    };
    if keys.is_empty() {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_query_advanced_distinct_requires_typed_equality_keys(),
        ));
    }
    let (vars, env) = terminal_env(&result, schema, parameters, shapes, trace, tags)?;
    let compiled = keys
        .iter()
        .map(|k| compile(&k.expr, schema, &vars))
        .collect::<Result<Vec<_>>>()?;
    if trace.is_some() {
        for expression in &compiled {
            preflight_provider_expression(expression, parameters, schema, remaining, &env)?;
        }
    }
    let mut groups: BTreeMap<String, (String, QueryRow)> = BTreeMap::new();
    for mut row in std::mem::take(&mut result.rows) {
        let mut cells = row.cells.clone();
        cells.extend(parameters.clone());
        let values = compiled
            .iter()
            .map(|c| evaluate(c, &cells, schema, remaining, &env))
            .collect::<Result<Vec<_>>>();
        let values = match values {
            Ok(v) => v,
            Err(_) => {
                mark_partial(
                    &mut result,
                    weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted()
                        .into(),
                );
                break;
            }
        };
        if values
            .iter()
            .any(|c| matches!(c, Cell::Invalid { .. } | Cell::Unavailable { .. }))
        {
            mark_partial(
                &mut result,
                weftext_messages::UiMessage::error_data_query_advanced_distinct_key_undecided()
                    .into(),
            );
            continue;
        }
        let key = match canonical_cells(&values, &compiled) {
            Ok(k) => k,
            Err(e) => {
                mark_partial_error(&mut result, e);
                continue;
            }
        };
        let canonical_whole = serde_json::to_string(
            &row.cells
                .iter()
                .map(|(name, c)| {
                    Ok((
                        name,
                        match c {
                            Cell::Value { value } => {
                                canonical_native(value, &result.terminal_schema[name])?
                            }
                            _ => serde_json::to_string(c)?,
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
        )?;
        let whole = format!("{}:{}", canonical_whole, serde_json::to_string(&row.cells)?);
        row.key = format!("distinct:{key}");
        row.entity_key = None;
        row.path = None;
        row.revision = None;
        row.lifecycle = None;
        row.read_only = true;
        match groups.get_mut(&key) {
            Some((old, r)) if whole < *old => {
                *old = whole;
                *r = row;
            }
            None => {
                groups.insert(key, (whole, row));
            }
            _ => {}
        }
    }
    result.rows = groups.into_values().map(|(_, r)| r).collect();
    sort_terminal(
        &mut result,
        order,
        schema,
        parameters,
        shapes,
        remaining,
        trace,
        tags,
    )?;
    if env
        .native_budget_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            &mut result,
            weftext_messages::UiMessage::error_data_query_advanced_consumed_field_decoder_budget_exhausted().into(),
        );
    }
    if env.tag_partial.load(std::sync::atomic::Ordering::Relaxed) {
        mark_partial(
            &mut result,
            weftext_messages::UiMessage::error_data_query_advanced_tag_structure_input_unavailable(
            )
            .into(),
        );
    }
    if env
        .provider_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            &mut result,
            weftext_messages::UiMessage::error_data_query_advanced_temporal_expansion_partial()
                .into(),
        );
    }
    result.total_known = result.rows.len();
    if let Some(t) = take {
        result.limited |= t < result.rows.len();
        result.rows.truncate(t);
    }
    Ok(result)
}

/// Apply the existing CEL expressions to a typed terminal from union/call.
#[allow(clippy::too_many_arguments)]
fn transform(
    mut result: QueryResult,
    plan: &QueryPlan,
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
    limits: &QueryBudget,
    trace: Option<&crate::query_providers::ProviderTrace>,
    tags: Option<&FrozenTagStructure>,
) -> Result<QueryResult> {
    let (mut vars, env) = terminal_env(&result, schema, parameters, shapes, trace, tags)?;
    let unnest = plan
        .unnest
        .as_ref()
        .map(|u| {
            if vars.contains_key(&u.binding) {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_query_advanced_unnest_binding_collides_with_terminal_column(),
                ));
            }
            let c = compile(&u.array, schema, &vars)?;
            let QType::List(item) = &c.ty else {
                return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_unnest_requires_array()));
            };
            vars.insert(u.binding.clone(), *item.clone());
            Ok(c)
        })
        .transpose()?;
    let predicate = plan
        .r#where
        .as_ref()
        .map(|p| compile(p, schema, &vars))
        .transpose()?;
    if predicate.as_ref().is_some_and(|p| p.ty != scalar("bool")) {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_query_advanced_where_requires_boolean_result(),
        ));
    }
    let columns = plan
        .select
        .iter()
        .map(|c| compile(&c.expr, schema, &vars))
        .collect::<Result<Vec<_>>>()?;
    let orders = if plan.group.is_some() {
        vec![]
    } else {
        plan.order
            .iter()
            .map(|o| {
                if !matches!(o.nulls.as_str(), "first" | "last") {
                    return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_sort_null_placement_invalid()));
                }
                let c = compile(&o.expr, schema, &vars)?;
                if !orderable(&c.ty) {
                    return Err(DataError::new(
                        weftext_messages::UiMessage::error_data_query_advanced_sort_requires_declared_scalar_comparator(),
                    ));
                }
                Ok(c)
            })
            .collect::<Result<Vec<_>>>()?
    };
    let (group_keys, aggregates) = if let Some(group) = &plan.group {
        let keys = group
            .keys
            .iter()
            .map(|k| compile(&k.expr, schema, &vars))
            .collect::<Result<Vec<_>>>()?;
        let aggs=group.aggregates.iter().map(|a|{
            if !matches!(a.op.as_str(),"count"|"sum"|"min"|"max"){return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_unknown_aggregate()));}
            if a.op=="count"{if a.expr.is_some(){return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_count_rejects_expression()));}Ok(None)}else{
                let c=compile(a.expr.as_deref().ok_or_else(||DataError::new(weftext_messages::UiMessage::error_data_query_advanced_aggregate_expression_required()))?,schema,&vars)?;
                if a.op=="sum"&&!matches!(&c.ty,QType::Scalar(s)if matches!(s.as_str(),"int"|"decimal"|"exact_decimal")){return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_sum_requires_integer_or_exact_decimal()));}
                if matches!(a.op.as_str(),"min"|"max")&&!orderable(&c.ty){return Err(DataError::new(weftext_messages::UiMessage::error_data_query_advanced_min_max_requires_scalar_comparator()));}
                Ok(Some(c))
            }
        }).collect::<Result<Vec<_>>>()?;
        let mut names = BTreeSet::new();
        for name in group
            .keys
            .iter()
            .map(|k| &k.name)
            .chain(group.aggregates.iter().map(|a| &a.name))
        {
            if name.is_empty() || !names.insert(name) {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_query_advanced_duplicate_group_column(),
                ));
            }
        }
        (keys, aggs)
    } else {
        (vec![], vec![])
    };
    let mut names = BTreeSet::new();
    for c in &plan.select {
        if c.name.is_empty() || !names.insert(&c.name) {
            return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_duplicate_projection_column(
                ),
            ));
        }
    }
    let mut rows = vec![];
    if trace.is_some() {
        for expression in unnest
            .iter()
            .chain(predicate.iter())
            .chain(&columns)
            .chain(&orders)
            .chain(&group_keys)
            .chain(aggregates.iter().flatten())
        {
            preflight_provider_expression(expression, parameters, schema, remaining, &env)?;
        }
    }
    'input: for row in std::mem::take(&mut result.rows) {
        let mut cells = row.cells.clone();
        cells.extend(parameters.clone());
        let expanded = if let (Some(u), Some(c)) = (&plan.unnest, &unnest) {
            match evaluate(c, &cells, schema, remaining, &env) {
                Ok(Cell::Value {
                    value: NativeValue::Array(a),
                }) => a
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let mut cells = cells.clone();
                        cells.insert(u.binding.clone(), Cell::value(v));
                        (format!("{}/item/{i}", row.key), cells)
                    })
                    .collect(),
                _ => {
                    mark_partial(
                        &mut result,
                        weftext_messages::UiMessage::error_data_query_advanced_unnest_input_undecided()
                            .into(),
                    );
                    continue;
                }
            }
        } else {
            vec![(row.key.clone(), cells)]
        };
        for (key, cells) in expanded {
            if rows.len() >= limits.max_rows {
                mark_partial(
                    &mut result,
                    weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted()
                        .into(),
                );
                break 'input;
            }
            if let Some(p) = &predicate {
                match evaluate(p, &cells, schema, remaining, &env) {
                    Ok(Cell::Value {
                        value: NativeValue::Bool(true),
                    }) => {}
                    Ok(Cell::Value {
                        value: NativeValue::Bool(false),
                    }) => continue,
                    _ => {
                        mark_partial(
                            &mut result,
                            weftext_messages::UiMessage::error_data_query_advanced_filter_undecided()
                                .into(),
                        );
                        continue;
                    }
                }
            }
            let values = columns
                .iter()
                .map(|c| evaluate(c, &cells, schema, remaining, &env))
                .collect::<Result<Vec<_>>>();
            let order = orders
                .iter()
                .map(|c| evaluate(c, &cells, schema, remaining, &env))
                .collect::<Result<Vec<_>>>();
            let keys = group_keys
                .iter()
                .map(|c| evaluate(c, &cells, schema, remaining, &env))
                .collect::<Result<Vec<_>>>();
            let aggs = aggregates
                .iter()
                .map(|c| {
                    c.as_ref()
                        .map(|c| evaluate(c, &cells, schema, remaining, &env))
                        .transpose()
                })
                .collect::<Result<Vec<_>>>();
            let (values, order, keys, aggs) = match (values, order, keys, aggs) {
                (Ok(v), Ok(o), Ok(k), Ok(a)) => (v, o, k, a),
                _ => {
                    mark_partial(
                        &mut result,
                        weftext_messages::UiMessage::error_data_query_advanced_host_budget_exhausted()
                            .into(),
                    );
                    break 'input;
                }
            };
            if order
                .iter()
                .any(|c| matches!(c, Cell::Invalid { .. } | Cell::Unavailable { .. }))
            {
                mark_partial(
                    &mut result,
                    weftext_messages::UiMessage::error_data_query_advanced_sort_input_undecided()
                        .into(),
                );
            }
            let mut output = row.clone();
            output.key = key;
            output.read_only |= plan.unnest.is_some();
            if !plan.select.is_empty() {
                output.cells = plan
                    .select
                    .iter()
                    .zip(values)
                    .map(|(c, v)| (c.name.clone(), v))
                    .collect();
            }
            rows.push((output, order, keys, aggs));
        }
    }
    if plan.group.is_none() && !plan.order.is_empty() {
        rows.sort_by(|(a, ak, _, _), (b, bk, _, _)| {
            for (((a, b), o), c) in ak.iter().zip(bk).zip(&plan.order).zip(&orders) {
                let mut cmp = compare_cell(a, b, &o.nulls, &c.ty);
                if o.descending && a.native().is_some() && b.native().is_some() {
                    cmp = cmp.reverse();
                }
                if !cmp.is_eq() {
                    return cmp;
                }
            }
            a.key.cmp(&b.key)
        });
    }
    if let Some(group) = &plan.group {
        let mut grouped: BTreeMap<String, TerminalGroupBucket> = BTreeMap::new();
        for (_, _, keys, aggs) in rows {
            if keys
                .iter()
                .any(|k| matches!(k, Cell::Invalid { .. } | Cell::Unavailable { .. }))
            {
                mark_partial(
                    &mut result,
                    weftext_messages::UiMessage::error_data_query_advanced_group_key_undecided()
                        .into(),
                );
                continue;
            }
            let key = match canonical_cells(&keys, &group_keys) {
                Ok(k) => k,
                Err(e) => {
                    mark_partial_error(&mut result, e);
                    continue;
                }
            };
            grouped
                .entry(key)
                .or_insert_with(|| (keys, vec![]))
                .1
                .push(aggs);
        }
        if group.keys.is_empty() && grouped.is_empty() {
            grouped.insert("[]".into(), (vec![], vec![]));
        }
        result.rows = grouped
            .into_iter()
            .map(|(key, (keys, values))| {
                let mut cells = group
                    .keys
                    .iter()
                    .zip(keys)
                    .map(|(k, v)| (k.name.clone(), v))
                    .collect::<BTreeMap<_, _>>();
                for (i, a) in group.aggregates.iter().enumerate() {
                    let vals = values.iter().map(|r| r[i].as_ref()).collect::<Vec<_>>();
                    let cell = aggregate_cell(&a.op, &vals, aggregates[i].as_ref().map(|c| &c.ty));
                    if matches!(cell, Cell::Invalid { .. } | Cell::Unavailable { .. }) {
                        mark_partial(&mut result, weftext_messages::UiMessage::error_data_query_advanced_aggregate_input_undecided().into());
                    }
                    cells.insert(a.name.clone(), cell);
                }
                QueryRow {
                    key: format!("group:{key}"),
                    entity_key: None,
                    path: None,
                    revision: None,
                    cells,
                    read_only: true,
                    lifecycle: None,
                }
            })
            .collect();
        result.terminal_schema = group
            .keys
            .iter()
            .zip(&group_keys)
            .map(|(k, c)| (k.name.clone(), c.ty.clone()))
            .chain(group.aggregates.iter().zip(&aggregates).map(|(a, c)| {
                (
                    a.name.clone(),
                    c.as_ref().map_or_else(|| scalar("int"), |c| c.ty.clone()),
                )
            }))
            .collect();
    } else {
        result.rows = rows.into_iter().map(|(r, _, _, _)| r).collect();
        if !plan.select.is_empty() {
            result.terminal_schema = plan
                .select
                .iter()
                .zip(&columns)
                .map(|(c, s)| (c.name.clone(), s.ty.clone()))
                .collect();
        }
    }
    if env
        .native_budget_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            &mut result,
            weftext_messages::UiMessage::error_data_query_advanced_consumed_field_decoder_budget_exhausted().into(),
        );
    }
    if env.tag_partial.load(std::sync::atomic::Ordering::Relaxed) {
        mark_partial(
            &mut result,
            weftext_messages::UiMessage::error_data_query_advanced_tag_structure_input_unavailable(
            )
            .into(),
        );
    }
    if env
        .provider_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            &mut result,
            weftext_messages::UiMessage::error_data_query_advanced_temporal_expansion_partial()
                .into(),
        );
    }
    if plan.group.is_some() {
        sort_terminal(
            &mut result,
            &plan.order,
            schema,
            parameters,
            shapes,
            remaining,
            trace,
            tags,
        )?;
    }
    result.total_known = result.rows.len();
    let distinct = plan.advanced.as_ref().and_then(|a| a.distinct.as_deref());
    if distinct.is_some() {
        return distinct_terminal(
            result, distinct, plan, schema, parameters, shapes, remaining, trace, tags,
        );
    }
    if let Some(t) = plan.take {
        result.limited |= t < result.rows.len();
        result.rows.truncate(t);
    }
    Ok(result)
}

// Native saved carriers persist domain defaults as actual author values. The
// tagged NativeValue representation belongs only to the API, never author data.
pub(crate) fn saved_plan_json(value: &NativeValue) -> Result<serde_json::Value> {
    fn convert(v: &NativeValue, advanced: bool, parameter: bool) -> Result<serde_json::Value> {
        Ok(match v {
            NativeValue::Record(m) => serde_json::Value::Object(
                m.iter()
                    .map(|m| {
                        Ok((
                            m.key.clone(),
                            if parameter && m.key == "default" {
                                serde_json::to_value(&m.value)?
                            } else {
                                convert(
                                    &m.value,
                                    m.key == "advanced",
                                    advanced && m.key == "parameters",
                                )?
                            },
                        ))
                    })
                    .collect::<Result<_>>()?,
            ),
            NativeValue::Array(a) => serde_json::Value::Array(
                a.iter()
                    .map(|v| convert(v, false, parameter))
                    .collect::<Result<_>>()?,
            ),
            NativeValue::None => serde_json::Value::Null,
            NativeValue::Text(s) => s.clone().into(),
            NativeValue::Bool(b) => (*b).into(),
            NativeValue::Int(s) => s
                .parse::<i64>()
                .map_err(|_| DataError::new(weftext_messages::UiMessage::error_data_query_advanced_invalid_saved_integer()))?
                .into(),
            _ => return Err(DataError::new(
                weftext_messages::UiMessage::error_data_query_advanced_saved_query_requires_native_scalars(),
            )),
        })
    }
    convert(value, false, false)
}
pub(crate) fn saved_plan_native(value: &serde_json::Value) -> Result<NativeValue> {
    fn convert(v: &serde_json::Value, advanced: bool, parameter: bool) -> Result<NativeValue> {
        Ok(match v {
            serde_json::Value::Object(m) => NativeValue::Record(
                m.iter()
                    .filter(|(_, v)| !v.is_null())
                    .map(|(k, v)| {
                        Ok(NativeMember {
                            key: k.clone(),
                            value: if parameter && k == "default" {
                                serde_json::from_value(v.clone())?
                            } else {
                                convert(v, k == "advanced", advanced && k == "parameters")?
                            },
                        })
                    })
                    .collect::<Result<_>>()?,
            ),
            serde_json::Value::Array(a) => NativeValue::Array(
                a.iter()
                    .map(|v| convert(v, false, parameter))
                    .collect::<Result<_>>()?,
            ),
            _ => json_native(v)?,
        })
    }
    convert(value, false, false)
}
