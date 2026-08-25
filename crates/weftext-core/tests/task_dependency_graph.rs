use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use tempfile::{TempDir, tempdir};
use weftext_core::{
    NodeId, TaskNodeDiagnosticCode, TaskRowKind, TaskWorkspaceProjection,
    TaskWorkspaceProjectionDiagnosticCode, WorkspaceNodeProjection, WorkspaceReadScope,
};

const ROOT: &str = "20000000-0000-4000-8000-000000000001";
const MISSING_SOURCE: &str = "20000000-0000-4000-8000-000000000002";
const REVERSE_SOURCE: &str = "20000000-0000-4000-8000-000000000003";
const NON_TASK: &str = "20000000-0000-4000-8000-000000000004";
const NON_TASK_SOURCE: &str = "20000000-0000-4000-8000-000000000005";
const INVALID_TARGET: &str = "20000000-0000-4000-8000-000000000006";
const INVALID_SOURCE: &str = "20000000-0000-4000-8000-000000000007";
const SELF: &str = "20000000-0000-4000-8000-000000000008";
const DUPLICATE: &str = "20000000-0000-4000-8000-000000000009";
const OPEN: &str = "20000000-0000-4000-8000-000000000010";
const BLOCKED: &str = "20000000-0000-4000-8000-000000000011";
const CLOSED: &str = "20000000-0000-4000-8000-000000000012";
const UNBLOCKED: &str = "20000000-0000-4000-8000-000000000013";
const CYCLE_A: &str = "20000000-0000-4000-8000-000000000014";
const CYCLE_B: &str = "20000000-0000-4000-8000-000000000015";
const CYCLE_C: &str = "20000000-0000-4000-8000-000000000016";
const CYCLE_DEPENDENT: &str = "20000000-0000-4000-8000-000000000017";
const ABSENT: &str = "29999999-9999-4999-8999-999999999999";

#[test]
#[allow(clippy::too_many_lines)]
fn complete_graph_invalidates_bad_targets_cycles_and_reverse_dependents() {
    let (_temporary, root) = workspace("Graph");
    write_node(&root, "Graph", ROOT, "= Graph\n");
    write_task(&root, "Missing", MISSING_SOURCE, "todo", Some(ABSENT));
    write_task(
        &root,
        "Reverse",
        REVERSE_SOURCE,
        "todo",
        Some(MISSING_SOURCE),
    );
    write_node(&root.join("Ordinary"), "Ordinary", NON_TASK, "= Ordinary\n");
    write_task(
        &root,
        "NonTaskSource",
        NON_TASK_SOURCE,
        "todo",
        Some(NON_TASK),
    );
    write_task(&root, "InvalidTarget", INVALID_TARGET, "waiting", None);
    write_task(
        &root,
        "InvalidSource",
        INVALID_SOURCE,
        "todo",
        Some(INVALID_TARGET),
    );
    write_task(&root, "Self", SELF, "todo", Some(SELF));
    write_task_with_dependencies(
        &root,
        "Duplicate",
        DUPLICATE,
        "todo",
        &format!("{OPEN} {OPEN}"),
    );
    write_task(&root, "Open", OPEN, "todo", None);
    write_task(&root, "Blocked", BLOCKED, "todo", Some(OPEN));
    write_node(
        &root.join("Closed"),
        "Closed",
        CLOSED,
        "= Closed\n:weftext-task: v1\n:weftext-task-state: completed\n:weftext-task-closed: 2026-08-24T12:00:00Z\n",
    );
    write_task(&root, "Unblocked", UNBLOCKED, "todo", Some(CLOSED));
    write_task(&root, "CycleA", CYCLE_A, "todo", Some(CYCLE_B));
    write_task(&root, "CycleB", CYCLE_B, "todo", Some(CYCLE_C));
    write_task(&root, "CycleC", CYCLE_C, "todo", Some(CYCLE_A));
    write_task(
        &root,
        "CycleDependent",
        CYCLE_DEPENDENT,
        "todo",
        Some(CYCLE_A),
    );

    let first = TaskWorkspaceProjection::rebuild(&root).expect("complete graph");
    let second = TaskWorkspaceProjection::rebuild(&root).expect("deterministic graph");
    assert_eq!(first.rows(), second.rows());
    assert_eq!(first.diagnostics(), second.diagnostics());

    let node_rows = first
        .rows()
        .iter()
        .filter(|row| row.kind == TaskRowKind::Node)
        .collect::<Vec<_>>();
    assert_eq!(node_rows.len(), 4);
    assert_eq!(
        first.task_node_row(node(OPEN)).unwrap().blocked,
        Some(false)
    );
    assert_eq!(
        first.task_node_row(node(BLOCKED)).unwrap().blocked,
        Some(true)
    );
    assert_eq!(
        first.task_node_row(node(CLOSED)).unwrap().blocked,
        Some(false)
    );
    assert_eq!(
        first
            .task_node_row(node(CLOSED))
            .unwrap()
            .closed_at
            .as_ref()
            .map(weftext_core::TaskNodeTemporal::as_str),
        Some("2026-08-24T12:00:00Z")
    );
    assert_eq!(
        first.task_node_row(node(UNBLOCKED)).unwrap().blocked,
        Some(false)
    );

    for invalid in [
        MISSING_SOURCE,
        REVERSE_SOURCE,
        NON_TASK_SOURCE,
        INVALID_TARGET,
        INVALID_SOURCE,
        SELF,
        DUPLICATE,
        CYCLE_A,
        CYCLE_B,
        CYCLE_C,
        CYCLE_DEPENDENT,
    ] {
        assert!(first.task_node_row(node(invalid)).is_none(), "{invalid}");
    }
    assert!(has_code(
        &first,
        MISSING_SOURCE,
        TaskWorkspaceProjectionDiagnosticCode::UnresolvedDependency,
    ));
    assert!(has_code(
        &first,
        REVERSE_SOURCE,
        TaskWorkspaceProjectionDiagnosticCode::InvalidDependencyTarget,
    ));
    assert!(has_code(
        &first,
        NON_TASK_SOURCE,
        TaskWorkspaceProjectionDiagnosticCode::NonTaskDependency,
    ));
    assert!(has_code(
        &first,
        INVALID_SOURCE,
        TaskWorkspaceProjectionDiagnosticCode::InvalidDependencyTarget,
    ));
    assert!(first.diagnostics().iter().any(|diagnostic| {
        diagnostic.node_id == node(SELF)
            && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile
            && diagnostic.task_node_code == Some(TaskNodeDiagnosticCode::SelfDependency)
    }));
    assert!(first.diagnostics().iter().any(|diagnostic| {
        diagnostic.node_id == node(DUPLICATE)
            && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile
            && diagnostic.task_node_code == Some(TaskNodeDiagnosticCode::DuplicateDependency)
    }));

    let expected_component = vec![node(CYCLE_A), node(CYCLE_B), node(CYCLE_C)];
    let cycles = first
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::DependencyCycle
        })
        .collect::<Vec<_>>();
    assert_eq!(cycles.len(), 3);
    for diagnostic in cycles {
        assert_eq!(diagnostic.related_node_ids, expected_component);
    }
    assert!(has_code(
        &first,
        CYCLE_DEPENDENT,
        TaskWorkspaceProjectionDiagnosticCode::InvalidDependencyTarget,
    ));
}

