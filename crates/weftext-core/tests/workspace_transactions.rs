use std::fs;

use tempfile::tempdir;
use weftext_core::{
    AdjacentHeadingBody, Anchor, AnnotationAction, AnnotationAppearance, AnnotationColor,
    AnnotationKind, AnnotationMark, AnnotationReplicaCompleteness, AnnotationStore,
    AnnotationTargetIntent, CalendarDate, ChronoPeriod, DocumentProfileId, NodeId,
    StructuralAction, WorkspaceDraftRegistryView, WorkspaceIdentityPolicy,
    WorkspaceTransactionError, WorkspaceTransactionPlan, analyze_document_for_profile,
    capture_annotation_sidecar_snapshot, commit_workspace_transaction, create_child_node,
    create_workspace, plan_adjacent_heading_body_setting, plan_annotation_action,
    plan_chrono_nodes, plan_copy_node, plan_create_child_node, plan_move_node, plan_rename_node,
    plan_restore_node, plan_trash_node, preview_workspace_transaction_draft_gate,
    read_node_annotations, read_workspace_revision, scan_workspace,
};

fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    (temporary, workspace)
}

fn append_document(node: &std::path::Path, text: &str) {
    let name = node.file_name().unwrap().to_str().unwrap();
    let path = node.join(format!("{name}.adoc"));
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str(text);
    fs::write(path, source).unwrap();
}

fn read_local_annotations(root: &std::path::Path, node_id: NodeId) -> AnnotationStore {
    read_node_annotations(
        root,
        node_id,
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )
    .unwrap()
}

fn plan_local_annotation(
    root: &std::path::Path,
    node_id: NodeId,
    action: AnnotationAction,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let snapshot = capture_annotation_sidecar_snapshot(
        root,
        node_id,
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )?;
    plan_annotation_action(root, &snapshot, action)
}

#[test]
fn create_plan_is_revision_checked_and_has_no_partial_stale_write() {
    let (_temporary, workspace) = setup();
    let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
    let plan = plan_create_child_node(&workspace, root_id, "Draft").unwrap();
    assert_eq!(plan.path_changes[0].new_path, "Draft");
    assert!(!workspace.join("Draft").exists());

    append_document(&workspace, "\nChanged after preview.\n");
    let changed = read_workspace_revision(&workspace).unwrap();
    let error = commit_workspace_transaction(&plan).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceTransactionError::StaleRevision { .. }
    ));
    assert_eq!(read_workspace_revision(&workspace).unwrap(), changed);
    assert!(!workspace.join("Draft").exists());
}

#[test]
fn move_preserves_identity_and_uuid_links_without_rewriting_documents() {
    let (_temporary, workspace) = setup();
    let alpha = create_child_node(&workspace, "Alpha").unwrap();
    let group = create_child_node(&workspace, "Group").unwrap();
    append_document(&workspace, &format!("\nSee node:{}[Alpha].\n", alpha.id));

    let plan = plan_move_node(&workspace, alpha.id, group.id, "Alpha").unwrap();
    assert!(plan.document_changes.is_empty());
    commit_workspace_transaction(&plan).unwrap();

    let destination = workspace.join("Group").join("Alpha");
    assert!(!workspace.join("Alpha").exists());
    assert!(destination.join("Alpha.adoc").is_file());
    assert_eq!(
        scan_workspace(&workspace)
            .nodes
            .iter()
            .find(|node| node.path == destination)
            .unwrap()
            .id,
        Some(alpha.id)
    );
    assert!(
        fs::read_to_string(workspace.join("Notes.adoc"))
            .unwrap()
            .contains(&format!("node:{}[Alpha]", alpha.id))
    );
}

