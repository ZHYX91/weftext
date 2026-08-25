use std::fs;

use tempfile::tempdir;
use weftext_core::{
    DocumentError, InventoryIssueCode, WorkspaceContentKind, WorkspaceItemIcon,
    WorkspaceItemIconFallback, WorkspaceTransactionError, create_child_node, create_workspace,
    derive_workspace_item_icon, plan_create_child_node, plan_import_resource, plan_move_node,
    plan_trash_node_at, read_node_document, read_workspace_revision,
    rebuild_workspace_search_index, scan_workspace, search_workspace, search_workspace_index,
};

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/content-boundary-v02")
}

#[test]
fn shared_fixture_classifies_mixed_content_without_reentering_unmanaged_subtrees() {
    let root = fixture();
    let before = fs::read(root.join("loose.md")).expect("loose bytes");
    let inventory = scan_workspace(&root);
    assert!(inventory.is_valid(), "{:?}", inventory.issues);
    assert_eq!(inventory.nodes.len(), 2);
    assert!(
        inventory
            .nodes
            .iter()
            .all(|node| node.name != "LooksLikeNode")
    );
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::ManagedNode
            && entry.relative_path == "Managed"
            && entry.parent_relative_path.as_deref() == Some("")
            && entry.node_id.is_some()
    }));
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::UnmanagedMarkdown && entry.relative_path == "loose.md"
    }));
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::UnmanagedDirectory
            && entry.relative_path == "Files/nested/LooksLikeNode"
    }));
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::UnmanagedMarkdown
            && entry.relative_path == "Files/nested/LooksLikeNode/LooksLikeNode.md"
            && entry.node_id.is_none()
            && entry.owner_node_id.is_none()
    }));
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::Resource
            && entry.relative_path == "resource.bin"
            && entry.owner_node_id.is_some()
    }));
    assert!(
        inventory
            .content
            .iter()
            .all(|entry| !entry.relative_path.starts_with("ignored"))
    );
    assert!(inventory.content.iter().all(|entry| {
        entry.relative_path != "content-boundary-v02.adoc"
            && entry.relative_path != "Managed/Managed.adoc"
    }));
    assert_eq!(
        fs::read(root.join("loose.md")).expect("loose bytes after"),
        before
    );
    assert!(inventory.content.iter().all(|entry| {
        entry.kind == WorkspaceContentKind::ManagedNode || entry.node_id.is_none()
    }));
    assert!(!root.join("Files/weftext.annotations.json").exists());
}

#[test]
fn node_annotation_sidecar_is_reserved_but_unmanaged_namesake_remains_visible() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    let root_id = create_workspace(&root).expect("workspace").id;
    fs::write(
        root.join("weftext.annotations.json"),
        format!("{{\"version\":3,\"document_id\":\"{root_id}\",\"annotations\":[]}}\n"),
    )
    .expect("annotation sidecar");
    fs::create_dir(root.join("Files")).expect("unmanaged directory");
    fs::write(
        root.join("Files/weftext.annotations.json"),
        b"ordinary unmanaged bytes",
    )
    .expect("unmanaged namesake");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged Files/\n",
    )
    .expect("rules");

    let inventory = scan_workspace(&root);
    assert!(inventory.is_valid(), "{:?}", inventory.issues);
    assert!(
        inventory
            .content
            .iter()
            .all(|entry| entry.relative_path != "weftext.annotations.json")
    );
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::Resource
            && entry.relative_path == "Files/weftext.annotations.json"
            && entry.owner_node_id.is_none()
    }));
}

#[test]
fn content_rules_cannot_reclassify_the_reserved_annotation_sidecar() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    let root_id = create_workspace(&root).expect("workspace").id;
    fs::write(
        root.join("weftext.annotations.json"),
        format!("{{\"version\":3,\"document_id\":\"{root_id}\",\"annotations\":[]}}\n"),
    )
    .expect("annotation sidecar");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged weftext.annotations.json\n",
    )
    .expect("rules");

    let inventory = scan_workspace(&root);
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == InventoryIssueCode::InvalidContentRules
            && issue.path == root.join("weftext.annotations.json")
    }));
    assert!(
        inventory
            .content
            .iter()
            .all(|entry| !entry.relative_path.ends_with("weftext.annotations.json"))
    );
}

