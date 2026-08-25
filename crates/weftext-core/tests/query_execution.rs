use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use tempfile::{TempDir, tempdir};
use weftext_core::{
    CalendarDate, NodeId, QueryAccessScope, QueryCellValue, QueryEvaluationContext,
    QueryEvaluationContextError, QueryExecutionBinding, QueryExecutionError, QueryField, QueryPlan,
    QueryRowIdentity, QueryWorkspaceIndex, TaskNodeState, TaskNodeTemporal, TaskRowEvidence,
    TaskRowKind, TaskWorkspaceProjectionDiagnosticCode, analyze_query_source, query_result_csv,
};

const ROOT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
const ALPHA_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
const BETA_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa3";
const GRAND_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa4";
const INVALID_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa5";
const DISCLOSURE_SOURCE_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa6";
const DISCLOSURE_TARGET_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa7";
const MISSING_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa9";

#[derive(Clone, Copy)]
enum HiddenDependencyFixture {
    Open,
    Closed,
    Invalid,
    Missing,
}

#[test]
fn node_queries_apply_scope_filter_sort_projection_and_limit_deterministically() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let context = context(Some(node(ROOT_ID)));
    let plan = query_plan(
        "nodes",
        concat!(
            "from nodes as node\n",
            "scope descendants(this.node)\n",
            "where node.depth >= 1\n",
            "select node.id, node.name, node.path, node.parent_id, node.depth\n",
            "order by node.path desc\n",
            "limit 2\n",
        ),
    );

    let first = index.execute(&plan, &access, &context).expect("node query");
    let second = index
        .execute(&plan, &access, &context)
        .expect("repeat node query");
    assert_eq!(first, second);
    assert_eq!(first.total_before_limit, 4);
    assert!(first.truncated);
    assert_eq!(first.rows.len(), 2);
    assert_eq!(
        cell(&first.rows[0], QueryField::Path),
        &QueryCellValue::Text("/Invalid".to_owned())
    );
    assert_eq!(
        cell(&first.rows[1], QueryField::Path),
        &QueryCellValue::Text("/Beta".to_owned())
    );
    assert!(matches!(
        first.rows[0].identity,
        QueryRowIdentity::Node { .. }
    ));
}

#[test]
fn permission_filtering_precedes_counts_groups_and_scope_errors_do_not_disclose() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let filtered = QueryAccessScope::filtered([node(ROOT_ID), node(ALPHA_ID)]);
    let workspace = query_plan(
        "nodes",
        concat!(
            "from nodes as node\n",
            "scope workspace\n",
            "where node.depth >= 0\n",
            "group by node.depth\n",
            "select node.id, node.path\n",
            "order by node.path asc\n",
            "limit 100\n",
        ),
    );
    let result = index
        .execute(&workspace, &filtered, &context(Some(node(ROOT_ID))))
        .expect("permission-filtered query");
    assert_eq!(result.total_before_limit, 2);
    assert_eq!(
        result
            .groups
            .iter()
            .map(|group| (&group.value, group.row_count))
            .collect::<Vec<_>>(),
        [
            (&QueryCellValue::Integer(0), 1),
            (&QueryCellValue::Integer(1), 1),
        ]
    );

    let scoped = query_plan(
        "nodes",
        concat!(
            "from nodes as node\n",
            "scope subtree(this.node)\n",
            "where node.depth >= 0\n",
            "select node.id\n",
            "order by node.id asc\n",
            "limit 100\n",
        ),
    );
    assert_eq!(
        index.execute(&scoped, &filtered, &context(Some(node(BETA_ID)))),
        Err(QueryExecutionError::UnavailableScope)
    );
    assert_eq!(
        index.execute(&scoped, &filtered, &context(Some(node(MISSING_ID)))),
        Err(QueryExecutionError::UnavailableScope)
    );

    let complete = QueryAccessScope::complete(index.node_ids());
    assert_eq!(
        index.execute(&scoped, &complete, &context(Some(node(MISSING_ID)))),
        Err(QueryExecutionError::MissingScopeNode(node(MISSING_ID)))
    );
    assert_eq!(
        index.execute(&scoped, &complete, &context(None)),
        Err(QueryExecutionError::MissingContext("this.node"))
    );
}

