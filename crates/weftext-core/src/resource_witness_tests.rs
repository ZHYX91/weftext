use super::*;
use std::os::windows::fs::OpenOptionsExt;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    source: PathBuf,
    leaf: PathBuf,
    owner: weftext_knowledge::NodeRef,
    reference: String,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note(&root, &root, "Note").unwrap();
        let owner = weftext_knowledge::NodeRef {
            workspace_id: core.require_workspace(&root).unwrap().workspace_id,
            node_id: note.node_id.unwrap(),
        };
        let source = PathBuf::from(note.path);
        let leaf = root.join("Note/owned.bin");
        let plan = core
            .prepare_import_resource(
                &root,
                &source,
                "owned.bin",
                b"original bytes".to_vec(),
                &fresh(),
            )
            .unwrap();
        core.execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        Self {
            _temp: temp,
            root,
            source,
            leaf,
            owner,
            reference: plan.resource_map.values().next().unwrap().clone(),
            core,
        }
    }
    fn qualify(&self) -> QualifiedResource {
        self.core
            .qualified_resource_at(
                &self.root,
                &self.source,
                &self.owner,
                &self.reference,
                &self.leaf,
                "live",
                None,
            )
            .unwrap()
    }
    fn commit_same_bytes(&mut self) -> String {
        accept_observed_resource(&mut self.core, &self.root, &self.leaf)
    }
}
fn accept_observed_resource(core: &mut Core, root: &Path, leaf: &Path) -> String {
    let snapshot = core
        .read_snapshot(root, &[display(leaf)])
        .unwrap()
        .files
        .remove(0);
    let before = snapshot.bytes.unwrap();
    let plan = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(leaf),
                    base_id: Some(snapshot.base_id),
                    before: Some(before.clone()),
                    after: before,
                }],
            },
        })
        .unwrap();
    core.execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    plan.revisions[&display(leaf)].clone()
}

#[test]
fn resource_witness_rejects_same_object_same_bytes_new_accepted_revision() {
    let mut f = Fixture::new();
    let identity = f.core.enable_sync(&f.root, &fresh()).unwrap().identity;
    let key = crate::sync::ObjectKey::Node(f.owner.node_id.clone());
    let keys = std::collections::BTreeSet::from([key.clone()]);
    let graph = f.core.read_sync_history(&identity).unwrap();
    let objects = graph.objects().unwrap();
    let base = &objects[&key].versions[0];
    let changed_bytes = b"equal independently authored resource bytes".to_vec();
    let blob = hash(&changed_bytes);
    let first_revision = fresh();
    let second_revision = fresh();
    // These are two genuine portable branches with changed bytes relative to
    // their shared base, as in the existing admitted resource-update fixture.
    // Apply the first, then explicitly choose the equally materialized second.
    for (index, revision) in [&first_revision, &second_revision].into_iter().enumerate() {
        let mut state = base.state.clone();
        state.files.insert("owned.bin".into(), blob.clone());
        state
            .file_revisions
            .insert("owned.bin".into(), revision.clone());
        let manifest = crate::sync::Manifest {
            version: 1,
            workspace_id: identity.workspace_id.clone(),
            replica_id: fresh(),
            operation_id: fresh(),
            parents: std::collections::BTreeSet::from([base.commit.clone()]),
            changes: vec![crate::sync::Change {
                object: key.clone(),
                before: std::collections::BTreeSet::from([base.revision.clone()]),
                revision: fresh(),
                after: state,
            }],
            dependencies: vec![],
        };
        f.core
            .receive_sync(
                &identity,
                crate::sync::SyncBundle {
                    manifests: std::collections::BTreeMap::from([(
                        manifest.id().unwrap(),
                        manifest,
                    )]),
                    blobs: std::collections::BTreeMap::from([(
                        blob.clone(),
                        changed_bytes.clone(),
                    )]),
                },
            )
            .unwrap();
        if index == 0 {
            let apply = f
                .core
                .prepare_operation(OperationRequest {
                    operation_id: fresh(),
                    root: display(&f.root),
                    intent: OperationIntent::ApplySync {
                        identity: identity.clone(),
                        objects: keys.clone(),
                    },
                })
                .unwrap();
            f.core
                .execute_operation(&apply.operation_id, &apply.plan_id)
                .unwrap();
        }
    }
    let original = f.qualify();
    assert_eq!(original.revision.as_ref(), Some(&first_revision));
    f.core
        .validate_resource_witness(&f.root, &original.witness)
        .unwrap();
    let objects = f
        .core
        .read_sync_history(&identity)
        .unwrap()
        .objects()
        .unwrap();
    let sides = &objects[&key];
    assert_eq!(sides.versions.len(), 2);
    let state = sides
        .versions
        .iter()
        .find(|v| v.state.file_revisions["owned.bin"] == second_revision)
        .unwrap()
        .state
        .clone();
    let merge = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::MergeSync {
                identity,
                resolutions: vec![crate::sync::SyncResolution {
                    object: key,
                    revisions: sides.versions.iter().map(|v| v.revision.clone()).collect(),
                    state,
                }],
                blobs: std::collections::BTreeMap::new(),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&merge.operation_id, &merge.plan_id)
        .unwrap();
    let data = f._temp.path().join("private");
    drop(f.core);
    f.core = Core::new(&data).unwrap();
    // Exact file identity, stamp and bytes alone cannot detect this change.
    f.core
        .validate_snapshot_witness(&f.root, &f.leaf, &original.witness.resource)
        .unwrap();
    assert_eq!(
        f.core
            .validate_resource_witness(&f.root, &original.witness)
            .unwrap_err()
            .code,
        "resourceVersion"
    );
    let current = f.qualify();
    assert_eq!(current.bytes(), original.bytes());
    assert_eq!(current.revision.as_ref(), Some(&second_revision));
    f.core
        .validate_resource_witness(&f.root, &current.witness)
        .unwrap();
    assert_eq!(
        f.core
            .qualified_resource_at(
                &f.root,
                &f.source,
                &f.owner,
                &f.reference,
                &f.leaf,
                "live",
                Some(&first_revision)
            )
            .err()
            .unwrap()
            .code,
        "resourceVersion"
    );
}

