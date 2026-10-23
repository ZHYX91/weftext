//! Real capacity and monotonic permit boundaries, included in default discovery.
use super::*;

#[test]
fn reminder_complete_65_set_is_atomic_overflow_and_restart_never_drains_another_summary() {
    let mut f = Fixture::new("absolute");
    for index in 1..65 {
        f.add_task(&format!("Task{index:03}"));
    }
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    crate::device_state::set_test_failpoint("beforePublish");
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
            .unwrap_err()
            .code,
        "injectedFailure"
    );
    assert_eq!(fs::read(&ledger).unwrap(), original);
    crate::device_state::set_test_failpoint("afterPublish");
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
            .unwrap_err()
            .code,
        "injectedFailure"
    );
    f.reopen();
    let (state, _) = f.core().read_reminders().unwrap();
    assert_eq!(state.summaries.len(), 1);
    assert_eq!(state.records.len(), 65);
    let (summary_key, summary) = state.summaries.first_key_value().unwrap();
    assert_eq!(summary.members.len(), 64);
    assert_eq!(summary.cutoff, "2026-10-23T10:00:00Z");
    assert_eq!(summary.next_allowed_at, "2026-10-23T10:15:00Z");
    let (overflow_key, _) = state
        .records
        .iter()
        .find(|(_, record)| matches!(record.delivery, Delivery::SummaryOverflow { .. }))
        .unwrap();
    assert!(overflow_key > summary.members.last().unwrap());
    assert!(matches!(
        f.core()
            .reminder_delivery_status(overflow_key)
            .unwrap()
            .delivery,
        ReminderRecordedDelivery::InAppOnly { .. }
    ));
    let raw =
        serde_json::to_value(f.core().reminder_delivery_status(overflow_key).unwrap()).unwrap();
    assert_eq!(raw["delivery"]["state"], "inAppOnly");
    assert_eq!(raw["delivery"]["summaryKey"], summary_key.as_str());
    assert!(raw["delivery"].get("summary_key").is_none());
    assert!(
        f.core()
            .original_reminder_notification(overflow_key)
            .is_err()
    );
    assert!(
        f.core()
            .refresh_reminder_notification(overflow_key)
            .is_err()
    );
    assert!(
        f.core()
            .confirm_reminder_in_system_history(overflow_key, &summary.attempt.request)
            .is_err()
    );
    assert!(
        f.core()
            .retry_reminder_summary_with_clock(overflow_key, || Ok(instant("2026-10-23T10:20:00Z")))
            .is_err()
    );
    let mut forged = permit(summary_key, &summary.attempt, None);
    forged.key = overflow_key.clone();
    assert!(
        f.core()
            .check_reminder_permit_with_clock(&forged, || Ok(instant("2026-10-23T10:00:01Z")))
            .is_err()
    );
    assert!(
        f.core()
            .finish_reminder_delivery(&forged, ReminderSubmissionOutcome::SubmittedToSystem)
            .is_err()
    );
    assert!(f.core().resolve_reminder_activation(overflow_key).is_err());
    assert!(matches!(
        f.core()
            .reminder_delivery_status(summary_key)
            .unwrap()
            .delivery,
        ReminderRecordedDelivery::Unknown { .. }
    ));
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:10:00Z")))
            .unwrap(),
        ReminderSummaryPreparation::CoolingDown { .. }
    ));
    assert!(matches!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:15:00Z")))
            .unwrap(),
        ReminderSummaryPreparation::NoNewMissed { .. }
    ));
    // Even personal handling of the true OS summary cannot silently handle overflow.
    f.core().handle_reminder(summary_key).unwrap();
    assert!(
        !f.core()
            .reminder_delivery_status(overflow_key)
            .unwrap()
            .handled
    );
    let activation = f
        .core()
        .resolve_reminder_activation(&summary.attempt.request.activation_key)
        .unwrap();
    assert!(activation.candidates.is_empty());
    assert_eq!(
        f.core()
            .reminder_original_notifications(None)
            .unwrap()
            .len(),
        1
    );
    state.validate().unwrap();
}

