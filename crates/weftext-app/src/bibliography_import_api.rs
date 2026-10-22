//! Bibliography upload proposals and the shared Core prepare/commit entrypoint.
//! The caller authorizes the destination session/base; this adapter cannot
//! choose existing Work identities from DOI, title, or a citation suggestion.
use weftext_core::{Core, CoreError, OperationPreview};
use weftext_export::bibliography::ImportBibliographyProposal;
use weftext_messages::UiMessage;

pub(crate) struct BibliographyImportInput {
    pub root: String,
    pub parent: String,
    pub name: String,
    pub operation_id: String,
    pub format: String,
    pub bytes: Vec<u8>,
    pub allow_losses: bool,
}

fn bibliography_error(code: &str, error: weftext_export::ExportError) -> CoreError {
    let (description, diagnostics) = error.into_parts();
    CoreError::from_product(code, None, description, diagnostics)
}

pub(crate) fn inspect_bibliography_import(
    format: &str,
    bytes: &[u8],
) -> Result<ImportBibliographyProposal, CoreError> {
    let format = weftext_export::bibliography_import::parse_bibliography_format(format)
        .map_err(|error| bibliography_error("bibliographyFormat", error))?;
    weftext_export::bibliography::import_bibliography(format, bytes)
        .map_err(|error| bibliography_error("bibliographyImportUnavailable", error))
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
            UiMessage::error_api_bibliography_import_publication_fields_conflict(
                weftext_messages::params::ErrorApiBibliographyImportPublicationFieldsConflict {
                    conflicts: conflicts.join(", "),
                },
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

#[cfg(test)]
mod tests {
    use super::*;
    use weftext_core::{CoreErrorDetails, ErrorDetailGroup};
    use weftext_messages::ErrorDescription;

    #[test]
    fn unsupported_format_retains_the_export_typed_reason() {
        let error = inspect_bibliography_import("unknown", b"").unwrap_err();
        assert_eq!(error.code, "bibliographyFormat");
        assert_eq!(
            error.explanation,
            UiMessage::error_export_bibliography_import_format_unsupported()
        );
        assert!(error.diagnostics.is_empty());
    }

    #[test]
    fn bibliography_bridge_keeps_all_parallel_problems_outside_the_causal_chain() {
        let diagnostics: Vec<_> = (0..48)
            .map(|index| {
                ErrorDescription::new(UiMessage::error_api_citation_work_bytes_unavailable(
                    weftext_messages::params::ErrorApiCitationWorkBytesUnavailable {
                        key: format!("entry-{index}"),
                    },
                ))
            })
            .collect();
        let error = bibliography_error(
            "bibliographyImportUnavailable",
            weftext_export::ExportError::new(
                UiMessage::error_api_bibliography_import_publication_fields_conflict(
                    weftext_messages::params::ErrorApiBibliographyImportPublicationFieldsConflict {
                        conflicts: "record 1 publication.title".into(),
                    },
                ),
            )
            .with_diagnostics(diagnostics.clone()),
        );
        assert_eq!(error.code, "bibliographyImportUnavailable");
        assert_eq!(
            error.explanation,
            UiMessage::error_api_bibliography_import_publication_fields_conflict(
                weftext_messages::params::ErrorApiBibliographyImportPublicationFieldsConflict {
                    conflicts: "record 1 publication.title".into(),
                }
            )
        );
        assert!(error.causes.as_slice().is_empty());
        let Some(CoreErrorDetails::Aggregate { groups }) = error.details else {
            panic!("missing typed parallel error details");
        };
        let [
            ErrorDetailGroup::Product {
                code,
                diagnostics: retained,
            },
        ] = groups.as_slice()
        else {
            panic!("unexpected parallel error detail kind");
        };
        assert!(code.is_none());
        assert_eq!(retained, &diagnostics);
    }
}
