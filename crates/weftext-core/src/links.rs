use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;

use serde::Serialize;

use crate::frontmatter::parse_node_aliases;
use crate::{
    AdjacentHeadingBody, DocumentError, DocumentLinkKind, DocumentSourceOccurrences, NodeId,
    WorkspaceRevisionError, analyze_document, read_node_document, read_workspace_revision,
    scan_workspace, strip_optional_canonical_extension,
};

pub type InternalLinkKind = DocumentLinkKind;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkMatchQuality {
    ExactCanonical,
    ExactAlias,
    NormalizedCanonical,
    NormalizedAlias,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLinkEntry {
    pub id: NodeId,
    pub name: String,
    pub locator: String,
    pub aliases: Vec<String>,
    pub trashed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingLink {
    pub source_node_id: NodeId,
    pub kind: InternalLinkKind,
    pub start: u64,
    pub end: u64,
    pub locator_start: u64,
    pub locator_end: u64,
    pub authored_locator: String,
    pub fragment: Option<String>,
    pub display_text: Option<String>,
    pub target_node_ids: Vec<NodeId>,
    pub canonical_locator: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PotentialMention {
    pub source_node_id: NodeId,
    pub start: u64,
    pub end: u64,
    pub matched_text: String,
    pub matched_scalar_length: u64,
    pub quality: LinkMatchQuality,
    pub target_node_ids: Vec<NodeId>,
    pub primary: bool,
    pub contained_by_start: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backlink {
    pub target_node_id: NodeId,
    pub source_node_id: NodeId,
    pub start: u64,
    pub end: u64,
    pub kind: InternalLinkKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLinkIndex {
    pub revision: crate::WorkspaceRevision,
    pub nodes: Vec<NodeLinkEntry>,
    pub outgoing: Vec<OutgoingLink>,
    pub backlinks: Vec<Backlink>,
    pub potential_mentions: Vec<PotentialMention>,
}

/// Rebuilds the derived Stage 1B link, alias, backlink, and mention evidence.
///
/// # Errors
///
/// Returns an error for invalid workspace authority, unreadable documents, or
/// unsupported top-level alias syntax needed by the index.
pub fn build_workspace_link_index(
    root: impl AsRef<Path>,
) -> Result<WorkspaceLinkIndex, LinkIndexError> {
    let root = root.as_ref();
    let revision = read_workspace_revision(root).map_err(LinkIndexError::WorkspaceRevision)?;
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err(LinkIndexError::InvalidInventory);
    }
    let root_setting = inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.metadata)
        .map_or(AdjacentHeadingBody::Separate, |metadata| {
            metadata.presentation.adjacent_heading_body
        });

    let (sources, nodes) = load_link_sources(root, &inventory)?;

    let resolver = LinkResolver::new(&nodes);
    let mut outgoing = Vec::new();
    let mut occurrences_by_node = BTreeMap::<NodeId, DocumentSourceOccurrences>::new();
    for node in nodes.iter().filter(|node| !node.trashed) {
        let source = sources
            .get(&node.id)
            .ok_or(LinkIndexError::InvalidInventory)?;
        let occurrences = analyze_document(source, root_setting).occurrences;
        for occurrence in &occurrences.links {
            let targets = resolver.resolve(node.id, &occurrence.authored_locator);
            let canonical_locator = (targets.len() == 1)
                .then(|| resolver.canonical_locator(targets[0]))
                .flatten();
            outgoing.push(OutgoingLink {
                source_node_id: node.id,
                kind: occurrence.kind,
                start: occurrence.start,
                end: occurrence.end,
                locator_start: occurrence.locator_start,
                locator_end: occurrence.locator_end,
                authored_locator: occurrence.authored_locator.clone(),
                fragment: occurrence.fragment.clone(),
                display_text: occurrence.display_text.clone(),
                target_node_ids: targets,
                canonical_locator,
            });
        }
        occurrences_by_node.insert(node.id, occurrences);
    }
    outgoing.sort_by_key(|link| (link.source_node_id, link.start));
    let mut backlinks = outgoing
        .iter()
        .flat_map(|link| {
            link.target_node_ids.iter().map(|target_node_id| Backlink {
                target_node_id: *target_node_id,
                source_node_id: link.source_node_id,
                start: link.start,
                end: link.end,
                kind: link.kind,
            })
        })
        .collect::<Vec<_>>();
    backlinks.sort_by_key(|link| (link.target_node_id, link.source_node_id, link.start));

    let mut potential_mentions = Vec::new();
    for node in nodes.iter().filter(|node| !node.trashed) {
        let source = sources
            .get(&node.id)
            .ok_or(LinkIndexError::InvalidInventory)?;
        let occurrences = occurrences_by_node
            .get(&node.id)
            .ok_or(LinkIndexError::InvalidInventory)?;
        let mut candidates = collect_mentions(node.id, source, occurrences, &nodes);
        mark_contained_candidates(&mut candidates);
        potential_mentions.extend(candidates);
    }
    potential_mentions.sort_by(compare_mentions);

    Ok(WorkspaceLinkIndex {
        revision,
        nodes,
        outgoing,
        backlinks,
        potential_mentions,
    })
}

fn load_link_sources(
    root: &Path,
    inventory: &crate::WorkspaceInventory,
) -> Result<(BTreeMap<NodeId, String>, Vec<NodeLinkEntry>), LinkIndexError> {
    let mut sources = BTreeMap::<NodeId, String>::new();
    let mut nodes = Vec::new();
    for node in &inventory.nodes {
        if crate::workspace_trash::is_trash_storage_path(root, &node.path) {
            continue;
        }
        let id = node.id.ok_or(LinkIndexError::InvalidInventory)?;
        let snapshot = read_node_document(&node.path).map_err(LinkIndexError::Document)?;
        let aliases = parse_canonical_aliases(&snapshot.source)?;
        let locator = node_locator(root, &node.path, &node.name)?;
        sources.insert(id, snapshot.source);
        nodes.push(NodeLinkEntry {
            id,
            name: node.name.clone(),
            locator,
            aliases,
            trashed: false,
        });
    }
    nodes.sort_by(|left, right| {
        natural_key(&left.locator)
            .cmp(&natural_key(&right.locator))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok((sources, nodes))
}

struct LinkResolver<'a> {
    nodes: &'a [NodeLinkEntry],
    exact_locator: HashMap<&'a str, NodeId>,
}

impl<'a> LinkResolver<'a> {
    fn new(nodes: &'a [NodeLinkEntry]) -> Self {
        Self {
            nodes,
            exact_locator: nodes
                .iter()
                .map(|node| (node.locator.as_str(), node.id))
                .collect(),
        }
    }

    fn resolve(&self, source: NodeId, authored: &str) -> Vec<NodeId> {
        let authored = strip_optional_canonical_extension(authored);
        if authored.is_empty() {
            return vec![source];
        }
        if let Ok(id) = NodeId::from_str(authored) {
            return self
                .nodes
                .iter()
                .any(|node| node.id == id)
                .then_some(id)
                .into_iter()
                .collect();
        }
        if authored.contains('/') {
            return self
                .exact_locator
                .get(authored)
                .copied()
                .into_iter()
                .collect();
        }
        let exact = self
            .nodes
            .iter()
            .filter(|node| {
                node.name == authored || node.aliases.iter().any(|alias| alias == authored)
            })
            .map(|node| node.id)
            .collect::<Vec<_>>();
        if !exact.is_empty() {
            return sorted_unique(exact);
        }
        let normalized = normalize(authored);
        sorted_unique(
            self.nodes
                .iter()
                .filter(|node| {
                    normalize(&node.name) == normalized
                        || node
                            .aliases
                            .iter()
                            .any(|alias| normalize(alias) == normalized)
                })
                .map(|node| node.id)
                .collect(),
        )
    }

    fn canonical_locator(&self, id: NodeId) -> Option<String> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.locator.clone())
    }
}

fn collect_mentions(
    source_node_id: NodeId,
    source: &str,
    occurrences: &DocumentSourceOccurrences,
    nodes: &[NodeLinkEntry],
) -> Vec<PotentialMention> {
    let mut grouped = BTreeMap::<(usize, usize, String), MentionBuilder>::new();
    let exclusions = occurrences
        .protected_ranges
        .iter()
        .map(|range| {
            usize::try_from(range.start).unwrap_or(source.len())
                ..usize::try_from(range.end).unwrap_or(source.len())
        })
        .collect::<Vec<_>>();
    for eligible in &occurrences.eligible_text_ranges {
        let block_start = usize::try_from(eligible.start).unwrap_or(source.len());
        let block_end = usize::try_from(eligible.end).unwrap_or(source.len());
        for node in nodes
            .iter()
            .filter(|node| node.id != source_node_id && !node.trashed)
        {
            collect_spelling_mentions(
                source,
                block_start..block_end,
                &exclusions,
                node,
                &node.name,
                true,
                &mut grouped,
            );
            for alias in &node.aliases {
                collect_spelling_mentions(
                    source,
                    block_start..block_end,
                    &exclusions,
                    node,
                    alias,
                    false,
                    &mut grouped,
                );
            }
        }
    }
    let mut result = grouped
        .into_values()
        .map(|builder| builder.finish(source_node_id))
        .collect::<Vec<_>>();
    result.sort_by(compare_mentions);
    result
}

#[allow(clippy::too_many_arguments)]
fn collect_spelling_mentions(
    source: &str,
    range: Range<usize>,
    exclusions: &[Range<usize>],
    node: &NodeLinkEntry,
    spelling: &str,
    canonical: bool,
    grouped: &mut BTreeMap<(usize, usize, String), MentionBuilder>,
) {
    if spelling.is_empty() {
        return;
    }
    for (relative, _) in source[range.clone()].match_indices(spelling) {
        let start = range.start + relative;
        let end = start + spelling.len();
        if overlaps_any(start..end, exclusions) {
            continue;
        }
        let key = (start, end, source[start..end].to_owned());
        grouped
            .entry(key)
            .or_insert_with(|| MentionBuilder::new(start, end, spelling, canonical, true, node))
            .add(node.id, canonical, true, &node.name);
    }

    let scalar_length = spelling.chars().count();
    if scalar_length == 0 {
        return;
    }
    let normalized_spelling = normalize(spelling);
    for (start, _) in source[range.clone()].char_indices() {
        let absolute_start = range.start + start;
        let Some(absolute_end) =
            end_after_scalars(source, absolute_start, range.end, scalar_length)
        else {
            continue;
        };
        if source[absolute_start..absolute_end] == *spelling
            || normalize(&source[absolute_start..absolute_end]) != normalized_spelling
            || overlaps_any(absolute_start..absolute_end, exclusions)
        {
            continue;
        }
        let text = source[absolute_start..absolute_end].to_owned();
        let key = (absolute_start, absolute_end, text);
        grouped
            .entry(key)
            .or_insert_with(|| {
                MentionBuilder::new(
                    absolute_start,
                    absolute_end,
                    spelling,
                    canonical,
                    false,
                    node,
                )
            })
            .add(node.id, canonical, false, &node.name);
    }
}

struct MentionBuilder {
    start: usize,
    end: usize,
    matched_text: String,
    quality: LinkMatchQuality,
    target_ids: Vec<NodeId>,
    target_names: Vec<String>,
}

impl MentionBuilder {
    fn new(
        start: usize,
        end: usize,
        spelling: &str,
        canonical: bool,
        exact: bool,
        node: &NodeLinkEntry,
    ) -> Self {
        let mut value = Self {
            start,
            end,
            matched_text: spelling.to_owned(),
            quality: quality(canonical, exact),
            target_ids: Vec::new(),
            target_names: Vec::new(),
        };
        value.add(node.id, canonical, exact, &node.name);
        value
    }

    fn add(&mut self, id: NodeId, canonical: bool, exact: bool, target_name: &str) {
        self.quality = self.quality.min(quality(canonical, exact));
        if !self.target_ids.contains(&id) {
            self.target_ids.push(id);
            self.target_names.push(target_name.to_owned());
        }
    }

    fn finish(mut self, source_node_id: NodeId) -> PotentialMention {
        let mut targets = self
            .target_ids
            .drain(..)
            .zip(self.target_names.drain(..))
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| {
            natural_key(&left.1)
                .cmp(&natural_key(&right.1))
                .then_with(|| left.0.cmp(&right.0))
        });
        PotentialMention {
            source_node_id,
            start: to_u64(self.start),
            end: to_u64(self.end),
            matched_scalar_length: to_u64(self.matched_text.chars().count()),
            matched_text: self.matched_text,
            quality: self.quality,
            target_node_ids: targets.into_iter().map(|target| target.0).collect(),
            primary: true,
            contained_by_start: None,
        }
    }
}

fn mark_contained_candidates(candidates: &mut [PotentialMention]) {
    candidates.sort_by(compare_mentions);
    let mut primaries = Vec::<(u64, u64)>::new();
    for candidate in candidates {
        if let Some((start, _)) = primaries
            .iter()
            .find(|(start, end)| *start <= candidate.start && *end >= candidate.end)
        {
            candidate.primary = false;
            candidate.contained_by_start = Some(*start);
        } else {
            primaries.push((candidate.start, candidate.end));
        }
    }
}

fn compare_mentions(left: &PotentialMention, right: &PotentialMention) -> Ordering {
    right
        .matched_scalar_length
        .cmp(&left.matched_scalar_length)
        .then_with(|| left.quality.cmp(&right.quality))
        .then_with(|| left.start.cmp(&right.start))
        .then_with(|| left.target_node_ids.cmp(&right.target_node_ids))
}

fn quality(canonical: bool, exact: bool) -> LinkMatchQuality {
    match (canonical, exact) {
        (true, true) => LinkMatchQuality::ExactCanonical,
        (false, true) => LinkMatchQuality::ExactAlias,
        (true, false) => LinkMatchQuality::NormalizedCanonical,
        (false, false) => LinkMatchQuality::NormalizedAlias,
    }
}

fn parse_canonical_aliases(source: &str) -> Result<Vec<String>, LinkIndexError> {
    parse_node_aliases(source).map_err(|error| LinkIndexError::InvalidAliases(error.to_string()))
}

fn node_locator(root: &Path, path: &Path, root_name: &str) -> Result<String, LinkIndexError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| LinkIndexError::Path(path.to_path_buf()))?;
    if relative.as_os_str().is_empty() {
        return Ok(root_name.to_owned());
    }
    let mut pieces = Vec::new();
    for component in relative.components() {
        let std::path::Component::Normal(piece) = component else {
            return Err(LinkIndexError::Path(path.to_path_buf()));
        };
        pieces.push(
            piece
                .to_str()
                .ok_or_else(|| LinkIndexError::Path(path.to_path_buf()))?,
        );
    }
    Ok(pieces.join("/"))
}

