use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::source_lexing::{
    decode_attribute_value, find_unquoted_equals, line_end, split_comma_parts, trim_range,
};
use crate::{NodeId, TaskNodeTemporal};

pub const QUERY_PROFILE_ID: &str = "weftext.query.v1";
pub const QUERY_EXPRESSION_CAPABILITY_ID: &str = "weftext.query-expression-subset.v0";
pub const QUERY_MAX_BODY_BYTES: usize = 16_384;
pub const QUERY_MAX_TOKENS: usize = 2_048;
pub const QUERY_MAX_EXPRESSION_NODES: usize = 256;
pub const QUERY_MAX_NESTING: usize = 32;
pub const QUERY_MAX_SORT_FIELDS: usize = 8;
pub const QUERY_MAX_PROJECTION_FIELDS: usize = 32;
pub const QUERY_MAX_IN_VALUES: usize = 64;
pub const QUERY_DEFAULT_LIMIT: u16 = 100;
pub const QUERY_MAX_LIMIT: u16 = 1_000;
pub const QUERY_MAX_ALIAS_BYTES: usize = 64;
pub const QUERY_MAX_CONTEXT_TEXT_BYTES: usize = 4_096;
pub const QUERY_MAX_STRING_LITERAL_BYTES: usize = 4_096;
pub const QUERY_MAX_OUTPUT_NAME_BYTES: usize = 64;
pub const QUERY_MAX_EVALUATION_STEPS: usize = 65_536;
pub const QUERY_MAX_RESULT_BYTES: usize = 4 * 1_024 * 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuerySource {
    Nodes,
    Tasks,
    Headings,
    Templates,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueryView {
    Table,
    List,
    TaskList,
    Board,
    Calendar,
    Timeline,
    Gallery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryNullPlacement {
    First,
    Last,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryValueType {
    String,
    Boolean,
    Number,
    Uuid,
    Temporal,
    Date,
    Instant,
    Duration,
    Null,
    List,
    Record,
    TaskKind,
    TaskState,
    Priority,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueryField {
    Id,
    Name,
    Path,
    ParentId,
    Depth,
    Kind,
    OwnerNodeId,
    OwnerNodeName,
    OwnerNodePath,
    OwnerNodeParentId,
    OwnerNodeDepth,
    OwnerNodeDisplayTitle,
    Description,
    Closed,
    State,
    ChecklistDepth,
    Priority,
    Created,
    Start,
    Scheduled,
    Due,
    ClosedAt,
    Blocked,
    DocumentTitle,
    DocumentSubtitle,
    DocumentDisplayTitle,
    NodeDisplayTitle,
    DocumentProperty,
    HeadingDocumentTitle,
    HeadingDocumentSubtitle,
    HeadingDocumentDisplayTitle,
    HeadingDocumentProperty,
    Title,
    Level,
    Anchor,
    HeadingParent,
    HeadingPath,
    PartCount,
    ParameterCount,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryFieldReference {
    pub alias: String,
    pub field: QueryField,
    pub custom_property: Option<String>,
    pub value_type: QueryValueType,
    pub nullable: bool,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum QueryScope {
    Workspace,
    SubtreeThisNode,
    DescendantsThisNode,
    SectionThisHeading,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryContextReference {
    ThisNodeId,
    ThisNodeName,
    ThisNodePath,
    ThisNodeDepth,
    ThisNodeDisplayTitle,
    ThisDocumentTitle,
    ThisDocumentSubtitle,
    ThisDocumentDisplayTitle,
    ThisDocumentProperty(String),
    ThisHeadingTitle,
    ThisHeadingLevel,
    ThisHeadingAnchor,
    ThisHeadingParent,
    ThisHeadingPath,
    ThisQueryTitle,
    ContextToday,
    ContextNow,
    ContextTimezone,
    ContextLocale,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryDocumentContext {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub properties: BTreeMap<String, String>,
    pub display_title: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryHeadingReference {
    pub title: String,
    pub level: u8,
    pub anchor: Option<String>,
    pub path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryHeadingContext {
    pub title: String,
    pub level: u8,
    pub anchor: Option<String>,
    pub parent: Option<QueryHeadingReference>,
    pub path: Vec<String>,
    pub range: Range<u64>,
    pub section_range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryBlockContext {
    pub title: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryNodeContext {
    pub id: NodeId,
    pub name: String,
    pub path: String,
    pub depth: u16,
    pub display_title: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryLexicalContext {
    pub node: Option<QueryNodeContext>,
    pub document: QueryDocumentContext,
    pub heading: Option<QueryHeadingContext>,
    pub query: QueryBlockContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Contains,
    StartsWith,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum QueryLiteral {
    String(String),
    Boolean(bool),
    Number(i64),
    Uuid(String),
    Temporal(TaskNodeTemporal),
    DurationDays(i32),
    Null,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueryValueExpressionKind {
    SourceField {
        reference: QueryFieldReference,
    },
    Literal {
        literal: QueryLiteral,
    },
    Context {
        reference: QueryContextReference,
    },
    DateOffset {
        base: Box<QueryValueExpression>,
        days: i32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryValueExpression {
    pub kind: QueryValueExpressionKind,
    pub value_type: QueryValueType,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueryExpressionKind {
    Boolean {
        value: QueryValueExpression,
    },
    Comparison {
        left: QueryValueExpression,
        operator: QueryComparisonOperator,
        right: QueryValueExpression,
    },
    In {
        left: QueryValueExpression,
        values: Vec<QueryValueExpression>,
    },
    IsNull {
        value: QueryValueExpression,
        negated: bool,
    },
    Not {
        expression: Box<QueryExpression>,
    },
    And {
        left: Box<QueryExpression>,
        right: Box<QueryExpression>,
    },
    Or {
        left: Box<QueryExpression>,
        right: Box<QueryExpression>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryExpression {
    pub kind: QueryExpressionKind,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySort {
    pub expression: QueryValueExpression,
    pub direction: QueryDirection,
    pub nulls: QueryNullPlacement,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProjection {
    pub expression: QueryValueExpression,
    pub output_name: String,
    pub output_explicit: bool,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryGroup {
    pub expression: QueryValueExpression,
    pub output_name: Option<String>,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPlan {
    pub expression_capability: String,
    pub source: QuerySource,
    pub alias: String,
    pub scope: QueryScope,
    pub filter: Option<QueryExpression>,
    pub sort: Vec<QuerySort>,
    pub group: Option<QueryGroup>,
    pub projection: Vec<QueryProjection>,
    pub limit: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryBlock {
    pub version: Option<u8>,
    pub source: Option<QuerySource>,
    pub view: Option<QueryView>,
    pub body: String,
    pub range: Range<u64>,
    pub header_range: Range<u64>,
    pub body_range: Range<u64>,
    pub lexical_context: QueryLexicalContext,
    pub plan: Option<QueryPlan>,
    pub valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryDiagnosticCode {
    MalformedBlock,
    UnterminatedBlock,
    UnknownAttribute,
    DuplicateAttribute,
    InvalidAttributeValue,
    MissingSource,
    MissingVersion,
    UnsupportedVersion,
    MissingFrom,
    MissingClause,
    InvalidAlias,
    AliasShadowing,
    AliasMismatch,
    MissingContext,
    DomainUnavailable,
    UnsupportedSource,
    UnsupportedView,
    ViewSourceMismatch,
    BodyTooLarge,
    TooManyTokens,
    InvalidToken,
    UnexpectedToken,
    DuplicateClause,
    ClauseOrder,
    UnknownField,
    TypeMismatch,
    InvalidLiteral,
    InvalidOperator,
    TooManyExpressionNodes,
    NestingTooDeep,
    TooManySortFields,
    TooManyProjectionFields,
    TooManyInValues,
    DuplicateField,
    DuplicateOutputName,
    DuplicateValue,
    NullComparison,
    StringTooLarge,
    InvalidLimit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryDiagnostic {
    pub code: QueryDiagnosticCode,
    pub message: String,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySourceAnalysis {
    pub blocks: Vec<QueryBlock>,
    pub diagnostics: Vec<QueryDiagnostic>,
}

/// Parses canonical `[.weftext-query,version=1,...]` literal blocks and their typed v1 bodies from exact `AsciiDoc`
/// source. All ranges are UTF-8 byte ranges and no query is executed.
#[must_use]
pub fn analyze_query_source(source: &str) -> QuerySourceAnalysis {
    let document = weftext_asciidoc::analyze(source);
    let protected = &document.protected_ranges;
    let mut analysis = QuerySourceAnalysis::default();
    let mut line_start = 0_usize;
    while line_start < source.len() {
        let line_stop = line_end(source, line_start, source.len());
        let line = trim_horizontal(source, line_start..line_stop);
        if !contains_offset(protected, line.start) && query_header_prefix(source, &line) {
            let (block, diagnostics, next) = parse_query_block(source, line, line_stop, &document);
            analysis.blocks.push(block);
            analysis.diagnostics.extend(diagnostics);
            line_start = next;
        } else {
            line_start = next_line_start(source, line_stop);
        }
    }
    analysis
}

fn parse_query_block(
    source: &str,
    header_range: Range<usize>,
    header_line_end: usize,
    document: &weftext_asciidoc::Analysis,
) -> (QueryBlock, Vec<QueryDiagnostic>, usize) {
    let mut diagnostics = Vec::new();
    let (version, view) = parse_query_header(source, header_range.clone(), &mut diagnostics);
    let lexical_context = query_lexical_context(source, &header_range, document);
    let opening_start = next_line_start(source, header_line_end);
    let opening_end = line_end(source, opening_start, source.len());
    if opening_start >= source.len() || &source[opening_start..opening_end] != "...." {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::MalformedBlock,
            "query attributes must be followed immediately by an unindented `....` delimiter",
            header_range.clone(),
        ));
        let block = QueryBlock {
            version,
            source: None,
            view,
            body: String::new(),
            range: to_u64_range(header_range.clone()),
            header_range: to_u64_range(header_range),
            body_range: to_u64_range(header_line_end..header_line_end),
            lexical_context,
            plan: None,
            valid: false,
        };
        return (block, diagnostics, next_line_start(source, header_line_end));
    }

    let body_start = next_line_start(source, opening_end);
    let mut cursor = body_start;
    let mut closing = None;
    while cursor < source.len() {
        let end = line_end(source, cursor, source.len());
        if &source[cursor..end] == "...." {
            closing = Some((cursor, end));
            break;
        }
        cursor = next_line_start(source, end);
    }
    let (body_end, block_end, next) = if let Some((closing_start, closing_end)) = closing {
        (
            closing_start,
            closing_end,
            next_line_start(source, closing_end),
        )
    } else {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::UnterminatedBlock,
            "query literal block is not closed",
            opening_start..source.len(),
        ));
        (source.len(), source.len(), source.len())
    };
    let body_range = body_start..body_end;
    let diagnostic_count = diagnostics.len();
    let plan = if body_range.len() > QUERY_MAX_BODY_BYTES {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::BodyTooLarge,
            format!("query body exceeds {QUERY_MAX_BODY_BYTES} bytes"),
            body_range.clone(),
        ));
        None
    } else {
        parse_query_body(source, body_range.clone(), &mut diagnostics)
    };
    let query_source = plan.as_ref().map(|plan| plan.source);
    if view == Some(QueryView::TaskList)
        && query_source.is_some_and(|value| value != QuerySource::Tasks)
    {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::ViewSourceMismatch,
            "the `task-list` presentation requires the `tasks` domain",
            header_range.clone(),
        ));
    }
    let valid = closing.is_some()
        && diagnostic_count == 0
        && diagnostics.is_empty()
        && version == Some(1)
        && plan.is_some();
    (
        QueryBlock {
            version,
            source: query_source,
            view,
            body: source[body_range.clone()].to_owned(),
            range: to_u64_range(header_range.start..block_end),
            header_range: to_u64_range(header_range),
            body_range: to_u64_range(body_range),
            lexical_context,
            plan: valid.then_some(plan).flatten(),
            valid,
        },
        diagnostics,
        next,
    )
}

#[allow(clippy::too_many_lines)]
fn parse_query_header(
    source: &str,
    range: Range<usize>,
    diagnostics: &mut Vec<QueryDiagnostic>,
) -> (Option<u8>, Option<QueryView>) {
    if !source[range.clone()].ends_with(']') {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::MalformedBlock,
            "query attribute list is not closed on its line",
            range,
        ));
        return (None, None);
    }
    let inner = range.start + 1..range.end - 1;
    let Ok(parts) = split_comma_parts(source, inner) else {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::MalformedBlock,
            "query attribute list contains an invalid quoted value",
            range,
        ));
        return (None, None);
    };
    if parts.is_empty() || &source[trim_range(source, parts[0].clone())] != ".weftext-query" {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::MalformedBlock,
            "`.weftext-query` must be the first block role",
            range,
        ));
        return (None, None);
    }
    let mut seen = BTreeSet::new();
    let mut version = None;
    let mut view = None;
    for part in parts.into_iter().skip(1) {
        let part = trim_range(source, part);
        let Some(equals) = find_unquoted_equals(source, part.clone()) else {
            diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::UnknownAttribute,
                "query block accepts named `version` and `view` attributes only",
                part,
            ));
            continue;
        };
        let name_range = trim_range(source, part.start..equals);
        let value_range = trim_range(source, equals + 1..part.end);
        let name = &source[name_range.clone()];
        if !matches!(name, "version" | "view") {
            diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::UnknownAttribute,
                format!("unknown query block attribute `{name}`"),
                name_range,
            ));
            continue;
        }
        if !seen.insert(name) {
            diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::DuplicateAttribute,
                format!("duplicate query block attribute `{name}`"),
                name_range,
            ));
            continue;
        }
        let Some(value) = decode_attribute_value(source, value_range.clone()) else {
            diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidAttributeValue,
                format!("query block attribute `{name}` has an invalid literal value"),
                value_range,
            ));
            continue;
        };
        match name {
            "version" => {
                version = if value == "1" {
                    Some(1)
                } else {
                    diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::UnsupportedVersion,
                        format!("unsupported query version `{value}`"),
                        value_range,
                    ));
                    None
                };
            }
            "view" => {
                view = match value.as_str() {
                    "table" => Some(QueryView::Table),
                    "list" => Some(QueryView::List),
                    "task-list" => Some(QueryView::TaskList),
                    "board" => Some(QueryView::Board),
                    "calendar" => Some(QueryView::Calendar),
                    "timeline" => Some(QueryView::Timeline),
                    "gallery" => Some(QueryView::Gallery),
                    _ => {
                        diagnostics.push(query_diagnostic(
                            QueryDiagnosticCode::UnsupportedView,
                            format!("unsupported initial query view `{value}`"),
                            value_range,
                        ));
                        None
                    }
                };
            }
            _ => unreachable!(),
        }
    }
    if !seen.contains("version") {
        diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::MissingVersion,
            "query block requires `version=1`",
            range,
        ));
    }
    (version, view)
}

fn query_header_prefix(source: &str, range: &Range<usize>) -> bool {
    let value = &source[range.clone()];
    value.starts_with("[.weftext-query")
        && value
            .as_bytes()
            .get("[.weftext-query".len())
            .is_some_and(|byte| matches!(byte, b',' | b']'))
}

fn trim_horizontal(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && matches!(source.as_bytes()[range.start], b' ' | b'\t') {
        range.start += 1;
    }
    while range.start < range.end && matches!(source.as_bytes()[range.end - 1], b' ' | b'\t') {
        range.end -= 1;
    }
    range
}

fn next_line_start(source: &str, end: usize) -> usize {
    match source.as_bytes().get(end..) {
        Some([b'\r', b'\n', ..]) => end + 2,
        Some([b'\r' | b'\n', ..]) => end + 1,
        _ => source.len(),
    }
}

fn contains_offset(ranges: &[Range<u64>], offset: usize) -> bool {
    let offset = u64::try_from(offset).unwrap_or(u64::MAX);
    ranges
        .iter()
        .any(|range| range.start <= offset && offset < range.end)
}

fn query_diagnostic(
    code: QueryDiagnosticCode,
    message: impl Into<String>,
    range: Range<usize>,
) -> QueryDiagnostic {
    QueryDiagnostic {
        code,
        message: message.into(),
        range: to_u64_range(range),
    }
}

fn to_u64_range(range: Range<usize>) -> Range<u64> {
    u64::try_from(range.start).unwrap_or(u64::MAX)..u64::try_from(range.end).unwrap_or(u64::MAX)
}

fn query_lexical_context(
    source: &str,
    header_range: &Range<usize>,
    analysis: &weftext_asciidoc::Analysis,
) -> QueryLexicalContext {
    let document = query_document_context_from_analysis(analysis);
    let header_start = u64::try_from(header_range.start).unwrap_or(u64::MAX);
    let heading = query_heading_contexts_from_analysis(source, analysis)
        .into_iter()
        .rfind(|heading| {
            heading.range.start < header_start
                && header_start >= heading.section_range.start
                && header_start < heading.section_range.end
        });
    let header_line_end = line_end(source, header_range.start, source.len());
    let opening_start = next_line_start(source, header_line_end);
    let query_title = analysis
        .blocks
        .iter()
        .find(|block| {
            block.kind == weftext_asciidoc::BlockKind::Literal
                && block.range.start == u64::try_from(opening_start).unwrap_or(u64::MAX)
                && block.roles.iter().any(|role| role == "weftext-query")
        })
        .and_then(|block| block.title.clone());
    QueryLexicalContext {
        node: None,
        document,
        heading,
        query: QueryBlockContext { title: query_title },
    }
}

pub(crate) fn analyze_query_document_context(
    source: &str,
) -> (QueryDocumentContext, Vec<QueryHeadingContext>) {
    let analysis = weftext_asciidoc::analyze(source);
    (
        query_document_context_from_analysis(&analysis),
        query_heading_contexts_from_analysis(source, &analysis),
    )
}

fn query_document_context_from_analysis(
    analysis: &weftext_asciidoc::Analysis,
) -> QueryDocumentContext {
    use weftext_asciidoc::BlockKind;

    let title = analysis
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::DocumentTitle)
        .map(|block| block.text.clone());
    let subtitle = analysis
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::DocumentSubtitle)
        .map(|block| block.text.clone());
    let properties = analysis
        .document_header
        .attributes
        .iter()
        .filter(|attribute| attribute.projected)
        .filter_map(|attribute| {
            attribute
                .literal_value
                .as_ref()
                .map(|value| (attribute.name.clone(), value.clone()))
        })
        .collect();
    QueryDocumentContext {
        display_title: title.clone(),
        title,
        subtitle,
        properties,
    }
}

