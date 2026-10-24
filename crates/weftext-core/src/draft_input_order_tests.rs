use super::*;

#[test]
fn late_frozen_save_preserves_newer_acked_draft_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("note.typ");
    let private = temp.path().join("private");
    fs::write(&source, "B").unwrap();
    let mut core = Core::new(&private).unwrap();
    let doc = core.open(&source, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    let operation_id = fresh();
    let intent = SaveRequest {
        operation_id: operation_id.clone(),
        session_id: doc.session_id.clone(),
        base_id: doc.base_id.clone(),
        source: "S".into(),
    };
    let receipt = core.save_ordered(intent.clone(), 1).unwrap().receipt;
    assert_eq!(receipt.operation_id, operation_id);
    assert_eq!(fs::read_to_string(&source).unwrap(), "S");
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.source, "Y");
    assert_eq!(draft.role, DraftRole::Proposal);
    assert_eq!(core.save_ordered(intent, 1).unwrap().receipt, receipt);
    drop(core);
    let mut core = Core::new(private).unwrap();
    let recovered = core.open(&source, None).unwrap();
    assert_eq!(recovered.recovered_draft.as_deref(), Some("Y"));
    assert!(!recovered.recovered_draft_conflict);
    assert_eq!(fs::read_to_string(source).unwrap(), "S");
}

#[test]
fn late_save_delivery_keeps_the_active_writer_and_newer_proposal() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    let operation_id = fresh();
    let saved = core
        .save_ordered(
            SaveRequest {
                operation_id: operation_id.clone(),
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: "S".into(),
            },
            1,
        )
        .unwrap();
    assert!(saved.recovery_issue.is_none());
    let (_, delivered) = core
        .refresh_local_saved_session(&operation_id, &doc.session_id)
        .unwrap();
    assert_eq!(delivered.session_id, doc.session_id);
    assert_eq!(delivered.recovered_draft.as_deref(), Some("Y"));
    assert!(!delivered.recovered_draft_conflict);
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.role, DraftRole::Proposal);
    assert_eq!(draft.base_source, "S");
    core.save_ordered(
        SaveRequest {
            operation_id: fresh(),
            session_id: doc.session_id.clone(),
            base_id: delivered.base_id,
            source: "Y".into(),
        },
        2,
    )
    .unwrap();
    assert_eq!(fs::read_to_string(note).unwrap(), "Y");
}

#[test]
fn an_ack_retry_and_frozen_save_requalify_the_actual_durable_draft() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    let session = &core.sessions[&doc.session_id];
    let draft = core
        .object_draft_path(
            &session.stamp.identity,
            session.file_scope.as_ref().unwrap(),
        )
        .unwrap();
    let original = fs::read(&draft).unwrap();
    fs::write(&draft, b"{broken").unwrap();
    assert!(core.persist_test_input(&doc.session_id, "S", 1).is_err());
    assert!(
        core.save_ordered(
            SaveRequest {
                operation_id: fresh(),
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: "S".into(),
            },
            1
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(note).unwrap(), "B");
    fs::write(&draft, original).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
}

#[test]
fn later_identical_aba_input_does_not_become_a_clean_baseline() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("note.typ");
    fs::write(&source, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&source, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    core.persist_test_input(&doc.session_id, "S", 3).unwrap();
    core.save_ordered(
        SaveRequest {
            operation_id: fresh(),
            session_id: doc.session_id.clone(),
            base_id: doc.base_id,
            source: "S".into(),
        },
        1,
    )
    .unwrap();
    let record = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(record.source, "S");
    assert_eq!(record.role, DraftRole::Proposal);
    assert!(
        core.persist_test_input(&doc.session_id, "older", 2)
            .is_err()
    );
}

#[test]
fn a_frozen_cut_cannot_be_relabelled_with_other_source_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("note.typ");
    fs::write(&file, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&file, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    assert!(
        core.save_ordered(
            SaveRequest {
                operation_id: fresh(),
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: "not ACKed S".into(),
            },
            1
        )
        .is_err()
    );
    assert!(
        core.persist_draft(&doc.session_id, "legacy overwrote Y")
            .is_err()
    );
    assert_eq!(
        core.current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap()
            .source,
        "Y"
    );
}

#[test]
fn qualified_protected_draft_read_count_for_one_normal_persist() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_draft(&doc.session_id, "S").unwrap();
    QUALIFIED_DRAFT_READ_CALLS.with(|count| count.set(0));
    core.persist_draft(&doc.session_id, "T").unwrap();
    let reads = QUALIFIED_DRAFT_READ_CALLS.with(std::cell::Cell::get);
    // Windows/NTFS fixture: one complete read before merging, one exact
    // protected read before replacement; no third scope-only payload read.
    assert_eq!(reads, 2, "protected Draft reads: {reads}");
    assert_eq!(
        core.current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap()
            .source,
        "T"
    );
}

