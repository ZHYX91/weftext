use std::sync::OnceLock;
use tempfile::TempDir;
use weftext_engine::*;

fn apply(source: &str, intent: EditIntent) -> EditOutput {
    edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent,
        },
    )
    .unwrap()
}
fn native(source: &str) {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    let dir = TempDir::new().unwrap();
    let out = ENGINE
        .get_or_init(|| Engine::new().unwrap())
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(out.ok, "{source}\n{:?}", out.diagnostics);
}

#[test]
fn literal_regions_coalesce_escaped_chinese_and_roundtrip_empty() {
    let raw = encode_literal("中文 a*b # [标题] 😀").unwrap();
    let map = inspect_editor(&raw);
    assert_eq!(map.literal_regions.len(), 1, "{map:?}");
    assert_eq!(map.literal_regions[0].text, "中文 a*b # [标题] 😀");
    let out = apply(
        &raw,
        EditIntent::ReplaceLiteral {
            range: map.literal_regions[0].range.clone(),
            text: "连续输入 _ 字符".into(),
        },
    );
    assert_eq!(out.map.literal_regions.len(), 1);
    assert_eq!(out.map.literal_regions[0].text, "连续输入 _ 字符");
    native(&out.source);
    let empty = apply(
        &out.source,
        EditIntent::ReplaceLiteral {
            range: out.map.literal_regions[0].range.clone(),
            text: String::new(),
        },
    );
    assert!(empty.source.is_empty());
    assert_eq!(empty.map.literal_regions[0].text, "");
    let again = apply(
        &empty.source,
        EditIntent::ReplaceLiteral {
            range: empty.map.literal_regions[0].range.clone(),
            text: "再输入".into(),
        },
    );
    native(&again.source);
}

#[test]
fn stale_hash_and_tampered_ranges_are_rejected() {
    let map = inspect_editor("中文");
    let mut range = map.literal_regions[0].range.clone();
    range.from += 1;
    assert!(
        edit(
            "中文",
            &EditRequest {
                source_hash: map.source_hash.clone(),
                intent: EditIntent::ReplaceLiteral {
                    range,
                    text: "x".into()
                }
            }
        )
        .is_err()
    );
    assert!(
        edit(
            "后继输入",
            &EditRequest {
                source_hash: map.source_hash,
                intent: EditIntent::SetTitle {
                    text: Some("旧标题".into())
                }
            }
        )
        .is_err()
    );
}

#[test]
fn heading_levels_empty_heading_and_list_are_native() {
    let out = apply(
        "",
        EditIntent::InsertBlock {
            at: 0,
            block: BlockInsert::Heading {
                text: "深标题".into(),
                level: 12,
            },
        },
    );
    native(&out.source);
    let heading = &out.map.structures[0];
    assert_eq!(heading.level, Some(12));
    let down = apply(
        &out.source,
        EditIntent::SetHeadingLevel {
            range: heading.range.clone(),
            level: 9,
        },
    );
    native(&down.source);
    let region = down
        .map
        .literal_regions
        .iter()
        .find(|r| r.role == "heading")
        .unwrap();
    let empty = apply(
        &down.source,
        EditIntent::ReplaceLiteral {
            range: region.range.clone(),
            text: String::new(),
        },
    );
    native(&empty.source);
    assert!(
        empty.map.literal_regions.iter().any(|r| r.text.is_empty()),
        "{:?}",
        empty.map
    );
    let list = apply(
        "",
        EditIntent::InsertBlock {
            at: 0,
            block: BlockInsert::BulletList {
                items: vec![String::new(), "列表第二项".into()],
            },
        },
    );
    native(&list.source);
    assert!(list.map.literal_regions.iter().any(|r| r.text.is_empty()));
}

#[test]
fn title_subtitle_roles_are_unique_and_not_headings() {
    let a = apply(
        "正文",
        EditIntent::SetTitle {
            text: Some("文档标题".into()),
        },
    );
    native(&a.source);
    assert_eq!(a.map.title.status, "writable");
    let b = apply(
        &a.source,
        EditIntent::SetSubtitle {
            text: Some("关联副标题".into()),
        },
    );
    native(&b.source);
    assert_eq!(b.map.subtitle.status, "writable");
    assert_eq!(
        b.map
            .structures
            .iter()
            .filter(|s| s.kind == "heading")
            .count(),
        0
    );
    let removed = apply(&b.source, EditIntent::SetTitle { text: None });
    native(&removed.source);
    assert!(!removed.source.contains("weftext-subtitle"));
    assert!(removed.source.contains("正文"));
    assert_eq!(
        inspect_editor("#title[A]\n#title[B]").title.status,
        "ambiguous"
    );
    assert_eq!(
        inspect_editor("#block[孤立] <weftext-subtitle>")
            .subtitle
            .status,
        "missing-unique-title"
    );
}

