use std::fs;

use tempfile::{TempDir, tempdir};
use weftext_core::{
    CreatedNode, NodeId, TaskNodeActionEvidence, TaskNodeDependencyReplacementDiagnostic,
    TaskNodeDependencyReplacementDiagnosticCode, TaskNodeDependencyReplacementError,
    TaskNodeDependencyReplacementRequest, WorkspaceDraftRegistryView, WorkspaceNodeProjection,
    WorkspaceReadScope, acquire_workspace_transaction_lease,
    commit_task_node_dependency_replacement_transaction,
    commit_task_node_dependency_replacement_transaction_scoped,
    commit_task_node_dependency_replacement_transaction_with_draft_gate, create_child_node,
    create_workspace, plan_create_child_node, plan_task_node_dependency_replacement_transaction,
    plan_task_node_dependency_replacement_transaction_scoped,
    prepare_workspace_transaction_recovery_fixture, read_node_document,
};

struct Fixture {
    _temporary: TempDir,
    root: CreatedNode,
}

fn fixture() -> Fixture {
    let temporary = tempdir().expect("temporary directory");
    let root = create_workspace(temporary.path().join("Tasks")).expect("workspace");
    Fixture {
        _temporary: temporary,
        root,
    }
}

fn child(root: &CreatedNode, name: &str) -> CreatedNode {
    create_child_node(&root.path, name).expect("child")
}

fn task_source(node_id: NodeId, title: &str, state: &str, depends_on: &[NodeId]) -> String {
    let dependency = if depends_on.is_empty() {
        String::new()
    } else {
        format!(
            ":weftext-task-depends-on: {}\n",
            depends_on
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    format!(
        "---\nweftext:\n  id: \"{node_id}\"\n---\n= {title}\n:weftext-task: v1\n:weftext-task-state: {state}\n{dependency}\nBody 😀 שלום\n"
    )
}

fn write_task(node: &CreatedNode, state: &str, depends_on: &[NodeId]) {
    let name = node
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap();
    fs::write(
        &node.document_path,
        task_source(node.id, name, state, depends_on),
    )
    .expect("write task");
}

fn write_non_task(node: &CreatedNode) {
    let name = node
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap();
    fs::write(
        &node.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= {}\n\nOrdinary\n",
            node.id, name
        ),
    )
    .expect("write ordinary node");
}

fn request(source: &CreatedNode, depends_on: Vec<NodeId>) -> TaskNodeDependencyReplacementRequest {
    let revision = read_node_document(&source.path).expect("snapshot").revision;
    TaskNodeDependencyReplacementRequest {
        evidence: TaskNodeActionEvidence {
            node_id: source.id,
            revision: revision.clone(),
            profile_revision: revision,
        },
        depends_on,
    }
}

fn scope(nodes: &[(&CreatedNode, Option<NodeId>, &str)]) -> WorkspaceReadScope {
    WorkspaceReadScope::new(
        nodes.iter().map(|(node, parent, locator)| {
            WorkspaceNodeProjection::new(node.id, *parent, *locator)
        }),
    )
    .expect("scope")
}

fn graph_diagnostics(
    error: TaskNodeDependencyReplacementError,
) -> Vec<TaskNodeDependencyReplacementDiagnostic> {
    match error {
        TaskNodeDependencyReplacementError::InvalidProposedGraph { diagnostics } => diagnostics,
        other => panic!("expected graph error, got {other:?}"),
    }
}

fn bounded_node_id(index: u32) -> NodeId {
    format!("10000000-0000-4000-8000-{index:012}")
        .parse()
        .expect("bounded node ID")
}

#[test]
fn replacement_is_sorted_empty_deletes_and_canonical_no_op_has_no_transaction() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let first = child(&fixture.root, "First");
    let second = child(&fixture.root, "Second");
    write_task(&source, "todo", &[]);
    write_task(&first, "todo", &[]);
    write_task(&second, "completed", &[]);

    let unsorted = request(&source, vec![second.id, first.id]);
    let plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &unsorted)
        .expect("sorted replacement plan");
    let mut canonical = vec![first.id, second.id];
    canonical.sort_unstable();
    assert_eq!(plan.summary().after_depends_on, canonical);
    assert_eq!(plan.source_edits().len(), 1);
    commit_task_node_dependency_replacement_transaction(&plan, &unsorted).expect("commit sorted");
    let source_text = fs::read_to_string(&source.document_path).expect("source text");
    assert!(source_text.contains(&format!(
        ":weftext-task-depends-on: {} {}\n",
        canonical[0], canonical[1]
    )));

    let no_op = request(&source, canonical);
    let no_op_plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &no_op)
        .expect("no-op plan");
    assert!(no_op_plan.source_edits().is_empty());
    assert_eq!(
        no_op_plan.summary().base_revision,
        no_op_plan.summary().next_revision
    );
    let before = fs::read(&source.document_path).expect("before no-op");
    let committed = commit_task_node_dependency_replacement_transaction(&no_op_plan, &no_op)
        .expect("verified no-op");
    assert!(committed.transaction.is_none());
    assert_eq!(
        fs::read(&source.document_path).expect("after no-op"),
        before
    );

    let clear = request(&source, Vec::new());
    let clear_plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &clear)
        .expect("clear plan");
    assert_eq!(clear_plan.source_edits().len(), 1);
    commit_task_node_dependency_replacement_transaction(&clear_plan, &clear).expect("clear commit");
    assert!(
        !fs::read_to_string(&source.document_path)
            .expect("cleared source")
            .contains(":weftext-task-depends-on:")
    );
}