#[test]
fn save_as_cannot_relabel_frozen_ack_with_different_source_or_newer_producer() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    let target = temp.path().join("separate.typ");
    let wrong = core.save_as_ordered(
        SaveAsRequest {
            session_id: doc.session_id.clone(),
            path: display(&target),
            source: "not S".into(),
            operation_id: fresh(),
        },
        1,
    );
    assert_eq!(wrong.unwrap_err().code, "draftChanged");
    assert!(!target.exists());
    let saved = core
        .save_as_ordered(
            SaveAsRequest {
                session_id: doc.session_id.clone(),
                path: display(&target),
                source: "S".into(),
                operation_id: fresh(),
            },
            1,
        )
        .unwrap();
    assert_eq!(saved.receipt.status, "saved");
    assert_eq!(fs::read_to_string(&target).unwrap(), "S");
    assert_eq!(
        core.current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap()
            .source,
        "Y"
    );
}

#[test]
fn out_of_order_ack_and_aba_never_make_the_earlier_cut_clean() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    assert_eq!(
        core.current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap()
            .source,
        "Y"
    );
    let request = SaveRequest {
        operation_id: fresh(),
        session_id: doc.session_id.clone(),
        base_id: doc.base_id,
        source: "S".into(),
    };
    assert_eq!(core.save(request.clone()).unwrap_err().code, "draftChanged");
    assert_eq!(fs::read_to_string(&note).unwrap(), "B");
    core.persist_test_input(&doc.session_id, "S", 3).unwrap();
    core.save_ordered(request, 1).unwrap();
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.source, "S");
    assert_eq!(draft.role, DraftRole::Proposal);
    assert_eq!(core.sessions[&doc.session_id].draft_input_revision, Some(3));
}

#[test]
fn generated_afterimage_cannot_claim_that_newer_input_was_based_on_it() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_test_input(&doc.session_id, "S", 1).unwrap();
    core.persist_test_input(&doc.session_id, "Y", 2).unwrap();
    let id = fresh();
    let outcome = core
        .save_ordered_derived(
            SaveRequest {
                operation_id: id.clone(),
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: "generated X from S".into(),
            },
            "S".into(),
            1,
        )
        .unwrap();
    let (_, delivered) = core
        .refresh_local_saved_session(&id, &doc.session_id)
        .unwrap();
    assert_eq!(delivered.recovered_draft.as_deref(), Some("Y"));
    assert!(delivered.recovered_draft_conflict);
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.base_source, "B");
    assert_eq!(draft.role, DraftRole::Proposal);
    assert_eq!(
        core.save_ordered(
            SaveRequest {
                operation_id: fresh(),
                session_id: doc.session_id.clone(),
                base_id: outcome.receipt.base_id,
                source: "Y".into()
            },
            2
        )
        .unwrap_err()
        .code,
        "draftConflict"
    );
    assert_eq!(fs::read_to_string(&note).unwrap(), "generated X from S");
    core.resolve_draft(&doc.session_id, "mergeRecovered", Some("reviewed X and Y"))
        .unwrap();
    assert_eq!(
        core.persist_test_input(&doc.session_id, "Y", 2)
            .unwrap_err()
            .code,
        "draftChanged"
    );
    core.persist_test_input(&doc.session_id, "reviewed X and Y", 3)
        .unwrap();
    core.save_ordered(
        SaveRequest {
            operation_id: fresh(),
            session_id: doc.session_id.clone(),
            base_id: core.sessions[&doc.session_id].base_id.clone(),
            source: "reviewed X and Y".into(),
        },
        3,
    )
    .unwrap();
    assert_eq!(fs::read_to_string(note).unwrap(), "reviewed X and Y");
}

