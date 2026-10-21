use super::*;

#[test]
fn root_preset_is_one_ordinary_canonical_birth_with_original_query_admission() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("资料库");
    let data = temp.path().join("state");
    let mut core = Core::new(&data).unwrap();
    let op = fresh();
    let input = LibraryCreateOptions {
        initial_root: Some(InitialRootNote {
            title: "可改的工作台😀".into(),
            queries: vec!["tasks".into(), "events".into()],
        }),
        ..Default::default()
    };
    let info = core
        .create_library_with_options_and_id(&root, input.clone(), &op)
        .unwrap();
    assert_eq!(core.operations.len(), 1);
    let path = root.join("资料库.typ");
    assert_eq!(
        info.root_note_path.as_deref(),
        Some(display(&path).as_str())
    );
    let doc = core.open(&path, Some(root.clone())).unwrap();
    assert_eq!(doc.kind, "managed");
    let definitions = weftext_data::saved_queries(&doc.source).unwrap();
    assert_eq!(definitions.len(), 2);
    let schema = core.read_config(&root, ConfigRole::Schema).unwrap();
    for definition in &definitions {
        assert!(
            core.is_definition_key_admitted(&root, doc.node_id.as_ref().unwrap(), &definition.key)
                .unwrap()
        );
        assert_eq!(definition.binding.schema, schema.role_ref);
        assert_eq!(Some(&definition.binding.revision), schema.revision.as_ref());
    }
    let inventory = core.inventory(&root).unwrap();
    assert_eq!(
        inventory
            .entries
            .iter()
            .filter(|e| e.kind == "node")
            .count(),
        1
    );
    assert!(
        core.admitted_compile_snapshot(&root, "live")
            .unwrap()
            .snapshot
            .complete
    );
    let changed = format!(
        "#metadata((id: \"{}\", types: ())) <weftext>\n\n自选正文\n",
        doc.node_id.as_ref().unwrap()
    );
    core.save(SaveRequest {
        session_id: doc.session_id.clone(),
        base_id: doc.base_id.clone(),
        source: changed.clone(),
        operation_id: fresh(),
    })
    .unwrap();
    drop(core);
    let mut core = Core::new(&data).unwrap();
    assert_eq!(
        core.create_library_with_options_and_id(&root, input, &op)
            .unwrap()
            .root_note_path,
        info.root_note_path
    );
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
}

#[test]
fn blank_root_has_no_schema_or_query_and_invalid_preset_creates_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("state");
    let mut core = Core::new(data).unwrap();
    let invalid = temp.path().join("invalid");
    for choices in [vec!["unknown".into()], vec!["tasks".into(), "tasks".into()]] {
        let input = LibraryCreateOptions {
            initial_root: Some(InitialRootNote {
                title: "".into(),
                queries: choices,
            }),
            ..Default::default()
        };
        assert!(
            core.create_library_with_options_and_id(&invalid, input, &fresh())
                .is_err()
        );
        assert!(!invalid.exists());
    }
    let root = temp.path().join("blank");
    core.create_library_with_options_and_id(
        &root,
        LibraryCreateOptions {
            initial_root: Some(InitialRootNote {
                title: "".into(),
                queries: vec![],
            }),
            ..Default::default()
        },
        &fresh(),
    )
    .unwrap();
    assert!(!root.join(".weftext-schema.json").exists());
    let doc = core.open(root.join("blank.typ"), Some(root)).unwrap();
    assert!(weftext_data::saved_queries(&doc.source).unwrap().is_empty());
}

fn options(enabled: bool) -> LibraryCreateOptions {
    LibraryCreateOptions {
        initial_style: Some(InitialLibraryStyle {
            source: "#let library-style(body) = { set text(lang: \"en\", region: none); body }\n"
                .into(),
            enabled,
        }),
        initial_root: None,
    }
}

fn prepare(core: &mut Core, root: &Path, input: LibraryCreateOptions, op: &str) -> journal::Plan {
    let guard = ParentGuard::acquire(root).unwrap();
    let plan = core
        .plan_library_creation(
            root,
            input.clone(),
            op,
            fingerprint(root, &input).unwrap(),
            guard.directory_identity().unwrap(),
        )
        .unwrap();
    core.stage(plan.clone()).unwrap();
    core.install_record(&plan).unwrap();
    plan
}

