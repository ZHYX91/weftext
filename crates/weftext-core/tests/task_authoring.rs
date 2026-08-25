use weftext_core::{
    TaskAuthoringDiagnosticCode, TaskDateField, TaskDateTime, TaskEditIntent, TaskEditTarget,
    TaskPhase, TaskPriority, TaskRepeatFrom, TaskResolution, TaskState, analyze_task_source,
    plan_task_edit,
};

const TASK_ID: &str = "11111111-1111-4111-8111-111111111111";

fn occurrence_target(source: &str, index: usize) -> TaskEditTarget {
    let analysis = analyze_task_source(source);
    TaskEditTarget::Occurrence {
        range: analysis.tasks[index].range.clone(),
    }
}

#[test]
fn simple_toggles_change_only_the_exact_native_marker_and_never_assign_identity() {
    let source = "= Tasks\r\n\r\n* [ ] 中文\r\n* [*] العربية\n";
    let close = plan_task_edit(
        source,
        &occurrence_target(source, 0),
        &TaskEditIntent::Toggle,
    )
    .expect("close simple task");
    assert_eq!(
        close.proposed_source,
        "= Tasks\r\n\r\n* [x] 中文\r\n* [*] العربية\n"
    );
    assert!(close.assigned_id.is_none());
    assert_eq!(close.target.state, TaskState::Closed);
    assert!(close.target.metadata.is_none());
    assert_eq!(close.edit.start, source.find("[ ]").unwrap() as u64 + 1);
    assert_eq!(close.edit.end, close.edit.start + 1);

    let reopen = plan_task_edit(
        source,
        &occurrence_target(source, 1),
        &TaskEditIntent::Toggle,
    )
    .expect("reopen simple task");
    assert!(reopen.proposed_source.ends_with("* [ ] العربية\n"));
    assert!(reopen.assigned_id.is_none());
}

#[test]
fn metadata_edit_promotes_once_and_records_the_generated_uuid() {
    let source = "* [ ] Plan release\n";
    let plan = plan_task_edit(
        source,
        &occurrence_target(source, 0),
        &TaskEditIntent::SetDate {
            field: TaskDateField::Due,
            value: Some(TaskDateTime::Date("2026-09-05".to_owned())),
        },
    )
    .expect("promote simple task");
    let assigned = plan.assigned_id.expect("generated task ID");
    assert_eq!(
        plan.proposed_source,
        format!("* [ ] Plan release task:[id={assigned},due=2026-09-05]\n")
    );
    assert_eq!(plan.target.metadata.as_ref().unwrap().id, assigned);
    assert_eq!(plan.target.state, TaskState::Open);
    assert!(plan.analysis.diagnostics.is_empty());

    let clear_noop = plan_task_edit(
        source,
        &occurrence_target(source, 0),
        &TaskEditIntent::SetPriority { priority: None },
    )
    .expect("clear absent simple metadata");
    assert_eq!(clear_noop.proposed_source, source);
    assert!(clear_noop.assigned_id.is_none());

    for intent in [
        TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::Normal),
        },
        TaskEditIntent::SetPhase {
            phase: Some(TaskPhase::Todo),
        },
    ] {
        let effective_default = plan_task_edit(source, &occurrence_target(source, 0), &intent)
            .expect("effective simple-task default");
        assert_eq!(effective_default.proposed_source, source);
        assert!(effective_default.assigned_id.is_none());
    }
}

#[test]
fn structured_toggle_repairs_lifecycle_fields_and_preserves_unrelated_macro_bytes() {
    let open = format!(
        "* [ ] Work task:[ id={TASK_ID}, phase=in-progress , priority=low,due=2026-09-05 ]  \r\n"
    );
    let closed = plan_task_edit(&open, &occurrence_target(&open, 0), &TaskEditIntent::Toggle)
        .expect("close structured task");
    assert_eq!(closed.target.state, TaskState::Closed);
    assert!(closed.proposed_source.contains("* [x] Work task:[ id="));
    assert!(!closed.proposed_source.contains("phase=in-progress"));
    assert!(
        closed
            .proposed_source
            .contains("priority=low,due=2026-09-05 ]  \r\n")
    );
    assert!(closed.analysis.diagnostics.is_empty());

    let source = format!(
        "* [x] Done task:[id={TASK_ID},priority=high,resolution=cancelled,closed=2026-09-01,due=2026-09-05]\n"
    );
    let reopened = plan_task_edit(
        &source,
        &TaskEditTarget::Id {
            id: TASK_ID.parse().unwrap(),
        },
        &TaskEditIntent::Toggle,
    )
    .expect("reopen structured task");
    assert_eq!(reopened.target.state, TaskState::Open);
    assert!(!reopened.proposed_source.contains("resolution="));
    assert!(!reopened.proposed_source.contains("closed="));
    assert!(reopened.proposed_source.contains("priority=high"));
    assert!(reopened.proposed_source.contains("due=2026-09-05"));
}