#[test]
fn task_scopes_and_filtered_access_use_owner_node_identity() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let complete = QueryAccessScope::complete(index.node_ids());

    for (scope, current, expected) in [
        ("scope descendants(this.node)", node(ROOT_ID), 4_usize),
        ("scope subtree(this.node)", node(ALPHA_ID), 3),
    ] {
        let plan = query_plan(
            "tasks",
            &format!(
                "from tasks as task\n{scope}\nwhere task.id is null or task.id is not null\nselect task.kind, task.owner_node.id, task.title\norder by task.title asc\nlimit 100\n"
            ),
        );
        let result = index
            .execute(&plan, &complete, &context(Some(current)))
            .expect("owner-scoped task query");
        assert_eq!(result.rows.len(), expected, "{scope}");
    }

    let filtered = QueryAccessScope::filtered([node(ROOT_ID), node(ALPHA_ID)]);
    let filtered_result = index
        .execute(
            &query_plan(
                "tasks",
                concat!(
                    "from tasks as task\n",
                    "scope workspace\n",
                    "where task.id is null or task.id is not null\n",
                    "select task.owner_node.id, task.owner_node.path, task.title\n",
                    "order by task.owner_node.path asc\n",
                    "limit 100\n",
                ),
            ),
            &filtered,
            &context(None),
        )
        .expect("filtered task query");
    assert_eq!(filtered_result.rows.len(), 4);
    assert!(filtered_result.rows.iter().all(|row| {
        matches!(
            cell(row, QueryField::OwnerNodeId),
            QueryCellValue::Uuid(value) if value == ROOT_ID || value == ALPHA_ID
        )
    }));
}

#[test]
fn task_queries_use_effective_values_and_explicit_offset_date_comparison() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let plan = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.closed = false\n",
            "  and task.due is not null\n",
            "  and task.due <= context.today + P14D\n",
            "group by task.state as group_state\n",
            "select task.kind, task.id as task_id, task.owner_node.id as owner_node_id, task.title, task.closed, task.state, task.priority, task.due\n",
            "order by task.due asc nulls last, task.priority desc\n",
            "limit 100\n",
        ),
    );
    let result = index
        .execute(&plan, &access, &context(Some(node(ROOT_ID))))
        .expect("task query");

    assert_eq!(result.total_before_limit, 2);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result
            .groups
            .iter()
            .map(|group| (&group.value, group.row_count))
            .collect::<Vec<_>>(),
        [
            (&QueryCellValue::TaskState(TaskNodeState::Todo), 1),
            (&QueryCellValue::TaskState(TaskNodeState::InProgress), 1),
        ]
    );
    let descriptions = result
        .rows
        .iter()
        .map(|row| cell(row, QueryField::Title))
        .collect::<Vec<_>>();
    assert!(descriptions.contains(&&QueryCellValue::Text("Soon high".to_owned())));
    assert!(descriptions.contains(&&QueryCellValue::Text("UTC included".to_owned())));
    assert!(!descriptions.contains(&&QueryCellValue::Text("UTC excluded".to_owned())));
    assert!(
        result
            .rows
            .iter()
            .all(|row| matches!(row.identity, QueryRowIdentity::Task { .. }))
    );
}

#[test]
fn null_comparisons_error_unless_boolean_short_circuit_guards_them() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let unguarded = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.due <= date(\"2026-09-30\")\n",
            "select task.title\n",
            "order by task.title asc\n",
            "limit 100\n",
        ),
    );
    assert_eq!(
        index.execute(&unguarded, &access, &context(None)),
        Err(QueryExecutionError::NullComparison)
    );

    let guarded = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.due is not null and task.due <= date(\"2026-09-30\")\n",
            "select task.title\n",
            "order by task.title asc\n",
            "limit 100\n",
        ),
    );
    assert_eq!(
        index
            .execute(&guarded, &access, &context(None))
            .expect("null guard")
            .rows
            .len(),
        3
    );

    let propagated = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where false or not task.due <= date(\"2026-09-30\")\n",
            "select task.title\n",
            "order by task.title asc\n",
            "limit 100\n",
        ),
    );
    assert_eq!(
        index.execute(&propagated, &access, &context(None)),
        Err(QueryExecutionError::NullComparison)
    );

    let membership = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.due in [null, date(\"2026-08-25\")]\n",
            "select task.title\n",
            "order by task.title asc\n",
            "limit 100\n",
        ),
    );
    let membership = index
        .execute(&membership, &access, &context(None))
        .expect("null in-list values never match and null left operands are false");
    assert_eq!(membership.rows.len(), 1);
    assert_eq!(
        cell(&membership.rows[0], QueryField::Title),
        &QueryCellValue::Text("Soon high".to_owned())
    );
}