#[test]
fn library_creation_initial_style_has_one_commit_real_admission_history_and_sync() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let op = fresh();
    let mut core = Core::new(&data).unwrap();
    let input = options(true);
    let info = core
        .create_library_with_options_and_id(&root, input.clone(), &op)
        .unwrap();
    assert_eq!(core.operations.len(), 1);
    let style = core.read_library_style(&root).unwrap();
    assert!(style.installed && style.enabled);
    assert_eq!(style.source, input.initial_style.as_ref().unwrap().source);
    let settings: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".weftext-settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["defaultStyle"]["resource"],
        style.resource_ref.as_ref().unwrap().as_str()
    );
    assert!(!settings.to_string().contains("locale"));
    let snapshot = core.admitted_compile_snapshot(&root, "live").unwrap();
    assert!(snapshot.snapshot.complete);
    assert!(snapshot.style.is_some());
    let node = core
        .open(root.join("Styles/Styles.typ"), Some(root.clone()))
        .unwrap();
    assert_eq!(node.kind, "managed");
    let history = core.list_history(&node.session_id).unwrap();
    assert_eq!(history.versions.len(), 1);
    assert_eq!(history.versions[0].operation_id, op);
    assert_eq!(history.versions[0].status, "available");
    assert_eq!(
        core.read_history(&node.session_id, &op).unwrap().source,
        node.source
    );
    let binding = core.enable_sync(&root, &fresh()).unwrap();
    assert_eq!(binding.identity.workspace_id, info.workspace_id);
    assert!(
        binding
            .installed_objects
            .contains(&sync::ObjectKey::Node(node.node_id.clone().unwrap()))
    );
    let graph = core.read_sync_history(&binding.identity).unwrap();
    let objects = graph.objects().unwrap();
    let style_state = &objects[&sync::ObjectKey::Node(node.node_id.unwrap())].versions[0].state;
    assert_eq!(
        graph.blob(&style_state.files["library-style.typ"]).unwrap(),
        style.source.as_bytes()
    );
    assert!(style_state.files.contains_key("weftext.resources.json"));
    assert!(
        objects[&sync::ObjectKey::Settings].versions[0]
            .state
            .files
            .contains_key(".weftext-settings.json")
    );
    drop(core);
    let mut core = Core::new(&data).unwrap();
    assert_eq!(
        core.create_library_with_options_and_id(&root, input, &op)
            .unwrap()
            .workspace_id,
        info.workspace_id
    );
    assert_eq!(core.operations.len(), 2);
}

#[test]
fn library_creation_disabled_style_and_exact_input_retry_remain_distinct() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let op = fresh();
    let input = options(false);
    core.create_library_with_options_and_id(&root, input.clone(), &op)
        .unwrap();
    let style = core.read_library_style(&root).unwrap();
    assert!(style.installed && !style.enabled);
    assert_eq!(style.source, input.initial_style.as_ref().unwrap().source);
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(true), &op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    let mut changed = input.clone();
    changed.initial_style.as_mut().unwrap().source.push(' ');
    assert_eq!(
        core.create_library_with_options_and_id(&root, changed, &op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        core.create_library_with_id(&root, &op).unwrap_err().code,
        "operationConflict"
    );
    core.create_library_with_options_and_id(&root, input, &op)
        .unwrap();
    assert_eq!(core.operations.len(), 1);
}

