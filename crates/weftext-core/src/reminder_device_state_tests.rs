//! Real Windows/NTFS Core fixtures; external outcomes below are simulated
//! adapter results, not evidence that Windows displayed a notification.
use super::*;
use weftext_data::NativeValue;

const INTENT: &str = "a46e68ef-30a3-4f74-bba2-13327ce81f11";
const TRIGGER: &str = "2026-10-23T09:00:00Z";
const AT: &str = "2026-10-23T09:00:01Z";
fn options() -> ReminderNotificationOptions {
    ReminderNotificationOptions {
        app_id: "Weftext.Core.ReminderTests".into(),
        title: "Task reminder".into(),
        body: "Open your saved task".into(),
    }
}
fn submission_observation(result: ReminderOsCallResult) -> ReminderSubmissionObservation {
    ReminderSubmissionObservation {
        started_at: "2026-10-23T09:00:01.000000001Z".into(),
        returned_at: "2026-10-23T09:00:01.000000003Z".into(),
        elapsed_nanoseconds: "2".into(),
        result,
    }
}
fn instant(value: &str) -> ExactInstant {
    ExactInstant::parse(value).unwrap()
}
fn task_source(kind: &str) -> String {
    let reminder = if kind == "absolute" {
        format!("(id: \"{INTENT}\", kind: \"absolute\", at: \"{TRIGGER}\")")
    } else {
        format!("(id: \"{INTENT}\", kind: \"due_before\", before: \"1800\")")
    };
    format!(
        "#metadata((types: (\"task\",), status: \"open\", due: \"2026-10-23T09:30:00Z\", reminder: {reminder})) <weftext>\n= Task\n正文😀\n"
    )
}
struct Fixture {
    core: Option<Core>,
    data: PathBuf,
    root: PathBuf,
    owner: NodeRef,
    path: PathBuf,
    _temp: tempfile::TempDir,
}
impl Fixture {
    fn new(kind: &str) -> Self {
        Self::with_source(task_source(kind))
    }
    fn with_source(source: String) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("state");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        let library = core.create_library(&root).unwrap();
        let operation = fresh();
        let preview = core
            .prepare_operation(OperationRequest {
                operation_id: operation.clone(),
                root: display(&root),
                intent: OperationIntent::NewNode {
                    parent: display(&root),
                    name: "Task".into(),
                    source: Some(source),
                },
            })
            .unwrap();
        let owner = NodeRef {
            workspace_id: library.workspace_id,
            node_id: preview.new_node_id.clone().unwrap(),
        };
        let receipt = core
            .execute_operation(&operation, &preview.plan_id)
            .unwrap();
        let path = Path::new(&receipt.path).join("Task.typ");
        let settings = core.set_reminder_library(&root, true, 0, &fresh()).unwrap();
        core.set_reminder_subscription(
            &root,
            &owner,
            INTENT,
            true,
            settings.settings_revision,
            &fresh(),
        )
        .unwrap();
        Self {
            core: Some(core),
            data,
            root,
            owner,
            path,
            _temp: temp,
        }
    }
    fn core(&mut self) -> &mut Core {
        self.core.as_mut().unwrap()
    }
    fn candidate(&mut self) -> TaskReminderCandidate {
        let root = self.root.clone();
        let owner = self.owner.clone();
        self.core().task_reminder_candidate(&root, &owner).unwrap()
    }
    fn begin(&mut self, at: &str) -> ReminderDeliveryPermit {
        let candidate = self.candidate();
        match self
            .core()
            .begin_reminder_delivery_at(&candidate, options(), instant(at))
            .unwrap()
        {
            ReminderDeliveryStart::Started(permit) => permit,
            ReminderDeliveryStart::Retained(status) => {
                panic!("Expected a new original permit, got {status:?}")
            }
        }
    }
    fn summary(&mut self, at: &str) -> ReminderDeliveryPermit {
        let at = instant(at);
        match self
            .core()
            .prepare_reminder_summary_with_clock(options(), || Ok(at.clone()))
            .unwrap()
        {
            ReminderSummaryPreparation::Started { permit, .. } => *permit,
            other => panic!("Expected complete missed summary, got {other:?}"),
        }
    }
    fn reopen(&mut self) {
        drop(self.core.take());
        self.core = Some(Core::new(&self.data).unwrap());
    }
    fn edit(&mut self, key: &str, value: NativeValue) {
        let path = self.path.clone();
        let root = self.root.clone();
        let document = self.core().open(path, Some(root)).unwrap();
        let patch = weftext_data::patch_field(
            &document.source,
            &Schema::bundled().unwrap(),
            key,
            &[],
            &value,
        )
        .unwrap();
        let result = self
            .core()
            .save(SaveRequest {
                session_id: document.session_id.clone(),
                base_id: document.base_id,
                source: patch.source,
                operation_id: fresh(),
            })
            .unwrap();
        assert_eq!(result.receipt.status, "saved");
        self.core().close_session(&document.session_id).unwrap();
    }
    fn append_body(&mut self) {
        let path = self.path.clone();
        let root = self.root.clone();
        let document = self.core().open(path, Some(root)).unwrap();
        self.core()
            .save(SaveRequest {
                session_id: document.session_id.clone(),
                base_id: document.base_id,
                source: document.source + "\n只改正文\n",
                operation_id: fresh(),
            })
            .unwrap();
        self.core().close_session(&document.session_id).unwrap();
    }
    fn add_task(&mut self, name: &str) -> TaskReminderCandidate {
        let root = self.root.clone();
        let operation = fresh();
        let preview = self
            .core()
            .prepare_operation(OperationRequest {
                operation_id: operation.clone(),
                root: display(&root),
                intent: OperationIntent::NewNode {
                    parent: display(&root),
                    name: name.into(),
                    source: Some(task_source("absolute")),
                },
            })
            .unwrap();
        let owner = NodeRef {
            workspace_id: self.owner.workspace_id.clone(),
            node_id: preview.new_node_id.clone().unwrap(),
        };
        self.core()
            .execute_operation(&operation, &preview.plan_id)
            .unwrap();
        let settings = self.core().reminder_settings().unwrap();
        self.core()
            .set_reminder_subscription(
                &root,
                &owner,
                INTENT,
                true,
                settings.settings_revision,
                &fresh(),
            )
            .unwrap();
        self.core().task_reminder_candidate(&root, &owner).unwrap()
    }
    fn import_task_batches(&mut self, count: usize) {
        use weftext_export::exchange::{
            NativeEntry, NativeFileRole, read_native_package, write_native_package,
        };
        let root = self.root.clone();
        let original = fs::read_to_string(&self.path).unwrap();
        let source_id = self.owner.node_id.clone();
        let workspace_id = self.owner.workspace_id.clone();
        let mut template =
            read_native_package(&self.core().export_native_package(&root).unwrap()).unwrap();
        let sources = template
            .manifest
            .entries
            .iter()
            .filter(|(_, entry)| entry.role == NativeFileRole::Source)
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        for path in sources {
            template.manifest.entries.remove(&path);
            template.files.remove(&path);
        }
        for (batch, start) in (0..count).step_by(MAX_SUBSCRIPTION_CHANGES).enumerate() {
            let mut package = template.clone();
            package.manifest.package_id = fresh();
            let mut ids = vec![];
            for index in start..(start + MAX_SUBSCRIPTION_CHANGES).min(count) {
                let id = fresh();
                let bytes = original.replace(&source_id, &id).into_bytes();
                let path = format!("Task{index:04}/Task{index:04}.typ");
                let mut entry = NativeEntry::new(NativeFileRole::Source, &bytes);
                entry.node_id = Some(id.clone());
                entry.revision = Some(fresh());
                package.manifest.entries.insert(path.clone(), entry);
                package.files.insert(path, bytes);
                ids.push(id);
            }
            let prepared = self
                .core()
                .prepare_native_import(
                    &root,
                    &root,
                    &format!("Batch{batch:02}"),
                    write_native_package(&package).unwrap(),
                    &fresh(),
                    false,
                )
                .unwrap();
            self.core()
                .execute_operation(&prepared.operation_id, &prepared.plan_id)
                .unwrap();
            let changes = ids
                .iter()
                .map(|id| ReminderSubscription {
                    owner: NodeRef {
                        workspace_id: workspace_id.clone(),
                        node_id: prepared.node_map[id].clone(),
                    },
                    reminder_id: INTENT.into(),
                    enabled: true,
                })
                .collect::<Vec<_>>();
            let revision = self.core().reminder_settings().unwrap().settings_revision;
            self.core()
                .set_reminder_subscriptions(&root, &changes, revision, &fresh())
                .unwrap();
        }
        assert_eq!(
            self.core().reminder_settings().unwrap().subscriptions.len(),
            count + 1
        );
    }
    fn operate(&mut self, intent: OperationIntent) -> OperationReceipt {
        let root = self.root.clone();
        let operation = fresh();
        let preview = self
            .core()
            .prepare_operation(OperationRequest {
                operation_id: operation.clone(),
                root: display(&root),
                intent,
            })
            .unwrap();
        self.core()
            .execute_operation(&operation, &preview.plan_id)
            .unwrap()
    }
}
#[test]
fn reminder_new_library_uses_bundled_schema_without_materialization_and_draft_is_not_authority() {
    let mut f = Fixture::new("due_before");
    assert!(!f.root.join(".weftext-schema.json").exists());
    let candidate = f.candidate();
    assert_eq!(
        candidate.resolution.trigger_instant.as_deref(),
        Some(TRIGGER)
    );
    let path = f.path.clone();
    let root = f.root.clone();
    let document = f.core().open(path, Some(root)).unwrap();
    let draft = weftext_data::patch_field(
        &document.source,
        &Schema::bundled().unwrap(),
        "tasks/due",
        &[],
        &NativeValue::Text("2026-10-24T09:30:00Z".into()),
    )
    .unwrap();
    f.core()
        .persist_draft(&document.session_id, &draft.source)
        .unwrap();
    let saved = f
        .core()
        .task_reminder_for_session(&document.session_id)
        .unwrap();
    assert_eq!(
        saved.resolution.trigger_instant,
        candidate.resolution.trigger_instant
    );
    assert!(!f.root.join(".weftext-schema.json").exists());
    let _permit = f.begin(AT);
}
#[test]
fn reminder_begin_is_durable_and_restart_never_allocates_another_effect() {
    let mut f = Fixture::new("absolute");
    let device = f.core().reminder_settings().unwrap().device_id;
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    let request = permit.request().clone();
    f.reopen();
    assert_eq!(f.core().reminder_settings().unwrap().device_id, device);
    assert!(matches!(
        f.core().reminder_delivery_status(&key).unwrap().delivery,
        ReminderRecordedDelivery::Unknown { .. }
    ));
    let candidate = f.candidate();
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
    assert_eq!(
        f.core().reminder_original_notifications(None).unwrap()[0].request,
        request
    );
    assert_eq!(
        f.core().reminder_records_page(None).unwrap().records.len(),
        1
    );
}
#[test]
fn reminder_body_and_absolute_due_edits_keep_the_original_submitted_effect() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    let definition = f.candidate().resolution.definition_digest;
    f.core()
        .finish_reminder_delivery(&permit, ReminderSubmissionOutcome::SubmittedToSystem, None)
        .unwrap();
    f.append_body();
    f.edit(
        "tasks/due",
        NativeValue::Text("2026-10-24T12:30:00Z".into()),
    );
    let candidate = f.candidate();
    assert_eq!(candidate.resolution.definition_digest, definition);
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
    assert_eq!(
        f.core().reminder_records_page(None).unwrap().records[0]
            .status
            .execution_key,
        key
    );
}

