use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use weftext_core::{
    TRASH_ITEM_MANIFEST_FILE_NAME, TRASH_ITEMS_DIRECTORY_NAME,
    TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE, TrashItemKind, TrashOriginStatus,
    TrashResourceSelection, TrashRestoreBlockedReason, TrashRestoreMode,
    TrashReviewedReplanAuthorization, TrashReviewedRequest, WORKSPACE_TRANSACTION_LEASE_FILE_NAME,
    WorkspaceIdentityPolicy, WorkspaceTransactionError, WorkspaceTrashState,
    acquire_workspace_transaction_lease, build_workspace_link_index, build_workspace_navigation,
    commit_workspace_transaction, confirm_permanent_delete_trash_items, create_child_node,
    create_workspace, load_legacy_trash_migration_backup, plan_migrate_legacy_workspace_trash_at,
    plan_migrate_legacy_workspace_trash_at_with_backup, plan_permanently_delete_trash_items,
    plan_restore_trash_item, plan_trash_node_at, plan_trash_resources_at,
    prepare_legacy_trash_migration_backup, prepare_workspace_transaction_recovery_fixture,
    preview_permanent_delete_trash_items, project_workspace_trash_items,
    project_workspace_trash_state, read_workspace_revision, recover_workspace_transactions,
    replan_reviewed_trash_request, scan_workspace,
};

const TIME: &str = "2026-08-24T12:00:00+08:00";

fn setup() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    (temporary, workspace)
}

fn root_id(workspace: &Path) -> weftext_core::NodeId {
    scan_workspace(workspace)
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .unwrap()
        .id
        .unwrap()
}

#[test]
fn public_transaction_lease_creates_one_durable_anchor_and_excludes_other_owners() {
    let (_temporary, workspace) = setup();
    let anchor = workspace.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
    assert!(!anchor.exists());

    let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
    let metadata = fs::metadata(&anchor).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.len(), 0);
    assert!(acquire_workspace_transaction_lease(&workspace).is_err());

    drop(lease);
    let replacement = acquire_workspace_transaction_lease(&workspace).unwrap();
    drop(replacement);
    assert_eq!(fs::metadata(&anchor).unwrap().len(), 0);
    fs::write(&anchor, b"tampered anchor").unwrap();
    assert!(acquire_workspace_transaction_lease(&workspace).is_err());
    assert_eq!(fs::read(anchor).unwrap(), b"tampered anchor");
}

#[test]
fn node_items_allow_repeated_names_and_restore_never_overwrites() {
    let (_temporary, workspace) = setup();
    let first = create_child_node(&workspace, "Same").unwrap();
    fs::write(first.path.join("asset.bin"), b"first").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, first.id, TIME).unwrap()).unwrap();
    let second = create_child_node(&workspace, "Same").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, second.id, TIME).unwrap())
        .unwrap();

    let projected = project_workspace_trash_items(&workspace).unwrap();
    assert_eq!(projected.len(), 2);
    assert_ne!(
        projected[0].manifest.trash_item_id(),
        projected[1].manifest.trash_item_id()
    );
    assert!(
        projected
            .iter()
            .all(|item| item.manifest.original_name() == "Same")
    );

    create_child_node(&workspace, "Same").unwrap();
    let first_item = projected
        .iter()
        .find(|item| item.manifest.node_id() == Some(first.id))
        .unwrap();
    assert!(matches!(
        plan_restore_trash_item(
            &workspace,
            first_item.manifest.trash_item_id(),
            TrashRestoreMode::Original,
        ),
        Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            TrashRestoreBlockedReason::NameConflict
        ))
    ));
    let restore = plan_restore_trash_item(
        &workspace,
        first_item.manifest.trash_item_id(),
        TrashRestoreMode::ExistingTarget {
            target_node_id: root_id(&workspace),
            name: "Recovered".to_owned(),
        },
    )
    .unwrap();
    commit_workspace_transaction(&restore).unwrap();
    assert_eq!(
        fs::read(workspace.join("Recovered/asset.bin")).unwrap(),
        b"first"
    );

    commit_workspace_transaction(&plan_trash_node_at(&workspace, first.id, TIME).unwrap()).unwrap();
    let repeated_item_id = scan_workspace(&workspace)
        .trash_items
        .iter()
        .find(|item| item.manifest.node_id() == Some(first.id))
        .unwrap()
        .manifest
        .trash_item_id();
    commit_workspace_transaction(
        &plan_restore_trash_item(&workspace, repeated_item_id, TrashRestoreMode::Original).unwrap(),
    )
    .unwrap();
    assert!(workspace.join("Recovered/Recovered.adoc").is_file());
    assert_eq!(scan_workspace(&workspace).trash_items.len(), 1);
}