#[test]
fn library_creation_pending_subtree_resumes_after_restart_in_same_plan() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let op = fresh();
    let mut core = Core::new(&data).unwrap();
    let mut input = options(true);
    input.initial_root = Some(InitialRootNote {
        title: "待恢复的普通首页".into(),
        queries: vec!["tasks".into(), "events".into()],
    });
    let plan = prepare(&mut core, &root, input.clone(), &op);
    let guard = ParentGuard::acquire(&root).unwrap();
    let directory = guard.create_directory().unwrap();
    core.record_materialized(&op, &root, &directory).unwrap();
    let styles = ParentGuard::child_directory(&directory, "Styles").unwrap();
    core.record_materialized(&op, &root.join("Styles"), &styles)
        .unwrap();
    // An allocated empty leaf is an exact before-state only with this original
    // materialization certificate, never merely because its name is expected.
    let empty = ParentGuard::child_file(&styles, "library-style.typ").unwrap();
    core.record_materialized(&op, &root.join("Styles/library-style.typ"), &empty)
        .unwrap();
    drop((empty, styles, directory, guard, core));
    let mut core = Core::new(&data).unwrap();
    assert!(core.workspace(&root).unwrap().is_none());
    let pending = core.inspect_recovery(&op).unwrap();
    assert!(
        pending
            .allowed_actions
            .contains(&"resumeOriginalPlan".into()),
        "{pending:?}"
    );
    assert_eq!(
        core.create_library_with_options_and_id(&root, input, &op)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert_eq!(
        core.resolve_recovery(&op, "resumeOriginalPlan")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        serde_json::to_vec(&core.operations[&op]).unwrap(),
        serde_json::to_vec(&plan).unwrap()
    );
    assert_eq!(core.operations.len(), 1);
    let original = plan
        .files
        .iter()
        .find(|(name, _)| name == "library.typ")
        .unwrap();
    assert_eq!(fs::read(root.join("library.typ")).unwrap(), original.1);
    let root_note = core
        .open(root.join("library.typ"), Some(root.clone()))
        .unwrap();
    for definition in weftext_data::saved_queries(&root_note.source).unwrap() {
        assert!(
            core.is_definition_key_admitted(
                &root,
                root_note.node_id.as_ref().unwrap(),
                &definition.key
            )
            .unwrap()
        );
    }
    assert!(
        core.admitted_compile_snapshot(&root, "live")
            .unwrap()
            .snapshot
            .complete
    );
    assert_eq!(
        core.read_library_style(&root).unwrap().source,
        options(true).initial_style.unwrap().source
    );
}

#[test]
fn library_creation_after_images_can_be_adopted_with_original_bindings() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let op = fresh();
    let mut core = Core::new(&data).unwrap();
    let plan = prepare(&mut core, &root, options(true), &op);
    let guard = ParentGuard::acquire(&root).unwrap();
    let directory = guard.create_directory().unwrap();
    core.record_materialized(&op, &root, &directory).unwrap();
    let styles = ParentGuard::child_directory(&directory, "Styles").unwrap();
    core.record_materialized(&op, &root.join("Styles"), &styles)
        .unwrap();
    let Mutation::CreateTree { files, .. } = &plan.transaction.as_ref().unwrap().mutation else {
        unreachable!()
    };
    for change in files {
        let parent = if Path::new(&change.path).parent() == Some(root.as_path()) {
            &directory
        } else {
            &styles
        };
        let mut file = ParentGuard::child_file(
            parent,
            Path::new(&change.path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap();
        core.record_materialized(&op, Path::new(&change.path), &file)
            .unwrap();
        write_blob(&mut file, &change.after).unwrap();
    }
    drop((styles, directory, guard, core));
    let mut core = Core::new(&data).unwrap();
    let inspected = core.inspect_recovery(&op).unwrap();
    assert_eq!(inspected.coherence, "knownAfter");
    assert!(
        inspected
            .allowed_actions
            .contains(&"adoptAfterImages".into())
    );
    assert_eq!(
        core.resolve_recovery(&op, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    assert!(core.read_library_style(&root).unwrap().installed);
    assert_eq!(core.read_births(&op).unwrap().len(), 1);
}

#[test]
fn library_creation_same_path_replacement_root_never_becomes_original_result() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let held = temp.path().join("original");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let op = fresh();
    core.create_library_with_options_and_id(&root, options(true), &op)
        .unwrap();
    fs::rename(&root, &held).unwrap();
    fs::create_dir(&root).unwrap();
    for name in [
        ".weftext-workspace.json",
        ".weftext-format",
        ".weftext-rules",
        ".weftext-mvp-commit.json",
        ".weftext-settings.json",
    ] {
        fs::copy(held.join(name), root.join(name)).unwrap();
    }
    let receipt = core
        .create_library_with_options_and_id(&root, options(true), &op)
        .unwrap();
    assert_eq!(receipt.operation_id, op);
    assert_eq!(receipt.status, "committed");
    assert_eq!(
        core.reopen_local_library_creation(&op).err().unwrap().code,
        "committedResultUnavailable"
    );
    assert!(core.workspace(&root).unwrap().is_none());
    assert_eq!(
        core.read_library_style(&root).unwrap_err().code,
        "unadmittedLibrary"
    );
}

#[test]
fn library_creation_recovery_rejects_unbound_equal_bytes_and_changed_member() {
    for bind in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let op = fresh();
        prepare(&mut core, &root, options(true), &op);
        let guard = ParentGuard::acquire(&root).unwrap();
        let directory = guard.create_directory().unwrap();
        if bind {
            core.record_materialized(&op, &root, &directory).unwrap();
        }
        let mut file = ParentGuard::child_file(&directory, ".weftext-format").unwrap();
        if bind {
            core.record_materialized(&op, &root.join(".weftext-format"), &file)
                .unwrap();
        }
        write_blob(
            &mut file,
            if bind {
                b"third state"
            } else {
                b"weftext.typst.v1\n"
            },
        )
        .unwrap();
        drop((file, directory, guard));
        let before = fs::read(root.join(".weftext-format")).unwrap();
        assert_eq!(core.inspect_recovery(&op).unwrap().coherence, "thirdState");
        assert_eq!(
            core.resolve_recovery(&op, "resumeOriginalPlan")
                .unwrap_err()
                .code,
            "recoveryConflict"
        );
        assert_eq!(fs::read(root.join(".weftext-format")).unwrap(), before);
        assert!(!core.committed(&op));
    }
}

#[test]
fn library_creation_unknown_cancel_is_durable_exact_input_fence_without_tree() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    let op = fresh();
    assert_eq!(
        core.cancel_library_creation_with_options_and_id(&root, options(true), &op)
            .unwrap()
            .status,
        "resolved"
    );
    assert!(!root.exists());
    drop(core);
    let mut core = Core::new(&data).unwrap();
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(true), &op)
            .unwrap_err()
            .code,
        "operationCanceled"
    );
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(false), &op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        core.cancel_library_creation_with_options_and_id(&root, options(true), &op)
            .unwrap()
            .status,
        "resolved"
    );
    assert!(!root.exists());
}