#[test]
fn reminder_rename_move_trash_and_restore_use_current_identity_without_replaying_delivery() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    f.core()
        .finish_reminder_delivery(&permit, ReminderSubmissionOutcome::SubmittedToSystem, None)
        .unwrap();
    let root = f.root.clone();
    let _parent = f.add_task("Parent");
    f.operate(OperationIntent::RenameNode {
        source: display(&root.join("Task")),
        name: "Renamed".into(),
    });
    f.operate(OperationIntent::MoveNode {
        source: display(&root.join("Renamed")),
        parent: display(&root.join("Parent")),
    });
    f.path = root.join("Parent/Renamed/Renamed.typ");
    let candidate = f.candidate();
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
    let document = f.core().open_reminder_record(&key).unwrap();
    assert_eq!(Path::new(&document.path), f.path);
    assert_eq!(document.node_id.as_deref(), Some(f.owner.node_id.as_str()));
    f.core().close_session(&document.session_id).unwrap();
    let trash = f.operate(OperationIntent::TrashNode {
        source: display(f.path.parent().unwrap()),
    });
    assert!(
        !f.core()
            .refresh_reminder_notification(&key)
            .unwrap()
            .eligible
    );
    let activation = f
        .core()
        .resolve_reminder_activation(&permit.request().activation_key)
        .unwrap();
    assert!(activation.candidates.is_empty());
    assert_eq!(activation.issues.len(), 1);
    assert_eq!(
        f.core().open_reminder_record(&key).unwrap_err().code,
        "reminderUnavailable"
    );
    f.operate(OperationIntent::RestoreNode {
        trash_operation_id: trash.operation_id,
        node_id: None,
        parent: display(&root),
        name: "Restored".into(),
    });
    f.path = root.join("Restored/Restored.typ");
    let candidate = f.candidate();
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
    assert_eq!(
        f.core()
            .reminder_delivery_status(&key)
            .unwrap()
            .execution_key,
        key
    );
}
#[test]
fn reminder_due_edits_invalidate_old_relative_activation_and_a_b_a_does_not_replay() {
    let mut f = Fixture::new("due_before");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    let activation = permit.request().activation_key.clone();
    f.core()
        .finish_reminder_delivery(&permit, ReminderSubmissionOutcome::SubmittedToSystem, None)
        .unwrap();
    f.edit(
        "tasks/due",
        NativeValue::Text("2026-10-23T09:35:00Z".into()),
    );
    let changed = f.candidate();
    assert_eq!(
        changed.resolution.trigger_instant.as_deref(),
        Some("2026-10-23T09:05:00Z")
    );
    assert!(
        !f.core()
            .refresh_reminder_notification(&key)
            .unwrap()
            .eligible
    );
    assert!(
        f.core()
            .resolve_reminder_activation(&activation)
            .unwrap()
            .candidates
            .is_empty()
    );
    let other = f
        .core()
        .begin_reminder_delivery_at(&changed, options(), instant("2026-10-23T09:05:01Z"))
        .unwrap();
    assert!(matches!(other, ReminderDeliveryStart::Started(_)));
    f.edit(
        "tasks/due",
        NativeValue::Text("2026-10-23T09:30:00Z".into()),
    );
    let original = f.candidate();
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&original, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
    assert_eq!(
        f.core().reminder_delivery_status(&key).unwrap().invalidated,
        Some(ReminderCancellation::DefinitionChanged)
    );
}
#[test]
fn reminder_completion_and_personal_handling_are_orthogonal_to_external_result() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    f.core()
        .finish_reminder_delivery(&permit, ReminderSubmissionOutcome::SubmittedToSystem, None)
        .unwrap();
    f.edit("tasks/status", NativeValue::Text("done".into()));
    let eligibility = f.core().refresh_reminder_notification(&key).unwrap();
    assert!(!eligibility.eligible);
    assert_eq!(
        f.core().reminder_delivery_status(&key).unwrap().invalidated,
        Some(ReminderCancellation::TaskClosed)
    );
    f.edit("tasks/status", NativeValue::Text("open".into()));
    let candidate = f.candidate();
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
    let handled = f.core().handle_reminder(&key).unwrap();
    assert!(handled.handled);
    assert!(matches!(
        handled.delivery,
        ReminderRecordedDelivery::SubmittedToSystem { .. }
    ));
    assert!(f.candidate().task_open);
}
#[test]
fn reminder_permit_is_single_use_and_normal_admission_cannot_cross_target_minute() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin("2026-10-23T09:00:59Z");
    let at = instant("2026-10-23T09:01:00Z");
    assert_eq!(
        f.core()
            .check_reminder_permit_with_clock(&permit, || Ok(at.clone()))
            .unwrap_err()
            .code,
        "reminderMissed"
    );
    f.core()
        .finish_reminder_delivery(
            &permit,
            ReminderSubmissionOutcome::FailedBeforeCall(
                UiMessage::error_reminder_minute_ended_before_call().into(),
            ),
            None,
        )
        .unwrap();
    let summary = f.summary("2026-10-23T09:01:00Z");
    f.core()
        .check_reminder_permit_with_clock(&summary, || Ok(at.clone()))
        .unwrap();
    assert_eq!(
        f.core()
            .check_reminder_permit_with_clock(&summary, || Ok(at.clone()))
            .unwrap_err()
            .code,
        "reminderPermitUsed"
    );
}
#[test]
fn reminder_permit_original_inputs_reject_changed_body_without_recapture() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    assert!(!format!("{permit:?}").contains("正文😀"));
    f.append_body();
    let changed = fs::read(&f.path).unwrap();
    assert_eq!(
        f.core()
            .check_reminder_permit_with_clock(&permit, || Ok(instant(AT)))
            .unwrap_err()
            .code,
        "staleBase"
    );
    assert!(!permit.admitted.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(fs::read(&f.path).unwrap(), changed);
}
#[test]
fn reminder_begin_uses_a_fresh_clock_after_original_input_proof() {
    for (after, error) in [
        ("2026-10-23T09:00:06Z", None),
        ("2026-10-23T09:01:00Z", Some("reminderMissed")),
        ("2026-10-23T09:00:00Z", Some("reminderPermitExpired")),
    ] {
        let mut f = Fixture::new("absolute");
        let candidate = f.candidate();
        let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
        let original = fs::read(&ledger).unwrap();
        let mut reads = 0;
        let result = f
            .core()
            .begin_reminder_delivery_with_clock(&candidate, options(), || {
                reads += 1;
                Ok(instant(if reads == 1 { AT } else { after }))
            });
        if let Some(error) = error {
            assert_eq!(result.unwrap_err().code, error);
            assert_eq!(fs::read(&ledger).unwrap(), original);
        } else {
            let ReminderDeliveryStart::Started(permit) = result.unwrap() else {
                panic!("Expected the original fresh attempt")
            };
            assert_eq!(permit.expires_at(), "2026-10-23T09:00:11Z");
            f.core()
                .check_reminder_permit_with_clock(&permit, || Ok(instant(after)))
                .unwrap();
        }
    }
}
#[test]
fn reminder_permit_original_inputs_protect_records_and_reject_new_observation_names() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let owner = f.owner.node_id.clone();
    let operation = f
        .core()
        .operations
        .values()
        .find(|plan| {
            plan.node_id.as_deref() == Some(&owner)
                || plan.transaction.as_ref().is_some_and(|transaction| {
                    transaction
                        .births()
                        .iter()
                        .any(|birth| birth.node_id == owner)
                })
        })
        .unwrap()
        .operation_id
        .clone();
    let record = f.core().journal_path(&operation, "observations");
    let original = fs::read(&record).unwrap();
    let error = fs::write(&record, b"{invalid original record").unwrap_err();
    assert!(matches!(error.raw_os_error(), Some(5 | 32 | 33)));
    assert_eq!(fs::read(&record).unwrap(), original);
    let late = record.parent().unwrap().join(format!(
        "{operation}.recovery-observations-{}.json",
        fresh()
    ));
    fs::write(&late, b"[]").unwrap();
    assert_eq!(
        f.core()
            .check_reminder_permit_with_clock(&permit, || Ok(instant(AT)))
            .unwrap_err()
            .code,
        "staleBase"
    );
    assert!(!permit.admitted.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(fs::read(late).unwrap(), b"[]");
}
#[test]
fn reminder_permit_original_inputs_still_reject_a_late_duplicate_node() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let duplicate = f.root.join("Duplicate");
    fs::create_dir(&duplicate).unwrap();
    fs::copy(&f.path, duplicate.join("Duplicate.typ")).unwrap();
    assert!(
        f.core()
            .check_reminder_permit_with_clock(&permit, || Ok(instant(AT)))
            .is_err()
    );
    assert!(!permit.admitted.load(std::sync::atomic::Ordering::Acquire));
}
#[test]
fn reminder_final_input_proof_cannot_extend_a_normal_permit_past_its_civil_minute() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin("2026-10-23T09:00:58Z");
    let mut reads = 0;
    let error = f
        .core()
        .check_reminder_permit_with_clock(&permit, || {
            reads += 1;
            Ok(instant(if reads < 4 {
                "2026-10-23T09:00:59.999999999Z"
            } else {
                "2026-10-23T09:01:00Z"
            }))
        })
        .unwrap_err();
    assert_eq!(error.code, "reminderMissed");
    assert!(!permit.admitted.load(std::sync::atomic::Ordering::Acquire));
    assert!(matches!(
        f.core()
            .reminder_delivery_status(permit.execution_key())
            .unwrap()
            .delivery,
        ReminderRecordedDelivery::Unknown { .. }
    ));
}

