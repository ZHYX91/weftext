use std::fs;

use tempfile::tempdir;
use weftext_core::{
    DocumentEdit, DocumentError, FrontmatterError, commit_document_edit, create_workspace,
    patch_node_icon_property, plan_document_edit, read_node_document, read_node_icon_from_source,
    resolve_node_icon_from_source,
};

fn replace_all(
    node: &std::path::Path,
    revision: &weftext_core::DocumentRevision,
    old_len: usize,
    source: String,
) -> Result<(), DocumentError> {
    let plan = plan_document_edit(
        node,
        revision,
        [DocumentEdit {
            start: 0,
            end: u64::try_from(old_len).expect("source length"),
            replacement: source,
        }],
    )?;
    commit_document_edit(&plan)?;
    Ok(())
}

#[test]
fn one_icon_scalar_resolves_and_picker_actions_preserve_unrelated_yaml() {
    let temporary = tempdir().expect("temporary directory");
    let workspace = temporary.path().join("Icon scalar");
    let created = create_workspace(&workspace).expect("workspace");
    let initial = read_node_document(&workspace).expect("initial source");
    let source = format!(
        "---\r\nweftext:\r\n  # preserve this comment\r\n  id: \"{}\"\r\n  aliases:\r\n    - 图标\r\n  icon: \"weftext:project\"\r\n---\r\n= Icon scalar\r\n",
        initial.node_id
    );
    fs::write(&created.document_path, source.as_bytes()).expect("icon fixture");

    let snapshot = read_node_document(&workspace).expect("icon source");
    assert_eq!(
        read_node_icon_from_source(&snapshot.source),
        Ok(Some("weftext:project".to_owned()))
    );
    assert_eq!(
        resolve_node_icon_from_source(&snapshot.source).map(|icon| icon.glyph),
        Some("项".to_owned())
    );

    let selected =
        patch_node_icon_property(&snapshot.source, Some("weftext:book")).expect("replace icon");
    assert!(selected.contains("  # preserve this comment\r\n"));
    assert!(selected.contains("  aliases:\r\n    - 图标\r\n"));
    assert!(selected.contains("  icon: \"weftext:book\"\r\n"));
    replace_all(
        &workspace,
        &snapshot.revision,
        snapshot.source.len(),
        selected,
    )
    .expect("revision-checked icon replacement");

    let committed = read_node_document(&workspace).expect("selected icon");
    assert_eq!(
        resolve_node_icon_from_source(&committed.source).map(|icon| icon.glyph),
        Some("书".to_owned())
    );
    let cleared = patch_node_icon_property(&committed.source, None).expect("clear property");
    assert!(!cleared.contains("  icon:"));
    assert!(cleared.contains("  aliases:\r\n    - 图标\r\n"));
}

#[test]
fn portable_icons_are_narrow_revision_checked_and_invalid_values_fail_closed() {
    let temporary = tempdir().expect("temporary directory");
    let workspace = temporary.path().join("Icons");
    let _ = create_workspace(&workspace).expect("workspace");
    let initial = read_node_document(&workspace).expect("initial source");
    let emoji = patch_node_icon_property(&initial.source, Some("😀")).expect("emoji patch");
    replace_all(&workspace, &initial.revision, initial.source.len(), emoji).expect("emoji commit");
    let committed = read_node_document(&workspace).expect("emoji source");
    assert_eq!(
        resolve_node_icon_from_source(&committed.source).map(|icon| icon.glyph),
        Some("😀".to_owned())
    );

    let stale_base = committed.clone();
    let external = format!("{}external\r\n", committed.source);
    replace_all(
        &workspace,
        &committed.revision,
        committed.source.len(),
        external.clone(),
    )
    .expect("external commit");
    let stale_source = patch_node_icon_property(&stale_base.source, Some("weftext:book"))
        .expect("stale draft patch");
    assert!(matches!(
        replace_all(
            &workspace,
            &stale_base.revision,
            stale_base.source.len(),
            stale_source,
        ),
        Err(DocumentError::StaleRevision { .. })
    ));
    assert_eq!(
        read_node_document(&workspace)
            .expect("preserved external")
            .source,
        external
    );

    let current = read_node_document(&workspace).expect("current source");
    assert_eq!(
        patch_node_icon_property(&current.source, Some("vendor:custom")),
        Err(FrontmatterError::InvalidIcon)
    );
    let future = patch_node_icon_property(&current.source, Some("weftext:unknown"))
        .expect("preserve a future Weftext-owned token");
    assert_eq!(
        read_node_icon_from_source(&future),
        Ok(Some("weftext:unknown".to_owned()))
    );
    assert!(resolve_node_icon_from_source(&future).is_none());
    let legacy_list = format!(
        "---\nweftext:\n  id: \"{}\"\n  icon:\n    - weftext:book\n    - 😀\n---\n",
        current.node_id
    );
    assert!(read_node_icon_from_source(&legacy_list).is_err());
    assert!(patch_node_icon_property(&legacy_list, None).is_err());
}
