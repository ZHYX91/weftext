use std::fs;

use tempfile::tempdir;
use weftext_core::{
    CitationAccessScope, CitationWorkspaceDiagnosticCode, CitationWorkspaceError,
    CitationWorkspaceIndex, InventoryIssueCode, create_workspace, scan_workspace,
};

#[test]
fn canonical_workspace_diagnoses_unresolved_occurrences_without_record_authority() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    let created = create_workspace(&root).unwrap();
    let mut source = fs::read_to_string(&created.document_path).unwrap();
    source.push_str("= Workspace\n\nMissing cite:[not-present].\n");
    fs::write(&created.document_path, source).unwrap();

    let index = CitationWorkspaceIndex::rebuild(&root).unwrap();
    assert_eq!(index.reference_node_ids().count(), 0);
    let complete = index
        .analyze_component(
            created.id,
            &CitationAccessScope::complete(std::iter::empty()),
        )
        .unwrap();
    assert_eq!(complete.clusters.len(), 0);
    assert!(complete.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == CitationWorkspaceDiagnosticCode::MissingReferenceKey
            && diagnostic.reference_node_ids.is_empty()
    }));

    let filtered = index
        .analyze_component(
            created.id,
            &CitationAccessScope::filtered(std::iter::empty()),
        )
        .unwrap();
    assert!(filtered.diagnostics.iter().all(|diagnostic| {
        diagnostic.code == CitationWorkspaceDiagnosticCode::UnavailableReferenceKey
            && diagnostic.reference_node_ids.is_empty()
    }));
}

#[test]
fn markerless_runtime_and_top_level_reference_fail_closed() {
    let temporary = tempdir().unwrap();
    let markerless = temporary.path().join("Markerless");
    fs::create_dir(&markerless).unwrap();
    fs::write(
        markerless.join("Markerless.md"),
        "---\n_weftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n# Legacy\n",
    )
    .unwrap();
    assert!(matches!(
        CitationWorkspaceIndex::rebuild(&markerless),
        Err(CitationWorkspaceError::InvalidWorkspace(
            InventoryIssueCode::InvalidWorkspaceGeneration
        ))
    ));

    let canonical = temporary.path().join("Canonical");
    let created = create_workspace(&canonical).unwrap();
    let source = fs::read_to_string(&created.document_path)
        .unwrap()
        .replacen(
            "---\n",
            "---\nreference:\n  key: retired\n  type: book\n  title: Retired\n",
            1,
        );
    fs::write(&created.document_path, source).unwrap();
    assert_eq!(
        scan_workspace(&canonical).issues[0].code,
        InventoryIssueCode::InvalidMetadata
    );
    assert!(matches!(
        CitationWorkspaceIndex::rebuild(&canonical),
        Err(CitationWorkspaceError::InvalidWorkspace(
            InventoryIssueCode::InvalidMetadata
        ))
    ));
}