#[test]
fn same_named_resource_batch_has_one_operation_and_each_file_restores_independently() {
    let (_temporary, workspace) = setup();
    let owner = create_child_node(&workspace, "OwnerA").unwrap();
    let other_owner = create_child_node(&workspace, "OwnerB").unwrap();
    fs::write(owner.path.join("a.bin"), b"alpha").unwrap();
    fs::write(other_owner.path.join("a.bin"), b"beta").unwrap();
    let plan = plan_trash_resources_at(
        &workspace,
        vec![
            TrashResourceSelection {
                owner_node_id: owner.id,
                name: "a.bin".to_owned(),
            },
            TrashResourceSelection {
                owner_node_id: other_owner.id,
                name: "a.bin".to_owned(),
            },
        ],
        TIME,
    )
    .unwrap();
    assert_eq!(plan.trash_item_changes().len(), 2);
    assert!(plan.scope_summary.is_none());
    assert!(plan.draft_sensitive_node_ids.contains(&owner.id));
    assert!(plan.draft_sensitive_node_ids.contains(&other_owner.id));
    assert_eq!(
        plan.trash_item_changes()[0].manifest.operation_id(),
        plan.trash_item_changes()[1].manifest.operation_id()
    );
    commit_workspace_transaction(&plan).unwrap();
    assert!(!owner.path.join("a.bin").exists());
    assert!(!other_owner.path.join("a.bin").exists());

    let item = project_workspace_trash_items(&workspace)
        .unwrap()
        .into_iter()
        .find(|item| item.manifest.original_owner_node_id() == Some(owner.id))
        .unwrap();
    assert_eq!(item.manifest.kind(), TrashItemKind::Resource);
    fs::write(owner.path.join("a.bin"), b"occupied").unwrap();
    assert!(matches!(
        plan_restore_trash_item(
            &workspace,
            item.manifest.trash_item_id(),
            TrashRestoreMode::Original,
        ),
        Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            TrashRestoreBlockedReason::NameConflict
        ))
    ));
    fs::remove_file(owner.path.join("a.bin")).unwrap();
    commit_workspace_transaction(
        &plan_restore_trash_item(
            &workspace,
            item.manifest.trash_item_id(),
            TrashRestoreMode::Original,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(owner.path.join("a.bin")).unwrap(), b"alpha");
    assert!(!other_owner.path.join("a.bin").exists());
    assert_eq!(scan_workspace(&workspace).trash_items.len(), 1);
}