#[test]
fn run_in_is_per_heading_reversible_and_preserves_reference_label() {
    let source = "#set heading(numbering: \"1.1\")\n== 方法 <methods>\n\n正文 *强调*。\n\n== 下一节\n第二段。";
    let map = inspect_editor(source);
    let first = map.structures.iter().find(|s| s.kind == "heading").unwrap();
    assert_eq!(first.label.as_deref(), Some("methods"), "{first:?}");
    let a = apply(
        source,
        EditIntent::SetRunIn {
            range: first.range.clone(),
            enabled: true,
        },
    );
    native(&a.source);
    assert!(a.source.contains("show <methods>"));
    assert!(
        a.map
            .structures
            .iter()
            .find(|s| s.kind == "heading")
            .unwrap()
            .run_in
    );
    let first = a
        .map
        .structures
        .iter()
        .find(|s| s.kind == "heading")
        .unwrap();
    let b = apply(
        &a.source,
        EditIntent::SetRunIn {
            range: first.range.clone(),
            enabled: false,
        },
    );
    native(&b.source);
    assert!(!b.source.contains("show <methods>"));
    assert!(b.source.contains("<methods>"));
    let generated = "== 无标记\n正文。";
    let h = inspect_editor(generated).structures[0].range.clone();
    let a = apply(
        generated,
        EditIntent::SetRunIn {
            range: h,
            enabled: true,
        },
    );
    native(&a.source);
    assert!(a.source.contains("weftext-heading-1"));
    let no_body = "== 标题\n- 列表";
    let h = inspect_editor(no_body).structures[0].range.clone();
    assert!(
        edit(
            no_body,
            &EditRequest {
                source_hash: source_hash(no_body),
                intent: EditIntent::SetRunIn {
                    range: h,
                    enabled: true
                }
            }
        )
        .is_err()
    );
}

#[test]
fn math_table_inline_and_duplicate_cells_keep_exact_ownership() {
    let source = "前文\n\n#table(columns: 2, [相同], [相同], [丙], [丁])\n\n后文";
    let map = inspect_editor(source);
    let table = map.structures.iter().find(|s| s.kind == "table").unwrap();
    let cells = &table.table.as_ref().unwrap().cells;
    let a = apply(
        source,
        EditIntent::SetTableCell {
            range: cells[1].clone(),
            text: "改第二格".into(),
        },
    );
    native(&a.source);
    assert_eq!(a.patches.len(), 1);
    assert!(a.source.contains("[相同], [改第二格]"));
    assert_eq!(&a.source[..a.patches[0].from], &source[..a.patches[0].from]);
    let table = a.map.structures.iter().find(|s| s.kind == "table").unwrap();
    let b = apply(
        &a.source,
        EditIntent::TableEdit {
            range: table.range.clone(),
            action: TableAction::InsertRow { index: 1 },
        },
    );
    native(&b.source);
    assert_eq!(
        b.map
            .structures
            .iter()
            .find(|s| s.kind == "table")
            .unwrap()
            .table
            .as_ref()
            .unwrap()
            .rows,
        3
    );
    let math = apply(
        "",
        EditIntent::InsertBlock {
            at: 0,
            block: BlockInsert::Math {
                text: "\"总价\" = frac(1, 2) + sqrt(x)".into(),
                display: true,
            },
        },
    );
    native(&math.source);
    let r = math.map.structures[0].range.clone();
    assert!(
        edit(
            &math.source,
            &EditRequest {
                source_hash: math.source_hash,
                intent: EditIntent::ReplaceMath {
                    range: r,
                    text: "x$ #panic() $y".into(),
                    display: false
                }
            }
        )
        .is_err()
    );
    let map = inspect_editor("文字");
    let emph = apply(
        "文字",
        EditIntent::SetInline {
            range: map.literal_regions[0].range.clone(),
            format: InlineFormat::Strike,
        },
    );
    native(&emph.source);
}

