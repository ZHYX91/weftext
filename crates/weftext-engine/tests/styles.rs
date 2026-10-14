use std::{collections::BTreeMap, fs, sync::OnceLock};
use tempfile::TempDir;
use weftext_engine::*;

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::new_with_system_fonts(false).unwrap())
}

#[test]
fn library_inverse_requires_certified_single_passthrough() {
    let style = StyleModule {
        source: DEFAULT_LIBRARY_STYLE.into(),
        path: "styles/library-style.typ".into(),
        enabled: true,
        entry_function: "library-style".into(),
    };
    assert!(style_allows_rich_editing(&style));
    let dir = TempDir::new().unwrap();
    for source in [
        "#let library-style(body)={body; body}",
        "#let library-style(body)={show heading: it=>[变换]; body}",
    ] {
        let style = StyleModule {
            source: source.into(),
            ..style.clone()
        };
        assert!(!style_allows_rich_editing(&style));
        let map = inspect_editor_with_style("正文", &style);
        assert!(map.literal_regions.is_empty());
        assert_eq!(
            map.unavailable_reason.as_deref(),
            Some("unverified-library-style")
        );
        let out = engine()
            .render_with_style("正文", &dir.path().join("note.typ"), dir.path(), &style)
            .unwrap();
        assert!(out.ok, "{:?}", out.diagnostics);
        assert!(!text_node(out.tree.as_ref().unwrap(), "正文").is_some_and(|n| n.writable));
    }
}

#[test]
fn final_html_preserves_native_intentional_spaces_and_tabs() {
    let source = encode_literal("中  文\t  尾 \t").unwrap();
    let dir = TempDir::new().unwrap();
    let out = engine()
        .render(&source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(out.ok);
    let tree = out.tree.as_ref().unwrap();
    assert!(out.html.contains("white-space: pre-wrap"));
    assert_eq!(
        out.products.as_ref().unwrap().body.as_deref(),
        Some("中  文\t  尾 \t")
    );
    fn all_text_writable(node: &RenderNode) -> bool {
        (node.kind != "text" || node.writable) && node.children.iter().all(all_text_writable)
    }
    assert!(all_text_writable(tree), "{tree:?}");
}
fn text_node<'a>(node: &'a RenderNode, text: &str) -> Option<&'a RenderNode> {
    if node.kind == "text" && node.text.as_deref() == Some(text) {
        return Some(node);
    }
    node.children.iter().find_map(|n| text_node(n, text))
}
fn style_for_text<'a>(node: &'a RenderNode, text: &str) -> Option<&'a str> {
    if node
        .children
        .iter()
        .any(|c| c.kind == "text" && c.text.as_deref() == Some(text))
    {
        return node.attrs.get("style").map(String::as_str);
    }
    node.children.iter().find_map(|n| style_for_text(n, text))
}
fn node_tag<'a>(node: &'a RenderNode, tag: &str) -> Option<&'a RenderNode> {
    if node.tag.as_deref() == Some(tag) {
        return Some(node);
    }
    node.children.iter().find_map(|n| node_tag(n, tag))
}

