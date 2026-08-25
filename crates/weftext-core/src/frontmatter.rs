use std::fmt;
use std::ops::Range;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use weftext_asciidoc::{
    EnvelopeAdjacentHeadingBody, EnvelopeChildSort, EnvelopeChildSortDirection, EnvelopeField,
    EnvelopeFieldKind, EnvelopeFieldValue, EnvelopeIssue, EnvelopeIssueCode, EnvelopeIssueSeverity,
    EnvelopeSemantic, ManagedEnvelopePatch, ManagedEnvelopePatchError, analyze_managed_envelope,
    patch_managed_envelope,
};

use crate::{ChildSort, NodeId, SiblingOrder, SortDirection, SortMode};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjacentHeadingBody {
    #[default]
    Separate,
    RunIn,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PresentationSettings {
    pub adjacent_heading_body: AdjacentHeadingBody,
    pub adjacent_heading_body_explicit: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeMetadata {
    pub id: Option<NodeId>,
    pub child_sort: ChildSort,
    pub sibling_order: SiblingOrder,
    pub presentation: PresentationSettings,
}

pub const NODE_METADATA_PROJECTION_SCHEMA: &str = "weftext.node-metadata.v1";

/// The structural scope needed to interpret the root-only presentation field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeMetadataScope {
    WorkspaceRoot,
    Node,
}

/// A shell-safe, typed projection of the canonical shallow `weftext` envelope.
///
/// This is derived from exact source by Core. Product shells must consume this
/// model instead of parsing or normalizing YAML independently.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetadataProjection {
    pub schema: String,
    pub id: NodeId,
    pub icon: Option<String>,
    pub resolved_icon: Option<crate::ResolvedNodeIcon>,
    pub aliases: Vec<String>,
    pub child_sort: SortMode,
    pub child_sort_direction: SortDirection,
    pub sibling_rank: Option<u64>,
    pub adjacent_heading_body: Option<AdjacentHeadingBody>,
    pub diagnostics: Vec<FrontmatterDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontmatterDiagnosticCode {
    UnknownWeftextField,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmatterDiagnostic {
    pub code: FrontmatterDiagnosticCode,
    pub field: String,
    pub range: Range<u64>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontmatterError {
    Unclosed,
    MissingEnvelope,
    MissingWeftextMapping,
    DuplicateReservedKey,
    UnknownTopLevelKey(String),
    UnsupportedReservedYaml,
    InvalidId(crate::NodeIdError),
    InvalidIcon,
    InvalidAliases,
    InvalidSortMode,
    InvalidSortDirection,
    InvalidRank,
    InvalidAdjacentHeadingBody,
    WorkspaceSettingOutsideRoot,
    SiblingRankOnWorkspaceRoot,
    MissingIdentity,
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unclosed => formatter.write_str("frontmatter is not closed"),
            Self::MissingEnvelope => formatter.write_str("document YAML envelope is missing"),
            Self::MissingWeftextMapping => formatter
                .write_str("frontmatter must contain exactly one top-level `weftext` mapping"),
            Self::DuplicateReservedKey => {
                formatter.write_str("duplicate `weftext` mapping or field")
            }
            Self::UnknownTopLevelKey(key) => {
                write!(
                    formatter,
                    "unknown top-level YAML key `{key}`; only `weftext` is permitted"
                )
            }
            Self::UnsupportedReservedYaml => {
                formatter.write_str("unsupported or ambiguous `weftext` YAML shape")
            }
            Self::InvalidId(error) => write!(formatter, "invalid weftext.id: {error}"),
            Self::InvalidIcon => {
                formatter.write_str("weftext.icon must be one literal emoji or a Weftext token")
            }
            Self::InvalidAliases => {
                formatter.write_str("weftext.aliases must be an ordered list of literal strings")
            }
            Self::InvalidSortMode => {
                formatter.write_str("weftext.child_sort must be name or manual")
            }
            Self::InvalidSortDirection => {
                formatter.write_str("weftext.child_sort_direction must be ascending or descending")
            }
            Self::InvalidRank => {
                formatter.write_str("weftext.sibling_rank must be a positive integer")
            }
            Self::InvalidAdjacentHeadingBody => {
                formatter.write_str("weftext.adjacent_heading_body must be separate or run_in")
            }
            Self::WorkspaceSettingOutsideRoot => formatter
                .write_str("weftext.adjacent_heading_body is valid only on the workspace root"),
            Self::SiblingRankOnWorkspaceRoot => formatter
                .write_str("weftext.sibling_rank is valid only on a non-root node with a parent"),
            Self::MissingIdentity => formatter.write_str("node document is missing weftext.id"),
        }
    }
}

impl std::error::Error for FrontmatterError {}

/// Reads and validates the complete canonical Weftext envelope without
/// normalizing or reserializing YAML.
///
/// # Errors
///
/// Unknown top-level keys, duplicate fields, ambiguous known scalars, YAML
/// aliases/tags on known fields, and malformed known values fail closed.
/// Unknown inner fields are preserved and available through
/// [`parse_node_metadata_with_diagnostics`].
pub fn parse_node_metadata(source: &str) -> Result<NodeMetadata, FrontmatterError> {
    parse_node_metadata_with_diagnostics(source).map(|(metadata, _)| metadata)
}

/// Reads canonical metadata and reports preserved forward-compatible fields.
///
/// The returned diagnostics are non-fatal. Normal mutations operate on exact
/// ranges for known fields and therefore retain every byte in these opaque
/// future fields.
///
/// # Errors
///
/// Returns an error for an invalid canonical envelope or malformed known
/// field.
pub fn parse_node_metadata_with_diagnostics(
    source: &str,
) -> Result<(NodeMetadata, Vec<FrontmatterDiagnostic>), FrontmatterError> {
    let envelope = canonical_envelope(source)?;
    let metadata = metadata_from_envelope(&envelope)?;
    let diagnostics = envelope
        .fields
        .iter()
        .filter(|field| field.kind == EnvelopeFieldKind::Unknown)
        .map(|field| FrontmatterDiagnostic {
            code: FrontmatterDiagnosticCode::UnknownWeftextField,
            field: field.name.clone(),
            range: field.key_range.clone(),
            message: format!(
                "unknown `weftext` field `{}` is preserved but unsupported by this profile",
                field.name
            ),
        })
        .collect();
    Ok((metadata, diagnostics))
}

/// Projects the complete supported canonical node metadata without exposing a
/// YAML parser or a normalized frontmatter representation to product shells.
///
/// Unknown forward-compatible inner fields remain exact source and are
/// reported as diagnostics. A future valid `weftext:*` icon token is returned
/// literally while `resolved_icon` remains absent until the client supports it.
///
/// # Errors
///
/// Returns an error for an invalid envelope, missing identity, or a root-only
/// presentation setting on an ordinary node.
pub fn project_node_metadata(
    source: &str,
    scope: NodeMetadataScope,
) -> Result<NodeMetadataProjection, FrontmatterError> {
    let envelope = canonical_envelope(source)?;
    let metadata = metadata_from_envelope(&envelope)?;
    validate_node_metadata_scope(&metadata, scope)?;
    let id = metadata.id.ok_or(FrontmatterError::MissingIdentity)?;
    let icon = scalar_field(&envelope, EnvelopeFieldKind::Icon).map(ToOwned::to_owned);
    let aliases = string_list_field(&envelope, EnvelopeFieldKind::Aliases)?;
    let diagnostics = envelope
        .fields
        .iter()
        .filter(|field| field.kind == EnvelopeFieldKind::Unknown)
        .map(|field| FrontmatterDiagnostic {
            code: FrontmatterDiagnosticCode::UnknownWeftextField,
            field: field.name.clone(),
            range: field.key_range.clone(),
            message: format!(
                "unknown `weftext` field `{}` is preserved but unsupported by this profile",
                field.name
            ),
        })
        .collect();
    Ok(NodeMetadataProjection {
        schema: NODE_METADATA_PROJECTION_SCHEMA.to_owned(),
        resolved_icon: icon.as_deref().and_then(crate::resolve_node_icon),
        icon,
        aliases,
        id,
        child_sort: metadata.child_sort.mode,
        child_sort_direction: metadata.child_sort.direction,
        sibling_rank: metadata.sibling_order.rank,
        adjacent_heading_body: (scope == NodeMetadataScope::WorkspaceRoot)
            .then_some(metadata.presentation.adjacent_heading_body),
        diagnostics,
    })
}

pub(crate) fn validate_node_metadata_scope(
    metadata: &NodeMetadata,
    scope: NodeMetadataScope,
) -> Result<(), FrontmatterError> {
    if scope == NodeMetadataScope::Node && metadata.presentation.adjacent_heading_body_explicit {
        return Err(FrontmatterError::WorkspaceSettingOutsideRoot);
    }
    if scope == NodeMetadataScope::WorkspaceRoot && metadata.sibling_order.rank.is_some() {
        return Err(FrontmatterError::SiblingRankOnWorkspaceRoot);
    }
    Ok(())
}

pub(crate) fn parse_node_aliases(source: &str) -> Result<Vec<String>, FrontmatterError> {
    let envelope = canonical_envelope(source)?;
    string_list_field(&envelope, EnvelopeFieldKind::Aliases)
}

pub(crate) fn parse_node_icon_value(source: &str) -> Result<Option<String>, FrontmatterError> {
    let envelope = canonical_envelope(source)?;
    Ok(scalar_field(&envelope, EnvelopeFieldKind::Icon).map(ToOwned::to_owned))
}

fn metadata_from_envelope(envelope: &EnvelopeSemantic) -> Result<NodeMetadata, FrontmatterError> {
    let id = scalar_field(envelope, EnvelopeFieldKind::Id)
        .ok_or(FrontmatterError::MissingIdentity)
        .and_then(|value| NodeId::from_str(value).map_err(FrontmatterError::InvalidId))?;
    let mode = match scalar_field(envelope, EnvelopeFieldKind::ChildSort) {
        None | Some("name") => SortMode::Name,
        Some("manual") => SortMode::Manual,
        Some(_) => return Err(FrontmatterError::InvalidSortMode),
    };
    let direction = match scalar_field(envelope, EnvelopeFieldKind::ChildSortDirection) {
        None | Some("ascending") => SortDirection::Ascending,
        Some("descending") => SortDirection::Descending,
        Some(_) => return Err(FrontmatterError::InvalidSortDirection),
    };
    let rank = scalar_field(envelope, EnvelopeFieldKind::SiblingRank)
        .map(|value| {
            let parsed = value
                .parse::<u64>()
                .map_err(|_| FrontmatterError::InvalidRank)?;
            (parsed > 0)
                .then_some(parsed)
                .ok_or(FrontmatterError::InvalidRank)
        })
        .transpose()?;
    let adjacent = scalar_field(envelope, EnvelopeFieldKind::AdjacentHeadingBody);
    let adjacent_heading_body = match adjacent {
        None | Some("separate") => AdjacentHeadingBody::Separate,
        Some("run_in") => AdjacentHeadingBody::RunIn,
        Some(_) => return Err(FrontmatterError::InvalidAdjacentHeadingBody),
    };
    Ok(NodeMetadata {
        id: Some(id),
        child_sort: ChildSort { mode, direction },
        sibling_order: SiblingOrder { rank },
        presentation: PresentationSettings {
            adjacent_heading_body,
            adjacent_heading_body_explicit: adjacent.is_some(),
        },
    })
}

fn scalar_field(envelope: &EnvelopeSemantic, kind: EnvelopeFieldKind) -> Option<&str> {
    envelope
        .fields
        .iter()
        .find(|field| field.kind == kind)
        .and_then(|field| match &field.value {
            EnvelopeFieldValue::Scalar { value } => Some(value.as_str()),
            EnvelopeFieldValue::StringList { .. } | EnvelopeFieldValue::Opaque => None,
        })
}

fn string_list_field(
    envelope: &EnvelopeSemantic,
    kind: EnvelopeFieldKind,
) -> Result<Vec<String>, FrontmatterError> {
    match envelope.fields.iter().find(|field| field.kind == kind) {
        Some(EnvelopeField {
            value: EnvelopeFieldValue::StringList { items },
            ..
        }) => Ok(items.iter().map(|item| item.value.clone()).collect()),
        Some(_) => Err(FrontmatterError::InvalidAliases),
        None => Ok(Vec::new()),
    }
}

fn canonical_envelope(source: &str) -> Result<EnvelopeSemantic, FrontmatterError> {
    let analysis = analyze_managed_envelope(source);
    match analysis.probe.state {
        weftext_asciidoc::EnvelopeProbeState::Absent => {
            return Err(FrontmatterError::MissingEnvelope);
        }
        weftext_asciidoc::EnvelopeProbeState::Unclosed => {
            return Err(FrontmatterError::Unclosed);
        }
        weftext_asciidoc::EnvelopeProbeState::Closed => {}
    }
    let envelope = analysis
        .semantic
        .ok_or(FrontmatterError::UnsupportedReservedYaml)?;
    if let Some(issue) = envelope
        .issues
        .iter()
        .find(|issue| issue.severity == EnvelopeIssueSeverity::Error)
    {
        return Err(frontmatter_error_from_profile(source, &envelope, issue));
    }
    Ok(envelope)
}

fn frontmatter_error_from_profile(
    source: &str,
    envelope: &EnvelopeSemantic,
    issue: &EnvelopeIssue,
) -> FrontmatterError {
    match issue.code {
        EnvelopeIssueCode::MissingWeftextMapping => FrontmatterError::MissingWeftextMapping,
        EnvelopeIssueCode::MissingRequiredField => FrontmatterError::MissingIdentity,
        EnvelopeIssueCode::LegacyTopLevelKey | EnvelopeIssueCode::UnknownTopLevelKey => {
            FrontmatterError::UnknownTopLevelKey(exact_profile_range(source, &issue.range))
        }
        EnvelopeIssueCode::DuplicateTopLevelKey | EnvelopeIssueCode::DuplicateField => {
            FrontmatterError::DuplicateReservedKey
        }
        EnvelopeIssueCode::InvalidValue | EnvelopeIssueCode::InvalidStructure => {
            match profile_issue_field(envelope, issue).map(|field| field.kind) {
                Some(EnvelopeFieldKind::Id) => scalar_field(envelope, EnvelopeFieldKind::Id)
                    .and_then(|value| NodeId::from_str(value).err())
                    .map_or(
                        FrontmatterError::UnsupportedReservedYaml,
                        FrontmatterError::InvalidId,
                    ),
                Some(EnvelopeFieldKind::Icon) => FrontmatterError::InvalidIcon,
                Some(EnvelopeFieldKind::Aliases) => FrontmatterError::InvalidAliases,
                Some(EnvelopeFieldKind::ChildSort) => FrontmatterError::InvalidSortMode,
                Some(EnvelopeFieldKind::ChildSortDirection) => {
                    FrontmatterError::InvalidSortDirection
                }
                Some(EnvelopeFieldKind::SiblingRank) => FrontmatterError::InvalidRank,
                Some(EnvelopeFieldKind::AdjacentHeadingBody) => {
                    FrontmatterError::InvalidAdjacentHeadingBody
                }
                Some(EnvelopeFieldKind::Unknown) | None => {
                    FrontmatterError::UnsupportedReservedYaml
                }
            }
        }
        EnvelopeIssueCode::UnsafeYamlFeature | EnvelopeIssueCode::UnknownWeftextField => {
            FrontmatterError::UnsupportedReservedYaml
        }
    }
}

fn profile_issue_field<'a>(
    envelope: &'a EnvelopeSemantic,
    issue: &EnvelopeIssue,
) -> Option<&'a EnvelopeField> {
    envelope
        .fields
        .iter()
        .find(|field| issue.range.start < field.range.end && field.range.start < issue.range.end)
}