#[test]
fn deleting_parent_with_live_descendants_creates_one_complete_item() {
    let (_temporary, workspace) = setup();
    let parent = create_child_node(&workspace, "WholeTree").unwrap();
    let child = create_child_node(&parent.path, "Nested").unwrap();
    fs::write(child.path.join("asset.bin"), b"nested bytes").unwrap();

    let plan = plan_trash_node_at(&workspace, parent.id, TIME).unwrap();
    let scope = plan.scope_summary.as_ref().unwrap();
    assert_eq!(scope.root_node.node_id, parent.id);
    assert_eq!(scope.descendant_node_count, 1);
    assert_eq!(scope.resource_count, 1);
    assert_eq!(scope.identity_policy, WorkspaceIdentityPolicy::Preserve);
    assert_eq!(scope.trash_item_count, 1);
    assert_eq!(
        scope.operation_id,
        Some(plan.trash_item_changes()[0].manifest.operation_id())
    );
    assert!(plan.draft_sensitive_node_ids.contains(&parent.id));
    assert!(plan.draft_sensitive_node_ids.contains(&child.id));
    commit_workspace_transaction(&plan).unwrap();
    let inventory = scan_workspace(&workspace);
    assert_eq!(inventory.trash_items.len(), 1);
    assert_eq!(inventory.trash_items[0].manifest.node_id(), Some(parent.id));
    assert!(
        inventory.trash_items[0]
            .node_locators
            .contains_key(&parent.id)
    );
    assert!(
        inventory.trash_items[0]
            .node_locators
            .contains_key(&child.id)
    );
    assert_eq!(
        fs::read(
            inventory.trash_items[0]
                .payload_path
                .join("Nested/asset.bin")
        )
        .unwrap(),
        b"nested bytes"
    );
    let links = build_workspace_link_index(&workspace).unwrap();
    assert!(
        links
            .nodes
            .iter()
            .all(|node| !node.locator.starts_with(".weftext-trash"))
    );
}

#[test]
fn resource_restore_rejects_case_fold_collision() {
    let (_temporary, workspace) = setup();
    let owner = create_child_node(&workspace, "Owner").unwrap();
    fs::write(owner.path.join("case.bin"), b"original").unwrap();
    commit_workspace_transaction(
        &plan_trash_resources_at(
            &workspace,
            vec![TrashResourceSelection {
                owner_node_id: owner.id,
                name: "case.bin".to_owned(),
            }],
            TIME,
        )
        .unwrap(),
    )
    .unwrap();
    let item_id = scan_workspace(&workspace).trash_items[0]
        .manifest
        .trash_item_id();
    fs::write(owner.path.join("CASE.BIN"), b"occupied").unwrap();
    assert!(matches!(
        plan_restore_trash_item(&workspace, item_id, TrashRestoreMode::Original),
        Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            TrashRestoreBlockedReason::NameConflict | TrashRestoreBlockedReason::CaseFoldConflict
        ))
    ));
}

#[test]
fn node_restore_rejects_case_fold_collision() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "CaseNode").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, node.id, TIME).unwrap()).unwrap();
    let item_id = scan_workspace(&workspace).trash_items[0]
        .manifest
        .trash_item_id();
    create_child_node(&workspace, "CASENODE").unwrap();
    assert!(matches!(
        plan_restore_trash_item(&workspace, item_id, TrashRestoreMode::Original),
        Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            TrashRestoreBlockedReason::NameConflict | TrashRestoreBlockedReason::CaseFoldConflict
        ))
    ));
}

#[test]
fn planned_item_id_occupancy_never_clobbers_external_evidence() {
    let (_temporary, workspace) = setup();
    let seed = create_child_node(&workspace, "Seed").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, seed.id, TIME).unwrap()).unwrap();
    let target = create_child_node(&workspace, "Collision").unwrap();
    let preview = plan_trash_node_at(&workspace, target.id, TIME).unwrap();
    let item_id = preview.trash_item_changes()[0].manifest.trash_item_id();
    let occupied = workspace
        .join(".weftext-trash")
        .join(TRASH_ITEMS_DIRECTORY_NAME)
        .join(item_id.to_string());
    fs::create_dir(&occupied).unwrap();
    assert!(commit_workspace_transaction(&preview).is_err());
    assert!(occupied.is_dir());
    assert!(target.path.is_dir());
    assert_eq!(
        project_workspace_trash_state(&workspace).unwrap().state,
        WorkspaceTrashState::ReconciliationRequired
    );
}

