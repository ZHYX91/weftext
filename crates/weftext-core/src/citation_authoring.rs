use std::fmt::Write as _;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::{
    BibliographyInclusion, CitationAccessScope, CitationForm, CitationSourceAnalysis,
    CitationWorkspaceIndex, DocumentEdit, DocumentRevision, NodeId, ReferenceAnalysis,
    analyze_citation_source, analyze_reference_metadata,
};

const MAX_AUTHORING_ITEMS: usize = 1_024;

pub const REFERENCE_RECORD_WRITES_RETIREMENT: &str = "reference-record creation, field editing, and key renaming are unavailable until the canonical typed Citation Data construct and recoverable converter are accepted";

#[must_use]
pub const fn reference_record_writes_available() -> bool {
    false
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationAuthoringPlan {
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub edit: DocumentEdit,
    pub proposed_source: String,
    pub analysis: CitationAuthoringAnalysis,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationAuthoringAnalysis {
    pub reference: ReferenceAnalysis,
    pub citations: CitationSourceAnalysis,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationClusterIntent {
    pub form: CitationForm,
    pub items: Vec<CitationItemIntent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationItemIntent {
    pub reference_node_id: NodeId,
    pub label: Option<String>,
    pub locator: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CitationMacroIntent {
    Citation { cluster: CitationClusterIntent },
    NoCite { reference_node_ids: Vec<NodeId> },
    Bibliography { inclusion: BibliographyInclusion },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CitationEditTarget {
    Insert { offset: u64 },
    Replace { range: Range<u64> },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationAuthoringDiagnosticCode {
    InvalidSource,
    ReferenceUnavailable,
    InvalidCitationIntent,
    InvalidEditTarget,
    RequestLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationAuthoringFailure {
    pub code: CitationAuthoringDiagnosticCode,
    pub message: String,
    pub range: Option<Range<u64>>,
}

impl std::fmt::Display for CitationAuthoringFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CitationAuthoringFailure {}

/// Returns the Core-owned reference and citation model for one exact source draft.
#[must_use]
pub fn analyze_citation_authoring_source(source: &str) -> CitationAuthoringAnalysis {
    CitationAuthoringAnalysis {
        reference: analyze_reference_metadata(source),
        citations: analyze_citation_source(source),
    }
}

/// Plans insertion or replacement of one canonical citation surface.
///
/// Reference selections carry UUIDs. Core resolves their current authored keys through the
/// supplied permission scope and refuses ambiguous, missing, or hidden selections without
/// returning alternate candidates.
///
/// # Errors
///
/// Returns a fail-closed diagnostic for an invalid draft, unavailable selected reference,
/// malformed typed intent, protected insertion point, or range that is not an exact existing
/// citation surface of the same kind.
pub fn plan_citation_macro_edit(
    index: &CitationWorkspaceIndex,
    component_node_id: NodeId,
    source: &str,
    scope: &CitationAccessScope,
    target: &CitationEditTarget,
    intent: &CitationMacroIntent,
) -> Result<CitationAuthoringPlan, CitationAuthoringFailure> {
    let current = analyze_citation_source(source);
    if let Some(diagnostic) = current.diagnostics.first() {
        return Err(authoring_failure(
            CitationAuthoringDiagnosticCode::InvalidSource,
            diagnostic.message.clone(),
            Some(diagnostic.range.clone()),
        ));
    }
    validate_target_kind(&current, target, intent)?;
    let replacement = canonical_macro(index, component_node_id, scope, intent)?;
    let edit_range = target_range(source, target)?;
    let mut proposed_source = source.to_owned();
    proposed_source.replace_range(edit_range.clone(), &replacement);
    let next = analyze_citation_source(&proposed_source);
    if let Some(diagnostic) = next.diagnostics.first() {
        return Err(authoring_failure(
            CitationAuthoringDiagnosticCode::InvalidCitationIntent,
            diagnostic.message.clone(),
            Some(diagnostic.range.clone()),
        ));
    }
    let expected = edit_range.start as u64..(edit_range.start + replacement.len()) as u64;
    if !analysis_contains_macro(&next, intent, &expected) {
        return Err(authoring_failure(
            CitationAuthoringDiagnosticCode::InvalidEditTarget,
            "the citation surface is not eligible at the requested exact source range".to_owned(),
            Some(expected),
        ));
    }
    Ok(build_plan(source, proposed_source))
}

fn canonical_macro(
    index: &CitationWorkspaceIndex,
    _component_node_id: NodeId,
    scope: &CitationAccessScope,
    intent: &CitationMacroIntent,
) -> Result<String, CitationAuthoringFailure> {
    match intent {
        CitationMacroIntent::Citation { cluster } => {
            if cluster.items.is_empty() || cluster.items.len() > MAX_AUTHORING_ITEMS {
                return Err(intent_failure(
                    "citation cluster item count is outside the v1 limit",
                ));
            }
            if cluster.form == CitationForm::Narrative && cluster.items.len() != 1 {
                return Err(intent_failure(
                    "narrative citations accept exactly one selected reference",
                ));
            }
            let mut result = match cluster.form {
                CitationForm::Parenthetical => "cite:".to_owned(),
                CitationForm::Narrative => "cite:narrative".to_owned(),
            };
            for (item_index, item) in cluster.items.iter().enumerate() {
                if item_index > 0 {
                    result.push('+');
                }
                let key = selectable_key(index, scope, item.reference_node_id)?;
                result.push('[');
                result.push_str(key);
                for (name, value) in [
                    ("label", item.label.as_deref()),
                    ("locator", item.locator.as_deref()),
                    ("prefix", item.prefix.as_deref()),
                    ("suffix", item.suffix.as_deref()),
                ] {
                    if let Some(value) = value {
                        let encoded =
                            serde_json::to_string(value).expect("serializing a String cannot fail");
                        let _ = write!(result, ",{name}={encoded}");
                    }
                }
                result.push(']');
            }
            Ok(result)
        }
        CitationMacroIntent::NoCite { reference_node_ids } => {
            if reference_node_ids.is_empty() || reference_node_ids.len() > MAX_AUTHORING_ITEMS {
                return Err(intent_failure("nocite item count is outside the v1 limit"));
            }
            let mut keys = Vec::with_capacity(reference_node_ids.len());
            for node_id in reference_node_ids {
                keys.push(selectable_key(index, scope, *node_id)?);
            }
            Ok(format!("nocite::[{}]", keys.join(",")))
        }
        CitationMacroIntent::Bibliography { inclusion } => Ok(match inclusion {
            BibliographyInclusion::Cited => "bibliography::[]".to_owned(),
            BibliographyInclusion::All => "bibliography::[include=all]".to_owned(),
        }),
    }
}

fn selectable_key<'a>(
    index: &'a CitationWorkspaceIndex,
    scope: &CitationAccessScope,
    node_id: NodeId,
) -> Result<&'a str, CitationAuthoringFailure> {
    let Some(declaration) = index.declaration_for_node(node_id) else {
        return Err(reference_unavailable());
    };
    if !scope.allows(node_id)
        || index
            .declarations_for_key(&declaration.citation_data.key)
            .len()
            != 1
    {
        return Err(reference_unavailable());
    }
    Ok(&declaration.citation_data.key)
}

fn validate_target_kind(
    analysis: &CitationSourceAnalysis,
    target: &CitationEditTarget,
    intent: &CitationMacroIntent,
) -> Result<(), CitationAuthoringFailure> {
    let CitationEditTarget::Replace { range } = target else {
        return Ok(());
    };
    let exists = match intent {
        CitationMacroIntent::Citation { .. } => analysis
            .clusters
            .iter()
            .any(|candidate| candidate.range == *range),
        CitationMacroIntent::NoCite { .. } => analysis
            .nocites
            .iter()
            .any(|candidate| candidate.range == *range),
        CitationMacroIntent::Bibliography { .. } => analysis
            .bibliographies
            .iter()
            .any(|candidate| candidate.range == *range),
    };
    if exists {
        Ok(())
    } else {
        Err(authoring_failure(
            CitationAuthoringDiagnosticCode::InvalidEditTarget,
            "replacement range is not an exact existing citation surface of the requested kind"
                .to_owned(),
            Some(range.clone()),
        ))
    }
}

fn analysis_contains_macro(
    analysis: &CitationSourceAnalysis,
    intent: &CitationMacroIntent,
    expected: &Range<u64>,
) -> bool {
    match intent {
        CitationMacroIntent::Citation { .. } => analysis
            .clusters
            .iter()
            .any(|candidate| candidate.range == *expected),
        CitationMacroIntent::NoCite { .. } => analysis
            .nocites
            .iter()
            .any(|candidate| candidate.range == *expected),
        CitationMacroIntent::Bibliography { .. } => analysis
            .bibliographies
            .iter()
            .any(|candidate| candidate.range == *expected),
    }
}

fn target_range(
    source: &str,
    target: &CitationEditTarget,
) -> Result<Range<usize>, CitationAuthoringFailure> {
    let (start, end) = match target {
        CitationEditTarget::Insert { offset } => (*offset, *offset),
        CitationEditTarget::Replace { range } => (range.start, range.end),
    };
    let start = usize::try_from(start).map_err(|_| invalid_target())?;
    let end = usize::try_from(end).map_err(|_| invalid_target())?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(invalid_target());
    }
    Ok(start..end)
}

fn build_plan(source: &str, proposed_source: String) -> CitationAuthoringPlan {
    let edit = minimal_edit(source, &proposed_source);
    let analysis = analyze_citation_authoring_source(&proposed_source);
    CitationAuthoringPlan {
        base_revision: DocumentRevision::from_source(source),
        next_revision: DocumentRevision::from_source(&proposed_source),
        edit,
        proposed_source,
        analysis,
    }
}

fn minimal_edit(source: &str, proposed: &str) -> DocumentEdit {
    let mut prefix = source
        .bytes()
        .zip(proposed.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while !source.is_char_boundary(prefix) || !proposed.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let source_tail = &source[prefix..];
    let proposed_tail = &proposed[prefix..];
    let mut suffix = source_tail
        .bytes()
        .rev()
        .zip(proposed_tail.bytes().rev())
        .take_while(|(left, right)| left == right)
        .count()
        .min(source_tail.len().min(proposed_tail.len()));
    while suffix > 0
        && (!source.is_char_boundary(source.len() - suffix)
            || !proposed.is_char_boundary(proposed.len() - suffix))
    {
        suffix -= 1;
    }
    DocumentEdit {
        start: prefix as u64,
        end: (source.len() - suffix) as u64,
        replacement: proposed[prefix..proposed.len() - suffix].to_owned(),
    }
}

fn reference_unavailable() -> CitationAuthoringFailure {
    authoring_failure(
        CitationAuthoringDiagnosticCode::ReferenceUnavailable,
        "the selected reference is unavailable in this scope".to_owned(),
        None,
    )
}

fn intent_failure(message: &str) -> CitationAuthoringFailure {
    authoring_failure(
        CitationAuthoringDiagnosticCode::InvalidCitationIntent,
        message.to_owned(),
        None,
    )
}

fn invalid_target() -> CitationAuthoringFailure {
    authoring_failure(
        CitationAuthoringDiagnosticCode::InvalidEditTarget,
        "citation edit range is not a valid UTF-8 source range".to_owned(),
        None,
    )
}

fn authoring_failure(
    code: CitationAuthoringDiagnosticCode,
    message: String,
    range: Option<Range<u64>>,
) -> CitationAuthoringFailure {
    CitationAuthoringFailure {
        code,
        message,
        range,
    }
}
