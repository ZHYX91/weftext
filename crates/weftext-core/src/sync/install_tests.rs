use super::*;
use crate::SaveRequest;
use std::fs;

fn edit(core: &mut Core, root: &Path, text: &str) -> String {
    let note = core
        .open(root.join("Note/Note.typ"), Some(root.into()))
        .unwrap();
    let source = format!("{}\n{text}", note.source);
    core.save(SaveRequest {
        session_id: note.session_id,
        base_id: note.base_id,
        source: source.clone(),
        operation_id: fresh(),
    })
    .unwrap();
    source
}
fn request(root: &Path, identity: &SyncIdentity, objects: BTreeSet<ObjectKey>) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent: OperationIntent::CloneSync {
            identity: identity.clone(),
            objects,
        },
    }
}

#[test]
fn actual_independent_partial_replica_saves_offline_and_retains_both_conflict_branches() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("first-library");
    let b = temp.path().join("second-library");
    let mut first = Core::new(temp.path().join("first-private")).unwrap();
    first.create_library(&a).unwrap();
    let note = first.create_note(&a, &a, "Note").unwrap();
    first.create_note(&a, &a, "NotSelected").unwrap();
    let info = first.enable_sync(&a, &fresh()).unwrap();
    let selected = BTreeSet::from([ObjectKey::Node(note.node_id.clone().unwrap())]);
    let second_data = temp.path().join("second-private");
    let mut second = Core::new(&second_data).unwrap();
    second
        .receive_sync(
            &info.identity,
            first
                .export_sync_objects(&info.identity, &selected)
                .unwrap(),
        )
        .unwrap();
    let input = request(&b, &info.identity, selected.clone());
    let preview = second.prepare_operation(input.clone()).unwrap();
    assert!(!b.exists());
    assert_eq!(
        second
            .prepare_operation(request(
                &temp.path().join("another-replica"),
                &info.identity,
                selected.clone()
            ))
            .unwrap_err()
            .code,
        "syncSetupPending"
    );
    let receipt = second
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        second
            .with_actor("unprivileged", |c| c
                .execute_operation(&preview.operation_id, &preview.plan_id))
            .unwrap_err()
            .code,
        "syncOwnerRequired"
    );
    assert_eq!(
        second
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .operation_id,
        receipt.operation_id
    );
    assert_eq!(
        second.prepare_operation(input).unwrap().plan_id,
        preview.plan_id
    );
    let opened = second
        .open(b.join("Note/Note.typ"), Some(b.clone()))
        .unwrap();
    assert_eq!(opened.node_id, note.node_id);
    assert_eq!(
        opened.workspace_id.as_deref(),
        Some(info.identity.workspace_id.as_str())
    );
    assert_eq!(opened.source, note.source);
    assert!(!b.join("NotSelected").exists());
    assert!(second.sync_binding_info(&b).unwrap().partial);
    assert_ne!(
        second.sync_binding_info(&b).unwrap().replica_id,
        info.replica_id
    );
    let a_source = edit(&mut first, &a, "first offline branch");
    let b_source = edit(&mut second, &b, "second offline branch");
    drop(second);
    let second = Core::new(&second_data).unwrap();
    first.publish_sync(&a).unwrap();
    second.publish_sync(&b).unwrap();
    first
        .receive_sync(
            &info.identity,
            second
                .export_sync_objects(&info.identity, &selected)
                .unwrap(),
        )
        .unwrap();
    second
        .receive_sync(
            &info.identity,
            first
                .export_sync_objects(&info.identity, &selected)
                .unwrap(),
        )
        .unwrap();
    let first_graph = first.read_sync_history(&info.identity).unwrap();
    let second_graph = second.read_sync_history(&info.identity).unwrap();
    let key = selected.first().unwrap();
    let first_objects = first_graph.objects().unwrap();
    let second_objects = second_graph.objects().unwrap();
    assert_eq!(first_objects[key], second_objects[key]);
    assert_eq!(first_objects[key].versions.len(), 2);
    assert_eq!(first_objects[key].common_bases.len(), 1);
    assert_eq!(
        fs::read_to_string(a.join("Note/Note.typ")).unwrap(),
        a_source
    );
    assert_eq!(
        fs::read_to_string(b.join("Note/Note.typ")).unwrap(),
        b_source
    );
    assert!(second_graph.blob(&hash(a_source.as_bytes())).is_some());
    assert!(second_graph.blob(&hash(b_source.as_bytes())).is_some());
}

