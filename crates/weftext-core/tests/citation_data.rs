use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use weftext_core::{CITATION_DATA_PROFILE_ID, ReferenceDiagnosticCode, analyze_reference_metadata};

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    profile: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    source: String,
    valid: bool,
    key: Option<String>,
    code: Option<String>,
}

#[test]
fn citation_data_fixture_corpus_matches_the_runtime_contract() {
    let fixture_root = fixture_root();
    let manifest: FixtureManifest = read_json(&fixture_root.join("manifest.json"));

    assert_eq!(manifest.profile, CITATION_DATA_PROFILE_ID);
    assert!(!manifest.cases.is_empty());

    for case in manifest.cases {
        let source = fs::read_to_string(fixture_root.join(&case.source))
            .unwrap_or_else(|error| panic!("could not read fixture {}: {error}", case.id));
        let analysis = analyze_reference_metadata(&source);
        assert_eq!(analysis.profile, CITATION_DATA_PROFILE_ID, "{}", case.id);

        if case.valid {
            assert!(
                analysis.diagnostics.is_empty(),
                "{}: {:?}",
                case.id,
                analysis.diagnostics
            );
            let citation_data = analysis
                .citation_data
                .unwrap_or_else(|| panic!("{} should produce Citation Data", case.id));
            assert_eq!(citation_data.key, case.key.expect("valid fixture key"));
            assert!(analysis.mapping_range.is_some(), "{}", case.id);
            assert!(!analysis.field_ranges.is_empty(), "{}", case.id);
            for field in analysis.field_ranges {
                assert_valid_source_range(&source, &field.value_range, &case.id);
                if let Some(range) = field.key_range {
                    assert_valid_source_range(&source, &range, &case.id);
                }
            }
        } else {
            assert!(analysis.citation_data.is_none(), "{}", case.id);
            let expected = case.code.expect("invalid fixture diagnostic code");
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic_code(diagnostic.code) == expected),
                "{} expected {expected}, got {:?}",
                case.id,
                analysis.diagnostics
            );
        }
    }
}

#[test]
fn citation_data_schema_is_closed_and_identifies_the_profile() {
    let schema: Value = read_json(&workspace_root().join("schemas/citation-data-v1.schema.json"));

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        schema["$id"],
        "https://weftext.org/schemas/citation-data-v1.schema.json"
    );
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["$defs"]["name"]["additionalProperties"], false);
    assert_eq!(schema["$defs"]["date"]["additionalProperties"], false);
    assert_eq!(
        schema["required"],
        serde_json::json!(["key", "type", "title"])
    );
}

#[test]
fn ranges_are_exact_utf8_source_ranges_for_cjk_and_rtl_scalars() {
    let source = fs::read_to_string(fixture_root().join("valid/full-cjk-rtl.adoc"))
        .expect("read full Citation Data fixture");
    let analysis = analyze_reference_metadata(&source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );

    assert_eq!(
        field_source(&source, &analysis, "reference.title"),
        "\"结构化文档与 العربية\""
    );
    assert_eq!(
        field_source(&source, &analysis, "reference.author[0].family"),
        "王"
    );
    assert_eq!(
        field_source(&source, &analysis, "reference.author[1].literal"),
        "\"مثال للأبحاث\""
    );

    let title = analysis
        .field_ranges
        .iter()
        .find(|field| field.path == "reference.title")
        .expect("title range");
    assert_eq!(
        slice(&source, title.key_range.as_ref().expect("title key range")),
        "title"
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture_root() -> PathBuf {
    workspace_root().join("tests/fixtures/citation-data-v1")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("could not parse {}: {error}", path.display()))
}

fn diagnostic_code(code: ReferenceDiagnosticCode) -> String {
    serde_json::to_value(code)
        .expect("serialize diagnostic code")
        .as_str()
        .expect("diagnostic code is a string")
        .to_owned()
}

fn assert_valid_source_range(source: &str, range: &Range<u64>, fixture_id: &str) {
    let start = usize::try_from(range.start).expect("range start fits usize");
    let end = usize::try_from(range.end).expect("range end fits usize");
    assert!(start < end, "{fixture_id}: empty source range {range:?}");
    assert!(
        end <= source.len(),
        "{fixture_id}: range outside source {range:?}"
    );
    assert!(
        source.is_char_boundary(start),
        "{fixture_id}: invalid UTF-8 start"
    );
    assert!(
        source.is_char_boundary(end),
        "{fixture_id}: invalid UTF-8 end"
    );
}

fn field_source<'a>(
    source: &'a str,
    analysis: &weftext_core::ReferenceAnalysis,
    path: &str,
) -> &'a str {
    let field = analysis
        .field_ranges
        .iter()
        .find(|field| field.path == path)
        .unwrap_or_else(|| panic!("missing field range {path}"));
    slice(source, &field.value_range)
}

fn slice<'a>(source: &'a str, range: &Range<u64>) -> &'a str {
    let start = usize::try_from(range.start).expect("range start fits usize");
    let end = usize::try_from(range.end).expect("range end fits usize");
    &source[start..end]
}
