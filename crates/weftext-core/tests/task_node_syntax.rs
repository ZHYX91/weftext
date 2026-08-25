use std::str::FromStr;

use serde_json::json;
use weftext_core::{
    DocumentPropertyPatchError, DocumentRevision, NodeId, TaskNodeDiagnosticCode, TaskNodePriority,
    TaskNodeState, TaskNodeTemporal, analyze_document_header_properties, analyze_task_node_profile,
    patch_document_header_property,
};

const ALL_FIELDS: &str = include_str!("../../../tests/fixtures/task-node-v1/valid/all-fields.adoc");
const RESERVED_RESIDUE: &str =
    include_str!("../../../tests/fixtures/task-node-v1/invalid/reserved-residue.adoc");
const OWN_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn codes(source: &str, node_id: Option<NodeId>) -> Vec<TaskNodeDiagnosticCode> {
    analyze_task_node_profile(source, node_id)
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn task_source(state: &str, extra: &str) -> String {
    format!("= Task\n:weftext-task: v1\n:weftext-task-state: {state}\n{extra}\nBody\n")
}

#[test]
fn valid_all_fields_profile_has_exact_evidence_revision_and_schema_shape() {
    let analysis =
        analyze_task_node_profile(ALL_FIELDS, Some(NodeId::from_str(OWN_ID).expect("node ID")));
    assert!(analysis.declared);
    assert!(analysis.has_reserved_evidence);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:#?}",
        analysis.diagnostics
    );
    assert_eq!(
        analysis.profile_revision,
        DocumentRevision::from_source(ALL_FIELDS)
    );
    assert_eq!(analysis.attributes.len(), 9);
    for attribute in &analysis.attributes {
        let start = usize::try_from(attribute.range.start).expect("range start");
        let end = usize::try_from(attribute.range.end).expect("range end");
        assert!(ALL_FIELDS.is_char_boundary(start));
        assert!(ALL_FIELDS.is_char_boundary(end));
        assert!(ALL_FIELDS[start..end].starts_with(':'));
    }
    let title = analysis.title.as_ref().expect("task-node title");
    assert_eq!(title.title, "发布 Weftext 😀");
    assert_eq!(
        &ALL_FIELDS[usize::try_from(title.text_range.start).unwrap()
            ..usize::try_from(title.text_range.end).unwrap()],
        "发布 Weftext 😀"
    );

    let profile = analysis.profile.as_ref().expect("valid task profile");
    assert_eq!(profile.state, TaskNodeState::Completed);
    assert_eq!(profile.priority, Some(TaskNodePriority::Highest));
    assert_eq!(profile.effective_priority(), TaskNodePriority::Highest);
    assert!(matches!(profile.created, Some(TaskNodeTemporal::Date(_))));
    assert!(matches!(profile.start, Some(TaskNodeTemporal::Instant(_))));
    assert_eq!(profile.depends_on.len(), 2);
    assert_eq!(
        serde_json::to_value(profile).expect("serialized profile"),
        json!({
            "profile": "v1",
            "state": "completed",
            "priority": "highest",
            "created": "2026-08-25",
            "start": "2026-08-25T01:02:03Z",
            "scheduled": "2026-08-26T09:30:00+08:00",
            "due": "2028-02-29",
            "closed": "2026-08-27T10:11:12.123-05:30",
            "depends-on": "9b74c989-7bac-472f-9a8f-01f0db9f7a10 7a1d2054-11f0-4a47-9876-001122334455"
        })
    );
}

#[test]
fn absent_authored_priority_has_normal_effective_rank() {
    let source = task_source("todo", "");
    let analysis = analyze_task_node_profile(&source, None);
    let profile = analysis.profile.expect("valid profile");
    assert_eq!(profile.priority, None);
    assert_eq!(profile.effective_priority(), TaskNodePriority::Normal);
    assert!(
        TaskNodePriority::Lowest < TaskNodePriority::Low
            && TaskNodePriority::Low < TaskNodePriority::Normal
            && TaskNodePriority::Normal < TaskNodePriority::Medium
            && TaskNodePriority::Medium < TaskNodePriority::High
            && TaskNodePriority::High < TaskNodePriority::Highest
    );
}

#[test]
fn ordinary_nodes_and_body_redefinitions_do_not_declare_a_task() {
    let source = "= Note\n:status: draft\n\n:weftext-task: v1\n:weftext-task-state: todo\n";
    let analysis = analyze_task_node_profile(source, None);
    assert!(!analysis.declared);
    assert!(!analysis.has_reserved_evidence);
    assert!(analysis.profile.is_none());
    assert!(analysis.diagnostics.is_empty());
}

#[test]
fn reserved_residue_without_the_exact_marker_is_invalid_not_custom_property() {
    let analysis = analyze_task_node_profile(RESERVED_RESIDUE, None);
    assert!(!analysis.declared);
    assert!(analysis.has_reserved_evidence);
    assert!(analysis.profile.is_none());
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TaskNodeDiagnosticCode::MissingProfileMarker)
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TaskNodeDiagnosticCode::UnknownAttribute)
    );

    let properties = analyze_document_header_properties(RESERVED_RESIDUE);
    assert!(
        properties
            .properties
            .iter()
            .all(|property| !property.name.starts_with("weftext-task"))
    );
    for name in ["weftext-task", "weftext-task-state", "weftext-task-unknown"] {
        assert_eq!(
            patch_document_header_property("= T\n\n", name, Some("value")),
            Err(DocumentPropertyPatchError::InvalidName)
        );
    }
}

