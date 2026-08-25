use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use tempfile::TempDir;

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    workspace: PathBuf,
    snapshot_parent: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary fixture root");
        let root = std::fs::canonicalize(temporary.path()).expect("canonical temporary root");
        let workspace = root.join("资料库");
        let snapshot_parent = root.join("快照");
        std::fs::create_dir(&workspace).expect("workspace directory");
        std::fs::create_dir(&snapshot_parent).expect("snapshot parent");
        std::fs::write(workspace.join(".weftext-format"), b"weftext.asciidoc.v1\n")
            .expect("format marker");
        std::fs::write(
            workspace.join("资料库.adoc"),
            concat!(
                "---\n",
                "weftext:\n",
                "  id: \"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa\"\n",
                "---\n",
                "= 资料库\n\n",
                "完整备份验收。\n",
            ),
        )
        .expect("canonical document");
        std::fs::write(workspace.join("payload.bin"), [0_u8, 1, 2, 127, 128, 255])
            .expect("binary resource");
        Self {
            _temporary: temporary,
            root,
            workspace,
            snapshot_parent,
        }
    }

    fn preview(&self) -> Output {
        command(
            &["backup", "preview"],
            &[&self.workspace, &self.snapshot_parent],
        )
    }

    fn commit(&self, snapshot_id: &str, plan_digest: &str) -> Output {
        command(
            &["backup", "commit"],
            &[
                &self.workspace,
                &self.snapshot_parent,
                Path::new(snapshot_id),
                Path::new(plan_digest),
            ],
        )
    }

    fn commit_preview(&self) -> PathBuf {
        let preview = self.preview();
        assert_success(&preview);
        let preview = output_json(&preview);
        let snapshot_id = preview["backup"]["snapshotId"]
            .as_str()
            .expect("snapshot ID");
        let plan_digest = preview["backup"]["planDigest"]
            .as_str()
            .expect("backup plan digest");
        let committed = self.commit(snapshot_id, plan_digest);
        assert_success(&committed);
        only_child_directory(&self.snapshot_parent)
    }
}

fn prepare_scoped_restore_source(
    fixture: &Fixture,
) -> (weftext_core::NodeId, weftext_core::NodeId, String, PathBuf) {
    let source = weftext_core::create_child_node(&fixture.workspace, "Source").unwrap();
    std::fs::write(
        source.path.join("asset.bin"),
        b"exact-scoped-resource\0\xff",
    )
    .unwrap();
    let annotation_sidecar = weftext_core::AnnotationStore::empty(source.id)
        .to_pretty_json()
        .unwrap();
    std::fs::write(
        source.path.join("weftext.annotations.json"),
        &annotation_sidecar,
    )
    .unwrap();
    let nested = weftext_core::create_child_node(&source.path, "Nested").unwrap();
    (
        source.id,
        nested.id,
        annotation_sidecar,
        fixture.commit_preview(),
    )
}

#[test]
fn preview_commit_verify_and_alternate_restore_form_one_bound_command_chain() {
    let fixture = Fixture::new();
    let original = physical_files(&fixture.workspace);

    let preview = fixture.preview();
    assert_success(&preview);
    let preview = output_json(&preview);
    assert_eq!(preview["schema"], "weftext.cli.v1");
    assert_eq!(preview["ok"], true);
    assert_eq!(preview["backup"]["stage"], "preview");
    assert!(preview["backup"]["snapshotId"].is_string());
    assert!(preview["backup"]["planDigest"].is_string());
    assert_eq!(physical_files(&fixture.workspace), original);
    assert!(
        std::fs::read_dir(&fixture.snapshot_parent)
            .expect("read untouched snapshot parent")
            .next()
            .is_none(),
        "preview must not create a snapshot"
    );

    let snapshot_id = preview["backup"]["snapshotId"]
        .as_str()
        .expect("snapshot ID");
    let plan_digest = preview["backup"]["planDigest"]
        .as_str()
        .expect("backup plan digest");
    let commit = fixture.commit(snapshot_id, plan_digest);
    assert_success(&commit);
    let commit = output_json(&commit);
    assert_eq!(commit["backup"]["stage"], "committed");
    assert_eq!(commit["backup"]["snapshotId"], snapshot_id);
    assert_eq!(commit["backup"]["planDigest"], plan_digest);
    assert_eq!(physical_files(&fixture.workspace), original);

    let snapshot = only_child_directory(&fixture.snapshot_parent);
    let verified = command(&["backup", "verify"], &[&snapshot]);
    assert_success(&verified);
    assert_eq!(output_json(&verified)["backup"]["stage"], "verified");

    let restore_parent = fixture.root.join("恢复目标");
    std::fs::create_dir(&restore_parent).expect("restore parent");
    let restored = restore_parent.join("资料库");
    let dry_run = command(&["backup", "dry-run"], &[&snapshot, &restored]);
    assert_success(&dry_run);
    assert_eq!(output_json(&dry_run)["backup"]["stage"], "dry_run");
    assert!(
        !restored.exists(),
        "restore dry-run must not create its target"
    );

    let restore_preview = command(&["backup", "restore-preview"], &[&snapshot, &restored]);
    assert_success(&restore_preview);
    let restore_preview = output_json(&restore_preview);
    assert_eq!(restore_preview["backup"]["stage"], "restore_preview");
    assert!(!restored.exists(), "restore preview must be read-only");
    let restore_id = restore_preview["backup"]["restoreId"]
        .as_str()
        .expect("restore ID");
    let restore_digest = restore_preview["backup"]["planDigest"]
        .as_str()
        .expect("restore plan digest");

    let restore = command(
        &["backup", "restore-commit"],
        &[
            &snapshot,
            &restored,
            Path::new(restore_id),
            Path::new(restore_digest),
        ],
    );
    assert_success(&restore);
    assert_eq!(output_json(&restore)["backup"]["stage"], "restored");
    assert_eq!(physical_files(&restored), original);
}

