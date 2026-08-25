use std::collections::BTreeSet;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::source_lexing::{
    complement_ranges, decode_attribute_value, find_closing_bracket, find_unquoted_equals,
    line_end, split_comma_parts, trim_range,
};

const CITATION_ATTRIBUTES: &[&str] = &["label", "locator", "prefix", "suffix"];
const LOCATOR_LABELS: &[&str] = &[
    "book",
    "chapter",
    "column",
    "figure",
    "folio",
    "issue",
    "line",
    "note",
    "opus",
    "page",
    "paragraph",
    "part",
    "section",
    "sub-verbo",
    "verse",
    "volume",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationForm {
    Parenthetical,
    Narrative,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationAttribute {
    pub name: String,
    pub value: String,
    pub range: Range<u64>,
    pub name_range: Range<u64>,
    pub value_range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationKeyOccurrence {
    pub key: String,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationItem {
    pub range: Range<u64>,
    pub key: CitationKeyOccurrence,
    pub label: String,
    pub locator: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub attributes: Vec<CitationAttribute>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationCluster {
    pub form: CitationForm,
    pub range: Range<u64>,
    pub items: Vec<CitationItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoCiteOccurrence {
    pub range: Range<u64>,
    pub keys: Vec<CitationKeyOccurrence>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BibliographyInclusion {
    Cited,
    All,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyOccurrence {
    pub range: Range<u64>,
    pub inclusion: BibliographyInclusion,
    pub attributes: Vec<CitationAttribute>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationDiagnosticCode {
    UnknownTarget,
    UnknownAttribute,
    DuplicateAttribute,
    InvalidAttributeValue,
    InvalidKey,
    UnsupportedLocatorLabel,
    LabelWithoutLocator,
    MalformedMacro,
    EmptyCluster,
    NarrativeCluster,
    BlockMacroPlacement,
    DuplicateBibliography,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationDiagnostic {
    pub code: CitationDiagnosticCode,
    pub message: String,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationSourceAnalysis {
    pub clusters: Vec<CitationCluster>,
    pub nocites: Vec<NoCiteOccurrence>,
    pub bibliographies: Vec<BibliographyOccurrence>,
    pub diagnostics: Vec<CitationDiagnostic>,
}

#[derive(Debug)]
struct Candidate<T> {
    value: Option<T>,
    diagnostics: Vec<CitationDiagnostic>,
    consumed_end: usize,
}

#[derive(Debug)]
struct ParsedItem {
    item: Option<CitationItem>,
    diagnostics: Vec<CitationDiagnostic>,
}

/// Parses canonical Weftext citation macros without resolving their authored keys.
#[must_use]
pub fn analyze_citation_source(source: &str) -> CitationSourceAnalysis {
    let protected = weftext_asciidoc::analyze(source).protected_ranges;
    let eligible = complement_ranges(source.len(), &protected);
    let mut result = CitationSourceAnalysis::default();

    for eligible_range in eligible {
        let mut cursor = eligible_range.start;
        while cursor < eligible_range.end {
            let remaining = &source[cursor..eligible_range.end];
            let next = [
                remaining.find("cite:").map(|index| (index, 0_u8)),
                remaining.find("nocite::").map(|index| (index, 1_u8)),
                remaining.find("bibliography::").map(|index| (index, 2_u8)),
            ]
            .into_iter()
            .flatten()
            .min_by_key(|(index, kind)| (*index, *kind));
            let Some((relative, kind)) = next else {
                break;
            };
            let start = cursor + relative;
            if !is_macro_boundary(source, start) {
                cursor = start + 1;
                continue;
            }
            match kind {
                0 => {
                    let candidate = parse_citation(source, start, eligible_range.end);
                    if let Some(cluster) = candidate.value {
                        result.clusters.push(cluster);
                    }
                    result.diagnostics.extend(candidate.diagnostics);
                    cursor = candidate.consumed_end.max(start + "cite:".len());
                }
                1 => {
                    let candidate = parse_nocite(source, start, eligible_range.end);
                    if let Some(nocite) = candidate.value {
                        result.nocites.push(nocite);
                    }
                    result.diagnostics.extend(candidate.diagnostics);
                    cursor = candidate.consumed_end.max(start + "nocite::".len());
                }
                2 => {
                    let candidate = parse_bibliography(source, start, eligible_range.end);
                    if let Some(bibliography) = candidate.value {
                        result.bibliographies.push(bibliography);
                    }
                    result.diagnostics.extend(candidate.diagnostics);
                    cursor = candidate.consumed_end.max(start + "bibliography::".len());
                }
                _ => unreachable!(),
            }
        }
    }
    if result.bibliographies.len() > 1 {
        for bibliography in result.bibliographies.iter().skip(1) {
            result.diagnostics.push(diagnostic(
                CitationDiagnosticCode::DuplicateBibliography,
                "a document may contain at most one bibliography placement",
                to_usize_range(&bibliography.range),
            ));
        }
        result.bibliographies.clear();
    }
    result
}

fn parse_citation(source: &str, start: usize, limit: usize) -> Candidate<CitationCluster> {
    let target_start = start + "cite:".len();
    let Some(open) = source[target_start..limit]
        .find('[')
        .map(|at| target_start + at)
    else {
        return malformed_candidate(source, start, limit, "citation macro has no attribute list");
    };
    if line_end(source, start, limit) < open {
        return malformed_candidate(
            source,
            start,
            limit,
            "citation macro crosses a line boundary",
        );
    }
    let target = &source[target_start..open];
    let form = match target {
        "" => CitationForm::Parenthetical,
        "narrative" => CitationForm::Narrative,
        _ => {
            let end = find_closing_bracket(source, open, limit)
                .map_or_else(|| line_end(source, start, limit), |close| close + 1);
            return Candidate {
                value: None,
                diagnostics: vec![diagnostic(
                    CitationDiagnosticCode::UnknownTarget,
                    format!("unknown cite target `{target}`"),
                    target_start..open,
                )],
                consumed_end: end,
            };
        }
    };
    let Some(first_close) = find_closing_bracket(source, open, limit) else {
        return malformed_candidate(
            source,
            start,
            limit,
            "citation attribute list is not closed",
        );
    };
    let mut parsed_items = Vec::new();
    let first = parse_citation_item(source, open, first_close);
    let mut diagnostics = first.diagnostics;
    if let Some(item) = first.item {
        parsed_items.push(item);
    }
    let mut end = first_close + 1;
    let mut has_chained_item = false;
    while source[end..limit].starts_with("+[") {
        has_chained_item = true;
        let chained_open = end + 1;
        let Some(chained_close) = find_closing_bracket(source, chained_open, limit) else {
            diagnostics.push(diagnostic(
                CitationDiagnosticCode::MalformedMacro,
                "chained citation item is not closed",
                end..line_end(source, end, limit),
            ));
            return Candidate {
                value: None,
                diagnostics,
                consumed_end: line_end(source, end, limit),
            };
        };
        let chained = parse_citation_item(source, chained_open, chained_close);
        diagnostics.extend(chained.diagnostics);
        if let Some(item) = chained.item {
            parsed_items.push(item);
        }
        end = chained_close + 1;
    }
    if form == CitationForm::Narrative && has_chained_item {
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::NarrativeCluster,
            "narrative citations accept exactly one item",
            first_close + 1..end,
        ));
    }
    if source[end..limit].starts_with('+') {
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::MalformedMacro,
            "citation chain must use an immediate `+[... ]` item",
            end..line_end(source, end, limit),
        ));
    }
    let value = diagnostics.is_empty().then_some(CitationCluster {
        form,
        range: to_u64(start)..to_u64(end),
        items: parsed_items,
    });
    Candidate {
        value,
        diagnostics,
        consumed_end: end,
    }
}

#[allow(clippy::too_many_lines)]
fn parse_citation_item(source: &str, open: usize, close: usize) -> ParsedItem {
    let item_range = open + 1..close;
    let parts = match split_comma_parts(source, item_range.clone()) {
        Ok(parts) => parts,
        Err(range) => {
            return ParsedItem {
                item: None,
                diagnostics: vec![diagnostic(
                    CitationDiagnosticCode::MalformedMacro,
                    "citation item contains an invalid quoted value",
                    range,
                )],
            };
        }
    };
    let mut diagnostics = Vec::new();
    let Some(first) = parts
        .first()
        .cloned()
        .map(|range| trim_range(source, range))
    else {
        return ParsedItem {
            item: None,
            diagnostics: vec![diagnostic(
                CitationDiagnosticCode::EmptyCluster,
                "citation item is empty",
                item_range,
            )],
        };
    };
    if first.is_empty() {
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::EmptyCluster,
            "citation item is empty",
            item_range.clone(),
        ));
    }
    if source[first.clone()].contains('=') || !valid_citation_key(&source[first.clone()]) {
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::InvalidKey,
            "citation key must match ^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$",
            first.clone(),
        ));
    }
    let key = CitationKeyOccurrence {
        key: source[first.clone()].to_owned(),
        range: to_u64(first.start)..to_u64(first.end),
    };
    let mut attributes = Vec::new();
    let mut names = BTreeSet::new();
    for part in parts.into_iter().skip(1) {
        let part = trim_range(source, part);
        let Some(equals) = find_unquoted_equals(source, part.clone()) else {
            diagnostics.push(diagnostic(
                CitationDiagnosticCode::UnknownAttribute,
                "citation attributes must be named",
                part,
            ));
            continue;
        };
        let name_range = trim_range(source, part.start..equals);
        let value_range = trim_range(source, equals + 1..part.end);
        let name = source[name_range.clone()].to_owned();
        if !CITATION_ATTRIBUTES.contains(&name.as_str()) {
            diagnostics.push(diagnostic(
                CitationDiagnosticCode::UnknownAttribute,
                format!("unknown citation attribute `{name}`"),
                name_range.clone(),
            ));
            continue;
        }
        if !names.insert(name.clone()) {
            diagnostics.push(diagnostic(
                CitationDiagnosticCode::DuplicateAttribute,
                format!("duplicate citation attribute `{name}`"),
                name_range.clone(),
            ));
            continue;
        }
        let Some(value) = decode_attribute_value(source, value_range.clone()) else {
            diagnostics.push(diagnostic(
                CitationDiagnosticCode::InvalidAttributeValue,
                format!("citation attribute `{name}` has an invalid value"),
                value_range,
            ));
            continue;
        };
        attributes.push(CitationAttribute {
            name,
            value,
            range: to_u64(part.start)..to_u64(part.end),
            name_range: to_u64(name_range.start)..to_u64(name_range.end),
            value_range: to_u64(value_range.start)..to_u64(value_range.end),
        });
    }
    let locator = attribute_value(&attributes, "locator").map(ToOwned::to_owned);
    let explicit_label = attribute_value(&attributes, "label").map(ToOwned::to_owned);
    if explicit_label.is_some() && locator.is_none() {
        let range = attributes
            .iter()
            .find(|attribute| attribute.name == "label")
            .map_or(item_range.clone(), |attribute| {
                to_usize_range(&attribute.range)
            });
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::LabelWithoutLocator,
            "citation label requires a locator",
            range,
        ));
    }
    if let Some(label) = &explicit_label
        && !LOCATOR_LABELS.contains(&label.as_str())
    {
        let range = attributes
            .iter()
            .find(|attribute| attribute.name == "label")
            .map_or(item_range.clone(), |attribute| {
                to_usize_range(&attribute.value_range)
            });
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::UnsupportedLocatorLabel,
            format!("unsupported citation locator label `{label}`"),
            range,
        ));
    }
    let item = diagnostics.is_empty().then(|| CitationItem {
        range: to_u64(item_range.start)..to_u64(item_range.end),
        key,
        label: explicit_label.unwrap_or_else(|| "page".to_owned()),
        locator,
        prefix: attribute_value(&attributes, "prefix").map(ToOwned::to_owned),
        suffix: attribute_value(&attributes, "suffix").map(ToOwned::to_owned),
        attributes,
    });
    ParsedItem { item, diagnostics }
}

