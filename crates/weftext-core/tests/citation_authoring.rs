use std::fs;

use tempfile::tempdir;
use weftext_core::{
    BibliographyInclusion, CitationAccessScope, CitationAuthoringDiagnosticCode,
    CitationEditTarget, CitationForm, CitationItemIntent, CitationMacroIntent,
    CitationWorkspaceIndex, REFERENCE_RECORD_WRITES_RETIREMENT, citation_presentation_capabilities,
    create_workspace, plan_citation_macro_edit, reference_record_writes_available,
};

#[test]
fn reference_record_writes_are_absent_from_the_advertised_capability() {
    assert!(!reference_record_writes_available());
    let capabilities = citation_presentation_capabilities();
    assert!(!capabilities.reference_record_writes_available);
    assert_eq!(
        capabilities.reference_record_writes_reason,
        REFERENCE_RECORD_WRITES_RETIREMENT
    );
}

#[test]
fn canonical_workspace_has_no_implicit_reference_records() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    let created = create_workspace(&root).unwrap();
    let mut source = fs::read_to_string(&created.document_path).unwrap();
    source.push_str("= Workspace\n\nBody.\n");
    fs::write(&created.document_path, source).unwrap();

    let index = CitationWorkspaceIndex::rebuild(&root).unwrap();
    assert_eq!(index.reference_node_ids().count(), 0);
    assert!(index.diagnostics().is_empty());
    assert!(
        index
            .search_references(
                "anything",
                &CitationAccessScope::complete(std::iter::empty()),
                20,
            )
            .unwrap()
            .is_empty()
    );
}

#[test]
fn citation_occurrence_authoring_remains_typed_without_record_writes() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Workspace");
    let created = create_workspace(&root).unwrap();
    let mut source = fs::read_to_string(&created.document_path).unwrap();
    source.push_str("= Workspace\n\nBody.\n");
    fs::write(&created.document_path, &source).unwrap();
    let index = CitationWorkspaceIndex::rebuild(&root).unwrap();
    let scope = CitationAccessScope::complete(std::iter::empty());

    let bibliography = plan_citation_macro_edit(
        &index,
        created.id,
        &source,
        &scope,
        &CitationEditTarget::Insert {
            offset: source.len() as u64,
        },
        &CitationMacroIntent::Bibliography {
            inclusion: BibliographyInclusion::Cited,
        },
    )
    .unwrap();
    assert!(bibliography.proposed_source.ends_with("bibliography::[]"));

    let unavailable = plan_citation_macro_edit(
        &index,
        created.id,
        &source,
        &scope,
        &CitationEditTarget::Insert {
            offset: source.len() as u64,
        },
        &CitationMacroIntent::Citation {
            cluster: weftext_core::CitationClusterIntent {
                form: CitationForm::Parenthetical,
                items: vec![CitationItemIntent {
                    reference_node_id: weftext_core::NodeId::new_v4(),
                    label: None,
                    locator: None,
                    prefix: None,
                    suffix: None,
                }],
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        unavailable.code,
        CitationAuthoringDiagnosticCode::ReferenceUnavailable
    );
}
