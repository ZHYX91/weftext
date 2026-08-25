use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{TempDir, tempdir};
use weftext_core::{
    ChecklistToggleError, ChecklistToggleEvidence, CreatedNode, NodeId, TaskActionTransactionError,
    TaskNodeActionEvidence, TaskNodeClosedEdit, TaskNodeEditError, TaskNodeEditIntent,
    TaskNodeEditRequest, TaskNodePriority, TaskNodeState, TaskNodeTemporal,
    WorkspaceNodeProjection, WorkspaceReadScope, acquire_workspace_transaction_lease,
    commit_checklist_toggle_transaction, commit_checklist_toggle_transaction_scoped,
    commit_task_node_edit_transaction, commit_task_node_edit_transaction_scoped,
    commit_workspace_transaction, create_child_node, create_workspace,
    plan_checklist_toggle_transaction, plan_checklist_toggle_transaction_scoped,
    plan_create_child_node, plan_task_node_edit_transaction,
    plan_task_node_edit_transaction_scoped, plan_trash_node_at,
    prepare_workspace_transaction_recovery_fixture, read_node_document,
};

struct WorkspaceFixture {
    _temporary: TempDir,
    root: CreatedNode,
    child: CreatedNode,
}

fn workspace() -> WorkspaceFixture {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("Workspace")).expect("workspace");
    let child = create_child_node(&root.path, "Child").expect("child");
    WorkspaceFixture {
        _temporary: temporary,
        root,
        child,
    }
}

fn source(node_id: NodeId, title: &str, tail: &str, line_ending: &str) -> String {
    let eol = line_ending;
    format!("---{eol}weftext:{eol}  id: \"{node_id}\"{eol}---{eol}= {title}{eol}{tail}")
}

fn write_source(node: &CreatedNode, source: &str) {
    fs::write(&node.document_path, source).expect("write source");
}

fn checklist_evidence(node: &CreatedNode) -> ChecklistToggleEvidence {
    let snapshot = read_node_document(&node.path).expect("snapshot");
    let occurrence = weftext_asciidoc::analyze(&snapshot.source)
        .checklists
        .into_iter()
        .next()
        .expect("checklist");
    ChecklistToggleEvidence {
        owner_node_id: node.id,
        revision: snapshot.revision,
        occurrence: occurrence.parser_occurrence,
        authored_marker: occurrence.authored_marker,
        marker_range: occurrence.marker_range,
    }
}

fn task_request(node: &CreatedNode, intent: TaskNodeEditIntent) -> TaskNodeEditRequest {
    let revision = read_node_document(&node.path).expect("snapshot").revision;
    TaskNodeEditRequest {
        evidence: TaskNodeActionEvidence {
            node_id: node.id,
            revision: revision.clone(),
            profile_revision: revision,
        },
        intent,
    }
}

fn visible_scope(root: &CreatedNode, child: &CreatedNode) -> WorkspaceReadScope {
    WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(root.id, None, ""),
        WorkspaceNodeProjection::new(child.id, Some(root.id), "Child"),
    ])
    .expect("scope")
}

#[test]
fn planning_is_read_only_and_checklist_root_and_child_commit_exactly() {
    let fixture = workspace();
    let root_source = source(
        fixture.root.id,
        "根 😀",
        "\r\n* [ ] 根 checklist שלום",
        "\r\n",
    );
    write_source(&fixture.root, &root_source);
    let before_tree = snapshot_tree(&fixture.root.path);
    let root_evidence = checklist_evidence(&fixture.root);
    let root_plan = plan_checklist_toggle_transaction(&fixture.root.path, &root_evidence)
        .expect("root checklist plan");
    assert_eq!(snapshot_tree(&fixture.root.path), before_tree);
    assert_eq!(
        &root_source[root_plan.source_plan().edit.range.clone()],
        "[ ]"
    );
    assert_eq!(root_plan.source_plan().edit.replacement, "[x]");

    let committed =
        commit_checklist_toggle_transaction(&root_plan).expect("commit root checklist toggle");
    let reopened = read_node_document(&fixture.root.path).expect("reopen root");
    assert_eq!(reopened.source, root_plan.source_plan().proposed_source);
    assert_eq!(reopened.revision, root_plan.summary().next_revision);
    assert_eq!(committed.document.revision, reopened.revision);
    assert_eq!(committed.summary, *root_plan.summary());
    assert!(reopened.source.ends_with("根 checklist שלום"));

    let child_source = source(fixture.child.id, "Child", "\n\n* [*] 完成 🧪 مرحبا\n", "\n");
    write_source(&fixture.child, &child_source);
    let child_plan =
        plan_checklist_toggle_transaction(&fixture.root.path, &checklist_evidence(&fixture.child))
            .expect("child checklist plan");
    commit_checklist_toggle_transaction(&child_plan).expect("commit child checklist");
    assert!(
        read_node_document(&fixture.child.path)
            .expect("child reopen")
            .source
            .contains("* [ ] 完成 🧪 مرحبا")
    );
}

