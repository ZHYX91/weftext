use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::Value;
use tempfile::{TempDir, tempdir};
use weftext_core::{
    DocumentRevision, NodeId, TaskNodePriority, TaskNodeState, TaskNodeTemporal, TaskRowEvidence,
    TaskRowKind, TaskWorkspaceProjection, TaskWorkspaceProjectionDiagnosticCode,
};

const ROOT_ID: &str = "10000000-0000-4000-8000-000000000001";
const TASK_ID: &str = "10000000-0000-4000-8000-000000000002";
const MISSING_TITLE_ID: &str = "10000000-0000-4000-8000-000000000003";
const INVALID_ID: &str = "10000000-0000-4000-8000-000000000004";
const TRASH_ID: &str = "10000000-0000-4000-8000-000000000005";

#[test]
#[allow(clippy::too_many_lines)]
fn tagged_rows_preserve_nullability_exact_evidence_and_full_revision() {
    let (_temporary, root) = workspace("Workspace");
    write_node(
        &root,
        "Workspace",
        ROOT_ID,
        concat!(
            "= Root\n",
            ":weftext-task: v1\n",
            ":weftext-task-state: todo\n\n",
            "* [ ] root checklist\n",
        ),
    );
    let task_source = format!(
        concat!(
            "---\r\n",
            "weftext:\r\n",
            "  id: \"{task_id}\"\r\n",
            "---\r\n",
            "= 发布 🧭 مرحبا\r\n",
            ":weftext-task: v1\r\n",
            ":weftext-task-state: in-progress\r\n",
            ":weftext-task-created: 2026-08-01\r\n",
            ":weftext-task-start: 2026-08-25T09:00:00Z\r\n",
            ":weftext-task-scheduled: 2026-09-01\r\n",
            ":weftext-task-due: 2026-09-05T10:20:30+08:00\r\n",
            "\r\n",
            "* [x] 完成 مرحبا ✅\r\n",
            "** [ ] 子项 🧪"
        ),
        task_id = TASK_ID,
    );
    write_exact_node(&root.join("Task"), "Task", &task_source);
    write_node(
        &root.join("NoTitle"),
        "NoTitle",
        MISSING_TITLE_ID,
        ":weftext-task: v1\n:weftext-task-state: todo\n",
    );
    write_node(
        &root.join("Invalid"),
        "Invalid",
        INVALID_ID,
        "= Invalid\n:weftext-task: v1\n:weftext-task-state: waiting\n\n* [ ] survives invalid profile\n",
    );

    let projection = TaskWorkspaceProjection::rebuild(&root).expect("projection");
    let task_id = node(TASK_ID);
    let task_row = projection.task_node_row(task_id).expect("task-node row");
    assert_eq!(task_row.kind, TaskRowKind::Node);
    assert_eq!(task_row.id, Some(task_id));
    assert_eq!(task_row.owner_node_id, task_id);
    assert_eq!(task_row.owner_node_name, "Task");
    assert_eq!(task_row.owner_node_path, "/Task");
    assert_eq!(task_row.description, "发布 🧭 مرحبا");
    assert_eq!(task_row.state, TaskNodeState::InProgress);
    assert!(!task_row.closed);
    assert_eq!(task_row.checklist_depth, None);
    assert_eq!(task_row.priority, Some(TaskNodePriority::Normal));
    assert_eq!(task_row.blocked, Some(false));
    assert_eq!(
        task_row.created.as_ref().map(TaskNodeTemporal::as_str),
        Some("2026-08-01")
    );
    assert_eq!(
        task_row.start.as_ref().map(TaskNodeTemporal::as_str),
        Some("2026-08-25T09:00:00Z")
    );
    assert_eq!(
        task_row.scheduled.as_ref().map(TaskNodeTemporal::as_str),
        Some("2026-09-01")
    );
    assert_eq!(task_row.closed_at, None);
    match &task_row.evidence {
        TaskRowEvidence::Node {
            node_id,
            revision,
            profile_revision,
        } => {
            assert_eq!(*node_id, task_id);
            assert_eq!(*revision, DocumentRevision::from_source(&task_source));
            assert_eq!(revision, profile_revision);
        }
        TaskRowEvidence::Checklist { .. } => panic!("expected node evidence"),
    }

    let checklists = projection
        .rows_for_owner_node(task_id)
        .filter(|row| row.kind == TaskRowKind::Checklist)
        .collect::<Vec<_>>();
    assert_eq!(checklists.len(), 2);
    assert_eq!(checklists[0].description, "完成 مرحبا ✅");
    assert_eq!(checklists[0].id, None);
    assert_eq!(checklists[0].state, TaskNodeState::Completed);
    assert!(checklists[0].closed);
    assert_eq!(checklists[0].checklist_depth, Some(1));
    assert_eq!(checklists[0].priority, None);
    assert_eq!(checklists[0].blocked, None);
    match &checklists[0].evidence {
        TaskRowEvidence::Checklist {
            revision,
            occurrence,
            marker_range,
            description_range,
            ..
        } => {
            assert_eq!(*revision, DocumentRevision::from_source(&task_source));
            assert_eq!(slice(&task_source, marker_range), "[x]");
            assert_eq!(slice(&task_source, description_range), "完成 مرحبا ✅");
            assert!(occurrence.branch_complete);
        }
        TaskRowEvidence::Node { .. } => panic!("expected checklist evidence"),
    }

    let checklist_json = serde_json::to_value(checklists[0]).expect("row JSON");
    for field in [
        "id",
        "priority",
        "created",
        "start",
        "scheduled",
        "due",
        "closedAt",
        "blocked",
    ] {
        assert_eq!(checklist_json.get(field), Some(&Value::Null), "{field}");
    }
    assert_eq!(checklist_json["kind"], "checklist");
    assert_eq!(checklist_json["evidence"]["kind"], "checklist");
    let node_json = serde_json::to_value(task_row).expect("node JSON");
    assert_eq!(node_json["kind"], "node");
    assert_eq!(node_json["evidence"]["kind"], "node");
    assert_eq!(node_json["checklistDepth"], Value::Null);
    assert_eq!(node_json["priority"], "normal");

    assert!(projection.task_node_row(node(ROOT_ID)).is_none());
    assert!(projection.task_node_row(node(MISSING_TITLE_ID)).is_none());
    assert!(projection.task_node_row(node(INVALID_ID)).is_none());
    assert_eq!(
        projection
            .rows_for_owner_node(node(INVALID_ID))
            .filter(|row| row.kind == TaskRowKind::Checklist)
            .count(),
        1
    );
    assert!(projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.node_id == node(ROOT_ID)
            && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile
    }));
    assert!(projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.node_id == node(MISSING_TITLE_ID)
            && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::MissingTaskTitle
    }));
    assert!(projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.node_id == node(INVALID_ID)
            && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile
    }));

    let changed_source = format!("{task_source}\r\nbody changed");
    write_exact_node(&root.join("Task"), "Task", &changed_source);
    let changed = TaskWorkspaceProjection::rebuild(&root).expect("changed projection");
    let changed_evidence = &changed
        .task_node_row(task_id)
        .expect("changed row")
        .evidence;
    let TaskRowEvidence::Node {
        revision,
        profile_revision,
        ..
    } = changed_evidence
    else {
        panic!("expected node evidence");
    };
    assert_eq!(*revision, DocumentRevision::from_source(&changed_source));
    assert_eq!(revision, profile_revision);
    assert_ne!(*revision, DocumentRevision::from_source(&task_source));
}

