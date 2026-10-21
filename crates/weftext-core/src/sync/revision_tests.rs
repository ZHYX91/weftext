use super::*;
use crate::{ConfigRole, Core, OperationIntent, OperationRequest, SaveRequest, display, fresh};
use std::fs;

fn commit(core: &mut Core, request: OperationRequest) {
    let preview = core.prepare_operation(request).unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

#[test]
fn sync_rules_exact_role_clone_update_restart_and_invalid_content() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("a");
    let replica = temp.path().join("b");
    let receiver_data = temp.path().join("b-private");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&root).unwrap();
    let identity = a.enable_sync(&root, &fresh()).unwrap().identity;
    let graph = a.read_sync_history(&identity).unwrap();
    let original = graph.objects().unwrap()[&ObjectKey::Rules].versions[0].clone();
    assert_eq!(original.state.files.len(), 1);
    assert!(original.state.files.contains_key(".weftext-rules"));
    let keys = BTreeSet::from([ObjectKey::Rules]);
    let mut b = Core::new(&receiver_data).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    commit(
        &mut b,
        OperationRequest {
            operation_id: fresh(),
            root: display(&replica),
            intent: OperationIntent::CloneSync {
                identity: identity.clone(),
                objects: keys.clone(),
            },
        },
    );
    assert_eq!(
        fs::read(replica.join(".weftext-rules")).unwrap(),
        fs::read(root.join(".weftext-rules")).unwrap()
    );
    let after = br#"{"version":1,"ignored":["raw.txt"],"unmanaged":[]}"#.to_vec();
    let config = a.read_config(&root, ConfigRole::Rules).unwrap();
    let preview = a
        .prepare_config_save(
            &root,
            ConfigRole::Rules,
            config.bytes,
            after.clone(),
            &fresh(),
        )
        .unwrap();
    a.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    a.publish_sync(&root).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&replica),
        intent: OperationIntent::ApplySync {
            identity: identity.clone(),
            objects: keys.clone(),
        },
    };
    let preview = b.prepare_operation(request.clone()).unwrap();
    drop(b);
    let mut b = Core::new(&receiver_data).unwrap();
    assert_eq!(
        b.prepare_operation(request).unwrap().plan_id,
        preview.plan_id
    );
    b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(fs::read(replica.join(".weftext-rules")).unwrap(), after);
    let graph = a.read_sync_history(&identity).unwrap();
    let current = graph.objects().unwrap()[&ObjectKey::Rules].versions[0].clone();
    let bytes = br#"{"version":1,"ignored":[],"unmanaged":[],"unknown":true}"#.to_vec();
    let blob = hash(&bytes);
    let mut state = current.state;
    state.files.insert(".weftext-rules".into(), blob.clone());
    state
        .file_revisions
        .insert(".weftext-rules".into(), fresh());
    let manifest = Manifest {
        version: 1,
        workspace_id: identity.workspace_id.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::from([current.commit]),
        dependencies: vec![],
        changes: vec![Change {
            object: ObjectKey::Rules,
            before: BTreeSet::from([current.revision]),
            revision: fresh(),
            after: state,
        }],
    };
    b.receive_sync(
        &identity,
        SyncBundle {
            manifests: BTreeMap::from([(manifest.id().unwrap(), manifest)]),
            blobs: BTreeMap::from([(blob, bytes)]),
        },
    )
    .unwrap();
    assert_eq!(
        b.prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&replica),
            intent: OperationIntent::ApplySync {
                identity,
                objects: keys
            },
        })
        .unwrap_err()
        .code,
        "unsupportedRules"
    );
    assert_eq!(fs::read(replica.join(".weftext-rules")).unwrap(), after);
    let mut wrong_role = original.state;
    let blob = wrong_role.files.remove(".weftext-rules").unwrap();
    let revision = wrong_role.file_revisions.remove(".weftext-rules").unwrap();
    wrong_role.files.insert("other.json".into(), blob);
    wrong_role
        .file_revisions
        .insert("other.json".into(), revision);
    let wrong = Manifest {
        version: 1,
        workspace_id: fresh(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::new(),
        dependencies: vec![],
        changes: vec![Change {
            object: ObjectKey::Rules,
            before: BTreeSet::new(),
            revision: fresh(),
            after: wrong_role,
        }],
    };
    assert!(wrong.id().is_err());
}