#[test]
fn complex_generated_and_arbitrary_show_keep_source_without_false_addresses() {
    let source = "#let x = [生成]\n#x #x\n\n旁边正文";
    let map = inspect_editor(source);
    assert!(!map.literal_regions.iter().any(|r| r.text == "生成"));
    assert!(map.literal_regions.iter().any(|r| r.text == "旁边正文"));
    let map = inspect_editor("#show text: it => upper(it.text)\n正文");
    assert!(map.literal_regions.is_empty());
    assert!(map.unavailable_reason.is_some());
    let table = "#table(columns: 2, table.cell(colspan: 2)[合并], [甲], [乙])";
    let map = inspect_editor(table);
    assert!(!map.structures[0].table.as_ref().unwrap().grid_writable);
}

#[test]
fn list_split_raw_and_source_preservation() {
    let source = "// 作者注释\n- 中文😀末尾\n\n后文";
    let map = inspect_editor(source);
    let item = map
        .structures
        .iter()
        .find(|s| s.kind == "bulletListItem")
        .unwrap();
    let out = apply(
        source,
        EditIntent::ListEdit {
            range: item.range.clone(),
            action: ListAction::Split { at: 4 },
        },
    );
    native(&out.source);
    assert!(out.source.starts_with("// 作者注释\n"));
    assert!(out.source.ends_with("\n\n后文"));
    let raw = apply(
        "",
        EditIntent::InsertBlock {
            at: 0,
            block: BlockInsert::Raw {
                text: "中文代码 ` `".into(),
                language: Some("python".into()),
                block: true,
            },
        },
    );
    native(&raw.source);
}

#[test]
fn root_metadata_certification_is_binding_only_and_label_unique() {
    let valid = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", amount: decimal(\"2.50\"))) <weftext>\n#metadata((where: \"x\")) <weftext-query>";
    assert!(certify_metadata_declaration(valid, "weftext").is_some());
    assert!(certify_metadata_declaration(valid, "weftext-query").is_some());
    assert!(
        certify_metadata_declaration(
            "#let metadata = x=>x\n#metadata((id: \"x\")) <weftext>",
            "weftext"
        )
        .is_none()
    );
    assert!(
        certify_metadata_declaration(
            "#metadata((id: \"x\")) <weftext>\n= 标题 <weftext>",
            "weftext"
        )
        .is_none()
    );
}

#[test]
fn adding_native_title_preserves_root_and_query_metadata_certification() {
    let source = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"task\",), status: \"open\")) <weftext>\n#metadata((where: \"status == \\\"open\\\"\")) <weftext-query>\n\n正文";
    let out = apply(
        source,
        EditIntent::SetTitle {
            text: Some("白云山".into()),
        },
    );
    assert!(
        out.source
            .starts_with("#set document(title: [白云山])\n#title()"),
        "{}",
        out.source
    );
    native(&out.source);
    for label in ["weftext", "weftext-query"] {
        let before = certify_metadata_declaration(source, label).unwrap();
        let after = certify_metadata_declaration(&out.source, label).unwrap();
        assert_eq!(before.raw, after.raw);
        assert_eq!(&out.source[after.from..after.to], after.raw);
    }
    let inspected = inspect_metadata(&out.source);
    assert_eq!(inspected.identity_status, "valid");
    assert_eq!(
        inspected.id.as_deref(),
        Some("32eb9a12-e752-4bfa-8cdf-65e00f23ac26")
    );
    // Falling through to the existing native whitelist must not certify an
    // arbitrary function, binding mutation, import, include, or show prefix.
    for prefix in [
        "#unknown()\n",
        "#let title(body)=body\n",
        "#let metadata(value)=value\n",
        "#import \"other.typ\": *\n",
        "#include \"other.typ\"\n",
        "#show heading: it=>it\n",
        "#metadata((computed: 1+1)) <other>\n",
        "#metadata((x:1, \"x\":2)) <other>\n",
    ] {
        let guarded = format!("{prefix}{}", out.source);
        assert!(
            certify_metadata_declaration(&guarded, "weftext").is_none(),
            "{prefix}"
        );
    }
}