fn exact_profile_range(source: &str, range: &Range<u64>) -> String {
    let start = usize::try_from(range.start).ok();
    let end = usize::try_from(range.end).ok();
    start
        .zip(end)
        .and_then(|(start, end)| source.get(start..end))
        .unwrap_or("unknown")
        .to_owned()
}

#[must_use]
pub(crate) fn new_node_document(id: NodeId) -> String {
    weftext_asciidoc::new_managed_document_envelope(id.as_uuid())
        .expect("NodeId is always canonical UUIDv4")
}

pub(crate) fn replace_node_id(source: &str, id: NodeId) -> Result<String, FrontmatterError> {
    apply_profile_patch(
        source,
        ManagedEnvelopePatch::Id(id.as_uuid()),
        FrontmatterError::InvalidId(crate::NodeIdError::InvalidUuid),
    )
}

pub(crate) fn set_node_icon(source: &str, value: Option<&str>) -> Result<String, FrontmatterError> {
    apply_profile_patch(
        source,
        ManagedEnvelopePatch::Icon(value.map(ToOwned::to_owned)),
        FrontmatterError::InvalidIcon,
    )
}

pub(crate) fn set_node_aliases(
    source: &str,
    aliases: &[String],
) -> Result<String, FrontmatterError> {
    apply_profile_patch(
        source,
        ManagedEnvelopePatch::Aliases(aliases.to_vec()),
        FrontmatterError::InvalidAliases,
    )
}

