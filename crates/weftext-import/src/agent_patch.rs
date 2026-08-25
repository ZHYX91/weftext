use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::contract::require_version;
use crate::{
    AGENT_PATCH_CONTRACT_VERSION, AgentEnhancementPolicy, EgressDisclosure, ImportDocument,
    ImportError, ImportErrorCode, ImportNode, ImportNodeKind, ImportPlan, ProvenanceKind,
    ProvenanceRecord, Sha256Digest, SourceArtifact, sha256_bytes,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
pub enum AgentPatchOperation {
    CorrectText {
        node_id: String,
        expected_text_digest: Sha256Digest,
        replacement: String,
    },
    ClassifyHeading {
        node_id: String,
        level: u8,
        title: String,
    },
    RepairReadingOrder {
        parent_node_id: Option<String>,
        ordered_child_ids: Vec<String>,
    },
    ReconstructTable {
        node_id: String,
        header_rows: u16,
        rows: Vec<Vec<String>>,
    },
    TranscribeFormula {
        node_id: String,
        notation: String,
        source: String,
    },
    DescribeFigure {
        node_id: String,
        description: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentImportPatch {
    pub contract_version: String,
    pub patch_id: String,
    pub base_ir_revision: Sha256Digest,
    pub selected_node_ids: Vec<String>,
    pub operations: Vec<AgentPatchOperation>,
    pub provider: String,
    pub model: String,
    pub egress: EgressDisclosure,
}

impl AgentImportPatch {
    /// Creates a content-addressed typed patch over selected IR nodes.
    ///
    /// # Errors
    ///
    /// Returns an error when the patch material cannot be serialized.
    pub fn create(
        base_ir_revision: Sha256Digest,
        selected_node_ids: Vec<String>,
        operations: Vec<AgentPatchOperation>,
        provider: impl Into<String>,
        model: impl Into<String>,
        egress: EgressDisclosure,
    ) -> Result<Self, ImportError> {
        let provider = provider.into();
        let model = model.into();
        let material = serde_json::to_vec(&(
            &base_ir_revision,
            &selected_node_ids,
            &operations,
            &provider,
            &model,
            &egress,
        ))
        .map_err(|error| ImportError::serialization(&error))?;
        let digest = sha256_bytes(&material);
        Ok(Self {
            contract_version: AGENT_PATCH_CONTRACT_VERSION.to_owned(),
            patch_id: format!("agent-patch-{}", &digest.as_str()[..24]),
            base_ir_revision,
            selected_node_ids,
            operations,
            provider,
            model,
            egress,
        })
    }

    fn validate(&self, document: &ImportDocument, plan: &ImportPlan) -> Result<(), ImportError> {
        require_version(
            &self.contract_version,
            AGENT_PATCH_CONTRACT_VERSION,
            "agent import patch",
        )?;
        if self.base_ir_revision != document.revision {
            return Err(ImportError::new(
                ImportErrorCode::StaleAgentPatch,
                "agent patch base IR revision is stale",
            ));
        }
        let expected = Self::create(
            self.base_ir_revision.clone(),
            self.selected_node_ids.clone(),
            self.operations.clone(),
            self.provider.clone(),
            self.model.clone(),
            self.egress.clone(),
        )?;
        if expected.patch_id != self.patch_id {
            return invalid_patch("agent patch id does not match its typed operations");
        }
        plan.limits.check(
            "agent selected node count",
            u64::try_from(self.selected_node_ids.len()).unwrap_or(u64::MAX),
            u64::from(plan.limits.max_agent_selected_nodes),
        )?;
        plan.limits.check(
            "agent patch operation count",
            u64::try_from(self.operations.len()).unwrap_or(u64::MAX),
            u64::from(plan.limits.max_agent_operations),
        )?;
        let serialized =
            serde_json::to_vec(self).map_err(|error| ImportError::serialization(&error))?;
        plan.limits.check(
            "agent patch bytes",
            u64::try_from(serialized.len()).unwrap_or(u64::MAX),
            plan.limits.max_agent_output_bytes,
        )?;
        if self.selected_node_ids.is_empty() || self.operations.is_empty() {
            return invalid_patch("agent patches require selected nodes and typed operations");
        }
        let selected = self
            .selected_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if selected.len() != self.selected_node_ids.len()
            || selected.iter().any(|id| !document.contains_node(id))
        {
            return Err(ImportError::new(
                ImportErrorCode::AgentPatchOutOfScope,
                "agent patch selection contains duplicate or unavailable IR node ids",
            ));
        }
        let AgentEnhancementPolicy::SelectedRegionsOnly { provider } = &plan.agent_enhancement
        else {
            return invalid_patch("the import plan did not authorize agent enhancement");
        };
        if provider != &self.provider {
            return invalid_patch("agent patch provider differs from the reviewed import plan");
        }
        if self.egress != plan.egress {
            return invalid_patch("agent patch egress differs from the reviewed import plan");
        }
        match &self.egress {
            EgressDisclosure::AgentSelectedEvidence {
                provider,
                selected_node_ids,
                disclosed_bytes,
                ..
            } if provider == &self.provider
                && selected_node_ids == &self.selected_node_ids
                && *disclosed_bytes <= plan.limits.max_agent_output_bytes => {}
            _ => {
                return invalid_patch(
                    "agent patch egress must disclose the exact provider, selected IR nodes, and bounded bytes",
                );
            }
        }

        let mut operation_keys = BTreeSet::new();
        for operation in &self.operations {
            let targets = operation_targets(operation);
            if targets
                .iter()
                .any(|target| !selected.contains(target.as_str()))
            {
                return Err(ImportError::new(
                    ImportErrorCode::AgentPatchOutOfScope,
                    "agent patch operation refers outside the selected IR nodes",
                ));
            }
            let key = operation_key(operation);
            if !operation_keys.insert(key) {
                return invalid_patch("an agent patch may modify each typed target only once");
            }
        }
        Ok(())
    }
}

/// Applies a stale-checked typed patch and revalidates the complete IR.
///
/// # Errors
///
/// Returns an error for stale, out-of-scope, malformed, over-limit, or
/// semantically invalid operations.
pub fn apply_agent_patch(
    document: &ImportDocument,
    patch: &AgentImportPatch,
    source: &SourceArtifact,
    plan: &ImportPlan,
) -> Result<ImportDocument, ImportError> {
    document.validate(source, plan)?;
    patch.validate(document, plan)?;
    let mut patched = document.clone();
    let provenance = ProvenanceRecord {
        kind: ProvenanceKind::AgentEnhancement,
        component_id: patch.provider.clone(),
        component_version: patch.model.clone(),
        input_digests: vec![document.revision.clone()],
        output_digest: None,
        source_locations: Vec::new(),
    };
    for operation in &patch.operations {
        apply_operation(&mut patched, operation, &provenance)?;
    }
    patched.provenance.push(provenance);
    patched.recompute_revision()?;
    patched.validate(source, plan)?;
    Ok(patched)
}

// Keeping the exhaustive operation dispatch together makes the typed, closed
// patch surface auditable as one boundary.
#[allow(clippy::too_many_lines)]
fn apply_operation(
    document: &mut ImportDocument,
    operation: &AgentPatchOperation,
    provenance: &ProvenanceRecord,
) -> Result<(), ImportError> {
    match operation {
        AgentPatchOperation::CorrectText {
            node_id,
            expected_text_digest,
            replacement,
        } => {
            let node = find_node_mut(&mut document.nodes, node_id)?;
            let ImportNodeKind::Paragraph { text } = &mut node.kind else {
                return invalid_patch("correct_text targets only paragraph IR nodes");
            };
            if sha256_bytes(text.as_bytes()) != *expected_text_digest {
                return Err(ImportError::new(
                    ImportErrorCode::StaleAgentPatch,
                    "agent text correction evidence is stale",
                ));
            }
            text.clone_from(replacement);
            node.provenance.push(provenance.clone());
        }
        AgentPatchOperation::ClassifyHeading {
            node_id,
            level,
            title,
        } => {
            let node = find_node_mut(&mut document.nodes, node_id)?;
            if !matches!(
                &node.kind,
                ImportNodeKind::Paragraph { .. } | ImportNodeKind::Section { .. }
            ) {
                return invalid_patch(
                    "classify_heading targets only paragraph or section IR nodes",
                );
            }
            let children = match std::mem::replace(
                &mut node.kind,
                ImportNodeKind::Paragraph {
                    text: String::new(),
                },
            ) {
                ImportNodeKind::Section { children, .. } => children,
                _ => Vec::new(),
            };
            node.kind = ImportNodeKind::Section {
                level: *level,
                title: title.clone(),
                children,
            };
            node.provenance.push(provenance.clone());
        }
        AgentPatchOperation::RepairReadingOrder {
            parent_node_id,
            ordered_child_ids,
        } => {
            let children = match parent_node_id {
                Some(parent) => {
                    let node = find_node_mut(&mut document.nodes, parent)?;
                    let ImportNodeKind::Section { children, .. } = &mut node.kind else {
                        return invalid_patch("reading-order parent must be a section IR node");
                    };
                    node.provenance.push(provenance.clone());
                    children
                }
                None => &mut document.nodes,
            };
            reorder_exact(children, ordered_child_ids)?;
        }
        AgentPatchOperation::ReconstructTable {
            node_id,
            header_rows,
            rows,
        } => {
            let node = find_node_mut(&mut document.nodes, node_id)?;
            if !matches!(
                &node.kind,
                ImportNodeKind::Paragraph { .. } | ImportNodeKind::Table { .. }
            ) {
                return invalid_patch("reconstruct_table targets only paragraph or table IR nodes");
            }
            node.kind = ImportNodeKind::Table {
                header_rows: *header_rows,
                rows: rows.clone(),
            };
            node.provenance.push(provenance.clone());
        }
        AgentPatchOperation::TranscribeFormula {
            node_id,
            notation,
            source,
        } => {
            let node = find_node_mut(&mut document.nodes, node_id)?;
            if !matches!(
                &node.kind,
                ImportNodeKind::Paragraph { .. } | ImportNodeKind::Formula { .. }
            ) {
                return invalid_patch(
                    "transcribe_formula targets only paragraph or formula IR nodes",
                );
            }
            node.kind = ImportNodeKind::Formula {
                notation: notation.clone(),
                source: source.clone(),
            };
            node.provenance.push(provenance.clone());
        }
        AgentPatchOperation::DescribeFigure {
            node_id,
            description,
        } => {
            let node = find_node_mut(&mut document.nodes, node_id)?;
            let ImportNodeKind::Figure { alt, .. } = &mut node.kind else {
                return invalid_patch("describe_figure targets only figure IR nodes");
            };
            alt.clone_from(description);
            node.provenance.push(provenance.clone());
        }
    }
    Ok(())
}

fn find_node_mut<'a>(
    nodes: &'a mut [ImportNode],
    id: &str,
) -> Result<&'a mut ImportNode, ImportError> {
    find_node_mut_option(nodes, id).ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::AgentPatchOutOfScope,
            "agent patch target is not present in the IR",
        )
    })
}

