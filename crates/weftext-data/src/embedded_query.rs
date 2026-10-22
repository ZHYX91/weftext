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
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_query_native_writer_not_unique(),
        ));
    }
    if action == "remove" {
        return replace_embedded_query_source(source, id, "");
    }
    if action != "up" && action != "down" {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_unknown_query_arrangement(),
        ));
    }
    let mut queries = embedded_queries(source);
    queries.sort_by_key(|q| q.source.range.from);
    let index = queries.iter().position(|q| q.source.id == id).unwrap();
    let other = if action == "up" {
        index.checked_sub(1).and_then(|i| queries.get(i))
    } else {
        queries.get(index + 1)
    }
    .ok_or_else(|| {
        DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_adjacent_query_absent(),
        )
    })?;
    if !other.writable {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_adjacent_query_writer_not_unique(
            ),
        ));
    }
    let (first, last) = if target.source.range.from < other.source.range.from {
        (&target.source.range, &other.source.range)
    } else {
        (&other.source.range, &target.source.range)
    };
    let separator = &source[first.to..last.from];
    if !separator.chars().all(char::is_whitespace) {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_author_content_blocks_query_move(
            ),
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
    pub control_kind: Option<DataErrorKind>,
    pub control_reason: Option<ErrorDescription>,
    pub control_diagnostics: Vec<ErrorDescription>,
}

fn control_error(error: DataError) -> (DataErrorKind, ErrorDescription, Vec<ErrorDescription>) {
    let (kind, description, details) = error.into_parts();
    (
        kind,
        description,
        details
            .map(DataErrorDetails::into_diagnostics)
            .unwrap_or_default(),
    )
}

fn decode(source: &weftext_engine::EmbeddedQuerySource) -> Result<SavedQuery> {
    let definitions = saved_queries(&source.range.raw)?;
    let [definition] = definitions.as_slice() else {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_query_occurrence_not_direct_definition(),
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
                && a.union_all.is_none()
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
                    Some(weftext_messages::UiMessage::error_data_embedded_query_duplicate_query_key_blocks_writer().into());
            }
            let writable = source.range.writable;
            let (definition, control_kind, control_reason, control_diagnostics) =
                match decode(&source) {
                    Ok(definition) => {
                        let unsupported = !supported_builder(&definition.plan);
                        (
                            Some(definition),
                            unsupported.then_some(DataErrorKind::Unavailable),
                            unsupported.then(|| UiMessage::embedded_query_builder_unsupported().into()),
                            vec![],
                        )
                    }
                    Err(error) => {
                        let (kind, reason, diagnostics) = control_error(error);
                        (None, Some(kind), Some(reason), diagnostics)
                    }
                };
            let builder_supported = writable && control_reason.is_none();
            let runnable = writable && definition.is_some();
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
                control_kind,
                control_reason,
                control_diagnostics,
                source,
            }
        })
        .collect()
}