fn query_heading_contexts_from_analysis(
    source: &str,
    analysis: &weftext_asciidoc::Analysis,
) -> Vec<QueryHeadingContext> {
    use weftext_asciidoc::BlockKind;

    let headings = analysis
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| {
            block.kind == BlockKind::Heading
                && block
                    .heading_level
                    .is_some_and(|level| (1..=9).contains(&level))
        })
        .collect::<Vec<_>>();
    let mut stack = Vec::<&weftext_asciidoc::Block>::new();
    let mut contexts = Vec::new();
    for (heading_index, block) in &headings {
        let level = block.heading_level.expect("filtered body heading");
        while stack
            .last()
            .is_some_and(|parent| parent.heading_level.is_some_and(|value| value >= level))
        {
            stack.pop();
        }
        stack.push(block);
        let path = stack.clone();
        let section_end = analysis
            .blocks
            .iter()
            .skip(*heading_index + 1)
            .find(|candidate| {
                candidate.kind == BlockKind::Heading
                    && candidate.heading_level.is_some_and(|value| value <= level)
            })
            .map_or_else(
                || u64::try_from(source.len()).unwrap_or(u64::MAX),
                |candidate| candidate.range.start,
            );
        contexts.push(QueryHeadingContext {
            title: block.text.clone(),
            level,
            anchor: block.block_id.clone(),
            parent: (path.len() >= 2).then(|| {
                let parent = path[path.len() - 2];
                QueryHeadingReference {
                    title: parent.text.clone(),
                    level: parent.heading_level.expect("body heading parent"),
                    anchor: parent.block_id.clone(),
                    path: path[..path.len().saturating_sub(1)]
                        .iter()
                        .map(|heading| heading.text.clone())
                        .collect(),
                }
            }),
            path: path.iter().map(|heading| heading.text.clone()).collect(),
            range: block.range.clone(),
            section_range: block.range.start..section_end,
        });
    }
    contexts
}

