use crate::*;
use aho_corasick::AhoCorasick;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use unicode_segmentation::UnicodeSegmentation;
use weftext_engine::RenderNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalFragment {
    pub id: String,
    pub node_key: String,
    pub text: String,
    pub source_range: Option<ExactRange>,
}
/// A leaf is one logical fragment; separate final-tree leaves are never joined
/// on the basis of screen position or source proximity.
pub fn visible_fragments(tree: &RenderNode) -> Vec<LogicalFragment> {
    fn walk(node: &RenderNode, excluded: bool, output: &mut Vec<LogicalFragment>) {
        let excluded = excluded
            || node.kind != "text" && node.kind != "element"
            || node.tag.as_deref().is_some_and(|tag| {
                matches!(
                    tag,
                    "a" | "code" | "pre" | "math" | "script" | "style" | "head"
                )
            })
            || node.attrs.get("class").is_some_and(|class| {
                class
                    .split_whitespace()
                    .any(|c| matches!(c, "typst-math" | "typst-raw"))
            });
        if !excluded
            && node.kind == "text"
            && let Some(text) = &node.text
        {
            let source_range = node
                .origin
                .as_ref()
                .filter(|origin| node.writable && origin.is_main && origin.raw == *text)
                .map(|origin| ExactRange {
                    from: origin.from,
                    to: origin.to,
                    utf16_from: origin.utf16_from,
                    utf16_to: origin.utf16_to,
                    raw: origin.raw.clone(),
                });
            output.push(LogicalFragment {
                id: node.key.clone(),
                node_key: node.key.clone(),
                text: text.clone(),
                source_range,
            });
        }
        if !excluded {
            for child in &node.children {
                walk(child, false, output);
            }
        }
    }
    let mut output = vec![];
    walk(tree, false, &mut output);
    output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MentionHit {
    pub start_scalar: usize,
    pub end_scalar: usize,
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub targets: Vec<NodeRef>,
    pub terms: Vec<String>,
    pub writable: bool,
    pub source_range: Option<ExactRange>,
    pub lane: Option<usize>,
    pub component: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FragmentMatches {
    pub id: String,
    pub node_key: String,
    pub text: String,
    pub coverage: Coverage,
    pub scanned_scalars: usize,
    pub partial_reason: Option<String>,
    pub hits: Vec<MentionHit>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MentionScan {
    pub generation: String,
    pub coverage: Coverage,
    pub fragments: Vec<FragmentMatches>,
}

/// Exact Unicode matching, with every overlapping pattern returned. Input
/// dictionary is restricted to currently disclosable title/alias values.
pub fn scan_mentions(
    inventory: &InventorySnapshot,
    current: &NodeRef,
    fragments: &[LogicalFragment],
    scalar_budget: usize,
) -> Result<MentionScan> {
    scan_mentions_bounded(inventory, current, fragments, scalar_budget, 20_000)
}

/// Counts target pairs, so thousands of equal-title targets cannot multiply an
/// apparently small range budget. A range's targets are retained all together.
pub fn scan_mentions_bounded(
    inventory: &InventorySnapshot,
    current: &NodeRef,
    fragments: &[LogicalFragment],
    scalar_budget: usize,
    candidate_budget: usize,
) -> Result<MentionScan> {
    let mut dictionary: BTreeMap<String, BTreeSet<NodeRef>> = BTreeMap::new();
    for node in &inventory.nodes {
        if &node.owner == current {
            continue;
        }
        for term in node.title.iter().chain(node.aliases.iter()) {
            if !term.is_empty() {
                dictionary
                    .entry(term.clone())
                    .or_default()
                    .insert(node.owner.clone());
            }
        }
    }
    let terms: Vec<_> = dictionary.keys().cloned().collect();
    let automaton = if terms.is_empty() {
        None
    } else {
        Some(
            AhoCorasick::new(&terms)
                .map_err(|e| KnowledgeError::new("matchBudget", e.to_string()))?,
        )
    };
    let mut remaining = scalar_budget;
    let mut candidates_remaining = candidate_budget;
    let mut output_bytes_remaining = 8 * 1024 * 1024usize;
    let mut stopped_reason: Option<String> = None;
    let mut scan = MentionScan {
        generation: inventory.generation.clone(),
        coverage: inventory.coverage,
        fragments: vec![],
    };
    for fragment in fragments {
        let mut scalar_bytes = vec![];
        let mut scalar_utf16 = vec![];
        let mut utf16 = 0;
        let mut chars = fragment.text.char_indices();
        let limit = if stopped_reason.is_some() {
            0
        } else {
            remaining
        };
        for (byte, ch) in chars.by_ref().take(limit) {
            scalar_bytes.push(byte);
            scalar_utf16.push(utf16);
            utf16 += ch.len_utf16();
        }
        let tail = chars.next();
        let end_byte = tail.map(|(byte, _)| byte).unwrap_or(fragment.text.len());
        scalar_bytes.push(end_byte);
        scalar_utf16.push(utf16);
        let mut count = scalar_bytes.len() - 1;
        let mut complete = tail.is_none() && stopped_reason.is_none();
        let mut partial_reason = stopped_reason
            .clone()
            .or_else(|| (!complete).then(|| "scalarBudget".into()));
        let mut grouped: BTreeMap<(usize, usize), (BTreeSet<NodeRef>, BTreeSet<String>)> =
            BTreeMap::new();
        if let Some(automaton) = &automaton {
            for hit in automaton.find_overlapping_iter(&fragment.text[..end_byte]) {
                let Some(start) = scalar_bytes.binary_search(&hit.start()).ok() else {
                    continue;
                };
                let Some(end) = scalar_bytes.binary_search(&hit.end()).ok() else {
                    continue;
                };
                let term = &terms[hit.pattern().as_usize()];
                let targets = &dictionary[term];
                let estimated_bytes = term
                    .len()
                    .saturating_mul(if fragment.source_range.is_some() {
                        2
                    } else {
                        1
                    })
                    .saturating_add(
                        targets
                            .iter()
                            .map(|target| target.workspace_id.len() + target.node_id.len() + 64)
                            .sum::<usize>(),
                    );
                if targets.len() > candidates_remaining || estimated_bytes > output_bytes_remaining
                {
                    count = end;
                    complete = false;
                    let reason = if targets.len() > candidates_remaining {
                        "candidateBudget"
                    } else {
                        "outputBudget"
                    };
                    stopped_reason = Some(reason.into());
                    partial_reason = stopped_reason.clone();
                    break;
                }
                candidates_remaining -= targets.len();
                output_bytes_remaining -= estimated_bytes;
                let group = grouped.entry((start, end)).or_default();
                group.0.extend(targets.iter().cloned());
                group.1.insert(term.clone());
            }
        }
        remaining = remaining.saturating_sub(count);
        if !complete {
            scan.coverage = Coverage::Partial;
        }
        let boundaries: HashSet<_> = if grouped.is_empty() || fragment.source_range.is_none() {
            HashSet::new()
        } else {
            fragment
                .text
                .grapheme_indices(true)
                .map(|(byte, _)| byte)
                .take_while(|byte| *byte <= end_byte)
                .chain(std::iter::once(fragment.text.len()))
                .collect()
        };
        let mut hits: Vec<_> = grouped
            .into_iter()
            .map(|((start, end), (targets, terms))| {
                let from = scalar_bytes[start];
                let to = scalar_bytes[end];
                let start_utf16 = scalar_utf16[start];
                let end_utf16 = scalar_utf16[end];
                let whole_graphemes = boundaries.contains(&from) && boundaries.contains(&to);
                let source_range = fragment
                    .source_range
                    .as_ref()
                    .filter(|range| range.raw == fragment.text && whole_graphemes)
                    .map(|range| ExactRange {
                        from: range.from + from,
                        to: range.from + to,
                        utf16_from: range.utf16_from + start_utf16,
                        utf16_to: range.utf16_from + end_utf16,
                        raw: fragment.text[from..to].into(),
                    });
                MentionHit {
                    start_scalar: start,
                    end_scalar: end,
                    start_utf16,
                    end_utf16,
                    targets: targets.into_iter().collect(),
                    terms: terms.into_iter().collect(),
                    writable: source_range.is_some(),
                    source_range,
                    lane: None,
                    component: 0,
                }
            })
            .collect();
        hits.sort_by_key(|hit| (hit.start_scalar, std::cmp::Reverse(hit.end_scalar)));
        let mut component_end = 0;
        let mut component = 0;
        let mut lane_ends = [0, 0];
        for (index, hit) in hits.iter_mut().enumerate() {
            if index > 0 && hit.start_scalar >= component_end {
                component += 1;
            }
            component_end = component_end.max(hit.end_scalar);
            hit.component = component;
            if let Some(lane) = lane_ends.iter().position(|end| *end <= hit.start_scalar) {
                hit.lane = Some(lane);
                lane_ends[lane] = hit.end_scalar;
            }
        }
        scan.fragments.push(FragmentMatches {
            id: fragment.id.clone(),
            node_key: fragment.node_key.clone(),
            text: fragment.text.clone(),
            coverage: if complete {
                Coverage::Complete
            } else {
                Coverage::Partial
            },
            scanned_scalars: count,
            partial_reason,
            hits,
        });
    }
    Ok(scan)
}

/// Conversion requires an explicit target choice and an exact current leaf map.
pub fn convert_mention(
    source: &NodeSnapshot,
    inventory: &InventorySnapshot,
    hit: &MentionHit,
    selected: &NodeRef,
) -> Result<SourcePatch> {
    if !hit.targets.contains(selected) {
        return Err(KnowledgeError::new(
            "selection",
            "Choose one of this range's explicit targets",
        ));
    }
    let target = inventory
        .nodes
        .iter()
        .find(|node| &node.owner == selected)
        .ok_or_else(|| {
            KnowledgeError::new("unavailable", "The selected target is no longer authorized")
        })?;
    let range = hit
        .source_range
        .clone()
        .filter(|_| hit.writable)
        .ok_or_else(|| {
            KnowledgeError::new(
                "unmapped",
                "This discovery does not have a whole-grapheme unique source writer",
            )
        })?;
    if !ordinary_text_range(&source.source, &range) {
        return Err(KnowledgeError::new(
            "unmapped",
            "The current range is not one exact whole-grapheme ordinary native text leaf",
        ));
    }
    if !target
        .title
        .iter()
        .chain(target.aliases.iter())
        .any(|term| term == &range.raw)
    {
        return Err(KnowledgeError::new(
            "staleDictionary",
            "The exact target term changed; scan again",
        ));
    }
    if !weftext_engine::standard_binding_available(&source.source, "link") {
        return Err(KnowledgeError::new(
            "sourceEntry",
            "The native link binding is not proven",
        ));
    }
    let address = target_address(source, target, None, LinkForm::Id)?;
    let destination = weftext_engine::encode_string(&address)
        .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
    let display = weftext_engine::encode_literal(&range.raw)
        .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
    SourcePatch::new(
        &source.source,
        range,
        format!("#link({destination})[{display}]"),
    )
}
