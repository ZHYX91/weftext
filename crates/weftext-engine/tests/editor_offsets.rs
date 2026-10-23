use serde_json::Value;
use weftext_engine::inspect_editor;
use weftext_messages::{ErrorDescription, UiMessage};

fn assert_ranges(source: &str, value: &Value, path: &str, visited: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let (Some(from), Some(to), Some(utf16_from), Some(utf16_to), Some(raw)) = (
                object.get("from").and_then(Value::as_u64),
                object.get("to").and_then(Value::as_u64),
                object.get("utf16From").and_then(Value::as_u64),
                object.get("utf16To").and_then(Value::as_u64),
                object.get("raw").and_then(Value::as_str),
            ) {
                let (from, to) = (from as usize, to as usize);
                assert!(
                    from <= to && source.is_char_boundary(from) && source.is_char_boundary(to),
                    "{path}: {from}..{to}"
                );
                assert_eq!(raw, &source[from..to], "{path}");
                assert_eq!(
                    utf16_from as usize,
                    source[..from].encode_utf16().count(),
                    "{path}"
                );
                assert_eq!(
                    utf16_to as usize,
                    source[..to].encode_utf16().count(),
                    "{path}"
                );
                visited.push(path.into());
            }
            for (key, child) in object {
                assert_ranges(source, child, &format!("{path}.{key}"), visited);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                assert_ranges(source, child, &format!("{path}[{index}]"), visited);
            }
        }
        _ => {}
    }
}

fn checked_map(source: &str) -> (Value, Vec<String>) {
    let map = serde_json::to_value(inspect_editor(source)).unwrap();
    let mut visited = vec![];
    assert_ranges(source, &map, "$", &mut visited);
    (map, visited)
}

#[test]
fn every_editor_range_keeps_original_byte_utf16_and_crlf_coordinates() {
    let source = concat!(
        "#set document(title: [文档😀])\r\n#title()\r\n",
        "#block[副题𠮷] <weftext-subtitle>\r\n",
        "= 标题e\u{301}😀\r\n\r\n",
        "前😀\r\n\r\n后𠮷e\u{301}\r\n\r\n终中\r\n\r\n",
        "*加粗😀* _强调𠮷_ #strike[划线中] #[包装😀]\r\n\r\n",
        "/ 术语😀: 解释𠮷\r\n- 项目中\r\n+ 有序😀\r\n\r\n",
        "`代码😀`\r\n\r\n```typ\r\n原始𠮷\r\n```\r\n\r\n",
        "$ x + 1 $\r\n\r\n",
        "#table(columns: 2, [格😀], [])\r\n\r\n",
        "#table(columns: 2, table.header([表头😀], [表头𠮷]), table.cell(colspan: 2)[跨格e\u{301}])\r\n\r\n",
        "#table(columns: 2, // 原注释😀\r\n[保留𠮷], [邻格中])\r\n\r\n",
        "正文#footnote[注😀] <note> 重复#footnote(<note>)\r\n",
        "#figure(table(columns: 1, [内容𠮷]), caption: [题注😀])\r\n",
    );
    let (map, visited) = checked_map(source);
    assert_eq!(map["syntaxValid"], true);
    assert!(map["unavailableReason"].is_null(), "{map}");
    for branch in [
        "literalRegions[",
        ".paragraphBreaks[",
        "structures[",
        ".table.cells[",
        ".table.logicalCells[",
        "$.title.range",
        "$.title.construct",
        "$.subtitle.range",
        "$.subtitle.construct",
        "$.notes.entries[",
        "$.bodyInsertion",
    ] {
        assert!(
            visited.iter().any(|path| path.contains(branch)),
            "range branch absent: {branch}; {visited:?}"
        );
    }
    assert!(
        visited
            .iter()
            .any(|path| path.contains(".table.logicalCells[") && path.ends_with(".body"))
    );
    assert!(
        visited
            .iter()
            .any(|path| path.contains("$.notes.entries[") && path.ends_with(".body"))
    );
    let tables: Vec<_> = map["structures"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|structure| structure.get("table").filter(|table| !table.is_null()))
        .collect();
    assert_eq!(tables.len(), 3);
    assert!(tables.iter().any(|table| table["topologyWritable"] == false
        && !table["cells"].as_array().unwrap().is_empty()));
    assert!(tables.iter().any(|table| {
        table["cells"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cell| cell["raw"] == "")
    }));
}

#[test]
fn empty_ascii_unicode_and_invalid_source_keep_all_present_ranges_accurate() {
    for source in [
        "",
        "ASCII\r\n",
        "中😀𠮷e\u{301}\r\n\r\n尾",
        "#table(columns: 1, [])",
        "= 中😀\r\n#let broken = (\r\n",
    ] {
        let (map, _) = checked_map(source);
        if source.contains("broken") {
            assert_eq!(map["syntaxValid"], false);
            let reason: ErrorDescription =
                serde_json::from_value(map["unavailableReason"].clone()).unwrap();
            assert_eq!(
                reason,
                ErrorDescription::from(UiMessage::error_engine_reason_syntax_invalid())
            );
        } else {
            assert_eq!(map["syntaxValid"], true, "{source:?}");
        }
    }
}
