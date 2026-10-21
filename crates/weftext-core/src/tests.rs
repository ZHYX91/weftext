use super::*;
use std::os::windows::fs::OpenOptionsExt;

struct Fixture {
    core: Core,
    temp: tempfile::TempDir,
    data: PathBuf,
    path: PathBuf,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let path = temp.path().join("plain.typ");
        fs::write(&path, source.as_bytes()).unwrap();
        let core = Core::new(&data).unwrap();
        Self {
            temp,
            data,
            path,
            core,
        }
    }
    fn open(&mut self) -> OpenedDocument {
        self.core.open(&self.path, None).unwrap()
    }
}
fn request(doc: &OpenedDocument, source: &str) -> SaveRequest {
    SaveRequest {
        session_id: doc.session_id.clone(),
        base_id: doc.base_id.clone(),
        source: source.into(),
        operation_id: fresh(),
    }
}

#[test]
fn temporary_admitted_membership_checks_release_cuts_on_rejection_commit_and_restart() {
    let mut f = Fixture::new("plain");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    f.core.create_note_fixture(&root, &root, "Note").unwrap();
    let cut = f.core.admitted_snapshot(&root).unwrap();
    let make_request = |name: &str| OperationRequest {
        operation_id: fresh(),
        root: display(&root),
        intent: OperationIntent::NewNode {
            parent: display(&root),
            name: name.into(),
            source: None,
        },
    };
    let bases = f
        .core
        .snapshots
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let cuts = f.core.admitted_cuts.len();
    let preview = f
        .core
        .prepare_admitted_operation(make_request("First"), &cut.cut_id)
        .unwrap();
    assert_eq!(f.core.admitted_cuts.len(), cuts);
    assert_eq!(
        f.core
            .snapshots
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        bases
    );
    // The original prepared Plan owns its evidence across restart; temporary
    // membership-check captures are neither original input nor new authority.
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert!(f.core.snapshots.is_empty());
    assert!(f.core.admitted_cuts.is_empty());
    assert!(root.join("First/First.typ").exists());

    let cut = f.core.admitted_snapshot(&root).unwrap();
    let late = f
        .core
        .prepare_admitted_operation(make_request("Late"), &cut.cut_id)
        .unwrap();
    f.core
        .create_note_fixture(&root, &root, "Concurrent")
        .unwrap();
    let bases = f
        .core
        .snapshots
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let cuts = f.core.admitted_cuts.len();
    let operation_count = f.core.operations.len();
    assert_eq!(
        f.core
            .prepare_admitted_operation(make_request("Rejected"), &cut.cut_id)
            .unwrap_err()
            .code,
        "scopeChanged"
    );
    assert_eq!(
        f.core
            .execute_operation(&late.operation_id, &late.plan_id)
            .unwrap_err()
            .code,
        "scopeChanged"
    );
    assert_eq!(f.core.operations.len(), operation_count);
    assert_eq!(f.core.admitted_cuts.len(), cuts);
    assert_eq!(
        f.core
            .snapshots
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        bases
    );
    let original_source = &cut.files[0];
    f.core
        .validate_snapshot(&root, &original_source.path, &original_source.base_id)
        .unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core
            .execute_operation(&late.operation_id, &late.plan_id)
            .unwrap_err()
            .code,
        "scopeChanged"
    );
    assert!(f.core.snapshots.is_empty());
    assert!(f.core.admitted_cuts.is_empty());
    assert!(!root.join("Late").exists());
    assert!(!root.join("Rejected").exists());
}

#[test]
fn owned_read_witness_releases_core_storage_but_keeps_exact_validation() {
    let mut f = Fixture::new("plain");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let doc = f.core.create_note_fixture(&root, &root, "Note").unwrap();
    let baseline = f.core.snapshots.len();
    let mut last = None;
    for _ in 0..40 {
        let snapshot = f
            .core
            .read_snapshot(&root, std::slice::from_ref(&doc.path))
            .unwrap()
            .files
            .remove(0);
        let witness = f.core.take_snapshot_witness(&snapshot.base_id).unwrap();
        assert_eq!(f.core.snapshots.len(), baseline);
        assert!(f.core.take_snapshot_witness(&snapshot.base_id).is_err());
        assert!(
            f.core
                .validate_snapshot(&root, &snapshot.path, &snapshot.base_id)
                .is_err()
        );
        f.core
            .validate_snapshot_witness(&root, &snapshot.path, &witness)
            .unwrap();
        last = Some(witness);
    }
    let witness = last.unwrap();
    assert!(
        f.core
            .validate_snapshot_witness(&root, &f.path, &witness)
            .is_err()
    );
    f.core
        .save(request(&doc, &(doc.source.clone() + "\nnew save")))
        .unwrap();
    assert!(
        f.core
            .validate_snapshot_witness(&root, &doc.path, &witness)
            .is_err()
    );
    drop(witness);
    assert_eq!(f.core.snapshots.len(), baseline);
}

#[test]
fn failed_bounded_read_does_not_register_partial_snapshot_bytes() {
    let mut f = Fixture::new("plain");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let doc = f.core.create_note_fixture(&root, &root, "Note").unwrap();
    let huge = root.join("large-owned-fixture.bin");
    fs::File::create(&huge)
        .unwrap()
        .set_len(64 * 1024 * 1024)
        .unwrap();
    let baseline = f.core.snapshots.len();
    let error = f
        .core
        .read_snapshot(&root, &[doc.path.clone(), display(&huge)])
        .err()
        .unwrap();
    assert_eq!(error.code, "budget");
    assert_eq!(f.core.snapshots.len(), baseline);
    assert_eq!(fs::read_to_string(&doc.path).unwrap(), doc.source);
}

fn operation(core: &mut Core, root: &Path, intent: OperationIntent) -> OperationReceipt {
    let p = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(root),
            intent,
        })
        .unwrap();
    core.execute_operation(&p.operation_id, &p.plan_id)
        .expect(&p.kind)
}

