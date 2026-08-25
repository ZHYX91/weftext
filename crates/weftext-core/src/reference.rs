use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use saphyr_parser::{Event, Parser, ScalarStyle, Span, SpannedEventReceiver};
use serde::{Deserialize, Serialize};

use crate::{DocumentEnvelopeState, probe_document_envelope};

pub const CITATION_DATA_PROFILE_ID: &str = "weftext.citation-data.v1";

const REQUIRED_FIELDS: &[&str] = &["key", "type", "title"];
const STRING_FIELDS: &[&str] = &[
    "key",
    "type",
    "title",
    "title-short",
    "abstract",
    "container-title",
    "container-title-short",
    "collection-title",
    "publisher",
    "publisher-place",
    "event",
    "event-place",
    "genre",
    "medium",
    "status",
    "version",
    "language",
    "source",
    "URL",
    "DOI",
    "ISBN",
    "ISSN",
    "PMCID",
    "PMID",
    "archive",
    "archive-place",
    "archive_location",
    "call-number",
    "note",
    "volume",
    "issue",
    "number",
    "edition",
    "page",
    "page-first",
    "collection-number",
];
const NAME_FIELDS: &[&str] = &[
    "author",
    "editor",
    "translator",
    "container-author",
    "collection-editor",
    "composer",
    "director",
    "illustrator",
    "interviewer",
    "original-author",
    "recipient",
    "reviewed-author",
];
const DATE_FIELDS: &[&str] = &[
    "issued",
    "accessed",
    "original-date",
    "event-date",
    "submitted",
];
const NAME_PARTS: &[&str] = &[
    "family",
    "given",
    "literal",
    "suffix",
    "non-dropping-particle",
    "dropping-particle",
    "ORCID",
];
const DATE_PARTS: &[&str] = &["date-parts", "literal", "season", "circa"];
const ITEM_TYPES: &[&str] = &[
    "article",
    "article-journal",
    "article-magazine",
    "article-newspaper",
    "bill",
    "book",
    "broadcast",
    "chapter",
    "classic",
    "collection",
    "dataset",
    "document",
    "entry",
    "entry-dictionary",
    "entry-encyclopedia",
    "event",
    "figure",
    "graphic",
    "hearing",
    "interview",
    "legal_case",
    "legislation",
    "manuscript",
    "map",
    "motion_picture",
    "musical_score",
    "pamphlet",
    "patent",
    "performance",
    "periodical",
    "personal_communication",
    "post",
    "post-weblog",
    "regulation",
    "report",
    "review",
    "review-book",
    "software",
    "song",
    "speech",
    "standard",
    "thesis",
    "treaty",
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceAnalysis {
    pub profile: &'static str,
    pub citation_data: Option<CitationData>,
    pub mapping_range: Option<Range<u64>>,
    pub field_ranges: Vec<ReferenceFieldRange>,
    pub diagnostics: Vec<ReferenceDiagnostic>,
}

impl Default for ReferenceAnalysis {
    fn default() -> Self {
        Self {
            profile: CITATION_DATA_PROFILE_ID,
            citation_data: None,
            mapping_range: None,
            field_ranges: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationData {
    pub key: String,
    pub item_type: String,
    pub title: String,
    pub fields: BTreeMap<String, ReferenceValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ReferenceValue {
    Text(String),
    Names(Vec<ReferenceName>),
    Date(ReferenceDate),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReferenceName {
    pub family: Option<String>,
    pub given: Option<String>,
    pub literal: Option<String>,
    pub suffix: Option<String>,
    pub non_dropping_particle: Option<String>,
    pub dropping_particle: Option<String>,
    #[serde(rename = "ORCID")]
    pub orcid: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceDate {
    pub date_parts: Option<Vec<Vec<i32>>>,
    pub literal: Option<String>,
    pub season: Option<String>,
    pub circa: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceFieldRange {
    pub path: String,
    pub key_range: Option<Range<u64>>,
    pub value_range: Range<u64>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceDiagnosticCode {
    InvalidEnvelope,
    InvalidYaml,
    DuplicateYamlKey,
    UnsupportedYaml,
    ReferenceMustBeMapping,
    MissingRequiredField,
    InvalidKey,
    InvalidType,
    UnknownField,
    InvalidValue,
    InvalidName,
    InvalidDate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceDiagnostic {
    pub code: ReferenceDiagnosticCode,
    pub message: String,
    pub path: Option<String>,
    pub range: Option<Range<u64>>,
}

#[derive(Clone, Debug)]
struct YamlNode {
    kind: YamlKind,
    span: Span,
}

#[derive(Clone, Debug)]
enum YamlKind {
    Scalar(YamlScalar),
    Sequence(Vec<YamlNode>),
    Mapping(Vec<(YamlNode, YamlNode)>),
    Alias,
}

#[derive(Clone, Debug)]
struct YamlScalar {
    value: String,
    style: ScalarStyle,
}

#[derive(Debug)]
enum Frame {
    Sequence {
        start: Span,
        values: Vec<YamlNode>,
    },
    Mapping {
        start: Span,
        entries: Vec<(YamlNode, YamlNode)>,
        pending_key: Option<YamlNode>,
    },
}

#[derive(Default)]
struct YamlBuilder {
    stack: Vec<Frame>,
    current_document: Option<YamlNode>,
    documents: Vec<YamlNode>,
    issues: Vec<ParseIssue>,
}

#[derive(Clone, Debug)]
struct ParseIssue {
    code: ReferenceDiagnosticCode,
    message: String,
    span: Span,
}

impl<'input> SpannedEventReceiver<'input> for YamlBuilder {
    fn on_event(&mut self, event: Event<'input>, span: Span) {
        match event {
            Event::StreamStart | Event::StreamEnd | Event::DocumentStart(_) | Event::Nothing => {}
            Event::DocumentEnd => {
                if let Some(document) = self.current_document.take() {
                    self.documents.push(document);
                }
            }
            Event::SequenceStart(anchor, tag) => {
                self.note_metadata(anchor, tag.is_some(), span);
                self.stack.push(Frame::Sequence {
                    start: span,
                    values: Vec::new(),
                });
            }
            Event::MappingStart(anchor, tag) => {
                self.note_metadata(anchor, tag.is_some(), span);
                self.stack.push(Frame::Mapping {
                    start: span,
                    entries: Vec::new(),
                    pending_key: None,
                });
            }
            Event::SequenceEnd => {
                let Some(Frame::Sequence { start, values }) = self.stack.pop() else {
                    return;
                };
                self.insert(YamlNode {
                    kind: YamlKind::Sequence(values),
                    span: Span::new(start.start, span.end),
                });
            }
            Event::MappingEnd => {
                let Some(Frame::Mapping {
                    start,
                    entries,
                    pending_key,
                }) = self.stack.pop()
                else {
                    return;
                };
                if let Some(key) = pending_key {
                    self.issues.push(ParseIssue {
                        code: ReferenceDiagnosticCode::InvalidYaml,
                        message: "YAML mapping key has no value".to_owned(),
                        span: key.span,
                    });
                }
                let mut seen = BTreeSet::new();
                for (key, _) in &entries {
                    if let Some(value) = scalar_value(key) {
                        if !seen.insert(value.to_owned()) {
                            self.issues.push(ParseIssue {
                                code: ReferenceDiagnosticCode::DuplicateYamlKey,
                                message: format!("duplicate YAML key `{value}`"),
                                span: key.span,
                            });
                        }
                    } else {
                        self.issues.push(ParseIssue {
                            code: ReferenceDiagnosticCode::UnsupportedYaml,
                            message: "mapping keys must be scalar strings".to_owned(),
                            span: key.span,
                        });
                    }
                }
                self.insert(YamlNode {
                    kind: YamlKind::Mapping(entries),
                    span: Span::new(start.start, span.end),
                });
            }
            Event::Scalar(value, style, anchor, tag) => {
                self.note_metadata(anchor, tag.is_some(), span);
                self.insert(YamlNode {
                    kind: YamlKind::Scalar(YamlScalar {
                        value: value.into_owned(),
                        style,
                    }),
                    span,
                });
            }
            Event::Alias(_) => {
                self.issues.push(ParseIssue {
                    code: ReferenceDiagnosticCode::UnsupportedYaml,
                    message: "YAML aliases are not supported in Citation Data v1".to_owned(),
                    span,
                });
                self.insert(YamlNode {
                    kind: YamlKind::Alias,
                    span,
                });
            }
        }
    }
}

impl YamlBuilder {
    fn note_metadata(&mut self, anchor: usize, tagged: bool, span: Span) {
        if anchor > 0 {
            self.issues.push(ParseIssue {
                code: ReferenceDiagnosticCode::UnsupportedYaml,
                message: "YAML anchors are not supported in Citation Data v1".to_owned(),
                span,
            });
        }
        if tagged {
            self.issues.push(ParseIssue {
                code: ReferenceDiagnosticCode::UnsupportedYaml,
                message: "YAML tags are not supported in Citation Data v1".to_owned(),
                span,
            });
        }
    }

    fn insert(&mut self, node: YamlNode) {
        match self.stack.last_mut() {
            Some(Frame::Sequence { values, .. }) => values.push(node),
            Some(Frame::Mapping {
                entries,
                pending_key,
                ..
            }) => {
                if let Some(key) = pending_key.take() {
                    entries.push((key, node));
                } else {
                    *pending_key = Some(node);
                }
            }
            None => self.current_document = Some(node),
        }
    }
}

struct ParsedYaml {
    root: YamlNode,
    content_start: usize,
    content_char_offsets: Vec<usize>,
    issues: Vec<ParseIssue>,
}

#[must_use]
pub fn analyze_reference_metadata(source: &str) -> ReferenceAnalysis {
    let parsed = match parse_yaml_envelope(source) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return ReferenceAnalysis::default(),
        Err(diagnostic) => {
            return ReferenceAnalysis {
                diagnostics: vec![diagnostic],
                ..ReferenceAnalysis::default()
            };
        }
    };
    let mut analysis = ReferenceAnalysis::default();
    for issue in &parsed.issues {
        analysis
            .diagnostics
            .push(diagnostic_from_issue(issue, &parsed));
    }
    let Some(root) = as_mapping(&parsed.root) else {
        analysis.diagnostics.push(diagnostic(
            ReferenceDiagnosticCode::InvalidYaml,
            "frontmatter must be a YAML mapping",
            None,
            Some(node_range(&parsed.root, &parsed)),
        ));
        return analysis;
    };
    let Some((reference_key, reference_node)) = mapping_entry(root, "reference") else {
        return analysis;
    };
    analysis.mapping_range = Some(union_range(
        node_range(reference_key, &parsed),
        node_range(reference_node, &parsed),
    ));
    collect_field_ranges(
        reference_node,
        "reference",
        None,
        &parsed,
        &mut analysis.field_ranges,
    );
    let Some(reference) = as_mapping(reference_node) else {
        analysis.diagnostics.push(diagnostic(
            ReferenceDiagnosticCode::ReferenceMustBeMapping,
            "top-level `reference` must be a mapping",
            Some("reference".to_owned()),
            Some(node_range(reference_node, &parsed)),
        ));
        return analysis;
    };
    let mut fields = BTreeMap::new();
    validate_reference_mapping(reference, &parsed, &mut fields, &mut analysis.diagnostics);
    if analysis.diagnostics.is_empty() {
        let key = text_field(&fields, "key").unwrap_or_default().to_owned();
        let item_type = text_field(&fields, "type").unwrap_or_default().to_owned();
        let title = text_field(&fields, "title").unwrap_or_default().to_owned();
        analysis.citation_data = Some(CitationData {
            key,
            item_type,
            title,
            fields,
        });
    }
    analysis
}

fn parse_yaml_envelope(source: &str) -> Result<Option<ParsedYaml>, ReferenceDiagnostic> {
    let envelope = probe_document_envelope(source);
    match envelope.state {
        DocumentEnvelopeState::Absent => return Ok(None),
        DocumentEnvelopeState::Unclosed => {
            return Err(diagnostic(
                ReferenceDiagnosticCode::InvalidEnvelope,
                "frontmatter is not closed",
                None,
                envelope.range,
            ));
        }
        DocumentEnvelopeState::Closed => {}
    }
    let range = envelope.content_range.ok_or_else(|| {
        diagnostic(
            ReferenceDiagnosticCode::InvalidEnvelope,
            "frontmatter content range is unavailable",
            None,
            envelope.range,
        )
    })?;
    let content_start = usize::try_from(range.start).unwrap_or(source.len());
    let content_end = usize::try_from(range.end).unwrap_or(source.len());
    let content = &source[content_start..content_end];
    let content_char_offsets = content
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(content.len()))
        .collect::<Vec<_>>();
    let mut builder = YamlBuilder::default();
    let mut parser = Parser::new_from_str(content).keep_tags(true);
    if let Err(error) = parser.load(&mut builder, true) {
        let relative_at = content_char_offsets
            .get(error.marker().index())
            .copied()
            .unwrap_or(content.len());
        let at = content_start.saturating_add(relative_at).min(source.len());
        return Err(diagnostic(
            ReferenceDiagnosticCode::InvalidYaml,
            format!("invalid YAML: {}", error.info()),
            None,
            Some(to_u64(at)..to_u64(at)),
        ));
    }
    if builder.documents.len() != 1 {
        return Err(diagnostic(
            ReferenceDiagnosticCode::InvalidYaml,
            "frontmatter must contain exactly one YAML document",
            None,
            Some(range),
        ));
    }
    Ok(Some(ParsedYaml {
        root: builder.documents.remove(0),
        content_start,
        content_char_offsets,
        issues: builder.issues,
    }))
}

fn validate_reference_mapping(
    mapping: &[(YamlNode, YamlNode)],
    parsed: &ParsedYaml,
    fields: &mut BTreeMap<String, ReferenceValue>,
    diagnostics: &mut Vec<ReferenceDiagnostic>,
) {
    for required in REQUIRED_FIELDS {
        if mapping_entry(mapping, required).is_none() {
            diagnostics.push(diagnostic(
                ReferenceDiagnosticCode::MissingRequiredField,
                format!("reference.{required} is required"),
                Some(format!("reference.{required}")),
                None,
            ));
        }
    }
    for (key_node, value_node) in mapping {
        let Some(key) = scalar_value(key_node) else {
            continue;
        };
        let path = format!("reference.{key}");
        if STRING_FIELDS.contains(&key) {
            if let Some(value) = citation_string(value_node) {
                fields.insert(key.to_owned(), ReferenceValue::Text(value.to_owned()));
            } else {
                diagnostics.push(diagnostic(
                    ReferenceDiagnosticCode::InvalidValue,
                    format!("{path} must be a non-empty YAML string"),
                    Some(path),
                    Some(node_range(value_node, parsed)),
                ));
            }
        } else if NAME_FIELDS.contains(&key) {
            if let Some(names) = validate_names(value_node, &path, parsed, diagnostics) {
                fields.insert(key.to_owned(), ReferenceValue::Names(names));
            }
        } else if DATE_FIELDS.contains(&key) {
            if let Some(date) = validate_date(value_node, &path, parsed, diagnostics) {
                fields.insert(key.to_owned(), ReferenceValue::Date(date));
            }
        } else {
            diagnostics.push(diagnostic(
                ReferenceDiagnosticCode::UnknownField,
                format!("unknown Citation Data v1 field `{key}`"),
                Some(path),
                Some(node_range(key_node, parsed)),
            ));
        }
    }
    if let Some(ReferenceValue::Text(key)) = fields.get("key")
        && !valid_citation_key(key)
    {
        let range = mapping_entry(mapping, "key").map(|(_, value)| node_range(value, parsed));
        diagnostics.push(diagnostic(
            ReferenceDiagnosticCode::InvalidKey,
            "reference.key must match ^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$",
            Some("reference.key".to_owned()),
            range,
        ));
    }
    if let Some(ReferenceValue::Text(item_type)) = fields.get("type")
        && !ITEM_TYPES.contains(&item_type.as_str())
    {
        let range = mapping_entry(mapping, "type").map(|(_, value)| node_range(value, parsed));
        diagnostics.push(diagnostic(
            ReferenceDiagnosticCode::InvalidType,
            format!("unsupported Citation Data v1 item type `{item_type}`"),
            Some("reference.type".to_owned()),
            range,
        ));
    }
}

fn validate_names(
    node: &YamlNode,
    path: &str,
    parsed: &ParsedYaml,
    diagnostics: &mut Vec<ReferenceDiagnostic>,
) -> Option<Vec<ReferenceName>> {
    let Some(sequence) = as_sequence(node) else {
        diagnostics.push(invalid_name(
            path,
            node,
            parsed,
            "must be a non-empty sequence",
        ));
        return None;
    };
    if sequence.is_empty() {
        diagnostics.push(invalid_name(path, node, parsed, "must not be empty"));
        return None;
    }
    let mut result = Vec::new();
    let mut valid = true;
    for (index, item) in sequence.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let Some(mapping) = as_mapping(item) else {
            diagnostics.push(invalid_name(&item_path, item, parsed, "must be a mapping"));
            valid = false;
            continue;
        };
        let mut name = ReferenceName::default();
        let mut item_valid = true;
        for (key_node, value_node) in mapping {
            let Some(key) = scalar_value(key_node) else {
                item_valid = false;
                continue;
            };
            if !NAME_PARTS.contains(&key) {
                diagnostics.push(diagnostic(
                    ReferenceDiagnosticCode::UnknownField,
                    format!("unknown Citation Data v1 name field `{key}`"),
                    Some(format!("{item_path}.{key}")),
                    Some(node_range(key_node, parsed)),
                ));
                item_valid = false;
                continue;
            }
            let Some(value) = citation_string(value_node) else {
                diagnostics.push(invalid_name(
                    &format!("{item_path}.{key}"),
                    value_node,
                    parsed,
                    "must be a non-empty YAML string",
                ));
                item_valid = false;
                continue;
            };
            match key {
                "family" => name.family = Some(value.to_owned()),
                "given" => name.given = Some(value.to_owned()),
                "literal" => name.literal = Some(value.to_owned()),
                "suffix" => name.suffix = Some(value.to_owned()),
                "non-dropping-particle" => name.non_dropping_particle = Some(value.to_owned()),
                "dropping-particle" => name.dropping_particle = Some(value.to_owned()),
                "ORCID" => name.orcid = Some(value.to_owned()),
                _ => unreachable!(),
            }
        }
        let personal = name.family.is_some() || name.given.is_some();
        let literal = name.literal.is_some();
        let personal_only = name.suffix.is_some()
            || name.non_dropping_particle.is_some()
            || name.dropping_particle.is_some();
        if literal == personal || (literal && personal_only) || (!personal && !literal) {
            diagnostics.push(invalid_name(
                &item_path,
                item,
                parsed,
                "must contain either literal or personal name parts, but not both",
            ));
            item_valid = false;
        }
        valid &= item_valid;
        result.push(name);
    }
    valid.then_some(result)
}

fn validate_date(
    node: &YamlNode,
    path: &str,
    parsed: &ParsedYaml,
    diagnostics: &mut Vec<ReferenceDiagnostic>,
) -> Option<ReferenceDate> {
    let Some(mapping) = as_mapping(node) else {
        diagnostics.push(invalid_date(path, node, parsed, "must be a mapping"));
        return None;
    };
    let mut result = ReferenceDate::default();
    let mut valid = true;
    for (key_node, value_node) in mapping {
        let Some(key) = scalar_value(key_node) else {
            valid = false;
            continue;
        };
        if !DATE_PARTS.contains(&key) {
            diagnostics.push(diagnostic(
                ReferenceDiagnosticCode::UnknownField,
                format!("unknown Citation Data v1 date field `{key}`"),
                Some(format!("{path}.{key}")),
                Some(node_range(key_node, parsed)),
            ));
            valid = false;
            continue;
        }
        match key {
            "date-parts" => {
                if let Some(parts) = parse_date_parts(value_node) {
                    result.date_parts = Some(parts);
                } else {
                    diagnostics.push(invalid_date(
                        &format!("{path}.date-parts"),
                        value_node,
                        parsed,
                        "must contain one or two [year, month, day] integer sequences",
                    ));
                    valid = false;
                }
            }
            "literal" => {
                if let Some(value) = citation_string(value_node) {
                    result.literal = Some(value.to_owned());
                } else {
                    diagnostics.push(invalid_date(
                        &format!("{path}.literal"),
                        value_node,
                        parsed,
                        "must be a non-empty YAML string",
                    ));
                    valid = false;
                }
            }
            "season" => {
                if let Some(value) = citation_string(value_node) {
                    result.season = Some(value.to_owned());
                } else {
                    diagnostics.push(invalid_date(
                        &format!("{path}.season"),
                        value_node,
                        parsed,
                        "must be a non-empty YAML string",
                    ));
                    valid = false;
                }
            }
            "circa" => {
                if let Some(value) = plain_scalar(value_node).and_then(|value| match value {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                }) {
                    result.circa = Some(value);
                } else {
                    diagnostics.push(invalid_date(
                        &format!("{path}.circa"),
                        value_node,
                        parsed,
                        "must be true or false",
                    ));
                    valid = false;
                }
            }
            _ => unreachable!(),
        }
    }
    if result.date_parts.is_some() == result.literal.is_some() {
        diagnostics.push(invalid_date(
            path,
            node,
            parsed,
            "must contain exactly one of date-parts or literal",
        ));
        valid = false;
    }
    valid.then_some(result)
}

fn parse_date_parts(node: &YamlNode) -> Option<Vec<Vec<i32>>> {
    let outer = as_sequence(node)?;
    if outer.is_empty() || outer.len() > 2 {
        return None;
    }
    let mut result = Vec::new();
    for item in outer {
        let parts = as_sequence(item)?;
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        let mut parsed = Vec::new();
        for value in parts {
            parsed.push(plain_scalar(value)?.parse::<i32>().ok()?);
        }
        if parsed.get(1).is_some_and(|month| !(1..=12).contains(month))
            || parsed.get(2).is_some_and(|day| !(1..=31).contains(day))
        {
            return None;
        }
        result.push(parsed);
    }
    Some(result)
}

fn collect_field_ranges(
    node: &YamlNode,
    path: &str,
    key_range: Option<Range<u64>>,
    parsed: &ParsedYaml,
    result: &mut Vec<ReferenceFieldRange>,
) {
    result.push(ReferenceFieldRange {
        path: path.to_owned(),
        key_range,
        value_range: node_range(node, parsed),
    });
    match &node.kind {
        YamlKind::Mapping(entries) => {
            for (key, value) in entries {
                if let Some(name) = scalar_value(key) {
                    collect_field_ranges(
                        value,
                        &format!("{path}.{name}"),
                        Some(node_range(key, parsed)),
                        parsed,
                        result,
                    );
                }
            }
        }
        YamlKind::Sequence(values) => {
            for (index, value) in values.iter().enumerate() {
                collect_field_ranges(value, &format!("{path}[{index}]"), None, parsed, result);
            }
        }
        YamlKind::Scalar(_) | YamlKind::Alias => {}
    }
}

fn mapping_entry<'a>(
    mapping: &'a [(YamlNode, YamlNode)],
    key: &str,
) -> Option<(&'a YamlNode, &'a YamlNode)> {
    mapping
        .iter()
        .find(|(candidate, _)| scalar_value(candidate) == Some(key))
        .map(|(key, value)| (key, value))
}

fn as_mapping(node: &YamlNode) -> Option<&[(YamlNode, YamlNode)]> {
    match &node.kind {
        YamlKind::Mapping(value) => Some(value),
        _ => None,
    }
}

fn as_sequence(node: &YamlNode) -> Option<&[YamlNode]> {
    match &node.kind {
        YamlKind::Sequence(value) => Some(value),
        _ => None,
    }
}

fn scalar_value(node: &YamlNode) -> Option<&str> {
    match &node.kind {
        YamlKind::Scalar(value) => Some(&value.value),
        _ => None,
    }
}

fn plain_scalar(node: &YamlNode) -> Option<&str> {
    match &node.kind {
        YamlKind::Scalar(YamlScalar {
            value,
            style: ScalarStyle::Plain,
        }) => Some(value),
        _ => None,
    }
}

fn citation_string(node: &YamlNode) -> Option<&str> {
    let YamlKind::Scalar(value) = &node.kind else {
        return None;
    };
    if value.value.is_empty() {
        return None;
    }
    if value.style == ScalarStyle::Plain && plain_scalar_is_non_string(&value.value) {
        return None;
    }
    Some(&value.value)
}

fn plain_scalar_is_non_string(value: &str) -> bool {
    matches!(
        value,
        "null" | "Null" | "NULL" | "~" | "true" | "True" | "TRUE" | "false" | "False" | "FALSE"
    ) || value.parse::<f64>().is_ok()
}

fn valid_citation_key(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn text_field<'a>(fields: &'a BTreeMap<String, ReferenceValue>, key: &str) -> Option<&'a str> {
    match fields.get(key) {
        Some(ReferenceValue::Text(value)) => Some(value),
        _ => None,
    }
}

fn invalid_name(
    path: &str,
    node: &YamlNode,
    parsed: &ParsedYaml,
    detail: &str,
) -> ReferenceDiagnostic {
    diagnostic(
        ReferenceDiagnosticCode::InvalidName,
        format!("{path} {detail}"),
        Some(path.to_owned()),
        Some(node_range(node, parsed)),
    )
}

fn invalid_date(
    path: &str,
    node: &YamlNode,
    parsed: &ParsedYaml,
    detail: &str,
) -> ReferenceDiagnostic {
    diagnostic(
        ReferenceDiagnosticCode::InvalidDate,
        format!("{path} {detail}"),
        Some(path.to_owned()),
        Some(node_range(node, parsed)),
    )
}

fn diagnostic(
    code: ReferenceDiagnosticCode,
    message: impl Into<String>,
    path: Option<String>,
    range: Option<Range<u64>>,
) -> ReferenceDiagnostic {
    ReferenceDiagnostic {
        code,
        message: message.into(),
        path,
        range,
    }
}

fn diagnostic_from_issue(issue: &ParseIssue, parsed: &ParsedYaml) -> ReferenceDiagnostic {
    diagnostic(
        issue.code,
        issue.message.clone(),
        None,
        Some(span_range(issue.span, parsed)),
    )
}

fn node_range(node: &YamlNode, parsed: &ParsedYaml) -> Range<u64> {
    span_range(node.span, parsed)
}

fn span_range(span: Span, parsed: &ParsedYaml) -> Range<u64> {
    let range = span_range_usize(span, parsed);
    to_u64(range.start)..to_u64(range.end)
}

fn span_range_usize(span: Span, parsed: &ParsedYaml) -> Range<usize> {
    let content_len = parsed.content_char_offsets.last().copied().unwrap_or(0);
    let start = parsed
        .content_char_offsets
        .get(span.start.index())
        .copied()
        .unwrap_or(content_len);
    let end = parsed
        .content_char_offsets
        .get(span.end.index())
        .copied()
        .unwrap_or(content_len)
        .max(start);
    parsed.content_start + start..parsed.content_start + end
}

fn union_range(first: Range<u64>, second: Range<u64>) -> Range<u64> {
    first.start.min(second.start)..first.end.max(second.end)
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