#[test]
fn group_counts_cover_authorized_matches_before_row_limit_and_forged_plans_fail() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let mut plan = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.id is null or task.id is not null\n",
            "group by task.state as group_state\n",
            "select task.title, task.state, task.priority\n",
            "order by task.priority desc\n",
            "limit 2\n",
        ),
    );
    let result = index
        .execute(&plan, &access, &context(None))
        .expect("grouped task query");
    assert_eq!(result.total_before_limit, 6);
    assert_eq!(result.rows.len(), 2);
    assert!(result.truncated);
    assert_eq!(
        result
            .groups
            .iter()
            .map(|group| group.row_count)
            .sum::<usize>(),
        6
    );
    assert_eq!(
        result.groups.last().expect("null group").value,
        QueryCellValue::TaskState(TaskNodeState::Completed)
    );
    assert!(result.groups.iter().all(|group| {
        group.column.output_name == "group_state" && group.column.path == "state"
    }));
    assert_eq!(
        result
            .groups
            .iter()
            .map(|group| &group.value)
            .collect::<Vec<_>>(),
        [
            &QueryCellValue::TaskState(TaskNodeState::Todo),
            &QueryCellValue::TaskState(TaskNodeState::InProgress),
            &QueryCellValue::TaskState(TaskNodeState::Completed),
        ]
    );

    let mut forged_output = plan.clone();
    forged_output.projection[0].output_name = "forged_title".to_owned();
    assert_eq!(
        index.execute(&forged_output, &access, &context(None)),
        Err(QueryExecutionError::InvalidPlan)
    );

    plan.limit = 0;
    assert_eq!(
        index.execute(&plan, &access, &context(None)),
        Err(QueryExecutionError::InvalidPlan)
    );

    let mut invalid_context = context(None);
    invalid_context.today = CalendarDate {
        year: 0,
        month: 0,
        day: 0,
    };
    plan.limit = 2;
    assert_eq!(
        index.execute(&plan, &access, &invalid_context),
        Err(QueryExecutionError::InvalidContext)
    );
}

#[test]
fn source_and_nested_result_strings_share_the_four_kibibyte_execution_limit() {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    write_node(
        &root,
        "Workspace",
        ROOT_ID,
        &format!(
            "= {}\n",
            "x".repeat(weftext_core::QUERY_MAX_STRING_LITERAL_BYTES + 1)
        ),
    );
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let plan = query_plan(
        "nodes",
        concat!(
            "from nodes as node\n",
            "scope workspace\n",
            "where true\n",
            "select node.document.title\n",
            "order by node.path asc\n",
            "limit 10\n",
        ),
    );
    assert_eq!(
        index.execute(&plan, &access, &context(None)),
        Err(QueryExecutionError::ResourceLimit)
    );
}

