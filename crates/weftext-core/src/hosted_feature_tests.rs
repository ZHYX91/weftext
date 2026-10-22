use super::*;

#[test]
fn hosted_content_import_losses_project_known_codes_without_private_details() {
    let mut f = Fixture::new();
    let (request, _) = f.proposal();
    let mut original = f.core.prepare_operation(request).unwrap();
    original.losses = [
        "generatedBibliographyNotInstalled",
        "resourceModuleContextPreserved",
        "settingsNotApplied",
        "unregisteredFileOmitted",
        "PRIVATE_UNKNOWN_CODE",
        "PRIVATE_OTHER_CODE",
    ]
    .iter()
    .map(|code| weftext_knowledge::Loss {
        path: "PRIVATE_SCHEMA_PATH".into(),
        code: (*code).into(),
        detail: "PRIVATE_PRODUCER_DETAILS".into(),
    })
    .collect();
    for kind in [
        FeatureIntentKind::ImportNative,
        FeatureIntentKind::ImportBibliography,
    ] {
        let projected = feature_losses(&original, kind);
        assert_eq!(projected.as_array().unwrap().len(), 5);
        for code in [
            "generatedBibliographyNotInstalled",
            "resourceModuleContextPreserved",
            "settingsNotApplied",
            "unregisteredFileOmitted",
            "content-import-loss",
        ] {
            assert!(
                projected
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|loss| loss["code"] == code)
            );
        }
        let public = serde_json::to_string(&projected).unwrap();
        for secret in [
            "PRIVATE_SCHEMA_PATH",
            "PRIVATE_PRODUCER_DETAILS",
            "PRIVATE_UNKNOWN_CODE",
            "PRIVATE_OTHER_CODE",
        ] {
            assert!(!public.contains(secret));
        }
        assert!(
            projected
                .as_array()
                .unwrap()
                .iter()
                .all(|loss| loss.as_object().unwrap().len() == 2)
        );
    }
}

const ACTOR: &str = "deployment:alice:review";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_FP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    _temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    source: PathBuf,
    sidecar: PathBuf,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let doc = core.create_note_fixture(&root, &root, "Note").unwrap();
        Self {
            _temp: temp,
            data,
            root,
            source: PathBuf::from(doc.path),
            sidecar: PathBuf::from(doc.root).join("Note/weftext.annotations.json"),
            core,
        }
    }
    fn proposal(&mut self) -> (OperationRequest, Vec<String>) {
        let paths = [
            display(&self.source),
            display(&self.sidecar),
            display(&self.root.join(".weftext-schema.json")),
        ];
        let snapshot = self.core.read_snapshot(&self.root, &paths).unwrap();
        let mut after = snapshot.files[0].bytes.clone().unwrap();
        after.extend_from_slice(b"\nReviewed feature change.");
        let request = OperationRequest {
            operation_id: fresh(),
            root: display(&self.root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![
                    FileChange {
                        path: paths[0].clone(),
                        base_id: Some(snapshot.files[0].base_id.clone()),
                        before: snapshot.files[0].bytes.clone(),
                        after,
                    },
                    FileChange {
                        path: paths[1].clone(),
                        base_id: Some(snapshot.files[1].base_id.clone()),
                        before: snapshot.files[1].bytes.clone(),
                        after: b"{\"version\":1,\"annotations\":[]}".to_vec(),
                    },
                ],
            },
        };
        (request, vec![snapshot.files[2].base_id.clone()])
    }
    fn prepare(&mut self, operation: &str) -> ActorFeatureStatus {
        let (request, dependencies) = self.proposal();
        self.core.with_actor(ACTOR, |core| core.actor_feature_prepare("Note", operation, FP, request, serde_json::json!({"kind":"annotation","before":null,"after":{"text":"reviewed"}}), &dependencies)).unwrap()
    }
    fn internal(&self, operation: &str) -> String {
        self.core.with_actor_for_test(operation)
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self._temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
}

