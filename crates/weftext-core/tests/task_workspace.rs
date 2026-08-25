use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use tempfile::{TempDir, tempdir};
use weftext_core::{
    NodeId, TaskId, TaskWorkspaceDiagnosticCode, TaskWorkspaceError, TaskWorkspaceIndex,
};

const ROOT_NODE_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
const CHILD_NODE_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
const THIRD_NODE_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa3";
const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
const TASK_C: &str = "33333333-3333-4333-8333-333333333333";
const DUPLICATE_TASK: &str = "44444444-4444-4444-8444-444444444444";
const INVALID_TARGET: &str = "55555555-5555-4555-8555-555555555555";
const MISSING_TASK: &str = "99999999-9999-4999-8999-999999999999";

#[test]
fn workspace_index_keeps_simple_occurrences_and_resolves_unique_structured_tasks() {
    let (_temporary, root) = workspace("Index");
    write_node(
        &root,
        "Index",
        ROOT_NODE_ID,
        &format!("= Tasks\n\n* [ ] Simple\n* [ ] A task:[id={TASK_A},depends-on=\"{TASK_B}\"]\n"),
    );
    write_node(
        &root.join("Child"),
        "Child",
        CHILD_NODE_ID,
        &format!("= Child\n\n* [ ] B task:[id={TASK_B},due=2026-09-01]\n"),
    );

    let index = TaskWorkspaceIndex::rebuild(&root).expect("task workspace index");
    assert!(index.diagnostics().is_empty(), "{:?}", index.diagnostics());
    assert_eq!(index.occurrences().len(), 3);
    assert_eq!(index.occurrences_for_node(node(ROOT_NODE_ID)).count(), 2);
    assert_eq!(index.declarations_for_id(task(TASK_A)).len(), 1);
    assert_eq!(
        index
            .unique_task(task(TASK_B))
            .expect("unique task B")
            .node_id,
        node(CHILD_NODE_ID)
    );
    assert!(index.occurrences()[0].task.metadata.is_none());
    assert_eq!(
        index.occurrences()[1].revision,
        index.occurrences()[0].revision
    );
}

#[test]
fn duplicate_missing_ambiguous_and_invalid_dependencies_fail_closed() {
    let (_temporary, root) = workspace("Diagnostics");
    write_node(
        &root,
        "Diagnostics",
        ROOT_NODE_ID,
        &format!(
            concat!(
                "= Diagnostics\n\n",
                "* [ ] First duplicate task:[id={duplicate}]\n",
                "* [ ] Invalid lifecycle task:[id={task_a},resolution=completed]\n",
                "* [ ] Missing target task:[id={task_b},depends-on=\"{missing}\"]\n",
                "* [ ] Ambiguous target task:[id={task_c},depends-on=\"{duplicate}\"]\n",
                "* [ ] Invalid target source task:[id={invalid_target}] trailing\n",
                "* [ ] Uses invalid target task:[id=66666666-6666-4666-8666-666666666666,depends-on=\"{invalid_target}\"]\n"
            ),
            duplicate = DUPLICATE_TASK,
            task_a = TASK_A,
            task_b = TASK_B,
            missing = MISSING_TASK,
            task_c = TASK_C,
            invalid_target = INVALID_TARGET,
        ),
    );
    write_node(
        &root.join("Child"),
        "Child",
        CHILD_NODE_ID,
        &format!("= Child\n\n* [x] Second duplicate task:[id={DUPLICATE_TASK}]\n"),
    );

    let index = TaskWorkspaceIndex::rebuild(&root).expect("diagnostic task index");
    let codes = index
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert_eq!(
        count(&codes, TaskWorkspaceDiagnosticCode::DuplicateTaskId),
        2
    );
    assert_eq!(
        count(&codes, TaskWorkspaceDiagnosticCode::UnresolvedDependency),
        1
    );
    assert_eq!(
        count(&codes, TaskWorkspaceDiagnosticCode::AmbiguousDependency),
        1
    );
    assert_eq!(
        count(&codes, TaskWorkspaceDiagnosticCode::InvalidDependencyTarget),
        1
    );
    assert!(codes.contains(&TaskWorkspaceDiagnosticCode::InvalidTaskSyntax));
    assert!(index.unique_task(task(DUPLICATE_TASK)).is_none());
    assert!(index.unique_task(task(INVALID_TARGET)).is_none());

    let missing = index
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == TaskWorkspaceDiagnosticCode::UnresolvedDependency)
        .expect("missing dependency diagnostic");
    assert_eq!(missing.task_id, Some(task(TASK_B)));
    assert_eq!(missing.dependency_id, Some(task(MISSING_TASK)));
    let root_source = fs::read_to_string(root.join("Diagnostics.adoc")).expect("root source");
    assert_eq!(
        slice(&root_source, &missing.range),
        format!("\"{MISSING_TASK}\"")
    );

    let duplicate = index
        .diagnostics()
        .iter()
        .find(|diagnostic| {
            diagnostic.code == TaskWorkspaceDiagnosticCode::DuplicateTaskId
                && diagnostic.node_id == node(ROOT_NODE_ID)
        })
        .expect("duplicate task diagnostic");
    assert_eq!(
        duplicate.related_node_ids,
        [node(ROOT_NODE_ID), node(CHILD_NODE_ID)]
    );
}

