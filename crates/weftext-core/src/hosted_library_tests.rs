use super::*;

const ACTOR: &str = "deployment:alice";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn scope_fixture() -> (tempfile::TempDir, Core, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    (temp, core, root)
}
fn operation_file(core: &Core, operation: &str) -> PathBuf {
    core.data_dir
        .join("actors")
        .join(hash(ACTOR.as_bytes()))
        .join("operations")
        .join(format!("{operation}.json"))
}

#[test]
fn actor_library_scoped_save_fences_output_and_draft_have_first_original_header() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let operation = fresh();
    let output = fresh();
    let fences = [fresh(), fresh(), fresh(), fresh(), fresh()];
    core.with_actor_for_root(ACTOR, &root, |core| {
        let opened = core.open(&note.path, Some(root.clone()))?;
        core.persist_draft(
            &opened.session_id,
            &(opened.source.clone() + "\nprivate draft"),
        )?;
        let draft: Draft =
            serde_json::from_slice(&fs::read(core.draft_path(Path::new(&note.path)))?)?;
        assert!(draft.library.as_ref().unwrap().workspace_id.is_some());
        core.actor_save(
            "Note",
            HostedSaveRequest {
                session_id: opened.session_id,
                base_id: opened.base_id,
                source: opened.source + "\nSaved.",
                operation_id: operation.clone(),
            },
        )?;
        core.actor_settle("Note", &fences[0])?;
        core.actor_field_settle("Note", "field", &fences[1])?;
        core.actor_feature_settle("Note", &fences[2])?;
        core.actor_definition_settle("Note", &fences[3])?;
        core.actor_history_restore_settle("Note", &fences[4])?;
        assert!(core.actor_output_claim(&output, FP)?);
        Ok(())
    })
    .unwrap();
    for id in std::iter::once(&operation).chain(fences.iter()) {
        let path = operation_file(&core, id);
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"{\"library\":"));
        let mut file = File::open(path).unwrap();
        let ActorLibraryHeader::Bound(binding) = read_actor_library_header(&mut file).unwrap()
        else {
            panic!("missing original scope");
        };
        assert_eq!(
            binding.workspace_id,
            Some(core.require_workspace(&root).unwrap().workspace_id)
        );
    }
    let bytes = fs::read(
        core.data_dir
            .join("actors")
            .join(hash(ACTOR.as_bytes()))
            .join("operations/outputs")
            .join(format!("{output}.json")),
    )
    .unwrap();
    assert!(bytes.starts_with(b"{\"library\":"));
    assert!(bytes.len() <= 4096);
}

#[test]
fn actor_library_scope_rejects_cross_root_session_records_and_restores_after_error_and_panic() {
    let (temp, mut core, root) = scope_fixture();
    let other = temp.path().join("Other");
    core.create_library(&other).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let id = fresh();
    let opened = core
        .with_actor_for_root(ACTOR, &root, |core| {
            core.actor_settle("Note", &id)?;
            core.open(&note.path, Some(root.clone()))
        })
        .unwrap();
    assert_eq!(
        core.with_actor_for_root(ACTOR, &other, |core| core
            .persist_draft(&opened.session_id, "foreign"))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        core.with_actor_for_root(ACTOR, &other, |core| core.actor_save_status(&id))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert!(
        core.actor.is_none() && core.actor_library.is_none() && core.actor_library_path.is_none()
    );
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<()> =
            core.with_actor_for_root(ACTOR, &root, |_| panic!("controlled fixture"));
    }));
    assert!(panic.is_err());
    assert!(
        core.actor.is_none() && core.actor_library.is_none() && core.actor_library_path.is_none()
    );
    assert!(core.with_actor_for_root(ACTOR, &root, |_| Ok(())).is_ok());
}

