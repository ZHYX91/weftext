use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;
use weftext_backup::{
    BackupEntryType, BackupError, SNAPSHOT_COMPLETION_FILE, SNAPSHOT_MANIFEST_FILE,
    SNAPSHOT_PROTECTION_FILE, ScopedRestoreCommitState, SnapshotRetentionPolicy,
    commit_alternate_restore, commit_full_workspace_backup, commit_restore_drill,
    commit_scoped_restore, commit_snapshot_retention, plan_alternate_restore,
    plan_full_workspace_backup, plan_restore_drill, plan_single_node_restore,
    plan_snapshot_retention, plan_subtree_restore, protect_full_workspace_snapshot,
    read_restore_drill_result, read_snapshot_retention_receipt, recover_snapshot_retention,
    verify_full_workspace_snapshot,
};
use weftext_core::{
    AnnotationStore, TRASH_DIRECTORY_NAME, TRASH_ITEM_MANIFEST_FILE_NAME,
    TRASH_ITEM_PAYLOAD_DIRECTORY_NAME, TRASH_ITEMS_DIRECTORY_NAME,
    WORKSPACE_TRANSACTION_LEASE_FILE_NAME, commit_workspace_transaction, create_child_node,
    create_workspace, plan_trash_node_at, read_workspace_revision, scan_workspace,
};

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    workspace: PathBuf,
    backup_parent: PathBuf,
    restore_parent: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary fixture root");
        let root = fs::canonicalize(temporary.path()).expect("canonical temporary fixture root");
        let workspace = root.join("source").join("资料库");
        fs::create_dir(root.join("source")).expect("source parent");
        create_workspace(&workspace).expect("canonical workspace");
        let child = create_child_node(&workspace, "Managed").expect("managed child");
        let trashable = create_child_node(&workspace, "Trashable").expect("trashable child");
        fs::write(
            trashable.path.join("asset.bin"),
            b"Core Trash item payload bytes\0\xff",
        )
        .expect("trash payload resource");
        fs::write(
            trashable
                .path
                .join(".__weftext-transaction-workspace-user-resource.bin"),
            b"nested transaction-like names remain ordinary Trash payload bytes",
        )
        .expect("transaction-like Trash payload resource");
        let trash_plan = plan_trash_node_at(&workspace, trashable.id, "2026-08-24T12:00:00+08:00")
            .expect("Core-authored Trash item plan");
        commit_workspace_transaction(&trash_plan).expect("Core-authored Trash item commit");
        fs::write(workspace.join("resource.bin"), [0, 1, 2, 255]).expect("resource");
        fs::write(workspace.join("loose.md"), b"visible unmanaged\r\n").expect("unmanaged");
        fs::create_dir(workspace.join("unmanaged")).expect("unmanaged directory");
        fs::write(
            workspace.join("unmanaged/nested.dat"),
            "精确 unmanaged 😀".as_bytes(),
        )
        .expect("unmanaged bytes");
        fs::write(
            workspace.join("unmanaged/.__weftext-transaction-workspace-user-note"),
            b"only root-level transaction state is operational",
        )
        .expect("nested transaction-like unmanaged bytes");
        fs::create_dir(workspace.join("ignored")).expect("ignored directory");
        fs::create_dir(workspace.join("ignored/empty")).expect("ignored empty directory");
        fs::write(
            workspace.join("ignored/secret.bin"),
            b"ignored bytes are backup bytes\0\xff",
        )
        .expect("ignored bytes");
        fs::write(
            child.path.join("weftext.annotations.json"),
            b"{\"version\":3,\"annotations\":[]}\n",
        )
        .expect("annotation sidecar");
        fs::create_dir(workspace.join(".git")).expect("physical dot-git directory");
        fs::write(workspace.join(".git/config"), b"physical metadata\n").expect("dot-git bytes");
        fs::write(
            workspace.join(".weftext-rules"),
            concat!(
                "weftext-content-rules-v1\n",
                "unmanaged loose.md\n",
                "unmanaged unmanaged/\n",
                "ignore ignored/\n"
            ),
        )
        .expect("content rules");
        assert!(scan_workspace(&workspace).is_valid());

        let backup_parent = root.join("backups");
        let restore_parent = root.join("alternate");
        fs::create_dir(&backup_parent).expect("backup parent");
        fs::create_dir(&restore_parent).expect("restore parent");
        Self {
            _temporary: temporary,
            root,
            workspace,
            backup_parent,
            restore_parent,
        }
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one acceptance test proves every physical class, including Core Trash item bytes, across preview, verify, and clean restore"
)]
fn full_physical_preview_commit_verify_and_clean_restore_are_exact() {
    let fixture = Fixture::new();
    let source_revision = read_workspace_revision(&fixture.workspace).expect("source revision");
    let source_inventory = scan_workspace(&fixture.workspace);
    let source_id = source_inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .expect("root node")
        .id
        .expect("root ID");
    let trash_item_id = source_inventory.trash_items[0].manifest.trash_item_id();
    let trash_item_root =
        format!("{TRASH_DIRECTORY_NAME}/{TRASH_ITEMS_DIRECTORY_NAME}/{trash_item_id}");
    assert_eq!(
        fs::read_dir(&fixture.backup_parent)
            .expect("backup parent")
            .count(),
        0
    );

    let plan = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent)
        .expect("coordinated backup preview");
    assert_eq!(
        read_workspace_revision(&fixture.workspace).unwrap(),
        source_revision
    );
    assert!(!plan.snapshot_directory.exists());
    assert_eq!(
        fs::read_dir(&fixture.backup_parent)
            .expect("still empty after preview")
            .count(),
        0
    );
    let locators = plan
        .entries
        .iter()
        .map(|entry| entry.locator.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        ".weftext-format",
        ".weftext-rules",
        ".git/config",
        "Managed/Managed.adoc",
        "Managed/weftext.annotations.json",
        "ignored/empty",
        "ignored/secret.bin",
        "loose.md",
        "resource.bin",
        "unmanaged/.__weftext-transaction-workspace-user-note",
        "unmanaged/nested.dat",
        "资料库.adoc",
    ] {
        assert!(
            locators.contains(required),
            "missing {required}: {locators:?}"
        );
    }
    for required in [
        trash_item_root.clone(),
        format!("{trash_item_root}/{TRASH_ITEM_MANIFEST_FILE_NAME}"),
        format!("{trash_item_root}/{TRASH_ITEM_PAYLOAD_DIRECTORY_NAME}"),
        format!("{trash_item_root}/{TRASH_ITEM_PAYLOAD_DIRECTORY_NAME}/Trashable"),
        format!(
            "{trash_item_root}/{TRASH_ITEM_PAYLOAD_DIRECTORY_NAME}/Trashable/.__weftext-transaction-workspace-user-resource.bin"
        ),
        format!("{trash_item_root}/{TRASH_ITEM_PAYLOAD_DIRECTORY_NAME}/Trashable/Trashable.adoc"),
        format!("{trash_item_root}/{TRASH_ITEM_PAYLOAD_DIRECTORY_NAME}/Trashable/asset.bin"),
    ] {
        assert!(
            locators.contains(required.as_str()),
            "missing Core Trash item byte locator {required}: {locators:?}"
        );
    }
    assert!(plan.entries.iter().all(|entry| {
        entry.sha256.len() == 64 && (entry.entry_type == BackupEntryType::File || entry.length == 0)
    }));

    let receipt = commit_full_workspace_backup(&plan).expect("marker-last snapshot commit");
    assert!(receipt.verified);
    assert!(
        plan.snapshot_directory
            .join(SNAPSHOT_MANIFEST_FILE)
            .is_file()
    );
    assert!(
        plan.snapshot_directory
            .join(SNAPSHOT_COMPLETION_FILE)
            .is_file()
    );
    let verification =
        verify_full_workspace_snapshot(&plan.snapshot_directory).expect("snapshot verification");
    assert!(verification.complete);
    assert_eq!(verification.workspace_root_id, source_id);
    assert_eq!(verification.workspace_revision, source_revision);
    assert_eq!(verification.entry_count, plan.entry_count);
    let snapshot_workspace = plan.snapshot_directory.join("content/资料库");
    assert!(scan_workspace(&snapshot_workspace).is_valid());
    assert_eq!(scan_workspace(&snapshot_workspace).trash_items.len(), 1);
    assert_eq!(
        read_workspace_revision(&snapshot_workspace).unwrap(),
        source_revision
    );

    let destination = fixture.restore_parent.join("资料库");
    let restore = plan_alternate_restore(&plan.snapshot_directory, &destination)
        .expect("read-only restore preview");
    assert!(!destination.exists());
    let restored = commit_alternate_restore(&restore).expect("clean alternate restore");
    assert!(restored.bytewise_verified);
    assert_eq!(restored.workspace_root_id, source_id);
    assert_eq!(
        read_workspace_revision(&destination).unwrap(),
        source_revision
    );
    assert!(scan_workspace(&destination).is_valid());
    assert_eq!(scan_workspace(&destination).trash_items.len(), 1);
    for entry in &plan.entries {
        let source = join_locator(&fixture.workspace, &entry.locator);
        let target = join_locator(&destination, &entry.locator);
        match entry.entry_type {
            BackupEntryType::Directory => assert!(target.is_dir()),
            BackupEntryType::File => {
                assert_eq!(fs::read(target).unwrap(), fs::read(source).unwrap());
            }
        }
    }
    assert!(matches!(
        plan_alternate_restore(&plan.snapshot_directory, &destination),
        Err(BackupError::RestoreTargetExists(_))
    ));
}