#[test]
fn checklist_stale_tamper_repeat_and_locator_change_fail_closed() {
    let fixture = workspace();
    let child_source = source(
        fixture.child.id,
        "Child",
        "\n\n* [ ] exact evidence\n",
        "\n",
    );
    write_source(&fixture.child, &child_source);
    let evidence = checklist_evidence(&fixture.child);
    let mut tampered = evidence.clone();
    tampered.occurrence.parser_ordinal_path.push(99);
    assert!(matches!(
        plan_checklist_toggle_transaction(&fixture.root.path, &tampered),
        Err(TaskActionTransactionError::ChecklistToggle(
            ChecklistToggleError::EvidenceMismatch
        ))
    ));

    let stale_plan =
        plan_checklist_toggle_transaction(&fixture.root.path, &evidence).expect("stale candidate");
    fs::write(
        &fixture.child.document_path,
        child_source.clone() + "external\n",
    )
    .expect("external write");
    assert!(matches!(
        commit_checklist_toggle_transaction(&stale_plan),
        Err(TaskActionTransactionError::ChecklistToggle(
            ChecklistToggleError::StaleDocumentRevision
        ))
    ));
    assert_eq!(
        fs::read_to_string(&fixture.child.document_path).expect("external bytes"),
        child_source.clone() + "external\n"
    );

    write_source(&fixture.child, &child_source);
    let repeat_plan =
        plan_checklist_toggle_transaction(&fixture.root.path, &checklist_evidence(&fixture.child))
            .expect("repeat plan");
    commit_checklist_toggle_transaction(&repeat_plan).expect("first commit");
    assert!(matches!(
        commit_checklist_toggle_transaction(&repeat_plan),
        Err(TaskActionTransactionError::ChecklistToggle(
            ChecklistToggleError::StaleDocumentRevision
        ))
    ));

    write_source(&fixture.child, &child_source);
    let moved_plan =
        plan_checklist_toggle_transaction(&fixture.root.path, &checklist_evidence(&fixture.child))
            .expect("moved plan");
    let moved_directory = fixture.root.path.join("Renamed");
    fs::rename(&fixture.child.path, &moved_directory).expect("move directory");
    fs::rename(
        moved_directory.join("Child.adoc"),
        moved_directory.join("Renamed.adoc"),
    )
    .expect("rename canonical document");
    assert!(matches!(
        commit_checklist_toggle_transaction(&moved_plan),
        Err(TaskActionTransactionError::ReviewedPlanMismatch)
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn task_state_priority_temporal_and_no_op_commit_with_typed_results() {
    let fixture = workspace();
    let task_source = source(
        fixture.child.id,
        "发布 😀 שלום",
        concat!(
            ":weftext-task: v1\n",
            ":weftext-task-state: todo\n",
            "\nBody مرحبا\n",
        ),
        "\n",
    );
    write_source(&fixture.child, &task_source);

    let state_request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetState {
            state: TaskNodeState::Completed,
            closed: TaskNodeClosedEdit::Preserve,
        },
    );
    let state_plan =
        plan_task_node_edit_transaction(&fixture.root.path, &state_request).expect("state plan");
    let state_commit =
        commit_task_node_edit_transaction(&state_plan).expect("state transaction commit");
    assert_eq!(state_commit.summary.after.state, TaskNodeState::Completed);
    assert_eq!(state_commit.summary.after.closed, None);
    assert_eq!(
        state_commit.document.revision,
        state_plan.summary().next_revision
    );
    assert_eq!(
        read_node_document(&fixture.child.path)
            .expect("state reopen")
            .revision,
        state_commit.document.revision
    );

    let priority_request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::High),
        },
    );
    let priority_plan = plan_task_node_edit_transaction(&fixture.root.path, &priority_request)
        .expect("priority plan");
    commit_task_node_edit_transaction(&priority_plan).expect("priority commit");

    let due = TaskNodeTemporal::parse("2026-08-25T09:10:11+08:00").expect("due");
    let due_request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetTemporal {
            field: weftext_core::TaskNodeTemporalField::Due,
            value: Some(due.clone()),
        },
    );
    let due_plan =
        plan_task_node_edit_transaction(&fixture.root.path, &due_request).expect("due plan");
    let due_commit = commit_task_node_edit_transaction(&due_plan).expect("due commit");
    assert_eq!(due_commit.summary.after.due, Some(due));

    let before_no_op = fs::read(&fixture.child.document_path).expect("before no-op");
    let no_op_request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::High),
        },
    );
    let no_op_plan =
        plan_task_node_edit_transaction(&fixture.root.path, &no_op_request).expect("no-op plan");
    assert!(no_op_plan.source_plan().edits.is_empty());
    assert_eq!(
        no_op_plan.summary().base_revision,
        no_op_plan.summary().next_revision
    );
    let no_op_commit =
        commit_task_node_edit_transaction(&no_op_plan).expect("verified no-op commit");
    assert_eq!(
        fs::read(&fixture.child.document_path).expect("after no-op"),
        before_no_op
    );
    assert_eq!(
        no_op_commit.document.revision,
        no_op_plan.summary().base_revision
    );
}

