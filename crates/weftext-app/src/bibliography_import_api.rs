//! Bibliography upload proposals and the shared Core prepare/commit entrypoint.
//! The caller authorizes the destination session/base; this adapter cannot
//! choose existing Work identities from DOI, title, or a citation suggestion.
use weftext_core::{Core, CoreError, OperationPreview};
use weftext_export::bibliography::ImportBibliographyProposal;

pub(crate) struct BibliographyImportInput {
    pub root: String,
    pub parent: String,
    pub name: String,
    pub operation_id: String,
    pub format: String,
    pub bytes: Vec<u8>,
    pub allow_losses: bool,
}

pub(crate) fn inspect_bibliography_import(
    format: &str,
    bytes: &[u8],
) -> Result<ImportBibliographyProposal, CoreError> {
    let format = weftext_export::bibliography_import::parse_bibliography_format(format)
        .map_err(|e| CoreError::new("bibliographyFormat", e.to_string()))?;
    weftext_export::bibliography::import_bibliography(format, bytes)
        .map_err(|e| CoreError::new("bibliographyImportUnavailable", e.to_string()))
}

pub(crate) fn prepare_bibliography_import(
    core: &mut Core,
    input: BibliographyImportInput,
) -> Result<OperationPreview, CoreError> {
    if core.operation_details(&input.operation_id).is_ok() {
        return core.prepare_bibliography_import(
            &input.root,
            &input.parent,
            &input.name,
            &input.format,
            input.bytes,
            &input.operation_id,
            input.allow_losses,
        );
    }
    let proposal = inspect_bibliography_import(&input.format, &input.bytes)?;
    let conflicts = proposal
        .records
        .iter()
        .enumerate()
        .flat_map(|(index, record)| {
            record
                .conflicts
                .iter()
                .map(move |conflict| format!("record {} publication.{}", index + 1, conflict.key))
        })
        .collect::<Vec<_>>();
    if !conflicts.is_empty() {
        return Err(CoreError::new(
            "bibliographyConflict",
            format!(
                "Imported fields conflict with exact preserved values: {}. Inspect the complete proposal and choose the conflicting values before importing.",
                conflicts.join(", ")
            ),
        ));
    }
    core.prepare_bibliography_import(
        &input.root,
        &input.parent,
        &input.name,
        &input.format,
        input.bytes,
        &input.operation_id,
        input.allow_losses,
    )
}