#[test]
fn every_member_of_a_dependency_cycle_gets_the_same_deterministic_component() {
    let (_temporary, root) = workspace("Cycle");
    write_node(
        &root,
        "Cycle",
        ROOT_NODE_ID,
        &format!("= Cycle\n\n* [ ] A task:[id={TASK_A},depends-on=\"{TASK_B}\"]\n"),
    );
    write_node(
        &root.join("Child"),
        "Child",
        CHILD_NODE_ID,
        &format!("= Child\n\n* [ ] B task:[id={TASK_B},depends-on=\"{TASK_C}\"]\n"),
    );
    write_node(
        &root.join("Third"),
        "Third",
        THIRD_NODE_ID,
        &format!("= Third\n\n* [ ] C task:[id={TASK_C},depends-on=\"{TASK_A}\"]\n"),
    );

    let first = TaskWorkspaceIndex::rebuild(&root).expect("cycle task index");
    let second = TaskWorkspaceIndex::rebuild(&root).expect("deterministic cycle task index");
    assert_eq!(first.diagnostics(), second.diagnostics());
    let cycles = first
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == TaskWorkspaceDiagnosticCode::DependencyCycle)
        .collect::<Vec<_>>();
    assert_eq!(cycles.len(), 3);
    for diagnostic in cycles {
        assert_eq!(
            diagnostic.related_task_ids,
            [task(TASK_A), task(TASK_B), task(TASK_C)]
        );
        assert_eq!(
            diagnostic.related_node_ids,
            [node(ROOT_NODE_ID), node(CHILD_NODE_ID), node(THIRD_NODE_ID)]
        );
    }
}

#[test]
fn task_workspace_runtime_refuses_markerless_markdown_generation() {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join("Markdown");
    fs::create_dir(&root).expect("workspace root");
    let source = format!("---\nweftext:\n  id: \"{ROOT_NODE_ID}\"\n---\n# Markdown\n");
    fs::write(root.join("Markdown.md"), source).expect("Markdown root document");
    assert!(matches!(
        TaskWorkspaceIndex::rebuild(&root),
        Err(TaskWorkspaceError::InvalidWorkspace(
            weftext_core::InventoryIssueCode::InvalidWorkspaceGeneration
        ))
    ));
}

fn workspace(name: &str) -> (TempDir, PathBuf) {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join(name);
    fs::create_dir(&root).expect("workspace root");
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
    (temporary, root)
}

fn write_node(directory: &Path, name: &str, node_id: &str, body: &str) {
    fs::create_dir_all(directory).expect("node directory");
    let source = format!("---\nweftext:\n  id: \"{node_id}\"\n---\n{body}");
    fs::write(directory.join(format!("{name}.adoc")), source).expect("node source");
}

fn node(value: &str) -> NodeId {
    NodeId::from_str(value).expect("valid node ID")
}

fn task(value: &str) -> TaskId {
    TaskId::from_str(value).expect("valid task ID")
}

fn count(values: &[TaskWorkspaceDiagnosticCode], expected: TaskWorkspaceDiagnosticCode) -> usize {
    values.iter().filter(|value| **value == expected).count()
}

fn slice<'a>(source: &'a str, range: &std::ops::Range<u64>) -> &'a str {
    let start = usize::try_from(range.start).expect("range start");
    let end = usize::try_from(range.end).expect("range end");
    &source[start..end]
}