#[test]
fn reminder_unknown_summary_never_regroups_and_only_new_keys_start_after_persisted_cooldown() {
    let mut f = Fixture::new("absolute");
    f.add_task("B");
    let summary = f.summary("2026-10-23T10:00:00.123456789123Z");
    f.reopen();
    f.add_task("C");
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant(
                "2026-10-23T10:15:00.123456789122Z"
            )))
            .unwrap(),
        ReminderSummaryPreparation::CoolingDown { .. }
    ));
    match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || {
            Ok(instant("2026-10-23T10:15:00.123456789123Z"))
        })
        .unwrap()
    {
        ReminderSummaryPreparation::Started {
            member_count,
            overflow_count,
            ..
        } => {
            assert_eq!(member_count, 1);
            assert_eq!(overflow_count, 0);
        }
        other => panic!("New original key did not qualify: {other:?}"),
    }
    assert!(matches!(
        f.core()
            .reminder_delivery_status(summary.execution_key())
            .unwrap()
            .delivery,
        ReminderRecordedDelivery::Unknown { .. }
    ));
}
#[test]
fn reminder_known_pre_call_summary_retry_keeps_original_members_request_cutoff_and_overflow() {
    let mut f = Fixture::new("absolute");
    for index in 1..65 {
        f.add_task(&format!("Task{index:03}"));
    }
    let original = f.summary("2026-10-23T10:00:00Z");
    let (before, _) = f.core().read_reminders().unwrap();
    let fixed = before.summaries[original.execution_key()].clone();
    assert_eq!(fixed.members.len(), 64);
    let overflow_before = before
        .records
        .iter()
        .filter(|(_, record)| matches!(record.delivery, Delivery::SummaryOverflow { .. }))
        .map(|(key, record)| (key.clone(), serde_json::to_value(record).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(overflow_before.len(), 1);
    f.core()
        .finish_reminder_delivery(
            &original,
            ReminderSubmissionOutcome::FailedBeforeCall(UiMessage::error_core_io_failure().into()),
            None,
        )
        .unwrap();
    f.reopen();
    f.add_task("B");
    let retry = match f
        .core()
        .retry_reminder_summary_with_clock(original.execution_key(), || {
            Ok(instant("2026-10-23T10:00:10Z"))
        })
        .unwrap()
    {
        ReminderDeliveryStart::Started(permit) => permit,
        other => panic!("Original pre-call failure was not retryable: {other:?}"),
    };
    assert_eq!(retry.execution_key(), original.execution_key());
    assert_eq!(retry.request(), original.request());
    let (after, _) = f.core().read_reminders().unwrap();
    let retried = &after.summaries[original.execution_key()];
    assert_eq!(retried.members, fixed.members);
    assert_eq!(retried.cutoff, fixed.cutoff);
    assert_eq!(retried.next_allowed_at, fixed.next_allowed_at);
    assert_eq!(retried.attempt.attempt_count, 2);
    let overflow_after = after
        .records
        .iter()
        .filter(|(_, record)| matches!(record.delivery, Delivery::SummaryOverflow { .. }))
        .map(|(key, record)| (key.clone(), serde_json::to_value(record).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(overflow_after, overflow_before);
    assert!(matches!(
        f.core()
            .reminder_delivery_status(&overflow_after[0].0)
            .unwrap()
            .delivery,
        ReminderRecordedDelivery::InAppOnly { .. }
    ));
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:10Z")))
            .unwrap(),
        ReminderSummaryPreparation::CoolingDown { .. }
    ));
    match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:15:00Z")))
        .unwrap()
    {
        ReminderSummaryPreparation::Started { member_count, .. } => assert_eq!(member_count, 1),
        other => panic!("Only the new B key should join a later batch: {other:?}"),
    }
    after.validate().unwrap();
}
#[test]
fn reminder_begin_publish_fault_never_returns_a_permit_and_retains_any_published_barrier() {
    for point in ["beforePublish", "afterPublish"] {
        let mut f = Fixture::new("absolute");
        let candidate = f.candidate();
        crate::device_state::set_test_failpoint(point);
        assert_eq!(
            f.core()
                .begin_reminder_delivery_at(&candidate, options(), instant(AT))
                .unwrap_err()
                .code,
            "injectedFailure"
        );
        let pending = fs::read_dir(f.data.join("device-state"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".pending"));
        // Before publication the complete staged original remains evidence.
        // After publication the exact canonical barrier is authoritative.
        assert_eq!(pending, point == "beforePublish");
        f.reopen();
        let candidate = f.candidate();
        let records = f.core().reminder_records_page(None).unwrap().records;
        let retry = f
            .core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap();
        if point == "beforePublish" {
            assert!(records.is_empty());
            assert!(matches!(retry, ReminderDeliveryStart::Started(_)));
        } else {
            assert_eq!(records.len(), 1);
            assert!(matches!(
                records[0].status.delivery,
                ReminderRecordedDelivery::Unknown { .. }
            ));
            assert!(matches!(retry, ReminderDeliveryStart::Retained(_)));
        }
    }
}

#[test]
fn reminder_finish_publish_faults_preserve_real_original_outcomes_across_restart() {
    for point in ["beforePublish", "afterPublish"] {
        let mut f = Fixture::new("absolute");
        let permit = f.begin(AT);
        let key = permit.execution_key().to_owned();
        let request = permit.request().clone();
        let observation = submission_observation(ReminderOsCallResult::ReturnedSuccess);
        crate::device_state::set_test_failpoint(point);
        assert_eq!(
            f.core()
                .finish_reminder_delivery(
                    &permit,
                    ReminderSubmissionOutcome::SubmittedToSystem,
                    Some(observation.clone())
                )
                .unwrap_err()
                .code,
            "injectedFailure"
        );
        f.reopen();
        let status = f.core().reminder_delivery_status(&key).unwrap();
        if point == "beforePublish" {
            assert!(status.submission_observation.is_none());
            assert!(matches!(
                status.delivery,
                ReminderRecordedDelivery::Unknown { .. }
            ));
        } else {
            assert_eq!(status.submission_observation, Some(observation));
            assert!(matches!(
                status.delivery,
                ReminderRecordedDelivery::SubmittedToSystem { .. }
            ));
        }
        let candidate = f.candidate();
        assert!(matches!(
            f.core()
                .begin_reminder_delivery_at(&candidate, options(), instant(AT))
                .unwrap(),
            ReminderDeliveryStart::Retained(_)
        ));
        assert_eq!(
            f.core().reminder_original_notifications(None).unwrap()[0].request,
            request
        );
    }
}
#[test]
fn reminder_missing_schema_is_rechecked_after_the_real_durable_action() {
    let mut f = Fixture::new("absolute");
    let root = f.root.clone();
    let config = f.core().read_config(&root, ConfigRole::Schema).unwrap();
    assert!(config.bytes.is_none());
    let witness = f.core().take_snapshot_witness(&config.base_id).unwrap();
    let candidate = f.candidate();
    let (state, digest) = f.core().read_reminders().unwrap();
    let error = f
        .core()
        .with_reminder_inputs(&root, &[witness], |core| {
            core.write_reminders(state, digest.as_deref())?;
            let config = core.read_config(&root, ConfigRole::Schema)?;
            let operation = fresh();
            let preview = core.prepare_config_save(
                &root,
                ConfigRole::Schema,
                config.bytes,
                crate::library_creation::ROOT_PRESET_SCHEMA
                    .as_bytes()
                    .to_vec(),
                &operation,
            )?;
            core.execute_operation(&operation, &preview.plan_id)?;
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "staleBase");
    assert!(root.join(".weftext-schema.json").exists());
    assert_eq!(
        f.candidate().resolution.definition_digest,
        candidate.resolution.definition_digest
    );
}
#[test]
fn reminder_copy_needs_new_consent_and_backup_transports_intent_without_device_records() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let root = f.root.clone();
    let source = f.path.parent().unwrap().to_path_buf();
    let operation = fresh();
    let preview = f
        .core()
        .prepare_operation(OperationRequest {
            operation_id: operation.clone(),
            root: display(&root),
            intent: OperationIntent::CopyNode {
                source: display(&source),
                parent: display(&root),
                name: "Copied".into(),
                allow_losses: false,
            },
        })
        .unwrap();
    let copied = NodeRef {
        workspace_id: f.owner.workspace_id.clone(),
        node_id: preview.node_map.get(&f.owner.node_id).cloned().unwrap(),
    };
    f.core()
        .execute_operation(&operation, &preview.plan_id)
        .unwrap();
    let candidate = f.core().task_reminder_candidate(&root, &copied).unwrap();
    assert_eq!(candidate.resolution.reminder_id.as_deref(), Some(INTENT));
    assert_eq!(
        f.core().reminder_eligibility(&candidate).unwrap_err().code,
        "reminderDisabled"
    );
    let destination = f._temp.path().join("backup");
    let operation = fresh();
    let preview = f
        .core()
        .prepare_operation(OperationRequest {
            operation_id: operation.clone(),
            root: display(&root),
            intent: OperationIntent::BackupLibrary {
                destination: display(&destination),
            },
        })
        .unwrap();
    f.core().handle_reminder(permit.execution_key()).unwrap();
    f.core()
        .execute_operation(&operation, &preview.plan_id)
        .unwrap();
    assert_eq!(
        fs::read(destination.join("library/Task/Task.typ")).unwrap(),
        fs::read(&f.path).unwrap()
    );
    assert!(!destination.join("private/device-state").exists());
}
#[test]
fn reminder_foreign_profile_and_second_owner_lease_never_adopt_or_replay() {
    let mut f = Fixture::new("absolute");
    let _permit = f.begin(AT);
    assert_eq!(Core::new(&f.data).err().unwrap().code, "instanceBusy");
    let other = f._temp.path().join("foreign-state");
    let mut foreign = Core::new(&other).unwrap();
    for kind in [DeviceRecordKind::Identity, DeviceRecordKind::Reminders] {
        let bytes = f
            .core()
            .read_device_record(kind, MAX_BYTES)
            .unwrap()
            .unwrap()
            .0;
        fs::write(foreign.device_record_path(kind), bytes).unwrap();
    }
    let identity_bytes = fs::read(foreign.device_record_path(DeviceRecordKind::Identity)).unwrap();
    assert_eq!(
        foreign.reminder_settings().unwrap_err().code,
        "foreignDeviceState"
    );
    assert_eq!(
        fs::read(foreign.device_record_path(DeviceRecordKind::Identity)).unwrap(),
        identity_bytes
    );
    drop(foreign);
}

#[test]
fn reminder_offline_selection_can_be_revoked_without_claiming_empty_scope_or_erasing_unknown() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let root = f.root.clone();
    let owner = f.owner.clone();
    let moved = f._temp.path().join("offline-library");
    fs::rename(&root, &moved).unwrap();
    let page = f.core().reminder_schedule_page(None).unwrap();
    assert!(page.candidates.is_empty());
    assert_eq!(page.issues.len(), 1);
    let settings = f.core().reminder_settings().unwrap();
    let settings = f
        .core()
        .set_reminder_subscription(
            Path::new(""),
            &owner,
            INTENT,
            false,
            settings.settings_revision,
            &fresh(),
        )
        .unwrap();
    let settings = f
        .core()
        .disable_reminder_library(&owner.workspace_id, settings.settings_revision, &fresh())
        .unwrap();
    assert!(!settings.libraries[0].enabled);
    assert!(!settings.subscriptions[0].enabled);
    let current = f
        .core()
        .refresh_reminder_notification(permit.execution_key())
        .unwrap();
    assert!(!current.eligible);
    assert!(current.issues.is_empty());
    assert!(matches!(
        current.original.delivery,
        ReminderDeliveryState::Unknown { .. }
    ));
    assert_eq!(
        f.core()
            .reminder_delivery_status(permit.execution_key())
            .unwrap()
            .invalidated,
        Some(ReminderCancellation::LibraryDisabled)
    );
    let original = fs::read(moved.join("Task/Task.typ")).unwrap();
    f.core().handle_reminder(permit.execution_key()).unwrap();
    assert_eq!(fs::read(moved.join("Task/Task.typ")).unwrap(), original);
    fs::rename(&moved, &root).unwrap();
    let page = f.core().reminder_schedule_page(None).unwrap();
    assert!(page.candidates.is_empty());
    assert!(page.issues.is_empty());
}

#[test]
fn reminder_history_confirmation_requires_the_complete_original_request_and_never_downgrades_success()
 {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    for member in 0..6 {
        let mut observed = permit.request().clone();
        match member {
            0 => observed.app_id.push_str(".other"),
            1 => observed.tag = "f".repeat(64),
            2 => observed.group.push_str(".other"),
            3 => observed.activation_key = fresh(),
            4 => observed.title.push_str(" other"),
            _ => observed.body.push_str(" other"),
        }
        assert_eq!(
            f.core()
                .confirm_reminder_in_system_history(permit.execution_key(), &observed)
                .unwrap_err()
                .code,
            "operationConflict"
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    f.core()
        .finish_reminder_delivery(
            &permit,
            ReminderSubmissionOutcome::Unknown(
                UiMessage::status_reminder_external_submission_unknown().into(),
            ),
            None,
        )
        .unwrap();
    let unknown = fs::read(&path).unwrap();
    assert!(
        f.core()
            .finish_reminder_delivery(
                &permit,
                ReminderSubmissionOutcome::FailedBeforeCall(
                    UiMessage::error_reminder_unknown_cannot_become_pre_call_failure().into()
                ),
                None,
            )
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), unknown);
    let status = f
        .core()
        .confirm_reminder_in_system_history(permit.execution_key(), permit.request())
        .unwrap();
    assert!(matches!(
        status.delivery,
        ReminderRecordedDelivery::SubmittedToSystem { .. }
    ));
    for outcome in [
        ReminderSubmissionOutcome::Unknown(
            UiMessage::status_reminder_external_submission_unknown().into(),
        ),
        ReminderSubmissionOutcome::FailedBeforeCall(
            UiMessage::error_reminder_permit_changed().into(),
        ),
    ] {
        let status = f
            .core()
            .finish_reminder_delivery(&permit, outcome, None)
            .unwrap();
        assert!(matches!(
            status.delivery,
            ReminderRecordedDelivery::SubmittedToSystem { .. }
        ));
    }
    f.reopen();
    let candidate = f.candidate();
    assert!(matches!(
        f.core()
            .begin_reminder_delivery_at(&candidate, options(), instant(AT))
            .unwrap(),
        ReminderDeliveryStart::Retained(_)
    ));
}

#[test]
fn reminder_original_effect_lookup_is_read_only_without_author_scope_and_has_no_member_alias() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    let root = f.root.clone();
    let offline = f._temp.path().join("offline-library");
    fs::rename(&root, &offline).unwrap();
    let observed = f
        .core()
        .original_reminder_notification(permit.execution_key())
        .unwrap();
    assert_eq!(observed.execution_key, permit.execution_key());
    assert_eq!(&observed.request, permit.request());
    assert!(matches!(
        observed.delivery,
        ReminderDeliveryState::Unknown { .. }
    ));
    assert!(
        f.core()
            .original_reminder_notification(&"f".repeat(64))
            .is_err()
    );
    assert_eq!(fs::read(&ledger).unwrap(), original);
    fs::rename(&offline, &root).unwrap();

    let mut f = Fixture::new("absolute");
    let a = f.candidate();
    f.add_task("B");
    let member_key = f.core().reminder_eligibility(&a).unwrap().execution_key;
    let summary = f.summary("2026-10-23T10:00:00Z");
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    let observed = f
        .core()
        .original_reminder_notification(summary.execution_key())
        .unwrap();
    assert_eq!(observed.execution_key, summary.execution_key());
    assert_eq!(&observed.request, summary.request());
    assert!(matches!(
        observed.delivery,
        ReminderDeliveryState::Unknown { .. }
    ));
    assert!(
        f.core()
            .original_reminder_notification(&member_key)
            .is_err()
    );
    assert_eq!(fs::read(&ledger).unwrap(), original);
}

#[test]
fn reminder_actor_cannot_read_decide_open_or_confirm_a_local_owner_effect() {
    fn denied<T>(result: Result<T>) {
        match result {
            Ok(_) => panic!("A hosted actor reached a device-local reminder capability"),
            Err(error) => assert_eq!(error.code, "syncOwnerRequired"),
        }
    }
    let mut f = Fixture::new("absolute");
    let candidate = f.candidate();
    let permit = f.begin(AT);
    let root = f.root.clone();
    let owner = f.owner.clone();
    let path = f.path.clone();
    let document = f.core().open(&path, Some(root.clone())).unwrap();
    let original_identity =
        fs::read(f.core().device_record_path(DeviceRecordKind::Identity)).unwrap();
    let original_reminders =
        fs::read(f.core().device_record_path(DeviceRecordKind::Reminders)).unwrap();
    let sessions = f.core().sessions.len();
    f.core()
        .with_actor_for_root("alice", &root, |core| {
            denied(core.reminder_settings());
            denied(core.reminder_records_page(None));
            denied(core.reminder_schedule_page(None));
            denied(core.task_reminder_candidate(&root, &owner));
            denied(core.task_reminder_for_session(&document.session_id));
            denied(core.reminder_eligibility(&candidate));
            denied(core.reminder_delivery_for_saved_task(&candidate));
            denied(core.set_reminder_library(&root, true, 2, &fresh()));
            denied(core.disable_reminder_library(&owner.workspace_id, 2, &fresh()));
            denied(core.set_reminder_subscription(&root, &owner, INTENT, false, 2, &fresh()));
            denied(core.begin_reminder_delivery(&candidate, options()));
            denied(core.prepare_reminder_summary(options()));
            denied(core.retry_reminder_summary(permit.execution_key()));
            denied(core.check_reminder_permit(&permit));
            denied(core.finish_reminder_delivery(
                &permit,
                ReminderSubmissionOutcome::SubmittedToSystem,
                None,
            ));
            denied(core.reminder_delivery_status(permit.execution_key()));
            denied(core.original_reminder_notification(permit.execution_key()));
            denied(core.reminder_original_notifications(None));
            denied(core.refresh_reminder_notification(permit.execution_key()));
            denied(
                core.confirm_reminder_in_system_history(permit.execution_key(), permit.request()),
            );
            denied(core.handle_reminder(permit.execution_key()));
            denied(core.resolve_reminder_activation(&permit.request().activation_key));
            denied(core.open_reminder_record(permit.execution_key()));
            assert_eq!(core.sessions.len(), sessions);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        fs::read(f.core().device_record_path(DeviceRecordKind::Identity)).unwrap(),
        original_identity
    );
    assert_eq!(
        fs::read(f.core().device_record_path(DeviceRecordKind::Reminders)).unwrap(),
        original_reminders
    );
    assert!(!permit.admitted.load(std::sync::atomic::Ordering::Acquire));
    f.core().close_session(&document.session_id).unwrap();
}

#[test]
fn reminder_actor_rejection_precedes_first_device_identity_allocation() {
    let temp = tempfile::tempdir().unwrap();
    let mut core = Core::new(temp.path().join("state")).unwrap();
    let root = temp.path().join("library");
    core.create_library(&root).unwrap();
    let identity = core.device_record_path(DeviceRecordKind::Identity);
    let reminders = core.device_record_path(DeviceRecordKind::Reminders);
    assert!(!identity.exists());
    assert!(!reminders.exists());
    core.with_actor_for_root("alice", &root, |core| {
        assert_eq!(
            core.reminder_settings().unwrap_err().code,
            "syncOwnerRequired"
        );
        assert_eq!(
            core.set_reminder_library(&root, true, 0, &fresh())
                .unwrap_err()
                .code,
            "syncOwnerRequired"
        );
        Ok(())
    })
    .unwrap();
    assert!(!identity.exists());
    assert!(!reminders.exists());
}

#[test]
fn reminder_subscription_batch_has_no_prefix_and_retries_before_recapture() {
    let mut f = Fixture::new("absolute");
    let second = f.add_task("Second");
    let root = f.root.clone();
    let first = ReminderSubscription {
        owner: f.owner.clone(),
        reminder_id: INTENT.into(),
        enabled: true,
    };
    let mut changes = vec![
        first.clone(),
        ReminderSubscription {
            owner: second.owner,
            reminder_id: INTENT.into(),
            enabled: true,
        },
    ];
    let revision = f.core().reminder_settings().unwrap().settings_revision;
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    assert!(
        f.core()
            .set_reminder_subscriptions(&root, &[first.clone(), first], revision, &fresh())
            .is_err()
    );
    changes[0].enabled = false;
    changes[1].reminder_id = fresh();
    assert!(
        f.core()
            .set_reminder_subscriptions(&root, &changes, revision, &fresh())
            .is_err()
    );
    assert_eq!(fs::read(&ledger).unwrap(), original);
    changes[0].enabled = true;
    changes[1].reminder_id = INTENT.into();
    let mutation = fresh();
    let applied = f
        .core()
        .set_reminder_subscriptions(&root, &changes, revision, &mutation)
        .unwrap();
    assert_eq!(applied.settings_revision, revision + 1);
    let committed = fs::read(&ledger).unwrap();
    fs::write(&f.path, "broken later author source").unwrap();
    let retry = f
        .core()
        .set_reminder_subscriptions(&root, &changes, revision, &mutation)
        .unwrap();
    assert_eq!(retry.settings_revision, applied.settings_revision);
    assert_eq!(fs::read(&ledger).unwrap(), committed);
}

#[test]
fn reminder_summary_rechecks_late_duplicate_before_publishing_original_capture() {
    let mut f = Fixture::new("absolute");
    f.add_task("Second");
    let copy = f.root.join("Copy");
    let source = fs::read(&f.path).unwrap();
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    let mut reads = 0;
    let error = f
        .core()
        .prepare_reminder_summary_with_clock(options(), || {
            reads += 1;
            if reads == 2 {
                fs::create_dir(&copy).unwrap();
                fs::write(copy.join("Copy.typ"), &source).unwrap();
            }
            Ok(instant("2026-10-23T10:00:00Z"))
        })
        .unwrap_err();
    assert_eq!(error.code, "reminderUnavailable");
    assert_eq!(fs::read(&ledger).unwrap(), original);
    let (state, _) = f.core().read_reminders().unwrap();
    assert!(state.records.is_empty() && state.summaries.is_empty());
}

#[test]
fn reminder_prepare_uses_fresh_attempt_clock_after_a_long_scan_and_rejects_rollback() {
    let mut f = Fixture::new("absolute");
    let mut reads = 0;
    let permit = match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || {
            reads += 1;
            Ok(instant(if reads == 1 {
                "2026-10-23T10:00:00Z"
            } else {
                "2026-10-23T10:00:06.123456789123Z"
            }))
        })
        .unwrap()
    {
        ReminderSummaryPreparation::Started { permit, .. } => permit,
        other => panic!("Long scan did not produce a fresh permit: {other:?}"),
    };
    assert_eq!(permit.expires_at(), "2026-10-23T10:00:11.123456789123Z");
    f.core()
        .check_reminder_permit_with_clock(&permit, || {
            Ok(instant("2026-10-23T10:00:06.123456789123Z"))
        })
        .unwrap();
    f.reopen();
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant(
                "2026-10-23T10:00:06.123456789122Z"
            )))
            .unwrap_err()
            .code,
        "reminderClockRollback"
    );
    assert_eq!(fs::read(&path).unwrap(), original);

    let mut f = Fixture::new("absolute");
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    let mut reads = 0;
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || {
                reads += 1;
                Ok(instant(if reads == 1 {
                    "2026-10-23T10:00:00Z"
                } else {
                    "2026-10-23T09:59:59Z"
                }))
            })
            .unwrap_err()
            .code,
        "reminderClockRollback"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn reminder_summary_unknown_library_is_not_consumed_and_can_qualify_after_recovery() {
    let mut f = Fixture::new("absolute");
    let first_root = f.root.clone();
    let first_owner = f.owner.clone();
    let second_root = f._temp.path().join("second-library");
    let library = f.core().create_library(&second_root).unwrap();
    let settings = f.core().reminder_settings().unwrap();
    f.core()
        .set_reminder_library(&second_root, true, settings.settings_revision, &fresh())
        .unwrap();
    f.root = second_root.clone();
    f.owner.workspace_id = library.workspace_id;
    let second = f.add_task("Second");
    f.root = first_root;
    f.owner = first_owner;
    let offline = f._temp.path().join("offline-second");
    fs::rename(&second_root, &offline).unwrap();
    match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
        .unwrap()
    {
        ReminderSummaryPreparation::Started {
            member_count,
            issue_count,
            issues,
            issues_complete,
            ..
        } => {
            assert_eq!(member_count, 1);
            assert_eq!(issue_count, 1);
            assert!(issues_complete);
            assert_eq!(issues[0].owner, second.owner);
        }
        other => panic!("Known scope was blocked or unknown scope was hidden: {other:?}"),
    }
    let (state, _) = f.core().read_reminders().unwrap();
    assert!(
        state
            .records
            .values()
            .all(|record| record.owner != second.owner)
    );
    f.reopen();
    fs::rename(&offline, &second_root).unwrap();
    match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:15:00Z")))
        .unwrap()
    {
        ReminderSummaryPreparation::Started {
            member_count,
            issue_count,
            issues_complete,
            ..
        } => {
            assert_eq!(member_count, 1);
            assert_eq!(issue_count, 0);
            assert!(issues_complete);
        }
        other => panic!("Recovered scope was silently consumed: {other:?}"),
    }
}

