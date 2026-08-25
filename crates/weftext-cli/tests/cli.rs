use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

struct CanonicalTempDir {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

impl CanonicalTempDir {
    fn path(&self) -> &Path {
        &self.root
    }
}

fn tempdir() -> io::Result<CanonicalTempDir> {
    let temporary = tempfile::tempdir()?;
    let root = std::fs::canonicalize(temporary.path())?;
    Ok(CanonicalTempDir {
        _temporary: temporary,
        root,
    })
}

#[test]
fn cli_creates_and_inventories_a_workspace() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Notes");
    let binary = env!("CARGO_BIN_EXE_weftext");

    let create = Command::new(binary)
        .args(["workspace", "create"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        create.status.success(),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );

    let inventory = Command::new(binary)
        .args(["workspace", "inventory"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(inventory.status.success());
    let value: serde_json::Value = serde_json::from_slice(&inventory.stdout).unwrap();
    assert_eq!(value["workspace"]["valid"], true);
    assert_eq!(value["workspace"]["syncDisposition"], "ready");
    assert_eq!(
        value["workspace"]["trashItems"].as_array().map(Vec::len),
        Some(0)
    );
    assert_eq!(value["workspace"]["legacyTrashMigrationRequired"], false);
    assert_eq!(
        std::fs::read(root.join(".weftext-format")).unwrap(),
        b"weftext.asciidoc.v1\n"
    );
    assert!(root.join("Notes.adoc").is_file());
    assert!(!root.join("Notes.md").exists());

    std::fs::write(root.join(".weftext-format"), b"weftext.asciidoc.v1\r\n").unwrap();
    let malformed_marker = Command::new(binary)
        .args(["document", "read"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(!malformed_marker.status.success());
    let malformed_marker: serde_json::Value =
        serde_json::from_slice(&malformed_marker.stderr).unwrap();
    assert!(!malformed_marker["error"].as_str().unwrap().is_empty());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one CLI lifecycle case proves every Trash review and commit command uses the same closed Core authority"
)]
fn cli_trash_uses_core_reviewed_requests_and_item_inventory() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Trash CLI");
    weftext_core::create_workspace(&root).expect("workspace");
    let child = weftext_core::create_child_node(&root, "Child").expect("child");
    let binary = env!("CARGO_BIN_EXE_weftext");

    let preview = Command::new(binary)
        .args(["trash", "node-preview"])
        .arg(&root)
        .arg(child.id.to_string())
        .output()
        .expect("node Trash preview");
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert!(child.path.is_dir(), "preview must remain read-only");
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(
        preview["plan"]["trashItemChanges"][0]["disposition"],
        "stored"
    );
    assert_eq!(preview["plan"]["pathChanges"], serde_json::json!([]));
    assert_eq!(preview["plan"]["documentChanges"], serde_json::json!([]));
    assert_eq!(preview["plan"]["generatedNodeIds"], serde_json::json!([]));
    assert_eq!(
        preview["reviewedRequest"]["schema"],
        "weftext.trash-reviewed-request/v1"
    );
    let reviewed = temporary.path().join("node-trash-review.json");
    std::fs::write(
        &reviewed,
        serde_json::to_vec(&preview["reviewedRequest"]).unwrap(),
    )
    .unwrap();

    weftext_core::create_child_node(&root, "Concurrent").expect("concurrent revision change");
    let stale = Command::new(binary)
        .args(["trash", "commit"])
        .arg(&root)
        .arg(format!("@{}", reviewed.display()))
        .output()
        .expect("stale reviewed request");
    assert!(!stale.status.success());
    assert!(
        child.path.is_dir(),
        "stale request must not mutate the subtree"
    );
    let refreshed = Command::new(binary)
        .args(["trash", "node-preview"])
        .arg(&root)
        .arg(child.id.to_string())
        .output()
        .expect("refreshed node Trash preview");
    assert!(refreshed.status.success());
    let refreshed: serde_json::Value = serde_json::from_slice(&refreshed.stdout).unwrap();
    std::fs::write(
        &reviewed,
        serde_json::to_vec(&refreshed["reviewedRequest"]).unwrap(),
    )
    .unwrap();

    let commit = Command::new(binary)
        .args(["trash", "commit"])
        .arg(&root)
        .arg(format!("@{}", reviewed.display()))
        .output()
        .expect("reviewed node Trash commit");
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(!child.path.exists());

    let inventory = Command::new(binary)
        .args(["trash", "inventory"])
        .arg(&root)
        .output()
        .expect("Trash inventory");
    assert!(inventory.status.success());
    let inventory: serde_json::Value = serde_json::from_slice(&inventory.stdout).unwrap();
    let node_item = &inventory["trash"]["items"][0];
    assert_eq!(node_item["manifest"]["nodeId"], child.id.to_string());
    assert_eq!(node_item["restore"]["originResolution"], "active");
    assert!(node_item.get("itemPath").is_none());
    let item_id = node_item["manifest"]["trashItemId"]
        .as_str()
        .unwrap()
        .to_owned();

    let restore = Command::new(binary)
        .args(["trash", "restore-original-preview"])
        .arg(&root)
        .arg(&item_id)
        .output()
        .expect("original restore preview");
    assert!(restore.status.success());
    let restore: serde_json::Value = serde_json::from_slice(&restore.stdout).unwrap();
    let restore_review = temporary.path().join("restore-review.json");
    std::fs::write(
        &restore_review,
        serde_json::to_vec(&restore["reviewedRequest"]).unwrap(),
    )
    .unwrap();
    let restored = Command::new(binary)
        .args(["trash", "commit"])
        .arg(&root)
        .arg(format!("@{}", restore_review.display()))
        .output()
        .expect("reviewed restore commit");
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    assert!(child.path.join("Child.adoc").is_file());

    std::fs::write(child.path.join("one.bin"), b"one").unwrap();
    std::fs::write(child.path.join("two.bin"), b"two-two").unwrap();
    let selections = serde_json::json!([
        {"ownerNodeId": child.id, "name": "one.bin"},
        {"ownerNodeId": child.id, "name": "two.bin"},
    ]);
    let resources = Command::new(binary)
        .args(["trash", "resources-preview"])
        .arg(&root)
        .arg(selections.to_string())
        .output()
        .expect("resource batch preview");
    assert!(resources.status.success());
    let resources: serde_json::Value = serde_json::from_slice(&resources.stdout).unwrap();
    assert_eq!(
        resources["plan"]["trashItemChanges"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        resources["plan"]["trashItemChanges"][0]["manifest"]["operationId"],
        resources["plan"]["trashItemChanges"][1]["manifest"]["operationId"]
    );
    let resources_review = temporary.path().join("resources-review.json");
    std::fs::write(
        &resources_review,
        serde_json::to_vec(&resources["reviewedRequest"]).unwrap(),
    )
    .unwrap();
    let resources_commit = Command::new(binary)
        .args(["trash", "commit"])
        .arg(&root)
        .arg(format!("@{}", resources_review.display()))
        .output()
        .expect("resource batch commit");
    assert!(resources_commit.status.success());
    assert!(!child.path.join("one.bin").exists());
    assert!(!child.path.join("two.bin").exists());

    let inventory = Command::new(binary)
        .args(["trash", "inventory"])
        .arg(&root)
        .output()
        .unwrap();
    let inventory: serde_json::Value = serde_json::from_slice(&inventory.stdout).unwrap();
    let items = inventory["trash"]["items"].as_array().unwrap();
    let one_item = items
        .iter()
        .find(|item| item["manifest"]["originalName"] == "one.bin")
        .unwrap();
    let one_id = one_item["manifest"]["trashItemId"].as_str().unwrap();
    let ids = serde_json::json!([one_id]).to_string();
    let permanent_preview = Command::new(binary)
        .args(["trash", "permanent-delete-preview"])
        .arg(&root)
        .arg(&ids)
        .output()
        .unwrap();
    assert!(permanent_preview.status.success());
    let permanent_preview: serde_json::Value =
        serde_json::from_slice(&permanent_preview.stdout).unwrap();
    assert_eq!(
        permanent_preview["permanentDeletePreview"]["items"][0]["payloadByteLength"],
        3
    );
    let evidence = permanent_preview["confirmationItems"].to_string();
    let mut wrong_evidence = permanent_preview["confirmationItems"].clone();
    wrong_evidence[0]["payloadSha256"] = serde_json::Value::String("0".repeat(64));
    let rejected = Command::new(binary)
        .args(["trash", "permanent-delete-review"])
        .arg(&root)
        .arg(wrong_evidence.to_string())
        .arg(weftext_core::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    let confirmed_delete = Command::new(binary)
        .args(["trash", "permanent-delete-review"])
        .arg(&root)
        .arg(&evidence)
        .arg(weftext_core::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE)
        .output()
        .unwrap();
    assert!(confirmed_delete.status.success());
    let confirmed_delete: serde_json::Value =
        serde_json::from_slice(&confirmed_delete.stdout).unwrap();
    let reviewed_request_path = temporary.path().join("permanent-review.json");
    std::fs::write(
        &reviewed_request_path,
        serde_json::to_vec(&confirmed_delete["reviewedRequest"]).unwrap(),
    )
    .unwrap();
    let permanently_deleted = Command::new(binary)
        .args(["trash", "permanent-delete-commit"])
        .arg(&root)
        .arg(format!("@{}", reviewed_request_path.display()))
        .arg(weftext_core::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE)
        .output()
        .unwrap();
    assert!(
        permanently_deleted.status.success(),
        "{}",
        String::from_utf8_lossy(&permanently_deleted.stderr)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one CLI migration case keeps external snapshot proof, reviewed commit, and explicit unknown-origin recovery together"
)]
fn cli_legacy_trash_migration_requires_external_snapshot_and_explicit_restore_target() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Legacy Trash CLI");
    let workspace = weftext_core::create_workspace(&root).expect("workspace");
    let child = weftext_core::create_child_node(&root, "Legacy").expect("legacy child");
    let trash = weftext_core::plan_trash_node_at(&root, child.id, "2026-08-24T12:00:00Z")
        .expect("item-backed Trash setup");
    weftext_core::commit_workspace_transaction(&trash).expect("Trash setup commit");
    let item = weftext_core::scan_workspace(&root).trash_items.remove(0);
    std::fs::rename(&item.payload_path, root.join(".weftext-trash/Legacy"))
        .expect("simulate historical direct entry");
    std::fs::remove_dir_all(
        root.join(".weftext-trash")
            .join(weftext_core::TRASH_ITEMS_DIRECTORY_NAME),
    )
    .expect("remove item store to leave only legacy authority");

    let binary = env!("CARGO_BIN_EXE_weftext");
    let degraded = Command::new(binary)
        .args(["trash", "inventory"])
        .arg(&root)
        .output()
        .expect("legacy read-only inventory");
    assert!(degraded.status.success());
    let degraded: serde_json::Value = serde_json::from_slice(&degraded.stdout).unwrap();
    assert_eq!(degraded["trash"]["state"], "legacy_migration_required");
    assert_eq!(degraded["trash"]["legacyMigrationRequired"], true);
    assert!(degraded["trash"]["items"].as_array().unwrap().is_empty());

    let snapshot_parent = temporary.path().join("migration snapshots");
    std::fs::create_dir(&snapshot_parent).unwrap();
    let preview = Command::new(binary)
        .args(["trash", "migration-preview"])
        .arg(&root)
        .arg(&snapshot_parent)
        .output()
        .expect("legacy migration preview");
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert!(root.join(".weftext-trash/Legacy/Legacy.adoc").is_file());
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(
        preview["plan"]["trashItemChanges"][0]["manifest"]["originStatus"],
        "unknown"
    );
    let snapshot_directory = preview["migrationBackup"]["snapshotDirectory"]
        .as_str()
        .expect("Core-created external snapshot directory")
        .to_owned();
    assert!(
        std::fs::canonicalize(&snapshot_directory)
            .unwrap()
            .starts_with(std::fs::canonicalize(&snapshot_parent).unwrap())
    );
    let reviewed = temporary.path().join("migration-review.json");
    std::fs::write(
        &reviewed,
        serde_json::to_vec(&preview["reviewedRequest"]).unwrap(),
    )
    .unwrap();

    let unauthorized = Command::new(binary)
        .args(["trash", "commit"])
        .arg(&root)
        .arg(format!("@{}", reviewed.display()))
        .output()
        .expect("ordinary commit must not authorize migration");
    assert!(!unauthorized.status.success());
    assert!(root.join(".weftext-trash/Legacy/Legacy.adoc").is_file());

    let migrated = Command::new(binary)
        .args(["trash", "migration-commit"])
        .arg(&root)
        .arg(format!("@{}", reviewed.display()))
        .arg(&snapshot_directory)
        .output()
        .expect("migration commit with reopened snapshot");
    assert!(
        migrated.status.success(),
        "{}",
        String::from_utf8_lossy(&migrated.stderr)
    );

    let inventory = Command::new(binary)
        .args(["trash", "inventory"])
        .arg(&root)
        .output()
        .unwrap();
    let inventory: serde_json::Value = serde_json::from_slice(&inventory.stdout).unwrap();
    assert_eq!(inventory["trash"]["state"], "ready");
    let item = &inventory["trash"]["items"][0];
    assert_eq!(item["manifest"]["originStatus"], "unknown");
    assert_eq!(item["restore"]["originalAvailable"], false);
    let item_id = item["manifest"]["trashItemId"].as_str().unwrap();

    let original = Command::new(binary)
        .args(["trash", "restore-original-preview"])
        .arg(&root)
        .arg(item_id)
        .output()
        .expect("unknown origin original restore");
    assert!(!original.status.success());

    let explicit = Command::new(binary)
        .args(["trash", "restore-existing-target-preview"])
        .arg(&root)
        .arg(item_id)
        .arg(workspace.id.to_string())
        .arg("Legacy restored")
        .output()
        .expect("explicit existing target preview");
    assert!(explicit.status.success());
    let explicit: serde_json::Value = serde_json::from_slice(&explicit.stdout).unwrap();
    let restore_review = temporary.path().join("legacy-restore-review.json");
    std::fs::write(
        &restore_review,
        serde_json::to_vec(&explicit["reviewedRequest"]).unwrap(),
    )
    .unwrap();
    let restored = Command::new(binary)
        .args(["trash", "commit"])
        .arg(&root)
        .arg(format!("@{}", restore_review.display()))
        .output()
        .expect("explicit target restore commit");
    assert!(restored.status.success());
    assert!(root.join("Legacy restored/Legacy restored.adoc").is_file());
}

#[test]
fn cli_previews_and_commits_narrow_node_envelope_actions() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Metadata");
    let created = weftext_core::create_workspace(&root).expect("workspace");
    let child = weftext_core::create_child_node(&root, "Child").expect("child");
    let binary = env!("CARGO_BIN_EXE_weftext");
    let child_before = weftext_core::read_node_document(&child.path).expect("child source");

    let preview = Command::new(binary)
        .args(["node", "aliases-preview"])
        .arg(&root)
        .arg(child.id.to_string())
        .arg(child_before.revision.to_string())
        .arg(r#"["别名","Alias"]"#)
        .output()
        .expect("aliases preview");
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview["plan"]["action"], "node_metadata");
    assert_eq!(
        weftext_core::read_node_document(&child.path)
            .expect("preview remains read-only")
            .source,
        child_before.source
    );

    let aliases = Command::new(binary)
        .args(["node", "aliases"])
        .arg(&root)
        .arg(child.id.to_string())
        .arg(child_before.revision.to_string())
        .arg(r#"["别名","Alias"]"#)
        .output()
        .expect("aliases commit");
    assert!(
        aliases.status.success(),
        "{}",
        String::from_utf8_lossy(&aliases.stderr)
    );
    let child_after_aliases =
        weftext_core::read_node_document(&child.path).expect("aliases source");
    assert!(
        child_after_aliases
            .source
            .contains("  aliases:\n    - \"别名\"\n    - \"Alias\"\n")
    );

    let root_before = weftext_core::read_node_document(&root).expect("root source");
    let child_sort = Command::new(binary)
        .args(["node", "child-sort"])
        .arg(&root)
        .arg(created.id.to_string())
        .arg(root_before.revision.to_string())
        .arg("name:descending")
        .output()
        .expect("child sort commit");
    assert!(
        child_sort.status.success(),
        "{}",
        String::from_utf8_lossy(&child_sort.stderr)
    );
    let root_after = weftext_core::read_node_document(&root).expect("sorted root");
    assert!(root_after.source.contains("  child_sort: name\n"));
    assert!(
        root_after
            .source
            .contains("  child_sort_direction: descending\n")
    );

    let sibling_rank = Command::new(binary)
        .args(["node", "sibling-rank"])
        .arg(&root)
        .arg(child.id.to_string())
        .arg(child_after_aliases.revision.to_string())
        .arg("2048")
        .output()
        .expect("sibling rank commit");
    assert!(
        sibling_rank.status.success(),
        "{}",
        String::from_utf8_lossy(&sibling_rank.stderr)
    );
    let ranked = weftext_core::read_node_document(&child.path).expect("ranked child");
    assert!(ranked.source.contains("  sibling_rank: 2048\n"));

    let stale = Command::new(binary)
        .args(["node", "aliases"])
        .arg(&root)
        .arg(child.id.to_string())
        .arg(child_before.revision.to_string())
        .arg("[]")
        .output()
        .expect("stale aliases commit");
    assert!(!stale.status.success());
    assert_eq!(
        weftext_core::read_node_document(&child.path)
            .expect("stale request preserved source")
            .source,
        ranked.source
    );
}

#[test]
fn cli_inventory_exposes_the_shared_core_content_contract_without_host_paths() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/content-boundary-v02");
    let output = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["workspace", "inventory"])
        .arg(&root)
        .output()
        .expect("inventory");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert!(value["workspace"].get("root").is_none());
    assert_eq!(
        value["workspace"]["nodes"].as_array().expect("nodes").len(),
        2
    );
    let content = value["workspace"]["content"].as_array().expect("content");
    assert!(content.iter().any(|entry| {
        entry["kind"] == "managed_node"
            && entry["path"] == "Managed"
            && entry["parentPath"] == ""
            && entry["nodeId"].is_string()
    }));
    assert!(content.iter().any(|entry| {
        entry["kind"] == "unmanaged_markdown"
            && entry["path"] == "loose.md"
            && entry["nodeId"].is_null()
    }));
    assert!(
        content
            .iter()
            .all(|entry| entry["path"] != "Managed/Managed.adoc")
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("IgnoredSearchToken"));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the shared Core action CLI scenario keeps create, rename, move, copy, preview evidence, and commit assertions together"
)]
fn cli_previews_and_commits_shared_core_workspace_actions() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Notes");
    let binary = env!("CARGO_BIN_EXE_weftext");
    let create_workspace = Command::new(binary)
        .args(["workspace", "create"])
        .arg(&root)
        .output()
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&create_workspace.stdout).unwrap();
    let root_id = created["node"]["id"].as_str().unwrap();

    let preview = Command::new(binary)
        .args(["node", "create-preview"])
        .arg(&root)
        .args([root_id, "Child"])
        .output()
        .unwrap();
    assert!(preview.status.success());
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview["plan"]["action"], "create");
    assert_eq!(preview["plan"]["pathChanges"][0]["newPath"], "Child");
    assert_eq!(preview["plan"]["capturedTarget"]["nodeId"], root_id);
    assert_eq!(
        preview["plan"]["capturedTarget"]["resolvedBy"],
        "caller_explicit"
    );
    assert_eq!(
        preview["plan"]["targetNodeIds"],
        serde_json::json!([root_id])
    );
    assert_eq!(
        preview["plan"]["draftSensitiveNodeIds"],
        serde_json::json!([])
    );
    assert!(!root.join("Child").exists());

    let commit = Command::new(binary)
        .args(["node", "create"])
        .arg(&root)
        .args([root_id, "Child"])
        .output()
        .unwrap();
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(root.join("Child/Child.adoc").is_file());

    let parent = Command::new(binary)
        .args(["node", "create"])
        .arg(&root)
        .args([root_id, "Parent"])
        .output()
        .unwrap();
    assert!(parent.status.success());
    let inventory = Command::new(binary)
        .args(["workspace", "inventory"])
        .arg(&root)
        .output()
        .unwrap();
    let inventory: serde_json::Value = serde_json::from_slice(&inventory.stdout).unwrap();
    let child_id = inventory["workspace"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["name"] == "Child")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let parent_id = inventory["workspace"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["name"] == "Parent")
        .unwrap()["id"]
        .as_str()
        .unwrap();

    let rename_preview = Command::new(binary)
        .args(["node", "rename-preview"])
        .arg(&root)
        .args([child_id, "Renamed"])
        .output()
        .unwrap();
    assert!(rename_preview.status.success());
    let rename_preview: serde_json::Value = serde_json::from_slice(&rename_preview.stdout).unwrap();
    assert_eq!(rename_preview["plan"]["action"], "rename");
    assert_eq!(
        rename_preview["plan"]["scopeSummary"]["identityPolicy"],
        "preserve"
    );
    assert!(root.join("Child/Child.adoc").is_file());
    let rename = Command::new(binary)
        .args(["node", "rename"])
        .arg(&root)
        .args([child_id, "Renamed"])
        .output()
        .unwrap();
    assert!(rename.status.success());
    assert!(root.join("Renamed/Renamed.adoc").is_file());

    let move_preview = Command::new(binary)
        .args(["node", "move-preview"])
        .arg(&root)
        .args([child_id, parent_id])
        .output()
        .unwrap();
    assert!(move_preview.status.success());
    let move_preview: serde_json::Value = serde_json::from_slice(&move_preview.stdout).unwrap();
    assert_eq!(move_preview["plan"]["action"], "move");
    assert_eq!(
        move_preview["plan"]["scopeSummary"]["rootNode"]["displayName"],
        "Renamed"
    );
    let moved = Command::new(binary)
        .args(["node", "move"])
        .arg(&root)
        .args([child_id, parent_id])
        .output()
        .unwrap();
    assert!(moved.status.success());
    assert!(root.join("Parent/Renamed/Renamed.adoc").is_file());

    let presentation = Command::new(binary)
        .args(["workspace", "presentation"])
        .arg(&root)
        .arg("run_in")
        .output()
        .unwrap();
    assert!(presentation.status.success());
    assert!(
        std::fs::read_to_string(root.join("Notes.adoc"))
            .unwrap()
            .contains("adjacent_heading_body: run_in")
    );
}

