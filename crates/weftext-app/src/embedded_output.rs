//! Materialize only retained, source-bound Query results. No evaluator or writer.
use super::{export_api::*, export_conversions::*, *};
use std::collections::BTreeSet;
use weftext_data::{EmbeddedQueryOccurrence, QueryResult};
use weftext_engine::RenderNode;
use weftext_export::ExportLoss;
use weftext_messages::{UiInteger, UiLocale, UiMessage};

pub(crate) struct FrozenEmbedded {
    pub(crate) occurrence: EmbeddedQueryOccurrence,
    pub(crate) frozen: FrozenQueryOutput,
    pub(crate) result: QueryResult,
}

pub(crate) fn freeze(
    state: &AppState,
    req: &PrepareRequest,
    root: &str,
) -> Result<Vec<FrozenEmbedded>, ApiError> {
    if req.embedded_queries.len() > 32 {
        return Err(ApiError::bad(
            UiMessage::error_api_embedded_output_embedded_query_count_budget(),
        ));
    }
    let mut seen = BTreeSet::new();
    req.embedded_queries
        .iter()
        .map(|selection| {
            if !seen.insert(&selection.occurrence_id) {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_office_duplicate_embedded_query_binding(),
                ));
            }
            let occurrence = weftext_data::embedded_query(&req.source, &selection.occurrence_id)
                .map_err(|error| {
                    ApiError::from_data(StatusCode::BAD_REQUEST, "invalid_request", error)
                })?;
            if !occurrence.runnable {
                return Err(ApiError::bad(
                    UiMessage::error_api_embedded_output_embedded_query_occurrence_unqualified(),
                ));
            }
            let frozen = frozen_query(state, &selection.query_id, root)?;
            let expected = EmbeddedQueryBasis {
                session_id: req.session_id.clone(),
                source_hash: weftext_engine::source_hash(&req.source),
                serial: req.serial,
                occurrence_id: selection.occurrence_id.clone(),
            };
            if frozen.embedded.as_ref() != Some(&expected) {
                return Err(ApiError::bad(
                    UiMessage::error_api_embedded_output_embedded_query_result_mismatch(),
                ));
            }
            let result = serde_json::from_value(frozen.value.clone()).map_err(|error| {
                ApiError::internal(
                    UiMessage::error_api_export_conversion_frozen_query_contract_unreadable(),
                )
                .diagnostic(weftext_messages::DiagnosticOrigin::Json, error.to_string())
            })?;
            Ok(FrozenEmbedded {
                occurrence,
                frozen,
                result,
            })
        })
        .collect()
}

pub(crate) fn preview(items: &[FrozenEmbedded]) -> Result<Value, ApiError> {
    let items = items.iter().map(|item| {
        let engine_diagnostics = item.frozen.value.get("engineDiagnostics").and_then(Value::as_array)
            .ok_or_else(|| ApiError::internal(UiMessage::error_api_export_conversion_frozen_query_contract_unreadable()))?;
        let product_errors = item.frozen.value.get("productErrors").and_then(Value::as_array)
            .ok_or_else(|| ApiError::internal(UiMessage::error_api_export_conversion_frozen_query_contract_unreadable()))?;
        Ok(json!({"occurrenceId":item.occurrence.source.id,"label":item.occurrence.source.label,
            "queryId":item.frozen.value["queryId"],"scope":"previouslyDisplayedFrozenResult",
            "cutId":item.frozen.value["cutId"],"coverage":item.frozen.value["coverage"],
            "freshness":item.frozen.value["freshness"],"limited":item.result.limited,
            "rowCount":item.result.rows.len(),"status":status(item)?,
            "diagnostics":item.frozen.value["diagnostics"],
            "engineDiagnostics":engine_diagnostics,
            "productErrors":product_errors,"columns":item.frozen.columns}))
    }).collect::<Result<Vec<_>, ApiError>>()?;
    Ok(json!(items))
}

fn row_count(count: usize) -> Result<UiInteger, ApiError> {
    i64::try_from(count)
        .ok()
        .and_then(UiInteger::new)
        .ok_or_else(|| {
            ApiError::bad(UiMessage::error_api_embedded_output_query_row_count_unrepresentable())
        })
}

