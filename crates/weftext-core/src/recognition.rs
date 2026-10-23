//! Recognition consumes immutable provenance and creates one atomic author tree.
use super::*;
use operations::{AbsentDependency, CapturedFile, Mutation};
use weftext_messages::UiMessage;

impl Core {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn plan_recognition_import(
        &mut self,
        root: &Path,
        parent: &Path,
        name: &str,
        format: &str,
        bytes: &[u8],
        evidence: &serde_json::Value,
        review: &weftext_export::recognition::RecognitionReview,
        allow_losses: bool,
        preview: &mut OperationPreview,
        dependencies: &mut Vec<CapturedFile>,
        absent: &mut Vec<AbsentDependency>,
    ) -> Result<Mutation> {
        let config = self.read_config(root, ConfigRole::Schema)?;
        let schema = match config.bytes {
            Some(bytes) => decode(&bytes)?,
            None => include_str!("../../../docs/design/examples/schema.authoring.json").into(),
        };
        let package = weftext_export::recognition::build_recognition_package(
            format,
            bytes,
            evidence,
            review,
            &schema,
            &preview.operation_id,
        )
        .map_err(|e| CoreError::from_export("recognitionImport", e))?;
        let mut mutation = self.plan_content_import(
            root,
            parent,
            name,
            package,
            None,
            allow_losses,
            None,
            preview,
            dependencies,
            absent,
        )?;
        // Provenance stores the final Core-allocated resource ref and schema
        // revision; provisional producer identities must never escape as facts.
        if let Mutation::CreateTree { files, .. } = &mut mutation {
            let original = preview
                .resource_map
                .iter()
                .find(|(path, _)| {
                    Path::new(path)
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("original."))
                })
                .map(|(_, reference)| reference.clone())
                .ok_or_else(|| {
                    CoreError::new(
                        "recognitionEvidence",
                        UiMessage::error_core_recognition_original_resource_allocation_missing(),
                    )
                })?;
            let evidence_file = files
                .iter_mut()
                .find(|f| f.path.ends_with("recognition-evidence.json"))
                .ok_or_else(|| {
                    CoreError::new(
                        "recognitionEvidence",
                        UiMessage::error_core_recognition_provenance_resource_missing(),
                    )
                })?;
            let mut retained: serde_json::Value = serde_json::from_slice(&evidence_file.after)?;
            retained["originalResource"] = original.into();
            retained["schemaRevision"] = config.revision.into();
            evidence_file.after = serde_json::to_vec_pretty(&retained)?;
            if let Some(change) = preview
                .changes
                .iter_mut()
                .find(|c| c.path == evidence_file.path)
            {
                change.bytes = evidence_file.after.len();
            }
        }
        Ok(mutation)
    }
}