#[test]
fn cli_reports_first_tier_dsh_support_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["agent", "dsh", "support"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["agent"]["harness"], "dsh");
    assert_eq!(value["agent"]["supportTier"], "first_tier");
    assert_eq!(value["agent"]["implementationStage"], "read_only_tools");
    assert_eq!(value["agent"]["ready"], false);
    assert_eq!(value["agent"]["supportedWireVersions"][0], "0.0.1");
    assert_eq!(value["agent"]["cancellation"], "runtime_termination");
    assert_eq!(value["agent"]["approvalRequests"], false);
    assert_eq!(value["agent"]["readOnlyMcpTools"], true);
    assert_eq!(value["agent"]["mutationTools"], false);
}

#[test]
fn cli_serves_scoped_read_only_mcp_tools() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let binary = env!("CARGO_BIN_EXE_weftext");
    assert!(
        Command::new(binary)
            .args(["workspace", "create"])
            .arg(&workspace)
            .output()
            .unwrap()
            .status
            .success()
    );

    let mut child = Command::new(binary)
        .args(["agent", "mcp", "serve"])
        .arg(&workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    )
    .unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let lines = String::from_utf8(output.stdout).unwrap();
    let responses = lines
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["result"]["tools"].as_array().unwrap().len(), 2);
}