#[test]
fn evaluation_context_rejects_invalid_dates_instants_and_non_iana_timezones() {
    let binding = || QueryExecutionBinding {
        node_id: None,
        heading: None,
    };
    assert_eq!(
        QueryEvaluationContext::new(
            CalendarDate {
                year: 0,
                month: 1,
                day: 1
            },
            instant(),
            "Asia/Shanghai".to_owned(),
            "zh-CN".to_owned(),
            binding(),
        ),
        Err(QueryEvaluationContextError::InvalidToday)
    );
    assert_eq!(
        QueryEvaluationContext::new(
            CalendarDate::new(2026, 8, 24).expect("valid date"),
            TaskNodeTemporal::Date("2026-08-24".to_owned()),
            "Asia/Shanghai".to_owned(),
            "zh-CN".to_owned(),
            binding(),
        ),
        Err(QueryEvaluationContextError::InvalidNow)
    );
    assert_eq!(
        QueryEvaluationContext::new(
            CalendarDate::new(2026, 8, 24).expect("valid date"),
            instant(),
            "Mars/Olympus_Mons".to_owned(),
            "zh-CN".to_owned(),
            binding(),
        ),
        Err(QueryEvaluationContextError::InvalidTimezone)
    );
    assert!(
        serde_json::from_value::<QueryEvaluationContext>(serde_json::json!({
            "today": {"year": 2026, "month": 8, "day": 24, "extra": true},
            "now": "2026-08-24T09:30:00+08:00",
            "timezone": "Asia/Shanghai",
            "locale": "zh-CN",
            "binding": {"nodeId": null, "heading": null}
        }))
        .is_err(),
        "query today must reject unknown nested fields"
    );
    let context_wire = |node_id: &str| {
        serde_json::json!({
            "today": {"year": 2026, "month": 8, "day": 24},
            "now": "2026-08-24T09:30:00+08:00",
            "timezone": "Asia/Shanghai",
            "locale": "zh-CN",
            "binding": {"nodeId": node_id, "heading": null}
        })
    };
    assert!(serde_json::from_value::<QueryEvaluationContext>(context_wire(ROOT_ID)).is_ok());
    for invalid in [
        ROOT_ID.to_uppercase(),
        ROOT_ID.replace('-', ""),
        "aaaaaaaa-aaaa-1aaa-8aaa-aaaaaaaaaaa1".to_owned(),
    ] {
        assert!(
            serde_json::from_value::<QueryEvaluationContext>(context_wire(&invalid)).is_err(),
            "query binding accepted noncanonical node ID {invalid}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn canonical_task_rows_have_tagged_identity_nullability_defaults_csv_and_tie_breaks() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());

    let defaults = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.id is null or task.id is not null\n",
            "select task.kind, task.id as task_id, task.owner_node.id as owner_node_id, task.title, task.closed, task.state, task.priority, task.due\n",
            "order by task.owner_node.path asc\n",
            "limit 100\n",
        ),
    );
    let result = index
        .execute(&defaults, &access, &context(Some(node(ROOT_ID))))
        .expect("default tasks query");
    assert_eq!(
        result
            .columns
            .iter()
            .map(|column| column.path.as_str())
            .collect::<Vec<_>>(),
        [
            "kind",
            "id",
            "owner_node.id",
            "title",
            "closed",
            "state",
            "priority",
            "due",
        ]
    );
    assert_eq!(result.total_before_limit, 6);

    let full = index
        .execute(
            &query_plan(
                "tasks",
                concat!(
                    "from tasks as task\n",
                    "scope workspace\n",
                    "where task.id is null or task.id is not null\n",
                    "select task.kind, task.id as task_id, task.owner_node.id as owner_node_id, task.owner_node.name, task.owner_node.path, ",
                    "task.title, task.closed, task.state, task.checklist_depth, task.priority, task.created, task.start, ",
                    "task.scheduled, task.due, task.closed_at, task.blocked\n",
                    "order by task.owner_node.path asc\n",
                    "limit 100\n",
                ),
            ),
            &access,
            &context(None),
        )
        .expect("complete canonical task field projection");
    let star = full
        .rows
        .iter()
        .find(|row| cell(row, QueryField::Title) == &QueryCellValue::Text("Star closed".to_owned()))
        .expect("star checklist row");
    assert_eq!(
        cell(star, QueryField::Kind),
        &QueryCellValue::TaskKind(TaskRowKind::Checklist)
    );
    assert_eq!(cell(star, QueryField::Id), &QueryCellValue::Null);
    assert_eq!(
        cell(star, QueryField::ChecklistDepth),
        &QueryCellValue::Integer(1)
    );
    assert_eq!(cell(star, QueryField::Priority), &QueryCellValue::Null);
    assert_eq!(cell(star, QueryField::Created), &QueryCellValue::Null);
    assert_eq!(cell(star, QueryField::Start), &QueryCellValue::Null);
    assert_eq!(cell(star, QueryField::Scheduled), &QueryCellValue::Null);
    assert_eq!(cell(star, QueryField::Due), &QueryCellValue::Null);
    assert_eq!(cell(star, QueryField::ClosedAt), &QueryCellValue::Null);
    assert_eq!(cell(star, QueryField::Blocked), &QueryCellValue::Null);
    let QueryRowIdentity::Task {
        evidence: TaskRowEvidence::Checklist {
            authored_marker, ..
        },
    } = &star.identity
    else {
        panic!("checklist identity must retain action evidence");
    };
    assert_eq!(*authored_marker, weftext_core::ChecklistMarker::CheckedStar);

    let node_row = full
        .rows
        .iter()
        .find(|row| cell(row, QueryField::Title) == &QueryCellValue::Text("Soon high".to_owned()))
        .expect("task-node row");
    assert_eq!(
        cell(node_row, QueryField::Kind),
        &QueryCellValue::TaskKind(TaskRowKind::Node)
    );
    assert_eq!(
        cell(node_row, QueryField::Id),
        &QueryCellValue::Uuid(ALPHA_ID.to_owned())
    );
    assert_eq!(
        cell(node_row, QueryField::OwnerNodeId),
        &QueryCellValue::Uuid(ALPHA_ID.to_owned())
    );
    assert_eq!(
        cell(node_row, QueryField::Priority),
        &QueryCellValue::Priority(weftext_core::TaskNodePriority::High)
    );
    assert_eq!(
        cell(node_row, QueryField::ChecklistDepth),
        &QueryCellValue::Null
    );
    assert_eq!(
        cell(node_row, QueryField::Blocked),
        &QueryCellValue::Boolean(false)
    );
    let QueryRowIdentity::Task {
        evidence:
            TaskRowEvidence::Node {
                revision: task_revision,
                profile_revision,
                ..
            },
    } = &node_row.identity
    else {
        panic!("task-node identity must retain revision-bound action evidence");
    };
    assert_eq!(task_revision, profile_revision);

    let node_projection = index
        .execute(
            &query_plan(
                "nodes",
                &format!(
                    concat!(
                        "from nodes as node\n",
                        "scope workspace\n",
                        "where node.id = uuid(\"{}\")\n",
                        "select node.id, node.name, node.path\n",
                        "order by node.path asc\n",
                        "limit 100\n",
                    ),
                    ALPHA_ID
                ),
            ),
            &access,
            &context(None),
        )
        .expect("node projection sharing the parsed document revision");
    let QueryRowIdentity::Node {
        revision: node_revision,
        ..
    } = &node_projection.rows[0].identity
    else {
        panic!("nodes source must retain document identity");
    };
    assert_eq!(node_revision, task_revision);

    let tied = query_plan(
        "tasks",
        &format!(
            concat!(
                "from tasks as task\n",
                "scope workspace\n",
                "where task.owner_node.id = uuid(\"{}\")\n",
                "select task.kind, task.title\n",
                "order by task.closed asc\n",
                "limit 100\n",
            ),
            ALPHA_ID
        ),
    );
    let tied = index
        .execute(&tied, &access, &context(None))
        .expect("canonical tie-break");
    assert_eq!(tied.rows.len(), 2);
    assert_eq!(
        cell(&tied.rows[0], QueryField::Kind),
        &QueryCellValue::TaskKind(TaskRowKind::Checklist)
    );
    assert_eq!(
        cell(&tied.rows[1], QueryField::Kind),
        &QueryCellValue::TaskKind(TaskRowKind::Node)
    );

    let csv_plan = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope workspace\n",
            "where task.kind = \"node\"\n",
            "select task.kind, task.id, task.owner_node.name, task.owner_node.path, task.state, task.priority, task.blocked\n",
            "order by task.owner_node.path asc\n",
            "limit 100\n",
        ),
    );
    let csv_result = index.execute(&csv_plan, &access, &context(None)).unwrap();
    let csv = query_result_csv(&csv_result);
    assert!(csv.starts_with("kind,id,name,path,state,priority,blocked\r\n"));
    assert!(
        csv.contains(
            "node,aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2,Alpha,/Alpha,in-progress,high,false"
        )
    );
}

