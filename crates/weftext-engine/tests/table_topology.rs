use std::sync::OnceLock;
use tempfile::TempDir;
use weftext_engine::*;

fn table(source: &str) -> StructureInfo {
    inspect_editor(source)
        .structures
        .into_iter()
        .find(|s| s.kind == "table")
        .unwrap()
}

fn apply(source: &str, action: TableAction) -> Result<EditOutput, EngineError> {
    edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent: EditIntent::TableEdit {
                range: table(source).range,
                action,
            },
        },
    )
}

fn native(source: &str) -> RenderOutput {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    let dir = TempDir::new().unwrap();
    let out = ENGINE
        .get_or_init(|| Engine::new().unwrap())
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(out.ok, "{source}\n{:?}", out.diagnostics);
    out
}

#[test]
fn logical_map_certifies_duplicate_empty_and_spanning_origins() {
    let source = "前\n#table(columns: 3, table.header([同], [], [同]), table.cell(rowspan: 2)[甲], table.cell(colspan: 2)[乙], [], [同])\n后";
    let t = table(source).table.unwrap();
    assert!(t.topology_writable, "{:?}", t.unavailable_reason);
    assert!(!t.grid_writable);
    assert_eq!((t.rows, t.columns, t.header_rows), (3, 3, 1));
    let cells = t.logical_cells;
    assert_eq!(cells.len(), 7);
    assert_eq!((cells[3].row, cells[3].column, cells[3].rowspan), (1, 0, 2));
    assert_eq!((cells[4].row, cells[4].column, cells[4].colspan), (1, 1, 2));
    assert_eq!((cells[5].row, cells[5].column), (2, 1));
    assert_ne!(cells[0].body.from, cells[2].body.from);
    for cell in cells {
        assert_eq!(&source[cell.body.from..cell.body.to], cell.body.raw);
    }
    native(source);
}

#[test]
fn clear_and_quoted_paste_touch_only_selected_bodies_and_keep_styles() {
    let source = "前\n#table( columns : 2, fill: gray,\n  [甲],  \n\t table.cell(fill: red)[乙],\n [同], /*body*/[丁])\n后";
    // Ambiguous argument comments refuse topology rather than normalize them.
    assert!(
        apply(
            source,
            TableAction::ClearRange {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0
            }
        )
        .is_err()
    );
    let source = source.replace(" /*body*/", " ");
    let out = apply(
        &source,
        TableAction::PasteRange {
            row: 0,
            column: 1,
            text: "\"换行\n值\"\r\n尾".into(),
            expand: false,
        },
    )
    .unwrap();
    assert_eq!(out.patches.len(), 2);
    assert!(out.source.contains("table.cell(fill: red)[换行\\\n值]"));
    assert!(
        out.source
            .starts_with("前\n#table( columns : 2, fill: gray,\n  [甲],  \n\t ")
    );
    assert!(out.source.contains("\n [同], [尾]"));
    native(&out.source);
    let cleared = apply(
        &out.source,
        TableAction::ClearRange {
            top: 0,
            left: 1,
            bottom: 1,
            right: 1,
        },
    )
    .unwrap();
    assert!(cleared.source.contains("table.cell(fill: red)[]"));
    assert!(cleared.source.contains("[同], []"));
}

#[test]
fn rows_columns_reorder_preserve_native_content_and_outer_source() {
    let source = "前\n#table(columns: 2,\n  [*甲*],\t[乙],\n  [$x$],  [#link(\"https://example.org\")[丁]])\n后";
    let moved = apply(source, TableAction::MoveColumn { from: 0, to: 1 }).unwrap();
    assert!(moved.source.starts_with(
        "前\n#table(columns: 2,\n  [乙],\t[*甲*],\n  [#link(\"https://example.org\")[丁]],  [$x$])"
    ));
    let inserted = apply(&moved.source, TableAction::InsertColumn { index: 1 }).unwrap();
    assert_eq!(table(&inserted.source).table.unwrap().columns, 3);
    let removed = apply(&inserted.source, TableAction::RemoveColumn { index: 1 }).unwrap();
    assert_eq!(table(&removed.source).table.unwrap().columns, 2);
    let moved_row = apply(&removed.source, TableAction::MoveRow { from: 0, to: 1 }).unwrap();
    assert!(moved_row.source.contains("[*甲*]"));
    assert!(moved_row.source.contains("[$x$]"));
    assert!(moved_row.source.ends_with("\n后"));
    native(&moved_row.source);
}

