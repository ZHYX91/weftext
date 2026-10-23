use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationTarget {
    pub entity_key: String,
    pub types: Vec<String>,
    pub lifecycle: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSource {
    pub entity_key: String,
    pub source: String,
    pub lifecycle: String,
}
/// Check explicitly installed acyclic rules over one complete authorized source
/// closure. Default relations allow cycles; this adds no undeclared business rule.
pub fn validate_relation_graph(
    owner: &str,
    before: &str,
    after: &str,
    schema: &Schema,
    sources: &[RelationSource],
    coverage_complete: bool,
) -> Validation {
    let mut out = Validation::valid();
    let proposed = project(after, schema);
    let original = project(before, schema);
    if !scalar_valid("node_ref", &NativeValue::Text(owner.into()))
        || proposed.id.as_deref() != owner.rsplit('/').next()
    {
        out.unavailable
            .push(weftext_messages::UiMessage::error_data_relations_exact_author_owner_qualification_unavailable().into());
        return out;
    }
    let constrained = schema
        .relations
        .iter()
        .filter(|r| {
            r.cycle_policy == "reject"
                && proposed.resolved_types.contains(&r.source_type)
                && relation_changed(r, &original, &proposed)
        })
        .collect::<Vec<_>>();
    if constrained.is_empty() {
        return out;
    }
    let mut keys = BTreeSet::new();
    let ambiguous = sources.iter().any(|s| !keys.insert(&s.entity_key));
    for relation in constrained {
        let refs = |p: &NoteProjection| -> std::result::Result<BTreeSet<String>, ErrorDescription> {
            let Some(f) = p.fields.iter().find(|f| f.field_id == relation.field) else {
                return Ok(BTreeSet::new());
            };
            match &f.cell {
                Cell::Absent | Cell::Null => Ok(BTreeSet::new()),
                Cell::Value { value } => {
                    let mut values = vec![];
                    extract(value, &relation.target_path, &mut values);
                    Ok(values
                        .into_iter()
                        .filter_map(NativeValue::text)
                        .map(str::to_owned)
                        .collect())
                }
                _ => Err(weftext_messages::UiMessage::error_data_relations_graph_source_assertion_unavailable().into()),
            }
        };
        let after_refs = match refs(&proposed) {
            Ok(v) => v,
            Err(e) => {
                out.unavailable.push(e);
                continue;
            }
        };
        let before_refs = refs(&original).unwrap_or_default();
        let new = after_refs
            .difference(&before_refs)
            .cloned()
            .collect::<Vec<_>>();
        if new.is_empty() {
            continue;
        }
        if !coverage_complete || sources.len() > 100_000 || ambiguous {
            out.unavailable
                .push(weftext_messages::UiMessage::error_data_relations_complete_authorized_graph_constraint_scope_unavailable().into());
            continue;
        }
        let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut readable = true;
        let mut edges = 0usize;
        for source in sources.iter().filter(|s| s.lifecycle == "live") {
            if source.entity_key == owner {
                continue;
            }
            let p = project(&source.source, schema);
            if !scalar_valid("node_ref", &NativeValue::Text(source.entity_key.clone()))
                || p.qualification != "qualified"
                || p.id.as_deref() != source.entity_key.rsplit('/').next()
                || !p.diagnostics.is_empty()
            {
                readable = false;
                break;
            }
            if p.resolved_types.contains(&relation.source_type) {
                match refs(&p) {
                    Ok(targets) => {
                        edges += targets.len();
                        graph.insert(source.entity_key.clone(), targets);
                    }
                    Err(_) => {
                        readable = false;
                        break;
                    }
                }
            }
        }
        if !readable || edges > 1_000_000 {
            out.unavailable
                .push(weftext_messages::UiMessage::error_data_relations_graph_constraint_source_closure_unavailable_or_over_budget().into());
            continue;
        }
        if relation.direction == "symmetric" {
            for (from, targets) in graph.clone() {
                for target in targets {
                    graph.entry(target).or_default().insert(from.clone());
                }
            }
            for target in before_refs {
                graph
                    .entry(owner.into())
                    .or_default()
                    .insert(target.clone());
                graph.entry(target).or_default().insert(owner.into());
            }
        } else {
            graph.insert(owner.into(), after_refs);
        }
        for target in new {
            if relation.direction == "symmetric"
                && graph
                    .get(owner)
                    .is_some_and(|targets| targets.contains(&target))
            {
                // Independently authored symmetric assertions of one pair are
                // one graph edge; a reciprocal fact is not a two-edge cycle.
                continue;
            }
            let mut visited = BTreeSet::new();
            let mut todo = vec![target.clone()];
            let mut cycle = false;
            while let Some(node) = todo.pop() {
                if node == owner {
                    cycle = true;
                    break;
                }
                if visited.insert(node.clone())
                    && let Some(next) = graph.get(&node)
                {
                    todo.extend(next.iter().cloned());
                }
            }
            if cycle {
                out.diagnostics.push(weftext_messages::UiMessage::error_data_relations_proposed_edge_creates_configured_cycle(weftext_messages::params::ErrorDataRelationsProposedEdgeCreatesConfiguredCycle {relation_id: (relation.id).to_string()}).into());
            }
            if relation.direction == "symmetric" {
                graph
                    .entry(owner.into())
                    .or_default()
                    .insert(target.clone());
                graph.entry(target).or_default().insert(owner.into());
            }
        }
    }
    out
}
/// Validate new/ref-target changes against an authorized frozen target snapshot.
/// Existing dangling assertions are retained; new ones require live exact IDs.
/// Core must freeze these consumed endpoint revisions as prepare dependencies.
pub fn validate_relation_change(
    before: &str,
    after: &str,
    schema: &Schema,
    targets: &[RelationTarget],
    coverage_complete: bool,
) -> Validation {
    validate_relation_change_inner(
        None,
        before,
        after,
        schema,
        targets,
        coverage_complete,
        true,
    )
}
pub fn validate_relation_change_for_owner(
    owner: &str,
    before: &str,
    after: &str,
    schema: &Schema,
    targets: &[RelationTarget],
    coverage_complete: bool,
) -> Validation {
    validate_relation_change_inner(
        Some(owner),
        before,
        after,
        schema,
        targets,
        coverage_complete,
        true,
    )
}
pub fn validate_relation_change_with_graph(
    owner: &str,
    before: &str,
    after: &str,
    schema: &Schema,
    targets: &[RelationTarget],
    sources: &[RelationSource],
    coverage_complete: bool,
) -> Validation {
    let mut out = validate_relation_change_inner(
        Some(owner),
        before,
        after,
        schema,
        targets,
        coverage_complete,
        false,
    );
    let graph = validate_relation_graph(owner, before, after, schema, sources, coverage_complete);
    out.diagnostics.extend(graph.diagnostics);
    out.unavailable.extend(graph.unavailable);
    out
}
fn validate_relation_change_inner(
    owner: Option<&str>,
    before: &str,
    after: &str,
    schema: &Schema,
    targets: &[RelationTarget],
    coverage_complete: bool,
    enforce_graph: bool,
) -> Validation {
    let mut out = Validation::valid();
    let proposed = project(after, schema);
    let original = project(before, schema);
    if proposed.qualification != "qualified" {
        out.unavailable.push(
            weftext_messages::UiMessage::error_data_relations_proposed_control_projection_unavailable()
                .into(),
        );
        return out;
    }
    let mut target_keys = BTreeSet::new();
    if targets.iter().any(|t| !target_keys.insert(&t.entity_key)) {
        out.unavailable.push(
            weftext_messages::UiMessage::error_data_relations_target_inventory_ambiguous().into(),
        );
        return out;
    }
    for relation in schema.relations.iter().filter(|r| {
        proposed.resolved_types.contains(&r.source_type)
            && relation_changed(r, &original, &proposed)
    }) {
        let Some(field) = proposed
            .fields
            .iter()
            .find(|f| f.field_id == relation.field)
        else {
            continue;
        };
        let value = match &field.cell {
            Cell::Absent | Cell::Null => continue,
            Cell::Value { value } => value,
            Cell::Invalid { .. } => {
                out.diagnostics.push(
                    weftext_messages::UiMessage::error_data_relations_invalid_author_field(
                        weftext_messages::params::ErrorDataRelationsInvalidAuthorField {
                            relation_id: (relation.id).to_string(),
                        },
                    )
                    .into(),
                );
                continue;
            }
            Cell::Unavailable { .. } => {
                out.unavailable.push(
                    weftext_messages::UiMessage::error_data_relations_author_field_unavailable(
                        weftext_messages::params::ErrorDataRelationsAuthorFieldUnavailable {
                            relation_id: (relation.id).to_string(),
                        },
                    )
                    .into(),
                );
                continue;
            }
        };
        let mut assertions = vec![];
        extract(value, &relation.target_path, &mut assertions);
        if relation
            .max_assertions
            .is_some_and(|max| assertions.len() > max)
        {
            out.diagnostics.push(
                weftext_messages::UiMessage::error_data_relations_source_assertion_cardinality_exceeded(weftext_messages::params::ErrorDataRelationsSourceAssertionCardinalityExceeded {
                    relation_id: (relation.id).to_string(),
                })
                .into(),
            );
        }
        let before_value = original
            .fields
            .iter()
            .find(|f| f.field_id == relation.field)
            .and_then(|f| f.cell.native());
        let mut before_assertions = vec![];
        if let Some(v) = before_value {
            extract(v, &relation.target_path, &mut before_assertions);
        }
        for assertion in assertions {
            if assertion.record().is_some() && assertion.member("text").is_some() {
                continue;
            }
            let Some(target) = assertion.text() else {
                out.diagnostics.push(
                    weftext_messages::UiMessage::error_data_relations_relation_target_not_ref_string(weftext_messages::params::ErrorDataRelationsRelationTargetNotRefString {
                        relation_id: (relation.id).to_string(),
                    })
                    .into(),
                );
                continue;
            };
            if relation.reject_self
                && owner.map_or_else(
                    || {
                        proposed
                            .id
                            .as_ref()
                            .is_some_and(|id| target.rsplit('/').next() == Some(id))
                    },
                    |owner| target == owner,
                )
            {
                out.diagnostics.push(
                    weftext_messages::UiMessage::error_data_relations_self_relation_forbidden(
                        weftext_messages::params::ErrorDataRelationsSelfRelationForbidden {
                            relation_id: (relation.id).to_string(),
                        },
                    )
                    .into(),
                );
            }
            let unchanged = before_assertions.iter().any(|v| v.text() == Some(target))
                && original.resolved_types.contains(&relation.source_type);
            let current = targets.iter().find(|t| t.entity_key == target);
            if current.is_none_or(|t| t.lifecycle != "live") {
                if !unchanged {
                    if coverage_complete {
                        out.diagnostics.push(weftext_messages::UiMessage::error_data_relations_new_target_requires_exact_live_admitted_node(weftext_messages::params::ErrorDataRelationsNewTargetRequiresExactLiveAdmittedNode {relation_id: (relation.id).to_string()}).into());
                    } else {
                        out.unavailable
                            .push(weftext_messages::UiMessage::error_data_relations_target_qualification_unavailable(weftext_messages::params::ErrorDataRelationsTargetQualificationUnavailable {relation_id: (relation.id).to_string()}).into());
                    }
                }
                continue;
            }
            let current = current.unwrap();
            let (target_types, check) = schema.expand_types(&current.types);
            if !check.is_valid() {
                out.unavailable.push(
                    weftext_messages::UiMessage::error_data_relations_target_type_closure_unavailable(weftext_messages::params::ErrorDataRelationsTargetTypeClosureUnavailable {
                        relation_id: (relation.id).to_string(),
                    })
                    .into(),
                );
                continue;
            }
            if relation.target_types["kind"].as_str() == Some("any_of")
                && !relation.target_types["ids"].as_array().is_some_and(|ids| {
                    ids.iter()
                        .any(|id| id.as_str().is_some_and(|id| target_types.contains(id)))
                })
            {
                out.diagnostics.push(
                    weftext_messages::UiMessage::error_data_relations_target_type_incompatible(
                        weftext_messages::params::ErrorDataRelationsTargetTypeIncompatible {
                            relation_id: (relation.id).to_string(),
                        },
                    )
                    .into(),
                );
            }
            if enforce_graph && relation.cycle_policy != "allow" && !unchanged {
                out.unavailable.push(weftext_messages::UiMessage::error_data_relations_cycle_admission_requires_complete_authorized_graph(weftext_messages::params::ErrorDataRelationsCycleAdmissionRequiresCompleteAuthorizedGraph {relation_id: (relation.id).to_string()}).into());
            }
        }
    }
    out
}
fn relation_changed(relation: &Relation, before: &NoteProjection, after: &NoteProjection) -> bool {
    if !before.resolved_types.contains(&relation.source_type) {
        return true;
    }
    let old = before.fields.iter().find(|f| f.field_id == relation.field);
    let new = after.fields.iter().find(|f| f.field_id == relation.field);
    match (old, new) {
        (Some(old), Some(new)) => old.cell != new.cell || old.raw != new.raw,
        (None, None) => false,
        _ => true,
    }
}
fn extract<'a>(v: &'a NativeValue, path: &[SchemaPath], out: &mut Vec<&'a NativeValue>) {
    if path.is_empty() {
        out.push(v);
        return;
    }
    match &path[0] {
        SchemaPath::Each => {
            if let Some(a) = v.array() {
                for item in a {
                    extract(item, &path[1..], out);
                }
            }
        }
        SchemaPath::Member { key } => {
            if let Some(v) = v.member(key) {
                extract(v, &path[1..], out);
            } else if key == "ref" && v.member("text").is_some() {
                // Literal branches count as authored assertions for cardinality but
                // have no endpoint lookup, inverse or automatic target creation.
                out.push(v);
            }
        }
    }
}