#[test]
fn missing_or_ambiguous_title_invalidates_only_the_task_profile() {
    let missing = ":weftext-task: v1\n:weftext-task-state: todo\n\nBody\n";
    assert!(codes(missing, None).contains(&TaskNodeDiagnosticCode::MissingDocumentTitle));
    assert!(analyze_task_node_profile(missing, None).profile.is_none());

    let additional = concat!(
        "= First\n",
        ":weftext-task: v1\n",
        ":weftext-task-state: todo\n\n",
        "= Second\n"
    );
    assert!(codes(additional, None).contains(&TaskNodeDiagnosticCode::AmbiguousDocumentTitle));
}

#[test]
fn unsupported_missing_duplicate_unknown_and_nonliteral_attributes_fail_closed() {
    let cases = [
        (
            task_source("todo", ":weftext-task: v2"),
            TaskNodeDiagnosticCode::DuplicateAttribute,
        ),
        (
            "= Task\n:weftext-task: v2\n:weftext-task-state: todo\n\nBody\n".to_owned(),
            TaskNodeDiagnosticCode::UnsupportedProfile,
        ),
        (
            "= Task\n:weftext-task: v1\n\nBody\n".to_owned(),
            TaskNodeDiagnosticCode::MissingState,
        ),
        (
            task_source("blocked", ""),
            TaskNodeDiagnosticCode::InvalidState,
        ),
        (
            task_source("todo", ":weftext-task-priority: urgent"),
            TaskNodeDiagnosticCode::InvalidPriority,
        ),
        (
            task_source("todo", ":weftext-task-rrule: FREQ=DAILY"),
            TaskNodeDiagnosticCode::UnknownAttribute,
        ),
        (
            "= Task\n:!weftext-task:\n:weftext-task-state: todo\n\nBody\n".to_owned(),
            TaskNodeDiagnosticCode::InvalidReservedAttributeSyntax,
        ),
        (
            "= Task\n:weftext-task: v1\n:weftext-task-state: todo \\\ncontinued\n\nBody\n"
                .to_owned(),
            TaskNodeDiagnosticCode::InvalidReservedAttributeSyntax,
        ),
    ];
    for (source, expected) in cases {
        let analysis = analyze_task_node_profile(&source, None);
        assert!(analysis.profile.is_none(), "{source}");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == expected),
            "missing {expected:?}: {:#?}",
            analysis.diagnostics
        );
    }
}

#[test]
fn temporal_values_are_calendar_and_explicit_offset_only() {
    for value in [
        "2026-02-29",
        "2026-08-25t01:02:03Z",
        "2026-08-25T01:02:03",
        "2026-08-25T24:00:00Z",
        "2026-08-25T01:02:60Z",
        "2026-08-25T01:02:03+24:00",
        "tomorrow",
        "{due}",
    ] {
        let source = task_source("todo", &format!(":weftext-task-due: {value}"));
        assert!(codes(&source, None).contains(&TaskNodeDiagnosticCode::InvalidTemporal));
    }
    for value in [
        "2028-02-29",
        "2026-08-25T01:02:03Z",
        "2026-08-25T01:02:03.1+08:00",
        "2026-08-25T01:02:03-00:00",
    ] {
        let source = task_source("todo", &format!(":weftext-task-due: {value}"));
        assert!(
            analyze_task_node_profile(&source, None).profile.is_some(),
            "{value}"
        );
    }
}

#[test]
fn closed_is_forbidden_on_open_states_and_optional_on_closed_states() {
    for state in ["todo", "in-progress", "on-hold"] {
        let source = task_source(state, ":weftext-task-closed: 2026-08-25");
        assert!(codes(&source, None).contains(&TaskNodeDiagnosticCode::ClosedOnOpenState));
    }
    for state in ["completed", "cancelled"] {
        assert!(
            analyze_task_node_profile(&task_source(state, ""), None)
                .profile
                .is_some()
        );
        assert!(
            analyze_task_node_profile(
                &task_source(state, ":weftext-task-closed: 2026-08-25"),
                None,
            )
            .profile
            .is_some()
        );
    }
}

#[test]
fn dependency_syntax_uniqueness_and_optional_self_check_are_exact() {
    let first = "9b74c989-7bac-472f-9a8f-01f0db9f7a10";
    for value in [
        format!("{first}  {OWN_ID}"),
        format!("{first}\t{OWN_ID}"),
        first.to_uppercase(),
        "9b74c989-7bac-172f-9a8f-01f0db9f7a10".to_owned(),
    ] {
        let source = task_source("todo", &format!(":weftext-task-depends-on: {value}"));
        assert!(codes(&source, None).contains(&TaskNodeDiagnosticCode::InvalidDependencies));
    }

    let duplicate = task_source(
        "todo",
        &format!(":weftext-task-depends-on: {first} {first}"),
    );
    assert!(codes(&duplicate, None).contains(&TaskNodeDiagnosticCode::DuplicateDependency));

    let self_edge = task_source("todo", &format!(":weftext-task-depends-on: {OWN_ID}"));
    assert!(
        analyze_task_node_profile(&self_edge, None)
            .profile
            .is_some()
    );
    assert!(
        codes(&self_edge, Some(NodeId::from_str(OWN_ID).expect("own ID")))
            .contains(&TaskNodeDiagnosticCode::SelfDependency)
    );
}
