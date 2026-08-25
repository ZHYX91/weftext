use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{TempDir, tempdir};
use weftext_core::{
    DocumentError, DocumentRevision, NodeId, StructuralAction, TaskDateTime, TaskEditIntent,
    TaskEditTarget, TaskId, TaskPriority, TaskRecurrenceCompletionContext, TaskState,
    TaskTransactionError, TaskWorkspaceDiagnosticCode, TaskWorkspaceIndex, WorkspaceNodeProjection,
    WorkspaceReadScope, WorkspaceTransactionError, commit_task_dependency_transaction,
    commit_task_edit_transaction, commit_task_recurrence_transaction,
    plan_task_dependency_transaction, plan_task_dependency_transaction_scoped,
    plan_task_edit_transaction, plan_task_edit_transaction_scoped,
    plan_task_recurrence_transaction, read_node_document,
};

const ROOT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
const CHILD_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
const MISSING: &str = "99999999-9999-4999-8999-999999999999";

#[test]
fn ordinary_task_edits_are_preview_only_revision_bound_and_recoverably_committed() {
    let (_temporary, root) = workspace(
        "Edits",
        "* [ ] Simple\n* [ ] Structured task:[id=11111111-1111-4111-8111-111111111111]\n",
        "* [ ] Other task:[id=22222222-2222-4222-8222-222222222222]\n",
    );
    let snapshot = read_node_document(&root).expect("root snapshot");
    let simple = TaskEditTarget::Occurrence {
        range: TaskWorkspaceIndex::rebuild(&root)
            .unwrap()
            .occurrences_for_node(node(ROOT_ID))
            .next()
            .unwrap()
            .task
            .range
            .clone(),
    };
    let plan = plan_task_edit_transaction(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &simple,
        &TaskEditIntent::Toggle,
    )
    .expect("task transaction preview");
    assert_eq!(
        plan.workspace_transaction().action,
        StructuralAction::TaskEdit
    );
    assert_eq!(plan.workspace_transaction().document_changes.len(), 1);
    assert_eq!(
        fs::read_to_string(root.join("Edits.adoc")).unwrap(),
        snapshot.source,
        "planning must not write"
    );

    let committed = commit_task_edit_transaction(&plan).expect("commit task edit");
    assert_eq!(committed.action, StructuralAction::TaskEdit);
    let committed_source = fs::read_to_string(root.join("Edits.adoc")).unwrap();
    assert!(committed_source.contains("* [x] Simple"));
    assert!(matches!(
        commit_task_edit_transaction(&plan),
        Err(WorkspaceTransactionError::StaleRevision { .. })
    ));

    let current = read_node_document(&root).unwrap();
    let no_change = plan_task_edit_transaction(
        &root,
        node(ROOT_ID),
        &current.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &TaskEditIntent::SetPriority { priority: None },
    )
    .expect_err("no-op transaction");
    assert!(matches!(
        no_change,
        TaskTransactionError::Workspace(WorkspaceTransactionError::NoChange)
    ));

    let stale = plan_task_edit_transaction(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::High),
        },
    )
    .expect_err("stale document preview");
    assert!(matches!(
        stale,
        TaskTransactionError::Workspace(WorkspaceTransactionError::Document(
            DocumentError::StaleRevision { .. }
        ))
    ));
}