#[test]
fn scoped_node_restore_commits_exact_resources_and_sidecar() {
    let fixture = Fixture::new();
    let (source_node_id, _, annotation_sidecar, snapshot) = prepare_scoped_restore_source(&fixture);

    let target = fixture.root.join("目标工作区");
    let target_root = weftext_core::create_workspace(&target).unwrap();
    let source_id = source_node_id.to_string();
    let target_parent_id = target_root.id.to_string();
    let restored_name = Path::new("Recovered");
    let preview = command(
        &["backup", "node-restore-preview"],
        &[
            &snapshot,
            &target,
            Path::new(&source_id),
            Path::new(&target_parent_id),
            restored_name,
        ],
    );
    assert_success(&preview);
    let preview = output_json(&preview);
    assert_eq!(preview["backup"]["stage"], "single_node_restore_preview");
    assert_eq!(preview["backup"]["commitState"], "ready");
    assert_eq!(preview["backup"]["blockers"], serde_json::json!([]));
    assert!(!target.join(restored_name).exists());
    let restore_id = preview["backup"]["restoreId"].as_str().unwrap();
    let plan_digest = preview["backup"]["planDigest"].as_str().unwrap();

    let committed = command(
        &["backup", "node-restore-commit"],
        &[
            &snapshot,
            &target,
            Path::new(&source_id),
            Path::new(&target_parent_id),
            restored_name,
            Path::new(restore_id),
            Path::new(plan_digest),
        ],
    );
    assert_success(&committed);
    let committed = output_json(&committed);
    assert_eq!(committed["backup"]["stage"], "single_node_restored");
    assert_eq!(committed["backup"]["receipt"]["exactBytesVerified"], true);
    assert_eq!(
        std::fs::read(target.join("Recovered/asset.bin")).unwrap(),
        b"exact-scoped-resource\0\xff"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("Recovered/weftext.annotations.json")).unwrap(),
        annotation_sidecar
    );
    assert!(!target.join("Recovered/Nested").exists());
}

