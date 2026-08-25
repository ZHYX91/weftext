use std::fs;

use tempfile::{TempDir, tempdir};
use uuid::Uuid;
use weftext_core::{
    ANNOTATION_STORE_VERSION, Anchor, Annotation, AnnotationBody, AnnotationKind,
    AnnotationReplicaCompleteness, AnnotationSidecarSnapshot, AnnotationState, AnnotationStore,
    AnnotationTargetIntent, ChecklistPromotionEvidence, CommittedTaskPromotion, CreatedNode,
    NodeId, StructuralAction, TaskNodeState, TaskPromotionError, TaskPromotionRequest,
    ThreadMessage, WorkspaceDraftRegistryView, WorkspaceNodeProjection, WorkspaceReadScope,
    WorkspaceTransactionError, build_annotation_target, capture_annotation_sidecar_snapshot,
    commit_task_promotion_transaction, commit_task_promotion_transaction_scoped,
    commit_task_promotion_transaction_with_draft_gate,
    commit_task_promotion_with_injected_verification_failure_for_recovery_fixture,
    create_child_node, create_workspace, plan_create_child_node, plan_task_promotion_transaction,
    plan_task_promotion_transaction_scoped, prepare_task_promotion_applying_recovery_fixture,
    prepare_task_promotion_committed_recovery_fixture, prepare_task_promotion_recovery_fixture,
    prepare_workspace_transaction_recovery_fixture, read_node_document,
    recover_workspace_transactions, scan_workspace,
};

struct Fixture {
    _temporary: TempDir,
    root: CreatedNode,
    source: CreatedNode,
}

fn fixture(source_body: &str) -> Fixture {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("Tasks")).expect("workspace");
    let source = create_child_node(&root.path, "Source").expect("source");
    let exact = format!(
        "---\nweftext:\n  id: \"{}\"\n---\n= Source\n\n{source_body}",
        source.id
    );
    fs::write(&source.document_path, exact).expect("source bytes");
    Fixture {
        _temporary: temporary,
        root,
        source,
    }
}

fn request(
    fixture: &Fixture,
    occurrence_index: usize,
    parent_node_id: NodeId,
    name: &str,
    title: &str,
    label: &str,
) -> TaskPromotionRequest {
    let snapshot = read_node_document(&fixture.source.path).expect("source snapshot");
    let occurrence =
        weftext_asciidoc::analyze(&snapshot.source).checklists[occurrence_index].clone();
    TaskPromotionRequest {
        evidence: ChecklistPromotionEvidence {
            source_node_id: fixture.source.id,
            document_revision: snapshot.revision,
            parser_occurrence: occurrence.parser_occurrence,
            authored_marker: occurrence.authored_marker,
            state: occurrence.state,
            item_range: occurrence.item_range,
            marker_range: occurrence.marker_range,
            description_range: occurrence.description_range,
            description: occurrence.description,
            list_depth: occurrence.list_depth,
        },
        parent_node_id,
        portable_name: name.to_owned(),
        document_title: title.to_owned(),
        logical_link_label: label.to_owned(),
    }
}

fn annotation_snapshot(fixture: &Fixture) -> AnnotationSidecarSnapshot {
    capture_annotation_sidecar_snapshot(
        &fixture.root.path,
        fixture.source.id,
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )
    .expect("complete annotation snapshot")
}

fn annotation(fixture: &Fixture, exact: &str, state: AnnotationState) -> Annotation {
    let snapshot = read_node_document(&fixture.source.path).expect("source snapshot");
    let start = u64::try_from(snapshot.source.find(exact).expect("annotation text")).unwrap();
    let target = build_annotation_target(
        snapshot.profile,
        &snapshot.source,
        snapshot.revision.as_str(),
        &AnnotationTargetIntent::TextRange {
            start,
            end: start + u64::try_from(exact.len()).unwrap(),
        },
    )
    .expect("exact annotation target");
    Annotation {
        id: Uuid::new_v4(),
        kind: AnnotationKind::Comment,
        target,
        appearance: None,
        suggested_source: None,
        labels: vec!["promotion".to_owned()],
        thread: vec![ThreadMessage {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            author_name: "Reviewer".to_owned(),
            body: AnnotationBody::asciidoc("Keep this exact thread".to_owned()),
            created_at: "2026-08-25T00:00:00Z".to_owned(),
            updated_at: "2026-08-25T00:00:00Z".to_owned(),
        }],
        state,
        resolution: None,
        created_at: "2026-08-25T00:00:00Z".to_owned(),
        updated_at: "2026-08-25T00:00:00Z".to_owned(),
    }
}

