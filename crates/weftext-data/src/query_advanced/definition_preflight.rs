//! Definition-time checks over exact supplied parameters and callee baselines.
//! Empty terminals below supply compiler types only, never evidence about rows.
use super::*;
use crate::query_providers::{ProviderTrace, trace_parameters};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryDefinitionPreflight {
    pub provider_dependencies: QueryProviderDependencies,
    pub deferred_parameters: Vec<String>,
    pub requires_actual_rows: bool,
    pub closed_invocations: Vec<QueryDefinitionInvocation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryDefinitionInvocation {
    pub plan: QueryPlan,
    pub baseline: Schema,
    pub parameters: BTreeMap<String, NativeValue>,
    pub definition: Option<QueryDefinitionInvocationSource>,
    pub schema_baseline: Option<ProviderBaseline>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryDefinitionInvocationSource {
    pub owner: String,
    pub selection: DefinitionSelection,
    pub source_revision: String,
    pub key: String,
}

pub fn preflight_query_definition(
    plan: &QueryPlan,
    schema: &Schema,
    context: &FrozenAdvancedInput,
    budget: &QueryBudget,
) -> Result<QueryDefinitionPreflight> {
    let source_bytes = context
        .definitions
        .iter()
        .flat_map(|owner| &owner.history)
        .try_fold(serde_json::to_vec(plan)?.len(), |bytes, revision| {
            bytes.checked_add(revision.source.len())
        })
        .ok_or_else(|| {
            DataError::budget(
                weftext_messages::UiMessage::error_data_definition_preflight_host_budget_exhausted(
                ),
            )
        })?;
    if source_bytes > budget.max_source_bytes {
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_definition_preflight_host_budget_exhausted(),
        ));
    }
    validate_query_plan_with_context(plan, schema, context)?;
    let (parameters, _) = parameter_bindings(plan, schema, &context.parameters, false)?;
    let deferred_parameters = parameters
        .iter()
        .filter(|(_, value)| matches!(value, Cell::Absent))
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let trace = ProviderTrace::new();
    let mut preflight = Preflight {
        context,
        budget,
        trace: trace.clone(),
        remaining: budget.max_steps,
        nodes: 0,
        requires_actual_rows: false,
        closed_invocations: vec![],
    };
    preflight.visit(
        plan,
        schema,
        &context.parameters,
        &BTreeSet::new(),
        0,
        None,
        None,
    )?;
    if preflight.closed_invocations.len() > MAX_PLAN_NODES
        || serde_json::to_vec(&preflight.closed_invocations)?.len() > budget.max_source_bytes
    {
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_definition_preflight_host_budget_exhausted(),
        ));
    }
    Ok(QueryDefinitionPreflight {
        provider_dependencies: trace.finish()?,
        deferred_parameters,
        requires_actual_rows: preflight.requires_actual_rows,
        closed_invocations: preflight.closed_invocations,
    })
}