#[test]
fn native_library_note_local_effective_styles_use_original_source() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("notes")).unwrap();
    fs::write(dir.path().join("notes/asset.typ"), "相对资源").unwrap();
    let module = StyleModule {
        path: "library-style.typ".into(),
        source: DEFAULT_LIBRARY_STYLE.into(),
        enabled: true,
        entry_function: "library-style".into(),
    };
    let source = "#set text(size: 14pt)\n正文 *强调* _斜体_\n\n#text(size: 20pt)[局部]\n\n#table(columns: 2, [甲], [乙])\n\n- 列表\n\n#include \"asset.typ\"";
    let out = engine()
        .render_with_style(
            source,
            &dir.path().join("notes/note.typ"),
            dir.path(),
            &module,
        )
        .unwrap();
    assert!(out.ok, "{:?}", out.diagnostics);
    let tree = out.tree.as_ref().unwrap();
    let normal = style_for_text(tree, "正文").unwrap();
    assert!(normal.contains("font-size: 14pt"), "{normal}");
    assert!(normal.contains("noto serif cjk sc"));
    let local = style_for_text(tree, "局部").unwrap();
    assert!(local.contains("font-size: 20pt"), "{local}");
    assert!(
        style_for_text(tree, "强调")
            .unwrap()
            .contains("font-weight: 700")
    );
    assert!(
        style_for_text(tree, "斜体")
            .unwrap()
            .contains("font-style: italic")
    );
    let cell = node_tag(tree, "td").unwrap();
    assert!(
        cell.attrs["style"].contains("border-top: 0.5pt"),
        "{:?}",
        cell.attrs
    );
    let list = node_tag(tree, "ul").unwrap();
    assert!(list.attrs["style"].contains("list-style-type: none"));
    assert!(out.html.contains('•'));
    let origin = text_node(tree, "局部").unwrap().origin.as_ref().unwrap();
    assert!(origin.is_main);
    assert_eq!(origin.path, "notes/note.typ");
    assert_eq!(&source[origin.from..origin.to], "局部");
    assert!(
        out.dependencies
            .iter()
            .any(|d| d.path == "notes/asset.typ" && d.status == "loaded")
    );
    assert!(
        out.dependencies
            .iter()
            .any(|d| d.path == "library-style.typ")
    );
    let broken = StyleModule {
        source: "#let library-style(body) = panic(\"bad module\")".into(),
        ..module.clone()
    };
    assert!(
        !engine()
            .render_with_style(
                source,
                &dir.path().join("notes/note.typ"),
                dir.path(),
                &broken
            )
            .unwrap()
            .ok
    );
    let disabled = StyleModule {
        enabled: false,
        ..broken
    };
    assert!(
        engine()
            .render_with_style(
                "可编辑",
                &dir.path().join("notes/note.typ"),
                dir.path(),
                &disabled
            )
            .unwrap()
            .ok
    );
}

#[test]
fn style_controls_certify_unique_direct_native_default_slots() {
    let map = inspect_library_style(DEFAULT_LIBRARY_STYLE);
    assert!(map.syntax_valid);
    assert_eq!(
        map.controls.iter().filter(|c| c.editable).count(),
        7,
        "{map:?}"
    );
    let mut changes = BTreeMap::new();
    changes.insert("text.size".into(), "13pt".into());
    changes.insert(
        "raw.font".into(),
        "(\"DejaVu Sans Mono\", \"Noto Sans Mono CJK TC\")".into(),
    );
    let out =
        apply_library_style_changes(DEFAULT_LIBRARY_STYLE, &map.source_hash, &changes).unwrap();
    assert_eq!(out.patches.len(), 2);
    assert!(out.source.contains("size: 13pt"));
    assert!(out.source.contains("set table(stroke: 0.5pt)"));
    changes.insert("text.size".into(), "13pt); panic(\"injection\")".into());
    assert!(
        apply_library_style_changes(DEFAULT_LIBRARY_STYLE, &map.source_hash, &changes).is_err()
    );
    let duplicate = "#let library-style(body) = {set text(size: 11pt); set text(size: 12pt); body}";
    assert!(
        !inspect_library_style(duplicate)
            .controls
            .iter()
            .find(|c| c.key == "text.size")
            .unwrap()
            .editable
    );
    let nested = "#let library-style(body) = {let nested() = {set text(size: 12pt); []}; body}";
    assert!(
        !inspect_library_style(nested)
            .controls
            .iter()
            .find(|c| c.key == "text.size")
            .unwrap()
            .editable
    );
    let shadowed = "#let library-style(text, body) = {set text(size: 12pt); body}";
    assert!(
        !inspect_library_style(shadowed)
            .controls
            .iter()
            .find(|c| c.key == "text.size")
            .unwrap()
            .editable
    );
}

