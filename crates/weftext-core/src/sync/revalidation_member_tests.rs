//! Real external observations, not hand-authored portable history versions.
use super::*;
use crate::fresh;
use std::fs;

struct Fixture {
    temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    data: PathBuf,
    identity: SyncIdentity,
    node: ObjectKey,
    source: PathBuf,
    asset: PathBuf,
    manifest: PathBuf,
    retained: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let data = temp.path().join("private");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note(&root, &root, "Note").unwrap();
        let source = PathBuf::from(&note.path);
        let imported = core
            .prepare_import_resource(
                &root,
                &source,
                "asset.bin",
                b"original registered bytes".to_vec(),
                &fresh(),
            )
            .unwrap();
        core.execute_operation(&imported.operation_id, &imported.plan_id)
            .unwrap();
        let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
        Self {
            asset: root.join("Note").join("asset.bin"),
            manifest: root.join("Note").join("weftext.resources.json"),
            retained: temp.path().join("retained-original-asset.bin"),
            temp,
            core,
            root,
            data,
            identity,
            node: ObjectKey::Node(note.node_id.unwrap()),
            source,
        }
    }
    fn observe_removed_resource(&self, update_manifest: bool) {
        let original = capture_optional(&self.asset).unwrap().unwrap();
        fs::rename(&self.asset, &self.retained).unwrap();
        assert_eq!(
            capture_optional(&self.retained)
                .unwrap()
                .unwrap()
                .stamp
                .identity,
            original.stamp.identity
        );
        if update_manifest {
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(&self.manifest).unwrap()).unwrap();
            manifest["resources"]
                .as_array_mut()
                .unwrap()
                .retain(|entry| entry["path"] != "asset.bin");
            fs::write(&self.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
        }
    }
    fn cut(&mut self) -> SyncAdoptionCut {
        self.core
            .read_sync_adoption_cut(
                &self.root,
                &BTreeSet::from([self.node.clone(), ObjectKey::Rules]),
            )
            .unwrap()
    }
    fn request(&self, cut: SyncAdoptionCut) -> OperationRequest {
        let mut intent = serde_json::to_value(cut).unwrap();
        intent["kind"] = "adoptSyncExternal".into();
        OperationRequest {
            operation_id: fresh(),
            root: display(&self.root),
            intent: serde_json::from_value(intent).unwrap(),
        }
    }
}

