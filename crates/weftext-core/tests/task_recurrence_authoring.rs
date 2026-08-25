use weftext_core::{
    DocumentRevision, TaskDateTime, TaskEditTarget, TaskId, TaskPriority,
    TaskRecurrenceCompletionContext, TaskRecurrenceCompletionDiagnosticCode, TaskRepeatFrom,
    TaskResolution, TaskState, analyze_task_source, plan_task_recurrence_completion,
};

const TASK_ID: &str = "11111111-1111-4111-8111-111111111111";
const DEPENDENCY_ID: &str = "22222222-2222-4222-8222-222222222222";

fn date(value: &str) -> TaskDateTime {
    TaskDateTime::Date(value.to_owned())
}

fn instant(value: &str) -> TaskDateTime {
    TaskDateTime::Instant(value.to_owned())
}

fn context(value: TaskDateTime) -> TaskRecurrenceCompletionContext {
    TaskRecurrenceCompletionContext {
        completed_at: value,
        utc_offset_minutes: 8 * 60,
    }
}

fn target(id: &str) -> TaskEditTarget {
    TaskEditTarget::Id {
        id: id.parse().expect("valid fixture task ID"),
    }
}

fn apply_edit(source: &str, edit: &weftext_core::DocumentEdit) -> String {
    let start = usize::try_from(edit.start).expect("edit start");
    let end = usize::try_from(edit.end).expect("edit end");
    format!("{}{}{}", &source[..start], edit.replacement, &source[end..])
}

fn next_due(rule: &str, anchor: &str) -> TaskDateTime {
    let source =
        format!("* [ ] Expand task:[id={TASK_ID},due={anchor},rrule={rule:?},repeat-from=due]\n");
    let plan = plan_task_recurrence_completion(&source, &target(TASK_ID), &context(date(anchor)))
        .expect("expand accepted recurrence");
    plan.next_task
        .expect("next occurrence")
        .metadata
        .expect("structured successor")
        .due
        .expect("successor due date")
}

#[test]
fn completion_closes_history_and_creates_one_fresh_structured_successor() {
    let source = format!(
        "* [ ] Ship 中文 task:[id={TASK_ID},phase=in-progress,priority=high,created=2026-08-01,start=2026-08-23,due=2026-08-24,rrule=\"FREQ=DAILY;INTERVAL=2;COUNT=3\",repeat-from=due,depends-on=\"{DEPENDENCY_ID}\"]\n"
    );
    let completion = context(instant("2026-08-24T17:30:00+08:00"));
    let plan = plan_task_recurrence_completion(&source, &target(TASK_ID), &completion)
        .expect("complete recurring task");

    assert_eq!(plan.base_revision, DocumentRevision::from_source(&source));
    assert_eq!(
        plan.next_revision,
        DocumentRevision::from_source(&plan.proposed_source)
    );
    assert_eq!(apply_edit(&source, &plan.edit), plan.proposed_source);
    assert!(!plan.stopped);
    assert!(plan.analysis.diagnostics.is_empty());
    assert_eq!(plan.analysis.tasks.len(), 2);

    assert_eq!(plan.completed_task.state, TaskState::Closed);
    let completed = plan.completed_task.metadata.expect("completed metadata");
    assert_eq!(completed.id.to_string(), TASK_ID);
    assert!(completed.phase.is_none());
    assert_eq!(completed.resolution, Some(TaskResolution::Completed));
    assert_eq!(completed.closed, Some(instant("2026-08-24T17:30:00+08:00")));
    assert_eq!(completed.recurrence.unwrap().count, Some(3));

    let next_id = plan.next_task_id.expect("successor ID");
    assert_ne!(next_id.to_string(), TASK_ID);
    let next = plan.next_task.expect("successor");
    assert_eq!(next.state, TaskState::Open);
    assert_eq!(next.description, "Ship 中文");
    let next = next.metadata.expect("successor metadata");
    assert_eq!(next.id, next_id);
    assert_eq!(next.priority, TaskPriority::High);
    assert_eq!(next.created, Some(completion.completed_at));
    assert_eq!(next.start, Some(date("2026-08-25")));
    assert_eq!(next.due, Some(date("2026-08-26")));
    assert_eq!(next.repeat_from, Some(TaskRepeatFrom::Due));
    assert_eq!(next.recurrence.unwrap().count, Some(2));
    assert_eq!(
        next.dependencies,
        vec![DEPENDENCY_ID.parse::<TaskId>().unwrap()]
    );
}