impl Core {
    fn with_actor_for_test(&self, operation: &str) -> String {
        let path = self
            .data_dir
            .join("actors")
            .join(hash(ACTOR.as_bytes()))
            .join("operations")
            .join(format!("{operation}.json"));
        serde_json::from_slice::<ActorOperation>(&fs::read(path).unwrap())
            .unwrap()
            .core_operation_id
    }
}

#[test]
fn hosted_feature_original_preview_plan_commit_and_private_draft_survive_restart() {
    let mut f = Fixture::new();
    let doc = f
        .core
        .with_actor(ACTOR, |core| core.open(&f.source, Some(f.root.clone())))
        .unwrap();
    f.core
        .with_actor(ACTOR, |core| {
            core.persist_draft(
                &doc.session_id,
                &(doc.source.clone() + "\nMY_PRIVATE_DRAFT"),
            )
        })
        .unwrap();
    let op = fresh();
    let prepared = f.prepare(&op);
    assert_eq!(prepared.status, "prepared");
    let internal = f.internal(&op);
    assert_ne!(internal, op);
    let public = serde_json::to_string(&prepared).unwrap();
    for secret in [
        &display(&f.root),
        &internal,
        "changes",
        "details",
        "nodeMap",
    ] {
        assert!(!public.contains(secret));
    }
    let plan = prepared.plan_id.clone().unwrap();
    assert!(!f.sidecar.exists());
    f.restart();
    let status = f
        .core
        .with_actor(ACTOR, |core| core.actor_feature_retry("Note", &op, FP))
        .unwrap()
        .unwrap();
    assert_eq!(status.plan_id, prepared.plan_id);
    assert_eq!(status.preview, prepared.preview);
    let (mut changed, dependencies) = f.proposal();
    if let OperationIntent::UpdateFiles { changes } = &mut changed.intent {
        changes[0]
            .after
            .extend_from_slice(b"DIFFERENT_REGENERATION");
    }
    let retry = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                changed,
                serde_json::json!({"newPreview":true}),
                &dependencies,
            )
        })
        .unwrap();
    assert_eq!(retry.preview, prepared.preview);
    assert_eq!(retry.plan_id, prepared.plan_id);
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_retry("Other", &op, FP))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core
                .actor_feature_retry("Note", &op, OTHER_FP))
            .is_err()
    );
    assert!(
        f.core
            .with_actor("deployment:bob:review", |core| core
                .actor_feature_status("Note", &op))
            .unwrap()
            .is_none()
    );
    assert!(
        f.core
            .with_actor("deployment:bob:review", |core| core
                .actor_feature_commit("Note", &op, &plan))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit(
                "Note",
                &op,
                &fresh()
            ))
            .is_err()
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit("Note", &op, &plan))
            .unwrap()
            .status,
        "committed"
    );
    assert!(
        fs::read_to_string(&f.source)
            .unwrap()
            .contains("Reviewed feature change.")
    );
    assert!(
        !fs::read_to_string(&f.source)
            .unwrap()
            .contains("DIFFERENT_REGENERATION")
    );
    assert!(f.sidecar.exists());
    let reopened = f
        .core
        .with_actor(ACTOR, |core| core.open(&f.source, Some(f.root.clone())))
        .unwrap();
    assert!(
        reopened
            .recovered_draft
            .unwrap()
            .contains("MY_PRIVATE_DRAFT")
    );
    f.restart();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit("Note", &op, &plan))
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &op))
            .unwrap()
            .status,
        "committed"
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_save_status("Note", &op))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_field_status(
                "Note",
                "tasks/status",
                &op
            ))
            .is_err()
    );
}

#[test]
fn hosted_feature_settle_fences_unknown_delayed_prepare_and_cancels_uninstalled_plan() {
    let mut f = Fixture::new();
    let unknown = fresh();
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Note", &unknown))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &unknown))
            .unwrap()
            .status,
        "cancelled"
    );
    f.restart();
    assert_eq!(f.prepare(&unknown).status, "cancelled");
    assert!(!f.core.operations.contains_key(&f.internal(&unknown)));
    let op = fresh();
    let prepared = f.prepare(&op);
    let internal = f.internal(&op);
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &op))
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(f.core.resolved(&internal).unwrap());
    f.restart();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit(
                "Note",
                &op,
                prepared.plan_id.as_deref().unwrap()
            ))
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(
        f.core
            .execute_operation(&internal, prepared.plan_id.as_deref().unwrap())
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core
                .actor_feature_retry("Note", &op, OTHER_FP))
            .is_err()
    );
    assert!(!f.sidecar.exists());
}

