use std::sync::OnceLock;

use tempfile::TempDir;
use weftext_engine::{Engine, RenderNode};

fn render(source: &str) -> RenderNode {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    let engine = ENGINE.get_or_init(|| Engine::new_with_system_fonts(false).unwrap());
    let dir = TempDir::new().unwrap();
    let output = engine
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(output.ok, "{source}\n{:?}", output.diagnostics);
    output.tree.unwrap()
}

#[derive(Debug)]
struct Run {
    text: String,
    language: String,
    source: String,
    carrier_style: String,
}

fn runs(source: &str, tree: &RenderNode) -> Vec<Run> {
    fn collect(source: &str, node: &RenderNode, language: &str, style: &str, out: &mut Vec<Run>) {
        if node.kind == "tag" {
            return;
        }
        let language = node
            .attrs
            .get("lang")
            .map(String::as_str)
            .unwrap_or(language);
        let style = node.attrs.get("style").map(String::as_str).unwrap_or(style);
        if let Some(text) = &node.text {
            let raw = node
                .origin
                .as_ref()
                .filter(|origin| origin.is_main)
                .map(|origin| source[origin.from..origin.to].to_owned())
                .unwrap_or_default();
            out.push(Run {
                text: text.clone(),
                language: language.to_owned(),
                source: raw,
                carrier_style: style.to_owned(),
            });
        }
        for child in &node.children {
            collect(source, child, language, style, out);
        }
    }
    let mut out = Vec::new();
    collect(source, tree, "", "", &mut out);
    out
}

#[test]
fn actual_space_and_literal_whitespace_keep_local_native_language_and_source() {
    let source = "#set text(lang:\"zh\",region:\"CN\")\n#text(lang:\"en\",region:\"US\")[Color\\u{20}English Normal\\u{9}Tab]";
    let tree = render(source);
    let actual = runs(source, &tree);
    assert_eq!(
        actual
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Color English Normal\tTab",
        "{actual:?}"
    );
    assert!(
        actual.iter().all(|run| run.language == "en-US"),
        "{actual:?}"
    );
    // Native realization may coalesce an ordinary space with adjacent text.
    // Its complete bytes and effective language must survive either grouping.
    assert!(
        actual
            .iter()
            .any(|run| run.source.contains(' ') && run.text.contains(' ')),
        "{actual:?}"
    );
    for (raw, text) in [("\\u{20}", " "), ("\\u{9}", "\t")] {
        let literal = actual.iter().find(|run| run.source == raw).unwrap();
        assert_eq!(literal.text, text);
        assert!(
            literal.carrier_style.contains("white-space: pre-wrap"),
            "{literal:?}"
        );
    }
}

#[test]
fn explicit_absent_region_applies_to_spaces_and_literal_whitespace() {
    let source =
        "#set text(lang:\"en\",region:\"US\")\n#text(lang:\"zh\",region:none)[无\\u{20}地区 普通]";
    let tree = render(source);
    let actual = runs(source, &tree);
    assert_eq!(
        actual
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "无 地区 普通",
        "{actual:?}"
    );
    assert!(actual.iter().all(|run| run.language == "zh"), "{actual:?}");
    assert_eq!(
        actual
            .iter()
            .flat_map(|run| run.text.chars())
            .filter(|character| *character == ' ')
            .count(),
        2,
        "{actual:?}"
    );
    assert!(
        actual.iter().any(|run| run.source == "\\u{20}"),
        "{actual:?}"
    );
}

#[test]
fn ordinary_space_between_native_style_boundaries_keeps_its_own_language() {
    let source = "#set text(lang:\"zh\",region:\"CN\")\n#text(lang:\"en\",region:\"US\")[#strong[A] #emph[B]]";
    let tree = render(source);
    let actual = runs(source, &tree);
    assert_eq!(
        actual
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "A B",
        "{actual:?}"
    );
    assert!(
        actual.iter().all(|run| run.language == "en-US"),
        "{actual:?}"
    );
    let normal = actual
        .iter()
        .find(|run| run.source == " " && run.text == " ")
        .unwrap_or_else(|| {
            panic!("Expected the actual native space between style boundaries: {actual:?}")
        });
    assert!(!normal.carrier_style.contains("white-space"), "{normal:?}");
}