fn parse_nocite(source: &str, start: usize, limit: usize) -> Candidate<NoCiteOccurrence> {
    let prefix_end = start + "nocite::".len();
    if !is_line_start(source, start) {
        return block_placement_candidate(source, start, limit, "nocite");
    }
    if !source[prefix_end..limit].starts_with('[') {
        return malformed_candidate(source, start, limit, "nocite macro has no attribute list");
    }
    let open = prefix_end;
    let Some(close) = find_closing_bracket(source, open, limit) else {
        return malformed_candidate(source, start, limit, "nocite attribute list is not closed");
    };
    let end = close + 1;
    if !line_tail_is_whitespace(source, end, limit) {
        return malformed_candidate(source, start, limit, "nocite macro must occupy one line");
    }
    let content = open + 1..close;
    let parts = split_comma_parts(source, content.clone()).unwrap_or_default();
    let mut keys = Vec::new();
    let mut diagnostics = Vec::new();
    for part in parts {
        let range = trim_range(source, part);
        let key = &source[range.clone()];
        if valid_citation_key(key) {
            keys.push(CitationKeyOccurrence {
                key: key.to_owned(),
                range: to_u64(range.start)..to_u64(range.end),
            });
        } else {
            diagnostics.push(diagnostic(
                if key.is_empty() {
                    CitationDiagnosticCode::EmptyCluster
                } else {
                    CitationDiagnosticCode::InvalidKey
                },
                "nocite key must match ^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$",
                range,
            ));
        }
    }
    if keys.is_empty() && diagnostics.is_empty() {
        diagnostics.push(diagnostic(
            CitationDiagnosticCode::EmptyCluster,
            "nocite requires at least one key",
            content,
        ));
    }
    Candidate {
        value: diagnostics.is_empty().then_some(NoCiteOccurrence {
            range: to_u64(start)..to_u64(end),
            keys,
        }),
        diagnostics,
        consumed_end: end,
    }
}

