use std::fs;

use tempfile::tempdir;
use weftext_core::{create_child_node, create_workspace, read_workspace_revision};

fn temporary_root() -> tempfile::TempDir {
    tempdir().unwrap()
}

#[test]
fn workspace_revision_covers_structure_documents_resources_and_sidecars() {
    let temporary = temporary_root();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let first = read_workspace_revision(&workspace).unwrap();

    fs::write(child.path.join("resource.bin"), b"one").unwrap();
    let with_resource = read_workspace_revision(&workspace).unwrap();
    assert_ne!(with_resource, first);

    fs::write(child.path.join("resource.bin"), b"two").unwrap();
    let changed_resource = read_workspace_revision(&workspace).unwrap();
    assert_ne!(changed_resource, with_resource);

    fs::write(
        child.path.join("weftext.annotations.json"),
        b"{\"schema\":\"test\"}",
    )
    .unwrap();
    assert_ne!(
        read_workspace_revision(&workspace).unwrap(),
        changed_resource
    );
}

#[test]
fn request_owned_transaction_directories_do_not_change_the_revision() {
    let temporary = temporary_root();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let first = read_workspace_revision(&workspace).unwrap();
    let transaction = workspace.join(".__weftext-transaction-workspace-test");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("journal.json"), b"incomplete").unwrap();
    assert_eq!(read_workspace_revision(&workspace).unwrap(), first);
}

#[test]
fn invalid_workspace_cannot_acquire_a_structural_revision() {
    let temporary = temporary_root();
    let workspace = temporary.path().join("Notes");
    fs::create_dir(&workspace).unwrap();
    let error = read_workspace_revision(&workspace).unwrap_err();
    assert!(error.to_string().contains("InvalidWorkspaceGeneration"));
}