#[test]
fn ordinary_paragraph_runs_have_official_break_proofs_and_preserve_edge_trivia() {
    let source = "= 标题\n\n第一段\n\n第二段\n\n#table(columns:1,[cell])";
    let map = inspect_editor(source);
    let region = map
        .literal_regions
        .iter()
        .find(|r| r.role == "body")
        .unwrap();
    assert_eq!(region.text, "第一段\n\n第二段");
    assert_eq!(region.paragraph_breaks.len(), 1);
    assert_eq!(region.range.raw, "第一段\n\n第二段");
    let out = apply(
        source,
        EditIntent::ReplaceLiteral {
            range: region.range.clone(),
            text: "合并段落".into(),
        },
    );
    native(&out.source);
    assert!(out.source.starts_with("= 标题\n\n"));
    assert!(out.source.ends_with("\n\n#table(columns:1,[cell])"));
}

#[test]
fn native_cross_inline_selection_splits_wrappers_without_changing_other_text() {
    let source = "前缀 *粗体文字* 和 _斜体文字_ 后缀";
    let from = source.find("体文字").unwrap();
    let to = source.find("体文字_ 后缀").unwrap() + "体文字".len();
    let range = SourceRange {
        from,
        to,
        utf16_from: source[..from].encode_utf16().count(),
        utf16_to: source[..to].encode_utf16().count(),
        raw: source[from..to].into(),
        writable: true,
    };
    let out = apply(
        source,
        EditIntent::SetInline {
            range,
            format: InlineFormat::Strike,
        },
    );
    native(&out.source);
    assert!(out.source.starts_with("前缀 "));
    assert!(out.source.ends_with(" 后缀"));
    assert!(out.source.contains("#strike["));
    assert!(out.source.contains("#strong[粗]"));
    let function = "前缀 #strike[删除文字] 和 _斜体_ 后缀";
    let from = function.find("文字").unwrap();
    let to = function.find("斜体").unwrap() + "斜体".len();
    let range = SourceRange {
        from,
        to,
        utf16_from: function[..from].encode_utf16().count(),
        utf16_to: function[..to].encode_utf16().count(),
        raw: function[from..to].into(),
        writable: true,
    };
    let out = apply(
        function,
        EditIntent::SetInline {
            range,
            format: InlineFormat::Strong,
        },
    );
    native(&out.source);
    assert!(!out.source.contains("##"));
}

#[test]
fn literal_minimal_patch_preserves_soft_wrap_and_crlf_paragraph_bytes() {
    let source = "第一段\r\n同段换行\r\n\r\n第二段";
    let map = inspect_editor(source);
    let region = &map.literal_regions[0];
    assert_eq!(region.text, "第一段 同段换行\n\n第二段");
    let out = apply(
        source,
        EditIntent::ReplaceLiteral {
            range: region.range.clone(),
            text: format!("{}追加", region.text),
        },
    );
    assert_eq!(out.patches.len(), 1);
    assert_eq!(out.patches[0].from, source.len());
    assert_eq!(out.source, format!("{source}追加"));
    native(&out.source);
}

#[test]
fn native_string_codec_roundtrips_controls_quotes_and_unicode() {
    for text in ["中文😀\\\"", "line\n\r\ttab", "\u{0008}\u{000c}\u{001f}"] {
        let encoded = encode_string(text).unwrap();
        let parsed = typst_syntax::Source::detached(format!("#{encoded}"));
        let decoded = parsed
            .root()
            .children()
            .find_map(|n| n.cast::<typst_syntax::ast::Str>())
            .unwrap()
            .get();
        assert_eq!(decoded, text);
    }
}

#[test]
fn native_string_title_edits_and_deletion_preserve_document_author() {
    let source = "#set document(title: \"标题\", author: \"作者\")\n#title()\n\n正文";
    let map = inspect_editor(source);
    assert_eq!(map.title.status, "writable");
    let a = apply(
        source,
        EditIntent::SetTitle {
            text: Some("新标题".into()),
        },
    );
    native(&a.source);
    assert!(a.source.contains("title: \"新标题\""));
    let b = apply(&a.source, EditIntent::SetTitle { text: None });
    native(&b.source);
    assert!(b.source.contains("author: \"作者\""));
    assert!(!b.source.contains("title:"));
    let orphan = "#title[A]\n= 非副标题 <weftext-subtitle>";
    assert_eq!(
        inspect_editor(orphan).subtitle.status,
        "unsupported-role-label"
    );
}

