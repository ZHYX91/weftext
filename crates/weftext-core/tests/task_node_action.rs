use std::str::FromStr;

use serde_json::json;
use weftext_core::{
    DocumentRevision, NodeId, TaskNodeActionEvidence, TaskNodeClosedEdit, TaskNodeEditError,
    TaskNodeEditIntent, TaskNodeEditRequest, TaskNodePriority, TaskNodeProfile, TaskNodeState,
    TaskNodeTemporal, TaskNodeTemporalField, analyze_task_node_profile, plan_task_node_source_edit,
};

const NODE_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn source(state: &str, attributes: &str) -> String {
    format!("= Task 😀\n:weftext-task: v1\n:weftext-task-state: {state}\n{attributes}\nBody שלום\n")
}

fn request(source: &str, intent: TaskNodeEditIntent) -> TaskNodeEditRequest {
    let revision = DocumentRevision::from_source(source);
    TaskNodeEditRequest {
        evidence: TaskNodeActionEvidence {
            node_id: NodeId::from_str(NODE_ID).expect("node ID"),
            revision: revision.clone(),
            profile_revision: revision,
        },
        intent,
    }
}

fn profile(source: &str) -> TaskNodeProfile {
    analyze_task_node_profile(source, Some(NodeId::from_str(NODE_ID).expect("node ID")))
        .profile
        .expect("valid profile")
}

fn temporal(value: &str) -> TaskNodeTemporal {
    TaskNodeTemporal::parse(value).expect("temporal")
}

#[test]
#[allow(clippy::too_many_lines)]
fn state_transitions_apply_closed_invariants_without_generating_time() {
    let open = source("todo", "");
    let closed = plan_task_node_source_edit(
        &open,
        &request(
            &open,
            TaskNodeEditIntent::SetState {
                state: TaskNodeState::Completed,
                closed: TaskNodeClosedEdit::Preserve,
            },
        ),
    )
    .expect("open to closed");
    assert_eq!(closed.summary.after.state, TaskNodeState::Completed);
    assert_eq!(closed.summary.after.closed, None);
    assert!(!closed.proposed_source.contains("weftext-task-closed"));
    assert_eq!(closed.edits.len(), 1);
    assert_eq!(
        closed.summary.next_revision,
        closed.summary.next_profile_revision
    );

    let explicitly_closed = plan_task_node_source_edit(
        &open,
        &request(
            &open,
            TaskNodeEditIntent::SetState {
                state: TaskNodeState::Completed,
                closed: TaskNodeClosedEdit::Set {
                    value: temporal("2026-08-25"),
                },
            },
        ),
    )
    .expect("open to closed with authored date");
    assert_eq!(explicitly_closed.edits.len(), 2);
    assert_eq!(
        explicitly_closed.summary.after.closed,
        Some(temporal("2026-08-25"))
    );

    let with_closed = source("completed", ":weftext-task-closed: 2026-08-25T01:02:03Z");
    let reopened = plan_task_node_source_edit(
        &with_closed,
        &request(
            &with_closed,
            TaskNodeEditIntent::SetState {
                state: TaskNodeState::InProgress,
                closed: TaskNodeClosedEdit::Preserve,
            },
        ),
    )
    .expect("closed to open");
    assert_eq!(reopened.summary.after.state, TaskNodeState::InProgress);
    assert_eq!(reopened.summary.after.closed, None);
    assert!(!reopened.proposed_source.contains("weftext-task-closed"));
    assert_eq!(reopened.edits.len(), 2);

    let preserved = plan_task_node_source_edit(
        &with_closed,
        &request(
            &with_closed,
            TaskNodeEditIntent::SetState {
                state: TaskNodeState::Cancelled,
                closed: TaskNodeClosedEdit::Preserve,
            },
        ),
    )
    .expect("closed resolution preserve");
    assert_eq!(preserved.summary.after.closed, profile(&with_closed).closed);
    assert_eq!(preserved.edits.len(), 1);

    let cleared = plan_task_node_source_edit(
        &with_closed,
        &request(
            &with_closed,
            TaskNodeEditIntent::SetState {
                state: TaskNodeState::Cancelled,
                closed: TaskNodeClosedEdit::Clear,
            },
        ),
    )
    .expect("closed resolution clear");
    assert_eq!(cleared.summary.after.closed, None);
    assert_eq!(cleared.edits.len(), 2);

    let replacement = temporal("2026-08-26");
    let set = plan_task_node_source_edit(
        &with_closed,
        &request(
            &with_closed,
            TaskNodeEditIntent::SetState {
                state: TaskNodeState::Cancelled,
                closed: TaskNodeClosedEdit::Set {
                    value: replacement.clone(),
                },
            },
        ),
    )
    .expect("closed resolution set");
    assert_eq!(set.summary.after.closed, Some(replacement));
    assert_eq!(set.edits.len(), 2);

    assert_eq!(
        plan_task_node_source_edit(
            &open,
            &request(
                &open,
                TaskNodeEditIntent::SetState {
                    state: TaskNodeState::OnHold,
                    closed: TaskNodeClosedEdit::Set {
                        value: temporal("2026-08-26"),
                    },
                },
            ),
        ),
        Err(TaskNodeEditError::InvalidIntent)
    );
}