#[test]
fn guarded_rename_move_trash_restore_preserve_node_and_physical_identity() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let note = f
        .core
        .create_note_fixture(&root, &root, "Original")
        .unwrap();
    let file_id = f.core.sessions[&note.session_id].stamp.identity.clone();
    let renamed = operation(
        &mut f.core,
        &root,
        OperationIntent::RenameNode {
            source: display(&root.join("Original")),
            name: "Renamed".into(),
        },
    );
    assert_eq!(renamed.node_ids, vec![note.node_id.clone().unwrap()]);
    assert!(!root.join("Original").exists());
    let opened = f
        .core
        .open(root.join("Renamed/Renamed.typ"), Some(root.clone()))
        .unwrap();
    assert_eq!(opened.node_id, note.node_id);
    assert_eq!(f.core.sessions[&opened.session_id].stamp.identity, file_id);
    fs::create_dir(root.join("Container")).unwrap();
    operation(
        &mut f.core,
        &root,
        OperationIntent::MoveNode {
            source: display(&root.join("Renamed")),
            parent: display(&root.join("Container")),
        },
    );
    let trash = operation(
        &mut f.core,
        &root,
        OperationIntent::TrashNode {
            source: display(&root.join("Container/Renamed")),
        },
    );
    assert!(Path::new(&trash.path).join("Renamed.typ").exists());
    let trashed = f
        .core
        .open(
            Path::new(&trash.path).join("Renamed.typ"),
            Some(root.clone()),
        )
        .unwrap();
    assert_eq!(trashed.kind, "managedTrash");
    assert_eq!(
        f.core
            .save(request(&trashed, &trashed.source))
            .unwrap_err()
            .code,
        "trashReadOnly"
    );
    assert_eq!(f.core.trash_entries(&root).unwrap().len(), 1);
    operation(
        &mut f.core,
        &root,
        OperationIntent::RestoreNode {
            trash_operation_id: trash.operation_id,
            node_id: None,
            parent: display(&root),
            name: "Restored".into(),
        },
    );
    assert!(f.core.trash_entries(&root).unwrap().is_empty());
    let restored = f
        .core
        .open(root.join("Restored/Restored.typ"), Some(root.clone()))
        .unwrap();
    assert_eq!(restored.node_id, note.node_id);
    assert_eq!(
        f.core.sessions[&restored.session_id].stamp.identity,
        file_id
    );
}

#[test]
fn frozen_plan_revalidation_no_replace_and_exact_retry() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let note = f
        .core
        .create_note_fixture(&root, &root, "Original")
        .unwrap();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&root),
        intent: OperationIntent::RenameNode {
            source: display(&root.join("Original")),
            name: "Target".into(),
        },
    };
    let plan = f.core.prepare_operation(request.clone()).unwrap();
    assert!(!root.join("Target").exists());
    assert_eq!(
        f.core.prepare_operation(request).unwrap().plan_id,
        plan.plan_id
    );
    fs::create_dir(root.join("Target")).unwrap();
    assert_eq!(
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap_err()
            .code,
        "exists"
    );
    assert!(Path::new(&note.path).exists());
    assert_eq!(
        f.core.operation_details(&plan.operation_id).unwrap().status,
        "prepared"
    );
    let good = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::RenameNode {
                source: display(&root.join("Original")),
                name: "Good".into(),
            },
        })
        .unwrap();
    let receipt = f
        .core
        .execute_operation(&good.operation_id, &good.plan_id)
        .unwrap();
    let replay = f
        .core
        .execute_operation(&good.operation_id, &good.plan_id)
        .unwrap();
    assert_eq!(receipt.path, replay.path);
    assert_eq!(
        f.core
            .execute_operation(&good.operation_id, &fresh())
            .unwrap_err()
            .code,
        "planConflict"
    );
}

