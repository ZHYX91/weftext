use std::fs;

use weftext_core::{
    DocumentEdit, DocumentError, InventoryIssueCode, WorkspaceError, commit_document_edit,
    create_child_node, create_workspace, plan_document_edit, read_node_document, scan_workspace,
};

#[test]
fn inventory_rejects_explicit_default_adjacent_heading_body_on_a_child() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).unwrap();
    create_child_node(&workspace, "Child").unwrap();

    let child_document = workspace.join("Child/Child.adoc");
    let source = fs::read_to_string(&child_document).unwrap();
    let invalid = source.replacen("  id:", "  adjacent_heading_body: separate\n  id:", 1);
    fs::write(&child_document, invalid).unwrap();

    let inventory = scan_workspace(&workspace);
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == InventoryIssueCode::WorkspaceSettingOutsideRoot
            && issue.path == child_document
    }));
    assert!(!inventory.is_valid());
}

#[test]
fn direct_document_edit_cannot_write_root_only_metadata_to_a_child() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).unwrap();
    create_child_node(&workspace, "Child").unwrap();

    let child_directory = workspace.join("Child");
    let snapshot = read_node_document(&child_directory).unwrap();
    let replacement =
        snapshot
            .source
            .replacen("  id:", "  adjacent_heading_body: separate\n  id:", 1);
    let error = plan_document_edit(
        &child_directory,
        &snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: snapshot.source.len() as u64,
            replacement,
        }],
    )
    .expect_err("child metadata must remain root-scoped");

    assert!(matches!(
        error,
        DocumentError::InvalidMetadata(message)
            if message.contains("adjacent_heading_body")
                && message.contains("workspace root")
    ));
    assert_eq!(
        fs::read_to_string(child_directory.join("Child.adoc")).unwrap(),
        snapshot.source,
        "a rejected preview must not write any bytes"
    );
}

#[test]
fn a_nested_format_marker_cannot_promote_a_child_to_workspace_root_scope() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).unwrap();
    create_child_node(&workspace, "Child").unwrap();

    let child_directory = workspace.join("Child");
    let snapshot = read_node_document(&child_directory).unwrap();
    let valid_plan = plan_document_edit(
        &child_directory,
        &snapshot.revision,
        [DocumentEdit {
            start: snapshot.source.len() as u64,
            end: snapshot.source.len() as u64,
            replacement: "\nA valid body edit.\n".to_owned(),
        }],
    )
    .unwrap();
    fs::write(
        child_directory.join(".weftext-format"),
        b"weftext.asciidoc.v1\n",
    )
    .unwrap();

    let commit_error = commit_document_edit(&valid_plan)
        .expect_err("commit must revalidate unique workspace-root authority");
    assert!(matches!(
        commit_error,
        DocumentError::InvalidWorkspaceFormat(_)
    ));
    assert_eq!(
        fs::read_to_string(child_directory.join("Child.adoc")).unwrap(),
        snapshot.source,
        "a rejected commit must not write any document bytes"
    );

    let error = plan_document_edit(
        &child_directory,
        &snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: snapshot.source.len() as u64,
            replacement: snapshot.source.replacen(
                "  id:",
                "  adjacent_heading_body: run_in\n  id:",
                1,
            ),
        }],
    )
    .expect_err("a nested marker must not grant root-only metadata authority");

    assert!(matches!(error, DocumentError::InvalidWorkspaceFormat(_)));
    let inventory = scan_workspace(&workspace);
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == InventoryIssueCode::InvalidWorkspaceGeneration
            && issue.path == child_directory.join(".weftext-format")
    }));
    let forged_root_inventory = scan_workspace(&child_directory);
    assert!(forged_root_inventory.issues.iter().any(|issue| {
        issue.code == InventoryIssueCode::InvalidWorkspaceGeneration
            && issue.path == workspace.join(".weftext-format")
    }));
    assert!(!forged_root_inventory.is_valid());
}

#[test]
fn inventory_and_direct_edits_reject_sibling_rank_on_the_workspace_root() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).unwrap();

    let snapshot = read_node_document(&workspace).unwrap();
    let replacement = snapshot
        .source
        .replacen("  id:", "  sibling_rank: 1024\n  id:", 1);
    let error = plan_document_edit(
        &workspace,
        &snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: snapshot.source.len() as u64,
            replacement: replacement.clone(),
        }],
    )
    .expect_err("the workspace root has no sibling rank authority");
    assert!(matches!(
        error,
        DocumentError::InvalidMetadata(message)
            if message.contains("sibling_rank") && message.contains("non-root")
    ));
    assert_eq!(
        fs::read_to_string(workspace.join("Workspace.adoc")).unwrap(),
        snapshot.source,
        "a rejected preview must not write any bytes"
    );

    fs::write(workspace.join("Workspace.adoc"), replacement).unwrap();
    let inventory = scan_workspace(&workspace);
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == InventoryIssueCode::InvalidMetadata
            && issue.path == workspace.join("Workspace.adoc")
            && issue.message.contains("sibling_rank")
    }));
    assert!(!inventory.is_valid());
}

#[test]
fn workspace_creation_cannot_write_a_nested_root_marker() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).unwrap();

    let nested = workspace.join("Nested");
    let error = create_workspace(&nested).expect_err("nested workspace roots must be rejected");
    assert!(matches!(
        error,
        WorkspaceError::InvalidParent(message) if message.contains("format marker")
    ));
    assert!(!nested.exists());
    assert!(scan_workspace(&workspace).is_valid());
}
