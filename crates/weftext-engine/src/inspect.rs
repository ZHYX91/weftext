use crate::types::*;
use std::collections::HashSet;
use typst_syntax::{
    LinkedNode, Source, SyntaxKind, SyntaxNode,
    ast::{self, AstNode},
};

/// Inspect the exact original source with the official parser, without evaluating it.
pub fn inspect_metadata(source: &str) -> SourceInfo {
    inspect(source)
}

pub fn inspect(source: &str) -> SourceInfo {
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    let valid = parsed.root().errors_and_warnings().0.is_empty();
    let mut out = SourceInfo {
        syntax_valid: valid,
        identity_status: if valid { "absent" } else { "invalid" }.into(),
        id: None,
        types: vec![],
        metadata: None,
        title: None,
        headings: vec![],
        links: vec![],
        syntax_tokens: vec![],
        body_insertion: None,
        writer_status: "complex-source-readonly".into(),
    };
    walk(&root, source, &mut out);
    let children: Vec<_> = root.children().collect();
    let candidates: Vec<_> = children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            let call = n.get().cast::<ast::FuncCall>()?;
            if call.callee().to_untyped().cast::<ast::Ident>()?.get() != "metadata" {
                return None;
            }
            let label = children
                .iter()
                .skip(i + 1)
                .find(|n| !n.kind().is_trivia())?;
            if label.get().cast::<ast::Label>()?.get() != "weftext" {
                return None;
            }
            Some((i, n, call, label))
        })
        .collect();
    let mut accepted = None;
    if candidates.len() > 1 {
        out.identity_status = "ambiguous".into();
    } else if let Some((i, n, call, label)) = candidates.first() {
        let safe_prefix = children[..*i]
            .iter()
            .all(|n| n.kind() == SyntaxKind::Hash || safe_standard_node(n.get()));
        if valid && safe_prefix {
            let args: Vec<_> = call.args().items().collect();
            if let [ast::Arg::Pos(expr)] = args.as_slice()
                && let Some(dict) = expr.to_untyped().cast::<ast::Dict>()
                && let Some(fields) = metadata_fields(dict, &parsed, source)
            {
                let id = fields
                    .iter()
                    .find(|f| f.key == "id")
                    .and_then(|f| f.value.as_ref())
                    .and_then(|v| v.as_str())
                    .map(str::to_owned);
                let types = fields.iter().find(|f| f.key == "types");
                let types_value = types.map(|f| f.value.as_ref());
                let type_strings = match types_value {
                    None => Some(vec![]),
                    Some(Some(serde_json::Value::Array(a))) => a
                        .iter()
                        .map(|v| v.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                };
                out.metadata = Some(MetadataInfo {
                    from: n.range().start,
                    to: label.range().end,
                    fields,
                });
                if let (Some(id), Some(types)) = (id, type_strings) {
                    if is_uuid_v4(&id) {
                        out.id = Some(id);
                        out.types = types;
                        out.identity_status = "valid".into();
                    } else {
                        out.identity_status = "invalid".into();
                    }
                } else {
                    out.identity_status = "unsupported".into();
                }
                // Metadata's runtime value must also be entirely static before its
                // declaration can be ignored in the simple writer profile.
                if out
                    .metadata
                    .as_ref()
                    .is_some_and(|m| m.fields.iter().all(|f| f.value.is_some()))
                {
                    accepted = Some((n.range().start, label.range().end));
                }
            } else {
                out.identity_status = "unsupported".into();
            }
        } else if valid {
            out.identity_status = "unsupported".into();
        }
    }
    let simple = valid
        && children.iter().all(|n| {
            if accepted.is_some_and(|(a, b)| n.range().start >= a && n.range().end <= b) {
                true
            } else if n.kind() == SyntaxKind::Hash {
                accepted.is_some_and(|(a, _)| n.range().end == a)
                    || children.iter().any(|next| {
                        next.range().start == n.range().end && safe_standard_node(next.get())
                    })
            } else {
                safe_standard_node(n.get())
            }
        });
    if simple {
        // The full simple profile also proves standard title binding and a
        // direct, unique title call. Complex source receives compile metadata.
        let titles: Vec<_> = children
            .iter()
            .filter_map(|n| n.get().cast::<ast::FuncCall>())
            .filter(|c| {
                c.callee()
                    .to_untyped()
                    .cast::<ast::Ident>()
                    .is_some_and(|i| i.get() == "title")
            })
            .collect();
        if let [title] = titles.as_slice() {
            out.title = title.args().items().find_map(|arg| match arg {
                ast::Arg::Pos(expr) => expr
                    .to_untyped()
                    .cast::<ast::ContentBlock>()
                    .map(|b| plain_markup(b.body().to_untyped())),
                _ => None,
            });
        }
        out.writer_status = "unique-literal-profile".into();
        // EOF is a real top-level sibling boundary only with a complete simple
        // markup tree. It is never inside metadata, a code block, or a comment.
        let last = children.last();
        let boundary_safe = last.is_none_or(|n| !matches!(n.kind(), SyntaxKind::LineComment));
        if boundary_safe {
            out.body_insertion = Some(SourceRange {
                from: source.len(),
                to: source.len(),
                utf16_from: source.encode_utf16().count(),
                utf16_to: source.encode_utf16().count(),
                raw: String::new(),
                writable: true,
            });
        }
    }
    out
}

