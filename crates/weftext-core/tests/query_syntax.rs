use std::collections::BTreeSet;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use weftext_core::{
    QUERY_EXPRESSION_CAPABILITY_ID, QUERY_MAX_BODY_BYTES, QUERY_PROFILE_ID, QueryDiagnosticCode,
    QueryExpression, QueryExpressionKind, QueryField, QueryScope, QuerySource,
    QueryValueExpression, QueryValueExpressionKind, QueryView, analyze_query_source,
};

#[derive(Debug, Deserialize)]
struct Manifest {
    profile: String,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    id: String,
    source: Option<String>,
    source_text: Option<String>,
    valid: bool,
    blocks: usize,
    #[serde(default)]
    codes: Vec<String>,
    features: Vec<String>,
}

#[test]
fn query_fixture_corpus_matches_the_parser_contract() {
    let root = fixture_root();
    let manifest: Manifest = read_json(&root.join("manifest.json"));
    assert_eq!(manifest.profile, QUERY_PROFILE_ID);
    let mut covered_features = BTreeSet::new();

    for case in manifest.cases {
        let source = case.source_text.clone().unwrap_or_else(|| {
            fs::read_to_string(root.join(case.source.as_ref().expect("fixture source path")))
                .expect("read query fixture")
        });
        let analysis = analyze_query_source(&source);
        covered_features.extend(case.features);
        assert_eq!(analysis.blocks.len(), case.blocks, "{}", case.id);
        assert_analysis_ranges(&source, &analysis);
        if case.valid {
            assert!(
                analysis.diagnostics.is_empty(),
                "{}: {:?}",
                case.id,
                analysis.diagnostics
            );
            assert!(
                analysis
                    .blocks
                    .iter()
                    .all(|block| block.valid && block.plan.is_some()),
                "{}: {:?}",
                case.id,
                analysis.blocks
            );
        } else {
            assert!(!analysis.diagnostics.is_empty(), "{}", case.id);
            assert!(
                analysis.blocks.iter().all(|block| !block.valid),
                "{}: {:?}",
                case.id,
                analysis.blocks
            );
            let actual = analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic_code(diagnostic.code))
                .collect::<BTreeSet<_>>();
            for expected in case.codes {
                assert!(
                    actual.contains(&expected),
                    "{} expected {expected}, got {:?}",
                    case.id,
                    analysis.diagnostics
                );
            }
        }
    }

    for feature in [
        "tasks",
        "nodes",
        "boolean-precedence",
        "date-arithmetic",
        "scope",
        "ordering",
        "grouping",
        "projection",
        "limit",
        "protected-context",
        "mixed-line-ending",
        "typing",
        "resource-bound",
        "negative",
        "malformed",
    ] {
        assert!(
            covered_features.contains(feature),
            "missing fixture feature {feature}"
        );
    }
}

#[test]
fn task_query_has_frozen_precedence_typed_fields_and_date_offset() {
    let source = fs::read_to_string(fixture_root().join("valid/tasks-full.adoc"))
        .expect("read full task query fixture");
    let analysis = analyze_query_source(&source);
    let block = &analysis.blocks[0];
    assert_eq!(block.source, Some(QuerySource::Tasks));
    assert_eq!(block.view, Some(QueryView::TaskList));
    let plan = block.plan.as_ref().expect("typed query plan");
    assert_eq!(plan.expression_capability, QUERY_EXPRESSION_CAPABILITY_ID);
    assert_eq!(plan.scope, QueryScope::Workspace);
    assert_eq!(plan.limit, 250);
    let group = &plan.group.as_ref().expect("group").expression;
    let QueryValueExpressionKind::SourceField { reference } = &group.kind else {
        panic!("group key must use the shared source-field expression node");
    };
    assert_eq!(reference.field, QueryField::State);
    assert_eq!(plan.sort.len(), 2);
    assert_eq!(plan.projection.len(), 8);

    let filter = plan.filter.as_ref().expect("filter");
    let QueryExpressionKind::And { left, right } = &filter.kind else {
        panic!("top-level filter must preserve left-associative and: {filter:?}");
    };
    assert!(matches!(right.kind, QueryExpressionKind::In { .. }));
    let QueryExpressionKind::And {
        left: first,
        right: second,
    } = &left.kind
    else {
        panic!("first two predicates must bind before final and: {left:?}");
    };
    assert!(matches!(first.kind, QueryExpressionKind::Not { .. }));
    assert!(matches!(second.kind, QueryExpressionKind::Or { .. }));
    let date_offset = find_date_offset(second).expect("today plus duration");
    let QueryValueExpressionKind::DateOffset { days, .. } = date_offset.kind else {
        unreachable!()
    };
    assert_eq!(days, 14);
}