#[test]
fn same_bytes_replacement_invalidates_snapshot_and_multi_file_reader_barrier() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let note = f.core.create_note_fixture(&root, &root, "Note").unwrap();
    let source_path = PathBuf::from(&note.path);
    let annotations = source_path
        .parent()
        .unwrap()
        .join("weftext.annotations.json");
    let cut = f
        .core
        .read_snapshot(&root, &[note.path.clone(), display(&annotations)])
        .unwrap();
    fs::rename(&source_path, source_path.with_extension("old")).unwrap();
    fs::write(&source_path, cut.files[0].bytes.as_ref().unwrap()).unwrap();
    let bad = OperationRequest {
        operation_id: fresh(),
        root: display(&root),
        intent: OperationIntent::UpdateFiles {
            changes: vec![FileChange {
                path: note.path.clone(),
                base_id: Some(cut.files[0].base_id.clone()),
                before: cut.files[0].bytes.clone(),
                after: cut.files[0].bytes.clone().unwrap(),
            }],
        },
    };
    assert!(matches!(
        f.core.prepare_operation(bad).unwrap_err().code.as_str(),
        "identityConflict" | "staleBase"
    ));
    fs::remove_file(&source_path).unwrap();
    fs::rename(source_path.with_extension("old"), &source_path).unwrap();
    let cut = f
        .core
        .read_snapshot(&root, &[note.path.clone(), display(&annotations)])
        .unwrap();
    let p = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![
                    FileChange {
                        path: note.path.clone(),
                        base_id: Some(cut.files[0].base_id.clone()),
                        before: cut.files[0].bytes.clone(),
                        after: format!("{}changed", note.source).into_bytes(),
                    },
                    FileChange {
                        path: display(&annotations),
                        base_id: Some(cut.files[1].base_id.clone()),
                        before: None,
                        after: b"{\"version\":1,\"annotations\":[]}".to_vec(),
                    },
                ],
            },
        })
        .unwrap();
    journal::private_create(
        &f.core.journal_path(&p.operation_id, "install"),
        b"{\"testCrash\":true}",
    )
    .unwrap();
    fs::write(&source_path, "partial").unwrap();
    assert_eq!(
        f.core
            .open(&source_path, Some(root.clone()))
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert_eq!(
        f.core
            .read_snapshot(&root, &[display(&annotations)])
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert_eq!(
        f.core
            .execute_operation(&p.operation_id, &p.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    let data = f.data.clone();
    drop(f.core);
    let mut reopened = Core::new(data).unwrap();
    assert_eq!(
        reopened.operation_details(&p.operation_id).unwrap().status,
        "recoveryPending"
    );
    assert_eq!(
        reopened.open(source_path, Some(root)).unwrap_err().code,
        "recoveryPending"
    );
}

#[test]
fn style_initialization_is_one_frozen_decision_and_backs_up_private_state() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let before = f.core.read_library_style(&root).unwrap();
    assert!(!before.installed);
    let p = f
        .core
        .prepare_library_style(&root, &before.base_id, &before.source, true, &fresh())
        .unwrap();
    assert!(!root.join("Styles").exists());
    assert!(!root.join(".weftext-settings.json").exists());
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let after = f.core.read_library_style(&root).unwrap();
    assert!(after.installed && after.enabled);
    assert_eq!(after.source, before.source);
    let changed = after.source.replace("size: 11pt", "size: 18pt");
    assert_ne!(changed, after.source);
    let module = f
        .core
        .read_snapshot(&root, std::slice::from_ref(&after.path))
        .unwrap()
        .files
        .remove(0);
    assert_eq!(
        f.core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: module.path,
                        base_id: Some(module.base_id),
                        before: module.bytes,
                        after: changed.as_bytes().to_vec()
                    }]
                }
            })
            .unwrap_err()
            .code,
        "unsupportedRole"
    );
    let p2 = f
        .core
        .prepare_library_style(&root, &after.base_id, &changed, false, &fresh())
        .unwrap();
    f.core
        .execute_operation(&p2.operation_id, &p2.plan_id)
        .unwrap();
    let saved = f.core.read_library_style(&root).unwrap();
    assert!(!saved.enabled);
    assert_eq!(saved.source, changed);
    fs::write(root.join("ignored.bin"), b"unlisted bytes").unwrap();
    let backup = f.temp.path().join("backup");
    let receipt = operation(
        &mut f.core,
        &root,
        OperationIntent::BackupLibrary {
            destination: display(&backup),
        },
    );
    assert_eq!(
        fs::read(backup.join("library/ignored.bin")).unwrap(),
        b"unlisted bytes"
    );
    assert!(backup.join("private/journal").is_dir());
    assert!(backup.join("backup.json").exists());
    assert_eq!(
        f.core
            .execute_operation(&receipt.operation_id, &receipt.plan_id)
            .unwrap()
            .path,
        receipt.path
    );
}

#[test]
fn exact_bom_crlf_and_invalid_native_source_are_preserved() {
    let original = "\u{feff}// comment\r\n= 中文\r\n\r\n#broken(\r\n";
    let mut f = Fixture::new(original);
    let doc = f.open();
    assert_eq!(doc.source, original);
    assert_eq!(doc.kind, "ordinary");
    let receipt = f.core.save(request(&doc, original)).unwrap();
    assert_eq!(receipt.receipt.base_id, doc.base_id);
    assert_eq!(fs::read(&f.path).unwrap(), original.as_bytes());
    assert!(!f.temp.path().join(".weftext-workspace.json").exists());
    let source = "\u{feff}= Changed\r\n#still-invalid(\n";
    f.core.save(request(&doc, source)).unwrap();
    assert_eq!(fs::read(&f.path).unwrap(), source.as_bytes());
}

#[test]
fn successive_saves_shorten_without_tail_and_rotate_only_their_base() {
    let mut f = Fixture::new("a very long original\n");
    let mut doc = f.open();
    let first = f.core.save(request(&doc, "短\n")).unwrap();
    assert_ne!(first.receipt.base_id, doc.base_id);
    doc.base_id = first.receipt.base_id;
    let second = f.core.save(request(&doc, "x\n")).unwrap();
    assert_eq!(fs::read(&f.path).unwrap(), b"x\n");
    assert_ne!(second.receipt.base_id, doc.base_id);
}

#[test]
fn external_change_conflicts_and_draft_survives_restart() {
    let mut f = Fixture::new("before\n");
    let doc = f.open();
    fs::write(&f.path, "external\n").unwrap();
    let error = f.core.save(request(&doc, "our draft\n")).unwrap_err();
    assert_eq!(error.code, "conflict");
    assert_eq!(error.current_source.as_deref(), Some("external\n"));
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "external\n");
    drop(f.core);
    let mut restarted = Core::new(&f.data).unwrap();
    let restored = restarted.open(&f.path, None).unwrap();
    assert_eq!(restored.recovered_draft.as_deref(), Some("our draft\n"));
    assert!(restored.recovered_draft_conflict);
    assert_eq!(
        restored.recovered_draft_base_source.as_deref(),
        Some("before\n")
    );
    assert_eq!(
        restarted
            .save(request(&restored, "our draft\n"))
            .unwrap_err()
            .code,
        "draftConflict"
    );
    restarted
        .resolve_draft(
            &restored.session_id,
            "mergeRecovered",
            Some("external + our draft\n"),
        )
        .unwrap();
    restarted
        .save(request(&restored, "external + our draft\n"))
        .unwrap();
    assert_eq!(
        fs::read_to_string(&f.path).unwrap(),
        "external + our draft\n"
    );
    assert_eq!(
        fs::read_dir(f.data.join("draft-history")).unwrap().count(),
        2
    );
}

#[test]
fn equal_bytes_in_replacement_are_not_a_basis() {
    let mut f = Fixture::new("same\n");
    let doc = f.open();
    let parked = f.temp.path().join("original-parked.typ");
    fs::rename(&f.path, &parked).unwrap();
    fs::write(&f.path, "same\n").unwrap();
    assert_eq!(
        f.core.save(request(&doc, "new\n")).unwrap_err().code,
        "conflict"
    );
    assert_eq!(fs::read_to_string(&parked).unwrap(), "same\n");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "same\n");
}

