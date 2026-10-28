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

#[test]
fn bounded_snapshot_batch_keeps_input_order_absence_and_distinct_original_revisions() {
    let (_temp, mut core, first, _operation_id) = fixture();
    let root = Path::new(&first.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let second = core.create_note_fixture(&root, &root, "Second").unwrap();
    let second_capture = capture_optional(&core.backend, Path::new(&second.path))
        .unwrap()
        .unwrap();
    let ordinary = root.join("ordinary.typ");
    fs::write(&ordinary, b"ordinary original").unwrap();
    let paths = vec![
        display(&root.join("absent-first.json")),
        first.path.clone(),
        display(&ordinary),
        second.path.clone(),
        display(&root.join("absent-last.json")),
    ];
    let first_revision = core.accepted_revision(&first).unwrap().unwrap();
    let second_revision = core.accepted_revision(&second_capture).unwrap().unwrap();
    assert_ne!(first_revision, second_revision);
    let first_owner = core
        .admitted_binding(&first.stamp.identity)
        .unwrap()
        .unwrap()
        .node_id;
    let cut = core.read_snapshot(&root, &paths).unwrap();
    assert!(cut.complete);
    assert_eq!(
        cut.files.iter().map(|file| &file.path).collect::<Vec<_>>(),
        paths.iter().collect::<Vec<_>>()
    );
    for index in [0, 4] {
        let file = &cut.files[index];
        assert!(file.bytes.is_none());
        assert!(file.node_id.is_none());
        assert!(file.revision.is_none());
        assert_eq!(file.currentness, "observed");
    }
    assert_eq!(cut.files[1].bytes.as_ref(), Some(&first.bytes));
    assert_eq!(cut.files[1].node_id.as_ref(), Some(&first_owner));
    assert_eq!(cut.files[1].revision.as_ref(), Some(&first_revision));
    assert_eq!(cut.files[3].node_id, second.node_id);
    assert_eq!(cut.files[3].revision.as_ref(), Some(&second_revision));
    assert_eq!(cut.files[2].kind, "file");
    assert!(cut.files[2].node_id.is_none());
    assert!(cut.files[2].revision.is_none());
    assert_eq!(cut.files[2].currentness, "observed");
    let bases = cut
        .files
        .iter()
        .map(|file| file.base_id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(bases.len(), paths.len());
    for file in &cut.files {
        core.validate_snapshot(&root, &file.path, &file.base_id)
            .unwrap();
    }
    let later = core.read_snapshot(&root, &paths).unwrap();
    assert!(
        later
            .files
            .iter()
            .all(|file| !bases.contains(&file.base_id))
    );
    assert_eq!(later.files[1].revision, cut.files[1].revision);
    assert_eq!(later.files[3].revision, cut.files[3].revision);
}

#[test]
fn bounded_snapshot_batch_reloads_original_records_and_never_publishes_partial_failure() {
    let (_temp, mut core, captured, operation_id) = fixture();
    let root = Path::new(&captured.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let other = core.create_note_fixture(&root, &root, "Other").unwrap();
    let paths = vec![
        captured.path.clone(),
        display(&root.join("absent.json")),
        other.path,
    ];
    core.read_snapshot(&root, &paths).unwrap();
    for suffix in ["commit", "births", "observations"] {
        let record = core.journal_path(&operation_id, suffix);
        let original = fs::read(&record).unwrap();
        let damaged = b"{invalid retained batch evidence";
        let before = core.snapshots.len();
        fs::write(&record, damaged).unwrap();
        assert!(core.read_snapshot(&root, &paths).is_err(), "{suffix}");
        assert_eq!(core.snapshots.len(), before, "{suffix}");
        assert_eq!(fs::read(&record).unwrap(), damaged);
        fs::write(&record, &original).unwrap();
        core.read_snapshot(&root, &paths).unwrap();
        if suffix != "commit" {
            let retained = record.with_extension("retained-fixture");
            fs::rename(&record, &retained).unwrap();
            let before = core.snapshots.len();
            assert_eq!(
                core.read_snapshot(&root, &paths).err().unwrap().code,
                "identityPending",
                "{suffix}"
            );
            assert_eq!(core.snapshots.len(), before);
            assert_eq!(fs::read(&retained).unwrap(), original);
            fs::rename(&retained, &record).unwrap();
            core.read_snapshot(&root, &paths).unwrap();
        }
    }
}

#[test]
fn bounded_absent_snapshot_does_not_turn_unrelated_journal_damage_into_presence() {
    let (_temp, mut core, captured, operation_id) = fixture();
    let root = Path::new(&captured.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let ordinary = root.join("ordinary.typ");
    fs::write(&ordinary, b"ordinary original").unwrap();
    let absent = display(&root.join("absent.json"));
    for suffix in ["births", "observations"] {
        let record = core.journal_path(&operation_id, suffix);
        let original = fs::read(&record).unwrap();
        let damaged = b"{invalid unrelated original";
        fs::write(&record, damaged).unwrap();
        let cut = core
            .read_snapshot(&root, std::slice::from_ref(&absent))
            .unwrap();
        assert!(cut.files[0].bytes.is_none());
        core.validate_snapshot(&root, &absent, &cut.files[0].base_id)
            .unwrap();
        let before = core.snapshots.len();
        assert!(
            core.read_snapshot(&root, &[absent.clone(), display(&ordinary)])
                .is_err()
        );
        assert_eq!(core.snapshots.len(), before);
        assert_eq!(fs::read(&record).unwrap(), damaged);
        fs::write(&record, original).unwrap();
    }
}

#[test]
fn managed_resource_snapshot_retains_only_returned_resource_and_manifest_tokens() {
    let (_temp, mut core, captured, _operation_id) = fixture();
    let path = Path::new(&captured.path);
    let directory = path.parent().unwrap();
    let root = directory.parent().unwrap();
    let manifest = directory.join("weftext.resources.json");
    let before = core.snapshots.len();
    for _ in 0..3 {
        assert!(
            core.managed_resource_snapshot(root, path)
                .unwrap()
                .is_empty()
        );
        assert_eq!(core.snapshots.len(), before);
    }
    fs::write(&manifest, br#"{"version":1,"resources":[]}"#).unwrap();
    assert!(
        core.managed_resource_snapshot(root, path)
            .unwrap()
            .is_empty()
    );
    assert_eq!(core.snapshots.len(), before);
    let damaged = b"{invalid original resource manifest";
    fs::write(&manifest, damaged).unwrap();
    assert!(core.managed_resource_snapshot(root, path).is_err());
    assert_eq!(core.snapshots.len(), before);
    assert_eq!(fs::read(&manifest).unwrap(), damaged);
    let id = fresh();
    let bytes = serde_json::to_vec(&serde_json::json!({
        "version":1,"resources":[{"resourceId":id,"path":"asset.bin"}]
    }))
    .unwrap();
    fs::write(&manifest, &bytes).unwrap();
    assert_eq!(
        core.managed_resource_snapshot(root, path)
            .err()
            .unwrap()
            .code,
        "resourceMissing"
    );
    assert_eq!(core.snapshots.len(), before);
    assert_eq!(fs::read(&manifest).unwrap(), bytes);
    fs::write(directory.join("asset.bin"), b"original resource bytes").unwrap();
    let resources = core.managed_resource_snapshot(root, path).unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].resource_id, id);
    assert_eq!(resources[0].identity_status, "unadmitted");
    assert_eq!(core.snapshots.len(), before + 2);
    let resource = &resources[0];
    for (path, base) in [
        (&resource.path, &resource.base_id),
        (&resource.manifest_path, &resource.manifest_base_id),
    ] {
        core.validate_snapshot(root, path, base).unwrap();
    }
    let witness = core.snapshot_witness(&resource.base_id).unwrap();
    core.release_snapshot_bases(&[resource.base_id.clone(), resource.manifest_base_id.clone()]);
    assert_eq!(core.snapshots.len(), before);
    core.validate_snapshot_witness(root, &resource.path, &witness)
        .unwrap();
}

#[test]
fn managed_resource_batch_keeps_ordered_member_errors_and_reloads_original_facts() {
    let (_temp, mut core, captured, operation_id) = fixture();
    let first = Path::new(&captured.path);
    let root = first.parent().unwrap().parent().unwrap().to_path_buf();
    let second = core.create_note_fixture(&root, &root, "Second").unwrap();
    let first_manifest = first.parent().unwrap().join("weftext.resources.json");
    let second_directory = Path::new(&second.path).parent().unwrap();
    fs::write(&first_manifest, b"{invalid retained manifest").unwrap();
    let resource_id = fresh();
    fs::write(
        second_directory.join("weftext.resources.json"),
        serde_json::to_vec(&serde_json::json!({
            "version":1,"resources":[{"resourceId":resource_id,"path":"asset.bin"}]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(second_directory.join("asset.bin"), b"resource bytes").unwrap();
    let paths = vec![captured.path.clone(), second.path, captured.path.clone()];
    let before = core.snapshots.len();
    let batch = core.managed_resource_snapshots(&root, &paths).unwrap();
    assert!(!batch.budget_exhausted);
    assert_eq!(batch.entries.len(), paths.len());
    assert_eq!(core.snapshots.len(), before + 2);
    for (index, (path, _)) in batch.entries.iter().enumerate() {
        assert_eq!(path, &paths[index]);
    }
    assert!(batch.entries[0].1.is_err());
    assert!(batch.entries[2].1.is_err());
    let resource = &batch.entries[1].1.as_ref().unwrap()[0];
    assert_eq!(resource.resource_id, resource_id);
    assert_eq!(resource.identity_status, "unadmitted");
    core.validate_snapshot(&root, &resource.path, &resource.base_id)
        .unwrap();
    for suffix in ["commit", "births", "observations"] {
        let record = core.journal_path(&operation_id, suffix);
        let original = fs::read(&record).unwrap();
        let damaged = b"{invalid retained resource qualification";
        fs::write(&record, damaged).unwrap();
        let before = core.snapshots.len();
        let batch = core.managed_resource_snapshots(&root, &paths).unwrap();
        assert_eq!(batch.entries.len(), paths.len());
        assert!(batch.entries.iter().all(|(_, result)| result.is_err()));
        assert_eq!(core.snapshots.len(), before);
        assert_eq!(fs::read(&record).unwrap(), damaged);
        fs::write(&record, original).unwrap();
    }
}

#[test]
fn managed_resource_batch_rechecks_complete_original_resource_births() {
    let (_temp, mut core, captured, _operation_id) = fixture();
    let path = Path::new(&captured.path);
    let root = path.parent().unwrap().parent().unwrap();
    let preview = core
        .prepare_import_resource(root, path, "owned.bin", b"owned bytes".to_vec(), &fresh())
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let paths = vec![captured.path.clone()];
    let batch = core.managed_resource_snapshots(root, &paths).unwrap();
    let resource = &batch.entries[0].1.as_ref().unwrap()[0];
    assert_eq!(resource.identity_status, "admitted");
    assert!(resource.revision.is_some());
    assert_eq!(resource.bytes, b"owned bytes");
    let record = core.journal_path(&preview.operation_id, "resource-births");
    let original = fs::read(&record).unwrap();
    let damaged = b"{invalid original resource Birth";
    fs::write(&record, damaged).unwrap();
    let before = core.snapshots.len();
    let rejected = core.managed_resource_snapshots(root, &paths).unwrap();
    assert!(rejected.entries[0].1.is_err());
    assert!(core.managed_resource_snapshot(root, path).is_err());
    assert_eq!(core.snapshots.len(), before);
    assert_eq!(fs::read(&record).unwrap(), damaged);
    fs::write(&record, original).unwrap();
    let batch = core.managed_resource_snapshots(root, &paths).unwrap();
    assert_eq!(
        batch.entries[0].1.as_ref().unwrap()[0].identity_status,
        "admitted"
    );
}

#[test]
fn managed_resource_batch_stops_at_original_aggregate_budget_without_publishing_excess() {
    let (_temp, mut core, captured, _operation_id) = fixture();
    let path = Path::new(&captured.path);
    let directory = path.parent().unwrap();
    let root = directory.parent().unwrap();
    let leaf = directory.join("large-owned-fixture.bin");
    fs::File::create(&leaf)
        .unwrap()
        .set_len(33 * 1024 * 1024)
        .unwrap();
    fs::write(
        directory.join("weftext.resources.json"),
        serde_json::to_vec(&serde_json::json!({
            "version":1,"resources":[{"resourceId":fresh(),"path":"large-owned-fixture.bin"}]
        }))
        .unwrap(),
    )
    .unwrap();
    let before = core.snapshots.len();
    let batch = core
        .managed_resource_snapshots(
            root,
            &[
                captured.path.clone(),
                captured.path.clone(),
                display(&root.join("NeverCaptured/NeverCaptured.typ")),
            ],
        )
        .unwrap();
    assert!(batch.budget_exhausted);
    assert_eq!(batch.entries.len(), 1);
    let resource = &batch.entries[0].1.as_ref().unwrap()[0];
    assert_eq!(resource.bytes.len(), 33 * 1024 * 1024);
    assert_eq!(core.snapshots.len(), before + 2);
    core.validate_snapshot(root, &resource.path, &resource.base_id)
        .unwrap();
    assert_eq!(fs::metadata(&leaf).unwrap().len(), 33 * 1024 * 1024);
}

#[test]
fn empty_resource_batch_still_requires_the_original_library_authority() {
    let (temp, mut core, captured, _operation_id) = fixture();
    let root = Path::new(&captured.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let before = core.snapshots.len();
    let batch = core.managed_resource_snapshots(root, &[]).unwrap();
    assert!(batch.entries.is_empty());
    assert!(!batch.budget_exhausted);
    assert_eq!(core.snapshots.len(), before);
    let unadmitted = temp.path().join("unadmitted");
    fs::create_dir(&unadmitted).unwrap();
    assert!(core.managed_resource_snapshots(&unadmitted, &[]).is_err());
    assert_eq!(core.snapshots.len(), before);
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
    let snapshots = core.snapshots.len();
    assert!(core.read_snapshot(&root, &[display(&path)]).is_err());
    assert_eq!(core.snapshots.len(), snapshots);
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