struct Preflight<'a> {
    context: &'a FrozenAdvancedInput,
    budget: &'a QueryBudget,
    trace: ProviderTrace,
    remaining: u64,
    nodes: usize,
    requires_actual_rows: bool,
    closed_invocations: Vec<QueryDefinitionInvocation>,
}
impl Preflight<'_> {
    #[allow(clippy::too_many_arguments)]
    fn visit(
        &mut self,
        plan: &QueryPlan,
        schema: &Schema,
        values: &BTreeMap<String, NativeValue>,
        unknown: &BTreeSet<String>,
        depth: usize,
        baseline: Option<ProviderBaseline>,
        definition_source: Option<QueryDefinitionInvocationSource>,
    ) -> Result<QueryResult> {
        let outer_rows = self.requires_actual_rows;
        self.requires_actual_rows = false;
        let invocation_start = self.closed_invocations.len();
        self.nodes += 1;
        if depth > MAX_CALL_DEPTH || self.nodes > MAX_PLAN_NODES || self.remaining == 0 {
            return Err(DataError::budget(
                weftext_messages::UiMessage::error_data_definition_preflight_host_budget_exhausted(
                ),
            ));
        }
        self.remaining -= 1;
        let (mut parameters, shapes) = parameter_bindings(plan, schema, values, false)?;
        for name in unknown {
            if let Some(value) = parameters.get_mut(name) {
                *value = Cell::Absent;
            }
        }
        if depth > 0
            && parameters
                .iter()
                .any(|(name, value)| matches!(value, Cell::Absent) && !unknown.contains(name))
        {
            return Err(DataError::new(weftext_messages::UiMessage::error_data_definition_preflight_required_nested_query_parameter_missing()));
        }
        let trace = self.trace.at(baseline.clone());
        let mut trace_plan = plan.clone();
        if let Some(advanced) = &mut trace_plan.advanced {
            for parameter in &mut advanced.parameters {
                if unknown.contains(&parameter.name) {
                    parameter.default = None;
                }
            }
        }
        trace_parameters(&trace_plan, schema, values, &trace, &mut self.remaining)?;
        let advanced = plan.advanced.clone().unwrap_or_default();
        advanced.validate_union()?;
        let mut terminal_inputs = None;
        let mut producer_bound = true;
        let mut result = if let Some(call) = &advanced.call {
            let (revision, definition) = selected(call, &self.context.definitions)?;
            let baseline_schema = exact_definition_schema(self.context, &definition, schema)?;
            let target = definition
                .plan
                .advanced
                .as_ref()
                .map_or(&[][..], |advanced| advanced.parameters.as_slice());
            let vars = shapes
                .iter()
                .map(|(name, shape)| (name.clone(), shape_type(schema, shape)))
                .collect();
            let mut env = query_env_traced(schema, Some(&trace))?;
            env.tag_structure = self.context.tag_structure.clone();
            env.parameter_shapes = shapes.clone();
            let mut args = BTreeMap::new();
            let mut undecided = BTreeSet::new();
            for (name, expression) in &call.arguments {
                let compiled = compile(expression, schema, &vars)?;
                self.requires_actual_rows |= expression_requires_rows(&compiled, &shapes);
                if let Some(cell) = preflight_definition_expression(
                    &compiled,
                    &parameters,
                    schema,
                    &mut self.remaining,
                    &env,
                )? {
                    let value = match cell {
                        Cell::Value { value } => value,
                        Cell::Null => NativeValue::None,
                        _ => unreachable!(),
                    };
                    let parameter = target
                        .iter()
                        .find(|parameter| &parameter.name == name)
                        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_definition_preflight_unknown_callee_parameter()))?;
                    validate_parameter(baseline_schema, parameter, &value)?;
                    args.insert(name.clone(), value);
                } else {
                    undecided.insert(name.clone());
                }
            }
            let child = self.visit(
                &definition.plan,
                baseline_schema,
                &args,
                &undecided,
                depth + 1,
                Some(ProviderBaseline {
                    schema: definition.binding.schema.clone(),
                    revision: definition.binding.revision.clone(),
                }),
                Some(QueryDefinitionInvocationSource {
                    owner: call.owner.clone(),
                    selection: call.selection.clone(),
                    source_revision: revision.revision.clone(),
                    key: definition.key.clone(),
                }),
            )?;
            producer_bound = undecided.is_empty();
            terminal_inputs = Some(child.terminal_schema.clone());
            self.terminal(child, plan, schema, &parameters, &shapes, &trace)?
        } else if advanced.union_all.is_some() {
            let mut first = None;
            for branch in advanced.union_branches() {
                let (values, unknown) = inherited_parameters(branch, &parameters);
                producer_bound &= unknown.is_empty();
                let branch = self.visit(
                    branch,
                    schema,
                    &values,
                    &unknown,
                    depth + 1,
                    baseline.clone(),
                    definition_source.clone(),
                )?;
                if first.is_none() {
                    first = Some(branch);
                }
            }
            let input = first.unwrap();
            terminal_inputs = Some(input.terminal_schema.clone());
            self.terminal(input, plan, schema, &parameters, &shapes, &trace)?
        } else {
            if let Some(related) = &advanced.related {
                self.requires_actual_rows = true;
                let (values, unknown) = inherited_parameters(&related.from, &parameters);
                producer_bound = unknown.is_empty();
                self.visit(
                    &related.from,
                    schema,
                    &values,
                    &unknown,
                    depth + 1,
                    baseline.clone(),
                    definition_source.clone(),
                )?;
            }
            let mut leaf = plan.clone();
            leaf.advanced = None;
            if plan.group.is_some() || advanced.distinct.is_some() {
                leaf.order.clear();
            }
            let empty = FrozenQueryInput {
                cut_id: "definition-preflight-types-only".into(),
                schema_revision: "exact-supplied".into(),
                coverage: "complete".into(),
                freshness: "stale".into(),
                notes: vec![],
                advanced: None,
            };
            let mut result = execute_query_leaf_traced(
                &empty,
                &leaf,
                schema,
                self.budget,
                &parameters,
                &shapes,
                &mut self.remaining,
                &BTreeMap::new(),
                Some(&trace),
            )?;
            result.rows.clear();
            if plan.group.is_some() {
                sort_terminal(
                    &mut result,
                    &plan.order,
                    schema,
                    &parameters,
                    &shapes,
                    &mut self.remaining,
                    Some(&trace),
                    self.context.tag_structure.as_ref(),
                )?;
            }
            result = distinct_terminal(
                result,
                advanced.distinct.as_deref(),
                plan,
                schema,
                &parameters,
                &shapes,
                &mut self.remaining,
                Some(&trace),
                self.context.tag_structure.as_ref(),
            )?;
            self.expressions(
                plan,
                schema,
                &parameters,
                &shapes,
                None,
                &result.terminal_schema,
                &trace,
            )?;
            result
        };
        // No row or aggregate result leaves this helper. The terminal is only
        // carried internally to reuse the original type/transform machinery.
        result.rows.clear();
        let requires_rows = self.requires_actual_rows;
        if requires_rows
            && parameters
                .values()
                .all(|value| matches!(value, Cell::Value { .. } | Cell::Null))
        {
            // A fully bound ancestor is one exact original invocation and
            // already covers its descendants. A partially bound ancestor keeps
            // the independently closed callees discovered underneath it.
            self.closed_invocations.truncate(invocation_start);
            self.closed_invocations.push(QueryDefinitionInvocation {
                plan: plan.clone(),
                baseline: schema.clone(),
                parameters: parameter_subset(plan, &parameters),
                definition: definition_source,
                schema_baseline: baseline,
            });
        } else if requires_rows && producer_bound {
            for projection in self.closed_projections(
                plan,
                schema,
                &parameters,
                &shapes,
                terminal_inputs.as_ref(),
                &result.terminal_schema,
            )? {
                self.closed_invocations.push(QueryDefinitionInvocation {
                    parameters: parameter_subset(&projection, &parameters),
                    plan: projection,
                    baseline: schema.clone(),
                    definition: definition_source.clone(),
                    schema_baseline: baseline.clone(),
                });
            }
        }
        self.requires_actual_rows |= outer_rows;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn closed_projections(
        &self,
        plan: &QueryPlan,
        schema: &Schema,
        parameters: &BTreeMap<String, Cell>,
        shapes: &BTreeMap<String, Shape>,
        terminal: Option<&BTreeMap<String, QType>>,
        outputs: &BTreeMap<String, QType>,
    ) -> Result<Vec<QueryPlan>> {
        let mut vars = terminal.cloned().unwrap_or(symbols(schema)?);
        vars.extend(
            shapes
                .iter()
                .map(|(name, shape)| (name.clone(), shape_type(schema, shape))),
        );
        if let Some(unnest) = &plan.unnest {
            let expression = compile(&unnest.array, schema, &vars)?;
            if expression_uses_unknown_parameters(&expression, parameters) {
                return Ok(vec![]);
            }
            if let QType::List(item) = expression.ty {
                vars.insert(unnest.binding.clone(), *item);
            }
        }
        if let Some(predicate) = plan
            .r#where
            .as_ref()
            .filter(|predicate| !predicate.trim().is_empty())
            && expression_uses_unknown_parameters(&compile(predicate, schema, &vars)?, parameters)
        {
            return Ok(vec![]);
        }
        let mut columns = Vec::new();
        for column in &plan.select {
            let expression = compile(&column.expr, schema, &vars)?;
            if !expression_uses_unknown_parameters(&expression, parameters) {
                columns.push(column.clone());
            }
        }
        if plan.select.is_empty() && plan.group.is_none() {
            if let Some(terminal) = terminal {
                columns.extend(terminal.keys().map(|name| QueryColumn {
                    name: name.clone(),
                    expr: name.clone(),
                }));
            } else {
                columns.extend([
                    QueryColumn {
                        name: "title".into(),
                        expr: "title".into(),
                    },
                    QueryColumn {
                        name: "types".into(),
                        expr: "types".into(),
                    },
                ]);
            }
        }
        let mut projection = plan.clone();
        projection.select = columns;
        projection.group = None;
        projection.order.clear();
        projection.take = None;
        if let Some(advanced) = &mut projection.advanced {
            advanced.distinct = None;
            advanced.parameters.retain(|parameter| {
                !parameters
                    .get(&parameter.name)
                    .is_some_and(|value| matches!(value, Cell::Absent))
            });
        }
        let known = |expression: &str,
                     vars: &BTreeMap<String, QType>,
                     parameters: &BTreeMap<String, Cell>|
         -> Result<bool> {
            Ok(!expression_uses_unknown_parameters(
                &compile(expression, schema, vars)?,
                parameters,
            ))
        };
        let mut output_names = projection
            .select
            .iter()
            .map(|column| column.name.clone())
            .collect::<BTreeSet<_>>();
        let mut group_bound = true;
        if let Some(group) = &plan.group {
            group_bound = group.keys.iter().try_fold(true, |bound, column| {
                Ok::<_, DataError>(bound && known(&column.expr, &vars, parameters)?)
            })?;
            if group_bound {
                let mut kept = group.clone();
                kept.aggregates = group
                    .aggregates
                    .iter()
                    .filter_map(|aggregate| {
                        match aggregate
                            .expr
                            .as_ref()
                            .map(|expression| known(expression, &vars, parameters))
                            .transpose()
                        {
                            Ok(Some(false)) => None,
                            Ok(_) => Some(Ok(aggregate.clone())),
                            Err(error) => Some(Err(error)),
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                output_names = kept
                    .keys
                    .iter()
                    .map(|column| column.name.clone())
                    .chain(
                        kept.aggregates
                            .iter()
                            .map(|aggregate| aggregate.name.clone()),
                    )
                    .collect();
                projection.group = Some(kept);
            } else {
                // Group membership controlled by an unknown parameter is
                // deferred. Its independent per-row operands still execute at
                // the original stage before grouping, over actual input rows.
                let expressions = group.keys.iter().map(|column| column.expr.as_str()).chain(
                    group
                        .aggregates
                        .iter()
                        .filter_map(|aggregate| aggregate.expr.as_deref()),
                );
                for expression in expressions {
                    if known(expression, &vars, parameters)? {
                        push_operand(&mut projection.select, expression);
                    }
                }
                output_names.clear();
            }
        }
        // An empty select at a producer terminal means passthrough. A leaf
        // global group otherwise uses a constant row carrier solely to retain
        // the real original group/aggregate machinery, never a synthetic row.
        if projection.select.is_empty() {
            if let Some(inputs) = terminal {
                projection
                    .select
                    .extend(inputs.keys().map(|name| QueryColumn {
                        name: name.clone(),
                        expr: name.clone(),
                    }));
            } else {
                projection.select.push(QueryColumn {
                    name: "__preflight_row".into(),
                    expr: "1".into(),
                });
            }
        }
        let mut output_parameters = parameters.clone();
        for name in outputs.keys().filter(|name| !output_names.contains(*name)) {
            output_parameters.insert(name.clone(), Cell::Absent);
        }
        let mut output_vars = outputs.clone();
        output_vars.extend(
            shapes
                .iter()
                .map(|(name, shape)| (name.clone(), shape_type(schema, shape))),
        );
        let mut distinct_bound = true;
        let mut extra = Vec::new();
        if let Some(keys) = plan
            .advanced
            .as_ref()
            .and_then(|advanced| advanced.distinct.as_ref())
        {
            distinct_bound = group_bound
                && keys.iter().try_fold(true, |bound, column| {
                    Ok::<_, DataError>(
                        bound && known(&column.expr, &output_vars, &output_parameters)?,
                    )
                })?;
            if distinct_bound {
                projection.advanced.as_mut().unwrap().distinct = Some(keys.clone());
            } else if group_bound {
                let independent = keys
                    .iter()
                    .filter_map(|column| {
                        match known(&column.expr, &output_vars, &output_parameters) {
                            Ok(true) => Some(Ok(column.clone())),
                            Ok(false) => None,
                            Err(error) => Some(Err(error)),
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                if !independent.is_empty() {
                    // Distinct keys are eagerly evaluated before choosing a
                    // representative. Replay independent keys over the exact
                    // pre-distinct terminal, with the original executor.
                    extra.push(QueryPlan {
                        select: independent,
                        advanced: Some(QueryAdvanced {
                            parameters: projection
                                .advanced
                                .as_ref()
                                .map_or(vec![], |advanced| advanced.parameters.clone()),
                            union_all: Some(vec![projection.clone()]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                }
            }
        }
        if group_bound && distinct_bound {
            let (order_vars, order_parameters) = if plan.group.is_some()
                || plan
                    .advanced
                    .as_ref()
                    .is_some_and(|advanced| advanced.distinct.is_some())
            {
                (&output_vars, &output_parameters)
            } else {
                (&vars, parameters)
            };
            projection.order = plan
                .order
                .iter()
                .filter_map(
                    |order| match known(&order.expr, order_vars, order_parameters) {
                        Ok(true) => Some(Ok(order.clone())),
                        Ok(false) => None,
                        Err(error) => Some(Err(error)),
                    },
                )
                .collect::<Result<Vec<_>>>()?;
        }
        // This is an evidence projection of exact authored expressions. It is
        // never serialized back into the author's saved Query definition.
        extra.insert(0, projection);
        Ok(extra)
    }

    fn terminal(
        &mut self,
        input: QueryResult,
        plan: &QueryPlan,
        schema: &Schema,
        parameters: &BTreeMap<String, Cell>,
        shapes: &BTreeMap<String, Shape>,
        trace: &ProviderTrace,
    ) -> Result<QueryResult> {
        let inputs = input.terminal_schema.clone();
        let mut typed_plan = plan.clone();
        typed_plan.order.clear();
        if let Some(advanced) = &mut typed_plan.advanced {
            advanced.distinct = None;
        }
        let mut result = transform(
            input,
            &typed_plan,
            schema,
            parameters,
            shapes,
            &mut self.remaining,
            self.budget,
            Some(trace),
            self.context.tag_structure.as_ref(),
        )?;
        // A global group may generate an empty-input count row. It is a type
        // witness only and must not select a provider or validate a runtime row.
        result.rows.clear();
        self.expressions(
            plan,
            schema,
            parameters,
            shapes,
            Some(&inputs),
            &result.terminal_schema,
            trace,
        )?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn expressions(
        &mut self,
        plan: &QueryPlan,
        schema: &Schema,
        parameters: &BTreeMap<String, Cell>,
        shapes: &BTreeMap<String, Shape>,
        terminal: Option<&BTreeMap<String, QType>>,
        outputs: &BTreeMap<String, QType>,
        trace: &ProviderTrace,
    ) -> Result<()> {
        let mut vars = terminal.cloned().unwrap_or(symbols(schema)?);
        vars.extend(
            shapes
                .iter()
                .map(|(name, shape)| (name.clone(), shape_type(schema, shape))),
        );
        let mut env = query_env_traced(schema, Some(trace))?;
        env.tag_structure = self.context.tag_structure.clone();
        env.parameter_shapes = shapes.clone();
        if let Some(unnest) = &plan.unnest {
            let compiled =
                self.expression(&unnest.array, schema, parameters, shapes, &vars, &env)?;
            if let QType::List(item) = compiled.ty {
                vars.insert(unnest.binding.clone(), *item);
            }
        }
        for expression in plan
            .r#where
            .iter()
            .filter(|expression| !expression.trim().is_empty())
            .chain(plan.select.iter().map(|column| &column.expr))
        {
            self.expression(expression, schema, parameters, shapes, &vars, &env)?;
        }
        if plan.select.is_empty() && terminal.is_none() && plan.group.is_none() {
            self.requires_actual_rows = true;
        }
        if let Some(group) = &plan.group {
            self.requires_actual_rows = true;
            for expression in group.keys.iter().map(|column| &column.expr).chain(
                group
                    .aggregates
                    .iter()
                    .filter_map(|aggregate| aggregate.expr.as_ref()),
            ) {
                self.expression(expression, schema, parameters, shapes, &vars, &env)?;
            }
        }
        self.requires_actual_rows |= !plan.types.is_empty() || plan.scope.directory.is_some();
        let mut output_vars = outputs.clone();
        output_vars.extend(
            shapes
                .iter()
                .map(|(name, shape)| (name.clone(), shape_type(schema, shape))),
        );
        let advanced = plan.advanced.as_ref();
        let order_vars = if plan.group.is_some()
            || advanced.is_some_and(|advanced| advanced.distinct.is_some())
        {
            &output_vars
        } else {
            &vars
        };
        for order in &plan.order {
            self.expression(&order.expr, schema, parameters, shapes, order_vars, &env)?;
        }
        if let Some(keys) = advanced.and_then(|advanced| advanced.distinct.as_ref()) {
            for column in keys {
                self.expression(&column.expr, schema, parameters, shapes, &output_vars, &env)?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn expression(
        &mut self,
        expression: &str,
        schema: &Schema,
        parameters: &BTreeMap<String, Cell>,
        shapes: &BTreeMap<String, Shape>,
        vars: &BTreeMap<String, QType>,
        env: &QueryRuntime,
    ) -> Result<Compiled> {
        let compiled = compile(expression, schema, vars)?;
        self.requires_actual_rows |= expression_requires_rows(&compiled, shapes);
        preflight_definition_expression(&compiled, parameters, schema, &mut self.remaining, env)?;
        Ok(compiled)
    }
}

fn inherited_parameters(
    plan: &QueryPlan,
    parameters: &BTreeMap<String, Cell>,
) -> (BTreeMap<String, NativeValue>, BTreeSet<String>) {
    let known = parameter_subset(plan, parameters);
    let unknown = plan
        .advanced
        .as_ref()
        .into_iter()
        .flat_map(|advanced| &advanced.parameters)
        .filter(|parameter| {
            parameters
                .get(&parameter.name)
                .is_some_and(|value| matches!(value, Cell::Absent))
        })
        .map(|parameter| parameter.name.clone())
        .collect();
    (known, unknown)
}

fn push_operand(columns: &mut Vec<QueryColumn>, expression: &str) {
    let mut index = columns.len();
    let name = loop {
        let name = format!("__preflight_operand_{index}");
        if columns.iter().all(|column| column.name != name) {
            break name;
        }
        index += 1;
    };
    columns.push(QueryColumn {
        name,
        expr: expression.into(),
    });
}
