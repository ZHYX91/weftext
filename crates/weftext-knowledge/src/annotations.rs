use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceAnchor {
    pub node: NodeRef,
    pub revision_id: String,
    pub source_range: ExactRange,
    pub visible_excerpt: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelRect {
    pub x: u64,
    pub y: u64,
    pub width: u64,
    pub height: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoRegion {
    pub pixel_width: u64,
    pub pixel_height: u64,
    pub rect: PixelRect,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timebase {
    pub numerator: u64,
    pub denominator: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageFacts {
    pub width: f64,
    pub height: f64,
    pub rotation: u16,
    pub extraction_id: Option<String>,
    pub extracted_scalars: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ResourceFacts {
    Pdf {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        pages: Vec<PdfPageFacts>,
    },
    Image {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        width: u64,
        height: u64,
    },
    Audio {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        timebase: Timebase,
        #[serde(rename = "durationTick")]
        duration_tick: u64,
    },
    Video {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        timebase: Timebase,
        #[serde(rename = "durationTick")]
        duration_tick: u64,
        width: u64,
        height: u64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AnnotationTarget {
    WholeDocument {
        node: NodeRef,
    },
    SourceRange {
        anchor: SourceAnchor,
    },
    SourceElement {
        anchor: SourceAnchor,
        #[serde(rename = "elementRole")]
        element_role: String,
    },
    WholeResource {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
    },
    PdfRegion {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        #[serde(rename = "pageIndex")]
        page_index: u32,
        #[serde(rename = "pageWidth")]
        page_width: f64,
        #[serde(rename = "pageHeight")]
        page_height: f64,
        rotation: u16,
        rect: PointRect,
    },
    PdfTextRange {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        #[serde(rename = "pageIndex")]
        page_index: u32,
        #[serde(rename = "extractionId")]
        extraction_id: String,
        from: usize,
        to: usize,
    },
    ImageRegion {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        #[serde(rename = "pixelWidth")]
        pixel_width: u64,
        #[serde(rename = "pixelHeight")]
        pixel_height: u64,
        rect: PixelRect,
    },
    MediaInterval {
        resource: String,
        #[serde(rename = "revisionId")]
        revision_id: String,
        timebase: Timebase,
        #[serde(rename = "startTick")]
        start_tick: u64,
        #[serde(rename = "endTick")]
        end_tick: u64,
        #[serde(rename = "durationTick")]
        duration_tick: u64,
        region: Option<VideoRegion>,
    },
}
impl AnnotationTarget {
    pub(crate) fn resource_basis(&self) -> Option<(&str, &str)> {
        match self {
            Self::WholeResource {
                resource,
                revision_id,
            }
            | Self::PdfRegion {
                resource,
                revision_id,
                ..
            }
            | Self::PdfTextRange {
                resource,
                revision_id,
                ..
            }
            | Self::ImageRegion {
                resource,
                revision_id,
                ..
            }
            | Self::MediaInterval {
                resource,
                revision_id,
                ..
            } => Some((resource, revision_id)),
            _ => None,
        }
    }
    pub fn anchor(&self) -> Option<&SourceAnchor> {
        match self {
            Self::SourceRange { anchor } | Self::SourceElement { anchor, .. } => Some(anchor),
            _ => None,
        }
    }
    pub(crate) fn validate(&self) -> Result<()> {
        fn pixels(rect: &PixelRect, width: u64, height: u64) -> bool {
            width > 0
                && height > 0
                && rect.width > 0
                && rect.height > 0
                && rect
                    .x
                    .checked_add(rect.width)
                    .is_some_and(|end| end <= width)
                && rect
                    .y
                    .checked_add(rect.height)
                    .is_some_and(|end| end <= height)
        }
        let valid = match self {
            Self::WholeDocument { node } => {
                !node.workspace_id.is_empty() && !node.node_id.is_empty()
            }
            Self::SourceRange { anchor } | Self::SourceElement { anchor, .. } => {
                !anchor.revision_id.is_empty()
                    && anchor.source_range.to >= anchor.source_range.from
                    && anchor.source_range.to - anchor.source_range.from
                        == anchor.source_range.raw.len()
                    && anchor.source_range.utf16_to >= anchor.source_range.utf16_from
                    && anchor.source_range.utf16_to - anchor.source_range.utf16_from
                        == anchor.source_range.raw.encode_utf16().count()
            }
            Self::WholeResource {
                resource,
                revision_id,
            } => !resource.is_empty() && !revision_id.is_empty(),
            Self::PdfRegion {
                resource,
                revision_id,
                page_width,
                page_height,
                rotation,
                rect,
                ..
            } => {
                !resource.is_empty()
                    && !revision_id.is_empty()
                    && [
                        *page_width,
                        *page_height,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                    ]
                    .iter()
                    .all(|v| v.is_finite())
                    && *page_width > 0.0
                    && *page_height > 0.0
                    && matches!(rotation, 0 | 90 | 180 | 270)
                    && rect.x >= 0.0
                    && rect.y >= 0.0
                    && rect.width > 0.0
                    && rect.height > 0.0
                    && rect.x + rect.width <= *page_width
                    && rect.y + rect.height <= *page_height
            }
            Self::PdfTextRange {
                resource,
                revision_id,
                extraction_id,
                from,
                to,
                ..
            } => {
                !resource.is_empty()
                    && !revision_id.is_empty()
                    && !extraction_id.is_empty()
                    && from < to
            }
            Self::ImageRegion {
                resource,
                revision_id,
                pixel_width,
                pixel_height,
                rect,
            } => {
                !resource.is_empty()
                    && !revision_id.is_empty()
                    && pixels(rect, *pixel_width, *pixel_height)
            }
            Self::MediaInterval {
                resource,
                revision_id,
                timebase,
                start_tick,
                end_tick,
                duration_tick,
                region,
            } => {
                !resource.is_empty()
                    && !revision_id.is_empty()
                    && timebase.numerator > 0
                    && timebase.denominator > 0
                    && start_tick < end_tick
                    && end_tick <= duration_tick
                    && region
                        .as_ref()
                        .is_none_or(|r| pixels(&r.rect, r.pixel_width, r.pixel_height))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(KnowledgeError::new(
                "invalidTarget",
                "Annotation target coordinates or evidence are invalid; original JSON is retained",
            ))
        }
    }
}

/// Shape validation is supplemented with exact authorized resource facts. UI
/// page dimensions, extraction IDs and timebases cannot certify themselves.
pub fn validate_resource_target(target: &AnnotationTarget, facts: &ResourceFacts) -> Result<()> {
    target.validate()?;
    let matching = match (target, facts) {
        (
            AnnotationTarget::PdfRegion {
                resource,
                revision_id,
                page_index,
                page_width,
                page_height,
                rotation,
                ..
            },
            ResourceFacts::Pdf {
                resource: r,
                revision_id: v,
                pages,
            },
        ) => {
            resource == r
                && revision_id == v
                && pages.get(*page_index as usize).is_some_and(|page| {
                    page.width == *page_width
                        && page.height == *page_height
                        && page.rotation == *rotation
                })
        }
        (
            AnnotationTarget::PdfTextRange {
                resource,
                revision_id,
                page_index,
                extraction_id,
                from,
                to,
            },
            ResourceFacts::Pdf {
                resource: r,
                revision_id: v,
                pages,
            },
        ) => {
            resource == r
                && revision_id == v
                && pages.get(*page_index as usize).is_some_and(|page| {
                    page.extraction_id.as_ref() == Some(extraction_id)
                        && page
                            .extracted_scalars
                            .is_some_and(|count| from < to && *to <= count)
                })
        }
        (
            AnnotationTarget::ImageRegion {
                resource,
                revision_id,
                pixel_width,
                pixel_height,
                ..
            },
            ResourceFacts::Image {
                resource: r,
                revision_id: v,
                width,
                height,
            },
        ) => resource == r && revision_id == v && pixel_width == width && pixel_height == height,
        (
            AnnotationTarget::MediaInterval {
                resource,
                revision_id,
                timebase,
                start_tick: _,
                end_tick: _,
                duration_tick,
                region,
            },
            ResourceFacts::Audio {
                resource: r,
                revision_id: v,
                timebase: t,
                duration_tick: d,
            },
        ) => {
            resource == r
                && revision_id == v
                && timebase.numerator == t.numerator
                && timebase.denominator == t.denominator
                && duration_tick == d
                && region.is_none()
        }
        (
            AnnotationTarget::MediaInterval {
                resource,
                revision_id,
                timebase,
                duration_tick,
                region,
                ..
            },
            ResourceFacts::Video {
                resource: r,
                revision_id: v,
                timebase: t,
                duration_tick: d,
                width,
                height,
            },
        ) => {
            resource == r
                && revision_id == v
                && timebase.numerator == t.numerator
                && timebase.denominator == t.denominator
                && duration_tick == d
                && region.as_ref().is_none_or(|region| {
                    region.pixel_width == *width && region.pixel_height == *height
                })
        }
        (
            AnnotationTarget::WholeResource {
                resource,
                revision_id,
            },
            ResourceFacts::Pdf {
                resource: r,
                revision_id: v,
                ..
            }
            | ResourceFacts::Image {
                resource: r,
                revision_id: v,
                ..
            }
            | ResourceFacts::Audio {
                resource: r,
                revision_id: v,
                ..
            }
            | ResourceFacts::Video {
                resource: r,
                revision_id: v,
                ..
            },
        ) => resource == r && revision_id == v,
        _ => false,
    };
    if matching {
        Ok(())
    } else {
        Err(KnowledgeError::new(
            "resourceBasis",
            "The annotation coordinate basis differs from the exact authorized resource",
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Suggestion {
    pub replacement: String,
    pub state: String,
    pub confirmed_revision: String,
    #[serde(default)]
    pub requires_reconfirmation: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationRecord {
    pub annotation_id: String,
    pub reply_to: Option<String>,
    pub attribution: String,
    pub created_at: String,
    pub purpose: String,
    pub body: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub appearance: Value,
    pub review_state: String,
    pub target: AnnotationTarget,
    pub suggestion: Option<Suggestion>,
    #[serde(default)]
    pub target_history: Vec<AnnotationTarget>,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

/// Keeps original JSON for exact before images; mutations replace only selected
/// understood members in the original value and preserve unknown data.
#[derive(Debug, Clone)]
pub struct AnnotationDocument {
    pub original: Vec<u8>,
    pub records: Vec<AnnotationRecord>,
    value: Value,
    target_errors: BTreeMap<String, String>,
}
pub fn decode_annotations(bytes: &[u8]) -> Result<AnnotationDocument> {
    let UniqueValue(value) = serde_json::from_slice(bytes).map_err(|e| {
        KnowledgeError::new(
            "damagedAnnotations",
            format!("Invalid annotation JSON; original bytes must be retained: {e}"),
        )
    })?;
    verify_json_number_preservation(bytes)?;
    if value.get("version").and_then(Value::as_u64) != Some(1) {
        return Err(KnowledgeError::new(
            "unsupportedAnnotations",
            "Unknown annotation format; original bytes must be retained",
        ));
    }
    let items = value
        .get("annotations")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            KnowledgeError::new(
                "damagedAnnotations",
                "Missing annotations array; do not initialize over these bytes",
            )
        })?;
    let mut records = vec![];
    let mut ids = BTreeSet::new();
    let mut target_errors = BTreeMap::new();
    for item in items {
        let record: AnnotationRecord = serde_json::from_value(item.clone()).map_err(|e| {
            KnowledgeError::new(
                "unsupportedAnnotations",
                format!(
                    "An annotation is not safely editable; original bytes must be retained: {e}"
                ),
            )
        })?;
        if record.annotation_id.is_empty()
            || !ids.insert(record.annotation_id.clone())
            || !matches!(record.review_state.as_str(), "open" | "resolved")
            || !matches!(record.purpose.as_str(), "comment" | "mark" | "suggestion")
        {
            return Err(KnowledgeError::new(
                "damagedAnnotations",
                "Invalid or duplicate annotation identity/state",
            ));
        }
        if let Err(error) = record.target.validate() {
            target_errors.insert(record.annotation_id.clone(), error.message);
        }
        if record.purpose == "suggestion" && record.suggestion.is_none()
            || record
                .suggestion
                .as_ref()
                .is_some_and(|s| !matches!(s.state.as_str(), "pending" | "accepted" | "rejected"))
        {
            return Err(KnowledgeError::new(
                "damagedAnnotations",
                "Invalid suggestion state",
            ));
        }
        records.push(record);
    }
    for record in &records {
        if record
            .reply_to
            .as_ref()
            .is_some_and(|parent| !ids.contains(parent) || parent == &record.annotation_id)
        {
            return Err(KnowledgeError::new(
                "damagedAnnotations",
                "Reply target is missing or self-referential",
            ));
        }
        let mut seen = BTreeSet::new();
        let mut next = record.reply_to.as_ref();
        while let Some(parent) = next {
            if !seen.insert(parent) {
                return Err(KnowledgeError::new(
                    "damagedAnnotations",
                    "Reply cycle; original discussion is retained",
                ));
            }
            next = records
                .iter()
                .find(|r| &r.annotation_id == parent)
                .and_then(|r| r.reply_to.as_ref());
        }
    }
    Ok(AnnotationDocument {
        original: bytes.to_vec(),
        records,
        value,
        target_errors,
    })
}

/// Unknown extension values must survive a read/edit/write cycle numerically.
/// serde_json can otherwise round a large integer or precise decimal through
/// f64. The JSON parser still owns validity; this audit only rejects lossy
/// numeric storage after a successful parse, without creating a new dialect.
fn verify_json_number_preservation(bytes: &[u8]) -> Result<()> {
    fn decimal_key(token: &str) -> Option<(bool, String, i64)> {
        let negative = token.starts_with('-');
        let token = token.trim_start_matches('-');
        let (mantissa, exponent) = token.split_once(['e', 'E']).unwrap_or((token, "0"));
        let exponent = exponent.parse::<i64>().ok()?;
        let fraction = mantissa.split_once('.').map_or(0, |(_, f)| f.len()) as i64;
        let mut digits = mantissa.chars().filter(|c| *c != '.').collect::<String>();
        let mut exponent = exponent.checked_sub(fraction)?;
        while digits.ends_with('0') && !digits.is_empty() {
            digits.pop();
            exponent = exponent.checked_add(1)?;
        }
        let digits = digits.trim_start_matches('0').to_owned();
        Some(if digits.is_empty() {
            (false, String::new(), 0)
        } else {
            (negative, digits, exponent)
        })
    }
    let mut at = 0;
    let mut in_string = false;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if in_string => {
                at += 2;
                continue;
            }
            b'"' => in_string = !in_string,
            b'-' | b'0'..=b'9' if !in_string => {
                let start = at;
                while at < bytes.len()
                    && matches!(bytes[at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    at += 1;
                }
                let token = std::str::from_utf8(&bytes[start..at]).map_err(|_| {
                    KnowledgeError::new("damagedAnnotations", "Invalid JSON numeric bytes")
                })?;
                let number: serde_json::Number = serde_json::from_str(token).map_err(|_| {
                    KnowledgeError::new("damagedAnnotations", "JSON number cannot be retained")
                })?;
                if decimal_key(token).is_none()
                    || decimal_key(token) != decimal_key(&number.to_string())
                {
                    return Err(KnowledgeError::new(
                        "unsupportedAnnotations",
                        "A JSON number cannot round-trip exactly; retain the original sidecar without overwriting it",
                    ));
                }
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    Ok(())
}
impl AnnotationDocument {
    pub fn create(&self, record: AnnotationRecord) -> Result<Vec<u8>> {
        self.create_many(vec![record])
    }
    pub fn create_many(&self, records: Vec<AnnotationRecord>) -> Result<Vec<u8>> {
        let mut value = self.value.clone();
        for record in records {
            record.target.validate()?;
            value["annotations"].as_array_mut().unwrap().push(
                serde_json::to_value(record)
                    .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?,
            );
        }
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
        decode_annotations(&bytes)?;
        Ok(bytes)
    }
    fn change(&self, id: &str, edits: &[(&str, Value)]) -> Result<Vec<u8>> {
        if !self.records.iter().any(|record| record.annotation_id == id) {
            return Err(KnowledgeError::new(
                "missingAnnotation",
                "The selected annotation is not in this exact document",
            ));
        }
        let mut value = self.value.clone();
        let item = value["annotations"]
            .as_array_mut()
            .and_then(|items| {
                items
                    .iter_mut()
                    .find(|item| item["annotationId"].as_str() == Some(id))
            })
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                KnowledgeError::new("damagedAnnotations", "The original record is not editable")
            })?;
        for (key, after) in edits {
            item.insert((*key).into(), after.clone());
        }
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
        decode_annotations(&bytes)?;
        Ok(bytes)
    }
    pub fn resolve(&self, id: &str, resolved: bool) -> Result<Vec<u8>> {
        self.change(
            id,
            &[(
                "reviewState",
                Value::String(if resolved { "resolved" } else { "open" }.into()),
            )],
        )
    }
    /// Rejection needs annotation qualification only; no target source read.
    pub fn reject(&self, id: &str) -> Result<Vec<u8>> {
        let record = self
            .records
            .iter()
            .find(|r| r.annotation_id == id)
            .ok_or_else(|| {
                KnowledgeError::new("missingAnnotation", "Select a stored annotation")
            })?;
        let mut suggestion = record
            .suggestion
            .clone()
            .filter(|s| s.state == "pending")
            .ok_or_else(|| {
                KnowledgeError::new(
                    "suggestionState",
                    "Only pending suggestions can be rejected",
                )
            })?;
        suggestion.state = "rejected".into();
        self.change(
            id,
            &[("suggestion", serde_json::to_value(suggestion).unwrap())],
        )
    }
    pub fn reply(&self, reply: AnnotationRecord) -> Result<Vec<u8>> {
        if reply.reply_to.is_none() {
            return Err(KnowledgeError::new(
                "replyTarget",
                "A reply requires an explicit existing parent",
            ));
        }
        self.create(reply)
    }
    pub fn reattach(&self, id: &str, target: AnnotationTarget) -> Result<Vec<u8>> {
        target.validate()?;
        let record = self
            .records
            .iter()
            .find(|r| r.annotation_id == id)
            .ok_or_else(|| {
                KnowledgeError::new("missingAnnotation", "Select a stored annotation")
            })?;
        if record
            .suggestion
            .as_ref()
            .is_some_and(|s| s.state == "pending")
        {
            return Err(KnowledgeError::new(
                "reconfirmationRequired",
                "A pending suggestion requires a separate reconfirmed target and replacement preview",
            ));
        }
        let mut history = record.target_history.clone();
        history.push(record.target.clone());
        self.change(
            id,
            &[
                ("target", serde_json::to_value(target).unwrap()),
                ("targetHistory", serde_json::to_value(history).unwrap()),
            ],
        )
    }
    /// A separate explicit action, never an automatic relocation. The host must
    /// preview the exact selected current target and replacement under Core.
    pub fn reconfirm(
        &self,
        id: &str,
        target: AnnotationTarget,
        replacement: String,
    ) -> Result<Vec<u8>> {
        target.validate()?;
        let anchor = target.anchor().ok_or_else(|| {
            KnowledgeError::new(
                "unsupportedSuggestion",
                "Reconfirmation needs an exact authored source target",
            )
        })?;
        let record = self
            .records
            .iter()
            .find(|r| r.annotation_id == id)
            .ok_or_else(|| {
                KnowledgeError::new("missingAnnotation", "Select the stored pending suggestion")
            })?;
        if record
            .suggestion
            .as_ref()
            .is_none_or(|s| s.state != "pending")
        {
            return Err(KnowledgeError::new(
                "suggestionState",
                "Only a pending suggestion can be reconfirmed",
            ));
        }
        let suggestion = Suggestion {
            replacement,
            state: "pending".into(),
            confirmed_revision: anchor.revision_id.clone(),
            requires_reconfirmation: false,
        };
        let mut history = record.target_history.clone();
        history.push(record.target.clone());
        self.change(
            id,
            &[
                ("target", serde_json::to_value(target).unwrap()),
                ("targetHistory", serde_json::to_value(history).unwrap()),
                ("suggestion", serde_json::to_value(suggestion).unwrap()),
                ("copyTargetState", Value::Null),
            ],
        )
    }
}

pub fn source_anchor(
    node: &NodeSnapshot,
    from: usize,
    to: usize,
    visible_excerpt: String,
) -> Result<SourceAnchor> {
    let boundaries: BTreeSet<_> = node
        .source
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(node.source.len()))
        .collect();
    if !boundaries.contains(&from) || !boundaries.contains(&to) {
        return Err(KnowledgeError::new(
            "partialGrapheme",
            "Annotation endpoints must be whole graphemes",
        ));
    }
    Ok(SourceAnchor {
        node: node.owner.clone(),
        revision_id: node.revision_id.clone(),
        source_range: ExactRange::new(&node.source, from, to)?,
        visible_excerpt,
    })
}
#[derive(Debug, Clone)]
pub struct ExactEdit {
    pub before: NodeSnapshot,
    pub after: NodeSnapshot,
    pub patch: SourcePatch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetResolution {
    pub status: String,
    pub current_range: Option<ExactRange>,
}
pub fn resolve_anchor(
    anchor: &SourceAnchor,
    current: Option<&NodeSnapshot>,
    history: &[ExactEdit],
) -> TargetResolution {
    let failure = |status: &str| TargetResolution {
        status: status.into(),
        current_range: None,
    };
    let Some(current) = current else {
        return failure("unavailable");
    };
    if anchor.node != current.owner {
        return failure("orphaned");
    }
    if anchor.revision_id == current.revision_id {
        return if anchor.source_range.verify(&current.source) {
            TargetResolution {
                status: "exact".into(),
                current_range: Some(anchor.source_range.clone()),
            }
        } else {
            failure("stale")
        };
    }
    let mut revision = anchor.revision_id.clone();
    let mut from = anchor.source_range.from;
    let mut to = anchor.source_range.to;
    let mut visited = BTreeSet::new();
    while revision != current.revision_id {
        if !visited.insert(revision.clone()) {
            return failure("ambiguous");
        }
        let candidates: Vec<_> = history
            .iter()
            .filter(|edit| edit.before.revision_id == revision && edit.before.owner == anchor.node)
            .collect();
        let [edit] = candidates.as_slice() else {
            return failure(if candidates.is_empty() {
                "stale"
            } else {
                "ambiguous"
            });
        };
        if edit.after.owner != anchor.node
            || !edit.patch.range.verify(&edit.before.source)
            || edit.patch.after != edit.after.source
            || !SourcePatch::new(
                &edit.before.source,
                edit.patch.range.clone(),
                edit.patch.replacement.clone(),
            )
            .is_ok_and(|verified| verified.after == edit.patch.after)
            || edit.before.source.get(from..to) != Some(anchor.source_range.raw.as_str())
        {
            return failure("stale");
        }
        let start = edit.patch.range.from;
        let end = edit.patch.range.to;
        if start < to && end > from || start == end && start > from && start < to {
            return failure("stale");
        }
        if end <= from {
            let delta = edit.patch.replacement.len() as isize - (end - start) as isize;
            let Some(new_from) = from.checked_add_signed(delta) else {
                return failure("stale");
            };
            let Some(new_to) = to.checked_add_signed(delta) else {
                return failure("stale");
            };
            from = new_from;
            to = new_to;
        }
        revision = edit.after.revision_id.clone();
    }
    match ExactRange::new(&current.source, from, to) {
        Ok(range) if range.raw == anchor.source_range.raw => TargetResolution {
            status: "mapped".into(),
            current_range: Some(range),
        },
        _ => failure("stale"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationView {
    pub annotation_id: String,
    pub reply_to: Option<String>,
    pub attribution: String,
    pub created_at: String,
    pub purpose: String,
    pub body: String,
    pub labels: Vec<String>,
    pub review_state: String,
    pub visible_excerpt: Option<String>,
    pub target_state: String,
    pub suggestion_state: Option<String>,
}
/// Stored discussion projection never serializes raw source preimage, target
/// private URIs, offsets, or fresh target context. The host authorizes this read.
pub fn annotation_views(
    document: &AnnotationDocument,
    current: Option<&NodeSnapshot>,
    history: &[ExactEdit],
) -> Vec<AnnotationView> {
    document
        .records
        .iter()
        .map(|record| AnnotationView {
            annotation_id: record.annotation_id.clone(),
            reply_to: record.reply_to.clone(),
            attribution: record.attribution.clone(),
            created_at: record.created_at.clone(),
            purpose: record.purpose.clone(),
            body: record.body.clone(),
            labels: record.labels.clone(),
            review_state: record.review_state.clone(),
            visible_excerpt: record.target.anchor().map(|a| a.visible_excerpt.clone()),
            target_state: if document.target_errors.contains_key(&record.annotation_id) {
                "invalid".into()
            } else if record
                .unknown
                .get("copyTargetState")
                .and_then(Value::as_str)
                == Some("stale")
            {
                "stale".into()
            } else {
                record
                    .target
                    .anchor()
                    .map(|anchor| resolve_anchor(anchor, current, history).status)
                    .unwrap_or_else(|| "unavailable".into())
            },
            suggestion_state: record.suggestion.as_ref().map(|s| s.state.clone()),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionPlan {
    pub files: Vec<PlannedFile>,
    pub source_patch: SourcePatch,
}
/// Produces both frozen before/after images, never a file write. Core must bind
/// permissions/generations and commit the entire set under one operation ID.
pub fn suggestion_plan(
    document: &AnnotationDocument,
    annotation_path: &str,
    id: &str,
    source: &NodeSnapshot,
) -> Result<SuggestionPlan> {
    if document.target_errors.contains_key(id) {
        return Err(KnowledgeError::new(
            "invalidTarget",
            "An invalid annotation target cannot qualify a suggestion",
        ));
    }
    let record = document
        .records
        .iter()
        .find(|r| r.annotation_id == id)
        .ok_or_else(|| KnowledgeError::new("missingAnnotation", "The annotation is missing"))?;
    let mut suggestion = record
        .suggestion
        .clone()
        .filter(|s| s.state == "pending")
        .ok_or_else(|| KnowledgeError::new("suggestionState", "The suggestion is not pending"))?;
    let anchor = record.target.anchor().ok_or_else(|| {
        KnowledgeError::new(
            "unsupportedSuggestion",
            "Only a verified exact authored source selection is executable",
        )
    })?;
    if suggestion.requires_reconfirmation
        || anchor.node != source.owner
        || anchor.revision_id != source.revision_id
        || suggestion.confirmed_revision != source.revision_id
        || !anchor.source_range.verify(&source.source)
    {
        return Err(KnowledgeError::new(
            "staleSuggestion",
            "The exact source basis changed; retain pending state and reconfirm",
        ));
    }
    // A replacement of literal source text is the first safe operation profile.
    // Complex formatted selections need the Editor's separate verified operation.
    let safe = writable_suggestion_range(&source.source, &anchor.source_range);
    if !safe
        || anchor
            .source_range
            .raw
            .chars()
            .any(|c| "#[]{}$*_<>@`\\".contains(c))
    {
        return Err(KnowledgeError::new(
            "sourceEntry",
            "A formatted or structural selection requires an Editor proof",
        ));
    }
    let encoded = weftext_engine::encode_literal(&suggestion.replacement)
        .map_err(|e| KnowledgeError::new("encoding", e.to_string()))?;
    let patch = SourcePatch::new(&source.source, anchor.source_range.clone(), encoded)?;
    suggestion.state = "accepted".into();
    let after = document.change(
        id,
        &[("suggestion", serde_json::to_value(suggestion).unwrap())],
    )?;
    Ok(SuggestionPlan {
        files: vec![
            PlannedFile {
                path: source.path.clone(),
                before: Some(source.source.as_bytes().to_vec()),
                after: patch.after.as_bytes().to_vec(),
            },
            PlannedFile {
                path: annotation_path.into(),
                before: Some(document.original.clone()),
                after,
            },
        ],
        source_patch: patch,
    })
}

/// Conservative read-only recovery of stored discussion in parseable JSON.
/// Never returns a target/preimage/replacement and never qualifies execution.
pub fn recover_discussion(bytes: &[u8]) -> Vec<AnnotationView> {
    let Ok(UniqueValue(value)) = serde_json::from_slice(bytes) else {
        return vec![];
    };
    let Some(items) = value.get("annotations").and_then(Value::as_array) else {
        return vec![];
    };
    items
        .iter()
        .filter_map(|item| {
            Some(AnnotationView {
                annotation_id: item.get("annotationId")?.as_str()?.into(),
                reply_to: item
                    .get("replyTo")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                attribution: item
                    .get("attribution")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                created_at: item
                    .get("createdAt")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                purpose: "recovered".into(),
                body: item.get("body")?.as_str()?.into(),
                labels: vec![],
                review_state: item
                    .get("reviewState")
                    .and_then(Value::as_str)
                    .unwrap_or("open")
                    .into(),
                visible_excerpt: None,
                target_state: "recoveredUnverified".into(),
                suggestion_state: None,
            })
        })
        .collect()
}

struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON with no duplicate members")
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = vec![];
                while let Some(UniqueValue(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, UniqueValue(value))) =
                    map.next_entry::<String, UniqueValue>()?
                {
                    if values.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON member"));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
