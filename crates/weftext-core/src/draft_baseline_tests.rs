use super::*;

fn save(core: &mut Core, doc: &OpenedDocument, source: &str) -> Receipt {
    core.save(SaveRequest {
        session_id: doc.session_id.clone(),
        base_id: doc.base_id.clone(),
        source: source.into(),
        operation_id: fresh(),
    })
    .unwrap()
    .receipt
}

#[test]
fn committed_baselines_survive_redundant_persist_and_retire_without_history_budget_growth() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ordinary.typ");
    let data = temp.path().join("private");
    fs::write(&path, "initial").unwrap();
    let mut core = Core::new(&data).unwrap();
    let mut doc = core.open(&path, None).unwrap();
    let mut original = None;
    for index in 0..130 {
        let source = format!("Saved {index} 中文😀");
        let receipt = save(&mut core, &doc, &source);
        doc.base_id = receipt.base_id.clone();
        core.persist_draft(&doc.session_id, &source).unwrap();
        core.persist_draft(&doc.session_id, &source).unwrap();
        let record = core
            .current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            record.role,
            DraftRole::CommittedBaseline {
                operation_id: receipt.operation_id.clone()
            }
        );
        assert!(record.alternatives.is_empty());
        let expected = hash(source.as_bytes());
        core.flush_document_draft(DocumentDraftFlush {
            session_id: doc.session_id.clone(),
            base_id: doc.base_id.clone(),
            source_digest: expected,
        })
        .unwrap();
        if original.is_none() {
            original = Some((
                receipt.operation_id.clone(),
                fs::read(core.journal_path(&receipt.operation_id, "plan")).unwrap(),
                fs::read(core.journal_path(&receipt.operation_id, "commit")).unwrap(),
            ));
        }
        let producer = core.sessions[&doc.session_id].draft_producer_id.clone();
        core.close_session(&doc.session_id).unwrap();
        if index % 17 == 0 {
            drop(core);
            core = Core::new(&data).unwrap();
        }
        doc = core.open(&path, None).unwrap();
        assert_ne!(core.sessions[&doc.session_id].draft_producer_id, producer);
        assert!(doc.recovered_draft.is_none());
        assert!(!doc.recovered_draft_conflict);
    }
    assert_eq!(
        fs::read_dir(core.draft_directory("draft-history"))
            .unwrap()
            .count(),
        0
    );
    let (id, plan, commit) = original.unwrap();
    assert_eq!(fs::read(core.journal_path(&id, "plan")).unwrap(), plan);
    assert_eq!(fs::read(core.journal_path(&id, "commit")).unwrap(), commit);
    core.persist_draft(&doc.session_id, "Latest uncommitted Y")
        .unwrap();
    drop(core);
    let mut core = Core::new(data).unwrap();
    let recovered = core.open(path, None).unwrap();
    assert_eq!(
        recovered.recovered_draft.as_deref(),
        Some("Latest uncommitted Y")
    );
    assert!(!recovered.recovered_draft_conflict);
    assert!(recovered.recovered_draft_variants.is_empty());
}