#[test]
fn hosted_feature_prepare_failure_keeps_original_intent_without_late_repreparation() {
    let mut f = Fixture::new();
    let (request, _) = f.proposal();
    let op = fresh();
    let error = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request.clone(),
                serde_json::json!({"original":true}),
                &[fresh()],
            )
        })
        .unwrap_err();
    assert_eq!(error.code, "staleBase");
    f.restart();
    let retry = f
        .core
        .with_actor(ACTOR, |core| {
            core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request,
                serde_json::json!({"replacement":true}),
                &[],
            )
        })
        .unwrap();
    assert_eq!(retry.status, "notCommitted");
    assert_eq!(retry.preview, serde_json::json!({"original":true}));
    assert!(retry.plan_id.is_none());
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &op))
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(!f.sidecar.exists());
}

#[test]
fn hosted_feature_install_marker_cannot_be_cancelled_or_blindly_replayed() {
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = f.prepare(&op);
    let internal = f.internal(&op);
    journal::private_create(
        &f.core.journal_path(&internal, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    let source_before = fs::read(&f.source).unwrap();
    f.restart();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_settle("Note", &op))
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit(
                "Note",
                &op,
                prepared.plan_id.as_deref().unwrap()
            ))
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert!(!f.core.resolved(&internal).unwrap());
    assert_eq!(fs::read(&f.source).unwrap(), source_before);
    assert!(!f.sidecar.exists());
    f.core.resolve_record(&internal, "keepCurrent").unwrap();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Note", &op))
            .unwrap()
            .unwrap()
            .status,
        "resolved"
    );
}

#[test]
fn hosted_feature_cancel_decision_survives_crash_before_actor_tombstone_update() {
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = f.prepare(&op);
    let internal = f.internal(&op);
    // Interrupt settlement after the Core decision but before updating its
    // actor record. The original Core resolution still fences all execution.
    f.core.resolve_record(&internal, "cancelPrepared").unwrap();
    f.restart();
    let status = f
        .core
        .with_actor(ACTOR, |core| core.actor_feature_status("Note", &op))
        .unwrap()
        .unwrap();
    assert_eq!(status.status, "cancelled");
    assert_eq!(status.preview, prepared.preview);
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit(
                "Note",
                &op,
                prepared.plan_id.as_deref().unwrap()
            ))
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(!f.sidecar.exists());
    assert!(
        !fs::read_to_string(&f.source)
            .unwrap()
            .contains("Reviewed feature change.")
    );
}

#[test]
fn hosted_feature_rechecks_frozen_dependencies_before_commit() {
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = f.prepare(&op);
    fs::write(
        f.root.join(".weftext-schema.json"),
        b"external configuration appeared",
    )
    .unwrap();
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_commit(
                "Note",
                &op,
                prepared.plan_id.as_deref().unwrap()
            ))
            .is_err()
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Note", &op))
            .unwrap()
            .unwrap()
            .status,
        "prepared"
    );
    assert!(!f.sidecar.exists());
    assert!(
        !fs::read_to_string(&f.source)
            .unwrap()
            .contains("Reviewed feature change.")
    );
}

