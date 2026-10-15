use super::*;
use crate::operations::{capture_optional, write_blob};
use std::fs;

struct Fixture {
    _temp: tempfile::TempDir,
    first: Core,
    second: Core,
    source: PathBuf,
    target: PathBuf,
    data: PathBuf,
    identity: SyncIdentity,
    objects: BTreeSet<ObjectKey>,
    parent: String,
    child: String,
    other: String,
    live: String,
}

fn execute(core: &mut Core, root: &Path, intent: OperationIntent) -> OperationReceipt {
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(root),
            intent,
        })
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap()
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("receiver");
    let data = temp.path().join("receiver-private");
    let mut first = Core::new(temp.path().join("source-private")).unwrap();
    first.create_library(&source).unwrap();
    let parent = first.create_note(&source, &source, "Parent").unwrap();
    let child = first
        .create_note(&source, source.join("Parent"), "Child")
        .unwrap();
    let other = first.create_note(&source, &source, "Other").unwrap();
    let live = first.create_note(&source, &source, "Live").unwrap();
    let resource = first
        .prepare_import_resource(
            &source,
            &child.path,
            "original.bin",
            vec![0, 255, 7],
            &fresh(),
        )
        .unwrap();
    first
        .execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    let info = first.enable_sync(&source, &fresh()).unwrap();
    for name in ["Parent", "Other"] {
        execute(
            &mut first,
            &source,
            OperationIntent::TrashNode {
                source: display(&source.join(name)),
            },
        );
    }
    first.publish_sync(&source).unwrap();
    let objects = info.installed_objects.into_iter().collect();
    let second = Core::new(&data).unwrap();
    second
        .receive_sync(
            &info.identity,
            first.export_sync_objects(&info.identity, &objects).unwrap(),
        )
        .unwrap();
    Fixture {
        _temp: temp,
        first,
        second,
        source,
        target,
        data,
        identity: info.identity,
        objects,
        parent: parent.node_id.unwrap(),
        child: child.node_id.unwrap(),
        other: other.node_id.unwrap(),
        live: live.node_id.unwrap(),
    }
}

fn request(f: &Fixture) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(&f.target),
        intent: OperationIntent::CloneSync {
            identity: f.identity.clone(),
            objects: f.objects.clone(),
        },
    }
}

fn assert_installed(f: &mut Fixture, operation_id: &str) {
    let setup = f.second.sync_setup_for(&f.target).unwrap().clone();
    let (local, _) = f.second.sync_local_state(&setup).unwrap();
    let graph = f.first.read_sync_history(&f.identity).unwrap();
    let versions = graph.objects().unwrap();
    for key in &f.objects {
        assert_eq!(local[key].state, versions[key].versions[0].state);
        assert_eq!(local[key].revision, versions[key].versions[0].revision);
        for (name, digest) in &local[key].state.files {
            let file = capture_optional(&Path::new(&local[key].directory).join(name))
                .unwrap()
                .unwrap();
            assert_eq!(hash(&file.bytes), *digest);
            assert_eq!(
                f.second.accepted_revision(&file).as_ref(),
                local[key].state.file_revisions.get(name)
            );
            assert_eq!(
                f.second
                    .materialized_identity(operation_id, Path::new(&file.path))
                    .unwrap(),
                Some(file.stamp.identity)
            );
        }
    }
    let parent = &local[&ObjectKey::Node(f.parent.clone())];
    let child = &local[&ObjectKey::Node(f.child.clone())];
    let other = &local[&ObjectKey::Node(f.other.clone())];
    assert!(Path::new(&parent.directory).starts_with(f.target.join(".weftext-trash")));
    assert!(Path::new(&other.directory).starts_with(f.target.join(".weftext-trash")));
    assert_ne!(parent.directory, other.directory);
    assert_eq!(
        Path::new(&child.directory),
        Path::new(&parent.directory).join("Child")
    );
    assert!(
        !child.state.retired,
        "inherited lifecycle must not rewrite the received state"
    );
    assert!(!f.target.join("Parent").exists());
    assert!(!f.target.join("Other").exists());
    let live = f.second.admitted_snapshot(&f.target).unwrap();
    assert!(live.complete);
    assert_eq!(live.membership_ids, vec![f.live.clone()]);
    let trash = f.second.trash_admitted_snapshot(&f.target).unwrap();
    assert!(trash.complete);
    assert_eq!(
        trash.membership_ids.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([f.parent.clone(), f.child.clone(), f.other.clone()])
    );
    let entries = f.second.trash_entries(&f.target).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .all(|entry| entry.operation_id == operation_id)
    );
    let owner = weftext_knowledge::NodeRef {
        workspace_id: f.identity.workspace_id.clone(),
        node_id: f.child.clone(),
    };
    f.second
        .qualified_query_node_at(
            &f.target,
            &Path::new(&child.directory).join("Child.typ"),
            &owner,
            "trash",
        )
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(Path::new(&child.directory).join("weftext.resources.json")).unwrap(),
    )
    .unwrap();
    let resource = &manifest["resources"][0];
    let reference = format!(
        "weftext-resource://{}/{}",
        f.identity.workspace_id,
        resource["resourceId"].as_str().unwrap()
    );
    let selected = f
        .second
        .qualified_resource_at(
            &f.target,
            &Path::new(&child.directory).join("Child.typ"),
            &owner,
            &reference,
            &Path::new(&child.directory).join(resource["path"].as_str().unwrap()),
            "trash",
            None,
        )
        .unwrap();
    assert_eq!(selected.bytes(), [0, 255, 7]);
}

