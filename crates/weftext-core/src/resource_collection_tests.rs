use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::os::windows::fs::OpenOptionsExt;

struct Fixture {
    temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    source: PathBuf,
    owner: weftext_knowledge::NodeRef,
    core: Core,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("Library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note(&root, &root, "Note").unwrap();
        let owner = weftext_knowledge::NodeRef {
            workspace_id: core.require_workspace(&root).unwrap().workspace_id,
            node_id: note.node_id.unwrap(),
        };
        Self {
            temp,
            data,
            root,
            source: PathBuf::from(note.path),
            owner,
            core,
        }
    }
    fn collection(&self) -> Result<QualifiedResourceCollection> {
        self.core
            .qualified_resource_collection_at(&self.root, &self.source, &self.owner, "live")
    }
    fn manifest(&self) -> PathBuf {
        self.source.parent().unwrap().join("weftext.resources.json")
    }
    fn import(&mut self, name: &str) -> String {
        let plan = self
            .core
            .prepare_import_resource(
                &self.root,
                &self.source,
                name,
                b"owned bytes".to_vec(),
                &fresh(),
            )
            .unwrap();
        self.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        plan.resource_map.values().next().unwrap().clone()
    }
    fn write_manifest(&mut self, document: serde_json::Value) -> OperationPreview {
        let path = self.manifest();
        let snapshot = self
            .core
            .read_snapshot(&self.root, &[display(&path)])
            .unwrap()
            .files
            .remove(0);
        let plan = self
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&path),
                        base_id: Some(snapshot.base_id),
                        before: snapshot.bytes,
                        after: serde_json::to_vec(&document).unwrap(),
                    }],
                },
            })
            .unwrap();
        self.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        plan
    }
    fn execute(&mut self, intent: OperationIntent) -> OperationPreview {
        let plan = self
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent,
            })
            .unwrap();
        self.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        plan
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
    fn docx_plan(&mut self, name: &str) -> OperationPreview {
        let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
        let rendered = engine
            .render(
                "Imported text",
                &self.temp.path().join("producer.typ"),
                self.temp.path(),
            )
            .unwrap();
        assert!(rendered.ok, "{:?}", rendered.diagnostics);
        let docx = weftext_export::docx::export_docx(weftext_export::docx::DocxInput {
            render: &rendered,
            template: None,
            bindings: &BTreeMap::new(),
            options: weftext_export::docx::DocxOptions::default(),
        })
        .unwrap();
        self.core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent: OperationIntent::ImportDocx {
                    parent: display(&self.root),
                    name: name.into(),
                    bytes: docx.bytes,
                    allow_losses: true,
                },
            })
            .unwrap()
    }
}

#[test]
fn resource_collection_imported_owner_bridge_is_original_slot_and_survives_restart() {
    let mut f = Fixture::new();
    let plan = f.docx_plan("Imported");
    let source = f.root.join("Imported/document/document.typ");
    assert!(
        f.core
            .qualified_imported_query_node_at(&f.root, &source)
            .is_err()
    );
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    f.restart();
    let hidden = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&f.source)
        .unwrap();
    let original = f
        .core
        .qualified_imported_query_node_at(&f.root, &source)
        .unwrap();
    assert!(
        plan.node_map
            .values()
            .any(|id| id == &original.owner.node_id)
    );
    let collection = f
        .core
        .qualified_resource_collection_at(&f.root, &source, &original.owner, "live")
        .unwrap();
    assert!(
        collection
            .entries
            .iter()
            .any(|entry| entry.relative_path == "original.docx")
    );
    f.core
        .validate_query_node_witness(&f.root, &original.witness)
        .unwrap();
    drop(hidden);
    // A different physical file with the same author UUID is not this producer.
    let bytes = fs::read(&source).unwrap();
    let retained = f.root.join("retained-imported.typ");
    fs::rename(&source, &retained).unwrap();
    fs::write(&source, bytes).unwrap();
    assert!(
        f.core
            .qualified_imported_query_node_at(&f.root, &source)
            .is_err()
    );
    fs::remove_file(&source).unwrap();
    fs::rename(retained, &source).unwrap();
    f.execute(OperationIntent::RenameNode {
        source: display(source.parent().unwrap()),
        name: "Moved".into(),
    });
    let moved = f.root.join("Imported/Moved/Moved.typ");
    assert_eq!(
        f.core
            .qualified_imported_query_node_at(&f.root, &moved)
            .err()
            .unwrap()
            .code,
        "resourceOwner"
    );
    f.core
        .qualified_query_node_at(&f.root, &moved, &original.owner, "live")
        .unwrap();
}

