use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use weftext_core::{CitationDiagnosticCode, CitationForm, analyze_citation_source};

#[derive(Debug, Deserialize)]
struct Manifest {
    profile: String,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    source: String,
    valid: bool,
    #[serde(default)]
    clusters: usize,
    #[serde(default)]
    items: usize,
    #[serde(default)]
    nocites: usize,
    #[serde(default)]
    bibliographies: usize,
    code: Option<String>,
}

#[test]
fn citation_fixture_corpus_matches_the_parser_contract() {
    let root = fixture_root();
    let manifest: Manifest = read_json(&root.join("manifest.json"));
    assert_eq!(manifest.profile, "weftext.citations.v1");

    for case in manifest.cases {
        let source = fs::read_to_string(root.join(&case.source)).expect("read citation fixture");
        let analysis = analyze_citation_source(&source);
        if case.valid {
            assert!(
                analysis.diagnostics.is_empty(),
                "{}: {:?}",
                case.id,
                analysis.diagnostics
            );
            assert_eq!(
                analysis.clusters.len(),
                case.clusters,
                "{}: {:?}",
                case.id,
                analysis.clusters
            );
            assert_eq!(
                analysis
                    .clusters
                    .iter()
                    .map(|cluster| cluster.items.len())
                    .sum::<usize>(),
                case.items,
                "{}",
                case.id
            );
            assert_eq!(analysis.nocites.len(), case.nocites, "{}", case.id);
            assert_eq!(
                analysis.bibliographies.len(),
                case.bibliographies,
                "{}",
                case.id
            );
            assert_analysis_ranges(&source, &analysis);
        } else {
            assert!(!analysis.diagnostics.is_empty(), "{}", case.id);
            let expected = case.code.expect("invalid fixture code");
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
fn cluster_items_attributes_and_utf8_ranges_are_exact() {
    let source = fs::read_to_string(fixture_root().join("valid/all-forms.adoc"))
        .expect("read all-forms fixture");
    let analysis = analyze_citation_source(&source);
    let cluster = &analysis.clusters[2];

    assert_eq!(cluster.form, CitationForm::Parenthetical);
    assert_eq!(
        slice(&source, &cluster.range),
        "cite:[wang2024,label=page,locator=\"59-61\",prefix=\"参见 \",suffix=\", خاصة\"]+[smith2025,label=chapter,locator=2]"
    );
    assert_eq!(slice(&source, &cluster.items[0].key.range), "wang2024");
    assert_eq!(cluster.items[0].prefix.as_deref(), Some("参见 "));
    assert_eq!(cluster.items[0].suffix.as_deref(), Some(", خاصة"));
    assert_eq!(cluster.items[1].label, "chapter");
    assert_eq!(cluster.items[1].locator.as_deref(), Some("2"));
    for attribute in &cluster.items[0].attributes {
        assert!(source.is_char_boundary(to_usize(attribute.range.start)));
        assert!(source.is_char_boundary(to_usize(attribute.range.end)));
        assert!(!slice(&source, &attribute.name_range).is_empty());
        assert!(!slice(&source, &attribute.value_range).is_empty());
    }
}

#[test]
fn locator_without_an_explicit_label_defaults_to_page() {
    let analysis = analyze_citation_source("cite:[wang2024,locator=7]");
    assert!(analysis.diagnostics.is_empty());
    assert_eq!(analysis.clusters[0].items[0].label, "page");
}

#[test]
fn separate_parenthetical_clusters_keep_their_immediate_chain_items() {
    let source = "cite:[alpha2024]+[beta2024] and cite:[gamma2025]+[delta2025]";
    let analysis = analyze_citation_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert_eq!(analysis.clusters.len(), 2);
    assert!(
        analysis
            .clusters
            .iter()
            .all(|cluster| cluster.items.len() == 2)
    );
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/citations-v1")
        .canonicalize()
        .expect("citation fixture root")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let source = fs::read_to_string(path).expect("read fixture manifest");
    serde_json::from_str(&source).expect("parse fixture manifest")
}

fn diagnostic_code(code: CitationDiagnosticCode) -> String {
    serde_json::to_value(code)
        .expect("serialize diagnostic code")
        .as_str()
        .expect("diagnostic code is a string")
        .to_owned()
}

fn assert_analysis_ranges(source: &str, analysis: &weftext_core::CitationSourceAnalysis) {
    for cluster in &analysis.clusters {
        assert_range(source, &cluster.range);
        for item in &cluster.items {
            assert_range(source, &item.range);
            assert_range(source, &item.key.range);
            for attribute in &item.attributes {
                assert_range(source, &attribute.range);
                assert_range(source, &attribute.name_range);
                assert_range(source, &attribute.value_range);
            }
        }
    }
    for nocite in &analysis.nocites {
        assert_range(source, &nocite.range);
        for key in &nocite.keys {
            assert_range(source, &key.range);
        }
    }
    for bibliography in &analysis.bibliographies {
        assert_range(source, &bibliography.range);
    }
}

fn assert_range(source: &str, range: &Range<u64>) {
    let start = to_usize(range.start);
    let end = to_usize(range.end);
    assert!(start < end);
    assert!(end <= source.len());
    assert!(source.is_char_boundary(start));
    assert!(source.is_char_boundary(end));
}

fn slice<'a>(source: &'a str, range: &Range<u64>) -> &'a str {
    &source[to_usize(range.start)..to_usize(range.end)]
}

fn to_usize(value: u64) -> usize {
    usize::try_from(value).expect("source range fits usize")
}