#[test]
fn count_and_until_stop_without_creating_a_partial_successor() {
    let source = format!(
        "* [ ] Counted task:[id={TASK_ID},due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=3\",repeat-from=due]\n"
    );
    let first =
        plan_task_recurrence_completion(&source, &target(TASK_ID), &context(date("2026-08-24")))
            .expect("first occurrence");
    let second_id = first.next_task_id.expect("second ID");
    let second = plan_task_recurrence_completion(
        &first.proposed_source,
        &TaskEditTarget::Id { id: second_id },
        &context(date("2026-08-25")),
    )
    .expect("second occurrence");
    assert_eq!(
        second
            .next_task
            .as_ref()
            .unwrap()
            .metadata
            .as_ref()
            .unwrap()
            .recurrence
            .as_ref()
            .unwrap()
            .count,
        Some(1)
    );
    let third = plan_task_recurrence_completion(
        &second.proposed_source,
        &TaskEditTarget::Id {
            id: second.next_task_id.unwrap(),
        },
        &context(date("2026-08-26")),
    )
    .expect("final occurrence");
    assert!(third.stopped);
    assert!(third.next_task.is_none());
    assert!(third.next_task_id.is_none());
    assert_eq!(third.analysis.tasks.len(), 3);
    assert!(
        third
            .analysis
            .tasks
            .iter()
            .all(|task| task.state == TaskState::Closed)
    );

    let until_source = format!(
        "* [ ] Bounded task:[id={TASK_ID},due=2026-08-24,rrule=\"FREQ=DAILY;UNTIL=2026-08-25\",repeat-from=due]\n"
    );
    let allowed = plan_task_recurrence_completion(
        &until_source,
        &target(TASK_ID),
        &context(date("2026-08-24")),
    )
    .expect("UNTIL is inclusive");
    assert_eq!(
        allowed
            .next_task
            .as_ref()
            .unwrap()
            .metadata
            .as_ref()
            .unwrap()
            .due,
        Some(date("2026-08-25"))
    );
    let stopped = plan_task_recurrence_completion(
        &allowed.proposed_source,
        &TaskEditTarget::Id {
            id: allowed.next_task_id.unwrap(),
        },
        &context(date("2026-08-25")),
    )
    .expect("stop beyond UNTIL");
    assert!(stopped.stopped);
    assert_eq!(stopped.analysis.tasks.len(), 2);
}

#[test]
fn every_accepted_frequency_and_filter_combination_expands_deterministically() {
    for (rule, anchor, expected) in [
        ("FREQ=DAILY;INTERVAL=2", "2026-08-24", "2026-08-26"),
        (
            "FREQ=WEEKLY;BYDAY=MO,FR;WKST=MO",
            "2026-08-24",
            "2026-08-28",
        ),
        ("FREQ=WEEKLY;INTERVAL=2;WKST=MO", "2026-08-24", "2026-09-07"),
        (
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,FR;WKST=MO",
            "2026-08-28",
            "2026-09-07",
        ),
        ("FREQ=MONTHLY;BYMONTHDAY=15,-1", "2026-08-15", "2026-08-31"),
        (
            "FREQ=MONTHLY;BYDAY=FR;BYMONTHDAY=28,29,30,31",
            "2026-08-24",
            "2026-08-28",
        ),
        ("FREQ=MONTHLY;BYDAY=FR", "2026-08-24", "2026-08-28"),
        ("FREQ=MONTHLY", "2026-01-31", "2026-03-31"),
        ("FREQ=YEARLY", "2024-02-29", "2028-02-29"),
        ("FREQ=YEARLY;BYMONTH=9", "2026-08-24", "2026-09-24"),
        ("FREQ=YEARLY;BYDAY=FR", "2026-08-24", "2026-08-28"),
        ("FREQ=YEARLY;BYMONTHDAY=1", "2026-08-24", "2026-09-01"),
        ("FREQ=YEARLY;BYMONTH=9;BYDAY=MO", "2026-08-24", "2026-09-07"),
        (
            "FREQ=YEARLY;BYMONTH=9;BYMONTHDAY=7",
            "2026-08-24",
            "2026-09-07",
        ),
        (
            "FREQ=YEARLY;BYDAY=FR;BYMONTHDAY=28,29,30,31",
            "2026-08-24",
            "2026-08-28",
        ),
        (
            "FREQ=YEARLY;BYDAY=MO;BYMONTHDAY=7;BYMONTH=9",
            "2026-08-24",
            "2026-09-07",
        ),
    ] {
        assert_eq!(next_due(rule, anchor), date(expected), "rule {rule}");
    }
}

