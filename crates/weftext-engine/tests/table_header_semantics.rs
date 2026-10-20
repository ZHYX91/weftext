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

fn header_repeats(node: &RenderNode) -> Vec<&str> {
    let mut repeats = Vec::new();
    fn visit<'a>(node: &'a RenderNode, repeats: &mut Vec<&'a str>) {
        if node.attrs.get("data-weftext-role").map(String::as_str) == Some("table-header-row") {
            assert_eq!(node.tag.as_deref(), Some("tr"));
            repeats.push(node.attrs["data-weftext-repeat"].as_str());
        } else {
            assert!(!node.attrs.contains_key("data-weftext-repeat"));
        }
        for child in &node.children {
            visit(child, repeats);
        }
    }
    visit(node, &mut repeats);
    repeats
}

#[test]
fn native_table_headers_preserve_each_resolved_repeat_request() {
    for (source, expected) in [
        (
            "#table(columns:2, table.header([甲],[乙]), [丙],[丁])",
            vec!["true"],
        ),
        (
            "#table(columns:2, table.header(repeat:false,[甲],[乙],[丙],[丁]), [戊],[己])",
            vec!["false", "false"],
        ),
        (
            "#set table.header(repeat:false)\n#table(columns:1, table.header([甲]), table.header(repeat:true)[乙], [丙])",
            vec!["false", "true"],
        ),
        (
            "#table(columns:1, row-gutter:1pt, table.header([甲]), [乙], table.header(repeat:false)[丙], [丁], table.header[戊], [己])",
            vec!["true", "false", "true"],
        ),
    ] {
        let tree = render(source);
        assert_eq!(header_repeats(&tree), expected, "{source}\n{tree:?}");
    }
}

#[test]
fn author_html_and_custom_table_show_cannot_forge_header_repetition() {
    for source in [
        "#html.elem(\"table\")[#html.elem(\"thead\")[#html.elem(\"tr\",attrs:(\"data-weftext-role\":\"table-header-row\",\"data-weftext-repeat\":\"true\"))[#html.elem(\"th\")[伪造]]]]",
        "#show table: it => html.elem(\"table\")[#html.elem(\"thead\")[#html.elem(\"tr\")[#html.elem(\"th\")[替换]]]]\n#table(columns:1,table.header[原表头],[正文])",
        "#table(columns:1,[普通第一行],[普通第二行])",
    ] {
        let tree = render(source);
        assert!(header_repeats(&tree).is_empty(), "{source}\n{tree:?}");
    }
}