#[test]
fn node_owned_markdown_is_a_resource_without_content_rules() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    let root_id = create_workspace(&root).expect("workspace").id;
    let child = create_child_node(&root, "Child").expect("child");
    fs::write(root.join("loose.md"), b"unchanged\r\nbytes\r\n").expect("loose");
    fs::write(child.path.join("notes.md"), b"child attachment\n").expect("child attachment");
    fs::create_dir(root.join("Plain")).expect("plain directory");
    let inventory = scan_workspace(&root);
    let codes = inventory
        .issues
        .iter()
        .map(|issue| issue.code)
        .collect::<Vec<_>>();
    assert!(codes.contains(&InventoryIssueCode::MissingNodeDocument));
    assert!(!inventory.is_valid());
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::Resource
            && entry.relative_path == "loose.md"
            && entry.owner_node_id == Some(root_id)
    }));
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::Resource
            && entry.relative_path == "Child/notes.md"
            && entry.owner_node_id == Some(child.id)
    }));
    assert_eq!(
        fs::read(root.join("loose.md")).expect("bytes"),
        b"unchanged\r\nbytes\r\n"
    );
}

#[test]
fn canonical_document_cannot_be_classified_separately() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    create_workspace(&root).expect("workspace");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged Root.adoc\n",
    )
    .expect("rules");
    let inventory = scan_workspace(&root);
    assert!(
        inventory
            .issues
            .iter()
            .any(|issue| { issue.code == InventoryIssueCode::CanonicalDocumentBoundary })
    );
    assert!(matches!(
        read_node_document(&root),
        Err(DocumentError::ContentBoundary(_))
    ));
}

#[test]
fn invalid_rules_and_traversal_fail_closed_without_widening_inventory() {
    for rule in [
        "ignore ../outside",
        "ignore C:/outside",
        "unmanaged bad**glob",
    ] {
        let temporary = tempdir().expect("temporary");
        let root = temporary.path().join("Root");
        create_workspace(&root).expect("workspace");
        fs::write(
            root.join(".weftext-rules"),
            format!("weftext-content-rules-v1\n{rule}\n"),
        )
        .expect("rules");
        let inventory = scan_workspace(&root);
        assert!(inventory.nodes.is_empty());
        assert_eq!(
            inventory.issues[0].code,
            InventoryIssueCode::InvalidContentRules
        );
    }
}

#[test]
fn nested_rule_authority_cannot_override_the_workspace_root_boundary() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    create_workspace(&root).expect("workspace");
    let nested = create_child_node(&root, "Files").expect("future unmanaged directory");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged Files/\n",
    )
    .expect("root rules");
    fs::write(
        nested.path.join(".weftext-rules"),
        "weftext-content-rules-v1\n",
    )
    .expect("nested rules");
    let inventory = scan_workspace(&root);
    assert!(inventory.issues.iter().any(|issue| {
        issue.code == InventoryIssueCode::InvalidContentRules
            && issue.path == root.join("Files/.weftext-rules")
    }));
    assert!(matches!(
        read_node_document(&nested.path),
        Err(DocumentError::ContentBoundary(_))
    ));
}

#[test]
fn search_and_revision_exclude_ignored_and_unmanaged_markdown_from_node_features() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    create_workspace(&root).expect("workspace");
    fs::create_dir(root.join("ignored")).expect("ignored");
    fs::write(root.join("ignored/secret.md"), "IgnoredSearchToken").expect("ignored file");
    fs::write(root.join("loose.md"), "UnmanagedSearchToken").expect("loose");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged loose.md\nignore ignored/\n",
    )
    .expect("rules");
    assert!(
        search_workspace(&root, "IgnoredSearchToken")
            .expect("search")
            .is_empty()
    );
    assert!(
        search_workspace(&root, "UnmanagedSearchToken")
            .expect("search")
            .is_empty()
    );
    let index = temporary.path().join("derived/search.json");
    rebuild_workspace_search_index(&root, &index).expect("rebuild index");
    assert!(
        search_workspace_index(&index, "IgnoredSearchToken")
            .expect("indexed search")
            .is_empty()
    );
    assert!(
        search_workspace_index(&index, "UnmanagedSearchToken")
            .expect("indexed search")
            .is_empty()
    );
    let before = read_workspace_revision(&root).expect("revision");
    fs::write(root.join("ignored/secret.md"), "changed ignored bytes").expect("change ignored");
    assert_eq!(read_workspace_revision(&root).expect("revision"), before);
}