#[test]
fn stale_and_retry_input_are_distinct_from_commit() {
    let mut f = Fixture::new("before");
    let doc = f.open();
    let req = request(&doc, "after");
    let id = req.operation_id.clone();
    let first = f.core.save(req).unwrap();
    let retry = f
        .core
        .save(SaveRequest {
            operation_id: id.clone(),
            ..request(&doc, "after")
        })
        .unwrap();
    assert_eq!(first.receipt.base_id, retry.receipt.base_id);
    assert_eq!(
        f.core
            .save(SaveRequest {
                operation_id: id,
                ..request(&doc, "different")
            })
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core.save(request(&doc, "stale")).unwrap_err().code,
        "staleBase"
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "after");
}

#[test]
fn create_only_save_as_preserves_original_and_existing_target() {
    let mut f = Fixture::new("before");
    let doc = f.open();
    let target = f.temp.path().join("target.typ");
    fs::write(&target, "external").unwrap();
    let args = |path: &Path| SaveAsRequest {
        session_id: doc.session_id.clone(),
        path: display(path),
        source: "copy".into(),
        operation_id: fresh(),
    };
    assert_eq!(f.core.save_as(args(&target)).unwrap_err().code, "exists");
    assert_eq!(fs::read_to_string(&target).unwrap(), "external");
    let new = f.temp.path().join("new.typ");
    f.core.save_as(args(&new)).unwrap();
    assert_eq!(fs::read_to_string(&new).unwrap(), "copy");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "before");
}

#[test]
fn open_writer_busy_preserves_draft_and_never_writes() {
    let mut f = Fixture::new("before");
    let doc = f.open();
    let writer = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(7)
        .open(&f.path)
        .unwrap();
    assert_eq!(f.core.save(request(&doc, "ours")).unwrap_err().code, "busy");
    drop(writer);
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "before");
    assert_eq!(f.core.read_draft(&f.path).unwrap().unwrap().source, "ours");
}

#[test]
fn parent_chain_guards_block_ancestor_rename_and_leaf_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let path = parent.join("note.typ");
    fs::write(&path, "x").unwrap();
    let guard = ParentGuard::acquire(&path).unwrap();
    let file = guard.open_file(true, false).unwrap();
    assert!(fs::rename(&parent, temp.path().join("moved")).is_err());
    assert!(fs::rename(&path, parent.join("replaced.typ")).is_err());
    assert!(
        OpenOptions::new()
            .write(true)
            .share_mode(7)
            .open(&path)
            .is_err()
    );
    drop(file);
    drop(guard);
    fs::rename(&parent, temp.path().join("moved")).unwrap();
}

#[test]
fn namespace_leaf_rename_keeps_guards_and_identity_through_directory_move() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("Original");
    let target_parent = temp.path().join("Target");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&target_parent).unwrap();
    fs::write(source.join("Original.typ"), "original content").unwrap();

    let guard = backend::NamespaceGuard::acquire(&source, &target_parent).unwrap();
    let directory_id = guard.source_identity().unwrap();
    let leaf = guard.open_source_file("Original.typ", true).unwrap();
    let file_id = backend::stamp(&leaf).unwrap().identity;
    assert!(fs::rename(&source, temp.path().join("External")).is_err());
    assert!(fs::rename(source.join("Original.typ"), source.join("External.typ")).is_err());

    guard.rename_leaf(&leaf, "改名.typ").unwrap();
    assert_eq!(backend::stamp(&leaf).unwrap().identity, file_id);
    assert_eq!(guard.source_identity().unwrap(), directory_id);
    assert!(!source.join("Original.typ").exists());
    drop(leaf);
    // The directory's own protection remains after releasing the leaf for the move.
    assert!(fs::rename(&source, temp.path().join("External")).is_err());
    assert!(fs::rename(&target_parent, temp.path().join("ExternalTarget")).is_err());

    guard.move_source("改名").unwrap();
    assert_eq!(guard.source_identity().unwrap(), directory_id);
    let moved = guard.open_moved_file("改名.typ").unwrap();
    assert_eq!(backend::stamp(&moved).unwrap().identity, file_id);
    assert!(!source.exists());
    drop(moved);
    assert!(fs::rename(target_parent.join("改名"), temp.path().join("External")).is_err());
    drop(guard);
    assert_eq!(
        fs::read_to_string(target_parent.join("改名/改名.typ")).unwrap(),
        "original content"
    );
}

#[test]
fn namespace_leaf_rename_refuses_an_occupied_name_without_replacing_either_file() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("Original");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("Original.typ"), "original content").unwrap();
    fs::write(source.join("Occupied.typ"), "other content").unwrap();
    let guard = backend::NamespaceGuard::acquire(&source, temp.path()).unwrap();
    let leaf = guard.open_source_file("Original.typ", true).unwrap();
    let file_id = backend::stamp(&leaf).unwrap().identity;

    assert!(guard.rename_leaf(&leaf, "Occupied.typ").is_err());
    assert_eq!(backend::stamp(&leaf).unwrap().identity, file_id);
    drop(leaf);
    drop(guard);
    assert_eq!(
        fs::read_to_string(source.join("Original.typ")).unwrap(),
        "original content"
    );
    assert_eq!(
        fs::read_to_string(source.join("Occupied.typ")).unwrap(),
        "other content"
    );
}

#[test]
fn hard_link_alias_is_refused() {
    let mut f = Fixture::new("before");
    let alias = f.temp.path().join("alias.typ");
    fs::hard_link(&f.path, &alias).unwrap();
    assert_eq!(f.core.open(&f.path, None).unwrap_err().code, "hardLink");
    assert_eq!(fs::read_to_string(&alias).unwrap(), "before");
}