#[test]
fn merge_split_and_transpose_keep_content_in_documented_origin_order() {
    let source = "#table(columns: 2, [甲], [$x$], [#link(\"https://example.org\")[乙]], [])";
    let merged = apply(
        source,
        TableAction::Merge {
            top: 0,
            left: 0,
            bottom: 1,
            right: 0,
        },
    )
    .unwrap();
    let info = table(&merged.source).table.unwrap();
    assert_eq!(info.logical_cells.len(), 3);
    assert!(
        merged
            .map
            .literal_regions
            .iter()
            .any(|region| region.text.starts_with('甲'))
    );
    assert!(
        merged
            .source
            .contains("table.cell(rowspan: 2)[甲\\\n\\\n#[#link(\"https://example.org\")[乙]]]")
    );
    native(&merged.source);
    let transposed = apply(&merged.source, TableAction::Transpose).unwrap();
    let info = table(&transposed.source).table.unwrap();
    assert!(
        info.logical_cells
            .iter()
            .any(|c| c.colspan == 2 && c.body.raw.contains("甲"))
    );
    native(&transposed.source);
    let split = apply(&transposed.source, TableAction::Split { row: 0, column: 0 }).unwrap();
    let info = table(&split.source).table.unwrap();
    assert_eq!(info.logical_cells.len(), 4);
    assert!(
        info.logical_cells
            .iter()
            .all(|c| c.rowspan == 1 && c.colspan == 1)
    );
    assert_eq!(
        info.logical_cells[0].body.raw,
        "甲\\\n\\\n#[#link(\"https://example.org\")[乙]]"
    );
    assert_eq!(info.logical_cells[1].body.raw, "");
    native(&split.source);
}

#[test]
fn header_changes_are_native_and_direction_loss_is_never_implicit() {
    let source = "#table(columns: 2, [甲], [乙], [丙], [丁])";
    let out = apply(
        source,
        TableAction::SetHeader {
            rows: 1,
            columns: 0,
        },
    )
    .unwrap();
    assert!(out.source.contains("table.header([甲], [乙])"));
    native(&out.source);
    assert!(apply(&out.source, TableAction::Transpose).is_err());
    assert!(
        apply(
            &out.source,
            TableAction::SetHeader {
                rows: 0,
                columns: 1
            }
        )
        .is_err()
    );
    let without = apply(
        &out.source,
        TableAction::SetHeader {
            rows: 0,
            columns: 0,
        },
    )
    .unwrap();
    assert!(!without.source.contains("table.header"));
    let transposed = apply(&without.source, TableAction::Transpose).unwrap();
    native(&transposed.source);
    let source =
        "#table(columns: 2, table.header(repeat: false, level: 1,\n [甲],\t[乙],),\n [丙], [丁])";
    let moved = apply(source, TableAction::MoveColumn { from: 0, to: 1 }).unwrap();
    assert!(
        moved
            .source
            .contains("table.header(repeat: false, level: 1,\n [乙],\t[甲],)")
    );
    native(&moved.source);
}

