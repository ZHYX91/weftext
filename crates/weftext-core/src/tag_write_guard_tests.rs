//! Original local Tags roles retain absence/birth eligibility across every
//! mutation and recovery path, not merely the specialized UpdateFiles API.
use super::*;
use std::collections::BTreeMap;

const TAGS: &str = ".weftext-tags.json";

fn fixture() -> (tempfile::TempDir, Core, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("Destination");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    (temp, core, root, data)
}

fn prepare_tag(core: &mut Core, root: &Path, name: &str) -> NavigationPreparation {
    let basis = core.read_tag_registry(root).unwrap();
    core.prepare_create_tag(root, name, &basis.base_id, &fresh())
        .unwrap()
}

fn commit(core: &mut Core, preview: &OperationPreview) {
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

fn commit_competing_registry_then_preserve_original(
    core: &mut Core,
    root: &Path,
    retained: &Path,
) -> Vec<u8> {
    let other = prepare_tag(core, root, "Committed elsewhere first");
    commit(core, &other.preview);
    let bytes = fs::read(root.join(TAGS)).unwrap();
    fs::rename(root.join(TAGS), retained).unwrap();
    bytes
}

fn mark_install(core: &Core, preview: &OperationPreview) {
    journal::private_create(
        &core.journal_path(&preview.operation_id, "install"),
        br#"{"testInterruption":true}"#,
    )
    .unwrap();
}

fn assert_fenced_recovery(core: &mut Core, preview: &OperationPreview) {
    let recovery = core.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "thirdState", "{recovery:?}");
    assert!(
        recovery
            .members
            .iter()
            .any(|member| member.kind == "tagRegistry"
                && !member.matches_before
                && !member.matches_after)
    );
    assert!(recovery.allowed_actions.is_empty());
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            core.resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
}

#[test]
fn absent_tag_plan_cannot_rebirth_after_competing_commit_loss_and_restart() {
    for installing in [false, true] {
        let (temp, mut core, root, data) = fixture();
        let original = prepare_tag(&mut core, &root, "Original pending tag");
        let retained = temp.path().join("retained-committed-tags.json");
        let committed_bytes =
            commit_competing_registry_then_preserve_original(&mut core, &root, &retained);
        if installing {
            mark_install(&core, &original.preview);
        }
        drop(core);
        let mut core = Core::new(data).unwrap();
        if installing {
            assert_fenced_recovery(&mut core, &original.preview);
        } else {
            assert_eq!(
                core.execute_operation(&original.preview.operation_id, &original.preview.plan_id)
                    .unwrap_err()
                    .code,
                "identityConflict"
            );
        }
        assert!(!root.join(TAGS).exists());
        assert_eq!(fs::read(retained).unwrap(), committed_bytes);
    }
}

fn native_package(core: &mut Core, directory: &Path) -> Vec<u8> {
    let root = directory.join("Origin");
    core.create_library(&root).unwrap();
    core.create_note_fixture(&root, &root, "Note").unwrap();
    let tag = prepare_tag(core, &root, "Imported tag");
    commit(core, &tag.preview);
    core.export_native_package(root).unwrap()
}

fn prepare_native(core: &mut Core, root: &Path, bytes: Vec<u8>) -> OperationPreview {
    core.prepare_native_import(root, root, "Imported", bytes, &fresh(), true)
        .unwrap()
}

#[test]
fn native_root_tag_update_cannot_rebirth_after_competing_commit_loss_and_restart() {
    for installing in [false, true] {
        let (temp, mut core, root, data) = fixture();
        let bytes = native_package(&mut core, temp.path());
        let original = prepare_native(&mut core, &root, bytes);
        assert!(
            matches!(&core.operations[&original.operation_id].transaction.as_ref().unwrap().mutation,
            operations::Mutation::CreateTree { updates, .. } if updates.iter().any(|change| Path::new(&change.path) == root.join(TAGS) && change.before.is_none()))
        );
        let retained = temp.path().join("retained-committed-tags.json");
        let committed_bytes =
            commit_competing_registry_then_preserve_original(&mut core, &root, &retained);
        if installing {
            mark_install(&core, &original);
        }
        drop(core);
        let mut core = Core::new(data).unwrap();
        if installing {
            assert_fenced_recovery(&mut core, &original);
        } else {
            assert_eq!(
                core.execute_operation(&original.operation_id, &original.plan_id)
                    .unwrap_err()
                    .code,
                "identityConflict"
            );
        }
        assert!(!root.join(TAGS).exists());
        assert!(!root.join("Imported").exists());
        assert_eq!(fs::read(retained).unwrap(), committed_bytes);
    }
}

fn materialize_change(core: &Core, operation: &str, change: &FileChange) {
    let path = Path::new(&change.path);
    let guard = ParentGuard::acquire(path).unwrap();
    let mut file = guard.open_file(true, change.before.is_none()).unwrap();
    if change.before.is_none() {
        core.record_materialized(operation, path, &file).unwrap();
    }
    operations::write_blob(&mut file, &change.after).unwrap();
}

