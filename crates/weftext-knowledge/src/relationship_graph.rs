//! Read-only graph projection of the same admitted/authorized knowledge cut.
//! It adds neither an author fact store nor directory/heading/potential edges.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub key: String,
    pub reference: String,
    pub title: String,
    pub directory: String,
    /// Direct, separately authorized typed tag references, never inferred ancestors.
    pub tags: Vec<String>,
    pub tags_known: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTag {
    pub reference: String,
    pub tag_id: String,
    pub display_name: String,
    pub parent_tag_id: Option<String>,
}
pub fn graph_tag_tree(workspace: &str, document: &weftext_data::TagDocument) -> Vec<GraphTag> {
    document
        .tags
        .iter()
        .map(|tag| GraphTag {
            reference: format!("weftext-tag://{workspace}/{}", tag.tag_id),
            tag_id: tag.tag_id.clone(),
            display_name: tag.display_name.clone(),
            parent_tag_id: tag.parent_tag_id.clone(),
        })
        .collect()
}
pub fn graph_tag_membership_known(source: &str, schema: &weftext_data::Schema) -> bool {
    weftext_data::narrow_profile(source, schema, &BTreeSet::from(["core/tags".into()]), None)
        .is_ok_and(|profile| {
            profile.fields.iter().any(|field| {
                field.field_id == "core/tags"
                    && !matches!(
                        field.cell,
                        weftext_data::Cell::Invalid { .. } | weftext_data::Cell::Unavailable { .. }
                    )
            })
        })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub key: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    pub direction: String,
    pub derived: bool,
    pub author_source: String,
    pub field_id: Option<String>,
    pub item_path: Vec<String>,
    pub range: Option<ExactRange>,
    pub source_hash: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipGraph {
    pub generation: String,
    pub root: Option<String>,
    /// Present only after a complete, independently authorized Core-qualified
    /// Tags object from this same cut. None does not mean an empty forest.
    pub tag_tree: Option<Vec<GraphTag>>,
    pub scope: &'static str,
    pub coverage: Coverage,
    pub truncated: bool,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
/// Qualified relation semantics for one actual author slot. Hosted callers must
/// independently authorize both the field and relation before supplying these.
#[derive(Debug, Clone)]
pub struct GraphRelation {
    pub source: NodeRef,
    pub field_id: String,
    pub path: Vec<weftext_data::SchemaPath>,
    pub id: String,
    pub direction: String,
    pub inverse: Option<String>,
}
pub fn graph_key(node: &NodeRef) -> String {
    format!("{}/{}", node.workspace_id, node.node_id)
}
pub fn graph_relations(
    source: &NodeSnapshot,
    schema: &weftext_data::Schema,
    fields: Option<&BTreeSet<String>>,
    relations: Option<&BTreeSet<String>>,
) -> Vec<GraphRelation> {
    // Control metadata alone qualifies the declared source type; do not decode
    // any ungranted values just to discover relation definitions.
    let Ok((_, _, resolved_types)) = weftext_data::narrow_query_metadata(&source.source, schema)
    else {
        return vec![];
    };
    schema
        .relations
        .iter()
        .filter(|r| {
            resolved_types.contains(&r.source_type)
                && fields.is_none_or(|f| f.contains(&r.field))
                && relations.is_none_or(|allowed| allowed.contains(&r.id))
        })
        .map(|r| GraphRelation {
            source: source.owner.clone(),
            field_id: r.field.clone(),
            path: r.target_path.clone(),
            id: r.id.clone(),
            direction: r.direction.clone(),
            inverse: r.inverse.clone(),
        })
        .collect()
}
fn matches_path(pattern: &[weftext_data::SchemaPath], path: &[String]) -> bool {
    pattern.len() == path.len() && pattern.iter().zip(path).all(|(pattern, raw)| {
        let Ok(actual) = serde_json::from_str::<weftext_data::ValuePath>(raw) else { return false; };
        matches!((pattern, &actual), (weftext_data::SchemaPath::Each, weftext_data::ValuePath::Index { .. }))
            || matches!((pattern, actual), (weftext_data::SchemaPath::Member { key: a }, weftext_data::ValuePath::Member { key: b }) if a == &b)
    })
}
pub fn relationship_graph(
    inventory: &InventorySnapshot,
    properties: &[PropertyReferenceInput],
    relations: &[GraphRelation],
    properties_complete: bool,
    node_budget: usize,
    edge_budget: usize,
) -> RelationshipGraph {
    let mut result = RelationshipGraph {
        generation: inventory.generation.clone(),
        root: None,
        tag_tree: None,
        scope: "authorizedNativeSourcesAndTypedReferences",
        coverage: if properties_complete {
            inventory.coverage
        } else {
            Coverage::Partial
        },
        truncated: inventory.nodes.len() > node_budget,
        nodes: vec![],
        edges: vec![],
    };
    let admitted = inventory
        .nodes
        .iter()
        .take(node_budget)
        .map(|n| (n.owner.clone(), n))
        .collect::<BTreeMap<_, _>>();
    let hashes = admitted
        .values()
        .map(|node| {
            (
                node.owner.clone(),
                weftext_engine::source_hash(&node.source),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut tags: BTreeMap<&NodeRef, BTreeSet<&str>> = BTreeMap::new();
    for property in properties
        .iter()
        .filter(|p| p.field_id == "core/tags" && p.target.starts_with("weftext-tag://"))
    {
        tags.entry(&property.source)
            .or_default()
            .insert(&property.target);
    }
    let mut by_field: BTreeMap<(&NodeRef, &str), Vec<&GraphRelation>> = BTreeMap::new();
    for relation in relations {
        by_field
            .entry((&relation.source, &relation.field_id))
            .or_default()
            .push(relation);
    }
    for node in admitted.values() {
        result.nodes.push(GraphNode {
            key: graph_key(&node.owner),
            reference: format!(
                "weftext-node://{}/{}",
                node.owner.workspace_id, node.owner.node_id
            ),
            title: node.title.clone().unwrap_or_else(|| "无标题笔记".into()),
            directory: node
                .library_path
                .rsplit_once('/')
                .map_or("", |(directory, _)| directory)
                .into(),
            tags: tags
                .get(&node.owner)
                .into_iter()
                .flat_map(|tags| tags.iter().map(|s| (*s).to_owned()))
                .collect(),
            tags_known: false,
        });
    }
    let mut remaining = edge_budget;
    for source in admitted.values() {
        let scan = native_links_bounded(&source.source, remaining, 4 * 1024 * 1024);
        if !scan.complete {
            result.truncated = true;
        }
        if !source_link_projection_complete(&source.source, &scan.links) {
            result.coverage = Coverage::Partial;
        }
        remaining = remaining.saturating_sub(scan.links.len());
        for occurrence in scan.links {
            if occurrence.target.starts_with('#') {
                continue;
            }
            let resolution = resolve_link(&occurrence.target, source, inventory);
            let Some(target) = resolution
                .target
                .filter(|target| admitted.contains_key(target))
            else {
                continue;
            };
            if result.edges.len() == edge_budget {
                result.truncated = true;
                break;
            }
            result.edges.push(GraphEdge {
                key: format!(
                    "body:{}:{}:{}",
                    graph_key(&source.owner),
                    occurrence.from,
                    occurrence.to
                ),
                source: graph_key(&source.owner),
                target: graph_key(&target),
                relation: "正文链接".into(),
                direction: "directed".into(),
                derived: false,
                author_source: graph_key(&source.owner),
                field_id: None,
                item_path: vec![],
                range: ExactRange::new(&source.source, occurrence.from, occurrence.to).ok(),
                source_hash: hashes[&source.owner].clone(),
            });
        }
        // Reuse the native citation binding certificate. A word resembling a
        // citation and an unclassified @ reference never creates a graph fact.
        let citations = weftext_engine::inspect_native_citation_editor(&source.source);
        for citation in citations.citations.iter().filter(|c| c.binding_proven) {
            if result.edges.len() >= edge_budget {
                result.truncated = true;
                break;
            }
            let Some(binding) = citations.bindings.iter().find(|b| b.key == citation.key) else {
                continue;
            };
            let Some(target) = resolve_link(&binding.work, source, inventory)
                .target
                .filter(|target| admitted.contains_key(target))
            else {
                continue;
            };
            result.edges.push(GraphEdge {
                key: format!(
                    "citation:{}:{}:{}",
                    graph_key(&source.owner),
                    citation.range.from,
                    citation.range.to
                ),
                source: graph_key(&source.owner),
                target: graph_key(&target),
                relation: "文献引用".into(),
                direction: "directed".into(),
                derived: false,
                author_source: graph_key(&source.owner),
                field_id: None,
                item_path: vec![citation.key.clone()],
                range: ExactRange::new(&source.source, citation.range.from, citation.range.to).ok(),
                source_hash: hashes[&source.owner].clone(),
            });
        }
    }
    for (ordinal, property) in properties.iter().enumerate() {
        let Some(source) = admitted.get(&property.source) else {
            continue;
        };
        let resolution = resolve_link(&property.target, source, inventory);
        let Some(target) = resolution
            .target
            .filter(|target| admitted.contains_key(target))
        else {
            continue;
        };
        let matching = by_field
            .get(&(&property.source, property.field_id.as_str()))
            .into_iter()
            .flat_map(|relations| {
                relations
                    .iter()
                    .filter(|r| matches_path(&r.path, &property.item_path))
            })
            .collect::<Vec<_>>();
        let entries = if matching.is_empty() {
            vec![(
                format!("属性 {}", property.field_id),
                "directed".to_owned(),
                None,
            )]
        } else {
            matching
                .iter()
                .map(|r| (r.id.clone(), r.direction.clone(), r.inverse.clone()))
                .collect()
        };
        for (relation, direction, inverse) in entries {
            if result.edges.len() >= edge_budget {
                result.truncated = true;
                break;
            }
            let edge = GraphEdge {
                key: format!("field:{}:{ordinal}:{relation}", graph_key(&source.owner)),
                source: graph_key(&source.owner),
                target: graph_key(&target),
                relation,
                direction,
                derived: false,
                author_source: graph_key(&source.owner),
                field_id: Some(property.field_id.clone()),
                item_path: property.item_path.clone(),
                range: None,
                source_hash: hashes[&source.owner].clone(),
            };
            result.edges.push(edge.clone());
            if let Some(inverse) = inverse {
                if result.edges.len() >= edge_budget {
                    result.truncated = true;
                    break;
                }
                result.edges.push(GraphEdge {
                    key: format!("inverse:{}", edge.key),
                    source: edge.target.clone(),
                    target: edge.source.clone(),
                    relation: inverse,
                    derived: true,
                    ..edge
                });
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    const WORKSPACE: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
    const A: &str = "12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    const B: &str = "8aa10c08-3e7d-4d10-9488-92bc7f393050";
    fn owner(id: &str) -> NodeRef {
        NodeRef {
            workspace_id: WORKSPACE.into(),
            node_id: id.into(),
        }
    }
    fn uri(id: &str) -> String {
        format!("weftext-node://{WORKSPACE}/{id}")
    }
    fn node(id: &str, body: &str) -> NodeSnapshot {
        NodeSnapshot {
            owner: owner(id),
            path: format!("file:///D:/notes/{id}.typ"),
            library_path: format!("notes/{id}.typ"),
            revision_id: "revision".into(),
            source: format!("#metadata((id: \"{id}\", types: (\"task\",))) <weftext>\n{body}"),
            title: Some(id.into()),
            aliases: vec![],
            labels: vec![],
            labels_complete: true,
        }
    }
    fn inventory(nodes: Vec<NodeSnapshot>) -> InventorySnapshot {
        InventorySnapshot {
            generation: "cut".into(),
            coverage: Coverage::Complete,
            nodes,
            resources: vec![],
        }
    }
    #[test]
    fn multiple_typed_sources_symmetric_and_inverse_keep_original_author_fact() {
        let inv = inventory(vec![
            node(
                A,
                &format!("#link(\"{}\")[one]\n#link(\"{}\")[two]", uri(B), uri(B)),
            ),
            node(B, ""),
        ]);
        let property = PropertyReferenceInput {
            source: owner(A),
            field_id: "tasks/dependencies".into(),
            item_path: vec![
                "{\"kind\":\"index\",\"index\":0}".into(),
                "{\"kind\":\"member\",\"key\":\"target\"}".into(),
            ],
            target: uri(B),
        };
        let directed = GraphRelation {
            source: owner(A),
            field_id: property.field_id.clone(),
            path: vec![
                weftext_data::SchemaPath::Each,
                weftext_data::SchemaPath::Member {
                    key: "target".into(),
                },
            ],
            id: "tasks/dependency".into(),
            direction: "directed".into(),
            inverse: Some("dependent".into()),
        };
        let symmetric = GraphRelation {
            id: "sibling".into(),
            direction: "symmetric".into(),
            inverse: None,
            ..directed.clone()
        };
        let graph = relationship_graph(&inv, &[property], &[directed, symmetric], true, 100, 100);
        assert_eq!(graph.edges.len(), 5);
        let inverse = graph.edges.iter().find(|e| e.derived).unwrap();
        assert_eq!(inverse.source, graph_key(&owner(B)));
        assert_eq!(inverse.author_source, graph_key(&owner(A)));
        assert_eq!(inverse.field_id.as_deref(), Some("tasks/dependencies"));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.direction == "symmetric")
                .count(),
            1
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .map(|e| &e.key)
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
    }
    #[test]
    fn independent_relation_and_field_grants_precede_semantic_projection() {
        let source = node(A, "");
        let schema = weftext_data::Schema::bundled().unwrap();
        let fields = BTreeSet::from(["tasks/dependencies".into()]);
        let relations = BTreeSet::from(["tasks/dependency".into()]);
        assert!(
            graph_relations(&source, &schema, Some(&fields), Some(&BTreeSet::new())).is_empty()
        );
        assert!(
            graph_relations(&source, &schema, Some(&BTreeSet::new()), Some(&relations)).is_empty()
        );
        let projected = graph_relations(&source, &schema, Some(&fields), Some(&relations));
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].inverse.as_deref(), Some("dependent"));
    }
    #[test]
    fn budgets_and_coverage_are_distinct_and_unavailable_endpoints_never_become_nodes() {
        let inv = inventory(vec![
            node(
                A,
                &format!("#link(\"{}\")[one]\n#link(\"{}\")[two]", uri(B), uri(B)),
            ),
            node(B, ""),
        ]);
        let short = relationship_graph(&inv, &[], &[], true, 1, 100);
        assert!(short.truncated);
        assert_eq!(short.nodes.len(), 1);
        assert!(short.edges.is_empty());
        assert_eq!(short.coverage, Coverage::Complete);
        let bounded_scan = relationship_graph(&inv, &[], &[], true, 100, 1);
        assert!(bounded_scan.truncated);
        assert_eq!(bounded_scan.coverage, Coverage::Partial);
        let partial = relationship_graph(&inv, &[], &[], false, 100, 100);
        assert_eq!(partial.coverage, Coverage::Partial);
        assert!(!partial.truncated);
        let hidden = inventory(vec![node(
            A,
            &format!("#link(\"{}\")[private target]", uri(B)),
        )]);
        let graph = relationship_graph(&hidden, &[], &[], true, 100, 100);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
    }
}
