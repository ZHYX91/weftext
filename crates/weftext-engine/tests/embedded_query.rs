use tempfile::TempDir;
use weftext_engine::{Engine, RenderNode, embedded_query_sources};

const QUERY: &str = "#metadata((kind: \"weftext.query\", key: \"f8597b15-6b7f-4a6a-9b52-c2732730fe41\", where: \"true\")) <weftext-query-one>";

#[test]
fn embedded_query_range_retains_complete_author_bytes_and_unicode_coordinates() {
    let source = format!("前😀\r\n\n{QUERY}\n\n后");
    let found = embedded_query_sources(&source);
    assert_eq!(found.len(), 1);
    let found = &found[0];
    assert_eq!(found.range.raw, QUERY);
    assert_eq!(&source[found.range.from..found.range.to], QUERY);
    assert_eq!(
        found.range.utf16_from,
        source[..found.range.from].encode_utf16().count()
    );
    assert_eq!(found.call_from, found.range.from + 1);
    assert!(found.range.writable);
    assert_ne!(
        found.id,
        embedded_query_sources(&format!("{source}!"))[0].id
    );
}

#[test]
fn invalid_duplicate_nested_and_include_sources_never_supply_a_local_writer() {
    for source in [
        format!("{QUERY}\n{QUERY}"),
        format!("#if true [{QUERY}]"),
        format!("{QUERY}\n#include \"a.typ\""),
        format!("{QUERY}\n#let broken = ("),
    ] {
        assert!(
            embedded_query_sources(&source)
                .iter()
                .all(|q| !q.range.writable),
            "{source}"
        );
    }
}

fn flatten<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    out.push(node);
    for child in &node.children {
        flatten(child, out);
    }
}

#[test]
fn query_marker_stays_at_actual_native_tree_position_and_out_of_body_products() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let source = format!("之前\n\n{QUERY}\n\n之后");
    let rendered = engine
        .render(&source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(rendered.ok, "{:?}", rendered.diagnostics);
    let mut nodes = vec![];
    flatten(rendered.tree.as_ref().unwrap(), &mut nodes);
    let index = nodes
        .iter()
        .position(|n| {
            n.attrs
                .get("data-weftext-query-label")
                .is_some_and(|label| label == "weftext-query-one")
        })
        .expect("native metadata tag");
    assert!(
        nodes[..index]
            .iter()
            .any(|n| n.text.as_deref().is_some_and(|t| t.contains("之前")))
    );
    assert!(
        nodes[index + 1..]
            .iter()
            .any(|n| n.text.as_deref().is_some_and(|t| t.contains("之后")))
    );
    let origin = nodes[index].origin.as_ref().unwrap();
    assert!(origin.is_main);
    assert_eq!(origin.from, embedded_query_sources(&source)[0].call_from);
    assert_eq!(nodes[index].kind, "tag");
    assert!(!nodes[index].writable);
    assert!(!rendered.products.unwrap().body.unwrap().contains("status"));
    assert!(!rendered.html.contains("weftext-query-one"));
}
