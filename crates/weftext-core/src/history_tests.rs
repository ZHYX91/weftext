use super::*;
use std::os::windows::fs::OpenOptionsExt;

struct HistoryFixture {
    core: Core,
    _temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    doc: OpenedDocument,
}
impl HistoryFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let doc = core.create_note(&root, &root, "Note").unwrap();
        Self {
            core,
            _temp: temp,
            data,
            root,
            doc,
        }
    }
    fn save(&mut self, suffix: &str) -> (String, String) {
        let source = format!("{}\n{suffix}", self.doc.source);
        let operation_id = fresh();
        let receipt = self
            .core
            .save(SaveRequest {
                session_id: self.doc.session_id.clone(),
                base_id: self.doc.base_id.clone(),
                source: source.clone(),
                operation_id: operation_id.clone(),
            })
            .unwrap();
        self.doc.source = source.clone();
        self.doc.base_id = receipt.base_id;
        (operation_id, source)
    }
    fn initial_version(&self) -> String {
        self.core
            .list_history(&self.doc.session_id)
            .unwrap()
            .versions
            .iter()
            .find(|v| v.kind == "createNote")
            .unwrap()
            .version_id
            .clone()
    }
    fn restore_request(
        &self,
        version: &str,
        action: &str,
        source: Option<&str>,
    ) -> HistoryRestoreRequest {
        HistoryRestoreRequest {
            session_id: self.doc.session_id.clone(),
            base_id: self.doc.base_id.clone(),
            version_id: version.into(),
            operation_id: fresh(),
            draft_action: action.into(),
            draft_source: source.map(str::to_owned),
        }
    }
    fn operate(&mut self, intent: OperationIntent) -> OperationReceipt {
        let p = self
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&self.root),
                intent,
            })
            .unwrap();
        self.core
            .execute_operation(&p.operation_id, &p.plan_id)
            .unwrap()
    }
}

#[test]
fn history_lists_real_commits_and_read_compare_preserve_saved_and_draft_bytes() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let (second, source2) = f.save("v2 中文");
    let (third, source3) = f.save("v3");
    let draft = format!("{source3}\nunsaved draft");
    f.core.persist_draft(&f.doc.session_id, &draft).unwrap();
    let draft_path = f.core.draft_path(Path::new(&f.doc.path));
    let draft_bytes = fs::read(&draft_path).unwrap();
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    assert_eq!(timeline.versions.len(), 3);
    assert_eq!(
        timeline.current_saved_version_id.as_deref(),
        Some(third.as_str())
    );
    assert_eq!(timeline.currentness, "saved");
    assert!(!timeline.retention.complete);
    assert_eq!(timeline.retention.available_versions, 3);
    assert!(
        timeline
            .versions
            .iter()
            .all(|v| v.saved_at_unix_ms.is_some()
                && v.time_basis == "commitClock"
                && v.commit_sequence.is_some())
    );
    assert_eq!(
        f.core
            .read_history(&f.doc.session_id, &second)
            .unwrap()
            .source,
        source2
    );
    let compared = f.core.compare_history(&f.doc.session_id, &initial).unwrap();
    assert_eq!(compared.current.source, source3);
    assert_eq!(compared.comparison, "currentSaved");
    assert!(!compared.equal);
    assert!(compared.selected.read_only);
    assert_eq!(compared.selected.preview_status, "sourceOnly");
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(fs::read(&f.doc.path).unwrap(), source3.as_bytes());
    // First creation's exact SourceRevision is also readable through the
    // existing revision API, not just through the timeline projection.
    let initial_read = f.core.read_history(&f.doc.session_id, &initial).unwrap();
    assert_eq!(
        f.core
            .read_source_revision(
                &f.root,
                f.doc.node_id.as_ref().unwrap(),
                initial_read.version.revision_id.as_ref().unwrap()
            )
            .unwrap(),
        initial_read.source.as_bytes()
    );
}

