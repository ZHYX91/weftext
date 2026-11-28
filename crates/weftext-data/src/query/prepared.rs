//! Row-independent CEL lowering and resource accounting.
use super::*;
use cel::common::ast::CallExpr;

pub(super) struct PreparedExpression {
    pub(super) ast: IdedExpr,
    inputs: BTreeSet<String>,
    functions: BTreeSet<String>,
    bindings: Vec<(String, Binding)>,
    cost: ExpressionCost,
}

enum Binding {
    State {
        field: String,
        expected: &'static str,
    },
    Constant {
        value: NativeValue,
        semantic: &'static str,
    },
}

/// Every node contributes the collection bound raised to the number of
/// comprehensions enclosing it (including itself). Count these once, before
/// lowering, so host rewrites cannot alter the user's evaluation budget.
#[derive(Default)]
struct ExpressionCost {
    nodes_by_depth: Vec<u64>,
    largest_literal: u64,
}

impl ExpressionCost {
    fn record(&mut self, ast: &IdedExpr, depth: usize) -> Result<()> {
        let depth = depth + usize::from(matches!(ast.expr, Expr::Comprehension(_)));
        if depth >= self.nodes_by_depth.len() {
            self.nodes_by_depth.resize(depth + 1, 0);
        }
        self.nodes_by_depth[depth] += 1;
        let literal = match &ast.expr {
            Expr::Map(m) => m.entries.len(),
            Expr::List(l) => l.elements.len(),
            _ => 0,
        };
        self.largest_literal = self.largest_literal.max(literal as u64);
        visit_children(ast, |child| self.record(child, depth))
    }

    fn for_row(&self, cells: &BTreeMap<String, Cell>) -> u64 {
        if self.nodes_by_depth.len() == 1 {
            return self.nodes_by_depth[0];
        }
        let bound = cells
            .values()
            .filter_map(Cell::native)
            .map(max_collection)
            .max()
            .unwrap_or(1) as u64;
        let bound = bound.max(self.largest_literal).max(1);
        self.nodes_by_depth.iter().rev().fold(0_u64, |sum, count| {
            sum.saturating_mul(bound).saturating_add(*count)
        })
    }
}

impl PreparedExpression {
    pub(super) fn new(ast: &IdedExpr, schema: &Schema) -> Result<Self> {
        let mut cost = ExpressionCost::default();
        cost.record(ast, 0)?;
        let mut prepared = Self {
            ast: IdedExpr::default(),
            inputs: BTreeSet::new(),
            functions: BTreeSet::new(),
            bindings: Vec::new(),
            cost,
        };
        let mut lowered = ast.clone();
        prepared.lower(&mut lowered, schema)?;
        prepared.ast = lowered;
        Ok(prepared)
    }

    fn binding(&mut self, binding: Binding) -> Expr {
        let name = format!("@weftext_binding_{}", self.bindings.len());
        self.bindings.push((name.clone(), binding));
        Expr::Ident(name)
    }

    fn lower(&mut self, ast: &mut IdedExpr, schema: &Schema) -> Result<()> {
        if let Expr::Call(call) = &ast.expr {
            let func = call.func_name.as_str();
            if func == "field" {
                let field = field_name(ast, schema).ok_or_else(|| {
                    DataError::new(UiMessage::error_data_query_field_unresolved())
                })?;
                self.inputs.insert(field.clone());
                ast.expr = Expr::Ident(field);
                return Ok(());
            }
            let expected = match func {
                "isAbsent" => Some("absent"),
                "isNull" => Some("null"),
                "isInvalid" => Some("invalid"),
                "isUnavailable" => Some("unavailable"),
                _ => None,
            };
            if let Some(expected) = expected {
                let field = field_name(&call.args[0], schema).ok_or_else(|| {
                    DataError::new(UiMessage::error_data_query_field_unresolved())
                })?;
                ast.expr = self.binding(Binding::State { field, expected });
                return Ok(());
            }
            if let Some(constructor) = TypedConstructor::named(func) {
                let literal = literal_text(&call.args[0]).expect("typechecked constant literal");
                ast.expr = self.binding(Binding::Constant {
                    semantic: constructor.semantic,
                    value: constructor.value(literal)?,
                });
                return Ok(());
            }
            self.functions.insert(call.func_name.clone());
        }
        if let Expr::Ident(name) = &ast.expr {
            // Alpha-renamed macro locals can never refer to an input cell.
            if !name.starts_with("@weftext_local_") {
                self.inputs.insert(name.clone());
            }
        }
        visit_children_mut(ast, |child| self.lower(child, schema))?;
        if let Expr::Map(map) = &mut ast.expr {
            let mut args = Vec::with_capacity(map.entries.len() * 2);
            for entry in std::mem::take(&mut map.entries) {
                let EntryExpr::MapEntry(entry) = entry.expr else {
                    return Err(DataError::new(
                        UiMessage::error_data_query_record_requires_static_string_keys(),
                    ));
                };
                args.push(entry.key);
                args.push(entry.value);
            }
            let name = "@weftext_ordered_record";
            self.functions.insert(name.into());
            ast.expr = Expr::Call(CallExpr {
                func_name: name.into(),
                target: None,
                args,
            });
        }
        Ok(())
    }

    pub(super) fn direct_input(&self) -> Option<&str> {
        match &self.ast.expr {
            Expr::Ident(name) if self.inputs.contains(name) => Some(name),
            _ => None,
        }
    }

    pub(super) fn inputs(&self) -> &BTreeSet<String> {
        &self.inputs
    }

    pub(super) fn cost(&self, cells: &BTreeMap<String, Cell>) -> u64 {
        self.cost.for_row(cells)
    }

