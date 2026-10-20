use std::{fs, sync::OnceLock};
use tempfile::TempDir;
use weftext_engine::{Engine, RenderNode};

const MODULE: &str = include_str!("../assets/weftext-endnotes.typ");
const IMPORT: &str = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n";

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::new_with_system_fonts(false).unwrap())
}

fn bundle() -> TempDir {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("weftext-endnotes.typ"), MODULE).unwrap();
    root
}

fn all_text(node: &RenderNode) -> String {
    let mut text = node.text.clone().unwrap_or_default();
    for child in &node.children {
        text.push_str(&all_text(child));
    }
    text
}

fn note_link_numbers(node: &RenderNode) -> Vec<String> {
    let mut numbers = vec![];
    if node.tag.as_deref() == Some("a")
        && node
            .attrs
            .get("href")
            .is_some_and(|href| href.contains("weftext-endnote-body-"))
    {
        numbers.push(all_text(node));
    }
    for child in &node.children {
        numbers.extend(note_link_numbers(child));
    }
    numbers
}

fn with_role<'a>(node: &'a RenderNode, role: &str) -> Vec<&'a RenderNode> {
    let mut found = vec![];
    if node.attrs.get("data-weftext-role").map(String::as_str) == Some(role) {
        found.push(node);
    }
    for child in &node.children {
        found.extend(with_role(child, role));
    }
    found
}

#[test]
fn native_endnotes_render_chinese_empty_multiple_paragraphs_and_references() {
    let root = bundle();
    let source = format!(
        "{IMPORT}正文#footnote[独立脚注] 与尾注#endnote(id: \"first\")[中文尾注 *强调*。\n\n第二段，#link(\"https://example.com\")[链接]。\n\n- 一项\n- 二项\n\n$x^2 + y^2$]\n\n再次#endnote-ref(\"first\")，空尾注#endnote(id: \"empty\")[]，同文独立#endnote(id: \"equal-a\")[同文] 和 #endnote(id: \"equal-b\")[同文]。再一个#footnote[另一脚注]。#context [脚注总数#counter(footnote).display(\"1\")]\n\n#endnotes()"
    );
    let html = engine()
        .render(&source, &root.path().join("note.typ"), root.path())
        .unwrap();
    assert!(html.ok, "{:?}", html.diagnostics);
    let text = all_text(html.tree.as_ref().unwrap());
    assert_eq!(text.matches("中文尾注").count(), 1, "{text}");
    assert_eq!(text.matches("第二段").count(), 1, "{text}");
    assert_eq!(text.matches("独立脚注").count(), 1, "{text}");
    assert_eq!(text.matches("另一脚注").count(), 1, "{text}");
    assert!(text.contains("脚注总数2"), "{text}");
    assert_eq!(text.matches("同文").count(), 3, "{text}");
    assert!(html.html.contains("<strong"), "{}", html.html);
    assert!(html.html.contains("https://example.com"), "{}", html.html);
    assert!(html.html.contains("<ul"), "{}", html.html);
    assert_eq!(
        note_link_numbers(html.tree.as_ref().unwrap()),
        ["1", "1", "2", "3", "4"]
    );
    let tree = html.tree.as_ref().unwrap();
    let references = with_role(tree, "endnote-reference");
    assert_eq!(references.len(), 5);
    assert_eq!(
        references
            .iter()
            .filter(|reference| reference.attrs["data-weftext-note-id"] == "first")
            .count(),
        2
    );
    let bodies = with_role(tree, "endnote-body");
    assert_eq!(bodies.len(), 4);
    assert!(
        all_text(
            bodies
                .iter()
                .find(|body| body.attrs["data-weftext-note-id"] == "empty")
                .unwrap()
        )
        .is_empty()
    );
    assert_eq!(with_role(tree, "endnotes").len(), 1);
    assert_eq!(
        html.labels
            .iter()
            .find(|label| label.label == "weftext-endnote-definition")
            .unwrap()
            .count,
        4
    );
    assert_eq!(
        html.labels
            .iter()
            .find(|label| label.label == "weftext-endnote-reference")
            .unwrap()
            .count,
        5
    );
    let paged = engine()
        .render_paged(&source, &root.path().join("note.typ"), root.path())
        .unwrap();
    assert!(paged.ok, "{:?}", paged.diagnostics);
    assert!(paged.page_count > 0);
    assert!(
        paged
            .pdf_base64
            .as_ref()
            .is_some_and(|pdf| pdf.starts_with("JVBER"))
    );
}

#[test]
fn changed_module_keeps_display_without_claiming_frozen_interchange_roles() {
    let root = bundle();
    fs::write(
        root.path().join("weftext-endnotes.typ"),
        format!("{MODULE}\n// author changed the bundled module\n"),
    )
    .unwrap();
    let source = format!("{IMPORT}正文#endnote(id: \"note\")[保留正文]\n\n#endnotes()");
    let html = engine()
        .render(&source, &root.path().join("note.typ"), root.path())
        .unwrap();
    assert!(html.ok, "{:?}", html.diagnostics);
    let tree = html.tree.as_ref().unwrap();
    assert!(all_text(tree).contains("保留正文"));
    assert!(with_role(tree, "endnote-reference").is_empty());
    assert!(with_role(tree, "endnote-body").is_empty());
    assert!(with_role(tree, "endnotes").is_empty());
}

#[test]
fn native_endnotes_reject_duplicate_and_missing_definitions() {
    for body in [
        "#endnote(id: \"same\")[一] #endnote(id: \"same\")[二]\n\n#endnotes()",
        "#endnote-ref(\"missing\")\n\n#endnotes()",
        "#endnote(id: \"\")[空键]\n\n#endnotes()",
        "#endnote(id: \"a\")[未显示列表]",
        "#endnote(id: \"a\")[重复列表]\n\n#endnotes()\n\n#endnotes()",
    ] {
        let root = bundle();
        let source = format!("{IMPORT}{body}");
        let html = engine()
            .render(&source, &root.path().join("note.typ"), root.path())
            .unwrap();
        assert!(!html.ok, "{body}");
        assert!(
            html.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == "error")
        );
        let paged = engine()
            .render_paged(&source, &root.path().join("note.typ"), root.path())
            .unwrap();
        assert!(!paged.ok, "{body}");
    }
}
