use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use weftext_core::{DocumentProfileId, DocumentRevision, NodeId, WorkspaceRevision};

pub const MARKDOWN_EXPORT_CONTRACT_VERSION: &str = "weftext.export.markdown.v1";
pub const MARKDOWN_EXPORT_RECEIPT_VERSION: &str = "weftext.export.receipt.v1";
pub const MARKDOWN_EXPORTER_COMPONENT_VERSION: &str = "weftext.markdown-exporter.v1";
pub const MAX_EXPORT_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_EXPORT_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) const MAX_BUNDLE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownMetadataPolicy {
    PreserveWeftext,
    RemoveWeftext,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownResourcePolicy {
    ExternalReferencesOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportDiagnosticSeverity {
    Warning,
    Omission,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExportDiagnostic {
    pub code: String,
    pub severity: ExportDiagnosticSeverity,
    pub message: String,
    pub source_start: Option<u64>,
    pub source_end: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MarkdownCompatibilityReport {
    pub exact_blocks: u64,
    pub lowered_blocks: u64,
    pub preserved_literal_blocks: u64,
    pub omitted_blocks: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExportComponentVersion {
    pub component_id: String,
    pub version: String,
}

/// Immutable reviewed authority for one external Markdown artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MarkdownExportPlan {
    pub contract_version: String,
    pub plan_id: String,
    pub bundle_digest: String,
    pub base_workspace_revision: WorkspaceRevision,
    pub source_node_id: NodeId,
    pub source_document_revision: DocumentRevision,
    pub source_profile: DocumentProfileId,
    pub source_byte_length: u64,
    pub semantic_model_version: u16,
    pub destination: PathBuf,
    pub metadata_policy: MarkdownMetadataPolicy,
    pub resource_policy: MarkdownResourcePolicy,
    pub media_type: String,
    pub artifact_digest: String,
    pub artifact: String,
    pub diagnostics: Vec<ExportDiagnostic>,
    pub report: MarkdownCompatibilityReport,
    pub components: Vec<ExportComponentVersion>,
}

impl MarkdownExportPlan {
    pub(crate) fn compute_bundle_digest(&self) -> Result<String, ExportError> {
        let material = serde_json::to_vec(&(
            &self.contract_version,
            &self.plan_id,
            &self.base_workspace_revision,
            &self.source_node_id,
            &self.source_document_revision,
            &self.source_profile,
            self.source_byte_length,
            self.semantic_model_version,
            &self.destination,
            self.metadata_policy,
            self.resource_policy,
            &self.media_type,
            &self.artifact_digest,
            &self.artifact,
            (&self.diagnostics, &self.report, &self.components),
        ))
        .map_err(|error| serialization("serialize Markdown export authority", &error))?;
        Ok(sha256(&material))
    }

    pub(crate) fn expected_plan_id(&self) -> Result<String, ExportError> {
        let material = serde_json::to_vec(&(
            &self.base_workspace_revision,
            &self.source_node_id,
            &self.source_document_revision,
            &self.source_profile,
            self.source_byte_length,
            self.semantic_model_version,
            &self.destination,
            self.metadata_policy,
            self.resource_policy,
            &self.artifact_digest,
            &self.diagnostics,
            &self.report,
            &self.components,
        ))
        .map_err(|error| serialization("serialize Markdown export plan identity", &error))?;
        Ok(format!("export-{}", &sha256(&material)[..24]))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownExportReceipt {
    pub contract_version: String,
    pub created_at: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub source_node_id: NodeId,
    pub source_document_revision: DocumentRevision,
    pub base_workspace_revision: WorkspaceRevision,
    pub destination: PathBuf,
    pub artifact_digest: String,
    pub artifact_byte_length: u64,
    pub status: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportErrorCode {
    InvalidWorkspace,
    InvalidSource,
    InvalidPlan,
    StalePlan,
    UnsafeDestination,
    DestinationExists,
    LimitExceeded,
    Io,
    Serialization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportError {
    code: ExportErrorCode,
    message: String,
}

impl ExportError {
    #[must_use]
    pub fn code(&self) -> ExportErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn new(code: ExportErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ExportError {}

pub(crate) fn validate_plan_self(plan: &MarkdownExportPlan) -> Result<(), ExportError> {
    if plan.contract_version != MARKDOWN_EXPORT_CONTRACT_VERSION
        || plan.media_type != "text/markdown; charset=utf-8"
        || plan.resource_policy != MarkdownResourcePolicy::ExternalReferencesOnly
        || plan.artifact.as_bytes().contains(&0)
        || u64::try_from(plan.artifact.len()).unwrap_or(u64::MAX) > MAX_EXPORT_ARTIFACT_BYTES
        || plan.source_byte_length > MAX_EXPORT_SOURCE_BYTES
        || plan.diagnostics.len() > 10_000
        || plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.code.is_empty()
                || diagnostic.code.len() > 128
                || diagnostic.message.is_empty()
                || diagnostic.message.len() > 4_096
                || !valid_diagnostic_range(diagnostic, plan.source_byte_length)
        })
        || plan.artifact_digest != sha256(plan.artifact.as_bytes())
        || plan.components != expected_components(plan.semantic_model_version)
    {
        return Err(ExportError::new(
            ExportErrorCode::InvalidPlan,
            "Markdown export bundle differs from the reviewed v1 contract",
        ));
    }
    if plan.plan_id != plan.expected_plan_id()?
        || plan.bundle_digest != plan.compute_bundle_digest()?
    {
        return Err(ExportError::new(
            ExportErrorCode::InvalidPlan,
            "Markdown export plan identity or bundle digest is stale or altered",
        ));
    }
    Ok(())
}

fn valid_diagnostic_range(diagnostic: &ExportDiagnostic, source_length: u64) -> bool {
    match (diagnostic.source_start, diagnostic.source_end) {
        (Some(start), Some(end)) => start <= end && end <= source_length,
        (None, None) => true,
        _ => false,
    }
}

pub(crate) fn expected_components(semantic_model_version: u16) -> Vec<ExportComponentVersion> {
    vec![
        ExportComponentVersion {
            component_id: "weftext-core-document-model".to_owned(),
            version: format!("semantic-model-v{semantic_model_version}"),
        },
        ExportComponentVersion {
            component_id: "weftext-markdown-exporter".to_owned(),
            version: MARKDOWN_EXPORTER_COMPONENT_VERSION.to_owned(),
        },
    ]
}

pub(crate) fn validate_timestamp(value: &str) -> Result<(), ExportError> {
    if value.len() < 20
        || value.len() > 64
        || !value.is_ascii()
        || value.as_bytes().get(10) != Some(&b'T')
        || !(value.ends_with('Z')
            || value
                .get(19..)
                .is_some_and(|suffix| suffix.contains('+') || suffix.contains('-')))
    {
        return Err(ExportError::new(
            ExportErrorCode::InvalidPlan,
            "export receipt timestamp must be explicit-offset RFC 3339 text",
        ));
    }
    Ok(())
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn serialization(context: &str, error: &impl fmt::Display) -> ExportError {
    ExportError::new(
        ExportErrorCode::Serialization,
        format!("{context}: {error}"),
    )
}

pub(crate) fn invalid_source(message: impl Into<String>) -> ExportError {
    ExportError::new(ExportErrorCode::InvalidSource, message)
}