#[test]
fn scoped_graph_keeps_unavailable_targets_non_disclosing_and_reads_only_visible_bodies() {
    let (_temporary, root) = workspace("Scoped");
    write_node(&root, "Scoped", ROOT, "= Scoped\n");
    write_task(&root, "Source", BLOCKED, "todo", Some(OPEN));
    write_task(&root, "Hidden", OPEN, "todo", None);
    let hidden_document = root.join("Hidden/Hidden.adoc");
    let source_scope = scope(&[(ROOT, None, ""), (BLOCKED, Some(ROOT), "Source")]);

    let open_hidden =
        TaskWorkspaceProjection::rebuild_scoped(&root, &source_scope).expect("hidden open target");
    assert_eq!(
        open_hidden.task_node_row(node(BLOCKED)).unwrap().blocked,
        Some(false)
    );
    let unavailable = open_hidden
        .diagnostics()
        .iter()
        .find(|diagnostic| {
            diagnostic.node_id == node(BLOCKED)
                && diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::UnresolvedDependency
        })
        .expect("unavailable dependency");
    assert_eq!(unavailable.dependency_id, None);
    assert!(unavailable.related_node_ids.is_empty());
    let baseline = scoped_json(&open_hidden);
    assert!(!baseline.contains(OPEN));
    assert!(!baseline.contains("Hidden"));
    assert!(!baseline.contains("/Hidden"));

    write_task(&root, "Hidden", OPEN, "completed", None);
    assert_eq!(
        scoped_json(
            &TaskWorkspaceProjection::rebuild_scoped(&root, &source_scope)
                .expect("hidden closed target")
        ),
        baseline
    );
    write_task(&root, "Hidden", OPEN, "waiting", None);
    assert_eq!(
        scoped_json(
            &TaskWorkspaceProjection::rebuild_scoped(&root, &source_scope)
                .expect("hidden invalid target")
        ),
        baseline
    );

    let mut poisoned = fs::read(&hidden_document).expect("hidden bytes");
    poisoned.push(0xff);
    fs::write(&hidden_document, poisoned).expect("poison hidden body");
    assert_eq!(
        scoped_json(
            &TaskWorkspaceProjection::rebuild_scoped(&root, &source_scope)
                .expect("hidden body is not opened")
        ),
        baseline
    );
    fs::remove_file(&hidden_document).expect("remove hidden document");
    assert_eq!(
        scoped_json(
            &TaskWorkspaceProjection::rebuild_scoped(&root, &source_scope)
                .expect("missing and hidden are identical")
        ),
        baseline
    );

    write_task(&root, "Hidden", OPEN, "todo", None);
    let visible_scope = scope(&[
        (ROOT, None, ""),
        (BLOCKED, Some(ROOT), "Source"),
        (OPEN, Some(ROOT), "Target"),
    ]);
    let visible_open = TaskWorkspaceProjection::rebuild_scoped(&root, &visible_scope)
        .expect("visible open target");
    assert_eq!(
        visible_open.task_node_row(node(BLOCKED)).unwrap().blocked,
        Some(true)
    );
    assert_eq!(
        visible_open
            .task_node_row(node(OPEN))
            .unwrap()
            .owner_node_path,
        "/Target"
    );

    write_task(&root, "Hidden", OPEN, "completed", None);
    let visible_closed = TaskWorkspaceProjection::rebuild_scoped(&root, &visible_scope)
        .expect("visible closed target");
    assert_eq!(
        visible_closed.task_node_row(node(BLOCKED)).unwrap().blocked,
        Some(false)
    );

    write_task(&root, "Hidden", OPEN, "waiting", None);
    let visible_invalid = TaskWorkspaceProjection::rebuild_scoped(&root, &visible_scope)
        .expect("visible invalid target");
    assert!(visible_invalid.task_node_row(node(BLOCKED)).is_none());
    assert!(has_code(
        &visible_invalid,
        BLOCKED,
        TaskWorkspaceProjectionDiagnosticCode::InvalidDependencyTarget,
    ));
}

