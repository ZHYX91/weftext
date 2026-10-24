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
fn current_sync_file_readers_reject_missing_original_participant_without_affecting_foreign_root() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let a = temp.path().join("A");
    let b = temp.path().join("B");
    core.create_library(&a).unwrap();
    let note = core.create_note_fixture(&a, &a, "Note").unwrap();
    core.enable_sync(&a, &fresh()).unwrap();
    let doc = core.open(&note.path, Some(a.clone())).unwrap();
    let id = fresh();
    core.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        source: doc.source + "\noriginal S",
        operation_id: id.clone(),
    })
    .unwrap();
    let original = core.operations[&id].clone();
    assert!(original.sync_publication.is_some());
    let commit_path = core.journal_path(&id, "commit");
    let commit_bytes = fs::read(&commit_path).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&commit_bytes).unwrap();
    old.as_object_mut().unwrap().remove("fileSave");
    let old_commit = serde_json::to_vec(&old).unwrap();
    fs::write(&commit_path, &old_commit).unwrap();
    core.operations.get_mut(&id).unwrap().file_save = None;
    assert!(core.committed(&id).unwrap());
    assert_eq!(
        core.sync_publications(&display(&a)).unwrap_err().code,
        "unsupportedFileSaveEvidence"
    );
    assert!(
        matches!(core.sync_local_state(core.sync_setup_for(&a).unwrap()), Err(error) if error.code == "unsupportedFileSaveEvidence")
    );
    core.create_library(&b).unwrap();
    core.enable_sync(&b, &fresh()).unwrap();
    assert!(core.sync_publications(&display(&b)).unwrap().is_empty());
    assert_eq!(fs::read(&commit_path).unwrap(), old_commit);
    core.operations.insert(id, original);
    fs::write(commit_path, commit_bytes).unwrap();
    assert_eq!(core.sync_publications(&display(&a)).unwrap().len(), 1);
}

#[test]
fn reused_root_does_not_publish_new_library_or_ordinary_save_into_original_foreign_replica() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let root = temp.path().join("root");
    core.create_library(&root).unwrap();
    core.create_note_fixture(&root, &root, "Old").unwrap();
    let a = core.enable_sync(&root, &fresh()).unwrap();
    let a_workspace = a.identity.workspace_id;
    fs::rename(&root, temp.path().join("retained-A")).unwrap();
    let created = fresh();
    core.create_library_with_id(&root, &created).unwrap();
    assert!(core.operations[&created].sync_publication.is_none());
    let path = root.join("ordinary.typ");
    fs::write(&path, "B").unwrap();
    let doc = core.open(&path, None).unwrap();
    let id = fresh();
    core.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        source: "B original source".into(),
        operation_id: id.clone(),
    })
    .unwrap();
    assert!(core.operations[&id].sync_publication.is_none());
    assert_ne!(
        core.require_workspace(&root).unwrap().workspace_id,
        a_workspace
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "B original source");
}

#[test]
fn sync_setup_releases_private_captures_and_retries_owned_original_bytes_after_restart() {
    for installed_tags in [false, true] {
        for blocker in [None, Some("plan"), Some("commit")] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("library");
            let data = temp.path().join("private");
            let mut core = Core::new(&data).unwrap();
            core.create_library(&root).unwrap();
            let note = core.create_note_fixture(&root, &root, "Note").unwrap();
            let retired = core.create_note_fixture(&root, &root, "Retired").unwrap();
            operation(
                &mut core,
                &root,
                OperationIntent::TrashNode {
                    source: display(&root.join("Retired")),
                },
            );
            if installed_tags {
                let tags = core.read_tag_registry(&root).unwrap();
                let prepared = core
                    .prepare_create_tag(&root, "Root", &tags.base_id, &fresh())
                    .unwrap();
                core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
                    .unwrap();
            }
            let independent = core.admitted_snapshot(&root).unwrap();
            let bases = core.snapshots.keys().cloned().collect::<BTreeSet<_>>();
            let cut_count = core.admitted_cuts.len();
            let compile_count = core.compile_dependency_cuts.len();
            let enable = fresh();
            let blocked_path = blocker.map(|kind| core.journal_path(&enable, kind));
            if let Some(path) = &blocked_path {
                fs::create_dir(path).unwrap();
            }
            let first = core.enable_sync(&root, &enable);
            if blocker.is_some() {
                assert!(first.is_err());
            } else {
                first.unwrap();
            }
            assert_eq!(
                core.snapshots.keys().cloned().collect::<BTreeSet<_>>(),
                bases
            );
            assert_eq!(core.admitted_cuts.len(), cut_count);
            assert_eq!(core.compile_dependency_cuts.len(), compile_count);
            let original_source = &independent.files[0];
            core.validate_snapshot(&root, &original_source.path, &original_source.base_id)
                .unwrap();
            let original_plan = fs::read(core.journal_path(&enable, "plan")).ok();
            if let Some(path) = blocked_path {
                fs::remove_dir(path).unwrap();
            }
            drop(core);
            let mut core = Core::new(&data).unwrap();
            let info = core.enable_sync(&root, &enable).unwrap();
            assert!(core.snapshots.is_empty());
            assert!(core.admitted_cuts.is_empty());
            assert!(core.compile_dependency_cuts.is_empty());
            if let Some(original) = original_plan {
                assert_eq!(
                    fs::read(core.journal_path(&enable, "plan")).unwrap(),
                    original
                );
            }
            let objects = core
                .read_sync_history(&info.identity)
                .unwrap()
                .objects()
                .unwrap();
            assert!(
                !objects[&ObjectKey::Node(note.node_id.unwrap())].versions[0]
                    .state
                    .retired
            );
            assert!(
                objects[&ObjectKey::Node(retired.node_id.unwrap())].versions[0]
                    .state
                    .retired
            );
            assert_eq!(objects.contains_key(&ObjectKey::Tags), installed_tags);
        }
    }
}