    pub(super) fn uses_function(&self, name: &str) -> bool {
        self.functions.contains(name)
    }

    pub(super) fn reads_native_budget(&self, cells: &BTreeMap<String, Cell>) -> bool {
        // State predicates also observe decode failures. CEL short-circuiting
        // cannot turn incomplete source coverage into a complete result.
        let has_failure = |field: &String| cells.get(field).is_some_and(Cell::is_native_budget);
        self.inputs.iter().any(has_failure) || self.bindings.iter().any(|(_, binding)| {
            matches!(binding, Binding::State { field, .. } if has_failure(field))
        })
    }

    pub(super) fn bind(
        &self,
        context: &mut Context,
        cells: &BTreeMap<String, Cell>,
        failures: &HostFailures,
    ) {
        for (name, binding) in &self.bindings {
            let value: Box<dyn Val> = match binding {
                Binding::Constant { value, semantic } => {
                    Box::new(ExactAtom::new(value.clone(), semantic, failures))
                }
                Binding::State { field, expected } => {
                    let state = cells.get(field).map(Cell::state).unwrap_or("absent");
                    Box::new(CelBool::from(state == *expected))
                }
            };
            context.add_variable_as_val(name, value);
        }
    }
}

pub(crate) fn visit_children(
    ast: &IdedExpr,
    mut f: impl FnMut(&IdedExpr) -> Result<()>,
) -> Result<()> {
    match &ast.expr {
        Expr::Call(c) => {
            if let Some(t) = &c.target {
                f(t)?;
            }
            for a in &c.args {
                f(a)?;
            }
        }
        Expr::Select(s) => f(&s.operand)?,
        Expr::List(l) => {
            for e in &l.elements {
                f(e)?;
            }
        }
        Expr::Map(m) => {
            for e in &m.entries {
                if let EntryExpr::MapEntry(e) = &e.expr {
                    f(&e.key)?;
                    f(&e.value)?;
                }
            }
        }
        Expr::Comprehension(c) => {
            for e in [
                &c.iter_range,
                &c.accu_init,
                &c.loop_cond,
                &c.loop_step,
                &c.result,
            ] {
                f(e)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_predicates_read_each_row_and_keep_decode_coverage() {
        let schema = Schema::bundled().unwrap();
        let env = query_env_traced(&schema, None).unwrap();
        let query = compile(
            "isUnavailable(field('tasks/due')) || isNull(due)",
            &schema,
            &symbols(&schema).unwrap(),
        )
        .unwrap();
        let failed = Cell::unavailable_group(
            DataErrorKind::NativeBudget,
            UiMessage::error_data_query_host_budget_exhausted(),
            vec![],
        );
        for (cell, expected, partial) in [
            (Cell::Null, true, false),
            (Cell::Absent, false, false),
            (failed, true, true),
        ] {
            let cells = BTreeMap::from([("due".into(), cell)]);
            assert_eq!(
                evaluate(&query, &cells, &schema, &mut 1000, &env).unwrap(),
                Cell::value(NativeValue::Bool(expected))
            );
            assert_eq!(
                env.native_budget_partial.load(AtomicOrdering::Relaxed),
                partial
            );
        }
    }

    #[test]
    fn nested_cost_agrees_with_recursive_source_node_charges_and_saturates() {
        fn reference(ast: &IdedExpr, bound: u64) -> u64 {
            let mut charge = 1_u64;
            visit_children(ast, |child| {
                charge = charge.saturating_add(reference(child, bound));
                Ok(())
            })
            .unwrap();
            if matches!(ast.expr, Expr::Comprehension(_)) {
                charge.saturating_mul(bound)
            } else {
                charge
            }
        }
        let schema = Schema::bundled().unwrap();
        let query = compile(
            "[1,2,3].exists(x, [2,3].exists(y, {'a':x}.a == y))",
            &schema,
            &BTreeMap::new(),
        )
        .unwrap();
        for width in [0, 1, 4, 12] {
            let cells = BTreeMap::from([(
                "extra".into(),
                Cell::value(NativeValue::Array(vec![NativeValue::None; width])),
            )]);
            assert_eq!(
                query.prepared.cost(&cells),
                reference(&query.ast, width.max(3) as u64)
            );
        }
        let source = "true".to_owned();
        let nested = (0..10).fold(source, |inner, i| format!("[1].exists(x{i}, {inner})"));
        let deep = compile(&nested, &schema, &BTreeMap::new()).unwrap();
        let wide = BTreeMap::from([(
            "extra".into(),
            Cell::value(NativeValue::Array(vec![NativeValue::None; 256])),
        )]);
        assert_eq!(deep.prepared.cost(&wide), u64::MAX);
    }

    #[test]
    fn indexed_records_compare_by_key_and_emit_authored_order() {
        let schema = Schema::bundled().unwrap();
        let env = query_env_traced(&schema, None).unwrap();
        let eval = |s| {
            let compiled = compile(s, &schema, &BTreeMap::new()).unwrap();
            evaluate(&compiled, &BTreeMap::new(), &schema, &mut 10000, &env).unwrap()
        };
        assert_eq!(
            eval("{'z': 2, 'a': 9} == {'a': 9, 'z': 2}"),
            Cell::value(NativeValue::Bool(true))
        );
        assert_eq!(
            eval("{'z': 2, 'a': 9} == {'a': 2, 'z': 9}"),
            Cell::value(NativeValue::Bool(false))
        );
        assert_eq!(
            eval("{'z': 2, 'a': 9}.map(k, k)"),
            Cell::value(NativeValue::Array(vec![
                NativeValue::Text("z".into()),
                NativeValue::Text("a".into())
            ]))
        );
    }
}
