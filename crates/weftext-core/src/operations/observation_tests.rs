use super::*;

fn fixture() -> (tempfile::TempDir, Core, CapturedFile, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("state")).unwrap();
    core.create_library(&root).unwrap();
    let operation_id = fresh();
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: operation_id.clone(),
            root: display(&root),
            intent: OperationIntent::NewNode {
                parent: display(&root),
                name: "Note".into(),
                source: None,
            },
        })
        .unwrap();
    let receipt = core
        .execute_operation(&operation_id, &preview.plan_id)
        .unwrap();
    let captured = capture_optional(&core.backend, &Path::new(&receipt.path).join("Note.typ"))
        .unwrap()
        .unwrap();
    (temp, core, captured, operation_id)
}

#[test]
fn observation_record_cut_preserves_fact_order_and_required_read_failures() {
    let (_temp, core, captured, operation_id) = fixture();
    let original = core.read_observation_facts(&operation_id).unwrap();
    let accepted = original
        .iter()
        .find(|fact| fact.stamp == captured.stamp && fact.hash == hash(&captured.bytes))
        .unwrap();
    assert_eq!(
        core.accepted_revision(&captured).unwrap().as_deref(),
        Some(accepted.revision.as_str())
    );
    let primary = core.journal_path(&operation_id, "observations");
    let recovery = core.journal_path(&operation_id, &format!("recovery-observations-{}", fresh()));
    let mut recovered = accepted.clone();
    recovered.revision = fresh();
    fs::write(
        &recovery,
        serde_json::to_vec(&vec![recovered.clone()]).unwrap(),
    )
    .unwrap();
    let mut cut = core.observation_record_paths().unwrap();
    // Deliberate input order proves the shared reader neither sorts records
    // nor turns the original first matching fact into a last-writer map.
    cut.paths = vec![recovery.clone(), primary];
    let facts = core
        .read_observation_facts_from_paths(&operation_id, &cut)
        .unwrap();
    assert_eq!(facts[0].revision, recovered.revision);
    assert_eq!(
        serde_json::to_value(&facts[1..]).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    fs::write(&recovery, b"{invalid original record").unwrap();
    assert!(
        core.read_observation_facts_from_paths(&operation_id, &cut)
            .is_err()
    );
    assert_eq!(fs::read(&recovery).unwrap(), b"{invalid original record");
    cut.paths.clear();
    assert_eq!(
        core.read_observation_facts_from_paths(&operation_id, &cut)
            .err()
            .unwrap()
            .code,
        "identityPending"
    );
}

#[test]
fn observation_record_cut_rejects_new_relevant_names_without_replacing_the_cut() {
    let (_temp, core, _captured, operation_id) = fixture();
    let cut = core.observation_record_paths().unwrap();
    let plans = core.committed_plans().unwrap();
    core.check_observation_record_paths(&cut, &plans).unwrap();
    let journal = core
        .journal_path(&operation_id, "plan")
        .parent()
        .unwrap()
        .to_path_buf();
    fs::write(
        journal.join(format!("{operation_id}.unrelated.json")),
        b"{}",
    )
    .unwrap();
    core.check_observation_record_paths(&cut, &plans).unwrap();
    let late = journal.join(format!(
        "{operation_id}.recovery-observations-{}.json",
        fresh()
    ));
    fs::write(&late, b"[]").unwrap();
    assert_eq!(
        core.check_observation_record_paths(&cut, &plans)
            .unwrap_err()
            .code,
        "identityPending"
    );
    assert!(!cut.paths.contains(&late));
    assert_eq!(fs::read(late).unwrap(), b"[]");
    // A new invocation may observe the retained new name; the old cut never
    // silently adopts it to replace a conclusion formed from earlier facts.
    core.check_observation_record_paths(&core.observation_record_paths().unwrap(), &plans)
        .unwrap();
}

// Exercise the unchanged public qualification families, rather than the
// implementation of the call-local observation or a second binding algorithm.
fn qualification_outcomes(
    core: &Core,
    root: &Path,
    path: &Path,
    session_id: &str,
    owner: &weftext_knowledge::NodeRef,
) -> [std::result::Result<(), String>; 3] {
    [
        core.qualified_node_at(root, path)
            .map(|_| ())
            .map_err(|e| e.code),
        core.qualified_node_for_session_at(root, path, session_id)
            .map(|_| ())
            .map_err(|e| e.code),
        core.qualified_query_node_at(root, path, owner, "live")
            .map(|_| ())
            .map_err(|e| e.code),
    ]
}

#[test]
fn qualified_read_observation_reloads_original_facts_after_a_successful_call() {
    let (_temp, mut core, captured, operation_id) = fixture();
    let path = Path::new(&captured.path);
    let root = path.parent().unwrap().parent().unwrap();
    let document = core.open(path, Some(root.to_path_buf())).unwrap();
    let owner = core.qualified_node_at(root, path).unwrap().owner;
    assert!(
        qualification_outcomes(&core, root, path, &document.session_id, &owner)
            .iter()
            .all(|r| r.is_ok())
    );
    for suffix in ["commit", "births", "observations"] {
        let record = core.journal_path(&operation_id, suffix);
        let original = fs::read(&record).unwrap();
        let damaged = b"{invalid retained original evidence";
        fs::write(&record, damaged).unwrap();
        assert!(
            qualification_outcomes(&core, root, path, &document.session_id, &owner)
                .iter()
                .all(|r| r.is_err()),
            "{suffix}"
        );
        assert_eq!(fs::read(&record).unwrap(), damaged);
        fs::write(&record, original).unwrap();
        assert!(
            qualification_outcomes(&core, root, path, &document.session_id, &owner)
                .iter()
                .all(|r| r.is_ok()),
            "{suffix}"
        );
    }
}

#[test]
fn qualified_read_observation_first_binding_does_not_hide_a_later_bad_birth() {
    let (_temp, mut core, captured, _operation_id) = fixture();
    let root = Path::new(&captured.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let later = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::NewNode {
                parent: display(&root),
                name: "Other".into(),
                source: None,
            },
        })
        .unwrap();
    core.execute_operation(&later.operation_id, &later.plan_id)
        .unwrap();
    // HashMap iteration is deliberately unspecified. Pick an actual earlier
    // matching producer and an actual later producer from the current order;
    // do not assume lexical or preparation-time ordering.
    let (path, later_operation) = {
        let producers = core
            .committed_plans()
            .unwrap()
            .into_iter()
            .filter(|plan| {
                plan.transaction
                    .as_ref()
                    .is_some_and(|t| !t.births().is_empty())
            })
            .collect::<Vec<_>>();
        assert!(producers.len() >= 2);
        let first = producers.first().unwrap();
        let last = producers.last().unwrap();
        assert_ne!(first.operation_id, last.operation_id);
        (
            PathBuf::from(&first.transaction.as_ref().unwrap().births()[0].path),
            last.operation_id.clone(),
        )
    };
    let document = core.open(&path, Some(root.clone())).unwrap();
    let owner = core.qualified_node_at(&root, &path).unwrap().owner;
    assert!(
        qualification_outcomes(&core, &root, &path, &document.session_id, &owner)
            .iter()
            .all(|r| r.is_ok())
    );
    let record = core.journal_path(&later_operation, "births");
    let original = fs::read(&record).unwrap();
    let damaged = b"{invalid later original Birth";
    fs::write(&record, damaged).unwrap();
    assert!(
        qualification_outcomes(&core, &root, &path, &document.session_id, &owner)
            .iter()
            .all(|r| r.is_err())
    );
    assert_eq!(fs::read(&record).unwrap(), damaged);
    fs::write(&record, original).unwrap();
    assert!(
        qualification_outcomes(&core, &root, &path, &document.session_id, &owner)
            .iter()
            .all(|r| r.is_ok())
    );
}

