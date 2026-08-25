use std::collections::BTreeMap;
use std::time::Instant;

use weftext_core::{
    BibliographyCompilation, BibliographyComponentInput, BibliographyInclusion,
    BibliographyOccurrence, BibliographyReferenceInput, CitationAssetLoadingPolicy, CitationData,
    CitationForm, CitationPresentationDiagnosticCode, CitationPresentationProfile,
    CitationPresentationRequest, CitationPresenterIsolation, DocumentRevision, NodeId,
    ReferenceDate, ReferenceName, ReferenceValue, ResolvedCitationCluster, ResolvedCitationItem,
    ResolvedReference, citation_presentation_capabilities, present_citations,
};

#[test]
fn capabilities_expose_offline_assets_and_retired_record_writes() {
    let capabilities = citation_presentation_capabilities();
    assert_eq!(
        capabilities.isolation,
        CitationPresenterIsolation::OfflineDataOnly
    );
    assert_eq!(
        capabilities.asset_loading,
        CitationAssetLoadingPolicy::PackagedAllowlist
    );
    assert_eq!(
        capabilities
            .styles
            .iter()
            .map(|asset| asset.id.as_str())
            .collect::<Vec<_>>(),
        ["apa", "vancouver", "chicago-notes"]
    );
    assert!(!capabilities.reference_record_writes_available);
    assert!(
        capabilities
            .reference_record_writes_reason
            .contains("typed Citation Data")
    );
}

#[test]
fn explicit_typed_compilation_presents_deterministically_without_workspace_records() {
    let compilation = typed_compilation(2, true);
    let request = CitationPresentationRequest::new(
        CitationPresentationProfile::new("apa", "en-US"),
        compilation,
    );
    let first = present_citations(&request).unwrap();
    let second = present_citations(&request).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.components.len(), 1);
    assert_eq!(first.components[0].citations.len(), 2);
    assert_eq!(
        first.components[0]
            .bibliography
            .as_ref()
            .unwrap()
            .entries
            .len(),
        2
    );
    assert!(
        first.components[0].citations[0]
            .content
            .plain_text()
            .contains("42")
    );
}

#[test]
fn numeric_and_note_profiles_consume_the_same_provider_neutral_input() {
    let compilation = typed_compilation(2, true);
    let numeric = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new("vancouver", "zh-CN"),
        compilation.clone(),
    ))
    .unwrap();
    assert!(
        numeric.components[0].citations[0]
            .content
            .plain_text()
            .chars()
            .any(|character| character.is_ascii_digit())
    );

    let notes = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new("chicago-notes", "ar"),
        compilation,
    ))
    .unwrap();
    assert_eq!(notes.components[0].citations[0].note_number, Some(1));
    assert_eq!(notes.components[0].citations[1].note_number, Some(2));
}

#[test]
fn unavailable_assets_and_unsupported_typed_data_fail_closed() {
    let compilation = typed_compilation(1, true);
    let unavailable = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new("arbitrary.csl", "en-US"),
        compilation.clone(),
    ))
    .unwrap_err();
    assert_eq!(
        unavailable.diagnostics[0].code,
        CitationPresentationDiagnosticCode::UnavailableStyle
    );

    let mut markup = compilation;
    let data = {
        let data = &mut markup.components[0].references[0].citation_data;
        data.title = "<script>alert(1)</script>".to_owned();
        data.fields
            .insert("title".to_owned(), ReferenceValue::Text(data.title.clone()));
        data.clone()
    };
    markup.components[0].clusters[0].items[0]
        .reference
        .citation_data
        .clone_from(&data);
    let unsupported = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new("apa", "en-US"),
        markup,
    ))
    .unwrap_err();
    assert_eq!(
        unsupported.diagnostics[0].code,
        CitationPresentationDiagnosticCode::UnsupportedReferenceData
    );
}

#[test]
fn empty_and_representative_large_typed_compilations_are_covered() {
    const REFERENCE_COUNT: usize = 256;

    let empty = typed_compilation(0, true);
    let empty_presentation = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new("apa", "en-US"),
        empty,
    ))
    .unwrap();
    assert!(empty_presentation.components[0].citations.is_empty());
    assert!(
        empty_presentation.components[0]
            .bibliography
            .as_ref()
            .unwrap()
            .entries
            .is_empty()
    );

    let started = Instant::now();
    let large = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new("vancouver", "en-US"),
        typed_compilation(REFERENCE_COUNT, true),
    ))
    .unwrap();
    assert_eq!(large.components[0].citations.len(), REFERENCE_COUNT);
    assert_eq!(
        large.components[0]
            .bibliography
            .as_ref()
            .unwrap()
            .entries
            .len(),
        REFERENCE_COUNT
    );
    eprintln!(
        "typed-citation-presentation: references={REFERENCE_COUNT} elapsed_ms={}",
        started.elapsed().as_millis()
    );
}

fn typed_compilation(count: usize, bibliography: bool) -> BibliographyCompilation {
    let component_node_id = NodeId::new_v4();
    let mut clusters = Vec::with_capacity(count);
    let mut references = Vec::with_capacity(count);
    for index in 0..count {
        let node_id = NodeId::new_v4();
        let citation_data = citation_data(index);
        let resolved = ResolvedReference {
            node_id,
            citation_data: citation_data.clone(),
            key_range: (index as u64 * 10)..(index as u64 * 10 + 4),
        };
        clusters.push(ResolvedCitationCluster {
            form: CitationForm::Parenthetical,
            range: (index as u64 * 10)..(index as u64 * 10 + 9),
            items: vec![ResolvedCitationItem {
                range: (index as u64 * 10)..(index as u64 * 10 + 8),
                label: "page".to_owned(),
                locator: (index == 0).then(|| "42".to_owned()),
                prefix: None,
                suffix: None,
                reference: resolved,
            }],
        });
        references.push(BibliographyReferenceInput {
            node_id,
            citation_data,
        });
    }
    BibliographyCompilation {
        components: vec![BibliographyComponentInput {
            component_node_id,
            revision: DocumentRevision::from_source("typed compilation"),
            placement: bibliography.then(|| BibliographyOccurrence {
                range: (count as u64 * 10 + 1)..(count as u64 * 10 + 20),
                inclusion: BibliographyInclusion::Cited,
                attributes: Vec::new(),
            }),
            clusters,
            references,
        }],
        diagnostics: Vec::new(),
    }
}

fn citation_data(index: usize) -> CitationData {
    let key = format!("reference{index}");
    let title = if index == 0 {
        "结构化文档".to_owned()
    } else {
        format!("Reference {index}")
    };
    let fields = BTreeMap::from([
        ("key".to_owned(), ReferenceValue::Text(key.clone())),
        (
            "type".to_owned(),
            ReferenceValue::Text("article-journal".to_owned()),
        ),
        ("title".to_owned(), ReferenceValue::Text(title.clone())),
        (
            "author".to_owned(),
            ReferenceValue::Names(vec![ReferenceName {
                family: Some(format!("Author{index}")),
                given: Some("Example".to_owned()),
                ..ReferenceName::default()
            }]),
        ),
        (
            "issued".to_owned(),
            ReferenceValue::Date(ReferenceDate {
                date_parts: Some(vec![vec![2024]]),
                ..ReferenceDate::default()
            }),
        ),
    ]);
    CitationData {
        key,
        item_type: "article-journal".to_owned(),
        title,
        fields,
    }
}
