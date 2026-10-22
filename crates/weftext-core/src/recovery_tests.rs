use super::*;

fn operate(core: &mut Core, root: &Path, intent: OperationIntent) -> OperationReceipt {
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
fn pair_plan(
    core: &mut Core,
    root: &Path,
    note: &OpenedDocument,
    existing_sidecar: bool,
) -> (OperationPreview, PathBuf, Vec<u8>) {
    pair_plan_inner(core, root, note, existing_sidecar, false)
}
fn pair_plan_inner(
    core: &mut Core,
    root: &Path,
    note: &OpenedDocument,
    existing_sidecar: bool,
    strong: bool,
) -> (OperationPreview, PathBuf, Vec<u8>) {
    let sidecar = Path::new(&note.path)
        .parent()
        .unwrap()
        .join("weftext.annotations.json");
    if existing_sidecar {
        fs::write(&sidecar, b"{\"version\":1,\"annotations\":[]}").unwrap();
    }
    let admitted = strong.then(|| core.admitted_snapshot(root).unwrap());
    let cut = core
        .read_snapshot(root, &[note.path.clone(), display(&sidecar)])
        .unwrap();
    let after_source = format!("{}\nRecovered original proposal.", note.source).into_bytes();
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent: OperationIntent::UpdateFiles {
            changes: vec![
                FileChange {
                    path: note.path.clone(),
                    base_id: Some(cut.files[0].base_id.clone()),
                    before: cut.files[0].bytes.clone(),
                    after: after_source.clone(),
                },
                FileChange {
                    path: display(&sidecar),
                    base_id: Some(cut.files[1].base_id.clone()),
                    before: cut.files[1].bytes.clone(),
                    after: b"{\"version\":1,\"annotations\":[],\"recovered\":true}".to_vec(),
                },
            ],
        },
    };
    let preview = if let Some(admitted) = admitted {
        core.prepare_admitted_operation(request, &admitted.cut_id)
    } else {
        core.prepare_operation(request)
    }
    .unwrap();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    (preview, sidecar, after_source)
}
#[test]
fn source_annotation_mixed_restart_resumes_only_original_remaining_member() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let (preview, sidecar, after_source) = pair_plan(&mut core, &root, &note, true);
    fs::write(&note.path, &after_source).unwrap();
    let source_modified = fs::metadata(&note.path).unwrap().modified().unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let recovery = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(recovery.coherence, "mixedKnown");
    assert!(
        recovery
            .allowed_actions
            .iter()
            .any(|a| a == "resumeOriginalPlan")
    );
    assert!(recovery.members.iter().any(|m| {
        m.path == note.path
            && m.state == "knownAfter"
            && m.current
                .source
                .as_ref()
                .unwrap()
                .contains("Recovered original proposal.")
    }));
    assert!(
        recovery
            .members
            .iter()
            .any(|m| m.path == display(&sidecar) && m.state == "knownBefore")
    );
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "keepCurrent")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    let resolved = restarted
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    assert_eq!(resolved.status, "committed");
    assert_eq!(fs::read(&note.path).unwrap(), after_source);
    assert_eq!(
        fs::metadata(&note.path).unwrap().modified().unwrap(),
        source_modified,
        "already-installed source must not be rewritten"
    );
    assert_eq!(
        fs::read(&sidecar).unwrap(),
        b"{\"version\":1,\"annotations\":[],\"recovered\":true}"
    );
    assert_eq!(
        restarted
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .operation_id,
        preview.operation_id
    );
    assert!(restarted.open(&note.path, Some(root)).is_ok());
}
#[test]
fn mixed_restart_creates_only_original_absent_sidecar_and_records_real_object() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let (preview, sidecar, after_source) = pair_plan(&mut core, &root, &note, false);
    fs::write(&note.path, &after_source).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    assert_eq!(
        restarted
            .inspect_recovery(&preview.operation_id)
            .unwrap()
            .coherence,
        "mixedKnown"
    );
    restarted
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    let actual = ParentGuard::acquire(&crate::BackendContext::local().unwrap(), &sidecar)
        .unwrap()
        .open_file(false, false)
        .unwrap();
    assert_eq!(
        restarted
            .materialized_identity(&preview.operation_id, &sidecar)
            .unwrap(),
        Some(stamp(&actual).unwrap().identity)
    );
}
#[test]
fn third_bytes_or_same_bytes_replacement_never_unblock_mixed_pair() {
    for replace in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let (preview, sidecar, after_source) = pair_plan(&mut core, &root, &note, true);
        fs::write(&note.path, &after_source).unwrap();
        if replace {
            let original = fs::read(&sidecar).unwrap();
            fs::rename(&sidecar, sidecar.with_extension("original.json")).unwrap();
            fs::write(&sidecar, original).unwrap();
        } else {
            fs::write(&sidecar, b"third external bytes").unwrap();
        }
        let external = fs::read(&sidecar).unwrap();
        drop(core);
        let mut restarted = Core::new(&data).unwrap();
        let recovery = restarted.inspect_recovery(&preview.operation_id).unwrap();
        assert_eq!(recovery.coherence, "thirdState");
        assert!(recovery.allowed_actions.is_empty());
        for action in ["keepCurrent", "adoptAfterImages", "resumeOriginalPlan"] {
            assert_eq!(
                restarted
                    .resolve_recovery(&preview.operation_id, action)
                    .unwrap_err()
                    .code,
                "recoveryConflict"
            );
        }
        assert_eq!(fs::read(&sidecar).unwrap(), external);
        assert!(!restarted.committed(&preview.operation_id).unwrap());
        assert_eq!(
            restarted.open(&note.path, Some(root)).unwrap_err().code,
            "recoveryPending"
        );
    }
}
#[test]
fn binary_members_report_exact_base64_and_refuse_unowned_after_image() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let (preview, sidecar, after_source) = pair_plan(&mut core, &root, &note, false);
    fs::write(&note.path, after_source).unwrap();
    fs::write(&sidecar, [0xff, 0, 0x80]).unwrap();
    let recovery = core.inspect_recovery(&preview.operation_id).unwrap();
    let member = recovery
        .members
        .iter()
        .find(|m| m.path == display(&sidecar))
        .unwrap();
    assert_eq!(member.current.byte_length, 3);
    assert_eq!(member.current.base64.as_deref(), Some("/wCA"));
    assert!(member.current.source.is_none());
    assert_eq!(member.identity_binding, "mismatch");
    assert!(recovery.allowed_actions.is_empty());
}
#[test]
fn restart_before_ui_rebind_preserves_draft_through_rename_move_trash_restore() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Original").unwrap();
    core.persist_draft(&note.session_id, "unsaved original draft")
        .unwrap();
    operate(
        &mut core,
        &root,
        OperationIntent::RenameNode {
            source: display(&root.join("Original")),
            name: "Renamed".into(),
        },
    );
    fs::create_dir(root.join("Container")).unwrap();
    operate(
        &mut core,
        &root,
        OperationIntent::MoveNode {
            source: display(&root.join("Renamed")),
            parent: display(&root.join("Container")),
        },
    );
    let trash = operate(
        &mut core,
        &root,
        OperationIntent::TrashNode {
            source: display(&root.join("Container/Renamed")),
        },
    );
    operate(
        &mut core,
        &root,
        OperationIntent::RestoreNode {
            trash_operation_id: trash.operation_id,
            node_id: None,
            parent: display(&root),
            name: "Restored".into(),
        },
    );
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let restored = restarted
        .open(root.join("Restored/Restored.typ"), Some(root))
        .unwrap();
    assert_eq!(
        restored.recovered_draft.as_deref(),
        Some("unsaved original draft")
    );
    assert_eq!(
        restored.recovered_draft_base_source.as_deref(),
        Some(note.source.as_str())
    );
    assert!(!restored.recovered_draft_conflict);
}
#[test]
fn nested_node_and_ordinary_drafts_follow_actual_moved_root_on_restart() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    core.create_note_fixture(&root, &root, "A").unwrap();
    let nested = core
        .create_note_fixture(&root, root.join("A"), "B")
        .unwrap();
    core.persist_draft(&nested.session_id, "nested node draft")
        .unwrap();
    fs::write(root.join("A/raw.typ"), "raw base").unwrap();
    let raw = core
        .open(root.join("A/raw.typ"), Some(root.clone()))
        .unwrap();
    core.persist_draft(&raw.session_id, "nested raw draft")
        .unwrap();
    operate(
        &mut core,
        &root,
        OperationIntent::RenameNode {
            source: display(&root.join("A")),
            name: "Moved".into(),
        },
    );
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let nested = restarted
        .open(root.join("Moved/B/B.typ"), Some(root.clone()))
        .unwrap();
    let raw = restarted
        .open(root.join("Moved/raw.typ"), Some(root))
        .unwrap();
    assert_eq!(nested.recovered_draft.as_deref(), Some("nested node draft"));
    assert_eq!(raw.recovered_draft.as_deref(), Some("nested raw draft"));
}
#[test]
fn replacement_at_old_draft_path_never_inherits_original_object_draft() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let path = temp.path().join("plain.typ");
    fs::write(&path, "base").unwrap();
    let mut core = Core::new(&data).unwrap();
    let note = core.open(&path, None).unwrap();
    core.persist_draft(&note.session_id, "original private draft")
        .unwrap();
    let original_draft_path = core.draft_path(&path);
    let original_draft_bytes = fs::read(&original_draft_path).unwrap();
    fs::rename(&path, path.with_extension("retained.typ")).unwrap();
    fs::write(&path, "base").unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let replacement = restarted.open(&path, None).unwrap();
    assert!(replacement.recovered_draft.is_none());
    assert_eq!(replacement.source, "base");
    assert_eq!(fs::read(original_draft_path).unwrap(), original_draft_bytes);
}
#[test]
fn relocated_competing_drafts_preserve_both_and_require_explicit_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Original").unwrap();
    core.persist_draft(&note.session_id, "first unsaved proposal")
        .unwrap();
    // Reproduce an original pathname record. The new primary must consume this
    // exact legacy proposal without replacing or deleting its original bytes.
    fs::rename(
        core.draft_path(Path::new(&note.path)),
        core.legacy_draft_path(Path::new(&note.path)),
    )
    .unwrap();
    let destination = root.join("Moved/Moved.typ");
    let mut second = core.read_draft(Path::new(&note.path)).unwrap().unwrap();
    second.path = display(&destination);
    second.source = "second unsaved proposal".into();
    private_replace(
        &crate::BackendContext::local().unwrap(),
        &core.draft_path(&destination),
        &serde_json::to_vec(&second).unwrap(),
    )
    .unwrap();
    operate(
        &mut core,
        &root,
        OperationIntent::RenameNode {
            source: display(&root.join("Original")),
            name: "Moved".into(),
        },
    );
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let opened = restarted.open(&destination, Some(root.clone())).unwrap();
    assert!(opened.recovered_draft_conflict);
    assert_eq!(
        opened.recovered_draft.as_deref(),
        Some("second unsaved proposal")
    );
    assert!(
        opened
            .recovered_draft_variants
            .iter()
            .any(|v| v.source == "first unsaved proposal" && v.base_source == note.source)
    );
    assert_eq!(
        opened.recovered_draft_base_source.as_deref(),
        Some(note.source.as_str())
    );
    restarted
        .persist_draft(&opened.session_id, "first unsaved proposal")
        .unwrap();
    drop(restarted);
    let mut restarted = Core::new(&data).unwrap();
    let opened = restarted.open(&destination, Some(root.clone())).unwrap();
    assert_eq!(
        opened.recovered_draft.as_deref(),
        Some("first unsaved proposal")
    );
    assert!(opened.recovered_draft_conflict);
    assert!(
        opened
            .recovered_draft_variants
            .iter()
            .any(|v| v.source == "second unsaved proposal" && v.base_source == note.source)
    );
    restarted
        .resolve_draft(
            &opened.session_id,
            "mergeRecovered",
            Some("explicit merged proposal"),
        )
        .unwrap();
    drop(restarted);
    let mut after_resolution = Core::new(&data).unwrap();
    let opened = after_resolution.open(&destination, Some(root)).unwrap();
    assert_eq!(
        opened.recovered_draft.as_deref(),
        Some("explicit merged proposal")
    );
    assert!(opened.recovered_draft_variants.is_empty());
    assert!(!opened.recovered_draft_conflict);
    assert!(
        after_resolution.draft_path(Path::new(&note.path)).exists(),
        "original draft retained independently of migration"
    );
}
#[test]
fn after_image_adoption_uses_original_create_only_objects_and_original_births() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::NewNode {
                parent: display(&root),
                name: "Fresh".into(),
                source: None,
            },
        })
        .unwrap();
    let plan = core.operations[&preview.operation_id].clone();
    let operations::Mutation::CreateTree {
        target,
        directories,
        files,
        ..
    } = &plan.transaction.as_ref().unwrap().mutation
    else {
        panic!("expected frozen create");
    };
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    let guard =
        ParentGuard::acquire(&crate::BackendContext::local().unwrap(), Path::new(target)).unwrap();
    let directory = guard.create_directory().unwrap();
    core.record_materialized(&preview.operation_id, Path::new(target), &directory)
        .unwrap();
    let mut directory_handles = std::collections::BTreeMap::from([(PathBuf::new(), directory)]);
    let mut paths = directories.iter().map(PathBuf::from).collect::<Vec<_>>();
    paths.sort_by_key(|p| p.components().count());
    for relative in paths {
        let parent = directory_handles
            .get(relative.parent().unwrap_or_else(|| Path::new("")))
            .unwrap();
        let child =
            ParentGuard::child_directory(parent, relative.file_name().unwrap().to_str().unwrap())
                .unwrap();
        core.record_materialized(
            &preview.operation_id,
            &Path::new(target).join(&relative),
            &child,
        )
        .unwrap();
        directory_handles.insert(relative, child);
    }
    let mut original_identities = Vec::new();
    for change in files {
        let relative = Path::new(&change.path).strip_prefix(target).unwrap();
        let parent = directory_handles
            .get(relative.parent().unwrap_or_else(|| Path::new("")))
            .unwrap();
        let mut file =
            ParentGuard::child_file(parent, relative.file_name().unwrap().to_str().unwrap())
                .unwrap();
        core.record_materialized(&preview.operation_id, Path::new(&change.path), &file)
            .unwrap();
        operations::write_blob(&mut file, &change.after).unwrap();
        original_identities.push(stamp(&file).unwrap().identity);
    }
    assert!(core.read_births(&preview.operation_id).unwrap().is_empty());
    drop(directory_handles);
    drop(guard);
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspection = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspection.coherence, "knownAfter");
    assert!(
        inspection
            .allowed_actions
            .iter()
            .any(|a| a == "adoptAfterImages")
    );
    let adopted = restarted
        .resolve_recovery(&preview.operation_id, "adoptAfterImages")
        .unwrap();
    assert_eq!(adopted.status, "committed");
    let births = restarted.read_births(&preview.operation_id).unwrap();
    assert_eq!(births.len(), 1);
    assert_eq!(Some(births[0].node_id.clone()), preview.new_node_id);
    assert!(original_identities.contains(&births[0].identity));
    assert_eq!(
        restarted
            .open(root.join("Fresh/Fresh.typ"), Some(root))
            .unwrap()
            .node_id,
        preview.new_node_id
    );
}
#[test]
fn intermediate_leaf_rename_is_visible_and_namespace_is_never_replayed() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Original").unwrap();
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::RenameNode {
                source: display(&root.join("Original")),
                name: "NewName".into(),
            },
        })
        .unwrap();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    let intermediate_path = root.join("Original/NewName.typ");
    fs::rename(&note.path, &intermediate_path).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspection = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspection.coherence, "mixedKnown");
    let actual = inspection
        .members
        .iter()
        .find(|m| Path::new(&m.path) == intermediate_path)
        .unwrap();
    assert_eq!(actual.state, "knownIntermediate");
    assert_eq!(actual.current.source.as_deref(), Some(note.source.as_str()));
    assert_eq!(actual.identity_binding, "original");
    assert!(inspection.allowed_actions.is_empty());
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "keepCurrent")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert_eq!(fs::read_to_string(&intermediate_path).unwrap(), note.source);
    assert!(!root.join("NewName").exists());
}
#[test]
fn moved_original_files_in_replaced_parent_do_not_qualify_for_resume() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let (preview, sidecar, after_source) = pair_plan(&mut core, &root, &note, true);
    fs::write(&note.path, &after_source).unwrap();
    let retained = root.join("RetainedParent");
    fs::rename(root.join("Note"), &retained).unwrap();
    fs::create_dir(root.join("Note")).unwrap();
    fs::rename(retained.join("Note.typ"), &note.path).unwrap();
    fs::rename(retained.join("weftext.annotations.json"), &sidecar).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspected = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspected.coherence, "thirdState");
    assert!(inspected.allowed_actions.is_empty());
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert_eq!(fs::read(&note.path).unwrap(), after_source);
    assert_eq!(
        fs::read(&sidecar).unwrap(),
        b"{\"version\":1,\"annotations\":[]}"
    );
}
#[test]
fn draft_reopen_accepts_windows_case_and_separator_spellings_of_same_object() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let path = temp.path().join("Plain.typ");
    fs::write(&path, "base").unwrap();
    let mut core = Core::new(&data).unwrap();
    let note = core.open(&path, None).unwrap();
    core.persist_draft(&note.session_id, "same object retained draft")
        .unwrap();
    drop(core);
    let alternate = PathBuf::from(display(&path).replace('\\', "/").to_lowercase());
    let mut restarted = Core::new(&data).unwrap();
    let reopened = restarted.open(alternate, None).unwrap();
    assert_eq!(
        reopened.recovered_draft.as_deref(),
        Some("same object retained draft")
    );
    assert!(!reopened.recovered_draft_conflict);
}
#[test]
fn strong_frozen_recovery_refuses_new_committed_membership() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let sidecar = Path::new(&note.path)
        .parent()
        .unwrap()
        .join("weftext.annotations.json");
    fs::write(&sidecar, b"{\"version\":1,\"annotations\":[]}").unwrap();
    let admitted = core.admitted_snapshot(&root).unwrap();
    assert!(admitted.complete);
    let bases = core
        .read_snapshot(&root, &[note.path.clone(), display(&sidecar)])
        .unwrap();
    let after_source = format!("{}\nOriginal strong result", note.source).into_bytes();
    let preview = core
        .prepare_admitted_operation(
            OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![
                        FileChange {
                            path: note.path.clone(),
                            base_id: Some(bases.files[0].base_id.clone()),
                            before: bases.files[0].bytes.clone(),
                            after: after_source.clone(),
                        },
                        FileChange {
                            path: display(&sidecar),
                            base_id: Some(bases.files[1].base_id.clone()),
                            before: bases.files[1].bytes.clone(),
                            after: b"{\"version\":1,\"annotations\":[],\"changed\":true}".to_vec(),
                        },
                    ],
                },
            },
            &admitted.cut_id,
        )
        .unwrap();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    fs::write(&note.path, &after_source).unwrap();
    core.create_note_fixture(&root, &root, "LaterMember")
        .unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let before_sidecar = fs::read(&sidecar).unwrap();
    let inspected = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert!(inspected.allowed_actions.is_empty());
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert_eq!(fs::read(&sidecar).unwrap(), before_sidecar);
    assert!(!restarted.committed(&preview.operation_id).unwrap());
}
#[test]
fn strong_frozen_recovery_resumes_original_pair_when_membership_is_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let (preview, sidecar, after_source) = pair_plan_inner(&mut core, &root, &note, true, true);
    fs::write(&note.path, &after_source).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspected = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspected.coherence, "mixedKnown");
    assert!(
        inspected
            .allowed_actions
            .iter()
            .any(|a| a == "resumeOriginalPlan")
    );
    restarted
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    assert!(restarted.committed(&preview.operation_id).unwrap());
    assert_eq!(
        fs::read(&sidecar).unwrap(),
        b"{\"version\":1,\"annotations\":[],\"recovered\":true}"
    );
}
#[test]
fn missing_materialized_participant_never_gets_replaced_by_a_new_object() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let (preview, sidecar, after_source) = pair_plan(&mut core, &root, &note, false);
    fs::write(&note.path, after_source).unwrap();
    let guard = ParentGuard::acquire(&crate::BackendContext::local().unwrap(), &sidecar).unwrap();
    let mut created = guard.open_file(true, true).unwrap();
    core.record_materialized(&preview.operation_id, &sidecar, &created)
        .unwrap();
    operations::write_blob(
        &mut created,
        b"{\"version\":1,\"annotations\":[],\"recovered\":true}",
    )
    .unwrap();
    drop(created);
    drop(guard);
    let retained = sidecar.with_extension("retained.json");
    fs::rename(&sidecar, &retained).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspection = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspection.coherence, "thirdState");
    assert!(inspection.allowed_actions.is_empty());
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert!(!sidecar.exists());
    assert!(retained.exists());
    assert!(!restarted.committed(&preview.operation_id).unwrap());
    fs::rename(&retained, &sidecar).unwrap();
    restarted
        .resolve_recovery(&preview.operation_id, "adoptAfterImages")
        .unwrap();
    assert!(restarted.committed(&preview.operation_id).unwrap());
}
#[test]
fn completed_namespace_after_state_adopts_only_original_directory_and_leaf() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Original").unwrap();
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::RenameNode {
                source: display(&root.join("Original")),
                name: "Moved".into(),
            },
        })
        .unwrap();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    fs::rename(&note.path, root.join("Original").join("Moved.typ")).unwrap();
    fs::rename(root.join("Original"), root.join("Moved")).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspected = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspected.coherence, "knownAfter");
    assert!(
        inspected
            .allowed_actions
            .iter()
            .any(|a| a == "adoptAfterImages")
    );
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "keepCurrent")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert_eq!(
        restarted
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    restarted
        .resolve_recovery(&preview.operation_id, "adoptAfterImages")
        .unwrap();
    let opened = restarted
        .open(root.join("Moved").join("Moved.typ"), Some(root))
        .unwrap();
    assert_eq!(opened.node_id, note.node_id);
    assert_eq!(opened.source, note.source);
    assert!(
        restarted
            .read_births(&preview.operation_id)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn mixed_original_recovery_keeps_readonly_noop_dependency_guarded_with_read_access() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
    let dependency = core
        .create_note_fixture(&root, &root, "ReadonlyDependency")
        .unwrap();
    let sidecar = Path::new(&note.path)
        .parent()
        .unwrap()
        .join("weftext.annotations.json");
    fs::write(&sidecar, b"{\"version\":1,\"annotations\":[]}").unwrap();
    let cut = core
        .read_snapshot(
            &root,
            &[
                note.path.clone(),
                display(&sidecar),
                dependency.path.clone(),
            ],
        )
        .unwrap();
    let after_source = format!("{}\nOriginal mixed proposal", note.source).into_bytes();
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![
                    FileChange {
                        path: note.path.clone(),
                        base_id: Some(cut.files[0].base_id.clone()),
                        before: cut.files[0].bytes.clone(),
                        after: after_source.clone(),
                    },
                    FileChange {
                        path: display(&sidecar),
                        base_id: Some(cut.files[1].base_id.clone()),
                        before: cut.files[1].bytes.clone(),
                        after: b"{\"version\":1,\"annotations\":[],\"recovered\":true}".to_vec(),
                    },
                    FileChange {
                        path: dependency.path.clone(),
                        base_id: Some(cut.files[2].base_id.clone()),
                        before: cut.files[2].bytes.clone(),
                        after: cut.files[2].bytes.clone().unwrap(),
                    },
                ],
            },
        })
        .unwrap();
    let original_permissions = fs::metadata(&dependency.path).unwrap().permissions();
    let mut permission = original_permissions.clone();
    permission.set_readonly(true);
    fs::set_permissions(&dependency.path, permission).unwrap();
    let dependency_modified = fs::metadata(&dependency.path).unwrap().modified().unwrap();
    journal::private_create(
        &crate::BackendContext::local().unwrap(),
        &core.journal_path(&preview.operation_id, "install"),
        b"{\"testInterruption\":true}",
    )
    .unwrap();
    fs::write(&note.path, &after_source).unwrap();
    drop(core);
    let mut restarted = Core::new(&data).unwrap();
    let inspected = restarted.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(inspected.coherence, "mixedKnown");
    assert!(
        inspected
            .allowed_actions
            .iter()
            .any(|a| a == "resumeOriginalPlan")
    );
    restarted
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    assert!(restarted.committed(&preview.operation_id).unwrap());
    assert_eq!(fs::read(&note.path).unwrap(), after_source);
    assert_eq!(
        fs::read(&sidecar).unwrap(),
        b"{\"version\":1,\"annotations\":[],\"recovered\":true}"
    );
    assert_eq!(
        fs::read_to_string(&dependency.path).unwrap(),
        dependency.source
    );
    assert_eq!(
        fs::metadata(&dependency.path).unwrap().modified().unwrap(),
        dependency_modified
    );
    fs::set_permissions(&dependency.path, original_permissions).unwrap();
}