#[test]
fn interrupted_write_third_state_and_partial_commit_never_reexecute() {
    let mut f = Fixture::new("before");
    let doc = f.open();
    let session = f.core.sessions[&doc.session_id].clone();
    let id = fresh();
    let plan = journal::Plan::file(
        id.clone(),
        hash(b"proposal"),
        &f.path,
        Some(b"before".to_vec()),
        b"after".to_vec(),
        Some(session.stamp.identity),
        "save",
    );
    f.core.stage(plan).unwrap();
    fs::write(&f.path, "third external version").unwrap();
    fs::write(
        f.data.join("journal").join(format!("{id}.commit.json")),
        "{incomplete decision",
    )
    .unwrap();
    drop(f.core);
    let mut restarted = Core::new(&f.data).unwrap();
    assert_eq!(
        restarted.open(&f.path, None).unwrap_err().code,
        "recoveryPending"
    );
    assert_eq!(
        restarted.inspect_recovery(&id).unwrap().status,
        "thirdState"
    );
    assert_eq!(
        restarted
            .resolve_recovery(&id, "rollback")
            .unwrap_err()
            .code,
        "unsupportedRecovery"
    );
    restarted.resolve_recovery(&id, "keepCurrent").unwrap();
    assert_eq!(
        fs::read_to_string(&f.path).unwrap(),
        "third external version"
    );
    assert_eq!(
        restarted.open(&f.path, None).unwrap().source,
        "third external version"
    );
}

#[test]
fn native_birth_admission_duplicate_and_unmanaged_browse() {
    let mut f = Fixture::new("outside");
    let root = f.temp.path().join("library");
    let library = f.core.create_library(&root).unwrap();
    let doc = f.core.create_note_fixture(&root, &root, "A").unwrap();
    assert_eq!(doc.kind, "managed");
    assert_eq!(doc.workspace_id, Some(library.workspace_id));
    let first_id = doc.node_id.clone().unwrap();
    assert_eq!(
        f.core
            .create_note_fixture(&root, &root, "A")
            .unwrap_err()
            .code,
        "exists"
    );
    assert_eq!(f.core.create_library(&root).unwrap_err().code, "exists");
    fs::create_dir(root.join("container")).unwrap();
    fs::create_dir(root.join("container/B")).unwrap();
    fs::write(
        root.join("container/B/B.typ"),
        format!("#metadata((id: \"{}\", types: ())) <weftext>\n", fresh()),
    )
    .unwrap();
    fs::write(root.join("loose.typ"), "plain").unwrap();
    let inv = f.core.inventory(&root).unwrap();
    assert!(
        inv.entries
            .iter()
            .any(|i| i.relative_path == "container/B/B.typ"
                && i.kind == "node"
                && i.identity_status == "unadmitted")
    );
    assert!(
        inv.entries
            .iter()
            .any(|i| i.relative_path == "loose.typ" && i.kind == "file")
    );
    fs::create_dir(root.join("copy")).unwrap();
    fs::write(root.join("copy/copy.typ"), &doc.source).unwrap();
    let inv = f.core.inventory(&root).unwrap();
    assert_eq!(
        inv.entries
            .iter()
            .filter(|i| i.node_id.as_deref() == Some(&first_id) && i.identity_status == "duplicate")
            .count(),
        2
    );
    drop(f.core);
    let mut restarted = Core::new(&f.data).unwrap();
    assert_eq!(
        restarted
            .open(PathBuf::from(&doc.path), None)
            .unwrap()
            .node_id,
        Some(first_id)
    );
    // A broken declaration remains a known node, never an ordinary bypass.
    fs::write(&doc.path, "broken native source").unwrap();
    let damaged = restarted.open(PathBuf::from(&doc.path), None).unwrap();
    assert_eq!(damaged.kind, "managedInvalid");
    assert_eq!(
        restarted
            .save(request(&damaged, "still broken"))
            .unwrap_err()
            .code,
        "identityConflict"
    );
}

#[test]
fn exact_rules_hide_ignored_and_browse_explicit_unmanaged_descendants() {
    let mut f = Fixture::new("outside");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    for dir in ["ignored", "unmanaged", "unmanaged/Child"] {
        fs::create_dir(root.join(dir)).unwrap();
    }
    fs::write(root.join("ignored/ignored.typ"), "hidden").unwrap();
    fs::write(root.join("unmanaged/Child/Child.typ"), "body").unwrap();
    fs::write(
        root.join(".weftext-rules"),
        r#"{"version":1,"ignored":["ignored"],"unmanaged":["unmanaged"]}"#,
    )
    .unwrap();
    let inventory = f.core.inventory(&root).unwrap();
    assert!(
        !inventory
            .entries
            .iter()
            .any(|i| i.relative_path.starts_with("ignored"))
    );
    let descendant = inventory
        .entries
        .iter()
        .find(|i| i.relative_path == "unmanaged/Child/Child.typ")
        .unwrap();
    assert_eq!(descendant.kind, "file");
    assert_eq!(descendant.identity_status, "unmanaged");
    assert_eq!(
        f.core
            .create_note_fixture(&root, root.join("unmanaged"), "New")
            .unwrap_err()
            .code,
        "excludedBoundary"
    );
    assert_eq!(
        f.core
            .open(root.join("unmanaged/Child/Child.typ"), Some(root))
            .unwrap()
            .kind,
        "ordinary"
    );
}

#[test]
fn symlink_paths_are_refused_when_windows_allows_fixture_creation() {
    let mut f = Fixture::new("before");
    let alias = f.temp.path().join("link.typ");
    match std::os::windows::fs::symlink_file(&f.path, &alias) {
        Ok(()) => assert_eq!(
            f.core.open(&alias, None).unwrap_err().code,
            "unsupportedPath"
        ),
        Err(error) => {
            // No admin escalation or ACL change for a test fixture.
            assert_eq!(error.raw_os_error(), Some(1314));
            eprintln!("Symlink fixture not exercised: Windows privilege 1314");
        }
    }
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "before");
}

