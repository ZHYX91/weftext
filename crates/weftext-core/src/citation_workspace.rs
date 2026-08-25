use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    BibliographyInclusion, BibliographyOccurrence, CitationData, CitationDiagnosticCode,
    CitationForm, CitationItem, DocumentRevision, InventoryIssueCode, NodeId, ReferenceDiagnostic,
    ReferenceValue, WorkspaceDocumentGeneration, analyze_citation_source,
    analyze_reference_metadata, parse_node_metadata, scan_workspace,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceDeclaration {
    pub node_id: NodeId,
    pub revision: DocumentRevision,
    pub citation_data: CitationData,
    pub mapping_range: Range<u64>,
    pub key_range: Range<u64>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationWorkspaceDiagnosticCode {
    InvalidReferenceMetadata,
    DuplicateReferenceKey,
    InvalidCitationSyntax,
    MissingReferenceKey,
    UnavailableReferenceKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationWorkspaceDiagnostic {
    pub code: CitationWorkspaceDiagnosticCode,
    pub message: String,
    pub component_node_id: Option<NodeId>,
    pub key: Option<String>,
    pub range: Option<Range<u64>>,
    pub reference_node_ids: Vec<NodeId>,
    pub citation_code: Option<CitationDiagnosticCode>,
    pub reference_diagnostics: Vec<ReferenceDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DisclosureMode {
    Complete,
    Filtered,
}

/// Explicit permission-filtered set of reference nodes available to one compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationAccessScope {
    reference_node_ids: BTreeSet<NodeId>,
    disclosure: DisclosureMode,
}

impl CitationAccessScope {
    /// Creates a scope that does not distinguish missing, hidden, or globally ambiguous keys.
    #[must_use]
    pub fn filtered(reference_node_ids: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            reference_node_ids: reference_node_ids.into_iter().collect(),
            disclosure: DisclosureMode::Filtered,
        }
    }

    /// Creates a scope known to contain every reference visible to the caller.
    ///
    /// This is appropriate for a local workspace or an Owner-equivalent complete view. A
    /// permission-filtered Server request must use [`Self::filtered`] so diagnostics cannot reveal
    /// hidden candidates.
    #[must_use]
    pub fn complete(reference_node_ids: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            reference_node_ids: reference_node_ids.into_iter().collect(),
            disclosure: DisclosureMode::Complete,
        }
    }

    #[must_use]
    pub fn allows(&self, node_id: NodeId) -> bool {
        self.reference_node_ids.contains(&node_id)
    }
}

#[derive(Clone, Debug)]
pub struct CitationWorkspaceIndex {
    generation: WorkspaceDocumentGeneration,
    node_paths: BTreeMap<NodeId, PathBuf>,
    declarations: BTreeMap<String, Vec<ReferenceDeclaration>>,
    declarations_by_node: BTreeMap<NodeId, ReferenceDeclaration>,
    diagnostics: Vec<CitationWorkspaceDiagnostic>,
}

impl CitationWorkspaceIndex {
    /// Rebuilds reference declarations from exact managed `AsciiDoc` source.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid workspace, a non-AsciiDoc generation, or a document that
    /// cannot be reopened through the selected Core generation boundary.
    pub fn rebuild(root: impl AsRef<Path>) -> Result<Self, CitationWorkspaceError> {
        Self::rebuild_internal(root.as_ref(), None)
    }

    /// Rebuilds citation data only from nodes in an already-authorized
    /// projection. The scope check occurs before any document body is opened.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid workspace/scope or an unreadable
    /// authorized document.
    pub fn rebuild_scoped(
        root: impl AsRef<Path>,
        scope: &crate::WorkspaceReadScope,
    ) -> Result<Self, CitationWorkspaceError> {
        Self::rebuild_internal(root.as_ref(), Some(scope))
    }

