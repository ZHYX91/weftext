use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use tempfile::tempdir;
use uuid::Uuid;
use weftext_core::{
    ANNOTATION_STORE_VERSION, Anchor, Annotation, AnnotationAction, AnnotationAppearance,
    AnnotationBody, AnnotationColor, AnnotationKind, AnnotationMark, AnnotationReanchorOutcome,
    AnnotationReplicaCompleteness, AnnotationResolution, AnnotationResourceMediaKind,
    AnnotationResourceRegion, AnnotationSidecarSnapshot, AnnotationState, AnnotationStore,
    AnnotationTargetIntent, AnnotationValidationError, DocumentProfileId, DocumentRevision,
    ThreadMessage, WorkspaceTransactionError, WorkspaceTransactionPlan, build_annotation_target,
    capture_annotation_sidecar_snapshot, commit_workspace_transaction, create_child_node,
    create_workspace, plan_annotation_action, plan_copy_node,
    prepare_workspace_transaction_recovery_fixture, read_node_annotations, read_node_document,
    reanchor_annotation, recover_workspace_transactions, scan_workspace,
};

const NOW: &str = "2026-08-24T12:00:00+08:00";

#[derive(Deserialize)]
struct AnnotationFixtureManifest {
    profile: String,
    schema: String,
    cases: Vec<AnnotationFixtureCase>,
}

#[derive(Deserialize)]
struct AnnotationFixtureCase {
    id: String,
    source: String,
    valid: bool,
    features: Vec<String>,
}

fn append_document(node: &std::path::Path, text: &str) {
    let name = node.file_name().unwrap().to_str().unwrap();
    let path = node.join(format!("{name}.adoc"));
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str(text);
    fs::write(path, source).unwrap();
}

fn local_annotation_snapshot(
    root: &std::path::Path,
    node_id: weftext_core::NodeId,
) -> AnnotationSidecarSnapshot {
    capture_annotation_sidecar_snapshot(
        root,
        node_id,
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )
    .unwrap()
}

fn read_local_annotations(
    root: &std::path::Path,
    node_id: weftext_core::NodeId,
) -> AnnotationStore {
    read_node_annotations(
        root,
        node_id,
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )
    .unwrap()
}

fn plan_local_annotation(
    root: &std::path::Path,
    node_id: weftext_core::NodeId,
    action: AnnotationAction,
) -> Result<WorkspaceTransactionPlan, weftext_core::WorkspaceTransactionError> {
    let snapshot = local_annotation_snapshot(root, node_id);
    plan_annotation_action(root, &snapshot, action)
}

fn document_comment_action(timestamp: &str) -> AnnotationAction {
    AnnotationAction::Create {
        kind: AnnotationKind::Comment,
        target: AnnotationTargetIntent::Document,
        appearance: None,
        labels: Vec::new(),
        body_source: Some("Review this document".to_owned()),
        suggested_source: None,
        author_id: Uuid::new_v4(),
        author_name: "Reviewer".to_owned(),
        timestamp: timestamp.to_owned(),
    }
}

fn message(author_id: Uuid) -> ThreadMessage {
    ThreadMessage {
        id: Uuid::new_v4(),
        author_id,
        author_name: "Reviewer".to_owned(),
        body: AnnotationBody::asciidoc("Review this".to_owned()),
        created_at: NOW.to_owned(),
        updated_at: NOW.to_owned(),
    }
}

fn comment(document_id: weftext_core::NodeId, target: Anchor) -> AnnotationStore {
    AnnotationStore {
        version: ANNOTATION_STORE_VERSION,
        document_id,
        annotations: vec![Annotation {
            id: Uuid::new_v4(),
            kind: AnnotationKind::Comment,
            target,
            appearance: Some(AnnotationAppearance {
                mark: AnnotationMark::Highlight,
                color: AnnotationColor::Yellow,
            }),
            suggested_source: None,
            labels: vec!["review".to_owned()],
            thread: vec![message(Uuid::new_v4())],
            state: AnnotationState::Open,
            resolution: None,
            created_at: NOW.to_owned(),
            updated_at: NOW.to_owned(),
        }],
    }
}