#[test]
fn projection_column_identity_preserves_nested_paths_and_property_keys() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let plan = query_plan(
        "nodes",
        concat!(
            "from nodes as node\n",
            "scope workspace\n",
            "where node.id = uuid(\"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1\")\n",
            "select node.id, node.document.properties[\"project-code\"] as project_code, node.document.properties[\"missing\"] as missing_property\n",
            "order by node.id asc\n",
            "limit 10\n",
        ),
    );
    let result = index
        .execute(&plan, &access, &context(None))
        .expect("properties");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result
            .columns
            .iter()
            .map(|column| {
                (
                    column.output_name.as_str(),
                    column.path.as_str(),
                    column.property_key.as_deref(),
                )
            })
            .collect::<Vec<_>>(),
        [
            ("id", "id", None),
            (
                "project_code",
                "document.properties[\"project-code\"]",
                Some("project-code")
            ),
            (
                "missing_property",
                "document.properties[\"missing\"]",
                Some("missing")
            ),
        ]
    );
    assert_eq!(
        result.rows[0].cells[1].value,
        QueryCellValue::Text("root".to_owned())
    );
    assert_eq!(result.rows[0].cells[2].value, QueryCellValue::Null);
    assert!(query_result_csv(&result).starts_with("id,project_code,missing_property\r\n"));
}

#[test]
fn invalid_task_profile_remains_a_node_and_exposes_canonical_diagnostics_only() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let nodes = index
        .execute(
            &query_plan(
                "nodes",
                &format!(
                    concat!(
                        "from nodes as node\n",
                        "scope workspace\n",
                        "where node.id = uuid(\"{}\")\n",
                        "select node.id, node.name, node.path\n",
                        "order by node.path asc\n",
                        "limit 100\n",
                    ),
                    INVALID_ID
                ),
            ),
            &access,
            &context(None),
        )
        .expect("invalid profile node remains visible");
    assert_eq!(nodes.rows.len(), 1);
    let tasks = index
        .execute(
            &query_plan(
                "tasks",
                &format!(
                    concat!(
                        "from tasks as task\n",
                        "scope workspace\n",
                        "where task.id is not null and task.id = uuid(\"{}\")\n",
                        "select task.id, task.title\n",
                        "order by task.title asc\n",
                        "limit 100\n",
                    ),
                    INVALID_ID
                ),
            ),
            &access,
            &context(None),
        )
        .expect("invalid profile is absent from tasks");
    assert!(tasks.rows.is_empty());
    assert!(index.task_diagnostics(&access).iter().any(|diagnostic| {
        diagnostic.node_id == node(INVALID_ID)
            && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile
    }));
}