// The typed body parser follows below. Keeping block recognition and body parsing in one module
// ensures there is one portable query grammar authority.
fn parse_query_body(
    source: &str,
    range: Range<usize>,
    diagnostics: &mut Vec<QueryDiagnostic>,
) -> Option<QueryPlan> {
    QueryParser::new(source, range, diagnostics).parse()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueryToken {
    kind: QueryTokenKind,
    range: Range<usize>,
    line_start: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum QueryTokenKind {
    Word(String),
    String(String),
    Integer(String),
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Plus,
    Minus,
    Comma,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    End,
}

#[allow(clippy::too_many_lines)]
fn lex_query_body(
    source: &str,
    range: Range<usize>,
    diagnostics: &mut Vec<QueryDiagnostic>,
) -> Vec<QueryToken> {
    let mut tokens = Vec::new();
    let mut cursor = range.start;
    let mut at_line_start = true;
    while cursor < range.end {
        let byte = source.as_bytes()[cursor];
        match byte {
            b' ' | b'\t' => {
                cursor += 1;
            }
            b'\r' | b'\n' => {
                cursor = next_line_start(source, cursor);
                at_line_start = true;
            }
            b'#' => {
                cursor = line_end(source, cursor, range.end);
            }
            b'"' => {
                let start = cursor;
                cursor += 1;
                let mut escaped = false;
                let mut close = None;
                for (relative, character) in source[cursor..range.end].char_indices() {
                    let index = cursor + relative;
                    if matches!(character, '\n' | '\r') {
                        break;
                    }
                    if escaped {
                        escaped = false;
                    } else if character == '\\' {
                        escaped = true;
                    } else if character == '"' {
                        close = Some(index);
                        break;
                    }
                }
                let Some(close) = close else {
                    diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::InvalidToken,
                        "query string literal is not closed on its line",
                        start..line_end(source, start, range.end),
                    ));
                    cursor = line_end(source, start, range.end);
                    continue;
                };
                let end = close + 1;
                match serde_json::from_str::<String>(&source[start..end]) {
                    Ok(value) if value.len() <= QUERY_MAX_STRING_LITERAL_BYTES => {
                        tokens.push(QueryToken {
                            kind: QueryTokenKind::String(value),
                            range: start..end,
                            line_start: at_line_start,
                        });
                    }
                    Ok(_) => diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::StringTooLarge,
                        format!(
                            "decoded query string literal exceeds {QUERY_MAX_STRING_LITERAL_BYTES} bytes"
                        ),
                        start..end,
                    )),
                    Err(_) => diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::InvalidToken,
                        "query string must use JSON-compatible escapes",
                        start..end,
                    )),
                }
                cursor = end;
                at_line_start = false;
            }
            b'0'..=b'9' => {
                let start = cursor;
                while cursor < range.end
                    && (source.as_bytes()[cursor].is_ascii_alphanumeric()
                        || source.as_bytes()[cursor] == b'-')
                {
                    cursor += 1;
                }
                let value = &source[start..cursor];
                tokens.push(QueryToken {
                    kind: if value.bytes().all(|value| value.is_ascii_digit()) {
                        QueryTokenKind::Integer(value.to_owned())
                    } else {
                        QueryTokenKind::Word(value.to_owned())
                    },
                    range: start..cursor,
                    line_start: at_line_start,
                });
                at_line_start = false;
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                let start = cursor;
                while cursor < range.end
                    && (source.as_bytes()[cursor].is_ascii_alphanumeric()
                        || matches!(source.as_bytes()[cursor], b'-' | b'_' | b'.'))
                {
                    cursor += 1;
                }
                tokens.push(QueryToken {
                    kind: QueryTokenKind::Word(source[start..cursor].to_owned()),
                    range: start..cursor,
                    line_start: at_line_start,
                });
                at_line_start = false;
            }
            b'=' => push_symbol(
                &mut tokens,
                QueryTokenKind::Equal,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'!' if source.as_bytes().get(cursor + 1) == Some(&b'=') => push_wide_symbol(
                &mut tokens,
                QueryTokenKind::NotEqual,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'<' if source.as_bytes().get(cursor + 1) == Some(&b'=') => push_wide_symbol(
                &mut tokens,
                QueryTokenKind::LessThanOrEqual,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'>' if source.as_bytes().get(cursor + 1) == Some(&b'=') => push_wide_symbol(
                &mut tokens,
                QueryTokenKind::GreaterThanOrEqual,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'<' => push_symbol(
                &mut tokens,
                QueryTokenKind::LessThan,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'>' => push_symbol(
                &mut tokens,
                QueryTokenKind::GreaterThan,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'+' => push_symbol(
                &mut tokens,
                QueryTokenKind::Plus,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'-' => push_symbol(
                &mut tokens,
                QueryTokenKind::Minus,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b',' => push_symbol(
                &mut tokens,
                QueryTokenKind::Comma,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'(' => push_symbol(
                &mut tokens,
                QueryTokenKind::LeftParen,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b')' => push_symbol(
                &mut tokens,
                QueryTokenKind::RightParen,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b'[' => push_symbol(
                &mut tokens,
                QueryTokenKind::LeftBracket,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            b']' => push_symbol(
                &mut tokens,
                QueryTokenKind::RightBracket,
                cursor,
                &mut cursor,
                &mut at_line_start,
            ),
            _ => {
                let character = source[cursor..range.end]
                    .chars()
                    .next()
                    .expect("cursor is before query range end");
                let end = cursor + character.len_utf8();
                diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidToken,
                    format!("unsupported query token `{character}`"),
                    cursor..end,
                ));
                cursor = end;
                at_line_start = false;
            }
        }
        if tokens.len() > QUERY_MAX_TOKENS {
            diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::TooManyTokens,
                format!("query body exceeds {QUERY_MAX_TOKENS} tokens"),
                range.clone(),
            ));
            tokens.clear();
            break;
        }
    }
    tokens.push(QueryToken {
        kind: QueryTokenKind::End,
        range: range.end..range.end,
        line_start: at_line_start,
    });
    tokens
}

fn push_symbol(
    tokens: &mut Vec<QueryToken>,
    kind: QueryTokenKind,
    start: usize,
    cursor: &mut usize,
    at_line_start: &mut bool,
) {
    *cursor += 1;
    tokens.push(QueryToken {
        kind,
        range: start..*cursor,
        line_start: *at_line_start,
    });
    *at_line_start = false;
}

fn push_wide_symbol(
    tokens: &mut Vec<QueryToken>,
    kind: QueryTokenKind,
    start: usize,
    cursor: &mut usize,
    at_line_start: &mut bool,
) {
    *cursor += 2;
    tokens.push(QueryToken {
        kind,
        range: start..*cursor,
        line_start: *at_line_start,
    });
    *at_line_start = false;
}

struct QueryParser<'a> {
    tokens: Vec<QueryToken>,
    position: usize,
    query_source: Option<QuerySource>,
    alias: Option<String>,
    diagnostics: &'a mut Vec<QueryDiagnostic>,
    expression_nodes: usize,
    nesting: usize,
}

fn source_field_expression(reference: QueryFieldReference) -> QueryValueExpression {
    QueryValueExpression {
        value_type: reference.value_type,
        range: reference.range.clone(),
        kind: QueryValueExpressionKind::SourceField { reference },
    }
}

impl<'a> QueryParser<'a> {
    fn new(source: &str, range: Range<usize>, diagnostics: &'a mut Vec<QueryDiagnostic>) -> Self {
        Self {
            tokens: lex_query_body(source, range, diagnostics),
            position: 0,
            query_source: None,
            alias: None,
            diagnostics,
            expression_nodes: 0,
            nesting: 0,
        }
    }

    fn parse(mut self) -> Option<QueryPlan> {
        if self.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.code,
                QueryDiagnosticCode::InvalidToken | QueryDiagnosticCode::TooManyTokens
            )
        }) {
            return None;
        }
        let mut scope = QueryScope::Workspace;
        let mut filter = None;
        let mut sort = Vec::new();
        let mut group = None;
        let mut projection = Vec::new();
        let mut limit = QUERY_DEFAULT_LIMIT;
        let mut seen = BTreeSet::new();
        let mut last_rank = 0_u8;
        while !matches!(self.current().kind, QueryTokenKind::End) {
            let Some((clause, rank)) = self.current_clause() else {
                self.unexpected("expected a query clause at the beginning of a line");
                return None;
            };
            if !self.current().line_start {
                self.unexpected("every query clause must begin on a new line");
                return None;
            }
            let clause_range = self.advance().range.clone();
            if !seen.insert(clause) {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::DuplicateClause,
                    format!("query clause `{clause}` occurs more than once"),
                    clause_range.clone(),
                ));
            }
            if rank < last_rank {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::ClauseOrder,
                    "query clauses must use `from`, `scope`, `where`, `group by`, `select`, `order by`, `limit` order",
                    clause_range,
                ));
            }
            last_rank = last_rank.max(rank);
            match clause {
                "from" => {
                    if let Some((source, alias)) = self.parse_from() {
                        self.query_source = Some(source);
                        self.alias = Some(alias);
                    }
                }
                "scope" => {
                    if let Some(value) = self.parse_scope() {
                        scope = value;
                    }
                }
                "where" => filter = self.parse_or(),
                "group" => group = self.parse_group(),
                "order" => sort = self.parse_order(),
                "select" => projection = self.parse_projection(),
                "limit" => {
                    if let Some(value) = self.parse_limit() {
                        limit = value;
                    }
                }
                _ => unreachable!(),
            }
        }
        let Some(source) = self.query_source else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::MissingFrom,
                "query body requires `from <domain> as <alias>`",
                self.current().range.clone(),
            ));
            return None;
        };
        let alias = self.alias.clone()?;
        for required in ["scope", "where", "select", "order", "limit"] {
            if !seen.contains(required) {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::MissingClause,
                    format!("query body requires the `{required}` clause"),
                    self.current().range.clone(),
                ));
            }
        }
        self.check_group_output_collision(group.as_ref(), &projection);
        Some(QueryPlan {
            expression_capability: QUERY_EXPRESSION_CAPABILITY_ID.to_owned(),
            source,
            alias,
            scope,
            filter,
            sort,
            group,
            projection,
            limit,
        })
    }

    fn check_group_output_collision(
        &mut self,
        group: Option<&QueryGroup>,
        projection: &[QueryProjection],
    ) {
        let Some(group) = group else {
            return;
        };
        let Some(group_output_name) = query_group_output_name(group) else {
            return;
        };
        if projection
            .iter()
            .any(|projection| projection.output_name == group_output_name)
        {
            self.diagnostics.push(QueryDiagnostic {
                code: QueryDiagnosticCode::DuplicateOutputName,
                message: format!(
                    "group output name `{group_output_name}` conflicts with a projected output name"
                ),
                range: group.range.clone(),
            });
        }
    }

    fn parse_from(&mut self) -> Option<(QuerySource, String)> {
        let domain = self.advance().clone();
        let QueryTokenKind::Word(domain_name) = &domain.kind else {
            self.unexpected(
                "`from` requires the `nodes`, `tasks`, `headings`, or `templates` domain",
            );
            return None;
        };
        let source = match domain_name.as_str() {
            "nodes" => QuerySource::Nodes,
            "tasks" => QuerySource::Tasks,
            "headings" => QuerySource::Headings,
            "templates" => QuerySource::Templates,
            _ => {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnsupportedSource,
                    format!("unsupported query domain `{domain_name}`"),
                    domain.range,
                ));
                return None;
            }
        };
        if self.consume_word("as").is_none() {
            self.unexpected("query domain requires an explicit `as <alias>` binding");
            return None;
        }
        let alias = self.advance().clone();
        let QueryTokenKind::Word(alias_name) = &alias.kind else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidAlias,
                "query alias must be a bounded lowercase ASCII identifier",
                alias.range,
            ));
            return None;
        };
        if is_reserved_query_identifier(alias_name) {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::AliasShadowing,
                "query alias cannot shadow a context root, domain, clause, operator, or function name",
                alias.range,
            ));
            return None;
        }
        if !valid_query_alias(alias_name) {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidAlias,
                "query alias must be a bounded lowercase ASCII identifier",
                alias.range,
            ));
            return None;
        }
        Some((source, alias_name.clone()))
    }

    fn parse_scope(&mut self) -> Option<QueryScope> {
        let token = self.advance().clone();
        let QueryTokenKind::Word(value) = &token.kind else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::UnexpectedToken,
                "scope requires `workspace`, `subtree(this.node)`, `descendants(this.node)`, or `section(this.heading)`",
                token.range,
            ));
            return None;
        };
        let scope = match value.as_str() {
            "workspace" => Some(QueryScope::Workspace),
            "subtree" => self.parse_scope_call("this.node", QueryScope::SubtreeThisNode),
            "descendants" => self.parse_scope_call("this.node", QueryScope::DescendantsThisNode),
            "section" => self.parse_scope_call("this.heading", QueryScope::SectionThisHeading),
            _ => {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    format!("unsupported query scope `{value}`"),
                    token.range.clone(),
                ));
                None
            }
        };
        if self.query_source == Some(QuerySource::Templates)
            && scope
                .as_ref()
                .is_some_and(|scope| *scope != QueryScope::Workspace)
        {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::DomainUnavailable,
                "the unavailable `templates` domain accepts only `scope workspace`",
                token.range,
            ));
            return None;
        }
        scope
    }

    fn parse_scope_call(&mut self, expected: &str, scope: QueryScope) -> Option<QueryScope> {
        let open = self.advance().clone();
        let reference = self.advance().clone();
        let close = self.advance().clone();
        if !matches!(open.kind, QueryTokenKind::LeftParen)
            || !matches!(&reference.kind, QueryTokenKind::Word(value) if value == expected)
            || !matches!(close.kind, QueryTokenKind::RightParen)
        {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidLiteral,
                format!("scope requires exactly `{expected}`"),
                open.range.start..close.range.end,
            ));
            None
        } else {
            Some(scope)
        }
    }

    fn parse_group(&mut self) -> Option<QueryGroup> {
        if self.consume_word("by").is_none() {
            self.unexpected("`group` must be followed by `by`");
            return None;
        }
        let field = self.parse_field()?;
        let start = field.range.start;
        let mut end = field.range.end;
        let output_name = if self.consume_word("as").is_some() {
            let (name, range) = self.parse_output_name()?;
            end = u64::try_from(range.end).unwrap_or(u64::MAX);
            Some(name)
        } else {
            None
        };
        Some(QueryGroup {
            expression: source_field_expression(field),
            output_name,
            range: start..end,
        })
    }

    fn parse_order(&mut self) -> Vec<QuerySort> {
        if self.consume_word("by").is_none() {
            self.unexpected("`order` must be followed by `by`");
            return Vec::new();
        }
        let mut values = Vec::new();
        let mut seen = BTreeSet::new();
        while let Some(field) = self.parse_field() {
            if !seen.insert((field.field, field.custom_property.clone())) {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::DuplicateField,
                    message: "a sort field may occur only once".to_owned(),
                    range: field.range.clone(),
                });
            }
            let mut nulls = QueryNullPlacement::Last;
            let Some((direction, direction_range)) = self.parse_optional_direction() else {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "every `order by` key requires explicit `asc` or `desc`",
                    self.current().range.clone(),
                ));
                return Vec::new();
            };
            let mut end = direction_range.end;
            if self.consume_word("nulls").is_some() {
                let placement = self.advance().clone();
                match &placement.kind {
                    QueryTokenKind::Word(value) if value == "first" => {
                        nulls = QueryNullPlacement::First;
                    }
                    QueryTokenKind::Word(value) if value == "last" => {
                        nulls = QueryNullPlacement::Last;
                    }
                    _ => self.diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::UnexpectedToken,
                        "`nulls` requires `first` or `last`",
                        placement.range.clone(),
                    )),
                }
                end = placement.range.end;
            }
            values.push(QuerySort {
                range: field.range.start..u64::try_from(end).unwrap_or(u64::MAX),
                expression: source_field_expression(field),
                direction,
                nulls,
            });
            if values.len() > QUERY_MAX_SORT_FIELDS {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::TooManySortFields,
                    format!("query accepts at most {QUERY_MAX_SORT_FIELDS} sort fields"),
                    self.current().range.clone(),
                ));
                break;
            }
            if !matches!(self.current().kind, QueryTokenKind::Comma) {
                break;
            }
            self.advance();
        }
        values
    }

    fn parse_projection(&mut self) -> Vec<QueryProjection> {
        let mut values = Vec::new();
        let mut seen_fields = BTreeSet::new();
        let mut seen_outputs = BTreeSet::new();
        while let Some(field) = self.parse_field() {
            if !seen_fields.insert((field.field, field.custom_property.clone())) {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::DuplicateField,
                    message: "a projected field may occur only once".to_owned(),
                    range: field.range.clone(),
                });
            }
            let start = field.range.start;
            let mut end = field.range.end;
            let explicit_output = if self.consume_word("as").is_some() {
                let Some((name, range)) = self.parse_output_name() else {
                    break;
                };
                end = u64::try_from(range.end).unwrap_or(u64::MAX);
                Some(name)
            } else {
                None
            };
            if field.custom_property.is_some() && explicit_output.is_none() {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::InvalidAlias,
                    message: "document property projections require an explicit `as snake_case_output` name".to_owned(),
                    range: field.range.clone(),
                });
            }
            let output_explicit = explicit_output.is_some();
            let output_name = explicit_output
                .unwrap_or_else(|| default_projection_output_name(field.field).to_owned());
            if !seen_outputs.insert(output_name.clone()) {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::DuplicateOutputName,
                    message: format!("projected output name `{output_name}` occurs more than once"),
                    range: start..end,
                });
            }
            values.push(QueryProjection {
                expression: source_field_expression(field),
                output_name,
                output_explicit,
                range: start..end,
            });
            if values.len() > QUERY_MAX_PROJECTION_FIELDS {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::TooManyProjectionFields,
                    format!("query accepts at most {QUERY_MAX_PROJECTION_FIELDS} projected fields"),
                    self.current().range.clone(),
                ));
                break;
            }
            if !matches!(self.current().kind, QueryTokenKind::Comma) {
                break;
            }
            self.advance();
        }
        values
    }

    fn parse_output_name(&mut self) -> Option<(String, Range<usize>)> {
        let token = self.advance().clone();
        let QueryTokenKind::Word(name) = &token.kind else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidAlias,
                "projection output name must be a bounded lowercase snake_case identifier",
                token.range,
            ));
            return None;
        };
        if !valid_query_output_name(name) {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidAlias,
                "projection output name must be a bounded lowercase snake_case identifier",
                token.range,
            ));
            return None;
        }
        Some((name.clone(), token.range))
    }

    fn parse_limit(&mut self) -> Option<u16> {
        let token = self.advance().clone();
        let QueryTokenKind::Integer(value) = &token.kind else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidLimit,
                format!("query limit must be an integer from 1 to {QUERY_MAX_LIMIT}"),
                token.range,
            ));
            return None;
        };
        let value = value.parse::<u16>().ok();
        if value.is_none_or(|value| value == 0 || value > QUERY_MAX_LIMIT) {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::InvalidLimit,
                format!("query limit must be an integer from 1 to {QUERY_MAX_LIMIT}"),
                token.range,
            ));
            None
        } else {
            value
        }
    }

    fn parse_optional_direction(&mut self) -> Option<(QueryDirection, Range<usize>)> {
        match &self.current().kind {
            QueryTokenKind::Word(value) if value == "asc" => {
                let range = self.advance().range.clone();
                Some((QueryDirection::Ascending, range))
            }
            QueryTokenKind::Word(value) if value == "desc" => {
                let range = self.advance().range.clone();
                Some((QueryDirection::Descending, range))
            }
            _ => None,
        }
    }

    fn parse_or(&mut self) -> Option<QueryExpression> {
        let mut expression = self.parse_and()?;
        while self.consume_word("or").is_some() {
            let right = self.parse_and()?;
            let range = expression.range.start..right.range.end;
            expression = self.expression(
                QueryExpressionKind::Or {
                    left: Box::new(expression),
                    right: Box::new(right),
                },
                range,
            )?;
        }
        Some(expression)
    }

    fn parse_and(&mut self) -> Option<QueryExpression> {
        let mut expression = self.parse_not()?;
        while self.consume_word("and").is_some() {
            let right = self.parse_not()?;
            let range = expression.range.start..right.range.end;
            expression = self.expression(
                QueryExpressionKind::And {
                    left: Box::new(expression),
                    right: Box::new(right),
                },
                range,
            )?;
        }
        Some(expression)
    }

    fn parse_not(&mut self) -> Option<QueryExpression> {
        if let Some(token) = self.consume_word("not") {
            let expression = self.parse_not()?;
            let range = to_u64_range(token.range.start..to_usize(expression.range.end));
            return self.expression(
                QueryExpressionKind::Not {
                    expression: Box::new(expression),
                },
                range,
            );
        }
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> Option<QueryExpression> {
        if matches!(&self.current().kind, QueryTokenKind::Word(value) if matches!(value.as_str(), "true" | "false"))
        {
            let token = self.advance().clone();
            let QueryTokenKind::Word(value) = token.kind else {
                unreachable!();
            };
            let range = to_u64_range(token.range);
            return self.expression(
                QueryExpressionKind::Boolean {
                    value: QueryValueExpression {
                        kind: QueryValueExpressionKind::Literal {
                            literal: QueryLiteral::Boolean(value == "true"),
                        },
                        value_type: QueryValueType::Boolean,
                        range: range.clone(),
                    },
                },
                range,
            );
        }
        if matches!(&self.current().kind, QueryTokenKind::Word(value) if matches!(value.as_str(), "contains" | "starts_with"))
        {
            return self.parse_pure_function_predicate();
        }
        if matches!(self.current().kind, QueryTokenKind::LeftParen) {
            let open = self.advance().range.clone();
            self.nesting += 1;
            if self.nesting > QUERY_MAX_NESTING {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::NestingTooDeep,
                    format!("query expression nesting exceeds {QUERY_MAX_NESTING}"),
                    open,
                ));
                return None;
            }
            let mut expression = self.parse_or()?;
            self.nesting -= 1;
            let close = self.advance().clone();
            if !matches!(close.kind, QueryTokenKind::RightParen) {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "query expression is missing `)`",
                    close.range,
                ));
                return None;
            }
            expression.range.end = u64::try_from(close.range.end).unwrap_or(u64::MAX);
            return Some(expression);
        }
        self.parse_predicate()
    }

    fn parse_pure_function_predicate(&mut self) -> Option<QueryExpression> {
        let function = self.advance().clone();
        let QueryTokenKind::Word(name) = &function.kind else {
            unreachable!();
        };
        let operator = match name.as_str() {
            "contains" => QueryComparisonOperator::Contains,
            "starts_with" => QueryComparisonOperator::StartsWith,
            _ => unreachable!(),
        };
        let open = self.advance().clone();
        if !matches!(open.kind, QueryTokenKind::LeftParen) {
            self.unexpected("pure query function requires `(`");
            return None;
        }
        let field = self.parse_field()?;
        let comma = self.advance().clone();
        if !matches!(comma.kind, QueryTokenKind::Comma) {
            self.unexpected("pure query function requires two comma-separated arguments");
            return None;
        }
        let mut value = self.parse_value()?;
        self.coerce_value(&field, &mut value, operator);
        let close = self.advance().clone();
        if !matches!(close.kind, QueryTokenKind::RightParen) {
            self.unexpected("pure query function call is missing `)`");
            return None;
        }
        self.expression(
            QueryExpressionKind::Comparison {
                left: source_field_expression(field),
                operator,
                right: value,
            },
            u64::try_from(function.range.start).unwrap_or(u64::MAX)
                ..u64::try_from(close.range.end).unwrap_or(u64::MAX),
        )
    }

    #[allow(clippy::too_many_lines)]
    fn parse_predicate(&mut self) -> Option<QueryExpression> {
        let field = self.parse_field()?;
        let start = field.range.start;
        if self.consume_word("is").is_some() {
            let negated = self.consume_word("not").is_some();
            let null = self.advance().clone();
            if !matches!(&null.kind, QueryTokenKind::Word(value) if value == "null") {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "`is` accepts only `null` or `not null`",
                    null.range,
                ));
                return None;
            }
            if !field.nullable {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::TypeMismatch,
                    message: "non-nullable query field cannot be tested for null".to_owned(),
                    range: field.range.clone(),
                });
            }
            return self.expression(
                QueryExpressionKind::IsNull {
                    value: source_field_expression(field),
                    negated,
                },
                start..u64::try_from(null.range.end).unwrap_or(u64::MAX),
            );
        }
        if self.consume_word("in").is_some() {
            let open = self.advance().clone();
            if !matches!(open.kind, QueryTokenKind::LeftBracket) {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "`in` requires a bracketed literal list",
                    open.range,
                ));
                return None;
            }
            let mut values = Vec::new();
            loop {
                let mut value = self.parse_value()?;
                if value.value_type != QueryValueType::Null {
                    self.coerce_value(&field, &mut value, QueryComparisonOperator::Equal);
                }
                if values
                    .iter()
                    .any(|existing| query_values_equal(existing, &value))
                {
                    self.diagnostics.push(QueryDiagnostic {
                        code: QueryDiagnosticCode::DuplicateValue,
                        message: "an `in` list may not repeat a value".to_owned(),
                        range: value.range.clone(),
                    });
                }
                values.push(value);
                if values.len() > QUERY_MAX_IN_VALUES {
                    self.diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::TooManyInValues,
                        format!("an `in` list accepts at most {QUERY_MAX_IN_VALUES} values"),
                        open.range.clone(),
                    ));
                    return None;
                }
                if !matches!(self.current().kind, QueryTokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            let close = self.advance().clone();
            if !matches!(close.kind, QueryTokenKind::RightBracket) {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "`in` literal list is missing `]`",
                    close.range,
                ));
                return None;
            }
            return self.expression(
                QueryExpressionKind::In {
                    left: source_field_expression(field),
                    values,
                },
                start..u64::try_from(close.range.end).unwrap_or(u64::MAX),
            );
        }

        let operator_token = self.advance().clone();
        let operator = match &operator_token.kind {
            QueryTokenKind::Equal => QueryComparisonOperator::Equal,
            QueryTokenKind::NotEqual => QueryComparisonOperator::NotEqual,
            QueryTokenKind::LessThan => QueryComparisonOperator::LessThan,
            QueryTokenKind::LessThanOrEqual => QueryComparisonOperator::LessThanOrEqual,
            QueryTokenKind::GreaterThan => QueryComparisonOperator::GreaterThan,
            QueryTokenKind::GreaterThanOrEqual => QueryComparisonOperator::GreaterThanOrEqual,
            _ => {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidOperator,
                    "expected `=`, `!=`, `<`, `<=`, `>`, `>=`, `in`, or `is null`; text predicates use pure `contains()`/`starts_with()` functions",
                    operator_token.range,
                ));
                return None;
            }
        };
        let mut value = self.parse_value()?;
        if value.value_type == QueryValueType::Null {
            self.diagnostics.push(QueryDiagnostic {
                code: QueryDiagnosticCode::NullComparison,
                message:
                    "ordinary comparison with `null` is undefined; use `is null` or `is not null`"
                        .to_owned(),
                range: field.range.start..value.range.end,
            });
            return None;
        }
        self.coerce_value(&field, &mut value, operator);
        let end = value.range.end;
        self.expression(
            QueryExpressionKind::Comparison {
                left: source_field_expression(field),
                operator,
                right: value,
            },
            start..end,
        )
    }

    #[allow(clippy::too_many_lines)]
    fn parse_value(&mut self) -> Option<QueryValueExpression> {
        let token = self.advance().clone();
        if matches!(&token.kind, QueryTokenKind::Word(value) if matches!(value.as_str(), "date" | "instant" | "uuid"))
        {
            let QueryTokenKind::Word(constructor) = &token.kind else {
                unreachable!();
            };
            let open = self.advance().clone();
            let literal = self.advance().clone();
            let close = self.advance().clone();
            let QueryTokenKind::String(text) = &literal.kind else {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "typed value constructors require one quoted string",
                    literal.range,
                ));
                return None;
            };
            if !matches!(open.kind, QueryTokenKind::LeftParen)
                || !matches!(close.kind, QueryTokenKind::RightParen)
            {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "typed value constructor requires exactly one parenthesized argument",
                    open.range.start..close.range.end,
                ));
                return None;
            }
            let (literal, value_type) = match constructor.as_str() {
                "date" => {
                    let Ok(value @ TaskNodeTemporal::Date(_)) = TaskNodeTemporal::parse(text)
                    else {
                        self.diagnostics.push(query_diagnostic(
                            QueryDiagnosticCode::InvalidLiteral,
                            "date() requires an exact YYYY-MM-DD value",
                            literal.range,
                        ));
                        return None;
                    };
                    (QueryLiteral::Temporal(value), QueryValueType::Date)
                }
                "instant" => {
                    let Ok(value @ TaskNodeTemporal::Instant(_)) = TaskNodeTemporal::parse(text)
                    else {
                        self.diagnostics.push(query_diagnostic(
                            QueryDiagnosticCode::InvalidLiteral,
                            "instant() requires an explicit-offset RFC 3339 value",
                            literal.range,
                        ));
                        return None;
                    };
                    (QueryLiteral::Temporal(value), QueryValueType::Instant)
                }
                "uuid" if NodeId::from_str(text).is_ok() => {
                    (QueryLiteral::Uuid(text.clone()), QueryValueType::Uuid)
                }
                "uuid" => {
                    self.diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::InvalidLiteral,
                        "uuid() requires a lowercase UUIDv4 string",
                        literal.range,
                    ));
                    return None;
                }
                _ => unreachable!(),
            };
            return Some(QueryValueExpression {
                kind: QueryValueExpressionKind::Literal { literal },
                value_type,
                range: u64::try_from(token.range.start).unwrap_or(u64::MAX)
                    ..u64::try_from(close.range.end).unwrap_or(u64::MAX),
            });
        }
        if matches!(&token.kind, QueryTokenKind::Word(value) if value == "this.document.properties")
        {
            let open = self.advance().clone();
            let key = self.advance().clone();
            let close = self.advance().clone();
            let QueryTokenKind::String(key) = &key.kind else {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "context property lookup requires a quoted property name",
                    key.range,
                ));
                return None;
            };
            if !matches!(open.kind, QueryTokenKind::LeftBracket)
                || !matches!(close.kind, QueryTokenKind::RightBracket)
                || !valid_context_text(key, false)
            {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "context property lookup key is empty, malformed, or too large",
                    open.range.start..close.range.end,
                ));
                return None;
            }
            return Some(QueryValueExpression {
                kind: QueryValueExpressionKind::Context {
                    reference: QueryContextReference::ThisDocumentProperty(key.clone()),
                },
                value_type: QueryValueType::String,
                range: to_u64_range(token.range.start..close.range.end),
            });
        }
        let (kind, value_type) = match &token.kind {
            QueryTokenKind::String(value) => (
                QueryValueExpressionKind::Literal {
                    literal: QueryLiteral::String(value.clone()),
                },
                QueryValueType::String,
            ),
            QueryTokenKind::Word(value) if value == "true" || value == "false" => (
                QueryValueExpressionKind::Literal {
                    literal: QueryLiteral::Boolean(value == "true"),
                },
                QueryValueType::Boolean,
            ),
            QueryTokenKind::Word(value) if query_context_reference(value).is_some() => {
                let (reference, value_type) =
                    query_context_reference(value).expect("guarded query context reference");
                (QueryValueExpressionKind::Context { reference }, value_type)
            }
            QueryTokenKind::Word(value) if value == "null" => (
                QueryValueExpressionKind::Literal {
                    literal: QueryLiteral::Null,
                },
                QueryValueType::Null,
            ),
            QueryTokenKind::Integer(value) => {
                let Ok(value) = value.parse::<i64>() else {
                    self.diagnostics.push(query_diagnostic(
                        QueryDiagnosticCode::InvalidLiteral,
                        "query integer is outside the signed 64-bit range",
                        token.range,
                    ));
                    return None;
                };
                (
                    QueryValueExpressionKind::Literal {
                        literal: QueryLiteral::Number(value),
                    },
                    QueryValueType::Number,
                )
            }
            _ => {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "expected a typed literal or an explicit `this.*`/`context.*` reference",
                    token.range,
                ));
                return None;
            }
        };
        let mut value = QueryValueExpression {
            kind,
            value_type,
            range: to_u64_range(token.range),
        };
        if matches!(
            self.current().kind,
            QueryTokenKind::Plus | QueryTokenKind::Minus
        ) {
            let operator = self.advance().clone();
            let duration_token = self.advance().clone();
            if operator.range.start == to_usize(value.range.end)
                || duration_token.range.start == operator.range.end
            {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::UnexpectedToken,
                    "date arithmetic operators require surrounding ASCII whitespace",
                    operator.range,
                ));
                return None;
            }
            let QueryTokenKind::Word(duration) = &duration_token.kind else {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "date arithmetic requires a `P1D` through `P36500D` duration",
                    duration_token.range,
                ));
                return None;
            };
            let Some(mut days) = parse_day_duration(duration) else {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "date arithmetic requires a `P1D` through `P36500D` duration",
                    duration_token.range,
                ));
                return None;
            };
            if !matches!(
                value.value_type,
                QueryValueType::Temporal | QueryValueType::Date | QueryValueType::Instant
            ) {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::TypeMismatch,
                    message: "date arithmetic base must be temporal".to_owned(),
                    range: value.range.clone(),
                });
            }
            if matches!(operator.kind, QueryTokenKind::Minus) {
                days = -days;
            }
            let range =
                value.range.start..u64::try_from(duration_token.range.end).unwrap_or(u64::MAX);
            let value_type = value.value_type;
            value = QueryValueExpression {
                kind: QueryValueExpressionKind::DateOffset {
                    base: Box::new(value),
                    days,
                },
                value_type,
                range,
            };
        }
        Some(value)
    }

    fn coerce_value(
        &mut self,
        field: &QueryFieldReference,
        value: &mut QueryValueExpression,
        operator: QueryComparisonOperator,
    ) {
        if matches!(
            operator,
            QueryComparisonOperator::Contains | QueryComparisonOperator::StartsWith
        ) {
            if field.value_type != QueryValueType::String
                || value.value_type != QueryValueType::String
            {
                self.type_mismatch(field, value, "text operators require text on both sides");
            }
            return;
        }
        if matches!(
            operator,
            QueryComparisonOperator::LessThan
                | QueryComparisonOperator::LessThanOrEqual
                | QueryComparisonOperator::GreaterThan
                | QueryComparisonOperator::GreaterThanOrEqual
        ) && !matches!(
            field.value_type,
            QueryValueType::Number | QueryValueType::Temporal | QueryValueType::Priority
        ) {
            self.diagnostics.push(QueryDiagnostic {
                code: QueryDiagnosticCode::InvalidOperator,
                message:
                    "ordered comparison is valid only for integer, temporal, or priority fields"
                        .to_owned(),
                range: field.range.clone(),
            });
        }
        if matches!(
            field.value_type,
            QueryValueType::TaskKind | QueryValueType::TaskState | QueryValueType::Priority
        ) && value.value_type == QueryValueType::String
        {
            let Some(text) = literal_text(value) else {
                self.type_mismatch(field, value, "enumerated fields require a literal string");
                return;
            };
            if !valid_enum_literal(field.value_type, text) {
                self.diagnostics.push(QueryDiagnostic {
                    code: QueryDiagnosticCode::InvalidLiteral,
                    message: format!("unsupported value `{text}` for this query field"),
                    range: value.range.clone(),
                });
                return;
            }
            value.value_type = field.value_type;
        }
        if field.value_type == QueryValueType::Temporal
            && matches!(
                value.value_type,
                QueryValueType::Date | QueryValueType::Instant
            )
        {
            return;
        }
        if field.value_type != value.value_type {
            self.type_mismatch(field, value, "query field and literal types do not match");
        }
    }

    fn type_mismatch(
        &mut self,
        field: &QueryFieldReference,
        value: &QueryValueExpression,
        message: &str,
    ) {
        self.diagnostics.push(QueryDiagnostic {
            code: QueryDiagnosticCode::TypeMismatch,
            message: message.to_owned(),
            range: field.range.start..value.range.end,
        });
    }

    fn parse_field(&mut self) -> Option<QueryFieldReference> {
        let token = self.advance().clone();
        let QueryTokenKind::Word(name) = &token.kind else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::UnexpectedToken,
                "expected a source field name",
                token.range,
            ));
            return None;
        };
        let Some(source) = self.query_source else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::MissingFrom,
                "`from <domain> as <alias>` must precede every field use",
                token.range,
            ));
            return None;
        };
        let alias = self.alias.clone()?;
        let Some(path) = name
            .strip_prefix(&alias)
            .and_then(|value| value.strip_prefix('.'))
        else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::AliasMismatch,
                format!("query field must use the bound `{alias}.` alias"),
                token.range,
            ));
            return None;
        };
        let mut custom_property = None;
        let mut end = token.range.end;
        let property_field = match (source, path) {
            (QuerySource::Nodes, "document.properties")
            | (QuerySource::Tasks, "owner_node.document.properties")
            | (QuerySource::Headings, "owning_node.document.properties") => {
                Some(QueryField::DocumentProperty)
            }
            (QuerySource::Headings, "document.properties") => {
                Some(QueryField::HeadingDocumentProperty)
            }
            _ => None,
        };
        let field_spec = if let Some(property_field) = property_field {
            let open = self.advance().clone();
            let key = self.advance().clone();
            let close = self.advance().clone();
            let QueryTokenKind::String(key) = &key.kind else {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "document property lookup requires a quoted property name",
                    key.range,
                ));
                return None;
            };
            if !matches!(open.kind, QueryTokenKind::LeftBracket)
                || !matches!(close.kind, QueryTokenKind::RightBracket)
                || !valid_context_text(key, false)
            {
                self.diagnostics.push(query_diagnostic(
                    QueryDiagnosticCode::InvalidLiteral,
                    "document property lookup key is empty, malformed, or too large",
                    open.range.start..close.range.end,
                ));
                return None;
            }
            custom_property = Some(key.clone());
            end = close.range.end;
            Some((property_field, QueryValueType::String, true))
        } else {
            query_field(source, path)
        };
        let Some((field, value_type, nullable)) = field_spec else {
            self.diagnostics.push(query_diagnostic(
                QueryDiagnosticCode::UnknownField,
                format!("unknown {source:?} query field `{name}`"),
                token.range,
            ));
            return None;
        };
        Some(QueryFieldReference {
            alias,
            field,
            custom_property,
            value_type,
            nullable,
            range: to_u64_range(token.range.start..end),
        })
    }

    fn expression(
        &mut self,
        kind: QueryExpressionKind,
        range: Range<u64>,
    ) -> Option<QueryExpression> {
        self.expression_nodes += 1;
        if self.expression_nodes > QUERY_MAX_EXPRESSION_NODES {
            self.diagnostics.push(QueryDiagnostic {
                code: QueryDiagnosticCode::TooManyExpressionNodes,
                message: format!(
                    "query expression exceeds {QUERY_MAX_EXPRESSION_NODES} expression nodes"
                ),
                range: range.clone(),
            });
            None
        } else {
            Some(QueryExpression { kind, range })
        }
    }

    fn current_clause(&self) -> Option<(&'static str, u8)> {
        if !self.current().line_start {
            return None;
        }
        match &self.current().kind {
            QueryTokenKind::Word(value) if value == "from" => Some(("from", 0)),
            QueryTokenKind::Word(value) if value == "scope" => Some(("scope", 1)),
            QueryTokenKind::Word(value) if value == "where" => Some(("where", 2)),
            QueryTokenKind::Word(value) if value == "group" => Some(("group", 3)),
            QueryTokenKind::Word(value) if value == "select" => Some(("select", 4)),
            QueryTokenKind::Word(value) if value == "order" => Some(("order", 5)),
            QueryTokenKind::Word(value) if value == "limit" => Some(("limit", 6)),
            _ => None,
        }
    }

    fn consume_word(&mut self, expected: &str) -> Option<QueryToken> {
        if matches!(&self.current().kind, QueryTokenKind::Word(value) if value == expected) {
            Some(self.advance().clone())
        } else {
            None
        }
    }

    fn unexpected(&mut self, message: &str) {
        self.diagnostics.push(query_diagnostic(
            QueryDiagnosticCode::UnexpectedToken,
            message,
            self.current().range.clone(),
        ));
    }

    fn current(&self) -> &QueryToken {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> &QueryToken {
        let position = self.position;
        if !matches!(self.tokens[position].kind, QueryTokenKind::End) {
            self.position += 1;
        }
        &self.tokens[position]
    }
}