#[test]
fn actor_library_ordinary_root_is_bound_but_damaged_library_controls_do_not_become_ordinary() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ordinary");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("note.typ"), "ordinary").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let id = fresh();
    core.with_actor_for_root(ACTOR, &root, |core| {
        let doc = core.open(root.join("note.typ"), Some(root.clone()))?;
        core.persist_draft(&doc.session_id, "ordinary draft")?;
        core.actor_settle("ordinary", &id)?;
        Ok(())
    })
    .unwrap();
    let record: ActorOperation =
        serde_json::from_slice(&fs::read(operation_file(&core, &id)).unwrap()).unwrap();
    assert!(record.library.is_some());
    assert!(record.library.unwrap().workspace_id.is_none());
    fs::write(root.join(".weftext-workspace.json"), b"broken").unwrap();
    assert_eq!(
        core.with_actor_for_root(ACTOR, &root, |_| Ok(()))
            .unwrap_err()
            .code,
        "unadmittedLibrary"
    );
}

#[test]
fn actor_library_legacy_records_and_recovered_draft_are_not_backfilled() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let operation = fresh();
    let output = fresh();
    core.with_actor(ACTOR, |core| {
        let doc = core.open(&note.path, Some(root.clone()))?;
        core.persist_draft(&doc.session_id, "legacy draft")?;
        core.actor_settle("Note", &operation)?;
        core.actor_output_claim(&output, FP)?;
        Ok(())
    })
    .unwrap();
    let original = fs::read(operation_file(&core, &operation)).unwrap();
    core.with_actor_for_root(ACTOR, &root, |core| {
        assert_eq!(
            core.actor_save_status(&operation)?.unwrap().status,
            "cancelled"
        );
        assert!(!core.actor_output_claim(&output, FP)?);
        let doc = core.open(&note.path, Some(root.clone()))?;
        assert_eq!(doc.recovered_draft.as_deref(), Some("legacy draft"));
        core.persist_draft(&doc.session_id, "another draft")?;
        let draft: Draft =
            serde_json::from_slice(&fs::read(core.draft_path(Path::new(&note.path)))?)?;
        assert!(draft.library.is_none());
        Ok(())
    })
    .unwrap();
    assert_eq!(
        fs::read(operation_file(&core, &operation)).unwrap(),
        original
    );
}

#[test]
fn actor_library_first_header_does_not_parse_payload_and_is_bounded_and_rewinds() {
    let (temp, mut core, root) = scope_fixture();
    let binding = core
        .with_actor_for_root(ACTOR, &root, |core| Ok(core.actor_library.clone().unwrap()))
        .unwrap();
    let path = temp.path().join("record.json");
    let mut bytes = b"{\"library\":".to_vec();
    bytes.extend(serde_json::to_vec(&binding).unwrap());
    bytes.extend_from_slice(b",\"feature\":THIS_IS_NOT_JSON");
    fs::write(&path, bytes).unwrap();
    let mut file = File::open(&path).unwrap();
    file.seek(SeekFrom::Start(7)).unwrap();
    assert!(
        matches!(read_actor_library_header(&mut file).unwrap(), ActorLibraryHeader::Bound(b) if b == binding)
    );
    assert_eq!(file.stream_position().unwrap(), 7);
    let mut oversized = binding;
    oversized.root = format!("D:/{}", "x".repeat(5000));
    fs::write(
        &path,
        format!(
            "{{\"library\":{}}}",
            serde_json::to_string(&oversized).unwrap()
        ),
    )
    .unwrap();
    assert_eq!(
        read_actor_library_header(&mut File::open(&path).unwrap())
            .err()
            .unwrap()
            .code,
        "budget"
    );
    fs::write(&path, b"{\"actor\":\"old\",\"feature\":BROKEN").unwrap();
    assert!(matches!(
        read_actor_library_header(&mut File::open(path).unwrap()).unwrap(),
        ActorLibraryHeader::LegacyUnbound
    ));
}