#[test]
fn scoped_cycles_are_diagnosed_only_when_the_complete_component_is_visible() {
    let (_temporary, root) = workspace("ScopedCycle");
    write_node(&root, "ScopedCycle", ROOT, "= Scoped cycle\n");
    write_task(&root, "A", CYCLE_A, "todo", Some(CYCLE_B));
    write_task(&root, "B", CYCLE_B, "todo", Some(CYCLE_A));

    let partial = scope(&[(ROOT, None, ""), (CYCLE_A, Some(ROOT), "A")]);
    let partial_projection =
        TaskWorkspaceProjection::rebuild_scoped(&root, &partial).expect("partial component");
    assert!(partial_projection.task_node_row(node(CYCLE_A)).is_some());
    assert!(!partial_projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::DependencyCycle
    }));

    let complete = scope(&[
        (ROOT, None, ""),
        (CYCLE_A, Some(ROOT), "A"),
        (CYCLE_B, Some(ROOT), "B"),
    ]);
    let complete_projection =
        TaskWorkspaceProjection::rebuild_scoped(&root, &complete).expect("complete component");
    assert!(complete_projection.task_node_row(node(CYCLE_A)).is_none());
    assert!(complete_projection.task_node_row(node(CYCLE_B)).is_none());
    assert_eq!(
        complete_projection
            .diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == TaskWorkspaceProjectionDiagnosticCode::DependencyCycle
            })
            .count(),
        2
    );
}

fn has_code(
    projection: &TaskWorkspaceProjection,
    node_id: &str,
    code: TaskWorkspaceProjectionDiagnosticCode,
) -> bool {
    projection
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.node_id == node(node_id) && diagnostic.code == code)
}

fn scoped_json(projection: &TaskWorkspaceProjection) -> String {
    serde_json::to_string(&(projection.rows(), projection.diagnostics())).expect("projection JSON")
}

fn scope(entries: &[(&str, Option<&str>, &str)]) -> WorkspaceReadScope {
    WorkspaceReadScope::new(entries.iter().map(|(node_id, parent_id, locator)| {
        WorkspaceNodeProjection::new(node(node_id), parent_id.map(node), (*locator).to_owned())
    }))
    .expect("read scope")
}

fn workspace(name: &str) -> (TempDir, PathBuf) {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join(name);
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    (temporary, root)
}

fn write_task(root: &Path, name: &str, node_id: &str, state: &str, dependency: Option<&str>) {
    write_task_with_dependencies(root, name, node_id, state, dependency.unwrap_or(""));
}

fn write_task_with_dependencies(
    root: &Path,
    name: &str,
    node_id: &str,
    state: &str,
    dependencies: &str,
) {
    let dependency = if dependencies.is_empty() {
        String::new()
    } else {
        format!(":weftext-task-depends-on: {dependencies}\n")
    };
    write_node(
        &root.join(name),
        name,
        node_id,
        &format!("= {name}\n:weftext-task: v1\n:weftext-task-state: {state}\n{dependency}"),
    );
}

fn write_node(directory: &Path, name: &str, node_id: &str, body: &str) {
    fs::create_dir_all(directory).expect("node directory");
    let source = format!("---\nweftext:\n  id: \"{node_id}\"\n---\n{body}");
    fs::write(directory.join(format!("{name}.adoc")), source).expect("node source");
}

fn node(value: &str) -> NodeId {
    NodeId::from_str(value).expect("valid node ID")
}