#[test]
fn task_root_invalid_profile_and_stale_profile_are_rejected() {
    let fixture = workspace();
    let root_task = source(
        fixture.root.id,
        "Root",
        ":weftext-task: v1\n:weftext-task-state: todo\n",
        "\n",
    );
    write_source(&fixture.root, &root_task);
    let root_request = task_request(
        &fixture.root,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::Normal),
        },
    );
    assert!(matches!(
        plan_task_node_edit_transaction(&fixture.root.path, &root_request),
        Err(TaskActionTransactionError::RootTaskIneligible)
    ));

    let invalid = source(
        fixture.child.id,
        "Invalid",
        ":weftext-task: v1\n:weftext-task-state: waiting\n",
        "\n",
    );
    write_source(&fixture.child, &invalid);
    let invalid_request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::High),
        },
    );
    assert!(matches!(
        plan_task_node_edit_transaction(&fixture.root.path, &invalid_request),
        Err(TaskActionTransactionError::TaskNodeEdit(
            TaskNodeEditError::InvalidCurrentProfile { .. }
        ))
    ));

    let valid = source(
        fixture.child.id,
        "Valid",
        ":weftext-task: v1\n:weftext-task-state: todo\n",
        "\n",
    );
    write_source(&fixture.child, &valid);
    let request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::High),
        },
    );
    let plan = plan_task_node_edit_transaction(&fixture.root.path, &request).expect("task plan");
    fs::write(
        &fixture.child.document_path,
        valid.clone() + "changed body\n",
    )
    .expect("change body");
    assert!(matches!(
        commit_task_node_edit_transaction(&plan),
        Err(TaskActionTransactionError::TaskNodeEdit(
            TaskNodeEditError::StaleDocumentRevision
        ))
    ));
}