#[test]
fn sync_received_file_reads_retained_ancestor_and_trash_without_current_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&root).unwrap();
    let note = a.create_note_fixture(&root, &root, "Note").unwrap();
    let object = ObjectKey::Node(note.node_id.unwrap());
    let identity = a.enable_sync(&root, &fresh()).unwrap().identity;
    let original =
        a.read_sync_history(&identity).unwrap().objects().unwrap()[&object].versions[0].clone();
    for suffix in ["second", "third"] {
        let doc = a.open(&note.path, Some(root.clone())).unwrap();
        a.save(SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            source: format!("{}\n{suffix}", doc.source),
            operation_id: fresh(),
        })
        .unwrap();
    }
    commit(
        &mut a,
        OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::TrashNode {
                source: display(&root.join("Note")),
            },
        },
    );
    a.publish_sync(&root).unwrap();
    let mut b = Core::new(temp.path().join("b-private")).unwrap();
    b.receive_sync(&identity, a.export_sync_history(&identity).unwrap())
        .unwrap();
    let exact = b
        .read_received_sync_file(&identity, &object, &original.revision, "Note.typ")
        .unwrap();
    assert_eq!(exact.bytes, note.source.as_bytes());
    assert_eq!(exact.object_revision, original.revision);
    assert_eq!(
        exact.file_revision,
        original.state.file_revisions["Note.typ"]
    );
    assert_eq!(
        b.read_received_sync_file(
            &identity,
            &ObjectKey::Node(fresh()),
            &original.revision,
            "Note.typ"
        )
        .unwrap_err()
        .code,
        "syncHistoryMissing"
    );
    assert_eq!(
        b.read_received_sync_file(&identity, &object, &original.revision, "Other.typ")
            .unwrap_err()
            .code,
        "syncHistoryMissing"
    );
    assert_eq!(
        b.read_received_sync_file(&identity, &object, &fresh(), "Note.typ")
            .unwrap_err()
            .code,
        "syncHistoryMissing"
    );
    assert!(
        b.with_actor("unprivileged", |c| c.read_received_sync_file(
            &identity,
            &object,
            &original.revision,
            "Note.typ"
        ))
        .is_err()
    );
}

#[test]
fn sync_received_file_staged_missing_blob_survives_restart_and_exact_hydration() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&root).unwrap();
    let note = a.create_note_fixture(&root, &root, "Note").unwrap();
    let object = ObjectKey::Node(note.node_id.unwrap());
    let identity = a.enable_sync(&root, &fresh()).unwrap().identity;
    let bundle = a.export_sync_history(&identity).unwrap();
    let revision = a.read_sync_history(&identity).unwrap().objects().unwrap()[&object].versions[0]
        .revision
        .clone();
    let data = temp.path().join("b-private");
    let b = Core::new(&data).unwrap();
    b.receive_sync(
        &identity,
        SyncBundle {
            manifests: bundle.manifests.clone(),
            blobs: BTreeMap::new(),
        },
    )
    .unwrap();
    assert_eq!(
        b.read_received_sync_file(&identity, &object, &revision, "Note.typ")
            .unwrap_err()
            .code,
        "syncIncomplete"
    );
    drop(b);
    let b = Core::new(&data).unwrap();
    assert_eq!(
        b.read_received_sync_file(&identity, &object, &revision, "Note.typ")
            .unwrap_err()
            .code,
        "syncIncomplete"
    );
    b.receive_sync(&identity, bundle).unwrap();
    assert_eq!(
        b.read_received_sync_file(&identity, &object, &revision, "Note.typ")
            .unwrap()
            .bytes,
        note.source.as_bytes()
    );
}
