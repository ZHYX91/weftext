//! Read-only diagnostics over the already authorized, frozen inventory.
use crate::*;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeConnectivity {
    pub node: NodeRef,
    pub incoming: usize,
    pub outgoing: usize,
    pub invalid_outgoing: usize,
    /// Absence is conclusive only when every source in the declared scope was checked.
    pub state: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDiagnostics {
    pub generation: String,
    pub coverage: Coverage,
    pub scope: &'static str,
    pub scanned_nodes: usize,
    pub issues: Vec<LinkEdge>,
    pub property_issues: Vec<PropertyDiagnostic>,
    pub nodes: Vec<NodeConnectivity>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDiagnostic {
    pub source: NodeRef,
    pub field_id: String,
    pub item_path: Vec<String>,
    pub address: String,
    pub resolution: LinkResolution,
}

pub fn link_diagnostics(
    inventory: &InventorySnapshot,
    properties: &[PropertyReferenceInput],
    properties_complete: bool,
    edge_budget: usize,
) -> LinkDiagnostics {
    let mut result = LinkDiagnostics {
        generation: inventory.generation.clone(),
        coverage: if properties_complete {
            inventory.coverage
        } else {
            Coverage::Partial
        },
        scope: "authorizedNativeSourcesAndTypedReferences",
        scanned_nodes: 0,
        issues: vec![],
        property_issues: vec![],
        nodes: vec![],
    };
    let mut counts: BTreeMap<_, (usize, usize, usize)> = inventory
        .nodes
        .iter()
        .map(|n| (n.owner.clone(), (0, 0, 0)))
        .collect();
    let mut remaining = edge_budget;
    let mut remaining_bytes = 4 * 1024 * 1024usize;
    for source in &inventory.nodes {
        let scan = native_links_bounded(&source.source, remaining, remaining_bytes);
        remaining = remaining.saturating_sub(scan.links.len());
        remaining_bytes = remaining_bytes.saturating_sub(scan.estimated_bytes);
        if !scan.complete || !source_link_projection_complete(&source.source, &scan.links) {
            result.coverage = Coverage::Partial;
        }
        result.scanned_nodes += 1;
        for occurrence in scan.links {
            // Intra-document references never become cross-note connections.
            if occurrence.target.starts_with('#') {
                continue;
            }
            let resolution = resolve_link(&occurrence.target, source, inventory);
            if resolution.status == "unavailable"
                || resolution.target.is_none() && resolution.status == "ambiguous"
            {
                result.coverage = Coverage::Partial;
            }
            if let Some(target) = &resolution.target {
                // Node identity and internal-fragment validity are independent.
                if target != &source.owner {
                    counts.get_mut(&source.owner).unwrap().1 += 1;
                    if let Some(count) = counts.get_mut(target) {
                        count.0 += 1;
                    }
                }
            }
            if !matches!(resolution.status.as_str(), "resolved" | "external") {
                counts.get_mut(&source.owner).unwrap().2 += 1;
                result.issues.push(LinkEdge {
                    source: source.owner.clone(),
                    occurrence,
                    resolution,
                });
            }
        }
        if !scan.complete {
            break;
        }
    }
    for input in properties {
        let Some(source) = inventory.nodes.iter().find(|n| n.owner == input.source) else {
            continue;
        };
        let resolution = resolve_link(&input.target, source, inventory);
        if !matches!(resolution.status.as_str(), "resolved" | "external") {
            counts.get_mut(&input.source).unwrap().2 += 1;
            if resolution.status == "unavailable"
                || resolution.target.is_none() && resolution.status == "ambiguous"
            {
                result.coverage = Coverage::Partial;
            }
            result.property_issues.push(PropertyDiagnostic {
                source: input.source.clone(),
                field_id: input.field_id.clone(),
                item_path: input.item_path.clone(),
                address: input.target.clone(),
                resolution: resolution.clone(),
            });
        }
        if let Some(target) = resolution.target.filter(|target| target != &input.source)
            && counts.contains_key(&target)
        {
            counts.get_mut(&input.source).unwrap().1 += 1;
            counts.get_mut(&target).unwrap().0 += 1;
        }
    }
    result.nodes = counts
        .into_iter()
        .map(|(node, (incoming, outgoing, invalid_outgoing))| {
            let state = if incoming > 0 || outgoing > 0 {
                "connected"
            } else if result.coverage == Coverage::Partial {
                "incomplete"
            } else if invalid_outgoing > 0 {
                "invalidOutgoing"
            } else {
                "isolated"
            };
            NodeConnectivity {
                node,
                incoming,
                outgoing,
                invalid_outgoing,
                state: state.into(),
            }
        })
        .collect();
    result
}