#[test]
fn duplicate_and_self_fail_before_dependency_document_utf8_is_opened() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let poisoned = child(&fixture.root, "Poisoned");
    write_task(&source, "todo", &[]);
    let mut poison = format!(
        "---\nweftext:\n  id: \"{}\"\n---\n= Poisoned\n\n",
        poisoned.id
    )
    .into_bytes();
    poison.push(0xff);
    fs::write(&poisoned.document_path, poison).expect("poison hidden body");

    let duplicate = request(&source, vec![poisoned.id, poisoned.id]);
    assert!(matches!(
        plan_task_node_dependency_replacement_transaction(&fixture.root.path, &duplicate),
        Err(TaskNodeDependencyReplacementError::DuplicateDependency)
    ));
    let self_edge = request(&source, vec![source.id]);
    assert!(matches!(
        plan_task_node_dependency_replacement_transaction(&fixture.root.path, &self_edge),
        Err(TaskNodeDependencyReplacementError::SelfDependency)
    ));
}

#[test]
fn dependency_value_limit_accepts_the_boundary_and_fails_before_target_body_io() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let poisoned = child(&fixture.root, "Poisoned");
    write_task(&source, "todo", &[]);
    let mut poison = format!(
        "---\nweftext:\n  id: \"{}\"\n---\n= Poisoned\n\n",
        poisoned.id
    )
    .into_bytes();
    poison.push(0xff);
    fs::write(&poisoned.document_path, poison).expect("poison target body");

    let maximum = (1..=110).map(bounded_node_id).collect::<Vec<_>>();
    let visible_source = scope(&[
        (&fixture.root, None, ""),
        (&source, Some(fixture.root.id), "Source"),
    ]);
    assert!(matches!(
        plan_task_node_dependency_replacement_transaction_scoped(
            &fixture.root.path,
            &request(&source, maximum.clone()),
            &visible_source,
        ),
        Err(TaskNodeDependencyReplacementError::DependencyUnavailable)
    ));

    let mut over_limit = maximum;
    over_limit.push(poisoned.id);
    assert!(matches!(
        plan_task_node_dependency_replacement_transaction(
            &fixture.root.path,
            &request(&source, over_limit),
        ),
        Err(TaskNodeDependencyReplacementError::DependencyLimitExceeded)
    ));
}

