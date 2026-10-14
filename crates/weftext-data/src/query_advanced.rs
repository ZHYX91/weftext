//! Query producer nodes over one immutable input. CEL and typed equality remain
//! owned by `query`; this module performs no IO or ambient definition lookup.
use crate::{query::*, *};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryAdvanced {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<QueryParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(deserialize_with = "nonempty_union")]
    pub union_all: Vec<QueryPlan>,
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
fn nonempty_union<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<QueryPlan>, D::Error> {
    let plans = Vec::<QueryPlan>::deserialize(deserializer)?;
    if plans.is_empty() {
        return Err(serde::de::Error::custom(
            "unionAll requires at least one producer",
        ));
    }
    Ok(plans)
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
        return Err(DataError(
            "definition_unavailable: owner history missing/ambiguous".into(),
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
        return Err(DataError(
            "definition_unavailable: exact revision missing/ambiguous".into(),
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
            return Err(DataError(
                "definition-call expansion budget exhausted".into(),
            ));
        }
        let Some(a) = &plan.advanced else {
            return Ok(());
        };
        if usize::from(!a.union_all.is_empty())
            + usize::from(a.related.is_some())
            + usize::from(a.call.is_some())
            > 1
        {
            return Err(DataError("Query requires one producer node".into()));
        }
        for child in &a.union_all {
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
                    return Err(DataError("definition-call cycle".into()));
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
            return Err(DataError(
                "Parameter name collides or is not a CEL identifier".into(),
            ));
        }
        // Reuse installation validation so invalid/unknown `use` never reaches
        // shape resolution. This creates no author schema or second authority.
        let mut checked = schema.clone();
        checked.fields.push(Field {
            id: format!("query-parameter/{}", p.name),
            key: p.name.clone(),
            shape: p.shape.clone(),
            nullable: p.nullable,
            default: Some(serde_json::json!({"kind":"absent"})),
        });
        Schema::from_author_json(&serde_json::to_string(&checked)?)?;
        if let Some(default) = &p.default {
            validate_parameter(schema, p, default)?;
        }
        let value = values.get(&p.name).or(p.default.as_ref());
        if required && value.is_none() {
            return Err(DataError(format!("Missing required parameter {}", p.name)));
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
        return Err(DataError("Unknown Query parameter".into()));
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
            return Err(DataError("Query plan expansion budget exhausted".into()));
        }
        let (parameters, shapes) = parameter_bindings(plan, schema, &BTreeMap::new(), false)?;
        let mut remaining = QueryBudget::default().max_steps;
        let a = plan.advanced.clone().unwrap_or_default();
        if usize::from(!a.union_all.is_empty())
            + usize::from(a.related.is_some())
            + usize::from(a.call.is_some())
            > 1
        {
            return Err(DataError("Query requires one producer node".into()));
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
                        .ok_or_else(|| DataError("Unknown callee parameter".into()))?;
                    let compiled = compile(e, schema, &vars)?;
                    if !(p.nullable && compiled.ty == scalar("none"))
                        && !compatible(&compiled.ty, &shape_type(baseline, &p.shape))
                    {
                        return Err(DataError(
                            "Saved-call argument type violates callee parameter shape".into(),
                        ));
                    }
                }
                if target
                    .iter()
                    .any(|p| !call.arguments.contains_key(&p.name) && p.default.is_none())
                {
                    return Err(DataError("Missing required callee parameter".into()));
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
                );
            }
            if !plan.select.is_empty()
                || plan.r#where.is_some()
                || !plan.order.is_empty()
                || plan.unnest.is_some()
                || plan.group.is_some()
                || a.distinct.is_some()
            {
                return Err(DataError(
                    "Saved-call terminal edits require exact definition context validation".into(),
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
            );
        }
        if !a.union_all.is_empty() {
            require_terminal_scope(plan)?;
            let mut first = None;
            for branch in &a.union_all {
                let result = visit(input, branch, schema, context, nodes, depth + 1)?;
                if first.as_ref().is_some_and(|first: &QueryResult| {
                    first.terminal_schema != result.terminal_schema
                }) {
                    return Err(DataError(
                        "union_all requires identical declared terminal types/columns".into(),
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
            );
        }
        if let Some(related) = &a.related {
            if !schema.relations.iter().any(|r| r.id == related.relation)
                || !matches!(related.direction.as_str(), "forward" | "inverse")
            {
                return Err(DataError("Unknown relation/direction".into()));
            }
            visit(input, &related.from, schema, context, nodes, depth + 1)?;
        }
        let mut leaf = plan.clone();
        leaf.advanced = None;
        if leaf.group.is_some() {
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
        )?;
        if plan.group.is_some() {
            sort_terminal(
                &mut result,
                &plan.order,
                schema,
                &parameters,
                &shapes,
                &mut remaining,
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
            return Err(DataError("Query scope expansion budget exhausted".into()));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        if let Some(call) = &a.call {
            let (_, definition) = selected(call, owners)?;
            visit(&definition.plan, owners, out, nodes, depth + 1)?;
        } else if !a.union_all.is_empty() {
            for p in &a.union_all {
                visit(p, owners, out, nodes, depth + 1)?;
            }
        } else {
            let lifecycle = if plan.scope.lifecycle.is_empty() {
                "live"
            } else {
                &plan.scope.lifecycle
            };
            if !matches!(lifecycle, "live" | "trash") {
                return Err(DataError("Unsupported lifecycle scope".into()));
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
            return Err(DataError("Query scope expansion budget exhausted".into()));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        if let Some(call) = &a.call {
            let (_, definition) = selected(call, owners)?;
            visit(&definition.plan, owners, out, nodes, depth + 1)?;
        } else if !a.union_all.is_empty() {
            for p in &a.union_all {
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
            return Err(DataError("CEL source exceeds profile budget".into()));
        }
        let program = Program::compile(source).map_err(|e| DataError(e.to_string()))?;
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
            return Err(DataError("Query product expansion budget exhausted".into()));
        }
        let a = plan.advanced.clone().unwrap_or_default();
        if plan.select.is_empty() && a.union_all.is_empty() && a.call.is_none() {
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
        for p in &a.union_all {
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
fn validate_parameter(
    schema: &Schema,
    parameter: &QueryParameter,
    value: &NativeValue,
) -> Result<()> {
    if matches!(value, NativeValue::None) && parameter.nullable {
        return Ok(());
    }
    if !schema.validate(&parameter.shape, value).is_valid() {
        return Err(DataError(format!(
            "Parameter {} violates its declared shape",
            parameter.name
        )));
    }
    Ok(())
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
        return Err(DataError(
            "definition_unavailable: exact schema baseline missing/ambiguous".into(),
        ));
    }
    let baseline = &matches[0].projection;
    if !baseline_contract_compatible(baseline, current)? {
        return Err(DataError(
            "definition_unavailable: schema compatibility requires an explicit migration".into(),
        ));
    }
    Ok(baseline)
}
pub(crate) fn execute_advanced_query(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
    limits: &QueryBudget,
) -> Result<QueryResult> {
    let empty = FrozenAdvancedInput::default();
    let context = input.advanced.as_ref().unwrap_or(&empty);
    let definition_bytes = context
        .definitions
        .iter()
        .flat_map(|o| &o.history)
        .try_fold(0usize, |bytes, r| bytes.checked_add(r.source.len()))
        .ok_or_else(|| DataError("host-budget-exhausted".into()))?;
    if definition_bytes > limits.max_source_bytes {
        return Err(DataError("host-budget-exhausted".into()));
    }
    definition_dependencies(plan, &context.definitions)?;
    let mut runner = Runner {
        input,
        context,
        limits,
        remaining: limits.max_steps,
        nodes: 0,
        source_bytes: definition_bytes,
    };
    runner.run(plan, schema, &context.parameters, 0)
}
struct Runner<'a> {
    input: &'a FrozenQueryInput,
    context: &'a FrozenAdvancedInput,
    limits: &'a QueryBudget,
    remaining: u64,
    nodes: usize,
    source_bytes: usize,
}
impl Runner<'_> {
    fn run(
        &mut self,
        plan: &QueryPlan,
        schema: &Schema,
        values: &BTreeMap<String, NativeValue>,
        depth: usize,
    ) -> Result<QueryResult> {
        self.nodes += 1;
        if depth > MAX_CALL_DEPTH || self.nodes > MAX_PLAN_NODES {
            return Err(DataError(
                "definition-call expansion budget exhausted".into(),
            ));
        }
        let (parameters, shapes) = bind_parameters(plan, schema, values)?;
        let advanced = plan.advanced.clone().unwrap_or_default();
        let producer_count = usize::from(!advanced.union_all.is_empty())
            + usize::from(advanced.related.is_some())
            + usize::from(advanced.call.is_some());
        if producer_count > 1 {
            return Err(DataError("Query requires one producer node".into()));
        }
        if let Some(call) = &advanced.call {
            let (_, definition) = selected(call, &self.context.definitions)?;
            let baseline = exact_definition_schema(self.context, &definition, schema)?;
            let vars = shapes
                .iter()
                .map(|(n, s)| (n.clone(), shape_type(schema, s)))
                .collect();
            let mut env = query_env(schema)?;
            env.parameter_shapes = shapes.clone();
            let mut args = BTreeMap::new();
            for (name, expression) in &call.arguments {
                let compiled = compile(expression, schema, &vars)?;
                let cell = evaluate(&compiled, &parameters, schema, &mut self.remaining, &env)?;
                let value = match cell {
                    Cell::Value { value } => value,
                    Cell::Null => NativeValue::None,
                    _ => return Err(DataError("Saved-call argument is undecided".into())),
                };
                args.insert(name.clone(), value);
            }
            let result = self.run(&definition.plan, baseline, &args, depth + 1)?;
            require_terminal_scope(plan)?;
            return transform(
                result,
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
                self.limits,
            );
        }
        if !advanced.union_all.is_empty() {
            require_terminal_scope(plan)?;
            let mut result: Option<QueryResult> = None;
            for (ordinal, branch) in advanced.union_all.iter().enumerate() {
                let supplied = parameter_subset(branch, &parameters);
                let mut child = self.run(branch, schema, &supplied, depth + 1)?;
                for row in &mut child.rows {
                    row.key = format!("union/{ordinal}/{}", row.key);
                }
                if let Some(result) = &mut result {
                    if result.terminal_schema != child.terminal_schema {
                        return Err(DataError(
                            "union_all requires identical declared terminal types/columns".into(),
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
                        mark_partial(result, "host-budget-exhausted");
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
            );
        }
        if let Some(related) = &advanced.related {
            let supplied = parameter_subset(&related.from, &parameters);
            let origin = self.run(&related.from, schema, &supplied, depth + 1)?;
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
            }
            let mut result = self.leaf(&filtered, &leaf, schema, &parameters, &shapes)?;
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
            );
        }
        let mut leaf = plan.clone();
        leaf.advanced = None;
        if advanced.distinct.is_some() {
            leaf.take = None;
        }
        let mut result = self.leaf(self.input, &leaf, schema, &parameters, &shapes)?;
        if advanced.distinct.is_some() {
            result = distinct_terminal(
                result,
                advanced.distinct.as_deref(),
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
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
    ) -> Result<QueryResult> {
        let bytes = input.notes.iter().map(|n| n.source.len()).sum::<usize>();
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
                return Err(DataError(
                    "scope_unavailable: exact frozen directory binding missing".into(),
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
        let mut result = execute_query_leaf(
            input,
            &resolved,
            schema,
            &limits,
            parameters,
            shapes,
            &mut self.remaining,
        )?;
        if grouped {
            sort_terminal(
                &mut result,
                &order,
                schema,
                parameters,
                shapes,
                &mut self.remaining,
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
        return Err(DataError(
            "Terminal producer scope belongs in its scan branches".into(),
        ));
    }
    Ok(())
}
fn mark_partial(result: &mut QueryResult, reason: &str) {
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
) -> Result<(Vec<FrozenNote>, Vec<String>, usize)> {
    if !matches!(related.direction.as_str(), "forward" | "inverse") {
        return Err(DataError(
            "related direction requires forward/inverse".into(),
        ));
    }
    let relation = schema
        .relations
        .iter()
        .find(|r| r.id == related.relation)
        .ok_or_else(|| DataError("Unknown registered RelationId".into()))?;
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
                .ok_or_else(|| DataError("related requires declared node identities".into()))?
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
        if *remaining == 0 || consumed.saturating_add(note.source.len()) > max_source_bytes {
            gaps.push("host-budget-exhausted".into());
            break;
        }
        *remaining -= 1;
        consumed += note.source.len();
        let p = project(&note.source, schema);
        if p.qualification != "qualified" {
            gaps.push("related source projection unavailable".into());
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
                        gaps.push("host-budget-exhausted".into());
                        break 'scan;
                    }
                    *remaining -= 1;
                    if !targets.contains_key(endpoint) {
                        let qualified = if let Some(target) =
                            input.notes.iter().find(|n| n.entity_key == endpoint)
                        {
                            if *remaining == 0
                                || consumed.saturating_add(target.source.len()) > max_source_bytes
                            {
                                gaps.push("host-budget-exhausted".into());
                                break 'scan;
                            }
                            *remaining -= 1;
                            consumed += target.source.len();
                            let projection = project(&target.source, schema);
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
                        gaps.push("related endpoint qualification unavailable".into());
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
            Some(Cell::Invalid { .. } | Cell::Unavailable { .. }) => {
                gaps.push("related assertion undecided".into())
            }
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
        gaps.push("related endpoint unavailable".into());
    }
    if origin.coverage == "partial" || input.coverage == "partial" {
        gaps.push("related scan coverage partial".into());
    }
    Ok((notes, gaps, consumed))
}

fn terminal_env(
    result: &QueryResult,
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
) -> Result<(BTreeMap<String, QType>, QueryRuntime)> {
    let mut vars = result.terminal_schema.clone();
    for (name, shape) in shapes {
        if vars
            .insert(name.clone(), shape_type(schema, shape))
            .is_some()
        {
            return Err(DataError("Parameter collides with terminal column".into()));
        }
    }
    let mut env = query_env(schema)?;
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

fn sort_terminal(
    result: &mut QueryResult,
    order: &[QueryOrder],
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
) -> Result<()> {
    let (vars, env) = terminal_env(result, schema, parameters, shapes)?;
    if !order.is_empty() {
        let compiled_order = order
            .iter()
            .map(|o| {
                if !matches!(o.nulls.as_str(), "first" | "last") {
                    return Err(DataError("Sort nulls must be first/last".into()));
                }
                let c = compile(&o.expr, schema, &vars)?;
                if !orderable(&c.ty) {
                    return Err(DataError(
                        "Sort needs one declared scalar comparator".into(),
                    ));
                }
                Ok(c)
            })
            .collect::<Result<Vec<_>>>()?;
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
                        mark_partial(result, "sort input undecided");
                    }
                    sorted.push((row, keys));
                }
                Err(_) => {
                    mark_partial(result, "host-budget-exhausted");
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
            "native-decoder traversal budget exhausted in consumed field",
        );
    }
    if env
        .provider_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(result, "temporal expansion partial");
    }
    Ok(())
}
fn distinct_terminal(
    mut result: QueryResult,
    keys: Option<&[QueryColumn]>,
    plan: &QueryPlan,
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
) -> Result<QueryResult> {
    let take = plan.take;
    let order = &plan.order;
    let Some(keys) = keys else {
        return Ok(result);
    };
    if keys.is_empty() {
        return Err(DataError(
            "distinct requires explicit typed equality keys".into(),
        ));
    }
    let (vars, env) = terminal_env(&result, schema, parameters, shapes)?;
    let compiled = keys
        .iter()
        .map(|k| compile(&k.expr, schema, &vars))
        .collect::<Result<Vec<_>>>()?;
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
                mark_partial(&mut result, "host-budget-exhausted");
                break;
            }
        };
        if values
            .iter()
            .any(|c| matches!(c, Cell::Invalid { .. } | Cell::Unavailable { .. }))
        {
            mark_partial(&mut result, "distinct key undecided");
            continue;
        }
        let key = match canonical_cells(&values, &compiled) {
            Ok(k) => k,
            Err(e) => {
                mark_partial(&mut result, &e.0);
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
    sort_terminal(&mut result, order, schema, parameters, shapes, remaining)?;
    if env
        .native_budget_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(
            &mut result,
            "native-decoder traversal budget exhausted in consumed field",
        );
    }
    if env
        .provider_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(&mut result, "temporal expansion partial");
    }
    result.total_known = result.rows.len();
    if let Some(t) = take {
        result.limited |= t < result.rows.len();
        result.rows.truncate(t);
    }
    Ok(result)
}

/// Apply the existing CEL expressions to a typed terminal from union/call.
fn transform(
    mut result: QueryResult,
    plan: &QueryPlan,
    schema: &Schema,
    parameters: &BTreeMap<String, Cell>,
    shapes: &BTreeMap<String, Shape>,
    remaining: &mut u64,
    limits: &QueryBudget,
) -> Result<QueryResult> {
    let (mut vars, env) = terminal_env(&result, schema, parameters, shapes)?;
    let unnest = plan
        .unnest
        .as_ref()
        .map(|u| {
            if vars.contains_key(&u.binding) {
                return Err(DataError(
                    "Unnest binding collides with terminal column".into(),
                ));
            }
            let c = compile(&u.array, schema, &vars)?;
            let QType::List(item) = &c.ty else {
                return Err(DataError("Unnest requires array".into()));
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
        return Err(DataError("where must return bool".into()));
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
                    return Err(DataError("Sort nulls must be first/last".into()));
                }
                let c = compile(&o.expr, schema, &vars)?;
                if !orderable(&c.ty) {
                    return Err(DataError(
                        "Sort needs one declared scalar comparator".into(),
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
            if !matches!(a.op.as_str(),"count"|"sum"|"min"|"max"){return Err(DataError("Unknown aggregate".into()));}
            if a.op=="count"{if a.expr.is_some(){return Err(DataError("count takes no expression".into()));}Ok(None)}else{
                let c=compile(a.expr.as_deref().ok_or_else(||DataError("Aggregate expression required".into()))?,schema,&vars)?;
                if a.op=="sum"&&!matches!(&c.ty,QType::Scalar(s)if matches!(s.as_str(),"int"|"decimal"|"exact_decimal")){return Err(DataError("sum needs int or exact decimal".into()));}
                if matches!(a.op.as_str(),"min"|"max")&&!orderable(&c.ty){return Err(DataError("min/max need an explicit scalar comparator".into()));}
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
                return Err(DataError("Duplicate group column".into()));
            }
        }
        (keys, aggs)
    } else {
        (vec![], vec![])
    };
    let mut names = BTreeSet::new();
    for c in &plan.select {
        if c.name.is_empty() || !names.insert(&c.name) {
            return Err(DataError("Duplicate projection column".into()));
        }
    }
    let mut rows = vec![];
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
                    mark_partial(&mut result, "unnest input undecided");
                    continue;
                }
            }
        } else {
            vec![(row.key.clone(), cells)]
        };
        for (key, cells) in expanded {
            if rows.len() >= limits.max_rows {
                mark_partial(&mut result, "host-budget-exhausted");
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
                        mark_partial(&mut result, "filter undecided");
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
                    mark_partial(&mut result, "host-budget-exhausted");
                    break 'input;
                }
            };
            if order
                .iter()
                .any(|c| matches!(c, Cell::Invalid { .. } | Cell::Unavailable { .. }))
            {
                mark_partial(&mut result, "sort input undecided");
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
                mark_partial(&mut result, "group key undecided");
                continue;
            }
            let key = match canonical_cells(&keys, &group_keys) {
                Ok(k) => k,
                Err(e) => {
                    mark_partial(&mut result, &e.0);
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
                        mark_partial(&mut result, "aggregate input undecided");
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
            "native-decoder traversal budget exhausted in consumed field",
        );
    }
    if env
        .provider_partial
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        mark_partial(&mut result, "temporal expansion partial");
    }
    if plan.group.is_some() {
        sort_terminal(
            &mut result,
            &plan.order,
            schema,
            parameters,
            shapes,
            remaining,
        )?;
    }
    result.total_known = result.rows.len();
    let distinct = plan.advanced.as_ref().and_then(|a| a.distinct.as_deref());
    if distinct.is_some() {
        return distinct_terminal(
            result, distinct, plan, schema, parameters, shapes, remaining,
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
                .map_err(|_| DataError("Invalid saved integer".into()))?
                .into(),
            _ => return Err(DataError(
                "Saved Query structure needs native scalar values except typed parameter defaults"
                    .into(),
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