#[test]
fn sync_adoption_removed_registered_member_is_a_real_no_write_portable_successor() {
    let mut f = Fixture::new();
    let original = f
        .core
        .read_sync_history(&f.identity)
        .unwrap()
        .objects()
        .unwrap()[&f.node]
        .versions[0]
        .clone();
    f.observe_removed_resource(true);
    let manifest = capture_optional(&f.manifest).unwrap().unwrap();
    let cut = f.cut();
    let json = serde_json::to_value(&cut).unwrap();
    assert_eq!(json["removedMembers"].as_array().unwrap().len(), 1);
    assert_eq!(json["removedMembers"][0]["path"], display(&f.asset));
    assert!(
        json["removedMembers"][0]["baseId"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    let request = f.request(cut);
    let preview = f.core.prepare_operation(request.clone()).unwrap();
    assert!(!f.asset.exists());
    assert_eq!(
        capture_optional(&f.manifest).unwrap().unwrap().stamp,
        manifest.stamp
    );
    assert_eq!(fs::read(&f.retained).unwrap(), b"original registered bytes");
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core.prepare_operation(request).unwrap().plan_id,
        preview.plan_id
    );
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    f.core.publish_sync(&f.root).unwrap();
    assert!(!f.asset.exists());
    assert_eq!(
        capture_optional(&f.manifest).unwrap().unwrap().stamp,
        manifest.stamp
    );
    let history = f.core.read_sync_history(&f.identity).unwrap();
    let objects = history.objects().unwrap();
    let [after] = objects[&f.node].versions.as_slice() else {
        panic!("One original adoption publishes one causal successor");
    };
    assert!(!after.state.files.contains_key("asset.bin"));
    assert!(!after.state.file_revisions.contains_key("asset.bin"));
    assert_eq!(
        history.blob(&original.state.files["asset.bin"]).unwrap(),
        b"original registered bytes"
    );
    assert_eq!(
        after.state.files["Note.typ"],
        original.state.files["Note.typ"]
    );
    assert_eq!(
        after.state.file_revisions["Note.typ"],
        original.state.file_revisions["Note.typ"]
    );
    assert_eq!(fs::read(f.retained).unwrap(), b"original registered bytes");
}

#[test]
fn sync_adoption_removed_member_requires_original_owner_and_accurate_after_manifest() {
    for missing_source in [false, true] {
        let mut f = Fixture::new();
        if missing_source {
            fs::rename(&f.source, f.temp.path().join("retained-original-note.typ")).unwrap();
        } else {
            f.observe_removed_resource(false);
        }
        let operations = f.core.operations.len();
        assert_eq!(
            f.core
                .read_sync_adoption_cut(
                    &f.root,
                    &BTreeSet::from([f.node.clone(), ObjectKey::Rules])
                )
                .unwrap_err()
                .code,
            "syncExternalChange"
        );
        assert_eq!(f.core.operations.len(), operations);
    }
    for replacement in ["resource", "source", "manifest"] {
        let mut f = Fixture::new();
        if replacement != "resource" {
            f.observe_removed_resource(true);
        }
        let path = match replacement {
            "resource" => &f.asset,
            "source" => &f.source,
            "manifest" => &f.manifest,
            _ => unreachable!(),
        };
        let bytes = fs::read(path).unwrap();
        fs::rename(
            path,
            f.temp
                .path()
                .join(format!("retained-replaced-{replacement}")),
        )
        .unwrap();
        fs::write(path, &bytes).unwrap();
        assert_eq!(
            f.core
                .read_sync_adoption_cut(
                    &f.root,
                    &BTreeSet::from([f.node.clone(), ObjectKey::Rules])
                )
                .unwrap_err()
                .code,
            "identityConflict",
            "{replacement}"
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn sync_adoption_removed_member_reappearing_after_review_does_not_gain_an_absence_authority() {
    for new_file in [false, true] {
        let mut f = Fixture::new();
        f.observe_removed_resource(true);
        let cut = f.cut();
        let request = f.request(cut);
        if new_file {
            fs::write(&f.asset, fs::read(&f.retained).unwrap()).unwrap();
        } else {
            fs::rename(&f.retained, &f.asset).unwrap();
        }
        let observed = capture_optional(&f.asset).unwrap().unwrap();
        assert_eq!(
            f.core.prepare_operation(request.clone()).unwrap_err().code,
            "staleBase"
        );
        assert!(!f.core.operations.contains_key(&request.operation_id));
        assert_eq!(
            capture_optional(&f.asset).unwrap().unwrap().stamp,
            observed.stamp
        );
    }
}

#[test]
fn sync_adoption_removed_members_are_explicit_unique_and_bound_to_the_exact_selected_absence() {
    for mutation in [
        "omitted",
        "duplicate",
        "alias",
        "overlap",
        "wrongBase",
        "outsideCut",
    ] {
        let mut f = Fixture::new();
        f.observe_removed_resource(true);
        let cut = f.cut();
        let mut input = serde_json::to_value(cut).unwrap();
        let member = input["removedMembers"][0].clone();
        let expected = match mutation {
            "omitted" => {
                input.as_object_mut().unwrap().remove("removedMembers");
                "syncExternalChange"
            }
            "duplicate" | "alias" => {
                let mut other = member.clone();
                if mutation == "alias" {
                    other["path"] = display(&f.asset).to_ascii_uppercase().into();
                }
                input["removedMembers"].as_array_mut().unwrap().push(other);
                "invalidSync"
            }
            "overlap" => {
                input["changes"].as_array_mut().unwrap().push(serde_json::json!({"path":member["path"],"baseId":member["baseId"],"before":[],"after":[]}));
                "invalidSync"
            }
            "wrongBase" => {
                input["removedMembers"][0]["baseId"] = input["changes"][0]["baseId"].clone();
                "staleBase"
            }
            "outsideCut" => {
                input["expectedInstalled"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|entry| entry["object"]["kind"] != "node");
                "syncAdoptionScope"
            }
            _ => unreachable!(),
        };
        input["kind"] = "adoptSyncExternal".into();
        let request = OperationRequest {
            operation_id: fresh(),
            root: display(&f.root),
            intent: serde_json::from_value(input).unwrap(),
        };
        let operations = f.core.operations.len();
        assert_eq!(
            f.core.prepare_operation(request.clone()).unwrap_err().code,
            expected,
            "{mutation}"
        );
        assert_eq!(f.core.operations.len(), operations);
        assert!(!f.core.operations.contains_key(&request.operation_id));
        assert!(!f.asset.exists());
        assert_eq!(fs::read(f.retained).unwrap(), b"original registered bytes");
    }
}

#[test]
fn sync_adoption_removed_member_original_prepared_and_recovery_keep_negative_slot_and_parent() {
    for install_only in [false, true] {
        for mutation in ["sameFile", "newFile", "parent"] {
            let mut f = Fixture::new();
            f.observe_removed_resource(true);
            let cut = f.cut();
            let request = f.request(cut);
            let preview = f.core.prepare_operation(request.clone()).unwrap();
            if install_only {
                f.core
                    .install_record(&f.core.operations[&preview.operation_id])
                    .unwrap();
            }
            let source = capture_optional(&f.source).unwrap().unwrap();
            let manifest = capture_optional(&f.manifest).unwrap().unwrap();
            match mutation {
                "sameFile" => fs::rename(&f.retained, &f.asset).unwrap(),
                "newFile" => fs::write(&f.asset, fs::read(&f.retained).unwrap()).unwrap(),
                "parent" => {
                    let parent = f.source.parent().unwrap();
                    let retained_parent = f.temp.path().join("retained-original-owner");
                    fs::rename(parent, &retained_parent).unwrap();
                    fs::create_dir(parent).unwrap();
                    for entry in fs::read_dir(&retained_parent).unwrap() {
                        let entry = entry.unwrap();
                        fs::rename(entry.path(), parent.join(entry.file_name())).unwrap();
                    }
                }
                _ => unreachable!(),
            }
            drop(f.core);
            f.core = Core::new(&f.data).unwrap();
            assert_eq!(
                serde_json::to_value(
                    f.core
                        .sync_operation_request(&preview.operation_id)
                        .unwrap()
                )
                .unwrap(),
                serde_json::to_value(&request).unwrap()
            );
            if install_only {
                let recovery = f.core.inspect_recovery(&preview.operation_id).unwrap();
                assert!(
                    recovery.allowed_actions.is_empty(),
                    "{mutation}: {}",
                    serde_json::to_string(&recovery).unwrap()
                );
                assert!(recovery.members.iter().any(|member| same(
                    Path::new(&member.path),
                    &f.asset
                ) && !member.matches_before
                    && !member.matches_after));
                for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
                    assert!(
                        f.core
                            .resolve_recovery(&preview.operation_id, action)
                            .is_err(),
                        "{mutation}/{action}"
                    );
                }
            } else {
                assert_eq!(
                    f.core
                        .execute_operation(&preview.operation_id, &preview.plan_id)
                        .unwrap_err()
                        .code,
                    "scopeChanged",
                    "{mutation}"
                );
                assert!(
                    !f.core
                        .journal_path(&preview.operation_id, "install")
                        .exists()
                );
            }
            assert!(!f.core.committed(&preview.operation_id));
            assert!(!f.core.resolved(&preview.operation_id));
            let current_source = capture_optional(&f.source).unwrap().unwrap();
            let current_manifest = capture_optional(&f.manifest).unwrap().unwrap();
            assert_eq!(current_source.bytes, source.bytes);
            assert_eq!(current_source.stamp.identity, source.stamp.identity);
            assert_eq!(current_manifest.bytes, manifest.bytes);
            assert_eq!(current_manifest.stamp.identity, manifest.stamp.identity);
            if mutation == "parent" {
                assert!(!f.asset.exists());
            } else {
                assert_eq!(fs::read(&f.asset).unwrap(), b"original registered bytes");
            }
        }
    }
}

#[test]
fn sync_adoption_removed_member_original_pending_namespace_writer_prevents_commit_and_recovery() {
    for install_original in [false, true] {
        let mut f = Fixture::new();
        f.observe_removed_resource(true);
        let cut = f.cut();
        let request = f.request(cut);
        let original = f.core.prepare_operation(request).unwrap();
        let other = f
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&f.root),
                intent: OperationIntent::NewNode {
                    parent: display(f.source.parent().unwrap()),
                    name: "asset.bin".into(),
                    source: None,
                },
            })
            .unwrap();
        let other_plan = &f.core.operations[&other.operation_id];
        let original_t = f.core.operations[&original.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        let other_t = other_plan.transaction.as_ref().unwrap();
        assert!(
            matches!(&other_t.mutation, crate::operations::Mutation::CreateTree { target, .. } if same(Path::new(target), &f.asset))
        );
        assert!(
            original_t
                .absent_dependencies
                .iter()
                .any(|negative| same(Path::new(&negative.path), &f.asset))
        );
        assert!(
            !original_t
                .files
                .iter()
                .any(|file| Path::new(&file.path).starts_with(&f.asset))
        );
        assert!(
            other_t
                .created_files()
                .iter()
                .all(|file| Path::new(&file.path).starts_with(&f.asset))
        );
        assert!(!other_t.created_files().is_empty());
        f.core.install_record(other_plan).unwrap();
        if install_original {
            f.core
                .install_record(&f.core.operations[&original.operation_id])
                .unwrap();
        }
        let source = fs::read(&f.source).unwrap();
        let manifest = fs::read(&f.manifest).unwrap();
        drop(f.core);
        f.core = Core::new(&f.data).unwrap();
        if install_original {
            assert!(
                f.core
                    .inspect_recovery(&original.operation_id)
                    .unwrap()
                    .allowed_actions
                    .is_empty()
            );
            for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
                assert!(
                    f.core
                        .resolve_recovery(&original.operation_id, action)
                        .is_err()
                );
            }
        } else {
            assert_eq!(
                f.core
                    .execute_operation(&original.operation_id, &original.plan_id)
                    .unwrap_err()
                    .code,
                "recoveryPending"
            );
            assert!(
                !f.core
                    .journal_path(&original.operation_id, "install")
                    .exists()
            );
        }
        assert!(!f.core.committed(&original.operation_id));
        assert!(!f.core.committed(&other.operation_id));
        assert!(!f.asset.exists());
        assert_eq!(fs::read(&f.source).unwrap(), source);
        assert_eq!(fs::read(&f.manifest).unwrap(), manifest);
        assert_eq!(fs::read(&f.retained).unwrap(), b"original registered bytes");
    }
}