#[test]
fn hosted_feature_rejects_unopened_intents_and_bad_identifiers_before_binding() {
    let mut f = Fixture::new();
    let op = fresh();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::BackupLibrary {
            destination: display(&f._temp.path().join("not-authorized-backup")),
        },
    };
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request,
                serde_json::Value::Null,
                &[]
            ))
            .unwrap_err()
            .code,
        "unsupportedIntent"
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Note", &op))
            .unwrap()
            .is_none()
    );
    let (request, dependencies) = f.proposal();
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "Note",
                &op,
                "not-a-digest",
                request.clone(),
                serde_json::Value::Null,
                &dependencies
            ))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "Note",
                "../escape",
                FP,
                request.clone(),
                serde_json::Value::Null,
                &dependencies
            ))
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_prepare(
                "Note",
                &op,
                FP,
                request.clone(),
                serde_json::Value::String("x".repeat(MAX_SOURCE_BYTES + 1)),
                &dependencies
            ))
            .is_err()
    );
    assert!(
        f.core
            .actor_feature_prepare(
                "Note",
                &op,
                FP,
                request,
                serde_json::Value::Null,
                &dependencies
            )
            .is_err()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_feature_status("Note", &op))
            .unwrap()
            .is_none()
    );
}

const IMPORT_ACTOR: &str = "deployment:alice:office-import";

fn docx_bytes() -> Vec<u8> {
    use weftext_engine::{RenderNode, RenderOutput, RenderProducts};
    use weftext_export::docx::{DocxInput, DocxOptions, export_docx};
    let text = RenderNode {
        key: "text".into(),
        kind: "text".into(),
        tag: None,
        text: Some("Imported author content".into()),
        attrs: BTreeMap::new(),
        children: vec![],
        origin: None,
        writable: false,
    };
    let paragraph = RenderNode {
        key: "paragraph".into(),
        kind: "element".into(),
        tag: Some("p".into()),
        text: None,
        attrs: BTreeMap::new(),
        children: vec![text],
        origin: None,
        writable: false,
    };
    let render = RenderOutput {
        ok: true,
        html: "<p>Imported author content</p>".into(),
        tree: Some(paragraph),
        diagnostics: vec![],
        source_info: weftext_engine::inspect_metadata(""),
        dependencies: vec![],
        font_families: vec![],
        compiler_version: "test".into(),
        labels: vec![],
        products: Some(RenderProducts {
            target: "html".into(),
            body_status: "present".into(),
            body: Some("Imported author content".into()),
            title_status: "absent".into(),
            title: None,
            metadata_title: None,
            native_title_bodies: vec![],
            headings: vec![],
        }),
    };
    export_docx(DocxInput {
        render: &render,
        template: None,
        bindings: &BTreeMap::new(),
        options: DocxOptions::default(),
    })
    .unwrap()
    .bytes
}

fn import_request(f: &Fixture, bytes: Vec<u8>) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::ImportDocx {
            parent: display(&f.root),
            name: "Imported".into(),
            bytes,
            allow_losses: false,
        },
    }
}

fn prepare_import(f: &mut Fixture, op: &str) -> ActorFeatureStatus {
    let bytes = docx_bytes();
    let preview = serde_json::json!({"summary":"Review original import","_officeImport":{"uploadDigest":hash(&bytes),"configHash":FP,"actorEpoch":0}});
    let request = import_request(f, bytes);
    f.core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_prepare("Imported", op, FP, request, preview, &[])
        })
        .unwrap()
}

