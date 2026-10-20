use weftext_engine::*;

fn run(source: &str, intent: EditIntent) -> Result<EditOutput, EngineError> {
    edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent,
        },
    )
}

#[test]
fn convert_one_root_block_preserves_unicode_formatting_and_all_other_bytes() {
    let source = "#metadata((note: true))\r\n\r\n第一 *段*😀\r\n\r\n= 标题\r\n\r\n尾段";
    let map = inspect_editor(source);
    assert_eq!(map.body_blocks.len(), 3);
    let block = &map.body_blocks[0];
    let out = run(
        source,
        EditIntent::ConvertBodyBlock {
            range: block.range.clone(),
            target: "heading".into(),
            level: Some(2),
        },
    )
    .unwrap();
    assert_eq!(
        out.source,
        source.replacen("第一 *段*😀", "== 第一 *段*😀", 1)
    );
    assert_eq!(out.patches.len(), 1);
    let heading = &inspect_editor(&out.source).body_blocks[0];
    let back = run(
        &out.source,
        EditIntent::ConvertBodyBlock {
            range: heading.range.clone(),
            target: "paragraph".into(),
            level: None,
        },
    )
    .unwrap();
    assert_eq!(back.source, source);
}

#[test]
fn move_only_adjacent_blank_separated_blocks_preserves_eol_and_prefix() {
    let source = "#metadata((note: true))\r\n\r\n前段😀\r\n\r\n= 标题\r\n\r\n尾段";
    let map = inspect_editor(source);
    let out = run(
        source,
        EditIntent::MoveBodyBlock {
            range: map.body_blocks[1].range.clone(),
            direction: "up".into(),
        },
    )
    .unwrap();
    assert_eq!(
        out.source,
        source.replace("前段😀\r\n\r\n= 标题", "= 标题\r\n\r\n前段😀")
    );
    let map = inspect_editor(&out.source);
    let back = run(
        &out.source,
        EditIntent::MoveBodyBlock {
            range: map.body_blocks[0].range.clone(),
            direction: "down".into(),
        },
    )
    .unwrap();
    assert_eq!(back.source, source);
}

#[test]
fn stale_nested_generated_labels_and_rule_barriers_never_gain_block_writers() {
    for source in ["#let f() = [生成]\n#f()", "#block[嵌套]", "= 标题 <label>"] {
        assert!(inspect_editor(source).body_blocks.is_empty(), "{source}");
    }
    let source = "前段\n\n// 保留注释\n\n后段";
    let map = inspect_editor(source);
    assert!(
        run(
            source,
            EditIntent::MoveBodyBlock {
                range: map.body_blocks[0].range.clone(),
                direction: "down".into()
            }
        )
        .is_err()
    );
    let source = "普通段落";
    let block = inspect_editor(source).body_blocks.remove(0);
    let request = EditRequest {
        source_hash: source_hash(source),
        intent: EditIntent::ConvertBodyBlock {
            range: block.range,
            target: "heading".into(),
            level: Some(1),
        },
    };
    assert!(edit("已改稿", &request).is_err());
}

#[test]
fn conversion_refuses_multiline_or_overridden_heading_semantics() {
    for source in [
        "第一行\n第二行",
        "#set heading(offset: 2)\n\n正文",
        "#let heading = text\n\n正文",
    ] {
        let map = inspect_editor(source);
        let block = map.body_blocks.last().unwrap();
        assert!(
            run(
                source,
                EditIntent::ConvertBodyBlock {
                    range: block.range.clone(),
                    target: "heading".into(),
                    level: Some(1)
                }
            )
            .is_err(),
            "{source}"
        );
    }
}

#[test]
fn image_path_is_encoded_as_one_native_string_and_shadowed_binding_refuses() {
    let source = "正文";
    let out = run(
        source,
        EditIntent::InsertBlock {
            at: source.len(),
            block: BlockInsert::Image {
                path: "assets/中文\"#图片.png".into(),
                alt: "图片".into(),
            },
        },
    )
    .unwrap();
    assert!(
        out.source
            .contains("#image(\"assets/中文\\\"#图片.png\", alt: \"图片\")")
    );
    let source = "#let image = text\n\n正文";
    assert!(
        run(
            source,
            EditIntent::InsertBlock {
                at: source.len(),
                block: BlockInsert::Image {
                    path: "a.png".into(),
                    alt: "".into()
                }
            }
        )
        .is_err()
    );
}
