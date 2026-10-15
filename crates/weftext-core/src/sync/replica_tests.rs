use super::*;
use crate::{OperationIntent, OperationRequest, SaveRequest};
use std::fs;

fn save(core: &mut Core, root: &Path, path: &Path, suffix: &str) -> String {
    let opened = core.open(path, Some(root.into())).unwrap();
    let source = format!("{}\n{suffix}", opened.source);
    let id = fresh();
    core.save(SaveRequest {
        session_id: opened.session_id,
        base_id: opened.base_id,
        source: source.clone(),
        operation_id: id,
    })
    .unwrap();
    source
}
#[track_caller]
fn operation(core: &mut Core, root: &Path, intent: OperationIntent) -> crate::OperationReceipt {
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
fn source(graph: &ReplicaGraph, object: &ObjectKey) -> String {
    let objects = graph.objects().unwrap();
    let state = &objects[object].versions[0].state;
    let blob = &state.files[&format!("{}.typ", state.name)];
    String::from_utf8(graph.blob(blob).unwrap().to_vec()).unwrap()
}

#[test]
fn saved_offline_versions_publish_original_outbox_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let object = ObjectKey::Node(note.node_id.clone().unwrap());
    let enable = fresh();
    let info = core.enable_sync(&root, &enable).unwrap();
    assert_eq!(
        core.enable_sync(&root, &enable).unwrap().identity,
        info.identity
    );
    let second = save(&mut core, &root, Path::new(&note.path), "offline second");
    let third = save(&mut core, &root, Path::new(&note.path), "offline third");
    assert_eq!(
        core.sync_binding_info(&root).unwrap().pending_publications,
        2
    );
    assert_eq!(
        source(&core.read_sync_history(&info.identity).unwrap(), &object),
        note.source
    );
    drop(core);
    let core = Core::new(&data).unwrap();
    assert_eq!(
        core.sync_binding_info(&root).unwrap().pending_publications,
        2
    );
    let result = core.publish_sync(&root).unwrap();
    assert!(
        result
            .commits
            .iter()
            .all(|c| c.admission == Admission::Accepted)
    );
    let graph = core.read_sync_history(&info.identity).unwrap();
    assert_eq!(source(&graph, &object), third);
    assert!(graph.blob(&hash(second.as_bytes())).is_some());
    assert_eq!(core.publish_sync(&root).unwrap().heads, graph.heads());
    assert_eq!(
        core.sync_binding_info(&root).unwrap().pending_publications,
        0
    );
    assert_eq!(fs::read_to_string(&note.path).unwrap(), third);
}

#[test]
fn source_and_annotations_publish_one_transaction_and_partial_export_preserves_it() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("first")).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    core.create_note(&root, &root, "Unrelated").unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    let annotation = root.join("Note/weftext.annotations.json");
    let cut = core
        .read_snapshot(&root, &[note.path.clone(), display(&annotation)])
        .unwrap();
    let changed = format!("{}\nreviewed source", note.source);
    let annotations = br#"{"version":1,"annotations":[],"unknown":"preserved"}"#.to_vec();
    let receipt = operation(
        &mut core,
        &root,
        OperationIntent::UpdateFiles {
            changes: vec![
                FileChange {
                    path: note.path,
                    base_id: Some(cut.files[0].base_id.clone()),
                    before: cut.files[0].bytes.clone(),
                    after: changed.as_bytes().to_vec(),
                },
                FileChange {
                    path: display(&annotation),
                    base_id: Some(cut.files[1].base_id.clone()),
                    before: None,
                    after: annotations.clone(),
                },
            ],
        },
    );
    core.publish_sync(&root).unwrap();
    let object = ObjectKey::Node(note.node_id.unwrap());
    let bundle = core
        .export_sync_objects(&info.identity, &BTreeSet::from([object.clone()]))
        .unwrap();
    let manifest = bundle
        .manifests
        .values()
        .find(|m| m.operation_id == receipt.operation_id)
        .unwrap();
    assert_eq!(manifest.changes.len(), 1);
    assert_eq!(manifest.changes[0].after.files.len(), 2);
    let receiver = Core::new(temp.path().join("second")).unwrap();
    receiver.receive_sync(&info.identity, bundle).unwrap();
    let graph = receiver.read_sync_history(&info.identity).unwrap();
    assert_eq!(graph.objects().unwrap().len(), 1);
    assert_eq!(source(&graph, &object), changed);
    assert_eq!(graph.blob(&hash(&annotations)).unwrap(), annotations);
}