pub(crate) fn query_field(
    source: QuerySource,
    name: &str,
) -> Option<(QueryField, QueryValueType, bool)> {
    match source {
        QuerySource::Nodes => node_query_field(name),
        QuerySource::Tasks => task_query_field(name),
        QuerySource::Headings => heading_query_field(name),
        QuerySource::Templates => template_query_field(name),
    }
}

fn node_query_field(name: &str) -> Option<(QueryField, QueryValueType, bool)> {
    match name {
        "id" => Some((QueryField::Id, QueryValueType::Uuid, false)),
        "name" => Some((QueryField::Name, QueryValueType::String, false)),
        "path" => Some((QueryField::Path, QueryValueType::String, false)),
        "parent_id" => Some((QueryField::ParentId, QueryValueType::Uuid, true)),
        "depth" => Some((QueryField::Depth, QueryValueType::Number, false)),
        "display_title" => Some((QueryField::NodeDisplayTitle, QueryValueType::String, false)),
        "document.title" => Some((QueryField::DocumentTitle, QueryValueType::String, true)),
        "document.subtitle" => Some((QueryField::DocumentSubtitle, QueryValueType::String, true)),
        "document.display_title" => Some((
            QueryField::DocumentDisplayTitle,
            QueryValueType::String,
            false,
        )),
        _ => None,
    }
}