#[test]
fn configured_owner_history_reads_exact_admitted_revision_without_other_sources() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let old = f.core.read_history(&f.doc.session_id, &initial).unwrap();
    let revision = old.version.revision_id.as_ref().unwrap();
    let owner = f.doc.node_id.as_ref().unwrap().clone();
    let (middle, middle_source) = f.save("saved revision is retained");
    let middle_revision = f
        .core
        .read_history(&f.doc.session_id, &middle)
        .unwrap()
        .version
        .revision_id
        .unwrap();
    let (_, saved) = f.save("latest is different");
    assert_eq!(
        f.core
            .read_source_revision(&f.root, &owner, &middle_revision)
            .unwrap(),
        middle_source.as_bytes()
    );
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    let other = f.core.create_note(&f.root, &f.root, "Other").unwrap();
    let guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&other.path)
        .unwrap();
    assert_eq!(
        f.core
            .read_source_revision_at(&f.root, &f.doc.path, &owner, revision)
            .unwrap(),
        old.source.as_bytes()
    );
    assert_eq!(
        f.core
            .read_source_revision_at(&f.root, &f.doc.path, &owner, &middle_revision)
            .unwrap(),
        middle_source.as_bytes()
    );
    let configured = f.root.join("Note/Note.typ");
    assert_eq!(
        f.core
            .read_source_revision_at(&f.root, &configured, &owner, &middle_revision)
            .unwrap(),
        middle_source.as_bytes()
    );
    assert!(
        f.core
            .read_source_revision_at(&f.root, &f.doc.path, &owner, &fresh())
            .is_err()
    );
    assert!(
        f.core
            .read_source_revision_at(
                &f.root,
                &f.doc.path,
                other.node_id.as_ref().unwrap(),
                revision,
            )
            .is_err()
    );
    drop(guard);
    assert!(
        f.core
            .read_source_revision_at(&f.root, &other.path, &owner, revision)
            .is_err()
    );
    fs::write(&f.doc.path, format!("{saved}\nnot admitted")).unwrap();
    assert!(
        f.core
            .read_source_revision_at(&f.root, &f.doc.path, &owner, revision)
            .is_err()
    );
    assert!(
        fs::read_to_string(&f.doc.path)
            .unwrap()
            .ends_with("not admitted")
    );
}

#[test]
fn history_legacy_time_and_clock_rollback_never_replace_commit_relationship() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let (second, _) = f.save("v2");
    let (third, _) = f.save("v3");
    for (id, time) in [(&initial, None), (&second, Some(200)), (&third, Some(100))] {
        let path = f.core.journal_path(id, "commit");
        let mut fact: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if let Some(time) = time {
            fact["savedAtUnixMs"] = serde_json::json!(time);
        } else {
            fact.as_object_mut().unwrap().remove("savedAtUnixMs");
            fact.as_object_mut().unwrap().remove("commitSequence");
        }
        fs::write(path, serde_json::to_vec(&fact).unwrap()).unwrap();
    }
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    let old = timeline
        .versions
        .iter()
        .find(|v| v.version_id == initial)
        .unwrap();
    assert_eq!(old.saved_at_unix_ms, None);
    assert_eq!(old.time_basis, "unknown");
    assert_eq!(old.commit_sequence, None);
    let v2 = timeline
        .versions
        .iter()
        .find(|v| v.version_id == second)
        .unwrap();
    let v3 = timeline
        .versions
        .iter()
        .find(|v| v.version_id == third)
        .unwrap();
    assert!(v2.saved_at_unix_ms > v3.saved_at_unix_ms);
    assert!(v2.commit_sequence < v3.commit_sequence);
    assert_eq!(
        timeline.current_saved_version_id.as_deref(),
        Some(third.as_str())
    );
    // Tied display clocks still carry the real commit relationship.
    let path = f.core.journal_path(&third, "commit");
    let mut fact: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    fact["savedAtUnixMs"] = serde_json::json!(200);
    fs::write(path, serde_json::to_vec(&fact).unwrap()).unwrap();
    let tied = f.core.list_history(&f.doc.session_id).unwrap();
    assert_eq!(tied.versions[0].version_id, third);
}