#[test]
fn backup_and_alternate_restore_destinations_cannot_be_nested_in_a_workspace() {
    let fixture = Fixture::new();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();

    let container = fixture.root.join("Container");
    create_workspace(&container).unwrap();

    assert!(matches!(
        plan_full_workspace_backup(&fixture.workspace, &container),
        Err(BackupError::Path(message)) if message.contains("outside every Weftext workspace root")
    ));
    let destination = container.join("资料库");
    assert!(matches!(
        plan_alternate_restore(&backup.snapshot_directory, &destination),
        Err(BackupError::Path(message)) if message.contains("outside every Weftext workspace root")
    ));
    assert!(!destination.exists());
    assert!(scan_workspace(&container).is_valid());
}

#[test]
fn ignored_byte_changes_stale_the_physical_preview_even_when_core_revision_is_unchanged() {
    let fixture = Fixture::new();
    let plan = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    let revision = read_workspace_revision(&fixture.workspace).unwrap();
    fs::write(
        fixture.workspace.join("ignored/secret.bin"),
        b"concurrent ignored change",
    )
    .expect("ignored concurrent edit");
    assert_eq!(
        read_workspace_revision(&fixture.workspace).unwrap(),
        revision
    );
    assert!(matches!(
        commit_full_workspace_backup(&plan),
        Err(BackupError::StalePreview)
    ));
    assert!(!plan.snapshot_directory.exists());
}