fn task_query_field(name: &str) -> Option<(QueryField, QueryValueType, bool)> {
    match name {
        "kind" => Some((QueryField::Kind, QueryValueType::TaskKind, false)),
        "id" => Some((QueryField::Id, QueryValueType::Uuid, true)),
        "owner_node.id" => Some((QueryField::OwnerNodeId, QueryValueType::Uuid, false)),
        "owner_node.name" => Some((QueryField::OwnerNodeName, QueryValueType::String, false)),
        "owner_node.path" => Some((QueryField::OwnerNodePath, QueryValueType::String, false)),
        "owner_node.parent_id" => Some((QueryField::OwnerNodeParentId, QueryValueType::Uuid, true)),
        "owner_node.depth" => Some((QueryField::OwnerNodeDepth, QueryValueType::Number, false)),
        "owner_node.display_title" => Some((
            QueryField::OwnerNodeDisplayTitle,
            QueryValueType::String,
            false,
        )),
        "owner_node.document.title" => {
            Some((QueryField::DocumentTitle, QueryValueType::String, true))
        }
        "owner_node.document.subtitle" => {
            Some((QueryField::DocumentSubtitle, QueryValueType::String, true))
        }
        "owner_node.document.display_title" => Some((
            QueryField::DocumentDisplayTitle,
            QueryValueType::String,
            false,
        )),
        "title" => Some((QueryField::Title, QueryValueType::String, false)),
        "closed" => Some((QueryField::Closed, QueryValueType::Boolean, false)),
        "state" => Some((QueryField::State, QueryValueType::TaskState, false)),
        "checklist_depth" => Some((QueryField::ChecklistDepth, QueryValueType::Number, true)),
        "priority" => Some((QueryField::Priority, QueryValueType::Priority, true)),
        "created" => Some((QueryField::Created, QueryValueType::Temporal, true)),
        "start" => Some((QueryField::Start, QueryValueType::Temporal, true)),
        "scheduled" => Some((QueryField::Scheduled, QueryValueType::Temporal, true)),
        "due" => Some((QueryField::Due, QueryValueType::Temporal, true)),
        "closed_at" => Some((QueryField::ClosedAt, QueryValueType::Temporal, true)),
        "blocked" => Some((QueryField::Blocked, QueryValueType::Boolean, true)),
        _ => None,
    }
}