#[test]
fn history_restore_retains_both_persisted_and_newer_buffer_and_retries_original() {
    let mut f = HistoryFixture::new();
    let initial_source = f.doc.source.clone();
    let initial = f.initial_version();
    let (v2, source2) = f.save("v2");
    let (v3, source3) = f.save("v3");
    let persisted = format!("{source3}\npersisted draft");
    let buffer = format!("{source3}\nnewer editor buffer");
    f.core.persist_draft(&f.doc.session_id, &persisted).unwrap();
    let denied = f.restore_request(&initial, "none", None);
    assert_eq!(
        f.core.restore_history(denied).unwrap_err().code,
        "draftProtectionRequired"
    );
    assert_eq!(fs::read(&f.doc.path).unwrap(), source3.as_bytes());
    let request = f.restore_request(&initial, "retain", Some(&buffer));
    let result = f.core.restore_history(request.clone()).unwrap();
    assert_eq!(result.source, initial_source);
    assert_eq!(result.receipt.status, "saved");
    assert_eq!(result.protected_drafts.len(), 2);
    assert!(
        result
            .protected_drafts
            .iter()
            .any(|d| d.source == persisted)
    );
    assert!(result.protected_drafts.iter().any(|d| d.source == buffer));
    assert_eq!(
        f.core.read_history(&f.doc.session_id, &v2).unwrap().source,
        source2
    );
    assert_eq!(
        f.core.read_history(&f.doc.session_id, &v3).unwrap().source,
        source3
    );
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    assert_eq!(timeline.versions.len(), 4);
    assert_eq!(
        timeline.current_saved_version_id.as_deref(),
        Some(request.operation_id.as_str())
    );
    let retried = f.core.restore_history(request.clone()).unwrap();
    assert_eq!(retried.receipt.base_id, result.receipt.base_id);
    assert_eq!(
        f.core
            .list_history(&f.doc.session_id)
            .unwrap()
            .versions
            .len(),
        4
    );
    let mut altered = request.clone();
    altered.version_id = v2;
    assert_eq!(
        f.core.restore_history(altered).unwrap_err().code,
        "operationConflict"
    );
    let status = f
        .core
        .history_restore_status(&request.operation_id)
        .unwrap();
    assert_eq!(status.status, "committed");
    assert_eq!(status.result.unwrap().source, initial_source);
    drop(f.core);
    let mut core = Core::new(&f.data).unwrap();
    let reopened = core.open(&f.doc.path, Some(f.root.clone())).unwrap();
    let restarted = core.list_history(&reopened.session_id).unwrap();
    assert_eq!(restarted.versions.len(), 4);
    assert!(
        restarted
            .protected_drafts
            .iter()
            .any(|d| d.source == buffer)
    );
    assert!(
        restarted
            .protected_drafts
            .iter()
            .any(|d| d.source == persisted)
    );
    // The immutable original request can reconcile after restart even though
    // its old session ID no longer exists.
    assert_eq!(
        core.restore_history(request).unwrap().receipt.base_id,
        result.receipt.base_id
    );
}

#[test]
fn history_restore_declines_external_edit_after_comparison_and_preserves_draft() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let (_, current) = f.save("v2");
    let draft = format!("{current}\nmy draft");
    f.core.persist_draft(&f.doc.session_id, &draft).unwrap();
    f.core.compare_history(&f.doc.session_id, &initial).unwrap();
    let request = f.restore_request(&initial, "retain", Some(&draft));
    let external = format!("{current}\nexternal edit");
    fs::write(&f.doc.path, &external).unwrap();
    assert_eq!(
        f.core.restore_history(request.clone()).unwrap_err().code,
        "staleBase"
    );
    assert_eq!(fs::read(&f.doc.path).unwrap(), external.as_bytes());
    assert_eq!(
        f.core
            .read_draft(Path::new(&f.doc.path))
            .unwrap()
            .unwrap()
            .source,
        draft
    );
    assert_eq!(
        f.core
            .history_restore_status(&request.operation_id)
            .unwrap()
            .status,
        "missing"
    );
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    assert_eq!(timeline.currentness, "external");
    assert_eq!(timeline.versions.len(), 2);
    assert_eq!(
        f.core
            .compare_history(&f.doc.session_id, &initial)
            .unwrap_err()
            .code,
        "currentSavedUnavailable"
    );
}

