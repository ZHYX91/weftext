use super::*;

const ALICE: &str = "deployment:alice:history";
const BOB: &str = "deployment:bob:history";
const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_FP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    core: Core,
    temp: tempfile::TempDir,
    data: PathBuf,
    root: PathBuf,
    path: PathBuf,
    original_source: String,
    version: String,
    doc: OpenedDocument,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let created = core.create_note_fixture(&root, &root, "Note").unwrap();
        let path = PathBuf::from(created.path);
        let mut doc = core
            .with_actor(ALICE, |c| c.open(&path, Some(root.clone())))
            .unwrap();
        let original_source = doc.source.clone();
        let version = core
            .with_actor(ALICE, |c| c.list_history(&doc.session_id))
            .unwrap()
            .current_saved_version_id
            .unwrap();
        let newer = original_source.clone() + "\n较新的已保存正文 😀\n";
        let status = core
            .with_actor(ALICE, |c| {
                c.actor_save(
                    "Note",
                    HostedSaveRequest {
                        session_id: doc.session_id.clone(),
                        base_id: doc.base_id.clone(),
                        source: newer.clone(),
                        operation_id: fresh(),
                    },
                )
            })
            .unwrap();
        doc.source = newer;
        doc.base_id = status.base_id.unwrap();
        Self {
            core,
            temp,
            data,
            root,
            path,
            original_source,
            version,
            doc,
        }
    }
    fn request(&self, operation: &str, draft_source: Option<String>) -> HistoryRestoreRequest {
        HistoryRestoreRequest {
            session_id: self.doc.session_id.clone(),
            base_id: self.doc.base_id.clone(),
            version_id: self.version.clone(),
            operation_id: operation.into(),
            draft_action: "retain".into(),
            draft_source,
        }
    }
    fn restore(&mut self, request: HistoryRestoreRequest) -> Result<ActorHistoryRestoreStatus> {
        self.core.with_actor(ALICE, |c| {
            c.actor_history_restore("Note", &self.root, &self.path, request, FP)
        })
    }
    fn status(&mut self, operation: &str) -> ActorHistoryRestoreStatus {
        self.core
            .with_actor(ALICE, |c| c.actor_history_restore_status("Note", operation))
            .unwrap()
            .unwrap()
    }
    fn restart(&mut self) {
        let placeholder = Core::new(self.temp.path().join(fresh())).unwrap();
        drop(std::mem::replace(&mut self.core, placeholder));
        self.core = Core::new(&self.data).unwrap();
    }
    fn internal(&mut self, operation: &str) -> String {
        self.core
            .with_actor(ALICE, |c| {
                Ok(serde_json::from_slice::<ActorOperation>(
                    &fs::read(c.actor_record(operation)?).unwrap(),
                )
                .unwrap()
                .core_operation_id)
            })
            .unwrap()
    }
}

#[test]
fn hosted_history_restore_new_save_keeps_later_history_and_all_own_drafts_without_other_actor_drafts()
 {
    let mut f = Fixture::new();
    let bob = f
        .core
        .with_actor(BOB, |c| c.open(&f.path, Some(f.root.clone())))
        .unwrap();
    f.core
        .with_actor(BOB, |c| {
            c.persist_draft(&bob.session_id, "Bob private draft")
        })
        .unwrap();
    let persisted = f.doc.source.clone() + "\nAlice持久草稿";
    let buffer = f.doc.source.clone() + "\nAlice更新的缓冲😀";
    f.core
        .with_actor(ALICE, |c| c.persist_draft(&f.doc.session_id, &persisted))
        .unwrap();
    let before = f
        .core
        .with_actor(ALICE, |c| c.list_history(&f.doc.session_id))
        .unwrap();
    let operation = fresh();
    let request = f.request(&operation, Some(buffer.clone()));
    let status = f.restore(request.clone()).unwrap();
    assert_eq!(status.status, "committed");
    assert_eq!(status.result_status, "available");
    assert!(status.result_error.is_none());
    assert!(
        serde_json::to_value(&status)
            .unwrap()
            .get("resultError")
            .is_none()
    );
    let result = status.result.unwrap();
    assert_eq!(result.source, f.original_source);
    assert_eq!(result.receipt.operation_id, operation);
    assert_ne!(f.internal(&operation), operation);
    assert!(
        result
            .protected_drafts
            .iter()
            .any(|d| d.source == persisted)
    );
    assert!(result.protected_drafts.iter().any(|d| d.source == buffer));
    assert!(
        !result
            .protected_drafts
            .iter()
            .any(|d| d.source.contains("Bob"))
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.original_source);
    let after = f
        .core
        .with_actor(ALICE, |c| c.list_history(&f.doc.session_id))
        .unwrap();
    assert!(after.versions.len() > before.versions.len());
    for version in before.versions {
        assert!(
            after
                .versions
                .iter()
                .any(|v| v.version_id == version.version_id)
        )
    }
    f.restart();
    assert_eq!(f.status(&operation).status, "committed");
    assert!(f.status(&operation).result.is_none());
    let result = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_history_restore_result_at("Note", &f.root, &f.path, &operation)
        })
        .unwrap();
    assert_eq!(result.receipt.operation_id, operation);
    assert_eq!(
        f.restore(request).unwrap().result.unwrap().source,
        f.original_source
    );
    assert!(
        f.core
            .with_actor(BOB, |c| c.actor_history_restore_status("Note", &operation))
            .unwrap()
            .is_none()
    );
    let bob = f
        .core
        .with_actor(BOB, |c| c.open(&f.path, Some(f.root.clone())))
        .unwrap();
    assert_eq!(bob.recovered_draft.as_deref(), Some("Bob private draft"));
    assert!(bob.recovered_draft_conflict);
}

