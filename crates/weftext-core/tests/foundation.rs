use std::fs;
use std::str::FromStr;

use tempfile::{TempDir, tempdir};
use uuid::Uuid;
use weftext_core::{
    Anchor, Annotation, AnnotationAppearance, AnnotationBody, AnnotationBodyFormat,
    AnnotationColor, AnnotationKind, AnnotationMark, AnnotationState, AnnotationStore,
    CalendarDate, ChronoPeriod, ChronoPlan, InventoryIssueCode, NodeId, SortDirection, SortMode,
    SyncDisposition, ThreadMessage, WorkspaceContentKind, WorkspaceIndex, classify_sync_state,
    create_child_node, create_workspace, parse_node_metadata, scan_workspace,
};

fn digest() -> String {
    "a".repeat(64)
}

fn test_directory() -> TempDir {
    tempdir().unwrap()
}

#[test]
fn node_id_requires_canonical_uuid_v4() {
    let id = NodeId::new_v4();
    assert_eq!(NodeId::from_str(&id.to_string()).unwrap(), id);
    assert!(NodeId::from_str(&id.to_string().to_uppercase()).is_err());
    assert!(NodeId::from_str("550e8400-e29b-11d4-a716-446655440000").is_err());
}

#[test]
fn frontmatter_reads_identity_and_sparse_order_without_rewriting_yaml() {
    let id = NodeId::new_v4();
    let source = format!(
        "---\nweftext:\n  id: \"{id}\"\n  child_sort: manual\n  child_sort_direction: descending\n  sibling_rank: 2048\n---\n= Kept exactly\nbody\n"
    );
    let metadata = parse_node_metadata(&source).unwrap();
    assert_eq!(metadata.id, Some(id));
    assert_eq!(metadata.child_sort.mode, SortMode::Manual);
    assert_eq!(metadata.child_sort.direction, SortDirection::Descending);
    assert_eq!(metadata.sibling_order.rank, Some(2048));
}

#[test]
fn workspace_creation_and_inventory_enforce_x_x_asciidoc() {
    let temporary = test_directory();
    let root = temporary.path().join("Knowledge");
    let created = create_workspace(&root).unwrap();
    assert_eq!(created.document_path, root.join("Knowledge.adoc"));

    let child = create_child_node(&root, "Project").unwrap();
    let canonical_root = fs::canonicalize(&root).unwrap();
    assert_eq!(
        child.document_path,
        canonical_root.join("Project/Project.adoc")
    );

    let inventory = scan_workspace(&root);
    assert!(inventory.is_valid(), "{:?}", inventory.issues);
    assert_eq!(inventory.root, root);
    assert_eq!(inventory.nodes.len(), 2);
    assert_eq!(
        inventory
            .nodes
            .iter()
            .find(|node| node.id == Some(child.id))
            .unwrap()
            .parent_id,
        Some(created.id)
    );

    let index = WorkspaceIndex::rebuild(&inventory).unwrap();
    assert_eq!(index.len(), 2);
    assert_eq!(
        index.path_for(child.id),
        Some(root.join("Project").as_path())
    );
}

#[cfg(unix)]
#[test]
fn workspace_entry_points_resolve_linked_ancestors_but_reject_a_linked_root() {
    use std::os::unix::fs::symlink;

    let temporary = test_directory();
    let physical_parent = temporary.path().join("physical");
    let linked_parent = temporary.path().join("linked-parent");
    fs::create_dir(&physical_parent).unwrap();
    symlink(&physical_parent, &linked_parent).unwrap();

    let selected_root = linked_parent.join("Knowledge");
    let created = create_workspace(&selected_root).unwrap();
    let canonical_root = fs::canonicalize(&selected_root).unwrap();
    let child = create_child_node(&selected_root, "Project").unwrap();
    let inventory = scan_workspace(&selected_root);

    assert!(inventory.is_valid(), "{:?}", inventory.issues);
    assert_eq!(inventory.root, selected_root);
    assert_eq!(child.path, canonical_root.join("Project"));
    assert_eq!(
        weftext_core::read_workspace_revision(&selected_root).unwrap(),
        weftext_core::read_workspace_revision(&canonical_root).unwrap()
    );

    let linked_root = temporary.path().join("linked-root");
    symlink(&canonical_root, &linked_root).unwrap();
    let linked_inventory = scan_workspace(&linked_root);
    assert_eq!(
        linked_inventory.issues[0].code,
        InventoryIssueCode::SymlinkUnsupported
    );
    assert!(create_child_node(&linked_root, "Rejected").is_err());
    assert_eq!(created.path, selected_root);
}