#[test]
fn annotation_v3_schema_and_fixture_corpus_match_core() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture_root = workspace_root.join("tests/fixtures/annotations-v3");
    let manifest: AnnotationFixtureManifest = serde_json::from_str(
        &fs::read_to_string(fixture_root.join("manifest.json")).expect("fixture manifest"),
    )
    .expect("valid fixture manifest");
    assert_eq!(manifest.profile, "weftext.annotations.v3");

    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(workspace_root.join(&manifest.schema)).expect("annotation schema"),
    )
    .expect("valid annotation schema JSON");
    assert_eq!(
        schema["$id"],
        "https://weftext.org/schemas/annotations-v3.schema.json"
    );
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["version"]["const"], 3);
    assert_eq!(
        schema["$defs"]["body"]["properties"]["format"]["const"],
        "weftext.asciidoc.inline.v1"
    );

    let mut covered = BTreeSet::new();
    for case in manifest.cases {
        let source = fs::read_to_string(fixture_root.join(&case.source)).expect("fixture source");
        let parsed = AnnotationStore::from_json(&source);
        assert_eq!(parsed.is_ok(), case.valid, "{}: {parsed:?}", case.id);
        if let Ok(store) = parsed {
            let canonical = store.to_pretty_json().expect("canonical fixture JSON");
            assert_eq!(
                AnnotationStore::from_json(&canonical).expect("canonical fixture reparses"),
                store
            );
        }
        covered.extend(case.features);
    }
    for feature in [
        "comment",
        "text-range",
        "resource-region",
        "suggestion-insert",
        "suggestion-delete",
        "document",
        "thread",
        "cjk",
        "rtl",
        "emoji",
        "legacy-body",
        "closed-schema",
        "resource-bounds",
        "identity",
        "semantic-validation",
    ] {
        assert!(
            covered.contains(feature),
            "missing fixture feature {feature}"
        );
    }
}

#[test]
fn v3_serialization_uses_target_theme_and_exact_asciidoc_inline_marker() {
    let document_id = weftext_core::NodeId::new_v4();
    let store = comment(document_id, Anchor::Document);
    let json = store.to_pretty_json().unwrap();

    assert!(json.contains("\"target\""));
    assert!(!json.contains("\"anchor\""));
    assert!(json.contains("\"theme\": \"yellow\""));
    assert!(!json.contains("\"color\""));
    assert!(json.contains("\"format\": \"weftext.asciidoc.inline.v1\""));
    assert_eq!(AnnotationStore::from_json(&json).unwrap(), store);

    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut legacy_body = value.clone();
    legacy_body["annotations"][0]["thread"][0]["body"]["format"] =
        serde_json::Value::String("weftext.markdown.inline.v1".to_owned());
    assert!(matches!(
        AnnotationStore::from_json(&legacy_body.to_string()),
        Err(AnnotationValidationError::InvalidJson(_))
    ));

    value["annotations"][0]["id"] =
        serde_json::Value::String("550E8400-E29B-41D4-A716-446655440000".to_owned());
    assert!(matches!(
        AnnotationStore::from_json(&value.to_string()),
        Err(AnnotationValidationError::InvalidUuid)
    ));
}