#[test]
fn common_header_span_cells_have_unique_local_writers_without_grid_flattening() {
    let source =
        "#table(columns: 2, table.header([甲], [乙]), table.cell(colspan:2)[合并], [一], [二])";
    let map = inspect_editor(source);
    let table = map
        .structures
        .iter()
        .find(|s| s.kind == "table")
        .unwrap()
        .table
        .as_ref()
        .unwrap();
    assert!(!table.grid_writable);
    assert_eq!(table.cells.len(), 5);
    let cell = table.cells.iter().find(|c| c.raw == "合并").unwrap();
    let out = apply(
        source,
        EditIntent::SetTableCell {
            range: cell.clone(),
            text: "修改合并内容".into(),
        },
    );
    native(&out.source);
    assert!(out.source.contains("table.cell(colspan:2)[修改合并内容]"));
    assert!(out.source.contains("table.header([甲], [乙])"));
    for source in [
        "#table(columns:(1fr,2fr),[甲],[乙])",
        "#table(columns:2, /*行注释*/ [甲],[乙])",
    ] {
        let map = inspect_editor(source);
        assert!(
            !map.structures
                .iter()
                .find(|s| s.kind == "table")
                .unwrap()
                .table
                .as_ref()
                .unwrap()
                .grid_writable
        );
    }
}

#[test]
fn literal_codec_preserves_spaces_tabs_and_continuous_trailing_space() {
    let source = encode_literal("a  b\t ").unwrap();
    let map = inspect_editor(&source);
    assert_eq!(map.literal_regions[0].text, "a  b\t ");
    let out = apply(
        &source,
        EditIntent::ReplaceLiteral {
            range: map.literal_regions[0].range.clone(),
            text: "a  b\t 后续 ".into(),
        },
    );
    native(&out.source);
    assert_eq!(out.map.literal_regions[0].text, "a  b\t 后续 ");
    let empty = apply(
        "",
        EditIntent::ReplaceLiteral {
            range: inspect_editor("").literal_regions[0].range.clone(),
            text: "  ".into(),
        },
    );
    assert_eq!(empty.map.literal_regions[0].text, "  ");
}

#[test]
fn run_in_keeps_root_metadata_certified_and_rejects_custom_counter_scope() {
    let metadata =
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n";
    let source = format!("{metadata}== 方法 <methods:one>\n\n正文。");
    let map = inspect_editor(&source);
    let heading = map.structures.iter().find(|s| s.kind == "heading").unwrap();
    let out = apply(
        &source,
        EditIntent::SetRunIn {
            range: heading.range.clone(),
            enabled: true,
        },
    );
    native(&out.source);
    assert!(out.source.starts_with(metadata));
    assert_eq!(inspect_metadata(&out.source).identity_status, "valid");
    assert!(out.map.unavailable_reason.is_none());
    for prefix in [
        "#set heading(numbering: n => str(n))\n",
        "#counter(heading).update(0)\n",
    ] {
        let source = format!("{prefix}== 方法\n正文");
        let map = inspect_editor(&source);
        let heading = map.structures.iter().find(|s| s.kind == "heading").unwrap();
        assert!(
            edit(
                &source,
                &EditRequest {
                    source_hash: source_hash(&source),
                    intent: EditIntent::SetRunIn {
                        range: heading.range.clone(),
                        enabled: true
                    }
                }
            )
            .is_err()
        );
    }
}