#[test]
fn retired_clone_installs_multiple_original_roots_and_inherited_children_with_exact_versions() {
    let mut f = fixture();
    let req = request(&f);
    let preview = f.second.prepare_operation(req.clone()).unwrap();
    assert!(!f.target.exists());
    assert!(
        preview
            .changes
            .iter()
            .any(|change| Path::new(&change.path).starts_with(f.target.join(".weftext-trash")))
    );
    drop(f.second);
    f.second = Core::new(&f.data).unwrap();
    assert_eq!(
        f.second.prepare_operation(req.clone()).unwrap().plan_id,
        preview.plan_id
    );
    f.second
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_installed(&mut f, &preview.operation_id);
    drop(f.second);
    f.second = Core::new(&f.data).unwrap();
    assert_installed(&mut f, &preview.operation_id);
    assert_eq!(
        f.second
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .plan_id,
        preview.plan_id
    );
    assert_eq!(
        f.second.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
    assert!(f.source.join("Live/Live.typ").exists());
}

#[test]
fn retired_clone_target_occupancy_after_prepared_restart_has_no_effect() {
    let mut f = fixture();
    let preview = f.second.prepare_operation(request(&f)).unwrap();
    drop(f.second);
    f.second = Core::new(&f.data).unwrap();
    fs::create_dir(&f.target).unwrap();
    fs::write(f.target.join("foreign.raw"), b"original foreign bytes").unwrap();
    assert!(
        f.second
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !f.second
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert_eq!(
        fs::read(f.target.join("foreign.raw")).unwrap(),
        b"original foreign bytes"
    );
    assert!(!f.target.join(".weftext-trash").exists());
}

// Reproduce the original CreateTree's exact install/materialization boundary,
// retaining its guarded physical objects and its unchanged serialized Plan.
fn materialize_original(f: &Fixture, preview: &OperationPreview) {
    let plan = &f.second.operations[&preview.operation_id];
    let Mutation::CreateTree {
        target,
        directories,
        files,
        ..
    } = &plan.transaction.as_ref().unwrap().mutation
    else {
        panic!("original CreateTree required")
    };
    f.second.install_record(plan).unwrap();
    let guard = ParentGuard::acquire(Path::new(target)).unwrap();
    let directory = guard.create_directory().unwrap();
    f.second
        .record_materialized(&preview.operation_id, Path::new(target), &directory)
        .unwrap();
    let mut handles = BTreeMap::from([(PathBuf::new(), directory)]);
    let mut dirs = directories.iter().map(PathBuf::from).collect::<Vec<_>>();
    dirs.sort_by_key(|path| path.components().count());
    for relative in dirs {
        let parent = &handles[relative.parent().unwrap_or_else(|| Path::new(""))];
        let child =
            ParentGuard::child_directory(parent, relative.file_name().unwrap().to_str().unwrap())
                .unwrap();
        f.second
            .record_materialized(
                &preview.operation_id,
                &Path::new(target).join(&relative),
                &child,
            )
            .unwrap();
        handles.insert(relative, child);
    }
    for change in files {
        let relative = Path::new(&change.path).strip_prefix(target).unwrap();
        let parent = &handles[relative.parent().unwrap_or_else(|| Path::new(""))];
        let mut file =
            ParentGuard::child_file(parent, relative.file_name().unwrap().to_str().unwrap())
                .unwrap();
        f.second
            .record_materialized(&preview.operation_id, Path::new(&change.path), &file)
            .unwrap();
        write_blob(&mut file, &change.after).unwrap();
    }
}

#[test]
fn retired_clone_original_known_after_recovery_admits_the_same_roots_and_file_objects() {
    let mut f = fixture();
    let preview = f.second.prepare_operation(request(&f)).unwrap();
    materialize_original(&f, &preview);
    let original = fs::read(f.second.journal_path(&preview.operation_id, "plan")).unwrap();
    drop(f.second);
    f.second = Core::new(&f.data).unwrap();
    let recovery = f.second.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "knownAfter");
    assert!(
        recovery
            .allowed_actions
            .iter()
            .any(|action| action == "adoptAfterImages")
    );
    f.second
        .resolve_recovery(&preview.operation_id, "adoptAfterImages")
        .unwrap();
    assert_installed(&mut f, &preview.operation_id);
    assert_eq!(
        fs::read(f.second.journal_path(&preview.operation_id, "plan")).unwrap(),
        original
    );
}

fn restore_request(f: &Fixture, clone: &str, node_id: Option<&str>) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(&f.target),
        intent: OperationIntent::RestoreNode {
            trash_operation_id: clone.into(),
            node_id: node_id.map(str::to_owned),
            parent: display(&f.target),
            name: "Restored".into(),
        },
    }
}