#[test]
fn reminder_summary_same_bytes_replacement_after_scan_cannot_publish_a_decision() {
    let mut f = Fixture::new("absolute");
    let path = f.path.clone();
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original_ledger = fs::read(&ledger).unwrap();
    let original_source = fs::read(&path).unwrap();
    let replaced = path.with_extension("retained-original");
    let mut reads = 0;
    assert!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || {
                reads += 1;
                if reads == 2 {
                    fs::rename(&path, &replaced).unwrap();
                    fs::write(&path, &original_source).unwrap();
                }
                Ok(instant("2026-10-23T10:00:00Z"))
            })
            .is_err()
    );
    assert_eq!(fs::read(&ledger).unwrap(), original_ledger);
    assert_eq!(fs::read(&replaced).unwrap(), original_source);
}

#[test]
fn reminder_normal_pre_call_failure_enters_missed_phase_once_across_restart_and_atomic_failure() {
    let mut f = Fixture::new("absolute");
    let normal = f.begin(AT);
    f.core()
        .finish_reminder_delivery(
            &normal,
            ReminderSubmissionOutcome::FailedBeforeCall(
                UiMessage::error_reminder_minute_ended_before_call().into(),
            ),
            None,
        )
        .unwrap();
    f.reopen();
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    crate::device_state::set_test_failpoint("beforePublish");
    assert!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
            .is_err()
    );
    assert_eq!(fs::read(&ledger).unwrap(), original);
    let summary = f.summary("2026-10-23T10:00:00Z");
    let (state, _) = f.core().read_reminders().unwrap();
    let Delivery::SummaryMember {
        prior_failed_attempt: Some(attempt),
        summary_key,
    } = &state.records[normal.execution_key()].delivery
    else {
        panic!("Missing original failed attempt");
    };
    assert_eq!(&attempt.request, normal.request());
    assert!(matches!(
        attempt.delivery,
        ReminderDeliveryState::FailedBeforeCall { .. }
    ));
    assert_eq!(summary_key, summary.execution_key());
    f.reopen();
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:15:00Z")))
            .unwrap(),
        ReminderSummaryPreparation::NoNewMissed { .. }
    ));
}