#[test]
fn branch_scope_identity_map_and_draft_gate_are_core_authority() {
    let (_temporary, workspace) = setup();
    let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
    let source = create_child_node(&workspace, "Source").unwrap();
    let child = create_child_node(&source.path, "Child").unwrap();
    let destination = create_child_node(&workspace, "Destination").unwrap();
    fs::write(source.path.join("asset.bin"), b"scope resource").unwrap();
    append_document(&workspace, "\nSee node:Source[Source].\n");

    let rename = plan_rename_node(&workspace, source.id, "Renamed").unwrap();
    assert_eq!(rename.action, StructuralAction::Rename);
    assert_eq!(
        rename.scope_summary.as_ref().unwrap().identity_policy,
        WorkspaceIdentityPolicy::Preserve
    );
    assert!(matches!(
        plan_move_node(&workspace, source.id, destination.id, "Renamed"),
        Err(WorkspaceTransactionError::Metadata(_))
    ));

    let copy = plan_copy_node(&workspace, source.id, destination.id, "Copy").unwrap();
    let scope = copy.scope_summary.as_ref().unwrap();
    assert_eq!(scope.root_node.node_id, source.id);
    assert_eq!(scope.root_node.display_name, "Source");
    assert_eq!(scope.descendant_node_count, 1);
    assert_eq!(scope.resource_count, 1);
    assert_eq!(scope.identity_policy, WorkspaceIdentityPolicy::Rekey);
    assert_eq!(copy.identity_map.len(), 2);
    assert!(
        copy.identity_map
            .iter()
            .any(|entry| entry.source_node_id == source.id)
    );
    assert!(
        copy.identity_map
            .iter()
            .any(|entry| entry.source_node_id == child.id)
    );
    assert!(copy.draft_sensitive_node_ids.contains(&source.id));
    assert!(copy.draft_sensitive_node_ids.contains(&child.id));

    let blocked = preview_workspace_transaction_draft_gate(
        &copy,
        &WorkspaceDraftRegistryView::new("device:1", [child.id]).unwrap(),
    )
    .unwrap();
    assert_eq!(blocked.blocking_dirty_node_ids, vec![child.id]);
    assert!(blocked.executable_token.is_none());
    let unrelated = NodeId::new_v4();
    let clean = preview_workspace_transaction_draft_gate(
        &copy,
        &WorkspaceDraftRegistryView::new("device:2", [unrelated]).unwrap(),
    )
    .unwrap();
    assert!(clean.blocking_dirty_node_ids.is_empty());
    assert!(clean.executable_token.is_some());

    let moving = plan_move_node(&workspace, source.id, destination.id, "Source").unwrap();
    assert!(moving.draft_sensitive_node_ids.contains(&root_id));
    let clean_view = WorkspaceDraftRegistryView::new("device:3", []).unwrap();
    let preview = preview_workspace_transaction_draft_gate(&moving, &clean_view).unwrap();
    let before = read_workspace_revision(&workspace).unwrap();
    let error = weftext_core::commit_workspace_transaction_with_draft_gate(
        &moving,
        preview.executable_token.as_ref().unwrap(),
        &WorkspaceDraftRegistryView::new("device:4", [root_id]).unwrap(),
    )
    .unwrap_err();
    assert!(
        matches!(error, WorkspaceTransactionError::DraftGateBlocked(ids) if ids == vec![root_id])
    );
    assert_eq!(read_workspace_revision(&workspace).unwrap(), before);
    assert!(source.path.is_dir());
}

#[test]
fn copy_rekeys_every_node_and_preserves_resources() {
    let (_temporary, workspace) = setup();
    let source = create_child_node(&workspace, "Source").unwrap();
    let child = create_child_node(&source.path, "Child").unwrap();
    append_document(
        &source.path,
        &format!("\nCopied link: node:{}[Child].\n", child.id),
    );
    fs::write(source.path.join("asset.bin"), b"payload").unwrap();
    let root_id = scan_workspace(&workspace)
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .unwrap()
        .id
        .unwrap();

    let plan = plan_copy_node(&workspace, source.id, root_id, "Copy").unwrap();
    assert_eq!(plan.generated_node_ids.len(), 2);
    commit_workspace_transaction(&plan).unwrap();

    let inventory = scan_workspace(&workspace);
    assert!(inventory.is_valid());
    let copied = inventory
        .nodes
        .iter()
        .find(|node| node.path == workspace.join("Copy"))
        .unwrap();
    let copied_child = inventory
        .nodes
        .iter()
        .find(|node| node.path == workspace.join("Copy/Child"))
        .unwrap();
    assert_ne!(copied.id, Some(source.id));
    assert_ne!(copied_child.id, Some(child.id));
    assert_eq!(
        fs::read(workspace.join("Copy/asset.bin")).unwrap(),
        b"payload"
    );
    assert!(
        fs::read_to_string(workspace.join("Copy/Copy.adoc"))
            .unwrap()
            .contains(&copied_child.id.unwrap().to_string())
    );
}

#[test]
fn trash_and_explicit_restore_preserve_identity() {
    let (_temporary, workspace) = setup();
    let note = create_child_node(&workspace, "Note").unwrap();
    append_document(
        &workspace,
        &format!("\nSee node:{}[Note]. A Note mention.\n", note.id),
    );
    let root_id = scan_workspace(&workspace)
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .unwrap()
        .id
        .unwrap();

    commit_workspace_transaction(&plan_trash_node(&workspace, note.id).unwrap()).unwrap();
    let inventory = scan_workspace(&workspace);
    assert!(inventory.is_valid());
    assert_eq!(inventory.trash_items.len(), 1);
    let trashed = &inventory.trash_items[0];
    assert_eq!(trashed.manifest.node_id(), Some(note.id));
    assert!(trashed.payload_path.join("Note.adoc").is_file());
    assert!(inventory.nodes.iter().all(|node| node.id != Some(note.id)));
    let trashed_index = weftext_core::build_workspace_link_index(&workspace).unwrap();
    assert!(trashed_index.nodes.iter().all(|node| node.id != note.id));
    assert!(
        trashed_index
            .outgoing
            .iter()
            .all(|link| !link.target_node_ids.contains(&note.id))
    );
    assert!(
        !trashed_index
            .potential_mentions
            .iter()
            .any(|mention| mention.target_node_ids.contains(&note.id))
    );

    commit_workspace_transaction(
        &plan_restore_node(&workspace, note.id, root_id, "Restored").unwrap(),
    )
    .unwrap();
    assert!(workspace.join("Restored/Restored.adoc").is_file());
    assert!(
        fs::read_to_string(workspace.join("Notes.adoc"))
            .unwrap()
            .contains(&note.id.to_string())
    );
    assert_eq!(
        scan_workspace(&workspace)
            .nodes
            .iter()
            .find(|node| node.path == workspace.join("Restored"))
            .unwrap()
            .id,
        Some(note.id)
    );
}