#[test]
fn hosted_history_dirty_buffer_none_is_rejected_and_changed_original_request_cannot_retry() {
    let mut f = Fixture::new();
    let operation = fresh();
    let mut request = f.request(&operation, Some("unsaved buffer".into()));
    request.draft_action = "none".into();
    assert_eq!(
        f.restore(request.clone()).unwrap_err().code,
        "draftProtectionRequired"
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.doc.source);
    assert_eq!(f.status(&operation).status, "notCommitted");
    assert_eq!(f.restore(request.clone()).unwrap().status, "notCommitted");
    request.draft_action = "retain".into();
    assert_eq!(
        f.restore(request.clone()).unwrap_err().code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c
                .actor_history_restore_for_request("Note", &request, OTHER_FP))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c
                .actor_history_restore_settle("Note", &operation))
            .unwrap()
            .status,
        "cancelled"
    );
}

#[test]
fn hosted_history_unknown_fence_and_preplan_restart_never_revive_original_request() {
    let mut f = Fixture::new();
    let unknown = fresh();
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore_settle("Note", &unknown))
            .unwrap()
            .status,
        "cancelled"
    );
    let late = f.request(&unknown, None);
    f.restart();
    assert_eq!(f.restore(late).unwrap().status, "cancelled");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.doc.source);
    let current = f
        .core
        .with_actor(ALICE, |c| c.open(&f.path, Some(f.root.clone())))
        .unwrap();
    f.doc = current;
    let operation = fresh();
    let request = f.request(&operation, Some(f.doc.source.clone() + "\nretained"));
    set_test_failpoint("historyActorDurable");
    assert_eq!(
        f.restore(request.clone()).unwrap_err().code,
        "injectedFailure"
    );
    f.restart();
    assert_eq!(f.status(&operation).status, "notCommitted");
    assert_eq!(f.restore(request.clone()).unwrap().status, "notCommitted");
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c
                .actor_history_restore_settle("Note", &operation))
            .unwrap()
            .status,
        "cancelled"
    );
    assert_eq!(f.restore(request).unwrap().status, "cancelled");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.doc.source);
}

#[test]
fn hosted_history_installed_and_committed_lost_response_statuses_are_never_cancelled_or_replayed() {
    for point in ["saveStaged", "sourceWritten", "saveCommitted"] {
        let mut f = Fixture::new();
        let operation = fresh();
        let request = f.request(&operation, Some(f.doc.source.clone() + "\nprivate buffer"));
        set_test_failpoint(point);
        let first = f.restore(request.clone());
        if point == "saveCommitted" {
            assert_eq!(first.unwrap().status, "committed");
        } else {
            assert_eq!(first.unwrap_err().code, "injectedFailure");
        }
        f.restart();
        let expected = if point == "saveCommitted" {
            "committed"
        } else {
            "recoveryPending"
        };
        assert_eq!(f.status(&operation).status, expected);
        assert_eq!(
            f.core
                .with_actor(ALICE, |c| c
                    .actor_history_restore_settle("Note", &operation))
                .unwrap()
                .status,
            expected
        );
        let retry = f.restore(request).unwrap();
        assert_eq!(retry.status, expected);
        assert_eq!(retry.result.is_some(), point == "saveCommitted");
    }
}