pub fn embedded_query(source: &str, id: &str) -> Result<EmbeddedQueryOccurrence> {
    embedded_queries(source)
        .into_iter()
        .find(|o| o.source.id == id)
        .ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_embedded_query_query_occurrence_basis_expired(),
            )
        })
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
        return Err(DataError::unavailable(
            weftext_messages::UiMessage::error_data_embedded_query_query_source_writer_unavailable(
            ),
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
        return Err(DataError::budget(
            weftext_messages::UiMessage::error_data_embedded_query_query_source_proposal_exceeds_budget(),
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
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_query_cannot_round_trip_basic_builder(),
        ));
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
        .ok_or_else(|| {
            DataError::new(
                weftext_messages::UiMessage::error_data_embedded_query_query_carrier_ast_location_lost(),
            )
        })?;
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(expr)] = args.as_slice() else {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_query_carrier_argument_ambiguous(
            ),
        ));
    };
    let dict = expr.to_untyped().cast::<ast::Dict>().ok_or_else(|| {
        DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_query_carrier_computed(),
        )
    })?;
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
                return Err(DataError::new(weftext_messages::UiMessage::error_data_embedded_query_query_builder_cannot_preserve_changed_value_source()));
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
        .ok_or_else(|| DataError::new(weftext_messages::UiMessage::error_data_embedded_query_query_builder_native_readback_failed()))?;
    if serde_json::to_value(readback)? != after {
        return Err(DataError::new(
            weftext_messages::UiMessage::error_data_embedded_query_query_builder_plan_readback_mismatch(),
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

#[cfg(test)]
mod control_error_tests {
    use super::*;

    fn query(extra: &str) -> String {
        format!(
            "#metadata((kind: \"weftext.query\", key: \"f8597b15-6b7f-4a6a-9b52-c2732730fe41\", binding: (schema: \"weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema\", revision: \"8a81e35f-ad48-4c12-af4f-3d2607102fa9\", language: \"weftext.query.cel.v1\"), scope: (library: \"current\", lifecycle: \"live\"), types: (\"task\",), /* 作者原文😀 */ where: \"status == 'open'\", select: ((name: \"task\", expr: \"self\"),), {extra})) <weftext-query-one>"
        )
    }

    #[test]
    fn actual_embedded_queries_keep_builder_and_decode_facts_separate_from_explanations() {
        let ordinary = query("");
        let supported = embedded_queries(&ordinary).remove(0);
        assert!(supported.writable && supported.builder_supported && supported.runnable);
        assert!(supported.control_kind.is_none());
        assert!(supported.control_reason.is_none());
        assert!(supported.control_diagnostics.is_empty());
        let wire = serde_json::to_value(&supported).unwrap();
        assert_eq!(wire.get("controlKind"), Some(&serde_json::Value::Null));
        assert_eq!(wire.get("controlReason"), Some(&serde_json::Value::Null));
        assert_eq!(wire["controlDiagnostics"], serde_json::json!([]));

        let advanced = query("advanced: (distinct: ((name: \"task\", expr: \"self\"),)),");
        let unsupported = embedded_queries(&advanced).remove(0);
        assert!(unsupported.writable && unsupported.runnable);
        assert!(!unsupported.builder_supported);
        assert_eq!(unsupported.control_kind, Some(DataErrorKind::Unavailable));
        assert_eq!(
            unsupported.control_reason.as_ref().unwrap().explanation,
            UiMessage::embedded_query_builder_unsupported()
        );
        assert!(unsupported.control_diagnostics.is_empty());
        assert_eq!(unsupported.source.range.raw, advanced);

        let computed = ordinary.replace("where: \"status == 'open'\"", "where: str(1 + 1)");
        let invalid = embedded_queries(&computed).remove(0);
        assert!(invalid.writable);
        assert!(!invalid.runnable && !invalid.builder_supported);
        assert_eq!(invalid.control_kind, Some(DataErrorKind::Invalid));
        assert_eq!(
            invalid.control_reason.as_ref().unwrap().explanation,
            UiMessage::error_data_query_saved_query_computed()
        );
        assert_eq!(invalid.source.range.raw, computed);
        let wire = serde_json::to_value(&invalid).unwrap();
        let reason = invalid.control_reason.as_ref().unwrap();
        let _ = reason.explanation.render(weftext_messages::UiLocale::ZhCn);
        let _ = reason.explanation.render(weftext_messages::UiLocale::EnUs);
        assert_eq!(serde_json::to_value(&invalid).unwrap(), wire);
    }

    #[test]
    fn actual_control_error_bridge_preserves_machine_kind_raw_and_96_parallel_descriptions() {
        let primary = UiMessage::error_data_query_saved_query_computed();
        let parallel = (0..96)
            .map(|index| {
                ErrorDescription::new(UiMessage::error_data_query_saved_query_key_computed())
                    .diagnostic(
                        DiagnosticOrigin::Dependency,
                        format!("original parser {index} 原文😀"),
                    )
            })
            .collect::<Vec<_>>();
        for actual_kind in [DataErrorKind::Invalid, DataErrorKind::NativeBudget] {
            let error = DataError::with_kind(actual_kind, primary.clone())
                .diagnostic(
                    DiagnosticOrigin::Dependency,
                    "original primary parser 原文😀",
                )
                .with_diagnostics(parallel.clone());
            let (kind, description, diagnostics) = control_error(error);
            assert_eq!(kind, actual_kind);
            assert_eq!(description.explanation, primary);
            assert_eq!(
                description.diagnostics[0].text,
                "original primary parser 原文😀"
            );
            assert!(description.causes.as_slice().is_empty());
            assert_eq!(diagnostics, parallel);
            let wire = serde_json::to_value(&diagnostics).unwrap();
            assert_eq!(wire.as_array().unwrap().len(), 96);
            assert_eq!(
                serde_json::from_value::<Vec<ErrorDescription>>(wire).unwrap(),
                parallel
            );
        }
    }
}