fn query_status(
    result: &QueryResult,
    label: &str,
    external_pending: bool,
) -> Result<Vec<UiMessage>, ApiError> {
    let mut messages = vec![
        UiMessage::error_api_embedded_output_frozen_rows(
            weftext_messages::params::ErrorApiEmbeddedOutputFrozenRows {
                label: label.into(),
                rows: row_count(result.rows.len())?,
            },
        ),
        if result.coverage == "complete" {
            UiMessage::error_api_embedded_output_complete_coverage()
        } else {
            UiMessage::error_api_embedded_output_partial_coverage()
        },
        if result.freshness == "current" {
            UiMessage::error_api_embedded_output_current_at_run()
        } else {
            UiMessage::error_api_embedded_output_currentness_unconfirmed()
        },
    ];
    if result.limited {
        messages.push(UiMessage::error_api_embedded_output_result_limit_reached());
    }
    if !result.aggregate_exact {
        messages.push(UiMessage::error_api_embedded_output_aggregate_inexact());
    }
    if external_pending {
        messages.push(UiMessage::error_api_embedded_output_external_draft_pending());
    }
    if result.coverage != "complete" && !result.diagnostics.is_empty() {
        messages.push(UiMessage::error_api_embedded_output_conditions_unknown());
    }
    if result.rows.is_empty() {
        messages.push(if result.coverage == "complete" {
            UiMessage::error_api_embedded_output_complete_empty()
        } else {
            UiMessage::error_api_embedded_output_partial_empty()
        });
    }
    Ok(messages)
}

pub(crate) fn status(item: &FrozenEmbedded) -> Result<Vec<UiMessage>, ApiError> {
    query_status(
        &item.result,
        item.occurrence
            .source
            .label
            .trim_start_matches("weftext-query-"),
        item.frozen.value["externalPending"] == true,
    )
}

/// Human-readable text is rendered only while producing the explicitly localized artifact.
fn render_status(item: &FrozenEmbedded, locale: UiLocale) -> Result<String, ApiError> {
    Ok(status(item)?
        .iter()
        .map(|message| message.render(locale))
        .collect::<Vec<_>>()
        .join(" "))
}

fn matches(n: &RenderNode, item: &FrozenEmbedded) -> bool {
    n.kind == "tag"
        && n.attrs.get("data-weftext-query-label") == Some(&item.occurrence.source.label)
        && n.origin
            .as_ref()
            .is_some_and(|o| o.is_main && o.from == item.occurrence.source.call_from)
}
fn count(n: &RenderNode, item: &FrozenEmbedded) -> usize {
    usize::from(matches(n, item)) + n.children.iter().map(|c| count(c, item)).sum::<usize>()
}
fn prefix_keys(n: &mut RenderNode, prefix: &str) {
    n.key = format!("{prefix}/{}", n.key);
    for c in &mut n.children {
        prefix_keys(c, prefix);
    }
}