#[test]
fn first_hosted_restore_retains_a_postcommit_draft_warning_but_later_lookup_does_not_invent_it() {
    let mut f = Fixture::new();
    let operation = fresh();
    let request = f.request(&operation, Some(f.doc.source.clone() + "\nprivate Y"));
    set_test_failpoint("saveDraftUpdate");
    let first = f.restore(request.clone()).unwrap();
    assert_eq!(first.status, "committed");
    let result = first.result.unwrap();
    assert_eq!(result.receipt.operation_id, operation);
    assert_eq!(result.recovery_issue.unwrap().code, "injectedFailure");
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.original_source);
    let internal = f.internal(&operation);
    let count = f.core.operations.len();
    for field in ["session", "base", "version", "source", "action"] {
        let mut changed = request.clone();
        match field {
            "session" => changed.session_id = fresh(),
            "base" => changed.base_id = fresh(),
            "version" => changed.version_id = fresh(),
            "source" => changed.draft_source = Some("different".into()),
            _ => changed.draft_action = "none".into(),
        }
        assert_eq!(
            f.core
                .with_actor(ALICE, |core| core
                    .actor_history_restore_for_request("Note", &changed, FP))
                .unwrap_err()
                .code,
            "operationConflict"
        );
    }
    f.restart();
    let known = f
        .core
        .with_actor(ALICE, |core| {
            core.actor_history_restore_for_request("Note", &request, FP)
        })
        .unwrap()
        .unwrap();
    assert_eq!(known.status, "committed");
    assert!(known.result.is_none());
    let retry = f.restore(request).unwrap();
    assert!(retry.result.unwrap().recovery_issue.is_none());
    assert_eq!(f.internal(&operation), internal);
    assert_eq!(f.core.operations.len(), count);
}

#[test]
fn hosted_history_category_actor_resource_and_basis_cannot_borrow_another_operation() {
    let mut f = Fixture::new();
    let operation = fresh();
    let request = f.request(&operation, None);
    f.restore(request.clone()).unwrap();
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_save_status("Note", &operation))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_definition_status("Note", &operation))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_feature_status("Note", &operation))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c
                .actor_history_restore_status("Other", &operation))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert!(
        f.core
            .with_actor(BOB, |c| c
                .actor_history_restore_for_request("Note", &request, FP))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        f.core
            .with_actor(BOB, |c| c.actor_history_restore_settle("Note", &operation))
            .unwrap()
            .status,
        "cancelled"
    );
    assert_eq!(f.status(&operation).status, "committed");
    let generic = fresh();
    f.core
        .with_actor(ALICE, |c| c.actor_settle("Note", &generic))
        .unwrap();
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore_settle("Note", &generic))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    let mut changed = request;
    changed.version_id = fresh();
    assert_eq!(f.restore(changed).unwrap_err().code, "operationConflict");
    let mut stale = f.request(&fresh(), None);
    stale.base_id = fresh();
    assert_eq!(f.restore(stale).unwrap_err().code, "staleBase");
}

#[test]
fn hosted_history_explicit_owner_result_and_navigation_do_not_scan_unconfigured_sources() {
    let mut f = Fixture::new();
    let hidden = f
        .core
        .create_note_fixture(&f.root, &f.root, "Unreadable")
        .unwrap();
    let duplicate = f.root.join("UnconfiguredCopy");
    fs::create_dir(&duplicate).unwrap();
    fs::write(duplicate.join("UnconfiguredCopy.typ"), &f.original_source).unwrap();
    let hidden_guard = ParentGuard::acquire(Path::new(&hidden.path)).unwrap();
    let _hidden_file = hidden_guard.open_file(true, false).unwrap();
    let selected = f.core.qualified_node_at(&f.root, &f.path).unwrap();
    assert_eq!(selected.owner.node_id, f.doc.node_id.clone().unwrap());
    assert!(
        f.core
            .qualified_node_at(&f.root, &duplicate.join("UnconfiguredCopy.typ"))
            .is_err()
    );
    let operation = fresh();
    let request = f.request(&operation, Some(f.doc.source.clone() + "\n保留"));
    assert_eq!(f.restore(request).unwrap().status, "committed");
    let result = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_history_restore_result_at("Note", &f.root, &f.path, &operation)
        })
        .unwrap();
    assert_eq!(result.source, f.original_source);
}