#[test]
fn writable_mapping_remains_busy_after_original_file_handle_closes() {
    let mut f = Fixture::new("before");
    let doc = f.open();
    let writer = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(7)
        .open(&f.path)
        .unwrap();
    // The fixture owns the complete file; no shared references to its byte contents.
    let mut map = unsafe { memmap2::MmapOptions::new().map_mut(&writer).unwrap() };
    drop(writer);
    map[0] = b'B';
    map.flush().unwrap();
    let error = f.core.save(request(&doc, "ours")).unwrap_err();
    assert_eq!(error.code, "busy", "unexpected mapping result: {error}");
    drop(map);
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "Before");
}

#[test]
fn directory_junction_is_rejected_without_symlink_privilege() {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn DeviceIoControl(
            handle: *mut c_void,
            code: u32,
            input: *const c_void,
            input_len: u32,
            output: *mut c_void,
            output_len: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }
    let mut f = Fixture::new("before");
    let target = f.temp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("note.typ"), "real bytes").unwrap();
    let junction = f.temp.path().join("junction");
    fs::create_dir(&junction).unwrap();
    let handle = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(7)
        .custom_flags(0x02000000 | 0x00200000)
        .open(&junction)
        .unwrap();
    let substitute = format!("\\??\\{}", target.display())
        .encode_utf16()
        .collect::<Vec<_>>();
    let printable = target.as_os_str().encode_wide().collect::<Vec<_>>();
    let mut payload = Vec::new();
    payload.extend_from_slice(&0xA0000003u32.to_le_bytes());
    let data_len = 8 + (substitute.len() + 1 + printable.len() + 1) * 2;
    payload.extend_from_slice(&(data_len as u16).to_le_bytes());
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&((substitute.len() * 2) as u16).to_le_bytes());
    payload.extend_from_slice(&(((substitute.len() + 1) * 2) as u16).to_le_bytes());
    payload.extend_from_slice(&((printable.len() * 2) as u16).to_le_bytes());
    for ch in substitute
        .into_iter()
        .chain([0])
        .chain(printable)
        .chain([0])
    {
        payload.extend_from_slice(&ch.to_le_bytes());
    }
    let mut returned = 0;
    let ok = unsafe {
        DeviceIoControl(
            handle.as_raw_handle(),
            0x900A4,
            payload.as_ptr().cast(),
            payload.len() as u32,
            std::ptr::null_mut(),
            0,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    assert_ne!(
        ok,
        0,
        "junction fixture failed: {}",
        std::io::Error::last_os_error()
    );
    drop(handle);
    assert_eq!(
        f.core
            .open(junction.join("note.typ"), None)
            .unwrap_err()
            .code,
        "unsupportedPath"
    );
    let inv = f.core.inventory(f.temp.path()).unwrap();
    assert!(
        inv.entries
            .iter()
            .any(|e| e.path == display(&junction) && e.kind == "blocked")
    );
    assert_eq!(
        fs::read_to_string(target.join("note.typ")).unwrap(),
        "real bytes"
    );
}

#[test]
fn private_directory_has_one_live_core_and_reopens_after_drop() {
    let f = Fixture::new("plain");
    match Core::new(&f.data) {
        Ok(_) => panic!("A second live Core acquired the same private directory"),
        Err(error) => assert_eq!(error.code, "instanceBusy"),
    }
    drop(f.core);
    assert!(Core::new(&f.data).is_ok());
}

#[test]
fn creation_replays_exact_original_ids_and_rejects_new_intents() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    let library_op = fresh();
    let first = f.core.create_library_with_id(&root, &library_op).unwrap();
    let replay = f.core.create_library_with_id(&root, &library_op).unwrap();
    assert_eq!(first.workspace_id, replay.workspace_id);
    assert_eq!(
        f.core
            .create_library_with_id(f.temp.path().join("other"), &library_op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    let node_op = fresh();
    let created = f
        .core
        .create_note_with_id(&root, &root, "标题 @key_", &node_op)
        .unwrap();
    let replay = f
        .core
        .create_note_with_id(&root, &root, "标题 @key_", &node_op)
        .unwrap();
    assert_eq!(created.node_ids, replay.node_ids);
    let created = f
        .core
        .open(
            Path::new(&created.path).join("标题 @key_.typ"),
            Some(root.clone()),
        )
        .unwrap();
    let replay = f
        .core
        .open(
            Path::new(&replay.path).join("标题 @key_.typ"),
            Some(root.clone()),
        )
        .unwrap();
    assert_eq!(created.node_id, replay.node_id);
    assert_eq!(created.source, replay.source);
    assert!(created.source.contains("\\@key\\_"));
    let map = weftext_engine::inspect_editor(&created.source);
    assert_eq!(map.title.status, "writable");
    assert_eq!(map.title.text.as_deref(), Some("标题 @key_"));
    assert!(!map.structures.iter().any(|s| s.kind == "heading"));
    assert_eq!(
        f.core
            .create_note_with_id(&root, &root, "Different", &node_op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .operations
            .values()
            .filter(|p| p
                .transaction
                .as_ref()
                .is_some_and(|t| t.preview.kind == "newNode"))
            .count(),
        1
    );
}

#[test]
fn stale_base_proposal_remains_unqualified_after_restart() {
    let mut f = Fixture::new("initial");
    let doc = f.open();
    f.core.save(request(&doc, "first saved")).unwrap();
    assert_eq!(
        f.core
            .save(request(&doc, "old-base draft"))
            .unwrap_err()
            .code,
        "staleBase"
    );
    drop(f.core);
    let mut core = Core::new(&f.data).unwrap();
    let restored = core.open(&f.path, None).unwrap();
    assert_eq!(restored.source, "first saved");
    assert_eq!(restored.recovered_draft.as_deref(), Some("old-base draft"));
    assert!(restored.recovered_draft_conflict);
    assert_eq!(
        core.save(request(&restored, "old-base draft"))
            .unwrap_err()
            .code,
        "draftConflict"
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "first saved");
}

#[test]
fn private_journal_publication_never_replaces_or_reads_partial_temps() {
    let mut f = Fixture::new("before");
    let id = fresh();
    fs::write(
        f.data.join("journal").join(format!("{}.pending", fresh())),
        "partial unpublished payload",
    )
    .unwrap();
    let plan = journal::Plan::file(
        id.clone(),
        hash(b"original"),
        &f.path,
        Some(b"before".to_vec()),
        b"after".to_vec(),
        None,
        "save",
    );
    f.core.stage(plan.clone()).unwrap();
    let mut changed = plan;
    changed.after = b"replacement".to_vec();
    assert_eq!(f.core.stage(changed).unwrap_err().code, "exists");
    assert!(fs::rename(f.data.join("journal"), f.data.join("moved-journal")).is_err());
    drop(f.core);
    let core = Core::new(&f.data).unwrap();
    assert_eq!(core.operations.len(), 1);
    assert_eq!(core.operations[&id].after, b"after");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), "before");
}

#[test]
fn selected_library_backup_excludes_foreign_plaintext_and_retains_draft_consumption() {
    let mut f = Fixture::new("ordinary");
    let first = f.temp.path().join("L1");
    let second = f.temp.path().join("L2");
    f.core.create_library(&first).unwrap();
    f.core.create_library(&second).unwrap();
    let a = f.core.create_note_fixture(&first, &first, "A").unwrap();
    let b = f.core.create_note_fixture(&second, &second, "B").unwrap();
    f.core
        .persist_draft(&a.session_id, "selected retained proposal")
        .unwrap();
    f.core
        .persist_draft(&b.session_id, "FOREIGN_LIBRARY_DRAFT_SECRET")
        .unwrap();
    let ordinary = f.open();
    f.core
        .persist_draft(&ordinary.session_id, "FOREIGN_ORDINARY_DRAFT_SECRET")
        .unwrap();
    operation(
        &mut f.core,
        &first,
        OperationIntent::RenameNode {
            source: display(&first.join("A")),
            name: "C".into(),
        },
    );
    let moved = f
        .core
        .open(first.join("C/C.typ"), Some(first.clone()))
        .unwrap();
    assert_eq!(
        moved.recovered_draft.as_deref(),
        Some("selected retained proposal")
    );
    f.core
        .resolve_draft(&moved.session_id, "discardRecovered", None)
        .unwrap();
    assert!(
        fs::read_dir(f.data.join("draft-history"))
            .unwrap()
            .any(|entry| {
                let draft: Draft =
                    serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap();
                draft.source == "selected retained proposal"
            }),
        "Explicit resolution must preserve the original selected proposal before backup"
    );
    let marker = f
        .core
        .draft_path(&first.join("C/C.typ"))
        .with_extension("relocation.json");
    assert!(marker.exists());
    operation(
        &mut f.core,
        &second,
        OperationIntent::RenameNode {
            source: display(&second.join("B")),
            name: "D".into(),
        },
    );
    f.core
        .open(second.join("D/D.typ"), Some(second.clone()))
        .unwrap();
    let foreign_marker = f
        .core
        .draft_path(&second.join("D/D.typ"))
        .with_extension("relocation.json");
    let target = f.temp.path().join("selected-backup");
    operation(
        &mut f.core,
        &first,
        OperationIntent::BackupLibrary {
            destination: display(&target),
        },
    );
    assert_eq!(
        fs::read(
            target
                .join("private/drafts")
                .join(marker.file_name().unwrap())
        )
        .unwrap(),
        fs::read(&marker).unwrap()
    );
    assert!(
        !target
            .join("private/drafts")
            .join(foreign_marker.file_name().unwrap())
            .exists()
    );
    let mut queue = vec![target.join("private")];
    let mut all = Vec::new();
    while let Some(dir) = queue.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                queue.push(path);
            } else {
                all.extend(fs::read(path).unwrap());
            }
        }
    }
    let text = String::from_utf8_lossy(&all);
    assert!(text.contains("selected retained proposal"));
    assert!(!text.contains("FOREIGN_LIBRARY_DRAFT_SECRET"));
    assert!(!text.contains("FOREIGN_ORDINARY_DRAFT_SECRET"));
}