pub(crate) fn materialize(
    tree: &mut RenderNode,
    items: &[FrozenEmbedded],
    locale: UiLocale,
) -> Result<Vec<ExportLoss>, ApiError> {
    for item in items {
        if count(tree, item) != 1 {
            return Err(ApiError::bad(
                UiMessage::error_api_embedded_output_unique_embedded_output_position_required(),
            ));
        }
    }
    fn visit(
        n: &mut RenderNode,
        items: &[FrozenEmbedded],
        losses: &mut Vec<ExportLoss>,
        locale: UiLocale,
    ) -> Result<(), ApiError> {
        if n.kind == "tag" && n.attrs.contains_key("data-weftext-query-label") {
            if let Some(item) = items.iter().find(|item| matches(n, item)) {
                let mut table = query_table(&item.result, &item.frozen.columns)?;
                prefix_keys(&mut table, &n.key);
                let mut replacement = node(
                    "div",
                    format!("{}/output", n.key),
                    vec![
                        node(
                            "p",
                            format!("{}/status", n.key),
                            vec![text_node(
                                render_status(item, locale)?,
                                format!("{}/text", n.key),
                            )],
                        ),
                        table,
                    ],
                );
                replacement
                    .attrs
                    .insert("data-weftext-frozen-query".into(), "true".into());
                *n = replacement;
                losses.extend(query_losses(&item.result));
                if item.frozen.value["freshness"] == "stale" {
                    losses.push(report(
                        "query_stale",
                        UiMessage::error_api_embedded_output_selected_stale_retained(),
                    ));
                }
            } else {
                let label = n.attrs["data-weftext-query-label"].clone();
                let message = UiMessage::error_api_embedded_output_missing_frozen_result(
                    weftext_messages::params::ErrorApiEmbeddedOutputMissingFrozenResult { label },
                );
                losses.push(report(
                    "embedded_query_results_not_materialized",
                    message.clone(),
                ));
                *n = node(
                    "p",
                    format!("{}/unavailable", n.key),
                    vec![text_node(message.render(locale), format!("{}/text", n.key))],
                );
            }
            return Ok(());
        }
        for child in &mut n.children {
            visit(child, items, losses, locale)?;
        }
        // Metadata inside an ordinary paragraph can become a real table. Preserve
        // preceding/following text as paragraphs; never flatten the table to runs.
        if n.tag.as_deref() == Some("p")
            && n.children
                .iter()
                .any(|c| c.attrs.contains_key("data-weftext-frozen-query"))
        {
            if n.attrs.contains_key("data-weftext-role")
                || n.children
                    .iter()
                    .any(|c| c.attrs.contains_key("data-weftext-role"))
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_embedded_output_heading_query_table_unsupported(),
                ));
            }
            let mut blocks = vec![];
            let mut inline = vec![];
            let paragraph = |key: String, children| {
                let mut part = node("p", key, children);
                // Keep paragraph formatting on both sides, without duplicating
                // an authored anchor when one paragraph becomes several blocks.
                part.attrs = n.attrs.clone();
                part.attrs.remove("id");
                part
            };
            for child in std::mem::take(&mut n.children) {
                if child.attrs.contains_key("data-weftext-frozen-query") {
                    if !inline.is_empty() {
                        blocks.push(paragraph(
                            format!("{}/before/{}", n.key, blocks.len()),
                            std::mem::take(&mut inline),
                        ));
                    }
                    blocks.push(child);
                } else {
                    inline.push(child);
                }
            }
            if !inline.is_empty() {
                blocks.push(paragraph(format!("{}/after", n.key), inline));
            }
            n.tag = Some("div".into());
            n.children = blocks;
        }
        Ok(())
    }
    fn check_placement(n: &RenderNode, in_inline: bool) -> Result<(), ApiError> {
        if in_inline && n.attrs.contains_key("data-weftext-frozen-query") {
            return Err(ApiError::bad(
                UiMessage::error_api_embedded_output_inline_query_table_unsupported(),
            ));
        }
        let in_inline = in_inline
            || !matches!(
                n.tag.as_deref(),
                Some(
                    "div"
                        | "section"
                        | "article"
                        | "main"
                        | "body"
                        | "html"
                        | "li"
                        | "td"
                        | "th"
                        | "ul"
                        | "ol"
                ) | None
            );
        for child in &n.children {
            check_placement(child, in_inline)?;
        }
        Ok(())
    }
    let mut losses = vec![];
    visit(tree, items, &mut losses, locale)?;
    check_placement(tree, false)?;
    Ok(losses)
}