fn write_annotations(fixture: &Fixture, annotations: Vec<Annotation>) {
    let store = AnnotationStore {
        version: ANNOTATION_STORE_VERSION,
        document_id: fixture.source.id,
        annotations,
    };
    fs::write(
        fixture.source.path.join("weftext.annotations.json"),
        store.to_pretty_json().expect("canonical annotation store"),
    )
    .expect("annotation sidecar");
}

fn assert_committed(
    fixture: &Fixture,
    request: &TaskPromotionRequest,
    committed: &CommittedTaskPromotion,
) {
    let summary = &committed.summary;
    assert_eq!(
        committed.transaction.action,
        StructuralAction::TaskPromotion
    );
    assert_eq!(
        committed.transaction.promotion_summary.as_ref(),
        Some(summary)
    );
    let destination = fixture.root.path.join(&request.portable_name);
    let task = read_node_document(&destination).expect("promoted task");
    assert_eq!(task.node_id, summary.generated_node_id);
    assert_eq!(
        fs::read_to_string(&fixture.source.document_path).expect("source"),
        format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Source\n\n{}",
            fixture.source.id, summary.replacement_source
        )
    );
    let profile = weftext_core::analyze_task_node_profile(&task.source, Some(task.node_id));
    assert!(profile.diagnostics.is_empty());
    assert_eq!(profile.profile.unwrap().state, summary.initial_state);
}

#[test]
fn leaf_open_x_and_star_promote_to_closed_task_profiles_without_checkbox_mirrors() {
    for (marker, expected_state) in [
        ("[ ]", TaskNodeState::Todo),
        ("[x]", TaskNodeState::Completed),
        ("[*]", TaskNodeState::Completed),
    ] {
        let fixture = fixture(&format!("* {marker} Ship 文缕 😀\n"));
        let request = request(
            &fixture,
            0,
            fixture.root.id,
            "Ship-文缕",
            "Ship 文缕 😀",
            "Ship [v1]: \\ ready, \"yes\"",
        );
        let snapshot = annotation_snapshot(&fixture);
        let plan =
            plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).expect("plan");
        assert_eq!(plan.summary().initial_state, expected_state);
        assert_eq!(
            plan.summary().generated_node_id,
            plan.summary().generated_node_id
        );
        assert!(!plan.task_document_source().contains("weftext-task-closed"));
        assert!(!plan.summary().replacement_source.contains("[ ]"));
        assert!(!plan.summary().replacement_source.contains("[x]"));
        assert!(!plan.summary().replacement_source.contains("[*]"));
        let committed = commit_task_promotion_transaction(&plan, &request).expect("commit");
        assert_committed(&fixture, &request, &committed);
    }
}

#[test]
fn nested_descendants_and_continuations_are_lifted_once() {
    let fixture = fixture("* [ ] Parent\n** [x] Child\n*** grandchild\n+\nAttached paragraph.\n");
    let request = request(
        &fixture,
        0,
        fixture.root.id,
        "Parent-task",
        "Parent task",
        "Parent task",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).expect("plan");
    assert_eq!(plan.summary().lifted_descendant_count, 2);
    assert_eq!(plan.summary().lifted_continuation_count, 1);
    assert!(
        plan.task_document_source()
            .ends_with("* [x] Child\n** grandchild\n+\nAttached paragraph.\n")
    );
    let generated = plan.summary().generated_node_id;
    commit_task_promotion_transaction(&plan, &request).expect("commit");
    let source = fs::read_to_string(&fixture.source.document_path).expect("source");
    assert!(source.ends_with(&format!("* node:{generated}[Parent task]\n")));
    assert_eq!(source.matches('[').count(), 1, "no checkbox mirror remains");
}