#[test]
fn portable_run_in_setting_is_narrowly_patched_and_drives_the_block_model() {
    let (_temporary, workspace) = setup();
    append_document(&workspace, "\n[#heading]\n== Heading\nBody text.\n");
    let before = fs::read_to_string(workspace.join("Notes.adoc")).unwrap();
    let plan = plan_adjacent_heading_body_setting(&workspace, AdjacentHeadingBody::RunIn).unwrap();
    assert_eq!(plan.document_changes.len(), 1);
    commit_workspace_transaction(&plan).unwrap();

    let after = fs::read_to_string(workspace.join("Notes.adoc")).unwrap();
    assert!(after.contains("  adjacent_heading_body: run_in\n"));
    assert!(after.ends_with("[#heading]\n== Heading\nBody text.\n"));
    assert!(after.len() > before.len());
    let model = analyze_document_for_profile(
        DocumentProfileId::AsciiDocV1,
        &after,
        AdjacentHeadingBody::RunIn,
    )
    .model;
    assert_eq!(model.run_in_groups.len(), 1);
    assert_eq!(
        model.blocks[usize::try_from(model.run_in_groups[0].heading_block).unwrap()]
            .block_id
            .as_deref(),
        Some("heading")
    );
    assert_eq!(
        model.blocks[usize::try_from(model.run_in_groups[0].body_block).unwrap()]
            .block_id
            .as_deref(),
        None
    );
}

#[test]
fn annotation_sidecar_creation_and_reply_use_recoverable_workspace_transactions() {
    let (_temporary, workspace) = setup();
    append_document(
        &workspace,
        "\n= Review\n\n========= Deep review\n\nBlock to review.\n",
    );
    let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
    let source = fs::read_to_string(workspace.join("Notes.adoc")).unwrap();
    let offset = u64::try_from(source.find("Block to review").unwrap()).unwrap();
    let create = plan_local_annotation(
        &workspace,
        root_id,
        AnnotationAction::Create {
            kind: AnnotationKind::Comment,
            target: AnnotationTargetIntent::BlockAt {
                source_offset: offset,
            },
            appearance: Some(AnnotationAppearance {
                mark: AnnotationMark::Highlight,
                color: AnnotationColor::Yellow,
            }),
            labels: vec!["verify".to_owned()],
            body_source: Some("Check this block".to_owned()),
            suggested_source: None,
            author_id: uuid::Uuid::new_v4(),
            author_name: "Reviewer".to_owned(),
            timestamp: "2026-08-21T12:00:00Z".to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&create).unwrap();
    let store = read_local_annotations(&workspace, root_id);
    assert_eq!(store.annotations.len(), 1);
    let annotation_id = store.annotations[0].id;
    assert_eq!(store.annotations[0].thread.len(), 1);
    assert!(matches!(
        &store.annotations[0].target,
        Anchor::Block { heading_path, .. }
            if heading_path == &["Deep review".to_owned()]
    ));

    let reply = plan_local_annotation(
        &workspace,
        root_id,
        AnnotationAction::Reply {
            annotation_id,
            body_source: "Confirmed".to_owned(),
            author_id: uuid::Uuid::new_v4(),
            author_name: "Reviewer".to_owned(),
            timestamp: "2026-08-21T12:05:00Z".to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&reply).unwrap();
    let updated = read_local_annotations(&workspace, root_id);
    assert_eq!(updated.annotations[0].thread.len(), 2);
    assert_eq!(updated.annotations[0].thread[1].body.source, "Confirmed");
}

#[test]
fn chrono_plan_creates_fixed_same_named_nodes_in_one_recoverable_transaction() {
    let (_temporary, workspace) = setup();
    let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
    let date = CalendarDate::new(2026, 8, 21).unwrap();
    let plan = plan_chrono_nodes(
        &workspace,
        root_id,
        date,
        &[
            ChronoPeriod::Quarter,
            ChronoPeriod::Month,
            ChronoPeriod::Week,
            ChronoPeriod::Day,
        ],
    )
    .unwrap();
    assert_eq!(plan.path_changes.len(), 5);
    assert!(!workspace.join("2026").exists());

    commit_workspace_transaction(&plan).unwrap();
    for relative in [
        "2026/2026.adoc",
        "2026/2026-Q3/2026-Q3.adoc",
        "2026/2026-08/2026-08.adoc",
        "2026/2026-W34/2026-W34.adoc",
        "2026/2026-08-21/2026-08-21.adoc",
    ] {
        assert!(workspace.join(relative).is_file(), "missing {relative}");
    }
    assert!(scan_workspace(&workspace).is_valid());
}