fn parse_bibliography(
    source: &str,
    start: usize,
    limit: usize,
) -> Candidate<BibliographyOccurrence> {
    let prefix_end = start + "bibliography::".len();
    if !is_line_start(source, start) {
        return block_placement_candidate(source, start, limit, "bibliography");
    }
    if !source[prefix_end..limit].starts_with('[') {
        return malformed_candidate(
            source,
            start,
            limit,
            "bibliography macro has no attribute list",
        );
    }
    let open = prefix_end;
    let Some(close) = find_closing_bracket(source, open, limit) else {
        return malformed_candidate(
            source,
            start,
            limit,
            "bibliography attribute list is not closed",
        );
    };
    let end = close + 1;
    if !line_tail_is_whitespace(source, end, limit) {
        return malformed_candidate(
            source,
            start,
            limit,
            "bibliography macro must occupy one line",
        );
    }
    let content = trim_range(source, open + 1..close);
    if content.is_empty() {
        return Candidate {
            value: Some(BibliographyOccurrence {
                range: to_u64(start)..to_u64(end),
                inclusion: BibliographyInclusion::Cited,
                attributes: Vec::new(),
            }),
            diagnostics: Vec::new(),
            consumed_end: end,
        };
    }
    let Some(equals) = find_unquoted_equals(source, content.clone()) else {
        return Candidate {
            value: None,
            diagnostics: vec![diagnostic(
                CitationDiagnosticCode::UnknownAttribute,
                "bibliography accepts only `include=all`",
                content,
            )],
            consumed_end: end,
        };
    };
    let name_range = trim_range(source, content.start..equals);
    let value_range = trim_range(source, equals + 1..content.end);
    let name = &source[name_range.clone()];
    let value = &source[value_range.clone()];
    if name != "include" {
        return Candidate {
            value: None,
            diagnostics: vec![diagnostic(
                CitationDiagnosticCode::UnknownAttribute,
                format!("unknown bibliography attribute `{name}`"),
                name_range,
            )],
            consumed_end: end,
        };
    }
    if value != "all" {
        return Candidate {
            value: None,
            diagnostics: vec![diagnostic(
                CitationDiagnosticCode::InvalidAttributeValue,
                "bibliography include value must be `all`",
                value_range,
            )],
            consumed_end: end,
        };
    }
    Candidate {
        value: Some(BibliographyOccurrence {
            range: to_u64(start)..to_u64(end),
            inclusion: BibliographyInclusion::All,
            attributes: vec![CitationAttribute {
                name: name.to_owned(),
                value: value.to_owned(),
                range: to_u64(content.start)..to_u64(content.end),
                name_range: to_u64(name_range.start)..to_u64(name_range.end),
                value_range: to_u64(value_range.start)..to_u64(value_range.end),
            }],
        }),
        diagnostics: Vec::new(),
        consumed_end: end,
    }
}