#[test]
fn absent_sidecars_require_explicit_complete_backend_authority() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let node = create_workspace(&workspace).unwrap();

    for completeness in [
        AnnotationReplicaCompleteness::PartialReplica,
        AnnotationReplicaCompleteness::Unknown,
    ] {
        assert!(matches!(
            capture_annotation_sidecar_snapshot(&workspace, node.id, completeness),
            Err(WorkspaceTransactionError::IncompleteAnnotationReplica)
        ));
        assert!(matches!(
            read_node_annotations(&workspace, node.id, completeness),
            Err(WorkspaceTransactionError::IncompleteAnnotationReplica)
        ));
    }
    assert!(!workspace.join("weftext.annotations.json").exists());

    let snapshot = local_annotation_snapshot(&workspace, node.id);
    assert_eq!(
        snapshot.expected_state(),
        &weftext_core::AnnotationSidecarExpectedState::ConfirmedAbsent
    );
    assert!(snapshot.store().annotations.is_empty());
    let plan = plan_annotation_action(&workspace, &snapshot, document_comment_action(NOW)).unwrap();
    assert_eq!(
        plan.annotation_sidecar_authority
            .as_ref()
            .map(|authority| authority.completeness),
        Some(AnnotationReplicaCompleteness::CompleteLocalWorkspace)
    );
    commit_workspace_transaction(&plan).unwrap();
    assert_eq!(
        read_local_annotations(&workspace, node.id)
            .annotations
            .len(),
        1
    );
}

#[test]
fn document_first_sidecar_later_never_overwrites_late_authority() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let node = create_workspace(&workspace).unwrap();
    let snapshot = local_annotation_snapshot(&workspace, node.id);
    let plan = plan_annotation_action(
        &workspace,
        &snapshot,
        document_comment_action("2026-08-24T12:01:00+08:00"),
    )
    .unwrap();

    let late_store = comment(node.id, Anchor::Document);
    let late_bytes = late_store.to_pretty_json().unwrap();
    fs::write(workspace.join("weftext.annotations.json"), &late_bytes).unwrap();

    assert!(matches!(
        commit_workspace_transaction(&plan),
        Err(WorkspaceTransactionError::StaleRevision { .. }
            | WorkspaceTransactionError::DestinationExists(_))
    ));
    assert_eq!(
        fs::read_to_string(workspace.join("weftext.annotations.json")).unwrap(),
        late_bytes
    );
    assert_eq!(read_local_annotations(&workspace, node.id), late_store);
}

#[test]
fn conflict_copy_appearing_after_preview_blocks_first_sidecar_creation() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let node = create_workspace(&workspace).unwrap();
    let snapshot = local_annotation_snapshot(&workspace, node.id);
    let plan = plan_annotation_action(
        &workspace,
        &snapshot,
        document_comment_action("2026-08-24T12:01:30+08:00"),
    )
    .unwrap();

    let conflict = workspace.join("weftext.annotations (conflicted copy).json");
    let conflict_bytes = AnnotationStore::empty(node.id).to_pretty_json().unwrap();
    fs::write(&conflict, &conflict_bytes).unwrap();

    assert!(matches!(
        commit_workspace_transaction(&plan),
        Err(WorkspaceTransactionError::StaleRevision { .. }
            | WorkspaceTransactionError::AnnotationSidecarReconciliationRequired)
    ));
    assert!(!workspace.join("weftext.annotations.json").exists());
    assert_eq!(fs::read_to_string(conflict).unwrap(), conflict_bytes);
}