fn heading_query_field(name: &str) -> Option<(QueryField, QueryValueType, bool)> {
    match name {
        "title" => Some((QueryField::Title, QueryValueType::String, false)),
        "level" => Some((QueryField::Level, QueryValueType::Number, false)),
        "anchor" => Some((QueryField::Anchor, QueryValueType::String, true)),
        "parent" => Some((QueryField::HeadingParent, QueryValueType::Record, true)),
        "path" => Some((QueryField::HeadingPath, QueryValueType::List, false)),
        "owning_node.id" => Some((QueryField::OwnerNodeId, QueryValueType::Uuid, false)),
        "owning_node.name" => Some((QueryField::OwnerNodeName, QueryValueType::String, false)),
        "owning_node.path" => Some((QueryField::OwnerNodePath, QueryValueType::String, false)),
        "owning_node.parent_id" => {
            Some((QueryField::OwnerNodeParentId, QueryValueType::Uuid, true))
        }
        "owning_node.depth" => Some((QueryField::OwnerNodeDepth, QueryValueType::Number, false)),
        "owning_node.display_title" => Some((
            QueryField::OwnerNodeDisplayTitle,
            QueryValueType::String,
            false,
        )),
        "owning_node.document.title" => {
            Some((QueryField::DocumentTitle, QueryValueType::String, true))
        }
        "owning_node.document.subtitle" => {
            Some((QueryField::DocumentSubtitle, QueryValueType::String, true))
        }
        "owning_node.document.display_title" => Some((
            QueryField::DocumentDisplayTitle,
            QueryValueType::String,
            false,
        )),
        "document.title" => Some((
            QueryField::HeadingDocumentTitle,
            QueryValueType::String,
            true,
        )),
        "document.subtitle" => Some((
            QueryField::HeadingDocumentSubtitle,
            QueryValueType::String,
            true,
        )),
        "document.display_title" => Some((
            QueryField::HeadingDocumentDisplayTitle,
            QueryValueType::String,
            false,
        )),
        _ => None,
    }
}