#[test]
fn original_feature_member_delivery_retains_actor_scope_and_physical_birth_after_move() {
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = prepare_import(&mut f, &op);
    let committed = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_commit_with_losses(
                "Imported",
                &op,
                prepared.plan_id.as_ref().unwrap(),
                true,
            )
        })
        .unwrap();
    let owner = committed.node_refs.first().unwrap().clone();
    let opened = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_open_original_node("Imported", &op, &owner)
        })
        .unwrap();
    let identity = f.core.sessions[&opened.session_id].stamp.identity.clone();
    let parent = f
        .core
        .create_note_fixture(&f.root, &f.root, "DeliveryParent")
        .unwrap();
    let preview = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::MoveNode {
                source: display(Path::new(&opened.path).parent().unwrap()),
                parent: display(Path::new(&parent.path).parent().unwrap()),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let old_directory = Path::new(&opened.path).parent().unwrap();
    let substitute = f
        .core
        .create_note_fixture(
            &f.root,
            old_directory.parent().unwrap(),
            old_directory.file_name().unwrap().to_str().unwrap(),
        )
        .unwrap();
    let delivered = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_open_original_node("Imported", &op, &owner)
        })
        .unwrap();
    assert_eq!(delivered.node_id.as_ref(), Some(&owner.node_id));
    assert_ne!(delivered.node_id, substitute.node_id);
    assert_eq!(
        f.core.sessions[&delivered.session_id].stamp.identity,
        identity
    );
    for (actor, resource) in [("deployment:other", "Imported"), (IMPORT_ACTOR, "Other")] {
        assert!(
            f.core
                .with_actor(actor, |core| core
                    .actor_feature_open_original_node(resource, &op, &owner))
                .is_err()
        );
    }
    let foreign = weftext_knowledge::NodeRef {
        workspace_id: owner.workspace_id.clone(),
        node_id: substitute.node_id.unwrap(),
    };
    assert!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_open_original_node("Imported", &op, &foreign))
            .is_err()
    );
    assert!(
        f.core
            .actor_feature_open_original_node("Imported", &op, &owner)
            .is_err()
    );
}

#[test]
fn hosted_feature_docx_import_retains_frozen_safe_losses_and_plan_across_restart() {
    let mut f = Fixture::new();
    let opened = f
        .core
        .with_actor("deployment:alice", |core| {
            core.open(&f.source, Some(f.root.clone()))
        })
        .unwrap();
    f.core
        .with_actor("deployment:alice", |core| {
            core.persist_draft(&opened.session_id, "PRIVATE SOURCE DRAFT")
        })
        .unwrap();
    let op = fresh();
    let prepared = prepare_import(&mut f, &op);
    let plan = prepared.plan_id.clone().unwrap();
    assert_eq!(prepared.status, "prepared");
    assert!(
        prepared.preview["losses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["code"] == "word-layout")
    );
    let public = serde_json::to_string(&prepared).unwrap();
    for secret in [
        &display(&f.root),
        "word/document.xml",
        "nodeMap",
        "resourceMap",
        "\"path\"",
        "\"detail\"",
    ] {
        assert!(!public.contains(secret));
    }
    assert!(!f.root.join("Imported").exists());
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_commit("Imported", &op, &plan))
            .unwrap_err()
            .code,
        "copyLoss"
    );
    f.restart();
    let retry = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_retry("Imported", &op, FP)
        })
        .unwrap()
        .unwrap();
    assert_eq!(retry.preview, prepared.preview);
    assert_eq!(retry.plan_id, prepared.plan_id);
    let changed = import_request(&f, b"different late upload".to_vec());
    let retry = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_prepare(
                "Imported",
                &op,
                FP,
                changed,
                serde_json::json!({"summary":"regenerated"}),
                &[],
            )
        })
        .unwrap();
    assert_eq!(retry.preview, prepared.preview);
    assert_eq!(retry.plan_id, prepared.plan_id);
    let committed = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_commit_with_losses("Imported", &op, &plan, true)
        })
        .unwrap();
    assert_eq!(committed.status, "committed");
    let source = f.root.join("Imported/document/document.typ");
    let imported = fs::read_to_string(&source).unwrap();
    assert!(
        imported.contains("Imported")
            && imported.contains("author")
            && imported.contains("content"),
        "{imported}"
    );
    assert_eq!(
        fs::read(f.root.join("Imported/document/original.docx")).unwrap(),
        docx_bytes()
    );
    assert!(
        f.root
            .join("Imported/document/weftext.annotations.json")
            .is_file()
    );
    f.core.open(&source, Some(f.root.clone())).unwrap();
    let reopened = f
        .core
        .with_actor("deployment:alice", |core| {
            core.open(&f.source, Some(f.root.clone()))
        })
        .unwrap();
    assert_eq!(
        reopened.recovered_draft.as_deref(),
        Some("PRIVATE SOURCE DRAFT")
    );
    f.restart();
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_commit_with_losses("Imported", &op, &plan, false))
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_settle("Imported", &op))
            .unwrap()
            .status,
        "committed"
    );
}