#[test]
fn reminder_summary_due_dependency_changes_get_a_new_key_but_body_edits_and_aba_do_not_replay() {
    let mut f = Fixture::new("due_before");
    let original = f.summary("2026-10-23T10:00:00Z");
    let (state, _) = f.core().read_reminders().unwrap();
    let original_member = state.summaries[original.execution_key()].members[0].clone();
    f.append_body();
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:15:00Z")))
            .unwrap(),
        ReminderSummaryPreparation::NoNewMissed { .. }
    ));
    f.edit(
        "tasks/due",
        NativeValue::Text("2026-10-23T09:31:00Z".into()),
    );
    let changed = f.summary("2026-10-23T10:15:00Z");
    let (state, _) = f.core().read_reminders().unwrap();
    assert_ne!(
        state.summaries[changed.execution_key()].members[0],
        original_member
    );
    f.edit(
        "tasks/due",
        NativeValue::Text("2026-10-23T09:30:00Z".into()),
    );
    f.reopen();
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:30:00Z")))
            .unwrap(),
        ReminderSummaryPreparation::NoNewMissed { .. }
    ));
    assert_eq!(f.core().read_reminders().unwrap().0.summaries.len(), 2);
    assert!(matches!(
        f.core()
            .reminder_delivery_status(&original_member)
            .unwrap()
            .delivery,
        ReminderRecordedDelivery::Unknown { .. }
    ));
}