fn template_query_field(name: &str) -> Option<(QueryField, QueryValueType, bool)> {
    match name {
        "id" => Some((QueryField::Id, QueryValueType::Uuid, false)),
        "name" => Some((QueryField::Name, QueryValueType::String, false)),
        "path" => Some((QueryField::Path, QueryValueType::String, false)),
        "display_title" => Some((QueryField::NodeDisplayTitle, QueryValueType::String, false)),
        "part_count" => Some((QueryField::PartCount, QueryValueType::Number, false)),
        "parameter_count" => Some((QueryField::ParameterCount, QueryValueType::Number, false)),
        _ => None,
    }
}

pub(crate) fn valid_query_alias(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= QUERY_MAX_ALIAS_BYTES
        && value.as_bytes()[0].is_ascii_lowercase()
        && !is_reserved_query_identifier(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub(crate) const fn is_document_property_field(field: QueryField) -> bool {
    matches!(
        field,
        QueryField::DocumentProperty | QueryField::HeadingDocumentProperty
    )
}

fn is_reserved_query_identifier(value: &str) -> bool {
    matches!(
        value,
        "this"
            | "context"
            | "row"
            | "nodes"
            | "tasks"
            | "headings"
            | "templates"
            | "version"
            | "view"
            | "source"
            | "from"
            | "as"
            | "scope"
            | "workspace"
            | "subtree"
            | "descendants"
            | "section"
            | "where"
            | "group"
            | "by"
            | "select"
            | "order"
            | "limit"
            | "asc"
            | "desc"
            | "nulls"
            | "first"
            | "last"
            | "and"
            | "or"
            | "not"
            | "in"
            | "is"
            | "null"
            | "true"
            | "false"
            | "contains"
            | "starts_with"
            | "ends_with"
            | "format_date"
            | "length"
            | "concat"
            | "coalesce"
            | "date"
            | "instant"
            | "uuid"
    )
}

pub(crate) fn valid_query_output_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= QUERY_MAX_OUTPUT_NAME_BYTES
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub(crate) fn query_group_output_name(group: &QueryGroup) -> Option<String> {
    let QueryValueExpressionKind::SourceField { reference } = &group.expression.kind else {
        return None;
    };
    Some(
        group
            .output_name
            .clone()
            .unwrap_or_else(|| default_projection_output_name(reference.field).to_owned()),
    )
}

pub(crate) const fn default_projection_output_name(field: QueryField) -> &'static str {
    match field {
        QueryField::Id | QueryField::OwnerNodeId => "id",
        QueryField::Name | QueryField::OwnerNodeName => "name",
        QueryField::Path | QueryField::OwnerNodePath | QueryField::HeadingPath => "path",
        QueryField::ParentId | QueryField::OwnerNodeParentId => "parent_id",
        QueryField::Depth | QueryField::OwnerNodeDepth => "depth",
        QueryField::NodeDisplayTitle
        | QueryField::OwnerNodeDisplayTitle
        | QueryField::DocumentDisplayTitle
        | QueryField::HeadingDocumentDisplayTitle => "display_title",
        QueryField::Kind => "kind",
        QueryField::Description => "description",
        QueryField::Closed => "closed",
        QueryField::State => "state",
        QueryField::ChecklistDepth => "checklist_depth",
        QueryField::Priority => "priority",
        QueryField::Created => "created",
        QueryField::Start => "start",
        QueryField::Scheduled => "scheduled",
        QueryField::Due => "due",
        QueryField::ClosedAt => "closed_at",
        QueryField::Blocked => "blocked",
        QueryField::DocumentTitle | QueryField::HeadingDocumentTitle | QueryField::Title => "title",
        QueryField::DocumentSubtitle | QueryField::HeadingDocumentSubtitle => "subtitle",
        QueryField::DocumentProperty | QueryField::HeadingDocumentProperty => "property",
        QueryField::Level => "level",
        QueryField::Anchor => "anchor",
        QueryField::HeadingParent => "parent",
        QueryField::PartCount => "part_count",
        QueryField::ParameterCount => "parameter_count",
    }
}

fn valid_context_text(value: &str, allow_empty: bool) -> bool {
    value.len() <= QUERY_MAX_CONTEXT_TEXT_BYTES
        && (allow_empty || !value.is_empty())
        && !value.contains(['\r', '\n', '\0'])
}

fn query_context_reference(value: &str) -> Option<(QueryContextReference, QueryValueType)> {
    let pair = match value {
        "this.node.id" => (QueryContextReference::ThisNodeId, QueryValueType::Uuid),
        "this.node.name" => (QueryContextReference::ThisNodeName, QueryValueType::String),
        "this.node.path" => (QueryContextReference::ThisNodePath, QueryValueType::String),
        "this.node.depth" => (QueryContextReference::ThisNodeDepth, QueryValueType::Number),
        "this.node.display_title" => (
            QueryContextReference::ThisNodeDisplayTitle,
            QueryValueType::String,
        ),
        "this.document.title" => (
            QueryContextReference::ThisDocumentTitle,
            QueryValueType::String,
        ),
        "this.document.subtitle" => (
            QueryContextReference::ThisDocumentSubtitle,
            QueryValueType::String,
        ),
        "this.document.display_title" => (
            QueryContextReference::ThisDocumentDisplayTitle,
            QueryValueType::String,
        ),
        "this.heading.title" => (
            QueryContextReference::ThisHeadingTitle,
            QueryValueType::String,
        ),
        "this.heading.level" => (
            QueryContextReference::ThisHeadingLevel,
            QueryValueType::Number,
        ),
        "this.heading.anchor" => (
            QueryContextReference::ThisHeadingAnchor,
            QueryValueType::String,
        ),
        "this.heading.parent" => (
            QueryContextReference::ThisHeadingParent,
            QueryValueType::Record,
        ),
        "this.heading.path" => (QueryContextReference::ThisHeadingPath, QueryValueType::List),
        "this.query.title" => (
            QueryContextReference::ThisQueryTitle,
            QueryValueType::String,
        ),
        "context.today" => (QueryContextReference::ContextToday, QueryValueType::Date),
        "context.now" => (QueryContextReference::ContextNow, QueryValueType::Instant),
        "context.timezone" => (
            QueryContextReference::ContextTimezone,
            QueryValueType::String,
        ),
        "context.locale" => (QueryContextReference::ContextLocale, QueryValueType::String),
        _ => return None,
    };
    Some(pair)
}

fn valid_enum_literal(value_type: QueryValueType, value: &str) -> bool {
    match value_type {
        QueryValueType::TaskKind => matches!(value, "checklist" | "node"),
        QueryValueType::TaskState => matches!(
            value,
            "todo" | "in-progress" | "on-hold" | "completed" | "cancelled"
        ),
        QueryValueType::Priority => matches!(
            value,
            "highest" | "high" | "medium" | "normal" | "low" | "lowest"
        ),
        _ => false,
    }
}

fn literal_text(value: &QueryValueExpression) -> Option<&str> {
    match &value.kind {
        QueryValueExpressionKind::Literal {
            literal: QueryLiteral::String(value),
        } => Some(value),
        _ => None,
    }
}

fn query_values_equal(left: &QueryValueExpression, right: &QueryValueExpression) -> bool {
    if left.value_type != right.value_type {
        return false;
    }
    match (&left.kind, &right.kind) {
        (
            QueryValueExpressionKind::Literal { literal: left },
            QueryValueExpressionKind::Literal { literal: right },
        ) => left == right,
        (
            QueryValueExpressionKind::Context { reference: left },
            QueryValueExpressionKind::Context { reference: right },
        ) => left == right,
        (
            QueryValueExpressionKind::DateOffset {
                base: left,
                days: left_days,
            },
            QueryValueExpressionKind::DateOffset {
                base: right,
                days: right_days,
            },
        ) => left_days == right_days && query_values_equal(left, right),
        _ => false,
    }
}

fn parse_day_duration(value: &str) -> Option<i32> {
    let days = value.strip_prefix('P')?.strip_suffix('D')?;
    if days.is_empty() || (days.len() > 1 && days.starts_with('0')) {
        return None;
    }
    let days = days.parse::<i32>().ok()?;
    (1..=36_500).contains(&days).then_some(days)
}

fn to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