#[test]
fn replacement_preserves_complete_branch_terminal_eol_or_eof() {
    for (body, expected_suffix) in [
        ("* [ ] leaf\n", "\n"),
        ("* [ ] leaf", ""),
        ("* [ ] parent\n** child", ""),
        ("* [ ] parent\r\n** child\n", "\n"),
    ] {
        let fixture = fixture(body);
        let request = request(
            &fixture,
            0,
            fixture.root.id,
            "Eol-task",
            "EOL task",
            "EOL task",
        );
        let snapshot = annotation_snapshot(&fixture);
        let plan =
            plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).expect("plan");
        assert!(
            plan.summary().replacement_source.ends_with(expected_suffix),
            "{body:?}"
        );
        if expected_suffix.is_empty() {
            assert!(!plan.summary().replacement_source.ends_with(['\n', '\r']));
        }
    }
}

#[test]
fn alternate_parent_is_explicit_and_generated_uuid_is_stable_through_commit_replan() {
    let fixture = fixture("* [ ] Alternate\n");
    let alternate = create_child_node(&fixture.root.path, "Projects").expect("alternate parent");
    let request = request(
        &fixture,
        0,
        alternate.id,
        "Alternate",
        "Alternate",
        "Alternate",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).expect("plan");
    let generated = plan.summary().generated_node_id;
    assert_eq!(plan.summary().generated_path, "Projects/Alternate");
    let committed = commit_task_promotion_transaction(&plan, &request).expect("commit");
    assert_eq!(committed.summary.generated_node_id, generated);
    assert!(
        committed
            .transaction
            .target_node_ids
            .contains(&fixture.source.id)
    );
    assert!(
        committed
            .transaction
            .target_node_ids
            .contains(&alternate.id)
    );
    assert!(alternate.path.join("Alternate/Alternate.adoc").is_file());
}

#[test]
fn source_node_can_be_the_default_parent_without_path_authority_regression() {
    let fixture = fixture("* [ ] Child task\n");
    let request = request(
        &fixture,
        0,
        fixture.source.id,
        "Child-task",
        "Child task",
        "Child task",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot)
        .expect("default-parent plan");
    assert_eq!(plan.summary().generated_path, "Source/Child-task");
    let committed = commit_task_promotion_transaction(&plan, &request).expect("commit");
    assert_eq!(committed.transaction.target_node_ids, [fixture.source.id]);
    assert!(
        fixture
            .source
            .path
            .join("Child-task/Child-task.adoc")
            .is_file()
    );
}

#[test]
fn incomplete_annotation_replica_cannot_authorize_missing_sidecar_absence() {
    let fixture = fixture("* [ ] Partial\n");
    assert!(matches!(
        capture_annotation_sidecar_snapshot(
            &fixture.root.path,
            fixture.source.id,
            AnnotationReplicaCompleteness::PartialReplica,
        ),
        Err(WorkspaceTransactionError::IncompleteAnnotationReplica)
    ));
    assert!(matches!(
        capture_annotation_sidecar_snapshot(
            &fixture.root.path,
            fixture.source.id,
            AnnotationReplicaCompleteness::Unknown,
        ),
        Err(WorkspaceTransactionError::IncompleteAnnotationReplica)
    ));
}