#[test]
fn sidecar_snapshot_rechecks_late_files_foreign_ids_and_unknown_fields() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let node = create_child_node(&workspace, "Source").unwrap();
    let absent = local_annotation_snapshot(&workspace, node.id);
    let sidecar = node.path.join("weftext.annotations.json");

    let valid = AnnotationStore::empty(node.id).to_pretty_json().unwrap();
    fs::write(&sidecar, &valid).unwrap();
    assert!(matches!(
        plan_annotation_action(&workspace, &absent, document_comment_action(NOW)),
        Err(WorkspaceTransactionError::StaleRevision { .. }
            | WorkspaceTransactionError::AnnotationSidecarChanged)
    ));

    let foreign = AnnotationStore::empty(weftext_core::NodeId::new_v4())
        .to_pretty_json()
        .unwrap();
    fs::write(&sidecar, foreign).unwrap();
    assert!(matches!(
        capture_annotation_sidecar_snapshot(
            &workspace,
            node.id,
            AnnotationReplicaCompleteness::CompleteLocalWorkspace
        ),
        Err(WorkspaceTransactionError::AnnotationSidecarReconciliationRequired)
    ));

    fs::write(
        &sidecar,
        format!(
            "{{\"version\":3,\"document_id\":\"{}\",\"annotations\":[],\"future\":true}}",
            node.id
        ),
    )
    .unwrap();
    assert!(matches!(
        capture_annotation_sidecar_snapshot(
            &workspace,
            node.id,
            AnnotationReplicaCompleteness::CompleteLocalWorkspace
        ),
        Err(WorkspaceTransactionError::Metadata(_))
    ));

    fs::write(&sidecar, &valid).unwrap();
    let conflict = node
        .path
        .join("weftext.annotations (conflicted copy 2026-08-24).json");
    fs::write(&conflict, &valid).unwrap();
    assert!(matches!(
        capture_annotation_sidecar_snapshot(
            &workspace,
            node.id,
            AnnotationReplicaCompleteness::CompleteLocalWorkspace
        ),
        Err(WorkspaceTransactionError::AnnotationSidecarReconciliationRequired)
    ));
    fs::remove_file(conflict).unwrap();

    let mut duplicate = comment(node.id, Anchor::Document);
    duplicate.annotations.push(duplicate.annotations[0].clone());
    fs::write(&sidecar, serde_json::to_vec_pretty(&duplicate).unwrap()).unwrap();
    assert!(matches!(
        capture_annotation_sidecar_snapshot(
            &workspace,
            node.id,
            AnnotationReplicaCompleteness::CompleteLocalWorkspace
        ),
        Err(WorkspaceTransactionError::AnnotationSidecarReconciliationRequired)
    ));

    let destination = create_child_node(&workspace, "Destination").unwrap();
    fs::write(
        &sidecar,
        AnnotationStore::empty(destination.id)
            .to_pretty_json()
            .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        plan_copy_node(&workspace, node.id, destination.id, "Copy"),
        Err(WorkspaceTransactionError::AnnotationSidecarReconciliationRequired)
    ));
    assert!(!destination.path.join("Copy").exists());
}

#[test]
fn prepared_absent_sidecar_creation_recovers_without_materializing_empty_authority() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let node = create_workspace(&workspace).unwrap();
    let plan = plan_local_annotation(
        &workspace,
        node.id,
        document_comment_action("2026-08-24T12:02:00+08:00"),
    )
    .unwrap();
    prepare_workspace_transaction_recovery_fixture(&plan).unwrap();

    let report = recover_workspace_transactions(&workspace).unwrap();
    assert_eq!(report.prepared_removed, 1);
    assert!(!workspace.join("weftext.annotations.json").exists());
    assert!(
        read_local_annotations(&workspace, node.id)
            .annotations
            .is_empty()
    );
}