#[test]
fn resource_witness_rejects_observed_same_bytes_becoming_accepted() {
    let mut f = Fixture::new();
    fs::write(&f.leaf, b"external observed bytes").unwrap();
    let observed = f.qualify();
    assert!(observed.revision.is_none());
    f.core
        .validate_resource_witness(&f.root, &observed.witness)
        .unwrap();
    let revision = f.commit_same_bytes();
    f.core
        .validate_snapshot_witness(&f.root, &f.leaf, &observed.witness.resource)
        .unwrap();
    assert_eq!(
        f.core
            .validate_resource_witness(&f.root, &observed.witness)
            .unwrap_err()
            .code,
        "resourceVersion"
    );
    let accepted = f.qualify();
    assert_eq!(accepted.bytes(), observed.bytes());
    assert_eq!(accepted.revision.as_deref(), Some(revision.as_str()));
    f.core
        .validate_resource_witness(&f.root, &accepted.witness)
        .unwrap();
}

#[test]
fn missing_resource_input_is_exact_readonly_evidence_and_detects_late_leaf() {
    let mut f = Fixture::new();
    let hidden = f
        .core
        .prepare_import_resource(
            &f.root,
            &f.source,
            "hidden.bin",
            b"private sibling".to_vec(),
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&hidden.operation_id, &hidden.plan_id)
        .unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(f.root.join("Note/hidden.bin"))
        .unwrap();
    let retained = f._temp.path().join("retained-original.bin");
    fs::rename(&f.leaf, &retained).unwrap();
    let snapshots = f.core.snapshots.len();
    let missing = match f
        .core
        .qualified_resource_input_at(&f.root, &f.source, &f.owner, &f.reference, &f.leaf, "live")
        .unwrap()
    {
        QualifiedResourceInput::Missing(witness) => witness,
        QualifiedResourceInput::Present(_) => {
            panic!("A missing selected leaf must not become present")
        }
    };
    assert!(missing.retained_bytes() > 0);
    assert_eq!(f.core.snapshots.len(), snapshots);
    f.core
        .validate_missing_resource_witness(&f.root, &missing)
        .unwrap();
    assert!(
        f.core
            .qualified_resource_at(
                &f.root,
                &f.source,
                &f.owner,
                &f.reference,
                &f.leaf,
                "live",
                None
            )
            .is_err()
    );
    drop(held);
    fs::rename(&retained, &f.leaf).unwrap();
    assert_eq!(
        f.core
            .validate_missing_resource_witness(&f.root, &missing)
            .unwrap_err()
            .code,
        "staleBase"
    );
    assert!(matches!(
        f.core
            .qualified_resource_input_at(
                &f.root,
                &f.source,
                &f.owner,
                &f.reference,
                &f.leaf,
                "live"
            )
            .unwrap(),
        QualifiedResourceInput::Present(_)
    ));
}