#[test]
fn history_missing_or_changed_exact_records_are_unavailable_without_latest_fallback() {
    let mut f = HistoryFixture::new();
    let (old, _) = f.save("v2");
    let (latest, _) = f.save("v3");
    let witness = f.core.journal_path(&old, "observations");
    let parked = witness.with_extension("owned-test-parked");
    fs::rename(&witness, &parked).unwrap();
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    assert_eq!(
        timeline
            .versions
            .iter()
            .find(|v| v.version_id == old)
            .unwrap()
            .status,
        "missing"
    );
    assert_eq!(
        timeline.current_saved_version_id.as_deref(),
        Some(latest.as_str())
    );
    assert_eq!(
        f.core
            .read_history(&f.doc.session_id, &old)
            .unwrap_err()
            .code,
        "historyUnavailable"
    );
    fs::rename(parked, witness).unwrap();
    let path = f.core.journal_path(&old, "plan");
    let original = fs::read(&path).unwrap();
    let mut plan: serde_json::Value = serde_json::from_slice(&original).unwrap();
    plan["after"] = serde_json::json!([1, 2, 3]);
    fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
    assert_eq!(
        f.core
            .list_history(&f.doc.session_id)
            .unwrap()
            .versions
            .iter()
            .find(|v| v.version_id == old)
            .unwrap()
            .status,
        "invalid"
    );
    assert!(f.core.read_history(&f.doc.session_id, &old).is_err());
    fs::write(path, original).unwrap();
    // Historical raw source remains available when the semantic configuration
    // is unavailable; the source is not compiled using current defaults.
    fs::write(f.root.join(".weftext-schema.json"), b"invalid schema").unwrap();
    assert_eq!(
        f.core
            .read_history(&f.doc.session_id, &old)
            .unwrap()
            .preview_status,
        "sourceOnly"
    );
}

#[test]
fn history_managed_moves_restart_and_copies_keep_independent_identity() {
    let mut f = HistoryFixture::new();
    let (_, source) = f.save("v2");
    f.operate(OperationIntent::RenameNode {
        source: display(&f.root.join("Note")),
        name: "Renamed".into(),
    });
    fs::create_dir(f.root.join("Container")).unwrap();
    f.operate(OperationIntent::MoveNode {
        source: display(&f.root.join("Renamed")),
        parent: display(&f.root.join("Container")),
    });
    let moved_path = f.root.join("Container").join("Renamed").join("Renamed.typ");
    let doc = f.core.open(&moved_path, Some(f.root.clone())).unwrap();
    assert_eq!(
        f.core.list_history(&doc.session_id).unwrap().versions.len(),
        2
    );
    f.operate(OperationIntent::CopyNode {
        source: display(moved_path.parent().unwrap()),
        parent: display(&f.root),
        name: "Copy".into(),
        allow_losses: false,
    });
    let copied = f
        .core
        .open(f.root.join("Copy/Copy.typ"), Some(f.root.clone()))
        .unwrap();
    assert_ne!(copied.node_id, doc.node_id);
    assert_eq!(
        f.core
            .list_history(&copied.session_id)
            .unwrap()
            .versions
            .len(),
        1
    );
    assert_eq!(
        f.core.list_history(&doc.session_id).unwrap().versions.len(),
        2
    );
    drop(f.core);
    let mut core = Core::new(&f.data).unwrap();
    let restarted = core.open(&moved_path, Some(f.root.clone())).unwrap();
    assert_eq!(restarted.source, source);
    assert_eq!(
        core.list_history(&restarted.session_id)
            .unwrap()
            .versions
            .len(),
        2
    );
}