#[test]
fn qualified_read_observation_keeps_current_file_and_related_pending_checks() {
    let (_temp, mut core, captured, _operation_id) = fixture();
    let path = Path::new(&captured.path);
    let root = path.parent().unwrap().parent().unwrap();
    let document = core.open(path, Some(root.to_path_buf())).unwrap();
    let owner = core.qualified_node_at(root, path).unwrap().owner;
    let held = root.join("held-original.typ");
    fs::rename(path, &held).unwrap();
    fs::write(path, &captured.bytes).unwrap();
    let replaced = qualification_outcomes(&core, root, path, &document.session_id, &owner);
    assert!(
        replaced
            .iter()
            .all(|r| r.as_ref().err().map(String::as_str) == Some("identityPending"))
    );
    assert_eq!(fs::read(path).unwrap(), captured.bytes);
    // These are owned temporary fixture files. Restore the original object
    // before separately creating the retained install-only Pending checkpoint.
    fs::remove_file(path).unwrap();
    fs::rename(&held, path).unwrap();
    assert!(
        qualification_outcomes(&core, root, path, &document.session_id, &owner)
            .iter()
            .all(|r| r.is_ok())
    );
    let pending = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(root),
            intent: OperationIntent::RenameNode {
                source: display(path.parent().unwrap()),
                name: "Moved".into(),
            },
        })
        .unwrap();
    core.install_record(&core.operations[&pending.operation_id])
        .unwrap();
    let blocked = qualification_outcomes(&core, root, path, &document.session_id, &owner);
    assert!(
        blocked
            .iter()
            .all(|r| r.as_ref().err().map(String::as_str) == Some("recoveryPending"))
    );
    assert_eq!(fs::read(path).unwrap(), captured.bytes);
    assert!(!core.committed(&pending.operation_id).unwrap());
}
