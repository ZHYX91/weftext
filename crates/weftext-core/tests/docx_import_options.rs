#![cfg(windows)]
use std::{collections::BTreeMap, fs};
use weftext_core::{Core, OperationIntent, OperationRequest};
use weftext_export::docx::{self, DocxImportOptions, OriginalImportAsset};

#[test]
fn docx_options_and_legacy_originals_share_one_durable_plan_and_exact_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("Library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
    let render = engine
        .render(
            "Original body",
            &temp.path().join("producer.typ"),
            temp.path(),
        )
        .unwrap();
    assert!(render.ok);
    let artifact = docx::export_docx(docx::DocxInput {
        render: &render,
        template: None,
        bindings: &BTreeMap::new(),
        options: docx::DocxOptions::default(),
    })
    .unwrap();
    let analysis = docx::analyze_docx_import(&artifact.bytes).unwrap();
    let op = uuid::Uuid::new_v4().to_string();
    let request = OperationRequest {
        operation_id: op.clone(),
        root: root.to_string_lossy().into(),
        intent: OperationIntent::ImportDocxWithOptions {
            parent: root.to_string_lossy().into(),
            name: "Imported".into(),
            bytes: artifact.bytes.clone(),
            allow_losses: true,
            options: DocxImportOptions {
                content_hash: analysis.content_hash,
                caption_pairings: vec![],
            },
            originals: vec![OriginalImportAsset {
                filename: "original.doc".into(),
                mime: "application/msword".into(),
                bytes: b"exact legacy original".to_vec(),
            }],
            conversion_losses: vec![weftext_export::exchange::NativeLoss {
                path: "original.doc".into(),
                code: "legacyStage".into(),
                description: weftext_messages::UiMessage::error_api_legacy_office_reply_preservation_unverified().into(),
                product_diagnostics: vec![],
            }],
        },
    };
    let plan = core.prepare_operation(request.clone()).unwrap();
    assert!(!root.join("Imported").exists());
    let files = core.prepared_operation_files(&op).unwrap();
    assert!(
        files
            .iter()
            .any(|file| file.path.ends_with("original-input-1.doc")
                && file.after == b"exact legacy original")
    );
    assert!(
        files
            .iter()
            .any(|file| file.path.ends_with("original.docx") && file.after == artifact.bytes)
    );
    assert!(plan.losses.iter().any(|loss| loss.code == "legacyStage"));
    let mut changed = request.clone();
    if let OperationIntent::ImportDocxWithOptions { originals, .. } = &mut changed.intent {
        originals[0].bytes = b"different original".to_vec();
    }
    assert!(core.prepare_operation(changed).is_err());
    let mut invalid = request.clone();
    invalid.operation_id = uuid::Uuid::new_v4().to_string();
    if let OperationIntent::ImportDocxWithOptions { options, name, .. } = &mut invalid.intent {
        options.content_hash = "stale".into();
        *name = "Refused".into();
    }
    assert!(core.prepare_operation(invalid).is_err());
    assert!(!root.join("Refused").exists());
    drop(core);
    let mut core = Core::new(&data).unwrap();
    let recovered = core.prepare_operation(request).unwrap();
    assert_eq!(recovered.plan_id, plan.plan_id);
    core.execute_operation(&op, &plan.plan_id).unwrap();
    assert_eq!(
        fs::read(root.join("Imported/document/original-input-1.doc")).unwrap(),
        b"exact legacy original"
    );
    assert!(
        core.qualified_imported_query_node_at(&root, &root.join("Imported/document/document.typ"))
            .is_ok()
    );
}