#[test]
fn cli_previews_commits_and_rejects_stale_document_edits() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("Notes");
    let binary = env!("CARGO_BIN_EXE_weftext");

    let create = Command::new(binary)
        .args(["workspace", "create"])
        .arg(&node)
        .output()
        .unwrap();
    assert!(create.status.success());

    let read = Command::new(binary)
        .args(["document", "read"])
        .arg(&node)
        .output()
        .unwrap();
    assert!(read.status.success());
    let snapshot: serde_json::Value = serde_json::from_slice(&read.stdout).unwrap();
    let revision = snapshot["document"]["revision"].as_str().unwrap();
    let source = snapshot["document"]["source"].as_str().unwrap();
    let end = source.len().to_string();

    let preview = Command::new(binary)
        .args(["document", "preview"])
        .arg(&node)
        .args([revision, &end, &end, "hello\n"])
        .output()
        .unwrap();
    assert!(preview.status.success());
    let preview_value: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview_value["plan"]["changed"], true);
    assert_eq!(preview_value["plan"]["baseRevision"], revision);
    assert!(
        preview_value["plan"]["nextSource"]
            .as_str()
            .unwrap()
            .ends_with("hello\n")
    );

    let commit = Command::new(binary)
        .args(["document", "commit"])
        .arg(&node)
        .args([revision, &end, &end, "hello\n"])
        .output()
        .unwrap();
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let commit_value: serde_json::Value = serde_json::from_slice(&commit.stdout).unwrap();
    assert_ne!(commit_value["commit"]["revision"], revision);

    let stale = Command::new(binary)
        .args(["document", "commit"])
        .arg(&node)
        .args([revision, &end, &end, "lost\n"])
        .output()
        .unwrap();
    assert!(!stale.status.success());
    let stale_value: serde_json::Value = serde_json::from_slice(&stale.stderr).unwrap();
    assert!(
        stale_value["error"]
            .as_str()
            .unwrap()
            .contains("stale document revision")
    );

    let final_read = Command::new(binary)
        .args(["document", "read"])
        .arg(&node)
        .output()
        .unwrap();
    let final_value: serde_json::Value = serde_json::from_slice(&final_read.stdout).unwrap();
    assert!(
        final_value["document"]["source"]
            .as_str()
            .unwrap()
            .ends_with("hello\n")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn cli_exposes_citation_occurrences_and_retires_reference_record_writes() {
    const ROOT_ID: &str = "11111111-1111-4111-8111-111111111111";
    const COMPONENT_ID: &str = "22222222-2222-4222-8222-222222222222";

    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Citations");
    std::fs::create_dir_all(root.join("Component")).unwrap();
    std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
    std::fs::write(
        root.join("Citations.adoc"),
        format!("---\nweftext:\n  id: \"{ROOT_ID}\"\n---\n= Citations\n"),
    )
    .unwrap();
    let component_source = format!(
        "---\nweftext:\n  id: \"{COMPONENT_ID}\"\n---\n= Component\n\nEvidence cite:[smith2024].\n\nbibliography::[]\n"
    );
    std::fs::write(root.join("Component/Component.adoc"), &component_source).unwrap();

    let binary = env!("CARGO_BIN_EXE_weftext");
    let capabilities = Command::new(binary)
        .args(["citation", "capabilities"])
        .output()
        .unwrap();
    assert!(capabilities.status.success());
    let capabilities: serde_json::Value = serde_json::from_slice(&capabilities.stdout).unwrap();
    assert_eq!(
        capabilities["capabilities"]["referenceRecordWritesAvailable"],
        false
    );
    assert!(
        capabilities["capabilities"]["referenceRecordWritesReason"]
            .as_str()
            .unwrap()
            .contains("typed Citation Data")
    );

    let validation = Command::new(binary)
        .args(["citation", "validate"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(!validation.status.success());
    let validation: serde_json::Value = serde_json::from_slice(&validation.stderr).unwrap();
    assert_eq!(validation["validation"]["valid"], false);
    assert!(
        validation["validation"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|component| !component["diagnostics"].as_array().unwrap().is_empty())
    );

    let inspect = Command::new(binary)
        .args(["citation", "inspect"])
        .arg(&root)
        .arg(COMPONENT_ID)
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let inspect: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(
        inspect["authoring"]["citations"]["clusters"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        inspect["authoring"]["citations"]["bibliographies"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert!(
        !inspect["analysis"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let search = Command::new(binary)
        .args(["citation", "search"])
        .arg(&root)
        .args(["smith", "10"])
        .output()
        .unwrap();
    assert!(search.status.success());
    let search: serde_json::Value = serde_json::from_slice(&search.stdout).unwrap();
    assert!(search["references"].as_array().unwrap().is_empty());

    for command in [
        "rename-preview",
        "rename",
        "reference-create-preview",
        "reference-create",
        "reference-edit-preview",
        "reference-edit",
    ] {
        let absent = Command::new(binary)
            .args(["citation", command])
            .output()
            .unwrap();
        assert!(!absent.status.success(), "{command} unexpectedly succeeded");
        let absent: serde_json::Value = serde_json::from_slice(&absent.stderr).unwrap();
        assert!(
            absent["error"]
                .as_str()
                .unwrap()
                .starts_with("usage: weftext citation"),
            "{command}: {absent}"
        );
        assert!(!absent.to_string().contains("reference-create"));
    }

    assert_eq!(
        std::fs::read_to_string(root.join("Component/Component.adoc")).unwrap(),
        component_source
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn prototype_bridge_exposes_citation_diagnostics_without_reference_write_routes() {
    const ROOT_ID: &str = "44444444-4444-4444-8444-444444444444";
    const COMPONENT_ID: &str = "55555555-5555-4555-8555-555555555555";

    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Bridge citations");
    std::fs::create_dir_all(root.join("Component")).unwrap();
    std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
    std::fs::write(
        root.join("Bridge citations.adoc"),
        format!("---\nweftext:\n  id: \"{ROOT_ID}\"\n---\n= Bridge citations\n"),
    )
    .unwrap();
    let component_source = format!(
        "---\nweftext:\n  id: \"{COMPONENT_ID}\"\n---\n= Component\n\nEvidence cite:[smith2024].\n\nbibliography::[]\n"
    );
    std::fs::write(root.join("Component/Component.adoc"), &component_source).unwrap();

    let mut bridge = ChildGuard::new(
        Command::new(env!("CARGO_BIN_EXE_weftext"))
            .args(["prototype", "serve"])
            .arg(&root)
            .arg("0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut ready_line = String::new();
    BufReader::new(bridge.child.stdout.take().unwrap())
        .read_line(&mut ready_line)
        .unwrap();
    let ready: serde_json::Value = serde_json::from_str(&ready_line).unwrap();
    let address = ready["endpoint"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    let token = ready["openUrl"]
        .as_str()
        .unwrap()
        .split("&token=")
        .nth(1)
        .unwrap();

    let capabilities = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/citation/capabilities",
        None,
    ));
    assert_eq!(
        capabilities["capabilities"]["referenceRecordWritesAvailable"],
        false
    );

    let validation = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/citation/validate",
        None,
    ));
    assert_eq!(validation["validation"]["valid"], false);
    assert!(
        validation["validation"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|component| !component["diagnostics"].as_array().unwrap().is_empty())
    );

    let search = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/citation/search?q=smith&limit=10",
        None,
    ));
    assert!(search["references"].as_array().unwrap().is_empty());

    let analyze = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/citation/analyze",
        Some(&serde_json::json!({
            "nodeId": COMPONENT_ID,
            "source": component_source,
            "styleId": "apa",
            "locale": "en-US",
        })),
    ));
    assert_eq!(
        analyze["authoring"]["citations"]["clusters"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        analyze["authoring"]["citations"]["bibliographies"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert!(
        !analyze["analysis"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(analyze["presentation"].is_null());
    assert!(analyze["presentationFailure"].is_object());

    for route in [
        "/api/citation/reference/create-preview",
        "/api/citation/reference/edit-preview",
        "/api/citation/rename-preview",
        "/api/citation/transaction/commit",
    ] {
        let absent = bridge_request(address, token, "POST", route, Some(&serde_json::json!({})));
        assert!(
            absent.starts_with("HTTP/1.1 404 Not Found"),
            "{route}: {absent}"
        );
        assert_eq!(
            response_json(&absent)["error"],
            "unknown prototype bridge route"
        );
    }

    let recovery = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/citation/recover",
        None,
    ));
    assert_eq!(recovery["recovery"]["applyingRolledBack"], 0);
    bridge.stop();
}

#[test]
#[allow(clippy::too_many_lines)]
fn prototype_bridge_exposes_stored_task_transactions() {
    const ROOT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
    const CHILD_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
    const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
    const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
    const TASK_R: &str = "33333333-3333-4333-8333-333333333333";

    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Bridge tasks");
    std::fs::create_dir_all(root.join("Child")).unwrap();
    std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
    std::fs::write(
        root.join("Bridge tasks.adoc"),
        format!(
            concat!(
                "---\nweftext:\n  id: \"{}\"\n---\n= Bridge tasks\n\n",
                "* [ ] Editable task:[id={}]\n",
                "* [ ] Repeat task:[id={},due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=2\",repeat-from=due]\n"
            ),
            ROOT_ID,
            TASK_A,
            TASK_R,
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("Child/Child.adoc"),
        format!(
            "---\nweftext:\n  id: \"{CHILD_ID}\"\n---\n= Child\n\n* [ ] Dependency task:[id={TASK_B}]\n"
        ),
    )
    .unwrap();

    let mut bridge = ChildGuard::new(
        Command::new(env!("CARGO_BIN_EXE_weftext"))
            .args(["prototype", "serve"])
            .arg(&root)
            .arg("0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut ready_line = String::new();
    BufReader::new(bridge.child.stdout.take().unwrap())
        .read_line(&mut ready_line)
        .unwrap();
    let ready: serde_json::Value = serde_json::from_str(&ready_line).unwrap();
    let address = ready["endpoint"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    let token = ready["openUrl"]
        .as_str()
        .unwrap()
        .split("&token=")
        .nth(1)
        .unwrap();

    let validation = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/task/validate",
        None,
    ));
    assert_eq!(validation["validation"]["valid"], true);
    assert_eq!(
        validation["validation"]["occurrences"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    let inspection = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/task/inspect?nodeId={ROOT_ID}"),
        None,
    ));
    assert_eq!(inspection["occurrences"].as_array().map(Vec::len), Some(2));
    let query = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/query/execute",
        Some(&serde_json::json!({
            "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
            "blockIndex": 0,
            "context": {
                "today": {"year": 2026, "month": 8, "day": 24},
                "now": "2026-08-24T09:30:00+08:00",
                "timezone": "Asia/Shanghai",
                "locale": "zh-CN",
                "binding": {"nodeId": ROOT_ID, "heading": null},
            },
        })),
    ));
    assert_eq!(query["valid"], true);
    assert_eq!(query["execution"]["result"]["totalBeforeLimit"], 2);
    assert_eq!(query["execution"]["result"]["columns"][0]["path"], "name");
    assert_eq!(
        query["execution"]["csv"],
        "name,path\r\nBridge tasks,/\r\nChild,/Child\r\n"
    );
    assert_eq!(
        query["execution"]["result"]["rows"][1]["cells"][1]["value"]["value"],
        "/Child"
    );

    let workspace = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/workspace",
        None,
    ));
    let document = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/document?nodeId={ROOT_ID}"),
        None,
    ));
    let edit = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/edit-preview",
        Some(&serde_json::json!({
            "nodeId": ROOT_ID,
            "baseWorkspaceRevision": workspace["workspace"]["revision"],
            "baseRevision": document["document"]["revision"],
            "target": {"kind": "id", "id": TASK_A},
            "intent": {"kind": "set_priority", "priority": "high"},
        })),
    ));
    assert!(
        edit["plan"]["authoring"]["proposedSource"]
            .as_str()
            .unwrap()
            .contains("priority=high")
    );
    assert!(
        !std::fs::read_to_string(root.join("Bridge tasks.adoc"))
            .unwrap()
            .contains("priority=high")
    );
    let edit_plan_id = edit["plan"]["planId"].as_str().unwrap().to_owned();
    let edited = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/transaction/commit",
        Some(&serde_json::json!({"planId": edit_plan_id})),
    ));
    assert_eq!(edited["result"]["task"]["metadata"]["priority"], "high");
    let replay = bridge_request(
        address,
        token,
        "POST",
        "/api/task/transaction/commit",
        Some(&serde_json::json!({"planId": edit_plan_id})),
    );
    assert!(replay.starts_with("HTTP/1.1 400 Bad Request"));

    let workspace = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/workspace",
        None,
    ));
    let document = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/document?nodeId={ROOT_ID}"),
        None,
    ));
    let dependencies = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/dependencies-preview",
        Some(&serde_json::json!({
            "nodeId": ROOT_ID,
            "baseWorkspaceRevision": workspace["workspace"]["revision"],
            "baseRevision": document["document"]["revision"],
            "target": {"kind": "id", "id": TASK_A},
            "dependencies": [TASK_B],
        })),
    ));
    let dependency_commit = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/transaction/commit",
        Some(&serde_json::json!({"planId": dependencies["plan"]["planId"]})),
    ));
    assert_eq!(dependency_commit["result"]["dependencies"][0], TASK_B);

    let workspace = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/workspace",
        None,
    ));
    let document = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/document?nodeId={ROOT_ID}"),
        None,
    ));
    let recurrence = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/recurrence-preview",
        Some(&serde_json::json!({
            "nodeId": ROOT_ID,
            "baseWorkspaceRevision": workspace["workspace"]["revision"],
            "baseRevision": document["document"]["revision"],
            "target": {"kind": "id", "id": TASK_R},
            "context": {
                "completedAt": {"kind": "date", "value": "2026-08-24"},
                "utcOffsetMinutes": 480,
            },
        })),
    ));
    let next_id = recurrence["plan"]["completion"]["nextTaskId"].clone();
    let recurred = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/transaction/commit",
        Some(&serde_json::json!({"planId": recurrence["plan"]["planId"]})),
    ));
    assert_eq!(recurred["result"]["nextTaskId"], next_id);

    let stale_workspace = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/workspace",
        None,
    ));
    let stale_document = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/document?nodeId={ROOT_ID}"),
        None,
    ));
    let child_path = root.join("Child/Child.adoc");
    let child = std::fs::read_to_string(&child_path).unwrap();
    std::fs::write(&child_path, format!("{child}\nExternal change.\n")).unwrap();
    let stale = bridge_request(
        address,
        token,
        "POST",
        "/api/task/edit-preview",
        Some(&serde_json::json!({
            "nodeId": ROOT_ID,
            "baseWorkspaceRevision": stale_workspace["workspace"]["revision"],
            "baseRevision": stale_document["document"]["revision"],
            "target": {"kind": "id", "id": TASK_A},
            "intent": {"kind": "set_priority", "priority": "low"},
        })),
    );
    assert!(stale.starts_with("HTTP/1.1 400 Bad Request"));
    assert!(
        response_json(&stale)["error"]
            .as_str()
            .unwrap()
            .contains("workspace changed")
    );
    let recovery = response_json(&bridge_request(
        address,
        token,
        "POST",
        "/api/task/recover",
        None,
    ));
    assert_eq!(recovery["recovery"]["applyingRolledBack"], 0);
    bridge.stop();
}