#[test]
fn new_node_and_nested_child_commit_while_offline_with_typed_parents() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    let parent = core.create_note(&root, &root, "Parent").unwrap();
    let child = core
        .create_note(&root, root.join("Parent"), "Child")
        .unwrap();
    core.publish_sync(&root).unwrap();
    let graph = core.read_sync_history(&info.identity).unwrap();
    assert_eq!(
        graph.objects().unwrap()[&ObjectKey::Node(child.node_id.unwrap())].versions[0]
            .state
            .parent,
        Some(ObjectKey::Node(parent.node_id.unwrap()))
    );
}

#[test]
fn rename_parent_keeps_child_save_location_and_trash_restore_retains_original_content() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let parent = core.create_note(&root, &root, "Parent").unwrap();
    let child = core
        .create_note(&root, root.join("Parent"), "Child")
        .unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    operation(
        &mut core,
        &root,
        OperationIntent::RenameNode {
            source: display(&root.join("Parent")),
            name: "Renamed".into(),
        },
    );
    let source = save(
        &mut core,
        &root,
        &root.join("Renamed/Child/Child.typ"),
        "edited after parent rename",
    );
    let trash = operation(
        &mut core,
        &root,
        OperationIntent::TrashNode {
            source: display(&root.join("Renamed")),
        },
    );
    core.publish_sync(&root).unwrap();
    let object = ObjectKey::Node(parent.node_id.unwrap());
    assert!(
        core.read_sync_history(&info.identity)
            .unwrap()
            .objects()
            .unwrap()[&object]
            .versions[0]
            .state
            .retired
    );
    operation(
        &mut core,
        &root,
        OperationIntent::RestoreNode {
            trash_operation_id: trash.operation_id,
            node_id: None,
            parent: display(&root),
            name: "Restored".into(),
        },
    );
    core.publish_sync(&root).unwrap();
    let graph = core.read_sync_history(&info.identity).unwrap();
    let restored = &graph.objects().unwrap()[&object].versions[0].state;
    assert_eq!(restored.name, "Restored");
    assert!(!restored.retired);
    assert_eq!(
        fs::read_to_string(root.join("Restored/Child/Child.typ")).unwrap(),
        source
    );
    assert_eq!(
        source.as_bytes(),
        graph.blob(&hash(source.as_bytes())).unwrap()
    );
    assert_eq!(
        graph.objects().unwrap()[&ObjectKey::Node(child.node_id.unwrap())].versions[0]
            .state
            .parent,
        Some(object)
    );
}