#[test]
fn readonly_source_and_resource_are_guarded_noop_dependencies_of_annotation_decision() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let note = f.core.create_note_fixture(&root, &root, "Note").unwrap();
    let p = f
        .core
        .prepare_import_resource(&root, &note.path, "owned.bin", vec![1, 2, 3, 4], &fresh())
        .unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let resource = root.join("Note/owned.bin");
    let mut original_permissions = Vec::new();
    for path in [Path::new(&note.path), resource.as_path()] {
        let mut permission = fs::metadata(path).unwrap().permissions();
        original_permissions.push((path.to_path_buf(), permission.clone()));
        permission.set_readonly(true);
        fs::set_permissions(path, permission).unwrap();
    }
    let sidecar = root.join("Note/weftext.annotations.json");
    let snapshot = f
        .core
        .read_snapshot(
            &root,
            &[
                note.path.clone(),
                display(&resource),
                display(&root.join("Note/weftext.resources.json")),
                display(&sidecar),
            ],
        )
        .unwrap();
    let changes = snapshot
        .files
        .iter()
        .map(|s| FileChange {
            path: s.path.clone(),
            base_id: Some(s.base_id.clone()),
            before: s.bytes.clone(),
            after: s
                .bytes
                .clone()
                .unwrap_or_else(|| b"{\"version\":1,\"annotations\":[]}".to_vec()),
        })
        .collect();
    let p = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::UpdateFiles { changes },
        })
        .unwrap();
    assert_eq!(
        p.changes
            .iter()
            .filter(|c| c.action == "readDependency")
            .count(),
        3
    );
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert!(sidecar.exists());
    assert_eq!(fs::read(&resource).unwrap(), vec![1, 2, 3, 4]);
    for (path, permission) in original_permissions {
        fs::set_permissions(path, permission).unwrap();
    }
}