fn actual_input(
    doc: &OpenedDocument,
    source: &str,
    revision: u64,
    input_id: &str,
) -> DraftInputRequest {
    DraftInputRequest {
        session_id: doc.session_id.clone(),
        source: source.into(),
        input_revision: revision,
        input_id: input_id.into(),
        base_id: Some(doc.base_id.clone()),
        retained_base_source: None,
    }
}
#[test]
fn debounced_equal_byte_edit_after_committed_s_remains_a_proposal_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    let doc = core.open(&note, None).unwrap();
    let original = fresh();
    core.persist_draft_ordered(actual_input(&doc, "S", 1, &original))
        .unwrap();
    let outcome = core
        .save_ordered(
            SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id.clone(),
                source: "S".into(),
                operation_id: fresh(),
            },
            1,
        )
        .unwrap();
    let mut current = doc.clone();
    current.base_id = outcome.receipt.base_id;
    // T and the Undo back to S happened within debounce: only new S is sent.
    let returned = fresh();
    core.persist_draft_ordered(actual_input(&current, "S", 2, &returned))
        .unwrap();
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.role, DraftRole::Proposal);
    assert_eq!(draft.input_id, Some(returned));
    drop(core);
    fs::write(&note, "external X").unwrap();
    let mut core = Core::new(&data).unwrap();
    let opened = core.open(&note, None).unwrap();
    assert_eq!(opened.recovered_draft.as_deref(), Some("S"));
    assert_eq!(opened.recovered_draft_base_source.as_deref(), Some("S"));
    assert!(opened.recovered_draft_conflict);
}
#[test]
fn redundant_same_generation_ack_preserves_only_its_qualified_committed_baseline() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    let doc = core.open(&note, None).unwrap();
    let input_id = fresh();
    core.persist_draft_ordered(actual_input(&doc, "S", 1, &input_id))
        .unwrap();
    let outcome = core
        .save_ordered(
            SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id.clone(),
                source: "S".into(),
                operation_id: fresh(),
            },
            1,
        )
        .unwrap();
    let mut current = doc.clone();
    current.base_id = outcome.receipt.base_id;
    core.persist_draft_ordered(actual_input(&current, "S", 2, &input_id))
        .unwrap();
    assert!(matches!(
        core.current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap()
            .role,
        DraftRole::CommittedBaseline { .. }
    ));
    drop(core);
    fs::write(&note, "external X").unwrap();
    let mut core = Core::new(&data).unwrap();
    assert!(core.open(&note, None).unwrap().recovered_draft.is_none());
}
#[test]
fn late_y_ack_uses_the_ui_base_before_generated_x_instead_of_mutable_core_base() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    core.persist_draft_ordered(actual_input(&doc, "S", 1, &fresh()))
        .unwrap();
    let outcome = core
        .save_ordered_derived(
            SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id.clone(),
                source: "generated X".into(),
                operation_id: fresh(),
            },
            "S".into(),
            1,
        )
        .unwrap();
    core.persist_draft_ordered(actual_input(&doc, "Y", 2, &fresh()))
        .unwrap();
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.source, "Y");
    assert_eq!(draft.base_source, "B");
    assert_eq!(
        core.save_ordered(
            SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: outcome.receipt.base_id,
                source: "Y".into(),
                operation_id: fresh()
            },
            2
        )
        .unwrap_err()
        .code,
        "draftConflict"
    );
    assert_eq!(fs::read_to_string(&note).unwrap(), "generated X");
}
#[test]
fn generated_staged_failure_does_not_replace_the_acknowledged_author_cut() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    let input = actual_input(&doc, "S", 1, &fresh());
    core.persist_draft_ordered(input.clone()).unwrap();
    hosted::set_test_failpoint("saveStaged");
    assert_eq!(
        core.save_ordered_derived(
            SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id.clone(),
                source: "generated X".into(),
                operation_id: fresh()
            },
            "S".into(),
            1
        )
        .unwrap_err()
        .code,
        "injectedFailure"
    );
    let draft = core
        .current_session_draft(&doc.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(draft.source, "S");
    assert_eq!(draft.base_source, "B");
    assert_eq!(draft.input_id, Some(input.input_id));
    assert_eq!(fs::read_to_string(&note).unwrap(), "B");
}
#[test]
fn ordered_ack_cannot_borrow_an_unknown_base_or_retransmit_a_different_generation() {
    let temp = tempfile::tempdir().unwrap();
    let note = temp.path().join("note.typ");
    fs::write(&note, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&note, None).unwrap();
    let input = actual_input(&doc, "S", 1, &fresh());
    core.persist_draft_ordered(input.clone()).unwrap();
    let mut changed = input.clone();
    changed.input_id = fresh();
    assert_eq!(
        core.persist_draft_ordered(changed).unwrap_err().code,
        "draftChanged"
    );
    let mut changed = input.clone();
    changed.input_revision = 2;
    changed.base_id = Some(fresh());
    assert_eq!(
        core.persist_draft_ordered(changed).unwrap_err().code,
        "draftChanged"
    );
    let mut retained = input;
    retained.input_revision = 2;
    retained.base_id = None;
    retained.retained_base_source = Some("earlier unknown B".into());
    core.persist_draft_ordered(retained).unwrap();
    assert!(
        !core
            .current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap()
            .base_known
    );
}
