// Standalone module harness while the HTTP consumer is integrated separately.
#[allow(dead_code)]
#[path = "../src/legacy_office.rs"]
mod legacy_office;

use legacy_office::{ConversionRequest, Direction, convert};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

#[tokio::test]
#[ignore = "Explicit real-host DOC ↔ DOCX test; retains its own acceptance directory"]
async fn real_libreoffice_doc_roundtrip_preserves_originals() {
    let root = std::env::var_os("WEFTEXT_LEGACY_ACCEPTANCE_DIR")
        .map(PathBuf::from)
        .expect("set a dedicated absolute acceptance directory");
    assert!(root.is_absolute());
    std::fs::create_dir_all(&root).unwrap();
    let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
    let source = "= 中文旧格式测试\n\n正文 Alpha 123 与 *强调*。\n\n#table(columns: 2, [甲], [乙], [一], [二])";
    let render = engine
        .render(source, &root.join("source.typ"), &root)
        .unwrap();
    assert!(render.ok, "{:?}", render.diagnostics);
    let docx = weftext_export::docx::export_docx(weftext_export::docx::DocxInput {
        render: &render,
        template: None,
        bindings: &BTreeMap::new(),
        options: Default::default(),
    })
    .unwrap();
    let bytes = docx.bytes;
    let outgoing = convert(
        ConversionRequest {
            backend: "libreoffice".into(),
            direction: Direction::DocxToDoc,
            bytes: bytes.clone(),
        },
        &root,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(
        outgoing.status, "succeeded",
        "{} {}",
        outgoing.message, outgoing.diagnostic
    );
    assert_eq!(outgoing.original.bytes, bytes);
    assert_eq!(
        std::fs::read(&outgoing.original.retained_path).unwrap(),
        bytes
    );
    assert_eq!(outgoing.intermediate_docx.as_ref().unwrap().bytes, bytes);
    let generated_doc = outgoing.generated.unwrap();
    assert!(
        generated_doc
            .bytes
            .starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1")
    );
    let incoming = convert(
        ConversionRequest {
            backend: "libreoffice".into(),
            direction: Direction::DocToDocx,
            bytes: generated_doc.bytes.clone(),
        },
        &root,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(
        incoming.status, "succeeded",
        "{} {}",
        incoming.message, incoming.diagnostic
    );
    assert_eq!(incoming.original.bytes, generated_doc.bytes);
    assert_eq!(
        std::fs::read(&incoming.original.retained_path).unwrap(),
        generated_doc.bytes
    );
    let generated_docx = incoming.generated.unwrap();
    assert_eq!(
        incoming.intermediate_docx.unwrap().bytes,
        generated_docx.bytes
    );
    assert!(weftext_export::docx::list_template_controls(&generated_docx.bytes).is_ok());
    println!(
        "backends: {}",
        serde_json::to_string(&legacy_office::probe().await).unwrap()
    );
    println!(
        "outbound={} inbound={}",
        outgoing.workspace.display(),
        incoming.workspace.display()
    );
}