#[test]
fn priority_all_six_explicit_normal_removal_and_no_op_are_distinct() {
    let base = source("todo", "");
    for priority in [
        TaskNodePriority::Lowest,
        TaskNodePriority::Low,
        TaskNodePriority::Normal,
        TaskNodePriority::Medium,
        TaskNodePriority::High,
        TaskNodePriority::Highest,
    ] {
        let plan = plan_task_node_source_edit(
            &base,
            &request(
                &base,
                TaskNodeEditIntent::SetPriority {
                    priority: Some(priority),
                },
            ),
        )
        .expect("set priority");
        assert_eq!(plan.summary.after.priority, Some(priority));
        assert_eq!(plan.edits.len(), 1);
        if priority == TaskNodePriority::Normal {
            assert!(
                plan.proposed_source
                    .contains(":weftext-task-priority: normal")
            );
        }
    }

    let authored = source("todo", ":weftext-task-priority: high");
    let removed = plan_task_node_source_edit(
        &authored,
        &request(
            &authored,
            TaskNodeEditIntent::SetPriority { priority: None },
        ),
    )
    .expect("remove priority");
    assert_eq!(removed.summary.after.priority, None);
    assert!(!removed.proposed_source.contains("weftext-task-priority"));

    let no_op = plan_task_node_source_edit(
        &authored,
        &request(
            &authored,
            TaskNodeEditIntent::SetPriority {
                priority: Some(TaskNodePriority::High),
            },
        ),
    )
    .expect("verified no-op");
    assert!(no_op.edits.is_empty());
    assert_eq!(no_op.proposed_source, authored);
    assert_eq!(no_op.summary.base_revision, no_op.summary.next_revision);
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_temporal_field_sets_changes_and_removes_canonical_values() {
    for field in [
        TaskNodeTemporalField::Created,
        TaskNodeTemporalField::Start,
        TaskNodeTemporalField::Scheduled,
        TaskNodeTemporalField::Due,
    ] {
        let base = source("todo", "");
        let first = temporal("2026-08-25");
        let set = plan_task_node_source_edit(
            &base,
            &request(
                &base,
                TaskNodeEditIntent::SetTemporal {
                    field,
                    value: Some(first),
                },
            ),
        )
        .expect("set temporal");
        assert_eq!(
            temporal_value(&set.summary.after, field).unwrap().as_str(),
            "2026-08-25"
        );

        let changed_value = temporal("2026-08-26T01:02:03+08:00");
        let changed = plan_task_node_source_edit(
            &set.proposed_source,
            &request(
                &set.proposed_source,
                TaskNodeEditIntent::SetTemporal {
                    field,
                    value: Some(changed_value),
                },
            ),
        )
        .expect("change temporal");
        assert_eq!(
            temporal_value(&changed.summary.after, field)
                .unwrap()
                .as_str(),
            "2026-08-26T01:02:03+08:00"
        );

        let removed = plan_task_node_source_edit(
            &changed.proposed_source,
            &request(
                &changed.proposed_source,
                TaskNodeEditIntent::SetTemporal { field, value: None },
            ),
        )
        .expect("remove temporal");
        assert_eq!(temporal_value(&removed.summary.after, field), None);
    }

    let closed = source("completed", "");
    let set_closed = plan_task_node_source_edit(
        &closed,
        &request(
            &closed,
            TaskNodeEditIntent::SetTemporal {
                field: TaskNodeTemporalField::Closed,
                value: Some(temporal("2026-08-25")),
            },
        ),
    )
    .expect("set closed temporal");
    assert_eq!(
        set_closed.summary.after.closed,
        Some(temporal("2026-08-25"))
    );
    let clear_closed = plan_task_node_source_edit(
        &set_closed.proposed_source,
        &request(
            &set_closed.proposed_source,
            TaskNodeEditIntent::SetTemporal {
                field: TaskNodeTemporalField::Closed,
                value: None,
            },
        ),
    )
    .expect("clear closed temporal");
    assert_eq!(clear_closed.summary.after.closed, None);

    let open = source("todo", "");
    assert_eq!(
        plan_task_node_source_edit(
            &open,
            &request(
                &open,
                TaskNodeEditIntent::SetTemporal {
                    field: TaskNodeTemporalField::Closed,
                    value: Some(temporal("2026-08-25")),
                },
            ),
        ),
        Err(TaskNodeEditError::InvalidIntent)
    );
    let open_clear = plan_task_node_source_edit(
        &open,
        &request(
            &open,
            TaskNodeEditIntent::SetTemporal {
                field: TaskNodeTemporalField::Closed,
                value: None,
            },
        ),
    )
    .expect("open closed clear no-op");
    assert!(open_clear.edits.is_empty());
}

#[test]
fn revision_dimensions_invalid_profiles_and_exact_formatting_fail_or_preserve() {
    let base = source("todo", "");
    let intent = TaskNodeEditIntent::SetPriority {
        priority: Some(TaskNodePriority::High),
    };
    let mut stale_document = request(&base, intent.clone());
    stale_document.evidence.revision = DocumentRevision::from_source("stale document");
    assert_eq!(
        plan_task_node_source_edit(&base, &stale_document),
        Err(TaskNodeEditError::StaleDocumentRevision)
    );
    let mut stale_profile = request(&base, intent.clone());
    stale_profile.evidence.profile_revision = DocumentRevision::from_source("stale profile");
    assert_eq!(
        plan_task_node_source_edit(&base, &stale_profile),
        Err(TaskNodeEditError::StaleProfileRevision)
    );

    let invalid = source("waiting", "");
    assert!(matches!(
        plan_task_node_source_edit(&invalid, &request(&invalid, intent.clone())),
        Err(TaskNodeEditError::InvalidCurrentProfile { diagnostics }) if !diagnostics.is_empty()
    ));

    let exact = format!(
        concat!(
            "---\r\nweftext:\r\n  id: \"{}\"\r\n---\r\n",
            "= 发布 😀 שלום\r\n",
            "// keep comment\r\n",
            ":weftext-task: v1\r\n",
            ":weftext-task-state: todo\r\n",
            ":custom-note: 中文\r\n",
            "\r\n",
            "Body مرحبا 😀",
        ),
        NODE_ID
    );
    let plan = plan_task_node_source_edit(&exact, &request(&exact, intent))
        .expect("CRLF exact source plan");
    assert_eq!(plan.edits.len(), 1);
    assert!(plan.proposed_source.contains(":custom-note: 中文\r\n"));
    assert!(plan.proposed_source.contains("// keep comment\r\n"));
    assert!(plan.proposed_source.ends_with("Body مرحبا 😀"));
    assert!(!plan.proposed_source.ends_with(['\r', '\n']));
    let edit = &plan.edits[0];
    assert_eq!(
        &exact[..edit.range.start],
        &plan.proposed_source[..edit.range.start]
    );
    assert_eq!(
        &exact[edit.range.end..],
        &plan.proposed_source[edit.range.start + edit.replacement.len()..]
    );
}

#[test]
fn request_serde_rejects_unknown_fields_and_enums_with_stable_spelling() {
    let base = source("todo", "");
    let request = request(
        &base,
        TaskNodeEditIntent::SetState {
            state: TaskNodeState::InProgress,
            closed: TaskNodeClosedEdit::Preserve,
        },
    );
    let value = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(value["intent"]["kind"], json!("set_state"));
    assert_eq!(value["intent"]["state"], json!("in-progress"));
    assert_eq!(value["intent"]["closed"]["kind"], json!("preserve"));
    let decoded: TaskNodeEditRequest =
        serde_json::from_value(value.clone()).expect("deserialize request");
    assert_eq!(decoded, request);

    let mut unknown_request = value.clone();
    unknown_request["extra"] = json!(true);
    assert!(serde_json::from_value::<TaskNodeEditRequest>(unknown_request).is_err());
    let mut unknown_intent = value.clone();
    unknown_intent["intent"]["extra"] = json!(true);
    assert!(serde_json::from_value::<TaskNodeEditRequest>(unknown_intent).is_err());
    let mut unknown_evidence = value.clone();
    unknown_evidence["evidence"]["extra"] = json!(true);
    assert!(serde_json::from_value::<TaskNodeEditRequest>(unknown_evidence).is_err());
    let mut unknown_state = value.clone();
    unknown_state["intent"]["state"] = json!("blocked");
    assert!(serde_json::from_value::<TaskNodeEditRequest>(unknown_state).is_err());
    let mut unknown_closed = value.clone();
    unknown_closed["intent"]["closed"]["extra"] = json!(true);
    assert!(serde_json::from_value::<TaskNodeEditRequest>(unknown_closed).is_err());

    let default_closed = json!({
        "evidence": value["evidence"].clone(),
        "intent": {"kind": "set_state", "state": "completed"}
    });
    let decoded: TaskNodeEditRequest =
        serde_json::from_value(default_closed).expect("default preserve");
    assert!(matches!(
        decoded.intent,
        TaskNodeEditIntent::SetState {
            closed: TaskNodeClosedEdit::Preserve,
            ..
        }
    ));

    assert!(serde_json::from_value::<TaskNodePriority>(json!("urgent")).is_err());
    assert!(serde_json::from_value::<TaskNodeTemporalField>(json!("finished")).is_err());
}

fn temporal_value(
    profile: &TaskNodeProfile,
    field: TaskNodeTemporalField,
) -> Option<&TaskNodeTemporal> {
    match field {
        TaskNodeTemporalField::Created => profile.created.as_ref(),
        TaskNodeTemporalField::Start => profile.start.as_ref(),
        TaskNodeTemporalField::Scheduled => profile.scheduled.as_ref(),
        TaskNodeTemporalField::Due => profile.due.as_ref(),
        TaskNodeTemporalField::Closed => profile.closed.as_ref(),
    }
}