/// Export-only compilation input. Keep author metadata intact and insert native
/// layout after an exact direct carrier witnessed by this Paged compilation.
/// No byte here is written back to the document or Draft.
pub(crate) fn paged_source(
    source: &str,
    items: &[FrozenEmbedded],
    witnesses: &[weftext_engine::CompiledLabel],
    locale: UiLocale,
) -> Result<(String, Vec<ExportLoss>), ApiError> {
    let occurrences = weftext_data::embedded_queries(source);
    let witnessed = |o: &EmbeddedQueryOccurrence| {
        witnesses.iter().any(|w| {
            w.label == o.source.label
                && w.count == 1
                && w.origins.len() == 1
                && w.origins[0].is_main
                && w.origins[0].from == o.source.call_from
        })
    };
    for item in items {
        if !witnessed(&item.occurrence) {
            return Err(ApiError::bad(
                UiMessage::error_api_embedded_output_paged_query_position_unproved(),
            ));
        }
    }
    if !witnesses.is_empty()
        && ["text", "table", "block", "parbreak"]
            .iter()
            .any(|name| !weftext_engine::standard_binding_available(source, name))
    {
        return Err(ApiError::bad(
            UiMessage::error_api_embedded_output_custom_query_pagination_unsupported(),
        ));
    }
    let mut patches = vec![];
    let mut losses = vec![];
    for witness in witnesses {
        let occurrence = occurrences
            .iter()
            .find(|o| o.runnable && o.source.label == witness.label && witnessed(o));
        let selected =
            occurrence.and_then(|o| items.iter().find(|i| i.occurrence.source.id == o.source.id));
        let missing = UiMessage::error_api_embedded_output_missing_paged_frozen_result(
            weftext_messages::params::ErrorApiEmbeddedOutputMissingPagedFrozenResult {
                label: witness.label.clone(),
            },
        );
        let caption = match selected {
            Some(item) => render_status(item, locale)?,
            None => missing.render(locale),
        };
        if selected.is_none() {
            losses.push(report("embedded_query_results_not_materialized", missing));
        }
        let Some(occurrence) = occurrence else {
            continue;
        };
        let literal = |value: &str| {
            weftext_engine::encode_string(value).map_err(|error| {
                ApiError::from_engine(StatusCode::BAD_REQUEST, "invalid_request", error)
            })
        };
        let mut native = format!("\n#block[\n#text({})\n#parbreak()\n", literal(&caption)?);
        if let Some(item) = selected {
            // Reuse the same typed-cell conversion and budget validation as DOCX.
            query_table(&item.result, &item.frozen.columns)?;
            native.push_str(&format!(
                "#table(columns: {},\n table.header(",
                item.frozen.columns.len()
            ));
            for name in &item.frozen.columns {
                native.push_str(&format!("text({}),", literal(name)?));
            }
            native.push_str("),\n");
            for row in &item.result.rows {
                for name in &item.frozen.columns {
                    let value =
                        cell_text(row.cells.get(name).unwrap_or(&weftext_data::Cell::Absent))?;
                    native.push_str(&format!("text({}),\n", literal(&value)?));
                }
            }
            native.push_str(")\n");
            if item.result.coverage != "complete"
                || item.result.limited
                || !item.result.aggregate_exact
            {
                losses.push(report(
                    "query_incomplete",
                    UiMessage::error_api_embedded_output_paged_partial_retained(),
                ));
            }
            if item.result.freshness == "stale" {
                losses.push(report(
                    "query_stale",
                    UiMessage::error_api_embedded_output_paged_stale_retained(),
                ));
            }
        }
        native.push_str("]\n");
        patches.push((occurrence.source.range.to, native));
    }
    patches.sort_by_key(|p| std::cmp::Reverse(p.0));
    let mut output = source.to_string();
    for (offset, native) in patches {
        output.insert_str(offset, &native);
    }
    if output.len() > 2 * 1024 * 1024 {
        return Err(ApiError::bad(
            UiMessage::error_api_embedded_output_embedded_pdf_input_budget(),
        ));
    }
    Ok((output, losses))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use weftext_messages::ErrorDescription;

    const QUERY: &str = "#metadata((kind: \"weftext.query\", key: \"f8597b15-6b7f-4a6a-9b52-c2732730fe41\", binding: (schema: \"weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema\", revision: \"8a81e35f-ad48-4c12-af4f-3d2607102fa9\", language: \"weftext.query.cel.v1\"), where: \"true\")) <weftext-query-one>";

    fn result() -> QueryResult {
        QueryResult {
            cut_id: "original-cut".into(),
            schema_revision: "original-schema".into(),
            rows: vec![],
            coverage: "complete".into(),
            freshness: "current".into(),
            diagnostics: vec![],
            total_known: 0,
            limited: false,
            aggregate_exact: true,
            runtime: "frozen".into(),
            terminal_schema: BTreeMap::new(),
        }
    }

    fn item(result: QueryResult) -> FrozenEmbedded {
        let source = QUERY;
        let occurrence = weftext_data::embedded_queries(source)
            .into_iter()
            .next()
            .unwrap();
        let mut frozen = serde_json::to_value(&result).unwrap();
        frozen["queryId"] = json!("retained-query");
        frozen["engineDiagnostics"] = json!([]);
        frozen["productErrors"] = json!([]);
        FrozenEmbedded {
            occurrence,
            result,
            frozen: FrozenQueryOutput {
                root: "authorized-root".into(),
                value: frozen,
                columns: vec!["value".into()],
                schema: Value::Null,
                embedded: None,
                born: std::time::Instant::now(),
            },
        }
    }

    #[test]
    fn status_preserves_all_semantic_components_without_rendering_or_rerunning() {
        let mut result = result();
        assert_eq!(
            query_status(&result, "original 😀", false).unwrap(),
            vec![
                UiMessage::error_api_embedded_output_frozen_rows(
                    weftext_messages::params::ErrorApiEmbeddedOutputFrozenRows {
                        label: "original 😀".into(),
                        rows: UiInteger::new(0).unwrap()
                    }
                ),
                UiMessage::error_api_embedded_output_complete_coverage(),
                UiMessage::error_api_embedded_output_current_at_run(),
                UiMessage::error_api_embedded_output_complete_empty(),
            ]
        );
        result.coverage = "partial".into();
        result.freshness = "stale".into();
        result.limited = true;
        result.aggregate_exact = false;
        result.diagnostics.push(ErrorDescription::new(
            UiMessage::error_api_embedded_output_conditions_unknown(),
        ));
        assert_eq!(
            query_status(&result, "original 😀", true).unwrap(),
            vec![
                UiMessage::error_api_embedded_output_frozen_rows(
                    weftext_messages::params::ErrorApiEmbeddedOutputFrozenRows {
                        label: "original 😀".into(),
                        rows: UiInteger::new(0).unwrap()
                    }
                ),
                UiMessage::error_api_embedded_output_partial_coverage(),
                UiMessage::error_api_embedded_output_currentness_unconfirmed(),
                UiMessage::error_api_embedded_output_result_limit_reached(),
                UiMessage::error_api_embedded_output_aggregate_inexact(),
                UiMessage::error_api_embedded_output_external_draft_pending(),
                UiMessage::error_api_embedded_output_conditions_unknown(),
                UiMessage::error_api_embedded_output_partial_empty(),
            ]
        );
        assert_eq!(result.cut_id, "original-cut");
        assert_eq!(result.freshness, "stale");
    }

    #[test]
    fn output_row_count_rejects_inexact_wire_numbers() {
        assert_eq!(
            row_count(100_000).unwrap(),
            UiInteger::new(100_000).unwrap()
        );
        if let Ok(count) = usize::try_from(UiInteger::MAX + 1) {
            let error = row_count(count).unwrap_err();
            assert_eq!(error.code, "invalid_request");
            assert_eq!(
                error.explanation,
                UiMessage::error_api_embedded_output_query_row_count_unrepresentable()
            );
        }
    }

    #[test]
    fn missing_result_renders_only_the_selected_output_locale_and_keeps_typed_loss() {
        let mut tree = node("span", "original-carrier".into(), vec![]);
        tree.kind = "tag".into();
        tree.attrs
            .insert("data-weftext-query-label".into(), "original 😀".into());
        let loss_message = UiMessage::error_api_embedded_output_missing_frozen_result(
            weftext_messages::params::ErrorApiEmbeddedOutputMissingFrozenResult {
                label: "original 😀".into(),
            },
        );
        let losses = materialize(&mut tree, &[], UiLocale::EnUs).unwrap();
        assert_eq!(
            tree.children[0].text.as_deref(),
            Some(loss_message.render(UiLocale::EnUs).as_str())
        );
        assert_ne!(
            tree.children[0].text.as_deref(),
            Some(loss_message.render(UiLocale::ZhCn).as_str())
        );
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].code, "embedded_query_results_not_materialized");
        assert_eq!(losses[0].description.explanation, loss_message);
        assert!(losses[0].product_diagnostics.is_empty());
    }

    #[test]
    fn preview_keeps_typed_state_and_all_associated_parallel_error_evidence() {
        let mut item = item(result());
        let engine: Vec<_> = (0..48).map(|index| json!({
            "entityKey":format!("entity-{index}"),
            "diagnostics":[weftext_engine::Diagnostic {
                severity: "error".into(),
                explanation: Some(UiMessage::error_api_embedded_output_conditions_unknown()),
                diagnostics: vec![],
                origin: None,
            }],
        })).collect();
        let errors: Vec<_> = (0..48).map(|index| json!({
            "entityKey":format!("entity-{index}"),
            "error":ApiError::bad(UiMessage::error_api_embedded_output_missing_frozen_result(weftext_messages::params::ErrorApiEmbeddedOutputMissingFrozenResult {
                label: format!("original-{index}"),
            })),
        })).collect();
        item.frozen.value["engineDiagnostics"] = json!(engine);
        item.frozen.value["productErrors"] = json!(errors);
        let expected_status = serde_json::to_value(status(&item).unwrap()).unwrap();
        let preview = preview(&[item]).unwrap();
        assert_eq!(preview[0]["status"], expected_status);
        assert_eq!(preview[0]["engineDiagnostics"], json!(engine));
        assert_eq!(preview[0]["productErrors"], json!(errors));
        assert_eq!(preview[0]["queryId"], "retained-query");
        assert_eq!(preview[0]["cutId"], "original-cut");
    }
    #[test]
    fn paged_missing_result_uses_explicit_locale_without_changing_author_input() {
        let occurrence = weftext_data::embedded_queries(QUERY)
            .into_iter()
            .next()
            .unwrap();
        assert!(occurrence.runnable);
        let witness = weftext_engine::CompiledLabel {
            label: occurrence.source.label.clone(),
            count: 1,
            origins: vec![weftext_engine::Origin {
                path: "owner.typ".into(),
                from: occurrence.source.call_from,
                to: occurrence.source.range.to,
                utf16_from: QUERY[..occurrence.source.call_from].encode_utf16().count(),
                utf16_to: QUERY[..occurrence.source.range.to].encode_utf16().count(),
                raw: occurrence.source.range.raw.clone(),
                syntax_kind: "FuncCall".into(),
                is_main: true,
            }],
        };
        let message = UiMessage::error_api_embedded_output_missing_paged_frozen_result(
            weftext_messages::params::ErrorApiEmbeddedOutputMissingPagedFrozenResult {
                label: occurrence.source.label,
            },
        );
        let (output, losses) = paged_source(QUERY, &[], &[witness], UiLocale::EnUs).unwrap();
        assert!(output.starts_with(QUERY));
        assert!(
            output
                .contains(&weftext_engine::encode_string(&message.render(UiLocale::EnUs)).unwrap())
        );
        assert!(
            !output
                .contains(&weftext_engine::encode_string(&message.render(UiLocale::ZhCn)).unwrap())
        );
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].code, "embedded_query_results_not_materialized");
        assert_eq!(losses[0].description.explanation, message);
        assert!(losses[0].product_diagnostics.is_empty());
    }

    #[test]
    fn preview_rejects_missing_or_invalid_required_parallel_evidence_arrays() {
        for key in ["engineDiagnostics", "productErrors"] {
            let mut missing = item(result());
            missing.frozen.value.as_object_mut().unwrap().remove(key);
            let error = preview(&[missing]).unwrap_err();
            assert_eq!(error.code, "internal");
            assert_eq!(
                error.explanation,
                UiMessage::error_api_export_conversion_frozen_query_contract_unreadable()
            );
            let mut invalid = item(result());
            invalid.frozen.value[key] = Value::Null;
            let error = preview(&[invalid]).unwrap_err();
            assert_eq!(
                error.explanation,
                UiMessage::error_api_export_conversion_frozen_query_contract_unreadable()
            );
        }
    }
}