#[test]
fn exact_annotations_outside_branch_rebase_and_inside_lifted_body_migrate() {
    let fixture = fixture("Prelude.\n\n* [ ] Parent\n** child\n");
    let outside = annotation(&fixture, "Prelude", AnnotationState::Open);
    let inside = annotation(&fixture, "child", AnnotationState::Open);
    let outside_id = outside.id;
    let inside_id = inside.id;
    write_annotations(&fixture, vec![outside, inside]);
    let request = request(
        &fixture,
        0,
        fixture.root.id,
        "Annotated",
        "Annotated",
        "Annotated",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot)
        .expect("annotation migration plan");
    assert_eq!(plan.summary().annotations.retained_in_source_count, 1);
    assert_eq!(plan.summary().annotations.migrated_to_task_node_count, 1);
    assert!(plan.summary().annotations.source_sidecar_rewritten);
    assert!(plan.summary().annotations.task_sidecar_created);
    let generated = plan.summary().generated_node_id;
    commit_task_promotion_transaction(&plan, &request).expect("commit");

    let source_store = AnnotationStore::from_json(
        &fs::read_to_string(fixture.source.path.join("weftext.annotations.json")).unwrap(),
    )
    .unwrap();
    let task_store = AnnotationStore::from_json(
        &fs::read_to_string(fixture.root.path.join("Annotated/weftext.annotations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(source_store.document_id, fixture.source.id);
    assert_eq!(task_store.document_id, generated);
    assert_eq!(source_store.annotations[0].id, outside_id);
    assert_eq!(task_store.annotations[0].id, inside_id);
    let source_revision = read_node_document(&fixture.source.path).unwrap().revision;
    let task_revision = read_node_document(fixture.root.path.join("Annotated"))
        .unwrap()
        .revision;
    assert!(matches!(
        &source_store.annotations[0].target,
        Anchor::TextRange { base_revision, .. } if base_revision == source_revision.as_str()
    ));
    assert!(matches!(
        &task_store.annotations[0].target,
        Anchor::TextRange { exact, base_revision, .. }
            if exact == "child" && base_revision == task_revision.as_str()
    ));
}

#[test]
fn principal_intersection_blocks_without_creating_task_or_rewriting_sidecar() {
    let fixture = fixture("* [ ] Parent\n** child\n");
    let principal = annotation(&fixture, "Parent", AnnotationState::Open);
    write_annotations(&fixture, vec![principal]);
    let original_sidecar = fs::read(fixture.source.path.join("weftext.annotations.json")).unwrap();
    let request = request(
        &fixture,
        0,
        fixture.root.id,
        "Blocked",
        "Blocked",
        "Blocked",
    );
    let snapshot = annotation_snapshot(&fixture);
    assert!(matches!(
        plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot),
        Err(TaskPromotionError::AnnotationBlockers { .. })
    ));
    assert!(!fixture.root.path.join("Blocked").exists());
    assert_eq!(
        fs::read(fixture.source.path.join("weftext.annotations.json")).unwrap(),
        original_sidecar
    );
}

#[test]
fn orphaned_source_annotation_is_preserved_without_automatic_reanchor() {
    let fixture = fixture("* [ ] Parent\n** child\n");
    let mut orphaned = annotation(&fixture, "child", AnnotationState::Orphaned);
    if let Anchor::TextRange { base_revision, .. } = &mut orphaned.target {
        *base_revision = "0".repeat(64);
    }
    let orphaned_id = orphaned.id;
    let original_target = orphaned.target.clone();
    write_annotations(&fixture, vec![orphaned]);
    let request = request(
        &fixture,
        0,
        fixture.root.id,
        "Orphan-safe",
        "Orphan safe",
        "Orphan safe",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot)
        .expect("orphan-preserving plan");
    assert_eq!(plan.summary().annotations.retained_in_source_count, 1);
    assert_eq!(plan.summary().annotations.migrated_to_task_node_count, 0);
    assert!(!plan.summary().annotations.source_sidecar_rewritten);
    commit_task_promotion_transaction(&plan, &request).expect("commit");
    let store = AnnotationStore::from_json(
        &fs::read_to_string(fixture.source.path.join("weftext.annotations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(store.annotations[0].id, orphaned_id);
    assert_eq!(store.annotations[0].target, original_target);
    assert_eq!(store.annotations[0].state, AnnotationState::Orphaned);
}

#[test]
fn relative_resource_include_and_xref_locators_fail_before_default_or_alternate_creation() {
    for attachment in [
        "image::picture.png[Alt]",
        "include::fragment.adoc[]",
        "See xref:relative-anchor[details].",
    ] {
        for alternate_parent in [false, true] {
            let fixture = fixture(&format!("* [ ] Relative\n+\n{attachment}\n"));
            let parent = if alternate_parent {
                create_child_node(&fixture.root.path, "Alternate").unwrap()
            } else {
                fixture.source.clone()
            };
            let request = request(
                &fixture,
                0,
                parent.id,
                "Unsafe-relative",
                "Unsafe relative",
                "Unsafe relative",
            );
            let snapshot = annotation_snapshot(&fixture);
            let result = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot);
            assert!(
                matches!(
                    result,
                    Err(TaskPromotionError::RelativeLocator
                        | TaskPromotionError::InvalidRequest
                        | TaskPromotionError::IncompletePromotionBranch)
                ),
                "attachment={attachment:?} alternate={alternate_parent} result={result:?}"
            );
            assert!(!parent.path.join("Unsafe-relative").exists());
        }
    }
}

#[test]
fn exact_casefold_nfc_nfd_reserved_and_device_name_conflicts_fail_without_suffixing() {
    for (existing, reviewed) in [
        ("Taken", "Taken"),
        ("CaseName", "casename"),
        ("Café", "Cafe\u{301}"),
        ("Straße", "STRASSE"),
    ] {
        let fixture = fixture("* [ ] Conflict\n");
        create_child_node(&fixture.root.path, existing).expect("existing child");
        let request = request(
            &fixture,
            0,
            fixture.root.id,
            reviewed,
            "Conflict",
            "Conflict",
        );
        assert!(matches!(
            plan_task_promotion_transaction(
                &fixture.root.path,
                &request,
                &annotation_snapshot(&fixture),
            ),
            Err(TaskPromotionError::DestinationConflict)
        ));
    }
    for invalid in ["CON", "LPT1", ".weftext-trash", "weftext.annotations.json"] {
        let fixture = fixture("* [ ] Invalid\n");
        let request = request(&fixture, 0, fixture.root.id, invalid, "Invalid", "Invalid");
        assert!(matches!(
            plan_task_promotion_transaction(
                &fixture.root.path,
                &request,
                &annotation_snapshot(&fixture),
            ),
            Err(TaskPromotionError::InvalidPortableName)
        ));
    }
}

#[test]
fn stale_document_workspace_dirty_gate_and_forged_serde_evidence_fail_closed() {
    let fixture = fixture("* [ ] Guarded\n");
    let request = request(
        &fixture,
        0,
        fixture.root.id,
        "Guarded",
        "Guarded",
        "Guarded",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).expect("plan");
    let dirty = WorkspaceDraftRegistryView::new("dirty", [fixture.source.id]).expect("registry");
    let preview = plan.preview_draft_gate(&dirty).expect("preview");
    assert_eq!(preview.blocking_dirty_node_ids, [fixture.source.id]);
    assert!(preview.executable_token.is_none());

    let clean = WorkspaceDraftRegistryView::new("clean", []).expect("registry");
    let token = plan
        .preview_draft_gate(&clean)
        .expect("preview")
        .executable_token
        .expect("token");
    assert!(matches!(
        commit_task_promotion_transaction_with_draft_gate(&plan, &request, &token, &dirty),
        Err(TaskPromotionError::WorkspaceTransaction(_))
    ));

    let mut value = serde_json::to_value(&request).expect("request JSON");
    value["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<TaskPromotionRequest>(value).is_err());
    let mut evidence = serde_json::to_value(&request.evidence).expect("evidence JSON");
    evidence["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ChecklistPromotionEvidence>(evidence).is_err());

    let mut forged_occurrence = request.clone();
    forged_occurrence
        .evidence
        .parser_occurrence
        .parser_ordinal_path
        .push(99);
    assert!(matches!(
        plan_task_promotion_transaction(
            &fixture.root.path,
            &forged_occurrence,
            &annotation_snapshot(&fixture),
        ),
        Err(TaskPromotionError::ParserEvidenceMismatch)
    ));

    fs::write(
        &fixture.source.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Source\n\n* [ ] changed\n",
            fixture.source.id
        ),
    )
    .expect("stale source");
    assert!(matches!(
        commit_task_promotion_transaction(&plan, &request),
        Err(TaskPromotionError::StaleWorkspaceRevision | TaskPromotionError::StaleDocumentRevision)
    ));
}

#[test]
fn subsequent_context_dependent_checklist_evidence_remains_exact_after_source_splice() {
    let fixture = fixture("* [ ] First\n\n* [ ] Later\n+\nUses {custom-name}.\n");
    let request = request(&fixture, 0, fixture.source.id, "First", "First", "First");
    let plan = plan_task_promotion_transaction(
        &fixture.root.path,
        &request,
        &annotation_snapshot(&fixture),
    )
    .expect("later parser context evidence shifts exactly");
    commit_task_promotion_transaction(&plan, &request).expect("commit first checklist");
    let source = fs::read_to_string(&fixture.source.document_path).unwrap();
    assert!(source.contains("* [ ] Later\n+\nUses {custom-name}.\n"));
}

#[test]
fn scoped_source_and_parent_authority_is_required_and_commit_scope_is_exact() {
    let fixture = fixture("* [ ] Scoped\n");
    let request = request(&fixture, 0, fixture.root.id, "Scoped", "Scoped", "Scoped");
    let complete = WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(fixture.root.id, None, ""),
        WorkspaceNodeProjection::new(fixture.source.id, Some(fixture.root.id), "Source"),
    ])
    .expect("scope");
    let source_only = WorkspaceReadScope::new([WorkspaceNodeProjection::new(
        fixture.source.id,
        None,
        "Source",
    )])
    .expect("source-only scope");
    assert!(matches!(
        plan_task_promotion_transaction_scoped(
            &fixture.root.path,
            &request,
            &annotation_snapshot(&fixture),
            &source_only,
        ),
        Err(TaskPromotionError::TargetUnavailable)
    ));
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction_scoped(&fixture.root.path, &request, &snapshot, &complete)
            .expect("scoped plan");
    assert!(matches!(
        commit_task_promotion_transaction_scoped(&plan, &request, &source_only),
        Err(TaskPromotionError::AuthorizationChanged)
    ));
    commit_task_promotion_transaction_scoped(&plan, &request, &complete).expect("scoped commit");
}

#[test]
fn generated_identity_is_not_a_request_field_and_inventory_has_one_task_node() {
    let fixture = fixture("* [ ] Identity\n");
    let request = request(
        &fixture,
        0,
        fixture.root.id,
        "Identity",
        "Identity",
        "Identity",
    );
    let json = serde_json::to_value(&request).expect("request JSON");
    assert!(json.get("generatedNodeId").is_none());
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).expect("plan");
    let generated = plan.summary().generated_node_id;
    commit_task_promotion_transaction(&plan, &request).expect("commit");
    let inventory = scan_workspace(&fixture.root.path);
    assert_eq!(
        inventory
            .nodes
            .iter()
            .filter(|node| node.id == Some(generated))
            .count(),
        1
    );
}

#[test]
fn document_local_fragments_anchors_attributes_conditionals_and_named_notes_fail_closed() {
    for attachment in [
        "See xref:#outside[details].",
        "[[inside-anchor]]\nAnchored paragraph.",
        "Uses {custom-name} and {docname} from the source header.",
        "ifdef::custom-name[]\nConditional text.\nendif::[]",
        "Named note footnote:source-note[definition].",
        "Named note footnote:source-note[].",
    ] {
        let fixture = fixture(&format!(
            "Outside [[outside]] reference.\n\n* [ ] Context\n+\n{attachment}\n"
        ));
        let request = request(
            &fixture,
            0,
            fixture.source.id,
            "Context-dependent",
            "Context dependent",
            "Context dependent",
        );
        let snapshot = annotation_snapshot(&fixture);
        let result = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot);
        assert!(
            matches!(
                result,
                Err(TaskPromotionError::DocumentContextDependency
                    | TaskPromotionError::InvalidRequest
                    | TaskPromotionError::IncompletePromotionBranch)
            ),
            "attachment={attachment:?} result={result:?}"
        );
        assert!(!fixture.source.path.join("Context-dependent").exists());
    }
}

#[test]
fn promotion_v2_journal_is_closed_and_tamper_or_wrong_root_fails_recovery() {
    let fixture = fixture("* [ ] Journal\n");
    let request = request(
        &fixture,
        0,
        fixture.source.id,
        "Journal-task",
        "Journal task",
        "Journal task",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).unwrap();
    let transaction = prepare_task_promotion_recovery_fixture(&plan).unwrap();
    let journal_path = transaction.join("journal.json");
    let mut journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    assert_eq!(journal["schema"], "weftext.workspace-transaction.v2");
    assert!(journal.get("promotion_summary").is_some());
    assert!(journal.get("task_promotion_authority").is_some());
    journal["action"] = serde_json::json!("rename");
    fs::write(&journal_path, serde_json::to_vec_pretty(&journal).unwrap()).unwrap();
    assert!(matches!(
        recover_workspace_transactions(&fixture.root.path),
        Err(WorkspaceTransactionError::InvalidJournal(_))
    ));

    let foreign = tempdir().unwrap();
    let foreign_root = create_workspace(foreign.path().join("Foreign")).unwrap();
    let foreign_transaction = foreign_root.path.join(transaction.file_name().unwrap());
    fs::rename(&transaction, &foreign_transaction).unwrap();
    assert!(recover_workspace_transactions(&foreign_root.path).is_err());
}

#[test]
fn unfinished_v1_prepared_journal_remains_recoverable() {
    let temporary = tempdir().unwrap();
    let root = create_workspace(temporary.path().join("V1")).unwrap();
    let plan = plan_create_child_node(&root.path, root.id, "Prepared-v1").unwrap();
    let transaction = prepare_workspace_transaction_recovery_fixture(&plan).unwrap();
    let journal: serde_json::Value =
        serde_json::from_slice(&fs::read(transaction.join("journal.json")).unwrap()).unwrap();
    assert_eq!(journal["schema"], "weftext.workspace-transaction.v1");
    assert!(journal.get("promotion_summary").is_none());
    assert!(journal.get("task_promotion_authority").is_none());
    let report = recover_workspace_transactions(&root.path).unwrap();
    assert_eq!(report.prepared_removed, 1);
    assert!(!root.path.join("Prepared-v1").exists());
}

#[test]
fn every_three_step_promotion_journal_boundary_rolls_back_exactly() {
    for boundary in 0..=3 {
        let fixture = fixture("Prelude.\n\n* [ ] Parent\n** child\n");
        let outside = annotation(&fixture, "Prelude", AnnotationState::Open);
        let inside = annotation(&fixture, "child", AnnotationState::Open);
        write_annotations(&fixture, vec![outside, inside]);
        let original_document = fs::read(&fixture.source.document_path).unwrap();
        let original_sidecar =
            fs::read(fixture.source.path.join("weftext.annotations.json")).unwrap();
        let request = request(
            &fixture,
            0,
            fixture.source.id,
            "Crash-safe",
            "Crash safe",
            "Crash safe",
        );
        let snapshot = annotation_snapshot(&fixture);
        let plan =
            plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).unwrap();
        let transaction =
            prepare_task_promotion_applying_recovery_fixture(&plan, boundary).unwrap();
        assert!(transaction.is_dir(), "boundary {boundary}");
        let report = recover_workspace_transactions(&fixture.root.path).unwrap();
        assert_eq!(report.applying_rolled_back, 1, "boundary {boundary}");
        assert_eq!(
            fs::read(&fixture.source.document_path).unwrap(),
            original_document,
            "boundary {boundary}"
        );
        assert_eq!(
            fs::read(fixture.source.path.join("weftext.annotations.json")).unwrap(),
            original_sidecar,
            "boundary {boundary}"
        );
        assert!(!fixture.source.path.join("Crash-safe").exists());
        let second = recover_workspace_transactions(&fixture.root.path).unwrap();
        assert_eq!(second, report, "boundary {boundary}");
        assert_eq!(
            fs::read(&fixture.source.document_path).unwrap(),
            original_document,
            "second recovery boundary {boundary}"
        );
        assert_eq!(
            fs::read(fixture.source.path.join("weftext.annotations.json")).unwrap(),
            original_sidecar,
            "second recovery boundary {boundary}"
        );
        assert!(!fixture.source.path.join("Crash-safe").exists());
    }
}