#[test]
fn journal_digest_binds_scope_and_draft_authority() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "BoundScope").unwrap();
    let plan = plan_trash_node_at(&workspace, node.id, TIME).unwrap();
    let transaction = prepare_workspace_transaction_recovery_fixture(&plan).unwrap();
    let journal_path = transaction.join("journal.json");
    let mut journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    let byte_total = journal["scope_summary"]["byteTotal"].as_u64().unwrap();
    journal["scope_summary"]["byteTotal"] = serde_json::json!(byte_total + 1);
    fs::write(&journal_path, serde_json::to_vec_pretty(&journal).unwrap()).unwrap();
    assert!(matches!(
        recover_workspace_transactions(&workspace),
        Err(WorkspaceTransactionError::InvalidJournal(_))
    ));
    assert!(node.path.is_dir());
    assert!(transaction.is_dir());
}

#[test]
fn child_first_then_parent_delete_projects_and_commits_atomic_ancestor_restore() {
    let (_temporary, workspace) = setup();
    let parent = create_child_node(&workspace, "Parent").unwrap();
    let child = create_child_node(&parent.path, "Child").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, child.id, TIME).unwrap()).unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, parent.id, TIME).unwrap())
        .unwrap();

    let projected = project_workspace_trash_items(&workspace).unwrap();
    let child_item = projected
        .iter()
        .find(|item| item.manifest.node_id() == Some(child.id))
        .unwrap();
    let parent_item = projected
        .iter()
        .find(|item| item.manifest.node_id() == Some(parent.id))
        .unwrap();
    assert!(child_item.restore.with_ancestors_available);
    assert_eq!(
        child_item.restore.required_ancestor_item_ids,
        vec![parent_item.manifest.trash_item_id()]
    );
    let restore = plan_restore_trash_item(
        &workspace,
        child_item.manifest.trash_item_id(),
        TrashRestoreMode::WithAncestors,
    )
    .unwrap();
    assert_eq!(restore.trash_item_changes().len(), 2);
    let scope = restore.scope_summary.as_ref().unwrap();
    assert_eq!(scope.root_node.node_id, parent.id);
    assert_eq!(scope.descendant_node_count, 1);
    assert_eq!(scope.trash_item_count, 2);
    assert_eq!(scope.identity_policy, WorkspaceIdentityPolicy::Preserve);
    assert!(scope.affected_document_node_ids.contains(&parent.id));
    assert!(scope.affected_document_node_ids.contains(&child.id));
    commit_workspace_transaction(&restore).unwrap();
    assert!(workspace.join("Parent/Parent.adoc").is_file());
    assert!(workspace.join("Parent/Child/Child.adoc").is_file());
    assert!(scan_workspace(&workspace).trash_items.is_empty());
}

#[test]
fn permanent_delete_requires_permission_phrase_and_exact_revision() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "Disposable").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, node.id, TIME).unwrap()).unwrap();
    let item_id = scan_workspace(&workspace).trash_items[0]
        .manifest
        .trash_item_id();
    let preview = preview_permanent_delete_trash_items(&workspace, vec![item_id]).unwrap();
    assert!(matches!(
        confirm_permanent_delete_trash_items(
            preview.clone(),
            false,
            TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE
        ),
        Err(WorkspaceTransactionError::PermanentDeleteAuthorizationRequired)
    ));
    let confirmation = confirm_permanent_delete_trash_items(
        preview,
        true,
        TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE,
    )
    .unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(workspace.join("Notes.adoc"))
        .unwrap()
        .write_all(b"\nchanged after destructive preview\n")
        .unwrap();
    let plan = plan_permanently_delete_trash_items(&workspace, &confirmation).unwrap_err();
    assert!(matches!(
        plan,
        WorkspaceTransactionError::StaleRevision { .. }
    ));

    let preview = preview_permanent_delete_trash_items(&workspace, vec![item_id]).unwrap();
    let confirmation = confirm_permanent_delete_trash_items(
        preview,
        true,
        TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE,
    )
    .unwrap();
    commit_workspace_transaction(
        &plan_permanently_delete_trash_items(&workspace, &confirmation).unwrap(),
    )
    .unwrap();
    assert!(scan_workspace(&workspace).trash_items.is_empty());
}