    #[allow(clippy::too_many_lines)]
    fn rebuild_internal(
        root: &Path,
        scope: Option<&crate::WorkspaceReadScope>,
    ) -> Result<Self, CitationWorkspaceError> {
        let inventory = scan_workspace(root);
        if let Some(scope) = scope {
            scope
                .validate_inventory(&inventory)
                .map_err(|_| CitationWorkspaceError::InvalidScope)?;
        } else if !inventory.is_valid() {
            return Err(CitationWorkspaceError::InvalidWorkspace(
                inventory
                    .issues
                    .first()
                    .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
            ));
        }
        if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
            return Err(CitationWorkspaceError::UnsupportedGeneration(
                inventory.generation,
            ));
        }
        let mut index = Self {
            generation: inventory.generation,
            node_paths: BTreeMap::new(),
            declarations: BTreeMap::new(),
            declarations_by_node: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        for node in &inventory.nodes {
            let Some(node_id) = node.id else {
                if scope.is_some() {
                    continue;
                }
                return Err(CitationWorkspaceError::InvalidWorkspace(
                    InventoryIssueCode::MissingIdentity,
                ));
            };
            if scope.is_some_and(|scope| !scope.allows(node_id)) {
                continue;
            }
            index.node_paths.insert(node_id, node.path.clone());
            let snapshot = crate::read_node_document(&node.path).map_err(|error| {
                CitationWorkspaceError::DocumentRead {
                    node_id,
                    message: error.to_string(),
                }
            })?;
            let analysis = analyze_reference_metadata(&snapshot.source);
            let Some(mapping_range) = analysis.mapping_range else {
                continue;
            };
            let Some(citation_data) = analysis.citation_data else {
                index.diagnostics.push(CitationWorkspaceDiagnostic {
                    code: CitationWorkspaceDiagnosticCode::InvalidReferenceMetadata,
                    message: "reference metadata is invalid and unavailable for resolution"
                        .to_owned(),
                    component_node_id: None,
                    key: None,
                    range: Some(mapping_range),
                    reference_node_ids: vec![node_id],
                    citation_code: None,
                    reference_diagnostics: analysis.diagnostics,
                });
                continue;
            };
            let key_range = analysis
                .field_ranges
                .iter()
                .find(|field| field.path == "reference.key")
                .map(|field| field.value_range.clone())
                .ok_or_else(|| CitationWorkspaceError::DocumentRead {
                    node_id,
                    message: "valid Citation Data did not expose reference.key range".to_owned(),
                })?;
            let declaration = ReferenceDeclaration {
                node_id,
                revision: snapshot.revision,
                citation_data,
                mapping_range,
                key_range,
            };
            index
                .declarations
                .entry(declaration.citation_data.key.clone())
                .or_default()
                .push(declaration.clone());
            index.declarations_by_node.insert(node_id, declaration);
        }

        for (key, declarations) in &mut index.declarations {
            declarations.sort_by_key(|declaration| declaration.node_id);
            if declarations.len() > 1 {
                index.diagnostics.push(CitationWorkspaceDiagnostic {
                    code: CitationWorkspaceDiagnosticCode::DuplicateReferenceKey,
                    message: format!("reference key `{key}` is declared by multiple nodes"),
                    component_node_id: None,
                    key: Some(key.clone()),
                    range: None,
                    reference_node_ids: declarations
                        .iter()
                        .map(|declaration| declaration.node_id)
                        .collect(),
                    citation_code: None,
                    reference_diagnostics: Vec::new(),
                });
            }
        }
        index.diagnostics.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.key.cmp(&right.key))
                .then_with(|| left.reference_node_ids.cmp(&right.reference_node_ids))
        });
        Ok(index)
    }

    #[must_use]
    pub const fn generation(&self) -> WorkspaceDocumentGeneration {
        self.generation
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[CitationWorkspaceDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn declarations_for_key(&self, key: &str) -> &[ReferenceDeclaration] {
        self.declarations.get(key).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn declaration_for_node(&self, node_id: NodeId) -> Option<&ReferenceDeclaration> {
        self.declarations_by_node.get(&node_id)
    }

    pub fn reference_node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.declarations_by_node.keys().copied()
    }

    /// Searches only reference declarations visible in an explicit access scope.
    ///
    /// Search fields never participate in citation identity resolution. A filtered scope omits an
    /// ambiguous key completely so a hidden duplicate cannot affect candidate counts or status.
    /// A complete local scope may return the declarations as non-selectable diagnostic rows.
    ///
    /// # Errors
    ///
    /// Returns an error for a query longer than 512 bytes or a result limit above 100.
    pub fn search_references(
        &self,
        query: &str,
        scope: &CitationAccessScope,
        limit: usize,
    ) -> Result<Vec<ReferenceSearchHit>, ReferenceSearchError> {
        if query.len() > 512 {
            return Err(ReferenceSearchError::QueryTooLong);
        }
        if limit > 100 {
            return Err(ReferenceSearchError::LimitTooLarge);
        }
        let needle = query.trim().to_lowercase();
        if needle.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut matches = Vec::new();
        for declaration in self.declarations_by_node.values() {
            if !scope.allows(declaration.node_id) {
                continue;
            }
            let declarations = self.declarations_for_key(&declaration.citation_data.key);
            let selectable = declarations.len() == 1;
            if !selectable && scope.disclosure == DisclosureMode::Filtered {
                continue;
            }
            if let Some((rank, matched_fields, contributors, identifiers)) =
                reference_match(&declaration.citation_data, &needle)
            {
                matches.push((
                    rank,
                    ReferenceSearchHit {
                        node_id: declaration.node_id,
                        key: declaration.citation_data.key.clone(),
                        item_type: declaration.citation_data.item_type.clone(),
                        title: declaration.citation_data.title.clone(),
                        contributors,
                        identifiers,
                        selectable,
                        matched_fields,
                    },
                ));
            }
        }
        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| left.key.cmp(&right.key))
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        matches.truncate(limit);
        Ok(matches.into_iter().map(|(_, hit)| hit).collect())
    }

    /// Resolves one component through exact keys and an explicit permission-filtered scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the component is absent or cannot be reopened exactly.
    pub fn analyze_component(
        &self,
        component_node_id: NodeId,
        scope: &CitationAccessScope,
    ) -> Result<CitationComponentAnalysis, CitationWorkspaceError> {
        let path = self
            .node_paths
            .get(&component_node_id)
            .ok_or(CitationWorkspaceError::MissingComponent(component_node_id))?;
        let snapshot = crate::read_node_document(path).map_err(|error| {
            CitationWorkspaceError::DocumentRead {
                node_id: component_node_id,
                message: error.to_string(),
            }
        })?;
        Ok(self.resolve_component_source(
            component_node_id,
            snapshot.revision,
            &snapshot.source,
            scope,
        ))
    }

    /// Resolves one exact source draft through the current workspace reference index.
    ///
    /// The supplied source must retain the requested component UUID. This supports unsaved
    /// Desktop/WebUI drafts without making the client a key resolver or citation parser.
    ///
    /// # Errors
    ///
    /// Returns an error when the component is absent, its source metadata is invalid, or its UUID
    /// does not match the requested component.
    pub fn analyze_component_source(
        &self,
        component_node_id: NodeId,
        source: &str,
        scope: &CitationAccessScope,
    ) -> Result<CitationComponentAnalysis, CitationWorkspaceError> {
        if !self.node_paths.contains_key(&component_node_id) {
            return Err(CitationWorkspaceError::MissingComponent(component_node_id));
        }
        let metadata =
            parse_node_metadata(source).map_err(|error| CitationWorkspaceError::DocumentRead {
                node_id: component_node_id,
                message: error.to_string(),
            })?;
        if metadata.id != Some(component_node_id) {
            return Err(CitationWorkspaceError::DocumentRead {
                node_id: component_node_id,
                message: "draft source identity does not match the requested component".to_owned(),
            });
        }
        Ok(self.resolve_component_source(
            component_node_id,
            DocumentRevision::from_source(source),
            source,
            scope,
        ))
    }

    fn resolve_component_source(
        &self,
        component_node_id: NodeId,
        revision: DocumentRevision,
        source: &str,
        scope: &CitationAccessScope,
    ) -> CitationComponentAnalysis {
        let source_analysis = analyze_citation_source(source);
        let mut diagnostics = source_analysis
            .diagnostics
            .into_iter()
            .map(|diagnostic| CitationWorkspaceDiagnostic {
                code: CitationWorkspaceDiagnosticCode::InvalidCitationSyntax,
                message: diagnostic.message,
                component_node_id: Some(component_node_id),
                key: None,
                range: Some(diagnostic.range),
                reference_node_ids: Vec::new(),
                citation_code: Some(diagnostic.code),
                reference_diagnostics: Vec::new(),
            })
            .collect::<Vec<_>>();

        let mut clusters = Vec::new();
        for cluster in source_analysis.clusters {
            let mut resolved = Vec::new();
            let mut available = true;
            for item in &cluster.items {
                match self.resolve_item(component_node_id, item, scope) {
                    Ok(item) => resolved.push(item),
                    Err(diagnostic) => {
                        diagnostics.push(*diagnostic);
                        available = false;
                    }
                }
            }
            if available {
                clusters.push(ResolvedCitationCluster {
                    form: cluster.form,
                    range: cluster.range,
                    items: resolved,
                });
            }
        }

        let mut nocites = Vec::new();
        for nocite in source_analysis.nocites {
            let mut references = Vec::new();
            let mut available = true;
            for key in &nocite.keys {
                match self.resolve_key(component_node_id, &key.key, &key.range, scope) {
                    Ok(declaration) => references.push(ResolvedReference {
                        node_id: declaration.node_id,
                        citation_data: declaration.citation_data.clone(),
                        key_range: key.range.clone(),
                    }),
                    Err(diagnostic) => {
                        diagnostics.push(*diagnostic);
                        available = false;
                    }
                }
            }
            if available {
                nocites.push(ResolvedNoCiteOccurrence {
                    range: nocite.range,
                    references,
                });
            }
        }
        diagnostics.sort_by(|left, right| {
            range_sort_key(left.range.as_ref())
                .cmp(&range_sort_key(right.range.as_ref()))
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.key.cmp(&right.key))
        });

        CitationComponentAnalysis {
            component_node_id,
            revision,
            clusters,
            nocites,
            bibliography: source_analysis.bibliographies.into_iter().next(),
            diagnostics,
        }
    }

    /// Builds one bibliography compilation from an exact unsaved source draft.
    ///
    /// # Errors
    ///
    /// Returns an error when the component source cannot be resolved through the current explicit
    /// access scope.
    pub fn collect_bibliography_input_for_source(
        &self,
        component_node_id: NodeId,
        source: &str,
        scope: &CitationAccessScope,
    ) -> Result<BibliographyCompilation, CitationWorkspaceError> {
        let analysis = self.analyze_component_source(component_node_id, source, scope)?;
        let diagnostics = analysis.diagnostics.clone();
        Ok(BibliographyCompilation {
            components: vec![self.component_input(analysis, scope)],
            diagnostics,
        })
    }

    /// Builds ordered bibliography inputs for an explicit ordered component and reference scope.
    ///
    /// # Errors
    ///
    /// Returns an error for a duplicate component or when a component cannot be analyzed.
    pub fn collect_bibliography_inputs(
        &self,
        component_node_ids: &[NodeId],
        scope: &CitationAccessScope,
    ) -> Result<BibliographyCompilation, CitationWorkspaceError> {
        let mut seen_components = BTreeSet::new();
        let mut components = Vec::new();
        let mut diagnostics = Vec::new();
        for component_node_id in component_node_ids {
            if !seen_components.insert(*component_node_id) {
                return Err(CitationWorkspaceError::DuplicateComponent(
                    *component_node_id,
                ));
            }
            let analysis = self.analyze_component(*component_node_id, scope)?;
            diagnostics.extend(analysis.diagnostics.clone());
            components.push(self.component_input(analysis, scope));
        }
        Ok(BibliographyCompilation {
            components,
            diagnostics,
        })
    }

    fn component_input(
        &self,
        analysis: CitationComponentAnalysis,
        scope: &CitationAccessScope,
    ) -> BibliographyComponentInput {
        let mut references = Vec::new();
        let mut seen_references = BTreeSet::new();
        let mut source_batches = analysis
            .clusters
            .iter()
            .map(|cluster| {
                (
                    cluster.range.start,
                    cluster
                        .items
                        .iter()
                        .map(|item| &item.reference)
                        .collect::<Vec<_>>(),
                )
            })
            .chain(analysis.nocites.iter().map(|nocite| {
                (
                    nocite.range.start,
                    nocite.references.iter().collect::<Vec<_>>(),
                )
            }))
            .collect::<Vec<_>>();
        source_batches.sort_by_key(|(start, _)| *start);
        for reference in source_batches
            .into_iter()
            .flat_map(|(_, references)| references)
        {
            push_bibliography_reference(&mut references, &mut seen_references, reference);
        }
        if analysis
            .bibliography
            .as_ref()
            .is_some_and(|bibliography| bibliography.inclusion == BibliographyInclusion::All)
        {
            for declarations in self.declarations.values() {
                let [declaration] = declarations.as_slice() else {
                    continue;
                };
                if scope.allows(declaration.node_id) && seen_references.insert(declaration.node_id)
                {
                    references.push(BibliographyReferenceInput {
                        node_id: declaration.node_id,
                        citation_data: declaration.citation_data.clone(),
                    });
                }
            }
        }
        BibliographyComponentInput {
            component_node_id: analysis.component_node_id,
            revision: analysis.revision,
            placement: analysis.bibliography,
            clusters: analysis.clusters,
            references,
        }
    }

    fn resolve_item(
        &self,
        component_node_id: NodeId,
        item: &CitationItem,
        scope: &CitationAccessScope,
    ) -> Result<ResolvedCitationItem, Box<CitationWorkspaceDiagnostic>> {
        let declaration =
            self.resolve_key(component_node_id, &item.key.key, &item.key.range, scope)?;
        Ok(ResolvedCitationItem {
            range: item.range.clone(),
            label: item.label.clone(),
            locator: item.locator.clone(),
            prefix: item.prefix.clone(),
            suffix: item.suffix.clone(),
            reference: ResolvedReference {
                node_id: declaration.node_id,
                citation_data: declaration.citation_data.clone(),
                key_range: item.key.range.clone(),
            },
        })
    }

    fn resolve_key<'a>(
        &'a self,
        component_node_id: NodeId,
        key: &str,
        range: &Range<u64>,
        scope: &CitationAccessScope,
    ) -> Result<&'a ReferenceDeclaration, Box<CitationWorkspaceDiagnostic>> {
        let declarations = self.declarations_for_key(key);
        if declarations.len() == 1 && scope.allows(declarations[0].node_id) {
            return Ok(&declarations[0]);
        }

        let complete = scope.disclosure == DisclosureMode::Complete;
        let (code, message, reference_node_ids) = if complete && declarations.is_empty() {
            (
                CitationWorkspaceDiagnosticCode::MissingReferenceKey,
                format!("reference key `{key}` does not exist"),
                Vec::new(),
            )
        } else if complete
            && declarations.len() > 1
            && declarations
                .iter()
                .all(|declaration| scope.allows(declaration.node_id))
        {
            (
                CitationWorkspaceDiagnosticCode::DuplicateReferenceKey,
                format!("reference key `{key}` is ambiguous"),
                declarations
                    .iter()
                    .map(|declaration| declaration.node_id)
                    .collect(),
            )
        } else {
            (
                CitationWorkspaceDiagnosticCode::UnavailableReferenceKey,
                format!("reference key `{key}` is unavailable in this scope"),
                Vec::new(),
            )
        };
        Err(Box::new(CitationWorkspaceDiagnostic {
            code,
            message,
            component_node_id: Some(component_node_id),
            key: Some(key.to_owned()),
            range: Some(range.clone()),
            reference_node_ids,
            citation_code: None,
            reference_diagnostics: Vec::new(),
        }))
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSearchField {
    Key,
    Title,
    Contributor,
    Identifier,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceSearchHit {
    pub node_id: NodeId,
    pub key: String,
    pub item_type: String,
    pub title: String,
    pub contributors: Vec<String>,
    pub identifiers: BTreeMap<String, String>,
    pub selectable: bool,
    pub matched_fields: Vec<ReferenceSearchField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceSearchError {
    QueryTooLong,
    LimitTooLarge,
}

impl fmt::Display for ReferenceSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryTooLong => formatter.write_str("reference search query is too long"),
            Self::LimitTooLarge => formatter.write_str("reference search limit exceeds 100"),
        }
    }
}