#[test]
fn unfinished_transactions_and_create_new_collisions_fail_closed() {
    let fixture = Fixture::new();
    let transaction = fixture
        .workspace
        .join(".__weftext-transaction-workspace-interrupted");
    fs::create_dir(&transaction).expect("unfinished transaction");
    assert!(matches!(
        plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent),
        Err(BackupError::UnfinishedTransaction(_))
    ));
    fs::remove_dir(&transaction).expect("remove test transaction");

    let plan = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    fs::create_dir(&plan.snapshot_directory).expect("snapshot collision");
    assert!(matches!(
        commit_full_workspace_backup(&plan),
        Err(BackupError::SnapshotExists(_))
    ));
}

#[cfg(any(unix, windows))]
#[test]
fn persistent_core_lease_is_not_payload_and_an_active_owner_blocks_preview() {
    let fixture = Fixture::new();
    let lease_path = fixture
        .workspace
        .join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
    assert_eq!(fs::metadata(&lease_path).unwrap().len(), 0);

    let preview = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent)
        .expect("unlocked persistent Core lease is accepted");
    assert!(
        preview
            .entries
            .iter()
            .all(|entry| entry.locator != WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
        "the operational lease is not portable workspace backup payload"
    );

    let _active_owner = hold_workspace_transaction_lease(&lease_path);
    assert!(matches!(
        plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent),
        Err(BackupError::CoreTransaction(_))
    ));
}