#[test]
fn post_apply_semantic_verification_failure_rolls_back_before_returning_error() {
    let fixture = fixture("* [ ] Verify rollback\n");
    let original = fs::read(&fixture.source.document_path).unwrap();
    let request = request(
        &fixture,
        0,
        fixture.source.id,
        "Verify-rollback",
        "Verify rollback",
        "Verify rollback",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan = plan_task_promotion_transaction(&fixture.root.path, &request, &snapshot).unwrap();
    assert!(matches!(
        commit_task_promotion_with_injected_verification_failure_for_recovery_fixture(&plan),
        Err(TaskPromotionError::WorkspaceTransaction(
            WorkspaceTransactionError::VerificationFailed(_)
        ))
    ));
    assert_eq!(fs::read(&fixture.source.document_path).unwrap(), original);
    assert!(!fixture.source.path.join("Verify-rollback").exists());
    assert!(!weftext_core::has_unfinished_workspace_transaction(&fixture.root.path).unwrap());
}

#[test]
fn scoped_promoted_root_never_discloses_hidden_physical_ancestors() {
    let temporary = tempdir().unwrap();
    let root = create_workspace(temporary.path().join("Scoped-root")).unwrap();
    let hidden = create_child_node(&root.path, "SecretAncestor").unwrap();
    let visible = create_child_node(&hidden.path, "Visible").unwrap();
    fs::write(
        &visible.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Visible\n\n* [ ] Scoped child\n",
            visible.id
        ),
    )
    .unwrap();
    let fixture = Fixture {
        _temporary: temporary,
        root,
        source: visible,
    };
    let scope = WorkspaceReadScope::new([WorkspaceNodeProjection::new(
        fixture.source.id,
        None,
        "Visible",
    )])
    .unwrap();
    let request = request(
        &fixture,
        0,
        fixture.source.id,
        "Logical-child",
        "Logical child",
        "Logical child",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction_scoped(&fixture.root.path, &request, &snapshot, &scope)
            .unwrap();
    assert_eq!(plan.summary().generated_path, "Visible/Logical-child");
    assert!(!format!("{plan:?}").contains("SecretAncestor"));
    let committed = commit_task_promotion_transaction_scoped(&plan, &request, &scope).unwrap();
    assert_eq!(committed.summary.generated_path, "Visible/Logical-child");
    assert_eq!(
        committed.transaction.path_changes[0].new_path,
        "Visible/Logical-child"
    );
    assert_eq!(
        committed
            .transaction
            .promotion_summary
            .as_ref()
            .unwrap()
            .generated_path,
        "Visible/Logical-child"
    );
    assert!(!format!("{committed:?}").contains("SecretAncestor"));
    assert!(
        fixture
            .source
            .path
            .join("Logical-child/Logical-child.adoc")
            .is_file()
    );
}

#[test]
fn scoped_v2_recovery_uses_private_physical_paths_and_public_logical_receipts() {
    let temporary = tempdir().unwrap();
    let root = create_workspace(temporary.path().join("Scoped-recovery")).unwrap();
    let hidden = create_child_node(&root.path, "SecretAncestor").unwrap();
    let visible = create_child_node(&hidden.path, "Visible").unwrap();
    fs::write(
        &visible.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Visible\n\n* [ ] Recovered child\n",
            visible.id
        ),
    )
    .unwrap();
    let fixture = Fixture {
        _temporary: temporary,
        root,
        source: visible,
    };
    let scope = WorkspaceReadScope::new([WorkspaceNodeProjection::new(
        fixture.source.id,
        None,
        "Visible",
    )])
    .unwrap();
    let request = request(
        &fixture,
        0,
        fixture.source.id,
        "Recovered-child",
        "Recovered child",
        "Recovered child",
    );
    let snapshot = annotation_snapshot(&fixture);
    let plan =
        plan_task_promotion_transaction_scoped(&fixture.root.path, &request, &snapshot, &scope)
            .unwrap();

    let prepared = prepare_task_promotion_recovery_fixture(&plan).unwrap();
    assert!(prepared.is_dir());
    assert_eq!(
        recover_workspace_transactions(&fixture.root.path)
            .unwrap()
            .prepared_removed,
        1
    );

    let applying = prepare_task_promotion_applying_recovery_fixture(&plan, 1).unwrap();
    assert!(applying.is_dir());
    assert_eq!(
        recover_workspace_transactions(&fixture.root.path)
            .unwrap()
            .applying_rolled_back,
        1
    );
    assert!(!fixture.source.path.join("Recovered-child").exists());

    let committed = prepare_task_promotion_committed_recovery_fixture(&plan).unwrap();
    assert!(committed.is_dir());
    let report = recover_workspace_transactions(&fixture.root.path).unwrap();
    assert_eq!(report.committed_cleaned, 1);
    let [receipt] = report.committed_transactions.as_slice() else {
        panic!("expected one recovered promotion receipt");
    };
    assert_eq!(receipt.path_changes[0].new_path, "Visible/Recovered-child");
    assert_eq!(
        receipt.promotion_summary.as_ref().unwrap().generated_path,
        "Visible/Recovered-child"
    );
    assert!(!format!("{report:?}").contains("SecretAncestor"));
    assert!(
        fixture
            .source
            .path
            .join("Recovered-child/Recovered-child.adoc")
            .is_file()
    );
}
