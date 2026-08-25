use std::fs;

use weftext_core::{
    AdjacentHeadingBody, FrontmatterError, NodeMetadataScope, SortDirection, SortMode,
    commit_workspace_transaction, create_child_node, create_workspace, plan_node_icon_setting,
    project_node_metadata, read_node_document,
};

#[test]
fn projects_one_typed_canonical_envelope_without_normalizing_future_fields() {
    let source = "---\r\nweftext:\r\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n  icon: 'weftext:future-token'\r\n  aliases:\r\n    - 文缕\r\n    - 'Weftext Notes'\r\n  child_sort: manual\r\n  adjacent_heading_body: run_in\r\n  future:\r\n    exact: [preserved]\r\n---\r\n= 文缕\r\n:status: draft\r\n";

    let projection = project_node_metadata(source, NodeMetadataScope::WorkspaceRoot).unwrap();
    assert_eq!(projection.schema, "weftext.node-metadata.v1");
    assert_eq!(
        projection.id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(projection.icon.as_deref(), Some("weftext:future-token"));
    assert_eq!(projection.resolved_icon, None);
    assert_eq!(projection.aliases, ["文缕", "Weftext Notes"]);
    assert_eq!(projection.child_sort, SortMode::Manual);
    assert_eq!(projection.child_sort_direction, SortDirection::Ascending);
    assert_eq!(projection.sibling_rank, None);
    assert_eq!(
        projection.adjacent_heading_body,
        Some(AdjacentHeadingBody::RunIn)
    );
    assert_eq!(projection.diagnostics.len(), 1);
    assert_eq!(projection.diagnostics[0].field, "future");

    assert_eq!(
        project_node_metadata(source, NodeMetadataScope::Node),
        Err(FrontmatterError::WorkspaceSettingOutsideRoot)
    );

    let root_rank = source.replacen(
        "  child_sort: manual\r\n",
        "  child_sort: manual\r\n  sibling_rank: 2048\r\n",
        1,
    );
    assert_eq!(
        project_node_metadata(&root_rank, NodeMetadataScope::WorkspaceRoot),
        Err(FrontmatterError::SiblingRankOnWorkspaceRoot)
    );
}

#[test]
fn icon_uses_the_stored_revision_checked_node_metadata_transaction() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().join("工作区");
    let root = create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "节点").unwrap();
    let child_directory = workspace.join("节点");
    let document_path = child_directory.join("节点.adoc");
    let before = fs::read(&document_path).unwrap();
    let snapshot = read_node_document(&child_directory).unwrap();

    let plan =
        plan_node_icon_setting(&workspace, child.id, &snapshot.revision, Some("🧭")).unwrap();
    assert_eq!(
        fs::read(&document_path).unwrap(),
        before,
        "preview is read-only"
    );
    commit_workspace_transaction(&plan).unwrap();

    let committed = read_node_document(&child_directory).unwrap();
    let projection = project_node_metadata(&committed.source, NodeMetadataScope::Node).unwrap();
    assert_eq!(projection.icon.as_deref(), Some("🧭"));
    assert_eq!(projection.resolved_icon.unwrap().glyph, "🧭");
    assert_eq!(projection.id, child.id);
    assert_ne!(projection.id, root.id);

    assert!(
        commit_workspace_transaction(&plan).is_err(),
        "the preview cannot be replayed after its base revision changed"
    );
}