#[test]
fn scoped_hidden_missing_and_poisoned_targets_are_indistinguishable() {
    let fixture = workspace();
    let hidden = create_child_node(&fixture.root.path, "Hidden").expect("hidden child");
    let visible_source = source(fixture.child.id, "Visible", "\n\n* [ ] visible\n", "\n");
    let hidden_source = source(hidden.id, "Hidden", "\n\n* [ ] hidden\n", "\n");
    write_source(&fixture.child, &visible_source);
    write_source(&hidden, &hidden_source);
    let hidden_evidence = checklist_evidence(&hidden);
    let mut poisoned = hidden_source.into_bytes();
    poisoned.push(0xff);
    fs::write(&hidden.document_path, poisoned).expect("poison hidden body");

    let scope = visible_scope(&fixture.root, &fixture.child);
    let hidden_error =
        plan_checklist_toggle_transaction_scoped(&fixture.root.path, &hidden_evidence, &scope)
            .expect_err("hidden target");
    let mut missing_evidence = hidden_evidence;
    missing_evidence.owner_node_id = NodeId::new_v4();
    let missing_error =
        plan_checklist_toggle_transaction_scoped(&fixture.root.path, &missing_evidence, &scope)
            .expect_err("missing target");
    assert!(matches!(
        hidden_error,
        TaskActionTransactionError::TargetUnavailable
    ));
    assert!(matches!(
        missing_error,
        TaskActionTransactionError::TargetUnavailable
    ));
    assert_eq!(hidden_error.to_string(), missing_error.to_string());
    assert_eq!(format!("{hidden_error:?}"), format!("{missing_error:?}"));

    let visible_plan = plan_checklist_toggle_transaction_scoped(
        &fixture.root.path,
        &checklist_evidence(&fixture.child),
        &scope,
    )
    .expect("visible plan despite poisoned hidden body");
    commit_checklist_toggle_transaction_scoped(&visible_plan, &scope)
        .expect("visible scoped commit");
}

#[test]
fn scoped_scope_changes_and_owner_cross_commit_fail_before_writing() {
    let fixture = workspace();
    let other = create_child_node(&fixture.root.path, "Other").expect("other child");
    let task_source = source(
        fixture.child.id,
        "Task",
        ":weftext-task: v1\n:weftext-task-state: todo\n",
        "\n",
    );
    write_source(&fixture.child, &task_source);
    let scope = visible_scope(&fixture.root, &fixture.child);
    let request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::Highest),
        },
    );
    let plan = plan_task_node_edit_transaction_scoped(&fixture.root.path, &request, &scope)
        .expect("scoped plan");
    let plan_debug = format!("{plan:?}");
    assert!(!plan_debug.contains(fixture.root.path.to_string_lossy().as_ref()));
    assert!(!plan_debug.contains("Child.adoc"));
    let owner_plan = plan_task_node_edit_transaction(&fixture.root.path, &request)
        .expect("owner-authorized plan");
    let before = fs::read(&fixture.child.document_path).expect("before bytes");

    assert!(matches!(
        commit_task_node_edit_transaction(&plan),
        Err(TaskActionTransactionError::AuthorizationChanged)
    ));
    assert!(matches!(
        commit_task_node_edit_transaction_scoped(&owner_plan, &scope),
        Err(TaskActionTransactionError::AuthorizationChanged)
    ));
    let shrunk = WorkspaceReadScope::new([WorkspaceNodeProjection::new(fixture.root.id, None, "")])
        .expect("shrunk");
    let expanded = WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(fixture.root.id, None, ""),
        WorkspaceNodeProjection::new(fixture.child.id, Some(fixture.root.id), "Child"),
        WorkspaceNodeProjection::new(other.id, Some(fixture.root.id), "Other"),
    ])
    .expect("expanded");
    let relocated = WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(fixture.root.id, None, ""),
        WorkspaceNodeProjection::new(fixture.child.id, Some(fixture.root.id), "Renamed"),
    ])
    .expect("relocated");
    for changed_scope in [&shrunk, &expanded, &relocated] {
        assert!(matches!(
            commit_task_node_edit_transaction_scoped(&plan, changed_scope),
            Err(TaskActionTransactionError::AuthorizationChanged)
        ));
        assert_eq!(
            fs::read(&fixture.child.document_path).expect("unchanged bytes"),
            before
        );
    }
    commit_task_node_edit_transaction_scoped(&plan, &scope).expect("matching scope commit");
}