#[test]
fn rich_copy_and_template_rewrite_self_refs_and_own_resource_bytes() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    let library = f.core.create_library(&root).unwrap();
    let note = f.core.create_note_fixture(&root, &root, "Person").unwrap();
    let id = note.node_id.clone().unwrap();
    let workspace = library.workspace_id;
    let source = format!(
        "#metadata((id: \"{id}\", types: (\"people/person\",), aliases: (\"Mary\",))) <weftext>\n= Person\n#link(\"weftext-node://{workspace}/{id}\")[self]\n#read(\"owned.bin\")"
    );
    f.core.save(request(&note, &source)).unwrap();
    let p = f
        .core
        .prepare_import_resource(
            &root,
            &note.path,
            "owned.bin",
            b"own bytes".to_vec(),
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let p = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::CopyNode {
                source: display(&root.join("Person")),
                parent: display(&root),
                name: "Copy".into(),
                allow_losses: false,
            },
        })
        .unwrap();
    assert!(p.losses.is_empty());
    let fresh_id = p.node_map[&id].clone();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let copied = fs::read_to_string(root.join("Copy/Copy.typ")).unwrap();
    assert!(copied.contains(&format!("weftext-node://{workspace}/{fresh_id}")));
    assert!(copied.contains("people/person"));
    let resources = f
        .core
        .managed_resource_snapshot(&root, root.join("Copy/Copy.typ"))
        .unwrap();
    assert_eq!(resources[0].identity_status, "admitted");
    assert_eq!(resources[0].bytes, b"own bytes");
    let basis = f
        .core
        .read_snapshot(&root, std::slice::from_ref(&note.path))
        .unwrap();
    let p = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::TemplateNode {
                source: note.path.clone(),
                parent: display(&root),
                name: "FromTemplate".into(),
                instantiated_source: source.replace("= Person", "= Instantiated"),
                source_base_id: basis.files[0].base_id.clone(),
            },
        })
        .unwrap();
    let target_id = p.new_node_id.clone().unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let created = fs::read_to_string(root.join("FromTemplate/FromTemplate.typ")).unwrap();
    assert!(created.contains("= Instantiated"));
    assert!(created.contains(&format!("weftext-node://{workspace}/{target_id}")));
    assert_eq!(
        f.core
            .managed_resource_snapshot(&root, root.join("FromTemplate/FromTemplate.typ"))
            .unwrap()[0]
            .bytes,
        b"own bytes"
    );
}

#[test]
fn admitted_compile_inputs_remain_frozen_and_selected_style_is_exact() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let note = f.core.create_note_fixture(&root, &root, "Note").unwrap();
    fs::write(root.join("Note/input.txt"), b"first").unwrap();
    let before = f.core.read_library_style(&root).unwrap();
    let p = f
        .core
        .prepare_library_style(&root, &before.base_id, &before.source, true, &fresh())
        .unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    let cut = f.core.admitted_compile_snapshot(&root, "live").unwrap();
    assert!(cut.snapshot.complete);
    assert!(cut.style.as_ref().unwrap().enabled);
    assert_eq!(cut.style.as_ref().unwrap().path, "Styles/library-style.typ");
    let rendered = weftext_engine::Engine::new_with_system_fonts(false)
        .unwrap()
        .render_frozen(&weftext_engine::FrozenRenderRequest {
            source: note.source.clone(),
            path: PathBuf::from(&note.path),
            root: root.clone(),
            files: cut.files.clone(),
            style: cut.style.clone(),
        })
        .unwrap();
    assert!(rendered.ok, "{:?}", rendered.diagnostics);
    fs::write(root.join("Note/input.txt"), b"later").unwrap();
    assert_eq!(cut.files["Note/input.txt"], b"first");
    assert_eq!(cut.files["Note/Note.typ"], note.source.as_bytes());
    assert!(
        f.core
            .prepare_admitted_operation(
                OperationRequest {
                    operation_id: fresh(),
                    root: display(&root),
                    intent: OperationIntent::NewNode {
                        parent: display(&root),
                        name: "New".into(),
                        source: None
                    }
                },
                &cut.snapshot.cut_id
            )
            .is_err()
    );
}

#[test]
fn default_frozen_node_has_native_title_and_separate_section_headings() {
    let mut f = Fixture::new("ordinary");
    let root = f.temp.path().join("library");
    f.core.create_library(&root).unwrap();
    let operation_id = fresh();
    let request = OperationRequest {
        operation_id: operation_id.clone(),
        root: display(&root),
        intent: OperationIntent::NewNode {
            parent: display(&root),
            name: "Title @key_".into(),
            source: None,
        },
    };
    let plan = f.core.prepare_operation(request.clone()).unwrap();
    let before = f.core.prepared_operation_files(&operation_id).unwrap()[0]
        .after
        .clone();
    let source = String::from_utf8(before.clone()).unwrap();
    let map = weftext_engine::inspect_editor(&source);
    assert!(map.syntax_valid);
    assert_eq!(map.title.status, "writable");
    assert_eq!(map.title.text.as_deref(), Some("Title @key_"));
    assert!(!map.structures.iter().any(|s| s.kind == "heading"));
    let with_section = weftext_engine::inspect_editor(&format!("{source}= Separate section\n"));
    assert_eq!(with_section.title.text, map.title.text);
    assert_eq!(
        with_section
            .structures
            .iter()
            .filter(|s| s.kind == "heading")
            .count(),
        1
    );
    assert_eq!(
        with_section
            .structures
            .iter()
            .find(|s| s.kind == "heading")
            .unwrap()
            .level,
        Some(1)
    );
    f.core
        .execute_operation(&operation_id, &plan.plan_id)
        .unwrap();
    assert_eq!(
        fs::read(root.join("Title @key_/Title @key_.typ")).unwrap(),
        before
    );
    let replay = f.core.prepare_operation(request).unwrap();
    assert_eq!(replay.plan_id, plan.plan_id);
    assert_eq!(
        f.core.prepared_operation_files(&operation_id).unwrap()[0].after,
        before
    );
}