fn find_node_mut_option<'a>(nodes: &'a mut [ImportNode], id: &str) -> Option<&'a mut ImportNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let ImportNodeKind::Section { children, .. } = &mut node.kind
            && let Some(found) = find_node_mut_option(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn reorder_exact(
    children: &mut Vec<ImportNode>,
    ordered_ids: &[String],
) -> Result<(), ImportError> {
    let current = children
        .iter()
        .map(|child| child.id.as_str())
        .collect::<BTreeSet<_>>();
    let requested = ordered_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if current != requested || requested.len() != ordered_ids.len() {
        return invalid_patch(
            "reading-order repair must be an exact permutation of the existing siblings",
        );
    }
    let mut old = std::mem::take(children);
    let mut reordered = Vec::with_capacity(old.len());
    for id in ordered_ids {
        let index = old
            .iter()
            .position(|node| node.id == *id)
            .expect("validated permutation contains every id exactly once");
        reordered.push(old.remove(index));
    }
    *children = reordered;
    Ok(())
}

fn operation_targets(operation: &AgentPatchOperation) -> Vec<String> {
    match operation {
        AgentPatchOperation::CorrectText { node_id, .. }
        | AgentPatchOperation::ClassifyHeading { node_id, .. }
        | AgentPatchOperation::ReconstructTable { node_id, .. }
        | AgentPatchOperation::TranscribeFormula { node_id, .. }
        | AgentPatchOperation::DescribeFigure { node_id, .. } => vec![node_id.clone()],
        AgentPatchOperation::RepairReadingOrder {
            parent_node_id,
            ordered_child_ids,
        } => parent_node_id
            .iter()
            .cloned()
            .chain(ordered_child_ids.iter().cloned())
            .collect(),
    }
}

fn operation_key(operation: &AgentPatchOperation) -> String {
    match operation {
        AgentPatchOperation::CorrectText { node_id, .. } => format!("text:{node_id}"),
        AgentPatchOperation::ClassifyHeading { node_id, .. } => format!("heading:{node_id}"),
        AgentPatchOperation::RepairReadingOrder { parent_node_id, .. } => {
            format!("order:{}", parent_node_id.as_deref().unwrap_or("$document"))
        }
        AgentPatchOperation::ReconstructTable { node_id, .. } => format!("table:{node_id}"),
        AgentPatchOperation::TranscribeFormula { node_id, .. } => format!("formula:{node_id}"),
        AgentPatchOperation::DescribeFigure { node_id, .. } => format!("figure:{node_id}"),
    }
}

fn invalid_patch<T>(message: &str) -> Result<T, ImportError> {
    Err(ImportError::new(
        ImportErrorCode::InvalidAgentPatch,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use super::{AgentImportPatch, AgentPatchOperation, apply_agent_patch};
    use crate::{
        AgentEnhancementPolicy, Confidence, EgressDisclosure, FakeAdapter, ImportAdapter,
        ImportDocument, ImportErrorCode, ImportLimits, ImportNode, ImportNodeKind, LocalOcrPolicy,
        OriginClass, PlanRequest, PortablePath, ResourcePolicy, SourceArtifact, SplitPolicy,
        sha256_bytes,
    };

    #[test]
    fn stale_and_out_of_scope_agent_patches_are_rejected() {
        let (source, plan, document) = fixture();
        let stale = AgentImportPatch::create(
            sha256_bytes(b"old IR"),
            vec!["paragraph-1".to_owned()],
            vec![AgentPatchOperation::CorrectText {
                node_id: "paragraph-1".to_owned(),
                expected_text_digest: sha256_bytes(b"uncertain text"),
                replacement: "corrected".to_owned(),
            }],
            "test-agent",
            "model-1",
            disclosure(&["paragraph-1"]),
        )
        .expect("stale patch");
        let error = apply_agent_patch(&document, &stale, &source, &plan)
            .expect_err("stale patch must fail");
        assert_eq!(error.code(), ImportErrorCode::StaleAgentPatch);

        let outside = AgentImportPatch::create(
            document.revision.clone(),
            vec!["paragraph-1".to_owned()],
            vec![AgentPatchOperation::CorrectText {
                node_id: "paragraph-2".to_owned(),
                expected_text_digest: sha256_bytes(b"stable text"),
                replacement: "not authorized".to_owned(),
            }],
            "test-agent",
            "model-1",
            disclosure(&["paragraph-1"]),
        )
        .expect("outside patch");
        let error = apply_agent_patch(&document, &outside, &source, &plan)
            .expect_err("out-of-scope patch must fail");
        assert_eq!(error.code(), ImportErrorCode::AgentPatchOutOfScope);
    }

    #[test]
    fn typed_patch_changes_only_the_selected_ir_node() {
        let (source, plan, document) = fixture();
        let patch = AgentImportPatch::create(
            document.revision.clone(),
            vec!["paragraph-1".to_owned()],
            vec![AgentPatchOperation::CorrectText {
                node_id: "paragraph-1".to_owned(),
                expected_text_digest: sha256_bytes(b"uncertain text"),
                replacement: "修正后的文本".to_owned(),
            }],
            "test-agent",
            "model-1",
            disclosure(&["paragraph-1"]),
        )
        .expect("patch");

        let patched = apply_agent_patch(&document, &patch, &source, &plan).expect("apply patch");

        assert_ne!(patched.revision, document.revision);
        assert_eq!(
            patched.nodes[0].kind,
            ImportNodeKind::Paragraph {
                text: "修正后的文本".to_owned()
            }
        );
        assert_eq!(patched.nodes[1], document.nodes[1]);
        assert!(
            patched
                .provenance
                .iter()
                .any(|record| { record.kind == crate::ProvenanceKind::AgentEnhancement })
        );
    }

    #[test]
    fn whole_document_replacement_is_not_in_the_patch_schema() {
        let forged = serde_json::json!({
            "type": "replace_document",
            "source": "---\nweftext: {}\n---\n= forged"
        });
        assert!(serde_json::from_value::<AgentPatchOperation>(forged).is_err());
    }

    #[test]
    fn patch_and_operation_contracts_reject_unknown_fields() {
        let (_, plan, document) = fixture();
        let patch = AgentImportPatch::create(
            document.revision.clone(),
            vec!["paragraph-1".to_owned()],
            vec![AgentPatchOperation::CorrectText {
                node_id: "paragraph-1".to_owned(),
                expected_text_digest: crate::sha256_bytes(b"uncertain text"),
                replacement: "corrected".to_owned(),
            }],
            "test-agent",
            "model-v1",
            plan.egress,
        )
        .expect("patch");
        let mut patch_value = serde_json::to_value(&patch).expect("patch JSON");
        patch_value
            .as_object_mut()
            .expect("patch object")
            .insert("rawAsciiDoc".to_owned(), serde_json::json!("forged"));
        assert!(serde_json::from_value::<AgentImportPatch>(patch_value).is_err());

        let mut operation = serde_json::to_value(&patch.operations[0]).expect("operation JSON");
        operation
            .as_object_mut()
            .expect("operation object")
            .insert("path".to_owned(), serde_json::json!("C:/workspace"));
        assert!(serde_json::from_value::<AgentPatchOperation>(operation).is_err());
    }

    fn fixture() -> (SourceArtifact, crate::ImportPlan, ImportDocument) {
        let limits = ImportLimits::default();
        let bytes = b"WEFTEXT-FAKE/1\nTitle\nuncertain text\nstable text\n";
        let source =
            SourceArtifact::from_bytes("agent.fake", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, bytes, &limits).expect("probe");
        let request = PlanRequest {
            destination: PortablePath::parse("AgentTarget").expect("path"),
            split_policy: SplitPolicy::SingleNode,
            resource_policy: ResourcePolicy::ExtractReferenced,
            local_ocr_policy: LocalOcrPolicy::Automatic,
            agent_enhancement: AgentEnhancementPolicy::SelectedRegionsOnly {
                provider: "test-agent".to_owned(),
            },
            egress: disclosure(&["paragraph-1"]),
        };
        let plan = adapter
            .plan(&source, &probe, request, limits)
            .expect("plan");
        let nodes = [
            ("paragraph-1", "uncertain text"),
            ("paragraph-2", "stable text"),
        ]
        .into_iter()
        .map(|(id, text)| ImportNode {
            id: id.to_owned(),
            kind: ImportNodeKind::Paragraph {
                text: text.to_owned(),
            },
            confidence: Confidence::from_basis_points(5_000).expect("confidence"),
            source_locations: Vec::new(),
            provenance: Vec::new(),
        })
        .collect();
        let document = ImportDocument::create(
            "document-agent-test",
            source.sha256.clone(),
            "Title",
            nodes,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("document");
        (source, plan, document)
    }

    fn disclosure(ids: &[&str]) -> EgressDisclosure {
        EgressDisclosure::AgentSelectedEvidence {
            provider: "test-agent".to_owned(),
            selected_node_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            disclosed_bytes: 128,
            retention: "provider retains no request data".to_owned(),
            redaction: "no redaction requested".to_owned(),
        }
    }
}