#[test]
fn trash_unmanaged_and_ignored_content_never_enters_the_projection() {
    let (_temporary, root) = workspace("Boundaries");
    write_node(
        &root,
        "Boundaries",
        ROOT_ID,
        "= Root\n\n* [ ] visible root checklist\n",
    );
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged Loose/\nignore Ignored/\n",
    )
    .expect("content rules");
    fs::create_dir(root.join("Loose")).expect("unmanaged directory");
    fs::write(
        root.join("Loose/Loose.adoc"),
        "= Secret unmanaged\n:weftext-task: v1\n:weftext-task-state: todo\n* [ ] no row\n",
    )
    .expect("unmanaged source");
    fs::create_dir(root.join("Ignored")).expect("ignored directory");
    fs::write(
        root.join("Ignored/Ignored.adoc"),
        "= Secret ignored\n* [ ] no row\n",
    )
    .expect("ignored source");
    write_node(
        &root.join(".weftext-trash"),
        ".weftext-trash",
        TRASH_ID,
        "= Trash\n:weftext-task: v1\n:weftext-task-state: todo\n\n* [ ] no row\n",
    );

    let projection = TaskWorkspaceProjection::rebuild(&root).expect("projection");
    assert_eq!(projection.rows().len(), 1);
    assert_eq!(projection.rows()[0].description, "visible root checklist");
    let serialized = serde_json::to_string(projection.rows()).expect("rows JSON");
    assert!(!serialized.contains("Secret"));
    assert!(!serialized.contains(TRASH_ID));
}

fn workspace(name: &str) -> (TempDir, PathBuf) {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join(name);
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    (temporary, root)
}

fn write_node(directory: &Path, name: &str, node_id: &str, body: &str) {
    let source = format!("---\nweftext:\n  id: \"{node_id}\"\n---\n{body}");
    write_exact_node(directory, name, &source);
}

fn write_exact_node(directory: &Path, name: &str, source: &str) {
    fs::create_dir_all(directory).expect("node directory");
    fs::write(directory.join(format!("{name}.adoc")), source).expect("node source");
}

fn node(value: &str) -> NodeId {
    NodeId::from_str(value).expect("valid node ID")
}

fn slice<'a>(source: &'a str, range: &std::ops::Range<u64>) -> &'a str {
    let start = usize::try_from(range.start).expect("range start");
    let end = usize::try_from(range.end).expect("range end");
    &source[start..end]
}