#[test]
fn history_external_rename_replacement_and_current_exclusion_do_not_join_by_name() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let old = f.root.join("Note");
    let new = f.root.join("External");
    fs::rename(&old, &new).unwrap();
    fs::rename(new.join("Note.typ"), new.join("External.typ")).unwrap();
    let doc = f
        .core
        .open(new.join("External.typ"), Some(f.root.clone()))
        .unwrap();
    assert_eq!(
        f.core.list_history(&doc.session_id).unwrap_err().code,
        "identityPending"
    );
    fs::rename(new.join("External.typ"), new.join("Note.typ")).unwrap();
    fs::rename(new, &old).unwrap();
    fs::write(
        f.root.join(".weftext-rules"),
        br#"{"version":1,"ignored":["Note"],"unmanaged":[]}"#,
    )
    .unwrap();
    assert_eq!(
        f.core
            .read_history(&f.doc.session_id, &initial)
            .unwrap_err()
            .code,
        "excludedBoundary"
    );
    fs::write(
        f.root.join(".weftext-rules"),
        br#"{"version":1,"ignored":[],"unmanaged":[]}"#,
    )
    .unwrap();
    let old_path = PathBuf::from(&f.doc.path);
    let parked = old_path.with_extension("owned-test-parked");
    fs::rename(&old_path, &parked).unwrap();
    fs::write(&old_path, &f.doc.source).unwrap();
    assert_eq!(
        f.core
            .read_history(&f.doc.session_id, &initial)
            .unwrap_err()
            .code,
        "historyUnavailable"
    );
    let replacement = f.core.open(&old_path, Some(f.root.clone())).unwrap();
    assert_eq!(replacement.kind, "ordinary");
    assert_eq!(
        f.core
            .list_history(&replacement.session_id)
            .unwrap_err()
            .code,
        "historyUnavailable"
    );
}

#[test]
fn history_draft_read_failure_stops_restore_before_any_save() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let (_, current) = f.save("v2");
    let draft = format!("{current}\nunsaved");
    f.core.persist_draft(&f.doc.session_id, &draft).unwrap();
    let path = f.core.draft_path(Path::new(&f.doc.path));
    let locked = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .unwrap();
    let request = f.restore_request(&initial, "retain", Some(&draft));
    assert!(f.core.restore_history(request.clone()).is_err());
    assert_eq!(fs::read(&f.doc.path).unwrap(), current.as_bytes());
    assert_eq!(
        f.core
            .history_restore_status(&request.operation_id)
            .unwrap()
            .status,
        "missing"
    );
    drop(locked);
    assert_eq!(
        f.core
            .read_draft(Path::new(&f.doc.path))
            .unwrap()
            .unwrap()
            .source,
        draft
    );
}

#[test]
fn history_unresolved_restore_is_not_listed_or_blindly_executed_again() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    f.save("v2");
    let original_request = f.restore_request(&initial, "none", None);
    f.core.restore_history(original_request).unwrap();
    let request = f.restore_request(&initial, "none", None);
    let mut pending = f
        .core
        .operations
        .values()
        .find(|p| p.history_restore.is_some())
        .unwrap()
        .clone();
    pending.operation_id = request.operation_id.clone();
    pending.fingerprint = fresh();
    pending.history_restore.as_mut().unwrap().fingerprint =
        hash(&serde_json::to_vec(&request).unwrap());
    f.core.stage(pending).unwrap();
    let original = fs::read(&f.doc.path).unwrap();
    assert_eq!(
        f.core
            .history_restore_status(&request.operation_id)
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert_eq!(
        f.core.restore_history(request).unwrap_err().code,
        "recoveryPending"
    );
    assert_eq!(
        f.core.list_history(&f.doc.session_id).unwrap_err().code,
        "recoveryPending"
    );
    assert_eq!(fs::read(&f.doc.path).unwrap(), original);
}

