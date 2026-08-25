//! Explicit, reviewable exports from canonical Weftext content to external artifacts.
//!
//! Export is separate from managed-document save. Preview freezes exact external bytes and a
//! compatibility report; commit revalidates source authority and publishes only those reviewed
//! bytes without changing the workspace.

#![forbid(unsafe_code)]

mod contract;
mod external_io;
mod markdown;

use std::fmt;
use std::path::Path;

use serde_json::Value;
use weftext_core::{
    DocumentAnalysisStatus, DocumentDiagnosticCode, NodeId, analyze_document_for_profile,
    read_node_document, read_workspace_revision, scan_workspace,
};

pub use contract::{
    ExportComponentVersion, ExportDiagnostic, ExportDiagnosticSeverity, ExportError,
    ExportErrorCode, MARKDOWN_EXPORT_CONTRACT_VERSION, MARKDOWN_EXPORT_RECEIPT_VERSION,
    MARKDOWN_EXPORTER_COMPONENT_VERSION, MAX_EXPORT_ARTIFACT_BYTES, MAX_EXPORT_SOURCE_BYTES,
    MarkdownCompatibilityReport, MarkdownExportPlan, MarkdownExportReceipt, MarkdownMetadataPolicy,
    MarkdownResourcePolicy,
};

use contract::{
    MAX_BUNDLE_BYTES, expected_components, serialization, sha256, validate_plan_self,
    validate_timestamp,
};
use external_io::{normalize_external_new_path, publish_create_new, read_regular_file_bounded};

/// Creates a read-only Markdown compatibility preview from one Core-authorized managed node.
///
/// # Errors
///
/// Rejects invalid workspaces, unknown identities, blocking profile diagnostics, unsafe external
/// destinations, and bounded-output failures.
pub fn preview_markdown_export(
    workspace: impl AsRef<Path>,
    node_id: NodeId,
    destination: impl AsRef<Path>,
    metadata_policy: MarkdownMetadataPolicy,
) -> Result<MarkdownExportPlan, ExportError> {
    let workspace = workspace.as_ref();
    let base_workspace_revision =
        read_workspace_revision(workspace).map_err(|error| workspace_error(&error))?;
    let inventory = scan_workspace(workspace);
    if !inventory.is_valid() {
        return Err(ExportError::new(
            ExportErrorCode::InvalidWorkspace,
            "Markdown export requires one valid canonical workspace inventory",
        ));
    }
    let root_setting = inventory
        .nodes
        .iter()
        .find(|node| node.path == inventory.root)
        .and_then(|node| node.metadata)
        .map(|metadata| metadata.presentation.adjacent_heading_body)
        .ok_or_else(|| {
            ExportError::new(
                ExportErrorCode::InvalidWorkspace,
                "workspace root presentation authority is missing",
            )
        })?;
    let mut matching = inventory
        .nodes
        .iter()
        .filter(|node| node.id == Some(node_id));
    let node = matching.next().ok_or_else(|| {
        ExportError::new(
            ExportErrorCode::InvalidSource,
            "Markdown export node identity is not present in the workspace",
        )
    })?;
    if matching.next().is_some() {
        return Err(ExportError::new(
            ExportErrorCode::InvalidWorkspace,
            "Markdown export node identity is duplicated",
        ));
    }
    let snapshot = read_node_document(&node.path).map_err(|error| source_error(&error))?;
    let source_byte_length = u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX);
    if source_byte_length > MAX_EXPORT_SOURCE_BYTES {
        return Err(ExportError::new(
            ExportErrorCode::LimitExceeded,
            "managed source exceeds the Markdown export source-byte limit",
        ));
    }
    let destination =
        normalize_external_new_path(workspace, destination.as_ref(), &["md", "markdown"])?;
    let analysis = analyze_document_for_profile(snapshot.profile, &snapshot.source, root_setting);
    if analysis.model.status == DocumentAnalysisStatus::Failed
        || analysis.model.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.code,
                DocumentDiagnosticCode::UnclosedFrontmatter | DocumentDiagnosticCode::ParserError
            )
        })
    {
        return Err(ExportError::new(
            ExportErrorCode::InvalidSource,
            "managed source has blocking parser diagnostics and cannot be exported",
        ));
    }
    let rendered = markdown::render(&snapshot.source, &analysis.model, metadata_policy)?;
    if u64::try_from(rendered.artifact.len()).unwrap_or(u64::MAX) > MAX_EXPORT_ARTIFACT_BYTES {
        return Err(ExportError::new(
            ExportErrorCode::LimitExceeded,
            "Markdown artifact exceeds the export output-byte limit",
        ));
    }
    let components = expected_components(analysis.model.semantic_model_version);
    let mut plan = MarkdownExportPlan {
        contract_version: MARKDOWN_EXPORT_CONTRACT_VERSION.to_owned(),
        plan_id: "pending".to_owned(),
        bundle_digest: "pending".to_owned(),
        base_workspace_revision,
        source_node_id: snapshot.node_id,
        source_document_revision: snapshot.revision,
        source_profile: snapshot.profile,
        source_byte_length,
        semantic_model_version: analysis.model.semantic_model_version,
        destination,
        metadata_policy,
        resource_policy: MarkdownResourcePolicy::ExternalReferencesOnly,
        media_type: "text/markdown; charset=utf-8".to_owned(),
        artifact_digest: sha256(rendered.artifact.as_bytes()),
        artifact: rendered.artifact,
        diagnostics: rendered.diagnostics,
        report: rendered.report,
        components,
    };
    plan.plan_id = plan.expected_plan_id()?;
    plan.bundle_digest = plan.compute_bundle_digest()?;
    validate_plan_self(&plan)?;
    Ok(plan)
}