#[test]
fn hosted_feature_docx_create_only_and_settlement_never_overwrite_an_occupied_slot() {
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = prepare_import(&mut f, &op);
    let plan = prepared.plan_id.unwrap();
    let target = f.root.join("Imported");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("occupied.txt"), b"EXTERNAL OCCUPANT").unwrap();
    assert!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_commit_with_losses("Imported", &op, &plan, true))
            .is_err()
    );
    assert_eq!(
        fs::read(target.join("occupied.txt")).unwrap(),
        b"EXTERNAL OCCUPANT"
    );
    assert!(!target.join("document").exists());
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_settle("Imported", &op))
            .unwrap()
            .status,
        "cancelled"
    );
    f.restart();
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_commit_with_losses("Imported", &op, &plan, true))
            .unwrap()
            .status,
        "cancelled"
    );
    let second = fresh();
    let request = import_request(&f, docx_bytes());
    assert!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core.actor_feature_prepare(
                "Imported",
                &second,
                FP,
                request,
                serde_json::json!({"summary":"occupied"}),
                &[]
            ))
            .is_err()
    );
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_status("Imported", &second))
            .unwrap()
            .unwrap()
            .status,
        "notCommitted"
    );
    assert_eq!(
        fs::read(target.join("occupied.txt")).unwrap(),
        b"EXTERNAL OCCUPANT"
    );
}

#[test]
fn hosted_feature_docx_unknown_fence_and_install_marker_survive_restart() {
    let mut f = Fixture::new();
    let unknown = fresh();
    f.core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_settle("Imported", &unknown)
        })
        .unwrap();
    f.restart();
    assert_eq!(prepare_import(&mut f, &unknown).status, "cancelled");
    let op = fresh();
    let prepared = prepare_import(&mut f, &op);
    let internal = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            Ok(core
                .actor_feature_record("Imported", &op)?
                .unwrap()
                .core_operation_id)
        })
        .unwrap();
    journal::private_create(
        &f.core.journal_path(&internal, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    f.restart();
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_settle("Imported", &op))
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core.actor_feature_commit_with_losses(
                "Imported",
                &op,
                prepared.plan_id.as_deref().unwrap(),
                true
            ))
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert!(!f.root.join("Imported").exists());
    assert!(!f.core.resolved(&internal).unwrap());
}

#[test]
fn hosted_feature_docx_upload_budget_is_checked_before_binding() {
    let mut f = Fixture::new();
    let op = fresh();
    let request = import_request(&f, vec![0; 16 * 1024 * 1024 + 1]);
    assert_eq!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core.actor_feature_prepare(
                "Imported",
                &op,
                FP,
                request,
                serde_json::json!({}),
                &[]
            ))
            .unwrap_err()
            .code,
        "budget"
    );
    assert!(
        f.core
            .with_actor(IMPORT_ACTOR, |core| core
                .actor_feature_status("Imported", &op))
            .unwrap()
            .is_none()
    );
}

#[test]
fn hosted_feature_import_loss_preview_never_copies_schema_paths_or_details() {
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = prepare_import(&mut f, &op);
    let internal = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            Ok(core
                .actor_feature_record("Imported", &op)?
                .unwrap()
                .core_operation_id)
        })
        .unwrap();
    let transaction = f
        .core
        .operations
        .get_mut(&internal)
        .unwrap()
        .transaction
        .as_mut()
        .unwrap();
    transaction.preview.losses.push(weftext_knowledge::Loss {
        path: "C:/PRIVATE_SCHEMA".into(),
        code: "unrecognized-private-detail".into(),
        detail: "PRIVATE_SCHEMA_CONTENT".into(),
    });
    let safe = serde_json::to_string(&import_losses(&transaction.preview)).unwrap();
    assert!(!safe.contains("PRIVATE_SCHEMA"));
    assert!(!safe.contains("unrecognized-private-detail"));
    assert!(safe.contains("docx-import-loss"));
    let status = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_status("Imported", &op)
        })
        .unwrap()
        .unwrap();
    assert_eq!(status.plan_id, prepared.plan_id);
    // The already-delivered preview is its persisted original. A later producer
    // or wording change must not replace it with newly mapped diagnostics.
    assert_eq!(status.preview, prepared.preview);
}

