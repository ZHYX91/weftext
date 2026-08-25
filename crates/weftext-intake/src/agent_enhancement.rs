use std::collections::BTreeSet;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use weftext_import::{
    AgentEnhancementSelection, AgentImportPatch, AsciiDocV1ProposalValidator,
    CanonicalProposalValidator, CommitResult, EgressDisclosure, ImportNode, ImportNodeKind,
    ImportPlan, ImportReceipt, ImportResource, Sha256Digest, apply_agent_patch, sha256_bytes,
};

use crate::{
    ImportPreviewBundle, IntakeError, IntakeErrorCode, MAX_BUNDLE_BYTES, import_error,
    invalid_bundle_error, read_regular_file_bounded, reject_duplicate_json_keys, serialization,
    validate_preview_bundle, write_bundle_bytes,
};

pub const AGENT_IMPORT_EVIDENCE_CONTRACT_VERSION: &str = "weftext.import-agent-evidence.v1";
pub const AGENT_ENHANCEMENT_PREVIEW_CONTRACT_VERSION: &str =
    "weftext.intake-agent-enhancement-preview.v1";
const MAX_AGENT_SELECTION_BYTES: u64 = 1024 * 1024;
const MAX_AGENT_PATCH_BYTES: u64 = 4 * 1024 * 1024;

/// User-reviewed selection made only after deterministic local extraction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentEvidenceSelection {
    pub provider: String,
    pub selected_node_ids: Vec<String>,
    pub retention: String,
    pub redaction: String,
}

/// Exact, bounded IR evidence that may be disclosed after approval.
///
/// Section children not named by `selected_node_ids` are removed. Figure bytes
/// are included only when their figure node is selected. The source artifact
/// itself and workspace paths are never part of this payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentImportEvidence {
    pub contract_version: String,
    pub evidence_digest: Sha256Digest,
    pub base_ir_revision: Sha256Digest,
    pub source_digest: Sha256Digest,
    pub provider: String,
    pub selected_node_ids: Vec<String>,
    pub nodes: Vec<ImportNode>,
    pub resources: Vec<ImportResource>,
}