#[test]
fn pristine_preview_establishes_only_the_core_lease_and_commit_reuses_it() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = fs::canonicalize(temporary.path()).expect("canonical temporary root");
    let workspace = root.join("Pristine");
    let backup_parent = root.join("backups");
    create_workspace(&workspace).expect("pristine workspace");
    fs::create_dir(&backup_parent).expect("backup parent");
    let lease_path = workspace.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
    let revision = read_workspace_revision(&workspace).expect("initial revision");
    assert!(!lease_path.exists());

    let preview =
        plan_full_workspace_backup(&workspace, &backup_parent).expect("coordinated preview");
    assert_eq!(fs::metadata(&lease_path).unwrap().len(), 0);
    assert_eq!(
        read_workspace_revision(&workspace).expect("revision after preview"),
        revision,
        "the durable coordination anchor is not workspace content or a revision change"
    );
    commit_full_workspace_backup(&preview).expect("coordinated pristine commit");
    assert_eq!(fs::metadata(&lease_path).unwrap().len(), 0);
    assert!(
        preview
            .entries
            .iter()
            .all(|entry| entry.locator != WORKSPACE_TRANSACTION_LEASE_FILE_NAME)
    );
    assert!(verify_full_workspace_snapshot(&preview.snapshot_directory).is_ok());
}

#[test]
fn incomplete_unknown_field_and_tampered_snapshots_are_rejected_read_only() {
    let fixture = Fixture::new();
    let incomplete =
        plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&incomplete).unwrap();
    fs::remove_file(incomplete.snapshot_directory.join(SNAPSHOT_COMPLETION_FILE))
        .expect("remove completion marker");
    assert!(matches!(
        verify_full_workspace_snapshot(&incomplete.snapshot_directory),
        Err(BackupError::IncompleteSnapshot(_))
    ));

    let unknown = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&unknown).unwrap();
    let manifest_path = unknown.snapshot_directory.join(SNAPSHOT_MANIFEST_FILE);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest
        .as_object_mut()
        .unwrap()
        .insert("futureAuthority".to_owned(), serde_json::json!(true));
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = verify_full_workspace_snapshot(&unknown.snapshot_directory).unwrap_err();
    assert!(error.to_string().contains("unknown field"));

    let tampered = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&tampered).unwrap();
    fs::write(
        tampered
            .snapshot_directory
            .join("content/资料库/ignored/secret.bin"),
        b"tampered",
    )
    .unwrap();
    assert!(matches!(
        plan_alternate_restore(
            &tampered.snapshot_directory,
            fixture.restore_parent.join("资料库")
        ),
        Err(BackupError::Verification(_))
    ));
    assert!(!fixture.restore_parent.join("资料库").exists());
}

#[test]
fn protected_restore_points_are_permanent_idempotent_and_tamper_evident() {
    let fixture = Fixture::new();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();

    let protected = protect_full_workspace_snapshot(&backup.snapshot_directory, "季度恢复点")
        .expect("create-new protection record");
    assert_eq!(protected.snapshot_id, backup.snapshot_id);
    assert_eq!(protected.label, "季度恢复点");
    assert!(protected.protected_at_unix_ms > 0);
    assert_eq!(
        protect_full_workspace_snapshot(&backup.snapshot_directory, "季度恢复点").unwrap(),
        protected,
        "the exact same protection request is idempotent"
    );
    assert!(
        protect_full_workspace_snapshot(&backup.snapshot_directory, "different label").is_err(),
        "v1 protection cannot be replaced or removed"
    );
    let verification = verify_full_workspace_snapshot(&backup.snapshot_directory).unwrap();
    assert_eq!(verification.protection, Some(protected));

    let protection_path = backup.snapshot_directory.join(SNAPSHOT_PROTECTION_FILE);
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&protection_path).unwrap()).unwrap();
    record
        .as_object_mut()
        .unwrap()
        .insert("futureAuthority".to_owned(), serde_json::json!(true));
    fs::write(
        &protection_path,
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    let error = verify_full_workspace_snapshot(&backup.snapshot_directory).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn reviewed_retention_keeps_latest_and_every_protected_restore_point() {
    let fixture = Fixture::new();
    let oldest = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&oldest).unwrap();
    protect_full_workspace_snapshot(&oldest.snapshot_directory, "permanent baseline").unwrap();
    std::thread::sleep(Duration::from_millis(5));
    let middle = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&middle).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    let newest = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&newest).unwrap();

    let plan = plan_snapshot_retention(
        &fixture.backup_parent,
        SnapshotRetentionPolicy {
            keep_latest_unprotected: 1,
        },
    )
    .expect("read-only retention preview");
    assert!(!plan.receipt_file.exists());
    assert!(oldest.snapshot_directory.exists());
    assert!(middle.snapshot_directory.exists());
    assert!(newest.snapshot_directory.exists());
    assert_eq!(
        plan.retained
            .iter()
            .map(|snapshot| snapshot.snapshot_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([oldest.snapshot_id, newest.snapshot_id])
    );
    assert_eq!(
        plan.pruned
            .iter()
            .map(|snapshot| snapshot.snapshot_id)
            .collect::<Vec<_>>(),
        vec![middle.snapshot_id]
    );

    let receipt = commit_snapshot_retention(&plan).expect("recoverable retention commit");
    assert_eq!(receipt.pruned_snapshot_ids, vec![middle.snapshot_id]);
    assert!(!middle.snapshot_directory.exists());
    assert!(verify_full_workspace_snapshot(&oldest.snapshot_directory).is_ok());
    assert!(verify_full_workspace_snapshot(&newest.snapshot_directory).is_ok());
    assert_eq!(
        read_snapshot_retention_receipt(&plan.receipt_file).unwrap(),
        receipt
    );
    assert!(
        commit_snapshot_retention(&plan).is_err(),
        "plans are single-use"
    );
    let recovery = recover_snapshot_retention(&fixture.backup_parent).unwrap();
    assert!(recovery.rolled_back_operation_ids.is_empty());
    assert!(recovery.finalized_operation_ids.is_empty());
}