#[test]
fn recurrence_transaction_validates_the_workspace_and_commits_history_plus_successor() {
    let (_temporary, root) = workspace(
        "Recurrence",
        &format!(
            "* [ ] Repeat task:[id={TASK_A},due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=2\",repeat-from=due,depends-on=\"{TASK_B}\"]\n"
        ),
        &format!("* [ ] Prerequisite task:[id={TASK_B}]\n"),
    );
    let snapshot = read_node_document(&root).unwrap();
    let plan = plan_task_recurrence_transaction(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &TaskRecurrenceCompletionContext {
            completed_at: TaskDateTime::Date("2026-08-24".to_owned()),
            utc_offset_minutes: 8 * 60,
        },
    )
    .expect("recurrence transaction preview");
    assert_eq!(
        plan.workspace_transaction().action,
        StructuralAction::TaskRecurrenceCompletion
    );
    let next_id = plan.completion.next_task_id.expect("successor ID");
    assert_eq!(
        fs::read_to_string(root.join("Recurrence.adoc")).unwrap(),
        snapshot.source
    );
    commit_task_recurrence_transaction(&plan).expect("commit recurrence");

    let index = TaskWorkspaceIndex::rebuild(&root).expect("post-commit task index");
    assert!(index.diagnostics().is_empty(), "{:?}", index.diagnostics());
    assert_eq!(
        index.unique_task(task(TASK_A)).unwrap().task.state,
        TaskState::Closed
    );
    let successor = index.unique_task(next_id).expect("unique successor");
    assert_eq!(successor.node_id, node(ROOT_ID));
    assert_eq!(successor.task.state, TaskState::Open);
    assert_eq!(
        successor.task.metadata.as_ref().unwrap().dependencies,
        [task(TASK_B)]
    );
}

#[test]
fn recurrence_commit_rejects_an_unrelated_workspace_change_after_preview() {
    let (_temporary, root) = workspace(
        "Stale",
        &format!(
            "* [ ] Repeat task:[id={TASK_A},due=2026-08-24,rrule=\"FREQ=DAILY\",repeat-from=due]\n"
        ),
        &format!("* [ ] Other task:[id={TASK_B}]\n"),
    );
    let snapshot = read_node_document(&root).unwrap();
    let plan = plan_task_recurrence_transaction(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &TaskRecurrenceCompletionContext {
            completed_at: TaskDateTime::Date("2026-08-24".to_owned()),
            utc_offset_minutes: 0,
        },
    )
    .expect("recurrence preview");
    let child_path = root.join("Child/Child.adoc");
    let mut child = fs::read_to_string(&child_path).unwrap();
    child.push_str("\nChanged elsewhere.\n");
    fs::write(&child_path, child).unwrap();

    assert!(matches!(
        commit_task_recurrence_transaction(&plan),
        Err(WorkspaceTransactionError::StaleRevision { .. })
    ));
    assert_eq!(
        fs::read_to_string(root.join("Stale.adoc")).unwrap(),
        snapshot.source
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn dependency_transactions_promote_validate_cycles_and_repair_invalid_graphs() {
    let (_temporary, root) = workspace(
        "Dependencies",
        &format!("* [ ] Simple\n* [ ] A task:[id={TASK_A},depends-on=\"{MISSING}\"]\n"),
        &format!("* [ ] B task:[id={TASK_B}]\n"),
    );
    let snapshot = read_node_document(&root).unwrap();
    let repair = plan_task_dependency_transaction(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &[task(TASK_B)],
    )
    .expect("repair unresolved dependency");
    assert_eq!(
        repair.workspace_transaction().action,
        StructuralAction::TaskDependencies
    );
    commit_task_dependency_transaction(&repair).expect("commit dependency repair");
    assert!(
        TaskWorkspaceIndex::rebuild(&root)
            .unwrap()
            .diagnostics()
            .is_empty()
    );

    let current = read_node_document(&root).unwrap();
    let simple_range = TaskWorkspaceIndex::rebuild(&root)
        .unwrap()
        .occurrences_for_node(node(ROOT_ID))
        .find(|occurrence| occurrence.task.metadata.is_none())
        .unwrap()
        .task
        .range
        .clone();
    let promotion = plan_task_dependency_transaction(
        &root,
        node(ROOT_ID),
        &current.revision,
        &TaskEditTarget::Occurrence {
            range: simple_range,
        },
        &[task(TASK_B)],
    )
    .expect("promote simple dependency task");
    let promoted_id = promotion.authoring.assigned_id.expect("promoted ID");
    commit_task_dependency_transaction(&promotion).expect("commit promotion");
    assert!(
        TaskWorkspaceIndex::rebuild(&root)
            .unwrap()
            .unique_task(promoted_id)
            .is_some()
    );

    let child = read_node_document(root.join("Child")).unwrap();
    let cycle = plan_task_dependency_transaction(
        &root,
        node(CHILD_ID),
        &child.revision,
        &TaskEditTarget::Id { id: task(TASK_B) },
        &[task(TASK_A)],
    )
    .expect_err("dependency cycle");
    let TaskTransactionError::ProposedWorkspaceInvalid { diagnostics, .. } = cycle else {
        panic!("unexpected cycle error: {cycle}");
    };
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TaskWorkspaceDiagnosticCode::DependencyCycle)
    );

    let child = read_node_document(root.join("Child")).unwrap();
    let missing = plan_task_dependency_transaction(
        &root,
        node(CHILD_ID),
        &child.revision,
        &TaskEditTarget::Id { id: task(TASK_B) },
        &[task(MISSING)],
    )
    .expect_err("missing dependency");
    let TaskTransactionError::ProposedWorkspaceInvalid { diagnostics, .. } = missing else {
        panic!("unexpected missing error: {missing}");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == TaskWorkspaceDiagnosticCode::UnresolvedDependency
    }));

    let root_snapshot = read_node_document(&root).unwrap();
    let duplicate = plan_task_dependency_transaction(
        &root,
        node(ROOT_ID),
        &root_snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &[task(TASK_B), task(TASK_B)],
    )
    .expect_err("duplicate dependency intent");
    assert!(matches!(duplicate, TaskTransactionError::Authoring(_)));
    let self_dependency = plan_task_dependency_transaction(
        &root,
        node(ROOT_ID),
        &root_snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &[task(TASK_A)],
    )
    .expect_err("self dependency intent");
    assert!(matches!(
        self_dependency,
        TaskTransactionError::Authoring(_)
    ));

    let child_path = root.join("Child/Child.adoc");
    let mut child_source = fs::read_to_string(&child_path).unwrap();
    child_source = child_source.replace(
        &format!("* [ ] B task:[id={TASK_B}]"),
        &format!("* [ ] B task:[id={TASK_B},depends-on=\"{TASK_A}\"]"),
    );
    fs::write(&child_path, child_source).unwrap();
    assert!(
        TaskWorkspaceIndex::rebuild(&root)
            .unwrap()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == TaskWorkspaceDiagnosticCode::DependencyCycle)
    );
    let root_snapshot = read_node_document(&root).unwrap();
    let cycle_repair = plan_task_dependency_transaction(
        &root,
        node(ROOT_ID),
        &root_snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &[],
    )
    .expect("repair existing cycle");
    commit_task_dependency_transaction(&cycle_repair).expect("commit cycle repair");
    assert!(
        TaskWorkspaceIndex::rebuild(&root)
            .unwrap()
            .diagnostics()
            .is_empty()
    );
}