#[test]
fn scoped_hidden_and_missing_are_identical_and_hidden_invalid_utf8_is_not_parsed() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let hidden = child(&fixture.root, "SecretTarget");
    write_task(&source, "todo", &[]);
    write_task(&hidden, "todo", &[]);
    let visible = scope(&[
        (&fixture.root, None, ""),
        (&source, Some(fixture.root.id), "Source"),
    ]);
    let missing_id: NodeId = "99999999-9999-4999-8999-999999999999"
        .parse()
        .expect("missing ID");

    let hidden_request = request(&source, vec![hidden.id]);
    let mut errors = vec![
        plan_task_node_dependency_replacement_transaction_scoped(
            &fixture.root.path,
            &hidden_request,
            &visible,
        )
        .expect_err("healthy hidden unavailable"),
    ];
    let mut poison =
        format!("---\nweftext:\n  id: \"{}\"\n---\n= Secret\n\n", hidden.id).into_bytes();
    poison.push(0xff);
    fs::write(&hidden.document_path, poison).expect("poison hidden body");
    errors.push(
        plan_task_node_dependency_replacement_transaction_scoped(
            &fixture.root.path,
            &hidden_request,
            &visible,
        )
        .expect_err("poisoned hidden unavailable"),
    );
    fs::remove_file(&hidden.document_path).expect("remove hidden document");
    errors.push(
        plan_task_node_dependency_replacement_transaction_scoped(
            &fixture.root.path,
            &hidden_request,
            &visible,
        )
        .expect_err("missing hidden document unavailable"),
    );
    let missing_error = plan_task_node_dependency_replacement_transaction_scoped(
        &fixture.root.path,
        &request(&source, vec![missing_id]),
        &visible,
    )
    .expect_err("missing unavailable");
    errors.push(missing_error);
    let hidden_id_text = hidden.id.to_string();
    for error in errors {
        assert!(matches!(
            &error,
            TaskNodeDependencyReplacementError::DependencyUnavailable
        ));
        let display = error.to_string();
        let debug = format!("{error:?}");
        for secret in ["SecretTarget", "Secret", hidden_id_text.as_str()] {
            assert!(!display.contains(secret));
            assert!(!debug.contains(secret));
        }
    }
}

#[test]
fn visible_non_task_invalid_and_transitively_invalid_targets_are_distinct() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let ordinary = child(&fixture.root, "Ordinary");
    let invalid = child(&fixture.root, "Invalid");
    let transitive = child(&fixture.root, "Transitive");
    let missing: NodeId = "99999999-9999-4999-8999-999999999999"
        .parse()
        .expect("missing");
    write_task(&source, "todo", &[]);
    write_non_task(&ordinary);
    write_task(&invalid, "waiting", &[]);
    write_task(&transitive, "todo", &[missing]);

    let non_task = graph_diagnostics(
        plan_task_node_dependency_replacement_transaction(
            &fixture.root.path,
            &request(&source, vec![ordinary.id]),
        )
        .expect_err("non-task target"),
    );
    assert!(non_task.iter().any(|diagnostic| {
        diagnostic.code == TaskNodeDependencyReplacementDiagnosticCode::NonTaskDependency
            && diagnostic.target_node_id == Some(ordinary.id)
    }));

    let invalid_profile = graph_diagnostics(
        plan_task_node_dependency_replacement_transaction(
            &fixture.root.path,
            &request(&source, vec![invalid.id]),
        )
        .expect_err("invalid target"),
    );
    assert!(invalid_profile.iter().any(|diagnostic| {
        diagnostic.code == TaskNodeDependencyReplacementDiagnosticCode::InvalidDependencyTarget
            && diagnostic.target_node_id == Some(invalid.id)
    }));

    let reverse = graph_diagnostics(
        plan_task_node_dependency_replacement_transaction(
            &fixture.root.path,
            &request(&source, vec![transitive.id]),
        )
        .expect_err("transitive invalid target"),
    );
    assert!(reverse.iter().any(|diagnostic| {
        diagnostic.source_node_id == transitive.id
            && diagnostic.code == TaskNodeDependencyReplacementDiagnosticCode::UnresolvedDependency
            && diagnostic.target_node_id.is_none()
    }));
    assert!(reverse.iter().any(|diagnostic| {
        diagnostic.source_node_id == source.id
            && diagnostic.code
                == TaskNodeDependencyReplacementDiagnosticCode::InvalidDependencyTarget
            && diagnostic.target_node_id == Some(transitive.id)
    }));
}

#[test]
fn scoped_transitive_hidden_dependency_is_identical_when_healthy_poisoned_or_missing() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let visible_target = child(&fixture.root, "VisibleTarget");
    let hidden = child(&fixture.root, "SecretTransitive");
    write_task(&source, "todo", &[]);
    write_task(&visible_target, "todo", &[hidden.id]);
    write_task(&hidden, "todo", &[]);
    let visible = scope(&[
        (&fixture.root, None, ""),
        (&source, Some(fixture.root.id), "Source"),
        (&visible_target, Some(fixture.root.id), "VisibleTarget"),
    ]);
    let replacement = request(&source, vec![visible_target.id]);
    let analyze = || {
        let diagnostics = graph_diagnostics(
            plan_task_node_dependency_replacement_transaction_scoped(
                &fixture.root.path,
                &replacement,
                &visible,
            )
            .expect_err("transitive hidden dependency"),
        );
        serde_json::to_string(&diagnostics).expect("diagnostics JSON")
    };

    let healthy = analyze();
    let mut poison =
        format!("---\nweftext:\n  id: \"{}\"\n---\n= Secret\n\n", hidden.id).into_bytes();
    poison.push(0xff);
    fs::write(&hidden.document_path, poison).expect("poison transitive hidden body");
    let poisoned = analyze();
    fs::remove_file(&hidden.document_path).expect("remove transitive hidden body");
    let missing = analyze();
    assert_eq!(healthy, poisoned);
    assert_eq!(healthy, missing);
    let hidden_id_text = hidden.id.to_string();
    for secret in ["SecretTransitive", hidden_id_text.as_str()] {
        assert!(!healthy.contains(secret));
    }
}