#[test]
fn empty_query_body_is_rejected_without_a_default_domain_or_clauses() {
    let source = "[.weftext-query,version=1]\n....\n# no implicit plan\n....";
    let analysis = analyze_query_source(source);
    assert_code(source, QueryDiagnosticCode::MissingFrom);
    assert!(analysis.blocks[0].plan.is_none());
}

#[test]
fn resource_bounds_fail_before_unbounded_query_work() {
    let oversized = format!(
        "[.weftext-query,version=1]\n....\n# {}\n....",
        "x".repeat(QUERY_MAX_BODY_BYTES)
    );
    assert_code(&oversized, QueryDiagnosticCode::BodyTooLarge);

    let nesting = format!(
        "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\nwhere {}task.closed = false{}\nselect task.id\norder by task.id asc\nlimit 10\n....",
        "(".repeat(33),
        ")".repeat(33)
    );
    assert_code(&nesting, QueryDiagnosticCode::NestingTooDeep);

    let expression = (0..140)
        .map(|_| "task.closed = false")
        .collect::<Vec<_>>()
        .join(" and ");
    let expression = format!(
        "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\nwhere {expression}\nselect task.id\norder by task.id asc\nlimit 10\n...."
    );
    assert_code(&expression, QueryDiagnosticCode::TooManyExpressionNodes);

    let tokens = format!(
        "[.weftext-query,version=1]\n....\n{}\n....",
        "x ".repeat(2_049)
    );
    assert_code(&tokens, QueryDiagnosticCode::TooManyTokens);

    let sorts = [
        "task.kind",
        "task.id",
        "task.owner_node.id",
        "task.owner_node.name",
        "task.owner_node.path",
        "task.title",
        "task.closed",
        "task.state",
        "task.priority",
    ]
    .map(|field| format!("{field} asc"))
    .join(", ");
    let sorts = format!(
        "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\nwhere task.closed = false\nselect task.id\norder by {sorts}\nlimit 10\n...."
    );
    assert_code(&sorts, QueryDiagnosticCode::TooManySortFields);

    let projection = std::iter::repeat_n("task.title", 33)
        .collect::<Vec<_>>()
        .join(", ");
    let projection = format!(
        "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\nwhere task.closed = false\nselect {projection}\norder by task.id asc\nlimit 10\n...."
    );
    assert_code(&projection, QueryDiagnosticCode::TooManyProjectionFields);

    let in_values = (0..65)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let in_values = format!(
        "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\nwhere task.checklist_depth in [{in_values}]\nselect task.id\norder by task.id asc\nlimit 10\n...."
    );
    assert_code(&in_values, QueryDiagnosticCode::TooManyInValues);
}

#[test]
fn retired_task_fields_are_unknown_without_compatibility_aliases() {
    for field in [
        "node-id",
        "node-name",
        "node-path",
        "phase",
        "resolution",
        "recurring",
        "repeat-from",
        "depth",
        "structured",
    ] {
        let source = format!(
            "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\nwhere task.closed = false\nselect task.{field}\norder by task.id asc\nlimit 10\n....\n"
        );
        assert_code(&source, QueryDiagnosticCode::UnknownField);
    }
    let nodes = analyze_query_source(
        "[.weftext-query,version=1]\n....\nfrom nodes as node\nscope workspace\nwhere node.depth >= 0\nselect node.depth\norder by node.id asc\nlimit 10\n....\n",
    );
    assert!(nodes.diagnostics.is_empty(), "{:?}", nodes.diagnostics);
}

#[test]
fn canonical_clauses_share_typed_value_expressions_and_where_true_is_explicit() {
    let source = concat!(
        "[.weftext-query,version=1,view=table]\n",
        "....\n",
        "from nodes as item\n",
        "scope workspace\n",
        "where true\n",
        "group by item.depth\n",
        "select item.id, item.document.title\n",
        "order by item.path asc\n",
        "limit 25\n",
        "....\n",
    );
    let analysis = analyze_query_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let plan = analysis.blocks[0].plan.as_ref().expect("canonical plan");
    assert!(matches!(
        plan.filter.as_ref().map(|value| &value.kind),
        Some(QueryExpressionKind::Boolean { .. })
    ));
    let expressions = plan
        .projection
        .iter()
        .map(|projection| &projection.expression)
        .chain(plan.sort.iter().map(|sort| &sort.expression))
        .chain(plan.group.iter().map(|group| &group.expression));
    assert!(expressions.into_iter().all(|expression| matches!(
        expression.kind,
        QueryValueExpressionKind::SourceField { .. }
    )));

    let old_group_direction = source.replace("group by item.depth", "group by item.depth asc");
    assert_code(&old_group_direction, QueryDiagnosticCode::UnexpectedToken);
    let missing_order_direction = source.replace("order by item.path asc", "order by item.path");
    assert_code(
        &missing_order_direction,
        QueryDiagnosticCode::UnexpectedToken,
    );
}