#[test]
fn reminder_64_member_permit_checks_within_its_real_monotonic_lifetime() {
    let mut f = Fixture::new("absolute");
    for index in 1..MAX_SUMMARY_MEMBERS {
        f.add_task(&format!("Task{index:03}"));
    }
    let origin = instant("2026-10-23T10:00:00Z");
    let clock_start = std::time::Instant::now();
    // Civil time is offset into the fixture's due period, but advances with
    // actual elapsed time. Neither the wall nor monotonic lifetime is frozen.
    let mut clock = || {
        origin
            .shifted_seconds(i64::try_from(clock_start.elapsed().as_secs()).unwrap())
            .map_err(temporal_error)
    };
    let permit = match f
        .core()
        .prepare_reminder_summary_with_clock(options(), &mut clock)
        .unwrap()
    {
        ReminderSummaryPreparation::Started { permit, .. } => *permit,
        other => panic!("Expected a complete original summary: {other:?}"),
    };
    let (state, _) = f.core().read_reminders().unwrap();
    assert_eq!(state.summaries.len(), 1);
    assert_eq!(state.summaries[permit.execution_key()].members.len(), 64);
    f.core()
        .check_reminder_permit_with_clock(&permit, &mut clock)
        .unwrap();
    eprintln!("64-member permit real age: {:?}", permit.issued.elapsed());
    assert!(permit.admitted.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        f.core()
            .check_reminder_permit_with_clock(&permit, &mut clock)
            .unwrap_err()
            .code,
        "reminderPermitUsed"
    );
}

#[test]
fn reminder_full_scan_guard_budget_never_publishes_its_first_64_prefix() {
    let mut f = Fixture::new("absolute");
    // Each real managed task contributes at least one distinct source input.
    // Keep the real 1024 limit and 1025 distinct admitted/subscribed tasks.
    // This tests input-budget atomicity using multiple genuine import commits;
    // the 65-member tests retain independent NewNode/single-enable history.
    f.import_task_batches(MAX_INPUT_PATHS);
    let ledger = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&ledger).unwrap();
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
            .unwrap_err()
            .code,
        "budget"
    );
    assert_eq!(fs::read(&ledger).unwrap(), original);
    f.reopen();
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T11:00:00Z")))
            .unwrap_err()
            .code,
        "budget"
    );
    let (state, _) = f.core().read_reminders().unwrap();
    assert!(state.records.is_empty());
    assert!(state.summaries.is_empty());
}

#[test]
fn reminder_full_valid_ledger_byte_capacity_rejects_whole_batch_without_a_permit() {
    let mut f = Fixture::new("absolute");
    let candidate = f.candidate();
    let (mut state, digest) = f.core().read_reminders().unwrap();
    let at = instant("2026-10-22T00:00:00Z");
    let description = CoreDescription::from(
        CoreError::new(
            "simulatedOriginalNativeResult",
            UiMessage::error_core_io_failure(),
        )
        .diagnostic(DiagnosticOrigin::Os, "x".repeat(7500)),
    );
    for _ in 0..1750 {
        let owner = NodeRef {
            workspace_id: candidate.owner.workspace_id.clone(),
            node_id: fresh(),
        };
        let definition = "d".repeat(64);
        let key = execution_key(
            &state.device_id,
            &owner,
            INTENT,
            &definition,
            "2026-10-22T00:00:00Z",
        )
        .unwrap();
        let mut attempt = make_attempt(&key, options(), &at, 1).unwrap();
        attempt.delivery = ReminderDeliveryState::Unknown {
            description: description.clone(),
        };
        state.records.insert(
            key,
            Record {
                owner,
                reminder_id: INTENT.into(),
                definition_digest: definition,
                trigger_instant: "2026-10-22T00:00:00Z".into(),
                delivery: Delivery::Individual { attempt },
                handled: false,
                invalidated: None,
            },
        );
    }
    let identity = state.libraries[&candidate.owner.workspace_id]
        .identity
        .clone();
    while serde_json::to_vec(&state).unwrap().len() < MAX_BYTES - 16 {
        let id = fresh();
        state.libraries.insert(
            id.clone(),
            LibrarySelection {
                root: "D:/".into(),
                identity: identity.clone(),
                enabled: false,
            },
        );
        let size = serde_json::to_vec(&state).unwrap().len();
        if size > MAX_BYTES - 16 {
            state.libraries.remove(&id);
            break;
        }
        let padding = (MAX_BYTES - 16 - size).min(32_768 - 3);
        state
            .libraries
            .get_mut(&id)
            .unwrap()
            .root
            .push_str(&"x".repeat(padding));
    }
    state.validate().unwrap();
    assert!(MAX_BYTES - serde_json::to_vec(&state).unwrap().len() < 300);
    f.core().write_reminders(state, digest.as_deref()).unwrap();
    let path = f.core().device_record_path(DeviceRecordKind::Reminders);
    let original = fs::read(&path).unwrap();
    validate_reminder_device_record(&original).unwrap();
    assert_eq!(
        f.core()
            .prepare_reminder_summary_with_clock(options(), || Ok(instant("2026-10-23T10:00:00Z")))
            .unwrap_err()
            .code,
        "budget"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
}