#[test]
fn replacement_adds_and_removes_cycles_repairs_old_edge_and_ignores_disconnected_cycle() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let target = child(&fixture.root, "Target");
    let cycle_x = child(&fixture.root, "CycleX");
    let cycle_y = child(&fixture.root, "CycleY");
    let missing: NodeId = "99999999-9999-4999-8999-999999999999"
        .parse()
        .expect("missing");
    write_task(&source, "todo", &[missing]);
    write_task(&target, "todo", &[source.id]);
    write_task(&cycle_x, "todo", &[cycle_y.id]);
    write_task(&cycle_y, "todo", &[cycle_x.id]);

    let cycle = graph_diagnostics(
        plan_task_node_dependency_replacement_transaction(
            &fixture.root.path,
            &request(&source, vec![target.id]),
        )
        .expect_err("new source-target cycle"),
    );
    assert!(cycle.iter().any(|diagnostic| {
        diagnostic.code == TaskNodeDependencyReplacementDiagnosticCode::DependencyCycle
    }));

    write_task(&target, "todo", &[]);
    let repair = request(&source, vec![target.id]);
    let plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &repair)
        .expect("repair source while disconnected cycle remains");
    commit_task_node_dependency_replacement_transaction(&plan, &repair).expect("repair commit");
    assert_eq!(
        weftext_core::analyze_task_node_profile(
            &fs::read_to_string(&source.document_path).expect("repaired source"),
            Some(source.id),
        )
        .profile
        .expect("valid repaired profile")
        .depends_on,
        vec![target.id]
    );
}

#[test]
fn commit_rechecks_request_scope_workspace_lease_drafts_and_repeat_authority() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let target = child(&fixture.root, "Target");
    write_task(&source, "todo", &[]);
    write_task(&target, "todo", &[]);
    let reviewed = request(&source, vec![target.id]);
    let plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &reviewed)
        .expect("owner plan");

    let tampered = request(&source, Vec::new());
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction(&plan, &tampered),
        Err(TaskNodeDependencyReplacementError::AuthorizationChanged)
    ));

    let dirty = WorkspaceDraftRegistryView::new("dirty source", [source.id]).expect("registry");
    let preview = plan
        .preview_draft_gate(&dirty)
        .expect("preview")
        .expect("changing plan");
    assert_eq!(preview.required_clean_node_ids, vec![source.id]);
    assert_eq!(preview.blocking_dirty_node_ids, vec![source.id]);
    assert!(preview.executable_token.is_none());
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction_with_draft_gate(
            &plan, &reviewed, None, &dirty,
        ),
        Err(TaskNodeDependencyReplacementError::AuthorizationChanged)
    ));

    let lease = acquire_workspace_transaction_lease(&fixture.root.path).expect("held lease");
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction(&plan, &reviewed),
        Err(TaskNodeDependencyReplacementError::RecoveryRequired)
    ));
    drop(lease);

    commit_task_node_dependency_replacement_transaction(&plan, &reviewed).expect("owner commit");
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction(&plan, &reviewed),
        Err(TaskNodeDependencyReplacementError::StaleDocumentRevision
            | TaskNodeDependencyReplacementError::StaleWorkspaceRevision)
    ));

    write_task(&source, "todo", &[]);
    let scoped_request = request(&source, vec![target.id]);
    let reviewed_scope = scope(&[
        (&fixture.root, None, ""),
        (&source, Some(fixture.root.id), "Source"),
        (&target, Some(fixture.root.id), "Target"),
    ]);
    let scoped_plan = plan_task_node_dependency_replacement_transaction_scoped(
        &fixture.root.path,
        &scoped_request,
        &reviewed_scope,
    )
    .expect("scoped plan");
    let smaller_scope = scope(&[
        (&fixture.root, None, ""),
        (&source, Some(fixture.root.id), "Source"),
    ]);
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction_scoped(
            &scoped_plan,
            &scoped_request,
            &smaller_scope,
        ),
        Err(TaskNodeDependencyReplacementError::AuthorizationChanged)
    ));
    commit_task_node_dependency_replacement_transaction_scoped(
        &scoped_plan,
        &scoped_request,
        &reviewed_scope,
    )
    .expect("matching scoped commit");
}

