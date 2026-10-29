use std::collections::BTreeMap;
use tempfile::TempDir;
use weftext_engine::*;

#[test]
fn inserted_png_uses_native_auto_format_despite_inherited_jpeg_default() {
    let directory = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let frozen = |source: &str| FrozenRenderRequest {
        source: source.into(),
        path: directory.path().join("note.typ"),
        root: directory.path().into(),
        files: BTreeMap::from([(
            "image.png".into(),
            include_bytes!("../../../packaging/windows/assets/StoreLogo.png").to_vec(),
        )]),
        style: None,
    };
    let source = "#set image(format: \"jpg\")";
    // Confirm this inherited default really selects the wrong decoder for the
    // exact PNG bytes. The positive case exercises native Typst, not just parse.
    let inherited = format!("{source}\n\n#image(\"image.png\", alt: \"original\")");
    assert!(!engine.render_frozen(&frozen(&inherited)).unwrap().ok);
    let edited = edit(
        source,
        &EditRequest {
            source_hash: source_hash(source),
            intent: EditIntent::InsertBlock {
                at: source.len(),
                block: BlockInsert::Image {
                    path: "image.png".into(),
                    alt: "original".into(),
                },
            },
        },
    )
    .unwrap();
    assert_eq!(edited.patches.len(), 1);
    assert!(edited.source.starts_with(source));
    assert!(edited.source.contains("format: auto"));
    let rendered = engine.render_frozen(&frozen(&edited.source)).unwrap();
    assert!(rendered.ok, "{:?}", rendered.diagnostics);
}
