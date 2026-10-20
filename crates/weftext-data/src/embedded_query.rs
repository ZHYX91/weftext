//! Draft-only adapters for existing native saved Query carriers. No IO, cache,
//! definition persistence, or alternate Query evaluator is introduced here.
use crate::*;
use serde::Serialize;
use typst_syntax::{
    Source, SyntaxKind,
    ast::{self, AstNode},
};

/// Only exact, independently qualified adjacent carriers may exchange places.
/// Intervening prose, comments or rules are barriers, never guessed away.
pub fn arrange_embedded_query(source: &str, id: &str, action: &str) -> Result<FieldPatch> {
    let target = embedded_query(source, id)?;
    if !target.writable {
        return Err(DataError("Query has no unique native writer".into()));
    }
    if action == "remove" {
        return replace_embedded_query_source(source, id, "");
    }
    if action != "up" && action != "down" {
        return Err(DataError("Unknown Query arrangement".into()));
    }
    let mut queries = embedded_queries(source);
    queries.sort_by_key(|q| q.source.range.from);
    let index = queries.iter().position(|q| q.source.id == id).unwrap();
    let other = if action == "up" {
        index.checked_sub(1).and_then(|i| queries.get(i))
    } else {
        queries.get(index + 1)
    }
    .ok_or_else(|| DataError("No adjacent Query in that direction".into()))?;
    if !other.writable {
        return Err(DataError("Adjacent Query has no unique writer".into()));
    }
    let (first, last) = if target.source.range.from < other.source.range.from {
        (&target.source.range, &other.source.range)
    } else {
        (&other.source.range, &target.source.range)
    };
    let separator = &source[first.to..last.from];
    if !separator.chars().all(char::is_whitespace) {
        return Err(DataError(
            "Intervening author content is a Query movement barrier; source retained".into(),
        ));
    }
    let text = format!("{}{}{}", last.raw, separator, first.raw);
    let edited = format!("{}{}{}", &source[..first.from], text, &source[last.to..]);
    Ok(FieldPatch {
        from: first.from,
        to: last.to,
        text,
        source: edited,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedQueryOccurrence {
    #[serde(flatten)]
    pub source: weftext_engine::EmbeddedQuerySource,
    pub key: Option<String>,
    pub kind: String,
    pub writable: bool,
    pub builder_supported: bool,
    pub runnable: bool,
    pub plan: Option<QueryPlan>,
    pub binding: Option<SavedQueryBinding>,
    pub call: Option<QueryCall>,
    pub control_reason: Option<String>,
}

fn decode(source: &weftext_engine::EmbeddedQuerySource) -> Result<SavedQuery> {
    let definitions = saved_queries(&source.range.raw)?;
    let [definition] = definitions.as_slice() else {
        return Err(DataError(
            "Query occurrence is not one direct definition".into(),
        ));
    };
    Ok(definition.clone())
}

fn static_key(raw: &str) -> Option<String> {
    let parsed = Source::detached(raw);
    let call = parsed
        .root()
        .children()
        .find_map(|n| n.cast::<ast::FuncCall>())?;
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(expr)] = args.as_slice() else {
        return None;
    };
    let dict = expr.to_untyped().cast::<ast::Dict>()?;
    dict.items().find_map(|item| match item {
        ast::DictItem::Named(n) if n.name().get() == "key" => n
            .expr()
            .to_untyped()
            .cast::<ast::Str>()
            .map(|s| s.get().to_string()),
        ast::DictItem::Keyed(n)
            if n.key()
                .to_untyped()
                .cast::<ast::Str>()
                .is_some_and(|s| s.get() == "key") =>
        {
            n.expr()
                .to_untyped()
                .cast::<ast::Str>()
                .map(|s| s.get().to_string())
        }
        _ => None,
    })
}

fn supported_builder(plan: &QueryPlan) -> bool {
    plan.unnest.is_none()
        && plan.advanced.as_ref().is_none_or(|a| {
            a.call.is_some()
                && a.parameters.is_empty()
                && a.union_all.is_empty()
                && a.distinct.is_none()
                && a.related.is_none()
        })
}

pub fn embedded_queries(source: &str) -> Vec<EmbeddedQueryOccurrence> {
    let sources = weftext_engine::embedded_query_sources(source);
    let keys: Vec<_> = sources.iter().map(|s| static_key(&s.range.raw)).collect();
    sources
        .into_iter()
        .zip(&keys)
        .map(|(mut source, key)| {
            let duplicate = key.as_ref().is_some_and(|key| {
                keys.iter()
                    .filter(|candidate| candidate.as_ref() == Some(key))
                    .count()
                    != 1
            });
            if duplicate {
                source.range.writable = false;
                source.reason =
                    Some("Query key 在原文中重复，无法取得唯一的定义写入目标；原文已保留。".into());
            }
            let definition = decode(&source);
            let writable = source.range.writable;
            let control_reason = definition
                .as_ref()
                .err()
                .map(ToString::to_string)
                .or_else(|| {
                    definition.as_ref().ok().and_then(|d| {
                        (!supported_builder(&d.plan)).then(|| {
                            "构建器不能完整表示此原生查询；保留原文，请使用明确的源码入口。"
                                .to_string()
                        })
                    })
                });
            let builder_supported = writable && control_reason.is_none();
            let runnable = writable && definition.is_ok();
            let definition = definition.ok();
            let call = definition
                .as_ref()
                .and_then(|d| d.plan.advanced.as_ref())
                .and_then(|a| a.call.clone());
            EmbeddedQueryOccurrence {
                key: key.clone(),
                kind: if call.is_some() { "call" } else { "definition" }.into(),
                writable,
                builder_supported,
                runnable,
                plan: definition.as_ref().map(|d| d.plan.clone()),
                binding: definition.map(|d| d.binding),
                call,
                control_reason,
                source,
            }
        })
        .collect()
}

pub fn embedded_query(source: &str, id: &str) -> Result<EmbeddedQueryOccurrence> {
    embedded_queries(source)
        .into_iter()
        .find(|o| o.source.id == id)
        .ok_or_else(|| DataError("Source basis expired: exact Query occurrence is absent".into()))
}

/// Replace only one current proved source range. Arbitrary source proposals may
/// be temporarily invalid; direct source editing must retain that user input.
pub fn replace_embedded_query_source(
    source: &str,
    id: &str,
    replacement: &str,
) -> Result<FieldPatch> {
    let occurrence = embedded_query(source, id)?;
    if !occurrence.writable {
        return Err(DataError(
            "Query source writer is unavailable; original source retained".into(),
        ));
    }
    let range = occurrence.source.range;
    let edited = format!(
        "{}{}{}",
        &source[..range.from],
        replacement,
        &source[range.to..]
    );
    if edited.len() > crate::native::NATIVE_MAX_BYTES {
        return Err(DataError(
            "Query source proposal exceeds source budget".into(),
        ));
    }
    Ok(FieldPatch {
        from: range.from,
        to: range.to,
        text: replacement.into(),
        source: edited,
    })
}

/// The basic builder changes explicit native value slots and preserves all
/// outside bytes, carrier key/binding, unchanged expressions and comments.
pub fn edit_embedded_query_plan(source: &str, id: &str, plan: &QueryPlan) -> Result<FieldPatch> {
    let occurrence = embedded_query(source, id)?;
    if !occurrence.builder_supported || !supported_builder(plan) {
        return Err(DataError("Query cannot completely round-trip through the basic builder; original source retained".into()));
    }
    let original = occurrence.plan.as_ref().unwrap();
    let before = serde_json::to_value(original)?;
    let after = serde_json::to_value(plan)?;
    let parsed = Source::detached(source);
    let call = parsed
        .root()
        .children()
        .filter_map(|n| n.cast::<ast::FuncCall>())
        .find(|c| {
            parsed
                .find(c.to_untyped().span())
                .is_some_and(|n| n.range().start == occurrence.source.call_from)
        })
        .ok_or_else(|| DataError("Query carrier no longer has its exact AST location".into()))?;
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(expr)] = args.as_slice() else {
        return Err(DataError("Query carrier argument is ambiguous".into()));
    };
    let dict = expr
        .to_untyped()
        .cast::<ast::Dict>()
        .ok_or_else(|| DataError("Query carrier is computed".into()))?;
    let node = crate::native::decode_node(expr.to_untyped(), &parsed);
    let mut changes: Vec<(usize, usize, String)> = vec![];
    let mut additions = vec![];
    for key in [
        "scope", "types", "where", "select", "order", "take", "group", "advanced",
    ] {
        if before.get(key) == after.get(key) {
            continue;
        }
        let encoded = crate::query_advanced::saved_plan_native(
            after.get(key).unwrap_or(&serde_json::Value::Null),
        )?
        .to_typst()?;
        if let Some((_, slot)) = node
            .children
            .iter()
            .find(|(path, _)| matches!(path, ValuePath::Member { key: member } if member == key))
        {
            // Replacing a commented/computed native value would destroy author
            // information the builder cannot represent. Keep it in source.
            let value = parsed.find(expr.to_untyped().span()).unwrap();
            fn comments(n: &typst_syntax::LinkedNode<'_>, from: usize, to: usize) -> bool {
                (n.range().start >= from
                    && n.range().end <= to
                    && matches!(n.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
                    || n.children().any(|c| comments(&c, from, to))
            }
            if slot.value.is_none() || comments(&value, slot.range.from, slot.range.to) {
                return Err(DataError("Changed Query value contains source the builder cannot preserve; original retained".into()));
            }
            changes.push((slot.range.from, slot.range.to, encoded));
        } else if !after.get(key).is_none_or(serde_json::Value::is_null) {
            additions.push(format!("{key}: {encoded}"));
        }
    }
    if !additions.is_empty() {
        let dict_range = parsed.find(dict.to_untyped().span()).unwrap().range();
        let close = dict_range.end - 1;
        let last = dict
            .to_untyped()
            .children()
            .rfind(|n| !n.kind().is_trivia() && n.kind() != SyntaxKind::RightParen)
            .map(|n| n.kind());
        let separator = if last == Some(SyntaxKind::Comma) {
            " "
        } else {
            ", "
        };
        changes.push((close, close, format!("{separator}{}", additions.join(", "))));
    }
    changes.sort_by_key(|c| std::cmp::Reverse(c.0));
    let mut edited = source.to_owned();
    for (from, to, replacement) in changes {
        edited.replace_range(from..to, &replacement);
    }
    let new_sources = embedded_queries(&edited);
    let readback = new_sources
        .iter()
        .find(|o| {
            o.key == occurrence.key && o.source.label == occurrence.source.label && o.writable
        })
        .and_then(|o| o.plan.as_ref())
        .ok_or_else(|| DataError("Builder proposal failed native AST readback".into()))?;
    if serde_json::to_value(readback)? != after {
        return Err(DataError(
            "Builder proposal did not preserve the requested Query plan".into(),
        ));
    }
    let target = occurrence.source.range;
    let suffix = source.len() - target.to;
    let replacement = edited[target.from..edited.len() - suffix].to_string();
    Ok(FieldPatch {
        from: target.from,
        to: target.to,
        text: replacement,
        source: edited,
    })
}