#[test]
fn mixed_normalization_consumes_only_closed_baselines_and_keeps_the_complete_remaining_view() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ordinary.typ");
    fs::write(&path, "initial").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let a = core.open(&path, None).unwrap();
    let b = core.open(&path, None).unwrap();
    core.persist_draft(&b.session_id, "Independent B 😀")
        .unwrap();
    let b_input = core.sessions[&b.session_id].draft_input.clone().unwrap();
    let receipt = save(&mut core, &a, "Committed A");
    let original_plan = fs::read(core.journal_path(&receipt.operation_id, "plan")).unwrap();
    let active = core.current_session_draft(&b.session_id).unwrap().unwrap();
    assert!(matches!(active.role, DraftRole::CommittedBaseline { .. }));
    assert!(
        active
            .alternatives
            .iter()
            .any(|entry| same_proposal(entry, &b_input))
    );
    core.close_session(&a.session_id).unwrap();
    let opened = core.open(&path, None).unwrap();
    assert_eq!(opened.recovered_draft.as_deref(), Some("Independent B 😀"));
    assert!(opened.recovered_draft_conflict);
    let stored = core
        .current_session_draft(&opened.session_id)
        .unwrap()
        .unwrap();
    assert!(same_proposal(&stored.proposal(), &b_input));
    assert!(stored.alternatives.is_empty());
    assert_eq!(
        opened.recovered_draft_witness,
        Some(stored.witness().unwrap())
    );
    assert_eq!(
        fs::read(core.journal_path(&receipt.operation_id, "plan")).unwrap(),
        original_plan
    );
    assert_eq!(
        fs::read_dir(core.draft_directory("draft-history"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn equal_text_undo_and_original_retry_cannot_issue_a_baseline_role() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ordinary.typ");
    fs::write(&path, "initial").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let a = core.open(&path, None).unwrap();
    let operation_id = fresh();
    let request = || SaveRequest {
        session_id: a.session_id.clone(),
        base_id: a.base_id.clone(),
        source: "S".into(),
        operation_id: operation_id.clone(),
    };
    core.save(request()).unwrap();
    core.persist_draft(&a.session_id, "Y").unwrap();
    core.persist_draft(&a.session_id, "S").unwrap();
    assert_eq!(
        core.current_session_draft(&a.session_id)
            .unwrap()
            .unwrap()
            .role,
        DraftRole::Proposal
    );
    core.save(request()).unwrap();
    assert_eq!(
        core.current_session_draft(&a.session_id)
            .unwrap()
            .unwrap()
            .role,
        DraftRole::Proposal
    );
    let a_producer = core.sessions[&a.session_id].draft_producer_id.clone();
    core.close_session(&a.session_id).unwrap();
    let b = core.open(path, None).unwrap();
    core.persist_draft(&b.session_id, "Independent Z").unwrap();
    let stored = core.current_session_draft(&b.session_id).unwrap().unwrap();
    assert!(stored.alternatives.iter().any(|entry| entry.source == "S"
        && entry.role == DraftRole::Proposal
        && entry.producer_id.as_ref() == Some(&a_producer)));
}

#[test]
fn baseline_requires_original_durable_evidence_and_never_acks_over_the_retained_record() {
    for failure in ["producer", "plan", "commit", "header"] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ordinary.typ");
        fs::write(&path, "initial").unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let a = core.open(&path, None).unwrap();
        let receipt = save(&mut core, &a, "S");
        let b = core.open(&path, None).unwrap();
        core.close_session(&a.session_id).unwrap();
        let record_path = core.draft_path(&path);
        let original_plan = core.journal_path(&receipt.operation_id, "plan");
        match failure {
            "producer" => {
                let mut raw: serde_json::Value =
                    serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
                raw["producerId"] = serde_json::json!(fresh());
                fs::write(&record_path, serde_json::to_vec(&raw).unwrap()).unwrap();
            }
            "plan" | "commit" => {
                let original = core.journal_path(&receipt.operation_id, failure);
                fs::rename(
                    original,
                    temp.path().join(format!("retained-{failure}.json")),
                )
                .unwrap();
            }
            "header" => {
                let mut binding = core.operations[&receipt.operation_id]
                    .file_save
                    .clone()
                    .unwrap();
                binding.draft_producer_id = fresh();
                let identity = core.operations[&receipt.operation_id]
                    .identity
                    .clone()
                    .unwrap();
                let raw = format!(
                    "{{\"fileSave\":{},\"identity\":{},\"after\":NOT_JSON_PROTECTED_BODY",
                    serde_json::to_string(&binding).unwrap(),
                    serde_json::to_string(&identity).unwrap()
                );
                fs::write(original_plan, raw).unwrap();
            }
            _ => unreachable!(),
        }
        let before = fs::read(&record_path).unwrap();
        assert!(core.persist_draft(&b.session_id, "Y").is_err(), "{failure}");
        assert_eq!(fs::read(&record_path).unwrap(), before, "{failure}");
        assert!(core.sessions[&b.session_id].draft_input.is_none());
        assert_eq!(fs::read(&path).unwrap(), b"S");
    }
}

#[test]
fn saved_descendant_baselines_follow_original_directory_members_across_restart() {
    for parent_node in [false, true] {
        for restart_before in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("library");
            let data = temp.path().join("private");
            let mut core = Core::new(&data).unwrap();
            core.create_library(&root).unwrap();
            if parent_node {
                core.create_note_fixture(&root, &root, "Folder").unwrap();
            } else {
                fs::create_dir(root.join("Folder")).unwrap();
                let admitted = core
                    .prepare_directory_admission(&root, root.join("Folder"), &fresh())
                    .unwrap();
                core.execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
                    .unwrap();
            }
            let doc = core
                .create_note_fixture(&root, root.join("Folder"), "Child")
                .unwrap();
            let source = format!("{}\nSaved descendant 中文😀", doc.source);
            let receipt = save(&mut core, &doc, &source);
            core.close_session(&doc.session_id).unwrap();
            if restart_before {
                drop(core);
                core = Core::new(&data).unwrap();
            }
            let preview = core
                .prepare_operation(OperationRequest {
                    operation_id: fresh(),
                    root: display(&root),
                    intent: if parent_node {
                        OperationIntent::RenameNode {
                            source: display(&root.join("Folder")),
                            name: "Moved".into(),
                        }
                    } else {
                        OperationIntent::RelocateDirectory {
                            source: display(&root.join("Folder")),
                            parent: display(&root),
                            name: "Moved".into(),
                        }
                    },
                })
                .unwrap();
            core.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            let path = root.join("Moved/Child/Child.typ");
            for _ in 0..3 {
                let moved = core.open(&path, Some(root.clone())).unwrap();
                assert_eq!(moved.source, source);
                assert!(moved.recovered_draft.is_none());
                assert!(!moved.recovered_draft_conflict);
                core.close_session(&moved.session_id).unwrap();
            }
            let history_before = fs::read_dir(core.draft_directory("draft-history"))
                .unwrap()
                .count();
            drop(core);
            let mut core = Core::new(&data).unwrap();
            let moved = core.open(&path, Some(root.clone())).unwrap();
            assert_eq!(
                core.operation_status(&receipt.operation_id).unwrap().status,
                "committed"
            );
            let next = format!("{source}\nFurther input");
            core.persist_draft(&moved.session_id, &next).unwrap();
            core.flush_document_draft(DocumentDraftFlush {
                session_id: moved.session_id.clone(),
                base_id: moved.base_id.clone(),
                source_digest: hash(next.as_bytes()),
            })
            .unwrap();
            save(&mut core, &moved, &next);
            assert_eq!(
                fs::read_dir(core.draft_directory("draft-history"))
                    .unwrap()
                    .count(),
                history_before
            );
        }
    }
}