#[test]
fn scoped_subtree_restore_preserves_every_id_resource_and_sidecar() {
    let fixture = Fixture::new();
    let (source_node_id, nested_node_id, annotation_sidecar, snapshot) =
        prepare_scoped_restore_source(&fixture);
    let source_id = source_node_id.to_string();

    let subtree_target = fixture.root.join("子树目标工作区");
    let subtree_root = weftext_core::create_workspace(&subtree_target).unwrap();
    let subtree_parent_id = subtree_root.id.to_string();
    let subtree_name = Path::new("RecoveredTree");
    let subtree_preview = command(
        &["backup", "subtree-restore-preview"],
        &[
            &snapshot,
            &subtree_target,
            Path::new(&source_id),
            Path::new(&subtree_parent_id),
            subtree_name,
        ],
    );
    assert_success(&subtree_preview);
    let subtree_preview = output_json(&subtree_preview);
    assert_eq!(
        subtree_preview["backup"]["stage"],
        "subtree_restore_preview"
    );
    assert_eq!(subtree_preview["backup"]["commitState"], "ready");
    assert_eq!(subtree_preview["backup"]["blockers"], serde_json::json!([]));
    assert_eq!(
        subtree_preview["backup"]["plan"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let restore_id = subtree_preview["backup"]["restoreId"].as_str().unwrap();
    let plan_digest = subtree_preview["backup"]["planDigest"].as_str().unwrap();
    let committed = command(
        &["backup", "subtree-restore-commit"],
        &[
            &snapshot,
            &subtree_target,
            Path::new(&source_id),
            Path::new(&subtree_parent_id),
            subtree_name,
            Path::new(restore_id),
            Path::new(plan_digest),
        ],
    );
    assert_success(&committed);
    let committed = output_json(&committed);
    assert_eq!(committed["backup"]["stage"], "subtree_restored");
    assert_eq!(committed["backup"]["receipt"]["exactBytesVerified"], true);
    assert_eq!(
        std::fs::read(subtree_target.join("RecoveredTree/asset.bin")).unwrap(),
        b"exact-scoped-resource\0\xff"
    );
    assert_eq!(
        std::fs::read_to_string(subtree_target.join("RecoveredTree/weftext.annotations.json"))
            .unwrap(),
        annotation_sidecar
    );
    assert!(
        subtree_target
            .join("RecoveredTree/Nested/Nested.adoc")
            .is_file()
    );
    let inventory = weftext_core::scan_workspace(&subtree_target);
    assert!(inventory.is_valid());
    let restored_ids = inventory
        .nodes
        .iter()
        .filter_map(|node| node.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(restored_ids.contains(&source_node_id));
    assert!(restored_ids.contains(&nested_node_id));
}

#[test]
fn restore_drill_preview_is_read_only_and_commit_records_verified_clean_restore() {
    let fixture = Fixture::new();
    let original = physical_files(&fixture.workspace);
    let snapshot = fixture.commit_preview();
    let drill_parent = fixture.root.join("演练恢复");
    let results_parent = fixture.root.join("演练记录");
    std::fs::create_dir(&drill_parent).unwrap();
    std::fs::create_dir(&results_parent).unwrap();

    let preview = command(
        &["backup", "drill-preview"],
        &[&snapshot, &drill_parent, &results_parent],
    );
    assert_success(&preview);
    let preview = output_json(&preview);
    assert_eq!(preview["backup"]["stage"], "drill_preview");
    assert_eq!(std::fs::read_dir(&drill_parent).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&results_parent).unwrap().count(), 0);
    let drill_id = preview["backup"]["drillId"].as_str().unwrap();
    let plan_digest = preview["backup"]["planDigest"].as_str().unwrap();

    let committed = command(
        &["backup", "drill-commit"],
        &[
            &snapshot,
            &drill_parent,
            &results_parent,
            Path::new(drill_id),
            Path::new(plan_digest),
        ],
    );
    assert_success(&committed);
    let committed = output_json(&committed);
    assert_eq!(committed["backup"]["stage"], "drill_completed");
    assert_eq!(committed["backup"]["receipt"]["openedClean"], true);
    assert_eq!(committed["backup"]["receipt"]["bytewiseVerified"], true);
    let restored = PathBuf::from(
        committed["backup"]["receipt"]["destinationRoot"]
            .as_str()
            .unwrap(),
    );
    let result = PathBuf::from(
        committed["backup"]["receipt"]["resultFile"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(physical_files(&restored), original);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(result).unwrap()).unwrap()["schema"],
        "weftext.full-workspace-restore-drill-result.v1",
    );
}

#[test]
fn snapshot_protection_is_visible_and_idempotent() {
    let fixture = Fixture::new();
    let snapshot = fixture.commit_preview();

    let protected = command(
        &["backup", "protect"],
        &[&snapshot, Path::new("release restore point")],
    );
    assert_success(&protected);
    let protected = output_json(&protected);
    assert_eq!(protected["backup"]["stage"], "protected");
    assert_eq!(
        protected["backup"]["protection"]["label"],
        "release restore point"
    );

    let repeated = command(
        &["backup", "protect"],
        &[&snapshot, Path::new("release restore point")],
    );
    assert_success(&repeated);
    assert_eq!(
        output_json(&repeated)["backup"]["protection"],
        protected["backup"]["protection"]
    );

    let verified = command(&["backup", "verify"], &[&snapshot]);
    assert_success(&verified);
    assert_eq!(
        output_json(&verified)["backup"]["verification"]["protection"]["label"],
        "release restore point"
    );
}

#[test]
fn retention_preview_commit_and_recovery_are_one_reviewed_chain() {
    let fixture = Fixture::new();
    let oldest = fixture.commit_preview();
    std::thread::sleep(Duration::from_millis(5));
    let preview = fixture.preview();
    assert_success(&preview);
    let preview = output_json(&preview);
    let newest_id = preview["backup"]["snapshotId"].as_str().unwrap();
    let backup_digest = preview["backup"]["planDigest"].as_str().unwrap();
    let committed = fixture.commit(newest_id, backup_digest);
    assert_success(&committed);
    let newest = fixture
        .snapshot_parent
        .join(format!("weftext-backup-{newest_id}"));

    let preview = command(
        &["backup", "retention-preview"],
        &[&fixture.snapshot_parent, Path::new("1")],
    );
    assert_success(&preview);
    let preview = output_json(&preview);
    assert_eq!(preview["backup"]["stage"], "retention_preview");
    assert_eq!(
        preview["backup"]["plan"]["pruned"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(oldest.exists(), "retention preview is read-only");
    assert!(newest.exists());
    let operation_id = preview["backup"]["operationId"].as_str().unwrap();
    let plan_digest = preview["backup"]["planDigest"].as_str().unwrap();

    let committed = command(
        &["backup", "retention-commit"],
        &[
            &fixture.snapshot_parent,
            Path::new("1"),
            Path::new(operation_id),
            Path::new(plan_digest),
        ],
    );
    assert_success(&committed);
    let committed = output_json(&committed);
    assert_eq!(committed["backup"]["stage"], "retention_committed");
    assert!(!oldest.exists());
    assert!(newest.exists());
    assert!(Path::new(committed["backup"]["plan"]["receiptFile"].as_str().unwrap()).is_file());

    let recovered = command(
        &["backup", "retention-recover"],
        &[&fixture.snapshot_parent],
    );
    assert_success(&recovered);
    assert_eq!(
        output_json(&recovered)["backup"]["stage"],
        "retention_recovered"
    );
}

#[test]
fn stale_digests_existing_targets_and_tampered_snapshots_fail_closed() {
    let fixture = Fixture::new();
    let preview = fixture.preview();
    assert_success(&preview);
    let preview = output_json(&preview);
    let snapshot_id = preview["backup"]["snapshotId"]
        .as_str()
        .expect("snapshot ID");
    let rejected = fixture.commit(snapshot_id, &"0".repeat(64));
    assert_failure(&rejected, "plan digest");
    assert!(
        std::fs::read_dir(&fixture.snapshot_parent)
            .expect("read snapshot parent")
            .next()
            .is_none(),
        "a rejected commit must not write"
    );

    let snapshot = fixture.commit_preview();
    let existing_parent = fixture.root.join("已存在目标");
    std::fs::create_dir(&existing_parent).expect("existing restore parent");
    let existing = existing_parent.join("资料库");
    std::fs::create_dir(&existing).expect("existing restore target");
    std::fs::write(existing.join("sentinel"), b"must survive").expect("target sentinel");
    let rejected = command(&["backup", "restore-preview"], &[&snapshot, &existing]);
    assert_failure(&rejected, "already exist");
    assert_eq!(
        std::fs::read(existing.join("sentinel")).expect("unchanged sentinel"),
        b"must survive"
    );

    let copied_payload = find_named_file(&snapshot, "payload.bin").expect("snapshot payload");
    std::fs::write(copied_payload, b"tampered").expect("tamper snapshot payload");
    let rejected = command(&["backup", "verify"], &[&snapshot]);
    assert_json_failure(&rejected);

    let absent_parent = fixture.root.join("不可恢复");
    std::fs::create_dir(&absent_parent).expect("tampered restore parent");
    let absent = absent_parent.join("资料库");
    let rejected = command(&["backup", "restore-preview"], &[&snapshot, &absent]);
    assert_json_failure(&rejected);
    assert!(!absent.exists());
}

fn command(prefix: &[&str], paths: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_weftext"));
    command.args(prefix);
    command.args(paths);
    command.output().expect("run backup CLI command")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output, message: &str) {
    assert_json_failure(output);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_json_failure(output: &Output) {
    assert!(!output.status.success());
    let payload = output_error_json(output);
    assert_eq!(payload["schema"], "weftext.cli.v1");
    assert_eq!(payload["ok"], false);
    assert!(payload["error"].is_string());
}

fn output_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("CLI JSON stdout")
}

fn output_error_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr).expect("CLI JSON stderr")
}

fn only_child_directory(parent: &Path) -> PathBuf {
    let mut directories = std::fs::read_dir(parent)
        .expect("read directory")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.is_dir());
    let only = directories.next().expect("one child directory");
    assert!(
        directories.next().is_none(),
        "only one snapshot is expected"
    );
    only
}

fn physical_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn collect(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = std::fs::read_dir(directory)
            .expect("read physical directory")
            .map(|entry| entry.expect("physical entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if directory == root
                && path
                    .file_name()
                    .is_some_and(|name| name == weftext_core::WORKSPACE_TRANSACTION_LEASE_FILE_NAME)
            {
                continue;
            }
            if path.is_dir() {
                collect(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .expect("relative file")
                        .to_path_buf(),
                    std::fs::read(path).expect("read physical file"),
                );
            }
        }
    }

    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

fn find_named_file(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find_named_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|file_name| file_name == name) {
            return Some(path);
        }
    }
    None
}