#[test]
fn duplicate_workspace_identity_blocks_all_structured_task_transactions() {
    let (_temporary, root) = workspace(
        "Duplicate",
        &format!("* [ ] A task:[id={TASK_A}]\n"),
        &format!("* [ ] Duplicate task:[id={TASK_A}]\n"),
    );
    let snapshot = read_node_document(&root).unwrap();
    let failure = plan_task_edit_transaction(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Occurrence {
            range: TaskWorkspaceIndex::rebuild(&root)
                .unwrap()
                .occurrences_for_node(node(ROOT_ID))
                .next()
                .unwrap()
                .task
                .range
                .clone(),
        },
        &TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::High),
        },
    )
    .expect_err("duplicate task identity");
    assert!(matches!(
        failure,
        TaskTransactionError::TargetWorkspaceInvalid { .. }
    ));
}

#[test]
fn scoped_task_transaction_never_opens_an_unavailable_document_body() {
    let (_temporary, root) = workspace(
        "Scoped",
        &format!("* [ ] Visible task:[id={TASK_A}]\n"),
        &format!("* [ ] Hidden task:[id={TASK_B}]\n"),
    );
    let child_document = root.join("Child/Child.adoc");
    let mut hidden_bytes = fs::read(&child_document).expect("hidden source");
    hidden_bytes.push(0xff);
    fs::write(&child_document, hidden_bytes).expect("poison hidden body");
    assert!(TaskWorkspaceIndex::rebuild(&root).is_err());

    let snapshot = read_node_document(&root).expect("visible snapshot");
    let scope = WorkspaceReadScope::new([WorkspaceNodeProjection::new(node(ROOT_ID), None, "")])
        .expect("visible projection");
    let plan = plan_task_edit_transaction_scoped(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &TaskEditIntent::SetPriority {
            priority: Some(TaskPriority::High),
        },
        &scope,
    )
    .expect("scoped task preview");
    commit_task_edit_transaction(&plan).expect("scoped commit");
    assert!(
        read_node_document(&root)
            .unwrap()
            .source
            .contains("priority=high")
    );
    let unavailable = plan_task_edit_transaction_scoped(
        &root,
        node(CHILD_ID),
        &DocumentRevision::from_source("unavailable"),
        &TaskEditTarget::Id { id: task(TASK_B) },
        &TaskEditIntent::Toggle,
        &scope,
    )
    .expect_err("scope rejection must precede hidden document read");
    assert!(matches!(
        unavailable,
        TaskTransactionError::TargetUnavailable
    ));
}

