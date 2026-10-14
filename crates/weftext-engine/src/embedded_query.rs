//! Exact authored Query carrier locations. These are syntax evidence only;
//! Query decoding/execution and Core authorization live in their existing owners.
use crate::{SourceRange, certify_metadata_declaration, source_hash, standard_binding_available};
use serde::{Deserialize, Serialize};
use typst_syntax::{
    LinkedNode, Source, SyntaxKind,
    ast::{self, AstNode},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedQuerySource {
    pub id: String,
    pub label: String,
    pub call_from: usize,
    pub range: SourceRange,
    pub reason: Option<String>,
}

pub fn embedded_query_sources(source: &str) -> Vec<EmbeddedQuerySource> {
    if source.len() > 8 * 1024 * 1024 {
        return vec![];
    }
    let parsed = Source::detached(source);
    let syntax_valid = parsed.root().errors_and_warnings().0.is_empty();
    let binding = standard_binding_available(source, "metadata");
    let hash = source_hash(source);
    let root = LinkedNode::new(parsed.root());
    fn visit(
        node: &LinkedNode<'_>,
        source: &str,
        hash: &str,
        syntax_valid: bool,
        binding: bool,
        out: &mut Vec<EmbeddedQuerySource>,
    ) {
        if node.kind() == SyntaxKind::Markup {
            let children: Vec<_> = node.children().collect();
            for (index, child) in children.iter().enumerate() {
                let Some(call) = child.get().cast::<ast::FuncCall>() else {
                    continue;
                };
                if !call
                    .callee()
                    .to_untyped()
                    .cast::<ast::Ident>()
                    .is_some_and(|i| i.get() == "metadata")
                {
                    continue;
                }
                let Some(label_node) = children
                    .iter()
                    .skip(index + 1)
                    .find(|n| !n.kind().is_trivia())
                else {
                    continue;
                };
                let Some(label) = label_node.get().cast::<ast::Label>() else {
                    continue;
                };
                let label = label.get().to_string();
                if !label.starts_with("weftext-query-") {
                    continue;
                }
                let call_from = child.range().start;
                let from = if source.as_bytes().get(call_from.wrapping_sub(1)) == Some(&b'#') {
                    call_from - 1
                } else {
                    call_from
                };
                let to = label_node.range().end;
                let direct = node.parent().is_none();
                let certified = direct
                    && syntax_valid
                    && binding
                    && certify_metadata_declaration(source, &label)
                        .is_some_and(|r| r.from == call_from && r.to == to);
                let mut range = crate::editor::source_range(source, from..to);
                range.writable = certified;
                let reason = (!certified).then(|| {
                    if !syntax_valid {
                        "源码语法暂不完整；保留原文，修复后重新定位。"
                    } else {
                        "此 Query 不是唯一、直接且已证明绑定的本地声明；保留原文，仅提供只读结果。"
                    }
                    .to_string()
                });
                out.push(EmbeddedQuerySource {
                    id: format!("{hash}:{from}:{to}"),
                    label,
                    call_from,
                    range,
                    reason,
                });
            }
        }
        for child in node.children() {
            visit(&child, source, hash, syntax_valid, binding, out);
        }
    }
    let mut out = vec![];
    visit(&root, source, &hash, syntax_valid, binding, &mut out);
    out
}
