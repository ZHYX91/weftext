use super::*;
use std::os::windows::fs::OpenOptionsExt;

#[test]
fn protected_history_draft_routing_validates_original_tuple_before_ignoring_foreign_body() {
    let mut f = HistoryFixture::new();
    let foreign = f._temp.path().join("foreign");
    f.core.create_library(&foreign).unwrap();
    let foreign_doc = f
        .core
        .create_note_fixture(&foreign, &foreign, "Foreign")
        .unwrap();
    let mut scope = f
        .core
        .capture_file_save_scope(&foreign.join("prospective.typ"))
        .unwrap();
    let path = f
        .core
        .draft_directory("draft-history")
        .join(format!("{}.json", fresh()));
    let foreign_identity = f.core.sessions[&foreign_doc.session_id]
        .stamp
        .identity
        .clone();
    let encoded = |scope: &QualifiedInputScope| {
        let mut bytes = b"{\"scope\":".to_vec();
        bytes.extend(serde_json::to_vec(scope).unwrap());
        bytes.extend_from_slice(b",\"identity\":");
        bytes.extend(serde_json::to_vec(&foreign_identity).unwrap());
        bytes.extend_from_slice(b",\"source\":BROKEN_FOREIGN_BODY");
        bytes
    };
    let valid_foreign = encoded(&scope);
    fs::write(&path, &valid_foreign).unwrap();
    assert!(f.core.list_history(&f.doc.session_id).is_ok());
    let QualifiedInputScope::Library { binding } = &mut scope else {
        panic!("library scope")
    };
    binding.root = display(&f._temp.path().join("contradictory-locator"));
    let damaged = encoded(&scope);
    fs::write(&path, &damaged).unwrap();
    assert_eq!(
        f.core.list_history(&f.doc.session_id).unwrap_err().code,
        "privateEvidence"
    );
    assert_eq!(fs::read(&path).unwrap(), damaged);
    fs::write(&path, &valid_foreign).unwrap();
    assert!(f.core.list_history(&f.doc.session_id).is_ok());
    assert_eq!(fs::read(path).unwrap(), valid_foreign);
}

