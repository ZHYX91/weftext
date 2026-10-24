use super::*;

#[test]
fn late_frozen_save_preserves_newer_acked_draft_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("note.typ");
    let private = temp.path().join("private");
    fs::write(&source, "B").unwrap();
    let mut core = Core::new(&private).unwrap();
    let doc = core.open(&source, None).unwrap();
    core.persist_draft_ordered(&doc.session_id, "S", 1).unwrap();
    core.persist_draft_ordered(&doc.session_id, "Y", 2).unwrap();
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
    let draft = core.current_session_draft(&doc.session_id).unwrap().unwrap();
    assert_eq!(draft.source, "Y");
    assert_eq!(draft.role, DraftRole::Proposal);
    assert_eq!(core.save_ordered(intent, 1).unwrap().receipt, receipt);
    drop(core);
    let mut core = Core::new(private).unwrap();
    let recovered = core.open(&source, None).unwrap();
    assert_eq!(recovered.recovered_draft.as_deref(), Some("Y"));
    assert!(recovered.recovered_draft_conflict);
    assert_eq!(fs::read_to_string(source).unwrap(), "S");
}

#[test]
fn later_identical_aba_input_does_not_become_a_clean_baseline() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("note.typ");
    fs::write(&source, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&source, None).unwrap();
    core.persist_draft_ordered(&doc.session_id, "S", 1).unwrap();
    core.persist_draft_ordered(&doc.session_id, "Y", 2).unwrap();
    core.persist_draft_ordered(&doc.session_id, "S", 3).unwrap();
    core.save_ordered(SaveRequest {
        operation_id: fresh(), session_id: doc.session_id.clone(),
        base_id: doc.base_id, source: "S".into(),
    }, 1).unwrap();
    let record = core.current_session_draft(&doc.session_id).unwrap().unwrap();
    assert_eq!(record.source, "S");
    assert_eq!(record.role, DraftRole::Proposal);
    assert!(core.persist_draft_ordered(&doc.session_id, "older", 2).is_err());
}

#[test]
fn a_frozen_cut_cannot_be_relabelled_with_other_source_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("note.typ");
    fs::write(&file, "B").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let doc = core.open(&file, None).unwrap();
    core.persist_draft_ordered(&doc.session_id, "S", 1).unwrap();
    core.persist_draft_ordered(&doc.session_id, "Y", 2).unwrap();
    assert!(core.save_ordered(SaveRequest {
        operation_id: fresh(), session_id: doc.session_id.clone(),
        base_id: doc.base_id, source: "not ACKed S".into(),
    }, 1).is_err());
    assert!(core.persist_draft(&doc.session_id, "legacy overwrote Y").is_err());
    assert_eq!(core.current_session_draft(&doc.session_id).unwrap().unwrap().source, "Y");
}