#[test]
fn history_reused_note_name_does_not_inherit_prebirth_moves() {
    let mut f = HistoryFixture::new();
    let original_id = f.doc.node_id.clone();
    f.operate(OperationIntent::RenameNode {
        source: display(&f.root.join("Note")),
        name: "Previous".into(),
    });
    let fresh_note = f.core.create_note(&f.root, &f.root, "Note").unwrap();
    let fresh_history = f.core.list_history(&fresh_note.session_id).unwrap();
    assert_ne!(fresh_note.node_id, original_id);
    assert_eq!(fresh_history.versions.len(), 1);
    assert_eq!(fresh_history.path, fresh_note.path);
    let previous = f
        .core
        .open(
            f.root.join("Previous").join("Previous.typ"),
            Some(f.root.clone()),
        )
        .unwrap();
    assert_eq!(previous.node_id, original_id);
    assert_eq!(
        f.core
            .list_history(&previous.session_id)
            .unwrap()
            .versions
            .len(),
        1
    );
}

#[test]
fn history_includes_external_preimage_only_after_explicit_admission_commits() {
    let mut f = HistoryFixture::new();
    let external = format!("{}\nexternal exact preimage", f.doc.source);
    fs::write(&f.doc.path, &external).unwrap();
    let snapshot = f
        .core
        .read_snapshot(&f.root, &[f.doc.path.clone()])
        .unwrap()
        .files
        .remove(0);
    let operation_id = fresh();
    let reserved = f
        .core
        .reserve_source_revision(&f.root, &f.doc.path, &snapshot.base_id, &operation_id)
        .unwrap();
    assert_eq!(
        f.core
            .list_history(&f.doc.session_id)
            .unwrap()
            .versions
            .len(),
        1
    );
    let after = format!("{external}\ncommitted update");
    let prepared = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: operation_id.clone(),
            root: display(&f.root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: f.doc.path.clone(),
                    base_id: Some(snapshot.base_id),
                    before: Some(external.as_bytes().to_vec()),
                    after: after.as_bytes().to_vec(),
                }],
            },
        })
        .unwrap();
    f.core
        .execute_operation(&operation_id, &prepared.plan_id)
        .unwrap();
    let doc = f.core.open(&f.doc.path, Some(f.root.clone())).unwrap();
    let history = f.core.list_history(&doc.session_id).unwrap();
    assert_eq!(history.versions.len(), 3);
    let admitted = history
        .versions
        .iter()
        .find(|v| v.kind == "admittedSource")
        .unwrap();
    assert_eq!(admitted.version_id, reserved);
    assert_eq!(admitted.operation_id, operation_id);
    assert_eq!(
        f.core
            .read_history(&doc.session_id, &admitted.version_id)
            .unwrap()
            .source,
        external
    );
    assert_eq!(
        f.core
            .compare_history(&doc.session_id, &reserved)
            .unwrap()
            .current
            .source,
        after
    );
}

#[test]
fn history_raced_external_write_never_rebases_the_protected_buffer() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let (_, saved) = f.save("v2");
    let buffer = format!("{saved}\nnew unsaved buffer");
    let request = f.restore_request(&initial, "retain", Some(&buffer));
    // Deterministic split at the real preflight boundary: the filesystem write
    // occurs after Base validation and before proposal capture/guarded save.
    f.core
        .check_session_base(&request.session_id, &request.base_id)
        .unwrap();
    let external = format!("{saved}\nexternal writer");
    fs::write(&f.doc.path, &external).unwrap();
    let fingerprint = hash(&serde_json::to_vec(&request).unwrap());
    assert_eq!(
        f.core
            .restore_history_after_preflight(request.clone(), fingerprint)
            .unwrap_err()
            .code,
        "conflict"
    );
    assert_eq!(fs::read(&f.doc.path).unwrap(), external.as_bytes());
    assert_eq!(
        f.core
            .history_restore_status(&request.operation_id)
            .unwrap()
            .status,
        "missing"
    );
    let protected = f
        .core
        .list_history(&f.doc.session_id)
        .unwrap()
        .protected_drafts;
    let buffer_draft = protected.iter().find(|d| d.source == buffer).unwrap();
    assert_eq!(buffer_draft.base_source, saved);
    assert!(buffer_draft.base_known);
    assert_ne!(buffer_draft.base_source, external);
}