#[test]
fn bundled_fonts_cover_chinese_raw_and_math_without_host_fonts() {
    let manifest = bundled_font_manifest();
    for entry in manifest["fonts"].as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        let data = bundled_font_asset(file).unwrap();
        assert_eq!(data.len() as u64, entry["bytes"].as_u64().unwrap());
        let font = typst::text::Font::new(typst::foundations::Bytes::new(data), 0).unwrap();
        let instance = font.clone().instantiate(
            typst::text::FontVariant::default(),
            typst::layout::Abs::pt(11.),
            &typst::text::FontVariations::default(),
        );
        if file.starts_with("Noto") {
            for character in "简体中文繁體漢字總價單價數量，。！？“”‘’（）【】《》—…".chars()
            {
                assert!(
                    instance.ttf().glyph_index(character).is_some(),
                    "{file} missing {character}"
                );
            }
        } else if entry["role"] == "math" {
            assert_eq!(font.info().family, "New Computer Modern Math");
            assert!(instance.ttf().tables().math.is_some());
        }
    }
    let dir = TempDir::new().unwrap();
    let style = StyleModule {
        path: "library-style.typ".into(),
        source: DEFAULT_LIBRARY_STYLE.into(),
        enabled: true,
        entry_function: "library-style".into(),
    };
    let source = "= 简繁標題\n\n普通中文。\n\n`代码中文 ABC`\n\n```python\n# 中文注释\nvalue = \"漢字\"\n```\n\n$ \"总价\" = \"单价\" times \"数量\" + frac(1,2) $\n\n#table(columns:2,[甲],[乙])";
    let html = engine()
        .render_with_style(source, &dir.path().join("note.typ"), dir.path(), &style)
        .unwrap();
    assert!(html.ok, "{:?}", html.diagnostics);
    assert!(html.html.contains("noto sans mono cjk sc"));
    assert!(html.html.contains("<math"));
    assert!(html.html.contains("总价"));
    let paged = engine()
        .render_paged_with_style(source, &dir.path().join("note.typ"), dir.path(), &style)
        .unwrap();
    assert!(paged.ok, "{:?}", paged.diagnostics);
    assert!(paged.pdf_base64.is_some());
}

#[test]
fn compiled_label_inventory_includes_generated_and_duplicate_occurrences() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("inc.typ"), "= 相同 <same>").unwrap();
    let out = engine()
        .render(
            "#include \"inc.typ\"\n#include \"inc.typ\"\n#heading[生成] <generated>",
            &dir.path().join("note.typ"),
            dir.path(),
        )
        .unwrap();
    assert!(out.ok, "{:?}", out.diagnostics);
    assert_eq!(
        out.labels.iter().find(|l| l.label == "same").unwrap().count,
        2
    );
    assert_eq!(
        out.labels
            .iter()
            .find(|l| l.label == "generated")
            .unwrap()
            .count,
        1
    );
}

#[test]
fn unsafe_author_css_attribute_remains_unavailable() {
    let dir = TempDir::new().unwrap();
    let out=engine().render("#html.elem(\"span\", attrs:(style:\"position:fixed; background:url(file:///secret)\"))[文字]",&dir.path().join("note.typ"),dir.path()).unwrap();
    assert!(out.ok);
    assert!(!out.html.contains("file:///secret"));
    assert!(!out.html.contains("position:fixed"));
}

#[test]
fn unsupported_native_paint_and_stroke_are_explicit_flow_diagnostics() {
    let dir = TempDir::new().unwrap();
    for source in [
        "#set text(fill: gradient.linear(red, blue))\n渐变文字",
        "#table(columns:1,stroke:(dash:\"dashed\", thickness:1pt),[单格])",
    ] {
        let out = engine()
            .render(source, &dir.path().join("note.typ"), dir.path())
            .unwrap();
        assert!(out.ok, "{source}\n{:?}", out.diagnostics);
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.message.contains("no flow style adapter")),
            "{:?}",
            out.diagnostics
        );
    }
}
