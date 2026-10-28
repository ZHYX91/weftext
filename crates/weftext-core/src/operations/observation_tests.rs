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