#[test]
fn completion_based_recurrence_shifts_authored_dates_by_one_shared_day_delta() {
    let source = format!(
        "* [ ] After completion task:[id={TASK_ID},created=2026-08-01,start=2026-08-18,scheduled=2026-08-19,due=2026-08-20,rrule=\"FREQ=DAILY\",repeat-from=completion]\n"
    );
    let plan =
        plan_task_recurrence_completion(&source, &target(TASK_ID), &context(date("2026-08-24")))
            .expect("completion-based recurrence");
    let next = plan.next_task.unwrap().metadata.unwrap();
    assert_eq!(next.created, Some(date("2026-08-24")));
    assert_eq!(next.start, Some(date("2026-08-23")));
    assert_eq!(next.scheduled, Some(date("2026-08-24")));
    assert_eq!(next.due, Some(date("2026-08-25")));

    let instant_source = format!(
        "* [ ] Timed task:[id={TASK_ID},start=2026-08-23T09:30:00+08:00,due=2026-08-24T09:30:00+08:00,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
    );
    let instant_plan = plan_task_recurrence_completion(
        &instant_source,
        &target(TASK_ID),
        &context(instant("2026-08-24T10:00:00+08:00")),
    )
    .expect("instant recurrence");
    let next = instant_plan.next_task.unwrap().metadata.unwrap();
    assert_eq!(next.start, Some(instant("2026-08-24T09:30:00+08:00")));
    assert_eq!(next.due, Some(instant("2026-08-25T09:30:00+08:00")));
}

#[test]
fn successor_insertion_preserves_nesting_utf8_and_mixed_line_boundaries() {
    let source = format!(
        "= Tasks\r\n\r\n** [ ] العربية task:[id={TASK_ID},due=2026-08-24,rrule=\"FREQ=DAILY\",repeat-from=due]\r\n* [ ] Tail\n"
    );
    let plan =
        plan_task_recurrence_completion(&source, &target(TASK_ID), &context(date("2026-08-24")))
            .expect("nested recurrence");
    let generated = format!(
        "\r\n** [ ] العربية task:[id={},due=2026-08-25,rrule=\"FREQ=DAILY\",repeat-from=due]\r\n* [ ] Tail\n",
        plan.next_task_id.unwrap()
    );
    assert!(plan.proposed_source.ends_with(&generated));
    assert_eq!(plan.next_task.unwrap().list_depth, 2);
}

#[test]
fn invalid_ambiguous_closed_and_unrepresentable_requests_fail_closed() {
    let recurring = format!(
        "* [ ] Open task:[id={TASK_ID},due=2026-08-24,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
    );
    let invalid_time =
        plan_task_recurrence_completion(&recurring, &target(TASK_ID), &context(date("2026-02-30")))
            .expect_err("forged date");
    assert_eq!(
        invalid_time.code,
        TaskRecurrenceCompletionDiagnosticCode::InvalidContext
    );

    let invalid_offset = plan_task_recurrence_completion(
        &recurring,
        &target(TASK_ID),
        &TaskRecurrenceCompletionContext {
            completed_at: date("2026-08-24"),
            utc_offset_minutes: 1_440,
        },
    )
    .expect_err("forged offset");
    assert_eq!(
        invalid_offset.code,
        TaskRecurrenceCompletionDiagnosticCode::InvalidContext
    );

    let plain = format!("* [ ] Plain task:[id={TASK_ID}]\n");
    let not_recurring =
        plan_task_recurrence_completion(&plain, &target(TASK_ID), &context(date("2026-08-24")))
            .expect_err("non-recurring task");
    assert_eq!(
        not_recurring.code,
        TaskRecurrenceCompletionDiagnosticCode::NotOpenRecurringTask
    );

    let closed = format!(
        "* [x] Closed task:[id={TASK_ID},resolution=completed,closed=2026-08-24,due=2026-08-24,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
    );
    let closed =
        plan_task_recurrence_completion(&closed, &target(TASK_ID), &context(date("2026-08-24")))
            .expect_err("closed recurrence");
    assert_eq!(
        closed.code,
        TaskRecurrenceCompletionDiagnosticCode::NotOpenRecurringTask
    );

    let duplicate = format!(
        "* [ ] A task:[id={TASK_ID},due=2026-08-24,rrule=\"FREQ=DAILY\",repeat-from=due]\n* [ ] B task:[id={TASK_ID},due=2026-08-24,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
    );
    let duplicate =
        plan_task_recurrence_completion(&duplicate, &target(TASK_ID), &context(date("2026-08-24")))
            .expect_err("duplicate identity");
    assert_eq!(
        duplicate.code,
        TaskRecurrenceCompletionDiagnosticCode::AmbiguousTarget
    );

    let end = format!(
        "* [ ] Last task:[id={TASK_ID},due=9999-12-31,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
    );
    let end = plan_task_recurrence_completion(&end, &target(TASK_ID), &context(date("9999-12-31")))
        .expect_err("calendar exhaustion");
    assert_eq!(
        end.code,
        TaskRecurrenceCompletionDiagnosticCode::UnrepresentableNextOccurrence
    );

    assert_eq!(analyze_task_source(&recurring).tasks.len(), 1);
}