#[test]
fn resource_collection_imported_owner_bridge_rejects_ordinary_copy_and_uncommitted_producer() {
    let mut f = Fixture::new();
    assert_eq!(
        f.core
            .qualified_imported_query_node_at(&f.root, &f.source)
            .err()
            .unwrap()
            .code,
        "resourceOwner"
    );
    let pending = f.docx_plan("Pending");
    let output = match &f.core.operations[&pending.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .mutation
    {
        Mutation::CreateTree { files, .. } => files
            .iter()
            .find(|file| file.path.ends_with("document.typ"))
            .unwrap()
            .clone(),
        _ => unreachable!(),
    };
    fs::create_dir_all(Path::new(&output.path).parent().unwrap()).unwrap();
    fs::write(&output.path, output.after).unwrap();
    assert_eq!(
        f.core
            .qualified_imported_query_node_at(&f.root, Path::new(&output.path))
            .err()
            .unwrap()
            .code,
        "resourceOwner"
    );
    let plan = f.docx_plan("Imported");
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    let source = f.root.join("Imported").join("document");
    f.execute(OperationIntent::CopyNode {
        source: display(&source),
        parent: display(&f.root),
        name: "Copy".into(),
        allow_losses: true,
    });
    assert_eq!(
        f.core
            .qualified_imported_query_node_at(&f.root, &f.root.join("Copy/Copy.typ"))
            .err()
            .unwrap()
            .code,
        "resourceOwner"
    );
}

#[test]
fn resource_collection_plan_does_not_fold_distinct_hardlink_leaf_dependencies() {
    let mut f = Fixture::new();
    let alias = f.source.parent().unwrap().join("alias.typ");
    let before = fs::read(&f.source).unwrap();
    let base = f
        .core
        .read_snapshot(&f.root, &[display(&f.source)])
        .unwrap()
        .files
        .remove(0);
    let original = f.core.take_snapshot_witness(&base.base_id).unwrap();
    let base = f
        .core
        .read_snapshot(&f.root, &[display(&f.source)])
        .unwrap()
        .files
        .remove(0);
    let plan = f
        .core
        .prepare_dependent_feature_operation_with_witnesses(
            OperationRequest {
                operation_id: fresh(),
                root: display(&f.root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&f.source),
                        base_id: Some(base.base_id),
                        before: Some(before.clone()),
                        after: (decode(&before).unwrap() + "\nReview\n").into_bytes(),
                    }],
                },
            },
            &serde_json::json!({"hardlink":"two namespace entries"}),
            &[],
            &[&original],
        )
        .unwrap();
    fs::hard_link(&f.source, &alias).unwrap();
    // The backend rejects distinct hardlink leaves before a second witness
    // could be captured or folded, and before a previously reviewed write.
    for path in [&f.source, &alias] {
        assert_eq!(
            f.core
                .read_snapshot(&f.root, &[display(path)])
                .unwrap_err()
                .code,
            "hardLink"
        );
    }
    assert_eq!(
        f.core
            .validate_snapshot_witness(&f.root, &f.source, &original)
            .unwrap_err()
            .code,
        "hardLink"
    );
    assert_eq!(
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap_err()
            .code,
        "hardLink"
    );
    assert_eq!(fs::read(&f.source).unwrap(), before);
}

#[test]
fn resource_collection_genuine_empty_then_original_import_and_restart() {
    let mut f = Fixture::new();
    let empty = f.collection().unwrap();
    assert!(!empty.initialized);
    assert!(empty.entries.is_empty());
    assert!(empty.revision.is_none());
    assert!(!f.manifest().exists());
    f.core
        .validate_resource_collection_witness(&f.root, &empty.witness)
        .unwrap();
    let mut pool = vec![];
    assert_eq!(
        Core::merge_resource_collection_witness(&mut pool, &empty.witness)
            .unwrap_err()
            .code,
        "basisRequired"
    );
    assert!(pool.is_empty());
    let reference = f.import("owned.bin");
    f.restart();
    assert!(
        f.core
            .validate_resource_collection_witness(&f.root, &empty.witness)
            .is_err()
    );
    let collection = f.collection().unwrap();
    assert!(collection.initialized);
    assert!(collection.revision.is_some());
    assert_eq!(collection.entries.len(), 1);
    assert_eq!(collection.entries[0].resource_ref, reference);
    assert_eq!(collection.entries[0].relative_path, "owned.bin");
    assert!(
        collection.witness.retained_bytes() >= fs::metadata(f.manifest()).unwrap().len() as usize
    );
    Core::merge_resource_collection_witness(&mut pool, &collection.witness).unwrap();
}