#[test]
fn clipboard_expansion_and_spans_validate_atomically() {
    let source = "#table(columns: 2, [甲], [乙], [丙], [丁])";
    assert!(
        apply(
            source,
            TableAction::PasteRange {
                row: 1,
                column: 1,
                text: "a\tb\nc\td".into(),
                expand: false
            }
        )
        .is_err()
    );
    let expanded = apply(
        source,
        TableAction::PasteRange {
            row: 1,
            column: 1,
            text: "a\tb\nc\td".into(),
            expand: true,
        },
    )
    .unwrap();
    let info = table(&expanded.source).table.unwrap();
    assert_eq!((info.rows, info.columns), (3, 3));
    native(&expanded.source);
    let cells = vec![TablePasteCell {
        row: 0,
        column: 0,
        rowspan: 2,
        colspan: 2,
        text: "合并\n文字".into(),
        header: false,
    }];
    let pasted = apply(
        source,
        TableAction::PasteCells {
            row: 0,
            column: 0,
            rows: 2,
            columns: 2,
            cells: cells.clone(),
            expand: false,
        },
    )
    .unwrap();
    assert_eq!(table(&pasted.source).table.unwrap().logical_cells.len(), 1);
    native(&pasted.source);
    assert!(
        apply(
            &pasted.source,
            TableAction::ClearRange {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0
            }
        )
        .is_err()
    );
    assert!(apply(&pasted.source, TableAction::InsertRow { index: 1 }).is_err());
    assert!(
        apply(
            source,
            TableAction::PasteCells {
                row: 0,
                column: 0,
                rows: 2,
                columns: 2,
                cells: vec![],
                expand: false
            }
        )
        .is_err()
    );
    let mut overlap = cells.clone();
    overlap.extend(cells);
    assert!(
        apply(
            source,
            TableAction::PasteCells {
                row: 0,
                column: 0,
                rows: 2,
                columns: 2,
                cells: overlap,
                expand: false
            }
        )
        .is_err()
    );
}

#[test]
fn clipboard_table_insertion_revalidates_spans_and_plain_text() {
    let source = "前文";
    let cells = vec![
        TablePasteCell {
            row: 0,
            column: 0,
            rowspan: 1,
            colspan: 2,
            text: "<表头>".into(),
            header: true,
        },
        TablePasteCell {
            row: 1,
            column: 0,
            rowspan: 1,
            colspan: 1,
            text: "#代码".into(),
            header: false,
        },
        TablePasteCell {
            row: 1,
            column: 1,
            rowspan: 1,
            colspan: 1,
            text: "换\n行".into(),
            header: false,
        },
    ];
    let out = edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent: EditIntent::InsertBlock {
                at: source.len(),
                block: BlockInsert::ClipboardTable {
                    rows: 2,
                    columns: 2,
                    cells,
                },
            },
        },
    )
    .unwrap();
    assert!(out.source.starts_with("前文\n\n#table("));
    assert_eq!(table(&out.source).table.unwrap().header_rows, 1);
    assert!(out.source.contains("[\\#代码]"));
    native(&out.source);
}

#[test]
fn clipboard_topology_cannot_discard_different_native_cell_styles() {
    let source = "#table(columns: 2, table.cell(fill: red)[甲], [乙])";
    let cells = vec![TablePasteCell {
        row: 0,
        column: 0,
        rowspan: 1,
        colspan: 2,
        text: "合并".into(),
        header: false,
    }];
    assert!(
        apply(
            source,
            TableAction::PasteCells {
                row: 0,
                column: 0,
                rows: 1,
                columns: 2,
                cells,
                expand: false
            }
        )
        .is_err()
    );
}

#[test]
fn merge_preserves_each_cell_native_lexical_scope() {
    let source = "#let x = 9\n#table(columns: 2, [#let x = 1\n甲], [#x])";
    let out = apply(
        source,
        TableAction::Merge {
            top: 0,
            left: 0,
            bottom: 0,
            right: 1,
        },
    )
    .unwrap();
    assert!(out.source.contains("#[#let x = 1\n甲]"));
    fn text(node: &RenderNode) -> String {
        format!(
            "{}{}",
            node.text.as_deref().unwrap_or(""),
            node.children.iter().map(text).collect::<String>()
        )
    }
    let compiled = native(&out.source);
    let content = text(compiled.tree.as_ref().unwrap());
    assert!(content.contains('9'), "{content}");
    assert!(!content.contains('1'), "{content}");
}