#[test]
fn protection_added_after_retention_preview_stales_the_destructive_plan() {
    let fixture = Fixture::new();
    let older = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&older).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    let newer = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&newer).unwrap();
    let plan = plan_snapshot_retention(
        &fixture.backup_parent,
        SnapshotRetentionPolicy {
            keep_latest_unprotected: 1,
        },
    )
    .unwrap();
    assert_eq!(plan.pruned[0].snapshot_id, older.snapshot_id);

    protect_full_workspace_snapshot(&older.snapshot_directory, "late protected point").unwrap();
    assert!(commit_snapshot_retention(&plan).is_err());
    assert!(verify_full_workspace_snapshot(&older.snapshot_directory).is_ok());
    assert!(verify_full_workspace_snapshot(&newer.snapshot_directory).is_ok());
    let refreshed = plan_snapshot_retention(
        &fixture.backup_parent,
        SnapshotRetentionPolicy {
            keep_latest_unprotected: 1,
        },
    )
    .unwrap();
    assert!(refreshed.pruned.is_empty());
}

#[test]
fn reviewed_restore_drill_reopens_exact_bytes_and_records_success_last() {
    let fixture = Fixture::new();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();
    let results_parent = fixture.root.join("drill-results");
    fs::create_dir(&results_parent).unwrap();

    let drill = plan_restore_drill(
        &backup.snapshot_directory,
        &fixture.restore_parent,
        &results_parent,
    )
    .expect("read-only drill preview");
    assert!(!drill.drill_directory.exists());
    assert!(!drill.result_file.exists());

    let receipt = commit_restore_drill(&drill).expect("verified restore drill");
    assert!(receipt.opened_clean);
    assert!(receipt.bytewise_verified);
    assert!(receipt.destination_root.join("资料库.adoc").is_file());
    assert!(scan_workspace(&receipt.destination_root).is_valid());
    let result = read_restore_drill_result(&receipt.result_file).expect("drill result record");
    assert_eq!(result.drill_id, drill.drill_id);
    assert_eq!(result.snapshot_id, backup.snapshot_id);
    assert!(result.completed_at_unix_ms > 0);
    assert!(result.opened_clean && result.bytewise_verified);
    for entry in &backup.entries {
        if entry.entry_type == BackupEntryType::File {
            assert_eq!(
                fs::read(join_locator(&fixture.workspace, &entry.locator)).unwrap(),
                fs::read(join_locator(&receipt.destination_root, &entry.locator)).unwrap(),
            );
        }
    }
    assert!(
        commit_restore_drill(&drill).is_err(),
        "drill plans are single-use"
    );
}