fn materialize_original(core: &Core, preview: &OperationPreview) {
    mark_install(core, preview);
    let transaction = core.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    match &transaction.mutation {
        operations::Mutation::Update { files } => {
            for file in files {
                materialize_change(core, &preview.operation_id, file);
            }
        }
        operations::Mutation::CreateTree {
            target,
            directories,
            files,
            updates,
            existing_target,
            ..
        } => {
            assert!(!existing_target);
            let target = Path::new(target);
            let guard = ParentGuard::acquire(target).unwrap();
            let directory = guard.create_directory().unwrap();
            core.record_materialized(&preview.operation_id, target, &directory)
                .unwrap();
            let mut handles = BTreeMap::from([(PathBuf::new(), directory)]);
            let mut directories = directories.iter().map(PathBuf::from).collect::<Vec<_>>();
            directories.sort_by_key(|path| path.components().count());
            for relative in directories {
                let parent = &handles[relative.parent().unwrap_or_else(|| Path::new(""))];
                let directory = ParentGuard::child_directory(
                    parent,
                    relative.file_name().unwrap().to_str().unwrap(),
                )
                .unwrap();
                core.record_materialized(
                    &preview.operation_id,
                    &target.join(&relative),
                    &directory,
                )
                .unwrap();
                handles.insert(relative, directory);
            }
            for change in files {
                let relative = Path::new(&change.path).strip_prefix(target).unwrap();
                let parent = &handles[relative.parent().unwrap_or_else(|| Path::new(""))];
                let mut file = ParentGuard::child_file(
                    parent,
                    relative.file_name().unwrap().to_str().unwrap(),
                )
                .unwrap();
                core.record_materialized(&preview.operation_id, Path::new(&change.path), &file)
                    .unwrap();
                operations::write_blob(&mut file, &change.after).unwrap();
            }
            for change in updates {
                materialize_change(core, &preview.operation_id, change);
            }
        }
        _ => unreachable!(),
    }
}

#[test]
fn original_created_tag_after_image_recovers_with_its_own_materialized_identity() {
    let (_temp, mut core, root, data) = fixture();
    let original = prepare_tag(&mut core, &root, "Original tag");
    materialize_original(&core, &original.preview);
    let before_identity = operations::capture_optional(&root.join(TAGS))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    drop(core);
    let mut core = Core::new(data).unwrap();
    let recovery = core
        .inspect_recovery(&original.preview.operation_id)
        .unwrap();
    assert_eq!(recovery.coherence, "knownAfter", "{recovery:?}");
    core.resolve_recovery(&original.preview.operation_id, "adoptAfterImages")
        .unwrap();
    assert_eq!(
        core.read_tag_registry(&root).unwrap().tags[0].tag_ref,
        original.target_ref
    );
    assert_eq!(
        operations::capture_optional(&root.join(TAGS))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        before_identity
    );
}

#[test]
fn original_native_tags_and_tree_after_images_recover_without_new_identities() {
    let (temp, mut core, root, data) = fixture();
    let bytes = native_package(&mut core, temp.path());
    let original = prepare_native(&mut core, &root, bytes);
    materialize_original(&core, &original);
    let before = fs::read(root.join(TAGS)).unwrap();
    let before_identity = operations::capture_optional(&root.join(TAGS))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    drop(core);
    let mut core = Core::new(data).unwrap();
    let recovery = core.inspect_recovery(&original.operation_id).unwrap();
    assert_eq!(recovery.coherence, "knownAfter", "{recovery:?}");
    core.resolve_recovery(&original.operation_id, "adoptAfterImages")
        .unwrap();
    assert!(core.read_tag_registry(&root).unwrap().complete);
    assert_eq!(core.read_tag_registry(&root).unwrap().tags.len(), 1);
    assert_eq!(fs::read(root.join(TAGS)).unwrap(), before);
    assert_eq!(
        operations::capture_optional(&root.join(TAGS))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        before_identity
    );
}

#[test]
fn native_first_tag_creation_still_executes_original_plan_after_restart() {
    let (temp, mut core, root, data) = fixture();
    let bytes = native_package(&mut core, temp.path());
    let original = prepare_native(&mut core, &root, bytes);
    drop(core);
    let mut core = Core::new(data).unwrap();
    commit(&mut core, &original);
    assert!(core.read_tag_registry(&root).unwrap().complete);
    assert_eq!(core.read_tag_registry(&root).unwrap().tags.len(), 1);
    assert!(root.join("Imported").is_dir());
}

#[test]
fn sync_tag_successors_require_v2_without_local_allocations() {
    let (_temp, core, root, _data) = fixture();
    let tag = fresh();
    for (version, accepted) in [(1, false), (2, true)] {
        let record = if version == 1 {
            serde_json::json!({"tagId":tag,"displayName":"Received"})
        } else {
            serde_json::json!({"tagId":tag,"displayName":"Received","parentTagId":null})
        };
        let mutation = operations::Mutation::Update {
            files: vec![FileChange {
                path: display(&root.join(TAGS)),
                base_id: None,
                before: None,
                after: serde_json::to_vec(&serde_json::json!({"version":version,"tags":[record]}))
                    .unwrap(),
            }],
        };
        assert_eq!(
            core.validate_tag_mutation_changes(
                &root,
                &fresh(),
                "unused-sync-fingerprint",
                &mutation,
                true
            )
            .is_ok(),
            accepted
        );
    }
}