#[test]
fn reminder_original_summary_retry_also_uses_a_fresh_clock_after_member_capture() {
    let mut f = Fixture::new("absolute");
    let original = f.summary("2026-10-23T10:00:00Z");
    f.core()
        .finish_reminder_delivery(
            &original,
            ReminderSubmissionOutcome::FailedBeforeCall(UiMessage::error_core_io_failure().into()),
            None,
        )
        .unwrap();
    let mut reads = 0;
    let retry = match f
        .core()
        .retry_reminder_summary_with_clock(original.execution_key(), || {
            reads += 1;
            Ok(instant(if reads == 1 {
                "2026-10-23T10:00:10Z"
            } else {
                "2026-10-23T10:00:16Z"
            }))
        })
        .unwrap()
    {
        ReminderDeliveryStart::Started(permit) => permit,
        other => panic!("Slow original retry did not get a fresh permit: {other:?}"),
    };
    assert_eq!(retry.expires_at(), "2026-10-23T10:00:21Z");
    assert_eq!(retry.request(), original.request());
    f.core()
        .check_reminder_permit_with_clock(&retry, || Ok(instant("2026-10-23T10:00:16Z")))
        .unwrap();
    f.core()
        .finish_reminder_delivery(
            &retry,
            ReminderSubmissionOutcome::FailedBeforeCall(UiMessage::error_core_io_failure().into()),
            None,
        )
        .unwrap();
    f.reopen();
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original_bytes = fs::read(&ledger).unwrap();
    // This is later than the immutable cutoff, but earlier than the last
    // trusted retry attempt. A restart must retain that stricter clock fence.
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:15Z")))
            .unwrap_err()
            .code,
        "reminderClockRollback"
    );
    assert_eq!(
        f.core()
            .retry_reminder_summary_with_clock(original.execution_key(), || Ok(instant(
                "2026-10-23T10:00:15Z"
            )))
            .unwrap_err()
            .code,
        "reminderClockRollback"
    );
    assert_eq!(fs::read(&ledger).unwrap(), original_bytes);
}