#[test]
fn retired_clone_restore_requires_exact_root_then_preserves_files_history_and_sync_predecessor() {
    let mut f = fixture();
    let preview = f.second.prepare_operation(request(&f)).unwrap();
    f.second
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let setup = f.second.sync_setup_for(&f.target).unwrap().clone();
    let (local, _) = f.second.sync_local_state(&setup).unwrap();
    let child_path =
        Path::new(&local[&ObjectKey::Node(f.child.clone())].directory).join("Child.typ");
    let child_before = capture_optional(&child_path).unwrap().unwrap();
    let parent_before = capture_optional(
        &Path::new(&local[&ObjectKey::Node(f.parent.clone())].directory).join("Parent.typ"),
    )
    .unwrap()
    .unwrap();
    let resource_before = capture_optional(&child_path.parent().unwrap().join("original.bin"))
        .unwrap()
        .unwrap();
    let opened = f.second.open(&child_path, Some(f.target.clone())).unwrap();
    let history = f.second.list_history(&opened.session_id).unwrap().versions;
    let ambiguous = restore_request(&f, &preview.operation_id, None);
    assert_eq!(
        f.second
            .prepare_operation(ambiguous.clone())
            .unwrap_err()
            .code,
        "trashSelectionRequired"
    );
    assert!(!f.second.operations.contains_key(&ambiguous.operation_id));
    for wrong in [&f.child, &f.live, &fresh()] {
        let wrong = restore_request(&f, &preview.operation_id, Some(wrong));
        assert_eq!(
            f.second.prepare_operation(wrong.clone()).unwrap_err().code,
            "trashSelectionMismatch"
        );
        assert!(!f.second.operations.contains_key(&wrong.operation_id));
    }
    let req = restore_request(&f, &preview.operation_id, Some(&f.parent));
    let restore = f.second.prepare_operation(req.clone()).unwrap();
    drop(f.second);
    f.second = Core::new(&f.data).unwrap();
    assert_eq!(
        f.second.prepare_operation(req).unwrap().plan_id,
        restore.plan_id
    );
    f.second
        .execute_operation(&restore.operation_id, &restore.plan_id)
        .unwrap();
    let child_after = capture_optional(&f.target.join("Restored/Child/Child.typ"))
        .unwrap()
        .unwrap();
    let parent_after = capture_optional(&f.target.join("Restored/Restored.typ"))
        .unwrap()
        .unwrap();
    assert_eq!(child_after.stamp.identity, child_before.stamp.identity);
    assert_eq!(parent_after.stamp.identity, parent_before.stamp.identity);
    assert_eq!(child_after.bytes, child_before.bytes);
    assert_eq!(parent_after.bytes, parent_before.bytes);
    let resources = f
        .second
        .managed_resource_snapshot(&f.target, f.target.join("Restored/Child/Child.typ"))
        .unwrap();
    assert_eq!(
        capture_optional(Path::new(&resources[0].path))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        resource_before.stamp.identity
    );
    assert_eq!(resources[0].bytes, resource_before.bytes);
    let opened = f
        .second
        .open(
            f.target.join("Restored/Child/Child.typ"),
            Some(f.target.clone()),
        )
        .unwrap();
    assert_eq!(
        f.second
            .list_history(&opened.session_id)
            .unwrap()
            .versions
            .iter()
            .map(|version| &version.version_id)
            .collect::<Vec<_>>(),
        history
            .iter()
            .map(|version| &version.version_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(f.second.trash_entries(&f.target).unwrap().len(), 1);
    assert_eq!(
        f.second.trash_entries(&f.target).unwrap()[0].node_id,
        f.other
    );
    assert_eq!(
        f.second
            .trash_admitted_snapshot(&f.target)
            .unwrap()
            .membership_ids,
        vec![f.other.clone()]
    );
    assert_eq!(
        f.second
            .admitted_snapshot(&f.target)
            .unwrap()
            .membership_ids
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([f.parent.clone(), f.child.clone(), f.live.clone()])
    );
    f.second.publish_sync(&f.target).unwrap();
    let graph = f.second.read_sync_history(&f.identity).unwrap();
    let object = ObjectKey::Node(f.parent.clone());
    let manifest = graph
        .manifests
        .values()
        .find(|manifest| manifest.operation_id == restore.operation_id)
        .unwrap();
    let change = manifest
        .changes
        .iter()
        .find(|change| change.object == object)
        .unwrap();
    assert_eq!(
        change.before,
        BTreeSet::from([local[&object].revision.clone()])
    );
    assert_eq!(
        change.after.file_revisions["Restored.typ"],
        local[&object].state.file_revisions["Parent.typ"]
    );
    assert!(!change.after.retired);
    f.first
        .receive_sync(
            &f.identity,
            f.second
                .export_sync_objects(&f.identity, &BTreeSet::from([object.clone()]))
                .unwrap(),
        )
        .unwrap();
    execute(
        &mut f.first,
        &f.source,
        OperationIntent::ApplySync {
            identity: f.identity.clone(),
            objects: BTreeSet::from([object]),
        },
    );
    assert_eq!(
        fs::read(f.source.join("Restored/Child/Child.typ")).unwrap(),
        child_before.bytes
    );
    assert_eq!(f.first.trash_entries(&f.source).unwrap().len(), 1);
}

fn copy_raw_tree(source: &Path, target: &Path) {
    fs::create_dir(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_raw_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[test]
fn retired_clone_same_bytes_replacement_never_supplies_trash_or_recovery_ownership() {
    for committed in [false, true] {
        let mut f = fixture();
        let preview = f.second.prepare_operation(request(&f)).unwrap();
        if committed {
            f.second
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
        } else {
            materialize_original(&f, &preview);
        }
        let plan = &f.second.operations[&preview.operation_id];
        let directory = PathBuf::from(
            &plan
                .sync_setup
                .as_ref()
                .unwrap()
                .initial
                .iter()
                .find(|object| object.object == ObjectKey::Node(f.parent.clone()))
                .unwrap()
                .directory,
        );
        let original_plan = fs::read(f.second.journal_path(&preview.operation_id, "plan")).unwrap();
        let original_install =
            fs::read(f.second.journal_path(&preview.operation_id, "install")).unwrap();
        let retained = f.target.parent().unwrap().join("retained-original-tree");
        fs::rename(&directory, &retained).unwrap();
        copy_raw_tree(&retained, &directory);
        drop(f.second);
        f.second = Core::new(&f.data).unwrap();
        if committed {
            assert_eq!(
                f.second.trash_entries(&f.target).unwrap_err().code,
                "identityConflict"
            );
            assert_eq!(
                f.second
                    .trash_admitted_snapshot(&f.target)
                    .unwrap_err()
                    .code,
                "scopeChanged"
            );
            let req = restore_request(&f, &preview.operation_id, Some(&f.parent));
            assert!(f.second.prepare_operation(req.clone()).is_err());
            assert!(!f.second.operations.contains_key(&req.operation_id));
        } else {
            let recovery = f.second.inspect_recovery(&preview.operation_id).unwrap();
            assert!(recovery.allowed_actions.is_empty());
            for action in ["keepCurrent", "adoptAfterImages", "resumeOriginalPlan"] {
                assert!(
                    f.second
                        .resolve_recovery(&preview.operation_id, action)
                        .is_err()
                );
            }
            assert!(!f.second.committed(&preview.operation_id));
        }
        assert_eq!(
            fs::read(f.second.journal_path(&preview.operation_id, "plan")).unwrap(),
            original_plan
        );
        assert_eq!(
            fs::read(f.second.journal_path(&preview.operation_id, "install")).unwrap(),
            original_install
        );
        assert_eq!(
            fs::read(retained.join("Parent.typ")).unwrap(),
            fs::read(directory.join("Parent.typ")).unwrap()
        );
    }
}

fn clone_preexisting_trash(nested: bool) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("receiver");
    let mut first = Core::new(temp.path().join("source-private")).unwrap();
    first.create_library(&source).unwrap();
    let parent = first.create_note(&source, &source, "Parent").unwrap();
    let parent_id = parent.node_id.unwrap();
    let child = nested.then(|| {
        first
            .create_note(&source, source.join("Parent"), "Child")
            .unwrap()
    });
    let live = first.create_note(&source, &source, "Live").unwrap();
    let owner_source = child
        .as_ref()
        .map_or(parent.path.as_str(), |child| child.path.as_str());
    let resource = first
        .prepare_import_resource(
            &source,
            owner_source,
            "original.bin",
            vec![9, 0, 255],
            &fresh(),
        )
        .unwrap();
    first
        .execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    let parent_before = capture_optional(Path::new(&parent.path)).unwrap().unwrap();
    let parent_revision = first.accepted_revision(&parent_before).unwrap();
    let child_before = child
        .as_ref()
        .map(|child| capture_optional(Path::new(&child.path)).unwrap().unwrap());
    let trash = execute(
        &mut first,
        &source,
        OperationIntent::TrashNode {
            source: display(&source.join("Parent")),
        },
    );
    let original_tree = source.join(".weftext-trash").join(&trash.operation_id);
    let original_root = ParentGuard::acquire(&original_tree.join(".original-root-proof"))
        .unwrap()
        .directory_identity()
        .unwrap();
    let enable = fresh();
    let info = first.enable_sync(&source, &enable).unwrap();
    let setup = first.sync_setup_for(&source).unwrap().clone();
    let graph = first.read_sync_history(&info.identity).unwrap();
    for object in &setup.initial {
        assert_eq!(
            graph.status(&object.commit).unwrap(),
            Admission::Accepted,
            "preexisting content must have a causally valid initial checkpoint: {:?}",
            object.object
        );
    }
    let parent_object = ObjectKey::Node(parent_id.clone());
    let versions = graph.objects().unwrap();
    let parent_state = &versions[&parent_object].versions[0].state;
    assert!(parent_state.retired);
    assert_eq!(parent_state.file_revisions["Parent.typ"], parent_revision);
    if let Some(child) = &child {
        let child_state =
            &versions[&ObjectKey::Node(child.node_id.clone().unwrap())].versions[0].state;
        assert_eq!(child_state.parent, Some(parent_object.clone()));
        assert!(
            !child_state.retired,
            "a child carried by the original Trash tree is not an independently retired root"
        );
    }
    assert_eq!(
        capture_optional(&original_tree.join("Parent.typ"))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        parent_before.stamp.identity
    );
    assert_eq!(
        ParentGuard::acquire(&original_tree.join(".original-root-proof"))
            .unwrap()
            .directory_identity()
            .unwrap(),
        original_root
    );
    let keys = info.installed_objects;
    let data = temp.path().join("receiver-private");
    let mut second = Core::new(&data).unwrap();
    second
        .receive_sync(
            &info.identity,
            first.export_sync_objects(&info.identity, &keys).unwrap(),
        )
        .unwrap();
    let req = OperationRequest {
        operation_id: fresh(),
        root: display(&target),
        intent: OperationIntent::CloneSync {
            identity: info.identity.clone(),
            objects: keys,
        },
    };
    let preview = second.prepare_operation(req.clone()).unwrap();
    second
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    drop(second);
    let mut second = Core::new(&data).unwrap();
    assert_eq!(
        second.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
    let entries = second.trash_entries(&target).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node_id, parent_id);
    let cloned_root = Path::new(&entries[0].path);
    let parent_after = capture_optional(&cloned_root.join("Parent.typ"))
        .unwrap()
        .unwrap();
    assert_eq!(parent_after.bytes, parent_before.bytes);
    assert_ne!(parent_after.stamp.identity, parent_before.stamp.identity);
    assert_eq!(
        second.accepted_revision(&parent_after).as_ref(),
        Some(&parent_revision)
    );
    let mut trash_nodes = BTreeSet::from([parent_id]);
    if let Some(child) = &child {
        let child_after = capture_optional(&cloned_root.join("Child/Child.typ"))
            .unwrap()
            .unwrap();
        assert_eq!(child_after.bytes, child_before.as_ref().unwrap().bytes);
        assert_ne!(
            child_after.stamp.identity,
            child_before.as_ref().unwrap().stamp.identity
        );
        trash_nodes.insert(child.node_id.clone().unwrap());
    }
    let child_directory = if nested {
        cloned_root.join("Child")
    } else {
        cloned_root.to_path_buf()
    };
    assert_eq!(
        fs::read(child_directory.join("original.bin")).unwrap(),
        vec![9, 0, 255]
    );
    assert_eq!(
        second
            .trash_admitted_snapshot(&target)
            .unwrap()
            .membership_ids
            .into_iter()
            .collect::<BTreeSet<_>>(),
        trash_nodes
    );
    assert_eq!(
        second.admitted_snapshot(&target).unwrap().membership_ids,
        vec![live.node_id.unwrap()]
    );
}

#[test]
fn retired_clone_before_enable_single_root_has_valid_checkpoint_and_original_file_revision() {
    clone_preexisting_trash(false);
}

#[test]
fn retired_clone_before_enable_subtree_keeps_original_parent_and_inherited_lifecycle() {
    clone_preexisting_trash(true);
}