#[test]
fn cloned_resources_root_bookmark_and_tags_get_real_receiver_admission() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("first-library");
    let b = temp.path().join("second-library");
    let mut first = Core::new(temp.path().join("first-private")).unwrap();
    first.create_library(&a).unwrap();
    let note = first.create_note(&a, &a, "Note").unwrap();
    let import = first
        .prepare_import_resource(&a, &note.path, "sample.bin", vec![0, 255, 3, 9], &fresh())
        .unwrap();
    first
        .execute_operation(&import.operation_id, &import.plan_id)
        .unwrap();
    let bookmark = first.prepare_directory_admission(&a, &a, &fresh()).unwrap();
    first
        .execute_operation(&bookmark.preview.operation_id, &bookmark.preview.plan_id)
        .unwrap();
    let tags = first.read_tag_registry(&a).unwrap();
    let tag = first
        .prepare_create_tag(&a, "标签", &tags.base_id, &fresh())
        .unwrap();
    first
        .execute_operation(&tag.preview.operation_id, &tag.preview.plan_id)
        .unwrap();
    let info = first.enable_sync(&a, &fresh()).unwrap();
    let graph = first.read_sync_history(&info.identity).unwrap();
    let keys = graph.objects().unwrap().keys().cloned().collect();
    let mut second = Core::new(temp.path().join("second-private")).unwrap();
    second
        .receive_sync(
            &info.identity,
            first.export_sync_objects(&info.identity, &keys).unwrap(),
        )
        .unwrap();
    let preview = second
        .prepare_operation(request(&b, &info.identity, keys))
        .unwrap();
    second
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let resources = second
        .managed_resource_snapshot(&b, b.join("Note/Note.typ"))
        .unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].bytes, vec![0, 255, 3, 9]);
    assert_eq!(resources[0].identity_status, "admitted");
    assert_eq!(
        second
            .resolve_navigation_target(&b, &bookmark.target_ref)
            .unwrap()
            .status,
        "managed"
    );
    let receiver_tags = second.read_tag_registry(&b).unwrap();
    assert!(receiver_tags.complete);
    assert_eq!(receiver_tags.tags[0].tag_ref, tag.target_ref);
    assert_eq!(receiver_tags.tags[0].display_name, "标签");
}

#[test]
fn clone_is_create_only_and_rechecks_target_after_review() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("first-library");
    let b = temp.path().join("second-library");
    let mut first = Core::new(temp.path().join("first-private")).unwrap();
    first.create_library(&a).unwrap();
    let note = first.create_note(&a, &a, "Note").unwrap();
    let info = first.enable_sync(&a, &fresh()).unwrap();
    let keys = BTreeSet::from([ObjectKey::Node(note.node_id.unwrap())]);
    let mut second = Core::new(temp.path().join("second-private")).unwrap();
    second
        .receive_sync(
            &info.identity,
            first.export_sync_objects(&info.identity, &keys).unwrap(),
        )
        .unwrap();
    let preview = second
        .prepare_operation(request(&b, &info.identity, keys.clone()))
        .unwrap();
    fs::create_dir(&b).unwrap();
    fs::write(b.join("sentinel.txt"), "belongs to user").unwrap();
    assert_eq!(
        second
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "exists"
    );
    assert!(!b.join("Note").exists());
    assert_eq!(
        fs::read_to_string(b.join("sentinel.txt")).unwrap(),
        "belongs to user"
    );
    assert!(
        second
            .prepare_operation(request(&b, &info.identity, keys))
            .is_err()
    );
}

#[test]
fn clone_rejects_content_identity_mismatch_before_creating_a_file() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let workspace = fresh();
    let wrong_source = format!("#metadata((id: \"{}\", title: [Wrong]))\nBody", fresh());
    let blob = hash(wrong_source.as_bytes());
    let object = ObjectKey::Node(fresh());
    let manifest = Manifest {
        version: 1,
        workspace_id: workspace.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::new(),
        dependencies: vec![],
        changes: vec![Change {
            object: object.clone(),
            before: BTreeSet::new(),
            revision: fresh(),
            after: ObjectState {
                file_revisions: BTreeMap::from([("Note.typ".into(), fresh())]),
                parent: None,
                name: "Note".into(),
                retired: false,
                files: BTreeMap::from([("Note.typ".into(), blob.clone())]),
            },
        }],
    };
    let id = manifest.id().unwrap();
    let identity = SyncIdentity {
        workspace_id: workspace,
        genesis: id.clone(),
    };
    core.receive_sync(
        &identity,
        SyncBundle {
            manifests: BTreeMap::from([(id, manifest)]),
            blobs: BTreeMap::from([(blob, wrong_source.into_bytes())]),
        },
    )
    .unwrap();
    let root = temp.path().join("replica");
    assert_eq!(
        core.prepare_operation(request(&root, &identity, BTreeSet::from([object])))
            .unwrap_err()
            .code,
        "invalidSync"
    );
    assert!(!root.exists());
}

#[test]
fn original_clone_after_images_can_be_adopted_after_commit_marker_failure() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("first-library");
    let b = temp.path().join("second-library");
    let private = temp.path().join("second-private");
    let mut first = Core::new(temp.path().join("first-private")).unwrap();
    first.create_library(&a).unwrap();
    let note = first.create_note(&a, &a, "Note").unwrap();
    let info = first.enable_sync(&a, &fresh()).unwrap();
    let keys = BTreeSet::from([ObjectKey::Node(note.node_id.clone().unwrap())]);
    let mut second = Core::new(&private).unwrap();
    second
        .receive_sync(
            &info.identity,
            first.export_sync_objects(&info.identity, &keys).unwrap(),
        )
        .unwrap();
    let preview = second
        .prepare_operation(request(&b, &info.identity, keys))
        .unwrap();
    second
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    // Controlled fixture models the missing final durable decision while
    // retaining every original materialization identity and exact after image.
    fs::rename(
        second.journal_path(&preview.operation_id, "commit"),
        temp.path().join("retained-commit-marker.json"),
    )
    .unwrap();
    drop(second);
    let mut second = Core::new(&private).unwrap();
    let recovery = second.operation_status(&preview.operation_id).unwrap();
    assert!(
        recovery
            .allowed_actions
            .contains(&"adoptAfterImages".to_string())
    );
    assert_eq!(
        second
            .resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    let current = second
        .open(b.join("Note/Note.typ"), Some(b.clone()))
        .unwrap();
    assert_eq!(current.node_id, note.node_id);
    assert_eq!(current.source, note.source);
    assert!(second.sync_binding_info(&b).unwrap().partial);
    assert_eq!(
        second
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .operation_id,
        preview.operation_id
    );
}