#[test]
#[allow(clippy::too_many_lines)]
fn prototype_bridge_uses_core_preview_commit_and_stale_revision_checks() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("Notes");
    let binary = env!("CARGO_BIN_EXE_weftext");

    let create = Command::new(binary)
        .args(["workspace", "create"])
        .arg(&node)
        .output()
        .unwrap();
    assert!(create.status.success());
    let created: serde_json::Value = serde_json::from_slice(&create.stdout).unwrap();
    let created_root_id = created["node"]["id"].as_str().unwrap();
    let startup_index_path = std::env::temp_dir()
        .join("weftext-search-indexes")
        .join(format!("{created_root_id}.json"));
    std::fs::create_dir_all(startup_index_path.parent().unwrap()).unwrap();
    std::fs::create_dir(&startup_index_path)
        .expect("replace the not-yet-created derived index with a blocking directory");

    let mut bridge = ChildGuard::new(
        Command::new(binary)
            .args(["prototype", "serve"])
            .arg(&node)
            .arg("0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut ready_line = String::new();
    BufReader::new(bridge.child.stdout.take().unwrap())
        .read_line(&mut ready_line)
        .unwrap();
    let ready: serde_json::Value = serde_json::from_str(&ready_line).unwrap();
    assert_eq!(ready["scope"], "workspace");
    assert_eq!(ready["server"], false);
    assert!(ready["searchIndex"].is_null());
    assert_eq!(
        ready["searchIndexWarning"]["code"],
        "derived_search_index_refresh_failed"
    );
    assert_eq!(ready["searchIndexWarning"]["rebuildRequired"], true);
    assert_eq!(ready["searchIndexWarning"]["workspaceOpenSucceeded"], true);
    let endpoint = ready["endpoint"].as_str().unwrap();
    let address = endpoint.strip_prefix("http://").unwrap();
    let open_url = ready["openUrl"].as_str().unwrap();
    let token = open_url.split("&token=").nth(1).unwrap();

    let workspace_state = bridge_request(address, token, "GET", "/api/workspace", None);
    assert!(workspace_state.starts_with("HTTP/1.1 200 OK"));
    let workspace_value = response_json(&workspace_state);
    let root_id = workspace_value["workspace"]["rootNodeId"].as_str().unwrap();
    assert_eq!(root_id, created_root_id);
    assert_eq!(
        workspace_value["workspace"]["documentFormat"]["generation"],
        "ascii_doc_v1"
    );
    assert!(workspace_value["searchIndex"].is_null());
    assert_eq!(
        workspace_value["searchIndexWarning"]["code"],
        "derived_search_index_refresh_failed"
    );

    let authoritative_document = bridge_request(address, token, "GET", "/api/document", None);
    assert!(authoritative_document.starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        response_json(&authoritative_document)["document"]["nodeId"],
        root_id
    );
    let unavailable_search = bridge_request(address, token, "GET", "/api/search?q=Notes", None);
    assert!(unavailable_search.starts_with("HTTP/1.1 400 Bad Request"));
    assert!(
        !response_json(&unavailable_search)["error"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
    );
    std::fs::remove_dir(&startup_index_path).expect("remove startup failure injection");

    let create_action = serde_json::json!({
        "action": "create",
        "parentId": root_id,
        "name": "Bridge child",
        "resolvedBy": "focused_pane",
    });
    let action_preview = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/preview",
        Some(&create_action),
    );
    assert!(action_preview.starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        response_json(&action_preview)["plan"]["capturedTarget"]["resolvedBy"],
        "focused_pane"
    );
    let plan_id = response_json(&action_preview)["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!node.join("Bridge child").exists());
    let action_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": plan_id})),
    );
    assert!(action_commit.starts_with("HTTP/1.1 200 OK"));
    assert!(node.join("Bridge child/Bridge child.adoc").is_file());
    let action_commit_value = response_json(&action_commit);
    let child_id = action_commit_value["workspace"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "Bridge child")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let read = bridge_request(address, token, "GET", "/api/document", None);
    assert!(read.starts_with("HTTP/1.1 200 OK"));
    let snapshot = response_json(&read);
    let revision = snapshot["document"]["revision"].as_str().unwrap();
    let source = snapshot["document"]["source"].as_str().unwrap();
    let next_source =
        format!("{source}hello from the live prototype node:{child_id}[Bridge child]\n");
    let edit = serde_json::json!({"revision": revision, "source": next_source});

    let preview = bridge_request(address, token, "POST", "/api/document/preview", Some(&edit));
    assert!(preview.starts_with("HTTP/1.1 200 OK"));
    let preview_value = response_json(&preview);
    assert_eq!(preview_value["plan"]["changed"], true);
    assert_eq!(preview_value["plan"]["baseRevision"], revision);

    let commit = bridge_request(address, token, "POST", "/api/document/commit", Some(&edit));
    assert!(commit.starts_with("HTTP/1.1 200 OK"));
    let commit_value = response_json(&commit);
    assert_ne!(commit_value["commit"]["revision"], revision);

    let icon_base = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/document",
        None,
    ));
    let icon_revision = icon_base["document"]["revision"].as_str().unwrap();
    assert_eq!(
        icon_base["document"]["metadata"]["schema"],
        "weftext.node-metadata.v1"
    );
    assert_eq!(icon_base["document"]["metadata"]["id"], root_id);
    let icon_preview = bridge_request(
        address,
        token,
        "POST",
        "/api/node/metadata/preview",
        Some(&serde_json::json!({
            "action": "icon",
            "icon": "weftext:book",
            "nodeId": root_id,
            "revision": icon_revision,
        })),
    );
    assert!(icon_preview.starts_with("HTTP/1.1 200 OK"));
    let icon_plan = response_json(&icon_preview)["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let icon_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": icon_plan})),
    );
    assert!(icon_commit.starts_with("HTTP/1.1 200 OK"));
    let icon_replay = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": icon_plan})),
    );
    assert!(icon_replay.starts_with("HTTP/1.1 409 Conflict"));
    let icon_after = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/document",
        None,
    ));
    assert_eq!(icon_after["document"]["metadata"]["icon"], "weftext:book");
    assert_eq!(
        icon_after["document"]["metadata"]["resolvedIcon"]["glyph"],
        "书"
    );
    assert!(
        icon_after["document"]["source"]
            .as_str()
            .is_some_and(|source| source.contains("  icon: \"weftext:book\"\n"))
    );

    let child_document = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/document?nodeId={child_id}&remember=false"),
        None,
    ));
    let metadata_preview = bridge_request(
        address,
        token,
        "POST",
        "/api/node/metadata/preview",
        Some(&serde_json::json!({
            "action": "aliases",
            "nodeId": child_id,
            "revision": child_document["document"]["revision"],
            "aliases": ["桥接别名", "Bridge alias"],
        })),
    );
    assert!(metadata_preview.starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        response_json(&metadata_preview)["plan"]["action"],
        "node_metadata"
    );
    let metadata_plan = response_json(&metadata_preview)["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let metadata_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": metadata_plan})),
    );
    assert!(metadata_commit.starts_with("HTTP/1.1 200 OK"));
    let aliased_child = response_json(&bridge_request(
        address,
        token,
        "GET",
        &format!("/api/document?nodeId={child_id}&remember=false"),
        None,
    ));
    assert!(
        aliased_child["document"]["source"]
            .as_str()
            .is_some_and(|source| source
                .contains("  aliases:\n    - \"桥接别名\"\n    - \"Bridge alias\"\n"))
    );

    let ambiguous_sort = bridge_request(
        address,
        token,
        "POST",
        "/api/node/metadata/preview",
        Some(&serde_json::json!({
            "action": "child_sort",
            "nodeId": child_id,
            "revision": aliased_child["document"]["revision"],
            "mode": "manual",
            "direction": "descending",
        })),
    );
    assert!(ambiguous_sort.starts_with("HTTP/1.1 422 Unprocessable Content"));

    let parsed = bridge_request(
        address,
        token,
        "POST",
        "/api/document/model",
        Some(&serde_json::json!({"source": "== 标题\n紧邻正文"})),
    );
    assert!(parsed.starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        response_json(&parsed)["model"]["blocks"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(response_json(&parsed)["profile"]["profile"], "ascii_doc_v1");
    assert_eq!(
        response_json(&parsed)["view"]["blocks"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    let search = bridge_request(
        address,
        token,
        "GET",
        "/api/search?q=live%20prototype",
        None,
    );
    assert!(search.starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        response_json(&search)["results"].as_array().map(Vec::len),
        Some(1)
    );

    let stale = bridge_request(address, token, "POST", "/api/document/commit", Some(&edit));
    assert!(stale.starts_with("HTTP/1.1 409 Conflict"));
    assert!(
        response_json(&stale)["error"]
            .as_str()
            .unwrap()
            .contains("stale document revision")
    );

    let rename_preview = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/preview",
        Some(
            &serde_json::json!({"action": "rename", "nodeId": child_id, "name": "Bridge moved", "resolvedBy": "explicit_row"}),
        ),
    );
    assert!(rename_preview.starts_with("HTTP/1.1 200 OK"));
    let rename_preview = response_json(&rename_preview);
    assert_eq!(rename_preview["plan"]["action"], "rename");
    assert_eq!(
        rename_preview["plan"]["capturedTarget"]["resolvedBy"],
        "explicit_row"
    );
    assert_eq!(
        rename_preview["plan"]["scopeSummary"]["identityPolicy"],
        "preserve"
    );
    let move_plan = rename_preview["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let move_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": move_plan})),
    );
    assert!(move_commit.starts_with("HTTP/1.1 200 OK"));
    assert!(node.join("Bridge moved/Bridge moved.adoc").is_file());
    assert!(
        std::fs::read_to_string(node.join("Notes.adoc"))
            .unwrap()
            .contains(&format!("node:{child_id}[Bridge child]"))
    );

    let before_trash = response_json(&bridge_request(address, token, "GET", "/api/trash", None));
    let trash_preview = bridge_request(
        address,
        token,
        "POST",
        "/api/trash/node/preview",
        Some(&serde_json::json!({
            "nodeId": child_id,
            "baseWorkspaceRevision": before_trash["trash"]["workspaceRevision"],
            "trashedAt": "2026-08-24T12:00:00Z",
            "resolvedBy": "explicit_row",
        })),
    );
    assert!(trash_preview.starts_with("HTTP/1.1 200 OK"));
    let trash_preview = response_json(&trash_preview);
    assert_eq!(trash_preview["plan"]["pathChanges"], serde_json::json!([]));
    assert_eq!(
        trash_preview["plan"]["documentChanges"],
        serde_json::json!([])
    );
    assert_eq!(
        trash_preview["plan"]["generatedNodeIds"],
        serde_json::json!([])
    );
    assert_eq!(
        trash_preview["plan"]["capturedTarget"]["resolvedBy"],
        "explicit_row"
    );
    assert!(
        !serde_json::to_string(&trash_preview["plan"])
            .unwrap()
            .contains(".weftext-trash")
    );
    let trash_plan = trash_preview["plan"]["planId"].as_str().unwrap().to_owned();
    let trash_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": trash_plan})),
    );
    assert!(trash_commit.starts_with("HTTP/1.1 200 OK"));
    assert!(!node.join("Bridge moved").exists());
    let trash_inventory = response_json(&bridge_request(address, token, "GET", "/api/trash", None));
    assert_eq!(trash_inventory["trash"]["state"], "ready");
    let trash_items = trash_inventory["trash"]["items"]
        .as_array()
        .expect("Core Trash item inventory");
    assert_eq!(trash_items.len(), 1);
    assert_eq!(trash_items[0]["manifest"]["kind"], "node");
    assert_eq!(trash_items[0]["manifest"]["nodeId"], child_id);
    assert!(trash_items[0].get("itemPath").is_none());
    let trash_item_id = trash_items[0]["manifest"]["trashItemId"]
        .as_str()
        .expect("temporary Trash item ID")
        .to_owned();
    let trashed_search =
        bridge_request(address, token, "GET", "/api/search?q=Bridge%20moved", None);
    let trashed_results = response_json(&trashed_search)["results"]
        .as_array()
        .cloned()
        .expect("search results");
    assert!(trashed_results.iter().all(|result| {
        result["name"] != "Bridge moved"
            && !result["path"]
                .as_str()
                .is_some_and(|path| path.starts_with(".weftext-trash"))
    }));

    let restore_preview = bridge_request(
        address,
        token,
        "POST",
        "/api/trash/restore/preview",
        Some(&serde_json::json!({
            "mode": "existing_target",
            "trashItemId": trash_item_id,
            "baseWorkspaceRevision": trash_inventory["trash"]["workspaceRevision"],
            "targetNodeId": root_id,
            "name": "Bridge restored",
        })),
    );
    assert!(restore_preview.starts_with("HTTP/1.1 200 OK"));
    let restore_plan = response_json(&restore_preview)["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let restore_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": restore_plan})),
    );
    assert!(restore_commit.starts_with("HTTP/1.1 200 OK"));
    assert!(node.join("Bridge restored/Bridge restored.adoc").is_file());

    let current = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/document",
        None,
    ));
    let current_revision = current["document"]["revision"].as_str().unwrap();
    let current_source = current["document"]["source"].as_str().unwrap();
    let committed_source =
        format!("{current_source}\nCLI authority survives derived-index failure\n");
    let index_path = std::env::temp_dir()
        .join("weftext-search-indexes")
        .join(format!("{root_id}.json"));
    std::fs::remove_file(&index_path).expect("remove derived index file for failure injection");
    std::fs::create_dir(&index_path).expect("replace derived index file with blocking directory");
    let authoritative_commit = bridge_request(
        address,
        token,
        "POST",
        "/api/document/commit",
        Some(&serde_json::json!({
            "revision": current_revision,
            "source": committed_source,
        })),
    );
    assert!(authoritative_commit.starts_with("HTTP/1.1 200 OK"));
    let authoritative_value = response_json(&authoritative_commit);
    assert_eq!(
        authoritative_value["searchIndexWarning"]["code"],
        "derived_search_index_refresh_failed"
    );
    assert_eq!(
        std::fs::read_to_string(node.join("Notes.adoc")).expect("authoritative source"),
        committed_source
    );
    std::fs::remove_dir(&index_path).expect("remove injected derived-index blocker");

    let denied = bridge_request(address, "wrong-token", "GET", "/api/document", None);
    assert!(denied.starts_with("HTTP/1.1 401 Unauthorized"));
    bridge.stop();
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one bridge boundary case keeps legacy read-only state, commit freeze, and raw host-path rejection together"
)]
fn prototype_bridge_keeps_legacy_trash_read_only_without_a_typed_snapshot_capability() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Legacy bridge");
    weftext_core::create_workspace(&root).unwrap();
    let child = weftext_core::create_child_node(&root, "Legacy").unwrap();
    let plan = weftext_core::plan_trash_node_at(&root, child.id, "2026-08-24T12:00:00Z").unwrap();
    weftext_core::commit_workspace_transaction(&plan).unwrap();
    let item = weftext_core::scan_workspace(&root).trash_items.remove(0);
    std::fs::rename(&item.payload_path, root.join(".weftext-trash/Legacy")).unwrap();
    std::fs::remove_dir_all(
        root.join(".weftext-trash")
            .join(weftext_core::TRASH_ITEMS_DIRECTORY_NAME),
    )
    .unwrap();
    let snapshots = temporary.path().join("external snapshots");
    std::fs::create_dir(&snapshots).unwrap();

    let binary = env!("CARGO_BIN_EXE_weftext");
    let mut bridge = ChildGuard::new(
        Command::new(binary)
            .args(["prototype", "serve"])
            .arg(&root)
            .arg("0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut ready_line = String::new();
    BufReader::new(bridge.child.stdout.take().unwrap())
        .read_line(&mut ready_line)
        .unwrap();
    let ready: serde_json::Value = serde_json::from_str(&ready_line).unwrap();
    let address = ready["endpoint"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    let token = ready["openUrl"]
        .as_str()
        .unwrap()
        .split("&token=")
        .nth(1)
        .unwrap();

    let workspace_payload = response_json(&bridge_request(
        address,
        token,
        "GET",
        "/api/workspace",
        None,
    ));
    assert_eq!(
        workspace_payload["workspace"]["trashLegacyMigrationRequired"],
        true
    );
    assert!(
        workspace_payload["workspace"]["trashItems"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        workspace_payload["workspace"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["id"] != child.id.to_string())
    );

    let blocked = bridge_request(
        address,
        token,
        "POST",
        "/api/workspace/action/commit",
        Some(&serde_json::json!({"planId": "not-a-migration-plan"})),
    );
    assert!(blocked.starts_with("HTTP/1.1 409 Conflict"));
    assert!(
        response_json(&blocked)["error"]
            .as_str()
            .unwrap()
            .contains("explicit migration")
    );

    let before = response_json(&bridge_request(address, token, "GET", "/api/trash", None));
    assert_eq!(before["trash"]["state"], "legacy_migration_required");
    let preview = bridge_request(
        address,
        token,
        "POST",
        "/api/trash/migrate-legacy/preview",
        Some(&serde_json::json!({
            "baseWorkspaceRevision": before["trash"]["workspaceRevision"],
            "trashedAt": "2026-08-24T12:30:00Z",
            "snapshotParent": snapshots,
        })),
    );
    assert!(
        preview.starts_with("HTTP/1.1 422 Unprocessable Content"),
        "{preview}"
    );
    assert!(
        response_json(&preview)["error"]
            .as_str()
            .unwrap()
            .contains("does not accept host snapshot paths")
    );
    assert_eq!(
        std::fs::read_dir(&snapshots).unwrap().count(),
        0,
        "a browser-provided host path must never create a snapshot"
    );
    assert!(root.join(".weftext-trash/Legacy/Legacy.adoc").is_file());
    assert_eq!(
        response_json(&bridge_request(address, token, "GET", "/api/trash", None))["trash"]["state"],
        "legacy_migration_required"
    );
    bridge.stop();
}

#[test]
#[allow(clippy::too_many_lines)]
fn cli_exposes_validated_task_preview_commit_and_recovery_actions() {
    const ROOT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
    const CHILD_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
    const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
    const TASK_B: &str = "22222222-2222-4222-8222-222222222222";

    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Task CLI");
    std::fs::create_dir_all(root.join("Child")).unwrap();
    std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
    let root_source = format!(
        concat!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Tasks\n\n",
            "* [ ] Editable task:[id={}]\n",
            "* [ ] Repeat task:[id=33333333-3333-4333-8333-333333333333,due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=2\",repeat-from=due]\n"
        ),
        ROOT_ID, TASK_A
    );
    std::fs::write(root.join("Task CLI.adoc"), &root_source).unwrap();
    std::fs::write(
        root.join("Child/Child.adoc"),
        format!(
            "---\nweftext:\n  id: \"{CHILD_ID}\"\n---\n= Child\n\n* [ ] Dependency task:[id={TASK_B}]\n"
        ),
    )
    .unwrap();
    let binary = env!("CARGO_BIN_EXE_weftext");

    let validation = Command::new(binary)
        .args(["task", "validate"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(validation.status.success());
    let validation: serde_json::Value = serde_json::from_slice(&validation.stdout).unwrap();
    assert_eq!(validation["validation"]["valid"], true);
    assert_eq!(
        validation["validation"]["occurrences"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let inspect = Command::new(binary)
        .args(["task", "inspect"])
        .arg(&root)
        .arg(ROOT_ID)
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let inspect: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(inspect["occurrences"].as_array().unwrap().len(), 2);
    let revision = inspect["occurrences"][0]["revision"].as_str().unwrap();

    let target_a = serde_json::json!({"kind": "id", "id": TASK_A}).to_string();
    let priority = serde_json::json!({"kind": "set_priority", "priority": "high"}).to_string();
    let preview = Command::new(binary)
        .args(["task", "edit-preview"])
        .arg(&root)
        .args([ROOT_ID, revision, &target_a, &priority])
        .output()
        .unwrap();
    assert!(preview.status.success());
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview["plan"]["transaction"]["action"], "task_edit");
    assert!(
        preview["plan"]["authoring"]["proposedSource"]
            .as_str()
            .unwrap()
            .contains("priority=high")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("Task CLI.adoc")).unwrap(),
        root_source
    );
    let commit = Command::new(binary)
        .args(["task", "edit"])
        .arg(&root)
        .args([ROOT_ID, revision, &target_a, &priority])
        .output()
        .unwrap();
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(
        std::fs::read_to_string(root.join("Task CLI.adoc"))
            .unwrap()
            .contains("priority=high")
    );

    let inspect = Command::new(binary)
        .args(["task", "inspect"])
        .arg(&root)
        .arg(ROOT_ID)
        .output()
        .unwrap();
    let inspect: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    let revision = inspect["occurrences"][0]["revision"].as_str().unwrap();
    let dependencies = serde_json::json!([TASK_B]).to_string();
    let dependency_commit = Command::new(binary)
        .args(["task", "dependencies"])
        .arg(&root)
        .args([ROOT_ID, revision, &target_a, &dependencies])
        .output()
        .unwrap();
    assert!(dependency_commit.status.success());

    let inspect = Command::new(binary)
        .args(["task", "inspect"])
        .arg(&root)
        .arg(ROOT_ID)
        .output()
        .unwrap();
    let inspect: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    let revision = inspect["occurrences"][0]["revision"].as_str().unwrap();
    let recurring = serde_json::json!({
        "kind": "id",
        "id": "33333333-3333-4333-8333-333333333333"
    })
    .to_string();
    let context = serde_json::json!({
        "completedAt": {"kind": "date", "value": "2026-08-24"},
        "utcOffsetMinutes": 480
    })
    .to_string();
    let recurrence = Command::new(binary)
        .args(["task", "recurrence"])
        .arg(&root)
        .args([ROOT_ID, revision, &recurring, &context])
        .output()
        .unwrap();
    assert!(
        recurrence.status.success(),
        "{}",
        String::from_utf8_lossy(&recurrence.stderr)
    );
    let recurrence: serde_json::Value = serde_json::from_slice(&recurrence.stdout).unwrap();
    assert_eq!(recurrence["stopped"], false);
    assert!(recurrence["nextTaskId"].is_string());

    let recovery = Command::new(binary)
        .args(["task", "recover"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(recovery.status.success());
    let recovery: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(recovery["recovery"]["applyingRolledBack"], 0);
}

#[test]
fn cli_executes_one_exact_canonical_query_block() {
    const ROOT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
    const CHILD_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Query CLI");
    std::fs::create_dir_all(root.join("Child")).unwrap();
    std::fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
    std::fs::write(
        root.join("Query CLI.adoc"),
        format!("---\nweftext:\n  id: \"{ROOT_ID}\"\n---\n= Query CLI\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("Child/Child.adoc"),
        format!("---\nweftext:\n  id: \"{CHILD_ID}\"\n---\n= Child\n"),
    )
    .unwrap();
    let query_path = temporary.path().join("view.adoc");
    std::fs::write(
        &query_path,
        "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
    )
    .unwrap();
    let context = serde_json::json!({
        "today": {"year": 2026, "month": 8, "day": 24},
        "now": "2026-08-24T09:30:00+08:00",
        "timezone": "Asia/Shanghai",
        "locale": "zh-CN",
        "binding": {"nodeId": ROOT_ID, "heading": null},
    })
    .to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["query", "execute"])
        .arg(&root)
        .arg(&query_path)
        .args(["0", &context])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["execution"]["blockIndex"], 0);
    assert_eq!(
        payload["execution"]["analysis"]["blocks"][0]["view"],
        "table"
    );
    assert_eq!(payload["execution"]["result"]["totalBeforeLimit"], 2);
    assert_eq!(payload["execution"]["result"]["columns"][0]["path"], "name");
    assert_eq!(
        payload["execution"]["csv"],
        "name,path\r\nQuery CLI,/\r\nChild,/Child\r\n"
    );
    assert_eq!(
        payload["execution"]["result"]["rows"][0]["cells"][0]["value"]["value"],
        "Query CLI"
    );
    assert_eq!(
        payload["execution"]["result"]["rows"][1]["cells"][1]["value"]["value"],
        "/Child"
    );
}

fn bridge_request(
    address: &str,
    token: &str,
    method: &str,
    path: &str,
    body: Option<&serde_json::Value>,
) -> String {
    let body = body.map_or_else(String::new, serde_json::Value::to_string);
    let mut stream = TcpStream::connect(address).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nOrigin: https://weftext-webui-prototype.zhengyx91.chatgpt.site\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn response_json(response: &str) -> serde_json::Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    const fn new(child: Child) -> Self {
        Self { child }
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.stop();
    }
}