#[test]
fn transactions_cannot_enter_or_move_across_content_boundaries() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    let root_node = create_workspace(&root).expect("workspace");
    let managed = create_child_node(&root, "Managed").expect("managed");
    fs::write(managed.path.join("plain.md"), b"do not move").expect("plain");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nunmanaged Managed/plain.md\nunmanaged Reserved/\n",
    )
    .expect("rules");
    assert!(matches!(
        plan_move_node(&root, managed.id, root_node.id, "Managed"),
        Err(WorkspaceTransactionError::ContentBoundary(_))
    ));
    assert!(matches!(
        plan_trash_node_at(&root, managed.id, "2026-08-24T12:00:00+08:00"),
        Err(WorkspaceTransactionError::ContentBoundary(_))
    ));
    assert!(matches!(
        plan_create_child_node(&root, root_node.id, "Reserved"),
        Err(WorkspaceTransactionError::ContentBoundary(_))
    ));
    assert!(managed.path.exists());
}

#[test]
fn resource_import_cannot_create_an_ignored_or_unmanaged_target() {
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    let root_node = create_workspace(&root).expect("workspace");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nignore private.bin\nunmanaged visible.bin\n",
    )
    .expect("rules");
    for name in ["private.bin", "visible.bin"] {
        assert!(plan_import_resource(&root, root_node.id, name, vec![1, 2, 3]).is_err());
        assert!(!root.join(name).exists());
    }
}

#[test]
fn default_icon_semantics_are_derived_for_every_content_class() {
    assert_eq!(
        derive_workspace_item_icon(None, WorkspaceItemIconFallback::ManagedNode),
        WorkspaceItemIcon::DefaultNode
    );
    assert_eq!(
        derive_workspace_item_icon(None, WorkspaceItemIconFallback::UnmanagedFolder),
        WorkspaceItemIcon::Folder
    );
    assert_eq!(
        derive_workspace_item_icon(None, WorkspaceItemIconFallback::UnmanagedMarkdown),
        WorkspaceItemIcon::MarkdownFile
    );
    assert_eq!(
        derive_workspace_item_icon(None, WorkspaceItemIconFallback::OrdinaryFile),
        WorkspaceItemIcon::File
    );
}

#[cfg(unix)]
#[test]
fn ignored_symlink_is_still_a_fail_closed_boundary_diagnostic() {
    use std::os::unix::fs::symlink;
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    create_workspace(&root).expect("workspace");
    symlink(temporary.path(), root.join("ignored-link")).expect("symlink");
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nignore ignored-link/\n",
    )
    .expect("rules");
    assert!(
        scan_workspace(&root)
            .issues
            .iter()
            .any(|issue| { issue.code == InventoryIssueCode::SymlinkUnsupported })
    );
    let linked_node = temporary.path().join("LinkedNode");
    create_workspace(&linked_node).expect("linked node target");
    symlink(&linked_node, root.join("linked-node")).expect("node symlink");
    assert!(read_node_document(root.join("linked-node")).is_err());
    let alias_container = tempdir().expect("alias container");
    let alias = alias_container.path().join("workspace-alias");
    symlink(temporary.path(), &alias).expect("workspace ancestor symlink");
    assert!(
        scan_workspace(alias.join("Root"))
            .issues
            .iter()
            .any(|issue| issue.code == InventoryIssueCode::SymlinkUnsupported)
    );
}

#[cfg(windows)]
#[test]
fn ignored_junction_is_still_a_fail_closed_boundary_diagnostic() {
    use std::process::Command;
    let temporary = tempdir().expect("temporary");
    let root = temporary.path().join("Root");
    create_workspace(&root).expect("workspace");
    let target = temporary.path().join("outside");
    fs::create_dir(&target).expect("target");
    let junction = root.join("ignored-link");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .status()
        .expect("mklink");
    assert!(status.success());
    fs::write(
        root.join(".weftext-rules"),
        "weftext-content-rules-v1\nignore ignored-link/\n",
    )
    .expect("rules");
    assert!(
        scan_workspace(&root)
            .issues
            .iter()
            .any(|issue| { issue.code == InventoryIssueCode::SymlinkUnsupported })
    );
    let linked_node = temporary.path().join("linked-node-target");
    create_workspace(&linked_node).expect("linked node target");
    let node_junction = root.join("linked-node-target");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&node_junction)
        .arg(&linked_node)
        .status()
        .expect("node mklink");
    assert!(status.success());
    assert!(read_node_document(&node_junction).is_err());
    let alias_container = tempdir().expect("alias container");
    let alias = alias_container.path().join("workspace-alias");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&alias)
        .arg(temporary.path())
        .status()
        .expect("workspace ancestor mklink");
    assert!(status.success());
    assert!(
        scan_workspace(alias.join("Root"))
            .issues
            .iter()
            .any(|issue| issue.code == InventoryIssueCode::SymlinkUnsupported)
    );
}
