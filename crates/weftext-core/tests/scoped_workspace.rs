use std::fs;

use tempfile::tempdir;
use weftext_core::{
    CalendarDate, CitationWorkspaceIndex, DocumentEdit, QueryAccessScope, QueryCellValue,
    QueryEvaluationContext, QueryExecutionBinding, QueryField, QueryWorkspaceIndex,
    TaskNodeTemporal, TaskWorkspaceIndex, WorkspaceNodeProjection, WorkspaceReadScope,
    analyze_query_source, commit_document_edit, commit_workspace_transaction, create_workspace,
    plan_create_child_node, plan_document_edit, read_node_document, scan_workspace,
    search_workspace_scoped,
};

#[test]
#[allow(clippy::too_many_lines)]
fn scoped_derived_indexes_never_open_hidden_bodies_or_expose_hidden_ancestors() {
    let temporary = tempdir().expect("temporary workspace");
    let root = temporary.path().join("Workspace");
    let workspace = create_workspace(&root).expect("workspace");
    let hidden_plan = plan_create_child_node(&root, workspace.id, "Hidden").expect("hidden plan");
    let hidden = *hidden_plan.generated_node_ids.first().expect("hidden node");
    commit_workspace_transaction(&hidden_plan).expect("hidden commit");
    let visible_plan = plan_create_child_node(&root, hidden, "Visible").expect("visible plan");
    let visible = *visible_plan
        .generated_node_ids
        .first()
        .expect("visible node");
    commit_workspace_transaction(&visible_plan).expect("visible commit");

    let visible_directory = root.join("Hidden/Visible");
    let visible_snapshot = read_node_document(&visible_directory).expect("visible snapshot");
    let visible_source = format!(
        concat!(
            "{}= Visible\n",
            ":weftext-task: v1\n",
            ":weftext-task-state: todo\n",
            ":weftext-task-depends-on: {}\n\n",
            "visible-token\n\n",
            "* [ ] authorized task\n",
        ),
        visible_snapshot.source, hidden,
    );
    let edit = plan_document_edit(
        &visible_directory,
        &visible_snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: visible_snapshot.source.len() as u64,
            replacement: visible_source,
        }],
    )
    .expect("visible edit");
    commit_document_edit(&edit).expect("visible commit");

    // Keep the hidden node's valid YAML envelope but make its body invalid
    // UTF-8. Metadata inventory remains possible; any accidental body read by
    // a scoped index now fails the test deterministically.
    let hidden_document = root.join("Hidden/Hidden.adoc");
    let mut hidden_bytes = fs::read(&hidden_document).expect("hidden bytes");
    hidden_bytes.extend_from_slice(b"= Hidden\nsecret-parent-token\n");
    hidden_bytes.push(0xff);
    fs::write(&hidden_document, hidden_bytes).expect("poison hidden body");
    assert!(scan_workspace(&root).is_valid());
    assert!(read_node_document(root.join("Hidden")).is_err());

    let scope = WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(workspace.id, None, ""),
        WorkspaceNodeProjection::new(visible, None, "Visible"),
    ])
    .expect("authorized projection");

    let search = search_workspace_scoped(&root, "visible-token", &scope).expect("scoped search");
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].id, visible);
    assert_eq!(search[0].path, "Visible");
    assert!(!format!("{search:?}").contains("Hidden"));

    let tasks = TaskWorkspaceIndex::rebuild_scoped(&root, &scope).expect("scoped tasks");
    assert_eq!(tasks.occurrences_for_node(visible).count(), 1);
    assert_eq!(tasks.occurrences_for_node(hidden).count(), 0);

    CitationWorkspaceIndex::rebuild_scoped(&root, &scope).expect("scoped citations");

    let queries = QueryWorkspaceIndex::rebuild_scoped(&root, &scope).expect("scoped query index");
    let analysis = analyze_query_source(
        "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.id, node.name, node.path, node.parent_id, node.depth\norder by node.path asc\nlimit 100\n....\n",
    );
    let plan = analysis.blocks[0].plan.as_ref().expect("query plan");
    let access = QueryAccessScope::complete(scope.node_ids());
    let context = QueryEvaluationContext::new(
        CalendarDate::new(2026, 8, 24).expect("date"),
        TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("instant"),
        "Asia/Shanghai".to_owned(),
        "zh-CN".to_owned(),
        QueryExecutionBinding {
            node_id: None,
            heading: None,
        },
    )
    .expect("context");
    let result = queries.execute(plan, &access, &context).expect("query");
    let visible_row = result
        .rows
        .iter()
        .find(|row| {
            row.cells.iter().any(|cell| {
                cell.column.field == QueryField::Id
                    && cell.value == QueryCellValue::Uuid(visible.to_string())
            })
        })
        .expect("visible row");
    assert!(visible_row.cells.iter().any(|cell| {
        cell.column.field == QueryField::Path
            && cell.value == QueryCellValue::Text("/Visible".to_owned())
    }));
    assert!(visible_row.cells.iter().any(|cell| {
        cell.column.field == QueryField::ParentId && cell.value == QueryCellValue::Null
    }));
    assert!(visible_row.cells.iter().any(|cell| {
        cell.column.field == QueryField::Depth && cell.value == QueryCellValue::Integer(0)
    }));
    assert!(!format!("{result:?}").contains("Hidden"));

    let task_analysis = analyze_query_source(
        "[.weftext-query,version=1,view=task-list]\n....\nfrom tasks as task\nscope subtree(this.node)\nwhere true\nselect task.kind, task.id as task_id, task.owner_node.id as owner_node_id, task.owner_node.name, task.owner_node.path, task.blocked\norder by task.kind asc\nlimit 100\n....\n",
    );
    let task_plan = task_analysis.blocks[0].plan.as_ref().expect("task plan");
    let task_context = QueryEvaluationContext::new(
        CalendarDate::new(2026, 8, 24).expect("date"),
        TaskNodeTemporal::parse("2026-08-24T09:30:00+08:00").expect("instant"),
        "Asia/Shanghai".to_owned(),
        "zh-CN".to_owned(),
        QueryExecutionBinding {
            node_id: Some(visible),
            heading: None,
        },
    )
    .expect("task context");
    let task_result = queries
        .execute(task_plan, &access, &task_context)
        .expect("scoped task query");
    assert_eq!(task_result.rows.len(), 2);
    let task_diagnostics = queries.task_diagnostics(&access);
    let task_projection = format!("{task_result:?}{task_diagnostics:?}");
    assert!(!task_projection.contains(&hidden.to_string()));
    assert!(!task_projection.contains("Hidden"));
    assert!(!task_projection.contains("secret-parent-token"));
    assert!(task_diagnostics.iter().any(|diagnostic| {
        diagnostic.node_id == visible
            && diagnostic.dependency_id.is_none()
            && diagnostic.related_node_ids.is_empty()
    }));
}