#[test]
fn actor_library_legacy_backup_validator_requires_original_plan_and_exact_namespace() {
    let (temp, mut core, root) = scope_fixture();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let operation = fresh();
    let unknown = fresh();
    core.with_actor(ACTOR, |core| {
        let doc = core.open(&note.path, Some(root.clone()))?;
        core.actor_save(
            "Note",
            HostedSaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                source: doc.source + "\nlegacy saved",
                operation_id: operation.clone(),
            },
        )?;
        core.actor_settle("Note", &unknown)?;
        Ok(())
    })
    .unwrap();
    let binding = core
        .with_actor_for_root(ACTOR, &root, |core| Ok(core.actor_library.clone().unwrap()))
        .unwrap();
    let path = operation_file(&core, &operation);
    let bytes = fs::read(&path).unwrap();
    assert!(
        core.validate_legacy_actor_backup_record(&path, &bytes, &binding)
            .is_ok()
    );
    assert!(
        core.validate_scoped_actor_record(&path, &bytes, &binding)
            .is_err()
    );
    let unknown_path = operation_file(&core, &unknown);
    assert!(
        core.validate_legacy_actor_backup_record(
            &unknown_path,
            &fs::read(&unknown_path).unwrap(),
            &binding
        )
        .is_err()
    );
    let wrong = core
        .data_dir
        .join("actors")
        .join(hash(b"different actor"))
        .join("operations")
        .join(format!("{operation}.json"));
    assert!(
        core.validate_legacy_actor_backup_record(&wrong, &bytes, &binding)
            .is_err()
    );
    let other = temp.path().join("Other");
    core.create_library(&other).unwrap();
    let other_binding = core
        .with_actor_for_root(ACTOR, &other, |core| {
            Ok(core.actor_library.clone().unwrap())
        })
        .unwrap();
    assert!(
        core.validate_legacy_actor_backup_record(&path, &bytes, &other_binding)
            .is_err()
    );
}

#[test]
fn actor_library_bound_clean_draft_remains_authoritative_over_same_fi_unbound_record() {
    let (_temp, mut core, root) = scope_fixture();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let bound_path = core
        .with_actor_for_root(ACTOR, &root, |core| {
            let opened = core.open(&note.path, Some(root.clone()))?;
            core.persist_draft(&opened.session_id, &opened.source)?;
            Ok(core.draft_path(Path::new(&note.path)))
        })
        .unwrap();
    // Both records come from real API contexts, with the same actor and FI.
    // The older unbound context remains unbound and must not steal a subsequent
    // explicitly scoped proposal from its existing clean primary.
    let (unbound_path, unbound_bytes) = core
        .with_actor(ACTOR, |core| {
            let opened = core.open(&note.path, Some(root.clone()))?;
            assert!(opened.recovered_draft.is_none());
            core.persist_draft(&opened.session_id, "retained unbound proposal")?;
            let path = core.draft_path(Path::new(&note.path));
            let bytes = fs::read(&path)?;
            let draft: Draft = serde_json::from_slice(&bytes)?;
            assert!(draft.library.is_none());
            Ok((path, bytes))
        })
        .unwrap();
    assert_ne!(bound_path, unbound_path);
    core.with_actor_for_root(ACTOR, &root, |core| {
        let opened = core.open(&note.path, Some(root.clone()))?;
        assert!(opened.recovered_draft.is_none());
        core.persist_draft(&opened.session_id, "new bound proposal")?;
        let reopened = core.open(&note.path, Some(root.clone()))?;
        assert_eq!(
            reopened.recovered_draft.as_deref(),
            Some("new bound proposal")
        );
        assert_eq!(core.draft_path(Path::new(&note.path)), bound_path);
        let draft: Draft = serde_json::from_slice(&fs::read(&bound_path)?)?;
        assert!(draft.library.is_some());
        Ok(())
    })
    .unwrap();
    assert_eq!(fs::read(&unbound_path).unwrap(), unbound_bytes);
    core.with_actor(ACTOR, |core| {
        let opened = core.open(&note.path, Some(root.clone()))?;
        assert_eq!(
            opened.recovered_draft.as_deref(),
            Some("retained unbound proposal")
        );
        Ok(())
    })
    .unwrap();
}