#[test]
fn library_creation_none_keeps_legacy_input_and_empty_library_semantics() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let op = fresh();
    let expected = hash(
        &serde_json::to_vec(&serde_json::json!({"kind":"createLibrary","path":display(&root)}))
            .unwrap(),
    );
    core.create_library_with_id(&root, &op).unwrap();
    assert_eq!(core.operations[&op].fingerprint, expected);
    assert!(!root.join("Styles").exists());
    assert!(!root.join(".weftext-settings.json").exists());
    assert!(core.read_births(&op).unwrap().is_empty());
    core.create_library_with_options_and_id(&root, LibraryCreateOptions::default(), &op)
        .unwrap();
    let canceled = fresh();
    let absent = temp.path().join("canceled");
    core.cancel_library_creation_with_options_and_id(
        &absent,
        LibraryCreateOptions::default(),
        &canceled,
    )
    .unwrap();
    assert_eq!(
        core.create_library_with_id(&absent, &canceled)
            .unwrap_err()
            .code,
        "operationCanceled"
    );
    assert!(!absent.exists());
}

#[test]
fn library_creation_pending_parent_replacement_cannot_supply_original_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let root = parent.join("library");
    let data = temp.path().join("private");
    let op = fresh();
    let mut core = Core::new(&data).unwrap();
    prepare(&mut core, &root, options(true), &op);
    drop(core);
    fs::rename(&parent, temp.path().join("original-parent")).unwrap();
    fs::create_dir(&parent).unwrap();
    let mut core = Core::new(&data).unwrap();
    assert_eq!(core.inspect_recovery(&op).unwrap().coherence, "thirdState");
    assert_eq!(
        core.resolve_recovery(&op, "resumeOriginalPlan")
            .unwrap_err()
            .code,
        "recoveryConflict"
    );
    assert!(!root.exists());
}

#[test]
fn library_creation_staged_before_tree_can_cancel_but_partial_tree_cannot() {
    for partial in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let op = fresh();
        prepare(&mut core, &root, options(true), &op);
        if partial {
            let guard = ParentGuard::acquire(&root).unwrap();
            let directory = guard.create_directory().unwrap();
            core.record_materialized(&op, &root, &directory).unwrap();
        }
        let result = core.cancel_library_creation_with_options_and_id(&root, options(true), &op);
        if partial {
            assert_eq!(result.unwrap_err().code, "recoveryConflict");
            assert!(!core.resolved(&op).unwrap());
            assert!(root.exists());
        } else {
            assert_eq!(result.unwrap().status, "resolved");
            assert!(!root.exists());
            assert_eq!(
                core.create_library_with_options_and_id(&root, options(true), &op)
                    .unwrap_err()
                    .code,
                "operationCanceled"
            );
        }
    }
}

