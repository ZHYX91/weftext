use std::collections::{BTreeMap, BTreeSet};
use tempfile::TempDir;
use weftext_engine::*;

fn check_origin(origin: &Origin, sources: &BTreeMap<String, String>, seen: &mut BTreeSet<String>) {
    let source = sources
        .get(&origin.path)
        .unwrap_or_else(|| panic!("unmapped origin: {origin:?}"));
    assert!(source.is_char_boundary(origin.from) && source.is_char_boundary(origin.to));
    assert_eq!(origin.raw, source[origin.from..origin.to]);
    assert_eq!(
        origin.utf16_from,
        source[..origin.from].encode_utf16().count()
    );
    assert_eq!(origin.utf16_to, source[..origin.to].encode_utf16().count());
    assert_eq!(origin.is_main, origin.path == "notes/主😀.typ");
    seen.insert(origin.path.clone());
}

fn check_tree(node: &RenderNode, sources: &BTreeMap<String, String>, seen: &mut BTreeSet<String>) {
    assert!(
        !node.writable,
        "complex style must not gain a writer: {node:?}"
    );
    if let Some(origin) = &node.origin {
        check_origin(origin, sources, seen);
    }
    for child in &node.children {
        check_tree(child, sources, seen);
    }
}

#[test]
fn actual_main_import_and_style_origins_keep_their_own_unicode_coordinates() {
    let dir = TempDir::new().unwrap();
    let main = "主😀𠮷e\u{301}\r\n\r\n#include \"辅😀.typ\"\r\n\r\n= 标题𠮷 <chapter>\r\n";
    let imported = "辅😀e\u{301}\r\n\r\n第二段𠮷\r\n";
    let style = StyleModule {
        path: "styles/样式😀.typ".into(),
        source: "#let library-style(body) = [\r\n样式😀𠮷e\u{301}\r\n\r\n#body\r\n]".into(),
        enabled: true,
        entry_function: "library-style".into(),
    };
    assert!(!style_allows_rich_editing(&style));
    let input = FrozenRenderRequest {
        source: main.into(),
        path: dir.path().join("notes/主😀.typ"),
        root: dir.path().into(),
        files: BTreeMap::from([("notes/辅😀.typ".into(), imported.as_bytes().to_vec())]),
        style: Some(style.clone()),
    };
    let output = Engine::new_with_system_fonts(false)
        .unwrap()
        .render_frozen(&input)
        .unwrap();
    assert!(output.ok, "{:?}", output.diagnostics);
    let sources = BTreeMap::from([
        ("notes/主😀.typ".into(), main.into()),
        ("notes/辅😀.typ".into(), imported.into()),
        (style.path, style.source),
    ]);
    let mut seen = BTreeSet::new();
    check_tree(output.tree.as_ref().unwrap(), &sources, &mut seen);
    for label in output.labels {
        for origin in label.origins {
            check_origin(&origin, &sources, &mut seen);
        }
    }
    for diagnostic in output.diagnostics {
        if let Some(origin) = diagnostic.origin {
            check_origin(&origin, &sources, &mut seen);
        }
    }
    assert_eq!(seen, sources.keys().cloned().collect());
    assert_eq!(output.source_info.writer_status, "unverified-library-style");
}

#[test]
fn actual_style_diagnostic_keeps_its_imported_unicode_source_range() {
    let dir = TempDir::new().unwrap();
    let style = StyleModule {
        path: "styles/坏😀.typ".into(),
        source: "// 前😀𠮷\r\n#let library-style(body) = { 1 + \"错误e\u{301}\" }".into(),
        enabled: true,
        entry_function: "library-style".into(),
    };
    let input = FrozenRenderRequest {
        source: "主😀\r\n".into(),
        path: dir.path().join("notes/主😀.typ"),
        root: dir.path().into(),
        files: BTreeMap::new(),
        style: Some(style.clone()),
    };
    let output = Engine::new_with_system_fonts(false)
        .unwrap()
        .render_frozen(&input)
        .unwrap();
    assert!(!output.ok);
    let sources = BTreeMap::from([
        ("notes/主😀.typ".into(), input.source),
        (style.path.clone(), style.source),
    ]);
    let mut seen = BTreeSet::new();
    for diagnostic in output.diagnostics {
        if let Some(origin) = diagnostic.origin {
            check_origin(&origin, &sources, &mut seen);
        }
    }
    assert!(seen.contains(&style.path));
}