#[test]
fn parent_policy_and_child_rank_determine_manual_order() {
    let temporary = test_directory();
    let root = temporary.path().join("Root");
    let created = create_workspace(&root).unwrap();
    let beta = create_child_node(&root, "Beta").unwrap();
    let alpha = create_child_node(&root, "Alpha").unwrap();
    fs::write(
        &created.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n  child_sort: manual\n---\n",
            created.id
        ),
    )
    .unwrap();
    fs::write(
        &beta.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n  sibling_rank: 1024\n---\n",
            beta.id
        ),
    )
    .unwrap();
    fs::write(
        &alpha.document_path,
        format!(
            "---\nweftext:\n  id: \"{}\"\n  sibling_rank: 2048\n---\n",
            alpha.id
        ),
    )
    .unwrap();
    let inventory = scan_workspace(&root);
    let names = inventory
        .ordered_children(created.id)
        .iter()
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["Beta", "Alpha"]);
}

#[test]
fn inventory_reports_markdown_resource_and_duplicate_identity() {
    let temporary = test_directory();
    let root = temporary.path().join("Root");
    let created = create_workspace(&root).unwrap();
    let child = create_child_node(&root, "Child").unwrap();
    fs::write(root.join("loose.md"), "loose").unwrap();
    fs::write(
        &child.document_path,
        format!("---\nweftext:\n  id: \"{}\"\n---\n", created.id),
    )
    .unwrap();

    let inventory = scan_workspace(&root);
    let codes = inventory
        .issues
        .iter()
        .map(|issue| issue.code)
        .collect::<Vec<_>>();
    assert!(codes.contains(&InventoryIssueCode::DuplicateIdentity));
    assert!(inventory.content.iter().any(|entry| {
        entry.kind == WorkspaceContentKind::Resource
            && entry.relative_path == "loose.md"
            && entry.owner_node_id == Some(created.id)
    }));
    assert!(matches!(
        classify_sync_state(&inventory),
        SyncDisposition::NeedsUserResolution { .. }
    ));
}

#[test]
fn a_missing_identity_waits_instead_of_being_regenerated() {
    let temporary = test_directory();
    let root = temporary.path().join("Root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(".weftext-format"), b"weftext.asciidoc.v1\n").unwrap();
    fs::write(
        root.join("Root.adoc"),
        "---\nweftext:\n---\n= arriving from cloud\n",
    )
    .unwrap();
    let inventory = scan_workspace(&root);
    assert!(matches!(
        classify_sync_state(&inventory),
        SyncDisposition::WaitForMoreFiles { .. }
    ));
}

#[test]
fn chrono_paths_are_fixed_under_the_year_node() {
    let date = CalendarDate::new(2026, 8, 21).unwrap();
    let plan = ChronoPlan::build(
        date,
        &[
            ChronoPeriod::Year,
            ChronoPeriod::Quarter,
            ChronoPeriod::Month,
            ChronoPeriod::Week,
            ChronoPeriod::Day,
        ],
    );
    let values = plan
        .nodes
        .iter()
        .map(|node| node.relative_path.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        [
            "2026",
            "2026/2026-Q3",
            "2026/2026-08",
            "2026/2026-W34",
            "2026/2026-08-21",
        ]
    );
}

#[test]
fn annotation_store_combines_mark_color_and_thread() {
    let document_id = NodeId::new_v4();
    let annotation_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let store = AnnotationStore {
        version: weftext_core::ANNOTATION_STORE_VERSION,
        document_id,
        annotations: vec![Annotation {
            id: annotation_id,
            kind: AnnotationKind::Comment,
            target: Anchor::TextRange {
                exact: "selected text".to_owned(),
                prefix: "before ".to_owned(),
                suffix: " after".to_owned(),
                start: 7,
                end: 20,
                base_revision: digest(),
                block_id: None,
                heading_path: Vec::new(),
            },
            appearance: Some(AnnotationAppearance {
                mark: AnnotationMark::Highlight,
                color: AnnotationColor::Yellow,
            }),
            suggested_source: None,
            labels: vec!["important".to_owned()],
            thread: vec![ThreadMessage {
                id: message_id,
                author_id: Uuid::new_v4(),
                author_name: "Reviewer".to_owned(),
                body: AnnotationBody {
                    format: AnnotationBodyFormat::AsciiDocInlineV1,
                    source: "Review this.".to_owned(),
                },
                created_at: "2026-08-21T10:00:00+08:00".to_owned(),
                updated_at: "2026-08-21T10:00:00+08:00".to_owned(),
            }],
            state: AnnotationState::Open,
            resolution: None,
            created_at: "2026-08-21T10:00:00+08:00".to_owned(),
            updated_at: "2026-08-21T10:00:00+08:00".to_owned(),
        }],
    };
    let json = store.to_pretty_json().unwrap();
    let roundtrip = AnnotationStore::from_json(&json).unwrap();
    assert_eq!(roundtrip, store);
    assert!(!json.contains("#ffd"));
}