impl std::error::Error for ReferenceSearchError {}

type ReferenceMatch = (
    u8,
    Vec<ReferenceSearchField>,
    Vec<String>,
    BTreeMap<String, String>,
);

fn reference_match(data: &CitationData, needle: &str) -> Option<ReferenceMatch> {
    let contributors = data
        .fields
        .values()
        .filter_map(|value| match value {
            ReferenceValue::Names(names) => Some(names),
            ReferenceValue::Text(_) | ReferenceValue::Date(_) => None,
        })
        .flatten()
        .map(|name| {
            name.literal.clone().unwrap_or_else(|| {
                [name.given.as_deref(), name.family.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
        })
        .collect::<Vec<_>>();
    let identifiers = ["DOI", "ISBN", "ISSN", "PMCID", "PMID", "URL"]
        .into_iter()
        .filter_map(|field| match data.fields.get(field) {
            Some(ReferenceValue::Text(value)) => Some((field.to_owned(), value.clone())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();

    let key = data.key.to_lowercase();
    let title = data.title.to_lowercase();
    let mut rank = u8::MAX;
    let mut fields = Vec::new();
    if key == needle {
        rank = 0;
        fields.push(ReferenceSearchField::Key);
    } else if key.starts_with(needle) {
        rank = 1;
        fields.push(ReferenceSearchField::Key);
    } else if key.contains(needle) {
        rank = 2;
        fields.push(ReferenceSearchField::Key);
    }
    if title.starts_with(needle) {
        rank = rank.min(3);
        fields.push(ReferenceSearchField::Title);
    } else if title.contains(needle) {
        rank = rank.min(4);
        fields.push(ReferenceSearchField::Title);
    }
    if contributors
        .iter()
        .any(|value| value.to_lowercase().contains(needle))
    {
        rank = rank.min(5);
        fields.push(ReferenceSearchField::Contributor);
    }
    if identifiers
        .values()
        .any(|value| value.to_lowercase().contains(needle))
    {
        rank = rank.min(6);
        fields.push(ReferenceSearchField::Identifier);
    }
    if rank == u8::MAX {
        None
    } else {
        fields.sort();
        fields.dedup();
        Some((rank, fields, contributors, identifiers))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedReference {
    pub node_id: NodeId,
    pub citation_data: CitationData,
    pub key_range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCitationItem {
    pub range: Range<u64>,
    pub label: String,
    pub locator: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub reference: ResolvedReference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCitationCluster {
    pub form: CitationForm,
    pub range: Range<u64>,
    pub items: Vec<ResolvedCitationItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedNoCiteOccurrence {
    pub range: Range<u64>,
    pub references: Vec<ResolvedReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationComponentAnalysis {
    pub component_node_id: NodeId,
    pub revision: DocumentRevision,
    pub clusters: Vec<ResolvedCitationCluster>,
    pub nocites: Vec<ResolvedNoCiteOccurrence>,
    pub bibliography: Option<BibliographyOccurrence>,
    pub diagnostics: Vec<CitationWorkspaceDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyReferenceInput {
    pub node_id: NodeId,
    pub citation_data: CitationData,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyComponentInput {
    pub component_node_id: NodeId,
    pub revision: DocumentRevision,
    pub placement: Option<BibliographyOccurrence>,
    pub clusters: Vec<ResolvedCitationCluster>,
    pub references: Vec<BibliographyReferenceInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyCompilation {
    pub components: Vec<BibliographyComponentInput>,
    pub diagnostics: Vec<CitationWorkspaceDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CitationWorkspaceError {
    InvalidWorkspace(InventoryIssueCode),
    UnsupportedGeneration(WorkspaceDocumentGeneration),
    DocumentRead { node_id: NodeId, message: String },
    InvalidScope,
    MissingComponent(NodeId),
    DuplicateComponent(NodeId),
}

impl fmt::Display for CitationWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace(code) => {
                write!(formatter, "workspace inventory is invalid: {code:?}")
            }
            Self::UnsupportedGeneration(generation) => {
                write!(
                    formatter,
                    "citation runtime requires AsciiDoc v1, got {generation:?}"
                )
            }
            Self::DocumentRead { node_id, message } => {
                write!(formatter, "could not read node {node_id}: {message}")
            }
            Self::InvalidScope => formatter.write_str("citation read scope is invalid"),
            Self::MissingComponent(node_id) => {
                write!(
                    formatter,
                    "citation component {node_id} is not in the workspace"
                )
            }
            Self::DuplicateComponent(node_id) => {
                write!(
                    formatter,
                    "citation component {node_id} occurs more than once"
                )
            }
        }
    }
}

impl std::error::Error for CitationWorkspaceError {}

fn range_sort_key(range: Option<&Range<u64>>) -> (u64, u64) {
    range.map_or((u64::MAX, u64::MAX), |range| (range.start, range.end))
}

fn push_bibliography_reference(
    inputs: &mut Vec<BibliographyReferenceInput>,
    seen: &mut BTreeSet<NodeId>,
    reference: &ResolvedReference,
) {
    if seen.insert(reference.node_id) {
        inputs.push(BibliographyReferenceInput {
            node_id: reference.node_id,
            citation_data: reference.citation_data.clone(),
        });
    }
}