#[test]
fn manifest_tamper_and_sync_conflict_copy_fail_closed() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "Tamper").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, node.id, TIME).unwrap()).unwrap();
    let inventory = scan_workspace(&workspace);
    let item = &inventory.trash_items[0];
    let trusted_revision = read_workspace_revision(&workspace).unwrap();
    let conflict_copy = item.item_path.with_file_name(format!(
        "{} (conflicted copy)",
        item.manifest.trash_item_id()
    ));
    fs::create_dir(&conflict_copy).unwrap();
    assert!(!scan_workspace(&workspace).is_valid());
    fs::remove_dir(&conflict_copy).unwrap();
    assert!(scan_workspace(&workspace).is_valid());
    let manifest = item.item_path.join(TRASH_ITEM_MANIFEST_FILE_NAME);
    let canonical_manifest = fs::read(&manifest).unwrap();
    let mut noncanonical_manifest = canonical_manifest.clone();
    noncanonical_manifest.extend_from_slice(b" ");
    fs::write(&manifest, noncanonical_manifest).unwrap();
    assert!(!scan_workspace(&workspace).is_valid());
    fs::write(&manifest, canonical_manifest).unwrap();
    assert!(scan_workspace(&workspace).is_valid());

    fs::OpenOptions::new()
        .append(true)
        .open(item.payload_path.join("Tamper.adoc"))
        .unwrap()
        .write_all(b"tampered")
        .unwrap();
    assert!(!scan_workspace(&workspace).is_valid());
    assert!(project_workspace_trash_items(&workspace).is_err());
    let degraded_revision = read_workspace_revision(&workspace).unwrap();
    assert_ne!(degraded_revision, trusted_revision);
    fs::remove_file(&manifest).unwrap();
    let partial_arrival_revision = read_workspace_revision(&workspace).unwrap();
    assert_ne!(partial_arrival_revision, degraded_revision);
    let state = project_workspace_trash_state(&workspace).unwrap();
    assert_eq!(state.state, WorkspaceTrashState::ReconciliationRequired);
    assert!(state.items.is_empty());
    let navigation = build_workspace_navigation(&scan_workspace(&workspace)).unwrap();
    assert!(
        navigation
            .hierarchy
            .iter()
            .all(|node| !node.locator.starts_with(".weftext-trash"))
    );
}