#[test]
fn hosted_feature_docx_safe_loss_freeze_recovers_before_first_delivery_and_keeps_original_wording()
{
    let mut f = Fixture::new();
    let op = fresh();
    let prepared = prepare_import(&mut f, &op);
    f.core
        .with_actor(IMPORT_ACTOR, |core| {
            let mut record = core.actor_feature_record("Imported", &op)?.unwrap();
            assert_eq!(
                record.feature.as_ref().unwrap().import_losses.as_ref(),
                Some(&prepared.preview["losses"])
            );
            // Model a crash after Core's frozen plan but before its safe loss list
            // became durable or reached the browser.
            record.feature.as_mut().unwrap().import_losses = None;
            private_replace(&core.actor_record(&op)?, &serde_json::to_vec(&record)?)
        })
        .unwrap();
    f.restart();
    let recovered = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_status("Imported", &op)
        })
        .unwrap()
        .unwrap();
    assert_eq!(recovered.preview, prepared.preview);
    let original = serde_json::json!([{"code":"word-layout","message":"Original reviewed wording from a prior version"}]);
    f.core
        .with_actor(IMPORT_ACTOR, |core| {
            let mut record = core.actor_feature_record("Imported", &op)?.unwrap();
            assert!(record.feature.as_ref().unwrap().import_losses.is_some());
            record.feature.as_mut().unwrap().import_losses = Some(original.clone());
            private_replace(&core.actor_record(&op)?, &serde_json::to_vec(&record)?)
        })
        .unwrap();
    f.restart();
    let retained = f
        .core
        .with_actor(IMPORT_ACTOR, |core| {
            core.actor_feature_status("Imported", &op)
        })
        .unwrap()
        .unwrap();
    assert_eq!(retained.preview["losses"], original);
    assert_eq!(retained.plan_id, prepared.plan_id);
}

#[test]
fn hosted_output_fence_is_durable_actor_owned_and_separate_from_author_operations() {
    let mut f = Fixture::new();
    let op = fresh();
    assert!(f.core.actor_output_claim(&op, FP).is_err());
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_output_fingerprint(&op))
            .unwrap()
            .is_none()
    );
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_output_claim(&op, FP))
            .unwrap()
    );
    assert!(
        !f.core
            .with_actor(ACTOR, |core| core.actor_output_claim(&op, FP))
            .unwrap()
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_output_claim(&op, OTHER_FP))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    // The same UUID can independently identify an author proposal; there is no
    // output path or artifact data in its actor feature namespace.
    assert_eq!(f.prepare(&op).status, "prepared");
    f.restart();
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_output_fingerprint(&op))
            .unwrap()
            .as_deref(),
        Some(FP)
    );
    assert!(
        !f.core
            .with_actor(ACTOR, |core| core.actor_output_claim(&op, FP))
            .unwrap()
    );
    assert!(
        f.core
            .with_actor("deployment:bob:review", |core| core
                .actor_output_fingerprint(&op))
            .unwrap()
            .is_none()
    );
    assert!(
        f.core
            .with_actor("deployment:bob:review", |core| core
                .actor_output_claim(&op, OTHER_FP))
            .unwrap()
    );
    assert_eq!(
        f.core
            .with_actor(ACTOR, |core| core.actor_output_fingerprint(&op))
            .unwrap()
            .as_deref(),
        Some(FP)
    );
    for invalid in [
        "../escape",
        "ABCDEF01-2345-4678-9ABC-DEF012345678",
        &op.replace('-', ""),
    ] {
        assert!(
            f.core
                .with_actor(ACTOR, |core| core.actor_output_claim(invalid, FP))
                .is_err()
        );
    }
    assert!(
        f.core
            .with_actor(ACTOR, |core| core.actor_output_claim(&fresh(), "invalid"))
            .is_err()
    );
}