#[test]
fn protected_history_skips_other_physical_owner_before_its_private_body() {
    let mut f = HistoryFixture::new();
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    let scope = f.core.draft_session_scope(&other.session_id).unwrap();
    let identity = f.core.sessions[&other.session_id].stamp.identity.clone();
    let mut bytes = b"{\"scope\":".to_vec();
    bytes.extend(serde_json::to_vec(&scope).unwrap());
    bytes.extend_from_slice(b",\"identity\":");
    bytes.extend(serde_json::to_vec(&identity).unwrap());
    bytes.extend_from_slice(b",\"source\":BROKEN_OTHER_PRIVATE_BODY");
    let path = f
        .core
        .draft_directory("draft-history")
        .join(format!("{}.json", fresh()));
    let mut paths = vec![path];
    for _ in 0..128 {
        paths.push(
            f.core
                .draft_directory("draft-history")
                .join(format!("{}.json", fresh())),
        );
    }
    for path in &paths {
        fs::write(path, &bytes).unwrap();
    }
    assert!(f.core.list_history(&f.doc.session_id).is_ok());
    for path in paths {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    let mut reader = std::io::Cursor::new(&bytes);
    assert_eq!(
        draft_recovery::read_draft_metadata(&mut reader)
            .unwrap()
            .identity,
        identity
    );
}

#[test]
fn retained_restore_rejects_foreign_scope_or_file_before_private_body_and_recovers_original_result()
{
    let mut f = HistoryFixture::new();
    let version = f.initial_version();
    f.save("saved S");
    let request = f.restore_request(&version, "retain", Some("private pending Y"));
    let original_result = f.core.restore_history(request.clone()).unwrap();
    let original_plan = f.core.operations[&request.operation_id].clone();
    let archives = &original_plan
        .history_restore
        .as_ref()
        .unwrap()
        .draft_records;
    assert!(!archives.is_empty());
    let path = f
        .core
        .draft_directory("draft-history")
        .join(format!("{}.json", archives[0]));
    let original = fs::read(&path).unwrap();
    let plan_bytes = fs::read(f.core.journal_path(&request.operation_id, "plan")).unwrap();
    let commit_bytes = fs::read(f.core.journal_path(&request.operation_id, "commit")).unwrap();
    let source_bytes = fs::read(&f.doc.path).unwrap();
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    let foreign = f._temp.path().join("foreign");
    f.core.create_library(&foreign).unwrap();
    let foreign_doc = f
        .core
        .create_note_fixture(&foreign, &foreign, "Foreign")
        .unwrap();
    let count = f.core.operations.len();
    for doc in [&other, &foreign_doc] {
        let scope = f.core.draft_session_scope(&doc.session_id).unwrap();
        let identity = f.core.sessions[&doc.session_id].stamp.identity.clone();
        let mut damaged = b"{\"scope\":".to_vec();
        damaged.extend(serde_json::to_vec(&scope).unwrap());
        damaged.extend_from_slice(b",\"identity\":");
        damaged.extend(serde_json::to_vec(&identity).unwrap());
        damaged.extend_from_slice(b",\"source\":BROKEN_FOREIGN_PRIVATE_BODY");
        fs::write(&path, &damaged).unwrap();
        let status = f
            .core
            .history_restore_status(&request.operation_id)
            .unwrap();
        assert_eq!(status.status, "committed");
        assert_eq!(
            status.receipt.unwrap().base_id,
            original_result.receipt.base_id
        );
        assert!(status.result.is_none());
        assert_eq!(status.result_error.unwrap().code, "privateEvidence");
        assert_eq!(fs::read(&path).unwrap(), damaged);
        assert_eq!(
            fs::read(f.core.journal_path(&request.operation_id, "plan")).unwrap(),
            plan_bytes
        );
        assert_eq!(
            fs::read(f.core.journal_path(&request.operation_id, "commit")).unwrap(),
            commit_bytes
        );
        assert_eq!(fs::read(&f.doc.path).unwrap(), source_bytes);
        assert_eq!(f.core.operations.len(), count);
    }
    fs::write(&path, &original).unwrap();
    let restored = f
        .core
        .history_restore_status(&request.operation_id)
        .unwrap();
    assert_eq!(restored.status, "committed");
    assert_eq!(
        restored.result.unwrap().protected_drafts[0].source,
        original_result.protected_drafts[0].source
    );
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn protected_history_draft_budget_counts_original_records_even_when_alternatives_deduplicate() {
    let mut f = HistoryFixture::new();
    f.core
        .persist_draft(&f.doc.session_id, "one original private alternative")
        .unwrap();
    let primary = f.core.draft_path(Path::new(&f.doc.path));
    let original = fs::read(&primary).unwrap();
    for index in 0..129 {
        fs::write(
            f.core
                .draft_directory("draft-history")
                .join(format!("{index}.json")),
            &original,
        )
        .unwrap();
    }
    assert_eq!(
        f.core.list_history(&f.doc.session_id).unwrap_err().code,
        "budget"
    );
    assert_eq!(fs::read(primary).unwrap(), original);
}

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
        let doc = core.create_note_fixture(&root, &root, "Note").unwrap();
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
        self.doc.base_id = receipt.receipt.base_id;
        (operation_id, source)
    }
    fn initial_version(&self) -> String {
        self.core
            .list_history(&self.doc.session_id)
            .unwrap()
            .versions
            .iter()
            .find(|v| v.kind == "newNode")
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
fn history_descriptions_are_typed_without_rewriting_source_or_retained_plan() {
    let mut f = HistoryFixture::new();
    let (version, source) = f.save("作者原文 English 中文 {literal}");
    let plan_path = f.core.journal_path(&version, "plan");
    let original_plan = fs::read(&plan_path).unwrap();
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    assert_eq!(
        timeline.retention.description.explanation.key(),
        "history.retentionLimited"
    );
    let wire = serde_json::to_value(&timeline).unwrap();
    assert!(wire["retention"].get("message").is_none());
    assert!(wire["retention"]["description"]["diagnostics"].is_array());
    assert!(wire["versions"].as_array().unwrap().iter().all(|record| {
        record.get("message").is_none() && record["description"]["explanation"].is_object()
    }));
    let read = f.core.read_history(&f.doc.session_id, &version).unwrap();
    assert_eq!(read.source, source);
    assert_eq!(
        read.preview_description.explanation.key(),
        "history.sourceOnlyPreview"
    );
    let read_wire = serde_json::to_value(&read).unwrap();
    assert!(read_wire.get("previewMessage").is_none());
    assert!(read_wire["previewDescription"]["explanation"].is_object());
    assert_ne!(
        read.preview_description
            .explanation
            .render(weftext_messages::UiLocale::ZhCn),
        read.preview_description
            .explanation
            .render(weftext_messages::UiLocale::EnUs)
    );
    assert_eq!(fs::read(&plan_path).unwrap(), original_plan);
    assert_eq!(fs::read_to_string(&f.doc.path).unwrap(), source);
}

#[test]
fn unreadable_retained_history_keeps_the_original_diagnostic_in_its_description() {
    let mut f = HistoryFixture::new();
    let (version, _) = f.save("retained source");
    let plan = f.core.journal_path(&version, "plan");
    let preserved = f.data.join("original-plan-preserved.json");
    let original = fs::read(&plan).unwrap();
    fs::rename(&plan, &preserved).unwrap();
    let timeline = f.core.list_history(&f.doc.session_id).unwrap();
    let entry = timeline
        .versions
        .iter()
        .find(|entry| entry.version_id == version)
        .unwrap();
    assert_ne!(entry.status, "available");
    assert!(!entry.description.diagnostics.is_empty());
    assert!(
        entry
            .description
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.origin == DiagnosticOrigin::Os)
    );
    assert!(entry.description.details.is_some());
    let error = f
        .core
        .read_history(&f.doc.session_id, &version)
        .unwrap_err();
    assert_eq!(error.code, "historyUnavailable");
    assert_eq!(error.diagnostics, entry.description.diagnostics);
    assert_eq!(fs::read(&preserved).unwrap(), original);
    assert!(!plan.exists());
}

#[test]
fn retained_trash_root_reads_exact_history_and_resources_without_content_write_rights() {
    let mut f = HistoryFixture::new();
    let initial = f.initial_version();
    let (_, saved) = f.save("retained original source 中文");
    let imported = f
        .core
        .prepare_import_resource(
            &f.root,
            &f.doc.path,
            "owned.bin",
            b"owned original".to_vec(),
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&imported.operation_id, &imported.plan_id)
        .unwrap();
    let before_resource = f
        .core
        .managed_resource_snapshot(&f.root, &f.doc.path)
        .unwrap()
        .remove(0);
    let before_identity = operations::capture_optional(Path::new(&before_resource.path))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    let draft = format!("{saved}\nprivate unsaved draft");
    f.core.persist_draft(&f.doc.session_id, &draft).unwrap();
    let history = f
        .core
        .list_history(&f.doc.session_id)
        .unwrap()
        .versions
        .into_iter()
        .filter(|version| version.status == "available")
        .map(|version| {
            let source = f
                .core
                .read_history(&f.doc.session_id, &version.version_id)
                .unwrap()
                .source;
            (version.version_id, source)
        })
        .collect::<Vec<_>>();
    let receipt = f.operate(OperationIntent::TrashNode {
        source: display(Path::new(&f.doc.path).parent().unwrap()),
    });
    let retained = Path::new(&receipt.path).join("Note.typ");
    f.doc = f.core.open(&retained, Some(f.root.clone())).unwrap();
    assert_eq!(f.doc.recovered_draft.as_deref(), Some(draft.as_str()));
    for (version, source) in history {
        assert_eq!(
            f.core
                .read_history(&f.doc.session_id, &version)
                .unwrap()
                .source,
            source
        );
    }
    let resource = f
        .core
        .managed_resource_snapshot(&f.root, &retained)
        .unwrap()
        .remove(0);
    assert_eq!(resource.resource_ref, before_resource.resource_ref);
    assert_eq!(resource.bytes, before_resource.bytes);
    assert_eq!(resource.revision, before_resource.revision);
    assert_eq!(
        operations::capture_optional(Path::new(&resource.path))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        before_identity
    );
    let operations_before = f.core.operations.len();
    let request = f.restore_request(&initial, "retain", Some(&draft));
    assert_eq!(
        f.core.restore_history(request).unwrap_err().code,
        "readOnlyLifecycle"
    );
    assert!(
        f.core
            .prepare_import_resource(
                &f.root,
                &retained,
                "new.bin",
                b"no write".to_vec(),
                &fresh()
            )
            .is_err()
    );
    assert_eq!(f.core.operations.len(), operations_before);
    assert_eq!(fs::read_to_string(&retained).unwrap(), saved);
    let original = fs::read(&retained).unwrap();
    fs::rename(&retained, f._temp.path().join("retained-original.typ")).unwrap();
    fs::write(&retained, &original).unwrap();
    assert!(f.core.list_history(&f.doc.session_id).is_err());
    assert!(
        f.core
            .managed_resource_snapshot(&f.root, &retained)
            .is_err()
    );
    assert_eq!(fs::read(&retained).unwrap(), original);
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
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
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
    let initial_source = f.doc.source.clone();
    f.save("saved before external rename");
    let proposal = format!("{}\noriginal private proposal", f.doc.source);
    f.core.persist_draft(&f.doc.session_id, &proposal).unwrap();
    let draft_path = f.core.draft_path(Path::new(&f.doc.path));
    let draft_bytes = fs::read(&draft_path).unwrap();
    let old = f.root.join("Note");
    let new = f.root.join("External");
    fs::rename(&old, &new).unwrap();
    fs::rename(new.join("Note.typ"), new.join("External.typ")).unwrap();
    let doc = f
        .core
        .open(new.join("External.typ"), Some(f.root.clone()))
        .unwrap();
    assert!(doc.recovered_draft.is_none());
    assert!(doc.recovered_draft_variants.is_empty());
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(
        f.core.list_history(&doc.session_id).unwrap_err().code,
        "identityPending"
    );
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    let doc = f
        .core
        .open(new.join("External.typ"), Some(f.root.clone()))
        .unwrap();
    assert!(doc.recovered_draft.is_none());
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(
        f.core.list_history(&doc.session_id).unwrap_err().code,
        "identityPending"
    );
    let private_snapshot = |directory: &Path| {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let journal_before = private_snapshot(&f.data.join("journal"));
    let drafts_before = private_snapshot(&f.data.join("drafts"));
    let author_before = fs::read(new.join("External.typ")).unwrap();
    assert_eq!(
        f.core
            .persist_draft(&doc.session_id, "new input at the external position")
            .unwrap_err()
            .code,
        "identityPending"
    );
    let stale_operation = fresh();
    assert_eq!(
        f.core
            .save(SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: fresh(),
                source: "stale Save input at the external position".into(),
                operation_id: stale_operation.clone(),
            })
            .unwrap_err()
            .code,
        "identityPending"
    );
    assert!(!f.core.operations.contains_key(&stale_operation));
    assert_eq!(private_snapshot(&f.data.join("journal")), journal_before);
    assert_eq!(private_snapshot(&f.data.join("drafts")), drafts_before);
    assert_eq!(fs::read(new.join("External.typ")).unwrap(), author_before);
    fs::rename(new.join("External.typ"), new.join("Note.typ")).unwrap();
    fs::rename(new, &old).unwrap();
    f.doc = f.core.open(&f.doc.path, Some(f.root.clone())).unwrap();
    assert_eq!(f.doc.recovered_draft.as_deref(), Some(proposal.as_str()));
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(
        f.core
            .read_history(&f.doc.session_id, &initial)
            .unwrap()
            .source,
        initial_source
    );
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
    assert!(replacement.recovered_draft.is_none());
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(
        f.core
            .list_history(&replacement.session_id)
            .unwrap_err()
            .code,
        "historyUnavailable"
    );
}

#[test]
fn history_external_rollback_to_former_committed_path_does_not_replace_hidden_draft() {
    let mut f = HistoryFixture::new();
    f.save("saved before original application rename");
    let proposal = format!("{}\noriginal private proposal", f.doc.source);
    f.core.persist_draft(&f.doc.session_id, &proposal).unwrap();
    f.operate(OperationIntent::RenameNode {
        source: display(&f.root.join("Note")),
        name: "Renamed".into(),
    });
    let committed_path = f.root.join("Renamed/Renamed.typ");
    let moved = f.core.open(&committed_path, Some(f.root.clone())).unwrap();
    assert_eq!(moved.recovered_draft.as_deref(), Some(proposal.as_str()));
    let draft_path = f.core.draft_path(&committed_path);
    let draft_bytes = fs::read(&draft_path).unwrap();
    fs::rename(f.root.join("Renamed"), f.root.join("Note")).unwrap();
    fs::rename(f.root.join("Note/Renamed.typ"), &f.doc.path).unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    let external = f.core.open(&f.doc.path, Some(f.root.clone())).unwrap();
    assert!(external.recovered_draft.is_none());
    assert_eq!(
        f.core.list_history(&external.session_id).unwrap_err().code,
        "identityPending"
    );
    assert_eq!(
        f.core
            .persist_draft(&external.session_id, "input at a former historical slot")
            .unwrap_err()
            .code,
        "identityPending"
    );
    let operation_id = fresh();
    assert_eq!(
        f.core
            .save(SaveRequest {
                session_id: external.session_id,
                base_id: fresh(),
                source: "stale Save at a former historical slot".into(),
                operation_id: operation_id.clone(),
            })
            .unwrap_err()
            .code,
        "identityPending"
    );
    assert!(!f.core.operations.contains_key(&operation_id));
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(fs::read(&f.doc.path).unwrap(), f.doc.source.as_bytes());
    fs::rename(f.root.join("Note"), f.root.join("Renamed")).unwrap();
    fs::rename(f.root.join("Renamed/Note.typ"), &committed_path).unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    let restored = f.core.open(&committed_path, Some(f.root.clone())).unwrap();
    assert_eq!(restored.recovered_draft.as_deref(), Some(proposal.as_str()));
    assert_eq!(fs::read(&draft_path).unwrap(), draft_bytes);
    assert_eq!(
        f.core
            .list_history(&restored.session_id)
            .unwrap()
            .versions
            .len(),
        2
    );
}

#[test]
fn history_external_rename_keeps_original_actor_input_scope_across_authority_context() {
    let mut f = HistoryFixture::new();
    let actor = "deployment:external-draft";
    let proposal = format!("{}\noriginal unbound private proposal", f.doc.source);
    let (draft_path, draft_bytes) = f
        .core
        .with_actor(actor, |core| {
            let opened = core.open(&f.doc.path, Some(f.root.clone()))?;
            core.persist_draft(&opened.session_id, &proposal)?;
            let path = core.draft_path(Path::new(&f.doc.path));
            let bytes = fs::read(&path)?;
            let draft: Draft = serde_json::from_slice(&bytes)?;
            assert!(matches!(draft.scope, QualifiedInputScope::Library { .. }));
            Ok((path, bytes))
        })
        .unwrap();
    let original_directory = f.root.join("Note");
    let external_directory = f.root.join("External");
    fs::rename(&original_directory, &external_directory).unwrap();
    let external_source = external_directory.join("External.typ");
    fs::rename(external_directory.join("Note.typ"), &external_source).unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    f.core
        .with_actor_for_root(actor, &f.root, |core| {
            let opened = core.open(&external_source, Some(f.root.clone()))?;
            assert!(opened.recovered_draft.is_none());
            assert_eq!(
                core.list_history(&opened.session_id).unwrap_err().code,
                "identityPending"
            );
            assert_eq!(
                core.persist_draft(&opened.session_id, "new bound-context input")
                    .unwrap_err()
                    .code,
                "identityPending"
            );
            let operation_id = fresh();
            assert_eq!(
                core.save(SaveRequest {
                    session_id: opened.session_id,
                    base_id: fresh(),
                    source: "stale bound-context input".into(),
                    operation_id: operation_id.clone(),
                })
                .unwrap_err()
                .code,
                "identityPending"
            );
            assert!(!core.operations.contains_key(&operation_id));
            assert_eq!(fs::read(&draft_path)?, draft_bytes);
            assert_eq!(fs::read(&external_source)?, f.doc.source.as_bytes());
            let paths = fs::read_dir(core.draft_directory("drafts"))?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<std::io::Result<Vec<_>>>()?;
            assert_eq!(paths, vec![draft_path.clone()]);
            Ok(())
        })
        .unwrap();
    fs::rename(&external_source, external_directory.join("Note.typ")).unwrap();
    fs::rename(&external_directory, &original_directory).unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    f.core
        .with_actor_for_root(actor, &f.root, |core| {
            let opened = core.open(&f.doc.path, Some(f.root.clone()))?;
            assert_eq!(opened.recovered_draft.as_deref(), Some(proposal.as_str()));
            assert_eq!(fs::read(&draft_path)?, draft_bytes);
            let draft: Draft = serde_json::from_slice(&fs::read(&draft_path)?)?;
            assert!(
                matches!(draft.scope, QualifiedInputScope::Library { .. }),
                "authority changes do not retag original input"
            );
            assert_eq!(core.list_history(&opened.session_id)?.versions.len(), 1);
            Ok(())
        })
        .unwrap();
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
    let request = f.restore_request(&initial, "none", None);
    let original = fs::read(&f.doc.path).unwrap();
    crate::hosted::set_test_failpoint("saveStaged");
    assert_eq!(
        f.core.restore_history(request.clone()).unwrap_err().code,
        "injectedFailure"
    );
    let pending = &f.core.operations[&request.operation_id];
    f.core.validate_file_save_plan(pending).unwrap();
    let plan_bytes = fs::read(f.core.journal_path(&request.operation_id, "plan")).unwrap();
    assert!(!f.core.original_commit_fact(&request.operation_id).unwrap());
    let operation_count = f.core.operations.len();
    assert_eq!(
        f.core
            .history_restore_status(&request.operation_id)
            .unwrap()
            .status,
        "recoveryPending"
    );
    assert_eq!(
        f.core.restore_history(request.clone()).unwrap_err().code,
        "recoveryPending"
    );
    assert_eq!(
        f.core.list_history(&f.doc.session_id).unwrap_err().code,
        "recoveryPending"
    );
    assert_eq!(fs::read(&f.doc.path).unwrap(), original);
    assert_eq!(f.core.operations.len(), operation_count);
    assert_eq!(
        fs::read(f.core.journal_path(&request.operation_id, "plan")).unwrap(),
        plan_bytes
    );
}

#[test]
fn history_reused_note_name_does_not_inherit_prebirth_moves() {
    let mut f = HistoryFixture::new();
    let original_id = f.doc.node_id.clone();
    f.operate(OperationIntent::RenameNode {
        source: display(&f.root.join("Note")),
        name: "Previous".into(),
    });
    let fresh_note = f
        .core
        .create_note_fixture(&f.root, &f.root, "Note")
        .unwrap();
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