#[test]
fn reminder_summary_reports_a_sample_explicitly_without_consuming_unknown_scopes() {
    let mut f = Fixture::new("absolute");
    for index in 1..33 {
        f.add_task(&format!("Unavailable{index:02}"));
    }
    let root = f.root.clone();
    let offline = f._temp.path().join("offline-all");
    fs::rename(&root, &offline).unwrap();
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
        .unwrap()
    {
        ReminderSummaryPreparation::NoNewMissed {
            issues,
            issue_count,
            issues_complete,
        } => {
            assert_eq!(issue_count, 33);
            assert_eq!(issues.len(), 32);
            assert!(!issues_complete);
            assert!(issues.iter().all(|issue| !issue.error.code.is_empty()));
        }
        other => panic!("Unavailable scopes were treated as completed: {other:?}"),
    }
    assert_eq!(fs::read(&ledger).unwrap(), original);
    let (state, _) = f.core().read_reminders().unwrap();
    assert!(state.records.is_empty());
    assert!(state.summaries.is_empty());
    fs::rename(&offline, &root).unwrap();
    match f
        .core()
        .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
        .unwrap()
    {
        ReminderSummaryPreparation::Started {
            member_count,
            overflow_count,
            issue_count,
            issues_complete,
            ..
        } => {
            assert_eq!(member_count, 33);
            assert_eq!(overflow_count, 0);
            assert_eq!(issue_count, 0);
            assert!(issues_complete);
        }
        other => panic!("Previously unknown scopes were silently consumed: {other:?}"),
    }
}

#[path = "reminder_device_state_capacity_tests.rs"]
mod capacity;

#[test]
fn reminder_wake_hint_uses_actual_qualified_seconds_without_changing_http_or_record() {
    for (trigger, before) in [
        ("2026-10-23T09:00:59Z", "2026-10-23T09:00:58.25Z"),
        ("2026-10-23T09:01:00Z", "2026-10-23T09:00:59.25Z"),
        ("2026-10-23T09:00:37Z", "2026-10-23T09:00:36.25Z"),
    ] {
        let mut fixture = Fixture::with_source(task_source("absolute").replace(TRIGGER, trigger));
        let candidate = fixture.candidate();
        assert_eq!(
            candidate.resolution.trigger_instant.as_deref(),
            Some(trigger)
        );
        let record = fixture
            .core()
            .device_record_path(DeviceRecordKind::Reminders);
        let original = fs::read(&record).unwrap();
        let observed = std::time::Instant::now();
        let mut reads = 0;
        let upcoming = fixture
            .core()
            .reminder_eligibility_with_clock(&candidate, || {
                reads += 1;
                Ok((instant(before), observed))
            })
            .unwrap();
        assert_eq!(reads, 1);
        assert!(matches!(&upcoming.timing, ReminderTiming::Upcoming));
        assert_eq!(
            upcoming.next_recheck_at(),
            Some(observed + std::time::Duration::from_millis(750))
        );
        let wire = serde_json::to_value(&upcoming).unwrap();
        assert_eq!(
            wire.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            ["executionKey", "timing", "retained"].into_iter().collect()
        );
        let due = fixture
            .core()
            .reminder_eligibility_with_clock(&candidate, || Ok((instant(trigger), observed)))
            .unwrap();
        assert!(matches!(&due.timing, ReminderTiming::Due));
        assert!(due.next_recheck_at().is_none());
        let end = candidate
            .resolution
            .normal_delivery_until
            .as_deref()
            .unwrap();
        let missed = fixture
            .core()
            .reminder_eligibility_with_clock(&candidate, || Ok((instant(end), observed)))
            .unwrap();
        assert!(matches!(&missed.timing, ReminderTiming::Missed));
        assert!(missed.next_recheck_at().is_none());
        assert_eq!(fs::read(&record).unwrap(), original);
    }
}

#[test]
fn reminder_wake_hint_does_not_make_a_late_seconds_59_check_due() {
    let trigger = "2026-10-23T09:00:59Z";
    let mut fixture = Fixture::with_source(task_source("absolute").replace(TRIGGER, trigger));
    let candidate = fixture.candidate();
    let observed = std::time::Instant::now();
    let eligibility = fixture
        .core()
        .reminder_eligibility_with_clock(&candidate, || {
            Ok((instant("2026-10-23T09:01:03Z"), observed))
        })
        .unwrap();
    assert!(matches!(&eligibility.timing, ReminderTiming::Missed));
    assert!(eligibility.next_recheck_at().is_none());
    assert_eq!(
        fixture
            .core()
            .begin_reminder_delivery_at(&candidate, options(), instant("2026-10-23T09:01:03Z"))
            .unwrap_err()
            .code,
        "reminderMissed"
    );
}