#[test]
fn aliases_cannot_shadow_reserved_language_roots() {
    for reserved in [
        "row",
        "nodes",
        "tasks",
        "headings",
        "templates",
        "version",
        "view",
        "source",
        "from",
        "as",
        "scope",
        "workspace",
        "subtree",
        "descendants",
        "section",
        "where",
        "group",
        "by",
        "select",
        "order",
        "limit",
        "asc",
        "desc",
        "nulls",
        "first",
        "last",
        "and",
        "or",
        "not",
        "in",
        "is",
        "null",
        "true",
        "false",
        "contains",
        "starts_with",
        "ends_with",
        "format_date",
        "length",
        "concat",
        "coalesce",
        "date",
        "instant",
        "uuid",
    ] {
        let source = format!(
            "[.weftext-query,version=1]\n....\nfrom nodes as {reserved}\nscope workspace\nwhere true\nselect {reserved}.id\norder by {reserved}.id asc\nlimit 1\n....\n"
        );
        assert_code(&source, QueryDiagnosticCode::AliasShadowing);
    }
}

#[test]
fn outputs_and_closed_domain_records_are_canonical() {
    let headings = concat!(
        "[.weftext-query,version=1]\n....\n",
        "from headings as heading\nscope workspace\nwhere true\n",
        "group by heading.level as heading_level\n",
        "select heading.title as heading_title, heading.level, heading.anchor, heading.parent, heading.path as heading_path, ",
        "heading.owning_node.id as node_id, heading.owning_node.name as node_name, heading.owning_node.path as node_path, ",
        "heading.owning_node.parent_id, heading.owning_node.depth, heading.owning_node.display_title as node_display_title, ",
        "heading.owning_node.document.title as document_title, heading.owning_node.document.subtitle, ",
        "heading.owning_node.document.display_title as document_display_title, ",
        "heading.owning_node.document.properties[\"状态\"] as document_status, ",
        "heading.document.title as heading_document_title, heading.document.subtitle as heading_document_subtitle, ",
        "heading.document.display_title as heading_document_display_title, ",
        "heading.document.properties[\"状态\"] as heading_document_status\n",
        "order by heading.owning_node.path asc, heading.level asc\nlimit 100\n....\n",
    );
    let analysis = analyze_query_source(headings);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let plan = analysis.blocks[0].plan.as_ref().expect("heading plan");
    assert_eq!(plan.alias, "heading");
    assert_eq!(
        plan.group
            .as_ref()
            .and_then(|group| group.output_name.as_deref()),
        Some("heading_level")
    );
    assert!(
        plan.projection
            .iter()
            .all(|projection| !projection.output_name.is_empty())
    );
    let projected_fields = plan
        .projection
        .iter()
        .filter_map(|projection| match &projection.expression.kind {
            QueryValueExpressionKind::SourceField { reference } => Some(reference.field),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for field in [
        QueryField::HeadingDocumentTitle,
        QueryField::HeadingDocumentSubtitle,
        QueryField::HeadingDocumentDisplayTitle,
        QueryField::HeadingDocumentProperty,
    ] {
        assert!(projected_fields.contains(&field));
    }

    let old_heading_node = headings.replace("heading.owning_node.id", "heading.node.id");
    assert_code(&old_heading_node, QueryDiagnosticCode::UnknownField);

    let templates = concat!(
        "[.weftext-query,version=1]\n....\nfrom templates as template\nscope workspace\nwhere true\n",
        "select template.id, template.name, template.path, template.display_title, template.part_count, template.parameter_count\n",
        "order by template.path asc\nlimit 100\n....\n",
    );
    assert!(analyze_query_source(templates).diagnostics.is_empty());
    assert_code(
        &templates.replace("scope workspace", "scope subtree(this.node)"),
        QueryDiagnosticCode::DomainUnavailable,
    );

    let property_without_output = concat!(
        "[.weftext-query,version=1]\n....\nfrom nodes as item_2\nscope workspace\nwhere true\n",
        "select item_2.document.properties[\"status\"]\norder by item_2.id asc\nlimit 1\n....\n",
    );
    assert_code(property_without_output, QueryDiagnosticCode::InvalidAlias);
    let duplicate_output = property_without_output.replace(
        "item_2.document.properties[\"status\"]",
        "item_2.id as value, item_2.name as value",
    );
    assert_code(&duplicate_output, QueryDiagnosticCode::DuplicateOutputName);
    let group_projection_collision = concat!(
        "[.weftext-query,version=1]\n....\nfrom nodes as node\nscope workspace\nwhere true\n",
        "group by node.depth as id\nselect node.id\norder by node.path asc\nlimit 1\n....\n",
    );
    assert_code(
        group_projection_collision,
        QueryDiagnosticCode::DuplicateOutputName,
    );
}

#[test]
fn null_comparison_and_decoded_string_limits_are_stable_diagnostics() {
    let null_comparison = concat!(
        "[.weftext-query,version=1]\n....\nfrom tasks as task\nscope workspace\n",
        "where task.due = null\nselect task.title\norder by task.title asc\nlimit 10\n....\n",
    );
    assert_code(null_comparison, QueryDiagnosticCode::NullComparison);

    let oversized = format!(
        "[.weftext-query,version=1]\n....\nfrom nodes as node\nscope workspace\nwhere node.name = \"{}\"\nselect node.id\norder by node.id asc\nlimit 1\n....\n",
        "x".repeat(4_097)
    );
    assert_code(&oversized, QueryDiagnosticCode::StringTooLarge);
}

#[test]
fn lexical_context_uses_parser_owned_document_sections_and_query_blocks() {
    let canonical_body = concat!(
        "....\n",
        "from nodes as node\n",
        "scope workspace\n",
        "where true\n",
        "select node.id\n",
        "order by node.id asc\n",
        "limit 10\n",
        "....\n",
    );
    let source = format!(
        concat!(
            "= Authored title: Authored subtitle\n",
            ":status: 研究\n\n",
            "preamble\n\n",
            "[.weftext-query,version=1]\n{}\n",
            "== H1\n\n",
            ".H1 query\n",
            "[.weftext-query,version=1]\n{}\n",
            "=== H2\n\n",
            "[.weftext-query,version=1]\n{}\n",
            "========== H9\n\n",
            "[.weftext-query,version=1]\n{}",
        ),
        canonical_body, canonical_body, canonical_body, canonical_body,
    );
    let analysis = analyze_query_source(&source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert_eq!(analysis.blocks.len(), 4);
    let preamble = &analysis.blocks[0].lexical_context;
    assert_eq!(preamble.document.title.as_deref(), Some("Authored title"));
    assert_eq!(
        preamble.document.subtitle.as_deref(),
        Some("Authored subtitle")
    );
    assert_eq!(
        preamble
            .document
            .properties
            .get("status")
            .map(String::as_str),
        Some("研究")
    );
    assert!(preamble.heading.is_none());

    let h1 = analysis.blocks[1]
        .lexical_context
        .heading
        .as_ref()
        .expect("H1 context");
    assert_eq!((h1.title.as_str(), h1.level), ("H1", 1));
    assert!(h1.parent.is_none(), "{h1:?}");
    assert_eq!(
        analysis.blocks[1].lexical_context.query.title.as_deref(),
        Some("H1 query")
    );

    let h2 = analysis.blocks[2]
        .lexical_context
        .heading
        .as_ref()
        .expect("H2 context");
    assert_eq!((h2.title.as_str(), h2.level), ("H2", 2));
    assert_eq!(
        h2.parent.as_ref().map(|parent| parent.title.as_str()),
        Some("H1")
    );
    assert_eq!(h2.path, ["H1", "H2"]);

    let h9 = analysis.blocks[3]
        .lexical_context
        .heading
        .as_ref()
        .expect("H9 context");
    assert_eq!((h9.title.as_str(), h9.level), ("H9", 9));
    assert_eq!(
        h9.parent.as_ref().map(|parent| parent.title.as_str()),
        Some("H2")
    );

    let title_only = format!("= Only title\n\n[.weftext-query,version=1]\n{canonical_body}");
    assert!(
        analyze_query_source(&title_only).blocks[0]
            .lexical_context
            .heading
            .is_none()
    );
}

#[test]
fn retired_roots_and_ordinary_asciidoc_slot_text_are_inert() {
    let no_this_title = concat!(
        "[.weftext-query,version=1]\n....\n",
        "from nodes as node\nscope workspace\nwhere node.name = this.title\n",
        "select node.id\norder by node.id asc\nlimit 10\n....\n",
    );
    assert_code(no_this_title, QueryDiagnosticCode::InvalidLiteral);
    assert!(
        analyze_query_source("[query,source=nodes]\n....\nselect row.name\n....\n\nslot::main[]\n")
            .blocks
            .is_empty()
    );
    let generic = weftext_asciidoc::analyze(
        "[.weftext-query,version=1]\n....\nfrom nodes as node\nslot::main[]\n....\n",
    );
    assert!(!generic.blocks.is_empty());
    let ordinary = concat!(
        "---\nweftext:\n  id: \"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1\"\n---\n",
        "= Ordinary document\n\n",
        "inline slot:title[] remains authored text.\n\n",
        "slot::body[]\n",
    );
    let before = ordinary.as_bytes().to_vec();
    let analysis = analyze_query_source(ordinary);
    assert!(analysis.blocks.is_empty());
    assert!(analysis.diagnostics.is_empty());
    assert_eq!(ordinary.as_bytes(), before);
}

#[test]
fn typed_value_constructors_replace_retired_bare_temporal_and_uuid_literals() {
    let date = concat!(
        "[.weftext-query,version=1]\n....\n",
        "from tasks as task\nscope workspace\n",
        "where task.due >= date(\"2026-08-24\") and task.due < instant(\"2026-09-01T00:00:00Z\")\n",
        "select task.id\norder by task.id asc\nlimit 10\n....\n",
    );
    assert!(analyze_query_source(date).diagnostics.is_empty());
    let uuid = concat!(
        "[.weftext-query,version=1]\n....\n",
        "from nodes as node\nscope workspace\n",
        "where node.id = uuid(\"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1\")\n",
        "select node.id\norder by node.id asc\nlimit 10\n....\n",
    );
    assert!(analyze_query_source(uuid).diagnostics.is_empty());
    assert_code(
        &date.replace("date(\"2026-08-24\")", "@2026-08-24"),
        QueryDiagnosticCode::InvalidToken,
    );
    assert_code(
        &uuid.replace(
            "uuid(\"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1\")",
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1",
        ),
        QueryDiagnosticCode::InvalidLiteral,
    );
}

fn find_date_offset(expression: &QueryExpression) -> Option<&QueryValueExpression> {
    match &expression.kind {
        QueryExpressionKind::Comparison { right, .. }
            if matches!(right.kind, QueryValueExpressionKind::DateOffset { .. }) =>
        {
            Some(right)
        }
        QueryExpressionKind::Not { expression } => find_date_offset(expression),
        QueryExpressionKind::And { left, right } | QueryExpressionKind::Or { left, right } => {
            find_date_offset(left).or_else(|| find_date_offset(right))
        }
        QueryExpressionKind::Boolean { .. }
        | QueryExpressionKind::Comparison { .. }
        | QueryExpressionKind::In { .. }
        | QueryExpressionKind::IsNull { .. } => None,
    }
}

fn assert_code(source: &str, code: QueryDiagnosticCode) {
    let analysis = analyze_query_source(source);
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "expected {code:?}, got {:?}",
        analysis.diagnostics
    );
    assert!(analysis.blocks.iter().all(|block| !block.valid));
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/query-v1")
        .canonicalize()
        .expect("query fixture root")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let source = fs::read_to_string(path).expect("read JSON fixture");
    serde_json::from_str(&source).expect("parse JSON fixture")
}

fn diagnostic_code(code: QueryDiagnosticCode) -> String {
    serde_json::to_value(code)
        .expect("serialize query diagnostic code")
        .as_str()
        .expect("query diagnostic code is a string")
        .to_owned()
}

fn assert_analysis_ranges(source: &str, analysis: &weftext_core::QuerySourceAnalysis) {
    for block in &analysis.blocks {
        assert_range(source, &block.range, false);
        assert_range(source, &block.header_range, false);
        assert_range(source, &block.body_range, true);
        assert_eq!(slice(source, &block.body_range), block.body);
    }
    for diagnostic in &analysis.diagnostics {
        assert_range(source, &diagnostic.range, true);
    }
}

fn assert_range(source: &str, range: &Range<u64>, allow_empty: bool) {
    let start = to_usize(range.start);
    let end = to_usize(range.end);
    assert!(allow_empty || start < end, "empty range {range:?}");
    assert!(
        start <= end && end <= source.len(),
        "invalid range {range:?}"
    );
    assert!(
        source.is_char_boundary(start),
        "invalid UTF-8 start {range:?}"
    );
    assert!(source.is_char_boundary(end), "invalid UTF-8 end {range:?}");
}

fn slice<'a>(source: &'a str, range: &Range<u64>) -> &'a str {
    &source[to_usize(range.start)..to_usize(range.end)]
}

fn to_usize(value: u64) -> usize {
    usize::try_from(value).expect("source range fits usize")
}