#[test]
fn authentic_unfinished_journal_blocks_commit_without_writing_source() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let target = child(&fixture.root, "Target");
    write_task(&source, "todo", &[]);
    write_task(&target, "todo", &[]);
    let reviewed = request(&source, vec![target.id]);
    let plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &reviewed)
        .expect("dependency plan");
    let before = fs::read(&source.document_path).expect("before recovery block");

    let structural = plan_create_child_node(&fixture.root.path, fixture.root.id, "Pending")
        .expect("unrelated structural plan");
    prepare_workspace_transaction_recovery_fixture(&structural)
        .expect("authentic unfinished journal");
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction(&plan, &reviewed),
        Err(TaskNodeDependencyReplacementError::RecoveryRequired)
    ));
    assert_eq!(
        fs::read(&source.document_path).expect("unchanged under recovery"),
        before
    );
}

#[test]
fn plan_is_read_only_and_commit_rejects_stale_workspace_or_source() {
    let fixture = fixture();
    let source = child(&fixture.root, "Source");
    let target = child(&fixture.root, "Target");
    let unrelated = child(&fixture.root, "Unrelated");
    write_task(&source, "todo", &[]);
    write_task(&target, "todo", &[]);
    write_non_task(&unrelated);
    let reviewed = request(&source, vec![target.id]);
    let before = fs::read(&source.document_path).expect("source before plan");
    let plan = plan_task_node_dependency_replacement_transaction(&fixture.root.path, &reviewed)
        .expect("read-only plan");
    assert_eq!(
        fs::read(&source.document_path).expect("source after plan"),
        before
    );

    fs::write(
        &unrelated.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Unrelated\n\nExternal change\n",
            unrelated.id
        ),
    )
    .expect("unrelated workspace change");
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction(&plan, &reviewed),
        Err(TaskNodeDependencyReplacementError::StaleWorkspaceRevision)
    ));
    assert_eq!(
        fs::read(&source.document_path).expect("source after stale workspace"),
        before
    );

    let fresh_plan =
        plan_task_node_dependency_replacement_transaction(&fixture.root.path, &reviewed)
            .expect("fresh plan after unrelated change");
    fs::write(
        &source.document_path,
        task_source(source.id, "Source", "todo", &[]) + "External source change\n",
    )
    .expect("source change");
    assert!(matches!(
        commit_task_node_dependency_replacement_transaction(&fresh_plan, &reviewed),
        Err(TaskNodeDependencyReplacementError::StaleDocumentRevision
            | TaskNodeDependencyReplacementError::StaleWorkspaceRevision)
    ));
}

#[test]
fn root_is_ineligible_and_request_serde_rejects_unknown_fields() {
    let fixture = fixture();
    fs::write(
        &fixture.root.document_path,
        task_source(fixture.root.id, "Root", "todo", &[]),
    )
    .expect("root task syntax");
    let root_request = request(&fixture.root, Vec::new());
    assert!(matches!(
        plan_task_node_dependency_replacement_transaction(&fixture.root.path, &root_request),
        Err(TaskNodeDependencyReplacementError::RootTaskIneligible)
    ));

    let child = child(&fixture.root, "Child");
    write_task(&child, "todo", &[]);
    let valid = request(&child, Vec::new());
    let mut value = serde_json::to_value(&valid).expect("request JSON");
    value
        .as_object_mut()
        .expect("request object")
        .insert("unexpected".to_owned(), serde_json::json!(true));
    assert!(
        serde_json::from_value::<TaskNodeDependencyReplacementRequest>(value).is_err(),
        "request must reject unknown fields"
    );
    let mut evidence_value = serde_json::to_value(&valid).expect("evidence JSON");
    evidence_value["evidence"]["unexpected"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<TaskNodeDependencyReplacementRequest>(evidence_value).is_err(),
        "nested action evidence must reject unknown fields"
    );
}