#[test]
fn active_and_trashed_permanent_node_ids_must_remain_globally_unique() {
    let (_temporary, workspace) = setup();
    let trashed = create_child_node(&workspace, "TrashedIdentity").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, trashed.id, TIME).unwrap())
        .unwrap();

    let active = create_child_node(&workspace, "ActiveIdentity").unwrap();
    let source = fs::read_to_string(&active.document_path).unwrap();
    fs::write(
        &active.document_path,
        source.replace(&active.id.to_string(), &trashed.id.to_string()),
    )
    .unwrap();

    let inventory = scan_workspace(&workspace);
    assert!(!inventory.is_valid());
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == weftext_core::InventoryIssueCode::DuplicateIdentity
            && issue.path.starts_with(workspace.join(".weftext-trash"))
    }));
    let state = project_workspace_trash_state(&workspace).unwrap();
    assert_eq!(state.state, WorkspaceTrashState::ReconciliationRequired);
    assert!(state.items.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn direct_layout_migrates_to_unknown_origin_items_without_dual_authority() {
    let (temporary, workspace) = setup();
    let node = create_child_node(&workspace, "Legacy").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, node.id, TIME).unwrap()).unwrap();
    let inventory = scan_workspace(&workspace);
    let item = &inventory.trash_items[0];
    let direct = workspace.join(".weftext-trash/Legacy");
    fs::rename(&item.payload_path, &direct).unwrap();
    fs::remove_dir_all(
        workspace
            .join(".weftext-trash")
            .join(TRASH_ITEMS_DIRECTORY_NAME),
    )
    .unwrap();
    assert!(scan_workspace(&workspace).legacy_trash_format);
    let state = project_workspace_trash_state(&workspace).unwrap();
    assert_eq!(state.state, WorkspaceTrashState::LegacyMigrationRequired);
    assert!(state.items.is_empty());
    assert!(matches!(
        plan_migrate_legacy_workspace_trash_at(&workspace, TIME),
        Err(WorkspaceTransactionError::LegacyTrashMigrationBackupRequired)
    ));
    let snapshots = temporary.path().join("snapshots");
    fs::create_dir(&snapshots).unwrap();
    let backup = prepare_legacy_trash_migration_backup(&workspace, &snapshots).unwrap();
    assert!(
        backup
            .snapshot_directory()
            .starts_with(fs::canonicalize(&snapshots).unwrap())
    );
    let reopened =
        load_legacy_trash_migration_backup(&workspace, backup.snapshot_directory()).unwrap();
    assert_eq!(reopened.authority(), backup.authority());
    let snapshot_document = backup
        .snapshot_directory()
        .join("content/.weftext-trash/Legacy/Legacy.adoc");
    let snapshot_bytes = fs::read(&snapshot_document).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&snapshot_document)
        .unwrap()
        .write_all(b"tampered")
        .unwrap();
    assert!(matches!(
        plan_migrate_legacy_workspace_trash_at_with_backup(&workspace, TIME, &backup),
        Err(WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(_))
    ));
    fs::write(&snapshot_document, snapshot_bytes).unwrap();
    let plan =
        plan_migrate_legacy_workspace_trash_at_with_backup(&workspace, TIME, &backup).unwrap();
    let request = plan.reviewed_trash_request().unwrap();
    assert!(matches!(
        replan_reviewed_trash_request(
            &workspace,
            request,
            TrashReviewedReplanAuthorization::Ordinary,
        ),
        Err(WorkspaceTransactionError::LegacyTrashMigrationBackupRequired)
    ));
    let replay = replan_reviewed_trash_request(
        &workspace,
        request,
        TrashReviewedReplanAuthorization::LegacyMigration {
            backup: backup.clone(),
        },
    )
    .unwrap();
    assert_eq!(replay.plan_id, plan.plan_id);
    assert_eq!(plan.trash_item_changes().len(), 1);
    assert_eq!(
        plan.trash_item_changes()[0].manifest.origin_status(),
        TrashOriginStatus::Unknown
    );
    commit_workspace_transaction(&replay).unwrap();
    let post_migration_snapshot =
        load_legacy_trash_migration_backup(&workspace, backup.snapshot_directory()).unwrap();
    assert_eq!(post_migration_snapshot.authority(), backup.authority());
    let inventory = scan_workspace(&workspace);
    assert!(inventory.is_valid());
    assert!(!direct.exists());
    assert_eq!(inventory.trash_items.len(), 1);
    let projected = project_workspace_trash_items(&workspace).unwrap();
    assert_eq!(
        projected[0].restore.blocked_reason,
        Some(TrashRestoreBlockedReason::OriginUnknown)
    );
    let item_id = projected[0].manifest.trash_item_id();
    assert!(matches!(
        plan_restore_trash_item(&workspace, item_id, TrashRestoreMode::Original),
        Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            TrashRestoreBlockedReason::OriginUnknown
        ))
    ));
    commit_workspace_transaction(
        &plan_restore_trash_item(
            &workspace,
            item_id,
            TrashRestoreMode::ExistingTarget {
                target_node_id: root_id(&workspace),
                name: "RecoveredLegacy".to_owned(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        scan_workspace(&workspace)
            .nodes
            .iter()
            .find(|node| node.name == "RecoveredLegacy")
            .and_then(|node| node.id),
        Some(node.id)
    );
}

#[test]
fn trash_bytes_change_workspace_revision() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "Revision").unwrap();
    let before = read_workspace_revision(&workspace).unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, node.id, TIME).unwrap()).unwrap();
    let after = read_workspace_revision(&workspace).unwrap();
    assert_ne!(before, after);
    let item = &scan_workspace(&workspace).trash_items[0];
    let manifest_path = item.item_path.join(TRASH_ITEM_MANIFEST_FILE_NAME);
    let manifest_bytes = fs::read(&manifest_path).unwrap();
    fs::write(&manifest_path, [manifest_bytes.as_slice(), b" "].concat()).unwrap();
    let manifest_changed = read_workspace_revision(&workspace).unwrap();
    assert_ne!(manifest_changed, after);
    fs::write(&manifest_path, manifest_bytes).unwrap();

    let document_path = item.payload_path.join("Revision.adoc");
    let document_bytes = fs::read(&document_path).unwrap();
    fs::write(
        &document_path,
        [document_bytes.as_slice(), b"changed"].concat(),
    )
    .unwrap();
    let payload_changed = read_workspace_revision(&workspace).unwrap();
    assert_ne!(payload_changed, after);
    fs::write(&document_path, document_bytes).unwrap();

    fs::create_dir(item.payload_path.join("empty-authority")).unwrap();
    let empty_directory_changed = read_workspace_revision(&workspace).unwrap();
    assert_ne!(empty_directory_changed, after);
    fs::remove_dir(item.payload_path.join("empty-authority")).unwrap();

    let renamed_item = item
        .item_path
        .with_file_name(weftext_core::TrashItemId::new_v4().to_string());
    fs::rename(&item.item_path, &renamed_item).unwrap();
    let item_path_changed = read_workspace_revision(&workspace).unwrap();
    assert_ne!(item_path_changed, after);
}

