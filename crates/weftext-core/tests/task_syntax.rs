use std::collections::BTreeSet;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use weftext_core::{
    TASK_METADATA_FIELDS, TASK_PROFILE_ID, TaskDateTime, TaskDiagnosticCode, TaskPhase,
    TaskPriority, TaskRecurrenceFrequency, TaskRepeatFrom, TaskResolution, analyze_task_source,
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
    tasks: usize,
    #[serde(default)]
    structured: usize,
    #[serde(default)]
    codes: Vec<String>,
    features: Vec<String>,
}

#[test]
fn task_fixture_corpus_matches_the_parser_contract() {
    let root = fixture_root();
    let manifest: Manifest = read_json(&root.join("manifest.json"));
    assert_eq!(manifest.profile, TASK_PROFILE_ID);
    let mut covered_features = BTreeSet::new();

    for case in manifest.cases {
        let source = case.source_text.clone().unwrap_or_else(|| {
            fs::read_to_string(root.join(case.source.as_ref().expect("fixture source path")))
                .expect("read task fixture")
        });
        let analysis = analyze_task_source(&source);
        covered_features.extend(case.features);
        assert_eq!(analysis.tasks.len(), case.tasks, "{}", case.id);
        assert_analysis_ranges(&source, &analysis);

        if case.valid {
            assert!(
                analysis.diagnostics.is_empty(),
                "{}: {:?}",
                case.id,
                analysis.diagnostics
            );
            assert!(analysis.tasks.iter().all(|task| task.valid), "{}", case.id);
            assert_eq!(
                analysis
                    .tasks
                    .iter()
                    .filter(|task| task.metadata.is_some())
                    .count(),
                case.structured,
                "{}",
                case.id
            );
        } else {
            assert!(!analysis.diagnostics.is_empty(), "{}", case.id);
            assert!(analysis.tasks.iter().any(|task| !task.valid), "{}", case.id);
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

    let required = [
        "positive",
        "negative",
        "malformed",
        "cjk",
        "rtl",
        "mixed-line-ending",
        "nested-list",
        "protected-context",
        "date",
        "time",
        "recurrence",
        "identity",
        "dependency",
    ];
    for feature in required {
        assert!(
            covered_features.contains(feature),
            "missing fixture feature {feature}"
        );
    }
}

#[test]
fn simple_checklists_keep_exact_markers_and_never_gain_identity_on_read() {
    let source = fs::read_to_string(fixture_root().join("valid/basic-multilingual.adoc"))
        .expect("read basic task fixture");
    let analysis = analyze_task_source(&source);

    assert_eq!(analysis.tasks[0].description, "编写摘要");
    assert_eq!(analysis.tasks[0].list_depth, 1);
    assert_eq!(analysis.tasks[1].description, "مراجعة النص");
    assert_eq!(analysis.tasks[1].list_depth, 2);
    assert_eq!(analysis.tasks[2].authored_marker, "*");
    assert_eq!(analysis.tasks[2].list_depth, 3);
    assert!(analysis.tasks[2].metadata.is_none());
    assert_eq!(analysis.tasks[3].authored_marker, " ");
    assert!(analysis.tasks[3].metadata.is_none());
    assert_eq!(
        source,
        fs::read_to_string(fixture_root().join("valid/basic-multilingual.adoc")).unwrap()
    );

    let first = analysis.tasks[0]
        .metadata
        .as_ref()
        .expect("structured task");
    assert_eq!(first.phase, Some(TaskPhase::InProgress));
    assert_eq!(first.priority, TaskPriority::Highest);
    assert_eq!(
        first.created,
        Some(TaskDateTime::Instant(
            "2026-08-23T09:15:00+08:00".to_owned()
        ))
    );
    let second = analysis.tasks[1]
        .metadata
        .as_ref()
        .expect("structured task");
    assert_eq!(second.resolution, Some(TaskResolution::Completed));
}

#[test]
fn recurrence_dates_and_dependencies_are_typed_deterministically() {
    let source =
        fs::read_to_string(fixture_root().join("valid/dates-recurrence-dependencies.adoc"))
            .expect("read recurrence task fixture");
    let analysis = analyze_task_source(&source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );

    let weekly = analysis.tasks[1]
        .metadata
        .as_ref()
        .expect("weekly metadata");
    let recurrence = weekly.recurrence.as_ref().expect("weekly recurrence");
    assert_eq!(recurrence.frequency, TaskRecurrenceFrequency::Weekly);
    assert_eq!(recurrence.interval, 2);
    assert_eq!(recurrence.by_day, ["MO", "TH"]);
    assert_eq!(recurrence.count, Some(8));
    assert_eq!(recurrence.week_start.as_deref(), Some("MO"));
    assert_eq!(weekly.repeat_from, Some(TaskRepeatFrom::Due));
    assert_eq!(weekly.dependencies.len(), 1);

    let monthly = analysis.tasks[2]
        .metadata
        .as_ref()
        .expect("monthly metadata");
    let recurrence = monthly.recurrence.as_ref().expect("monthly recurrence");
    assert_eq!(recurrence.by_month_day, [1, -1]);
    assert_eq!(
        recurrence.until,
        Some(TaskDateTime::Date("2027-12-31".to_owned()))
    );
}

#[test]
fn protected_task_like_text_does_not_become_metadata_or_an_occurrence() {
    let source = fs::read_to_string(fixture_root().join("valid/protected-contexts.adoc"))
        .expect("read protected task fixture");
    let analysis = analyze_task_source(&source);

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert_eq!(analysis.tasks.len(), 2);
    assert!(analysis.tasks[0].description.contains("pass:[task:["));
    assert!(analysis.tasks[0].metadata.is_none());
    assert!(analysis.tasks[1].metadata.is_some());
}

#[test]
fn metadata_schema_and_core_accept_the_same_field_names() {
    let schema: serde_json::Value = read_json(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/task-metadata-v1.schema.json"),
    );
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    let properties = schema["properties"]
        .as_object()
        .expect("task schema properties")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        properties,
        TASK_METADATA_FIELDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(schema["properties"]["depends-on"]["maxLength"], 4096);
}

#[test]
fn dependency_diagnostics_and_decoded_tokens_are_bounded() {
    const OWN: &str = "11111111-1111-4111-8111-111111111111";
    const OTHER: &str = "22222222-2222-4222-8222-222222222222";
    let mixed = format!(
        "* [ ] Bounded task:[id={OWN},depends-on=\"{OWN} {OWN} {OTHER} {OTHER}  bad  bad\"]\n"
    );
    let analysis = analyze_task_source(&mixed);
    for code in [
        TaskDiagnosticCode::InvalidDependency,
        TaskDiagnosticCode::SelfDependency,
        TaskDiagnosticCode::DuplicateDependency,
    ] {
        assert_eq!(
            analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == code)
                .count(),
            1
        );
    }

    let escaped_spaces = "\\u0020".repeat(5_000);
    let escaped = format!("* [ ] Escaped task:[id={OWN},depends-on=\"{escaped_spaces}\"]\n");
    let escaped_analysis = analyze_task_source(&escaped);
    assert_eq!(
        escaped_analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == TaskDiagnosticCode::InvalidDependency)
            .count(),
        1
    );
    assert!(escaped_analysis.tasks[0].metadata.is_none());

    let dependency_ids = (1..=111)
        .map(|index| format!("00000000-0000-4000-8000-{index:012x}"))
        .collect::<Vec<_>>();
    let boundary_value = dependency_ids[..110].join(" ");
    assert_eq!(boundary_value.len(), 4_069);
    let boundary = format!("* [ ] Boundary task:[id={OWN},depends-on=\"{boundary_value}\"]\n");
    let boundary_analysis = analyze_task_source(&boundary);
    assert!(boundary_analysis.diagnostics.is_empty());
    assert_eq!(
        boundary_analysis.tasks[0]
            .metadata
            .as_ref()
            .expect("110 dependencies fit the target header")
            .dependencies
            .len(),
        110
    );

    let oversized_value = dependency_ids.join(" ");
    assert_eq!(oversized_value.len(), 4_106);
    let oversized = format!("* [ ] Oversized task:[id={OWN},depends-on=\"{oversized_value}\"]\n");
    let oversized_analysis = analyze_task_source(&oversized);
    assert_eq!(
        oversized_analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == TaskDiagnosticCode::InvalidDependency)
            .count(),
        1
    );
    assert!(oversized_analysis.tasks[0].metadata.is_none());
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tasks-v1")
        .canonicalize()
        .expect("task fixture root")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let source = fs::read_to_string(path).expect("read JSON fixture");
    serde_json::from_str(&source).expect("parse JSON fixture")
}

fn diagnostic_code(code: TaskDiagnosticCode) -> String {
    serde_json::to_value(code)
        .expect("serialize task diagnostic code")
        .as_str()
        .expect("task diagnostic code is a string")
        .to_owned()
}

fn assert_analysis_ranges(source: &str, analysis: &weftext_core::TaskSourceAnalysis) {
    for task in &analysis.tasks {
        assert_range(source, &task.range);
        assert_range(source, &task.marker_range);
        assert_range(source, &task.description_range);
        assert_eq!(slice(source, &task.description_range), task.description);
        if let Some(metadata) = &task.metadata {
            assert_range(source, &metadata.range);
            for attribute in &metadata.attributes {
                assert_range(source, &attribute.range);
                assert_range(source, &attribute.name_range);
                assert_range(source, &attribute.value_range);
            }
        }
    }
    for diagnostic in &analysis.diagnostics {
        assert_range(source, &diagnostic.range);
    }
}

fn assert_range(source: &str, range: &Range<u64>) {
    let start = to_usize(range.start);
    let end = to_usize(range.end);
    assert!(start < end, "empty range {range:?}");
    assert!(end <= source.len(), "out-of-bounds range {range:?}");
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