#[test]
fn library_creation_rejects_existing_directory_and_oversize_source_before_plan() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    fs::create_dir(&root).unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(true), &fresh())
            .unwrap_err()
            .code,
        "exists"
    );
    assert!(core.operations.is_empty());
    let mut input = options(true);
    input.initial_style.as_mut().unwrap().source = " ".repeat(MAX_SOURCE_BYTES + 1);
    let absent = temp.path().join("absent");
    assert_eq!(
        core.create_library_with_options_and_id(&absent, input, &fresh())
            .unwrap_err()
            .code,
        "tooLarge"
    );
    assert!(!absent.exists());
    assert!(core.operations.is_empty());
}

#[test]
fn library_creation_invalid_native_style_rejected_before_stage_and_complex_source_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    for (i, source) in [
        "#let library-style(body) = { set text(lang: \"en\"; body }",
        "#let another-style(body) = body",
        "#let library-style(body) = body\n#let library-style(body) = body",
    ]
    .iter()
    .enumerate()
    {
        let mut input = options(true);
        input.initial_style.as_mut().unwrap().source = (*source).into();
        let root = temp.path().join(format!("invalid-{i}"));
        assert_eq!(
            core.create_library_with_options_and_id(&root, input, &fresh())
                .unwrap_err()
                .code,
            "invalidStyle"
        );
        assert!(!root.exists());
        assert!(core.operations.is_empty());
    }
    let source = "#let chosen-language = \"en\"\n#let library-style(body) = { set text(lang: chosen-language); body }\n";
    let root = temp.path().join("complex");
    let mut input = options(true);
    input.initial_style.as_mut().unwrap().source = source.into();
    core.create_library_with_options_and_id(&root, input, &fresh())
        .unwrap();
    assert_eq!(core.read_library_style(&root).unwrap().source, source);
}

#[test]
fn library_creation_invalid_raw_request_cancellation_is_single_durable_terminal_fact() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("private");
    let root = PathBuf::from("relative-original-request");
    let mut core = Core::new(&data).unwrap();
    let op = fresh();
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(true), &op)
            .unwrap_err()
            .code,
        "invalidPath"
    );
    assert_eq!(
        core.cancel_library_creation_with_options_and_id(&root, options(true), &op)
            .unwrap()
            .status,
        "resolved"
    );
    assert!(core.journal_path(&op, "resolution").exists());
    assert!(!core.journal_path(&op, "plan").exists());
    assert!(!core.operations.contains_key(&op));
    // The one original resolution is a device fence, never an author Plan.
    drop(core);
    let mut core = Core::new(&data).unwrap();
    let status = core.inspect_recovery(&op).unwrap();
    assert_eq!(status.status, "resolved");
    assert!(status.members.is_empty() && status.allowed_actions.is_empty());
    assert!(status.current_source.is_none());
    assert!(core.recoveries().unwrap().is_empty());
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(true), &op)
            .unwrap_err()
            .code,
        "operationCanceled"
    );
    assert_eq!(
        core.create_library_with_options_and_id(&root, options(false), &op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        core.create_library_with_options_and_id(temp.path().join("different"), options(true), &op)
            .unwrap_err()
            .code,
        "operationConflict"
    );
    core.create_library(temp.path().join("unrelated")).unwrap();
}

#[test]
fn library_creation_cancel_terminal_recognizer_rejects_author_participants_or_changed_input() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let root = temp.path().join("library");
    let op = fresh();
    core.cancel_library_creation_with_options_and_id(&root, options(true), &op)
        .unwrap();
    let original = journal::Plan::file(
        op.clone(),
        request_fingerprint(&root, &options(true)).unwrap(),
        &root,
        None,
        serde_json::to_vec(&options(true)).unwrap(),
        None,
        "cancelLibraryCreation",
    );
    assert!(is_cancellation(&original));
    let other = prepare(
        &mut core,
        &temp.path().join("other"),
        options(true),
        &fresh(),
    );
    let mut changed = original.clone();
    changed.transaction = other.transaction;
    assert!(!is_cancellation(&changed));
    changed = original.clone();
    changed.identity = Some(FileIdentity {
        volume: 1,
        index: 1,
        created: 1,
    });
    assert!(!is_cancellation(&changed));
    changed = original.clone();
    changed.workspace_id = Some(fresh());
    assert!(!is_cancellation(&changed));
    changed = original.clone();
    changed.files.push(("author.typ".into(), vec![]));
    assert!(!is_cancellation(&changed));
    changed = original.clone();
    changed.after = serde_json::to_vec(&options(false)).unwrap();
    assert!(!is_cancellation(&changed));
    changed = original;
    changed.kind = "createLibrary".into();
    assert!(!is_cancellation(&changed));
}