#[test]
fn hosted_history_current_qualification_and_result_refuse_replacement_and_remapped_owner() {
    let mut f = Fixture::new();
    let operation = fresh();
    let request = f.request(&operation, None);
    f.restore(request.clone()).unwrap();
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore_result_at(
                "Note",
                &f.root,
                Path::new(&other.path),
                &operation
            ))
            .unwrap_err()
            .code,
        "identityPending"
    );
    assert_eq!(f.status(&operation).status, "committed");
    fs::rename(&f.path, f.temp.path().join("original-owned.typ")).unwrap();
    fs::write(&f.path, &f.original_source).unwrap();
    assert!(f.core.qualified_node_at(&f.root, &f.path).is_err());
    let retry = f.restore(request).unwrap();
    assert_eq!(retry.status, "committed");
    assert_eq!(retry.result_status, "unavailable");
    assert!(retry.result.is_none());
    let error = retry.result_error.as_ref().unwrap();
    assert_eq!(error.code, "identityPending");
    assert!(error.explanation.key().starts_with("error.core."));
    let wire = serde_json::to_value(&retry).unwrap();
    assert_eq!(wire["status"], "committed");
    assert_eq!(wire["resultError"]["code"], "identityPending");
    assert!(wire["resultError"].get("message").is_none());
    assert!(wire["resultError"]["explanation"].is_object());
    assert!(wire["resultError"]["diagnostics"].is_array());
    assert!(wire["resultError"]["causes"].is_array());
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c
                .actor_history_restore_settle("Note", &operation))
            .unwrap()
            .status,
        "committed"
    );
}

#[test]
fn hosted_history_session_qualification_keeps_stable_ref_for_external_body_and_other_actor_save() {
    let mut f = Fixture::new();
    let initial = f
        .core
        .with_actor(ALICE, |c| {
            c.qualified_node_for_session_at(&f.root, &f.path, &f.doc.session_id)
        })
        .unwrap()
        .owner;
    let bob = f
        .core
        .with_actor(BOB, |c| c.open(&f.path, Some(f.root.clone())))
        .unwrap();
    f.core
        .with_actor(BOB, |c| {
            c.actor_save(
                "Note",
                HostedSaveRequest {
                    session_id: bob.session_id,
                    base_id: bob.base_id,
                    source: bob.source + "\nBob合法保存",
                    operation_id: fresh(),
                },
            )
        })
        .unwrap();
    let same = f
        .core
        .with_actor(ALICE, |c| {
            c.qualified_node_for_session_at(&f.root, &f.path, &f.doc.session_id)
        })
        .unwrap();
    assert_eq!(same.owner, initial);
    fs::write(
        &f.path,
        fs::read_to_string(&f.path).unwrap() + "\n合法外部正文",
    )
    .unwrap();
    let external = f
        .core
        .with_actor(ALICE, |c| {
            c.qualified_node_for_session_at(&f.root, &f.path, &f.doc.session_id)
        })
        .unwrap();
    assert_eq!(external.owner, initial);
    assert!(external.revision.is_none());
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.compare_history(&f.doc.session_id, &f.version))
            .unwrap_err()
            .code,
        "currentSavedUnavailable"
    );
    assert!(
        f.core
            .with_actor(BOB, |c| c.qualified_node_for_session_at(
                &f.root,
                &f.path,
                &f.doc.session_id
            ))
            .is_err()
    );
    fs::write(
        &f.path,
        f.original_source
            .replace(f.doc.node_id.as_ref().unwrap(), &fresh()),
    )
    .unwrap();
    assert!(f.core.qualified_node_at(&f.root, &f.path).is_err());
}

#[test]
fn hosted_history_unknown_version_and_wrong_configured_path_have_no_source_effect() {
    let mut f = Fixture::new();
    let mut request = f.request(&fresh(), None);
    request.version_id = fresh();
    assert_eq!(f.restore(request).unwrap_err().code, "historyUnavailable");
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    let request = f.request(&fresh(), None);
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore(
                "Note",
                &f.root,
                Path::new(&other.path),
                request,
                FP
            ))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.doc.source);
}