#[test]
fn missing_resource_input_rejects_raw_registration_locator_and_unknown_birth() {
    let mut f = Fixture::new();
    fs::rename(&f.leaf, f._temp.path().join("retained-original.bin")).unwrap();
    let manifest = f.root.join("Note/weftext.resources.json");
    let original = fs::read(&manifest).unwrap();
    assert_eq!(
        f.core
            .qualified_resource_input_at(
                &f.root,
                &f.source,
                &f.owner,
                &f.reference,
                &f.root.join("Note/other.bin"),
                "live"
            )
            .err()
            .unwrap()
            .code,
        "resourcePath"
    );
    let mut raw = original.clone();
    raw.extend_from_slice(b"\n");
    fs::write(&manifest, &raw).unwrap();
    assert_eq!(
        f.core
            .qualified_resource_input_at(
                &f.root,
                &f.source,
                &f.owner,
                &f.reference,
                &f.leaf,
                "live"
            )
            .err()
            .unwrap()
            .code,
        "resourceManifest"
    );
    let unknown = fresh();
    let value =
        serde_json::json!({"version":1,"resources":[{"resourceId":unknown,"path":"ghost.bin"}]});
    let snapshot = f
        .core
        .read_snapshot(&f.root, &[display(&manifest)])
        .unwrap()
        .files
        .remove(0);
    let plan = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(&manifest),
                    base_id: Some(snapshot.base_id),
                    before: snapshot.bytes,
                    after: serde_json::to_vec(&value).unwrap(),
                }],
            },
        })
        .unwrap();
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    let reference = format!("weftext-resource://{}/{}", f.owner.workspace_id, unknown);
    assert_eq!(
        f.core
            .qualified_resource_input_at(
                &f.root,
                &f.source,
                &f.owner,
                &reference,
                &f.root.join("Note/ghost.bin"),
                "live"
            )
            .err()
            .unwrap()
            .code,
        "resourceUnadmitted"
    );
    // Registration disappearance is not evidence that its resource is absent.
    assert_eq!(
        f.core
            .qualified_resource_input_at(
                &f.root,
                &f.source,
                &f.owner,
                &f.reference,
                &f.leaf,
                "live"
            )
            .err()
            .unwrap()
            .code,
        "resourceMissing"
    );
}

#[test]
fn missing_resource_input_in_trash_keeps_original_owner_leaf_and_lifecycle() {
    let mut f = Fixture::new();
    let plan = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: OperationIntent::TrashNode {
                source: display(f.source.parent().unwrap()),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    let tree = f.root.join(".weftext-trash").join(&plan.operation_id);
    let owner = tree.join("Note.typ");
    let leaf = tree.join("owned.bin");
    fs::rename(&leaf, f._temp.path().join("retained-trash-resource.bin")).unwrap();
    let missing = match f
        .core
        .qualified_resource_input_at(&f.root, &owner, &f.owner, &f.reference, &leaf, "trash")
        .unwrap()
    {
        QualifiedResourceInput::Missing(witness) => witness,
        QualifiedResourceInput::Present(_) => panic!("The original trash resource is missing"),
    };
    f.core
        .validate_missing_resource_witness(&f.root, &missing)
        .unwrap();
    assert!(
        f.core
            .qualified_resource_input_at(&f.root, &owner, &f.owner, &f.reference, &leaf, "live")
            .is_err()
    );
    assert!(
        f.core
            .qualified_resource_input_at(
                &f.root,
                &tree.join(format!("{}.typ", plan.operation_id)),
                &f.owner,
                &f.reference,
                &leaf,
                "trash"
            )
            .is_err()
    );
    fs::rename(&owner, f._temp.path().join("retained-trash-owner.typ")).unwrap();
    assert!(
        f.core
            .validate_missing_resource_witness(&f.root, &missing)
            .is_err()
    );
    assert!(
        f.core
            .qualified_resource_input_at(&f.root, &owner, &f.owner, &f.reference, &leaf, "trash")
            .is_err()
    );
}