#[test]
fn typed_fields_replace_remove_and_validate_through_the_canonical_parser() {
    let source = format!("* [ ] Work task:[id={TASK_ID}, priority=low ,due=2026-09-05]\n");
    let high = plan_task_edit(
        &source,
        &occurrence_target(&source, 0),
        &TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::High),
        },
    )
    .expect("set priority");
    assert!(high.proposed_source.contains(" priority=high ,due="));

    let cleared = plan_task_edit(
        &high.proposed_source,
        &TaskEditTarget::Id {
            id: TASK_ID.parse().unwrap(),
        },
        &TaskEditIntent::SetPriority { priority: None },
    )
    .expect("clear priority");
    assert!(!cleared.proposed_source.contains("priority="));
    assert!(cleared.proposed_source.contains("due=2026-09-05"));

    let recurrence = plan_task_edit(
        &source,
        &occurrence_target(&source, 0),
        &TaskEditIntent::SetRecurrence {
            rrule: Some("FREQ=WEEKLY;BYDAY=MO,FR".to_owned()),
            repeat_from: Some(TaskRepeatFrom::Due),
        },
    )
    .expect("set recurrence");
    assert!(
        recurrence
            .proposed_source
            .contains("rrule=\"FREQ=WEEKLY;BYDAY=MO,FR\",repeat-from=due")
    );
    assert!(recurrence.analysis.diagnostics.is_empty());
}

#[test]
fn invalid_targets_lifecycle_and_recurrence_fail_without_a_source_write() {
    let source = format!(
        "* [ ] Open task:[id={TASK_ID},due=2026-09-05,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
    );
    let recurring = plan_task_edit(
        &source,
        &occurrence_target(&source, 0),
        &TaskEditIntent::Toggle,
    )
    .expect_err("recurring toggle must use completion plan");
    assert_eq!(
        recurring.code,
        TaskAuthoringDiagnosticCode::RecurrenceRequiresCompletionPlan
    );

    let resolution = plan_task_edit(
        &source,
        &occurrence_target(&source, 0),
        &TaskEditIntent::SetResolution {
            resolution: Some(TaskResolution::Completed),
        },
    )
    .expect_err("open resolution is invalid");
    assert_eq!(resolution.code, TaskAuthoringDiagnosticCode::InvalidIntent);

    let bad_pair = plan_task_edit(
        &source,
        &occurrence_target(&source, 0),
        &TaskEditIntent::SetRecurrence {
            rrule: Some("FREQ=DAILY".to_owned()),
            repeat_from: None,
        },
    )
    .expect_err("incomplete recurrence intent");
    assert_eq!(bad_pair.code, TaskAuthoringDiagnosticCode::InvalidIntent);

    let missing = plan_task_edit(
        &source,
        &TaskEditTarget::Occurrence { range: 0..1 },
        &TaskEditIntent::SetPhase {
            phase: Some(TaskPhase::Todo),
        },
    )
    .expect_err("missing occurrence");
    assert_eq!(missing.code, TaskAuthoringDiagnosticCode::InvalidTarget);
}

#[test]
fn duplicate_ids_and_empty_promotion_are_refused() {
    let duplicate = format!("* [ ] A task:[id={TASK_ID}]\n* [ ] B task:[id={TASK_ID}]\n");
    let failure = plan_task_edit(
        &duplicate,
        &TaskEditTarget::Id {
            id: TASK_ID.parse().unwrap(),
        },
        &TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::High),
        },
    )
    .expect_err("duplicate target");
    assert_eq!(failure.code, TaskAuthoringDiagnosticCode::AmbiguousTarget);

    let empty = "* [ ]   \n";
    let failure = plan_task_edit(
        empty,
        &occurrence_target(empty, 0),
        &TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::High),
        },
    )
    .expect_err("empty promotion");
    assert_eq!(failure.code, TaskAuthoringDiagnosticCode::InvalidIntent);
}
