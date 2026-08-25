use std::fs;
use std::path::Path;

use tempfile::tempdir;
use weftext_core::{
    NAVIGATION_PROJECTION_VERSION, WorkspaceContentKind, build_workspace_navigation,
    commit_workspace_transaction, create_child_node, create_workspace, plan_trash_node_at,
    scan_workspace,
};

const TRASH_TIME: &str = "2026-08-24T00:00:00Z";

#[test]
fn mixed_fixture_produces_one_ordered_shared_navigation_projection() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/content-boundary-v02");
    let inventory = scan_workspace(&root);
    let projection = build_workspace_navigation(&inventory).expect("navigation projection");

    assert_eq!(projection.version, NAVIGATION_PROJECTION_VERSION);
    assert_eq!(projection.hierarchy.len(), 2);
    assert_eq!(projection.hierarchy[0].locator, "");
    assert_eq!(projection.hierarchy[1].locator, "Managed");
    assert!(
        projection
            .hierarchy
            .iter()
            .all(|item| item.node_id.to_string().len() == 36)
    );

    let root_rows = projection
        .contents
        .iter()
        .filter(|item| item.parent_locator.as_deref() == Some(""))
        .map(|item| (item.kind, item.locator.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        root_rows,
        vec![
            (WorkspaceContentKind::ManagedNode, "Managed"),
            (WorkspaceContentKind::UnmanagedDirectory, "Files"),
            (WorkspaceContentKind::UnmanagedMarkdown, "loose.md"),
            (WorkspaceContentKind::Resource, "resource.bin"),
        ]
    );
    let unmanaged_rows = projection
        .contents
        .iter()
        .filter(|item| item.parent_locator.as_deref() == Some("Files"))
        .map(|item| item.locator.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        unmanaged_rows,
        vec![
            "Files/nested",
            "Files/2.md",
            "Files/10.md",
            "Files/asset.txt"
        ]
    );
    assert!(projection.contents.iter().all(|item| {
        item.locator != "Managed/Managed.adoc" && !item.locator.starts_with("ignored")
    }));
}

#[test]
fn hierarchy_uses_core_natural_and_manual_child_order() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    let workspace = create_workspace(&root).unwrap();
    let ten = create_child_node(&root, "Node 10").unwrap();
    let two = create_child_node(&root, "Node 2").unwrap();

    let projection = build_workspace_navigation(&scan_workspace(&root)).unwrap();
    assert_eq!(
        projection
            .hierarchy
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Workspace", "Node 2", "Node 10"]
    );

    fs::write(
        &workspace.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n  child_sort: manual\n  child_sort_direction: ascending\n---\n",
            workspace.id
        ),
    )
    .unwrap();
    fs::write(
        &ten.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n  sibling_rank: 10\n---\n",
            ten.id
        ),
    )
    .unwrap();
    fs::write(
        &two.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n  sibling_rank: 20\n---\n",
            two.id
        ),
    )
    .unwrap();
    let projection = build_workspace_navigation(&scan_workspace(&root)).unwrap();
    assert_eq!(
        projection
            .hierarchy
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Workspace", "Node 10", "Node 2"]
    );
}

#[test]
fn trash_item_store_is_not_projected_as_ordinary_navigation() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    create_workspace(&root).unwrap();
    let kept = create_child_node(&root, "Kept").unwrap();
    let removed = create_child_node(&root, "Removed").unwrap();

    let plan = plan_trash_node_at(&root, removed.id, TRASH_TIME).unwrap();
    commit_workspace_transaction(&plan).unwrap();

    let projection = build_workspace_navigation(&scan_workspace(&root)).unwrap();
    assert_eq!(
        projection
            .hierarchy
            .iter()
            .map(|node| node.node_id)
            .collect::<Vec<_>>(),
        vec![create_workspace_id(&root), kept.id]
    );
    assert!(projection.hierarchy.iter().all(|node| {
        node.name != ".weftext-trash" && !node.locator.starts_with(".weftext-trash")
    }));
    assert!(
        projection
            .contents
            .iter()
            .all(|item| !item.locator.starts_with(".weftext-trash"))
    );
    assert_eq!(projection.hierarchy[0].child_count, 1);
}

fn create_workspace_id(root: &Path) -> weftext_core::NodeId {
    scan_workspace(root)
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id)
        .unwrap()
}
