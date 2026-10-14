//! Conservative native citation declarations and authored cite slots. Ref syntax
//! (`@key`) deliberately remains unclassified until an official compile resolves it.
use crate::{EngineError, SourceRange, certify_metadata_declaration, standard_binding_available};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use typst_syntax::{
    LinkedNode, Source, SyntaxNode,
    ast::{self, AstNode},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CitationEntry {
    pub key: String,
    pub work: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationDeclaration {
    pub entries: Vec<CitationEntry>,
    pub range: SourceRange,
    pub work_slots: Vec<CitationWorkSlot>,
}

/// Exact static native string slots are scoped to the inspected source hash.
/// Copy producers may rewrite these proved values using their fresh Node map;
/// citation keys and lookalike strings elsewhere are deliberately absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationWorkSlot {
    pub key: String,
    pub work: String,
    pub range: SourceRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCitationOccurrence {
    pub key: String,
    pub range: SourceRange,
}

pub fn valid_citation_key(key: &str) -> bool {
    key.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

/// None means the owner contains no citation-role label; a present but
/// unqualified role returns an explicit error, never a partial declaration.
pub fn inspect_citation_declaration(
    source: &str,
) -> Result<Option<CitationDeclaration>, EngineError> {
    if source.len() > 8 * 1024 * 1024 {
        return Err(EngineError(
            "citation-budget: owner exceeds citation parser budget".into(),
        ));
    }
    let parsed = Source::detached(source);
    fn contains(node: &SyntaxNode) -> bool {
        node.cast::<ast::Label>()
            .is_some_and(|label| label.get() == "weftext-citations")
            || node.children().any(contains)
    }
    if !contains(parsed.root()) {
        return Ok(None);
    }
    let unavailable = || {
        EngineError(
            "citation-declaration: unique direct static standard metadata declaration required"
                .into(),
        )
    };
    let range =
        certify_metadata_declaration(source, "weftext-citations").ok_or_else(unavailable)?;
    if !standard_binding_available(source, "metadata") {
        return Err(unavailable());
    }
    let root = LinkedNode::new(parsed.root());
    let call = root
        .children()
        .find(|n| n.range().start == range.from)
        .and_then(|n| n.get().cast::<ast::FuncCall>())
        .ok_or_else(unavailable)?;
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(expr)] = args.as_slice() else {
        return Err(unavailable());
    };
    let dict = expr
        .to_untyped()
        .cast::<ast::Dict>()
        .ok_or_else(unavailable)?;
    let fields = dict_fields(dict).ok_or_else(unavailable)?;
    if fields.len() != 2
        || fields
            .iter()
            .find(|(k, _)| k == "kind")
            .and_then(|(_, e)| e.to_untyped().cast::<ast::Str>())
            .is_none_or(|s| s.get() != "weftext.citations")
    {
        return Err(unavailable());
    }
    let array = fields
        .iter()
        .find(|(k, _)| k == "entries")
        .and_then(|(_, e)| e.to_untyped().cast::<ast::Array>())
        .ok_or_else(unavailable)?;
    let mut entries = Vec::new();
    let mut work_slots = Vec::new();
    let mut keys = BTreeSet::new();
    for item in array.items() {
        if entries.len() >= 4096 {
            return Err(EngineError(
                "citation-budget: too many declared entries".into(),
            ));
        }
        let ast::ArrayItem::Pos(expr) = item else {
            return Err(unavailable());
        };
        let fields = dict_fields(
            expr.to_untyped()
                .cast::<ast::Dict>()
                .ok_or_else(unavailable)?,
        )
        .ok_or_else(unavailable)?;
        if fields.len() != 2 {
            return Err(unavailable());
        }
        let literal = |name| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .and_then(|(_, e)| e.to_untyped().cast::<ast::Str>())
                .map(|s| s.get().to_string())
        };
        let key = literal("key").ok_or_else(unavailable)?;
        let work = literal("work").ok_or_else(unavailable)?;
        if !valid_citation_key(&key) || !keys.insert(key.clone()) {
            return Err(EngineError(
                "citation-key: invalid or duplicate exact citation key".into(),
            ));
        }
        if !valid_node_ref(&work) {
            return Err(EngineError(
                "citation-work: complete stable Work NodeRef required".into(),
            ));
        }
        let work_expr = fields
            .iter()
            .find(|(name, _)| name == "work")
            .ok_or_else(unavailable)?
            .1;
        let work_range = parsed
            .find(work_expr.to_untyped().span())
            .ok_or_else(unavailable)?
            .range();
        work_slots.push(CitationWorkSlot {
            key: key.clone(),
            work: work.clone(),
            range: crate::editor::source_range(source, work_range),
        });
        entries.push(CitationEntry { key, work });
    }
    Ok(Some(CitationDeclaration {
        entries,
        range,
        work_slots,
    }))
}

pub fn citation_work_slots(source: &str) -> Result<Vec<CitationWorkSlot>, EngineError> {
    Ok(inspect_citation_declaration(source)?
        .map(|role| role.work_slots)
        .unwrap_or_default())
}

fn dict_fields(dict: ast::Dict<'_>) -> Option<Vec<(String, ast::Expr<'_>)>> {
    let mut keys = BTreeSet::new();
    dict.items()
        .map(|item| {
            let (key, expr) = match item {
                ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                ast::DictItem::Keyed(n) => (
                    n.key().to_untyped().cast::<ast::Str>()?.get().to_string(),
                    n.expr(),
                ),
                _ => return None,
            };
            keys.insert(key.clone()).then_some((key, expr))
        })
        .collect()
}

fn valid_node_ref(value: &str) -> bool {
    fn uuid(value: &str) -> bool {
        let bytes = value.as_bytes();
        bytes.len() == 36
            && bytes[14] == b'4'
            && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
            && bytes.iter().enumerate().all(|(i, b)| {
                if matches!(i, 8 | 13 | 18 | 23) {
                    *b == b'-'
                } else {
                    b.is_ascii_digit() || matches!(b, b'a'..=b'f')
                }
            })
    }
    value
        .strip_prefix("weftext-node://")
        .and_then(|v| v.split_once('/'))
        .is_some_and(|(library, node)| uuid(library) && uuid(node))
}

/// Only standard `cite` with a literal first Label yields an authored slot.
/// This is source evidence, not target-specific rendered occurrence evidence.
pub fn proven_static_cites(source: &str) -> Vec<NativeCitationOccurrence> {
    if source.len() > 8 * 1024 * 1024 || !standard_binding_available(source, "cite") {
        return vec![];
    }
    let parsed = Source::detached(source);
    fn walk(node: &LinkedNode<'_>, source: &str, out: &mut Vec<NativeCitationOccurrence>) {
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "cite")
        {
            let args: Vec<_> = call.args().items().collect();
            if let Some(ast::Arg::Pos(expr)) = args.first()
                && let Some(label) = expr.to_untyped().cast::<ast::Label>()
                && valid_citation_key(label.get())
                && args.iter().skip(1).all(|a| matches!(a, ast::Arg::Named(_)))
            {
                out.push(NativeCitationOccurrence {
                    key: label.get().to_string(),
                    range: crate::editor::source_range(source, node.range()),
                });
            }
        }
        for child in node.children() {
            walk(&child, source, out);
        }
    }
    let mut out = vec![];
    walk(&LinkedNode::new(parsed.root()), source, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const WORK: &str =
        "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/8aa10c08-3e7d-4d10-9488-92bc7f393050";
    fn source(entries: &str) -> String {
        format!(
            "#metadata((kind: \"weftext.citations\", entries: ({entries}))) <weftext-citations>\n= Heading <refs>\n@refs #cite(<chen2026>)"
        )
    }
    #[test]
    fn direct_stable_binding_does_not_promote_heading_refs() {
        let source = source(&format!("(key: \"chen2026\", work: \"{WORK}\"),"));
        let declaration = inspect_citation_declaration(&source).unwrap().unwrap();
        assert_eq!(declaration.entries[0].work, WORK);
        assert_eq!(declaration.work_slots[0].range.raw, format!("\"{WORK}\""));
        assert_eq!(
            &source[declaration.work_slots[0].range.from..declaration.work_slots[0].range.to],
            format!("\"{WORK}\"")
        );
        assert_eq!(
            proven_static_cites(&source)
                .iter()
                .map(|c| c.key.as_str())
                .collect::<Vec<_>>(),
            ["chen2026"]
        );
    }
    #[test]
    fn duplicate_dynamic_and_shadowed_roles_reject() {
        let entry = format!("(key: \"k\", work: \"{WORK}\"),");
        assert!(inspect_citation_declaration(&source(&format!("{entry}{entry}"))).is_err());
        assert!(
            inspect_citation_declaration(&format!("{}\n<weftext-citations>", source(&entry)))
                .is_err()
        );
        assert!(
            inspect_citation_declaration(&format!("#let metadata(x) = x\n{}", source(&entry)))
                .is_err()
        );
        assert!(
            inspect_citation_declaration(
                "#metadata((kind: \"weftext.citations\", entries: data)) <weftext-citations>"
            )
            .is_err()
        );
        assert!(inspect_citation_declaration(&format!("#context [{}]", source(&entry))).is_err());
        assert!(inspect_citation_declaration("@refs").unwrap().is_none());
        assert!(
            inspect_citation_declaration(&source(""))
                .unwrap()
                .unwrap()
                .entries
                .is_empty()
        );
    }
}