fn malformed_candidate<T>(source: &str, start: usize, limit: usize, message: &str) -> Candidate<T> {
    let end = line_end(source, start, limit);
    Candidate {
        value: None,
        diagnostics: vec![diagnostic(
            CitationDiagnosticCode::MalformedMacro,
            message,
            start..end,
        )],
        consumed_end: end,
    }
}

fn block_placement_candidate<T>(
    source: &str,
    start: usize,
    limit: usize,
    name: &str,
) -> Candidate<T> {
    let end = line_end(source, start, limit);
    Candidate {
        value: None,
        diagnostics: vec![diagnostic(
            CitationDiagnosticCode::BlockMacroPlacement,
            format!("{name} is a block macro and must begin its line"),
            start..end,
        )],
        consumed_end: end,
    }
}

fn attribute_value<'a>(attributes: &'a [CitationAttribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.as_str())
}

fn is_macro_boundary(source: &str, start: usize) -> bool {
    source[..start]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_alphanumeric() && !matches!(character, '_' | '-'))
}

fn is_line_start(source: &str, start: usize) -> bool {
    start == 0 || source.as_bytes().get(start.wrapping_sub(1)) == Some(&b'\n')
}

fn line_tail_is_whitespace(source: &str, end: usize, limit: usize) -> bool {
    source[end..line_end(source, end, limit)]
        .chars()
        .all(|character| matches!(character, ' ' | '\t'))
}

fn valid_citation_key(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn diagnostic(
    code: CitationDiagnosticCode,
    message: impl Into<String>,
    range: Range<usize>,
) -> CitationDiagnostic {
    CitationDiagnostic {
        code,
        message: message.into(),
        range: to_u64(range.start)..to_u64(range.end),
    }
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn to_usize_range(range: &Range<u64>) -> Range<usize> {
    usize::try_from(range.start).unwrap_or(usize::MAX)
        ..usize::try_from(range.end).unwrap_or(usize::MAX)
}