#[test]
fn workspace_lease_and_unfinished_journal_block_commits() {
    let fixture = workspace();
    let child_source = source(fixture.child.id, "Child", "\n\n* [ ] guarded\n", "\n");
    write_source(&fixture.child, &child_source);
    let plan =
        plan_checklist_toggle_transaction(&fixture.root.path, &checklist_evidence(&fixture.child))
            .expect("guarded plan");

    let lease = acquire_workspace_transaction_lease(&fixture.root.path).expect("manual lease");
    assert!(matches!(
        commit_checklist_toggle_transaction(&plan),
        Err(TaskActionTransactionError::RecoveryRequired)
    ));
    assert_eq!(
        fs::read_to_string(&fixture.child.document_path).expect("unchanged under lease"),
        child_source
    );
    drop(lease);

    let structural = plan_create_child_node(&fixture.root.path, fixture.root.id, "Pending")
        .expect("structural plan");
    prepare_workspace_transaction_recovery_fixture(&structural)
        .expect("authentic unfinished journal");
    assert!(matches!(
        commit_checklist_toggle_transaction(&plan),
        Err(TaskActionTransactionError::RecoveryRequired)
    ));
    assert_eq!(
        fs::read_to_string(&fixture.child.document_path).expect("unchanged under recovery"),
        child_source
    );
    let released = acquire_workspace_transaction_lease(&fixture.root.path)
        .expect("clean guard releases its lease on recovery failure");
    drop(released);
}

#[test]
fn identity_change_after_review_fails_closed() {
    let fixture = workspace();
    let task_source = source(
        fixture.child.id,
        "Task",
        ":weftext-task: v1\n:weftext-task-state: todo\n",
        "\n",
    );
    write_source(&fixture.child, &task_source);
    let request = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::Low),
        },
    );
    let plan =
        plan_task_node_edit_transaction(&fixture.root.path, &request).expect("identity plan");
    let replacement = NodeId::new_v4();
    let changed = task_source.replace(&fixture.child.id.to_string(), &replacement.to_string());
    write_source(&fixture.child, &changed);
    assert!(matches!(
        commit_task_node_edit_transaction(&plan),
        Err(TaskActionTransactionError::TargetUnavailable)
    ));
    assert_eq!(
        fs::read_to_string(&fixture.child.document_path).expect("changed identity bytes"),
        changed
    );
}

#[test]
fn real_trash_payload_is_never_an_ordinary_task_action_target() {
    let fixture = workspace();
    let task_source = source(
        fixture.child.id,
        "Trash-bound task",
        concat!(
            ":weftext-task: v1\n",
            ":weftext-task-state: todo\n",
            "\n* [ ] identity-free action\n",
        ),
        "\n",
    );
    write_source(&fixture.child, &task_source);
    let checklist = checklist_evidence(&fixture.child);
    let task = task_request(
        &fixture.child,
        TaskNodeEditIntent::SetPriority {
            priority: Some(TaskNodePriority::High),
        },
    );

    let trash = plan_trash_node_at(
        &fixture.root.path,
        fixture.child.id,
        "2026-08-25T12:00:00+08:00",
    )
    .expect("trash plan");
    commit_workspace_transaction(&trash).expect("trash commit");
    let trash_root = fixture.root.path.join(".weftext-trash");
    let before = snapshot_tree(&trash_root);

    assert!(matches!(
        plan_checklist_toggle_transaction(&fixture.root.path, &checklist),
        Err(TaskActionTransactionError::TargetUnavailable)
    ));
    assert!(matches!(
        plan_task_node_edit_transaction(&fixture.root.path, &task),
        Err(TaskActionTransactionError::TargetUnavailable)
    ));
    assert_eq!(snapshot_tree(&trash_root), before);
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn collect(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let mut entries = fs::read_dir(directory)
            .expect("read directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("entries");
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root).expect("relative").to_path_buf();
            if path.is_dir() {
                result.insert(relative, None);
                collect(root, &path, result);
            } else {
                result.insert(relative, Some(fs::read(&path).expect("file bytes")));
            }
        }
    }

    let mut result = BTreeMap::new();
    collect(root, root, &mut result);
    result
}
