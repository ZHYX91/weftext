use std::collections::HashSet;
use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    AdjacentHeadingBody, DocumentBlockKind, DocumentProfileId, DocumentRevision, NodeId,
    analyze_document_for_profile,
};

pub const ANNOTATION_STORE_VERSION: u32 = 3;
pub const MAX_ANNOTATION_STORE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ANNOTATIONS: usize = 10_000;
pub const MAX_MESSAGES_PER_ANNOTATION: usize = 1_000;
pub const MAX_TOTAL_MESSAGES: usize = 50_000;
pub const MAX_ANNOTATION_BODY_BYTES: usize = 64 * 1024;
pub const MAX_ANNOTATION_CONTEXT_BYTES: usize = 4 * 1024;
pub const MAX_ANNOTATION_LABELS: usize = 64;
pub const MAX_ANNOTATION_LABEL_BYTES: usize = 128;
pub const MAX_SUGGESTED_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_ANNOTATION_AUTHOR_NAME_BYTES: usize = 512;
const MAX_ANNOTATION_HEADING_DEPTH: usize = 32;
const MAX_ANNOTATION_TARGET_TEXT_BYTES: usize = 256 * 1024;
const MAX_RESOURCE_LOCATOR_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationKind {
    Comment,
    Mark,
    SuggestionInsert,
    SuggestionDelete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationMark {
    None,
    Highlight,
    Underline,
    Squiggle,
    Strike,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationColor {
    Yellow,
    Red,
    Green,
    Blue,
    Purple,
    Pink,
    Gray,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationState {
    Open,
    Resolved,
    Orphaned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationResolution {
    Resolved,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationResourceMediaKind {
    Image,
    Pdf,
    Audio,
    Video,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnnotationResourceRegion {
    Rect {
        page: Option<u32>,
        x_millionths: u32,
        y_millionths: u32,
        width_millionths: u32,
        height_millionths: u32,
    },
    TimeRange {
        start_milliseconds: u64,
        end_milliseconds: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnnotationAppearance {
    pub mark: AnnotationMark,
    #[serde(rename = "theme")]
    pub color: AnnotationColor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Anchor {
    TextRange {
        exact: String,
        prefix: String,
        suffix: String,
        start: u64,
        end: u64,
        base_revision: String,
        #[serde(default)]
        block_id: Option<String>,
        #[serde(default)]
        heading_path: Vec<String>,
    },
    InsertionPoint {
        prefix: String,
        suffix: String,
        position: u64,
        base_revision: String,
        #[serde(default)]
        block_id: Option<String>,
        #[serde(default)]
        heading_path: Vec<String>,
    },
    Block {
        exact: String,
        heading_path: Vec<String>,
        #[serde(default)]
        block_id: Option<String>,
        base_revision: String,
    },
    Document,
    ResourceRegion {
        resource_locator: String,
        resource_digest: String,
        media_kind: AnnotationResourceMediaKind,
        region: AnnotationResourceRegion,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadMessage {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub body: AnnotationBody,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AnnotationBodyFormat {
    #[serde(rename = "weftext.asciidoc.inline.v1")]
    AsciiDocInlineV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationBody {
    pub format: AnnotationBodyFormat,
    pub source: String,
}

impl AnnotationBody {
    #[must_use]
    pub fn asciidoc(source: String) -> Self {
        Self {
            format: AnnotationBodyFormat::AsciiDocInlineV1,
            source,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Annotation {
    pub id: Uuid,
    pub kind: AnnotationKind,
    pub target: Anchor,
    pub appearance: Option<AnnotationAppearance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_source: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub thread: Vec<ThreadMessage>,
    pub state: AnnotationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<AnnotationResolution>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationStore {
    pub version: u32,
    pub document_id: NodeId,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnnotationTargetIntent {
    Document,
    TextRange {
        start: u64,
        end: u64,
    },
    InsertionPoint {
        position: u64,
    },
    BlockAt {
        source_offset: u64,
    },
    ResourceRegion {
        resource_locator: String,
        resource_digest: String,
        media_kind: AnnotationResourceMediaKind,
        region: AnnotationResourceRegion,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnnotationAction {
    Create {
        kind: AnnotationKind,
        target: AnnotationTargetIntent,
        appearance: Option<AnnotationAppearance>,
        labels: Vec<String>,
        body_source: Option<String>,
        suggested_source: Option<String>,
        author_id: Uuid,
        author_name: String,
        timestamp: String,
    },
    Reply {
        annotation_id: Uuid,
        body_source: String,
        author_id: Uuid,
        author_name: String,
        timestamp: String,
    },
    EditMessage {
        annotation_id: Uuid,
        message_id: Uuid,
        body_source: String,
        author_id: Uuid,
        timestamp: String,
    },
    SetAppearance {
        annotation_id: Uuid,
        appearance: Option<AnnotationAppearance>,
        timestamp: String,
    },
    SetLabels {
        annotation_id: Uuid,
        labels: Vec<String>,
        timestamp: String,
    },
    SetResolved {
        annotation_id: Uuid,
        resolved: bool,
        timestamp: String,
    },
    Reanchor {
        annotation_id: Uuid,
        timestamp: String,
    },
    AcceptSuggestion {
        annotation_id: Uuid,
        timestamp: String,
    },
    RejectSuggestion {
        annotation_id: Uuid,
        timestamp: String,
    },
}

impl AnnotationStore {
    #[must_use]
    pub fn empty(document_id: NodeId) -> Self {
        Self {
            version: ANNOTATION_STORE_VERSION,
            document_id,
            annotations: Vec::new(),
        }
    }

    /// Parse and validate an annotation sidecar.
    ///
    /// # Errors
    ///
    /// Returns an error when the JSON is malformed or the decoded store violates
    /// the version, identity, anchor, digest, or comment invariants.
    pub fn from_json(source: &str) -> Result<Self, AnnotationValidationError> {
        if source.len() > MAX_ANNOTATION_STORE_BYTES {
            return Err(AnnotationValidationError::LimitExceeded(
                "annotation sidecar bytes",
            ));
        }
        let value: Value = serde_json::from_str(source)
            .map_err(|error| AnnotationValidationError::InvalidJson(error.to_string()))?;
        let version = value
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                AnnotationValidationError::InvalidJson(
                    "annotation version is missing or invalid".to_owned(),
                )
            })?;
        if version != u64::from(ANNOTATION_STORE_VERSION) {
            return Err(AnnotationValidationError::UnsupportedVersion(
                u32::try_from(version).unwrap_or(u32::MAX),
            ));
        }
        validate_serialized_ids(&value)?;
        let store: Self = serde_json::from_value(value)
            .map_err(|error| AnnotationValidationError::InvalidJson(error.to_string()))?;
        store.validate(store.document_id)?;
        Ok(store)
    }

    /// Validate and serialize the store as newline-terminated, indented JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when the store is invalid or serialization fails.
    pub fn to_pretty_json(&self) -> Result<String, AnnotationValidationError> {
        self.validate(self.document_id)?;
        let mut json = serde_json::to_string_pretty(self)
            .map_err(|error| AnnotationValidationError::InvalidJson(error.to_string()))?;
        json.push('\n');
        if json.len() > MAX_ANNOTATION_STORE_BYTES {
            return Err(AnnotationValidationError::LimitExceeded(
                "annotation sidecar bytes",
            ));
        }
        Ok(json)
    }

    /// Validate this sidecar against the node document that owns it.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported version, a document identity mismatch,
    /// duplicate IDs, invalid anchors or digests, or empty comment messages.
    pub fn validate(&self, expected_document: NodeId) -> Result<(), AnnotationValidationError> {
        if self.version != ANNOTATION_STORE_VERSION {
            return Err(AnnotationValidationError::UnsupportedVersion(self.version));
        }
        if self.document_id != expected_document {
            return Err(AnnotationValidationError::DocumentMismatch);
        }
        if self.annotations.len() > MAX_ANNOTATIONS {
            return Err(AnnotationValidationError::LimitExceeded("annotations"));
        }
        let mut ids = HashSet::new();
        let mut total_messages = 0_usize;
        for annotation in &self.annotations {
            validate_uuid(annotation.id)?;
            if !ids.insert(annotation.id) {
                return Err(AnnotationValidationError::DuplicateId(annotation.id));
            }
            validate_annotation(annotation)?;
            if annotation.thread.len() > MAX_MESSAGES_PER_ANNOTATION {
                return Err(AnnotationValidationError::LimitExceeded(
                    "messages per annotation",
                ));
            }
            total_messages = total_messages.saturating_add(annotation.thread.len());
            if total_messages > MAX_TOTAL_MESSAGES {
                return Err(AnnotationValidationError::LimitExceeded("total messages"));
            }
            for message in &annotation.thread {
                validate_uuid(message.id)?;
                if !ids.insert(message.id) {
                    return Err(AnnotationValidationError::DuplicateId(message.id));
                }
                if !valid_inline_body(&message.body.source) {
                    return Err(AnnotationValidationError::EmptyComment(message.id));
                }
                if !valid_uuid(message.author_id)
                    || message.author_name.trim().is_empty()
                    || message.author_name.len() > MAX_ANNOTATION_AUTHOR_NAME_BYTES
                    || message.author_name.contains(['\r', '\n', '\0'])
                {
                    return Err(AnnotationValidationError::InvalidActor(message.id));
                }
                if message.body.format != AnnotationBodyFormat::AsciiDocInlineV1 {
                    return Err(AnnotationValidationError::UnsupportedBodyFormat(message.id));
                }
                if !valid_explicit_offset_timestamp(&message.created_at)
                    || !valid_explicit_offset_timestamp(&message.updated_at)
                {
                    return Err(AnnotationValidationError::InvalidTimestamp(message.id));
                }
            }
            if !valid_explicit_offset_timestamp(&annotation.created_at)
                || !valid_explicit_offset_timestamp(&annotation.updated_at)
            {
                return Err(AnnotationValidationError::InvalidTimestamp(annotation.id));
            }
        }
        Ok(())
    }

    /// Rekeys a copied node's portable review identity and retargets source
    /// evidence to the copied document revision.
    pub(crate) fn rekey_for_copy(&mut self, document_id: NodeId, document_revision: &str) {
        self.document_id = document_id;
        for annotation in &mut self.annotations {
            annotation.id = Uuid::new_v4();
            for message in &mut annotation.thread {
                message.id = Uuid::new_v4();
            }
            set_target_revision(&mut annotation.target, document_revision);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnotationReanchorOutcome {
    Unchanged,
    Reanchored,
    Orphaned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationSuggestionEdit {
    pub range: Range<u64>,
    pub replacement: String,
}

/// Exact source geometry resolved from one revision-bound annotation target.
///
/// This is a mechanical migration primitive for Core transactions. It does not decide whether an
/// annotation is eligible to move, and its variants deliberately preserve the target kind.
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedAnnotationAnchor {
    TextRange { range: Range<u64> },
    InsertionPoint { position: u64 },
    Block { range: Range<u64> },
}

/// Fail-closed errors from exact annotation-anchor migration primitives.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnnotationAnchorMigrationError {
    InvalidAnchor,
    UnsupportedAnchor,
    SourceRevisionMismatch,
    AnchorRevisionMismatch,
    InvalidRange,
    ContextMismatch,
    NotFound,
    Ambiguous,
    TargetKindMismatch,
}

impl fmt::Display for AnnotationAnchorMigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAnchor => formatter.write_str("annotation anchor evidence is invalid"),
            Self::UnsupportedAnchor => {
                formatter.write_str("annotation anchor has no document source geometry")
            }
            Self::SourceRevisionMismatch => {
                formatter.write_str("document source differs from the supplied revision")
            }
            Self::AnchorRevisionMismatch => {
                formatter.write_str("annotation anchor is stale for the supplied revision")
            }
            Self::InvalidRange => {
                formatter.write_str("annotation anchor range is outside exact UTF-8 boundaries")
            }
            Self::ContextMismatch => {
                formatter.write_str("annotation anchor context differs from the exact source")
            }
            Self::NotFound => {
                formatter.write_str("annotation anchor has no exact semantic block match")
            }
            Self::Ambiguous => {
                formatter.write_str("annotation anchor has multiple exact semantic matches")
            }
            Self::TargetKindMismatch => {
                formatter.write_str("annotation destination geometry has a different target kind")
            }
        }
    }
}

impl std::error::Error for AnnotationAnchorMigrationError {}

/// Resolves one source-backed target against its exact authored revision.
///
/// Text ranges and insertion points are verified at their authored byte offsets; this function
/// never searches for a nearby or similar occurrence. Block targets must identify exactly one
/// parser block using all stored evidence. It never changes annotation state or orphans a target.
///
/// # Errors
///
/// Returns a typed error for stale revisions, malformed or non-UTF-8 ranges, changed context,
/// absent or ambiguous block evidence, and targets without document source geometry.
#[allow(dead_code)]
pub(crate) fn resolve_annotation_anchor_range(
    profile: DocumentProfileId,
    source: &str,
    revision: &DocumentRevision,
    target: &Anchor,
) -> Result<ResolvedAnnotationAnchor, AnnotationAnchorMigrationError> {
    require_exact_source_revision(source, revision)?;
    validate_anchor(target).map_err(|_| AnnotationAnchorMigrationError::InvalidAnchor)?;
    let blocks = exact_block_evidence(profile, source)?;
    match target {
        Anchor::TextRange {
            exact,
            prefix,
            suffix,
            start,
            end,
            base_revision,
            block_id,
            heading_path,
        } => {
            require_anchor_revision(base_revision, revision)?;
            let range = exact_source_range(source, *start, *end)?;
            if source[range.clone()] != *exact
                || source_prefix(source, range.start) != *prefix
                || source_suffix(source, range.end) != *suffix
            {
                return Err(AnnotationAnchorMigrationError::ContextMismatch);
            }
            require_unique_matching_containing_block(
                &blocks,
                &(*start..*end),
                block_id.as_deref(),
                heading_path,
            )?;
            Ok(ResolvedAnnotationAnchor::TextRange {
                range: *start..*end,
            })
        }
        Anchor::InsertionPoint {
            prefix,
            suffix,
            position,
            base_revision,
            block_id,
            heading_path,
        } => {
            require_anchor_revision(base_revision, revision)?;
            let authored_position = *position;
            let checked_position = exact_source_position(source, authored_position)?;
            if source_prefix(source, checked_position) != *prefix
                || source_suffix(source, checked_position) != *suffix
            {
                return Err(AnnotationAnchorMigrationError::ContextMismatch);
            }
            require_unique_matching_containing_block(
                &blocks,
                &(authored_position..authored_position),
                block_id.as_deref(),
                heading_path,
            )?;
            Ok(ResolvedAnnotationAnchor::InsertionPoint {
                position: authored_position,
            })
        }
        Anchor::Block {
            exact,
            heading_path,
            block_id,
            base_revision,
        } => {
            require_anchor_revision(base_revision, revision)?;
            let matches = blocks
                .iter()
                .filter(|candidate| {
                    candidate.evidence.exact == *exact
                        && candidate.evidence.block_id == *block_id
                        && candidate.evidence.heading_path == *heading_path
                })
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [candidate] => Ok(ResolvedAnnotationAnchor::Block {
                    range: candidate.range.clone(),
                }),
                [] => Err(AnnotationAnchorMigrationError::NotFound),
                _ => Err(AnnotationAnchorMigrationError::Ambiguous),
            }
        }
        Anchor::Document | Anchor::ResourceRegion { .. } => {
            Err(AnnotationAnchorMigrationError::UnsupportedAnchor)
        }
    }
}

/// Rebuilds a resolved source anchor at one caller-selected exact destination geometry.
///
/// The returned value is only a replacement [`Anchor`]. Annotation IDs, actors, messages,
/// timestamps, labels, appearance, and state remain outside this helper. The caller is responsible
/// for deciding which annotation should migrate and for mapping source to destination geometry.
///
/// # Errors
///
/// Returns a typed error unless the destination source/revision pair is exact, the source and
/// destination target kinds agree, and the destination belongs to exactly one semantic block.
#[allow(dead_code)]
pub(crate) fn rebuild_annotation_target_at_exact_range(
    profile: DocumentProfileId,
    destination_source: &str,
    destination_revision: &DocumentRevision,
    resolved_source: &ResolvedAnnotationAnchor,
    destination: &ResolvedAnnotationAnchor,
) -> Result<Anchor, AnnotationAnchorMigrationError> {
    require_exact_source_revision(destination_source, destination_revision)?;
    let blocks = exact_block_evidence(profile, destination_source)?;
    let target = match (resolved_source, destination) {
        (
            ResolvedAnnotationAnchor::TextRange {
                range: source_range,
            },
            ResolvedAnnotationAnchor::TextRange { range },
        ) => {
            require_nonempty_geometry(source_range)?;
            let checked = exact_source_range(destination_source, range.start, range.end)?;
            if checked.is_empty() {
                return Err(AnnotationAnchorMigrationError::InvalidRange);
            }
            let evidence = require_unique_containing_block(&blocks, range)?;
            Anchor::TextRange {
                exact: destination_source[checked.clone()].to_owned(),
                prefix: source_prefix(destination_source, checked.start),
                suffix: source_suffix(destination_source, checked.end),
                start: range.start,
                end: range.end,
                base_revision: destination_revision.as_str().to_owned(),
                block_id: evidence.block_id.clone(),
                heading_path: evidence.heading_path.clone(),
            }
        }
        (
            ResolvedAnnotationAnchor::InsertionPoint { .. },
            ResolvedAnnotationAnchor::InsertionPoint { position },
        ) => {
            let checked = exact_source_position(destination_source, *position)?;
            let evidence = require_unique_containing_block(&blocks, &(*position..*position))?;
            Anchor::InsertionPoint {
                prefix: source_prefix(destination_source, checked),
                suffix: source_suffix(destination_source, checked),
                position: *position,
                base_revision: destination_revision.as_str().to_owned(),
                block_id: evidence.block_id.clone(),
                heading_path: evidence.heading_path.clone(),
            }
        }
        (
            ResolvedAnnotationAnchor::Block {
                range: source_range,
            },
            ResolvedAnnotationAnchor::Block { range },
        ) => {
            require_nonempty_geometry(source_range)?;
            let candidate = require_unique_exact_block(&blocks, range)?;
            Anchor::Block {
                exact: candidate.exact.clone(),
                heading_path: candidate.heading_path.clone(),
                block_id: candidate.block_id.clone(),
                base_revision: destination_revision.as_str().to_owned(),
            }
        }
        _ => return Err(AnnotationAnchorMigrationError::TargetKindMismatch),
    };
    validate_anchor(&target).map_err(|_| AnnotationAnchorMigrationError::InvalidAnchor)?;
    Ok(target)
}

/// Converts one typed UI target into portable Core-owned evidence.
///
/// # Errors
///
/// Returns an error for non-UTF-8 boundaries, frontmatter targets, invalid
/// resource regions, or a range that does not belong to one semantic block.
pub fn build_annotation_target(
    profile: DocumentProfileId,
    source: &str,
    revision: &str,
    intent: &AnnotationTargetIntent,
) -> Result<Anchor, AnnotationValidationError> {
    validate_digest(revision)?;
    let target = match intent {
        AnnotationTargetIntent::Document => Anchor::Document,
        AnnotationTargetIntent::TextRange { start, end } => {
            let range = checked_source_range(source, *start, *end)?;
            if range.is_empty() {
                return Err(AnnotationValidationError::InvalidAnchor);
            }
            let evidence = block_evidence(profile, source, *start..*end)?;
            Anchor::TextRange {
                exact: source[range.clone()].to_owned(),
                prefix: source_prefix(source, range.start),
                suffix: source_suffix(source, range.end),
                start: *start,
                end: *end,
                base_revision: revision.to_owned(),
                block_id: evidence.block_id,
                heading_path: evidence.heading_path,
            }
        }
        AnnotationTargetIntent::InsertionPoint { position } => {
            let authored_position = *position;
            let position = checked_source_position(source, authored_position)?;
            let evidence = block_evidence(profile, source, authored_position..authored_position)?;
            Anchor::InsertionPoint {
                prefix: source_prefix(source, position),
                suffix: source_suffix(source, position),
                position: authored_position,
                base_revision: revision.to_owned(),
                block_id: evidence.block_id,
                heading_path: evidence.heading_path,
            }
        }
        AnnotationTargetIntent::BlockAt { source_offset } => {
            let evidence = block_evidence(profile, source, *source_offset..*source_offset)?;
            Anchor::Block {
                exact: evidence.exact,
                heading_path: evidence.heading_path,
                block_id: evidence.block_id,
                base_revision: revision.to_owned(),
            }
        }
        AnnotationTargetIntent::ResourceRegion {
            resource_locator,
            resource_digest,
            media_kind,
            region,
        } => Anchor::ResourceRegion {
            resource_locator: resource_locator.clone(),
            resource_digest: resource_digest.clone(),
            media_kind: *media_kind,
            region: region.clone(),
        },
    };
    validate_anchor(&target)?;
    Ok(target)
}

/// Reanchors one text target only when the current source yields one
/// deterministic match. Ambiguous or missing evidence becomes orphaned.
pub fn reanchor_annotation(
    annotation: &mut Annotation,
    profile: DocumentProfileId,
    source: &str,
    revision: &str,
) -> AnnotationReanchorOutcome {
    if target_revision(&annotation.target) == Some(revision)
        && target_matches_current_source(&annotation.target, source)
        && annotation.state != AnnotationState::Orphaned
    {
        return AnnotationReanchorOutcome::Unchanged;
    }
    let reanchored = reanchor_target(&mut annotation.target, profile, source, revision);
    if reanchored {
        if annotation.state == AnnotationState::Orphaned {
            annotation.state = AnnotationState::Open;
        }
        AnnotationReanchorOutcome::Reanchored
    } else if matches!(
        annotation.target,
        Anchor::Document | Anchor::ResourceRegion { .. }
    ) {
        AnnotationReanchorOutcome::Unchanged
    } else {
        annotation.state = AnnotationState::Orphaned;
        annotation.resolution = None;
        AnnotationReanchorOutcome::Orphaned
    }
}

fn target_matches_current_source(target: &Anchor, source: &str) -> bool {
    match target {
        Anchor::TextRange {
            exact,
            prefix,
            suffix,
            start,
            end,
            ..
        } => checked_source_range(source, *start, *end).is_ok_and(|range| {
            source[range.clone()] == *exact
                && source[..range.start].ends_with(prefix)
                && source[range.end..].starts_with(suffix)
        }),
        Anchor::InsertionPoint {
            prefix,
            suffix,
            position,
            ..
        } => checked_source_position(source, *position).is_ok_and(|position| {
            source[..position].ends_with(prefix) && source[position..].starts_with(suffix)
        }),
        Anchor::Block { .. } | Anchor::Document | Anchor::ResourceRegion { .. } => true,
    }
}

/// Resolves one open suggestion to an exact current-document edit.
///
/// # Errors
///
/// Returns an error when the annotation is not an open suggestion or its
/// target cannot be deterministically reanchored.
pub fn annotation_suggestion_edit(
    annotation: &mut Annotation,
    profile: DocumentProfileId,
    source: &str,
    revision: &str,
) -> Result<AnnotationSuggestionEdit, AnnotationValidationError> {
    if annotation.state != AnnotationState::Open {
        return Err(AnnotationValidationError::InvalidState);
    }
    if reanchor_annotation(annotation, profile, source, revision)
        == AnnotationReanchorOutcome::Orphaned
    {
        return Err(AnnotationValidationError::InvalidAnchor);
    }
    match (&annotation.kind, &annotation.target) {
        (AnnotationKind::SuggestionInsert, Anchor::InsertionPoint { position, .. }) => {
            Ok(AnnotationSuggestionEdit {
                range: *position..*position,
                replacement: annotation
                    .suggested_source
                    .clone()
                    .ok_or(AnnotationValidationError::InvalidSuggestion)?,
            })
        }
        (
            AnnotationKind::SuggestionDelete,
            Anchor::TextRange {
                start, end, exact, ..
            },
        ) => {
            let range = checked_source_range(source, *start, *end)?;
            if source[range] != *exact {
                return Err(AnnotationValidationError::InvalidAnchor);
            }
            Ok(AnnotationSuggestionEdit {
                range: *start..*end,
                replacement: String::new(),
            })
        }
        _ => Err(AnnotationValidationError::InvalidSuggestion),
    }
}

fn set_target_revision(target: &mut Anchor, revision: &str) {
    match target {
        Anchor::TextRange { base_revision, .. }
        | Anchor::InsertionPoint { base_revision, .. }
        | Anchor::Block { base_revision, .. } => revision.clone_into(base_revision),
        Anchor::Document | Anchor::ResourceRegion { .. } => {}
    }
}

fn target_revision(target: &Anchor) -> Option<&str> {
    match target {
        Anchor::TextRange { base_revision, .. }
        | Anchor::InsertionPoint { base_revision, .. }
        | Anchor::Block { base_revision, .. } => Some(base_revision),
        Anchor::Document | Anchor::ResourceRegion { .. } => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BlockEvidence {
    exact: String,
    block_id: Option<String>,
    heading_path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExactBlockEvidence {
    range: Range<u64>,
    evidence: BlockEvidence,
}

fn require_exact_source_revision(
    source: &str,
    revision: &DocumentRevision,
) -> Result<(), AnnotationAnchorMigrationError> {
    if DocumentRevision::from_source(source) == *revision {
        Ok(())
    } else {
        Err(AnnotationAnchorMigrationError::SourceRevisionMismatch)
    }
}

fn require_anchor_revision(
    base_revision: &str,
    revision: &DocumentRevision,
) -> Result<(), AnnotationAnchorMigrationError> {
    if base_revision == revision.as_str() {
        Ok(())
    } else {
        Err(AnnotationAnchorMigrationError::AnchorRevisionMismatch)
    }
}

fn exact_source_range(
    source: &str,
    start: u64,
    end: u64,
) -> Result<Range<usize>, AnnotationAnchorMigrationError> {
    let start = usize::try_from(start).map_err(|_| AnnotationAnchorMigrationError::InvalidRange)?;
    let end = usize::try_from(end).map_err(|_| AnnotationAnchorMigrationError::InvalidRange)?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(AnnotationAnchorMigrationError::InvalidRange);
    }
    Ok(start..end)
}

fn exact_source_position(
    source: &str,
    position: u64,
) -> Result<usize, AnnotationAnchorMigrationError> {
    let position =
        usize::try_from(position).map_err(|_| AnnotationAnchorMigrationError::InvalidRange)?;
    if position > source.len() || !source.is_char_boundary(position) {
        return Err(AnnotationAnchorMigrationError::InvalidRange);
    }
    Ok(position)
}

fn require_nonempty_geometry(range: &Range<u64>) -> Result<(), AnnotationAnchorMigrationError> {
    if range.start < range.end {
        Ok(())
    } else {
        Err(AnnotationAnchorMigrationError::InvalidRange)
    }
}

fn exact_block_evidence(
    profile: DocumentProfileId,
    source: &str,
) -> Result<Vec<ExactBlockEvidence>, AnnotationAnchorMigrationError> {
    let analysis =
        analyze_document_for_profile(profile, source, AdjacentHeadingBody::Separate).model;
    let mut heading_path = Vec::<(u8, String)>::new();
    let mut evidence = Vec::new();
    for block in analysis.blocks {
        if block.kind == DocumentBlockKind::Heading {
            let level = block.heading_level.unwrap_or(9);
            heading_path.retain(|(candidate, _)| *candidate < level);
            heading_path.push((level, block.text.clone()));
        }
        if block.kind == DocumentBlockKind::Frontmatter {
            continue;
        }
        exact_source_range(source, block.start, block.end)?;
        evidence.push(ExactBlockEvidence {
            range: block.start..block.end,
            evidence: BlockEvidence {
                exact: block.text,
                block_id: block.block_id,
                heading_path: heading_path.iter().map(|(_, text)| text.clone()).collect(),
            },
        });
    }
    Ok(evidence)
}

fn block_contains_range(block: &ExactBlockEvidence, range: &Range<u64>) -> bool {
    if range.start == range.end {
        block.range.start <= range.start && range.start <= block.range.end
    } else {
        block.range.start <= range.start && range.end <= block.range.end
    }
}

fn require_unique_matching_containing_block<'a>(
    blocks: &'a [ExactBlockEvidence],
    range: &Range<u64>,
    block_id: Option<&str>,
    heading_path: &[String],
) -> Result<&'a BlockEvidence, AnnotationAnchorMigrationError> {
    let matches = blocks
        .iter()
        .filter(|candidate| {
            block_contains_range(candidate, range)
                && candidate.evidence.block_id.as_deref() == block_id
                && candidate.evidence.heading_path == heading_path
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => Ok(&candidate.evidence),
        [] => Err(AnnotationAnchorMigrationError::ContextMismatch),
        _ => Err(AnnotationAnchorMigrationError::Ambiguous),
    }
}

fn require_unique_containing_block<'a>(
    blocks: &'a [ExactBlockEvidence],
    range: &Range<u64>,
) -> Result<&'a BlockEvidence, AnnotationAnchorMigrationError> {
    let matches = blocks
        .iter()
        .filter(|candidate| block_contains_range(candidate, range))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => Ok(&candidate.evidence),
        [] => Err(AnnotationAnchorMigrationError::ContextMismatch),
        _ => Err(AnnotationAnchorMigrationError::Ambiguous),
    }
}

fn require_unique_exact_block<'a>(
    blocks: &'a [ExactBlockEvidence],
    range: &Range<u64>,
) -> Result<&'a BlockEvidence, AnnotationAnchorMigrationError> {
    require_nonempty_geometry(range)?;
    let matches = blocks
        .iter()
        .filter(|candidate| candidate.range == *range)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => Ok(&candidate.evidence),
        [] => Err(AnnotationAnchorMigrationError::ContextMismatch),
        _ => Err(AnnotationAnchorMigrationError::Ambiguous),
    }
}

fn checked_source_range(
    source: &str,
    start: u64,
    end: u64,
) -> Result<Range<usize>, AnnotationValidationError> {
    let start = usize::try_from(start).map_err(|_| AnnotationValidationError::InvalidAnchor)?;
    let end = usize::try_from(end).map_err(|_| AnnotationValidationError::InvalidAnchor)?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(AnnotationValidationError::InvalidAnchor);
    }
    Ok(start..end)
}

fn checked_source_position(
    source: &str,
    position: u64,
) -> Result<usize, AnnotationValidationError> {
    let position =
        usize::try_from(position).map_err(|_| AnnotationValidationError::InvalidAnchor)?;
    if position > source.len() || !source.is_char_boundary(position) {
        return Err(AnnotationValidationError::InvalidAnchor);
    }
    Ok(position)
}

fn block_evidence(
    profile: DocumentProfileId,
    source: &str,
    range: Range<u64>,
) -> Result<BlockEvidence, AnnotationValidationError> {
    let analysis =
        analyze_document_for_profile(profile, source, AdjacentHeadingBody::Separate).model;
    let mut heading_path = Vec::<(u8, String)>::new();
    for block in analysis.blocks {
        if block.kind == DocumentBlockKind::Heading {
            let level = block.heading_level.unwrap_or(9);
            heading_path.retain(|(candidate, _)| *candidate < level);
            heading_path.push((level, block.text.clone()));
        }
        let contains = if range.start == range.end {
            block.start <= range.start && range.start <= block.end
        } else {
            block.start <= range.start && range.end <= block.end
        };
        if contains && block.kind != DocumentBlockKind::Frontmatter {
            return Ok(BlockEvidence {
                exact: block.text,
                block_id: block.block_id,
                heading_path: heading_path.into_iter().map(|(_, text)| text).collect(),
            });
        }
    }
    Err(AnnotationValidationError::InvalidAnchor)
}

fn source_prefix(source: &str, position: usize) -> String {
    let minimum = position.saturating_sub(MAX_ANNOTATION_CONTEXT_BYTES);
    let mut start = minimum;
    while start < position && !source.is_char_boundary(start) {
        start += 1;
    }
    source[start..position].to_owned()
}

fn source_suffix(source: &str, position: usize) -> String {
    let maximum = position
        .saturating_add(MAX_ANNOTATION_CONTEXT_BYTES)
        .min(source.len());
    let mut end = maximum;
    while end > position && !source.is_char_boundary(end) {
        end -= 1;
    }
    source[position..end].to_owned()
}

fn reanchor_target(
    target: &mut Anchor,
    profile: DocumentProfileId,
    source: &str,
    revision: &str,
) -> bool {
    if validate_digest(revision).is_err() {
        return false;
    }
    match target {
        Anchor::TextRange {
            exact,
            prefix,
            suffix,
            start,
            end,
            base_revision,
            block_id,
            heading_path,
        } => {
            let Some(range) = unique_text_target(
                profile,
                source,
                exact,
                prefix,
                suffix,
                block_id.as_deref(),
                heading_path,
            ) else {
                return false;
            };
            *start = u64::try_from(range.start).unwrap_or(u64::MAX);
            *end = u64::try_from(range.end).unwrap_or(u64::MAX);
            *prefix = source_prefix(source, range.start);
            *suffix = source_suffix(source, range.end);
            if let Ok(evidence) = block_evidence(profile, source, *start..*end) {
                *block_id = evidence.block_id;
                *heading_path = evidence.heading_path;
            }
            revision.clone_into(base_revision);
            true
        }
        Anchor::InsertionPoint {
            prefix,
            suffix,
            position,
            base_revision,
            block_id,
            heading_path,
        } => {
            let Some(next) = unique_insertion_target(source, prefix, suffix) else {
                return false;
            };
            *position = u64::try_from(next).unwrap_or(u64::MAX);
            *prefix = source_prefix(source, next);
            *suffix = source_suffix(source, next);
            if let Ok(evidence) = block_evidence(profile, source, *position..*position) {
                *block_id = evidence.block_id;
                *heading_path = evidence.heading_path;
            }
            revision.clone_into(base_revision);
            true
        }
        Anchor::Block {
            exact,
            heading_path,
            block_id,
            base_revision,
        } => {
            let Some(evidence) =
                unique_block_target(profile, source, exact, block_id.as_deref(), heading_path)
            else {
                return false;
            };
            *exact = evidence.exact;
            *block_id = evidence.block_id;
            *heading_path = evidence.heading_path;
            revision.clone_into(base_revision);
            true
        }
        Anchor::Document | Anchor::ResourceRegion { .. } => false,
    }
}

fn unique_text_target(
    profile: DocumentProfileId,
    source: &str,
    exact: &str,
    prefix: &str,
    suffix: &str,
    block_id: Option<&str>,
    heading_path: &[String],
) -> Option<Range<usize>> {
    let candidates = source
        .match_indices(exact)
        .map(|(start, value)| start..start + value.len())
        .collect::<Vec<_>>();
    let contextual = candidates
        .iter()
        .filter(|range| {
            source[..range.start].ends_with(prefix) && source[range.end..].starts_with(suffix)
        })
        .cloned()
        .collect::<Vec<_>>();
    if contextual.len() == 1 {
        return contextual.into_iter().next();
    }
    let evidenced = candidates
        .iter()
        .filter(|range| {
            let start = u64::try_from(range.start).unwrap_or(u64::MAX);
            let end = u64::try_from(range.end).unwrap_or(u64::MAX);
            block_evidence(profile, source, start..end).is_ok_and(|evidence| {
                block_id.is_some_and(|expected| evidence.block_id.as_deref() == Some(expected))
                    || (!heading_path.is_empty() && evidence.heading_path == heading_path)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    if evidenced.len() == 1 {
        evidenced.into_iter().next()
    } else if candidates.len() == 1 {
        candidates.into_iter().next()
    } else {
        None
    }
}

fn unique_insertion_target(source: &str, prefix: &str, suffix: &str) -> Option<usize> {
    let needle = format!("{prefix}{suffix}");
    if needle.is_empty() {
        return None;
    }
    let mut matches = source.match_indices(&needle);
    let first = matches.next()?.0 + prefix.len();
    matches.next().is_none().then_some(first)
}

fn unique_block_target(
    profile: DocumentProfileId,
    source: &str,
    exact: &str,
    block_id: Option<&str>,
    heading_path: &[String],
) -> Option<BlockEvidence> {
    let analysis =
        analyze_document_for_profile(profile, source, AdjacentHeadingBody::Separate).model;
    let mut current_path = Vec::<(u8, String)>::new();
    let mut matches = Vec::new();
    for block in analysis.blocks {
        if block.kind == DocumentBlockKind::Heading {
            let level = block.heading_level.unwrap_or(9);
            current_path.retain(|(candidate, _)| *candidate < level);
            current_path.push((level, block.text.clone()));
        }
        if block.kind == DocumentBlockKind::Frontmatter {
            continue;
        }
        let candidate_path = current_path
            .iter()
            .map(|(_, text)| text.clone())
            .collect::<Vec<_>>();
        if block_id.is_some_and(|expected| block.block_id.as_deref() == Some(expected))
            || (block.text == exact && candidate_path == heading_path)
        {
            matches.push(BlockEvidence {
                exact: block.text,
                block_id: block.block_id,
                heading_path: candidate_path,
            });
        }
    }
    (matches.len() == 1).then(|| matches.remove(0))
}

fn validate_serialized_ids(value: &Value) -> Result<(), AnnotationValidationError> {
    let annotations = value
        .get("annotations")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AnnotationValidationError::InvalidJson(
                "annotation store requires an annotations array".to_owned(),
            )
        })?;
    for annotation in annotations {
        validate_serialized_uuid(annotation.get("id"))?;
        let thread = annotation
            .get("thread")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AnnotationValidationError::InvalidJson(
                    "annotation requires a thread array".to_owned(),
                )
            })?;
        for message in thread {
            validate_serialized_uuid(message.get("id"))?;
            validate_serialized_uuid(message.get("author_id"))?;
        }
    }
    Ok(())
}

fn validate_serialized_uuid(value: Option<&Value>) -> Result<(), AnnotationValidationError> {
    let value = value.and_then(Value::as_str).ok_or_else(|| {
        AnnotationValidationError::InvalidJson("annotation UUID is missing or invalid".to_owned())
    })?;
    let parsed = Uuid::parse_str(value).map_err(|_| AnnotationValidationError::InvalidUuid)?;
    if parsed.to_string() != value || !valid_uuid(parsed) {
        return Err(AnnotationValidationError::InvalidUuid);
    }
    Ok(())
}

fn validate_uuid(value: Uuid) -> Result<(), AnnotationValidationError> {
    if valid_uuid(value) {
        Ok(())
    } else {
        Err(AnnotationValidationError::InvalidUuid)
    }
}

fn valid_uuid(value: Uuid) -> bool {
    value.get_version_num() == 4 && value.get_variant() == uuid::Variant::RFC4122
}

fn validate_annotation(annotation: &Annotation) -> Result<(), AnnotationValidationError> {
    validate_anchor(&annotation.target)?;
    validate_labels(&annotation.labels)?;
    if annotation
        .appearance
        .is_some_and(|appearance| appearance.mark == AnnotationMark::None)
    {
        return Err(AnnotationValidationError::InvalidKind);
    }
    match annotation.kind {
        AnnotationKind::Comment => {
            if annotation.thread.is_empty() || annotation.suggested_source.is_some() {
                return Err(AnnotationValidationError::InvalidKind);
            }
        }
        AnnotationKind::Mark => {
            if annotation.appearance.is_none() || annotation.suggested_source.is_some() {
                return Err(AnnotationValidationError::InvalidKind);
            }
        }
        AnnotationKind::SuggestionInsert => {
            if !matches!(annotation.target, Anchor::InsertionPoint { .. })
                || !annotation
                    .suggested_source
                    .as_deref()
                    .is_some_and(valid_suggested_source)
            {
                return Err(AnnotationValidationError::InvalidSuggestion);
            }
        }
        AnnotationKind::SuggestionDelete => {
            if !matches!(annotation.target, Anchor::TextRange { .. })
                || annotation.suggested_source.is_some()
            {
                return Err(AnnotationValidationError::InvalidSuggestion);
            }
        }
    }
    match annotation.state {
        AnnotationState::Open | AnnotationState::Orphaned if annotation.resolution.is_some() => {
            return Err(AnnotationValidationError::InvalidState);
        }
        AnnotationState::Resolved if annotation.resolution.is_none() => {
            return Err(AnnotationValidationError::InvalidState);
        }
        AnnotationState::Open | AnnotationState::Resolved | AnnotationState::Orphaned => {}
    }
    Ok(())
}

fn validate_labels(labels: &[String]) -> Result<(), AnnotationValidationError> {
    if labels.len() > MAX_ANNOTATION_LABELS {
        return Err(AnnotationValidationError::LimitExceeded(
            "annotation labels",
        ));
    }
    let mut seen = HashSet::new();
    for label in labels {
        if label.is_empty()
            || label.trim() != label
            || label.len() > MAX_ANNOTATION_LABEL_BYTES
            || label.contains(['\r', '\n', '\0'])
            || !seen.insert(label)
        {
            return Err(AnnotationValidationError::InvalidLabel);
        }
    }
    Ok(())
}

fn valid_inline_body(source: &str) -> bool {
    !source.trim().is_empty()
        && source.len() <= MAX_ANNOTATION_BODY_BYTES
        && !source.contains(['\r', '\n', '\0'])
}

fn valid_suggested_source(source: &str) -> bool {
    !source.is_empty() && source.len() <= MAX_SUGGESTED_SOURCE_BYTES && !source.contains('\0')
}

fn validate_anchor(anchor: &Anchor) -> Result<(), AnnotationValidationError> {
    match anchor {
        Anchor::TextRange {
            exact,
            prefix,
            suffix,
            start,
            end,
            base_revision,
            block_id,
            heading_path,
            ..
        } => {
            if exact.is_empty()
                || exact.len() > MAX_ANNOTATION_TARGET_TEXT_BYTES
                || start >= end
                || end.saturating_sub(*start) != u64::try_from(exact.len()).unwrap_or(u64::MAX)
                || !valid_context(prefix, suffix, block_id.as_deref(), heading_path)
            {
                return Err(AnnotationValidationError::InvalidAnchor);
            }
            validate_digest(base_revision)
        }
        Anchor::InsertionPoint {
            prefix,
            suffix,
            base_revision,
            block_id,
            heading_path,
            ..
        } => {
            if !valid_context(prefix, suffix, block_id.as_deref(), heading_path) {
                return Err(AnnotationValidationError::InvalidAnchor);
            }
            validate_digest(base_revision)
        }
        Anchor::Block {
            exact,
            heading_path,
            block_id,
            base_revision,
        } => {
            if exact.is_empty()
                || exact.len() > MAX_ANNOTATION_TARGET_TEXT_BYTES
                || !valid_context("", "", block_id.as_deref(), heading_path)
            {
                return Err(AnnotationValidationError::InvalidAnchor);
            }
            validate_digest(base_revision)
        }
        Anchor::Document => Ok(()),
        Anchor::ResourceRegion {
            resource_locator,
            resource_digest,
            media_kind,
            region,
        } => {
            validate_resource_locator(resource_locator)?;
            validate_digest(resource_digest)?;
            validate_resource_region(*media_kind, region)
        }
    }
}

fn valid_context(
    prefix: &str,
    suffix: &str,
    block_id: Option<&str>,
    heading_path: &[String],
) -> bool {
    prefix.len() <= MAX_ANNOTATION_CONTEXT_BYTES
        && suffix.len() <= MAX_ANNOTATION_CONTEXT_BYTES
        && !prefix.contains('\0')
        && !suffix.contains('\0')
        && block_id.is_none_or(|value| {
            !value.is_empty()
                && value.len() <= MAX_ANNOTATION_LABEL_BYTES
                && !value.contains(['\r', '\n', '\0'])
        })
        && heading_path.len() <= MAX_ANNOTATION_HEADING_DEPTH
        && heading_path.iter().all(|value| {
            !value.is_empty()
                && value.len() <= MAX_ANNOTATION_CONTEXT_BYTES
                && !value.contains('\0')
        })
}

fn validate_resource_locator(value: &str) -> Result<(), AnnotationValidationError> {
    if value.is_empty()
        || value.len() > MAX_RESOURCE_LOCATOR_BYTES
        || value.starts_with(['/', '\\'])
        || value.contains(['\\', '\0'])
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || value
            .as_bytes()
            .get(1)
            .is_some_and(|character| *character == b':')
    {
        return Err(AnnotationValidationError::InvalidResourceTarget);
    }
    Ok(())
}

fn validate_resource_region(
    media_kind: AnnotationResourceMediaKind,
    region: &AnnotationResourceRegion,
) -> Result<(), AnnotationValidationError> {
    match (media_kind, region) {
        (
            AnnotationResourceMediaKind::Image | AnnotationResourceMediaKind::Pdf,
            AnnotationResourceRegion::Rect {
                page,
                x_millionths,
                y_millionths,
                width_millionths,
                height_millionths,
            },
        ) => {
            let page_valid = match media_kind {
                AnnotationResourceMediaKind::Pdf => page.is_some_and(|value| value > 0),
                AnnotationResourceMediaKind::Image => page.is_none(),
                AnnotationResourceMediaKind::Audio | AnnotationResourceMediaKind::Video => false,
            };
            if !page_valid
                || *width_millionths == 0
                || *height_millionths == 0
                || x_millionths.saturating_add(*width_millionths) > 1_000_000
                || y_millionths.saturating_add(*height_millionths) > 1_000_000
            {
                return Err(AnnotationValidationError::InvalidResourceTarget);
            }
            Ok(())
        }
        (
            AnnotationResourceMediaKind::Audio | AnnotationResourceMediaKind::Video,
            AnnotationResourceRegion::TimeRange {
                start_milliseconds,
                end_milliseconds,
            },
        ) if start_milliseconds < end_milliseconds => Ok(()),
        _ => Err(AnnotationValidationError::InvalidResourceTarget),
    }
}

fn validate_digest(value: &str) -> Result<(), AnnotationValidationError> {
    let digest = value.strip_prefix("sha256:").unwrap_or(value);
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(AnnotationValidationError::InvalidDigest)
    }
}

fn valid_explicit_offset_timestamp(value: &str) -> bool {
    if !value.is_ascii() || value.len() < 20 || value.as_bytes().get(10) != Some(&b'T') {
        return false;
    }
    let date = &value[..10];
    if date.as_bytes().get(4) != Some(&b'-') || date.as_bytes().get(7) != Some(&b'-') {
        return false;
    }
    let Ok(year) = date[..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = date[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = date[8..10].parse::<u8>() else {
        return false;
    };
    if crate::CalendarDate::new(year, month, day).is_err() {
        return false;
    }
    let time = &value[11..];
    if time.len() < 9
        || time.as_bytes().get(2) != Some(&b':')
        || time.as_bytes().get(5) != Some(&b':')
    {
        return false;
    }
    let Ok(hour) = time[..2].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = time[3..5].parse::<u8>() else {
        return false;
    };
    let Ok(second) = time[6..8].parse::<u8>() else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut offset_start = 8;
    if time.as_bytes().get(offset_start) == Some(&b'.') {
        offset_start += 1;
        let fraction_start = offset_start;
        while time
            .as_bytes()
            .get(offset_start)
            .is_some_and(u8::is_ascii_digit)
        {
            offset_start += 1;
        }
        if offset_start == fraction_start {
            return false;
        }
    }
    if time.get(offset_start..) == Some("Z") {
        return true;
    }
    let Some(offset) = time.get(offset_start..) else {
        return false;
    };
    if offset.len() != 6
        || !matches!(offset.as_bytes()[0], b'+' | b'-')
        || offset.as_bytes()[3] != b':'
    {
        return false;
    }
    let Ok(offset_hour) = offset[1..3].parse::<u8>() else {
        return false;
    };
    let Ok(offset_minute) = offset[4..6].parse::<u8>() else {
        return false;
    };
    offset_hour <= 23 && offset_minute <= 59
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnnotationValidationError {
    InvalidJson(String),
    UnsupportedVersion(u32),
    LimitExceeded(&'static str),
    DocumentMismatch,
    InvalidUuid,
    DuplicateId(Uuid),
    EmptyComment(Uuid),
    InvalidActor(Uuid),
    InvalidTimestamp(Uuid),
    UnsupportedBodyFormat(Uuid),
    InvalidKind,
    InvalidSuggestion,
    InvalidState,
    InvalidLabel,
    InvalidAnchor,
    InvalidDigest,
    InvalidResourceTarget,
}

impl fmt::Display for AnnotationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(formatter, "invalid annotation JSON: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported annotation version {version}")
            }
            Self::LimitExceeded(limit) => {
                write!(formatter, "annotation {limit} exceeds the v3 limit")
            }
            Self::DocumentMismatch => {
                formatter.write_str("annotation document_id does not match the node")
            }
            Self::InvalidUuid => formatter
                .write_str("annotation and message IDs must be lowercase RFC 4122 UUIDv4 values"),
            Self::DuplicateId(id) => write!(formatter, "duplicate annotation or message ID {id}"),
            Self::EmptyComment(id) => write!(formatter, "annotation message {id} is empty"),
            Self::InvalidActor(id) => {
                write!(
                    formatter,
                    "annotation message {id} has no stable actor identity"
                )
            }
            Self::InvalidTimestamp(id) => {
                write!(
                    formatter,
                    "annotation or message {id} has an invalid offset timestamp"
                )
            }
            Self::UnsupportedBodyFormat(id) => write!(
                formatter,
                "annotation message {id} does not use weftext.asciidoc.inline.v1"
            ),
            Self::InvalidKind => {
                formatter.write_str("annotation kind, appearance, and thread are inconsistent")
            }
            Self::InvalidSuggestion => {
                formatter.write_str("annotation suggestion kind, target, or source is invalid")
            }
            Self::InvalidState => {
                formatter.write_str("annotation state and resolution are inconsistent")
            }
            Self::InvalidLabel => formatter.write_str("annotation label is invalid or duplicated"),
            Self::InvalidAnchor => formatter.write_str("annotation anchor is invalid"),
            Self::InvalidDigest => formatter.write_str("annotation source digest is invalid"),
            Self::InvalidResourceTarget => {
                formatter.write_str("annotation resource target is invalid")
            }
        }
    }
}

impl std::error::Error for AnnotationValidationError {}

#[cfg(test)]
mod exact_anchor_migration_tests {
    use super::*;

    fn byte_offset(source: &str, needle: &str) -> u64 {
        u64::try_from(source.find(needle).expect("needle in source")).unwrap()
    }

    fn last_byte_offset(source: &str, needle: &str) -> u64 {
        u64::try_from(source.rfind(needle).expect("needle in source")).unwrap()
    }

    #[test]
    fn exact_text_and_zero_width_points_preserve_crlf_unicode_and_repeated_offsets() {
        let source = concat!(
            "= 标题\r\n",
            "\r\n",
            "== مراجعة\r\n",
            "\r\n",
            "段落🙂 target שלום。\r\n",
            "\r\n",
            "段落🙂 target שלום。\r\n",
        );
        let revision = DocumentRevision::from_source(source);
        let start = last_byte_offset(source, "target");
        let target = build_annotation_target(
            DocumentProfileId::AsciiDocV1,
            source,
            revision.as_str(),
            &AnnotationTargetIntent::TextRange {
                start,
                end: start + 6,
            },
        )
        .unwrap();
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &revision,
                &target,
            ),
            Ok(ResolvedAnnotationAnchor::TextRange {
                range: start..start + 6,
            })
        );

        let position = last_byte_offset(source, "שלום");
        let point = build_annotation_target(
            DocumentProfileId::AsciiDocV1,
            source,
            revision.as_str(),
            &AnnotationTargetIntent::InsertionPoint { position },
        )
        .unwrap();
        let resolved_point = resolve_annotation_anchor_range(
            DocumentProfileId::AsciiDocV1,
            source,
            &revision,
            &point,
        );
        assert_eq!(
            resolved_point,
            Ok(ResolvedAnnotationAnchor::InsertionPoint { position })
        );

        let destination_source = "= Moved\r\n\r\n== مراجعة\r\n\r\nBefore🙂 after שלום.\r\n";
        let destination_revision = DocumentRevision::from_source(destination_source);
        let destination_position = byte_offset(destination_source, "after");
        let destination = ResolvedAnnotationAnchor::InsertionPoint {
            position: destination_position,
        };
        let rebuilt = rebuild_annotation_target_at_exact_range(
            DocumentProfileId::AsciiDocV1,
            destination_source,
            &destination_revision,
            resolved_point.as_ref().unwrap(),
            &destination,
        )
        .unwrap();
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                destination_source,
                &destination_revision,
                &rebuilt,
            ),
            Ok(destination)
        );
    }

    #[test]
    fn rebuild_returns_only_a_new_exact_target_for_the_same_geometry_kind() {
        let source = "= Source\n\n== Review\n\nKeep old target here.\n";
        let revision = DocumentRevision::from_source(source);
        let start = byte_offset(source, "old target");
        let source_target = build_annotation_target(
            DocumentProfileId::AsciiDocV1,
            source,
            revision.as_str(),
            &AnnotationTargetIntent::TextRange {
                start,
                end: start + 10,
            },
        )
        .unwrap();
        let resolved_source = resolve_annotation_anchor_range(
            DocumentProfileId::AsciiDocV1,
            source,
            &revision,
            &source_target,
        )
        .unwrap();

        let destination_source =
            "= Destination\r\n\r\n== Review\r\n\r\n移动🙂 exact target שלום.\r\n";
        let destination_revision = DocumentRevision::from_source(destination_source);
        let destination_start = byte_offset(destination_source, "exact target");
        let destination = ResolvedAnnotationAnchor::TextRange {
            range: destination_start..destination_start + 12,
        };
        let rebuilt = rebuild_annotation_target_at_exact_range(
            DocumentProfileId::AsciiDocV1,
            destination_source,
            &destination_revision,
            &resolved_source,
            &destination,
        )
        .unwrap();
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                destination_source,
                &destination_revision,
                &rebuilt,
            ),
            Ok(destination.clone())
        );
        assert!(matches!(
            rebuilt,
            Anchor::TextRange {
                exact,
                base_revision,
                ..
            } if exact == "exact target" && base_revision == destination_revision.as_str()
        ));

        assert_eq!(
            rebuild_annotation_target_at_exact_range(
                DocumentProfileId::AsciiDocV1,
                destination_source,
                &destination_revision,
                &resolved_source,
                &ResolvedAnnotationAnchor::InsertionPoint {
                    position: destination_start,
                },
            ),
            Err(AnnotationAnchorMigrationError::TargetKindMismatch)
        );
        assert_eq!(
            rebuild_annotation_target_at_exact_range(
                DocumentProfileId::AsciiDocV1,
                destination_source,
                &destination_revision,
                &resolved_source,
                &ResolvedAnnotationAnchor::TextRange {
                    range: destination_start..destination_start,
                },
            ),
            Err(AnnotationAnchorMigrationError::InvalidRange)
        );
    }

    #[test]
    fn exact_block_resolution_rejects_ambiguity_and_non_block_boundaries() {
        let source = "= Note\n\n== Unique\n\n[#stable]\nOnly block.\n";
        let revision = DocumentRevision::from_source(source);
        let target = build_annotation_target(
            DocumentProfileId::AsciiDocV1,
            source,
            revision.as_str(),
            &AnnotationTargetIntent::BlockAt {
                source_offset: byte_offset(source, "Only block"),
            },
        )
        .unwrap();
        let resolved = resolve_annotation_anchor_range(
            DocumentProfileId::AsciiDocV1,
            source,
            &revision,
            &target,
        )
        .unwrap();
        let ResolvedAnnotationAnchor::Block { range } = &resolved else {
            panic!("block target must resolve to block geometry");
        };
        assert!(range.start < range.end);

        let rebuilt = rebuild_annotation_target_at_exact_range(
            DocumentProfileId::AsciiDocV1,
            source,
            &revision,
            &resolved,
            &resolved,
        )
        .unwrap();
        assert_eq!(rebuilt, target);
        assert_eq!(
            rebuild_annotation_target_at_exact_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &revision,
                &resolved,
                &ResolvedAnnotationAnchor::Block {
                    range: range.start + 1..range.end,
                },
            ),
            Err(AnnotationAnchorMigrationError::ContextMismatch)
        );

        let ambiguous_source = "= Note\n\n== Same\n\nRepeated block.\n\nRepeated block.\n";
        let ambiguous_revision = DocumentRevision::from_source(ambiguous_source);
        let ambiguous_target = Anchor::Block {
            exact: "Repeated block.".to_owned(),
            heading_path: vec!["Same".to_owned()],
            block_id: None,
            base_revision: ambiguous_revision.as_str().to_owned(),
        };
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                ambiguous_source,
                &ambiguous_revision,
                &ambiguous_target,
            ),
            Err(AnnotationAnchorMigrationError::Ambiguous)
        );
    }

    #[test]
    fn stale_tampered_out_of_bounds_and_non_utf8_evidence_fail_closed() {
        let source = "= Note\n\n== Review\n\n前🙂 target אחר.\n";
        let revision = DocumentRevision::from_source(source);
        let start = byte_offset(source, "target");
        let target = build_annotation_target(
            DocumentProfileId::AsciiDocV1,
            source,
            revision.as_str(),
            &AnnotationTargetIntent::TextRange {
                start,
                end: start + 6,
            },
        )
        .unwrap();

        let stale_source_revision = DocumentRevision::from_source("different");
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &stale_source_revision,
                &target,
            ),
            Err(AnnotationAnchorMigrationError::SourceRevisionMismatch)
        );

        let mut stale_anchor = target.clone();
        let Anchor::TextRange { base_revision, .. } = &mut stale_anchor else {
            unreachable!();
        };
        *base_revision = "a".repeat(64);
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &revision,
                &stale_anchor,
            ),
            Err(AnnotationAnchorMigrationError::AnchorRevisionMismatch)
        );

        let mut tampered_context = target.clone();
        let Anchor::TextRange { prefix, .. } = &mut tampered_context else {
            unreachable!();
        };
        prefix.push('x');
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &revision,
                &tampered_context,
            ),
            Err(AnnotationAnchorMigrationError::ContextMismatch)
        );

        let emoji = byte_offset(source, "🙂");
        let non_utf8 = Anchor::TextRange {
            exact: "🙂".to_owned(),
            prefix: String::new(),
            suffix: String::new(),
            start: emoji + 1,
            end: emoji + 5,
            base_revision: revision.as_str().to_owned(),
            block_id: None,
            heading_path: Vec::new(),
        };
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &revision,
                &non_utf8,
            ),
            Err(AnnotationAnchorMigrationError::InvalidRange)
        );

        let outside = u64::try_from(source.len()).unwrap() + 1;
        let out_of_bounds = Anchor::TextRange {
            exact: "x".to_owned(),
            prefix: String::new(),
            suffix: String::new(),
            start: outside,
            end: outside + 1,
            base_revision: revision.as_str().to_owned(),
            block_id: None,
            heading_path: Vec::new(),
        };
        assert_eq!(
            resolve_annotation_anchor_range(
                DocumentProfileId::AsciiDocV1,
                source,
                &revision,
                &out_of_bounds,
            ),
            Err(AnnotationAnchorMigrationError::InvalidRange)
        );
    }
}