#[test]
fn hosted_history_same_byte_later_save_and_return_to_old_content_are_superseded_operations() {
    let mut f = Fixture::new();
    let operation = fresh();
    let request = f.request(&operation, None);
    f.restore(request).unwrap();
    assert!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore_current_at(
                "Note", &f.root, &f.path, &operation
            ))
            .unwrap()
    );
    let mut current = f
        .core
        .with_actor(ALICE, |c| c.open(&f.path, Some(f.root.clone())))
        .unwrap();
    for source in [
        f.original_source.clone(),
        f.original_source.clone() + "\nF新内容",
        f.original_source.clone(),
    ] {
        let saved = f
            .core
            .with_actor(ALICE, |c| {
                c.actor_save(
                    "Note",
                    HostedSaveRequest {
                        session_id: current.session_id.clone(),
                        base_id: current.base_id.clone(),
                        source,
                        operation_id: fresh(),
                    },
                )
            })
            .unwrap();
        current.base_id = saved.base_id.unwrap();
        assert!(
            !f.core
                .with_actor(ALICE, |c| c.actor_history_restore_current_at(
                    "Note", &f.root, &f.path, &operation
                ))
                .unwrap()
        );
    }
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.original_source);
    f.restart();
    assert!(
        !f.core
            .with_actor(ALICE, |c| c.actor_history_restore_current_at(
                "Note", &f.root, &f.path, &operation
            ))
            .unwrap()
    );
    assert_eq!(f.status(&operation).status, "committed");
}

#[test]
fn hosted_history_managed_rename_keeps_owner_and_copy_never_inherits_original_restore_or_history() {
    let mut f = Fixture::new();
    let operation = fresh();
    let request = f.request(&operation, None);
    f.restore(request).unwrap();
    let original = f.core.qualified_node_at(&f.root, &f.path).unwrap().owner;
    let rename_id = fresh();
    let renamed = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: rename_id.clone(),
            root: display(&f.root),
            intent: OperationIntent::RenameNode {
                source: display(f.path.parent().unwrap()),
                name: "Renamed".into(),
            },
        })
        .unwrap();
    f.core
        .execute_operation(&rename_id, &renamed.plan_id)
        .unwrap();
    let renamed_path = f.root.join("Renamed/Renamed.typ");
    assert_eq!(
        f.core
            .qualified_node_at(&f.root, &renamed_path)
            .unwrap()
            .owner,
        original
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore_result_at(
                "Note",
                &f.root,
                &renamed_path,
                &operation
            ))
            .unwrap()
            .source,
        f.original_source
    );
    let copy_id = fresh();
    let copied = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: copy_id.clone(),
            root: display(&f.root),
            intent: OperationIntent::CopyNode {
                source: display(renamed_path.parent().unwrap()),
                parent: display(&f.root),
                name: "Copy".into(),
                allow_losses: false,
            },
        })
        .unwrap();
    f.core.execute_operation(&copy_id, &copied.plan_id).unwrap();
    let copy_path = f.root.join("Copy/Copy.typ");
    assert_ne!(
        f.core.qualified_node_at(&f.root, &copy_path).unwrap().owner,
        original
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_history_restore_result_at(
                "Note", &f.root, &copy_path, &operation
            ))
            .unwrap_err()
            .code,
        "identityPending"
    );
    let copy_session = f
        .core
        .with_actor(ALICE, |c| c.open(&copy_path, Some(f.root.clone())))
        .unwrap();
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c
                .read_history(&copy_session.session_id, &f.version))
            .unwrap_err()
            .code,
        "historyUnavailable"
    );
    f.restart();
    assert_eq!(
        f.core
            .qualified_node_at(&f.root, &renamed_path)
            .unwrap()
            .owner,
        original
    );
}

#[test]
fn hosted_history_qualification_rejects_unproved_external_move_and_other_workspace_rebinding() {
    let mut f = Fixture::new();
    let target = f.root.join("Untracked");
    fs::rename(f.path.parent().unwrap(), &target).unwrap();
    fs::rename(target.join("Note.typ"), target.join("Untracked.typ")).unwrap();
    assert_eq!(
        f.core
            .qualified_node_at(&f.root, &target.join("Untracked.typ"))
            .err()
            .unwrap()
            .code,
        "identityPending"
    );
    let foreign = f.temp.path().join("foreign");
    f.core.create_library(&foreign).unwrap();
    fs::rename(&target, foreign.join("Untracked")).unwrap();
    assert_eq!(
        f.core
            .qualified_node_at(&foreign, &foreign.join("Untracked/Untracked.typ"))
            .err()
            .unwrap()
            .code,
        "identityPending"
    );
}