#[test]
fn text_reanchor_requires_one_deterministic_match_and_orphans_ambiguity() {
    let original = "= Note\n\nprefix target suffix\n";
    let revision = DocumentRevision::from_source(original);
    let start = u64::try_from(original.find("target").unwrap()).unwrap();
    let target = build_annotation_target(
        DocumentProfileId::AsciiDocV1,
        original,
        revision.as_str(),
        &AnnotationTargetIntent::TextRange {
            start,
            end: start + 6,
        },
    )
    .unwrap();
    let mut unique = comment(weftext_core::NodeId::new_v4(), target.clone())
        .annotations
        .remove(0);
    let shifted = "intro\n= Note\n\nprefix target suffix\n";
    let shifted_revision = DocumentRevision::from_source(shifted);
    assert_eq!(
        reanchor_annotation(
            &mut unique,
            DocumentProfileId::AsciiDocV1,
            shifted,
            shifted_revision.as_str(),
        ),
        AnnotationReanchorOutcome::Reanchored
    );
    assert!(matches!(
        unique.target,
        Anchor::TextRange { start, .. }
            if start == u64::try_from(shifted.find("target").unwrap()).unwrap()
    ));

    let ambiguous = "= Note\n\nother target tail\n\nother target tail\n";
    let ambiguous_revision = DocumentRevision::from_source(ambiguous);
    let mut orphaned = comment(weftext_core::NodeId::new_v4(), target)
        .annotations
        .remove(0);
    assert_eq!(
        reanchor_annotation(
            &mut orphaned,
            DocumentProfileId::AsciiDocV1,
            ambiguous,
            ambiguous_revision.as_str(),
        ),
        AnnotationReanchorOutcome::Orphaned
    );
    assert_eq!(orphaned.state, AnnotationState::Orphaned);
    assert_eq!(orphaned.resolution, None);
}