#[test]
fn filtered_access_rederives_task_graph_without_hidden_dependency_disclosure() {
    let hidden_observations = [
        HiddenDependencyFixture::Open,
        HiddenDependencyFixture::Closed,
        HiddenDependencyFixture::Invalid,
        HiddenDependencyFixture::Missing,
    ]
    .map(|fixture| filtered_dependency_observation(fixture, false));

    for (observation, blocked) in &hidden_observations {
        assert_eq!(blocked, &QueryCellValue::Boolean(false));
        assert_eq!(observation, &hidden_observations[0].0);
        assert!(!observation.contains(DISCLOSURE_TARGET_ID));
        assert!(!observation.contains("Hidden target"));
    }

    let (_, visible_open_blocked) =
        filtered_dependency_observation(HiddenDependencyFixture::Open, true);
    assert_eq!(visible_open_blocked, QueryCellValue::Boolean(true));
    let (_, visible_closed_blocked) =
        filtered_dependency_observation(HiddenDependencyFixture::Closed, true);
    assert_eq!(visible_closed_blocked, QueryCellValue::Boolean(false));
}

#[test]
fn every_this_reference_requires_an_explicit_owning_node_binding() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let source = concat!(
        ".Workspace\n",
        "[.weftext-query,version=1]\n",
        "....\n",
        "from nodes as node\n",
        "scope workspace\n",
        "where node.name = this.query.title\n",
        "select node.id, node.document.title\n",
        "order by node.id asc\n",
        "limit 10\n",
        "....\n",
    );
    let plan = analyze_query_source(source).blocks[0]
        .plan
        .clone()
        .expect("typed embedded plan");
    assert_eq!(
        index.execute(&plan, &access, &context(None)),
        Err(QueryExecutionError::MissingContext("this.node"))
    );
    assert_eq!(
        index.execute_source(source, 0, &access, &context(None)),
        Err(QueryExecutionError::MissingContext("this.node"))
    );
    let result = index
        .execute_source(source, 0, &access, &context(Some(node(ROOT_ID))))
        .expect("bound embedded query")
        .result
        .expect("query result");
    assert_eq!(result.rows.len(), 1);

    let section = query_plan(
        "tasks",
        concat!(
            "from tasks as task\n",
            "scope section(this.heading)\n",
            "where true\n",
            "select task.title\n",
            "order by task.title asc\n",
            "limit 10\n",
        ),
    );
    assert_eq!(
        index.execute(&section, &access, &context(Some(node(ROOT_ID)))),
        Err(QueryExecutionError::MissingHeadingContext)
    );
}

#[test]
fn heading_rows_expose_the_closed_owning_node_record_and_templates_stay_unavailable() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let headings = query_plan(
        "headings",
        concat!(
            "from headings as heading\n",
            "scope workspace\n",
            "where heading.owning_node.id = uuid(\"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2\")\n",
            "select heading.title as heading_title, heading.level, heading.owning_node.id as owning_node_id, ",
            "heading.owning_node.name, heading.owning_node.path as owning_node_path, ",
            "heading.owning_node.parent_id, heading.owning_node.depth, ",
            "heading.owning_node.display_title, heading.owning_node.document.title as owning_document_title, ",
            "heading.document.title as document_title, heading.document.subtitle, ",
            "heading.document.display_title as document_display_title, ",
            "heading.document.properties[\"状态\"] as document_status\n",
            "order by heading.level asc\n",
            "limit 100\n",
        ),
    );
    let result = index
        .execute(&headings, &access, &context(None))
        .expect("heading rows");
    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result
            .columns
            .iter()
            .map(|column| (column.output_name.as_str(), column.path.as_str()))
            .collect::<Vec<_>>(),
        [
            ("heading_title", "title"),
            ("level", "level"),
            ("owning_node_id", "owning_node.id"),
            ("name", "owning_node.name"),
            ("owning_node_path", "owning_node.path"),
            ("parent_id", "owning_node.parent_id"),
            ("depth", "owning_node.depth"),
            ("display_title", "owning_node.display_title"),
            ("owning_document_title", "owning_node.document.title"),
            ("document_title", "document.title"),
            ("subtitle", "document.subtitle"),
            ("document_display_title", "document.display_title"),
            ("document_status", "document.properties[\"状态\"]"),
        ]
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::OwnerNodeParentId),
        &QueryCellValue::Uuid(ROOT_ID.to_owned())
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::OwnerNodeDepth),
        &QueryCellValue::Integer(1)
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentTitle),
        &QueryCellValue::Text("Soon high".to_owned())
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentSubtitle),
        &QueryCellValue::Null
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentProperty),
        &QueryCellValue::Null
    );

    let templates = query_plan(
        "templates",
        concat!(
            "from templates as template\n",
            "scope workspace\n",
            "where true\n",
            "select template.id, template.name, template.path, template.display_title, template.part_count, template.parameter_count\n",
            "order by template.path asc\n",
            "limit 100\n",
        ),
    );
    assert_eq!(
        index.execute(&templates, &access, &context(None)),
        Err(QueryExecutionError::DomainUnavailable(
            weftext_core::QuerySource::Templates
        ))
    );
}