pub(crate) fn set_node_child_sort(
    source: &str,
    value: ChildSort,
) -> Result<String, FrontmatterError> {
    let (mode, direction) = match (value.mode, value.direction) {
        (SortMode::Name, SortDirection::Ascending) => (None, None),
        (SortMode::Name, SortDirection::Descending) => (
            Some(EnvelopeChildSort::Name),
            Some(EnvelopeChildSortDirection::Descending),
        ),
        (SortMode::Manual, _) => (Some(EnvelopeChildSort::Manual), None),
    };
    let source = apply_profile_patch(
        source,
        ManagedEnvelopePatch::ChildSort(mode),
        FrontmatterError::InvalidSortMode,
    )?;
    apply_profile_patch(
        &source,
        ManagedEnvelopePatch::ChildSortDirection(direction),
        FrontmatterError::InvalidSortDirection,
    )
}

pub(crate) fn set_node_sibling_rank(
    source: &str,
    rank: Option<u64>,
) -> Result<String, FrontmatterError> {
    apply_profile_patch(
        source,
        ManagedEnvelopePatch::SiblingRank(rank),
        FrontmatterError::InvalidRank,
    )
}

pub(crate) fn set_adjacent_heading_body(
    source: &str,
    value: AdjacentHeadingBody,
) -> Result<String, FrontmatterError> {
    let value = match value {
        AdjacentHeadingBody::Separate => EnvelopeAdjacentHeadingBody::Separate,
        AdjacentHeadingBody::RunIn => EnvelopeAdjacentHeadingBody::RunIn,
    };
    apply_profile_patch(
        source,
        ManagedEnvelopePatch::AdjacentHeadingBody(Some(value)),
        FrontmatterError::InvalidAdjacentHeadingBody,
    )
}