#[test]
fn plain_merge_and_cell_newlines_remain_exact_editable_literal_runs() {
    let source = "#table(columns: 2, [甲], [乙])";
    let merged = apply(
        source,
        TableAction::Merge {
            top: 0,
            left: 0,
            bottom: 0,
            right: 1,
        },
    )
    .unwrap();
    let body = table(&merged.source).table.unwrap().logical_cells[0]
        .body
        .clone();
    let region = merged
        .map
        .literal_regions
        .iter()
        .find(|region| region.range.from == body.from && region.range.to == body.to)
        .unwrap();
    assert_eq!(region.text, "甲\n\n乙");
    let edited = edit(
        &merged.source,
        &EditRequest {
            source_hash: merged.source_hash,
            intent: EditIntent::ReplaceLiteral {
                range: region.range.clone(),
                text: "首行\n次行\n".into(),
            },
        },
    )
    .unwrap();
    let body = table(&edited.source).table.unwrap().logical_cells[0]
        .body
        .clone();
    let region = edited
        .map
        .literal_regions
        .iter()
        .find(|region| region.range.from == body.from && region.range.to == body.to)
        .unwrap();
    assert_eq!(region.text, "首行\n次行\n");
    assert_eq!(body.raw, "首行\\\n次行\\\n");
    let compiled = native(&edited.source);
    let html = compiled.html;
    assert!(html.contains("<br"), "{html}");
    let empty = apply(
        "#table(columns: 2, [], [])",
        TableAction::Merge {
            top: 0,
            left: 0,
            bottom: 0,
            right: 1,
        },
    )
    .unwrap();
    let body = table(&empty.source).table.unwrap().logical_cells[0]
        .body
        .clone();
    assert!(
        empty
            .map
            .literal_regions
            .iter()
            .any(|region| region.range.from == body.from
                && region.range.to == body.to
                && region.text == "\n\n")
    );
}

#[test]
fn paste_rejects_final_table_budget_and_overflow_before_expansion() {
    let source = "#table(columns: 1, [甲])";
    assert!(
        apply(
            source,
            TableAction::PasteRange {
                row: usize::MAX,
                column: 0,
                text: "a\nb".into(),
                expand: true
            }
        )
        .is_err()
    );
    let text = vec!["x"; 1001].join("\t");
    assert!(
        apply(
            source,
            TableAction::PasteRange {
                row: 0,
                column: 0,
                text,
                expand: true
            }
        )
        .is_err()
    );
    let source = format!("#table(columns: 1, {})", vec!["[]"; 101].join(", "));
    let text = vec!["x"; 1000].join("\t");
    assert!(
        apply(
            &source,
            TableAction::PasteRange {
                row: 0,
                column: 0,
                text,
                expand: true
            }
        )
        .is_err()
    );
}

#[test]
fn dynamic_trivia_partial_topology_and_stale_requests_are_refused() {
    for source in [
        "#table(columns: (1fr, 2fr), [甲], [乙])",
        "#table(columns: 2, [甲], [乙], [丙])",
        "#table(columns: 2, /*unknown*/ [甲], [乙])",
        "#table(columns: 2, table.cell(x: 1)[甲], [乙])",
        "#let cells = ([甲], [乙]); #table(columns: 2, ..cells)",
        "#set table.cell(rowspan: 2)\n#table(columns: 2, [甲], [乙])",
        "#set table(fill: (x, y) => red)\n#table(columns: 2, [甲], [乙])",
        "#table(columns: 2, fill: (red, blue), [甲], [乙])",
        "#let red = (x, y) => white\n#table(columns: 2, fill: red, [甲], [乙])",
    ] {
        let info = table(source).table.unwrap();
        assert!(!info.topology_writable, "{source}");
        assert!(apply(source, TableAction::Transpose).is_err(), "{source}");
    }
    let source = "#table(columns: 1, [甲])";
    let request = EditRequest {
        source_hash: source_hash(source),
        intent: EditIntent::TableEdit {
            range: table(source).range,
            action: TableAction::Transpose,
        },
    };
    assert!(
        edit(&format!("前{source}"), &request)
            .unwrap_err()
            .description()
            .explanation
            == weftext_engine::UiMessage::error_engine_editor_source_stale()
    );
}