fn metadata_fields(
    dict: ast::Dict<'_>,
    parsed: &Source,
    source: &str,
) -> Option<Vec<MetadataField>> {
    let mut seen = HashSet::new();
    dict.items()
        .map(|item| {
            let (key, value) = match item {
                ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                ast::DictItem::Keyed(n) => (
                    n.key().to_untyped().cast::<ast::Str>()?.get().to_string(),
                    n.expr(),
                ),
                ast::DictItem::Spread(_) => return None,
            };
            if !seen.insert(key.clone()) {
                return None;
            }
            let range = parsed.find(value.to_untyped().span())?.range();
            let projected = static_value(value.to_untyped());
            Some(MetadataField {
                key,
                raw: source.get(range.clone())?.into(),
                from: range.start,
                to: range.end,
                utf16_from: utf16(source, range.start),
                utf16_to: utf16(source, range.end),
                writable: projected.is_some(),
                value: projected,
            })
        })
        .collect()
}

fn static_value(node: &SyntaxNode) -> Option<serde_json::Value> {
    use serde_json::Value;
    match node.kind() {
        SyntaxKind::Str => Some(Value::String(node.cast::<ast::Str>()?.get().to_string())),
        SyntaxKind::Bool => Some(Value::Bool(node.cast::<ast::Bool>()?.get())),
        SyntaxKind::None => Some(Value::Null),
        SyntaxKind::Int => Some(Value::Number(node.cast::<ast::Int>()?.get().into())),
        SyntaxKind::Float => {
            serde_json::Number::from_f64(node.cast::<ast::Float>()?.get()).map(Value::Number)
        }
        SyntaxKind::Array => node
            .cast::<ast::Array>()?
            .items()
            .map(|item| match item {
                ast::ArrayItem::Pos(expr) => static_value(expr.to_untyped()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        SyntaxKind::Dict => {
            let mut result = serde_json::Map::new();
            for item in node.cast::<ast::Dict>()?.items() {
                let (key, value) = match item {
                    ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                    ast::DictItem::Keyed(n) => (
                        n.key().to_untyped().cast::<ast::Str>()?.get().to_string(),
                        n.expr(),
                    ),
                    _ => return None,
                };
                if result
                    .insert(key, static_value(value.to_untyped())?)
                    .is_some()
                {
                    return None;
                }
            }
            Some(Value::Object(result))
        }
        _ => None,
    }
}

/// A small proven subset of standard calls/setters. No bindings, show rules,
/// import/include, or user calls can occur anywhere in this profile.
fn safe_standard_node(node: &SyntaxNode) -> bool {
    if simple_markup(node) {
        return true;
    }
    if let Some(set) = node.cast::<ast::SetRule>() {
        if set.condition().is_some() {
            return false;
        }
        let Some(target) = set.target().to_untyped().cast::<ast::Ident>() else {
            return false;
        };
        if !matches!(target.get().as_str(), "document" | "text" | "heading") {
            return false;
        }
        return set.args().items().all(|arg| match arg {
            ast::Arg::Named(n) => {
                let allowed = match target.get().as_str() {
                    "document" => matches!(
                        n.name().get().as_str(),
                        "title" | "author" | "description" | "keywords"
                    ),
                    "text" => matches!(
                        n.name().get().as_str(),
                        "font" | "lang" | "region" | "weight" | "style"
                    ),
                    "heading" => {
                        matches!(n.name().get().as_str(), "numbering" | "offset" | "outlined")
                    }
                    _ => false,
                };
                allowed
                    && (static_value(n.expr().to_untyped()).is_some()
                        || n.expr()
                            .to_untyped()
                            .cast::<ast::ContentBlock>()
                            .is_some_and(|b| simple_markup(b.body().to_untyped())))
            }
            _ => false,
        });
    }
    if let Some(call) = node.cast::<ast::FuncCall>() {
        if !call
            .callee()
            .to_untyped()
            .cast::<ast::Ident>()
            .is_some_and(|id| id.get() == "title")
        {
            return false;
        }
        let args: Vec<_> = call.args().items().collect();
        return args.is_empty()
            || matches!(args.as_slice(),[ast::Arg::Pos(expr)] if expr.to_untyped().cast::<ast::ContentBlock>().is_some_and(|b|simple_markup(b.body().to_untyped())));
    }
    false
}

/// Certify a unique direct-root application metadata role without evaluating
/// user code. The caller owns decoding and validating the role's value.
pub fn certify_metadata_declaration(source: &str, label_name: &str) -> Option<SourceRange> {
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return None;
    }
    let root = LinkedNode::new(parsed.root());
    let children: Vec<_> = root.children().collect();
    fn label_count(node: &LinkedNode<'_>, name: &str) -> usize {
        // A label value in query(<role>), a show selector, or a variable is
        // not an attachment declaration. Only markup's actual label syntax
        // can compete with the direct-root metadata role here.
        usize::from(
            node.parent_kind() == Some(SyntaxKind::Markup)
                && node
                    .get()
                    .cast::<ast::Label>()
                    .is_some_and(|l| l.get() == name),
        ) + node
            .children()
            .map(|n| label_count(&n, name))
            .sum::<usize>()
    }
    if label_count(&root, label_name) != 1 {
        return None;
    }
    let (index, call, label) = children.iter().enumerate().find_map(|(i, n)| {
        let call = n.get().cast::<ast::FuncCall>()?;
        if !call
            .callee()
            .to_untyped()
            .cast::<ast::Ident>()
            .is_some_and(|i| i.get() == "metadata")
        {
            return None;
        }
        let label = children
            .iter()
            .skip(i + 1)
            .find(|n| !n.kind().is_trivia())?;
        label
            .get()
            .cast::<ast::Label>()
            .filter(|l| l.get() == label_name)?;
        Some((i, n, label))
    })?;
    fn constant(node: &SyntaxNode) -> bool {
        if let Some(dict) = node.cast::<ast::Dict>() {
            let mut seen = HashSet::new();
            return dict.items().all(|item| match item {
                ast::DictItem::Named(n) => {
                    seen.insert(n.name().get().to_string()) && constant(n.expr().to_untyped())
                }
                ast::DictItem::Keyed(n) => {
                    n.key()
                        .to_untyped()
                        .cast::<ast::Str>()
                        .is_some_and(|key| seen.insert(key.get().to_string()))
                        && constant(n.expr().to_untyped())
                }
                _ => false,
            });
        }
        if let Some(array) = node.cast::<ast::Array>() {
            return array
                .items()
                .all(|item| matches!(item,ast::ArrayItem::Pos(e) if constant(e.to_untyped())));
        }
        if let Some(call) = node.cast::<ast::FuncCall>() {
            let mut named = HashSet::new();
            return call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| {
                    matches!(
                        i.get().as_str(),
                        "decimal" | "datetime" | "int" | "duration"
                    )
                })
                && call.args().items().all(|arg| match arg {
                    ast::Arg::Pos(e) => constant(e.to_untyped()),
                    ast::Arg::Named(n) => {
                        named.insert(n.name().get().to_string()) && constant(n.expr().to_untyped())
                    }
                    _ => false,
                });
        }
        if let Some(unary) = node.cast::<ast::Unary>() {
            return matches!(unary.op(), ast::UnOp::Pos | ast::UnOp::Neg)
                && matches!(
                    unary.expr().to_untyped().kind(),
                    SyntaxKind::Int | SyntaxKind::Float | SyntaxKind::Numeric
                );
        }
        if let Some(group) = node.cast::<ast::Parenthesized>() {
            return constant(group.expr().to_untyped());
        }
        matches!(
            node.kind(),
            SyntaxKind::Str
                | SyntaxKind::Bool
                | SyntaxKind::None
                | SyntaxKind::Int
                | SyntaxKind::Float
                | SyntaxKind::Numeric
        )
    }
    let safe_prefix = children[..index].iter().all(|n| {
        if matches!(
            n.kind(),
            SyntaxKind::Hash
                | SyntaxKind::Space
                | SyntaxKind::Parbreak
                | SyntaxKind::LineComment
                | SyntaxKind::BlockComment
        ) {
            return true;
        }
        if let Some(prior) = n.get().cast::<ast::FuncCall>() {
            let static_metadata = prior
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "metadata")
                && prior
                    .args()
                    .items()
                    .all(|a| matches!(a,ast::Arg::Pos(e) if constant(e.to_untyped())));
            if static_metadata {
                return true;
            }
        }
        if let Some(label) = n.get().cast::<ast::Label>() {
            return label.get() != label_name;
        }
        safe_standard_node(n.get())
    });
    if !safe_prefix {
        return None;
    }
    Some(crate::editor::source_range(
        source,
        call.range().start..label.range().end,
    ))
}