fn apply_profile_patch(
    source: &str,
    patch: ManagedEnvelopePatch,
    invalid_value: FrontmatterError,
) -> Result<String, FrontmatterError> {
    let _ = canonical_envelope(source)?;
    patch_managed_envelope(source, patch).map_err(|error| match error {
        ManagedEnvelopePatchError::InvalidValue => invalid_value,
        ManagedEnvelopePatchError::MissingEnvelope => FrontmatterError::MissingEnvelope,
        ManagedEnvelopePatchError::UnclosedEnvelope => FrontmatterError::Unclosed,
        ManagedEnvelopePatchError::InvalidEnvelope
        | ManagedEnvelopePatchError::UnsupportedRange => FrontmatterError::UnsupportedReservedYaml,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn parses_the_flat_canonical_envelope() {
        let source = format!(
            "---\nweftext:\n  id: \"{ID}\"\n  icon: \"weftext:book\"\n  aliases:\n    - 文缕\n    - 'Weftext'\n  child_sort: manual\n  child_sort_direction: descending\n  sibling_rank: 2048\n  adjacent_heading_body: run_in\n---\n= Title\n"
        );
        let metadata = parse_node_metadata(&source).expect("metadata");
        assert_eq!(metadata.id.map(|id| id.to_string()), Some(ID.to_owned()));
        assert_eq!(metadata.child_sort.mode, SortMode::Manual);
        assert_eq!(metadata.child_sort.direction, SortDirection::Descending);
        assert_eq!(metadata.sibling_order.rank, Some(2048));
        assert_eq!(
            metadata.presentation.adjacent_heading_body,
            AdjacentHeadingBody::RunIn
        );
        assert!(metadata.presentation.adjacent_heading_body_explicit);
        assert_eq!(
            parse_node_aliases(&source),
            Ok(vec!["文缕".to_owned(), "Weftext".to_owned()])
        );
        assert_eq!(
            parse_node_icon_value(&source),
            Ok(Some("weftext:book".to_owned()))
        );
    }

    #[test]
    fn rejects_legacy_and_unknown_top_level_shapes() {
        assert!(matches!(
            parse_node_metadata(&format!("---\n_weftext:\n  id: \"{ID}\"\n---\n")),
            Err(FrontmatterError::UnknownTopLevelKey(_))
        ));
        assert!(matches!(
            parse_node_metadata(&format!(
                "---\nweftext:\n  id: \"{ID}\"\nother: value\n---\n"
            )),
            Err(FrontmatterError::UnknownTopLevelKey(_))
        ));
    }

    #[test]
    fn preserves_and_diagnoses_unknown_inner_fields() {
        let source = format!(
            "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  future:\r\n    nested: [opaque, bytes]\r\n  icon: 😀\r\n---\r\n= Title\r\n"
        );
        let (metadata, diagnostics) =
            parse_node_metadata_with_diagnostics(&source).expect("forward-compatible metadata");
        assert_eq!(metadata.id.map(|id| id.to_string()), Some(ID.to_owned()));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            FrontmatterDiagnosticCode::UnknownWeftextField
        );
        assert_eq!(diagnostics[0].field, "future");

        let patched = set_node_icon(&source, Some("weftext:book")).expect("narrow icon patch");
        assert!(patched.contains("  future:\r\n    nested: [opaque, bytes]\r\n"));
        assert_eq!(
            patched,
            source.replacen("  icon: 😀", "  icon: \"weftext:book\"", 1)
        );
    }

    #[test]
    fn scalar_patches_preserve_every_unrelated_byte() {
        let source = format!(
            "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  aliases:\r\n    - 文缕\r\n  icon: '😀'\r\n---\r\n= Title\r\n"
        );
        let icon = set_node_icon(&source, Some("weftext:book")).expect("icon");
        assert_eq!(icon, source.replacen("'😀'", "\"weftext:book\"", 1));
        let presentation =
            set_adjacent_heading_body(&icon, AdjacentHeadingBody::RunIn).expect("presentation");
        assert!(
            presentation
                .contains("  icon: \"weftext:book\"\r\n  adjacent_heading_body: run_in\r\n---")
        );
        let cleared = set_node_icon(&presentation, None).expect("clear");
        assert!(!cleared.contains("  icon:"));
        assert!(cleared.contains("  aliases:\r\n    - 文缕\r\n"));
    }

    #[test]
    fn alias_and_order_patches_are_narrow_and_preserve_unknown_fields() {
        let source = format!(
            "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  aliases: [old, bytes]\r\n  future:\r\n    opaque: [unchanged]\r\n  child_sort: name\r\n  child_sort_direction: ascending\r\n  sibling_rank: 1024\r\n---\r\n= Title\r\n"
        );
        let aliases =
            set_node_aliases(&source, &["文缕".to_owned(), "Quoted \"alias\"".to_owned()])
                .expect("aliases");
        assert!(
            aliases.contains("  aliases:\r\n    - \"文缕\"\r\n    - \"Quoted \\\"alias\\\"\"\r\n")
        );
        assert!(aliases.contains("  future:\r\n    opaque: [unchanged]\r\n"));

        let manual = set_node_child_sort(
            &aliases,
            ChildSort {
                mode: SortMode::Manual,
                direction: SortDirection::Descending,
            },
        )
        .expect("manual sort");
        assert!(manual.contains("  child_sort: manual\r\n"));
        assert!(!manual.contains("child_sort_direction"));

        let default = set_node_child_sort(&manual, ChildSort::default()).expect("default sort");
        assert!(!default.contains("child_sort:"));
        assert!(!default.contains("child_sort_direction:"));
        let cleared = set_node_sibling_rank(&default, None).expect("clear rank");
        assert!(!cleared.contains("sibling_rank:"));
        assert!(cleared.contains("  future:\r\n    opaque: [unchanged]\r\n"));
    }

    #[test]
    fn alias_and_rank_writes_reject_ambiguous_values() {
        let source = format!("---\nweftext:\n  id: \"{ID}\"\n---\n");
        assert_eq!(
            set_node_aliases(&source, &["same".to_owned(), "same".to_owned()]),
            Err(FrontmatterError::InvalidAliases)
        );
        assert_eq!(
            set_node_aliases(&source, &["line\nbreak".to_owned()]),
            Err(FrontmatterError::InvalidAliases)
        );
        assert_eq!(
            set_node_sibling_rank(&source, Some(0)),
            Err(FrontmatterError::InvalidRank)
        );
    }

    #[test]
    fn duplicate_and_invalid_values_fail_closed() {
        let duplicate = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: 😀\n  icon: 😺\n---\n");
        assert_eq!(
            parse_node_metadata(&duplicate),
            Err(FrontmatterError::DuplicateReservedKey)
        );
        let list_icon = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: [weftext:book, 😀]\n---\n");
        assert!(parse_node_metadata(&list_icon).is_err());
        let unknown_icon = format!("---\nweftext:\n  id: \"{ID}\"\n  icon: vendor:custom\n---\n");
        assert_eq!(
            parse_node_metadata(&unknown_icon),
            Err(FrontmatterError::InvalidIcon)
        );
    }
}