#[test]
fn sync_setup_cannot_launder_forged_replaced_or_lost_tag_registry_identity() {
    for change in ["forged", "modified", "replacement", "missing"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let path = root.join(".weftext-tags.json");
        if change == "forged" {
            fs::write(
                &path,
                serde_json::to_vec(&serde_json::json!({
                    "version": 2,
                    "tags": [{"tagId": fresh(), "displayName": "Forged", "parentTagId": null}]
                }))
                .unwrap(),
            )
            .unwrap();
        } else {
            let tags = core.read_tag_registry(&root).unwrap();
            let prepared = core
                .prepare_create_tag(&root, "Original", &tags.base_id, &fresh())
                .unwrap();
            core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
                .unwrap();
            let raw = fs::read(&path).unwrap();
            match change {
                "modified" => {
                    let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                    value["tags"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::json!({
                            "tagId": fresh(), "displayName": "Unadmitted", "parentTagId": null
                        }));
                    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
                }
                "replacement" | "missing" => {
                    fs::rename(&path, temp.path().join("retained-original-tags.json")).unwrap();
                    if change == "replacement" {
                        fs::write(&path, raw).unwrap();
                    }
                }
                _ => unreachable!(),
            }
        }
        let original = path.exists().then(|| fs::read(&path).unwrap());
        let bases = core.snapshots.keys().cloned().collect::<BTreeSet<_>>();
        let cut_count = core.admitted_cuts.len();
        let operation_count = core.operations.len();
        let enable = fresh();
        let error = core.enable_sync(&root, &enable).unwrap_err();
        assert!(matches!(
            error.code.as_str(),
            "externalPending" | "identityConflict"
        ));
        assert_eq!(core.operations.len(), operation_count);
        assert!(!core.journal_path(&enable, "plan").exists());
        assert!(
            core.sync_setups()
                .unwrap()
                .iter()
                .all(|setup| setup.root != display(&root))
        );
        assert_eq!(
            core.snapshots.keys().cloned().collect::<BTreeSet<_>>(),
            bases
        );
        assert_eq!(core.admitted_cuts.len(), cut_count);
        assert!(core.compile_dependency_cuts.is_empty());
        assert_eq!(path.exists().then(|| fs::read(&path).unwrap()), original);
        assert_eq!(fs::read_to_string(&note.path).unwrap(), note.source);
    }
}

#[test]
fn rejected_incomplete_sync_setup_does_not_leave_private_cut_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let independent = core.admitted_snapshot(&root).unwrap();
    let bases = core.snapshots.keys().cloned().collect::<BTreeSet<_>>();
    let cut_count = core.admitted_cuts.len();
    fs::write(
        &note.path,
        note.source
            .replace(note.node_id.as_ref().unwrap(), &fresh()),
    )
    .unwrap();
    assert_eq!(
        core.enable_sync(&root, &fresh()).unwrap_err().code,
        "coverageIncomplete"
    );
    assert_eq!(
        core.snapshots.keys().cloned().collect::<BTreeSet<_>>(),
        bases
    );
    assert_eq!(core.admitted_cuts.len(), cut_count);
    assert!(core.admitted_cuts.contains_key(&independent.cut_id));
    assert!(core.compile_dependency_cuts.is_empty());
}

#[test]
fn saved_offline_versions_publish_original_outbox_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
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
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    core.create_note_fixture(&root, &root, "Unrelated").unwrap();
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
    let parent = core.create_note_fixture(&root, &root, "Parent").unwrap();
    let child = core
        .create_note_fixture(&root, root.join("Parent"), "Child")
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
    let parent = core.create_note_fixture(&root, &root, "Parent").unwrap();
    let child = core
        .create_note_fixture(&root, root.join("Parent"), "Child")
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
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
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
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
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
    assert!(core.committed(&pending[0]).unwrap());
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
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
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
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
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

#[cfg(windows)]
#[test]
fn sync_setup_selection_qualifies_only_actual_setup_candidates() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let note = temp.path().join("ordinary.typ");
    fs::write(&note, "B").unwrap();
    let doc = core.open(&note, None).unwrap();
    let id = fresh();
    core.save(SaveRequest {
        operation_id: id.clone(),
        session_id: doc.session_id,
        base_id: doc.base_id,
        source: "S".into(),
    })
    .unwrap();
    fs::write(
        core.journal_path(&id, "commit"),
        b"retained damaged unrelated commit",
    )
    .unwrap();
    assert!(core.sync_setups().unwrap().is_empty());
    assert!(core.local_operation_status(&id).is_err());
    assert_eq!(fs::read_to_string(note).unwrap(), "S");
    // A real setup remains an actual dependency; malformed original evidence
    // is never replaced by a cached successful qualification.
    let other = tempfile::tempdir().unwrap();
    let mut core = Core::new(other.path().join("private")).unwrap();
    let root = other.path().join("library");
    core.create_library(&root).unwrap();
    let id = fresh();
    core.enable_sync(&root, &id).unwrap();
    assert_eq!(core.sync_setups().unwrap().len(), 1);
    fs::write(
        core.journal_path(&id, "commit"),
        b"retained damaged setup commit",
    )
    .unwrap();
    assert!(core.sync_setups().is_err());
}