#[test]
fn heading_document_preserves_null_authored_title_and_subtitle_with_derived_display_title() {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    write_node(&root, "Workspace", ROOT_ID, "= Workspace\n");
    write_node(
        &root.join("NoTitle"),
        "NoTitle",
        ALPHA_ID,
        ":status: 待办\n\n== Body heading\n",
    );
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let plan = query_plan(
        "headings",
        concat!(
            "from headings as heading\n",
            "scope workspace\n",
            "where heading.owning_node.id = uuid(\"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2\")\n",
            "select heading.document.title as document_title, heading.document.subtitle as document_subtitle, ",
            "heading.document.display_title as document_display_title, ",
            "heading.document.properties[\"status\"] as document_status\n",
            "order by heading.title asc\n",
            "limit 10\n",
        ),
    );
    let result = index
        .execute(&plan, &access, &context(None))
        .expect("heading document projection");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentTitle),
        &QueryCellValue::Null
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentSubtitle),
        &QueryCellValue::Null
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentDisplayTitle),
        &QueryCellValue::Text("NoTitle".to_owned())
    );
    assert_eq!(
        cell(&result.rows[0], QueryField::HeadingDocumentProperty),
        &QueryCellValue::Text("待办".to_owned())
    );
}

#[test]
fn exact_source_execution_keeps_block_selection_and_invalid_diagnostics_in_core() {
    let (_temporary, root) = query_workspace();
    let index = QueryWorkspaceIndex::rebuild(&root).expect("query workspace index");
    let access = QueryAccessScope::complete(index.node_ids());
    let source = concat!(
        "= Views\n\n",
        "[.weftext-query,version=1,view=table]\n",
        "....\n",
        "from nodes as node\n",
        "scope workspace\n",
        "where true\n",
        "select node.name, node.path\n",
        "order by node.path asc\n",
        "limit 2\n",
        "....\n\n",
        "[.weftext-query,version=1,view=task-list]\n",
        "....\n",
        "from tasks as task\n",
        "scope workspace\n",
        "where task.unknown = true\n",
        "select task.id\n",
        "order by task.id asc\n",
        "limit 100\n",
        "....\n",
    );

    let executed = index
        .execute_source(source, 0, &access, &context(Some(node(ROOT_ID))))
        .expect("valid selected query");
    assert_eq!(executed.block_index, 0);
    assert_eq!(executed.analysis.blocks.len(), 2);
    assert_eq!(
        executed.result.as_ref().expect("query result").rows.len(),
        2
    );
    assert_eq!(
        executed.csv.as_deref(),
        Some("name,path\r\nWorkspace,/\r\nAlpha,/Alpha\r\n")
    );

    let invalid = index
        .execute_source(source, 1, &access, &context(Some(node(ROOT_ID))))
        .expect("invalid source returns diagnostics");
    assert!(invalid.result.is_none());
    assert!(!invalid.analysis.diagnostics.is_empty());

    let missing = index
        .execute_source(source, 9, &access, &context(Some(node(ROOT_ID))))
        .expect("missing block is not an execution failure");
    assert!(missing.result.is_none());
    assert_eq!(missing.analysis.blocks.len(), 2);

    let retired = index
        .execute_source(
            "[query,source=nodes]\n....\nscope workspace\n....\n",
            0,
            &access,
            &context(Some(node(ROOT_ID))),
        )
        .expect("retired outer syntax is inert");
    assert!(retired.analysis.blocks.is_empty());
    assert!(retired.result.is_none());
}

fn query_workspace() -> (TempDir, PathBuf) {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    write_node(
        &root,
        "Workspace",
        ROOT_ID,
        concat!(
            "= Workspace\n:project-code: root\n\n",
            "* [ ] Simple open\n",
            "* [*] Star closed\n",
        ),
    );
    write_node(
        &root.join("Alpha"),
        "Alpha",
        ALPHA_ID,
        concat!(
            "= Soon high\n",
            ":weftext-task: v1\n",
            ":weftext-task-state: in-progress\n",
            ":weftext-task-priority: high\n",
            ":weftext-task-due: 2026-08-25\n\n",
            "* [ ] Alpha checklist\n",
            "\n== Phase A\n\n",
            "=== Detail\n",
        ),
    );
    write_node(
        &root.join("Beta"),
        "Beta",
        BETA_ID,
        concat!(
            "= UTC included\n",
            ":weftext-task: v1\n",
            ":weftext-task-state: todo\n",
            ":weftext-task-due: 2026-09-07T15:00:00Z\n",
        ),
    );
    write_node(
        &root.join("Alpha").join("Grand"),
        "Grand",
        GRAND_ID,
        concat!(
            "= UTC excluded\n",
            ":weftext-task: v1\n",
            ":weftext-task-state: todo\n",
            ":weftext-task-due: 2026-09-07T23:30:00Z\n",
        ),
    );
    write_node(
        &root.join("Invalid"),
        "Invalid",
        INVALID_ID,
        "= Invalid\n:weftext-task: v1\n:weftext-task-state: waiting\n",
    );
    (temporary, root)
}