#[test]
fn relocated_bound_primary_does_not_adopt_or_hide_an_unbound_legacy_competitor() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = temp.path().join("library");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note_fixture(&root, &root, "Original").unwrap();
    core.persist_draft(&note.session_id, "bound original proposal")
        .unwrap();
    let destination = root.join("Moved/Moved.typ");
    // Exact old-format input: the original FI is known, but no library binding
    // was retained. It must not gain a binding by colliding with this primary.
    let mut legacy = core.read_draft(Path::new(&note.path)).unwrap().unwrap();
    legacy.library = None;
    legacy.path = display(&destination);
    legacy.source = "unbound retained competitor".into();
    let legacy_path = core.legacy_draft_path(&destination);
    let legacy_bytes = serde_json::to_vec(&legacy).unwrap();
    private_replace(
        &crate::BackendContext::local().unwrap(),
        &legacy_path,
        &legacy_bytes,
    )
    .unwrap();
    operate(
        &mut core,
        &root,
        OperationIntent::RenameNode {
            source: display(&root.join("Original")),
            name: "Moved".into(),
        },
    );
    for iteration in 0..3 {
        if iteration > 0 {
            drop(core);
            core = Core::new(&data).unwrap();
        }
        let opened = core.open(&destination, Some(root.clone())).unwrap();
        assert_eq!(
            opened.recovered_draft.as_deref(),
            Some("bound original proposal")
        );
        assert!(opened.recovered_draft_variants.is_empty());
        let draft = core.read_draft(&destination).unwrap().unwrap();
        assert!(draft.library.is_some());
        assert_eq!(fs::read(&legacy_path).unwrap(), legacy_bytes);
    }
    let opened = core.open(&destination, Some(root.clone())).unwrap();
    core.persist_draft(&opened.session_id, "new scoped input")
        .unwrap();
    let reopened = core.open(&destination, Some(root)).unwrap();
    assert_eq!(
        reopened.recovered_draft.as_deref(),
        Some("new scoped input")
    );
    assert_eq!(fs::read(&legacy_path).unwrap(), legacy_bytes);
}

#[test]
fn draft_complete_record_budget_rejection_preserves_readable_primary_and_original_input() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("note.typ");
    fs::write(&path, "original source").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let opened = core.open(&path, None).unwrap();
    core.persist_draft(&opened.session_id, "retained original proposal")
        .unwrap();
    let record_path = core.draft_path(&path);
    let before = fs::read(&record_path).unwrap();
    let mut draft = core.read_draft(&path).unwrap().unwrap();
    // Every individual source is within its ordinary source limit; only the
    // retained complete record exceeds the matching read/write byte budget.
    draft.alternatives = (0..9)
        .map(|index| DraftAlternative {
            path: display(&path),
            base_source: "original source".into(),
            source: format!("{index}{}", "x".repeat(MAX_SOURCE_BYTES - 1)),
            base_known: true,
        })
        .collect();
    assert_eq!(core.store_draft(&draft).unwrap_err().code, "budget");
    assert_eq!(fs::read(&record_path).unwrap(), before);
    assert_eq!(fs::read_to_string(&path).unwrap(), "original source");
    let reopened = core.open(&path, None).unwrap();
    assert_eq!(
        reopened.recovered_draft.as_deref(),
        Some("retained original proposal")
    );
}