#[test]
fn empty_and_retyped_list_slots_split_repeatedly_without_inverting_custom_boxes() {
    let a = apply(
        "- 甲",
        EditIntent::ListEdit {
            range: inspect_editor("- 甲").structures[0].range.clone(),
            action: ListAction::Split { at: 1 },
        },
    );
    let empty = a
        .map
        .structures
        .iter()
        .rfind(|s| s.kind == "bulletListItem")
        .unwrap();
    let b = apply(
        &a.source,
        EditIntent::ListEdit {
            range: empty.range.clone(),
            action: ListAction::Split { at: 0 },
        },
    );
    native(&b.source);
    assert_eq!(
        b.map
            .structures
            .iter()
            .filter(|s| s.kind == "bulletListItem")
            .count(),
        3
    );
    let slot = b
        .map
        .literal_regions
        .iter()
        .find(|r| r.text.is_empty())
        .unwrap();
    let c = apply(
        &b.source,
        EditIntent::ReplaceLiteral {
            range: slot.range.clone(),
            text: "中文😀".into(),
        },
    );
    let item = c
        .map
        .structures
        .iter()
        .find(|s| s.body.as_ref().is_some_and(|b| b.raw == "#box[中文😀]"))
        .unwrap();
    let d = apply(
        &c.source,
        EditIntent::ListEdit {
            range: item.range.clone(),
            action: ListAction::Split { at: 2 },
        },
    );
    native(&d.source);
    assert!(d.source.contains("- 中文\n- 😀"));
    for source in [
        "- #box(width:20pt)[正文]",
        "#let box(body)=body\n- #box[正文]",
    ] {
        let item = inspect_editor(source)
            .structures
            .into_iter()
            .find(|s| s.kind == "bulletListItem")
            .unwrap();
        assert!(
            edit(
                source,
                &EditRequest {
                    source_hash: source_hash(source),
                    intent: EditIntent::ListEdit {
                        range: item.range,
                        action: ListAction::Split { at: 1 }
                    }
                }
            )
            .is_err()
        );
    }
}

#[test]
fn native_raw_replacement_uses_adaptive_delimiters_and_preserves_neighbors() {
    let source = "前文 `旧代码` 后文\n\n```rust\nlet x=1;\n```";
    let map = inspect_editor(source);
    let raw = map
        .structures
        .iter()
        .find(|s| s.kind == "blockRaw")
        .unwrap();
    assert_eq!(raw.language.as_deref(), Some("rust"));
    let out = apply(
        source,
        EditIntent::ReplaceRaw {
            range: raw.range.clone(),
            text: "中文 `\n```\n结尾".into(),
            language: Some("typ".into()),
            block: true,
        },
    );
    native(&out.source);
    assert!(out.source.starts_with("前文 `旧代码` 后文\n\n"));
    assert!(out.source.contains("````typ"));
    let raw = out
        .map
        .structures
        .iter()
        .find(|s| s.kind == "blockRaw")
        .unwrap();
    assert_eq!(raw.language.as_deref(), Some("typ"));
    assert_eq!(raw.body.as_ref().unwrap().raw, "中文 `\n```\n结尾");
}

#[test]
fn source_heading_level_rule_disables_level_writer_without_losing_text_writer() {
    let source = "#set heading(offset:2)\n= 标题\n\n正文";
    let map = inspect_editor(source);
    let heading = map.structures.iter().find(|s| s.kind == "heading").unwrap();
    assert!(!heading.level_writable);
    assert_eq!(heading.level, Some(1));
    assert_eq!(
        heading
            .level_reason
            .as_ref()
            .map(|reason| &reason.explanation),
        Some(&UiMessage::error_engine_reason_heading_level_rule())
    );
    assert!(
        edit(
            source,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::SetHeadingLevel {
                    range: heading.range.clone(),
                    level: 2
                }
            }
        )
        .is_err()
    );
    assert!(map.literal_regions.iter().any(|r| r.text == "标题"));
    native(source);
}

#[test]
fn list_indent_requires_a_real_preceding_sibling() {
    let source = "- 甲\n- 乙";
    let map = inspect_editor(source);
    let first = map
        .structures
        .iter()
        .find(|s| s.kind == "bulletListItem")
        .unwrap();
    assert!(
        edit(
            source,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::ListEdit {
                    range: first.range.clone(),
                    action: ListAction::Indent
                }
            }
        )
        .is_err()
    );
    let second = map
        .structures
        .iter()
        .rfind(|s| s.kind == "bulletListItem")
        .unwrap();
    let out = apply(
        source,
        EditIntent::ListEdit {
            range: second.range.clone(),
            action: ListAction::Indent,
        },
    );
    native(&out.source);
    assert_eq!(out.source, "- 甲\n  - 乙");
    let second = out
        .map
        .structures
        .iter()
        .rfind(|s| s.kind == "bulletListItem")
        .unwrap();
    let restored = apply(
        &out.source,
        EditIntent::ListEdit {
            range: second.range.clone(),
            action: ListAction::Outdent,
        },
    );
    assert_eq!(restored.source, source);
}