#[test]
fn scoped_dependency_planning_does_not_reveal_hidden_task_identity() {
    let (_temporary, root) = workspace(
        "ScopedDependency",
        &format!("* [ ] Visible task:[id={TASK_A}]\n"),
        &format!("* [ ] Hidden task:[id={TASK_B}]\n"),
    );
    let snapshot = read_node_document(&root).expect("visible snapshot");
    let scope = WorkspaceReadScope::new([WorkspaceNodeProjection::new(node(ROOT_ID), None, "")])
        .expect("visible projection");
    let failure = plan_task_dependency_transaction_scoped(
        &root,
        node(ROOT_ID),
        &snapshot.revision,
        &TaskEditTarget::Id { id: task(TASK_A) },
        &[task(TASK_B)],
        &scope,
    )
    .expect_err("hidden dependency must look unresolved");
    let TaskTransactionError::ProposedWorkspaceInvalid { diagnostics, .. } = failure else {
        panic!("unexpected scoped dependency error: {failure}");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        TaskWorkspaceDiagnosticCode::UnresolvedDependency
    );
    assert_eq!(diagnostics[0].node_id, node(ROOT_ID));
    assert!(diagnostics[0].related_node_ids.is_empty());

    let unavailable = plan_task_edit_transaction_scoped(
        &root,
        node(CHILD_ID),
        &read_node_document(root.join("Child")).unwrap().revision,
        &TaskEditTarget::Id { id: task(TASK_B) },
        &TaskEditIntent::Toggle,
        &scope,
    )
    .expect_err("node outside scope");
    assert!(matches!(
        unavailable,
        TaskTransactionError::TargetUnavailable
    ));
}

fn workspace(name: &str, root_body: &str, child_body: &str) -> (TempDir, PathBuf) {
    let temporary = tempdir().expect("temporary workspace parent");
    let root = temporary.path().join(name);
    fs::create_dir(&root).unwrap();
    fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
    write_node(&root, name, ROOT_ID, root_body);
    write_node(&root.join("Child"), "Child", CHILD_ID, child_body);
    (temporary, root)
}

fn write_node(directory: &Path, name: &str, node_id: &str, body: &str) {
    fs::create_dir_all(directory).unwrap();
    let source = format!("---\nweftext:\n  id: \"{node_id}\"\n---\n= {name}\n\n{body}");
    fs::write(directory.join(format!("{name}.adoc")), source).unwrap();
}

fn node(value: &str) -> NodeId {
    value.parse().expect("valid node ID")
}

fn task(value: &str) -> TaskId {
    value.parse().expect("valid task ID")
}