#[test]
fn scoped_indexes_ignore_invalid_metadata_wholly_outside_the_projection() {
    let temporary = tempdir().expect("temporary workspace");
    let root = temporary.path().join("Workspace");
    let workspace = create_workspace(&root).expect("workspace");
    let hidden_plan = plan_create_child_node(&root, workspace.id, "Hidden").expect("hidden plan");
    let hidden = *hidden_plan.generated_node_ids.first().unwrap();
    commit_workspace_transaction(&hidden_plan).expect("hidden commit");
    let visible_plan = plan_create_child_node(&root, hidden, "Visible").expect("visible plan");
    let visible = *visible_plan.generated_node_ids.first().unwrap();
    commit_workspace_transaction(&visible_plan).expect("visible commit");

    let visible_directory = root.join("Hidden/Visible");
    let visible_snapshot = read_node_document(&visible_directory).unwrap();
    let visible_source = format!("{}visible-metadata-scope\n", visible_snapshot.source);
    let edit = plan_document_edit(
        &visible_directory,
        &visible_snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: visible_snapshot.source.len() as u64,
            replacement: visible_source,
        }],
    )
    .unwrap();
    commit_document_edit(&edit).unwrap();

    let hidden_document = root.join("Hidden/Hidden.adoc");
    let hidden_source = fs::read_to_string(&hidden_document).unwrap();
    fs::write(
        &hidden_document,
        hidden_source.replace(&hidden.to_string(), "not-a-node-id"),
    )
    .unwrap();
    let inventory = scan_workspace(&root);
    assert!(!inventory.is_valid());

    let scope = WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(workspace.id, None, ""),
        WorkspaceNodeProjection::new(visible, None, "Visible"),
    ])
    .unwrap();
    scope
        .validate_inventory(&inventory)
        .expect("hidden metadata issue is outside projection");
    assert_eq!(
        search_workspace_scoped(&root, "visible-metadata-scope", &scope)
            .unwrap()
            .len(),
        1
    );
    TaskWorkspaceIndex::rebuild_scoped(&root, &scope).unwrap();
    QueryWorkspaceIndex::rebuild_scoped(&root, &scope).unwrap();
    CitationWorkspaceIndex::rebuild_scoped(&root, &scope).unwrap();
}

#[test]
fn scoped_inventory_ignores_hidden_duplicates_but_rejects_a_duplicate_selected_id() {
    let temporary = tempdir().expect("temporary workspace");
    let root = temporary.path().join("Workspace");
    let workspace = create_workspace(&root).expect("workspace");
    let ancestor_plan =
        plan_create_child_node(&root, workspace.id, "HiddenA").expect("hidden A plan");
    let ancestor_id = ancestor_plan.generated_node_ids[0];
    commit_workspace_transaction(&ancestor_plan).unwrap();
    let visible_plan = plan_create_child_node(&root, ancestor_id, "Visible").unwrap();
    let visible = visible_plan.generated_node_ids[0];
    commit_workspace_transaction(&visible_plan).unwrap();
    let duplicate_container_plan = plan_create_child_node(&root, workspace.id, "HiddenB").unwrap();
    commit_workspace_transaction(&duplicate_container_plan).unwrap();

    let hidden_b_document = root.join("HiddenB/HiddenB.adoc");
    let hidden_b_source = fs::read_to_string(&hidden_b_document).unwrap();
    let duplicate_container_id = duplicate_container_plan.generated_node_ids[0];
    fs::write(
        &hidden_b_document,
        hidden_b_source.replace(
            &duplicate_container_id.to_string(),
            &ancestor_id.to_string(),
        ),
    )
    .unwrap();
    let scope = WorkspaceReadScope::new([
        WorkspaceNodeProjection::new(workspace.id, None, ""),
        WorkspaceNodeProjection::new(visible, None, "Visible"),
    ])
    .unwrap();
    let inventory = scan_workspace(&root);
    assert!(!inventory.is_valid());
    scope
        .validate_inventory(&inventory)
        .expect("duplicate IDs are wholly hidden");
    QueryWorkspaceIndex::rebuild_scoped(&root, &scope).unwrap();

    let hidden_b_source = fs::read_to_string(&hidden_b_document).unwrap();
    fs::write(
        &hidden_b_document,
        hidden_b_source.replace(&ancestor_id.to_string(), &visible.to_string()),
    )
    .unwrap();
    let inventory = scan_workspace(&root);
    assert!(scope.validate_inventory(&inventory).is_err());
    assert!(QueryWorkspaceIndex::rebuild_scoped(&root, &scope).is_err());
}
