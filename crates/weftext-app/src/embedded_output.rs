//! Materialize only retained, source-bound Query results. No evaluator or writer.
use super::{export_api::*, export_conversions::*, *};
use std::collections::BTreeSet;
use weftext_data::{EmbeddedQueryOccurrence, QueryResult};
use weftext_engine::RenderNode;
use weftext_export::ExportLoss;

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
        return Err(ApiError::bad("正文查询输出超过32项预算"));
    }
    let mut seen = BTreeSet::new();
    req.embedded_queries
        .iter()
        .map(|selection| {
            if !seen.insert(&selection.occurrence_id) {
                return Err(ApiError::bad("同一正文查询不能绑定两份结果"));
            }
            let occurrence = weftext_data::embedded_query(&req.source, &selection.occurrence_id)
                .map_err(|e| ApiError::bad(e.to_string()))?;
            if !occurrence.runnable {
                return Err(ApiError::bad("正文查询来源不唯一或不可执行，不能插入结果"));
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
                    "查询结果不属于本篇准确草稿和正文位置；没有换用其他结果",
                ));
            }
            let result = serde_json::from_value(frozen.value.clone())
                .map_err(|_| ApiError::internal("冻结Query合同不可读取"))?;
            Ok(FrozenEmbedded {
                occurrence,
                frozen,
                result,
            })
        })
        .collect()
}

pub(crate) fn preview(items: &[FrozenEmbedded]) -> Value {
    json!(items.iter().map(|item| json!({"occurrenceId":item.occurrence.source.id,"label":item.occurrence.source.label,"queryId":item.frozen.value["queryId"],"scope":"previouslyDisplayedFrozenResult","cutId":item.frozen.value["cutId"],"coverage":item.frozen.value["coverage"],"freshness":item.frozen.value["freshness"],"limited":item.result.limited,"rowCount":item.result.rows.len(),"diagnostics":item.frozen.value["diagnostics"],"columns":item.frozen.columns})).collect::<Vec<_>>())
}

pub(crate) fn status(item: &FrozenEmbedded) -> String {
    let result = &item.result;
    let label = item
        .occurrence
        .source
        .label
        .trim_start_matches("weftext-query-");
    let coverage = if result.coverage == "complete" {
        "完整范围"
    } else {
        "部分范围 partial"
    };
    let freshness = if result.freshness == "current" {
        "运行时为当前数据"
    } else {
        "冻结快照，当前性未确认 stale"
    };
    let mut text = format!(
        "查询 {label}：{} 行 · {coverage} · {freshness}。",
        result.rows.len()
    );
    if result.limited {
        text.push_str(" 已达结果上限。");
    }
    if !result.aggregate_exact {
        text.push_str(" 汇总不代表完整精确总数。");
    }
    if item.frozen.value["externalPending"] == true {
        text.push_str(" 有外部稿尚待接纳。");
    }
    if result.coverage != "complete" && !result.diagnostics.is_empty() {
        text.push_str(" 诊断：部分条件或数据尚无法确定，详细原因见输出预览。");
    }
    if result.rows.is_empty() {
        text.push_str(if result.coverage == "complete" {
            " 原运行范围内为空。"
        } else {
            " 已知范围没有结果，不能据此断言全库为空。"
        });
    }
    text
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
) -> Result<Vec<ExportLoss>, ApiError> {
    for item in items {
        if count(tree, item) != 1 {
            return Err(ApiError::bad(
                "冻结正文没有唯一对应的查询出现项，不能猜测插入位置",
            ));
        }
    }
    fn visit(
        n: &mut RenderNode,
        items: &[FrozenEmbedded],
        losses: &mut Vec<ExportLoss>,
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
                            vec![text_node(status(item), format!("{}/text", n.key))],
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
                        "保留所选过期结果及其状态，没有悄悄重新运行查询。",
                    ));
                }
            } else {
                let label = n.attrs["data-weftext-query-label"].clone();
                let message = format!(
                    "查询 {label} 没有与本次草稿匹配的已显示冻结结果；此处结果未纳入输出。请先运行或处理查询，再重新生成。"
                );
                losses.push(report("embedded_query_results_not_materialized", &message));
                *n = node(
                    "p",
                    format!("{}/unavailable", n.key),
                    vec![text_node(message, format!("{}/text", n.key))],
                );
            }
            return Ok(());
        }
        for child in &mut n.children {
            visit(child, items, losses)?;
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
                    "标题段内的查询表格暂不能安全转换；原稿保持不变",
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
                "行内样式或复杂段落中的查询表格暂不能安全转换；原稿保持不变",
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
    visit(tree, items, &mut losses)?;
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
                "原生分页没有唯一对应的正文查询位置，未借用 HTML 的位置证明",
            ));
        }
    }
    if !witnesses.is_empty()
        && ["text", "table", "block", "parbreak"]
            .iter()
            .any(|name| !weftext_engine::standard_binding_available(source, name))
    {
        return Err(ApiError::bad(
            "自定义绑定或模块使查询分页插入无法安全证明；原稿保留，请使用 DOCX 或独立结果输出",
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
        let caption = selected.map(status).unwrap_or_else(|| {
            format!(
                "查询 {} 的结果未纳入本次 PDF：缺少当前草稿的冻结结果或唯一可插入来源。",
                witness.label
            )
        });
        if selected.is_none() {
            losses.push(report("embedded_query_results_not_materialized", &caption));
        }
        let Some(occurrence) = occurrence else {
            continue;
        };
        let literal = |value: &str| {
            weftext_engine::encode_string(value).map_err(|e| ApiError::bad(e.to_string()))
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
                    "PDF 保留所选部分结果与非精确汇总状态，不将其当作完整总数。",
                ));
            }
            if item.result.freshness == "stale" {
                losses.push(report(
                    "query_stale",
                    "PDF 保留所选过期结果，没有重新运行查询。",
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
            "含查询结果的 PDF 编译输入超过 2 MiB；未截断结果",
        ));
    }
    Ok((output, losses))
}