/// Revalidates exact frozen bytes and current Core source authority without rendering again.
///
/// # Errors
///
/// Rejects altered bundles, stale workspace or document revisions, unknown nodes, unsafe external
/// destinations, and unsupported component evidence.
pub fn validate_markdown_export(
    workspace: impl AsRef<Path>,
    plan: &MarkdownExportPlan,
) -> Result<(), ExportError> {
    let workspace = workspace.as_ref();
    validate_plan_self(plan)?;
    let current_workspace_revision =
        read_workspace_revision(workspace).map_err(|error| workspace_error(&error))?;
    if current_workspace_revision != plan.base_workspace_revision {
        return Err(ExportError::new(
            ExportErrorCode::StalePlan,
            format!(
                "stale Markdown export: expected workspace revision {}, found {current_workspace_revision}",
                plan.base_workspace_revision
            ),
        ));
    }
    let inventory = scan_workspace(workspace);
    if !inventory.is_valid() {
        return Err(ExportError::new(
            ExportErrorCode::InvalidWorkspace,
            "Markdown export workspace inventory is no longer valid",
        ));
    }
    let mut matching = inventory
        .nodes
        .iter()
        .filter(|node| node.id == Some(plan.source_node_id));
    let node = matching.next().ok_or_else(|| {
        ExportError::new(
            ExportErrorCode::StalePlan,
            "Markdown export source node is no longer present",
        )
    })?;
    if matching.next().is_some() {
        return Err(ExportError::new(
            ExportErrorCode::InvalidWorkspace,
            "Markdown export source node identity is duplicated",
        ));
    }
    let snapshot = read_node_document(&node.path).map_err(|error| source_error(&error))?;
    if snapshot.node_id != plan.source_node_id
        || snapshot.revision != plan.source_document_revision
        || snapshot.profile != plan.source_profile
        || u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX) != plan.source_byte_length
    {
        return Err(ExportError::new(
            ExportErrorCode::StalePlan,
            "Markdown export source authority changed after preview",
        ));
    }
    let normalized =
        normalize_external_new_path(workspace, &plan.destination, &["md", "markdown"])?;
    if normalized != plan.destination {
        return Err(ExportError::new(
            ExportErrorCode::UnsafeDestination,
            "Markdown export destination is not its canonical external path",
        ));
    }
    Ok(())
}