#[test]
fn ready_single_node_restore_commits_through_core_and_is_single_use() {
    let fixture = Fixture::new();
    let leaf = create_child_node(fixture.workspace.join("Managed"), "Leaf").unwrap();
    fs::write(leaf.path.join("asset.bin"), b"leaf-resource\0\xff").unwrap();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();

    let target = fixture.root.join("Target");
    let target_root = create_workspace(&target).unwrap();
    let target_revision = read_workspace_revision(&target).unwrap();
    let plan = plan_single_node_restore(
        &backup.snapshot_directory,
        &target,
        leaf.id,
        target_root.id,
        "Recovered",
    )
    .expect("read-only single-node restore preview");
    assert_eq!(plan.commit_state, ScopedRestoreCommitState::Ready);
    assert!(plan.blockers.is_empty());
    assert_eq!(read_workspace_revision(&target).unwrap(), target_revision);
    assert!(!target.join("Recovered").exists());
    assert_eq!(plan.nodes.len(), 1);
    assert!(plan.entries.iter().any(|entry| {
        entry.source_locator.ends_with("Leaf/asset.bin")
            && entry.destination_locator == "Recovered/asset.bin"
    }));

    let receipt = commit_scoped_restore(&plan).expect("Core recoverable restore transaction");
    assert!(receipt.exact_bytes_verified);
    assert_eq!(receipt.restored_node_ids, vec![leaf.id]);
    assert_eq!(
        fs::read(target.join("Recovered/asset.bin")).unwrap(),
        b"leaf-resource\0\xff"
    );
    assert_eq!(
        scan_workspace(&target)
            .nodes
            .iter()
            .find(|node| node.path == target.join("Recovered"))
            .and_then(|node| node.id),
        Some(leaf.id)
    );
    assert!(
        commit_scoped_restore(&plan).is_err(),
        "a reviewed scoped restore cannot overwrite or replay its destination"
    );
}

#[test]
fn scoped_restore_rejects_stale_target_and_tampered_snapshot_without_writes() {
    let fixture = Fixture::new();
    let leaf = create_child_node(fixture.workspace.join("Managed"), "Leaf").unwrap();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();
    let target = fixture.root.join("Target");
    let target_root = create_workspace(&target).unwrap();

    let stale = plan_single_node_restore(
        &backup.snapshot_directory,
        &target,
        leaf.id,
        target_root.id,
        "Stale",
    )
    .unwrap();
    create_child_node(&target, "Concurrent").unwrap();
    assert!(matches!(
        commit_scoped_restore(&stale),
        Err(BackupError::StalePreview)
    ));
    assert!(!target.join("Stale").exists());

    let tampered = plan_single_node_restore(
        &backup.snapshot_directory,
        &target,
        leaf.id,
        target_root.id,
        "Tampered",
    )
    .unwrap();
    fs::write(
        backup
            .snapshot_directory
            .join("content/资料库/Managed/Leaf/Leaf.adoc"),
        b"tampered",
    )
    .unwrap();
    assert!(commit_scoped_restore(&tampered).is_err());
    assert!(!target.join("Tampered").exists());
}

#[test]
fn subtree_with_annotation_sidecar_restores_every_identity_and_exact_byte_atomically() {
    let fixture = Fixture::new();
    let inventory = scan_workspace(&fixture.workspace);
    let managed = inventory
        .nodes
        .iter()
        .find(|node| node.name == "Managed")
        .unwrap();
    let sidecar = AnnotationStore::empty(managed.id.unwrap())
        .to_pretty_json()
        .unwrap();
    fs::write(managed.path.join("weftext.annotations.json"), &sidecar).unwrap();
    let nested = create_child_node(&managed.path, "Nested").unwrap();
    let managed_id = managed.id.unwrap();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();
    let target = fixture.root.join("Target");
    let target_root = create_workspace(&target).unwrap();
    let plan = plan_subtree_restore(
        &backup.snapshot_directory,
        &target,
        managed_id,
        target_root.id,
        "RecoveredTree",
    )
    .expect("complete executable subtree inventory");
    assert_eq!(plan.commit_state, ScopedRestoreCommitState::Ready);
    assert!(plan.blockers.is_empty());
    assert_eq!(plan.nodes.len(), 2);
    assert!(plan.entries.iter().any(|entry| {
        entry.source_locator == "Managed/weftext.annotations.json"
            && entry.destination_locator == "RecoveredTree/weftext.annotations.json"
    }));
    assert!(!target.join("RecoveredTree").exists());
    let receipt = commit_scoped_restore(&plan).expect("atomic Core subtree restore");
    assert!(receipt.exact_bytes_verified);
    assert_eq!(
        receipt
            .restored_node_ids
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([managed_id, nested.id])
    );
    assert_eq!(
        fs::read_to_string(target.join("RecoveredTree/weftext.annotations.json")).unwrap(),
        sidecar
    );
    let restored = scan_workspace(&target);
    assert!(restored.is_valid());
    assert!(
        restored.nodes.iter().any(|node| {
            node.id == Some(managed_id) && node.path == target.join("RecoveredTree")
        })
    );
    assert!(restored.nodes.iter().any(|node| {
        node.id == Some(nested.id) && node.path == target.join("RecoveredTree/Nested")
    }));
}