#[test]
fn resource_collection_lost_committed_manifest_and_empty_manifest_cannot_reset() {
    for populated in [false, true] {
        let mut f = Fixture::new();
        if populated {
            f.import("owned.bin");
        } else {
            f.write_manifest(serde_json::json!({"version":1,"resources":[]}));
        }
        assert!(f.collection().unwrap().initialized);
        fs::remove_file(f.manifest()).unwrap();
        f.restart();
        assert_eq!(
            f.collection().err().unwrap().code,
            "resourceManifestUnavailable"
        );
        assert_eq!(
            f.core
                .prepare_import_resource(&f.root, &f.source, "new.bin", vec![1], &fresh())
                .unwrap_err()
                .code,
            "resourceManifestUnavailable"
        );
        assert!(!f.manifest().exists());
        assert!(!f.source.parent().unwrap().join("new.bin").exists());
    }
}

#[test]
fn resource_collection_delayed_original_first_import_cannot_reset_newly_lost_manifest() {
    let mut f = Fixture::new();
    let original = f
        .core
        .prepare_import_resource(&f.root, &f.source, "delayed.bin", vec![3], &fresh())
        .unwrap();
    f.import("installed.bin");
    fs::remove_file(f.manifest()).unwrap();
    f.restart();
    assert_eq!(
        f.core
            .execute_operation(&original.operation_id, &original.plan_id)
            .unwrap_err()
            .code,
        "resourceManifestUnavailable"
    );
    assert!(!f.manifest().exists());
    assert!(!f.source.parent().unwrap().join("delayed.bin").exists());
    assert_eq!(
        f.core
            .validate_recovery_scope(
                f.core.operations[&original.operation_id]
                    .transaction
                    .as_ref()
                    .unwrap()
            )
            .unwrap_err()
            .code,
        "resourceManifestUnavailable"
    );
}

#[test]
fn resource_collection_missing_observation_cannot_erase_original_empty_initialization() {
    let mut f = Fixture::new();
    let plan = f.write_manifest(serde_json::json!({"version":1,"resources":[]}));
    fs::remove_file(f.core.journal_path(&plan.operation_id, "observations")).unwrap();
    fs::remove_file(f.manifest()).unwrap();
    f.restart();
    assert_eq!(
        f.collection().err().unwrap().code,
        "resourceManifestUnavailable"
    );
    assert_eq!(
        f.core
            .prepare_import_resource(&f.root, &f.source, "new.bin", vec![1], &fresh())
            .unwrap_err()
            .code,
        "resourceManifestUnavailable"
    );
}