#[test]
fn close_readback_qualifies_foreign_fi_before_malformed_or_oversized_body() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("own.typ");
    let other = temp.path().join("other.typ");
    fs::write(&path, "original").unwrap();
    fs::write(&other, "foreign").unwrap();
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let own = core.open(&path, None).unwrap();
    let foreign = core.open(&other, None).unwrap();
    core.persist_draft(&own.session_id, "Own pending").unwrap();
    let record = core.draft_path(&path);
    let scope = core.draft_session_scope(&own.session_id).unwrap();
    let identity = &core.sessions[&foreign.session_id].stamp.identity;
    let header = format!(
        "{{\"scope\":{},\"identity\":{},\"source\":",
        serde_json::to_string(&scope).unwrap(),
        serde_json::to_string(identity).unwrap()
    );
    for bytes in [
        format!("{header}NOT_JSON").into_bytes(),
        [header.as_bytes(), &vec![b'x'; 16 * 1024 * 1024 + 1]].concat(),
    ] {
        fs::write(&record, &bytes).unwrap();
        let error = core
            .flush_document_draft(DocumentDraftFlush {
                session_id: own.session_id.clone(),
                base_id: own.base_id.clone(),
                source_digest: hash(b"Own pending"),
            })
            .unwrap_err();
        assert_eq!(error.code, "privateEvidence");
        assert_eq!(fs::read(&record).unwrap(), bytes);
    }
    assert_eq!(fs::read(&path).unwrap(), b"original");
}