#[test]
fn reminder_recheck_uses_exact_decimal_difference_and_ceil_nanoseconds() {
    for (at, trigger, delay) in [
        (
            "2026-10-23T09:00:58.25Z",
            "2026-10-23T09:00:59Z",
            std::time::Duration::from_millis(750),
        ),
        (
            "2026-10-23T09:00:58.999999999999Z",
            "2026-10-23T09:00:59Z",
            std::time::Duration::from_nanos(1),
        ),
        (
            "2026-10-23T09:00:58.0000000001Z",
            "2026-10-23T09:00:58.9999999999Z",
            std::time::Duration::from_secs(1),
        ),
        (
            "2026-10-23T09:00:59Z",
            "2026-10-23T09:00:59Z",
            std::time::Duration::ZERO,
        ),
        (
            "2026-10-23T09:01:03Z",
            "2026-10-23T09:00:59Z",
            std::time::Duration::ZERO,
        ),
    ] {
        assert_eq!(
            reminder_recheck_delay(&instant(at), &instant(trigger)).unwrap(),
            delay
        );
    }
    let tiny = ExactInstant {
        utc_seconds: instant(TRIGGER).utc_seconds,
        fraction: ExactDecimal::parse(&format!("0.{}1", "0".repeat(4093))).unwrap(),
    };
    assert_eq!(
        reminder_recheck_delay(&instant(TRIGGER), &tiny).unwrap(),
        std::time::Duration::from_nanos(1)
    );
    let future = instant("9999-12-31T23:59:59Z");
    let start = instant(TRIGGER);
    assert_eq!(
        reminder_recheck_delay(&start, &future).unwrap(),
        std::time::Duration::from_secs((future.utc_seconds - start.utc_seconds) as u64)
    );
}

#[test]
fn reminder_actual_call_observation_is_distinct_from_confirmation_and_survives_restart() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    let observation = submission_observation(ReminderOsCallResult::ReturnedSuccess);
    let status = f
        .core()
        .finish_reminder_delivery(
            &permit,
            ReminderSubmissionOutcome::SubmittedToSystem,
            Some(observation.clone()),
        )
        .unwrap();
    let ReminderRecordedDelivery::SubmittedToSystem { at } = &status.delivery else {
        panic!("expected the separately confirmed system result");
    };
    assert_ne!(at, &observation.started_at);
    assert_eq!(status.submission_observation, Some(observation.clone()));
    let wire = serde_json::to_value(&status).unwrap();
    assert_eq!(wire["submissionObservation"]["elapsedNanoseconds"], "2");
    assert_eq!(
        wire["submissionObservation"]["result"]["kind"],
        "returnedSuccess"
    );
    f.reopen();
    let original = fs::read(f.core().device_record_path(DeviceRecordKind::Reminders)).unwrap();
    assert_eq!(
        f.core()
            .reminder_delivery_status(&key)
            .unwrap()
            .submission_observation,
        Some(observation.clone())
    );
    let replayed = f
        .core()
        .finish_reminder_delivery(
            &permit,
            ReminderSubmissionOutcome::Unknown(
                UiMessage::status_reminder_external_submission_unknown().into(),
            ),
            None,
        )
        .unwrap();
    assert_eq!(replayed.submission_observation, Some(observation));
    assert_eq!(
        fs::read(f.core().device_record_path(DeviceRecordKind::Reminders)).unwrap(),
        original
    );
}

#[test]
fn reminder_returned_failure_keeps_unknown_and_original_observation_after_history_confirmation() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    let observation = submission_observation(ReminderOsCallResult::ReturnedFailure {
        hresult: -2147467259,
    });
    let unknown = || {
        ReminderSubmissionOutcome::Unknown(
            UiMessage::status_reminder_external_submission_unknown().into(),
        )
    };
    let status = f
        .core()
        .finish_reminder_delivery(&permit, unknown(), Some(observation.clone()))
        .unwrap();
    assert!(matches!(
        status.delivery,
        ReminderRecordedDelivery::Unknown { .. }
    ));
    assert_eq!(
        f.core()
            .finish_reminder_delivery(&permit, unknown(), None)
            .unwrap()
            .submission_observation,
        Some(observation.clone())
    );
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    let mut conflicting = observation.clone();
    conflicting.elapsed_nanoseconds = "3".into();
    assert_eq!(
        f.core()
            .finish_reminder_delivery(&permit, unknown(), Some(conflicting))
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    f.reopen();
    let confirmed = f
        .core()
        .confirm_reminder_in_system_history(&key, permit.request())
        .unwrap();
    assert!(matches!(
        confirmed.delivery,
        ReminderRecordedDelivery::SubmittedToSystem { .. }
    ));
    assert_eq!(confirmed.submission_observation, Some(observation));
}

#[test]
fn reminder_old_attempt_and_history_confirmation_do_not_invent_actual_call_times() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let key = permit.execution_key().to_owned();
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert!(
        raw["records"][&key]["delivery"]["attempt"]
            .get("submissionObservation")
            .is_none()
    );
    f.reopen();
    let status = f.core().reminder_delivery_status(&key).unwrap();
    assert!(status.submission_observation.is_none());
    assert!(serde_json::to_value(status).unwrap()["submissionObservation"].is_null());
    assert_eq!(fs::read(&path).unwrap(), original);
    let confirmed = f
        .core()
        .confirm_reminder_in_system_history(&key, permit.request())
        .unwrap();
    assert!(matches!(
        confirmed.delivery,
        ReminderRecordedDelivery::SubmittedToSystem { .. }
    ));
    assert!(confirmed.submission_observation.is_none());
}

#[test]
fn reminder_invalid_or_contradictory_actual_call_observation_cannot_change_the_original_barrier() {
    let mut f = Fixture::new("absolute");
    let permit = f.begin(AT);
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    let valid = submission_observation(ReminderOsCallResult::ReturnedSuccess);
    let mut invalid = Vec::new();
    for duration in ["-1", "01", "1.5", "340282366920938463463374607431768211456"] {
        let mut value = valid.clone();
        value.elapsed_nanoseconds = duration.into();
        invalid.push(value);
    }
    for field in ["started", "returned"] {
        let mut value = valid.clone();
        if field == "started" {
            value.started_at = "not an instant".into();
        } else {
            value.returned_at = "not an instant".into();
        }
        invalid.push(value);
    }
    invalid.push(submission_observation(
        ReminderOsCallResult::ReturnedFailure { hresult: 0 },
    ));
    // An observed failed Show cannot become a successful direct return.
    invalid.push(submission_observation(
        ReminderOsCallResult::ReturnedFailure {
            hresult: -2147467259,
        },
    ));
    for observation in invalid {
        assert!(
            f.core()
                .finish_reminder_delivery(
                    &permit,
                    ReminderSubmissionOutcome::SubmittedToSystem,
                    Some(observation)
                )
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    assert!(
        f.core()
            .finish_reminder_delivery(
                &permit,
                ReminderSubmissionOutcome::FailedBeforeCall(
                    UiMessage::error_core_io_failure().into()
                ),
                Some(valid),
            )
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn reminder_actual_call_uses_monotonic_duration_even_if_the_wall_clock_moves_back() {
    let observation = ReminderSubmissionObservation::from_system_times(
        UNIX_EPOCH + std::time::Duration::from_secs(2),
        UNIX_EPOCH + std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(10_000_000),
        ReminderOsCallResult::ReturnedSuccess,
    )
    .unwrap();
    assert_eq!(observation.started_at, "1970-01-01T00:00:02Z");
    assert_eq!(observation.returned_at, "1970-01-01T00:00:01Z");
    assert_eq!(observation.elapsed_nanoseconds, "10000000000000000");
    assert!(
        ReminderSubmissionObservation::from_system_times(
            UNIX_EPOCH - std::time::Duration::from_secs(1),
            UNIX_EPOCH,
            std::time::Duration::from_secs(1),
            ReminderOsCallResult::ReturnedSuccess,
        )
        .is_err()
    );
}