#[test]
fn resource_collection_original_first_import_afterimages_remain_recoverable() {
    let mut f = Fixture::new();
    let plan = f
        .core
        .prepare_import_resource(&f.root, &f.source, "original.bin", vec![9], &fresh())
        .unwrap();
    let changes = match &f.core.operations[&plan.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .mutation
    {
        Mutation::Update { files } => files.clone(),
        _ => unreachable!(),
    };
    journal::private_create(
        &f.core.journal_path(&plan.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    for change in &changes {
        let guard = ParentGuard::acquire(Path::new(&change.path)).unwrap();
        let mut handle = guard.open_file(true, true).unwrap();
        f.core
            .record_materialized(&plan.operation_id, Path::new(&change.path), &handle)
            .unwrap();
        operations::write_blob(&mut handle, &change.after).unwrap();
    }
    f.restart();
    let recovery = f.core.inspect_recovery(&plan.operation_id).unwrap();
    assert!(
        recovery
            .allowed_actions
            .iter()
            .any(|action| action == "adoptAfterImages")
    );
    f.core
        .resolve_recovery(&plan.operation_id, "adoptAfterImages")
        .unwrap();
    assert_eq!(
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap()
            .status,
        "committed"
    );
    let collection = f.collection().unwrap();
    assert_eq!(
        &collection.entries[0].resource_ref,
        plan.resource_map.values().next().unwrap()
    );
}

#[test]
fn resource_collection_import_with_original_node_and_membership_witnesses_uses_one_owner_handle() {
    for initialized in [false, true] {
        let mut f = Fixture::new();
        if initialized {
            f.import("existing.bin");
        }
        let configured_source = f.root.join("Note/Note.typ");
        let node = f
            .core
            .qualified_query_node_at(&f.root, &configured_source, &f.owner, "live")
            .unwrap();
        let collection = f
            .core
            .qualified_resource_collection_at(&f.root, &configured_source, &f.owner, "live")
            .unwrap();
        let mut pool = vec![];
        Core::merge_query_node_witness(&mut pool, &node.witness).unwrap();
        if initialized {
            Core::merge_resource_collection_witness(&mut pool, &collection.witness).unwrap();
        }
        let leaf = f.source.parent().unwrap().join("new.bin");
        let bases = f
            .core
            .read_snapshot(&f.root, &[display(&f.manifest()), display(&leaf)])
            .unwrap();
        let plan = f
            .core
            .prepare_dependent_feature_operation_with_witnesses(
                OperationRequest {
                    operation_id: fresh(),
                    root: display(&f.root),
                    intent: OperationIntent::ImportResource {
                        node: display(&configured_source),
                        name: "new.bin".into(),
                        bytes: vec![5],
                        manifest_base_id: bases.files[0].base_id.clone(),
                        resource_base_id: bases.files[1].base_id.clone(),
                    },
                },
                &serde_json::json!({"witnessed_import":true}),
                &[],
                &pool.iter().collect::<Vec<_>>(),
            )
            .unwrap();
        let source_identity = f
            .core
            .qualified_node_at(&f.root, &f.source)
            .unwrap()
            .identity;
        let count = f.core.operations[&plan.operation_id]
            .transaction
            .as_ref()
            .unwrap()
            .files
            .iter()
            .filter(|file| file.stamp.identity == source_identity)
            .count();
        assert_eq!(
            count, 1,
            "A configured slash alias must not become a second exclusive owner handle"
        );
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        assert_eq!(
            f.collection().unwrap().entries.len(),
            if initialized { 2 } else { 1 }
        );
    }
}

#[test]
fn resource_collection_old_path_new_owner_does_not_transfer_initialization() {
    let mut f = Fixture::new();
    f.execute(OperationIntent::RenameNode {
        source: display(f.source.parent().unwrap()),
        name: "Moved".into(),
    });
    f.source = f.root.join("Moved/Moved.typ");
    let new = f.core.create_note(&f.root, &f.root, "Note").unwrap();
    let plan = f
        .core
        .prepare_import_resource(&f.root, &new.path, "new-owner.bin", vec![2], &fresh())
        .unwrap();
    f.core
        .execute_operation(&plan.operation_id, &plan.plan_id)
        .unwrap();
    f.restart();
    // The new owner initialized a manifest at the original owner's old path,
    // after its identity-proved relocation. It belongs only to the new owner.
    assert!(!f.collection().unwrap().initialized);
    f.write_manifest(serde_json::json!({"version":1,"resources":[]}));
    f.execute(OperationIntent::RenameNode {
        source: display(f.source.parent().unwrap()),
        name: "Again".into(),
    });
    f.source = f.root.join("Again/Again.typ");
    fs::remove_file(f.manifest()).unwrap();
    f.restart();
    assert_eq!(
        f.collection().err().unwrap().code,
        "resourceManifestUnavailable"
    );
}

#[test]
fn resource_collection_symbols_preserve_unknown_entries_without_reading_siblings() {
    let mut f = Fixture::new();
    let first = f.import("owned.bin");
    let second = f.import("secret.bin");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    document["extension"] = serde_json::json!({"opaque":"retained"});
    document["resources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"future":true,"path":"broken-private.bin"}));
    document["resources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"resourceId":fresh(),"path":"forged.bin"}));
    f.write_manifest(document);
    let secret = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(f.source.parent().unwrap().join("secret.bin"))
        .unwrap();
    let collection = f.collection().unwrap();
    assert_eq!(collection.entries.len(), 2);
    assert_eq!(collection.entries[0].resource_ref, first);
    assert_eq!(collection.entries[1].resource_ref, second);
    assert_eq!(collection.unrecognized_entries, 2);
    f.core
        .validate_resource_collection_witness(&f.root, &collection.witness)
        .unwrap();
    drop(secret);
    f.import("third.bin");
    let after: serde_json::Value =
        serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    assert_eq!(after["extension"]["opaque"], "retained");
    assert_eq!(after["resources"][2]["future"], true);
    assert_eq!(after["resources"].as_array().unwrap().len(), 5);
}

#[test]
fn resource_collection_ambiguous_and_unadmitted_symbols_never_acquire_qualification() {
    let mut f = Fixture::new();
    f.import("owned.bin");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    let original = document["resources"][0].clone();
    document["resources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"resourceId":original["resourceId"],"path":"other.bin"}));
    f.write_manifest(document);
    let collection = f.collection().unwrap();
    assert!(collection.entries.is_empty());
    assert_eq!(collection.unrecognized_entries, 2);
    f.write_manifest(serde_json::json!({"version":1,"resources":[original, {"resourceId":fresh(),"path":"OWNED.bin"}]}));
    let collection = f.collection().unwrap();
    assert!(collection.entries.is_empty());
    assert_eq!(collection.unrecognized_entries, 2);
}

#[test]
fn resource_collection_raw_manifest_replacement_and_explicit_owner_mismatch_fail() {
    let mut f = Fixture::new();
    f.import("owned.bin");
    let collection = f.collection().unwrap();
    let bytes = fs::read(f.manifest()).unwrap();
    fs::rename(f.manifest(), f.root.join("retained-manifest.json")).unwrap();
    fs::write(f.manifest(), &bytes).unwrap();
    assert!(
        f.core
            .validate_resource_collection_witness(&f.root, &collection.witness)
            .is_err()
    );
    assert_eq!(
        f.collection().err().unwrap().code,
        "resourceManifestUnavailable"
    );
    let other = f.core.create_note(&f.root, &f.root, "Other").unwrap();
    assert!(
        f.core
            .qualified_resource_collection_at(&f.root, Path::new(&other.path), &f.owner, "live")
            .is_err()
    );
    fs::remove_file(f.root.join(".weftext-rules")).unwrap();
    assert_eq!(f.collection().err().unwrap().code, "rulesUnavailable");
}

#[test]
fn resource_collection_exact_trash_preserves_original_owner_and_leaf() {
    let mut f = Fixture::new();
    let reference = f.import("owned.bin");
    let live = f.collection().unwrap();
    let plan = f.execute(OperationIntent::TrashNode {
        source: display(f.source.parent().unwrap()),
    });
    let source = f
        .root
        .join(".weftext-trash")
        .join(&plan.operation_id)
        .join("Note.typ");
    assert!(
        f.core
            .validate_resource_collection_witness(&f.root, &live.witness)
            .is_err()
    );
    assert!(
        f.core
            .qualified_resource_collection_at(&f.root, &source, &f.owner, "live")
            .is_err()
    );
    let collection = f
        .core
        .qualified_resource_collection_at(&f.root, &source, &f.owner, "trash")
        .unwrap();
    assert_eq!(collection.entries[0].resource_ref, reference);
    assert_eq!(collection.entries[0].relative_path, "owned.bin");
    f.core
        .validate_resource_collection_witness(&f.root, &collection.witness)
        .unwrap();
    // These bytes are a read witness, never permission to import into Trash.
    assert!(
        f.core
            .prepare_import_resource(&f.root, &source, "new.bin", vec![1], &fresh())
            .is_err()
    );
}

#[test]
fn resource_collection_budget_bounds_all_retained_descriptors_and_imports() {
    let mut f = Fixture::new();
    let entries = (0..64)
        .map(|index| serde_json::json!({"resourceId":fresh(),"path":format!("{index}.bin")}))
        .collect::<Vec<_>>();
    f.write_manifest(serde_json::json!({"version":1,"resources":entries}));
    let collection = f.collection().unwrap();
    assert_eq!(collection.unrecognized_entries, 64);
    let before = fs::read(f.manifest()).unwrap();
    assert_eq!(
        f.core
            .prepare_import_resource(&f.root, &f.source, "new.bin", vec![1], &fresh())
            .unwrap_err()
            .code,
        "budget"
    );
    assert_eq!(before, fs::read(f.manifest()).unwrap());
    let entries = (0..65)
        .map(|index| serde_json::json!({"path":format!("{index}.bin")}))
        .collect::<Vec<_>>();
    f.write_manifest(serde_json::json!({"version":1,"resources":entries}));
    assert_eq!(f.collection().err().unwrap().code, "budget");
}

#[test]
fn resource_collection_original_positive_witnesses_protect_restarted_plan() {
    let mut f = Fixture::new();
    f.import("owned.bin");
    let collection = f.collection().unwrap();
    let mut witnesses = vec![];
    Core::merge_resource_collection_witness(&mut witnesses, &collection.witness).unwrap();
    let original = f
        .core
        .read_snapshot(&f.root, &[display(&f.source)])
        .unwrap()
        .files
        .remove(0);
    let before = original.bytes.unwrap();
    let after = decode(&before).unwrap() + "\nReviewed effect\n";
    let plan = f
        .core
        .prepare_dependent_feature_operation_with_witnesses(
            OperationRequest {
                operation_id: fresh(),
                root: display(&f.root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&f.source),
                        base_id: Some(original.base_id),
                        before: Some(before),
                        after: after.into_bytes(),
                    }],
                },
            },
            &serde_json::json!({"review":"original collection"}),
            &[],
            &witnesses.iter().collect::<Vec<_>>(),
        )
        .unwrap();
    f.restart();
    let before = fs::read(&f.source).unwrap();
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    document["changed_after_review"] = true.into();
    fs::write(f.manifest(), serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .is_err()
    );
    assert_eq!(before, fs::read(&f.source).unwrap());
}

#[test]
fn resource_collection_same_bytes_portable_branch_choice_invalidates_original_revision() {
    let mut f = Fixture::new();
    f.import("owned.bin");
    let identity = f.core.enable_sync(&f.root, &fresh()).unwrap().identity;
    let key = crate::sync::ObjectKey::Node(f.owner.node_id.clone());
    let keys = BTreeSet::from([key.clone()]);
    let objects = f
        .core
        .read_sync_history(&identity)
        .unwrap()
        .objects()
        .unwrap();
    let base = &objects[&key].versions[0];
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    document["same_branches"] = true.into();
    let changed_bytes = serde_json::to_vec(&document).unwrap();
    let digest = hash(&changed_bytes);
    let first_revision = fresh();
    let second_revision = fresh();
    for (index, revision) in [&first_revision, &second_revision].into_iter().enumerate() {
        let mut state = base.state.clone();
        state
            .files
            .insert("weftext.resources.json".into(), digest.clone());
        state
            .file_revisions
            .insert("weftext.resources.json".into(), revision.clone());
        let manifest = crate::sync::Manifest {
            version: 1,
            workspace_id: identity.workspace_id.clone(),
            replica_id: fresh(),
            operation_id: fresh(),
            parents: BTreeSet::from([base.commit.clone()]),
            changes: vec![crate::sync::Change {
                object: key.clone(),
                before: BTreeSet::from([base.revision.clone()]),
                revision: fresh(),
                after: state,
            }],
            dependencies: vec![],
        };
        f.core
            .receive_sync(
                &identity,
                crate::sync::SyncBundle {
                    manifests: BTreeMap::from([(manifest.id().unwrap(), manifest)]),
                    blobs: BTreeMap::from([(digest.clone(), changed_bytes.clone())]),
                },
            )
            .unwrap();
        if index == 0 {
            f.execute(OperationIntent::ApplySync {
                identity: identity.clone(),
                objects: keys.clone(),
            });
        }
    }
    let original = f.collection().unwrap();
    assert_eq!(original.revision.as_ref(), Some(&first_revision));
    let objects = f
        .core
        .read_sync_history(&identity)
        .unwrap()
        .objects()
        .unwrap();
    let sides = &objects[&key];
    let state = sides
        .versions
        .iter()
        .find(|version| version.state.file_revisions["weftext.resources.json"] == second_revision)
        .unwrap()
        .state
        .clone();
    f.execute(OperationIntent::MergeSync {
        identity,
        resolutions: vec![crate::sync::SyncResolution {
            object: key,
            revisions: sides
                .versions
                .iter()
                .map(|version| version.revision.clone())
                .collect(),
            state,
        }],
        blobs: BTreeMap::new(),
    });
    f.restart();
    f.core
        .validate_snapshot_witness(&f.root, f.manifest(), &original.witness.manifest)
        .unwrap();
    assert_eq!(
        f.core
            .validate_resource_collection_witness(&f.root, &original.witness)
            .unwrap_err()
            .code,
        "resourceVersion"
    );
    let current = f.collection().unwrap();
    assert_eq!(current.revision.as_ref(), Some(&second_revision));
    assert_eq!(
        current.entries[0].resource_ref,
        original.entries[0].resource_ref
    );
}