#[test]
fn scoped_restore_refuses_ignored_or_unowned_physical_boundaries() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.workspace.join("Managed/weftext.annotations.json")).unwrap();
    fs::create_dir(fixture.workspace.join("Managed/private")).unwrap();
    fs::write(
        fixture.workspace.join("Managed/private/secret.bin"),
        b"ignored but still backed up",
    )
    .unwrap();
    let mut rules = fs::read_to_string(fixture.workspace.join(".weftext-rules")).unwrap();
    rules.push_str("ignore Managed/private/\n");
    fs::write(fixture.workspace.join(".weftext-rules"), rules).unwrap();
    let inventory = scan_workspace(&fixture.workspace);
    assert!(inventory.is_valid());
    let managed_id = inventory
        .nodes
        .iter()
        .find(|node| node.name == "Managed")
        .and_then(|node| node.id)
        .unwrap();
    let backup = plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent).unwrap();
    commit_full_workspace_backup(&backup).unwrap();
    let target = fixture.root.join("Target");
    let target_root = create_workspace(&target).unwrap();

    assert!(matches!(
        plan_single_node_restore(
            &backup.snapshot_directory,
            &target,
            managed_id,
            target_root.id,
            "Rejected"
        ),
        Err(BackupError::ScopedRestoreBoundary(locator)) if locator == "Managed/private"
    ));
    assert!(!target.join("Rejected").exists());
}

#[cfg(unix)]
#[test]
fn physical_dot_git_symlink_is_rejected_even_though_core_discovery_ignores_dot_git() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.workspace.join(".git")).unwrap();
    symlink(&fixture.backup_parent, fixture.workspace.join(".git")).unwrap();
    assert!(scan_workspace(&fixture.workspace).is_valid());
    assert!(matches!(
        plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent),
        Err(BackupError::LinkedPath(_))
    ));
}

#[cfg(windows)]
#[test]
fn physical_dot_git_junction_is_rejected_even_though_core_discovery_ignores_dot_git() {
    use std::process::Command;

    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.workspace.join(".git")).unwrap();
    let junction = fixture.workspace.join(".git");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&fixture.backup_parent)
        .status()
        .expect("create junction");
    assert!(status.success());
    assert!(scan_workspace(&fixture.workspace).is_valid());
    assert!(matches!(
        plan_full_workspace_backup(&fixture.workspace, &fixture.backup_parent),
        Err(BackupError::LinkedPath(_))
    ));
}

fn join_locator(root: &Path, locator: &str) -> PathBuf {
    locator
        .split('/')
        .fold(root.to_path_buf(), |mut path, part| {
            path.push(part);
            path
        })
}

#[cfg(unix)]
fn hold_workspace_transaction_lease(path: &Path) -> File {
    use rustix::fs::{FlockOperation, flock};

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open Core lease");
    flock(&file, FlockOperation::NonBlockingLockExclusive).expect("hold Core lease");
    file
}

#[cfg(windows)]
fn hold_workspace_transaction_lease(path: &Path) -> File {
    use std::os::windows::fs::OpenOptionsExt as _;

    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_DELETE)
        .open(path)
        .expect("hold Core lease")
}
