//! Weftext's bounded adapter at final native HTML production points.
//! It consumes resolved styles and never evaluates author content again.
use crate::css;
use ecow::{EcoString, eco_format};
use typst_library::foundations::StyleChain;
use typst_library::text::{self, FontStyle, TextElem};
use typst_library::visualize::Paint;
use typst_syntax::ast::AstNode;
use typst_syntax::{LinkedNode, Source, Span, SyntaxKind, ast};

/// Certifies only the application's exact native run-in recipe. The caller
/// also has to observe the corresponding actual native producer and heading
/// tags in the final compilation. This is not a general show-rule interpreter.
pub fn controlled_run_in_producer(
    source: &Source,
    span: Span,
    producer: &str,
) -> Option<EcoString> {
    fn bindings_safe(node: &typst_syntax::SyntaxNode) -> bool {
        const NAMES: [&str; 4] = ["strong", "counter", "heading", "text"];
        if matches!(
            node.kind(),
            SyntaxKind::ModuleImport | SyntaxKind::ModuleInclude
        ) {
            return false;
        }
        if node.cast::<ast::LetBinding>().is_some_and(|binding| {
            binding
                .kind()
                .bindings()
                .iter()
                .any(|ident| NAMES.contains(&ident.get().as_str()))
        }) || node
            .cast::<ast::DestructAssignment>()
            .is_some_and(|binding| {
                binding
                    .pattern()
                    .bindings()
                    .iter()
                    .any(|ident| NAMES.contains(&ident.get().as_str()))
            })
        {
            return false;
        }
        if matches!(node.kind(), SyntaxKind::Params | SyntaxKind::Binary) {
            fn mentions(node: &typst_syntax::SyntaxNode) -> bool {
                node.cast::<ast::Ident>()
                    .is_some_and(|ident| NAMES.contains(&ident.get().as_str()))
                    || node.children().any(mentions)
            }
            // Refuse parameter shadowing and mutations of bindings used by the
            // controlled recipe. Its own `it.numbering != none` is harmless.
            if node.kind() == SyntaxKind::Params && mentions(node) {
                return false;
            }
            if let Some(binary) = node.cast::<ast::Binary>()
                && matches!(
                    binary.op(),
                    ast::BinOp::Assign
                        | ast::BinOp::AddAssign
                        | ast::BinOp::SubAssign
                        | ast::BinOp::MulAssign
                        | ast::BinOp::DivAssign
                )
                && mentions(binary.lhs().to_untyped())
            {
                return false;
            }
        }
        node.children().all(bindings_safe)
    }
    if !bindings_safe(source.root()) {
        return None;
    }
    let produced = source.find(span)?.range();
    let mut node: LinkedNode<'_> = source.find(span)?;
    loop {
        if node.kind() == SyntaxKind::ShowRule {
            let range = node.range();
            let raw = source.text().get(range.clone())?;
            let label = raw.strip_prefix("show <")?.split_once('>')?.0;
            if label.is_empty()
                || !label
                    .chars()
                    .all(|c| c.is_alphanumeric() || "-_:".contains(c) || c == '.')
            {
                return None;
            }
            let expected = format!(
                "show <{label}>: it => context {{\n  strong[\n    #if it.numbering != none {{\n      counter(heading).display(it.numbering)\n      text(\" \")\n    }}\n    #it.body\n  ]\n  text(\" \")\n}}"
            );
            if raw != expected {
                return None;
            }
            let (from, to) = match producer {
                "strong" => (raw.find("strong[")?, raw.find("\n  ]")? + "\n  ]".len()),
                "prefix-separator" => {
                    let text = "text(\" \")";
                    let from = raw.find(text)?;
                    (from, from + text.len())
                }
                "prefix-whitespace" => {
                    let text = "text(\" \")";
                    let from = raw.find(text)? + text.len();
                    (from, raw.find("#it.body")?)
                }
                _ => return None,
            };
            return (produced.start >= range.start + from && produced.end <= range.start + to)
                .then(|| label.into());
        }
        node = node.parent()?.clone();
    }
}

pub(crate) fn text_css(styles: StyleChain) -> css::Properties {
    let names = text::families(styles)
        .map(|name| {
            let escaped: String = name
                .as_str()
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || " -_".contains(c) {
                        c.to_string()
                    } else {
                        format!("\\{:x} ", c as u32)
                    }
                })
                .collect();
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let variant = text::variant(styles);
    let mut css = css::Properties::build(())
        .with("font-size", styles.resolve(TextElem::size))
        .finish();
    css.push("font-family", names);
    // Official conversion has already normalized native Space nodes. Preserve
    // intentional literal spaces/tabs arriving from escapes and raw content.
    css.push("white-space", "pre-wrap");
    css.push("font-weight", eco_format!("{}", variant.weight.to_number()));
    css.push(
        "font-style",
        match variant.style {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
        },
    );
    if let Paint::Solid(color) = styles.get_ref(TextElem::fill) {
        for property in css::Properties::build(())
            .with("color", color.clone())
            .finish()
            .iter()
        {
            css.push(property.name, property.value.clone());
        }
    }
    css
}