fn overlaps_any(range: Range<usize>, exclusions: &[Range<usize>]) -> bool {
    exclusions
        .iter()
        .any(|excluded| range.start < excluded.end && excluded.start < range.end)
}

fn end_after_scalars(source: &str, start: usize, end: usize, count: usize) -> Option<usize> {
    let mut cursor = start;
    for _ in 0..count {
        cursor = next_char_boundary(source, cursor, end);
        if cursor > end {
            return None;
        }
    }
    (cursor <= end).then_some(cursor)
}

fn next_char_boundary(source: &str, cursor: usize, end: usize) -> usize {
    if cursor >= end {
        return end.saturating_add(1);
    }
    cursor + source[cursor..end].chars().next().map_or(1, char::len_utf8)
}

fn normalize(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

fn natural_key(value: &str) -> String {
    normalize(value)
}

fn sorted_unique(mut values: Vec<NodeId>) -> Vec<NodeId> {
    values.sort();
    values.dedup();
    values
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[derive(Debug)]
pub enum LinkIndexError {
    InvalidInventory,
    InvalidAliases(String),
    Path(std::path::PathBuf),
    Document(DocumentError),
    WorkspaceRevision(WorkspaceRevisionError),
}

impl fmt::Display for LinkIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInventory => formatter.write_str("workspace inventory is invalid"),
            Self::InvalidAliases(message) => write!(formatter, "invalid aliases: {message}"),
            Self::Path(path) => {
                write!(formatter, "invalid workspace link path: {}", path.display())
            }
            Self::Document(error) => error.fmt(formatter),
            Self::WorkspaceRevision(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LinkIndexError {}