impl AgentImportEvidence {
    fn create(
        bundle: &ImportPreviewBundle,
        selection: &AgentEvidenceSelection,
    ) -> Result<Self, IntakeError> {
        validate_selection(bundle, selection)?;
        let selected = selection
            .selected_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let nodes = selection
            .selected_node_ids
            .iter()
            .map(|id| {
                find_node(&bundle.document.nodes, id)
                    .map(|node| sanitize_selected_node(node, &selected))
                    .ok_or_else(|| {
                        invalid_bundle_error(
                            "agent evidence selection contains an unavailable IR node",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let resource_ids = nodes
            .iter()
            .flat_map(figure_resource_ids)
            .collect::<BTreeSet<_>>();
        let resources = bundle
            .document
            .resources
            .iter()
            .filter(|resource| resource_ids.contains(resource.id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if resource_ids.len() != resources.len() {
            return Err(invalid_bundle_error(
                "selected agent evidence refers to an unavailable IR resource",
            ));
        }
        let mut evidence = Self {
            contract_version: AGENT_IMPORT_EVIDENCE_CONTRACT_VERSION.to_owned(),
            evidence_digest: sha256_bytes(b"pending"),
            base_ir_revision: bundle.document.revision.clone(),
            source_digest: bundle.source.sha256.clone(),
            provider: selection.provider.clone(),
            selected_node_ids: selection.selected_node_ids.clone(),
            nodes,
            resources,
        };
        evidence.evidence_digest = evidence.compute_digest()?;
        evidence.validate(bundle, selection)?;
        Ok(evidence)
    }

    /// Serializes the exact reviewed payload that may leave Weftext.
    ///
    /// # Errors
    ///
    /// Returns an error only if the closed evidence contract cannot be encoded.
    pub fn to_bytes(&self) -> Result<Vec<u8>, IntakeError> {
        serde_json::to_vec(self)
            .map_err(|error| serialization("serialize selected agent evidence", &error))
    }

    fn compute_digest(&self) -> Result<Sha256Digest, IntakeError> {
        let material = serde_json::to_vec(&(
            &self.contract_version,
            &self.base_ir_revision,
            &self.source_digest,
            &self.provider,
            &self.selected_node_ids,
            &self.nodes,
            &self.resources,
        ))
        .map_err(|error| serialization("digest selected agent evidence", &error))?;
        Ok(sha256_bytes(&material))
    }

    fn validate(
        &self,
        bundle: &ImportPreviewBundle,
        selection: &AgentEvidenceSelection,
    ) -> Result<(), IntakeError> {
        if self.contract_version != AGENT_IMPORT_EVIDENCE_CONTRACT_VERSION
            || self.evidence_digest != self.compute_digest()?
            || self.base_ir_revision != bundle.document.revision
            || self.source_digest != bundle.source.sha256
            || self.provider != selection.provider
            || self.selected_node_ids != selection.selected_node_ids
        {
            return Err(invalid_bundle_error(
                "selected agent evidence differs from its local IR authority",
            ));
        }
        let expected = Self::create_unchecked(bundle, selection)?;
        if self.nodes != expected.nodes || self.resources != expected.resources {
            return Err(invalid_bundle_error(
                "selected agent evidence contains data outside its exact IR selection",
            ));
        }
        Ok(())
    }

    fn create_unchecked(
        bundle: &ImportPreviewBundle,
        selection: &AgentEvidenceSelection,
    ) -> Result<Self, IntakeError> {
        let selected = selection
            .selected_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let nodes = selection
            .selected_node_ids
            .iter()
            .map(|id| {
                find_node(&bundle.document.nodes, id)
                    .map(|node| sanitize_selected_node(node, &selected))
                    .ok_or_else(|| {
                        invalid_bundle_error(
                            "agent evidence selection contains an unavailable IR node",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let resource_ids = nodes
            .iter()
            .flat_map(figure_resource_ids)
            .collect::<BTreeSet<_>>();
        let resources = bundle
            .document
            .resources
            .iter()
            .filter(|resource| resource_ids.contains(resource.id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        Ok(Self {
            contract_version: AGENT_IMPORT_EVIDENCE_CONTRACT_VERSION.to_owned(),
            evidence_digest: sha256_bytes(b"pending"),
            base_ir_revision: bundle.document.revision.clone(),
            source_digest: bundle.source.sha256.clone(),
            provider: selection.provider.clone(),
            selected_node_ids: selection.selected_node_ids.clone(),
            nodes,
            resources,
        })
    }
}

/// Immutable selection/egress preview awaiting approval before an agent call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentEnhancementPreview {
    pub contract_version: String,
    pub preview_digest: Sha256Digest,
    pub base_bundle_digest: Sha256Digest,
    pub local_plan: ImportPlan,
    pub selection: AgentEvidenceSelection,
    pub evidence: AgentImportEvidence,
    pub authorized_bundle: ImportPreviewBundle,
}

/// Reads an exact agent evidence selection from a bounded regular non-link file.
///
/// # Errors
///
/// Returns an error for unsafe file state, excessive size, duplicate JSON keys,
/// unknown fields, or any non-canonical contract shape.
pub fn read_agent_evidence_selection(
    path: impl AsRef<Path>,
) -> Result<AgentEvidenceSelection, IntakeError> {
    read_exact_json_contract(
        path.as_ref(),
        MAX_AGENT_SELECTION_BYTES,
        "agent evidence selection",
    )
}

/// Reads and fully validates an immutable agent enhancement review.
///
/// # Errors
///
/// Returns an error for unsafe file state, excessive size, duplicate/unknown
/// JSON fields, or altered local IR, source, plan, proposal, receipt, or egress
/// authority.
pub fn read_agent_enhancement_preview(
    path: impl AsRef<Path>,
) -> Result<AgentEnhancementPreview, IntakeError> {
    let preview =
        read_exact_json_contract(path.as_ref(), MAX_BUNDLE_BYTES, "agent enhancement preview")?;
    validate_agent_enhancement_preview(&preview)?;
    Ok(preview)
}

/// Reads a structurally exact typed agent patch from a bounded regular non-link file.
///
/// Semantic scope, revision, provider, egress, and operation validation remains
/// bound to `apply_approved_agent_patch` and its reviewed preview.
///
/// # Errors
///
/// Returns an error for unsafe file state, excessive size, duplicate JSON keys,
/// unknown fields, or any non-canonical contract shape.
pub fn read_agent_import_patch(path: impl AsRef<Path>) -> Result<AgentImportPatch, IntakeError> {
    read_exact_json_contract(
        path.as_ref(),
        MAX_AGENT_PATCH_BYTES,
        "typed agent import patch",
    )
}

/// Writes one fully validated agent enhancement review outside the workspace,
/// atomically and without overwrite.
///
/// # Errors
///
/// Returns an error for invalid authority, serialization, path escape,
/// overwrite, excessive size, write, or sync failure.
pub fn write_agent_enhancement_preview(
    workspace: impl AsRef<Path>,
    path: impl AsRef<Path>,
    preview: &AgentEnhancementPreview,
) -> Result<(), IntakeError> {
    validate_agent_enhancement_preview(preview)?;
    let bytes = serde_json::to_vec_pretty(preview)
        .map_err(|error| serialization("serialize agent enhancement preview", &error))?;
    write_bundle_bytes(workspace.as_ref(), path.as_ref(), &bytes)
}

/// Writes only the exact selected IR evidence authorized by a validated review.
///
/// This function does not perform network access. The emitted bytes are exactly
/// those whose digest and byte count are bound into the reviewed egress plan.
///
/// # Errors
///
/// Returns an error for invalid authority, path escape, overwrite, excessive
/// size, write, or sync failure.
pub fn write_agent_import_evidence(
    workspace: impl AsRef<Path>,
    path: impl AsRef<Path>,
    preview: &AgentEnhancementPreview,
) -> Result<(), IntakeError> {
    validate_agent_enhancement_preview(preview)?;
    let bytes = preview.evidence.to_bytes()?;
    write_bundle_bytes(workspace.as_ref(), path.as_ref(), &bytes)
}

fn read_exact_json_contract<T>(
    path: &Path,
    maximum_bytes: u64,
    contract_name: &str,
) -> Result<T, IntakeError>
where
    T: DeserializeOwned + Serialize,
{
    let bytes = read_regular_file_bounded(path, maximum_bytes)?;
    reject_duplicate_json_keys(&bytes)
        .map_err(|error| serialization(&format!("parse {contract_name} JSON"), &error))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| serialization(&format!("parse {contract_name} JSON"), &error))?;
    let contract: T = serde_json::from_value(value.clone())
        .map_err(|error| serialization(&format!("decode {contract_name} contract"), &error))?;
    let exact = serde_json::to_value(&contract)
        .map_err(|error| serialization(&format!("normalize {contract_name} contract"), &error))?;
    if value != exact {
        return Err(invalid_bundle_error(format!(
            "{contract_name} contains fields outside its exact contract"
        )));
    }
    Ok(contract)
}

impl AgentEnhancementPreview {
    fn compute_digest(&self) -> Result<Sha256Digest, IntakeError> {
        let material = serde_json::to_vec(&(
            &self.contract_version,
            &self.base_bundle_digest,
            &self.local_plan,
            &self.selection,
            &self.evidence,
            &self.authorized_bundle,
        ))
        .map_err(|error| serialization("digest agent enhancement preview", &error))?;
        Ok(sha256_bytes(&material))
    }
}

/// Creates the exact post-extraction selection and egress preview.
///
/// This function performs no network access and does not apply a patch. The
/// returned evidence bytes and derived plan are immutable review authority for
/// the subsequent agent call.
///
/// # Errors
///
/// Returns an error for an invalid/tampered local bundle, duplicate or missing
/// targets, excessive evidence, or a plan/receipt that cannot be regenerated.
pub fn prepare_agent_enhancement(
    local_bundle: &ImportPreviewBundle,
    selection: AgentEvidenceSelection,
    created_at: impl Into<String>,
) -> Result<AgentEnhancementPreview, IntakeError> {
    validate_preview_bundle(local_bundle)?;
    validate_selection(local_bundle, &selection)?;
    let evidence = AgentImportEvidence::create(local_bundle, &selection)?;
    let evidence_bytes = evidence.to_bytes()?;
    let disclosed_bytes = u64::try_from(evidence_bytes.len()).unwrap_or(u64::MAX);
    if disclosed_bytes > local_bundle.plan.limits.max_agent_output_bytes {
        return Err(IntakeError::new(
            IntakeErrorCode::LimitExceeded,
            "selected agent evidence exceeds the reviewed byte limit",
        ));
    }
    let derived_plan = local_bundle
        .plan
        .authorize_agent_enhancement(
            &local_bundle.source,
            &local_bundle.probe,
            AgentEnhancementSelection {
                provider: selection.provider.clone(),
                selected_node_ids: selection.selected_node_ids.clone(),
                disclosed_bytes,
                retention: selection.retention.clone(),
                redaction: selection.redaction.clone(),
            },
        )
        .map_err(|error| import_error(&error))?;
    let validated = AsciiDocV1ProposalValidator
        .render_and_validate(
            &local_bundle.source,
            &local_bundle.source_bytes,
            &derived_plan,
            &local_bundle.document,
        )
        .map_err(|error| import_error(&error))?;
    let preview_receipt = ImportReceipt::create(
        created_at,
        &local_bundle.source,
        &derived_plan,
        &local_bundle.document,
        &validated,
        local_bundle.components.clone(),
        CommitResult::PreviewOnly,
    )
    .map_err(|error| import_error(&error))?;
    let mut authorized_bundle = local_bundle.clone();
    authorized_bundle.plan = derived_plan;
    authorized_bundle.proposal = validated.proposal().clone();
    authorized_bundle.proposal_digest = validated.proposal_digest().clone();
    authorized_bundle.preview_receipt = preview_receipt;
    authorized_bundle.bundle_digest = authorized_bundle.compute_digest()?;
    validate_preview_bundle(&authorized_bundle)?;

    let mut preview = AgentEnhancementPreview {
        contract_version: AGENT_ENHANCEMENT_PREVIEW_CONTRACT_VERSION.to_owned(),
        preview_digest: sha256_bytes(b"pending"),
        base_bundle_digest: local_bundle.bundle_digest.clone(),
        local_plan: local_bundle.plan.clone(),
        selection,
        evidence,
        authorized_bundle,
    };
    preview.preview_digest = preview.compute_digest()?;
    validate_agent_enhancement_preview(&preview)?;
    Ok(preview)
}

/// Validates an immutable selection/egress preview without rerunning extraction.
///
/// # Errors
///
/// Returns an error for any altered digest, selection, IR fragment, plan,
/// proposal, receipt, or disclosed-byte count.
pub fn validate_agent_enhancement_preview(
    preview: &AgentEnhancementPreview,
) -> Result<(), IntakeError> {
    if preview.contract_version != AGENT_ENHANCEMENT_PREVIEW_CONTRACT_VERSION
        || preview.preview_digest != preview.compute_digest()?
    {
        return Err(invalid_bundle_error(
            "agent enhancement preview digest or contract is invalid",
        ));
    }
    validate_preview_bundle(&preview.authorized_bundle)?;
    validate_selection(&preview.authorized_bundle, &preview.selection)?;
    preview
        .evidence
        .validate(&preview.authorized_bundle, &preview.selection)?;
    let evidence_bytes = preview.evidence.to_bytes()?;
    let disclosed_bytes = u64::try_from(evidence_bytes.len()).unwrap_or(u64::MAX);
    let expected_plan = preview
        .local_plan
        .authorize_agent_enhancement(
            &preview.authorized_bundle.source,
            &preview.authorized_bundle.probe,
            AgentEnhancementSelection {
                provider: preview.selection.provider.clone(),
                selected_node_ids: preview.selection.selected_node_ids.clone(),
                disclosed_bytes,
                retention: preview.selection.retention.clone(),
                redaction: preview.selection.redaction.clone(),
            },
        )
        .map_err(|error| import_error(&error))?;
    if expected_plan != preview.authorized_bundle.plan {
        return Err(invalid_bundle_error(
            "agent enhancement plan differs from its local no-egress base",
        ));
    }
    match &preview.authorized_bundle.plan.egress {
        EgressDisclosure::AgentSelectedEvidence {
            provider,
            selected_node_ids,
            disclosed_bytes: planned_bytes,
            retention,
            redaction,
        } if provider == &preview.selection.provider
            && selected_node_ids == &preview.selection.selected_node_ids
            && *planned_bytes == disclosed_bytes
            && retention == &preview.selection.retention
            && redaction == &preview.selection.redaction => {}
        _ => {
            return Err(invalid_bundle_error(
                "agent enhancement disclosure differs from the exact evidence payload",
            ));
        }
    }
    Ok(())
}

/// Applies only a typed patch bound to an approved selection, then regenerates
/// the exact canonical proposal and `PreviewOnly` receipt.
///
/// No `AsciiDoc` text supplied by the agent is accepted and no workspace write
/// occurs here.
///
/// # Errors
///
/// Returns an error for an altered preview, stale/out-of-scope/whole-contract
/// patch, invalid regenerated IR, or inconsistent proposal/receipt authority.
pub fn apply_approved_agent_patch(
    approved: &AgentEnhancementPreview,
    patch: &AgentImportPatch,
    created_at: impl Into<String>,
) -> Result<ImportPreviewBundle, IntakeError> {
    validate_agent_enhancement_preview(approved)?;
    let bundle = &approved.authorized_bundle;
    let document = apply_agent_patch(&bundle.document, patch, &bundle.source, &bundle.plan)
        .map_err(|error| import_error(&error))?;
    let validated = AsciiDocV1ProposalValidator
        .render_and_validate(
            &bundle.source,
            &bundle.source_bytes,
            &bundle.plan,
            &document,
        )
        .map_err(|error| import_error(&error))?;
    let preview_receipt = ImportReceipt::create(
        created_at,
        &bundle.source,
        &bundle.plan,
        &document,
        &validated,
        bundle.components.clone(),
        CommitResult::PreviewOnly,
    )
    .map_err(|error| import_error(&error))?;
    let mut enhanced = bundle.clone();
    enhanced.document = document;
    enhanced.proposal = validated.proposal().clone();
    enhanced.proposal_digest = validated.proposal_digest().clone();
    enhanced.preview_receipt = preview_receipt;
    enhanced.bundle_digest = enhanced.compute_digest()?;
    validate_preview_bundle(&enhanced)?;
    if enhanced.preview_receipt.agent_provenance.is_empty() {
        return Err(invalid_bundle_error(
            "agent-enhanced preview omitted typed patch provenance",
        ));
    }
    Ok(enhanced)
}

fn validate_selection(
    bundle: &ImportPreviewBundle,
    selection: &AgentEvidenceSelection,
) -> Result<(), IntakeError> {
    if selection.provider.trim().is_empty()
        || selection.retention.trim().is_empty()
        || selection.redaction.trim().is_empty()
        || selection.selected_node_ids.is_empty()
    {
        return Err(invalid_bundle_error(
            "agent evidence selection requires provider, targets, retention, and redaction",
        ));
    }
    let unique = selection
        .selected_node_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if unique.len() != selection.selected_node_ids.len()
        || unique
            .iter()
            .any(|node_id| !bundle.document.contains_node(node_id))
    {
        return Err(invalid_bundle_error(
            "agent evidence selection contains duplicate or unavailable IR node ids",
        ));
    }
    Ok(())
}

fn find_node<'a>(nodes: &'a [ImportNode], id: &str) -> Option<&'a ImportNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let ImportNodeKind::Section { children, .. } = &node.kind
            && let Some(found) = find_node(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn sanitize_selected_node(node: &ImportNode, selected: &BTreeSet<&str>) -> ImportNode {
    let mut sanitized = node.clone();
    if let ImportNodeKind::Section { children, .. } = &mut sanitized.kind {
        *children = children
            .iter()
            .filter(|child| selected.contains(child.id.as_str()))
            .map(|child| sanitize_selected_node(child, selected))
            .collect();
    }
    sanitized
}

fn figure_resource_ids(node: &ImportNode) -> Vec<&str> {
    match &node.kind {
        ImportNodeKind::Figure { resource_id, .. } => vec![resource_id.as_str()],
        ImportNodeKind::Section { children, .. } => {
            children.iter().flat_map(figure_resource_ids).collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use weftext_core::{create_workspace, read_workspace_revision};
    use weftext_import::{
        AgentEnhancementPolicy, AgentPatchOperation, CancellationToken, EgressDisclosure,
        FakeAdapter, FakeWorker, ImportPipeline, ImportTempRoot, IntakeRequest, OriginClass,
        PlanRequest, PortablePath, Sha256Digest,
    };

    use super::*;

    #[test]
    fn selection_precedes_egress_and_typed_patch_regenerates_exact_preview() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let local = local_fake_bundle(&temporary, &workspace);
        let target = local.document.nodes[0].id.clone();
        let selection = AgentEvidenceSelection {
            provider: "reviewed-provider".to_owned(),
            selected_node_ids: vec![target.clone()],
            retention: "no-training; 30-day deletion".to_owned(),
            redaction: "no additional redaction".to_owned(),
        };

        let preview = prepare_agent_enhancement(&local, selection.clone(), "2026-08-24T00:00:01Z")
            .expect("selection preview");
        assert!(matches!(
            preview.local_plan.agent_enhancement,
            AgentEnhancementPolicy::Disabled
        ));
        assert_eq!(preview.evidence.selected_node_ids, vec![target.clone()]);
        assert!(!preview.evidence.to_bytes().unwrap().is_empty());

        let replacement = "Agent-corrected paragraph";
        let expected_text_digest = text_digest(&local.document.nodes[0]);
        let patch = AgentImportPatch::create(
            local.document.revision.clone(),
            vec![target.clone()],
            vec![AgentPatchOperation::CorrectText {
                node_id: target,
                expected_text_digest,
                replacement: replacement.to_owned(),
            }],
            selection.provider,
            "reviewed-model",
            preview.authorized_bundle.plan.egress.clone(),
        )
        .expect("typed patch");
        let enhanced = apply_approved_agent_patch(&preview, &patch, "2026-08-24T00:00:02Z")
            .expect("enhanced preview");

        assert!(
            enhanced.proposal.nodes[0]
                .exact_asciidoc
                .contains(replacement)
        );
        assert_eq!(enhanced.preview_receipt.agent_provenance.len(), 1);
        assert_ne!(enhanced.bundle_digest, local.bundle_digest);
        assert_ne!(
            enhanced.preview_receipt.receipt_id,
            local.preview_receipt.receipt_id
        );
    }

    #[test]
    fn evidence_and_patch_scope_fail_closed_without_disclosing_other_nodes() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let local = local_fake_bundle(&temporary, &workspace);
        let selected = local.document.nodes[0].id.clone();
        let preview = prepare_agent_enhancement(
            &local,
            AgentEvidenceSelection {
                provider: "reviewed-provider".to_owned(),
                selected_node_ids: vec![selected.clone()],
                retention: "delete-after-call".to_owned(),
                redaction: "none".to_owned(),
            },
            "2026-08-24T00:00:01Z",
        )
        .expect("selection preview");
        let bytes = String::from_utf8(preview.evidence.to_bytes().unwrap()).unwrap();
        assert!(!bytes.contains("C:\\"));
        assert!(!bytes.contains(workspace.to_string_lossy().as_ref()));

        let patch = AgentImportPatch::create(
            local.document.revision.clone(),
            vec!["unavailable-node".to_owned()],
            vec![AgentPatchOperation::CorrectText {
                node_id: "unavailable-node".to_owned(),
                expected_text_digest: Sha256Digest::parse(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )
                .unwrap(),
                replacement: "forged".to_owned(),
            }],
            "reviewed-provider",
            "reviewed-model",
            EgressDisclosure::AgentSelectedEvidence {
                provider: "reviewed-provider".to_owned(),
                selected_node_ids: vec!["unavailable-node".to_owned()],
                disclosed_bytes: 1,
                retention: "delete-after-call".to_owned(),
                redaction: "none".to_owned(),
            },
        )
        .expect("structurally typed patch");
        assert!(apply_approved_agent_patch(&preview, &patch, "2026-08-24T00:00:02Z").is_err());
        assert_eq!(selected, preview.evidence.selected_node_ids[0]);
    }

    #[test]
    fn external_agent_contract_files_are_exact_bounded_and_non_overwriting() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let local = local_fake_bundle(&temporary, &workspace);
        let selection = AgentEvidenceSelection {
            provider: "reviewed-provider".to_owned(),
            selected_node_ids: vec![local.document.nodes[0].id.clone()],
            retention: "delete-after-call".to_owned(),
            redaction: "selected-ir-only".to_owned(),
        };
        let selection_path = temporary.path().join("selection.json");
        std::fs::write(
            &selection_path,
            serde_json::to_vec_pretty(&selection).expect("selection JSON"),
        )
        .expect("selection file");
        assert_eq!(
            read_agent_evidence_selection(&selection_path).expect("exact selection"),
            selection
        );
        let duplicate_path = temporary.path().join("duplicate-selection.json");
        std::fs::write(
            &duplicate_path,
            format!(
                "{{\"provider\":\"first\",\"provider\":\"second\",\"selectedNodeIds\":[\"{}\"],\"retention\":\"delete\",\"redaction\":\"none\"}}",
                local.document.nodes[0].id
            ),
        )
        .expect("duplicate-key selection");
        assert!(read_agent_evidence_selection(&duplicate_path).is_err());

        let preview = prepare_agent_enhancement(&local, selection, "2026-08-24T00:00:01Z")
            .expect("agent review");
        let review_path = temporary.path().join("agent-review.json");
        write_agent_enhancement_preview(&workspace, &review_path, &preview)
            .expect("write exact review");
        assert_eq!(
            read_agent_enhancement_preview(&review_path).expect("read exact review"),
            preview
        );
        assert!(write_agent_enhancement_preview(&workspace, &review_path, &preview).is_err());
        let inside_workspace = workspace.join("agent-review.json");
        assert!(write_agent_enhancement_preview(&workspace, &inside_workspace, &preview).is_err());
        assert!(!inside_workspace.exists());

        let evidence_path = temporary.path().join("agent-evidence.json");
        write_agent_import_evidence(&workspace, &evidence_path, &preview)
            .expect("write exact evidence");
        assert_eq!(
            std::fs::read(evidence_path).expect("evidence bytes"),
            preview.evidence.to_bytes().expect("reviewed evidence")
        );
    }

    fn local_fake_bundle(
        temporary: &tempfile::TempDir,
        workspace: &std::path::Path,
    ) -> ImportPreviewBundle {
        let source_bytes = b"WEFTEXT-FAKE/1\nAgent import\nFake import paragraph\n".to_vec();
        let limits = crate::fake_limits();
        let temp_root =
            ImportTempRoot::initialize(temporary.path().join("intake")).expect("temp root");
        let pipeline = ImportPipeline::new(temp_root, Arc::new(AsciiDocV1ProposalValidator));
        let preview = pipeline
            .preview(
                IntakeRequest {
                    display_name: "sample.fake".to_owned(),
                    origin: OriginClass::LocalFile,
                    bytes: source_bytes.clone(),
                    plan: PlanRequest::single_node(
                        PortablePath::parse("Imported").expect("destination"),
                    ),
                    limits,
                    cancellation: CancellationToken::default(),
                },
                &FakeAdapter,
                Arc::new(FakeWorker::success()),
            )
            .expect("local preview");
        let receipt = preview
            .receipt("2026-08-24T00:00:00Z", CommitResult::PreviewOnly)
            .expect("receipt");
        ImportPreviewBundle::create(
            source_bytes,
            preview,
            read_workspace_revision(workspace).expect("workspace revision"),
            receipt,
        )
        .expect("bundle")
    }

    fn text_digest(node: &ImportNode) -> Sha256Digest {
        let ImportNodeKind::Paragraph { text } = &node.kind else {
            panic!("fake worker first node must be a paragraph");
        };
        weftext_import::sha256_bytes(text.as_bytes())
    }
}