fn filtered_dependency_observation(
    fixture: HiddenDependencyFixture,
    include_target: bool,
) -> (String, QueryCellValue) {
    let temporary = tempdir().expect("temporary disclosure workspace parent");
    let root = temporary.path().join("Workspace");
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    write_node(&root, "Workspace", ROOT_ID, "= Workspace\n");
    write_node(
        &root.join("Visible"),
        "Visible",
        DISCLOSURE_SOURCE_ID,
        &format!(
            concat!(
                "= Visible source\n",
                ":weftext-task: v1\n",
                ":weftext-task-state: todo\n",
                ":weftext-task-depends-on: {}\n",
            ),
            DISCLOSURE_TARGET_ID
        ),
    );
    if !matches!(fixture, HiddenDependencyFixture::Missing) {
        let state = match fixture {
            HiddenDependencyFixture::Open => "todo",
            HiddenDependencyFixture::Closed => "completed",
            HiddenDependencyFixture::Invalid => "waiting",
            HiddenDependencyFixture::Missing => unreachable!(),
        };
        write_node(
            &root.join("Hidden"),
            "Hidden",
            DISCLOSURE_TARGET_ID,
            &format!("= Hidden target\n:weftext-task: v1\n:weftext-task-state: {state}\n"),
        );
    }

    let index = QueryWorkspaceIndex::rebuild(&root).expect("complete query index");
    let mut authorized = vec![node(ROOT_ID), node(DISCLOSURE_SOURCE_ID)];
    if include_target {
        authorized.push(node(DISCLOSURE_TARGET_ID));
    }
    let access = QueryAccessScope::filtered(authorized);
    let result = index
        .execute(
            &query_plan(
                "tasks",
                &format!(
                    concat!(
                        "from tasks as task\n",
                        "scope workspace\n",
                        "where task.id = uuid(\"{}\")\n",
                        "select task.kind, task.id, task.title, task.blocked\n",
                        "order by task.id asc\n",
                        "limit 100\n",
                    ),
                    DISCLOSURE_SOURCE_ID,
                ),
            ),
            &access,
            &context(None),
        )
        .expect("access-derived task query");
    assert_eq!(result.rows.len(), 1);
    let blocked = cell(&result.rows[0], QueryField::Blocked).clone();
    let diagnostics = index.task_diagnostics(&access);
    if !include_target {
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            TaskWorkspaceProjectionDiagnosticCode::UnresolvedDependency
        );
        assert_eq!(diagnostics[0].dependency_id, None);
        assert!(diagnostics[0].related_node_ids.is_empty());
    }
    let observation =
        serde_json::to_string(&(result, diagnostics)).expect("serialized authorized observation");
    (observation, blocked)
}

fn write_node(directory: &Path, name: &str, node_id: &str, body: &str) {
    fs::create_dir_all(directory).expect("node directory");
    let source = format!("---\nweftext:\n  id: \"{node_id}\"\n---\n{body}");
    fs::write(directory.join(format!("{name}.adoc")), source).expect("node source");
}

fn query_plan(source: &str, body: &str) -> QueryPlan {
    let document = format!("[.weftext-query,version=1]\n....\n{body}....");
    let analysis = analyze_query_source(&document);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let plan = analysis.blocks[0].plan.clone().expect("typed query plan");
    assert_eq!(format!("{:?}", plan.source).to_ascii_lowercase(), source);
    plan
}

fn context(current_node_id: Option<NodeId>) -> QueryEvaluationContext {
    QueryEvaluationContext::new(
        CalendarDate::new(2026, 8, 24).expect("query date"),
        instant(),
        "Asia/Shanghai".to_owned(),
        "zh-CN".to_owned(),
        QueryExecutionBinding {
            node_id: current_node_id,
            heading: None,
        },
    )
    .expect("query context")
}

fn instant() -> TaskNodeTemporal {
    TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("query instant")
}

fn node(value: &str) -> NodeId {
    NodeId::from_str(value).expect("valid node ID")
}

fn cell(row: &weftext_core::QueryResultRow, field: QueryField) -> &QueryCellValue {
    &row.cells
        .iter()
        .find(|cell| cell.column.field == field)
        .expect("projected cell")
        .value
}
