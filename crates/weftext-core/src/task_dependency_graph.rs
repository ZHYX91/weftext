use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::{NodeId, TaskNodeState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskGraphPolicy {
    ProjectionComplete,
    ProjectionScoped,
    #[allow(dead_code)]
    MutationStrict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskGraphNode {
    pub(crate) node_id: NodeId,
    pub(crate) state: TaskNodeState,
    pub(crate) depends_on: Vec<NodeId>,
    pub(crate) dependency_range: Range<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskGraphTargetClassification {
    Valid,
    Invalid,
    NonTask,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum TaskGraphDiagnosticCode {
    UnresolvedDependency,
    NonTaskDependency,
    InvalidDependencyTarget,
    DependencyCycle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskGraphDiagnostic {
    pub(crate) code: TaskGraphDiagnosticCode,
    pub(crate) source_node_id: NodeId,
    pub(crate) target_node_id: Option<NodeId>,
    pub(crate) range: Range<u64>,
    pub(crate) related_node_ids: Vec<NodeId>,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskGraphResolution {
    pub(crate) valid_node_ids: BTreeSet<NodeId>,
    pub(crate) blocked_node_ids: BTreeSet<NodeId>,
    pub(crate) diagnostics: Vec<TaskGraphDiagnostic>,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_task_dependency_graph(
    nodes: &BTreeMap<NodeId, TaskGraphNode>,
    classifications: &BTreeMap<NodeId, TaskGraphTargetClassification>,
    policy: TaskGraphPolicy,
) -> TaskGraphResolution {
    let mut invalid = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for node in nodes.values() {
        for target_node_id in &node.depends_on {
            if nodes.contains_key(target_node_id) {
                continue;
            }
            match classifications.get(target_node_id) {
                None | Some(TaskGraphTargetClassification::Valid) => {
                    let unavailable_is_invalid = policy != TaskGraphPolicy::ProjectionScoped;
                    if unavailable_is_invalid {
                        invalid.insert(node.node_id);
                    }
                    let disclose_target = policy == TaskGraphPolicy::ProjectionComplete;
                    diagnostics.push(dependency_diagnostic(
                        TaskGraphDiagnosticCode::UnresolvedDependency,
                        unavailable_message(policy),
                        node,
                        disclose_target.then_some(*target_node_id),
                        Vec::new(),
                    ));
                }
                Some(TaskGraphTargetClassification::NonTask) => {
                    invalid.insert(node.node_id);
                    diagnostics.push(dependency_diagnostic(
                        TaskGraphDiagnosticCode::NonTaskDependency,
                        "dependency resolves to a visible managed node without a task profile",
                        node,
                        Some(*target_node_id),
                        vec![*target_node_id],
                    ));
                }
                Some(TaskGraphTargetClassification::Invalid) => {
                    invalid.insert(node.node_id);
                    diagnostics.push(dependency_diagnostic(
                        TaskGraphDiagnosticCode::InvalidDependencyTarget,
                        "dependency resolves to a visible invalid task profile",
                        node,
                        Some(*target_node_id),
                        vec![*target_node_id],
                    ));
                }
            }
        }
    }
    propagate_invalid_targets(nodes, &mut invalid, &mut diagnostics);

    let graph = nodes
        .iter()
        .filter(|(node_id, _)| !invalid.contains(node_id))
        .map(|(node_id, node)| {
            let dependencies = node
                .depends_on
                .iter()
                .filter(|target_node_id| {
                    nodes.contains_key(target_node_id) && !invalid.contains(target_node_id)
                })
                .copied()
                .collect::<Vec<_>>();
            (*node_id, dependencies)
        })
        .collect::<BTreeMap<_, _>>();
    for mut component in strongly_connected_components(&graph) {
        component.sort_unstable();
        let cyclic = component.len() > 1
            || component.first().is_some_and(|node_id| {
                graph
                    .get(node_id)
                    .is_some_and(|dependencies| dependencies.contains(node_id))
            });
        if !cyclic {
            continue;
        }
        for node_id in &component {
            invalid.insert(*node_id);
            let node = &nodes[node_id];
            diagnostics.push(TaskGraphDiagnostic {
                code: TaskGraphDiagnosticCode::DependencyCycle,
                source_node_id: *node_id,
                target_node_id: None,
                range: node.dependency_range.clone(),
                related_node_ids: component.clone(),
                message: "task-node dependency graph contains a cycle".to_owned(),
            });
        }
    }
    propagate_invalid_targets(nodes, &mut invalid, &mut diagnostics);

    let valid_node_ids = nodes
        .keys()
        .filter(|node_id| !invalid.contains(node_id))
        .copied()
        .collect::<BTreeSet<_>>();
    let blocked_node_ids = nodes
        .values()
        .filter(|node| valid_node_ids.contains(&node.node_id))
        .filter(|node| {
            node.depends_on.iter().any(|target_node_id| {
                valid_node_ids.contains(target_node_id)
                    && nodes
                        .get(target_node_id)
                        .is_some_and(|target| !target.state.is_closed())
            })
        })
        .map(|node| node.node_id)
        .collect();

    diagnostics.sort_by(compare_diagnostics);
    diagnostics.dedup();
    TaskGraphResolution {
        valid_node_ids,
        blocked_node_ids,
        diagnostics,
    }
}

fn compare_diagnostics(
    left: &TaskGraphDiagnostic,
    right: &TaskGraphDiagnostic,
) -> std::cmp::Ordering {
    left.source_node_id
        .cmp(&right.source_node_id)
        .then_with(|| left.range.start.cmp(&right.range.start))
        .then_with(|| left.range.end.cmp(&right.range.end))
        .then_with(|| left.code.cmp(&right.code))
        .then_with(|| left.target_node_id.cmp(&right.target_node_id))
        .then_with(|| left.related_node_ids.cmp(&right.related_node_ids))
        .then_with(|| left.message.cmp(&right.message))
}

const fn unavailable_message(policy: TaskGraphPolicy) -> &'static str {
    match policy {
        TaskGraphPolicy::ProjectionComplete => "dependency does not resolve to an active task node",
        TaskGraphPolicy::ProjectionScoped => {
            "dependency is unavailable in this authorized projection"
        }
        TaskGraphPolicy::MutationStrict => "dependency is unavailable to this authorized mutation",
    }
}

fn propagate_invalid_targets(
    nodes: &BTreeMap<NodeId, TaskGraphNode>,
    invalid: &mut BTreeSet<NodeId>,
    diagnostics: &mut Vec<TaskGraphDiagnostic>,
) {
    loop {
        let newly_invalid = nodes
            .values()
            .filter(|node| !invalid.contains(&node.node_id))
            .filter_map(|node| {
                let dependencies = node
                    .depends_on
                    .iter()
                    .filter(|target_node_id| invalid.contains(target_node_id))
                    .copied()
                    .collect::<BTreeSet<_>>();
                (!dependencies.is_empty()).then_some((node.node_id, dependencies))
            })
            .collect::<Vec<_>>();
        if newly_invalid.is_empty() {
            break;
        }
        for (node_id, dependencies) in newly_invalid {
            if !invalid.insert(node_id) {
                continue;
            }
            let node = &nodes[&node_id];
            for target_node_id in dependencies {
                diagnostics.push(dependency_diagnostic(
                    TaskGraphDiagnosticCode::InvalidDependencyTarget,
                    "dependency resolves to a task node invalidated by its dependency graph",
                    node,
                    Some(target_node_id),
                    vec![target_node_id],
                ));
            }
        }
    }
}

fn dependency_diagnostic(
    code: TaskGraphDiagnosticCode,
    message: &str,
    node: &TaskGraphNode,
    target_node_id: Option<NodeId>,
    related_node_ids: Vec<NodeId>,
) -> TaskGraphDiagnostic {
    TaskGraphDiagnostic {
        code,
        source_node_id: node.node_id,
        target_node_id,
        range: node.dependency_range.clone(),
        related_node_ids: related_node_ids
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        message: message.to_owned(),
    }
}

fn strongly_connected_components(graph: &BTreeMap<NodeId, Vec<NodeId>>) -> Vec<Vec<NodeId>> {
    let mut visited = BTreeSet::new();
    let mut finish_order = Vec::new();
    for start in graph.keys().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next_index)) = stack.last_mut() {
            let neighbors = graph.get(node).map_or(&[][..], Vec::as_slice);
            if *next_index < neighbors.len() {
                let next = neighbors[*next_index];
                *next_index += 1;
                if visited.insert(next) {
                    stack.push((next, 0));
                }
            } else {
                finish_order.push(*node);
                stack.pop();
            }
        }
    }

    let mut reverse = graph
        .keys()
        .copied()
        .map(|node_id| (node_id, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (node_id, dependencies) in graph {
        for target_node_id in dependencies {
            reverse.entry(*target_node_id).or_default().push(*node_id);
        }
    }
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for start in finish_order.into_iter().rev() {
        if !assigned.insert(start) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.push(node);
            for next in reverse.get(&node).map_or(&[][..], Vec::as_slice) {
                if assigned.insert(*next) {
                    stack.push(*next);
                }
            }
        }
        components.push(component);
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(last: u32) -> NodeId {
        format!("10000000-0000-4000-8000-{last:012}")
            .parse()
            .expect("node ID")
    }

    fn node(node_id: NodeId, state: TaskNodeState, depends_on: &[NodeId]) -> TaskGraphNode {
        TaskGraphNode {
            node_id,
            state,
            depends_on: depends_on.to_vec(),
            dependency_range: u64::from(node_id.as_uuid().as_bytes()[15])..99,
        }
    }

    fn valid_classifications(
        nodes: &BTreeMap<NodeId, TaskGraphNode>,
    ) -> BTreeMap<NodeId, TaskGraphTargetClassification> {
        nodes
            .keys()
            .copied()
            .map(|node_id| (node_id, TaskGraphTargetClassification::Valid))
            .collect()
    }

    #[test]
    fn unavailable_target_is_retained_only_by_projection_scoped_policy() {
        let source = id(1);
        let unavailable = id(9);
        let nodes = BTreeMap::from([(source, node(source, TaskNodeState::Todo, &[unavailable]))]);
        let classifications = valid_classifications(&nodes);

        let complete = resolve_task_dependency_graph(
            &nodes,
            &classifications,
            TaskGraphPolicy::ProjectionComplete,
        );
        let scoped = resolve_task_dependency_graph(
            &nodes,
            &classifications,
            TaskGraphPolicy::ProjectionScoped,
        );
        let strict = resolve_task_dependency_graph(
            &nodes,
            &classifications,
            TaskGraphPolicy::MutationStrict,
        );

        assert!(!complete.valid_node_ids.contains(&source));
        assert!(scoped.valid_node_ids.contains(&source));
        assert!(!strict.valid_node_ids.contains(&source));
        assert_eq!(complete.diagnostics[0].target_node_id, Some(unavailable));
        for resolution in [&scoped, &strict] {
            assert_eq!(resolution.diagnostics[0].target_node_id, None);
            assert!(resolution.diagnostics[0].related_node_ids.is_empty());
        }
    }

    #[test]
    fn visible_invalid_and_non_task_targets_reverse_invalidate_dependents() {
        let invalid_target = id(1);
        let direct = id(2);
        let transitive = id(3);
        let non_task = id(4);
        let other = id(5);
        let nodes = BTreeMap::from([
            (direct, node(direct, TaskNodeState::Todo, &[invalid_target])),
            (transitive, node(transitive, TaskNodeState::Todo, &[direct])),
            (other, node(other, TaskNodeState::Todo, &[non_task])),
        ]);
        let classifications = BTreeMap::from([
            (invalid_target, TaskGraphTargetClassification::Invalid),
            (direct, TaskGraphTargetClassification::Valid),
            (transitive, TaskGraphTargetClassification::Valid),
            (non_task, TaskGraphTargetClassification::NonTask),
            (other, TaskGraphTargetClassification::Valid),
        ]);
        for policy in [
            TaskGraphPolicy::ProjectionComplete,
            TaskGraphPolicy::ProjectionScoped,
            TaskGraphPolicy::MutationStrict,
        ] {
            let resolution = resolve_task_dependency_graph(&nodes, &classifications, policy);
            assert!(resolution.valid_node_ids.is_empty());
            assert!(resolution.diagnostics.iter().any(|diagnostic| {
                diagnostic.source_node_id == transitive
                    && diagnostic.code == TaskGraphDiagnosticCode::InvalidDependencyTarget
                    && diagnostic.target_node_id == Some(direct)
            }));
            assert!(resolution.diagnostics.iter().any(|diagnostic| {
                diagnostic.source_node_id == other
                    && diagnostic.code == TaskGraphDiagnosticCode::NonTaskDependency
            }));
        }
    }

    #[test]
    fn self_direct_and_long_cycles_are_sorted_and_reverse_invalidated() {
        let self_cycle = id(1);
        let first = id(2);
        let second = id(3);
        let third = id(4);
        let dependent = id(5);
        let direct_first = id(6);
        let direct_second = id(7);
        let nodes = BTreeMap::from([
            (
                self_cycle,
                node(self_cycle, TaskNodeState::Todo, &[self_cycle]),
            ),
            (first, node(first, TaskNodeState::Todo, &[second])),
            (second, node(second, TaskNodeState::Todo, &[third])),
            (third, node(third, TaskNodeState::Todo, &[first])),
            (dependent, node(dependent, TaskNodeState::Todo, &[third])),
            (
                direct_first,
                node(direct_first, TaskNodeState::Todo, &[direct_second]),
            ),
            (
                direct_second,
                node(direct_second, TaskNodeState::Todo, &[direct_first]),
            ),
        ]);
        let resolution = resolve_task_dependency_graph(
            &nodes,
            &valid_classifications(&nodes),
            TaskGraphPolicy::ProjectionComplete,
        );
        for policy in [
            TaskGraphPolicy::ProjectionScoped,
            TaskGraphPolicy::MutationStrict,
        ] {
            assert_eq!(
                resolution,
                resolve_task_dependency_graph(&nodes, &valid_classifications(&nodes), policy)
            );
        }
        assert!(resolution.valid_node_ids.is_empty());
        let cycle_diagnostics = resolution
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == TaskGraphDiagnosticCode::DependencyCycle)
            .collect::<Vec<_>>();
        assert_eq!(cycle_diagnostics.len(), 6);
        assert!(cycle_diagnostics.iter().any(|diagnostic| {
            diagnostic.source_node_id == self_cycle
                && diagnostic.related_node_ids == vec![self_cycle]
        }));
        for node_id in [first, second, third] {
            assert!(cycle_diagnostics.iter().any(|diagnostic| {
                diagnostic.source_node_id == node_id
                    && diagnostic.related_node_ids == vec![first, second, third]
            }));
        }
        for node_id in [direct_first, direct_second] {
            assert!(cycle_diagnostics.iter().any(|diagnostic| {
                diagnostic.source_node_id == node_id
                    && diagnostic.related_node_ids == vec![direct_first, direct_second]
            }));
        }
        assert!(resolution.diagnostics.windows(2).all(|pair| {
            compare_diagnostics(&pair[0], &pair[1]) != std::cmp::Ordering::Greater
        }));
        assert!(resolution.diagnostics.iter().any(|diagnostic| {
            diagnostic.source_node_id == dependent
                && diagnostic.code == TaskGraphDiagnosticCode::InvalidDependencyTarget
        }));
        let mut permuted = nodes;
        for node in permuted.values_mut() {
            node.depends_on.reverse();
        }
        assert_eq!(
            resolution,
            resolve_task_dependency_graph(
                &permuted,
                &valid_classifications(&permuted),
                TaskGraphPolicy::ProjectionComplete,
            )
        );
    }

    #[test]
    fn blocked_depends_only_on_valid_open_visible_targets() {
        let open = id(1);
        let closed = id(2);
        let blocked = id(3);
        let unblocked = id(4);
        let unavailable_source = id(5);
        let unavailable = id(9);
        let nodes = BTreeMap::from([
            (open, node(open, TaskNodeState::InProgress, &[])),
            (closed, node(closed, TaskNodeState::Completed, &[])),
            (blocked, node(blocked, TaskNodeState::Todo, &[open, closed])),
            (unblocked, node(unblocked, TaskNodeState::Todo, &[closed])),
            (
                unavailable_source,
                node(unavailable_source, TaskNodeState::Todo, &[unavailable]),
            ),
        ]);
        let resolution = resolve_task_dependency_graph(
            &nodes,
            &valid_classifications(&nodes),
            TaskGraphPolicy::ProjectionScoped,
        );
        assert_eq!(resolution.blocked_node_ids, BTreeSet::from([blocked]));
        assert!(resolution.valid_node_ids.contains(&unavailable_source));
    }
}
