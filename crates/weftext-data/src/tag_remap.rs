//! Exact, pure rewriting of typed TagRef constructors in CEL. Plain strings,
//! even equal URI-looking strings, retain their original source bytes.
use crate::{
    DataError, Result,
    query::{literal_text, visit_children_mut},
};
use cel::{
    Program,
    common::ast::{Expr, IdedExpr},
};
use std::collections::BTreeMap;
use weftext_messages::{DiagnosticOrigin, UiMessage};

pub fn remap_cel_tag_literals(
    source: &str,
    mappings: &BTreeMap<String, String>,
    source_workspace: &str,
) -> Result<String> {
    if source.len() > 16384 {
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_tag_remap_cel_source_budget_exceeded(),
        ));
    }
    let program = Program::compile(source).map_err(|error| {
        DataError::new(UiMessage::error_data_query_cel_syntax_invalid())
            .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
    })?;
    let mut patches = BTreeMap::new();
    fn visit(
        ast: &IdedExpr,
        program: &Program,
        mappings: &BTreeMap<String, String>,
        workspace: &str,
        patches: &mut BTreeMap<(usize, usize), String>,
    ) -> Result<()> {
        if let Expr::Call(call) = &ast.expr
            && call.func_name == "tagRef"
            && call.target.is_none()
            && call.args.len() == 1
            && let Some(reference) = literal_text(&call.args[0])
        {
            if let Some(target) = mappings.get(&reference) {
                let (start, stop) = program
                    .source_info()
                    .offset_for(call.args[0].id)
                    .ok_or_else(|| {
                        DataError::new(weftext_messages::UiMessage::error_data_tag_remap_cel_tag_ref_exact_source_range_missing())
                    })?;
                let source = &program.source_info().source;
                // Parser backends expose token positions. Verify the exact
                // literal against the AST and reject any ambiguous encoding.
                let byte_range = (start as usize, stop as usize + 1);
                let chars: Vec<_> = source
                    .char_indices()
                    .map(|(i, _)| i)
                    .chain(std::iter::once(source.len()))
                    .collect();
                let char_range = chars
                    .get(start as usize)
                    .zip(chars.get(stop as usize + 1))
                    .map(|(a, b)| (*a, *b));
                let mut ranges = std::collections::BTreeSet::new();
                for (from, to) in std::iter::once(byte_range).chain(char_range) {
                    if let Some(raw) = source.get(from..to)
                        && Program::compile(raw)
                            .ok()
                            .and_then(|p| literal_text(p.expression()))
                            .as_deref()
                            == Some(reference.as_str())
                    {
                        ranges.insert((from, to));
                    }
                }
                if ranges.len() != 1 {
                    return Err(DataError::unavailable(
                        weftext_messages::UiMessage::error_data_tag_remap_cel_literal_range_unavailable_or_ambiguous(),
                    ));
                }
                patches.insert(*ranges.first().unwrap(), serde_json::to_string(target)?);
            } else if reference
                .strip_prefix("weftext-tag://")
                .and_then(|r| r.split_once('/'))
                .is_some_and(|(w, _)| w == workspace)
            {
                return Err(DataError::new(
                    weftext_messages::UiMessage::error_data_tag_remap_source_workspace_tag_ref_import_mapping_incomplete(),
                ));
            }
        }
        let mut child = ast.clone();
        visit_children_mut(&mut child, |e| {
            visit(e, program, mappings, workspace, patches)
        })
    }
    visit(
        program.expression(),
        &program,
        mappings,
        source_workspace,
        &mut patches,
    )?;
    let mut after = source.to_string();
    for ((from, to), replacement) in patches.into_iter().rev() {
        after.replace_range(from..to, &replacement);
    }
    Program::compile(&after).map_err(|error| {
        DataError::new(UiMessage::error_data_tag_remap_cel_readback_failed())
            .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
    })?;
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_cel_retains_the_actual_dependency_diagnostic() {
        let source = "'作者😀' + (";
        let Err(original) = Program::compile(source) else {
            panic!("the author expression must be syntactically incomplete")
        };
        let error = remap_cel_tag_literals(source, &BTreeMap::new(), "unrelated").unwrap_err();
        assert_eq!(error.kind(), crate::DataErrorKind::Invalid);
        assert_eq!(
            error.description().explanation,
            UiMessage::error_data_query_cel_syntax_invalid()
        );
        assert_eq!(
            error.description().diagnostics,
            vec![crate::RawDiagnostic {
                origin: DiagnosticOrigin::Dependency,
                text: original.to_string(),
            }]
        );
    }

    #[test]
    fn only_typed_literals_change_even_after_unicode_and_in_macros() {
        let old = "weftext-tag://00000000-0000-4000-8000-000000000001/00000000-0000-4000-8000-000000000002";
        let new = "weftext-tag://00000000-0000-4000-8000-000000000003/00000000-0000-4000-8000-000000000004";
        let expression =
            format!("'中文😀' != '{old}' && [1,2].exists(x, tagInSubtree(tags, tagRef('{old}')))");
        let after = remap_cel_tag_literals(
            &expression,
            &BTreeMap::from([(old.into(), new.into())]),
            "00000000-0000-4000-8000-000000000001",
        )
        .unwrap();
        assert!(after.contains(&format!("!= '{old}'")));
        assert!(after.contains(&format!("tagRef(\"{new}\")")));
        assert_eq!(after.matches(old).count(), 1);
        assert!(
            remap_cel_tag_literals(
                &expression,
                &BTreeMap::new(),
                "00000000-0000-4000-8000-000000000001"
            )
            .is_err()
        );
    }
}