#[test]
fn content_rules_cannot_classify_any_deep_trash_item_payload_byte() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "ReservedStore").unwrap();
    fs::write(node.path.join("asset.bin"), b"portable payload").unwrap();
    commit_workspace_transaction(&plan_trash_node_at(&workspace, node.id, TIME).unwrap()).unwrap();
    let item_id = scan_workspace(&workspace).trash_items[0]
        .manifest
        .trash_item_id();
    fs::write(
        workspace.join(".weftext-rules"),
        format!(
            "weftext-content-rules-v1\nunmanaged .weftext-trash/{TRASH_ITEMS_DIRECTORY_NAME}/{item_id}/payload/ReservedStore/asset.bin\n"
        ),
    )
    .unwrap();

    let inventory = scan_workspace(&workspace);
    assert!(!inventory.is_valid());
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == weftext_core::InventoryIssueCode::TrashReconciliationRequired
            && issue.path.ends_with("ReservedStore/asset.bin")
    }));
    assert_eq!(
        project_workspace_trash_state(&workspace).unwrap().state,
        WorkspaceTrashState::ReconciliationRequired
    );
}

#[test]
fn reviewed_request_round_trips_and_replans_the_exact_generated_authority() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "Reviewed").unwrap();
    let preview = plan_trash_node_at(&workspace, node.id, TIME).unwrap();
    let request = preview.reviewed_trash_request().unwrap().clone();
    let bytes = request.to_canonical_json_bytes().unwrap();
    let decoded = TrashReviewedRequest::from_json_bytes(&bytes).unwrap();
    assert_eq!(decoded, request);

    let replay = replan_reviewed_trash_request(
        &workspace,
        &decoded,
        TrashReviewedReplanAuthorization::Ordinary,
    )
    .unwrap();
    assert_eq!(replay.plan_id, preview.plan_id);
    assert_eq!(replay.trash_item_changes(), preview.trash_item_changes());
    assert_eq!(
        replay.reviewed_trash_request(),
        preview.reviewed_trash_request()
    );
    commit_workspace_transaction(&replay).unwrap();
    assert_eq!(
        scan_workspace(&workspace).trash_items[0]
            .manifest
            .trash_item_id(),
        preview.trash_item_changes()[0].manifest.trash_item_id()
    );
}