pub(crate) fn simple_markup(node: &SyntaxNode) -> bool {
    match node.kind() {
        SyntaxKind::Markup
        | SyntaxKind::Heading
        | SyntaxKind::Strong
        | SyntaxKind::Emph
        | SyntaxKind::ListItem
        | SyntaxKind::EnumItem
        | SyntaxKind::TermItem => node.children().all(simple_markup),
        SyntaxKind::Text
        | SyntaxKind::Space
        | SyntaxKind::Parbreak
        | SyntaxKind::Linebreak
        | SyntaxKind::Escape
        | SyntaxKind::Shorthand
        | SyntaxKind::SmartQuote
        | SyntaxKind::Link
        | SyntaxKind::Label
        | SyntaxKind::HeadingMarker
        | SyntaxKind::ListMarker
        | SyntaxKind::EnumMarker
        | SyntaxKind::TermMarker
        | SyntaxKind::Star
        | SyntaxKind::Underscore
        | SyntaxKind::Colon
        | SyntaxKind::LineComment
        | SyntaxKind::BlockComment => true,
        // Raw/math are legitimate atoms but intentionally outside the first
        // reversible literal profile; their source is always preserved.
        _ => false,
    }
}

fn walk(node: &LinkedNode<'_>, source: &str, out: &mut SourceInfo) {
    let range = node.range();
    if node.kind() != SyntaxKind::Markup && out.syntax_tokens.len() < 100_000 {
        out.syntax_tokens.push(SyntaxToken {
            kind: format!("{:?}", node.kind()),
            from: range.start,
            to: range.end,
            utf16_from: utf16(source, range.start),
            utf16_to: utf16(source, range.end),
        });
    }
    if let Some(heading) = node.get().cast::<ast::Heading>() {
        out.headings.push(HeadingInfo {
            text: plain_markup(heading.body().to_untyped()),
            path: None,
            is_main: true,
            level: heading.depth().get(),
            from: range.start,
            to: range.end,
            utf16_from: utf16(source, range.start),
            utf16_to: utf16(source, range.end),
            generated: false,
            visible: false,
        });
    }
    let target = if let Some(link) = node.get().cast::<ast::Link>() {
        Some((link.get().to_string(), "url"))
    } else if let Some(reference) = node.get().cast::<ast::Ref>() {
        Some((reference.target().to_owned(), "ref"))
    } else if let Some(call) = node.get().cast::<ast::FuncCall>() {
        if call
            .callee()
            .to_untyped()
            .cast::<ast::Ident>()
            .is_some_and(|id| id.get() == "link")
        {
            call.args().items().find_map(|arg| match arg {
                ast::Arg::Pos(expr) => expr
                    .to_untyped()
                    .cast::<ast::Str>()
                    .map(|s| (s.get().to_string(), "unverified-call")),
                _ => None,
            })
        } else {
            None
        }
    } else {
        None
    };
    if let Some((target, kind)) = target {
        out.links.push(LinkInfo {
            target,
            kind: kind.into(),
            from: range.start,
            to: range.end,
            utf16_from: utf16(source, range.start),
            utf16_to: utf16(source, range.end),
        });
    }
    for child in node.children() {
        walk(&child, source, out);
    }
}

pub(crate) fn plain_markup(node: &SyntaxNode) -> String {
    if let Some(t) = node.cast::<ast::Text>() {
        return t.get().to_string();
    }
    if let Some(escape) = node.cast::<ast::Escape>() {
        return escape.get().to_string();
    }
    if let Some(shorthand) = node.cast::<ast::Shorthand>() {
        return shorthand.get().to_string();
    }
    if matches!(node.kind(), SyntaxKind::Space | SyntaxKind::Parbreak) {
        return " ".into();
    }
    node.children().map(plain_markup).collect()
}

fn is_uuid_v4(id: &str) -> bool {
    let bytes = id.as_bytes();
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