#[test]
fn accepting_insert_and_delete_suggestions_updates_document_and_sidecar_together() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let created = create_workspace(&workspace).unwrap();
    append_document(&workspace, "\n= Review\n\nBefore after.\n");
    let snapshot = read_node_document(&workspace).unwrap();
    let insert_at = u64::try_from(snapshot.source.find("after").unwrap()).unwrap();

    let create_insert = plan_local_annotation(
        &workspace,
        created.id,
        AnnotationAction::Create {
            kind: AnnotationKind::SuggestionInsert,
            target: AnnotationTargetIntent::InsertionPoint {
                position: insert_at,
            },
            appearance: None,
            labels: Vec::new(),
            body_source: None,
            suggested_source: Some("inserted ".to_owned()),
            author_id: Uuid::new_v4(),
            author_name: "Reviewer".to_owned(),
            timestamp: NOW.to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&create_insert).unwrap();
    let insert_id = read_local_annotations(&workspace, created.id).annotations[0].id;
    let accept_insert = plan_local_annotation(
        &workspace,
        created.id,
        AnnotationAction::AcceptSuggestion {
            annotation_id: insert_id,
            timestamp: "2026-08-24T12:01:00+08:00".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(accept_insert.document_changes.len(), 1);
    commit_workspace_transaction(&accept_insert).unwrap();
    assert!(
        read_node_document(&workspace)
            .unwrap()
            .source
            .contains("Before inserted after.")
    );
    let accepted = read_local_annotations(&workspace, created.id);
    assert_eq!(accepted.annotations[0].state, AnnotationState::Resolved);
    assert_eq!(
        accepted.annotations[0].resolution,
        Some(AnnotationResolution::Accepted)
    );

    let snapshot = read_node_document(&workspace).unwrap();
    let delete_start = u64::try_from(snapshot.source.find("inserted ").unwrap()).unwrap();
    let create_delete = plan_local_annotation(
        &workspace,
        created.id,
        AnnotationAction::Create {
            kind: AnnotationKind::SuggestionDelete,
            target: AnnotationTargetIntent::TextRange {
                start: delete_start,
                end: delete_start + u64::try_from("inserted ".len()).unwrap(),
            },
            appearance: None,
            labels: Vec::new(),
            body_source: Some("Remove this word".to_owned()),
            suggested_source: None,
            author_id: Uuid::new_v4(),
            author_name: "Reviewer".to_owned(),
            timestamp: "2026-08-24T12:02:00+08:00".to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&create_delete).unwrap();
    let store = read_local_annotations(&workspace, created.id);
    let delete_id = store
        .annotations
        .iter()
        .find(|annotation| annotation.kind == AnnotationKind::SuggestionDelete)
        .unwrap()
        .id;
    let accept_delete = plan_local_annotation(
        &workspace,
        created.id,
        AnnotationAction::AcceptSuggestion {
            annotation_id: delete_id,
            timestamp: "2026-08-24T12:03:00+08:00".to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&accept_delete).unwrap();
    assert!(
        read_node_document(&workspace)
            .unwrap()
            .source
            .contains("Before after.")
    );
}

#[test]
fn copying_a_node_rekeys_annotation_and_message_ids_but_preserves_actor_identity() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let root = create_workspace(&workspace).unwrap();
    let source = create_child_node(&workspace, "Source").unwrap();
    append_document(&source.path, "\n= Source\n\nReview target.\n");
    let snapshot = read_node_document(&source.path).unwrap();
    let offset = u64::try_from(snapshot.source.find("Review target").unwrap()).unwrap();
    let actor_id = Uuid::new_v4();
    let create = plan_local_annotation(
        &workspace,
        source.id,
        AnnotationAction::Create {
            kind: AnnotationKind::Comment,
            target: AnnotationTargetIntent::BlockAt {
                source_offset: offset,
            },
            appearance: None,
            labels: Vec::new(),
            body_source: Some("Keep actor identity".to_owned()),
            suggested_source: None,
            author_id: actor_id,
            author_name: "Reviewer".to_owned(),
            timestamp: NOW.to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&create).unwrap();
    let original = read_local_annotations(&workspace, source.id);

    let copy = plan_copy_node(&workspace, source.id, root.id, "Copy").unwrap();
    commit_workspace_transaction(&copy).unwrap();
    let copied_id = scan_workspace(&workspace)
        .nodes
        .iter()
        .find(|node| node.path == workspace.join("Copy"))
        .and_then(|node| node.id)
        .unwrap();
    let copied = read_local_annotations(&workspace, copied_id);
    assert_eq!(copied.document_id, copied_id);
    assert_ne!(copied.annotations[0].id, original.annotations[0].id);
    assert_ne!(
        copied.annotations[0].thread[0].id,
        original.annotations[0].thread[0].id
    );
    assert_eq!(copied.annotations[0].thread[0].author_id, actor_id);
    let copied_revision = read_node_document(workspace.join("Copy")).unwrap().revision;
    assert!(matches!(
        &copied.annotations[0].target,
        Anchor::Block { base_revision, .. } if base_revision == copied_revision.as_str()
    ));
}

#[test]
fn resource_regions_are_closed_by_media_kind_and_normalized_bounds() {
    let document_id = weftext_core::NodeId::new_v4();
    let mut store = AnnotationStore {
        version: ANNOTATION_STORE_VERSION,
        document_id,
        annotations: vec![Annotation {
            id: Uuid::new_v4(),
            kind: AnnotationKind::Mark,
            target: Anchor::ResourceRegion {
                resource_locator: "resources/page.png".to_owned(),
                resource_digest: "a".repeat(64),
                media_kind: AnnotationResourceMediaKind::Image,
                region: AnnotationResourceRegion::Rect {
                    page: None,
                    x_millionths: 100_000,
                    y_millionths: 200_000,
                    width_millionths: 300_000,
                    height_millionths: 400_000,
                },
            },
            appearance: Some(AnnotationAppearance {
                mark: AnnotationMark::Underline,
                color: AnnotationColor::Blue,
            }),
            suggested_source: None,
            labels: Vec::new(),
            thread: Vec::new(),
            state: AnnotationState::Open,
            resolution: None,
            created_at: NOW.to_owned(),
            updated_at: NOW.to_owned(),
        }],
    };
    assert!(store.to_pretty_json().is_ok());

    let Anchor::ResourceRegion { region, .. } = &mut store.annotations[0].target else {
        unreachable!();
    };
    let AnnotationResourceRegion::Rect { page, .. } = region else {
        unreachable!();
    };
    *page = Some(1);
    assert!(matches!(
        store.to_pretty_json(),
        Err(AnnotationValidationError::InvalidResourceTarget)
    ));
}