#[test]
fn reviewed_request_rejects_tamper_stale_state_and_missing_purge_reauthorization() {
    let (_temporary, workspace) = setup();
    let node = create_child_node(&workspace, "Reviewed").unwrap();
    let preview = plan_trash_node_at(&workspace, node.id, TIME).unwrap();
    let request = preview.reviewed_trash_request().unwrap().clone();
    let mut value = serde_json::to_value(&request).unwrap();
    value["authorityDigest"] = serde_json::Value::String("0".repeat(64));
    assert!(TrashReviewedRequest::from_json_bytes(&serde_json::to_vec(&value).unwrap()).is_err());

    fs::OpenOptions::new()
        .append(true)
        .open(workspace.join("Notes.adoc"))
        .unwrap()
        .write_all(b"\nstale reviewed request\n")
        .unwrap();
    assert!(matches!(
        replan_reviewed_trash_request(
            &workspace,
            &request,
            TrashReviewedReplanAuthorization::Ordinary,
        ),
        Err(WorkspaceTransactionError::StaleRevision { .. })
    ));

    let fresh = plan_trash_node_at(&workspace, node.id, TIME).unwrap();
    commit_workspace_transaction(&fresh).unwrap();
    let item_id = scan_workspace(&workspace).trash_items[0]
        .manifest
        .trash_item_id();
    let deletion = confirm_permanent_delete_trash_items(
        preview_permanent_delete_trash_items(&workspace, vec![item_id]).unwrap(),
        true,
        TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE,
    )
    .unwrap();
    let purge = plan_permanently_delete_trash_items(&workspace, &deletion).unwrap();
    let purge_request = purge.reviewed_trash_request().unwrap();
    assert!(matches!(
        replan_reviewed_trash_request(
            &workspace,
            purge_request,
            TrashReviewedReplanAuthorization::Ordinary,
        ),
        Err(WorkspaceTransactionError::PermanentDeleteAuthorizationRequired)
    ));
    assert!(matches!(
        replan_reviewed_trash_request(
            &workspace,
            purge_request,
            TrashReviewedReplanAuthorization::PermanentDelete {
                higher_permission_granted: true,
                exact_phrase: "wrong".to_owned(),
            },
        ),
        Err(WorkspaceTransactionError::PermanentDeleteConfirmationMismatch)
    ));
    let replay = replan_reviewed_trash_request(
        &workspace,
        purge_request,
        TrashReviewedReplanAuthorization::PermanentDelete {
            higher_permission_granted: true,
            exact_phrase: TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE.to_owned(),
        },
    )
    .unwrap();
    assert_eq!(replay.plan_id, purge.plan_id);
}

#[test]
fn manifest_contract_fixtures_are_closed_and_canonicalizable() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trash-item-v1");
    for name in [
        "valid-node-known.json",
        "valid-node-unknown.json",
        "valid-resource-known.json",
    ] {
        let bytes = fs::read(fixtures.join(name)).unwrap();
        let manifest = weftext_core::TrashItemManifest::from_json_bytes(&bytes).unwrap();
        let canonical = manifest.to_canonical_json_bytes().unwrap();
        assert_eq!(
            weftext_core::TrashItemManifest::from_json_bytes(&canonical).unwrap(),
            manifest
        );
    }
    for name in [
        "invalid-cross-kind-field.json",
        "invalid-known-null-origin.json",
        "invalid-node-self-origin.json",
        "invalid-unknown-field.json",
    ] {
        let bytes = fs::read(fixtures.join(name)).unwrap();
        assert!(weftext_core::TrashItemManifest::from_json_bytes(&bytes).is_err());
    }
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/trash-item-v1.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(schema["$defs"]["nodeKnown"]["additionalProperties"], false);
    assert_eq!(
        schema["$defs"]["resourceUnknown"]["additionalProperties"],
        false
    );
}

use std::io::Write as _;