#[test]
fn uncommitted_plan_does_not_publish_and_external_change_keeps_current_and_draft() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    let cut = core
        .read_snapshot(&root, std::slice::from_ref(&note.path))
        .unwrap();
    core.prepare_operation(OperationRequest {
        operation_id: fresh(),
        root: display(&root),
        intent: OperationIntent::UpdateFiles {
            changes: vec![FileChange {
                path: note.path.clone(),
                base_id: Some(cut.files[0].base_id.clone()),
                before: cut.files[0].bytes.clone(),
                after: format!("{}\nnever installed", note.source).into_bytes(),
            }],
        },
    })
    .unwrap();
    assert_eq!(
        core.sync_binding_info(&root).unwrap().pending_publications,
        0
    );
    let external = format!("{}\nexternal", note.source);
    fs::write(&note.path, &external).unwrap();
    let opened = core.open(&note.path, Some(root.clone())).unwrap();
    let draft = format!("{}\nretained user draft", opened.source);
    assert_eq!(
        core.save(SaveRequest {
            session_id: opened.session_id.clone(),
            base_id: opened.base_id,
            source: draft.clone(),
            operation_id: fresh()
        })
        .unwrap_err()
        .code,
        "syncExternalChange"
    );
    assert_eq!(fs::read_to_string(&note.path).unwrap(), external);
    assert_eq!(
        core.open(&note.path, Some(root))
            .unwrap()
            .recovered_draft
            .as_deref(),
        Some(draft.as_str())
    );
    assert_eq!(
        source(
            &core.read_sync_history(&info.identity).unwrap(),
            &ObjectKey::Node(note.node_id.unwrap())
        ),
        note.source
    );
}

#[test]
fn damaged_transport_cache_does_not_turn_saved_into_failed_or_allocate_new_operation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    let corrupted = data
        .join("sync")
        .join(&info.identity.workspace_id)
        .join("blobs")
        .join(format!("{}.blob", hash(note.source.as_bytes())));
    fs::write(&corrupted, "external corruption").unwrap();
    let changed = save(
        &mut core,
        &root,
        Path::new(&note.path),
        "saved despite transport failure",
    );
    let pending: Vec<_> = core
        .operations
        .values()
        .filter(|p| p.sync_publication.is_some())
        .map(|p| p.operation_id.clone())
        .collect();
    assert_eq!(pending.len(), 1);
    assert!(core.committed(&pending[0]));
    assert!(core.publish_sync(&root).is_err());
    assert_eq!(fs::read_to_string(&note.path).unwrap(), changed);
    assert_eq!(
        fs::read_to_string(corrupted).unwrap(),
        "external corruption"
    );
}

#[test]
fn moving_an_externally_changed_source_does_not_publish_an_older_body() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    core.enable_sync(&root, &fresh()).unwrap();
    let external = format!("{}\nexternal before rename", note.source);
    fs::write(&note.path, &external).unwrap();
    let result = core.prepare_operation(OperationRequest {
        operation_id: fresh(),
        root: display(&root),
        intent: OperationIntent::RenameNode {
            source: display(&root.join("Note")),
            name: "Renamed".into(),
        },
    });
    assert_eq!(result.unwrap_err().code, "syncExternalChange");
    assert!(!root.join("Renamed").exists());
    assert_eq!(fs::read_to_string(note.path).unwrap(), external);
}

#[test]
fn selected_library_backup_includes_sync_history_and_pending_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let target = temp.path().join("backup");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let info = core.enable_sync(&root, &fresh()).unwrap();
    save(
        &mut core,
        &root,
        Path::new(&note.path),
        "not yet transported",
    );
    let pending = core
        .operations
        .values()
        .find(|p| p.sync_publication.is_some())
        .unwrap()
        .operation_id
        .clone();
    operation(
        &mut core,
        &root,
        OperationIntent::BackupLibrary {
            destination: display(&target),
        },
    );
    let identity = target
        .join("private/sync")
        .join(&info.identity.workspace_id)
        .join("identity.json");
    assert_eq!(
        serde_json::from_slice::<SyncIdentity>(&fs::read(identity).unwrap()).unwrap(),
        info.identity
    );
    let outbox = target
        .join("private/journal")
        .join(format!("{pending}.plan.json"));
    let plan: Plan = serde_json::from_slice(&fs::read(outbox).unwrap()).unwrap();
    assert!(plan.sync_publication.is_some());
    assert!(
        target
            .join("private/sync")
            .join(info.identity.workspace_id)
            .join("manifests")
            .join(format!("{}.json", info.identity.genesis))
            .exists()
    );
}