/// Publishes only the exact reviewed artifact at a create-new external destination.
///
/// # Errors
///
/// Returns an error for stale authority, altered bytes, unsafe paths, overwrite attempts, I/O, or
/// post-write verification failure. The workspace is never written.
pub fn commit_markdown_export(
    workspace: impl AsRef<Path>,
    plan: &MarkdownExportPlan,
    created_at: impl Into<String>,
) -> Result<MarkdownExportReceipt, ExportError> {
    let workspace = workspace.as_ref();
    validate_markdown_export(workspace, plan)?;
    publish_create_new(&plan.destination, plan.artifact.as_bytes())?;
    let verified = read_regular_file_bounded(&plan.destination, MAX_EXPORT_ARTIFACT_BYTES)?;
    if verified != plan.artifact.as_bytes() || sha256(&verified) != plan.artifact_digest {
        return Err(ExportError::new(
            ExportErrorCode::Io,
            "published Markdown artifact failed exact byte verification",
        ));
    }
    let created_at = created_at.into();
    validate_timestamp(&created_at)?;
    Ok(MarkdownExportReceipt {
        contract_version: MARKDOWN_EXPORT_RECEIPT_VERSION.to_owned(),
        created_at,
        plan_id: plan.plan_id.clone(),
        plan_digest: plan.bundle_digest.clone(),
        source_node_id: plan.source_node_id,
        source_document_revision: plan.source_document_revision.clone(),
        base_workspace_revision: plan.base_workspace_revision.clone(),
        destination: plan.destination.clone(),
        artifact_digest: plan.artifact_digest.clone(),
        artifact_byte_length: u64::try_from(plan.artifact.len()).unwrap_or(u64::MAX),
        status: "committed".to_owned(),
    })
}

/// Writes one immutable export bundle outside the workspace.
///
/// # Errors
///
/// Rejects invalid plans, in-workspace paths, overwrites, oversized JSON, and I/O failures.
pub fn write_markdown_export_bundle(
    workspace: impl AsRef<Path>,
    path: impl AsRef<Path>,
    plan: &MarkdownExportPlan,
) -> Result<(), ExportError> {
    validate_plan_self(plan)?;
    let path = normalize_external_new_path(workspace.as_ref(), path.as_ref(), &["json"])?;
    let bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| serialization("serialize Markdown export bundle", &error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BUNDLE_BYTES {
        return Err(ExportError::new(
            ExportErrorCode::LimitExceeded,
            "Markdown export bundle exceeds its byte limit",
        ));
    }
    publish_create_new(&path, &bytes)
}

/// Reads and self-validates one export bundle from a regular non-link file.
///
/// # Errors
///
/// Rejects unsafe files, oversized or non-canonical JSON, unknown fields, and altered authority.
pub fn read_markdown_export_bundle(
    path: impl AsRef<Path>,
) -> Result<MarkdownExportPlan, ExportError> {
    let bytes = read_regular_file_bounded(path.as_ref(), MAX_BUNDLE_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| serialization("parse Markdown export bundle JSON", &error))?;
    let plan: MarkdownExportPlan = serde_json::from_value(value.clone())
        .map_err(|error| serialization("decode Markdown export bundle", &error))?;
    let normalized = serde_json::to_value(&plan)
        .map_err(|error| serialization("normalize Markdown export bundle", &error))?;
    if value != normalized {
        return Err(ExportError::new(
            ExportErrorCode::InvalidPlan,
            "Markdown export bundle contains fields outside its exact contract",
        ));
    }
    validate_plan_self(&plan)?;
    Ok(plan)
}

fn workspace_error(error: &impl fmt::Display) -> ExportError {
    ExportError::new(
        ExportErrorCode::InvalidWorkspace,
        format!("workspace export authority failed: {error}"),
    )
}

fn source_error(error: &impl fmt::Display) -> ExportError {
    ExportError::new(
        ExportErrorCode::InvalidSource,
        format!("managed export source failed validation: {error}"),
    )
}
