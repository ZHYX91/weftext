use std::fs;

use tempfile::tempdir;
use weftext_core::{
    DocumentEdit, DocumentError, commit_document_edit, create_workspace, plan_document_edit,
    read_node_document,
};

#[test]
fn exact_source_read_and_range_edit_preserve_frontmatter_bytes() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("笔记");
    let created = create_workspace(&node).unwrap();
    let original = fs::read_to_string(&created.document_path).unwrap();
    let source = original.replace("\n\n", "\r\n\r\n") + "正文：甲乙。\r\n";
    fs::write(&created.document_path, &source).unwrap();

    let snapshot = read_node_document(&node).unwrap();
    assert_eq!(snapshot.source, source);
    let start = snapshot.source.find("甲乙").unwrap();
    let plan = plan_document_edit(
        &node,
        &snapshot.revision,
        [DocumentEdit {
            start: u64::try_from(start).unwrap(),
            end: u64::try_from(start + "甲乙".len()).unwrap(),
            replacement: "甲乙丙".to_owned(),
        }],
    )
    .unwrap();

    assert!(plan.changed);
    assert_eq!(
        &plan.next_source()[..start],
        &snapshot.source[..start],
        "all source before the edit must remain byte-for-byte identical"
    );
    let committed = commit_document_edit(&plan).unwrap();
    let reopened = read_node_document(&node).unwrap();
    assert_eq!(reopened.source, plan.next_source());
    assert_eq!(reopened.revision, committed.revision);
    assert!(reopened.source.contains("正文：甲乙丙。\r\n"));
}

#[test]
fn stale_revision_fails_without_overwriting_external_content() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("Notes");
    let created = create_workspace(&node).unwrap();
    let snapshot = read_node_document(&node).unwrap();
    let plan = plan_document_edit(
        &node,
        &snapshot.revision,
        [DocumentEdit {
            start: u64::try_from(snapshot.source.len()).unwrap(),
            end: u64::try_from(snapshot.source.len()).unwrap(),
            replacement: "local\n".to_owned(),
        }],
    )
    .unwrap();
    let external = snapshot.source.clone() + "external\n";
    fs::write(&created.document_path, &external).unwrap();

    let error = commit_document_edit(&plan).unwrap_err();
    assert!(matches!(error, DocumentError::StaleRevision { .. }));
    assert_eq!(
        fs::read_to_string(&created.document_path).unwrap(),
        external
    );
}

#[test]
fn edit_order_is_canonical_and_overlap_is_rejected() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("Notes");
    create_workspace(&node).unwrap();
    let snapshot = read_node_document(&node).unwrap();
    let end = u64::try_from(snapshot.source.len()).unwrap();
    let first = DocumentEdit {
        start: end,
        end,
        replacement: "A".to_owned(),
    };
    let second = DocumentEdit {
        start: end,
        end,
        replacement: "B".to_owned(),
    };
    let left =
        plan_document_edit(&node, &snapshot.revision, [second.clone(), first.clone()]).unwrap();
    let right = plan_document_edit(&node, &snapshot.revision, [first, second]).unwrap();
    assert_eq!(left, right);

    let overlap = plan_document_edit(
        &node,
        &snapshot.revision,
        [
            DocumentEdit {
                start: 0,
                end: 2,
                replacement: String::new(),
            },
            DocumentEdit {
                start: 1,
                end: 3,
                replacement: String::new(),
            },
        ],
    )
    .unwrap_err();
    assert!(matches!(overlap, DocumentError::OverlappingEdits));
}

#[test]
fn non_character_boundaries_and_identity_changes_fail_closed() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("笔记");
    create_workspace(&node).unwrap();
    let snapshot = read_node_document(&node).unwrap();
    let source_with_cjk = snapshot.source.clone() + "文";
    fs::write(&snapshot.document_path, source_with_cjk).unwrap();
    let snapshot = read_node_document(&node).unwrap();
    let cjk_start = snapshot.source.find('文').unwrap();
    let boundary_error = plan_document_edit(
        &node,
        &snapshot.revision,
        [DocumentEdit {
            start: u64::try_from(cjk_start + 1).unwrap(),
            end: u64::try_from(cjk_start + 2).unwrap(),
            replacement: String::new(),
        }],
    )
    .unwrap_err();
    assert!(matches!(
        boundary_error,
        DocumentError::NonCharacterBoundary { .. }
    ));

    let id_text = snapshot.node_id.to_string();
    let id_start = snapshot.source.find(&id_text).unwrap();
    let replacement_id = weftext_core::NodeId::new_v4().to_string();
    let identity_error = plan_document_edit(
        &node,
        &snapshot.revision,
        [DocumentEdit {
            start: u64::try_from(id_start).unwrap(),
            end: u64::try_from(id_start + id_text.len()).unwrap(),
            replacement: replacement_id,
        }],
    )
    .unwrap_err();
    assert!(matches!(
        identity_error,
        DocumentError::IdentityChanged { .. }
    ));
}

#[test]
fn no_op_plan_does_not_rewrite_the_document() {
    let temporary = tempdir().unwrap();
    let node = temporary.path().join("Notes");
    let created = create_workspace(&node).unwrap();
    let snapshot = read_node_document(&node).unwrap();
    let before_modified = fs::metadata(&created.document_path)
        .unwrap()
        .modified()
        .unwrap();
    let plan = plan_document_edit(&node, &snapshot.revision, []).unwrap();
    assert!(!plan.changed);

    let committed = commit_document_edit(&plan).unwrap();
    let after_modified = fs::metadata(&created.document_path)
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(committed.revision, snapshot.revision);
    assert_eq!(before_modified, after_modified);
}
