use std::collections::BTreeMap;
use tempfile::TempDir;
use weftext_engine::*;

#[test]
fn inserted_png_uses_native_auto_format_despite_inherited_jpeg_default() {
    let directory = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    // Synthetic 1x1 RGBA PNG; this engine test has no packaging-asset input.
    let png = vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let frozen = |source: &str| FrozenRenderRequest {
        source: source.into(),
        path: directory.path().join("note.typ"),
        root: directory.path().into(),
        files: BTreeMap::from([("image.png".into(), png.clone())]),
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