#[test]
fn hosted_history_qualification_rejects_real_trash_children_through_windows_case_alias() {
    let mut f = Fixture::new();
    f.core
        .create_note_fixture(&f.root, f.path.parent().unwrap(), "Child")
        .unwrap();
    let operation = fresh();
    let plan = f
        .core
        .prepare_operation(OperationRequest {
            operation_id: operation.clone(),
            root: display(&f.root),
            intent: OperationIntent::TrashNode {
                source: display(f.path.parent().unwrap()),
            },
        })
        .unwrap();
    let receipt = f.core.execute_operation(&operation, &plan.plan_id).unwrap();
    let trashed = Path::new(&receipt.path).join("Child/Child.typ");
    assert!(trashed.exists());
    let alias = PathBuf::from(display(&trashed).replace(".weftext-trash", ".WEFTEXT-TRASH"));
    assert!(alias.exists());
    assert_eq!(
        f.core
            .qualified_node_at(&f.root, &trashed)
            .err()
            .unwrap()
            .code,
        "unsupportedRole"
    );
    assert_eq!(
        f.core
            .qualified_node_at(&f.root, &alias)
            .err()
            .unwrap()
            .code,
        "unsupportedRole"
    );
}

#[test]
fn hosted_history_restore_preserves_original_saved_definition_admission_without_new_reservation() {
    let mut f = Fixture::new();
    let bytes = include_bytes!("../../../docs/design/examples/schema.authoring.json").to_vec();
    let schema =
        weftext_data::Schema::from_author_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let config_operation = fresh();
    let config = f
        .core
        .prepare_config_save(&f.root, ConfigRole::Schema, None, bytes, &config_operation)
        .unwrap();
    f.core
        .execute_operation(&config_operation, &config.plan_id)
        .unwrap();
    let config = f.core.read_config(&f.root, ConfigRole::Schema).unwrap();
    let binding = weftext_data::SavedQueryBinding {
        schema: config.role_ref,
        revision: config.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let proposal_id = fresh();
    let proposal = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_definition_reserve(
                "Note",
                &f.root,
                HostedDefinitionReservationRequest {
                    session_id: f.doc.session_id.clone(),
                    base_id: f.doc.base_id.clone(),
                    operation_id: proposal_id.clone(),
                    intent_fingerprint: FP.into(),
                    binding,
                },
                |key, binding| {
                    weftext_data::upsert_saved_query(
                        &f.doc.source,
                        None,
                        key,
                        "weftext-query-historical",
                        binding,
                        &weftext_data::QueryPlan::default(),
                        &schema,
                    )
                    .map_err(|error| CoreError::from_data("invalidQueryDefinition", error))
                },
            )
        })
        .unwrap();
    let key = proposal.status.key.unwrap();
    let defined = proposal.patch.unwrap().source;
    let saved_id = fresh();
    let saved = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_save_with_definition_reservations(
                "Note",
                &f.root,
                HostedSaveRequest {
                    session_id: f.doc.session_id.clone(),
                    base_id: f.doc.base_id.clone(),
                    source: defined.clone(),
                    operation_id: saved_id.clone(),
                },
                std::slice::from_ref(&proposal_id),
            )
        })
        .unwrap();
    f.version = f
        .core
        .with_actor(ALICE, |c| c.list_history(&f.doc.session_id))
        .unwrap()
        .current_saved_version_id
        .unwrap();
    f.doc.base_id = saved.base_id.unwrap();
    let removed = f
        .core
        .with_actor(ALICE, |c| {
            c.actor_save(
                "Note",
                HostedSaveRequest {
                    session_id: f.doc.session_id.clone(),
                    base_id: f.doc.base_id.clone(),
                    source: f.doc.source.clone(),
                    operation_id: fresh(),
                },
            )
        })
        .unwrap();
    f.doc.base_id = removed.base_id.unwrap();
    let restore_id = fresh();
    let request = f.request(&restore_id, None);
    assert_eq!(f.restore(request).unwrap().result.unwrap().source, defined);
    assert!(
        f.core
            .is_definition_key_admitted(&f.root, f.doc.node_id.as_ref().unwrap(), &key)
            .unwrap()
    );
    assert_eq!(
        f.core
            .with_actor(ALICE, |c| c.actor_definition_status("Note", &proposal_id))
            .unwrap()
            .unwrap()
            .save_operation_id
            .as_deref(),
        Some(saved_id.as_str())
    );
}
