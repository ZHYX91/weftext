use super::*;
use crate::{Core, OperationIntent, OperationRequest, fresh};
use std::{collections::BTreeSet, fs, path::PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
    a: Core,
    b: Core,
    ar: PathBuf,
    br: PathBuf,
    identity: crate::sync::SyncIdentity,
    node: ObjectKey,
    keys: BTreeSet<ObjectKey>,
    retained: PathBuf,
    before: ObjectState,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let ar = temp.path().join("a-library");
        let br = temp.path().join("b-library");
        let mut a = Core::new(temp.path().join("a-private")).unwrap();
        a.create_library(&ar).unwrap();
        let note = a.create_note(&ar, &ar, "Note").unwrap();
        let imported = a
            .prepare_import_resource(
                &ar,
                Path::new(&note.path),
                "asset.bin",
                b"original portable resource".to_vec(),
                &fresh(),
            )
            .unwrap();
        a.execute_operation(&imported.operation_id, &imported.plan_id)
            .unwrap();
        let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
        let keys = a
            .read_sync_history(&identity)
            .unwrap()
            .objects()
            .unwrap()
            .into_keys()
            .collect::<BTreeSet<_>>();
        let mut b = Core::new(temp.path().join("b-private")).unwrap();
        b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
            .unwrap();
        let clone = b
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: crate::display(&br),
                intent: OperationIntent::CloneSync {
                    identity: identity.clone(),
                    objects: keys.clone(),
                },
            })
            .unwrap();
        b.execute_operation(&clone.operation_id, &clone.plan_id)
            .unwrap();
        let node = ObjectKey::Node(note.node_id.unwrap());
        let before = b
            .sync_local_state(b.sync_setup_for(&br).unwrap())
            .unwrap()
            .0[&node]
            .state
            .clone();
        Self {
            retained: temp.path().join("original-outside-author-tree.bin"),
            _temp: temp,
            a,
            b,
            ar,
            br,
            identity,
            node,
            keys,
            before,
        }
    }

    fn publish_original_external_removal(&mut self) -> ObjectState {
        let asset = self.ar.join("Note").join("asset.bin");
        let before = crate::operations::capture_optional(&asset)
            .unwrap()
            .unwrap();
        fs::rename(&asset, &self.retained).unwrap();
        assert_eq!(
            crate::operations::capture_optional(&self.retained)
                .unwrap()
                .unwrap()
                .stamp
                .identity,
            before.stamp.identity
        );
        let manifest_path = self.ar.join("Note").join("weftext.resources.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["resources"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["path"] != "asset.bin");
        fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let cut = self.a.read_sync_adoption_cut(&self.ar, &self.keys).unwrap();
        assert_eq!(cut.removed_members.len(), 1);
        let adopt = self
            .a
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: crate::display(&self.ar),
                intent: OperationIntent::AdoptSyncExternal {
                    identity: cut.identity,
                    expected_installed: cut.expected_installed,
                    changes: cut.changes,
                    removed_members: cut.removed_members,
                },
            })
            .unwrap();
        self.a
            .execute_operation(&adopt.operation_id, &adopt.plan_id)
            .unwrap();
        self.a.publish_sync(&self.ar).unwrap();
        self.b
            .receive_sync(
                &self.identity,
                self.a
                    .export_sync_objects(&self.identity, &BTreeSet::from([self.node.clone()]))
                    .unwrap(),
            )
            .unwrap();
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let objects = graph.objects().unwrap();
        let [chosen] = objects[&self.node].versions.as_slice() else {
            panic!("The original sender adoption publishes one actual successor");
        };
        chosen.state.clone()
    }
}

#[test]
fn member_retirement_qualifies_only_real_sender_removed_registered_resource_metadata() {
    let mut f = Fixture::new();
    let source_path = f.br.join("Note").join("Note.typ");
    let resource_path = f.br.join("Note").join("asset.bin");
    let source = crate::operations::capture_optional(&source_path)
        .unwrap()
        .unwrap();
    let asset = crate::operations::capture_optional(&resource_path)
        .unwrap()
        .unwrap();
    let chosen = f.publish_original_external_removal();
    let graph = f.b.read_sync_history(&f.identity).unwrap();
    let resource_ids = crate::sync::apply::validate_state(&graph, &f.node, &f.before).unwrap();
    let removed = qualify_member_retirements(&graph, &f.node, &f.before, &chosen).unwrap();
    assert_eq!(
        removed,
        vec![RetiredResourceMember {
            relative_path: "asset.bin".into(),
            resource_id: resource_ids["asset.bin"].clone(),
            digest: f.before.files["asset.bin"].clone(),
            file_revision: f.before.file_revisions["asset.bin"].clone(),
        }]
    );
    assert_eq!(
        f.b.sync_local_state(f.b.sync_setup_for(&f.br).unwrap())
            .unwrap()
            .0[&f.node]
            .state,
        f.before
    );
    // Qualification alone has no receiver effect, new identity or receipt.
    assert_eq!(
        crate::operations::capture_optional(&resource_path)
            .unwrap()
            .unwrap()
            .stamp,
        asset.stamp
    );
    assert_eq!(
        crate::operations::capture_optional(&source_path)
            .unwrap()
            .unwrap()
            .stamp,
        source.stamp
    );
    assert_eq!(fs::read(&resource_path).unwrap(), asset.bytes);
    assert_eq!(
        fs::read(&f.retained).unwrap(),
        b"original portable resource"
    );
    assert_eq!(
        graph.blob(&f.before.files["asset.bin"]).unwrap(),
        asset.bytes
    );
}

#[test]
fn member_retirement_preserves_original_canonical_rename_and_rejects_foreign_owner() {
    let mut f = Fixture::new();
    let before = f.before.clone();
    let renamed =
        f.a.prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: crate::display(&f.ar),
            intent: OperationIntent::RenameNode {
                source: crate::display(&f.ar.join("Note")),
                name: "Renamed".into(),
            },
        })
        .unwrap();
    f.a.execute_operation(&renamed.operation_id, &renamed.plan_id)
        .unwrap();
    f.a.publish_sync(&f.ar).unwrap();
    f.b.receive_sync(
        &f.identity,
        f.a.export_sync_objects(&f.identity, &BTreeSet::from([f.node.clone()]))
            .unwrap(),
    )
    .unwrap();
    let graph = f.b.read_sync_history(&f.identity).unwrap();
    let objects = graph.objects().unwrap();
    let [version] = objects[&f.node].versions.as_slice() else {
        panic!("Actual rename has one successor")
    };
    assert!(!version.state.files.contains_key("Note.typ"));
    assert!(version.state.files.contains_key("Renamed.typ"));
    assert_eq!(version.state.files["asset.bin"], before.files["asset.bin"]);
    assert!(
        qualify_member_retirements(&graph, &f.node, &before, &version.state)
            .unwrap()
            .is_empty()
    );
    assert!(
        qualify_member_retirements(&graph, &ObjectKey::Node(fresh()), &before, &version.state)
            .is_err()
    );
    assert!(f.br.join("Note").join("Note.typ").is_file());
    assert!(!f.br.join("Renamed").exists());
}
