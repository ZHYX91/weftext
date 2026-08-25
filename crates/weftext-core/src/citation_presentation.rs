use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::FromStr;

use hayagriva::citationberg::json::{
    DateValue, Item as CslItem, LiteralName, NameItem, NameValue, Value as CslValue, VecDate,
    VecDateRange,
};
use hayagriva::citationberg::taxonomy::{Kind, Locator};
use hayagriva::citationberg::{
    Display, FontStyle, FontVariant, FontWeight, IndependentStyle, Locale, LocaleCode,
    SecondFieldAlign, Style, TextDecoration, VerticalAlign,
};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, CitationItem as ProviderCitationItem, CitationRequest,
    CitePurpose, Elem, ElemChild, ElemChildren, ElemMeta, Formatting as ProviderFormatting,
    LocatorPayload, SpecificLocator,
    archive::{self, ArchivedStyle},
};
use serde::Serialize;

use crate::{
    BibliographyCompilation, BibliographyComponentInput, CitationData, CitationForm,
    DocumentRevision, NodeId, ReferenceDate, ReferenceName, ReferenceValue,
    ResolvedCitationCluster,
};

pub const CITATION_PRESENTER_ID: &str = "weftext.hayagriva";
pub const CITATION_PRESENTER_VERSION: &str = "0.8.1";

const STYLE_APA: &str = "apa";
const STYLE_VANCOUVER: &str = "vancouver";
const STYLE_CHICAGO_NOTES: &str = "chicago-notes";
const LOCALE_EN_US: &str = "en-US";
const LOCALE_ZH_CN: &str = "zh-CN";
const LOCALE_AR: &str = "ar";

const MAX_COMPONENTS: usize = 4_096;
const MAX_CLUSTERS_PER_COMPONENT: usize = 100_000;
const MAX_REFERENCES_PER_COMPONENT: usize = 100_000;
const MAX_ITEMS_PER_CLUSTER: usize = 1_024;
const MAX_PROVIDER_TEXT_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentationProfile {
    pub style_id: String,
    pub locale: String,
}

impl CitationPresentationProfile {
    #[must_use]
    pub fn new(style_id: impl Into<String>, locale: impl Into<String>) -> Self {
        Self {
            style_id: style_id.into(),
            locale: locale.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentationRequest {
    pub profile: CitationPresentationProfile,
    pub compilation: BibliographyCompilation,
}

impl CitationPresentationRequest {
    #[must_use]
    pub const fn new(
        profile: CitationPresentationProfile,
        compilation: BibliographyCompilation,
    ) -> Self {
        Self {
            profile,
            compilation,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentationCapabilities {
    pub provider_id: String,
    pub provider_version: String,
    pub styles: Vec<CitationPresentationAsset>,
    pub locales: Vec<CitationPresentationAsset>,
    pub isolation: CitationPresenterIsolation,
    pub asset_loading: CitationAssetLoadingPolicy,
    pub reference_record_writes_available: bool,
    pub reference_record_writes_reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationPresenterIsolation {
    OfflineDataOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationAssetLoadingPolicy {
    PackagedAllowlist,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentationAsset {
    pub id: String,
    pub label: String,
    pub license: String,
    pub attribution: String,
}

#[must_use]
pub fn citation_presentation_capabilities() -> CitationPresentationCapabilities {
    CitationPresentationCapabilities {
        provider_id: CITATION_PRESENTER_ID.to_owned(),
        provider_version: CITATION_PRESENTER_VERSION.to_owned(),
        styles: vec![
            CitationPresentationAsset {
                id: STYLE_APA.to_owned(),
                label: "American Psychological Association 7th edition".to_owned(),
                license: "CC-BY-SA-3.0".to_owned(),
                attribution: "Brenton M. Wiernik and CSL style contributors".to_owned(),
            },
            CitationPresentationAsset {
                id: STYLE_VANCOUVER.to_owned(),
                label: "Vancouver".to_owned(),
                license: "CC-BY-SA-3.0".to_owned(),
                attribution: "Michael Berkowitz and CSL style contributors".to_owned(),
            },
            CitationPresentationAsset {
                id: STYLE_CHICAGO_NOTES.to_owned(),
                label: "Chicago Manual of Style 17th edition (note)".to_owned(),
                license: "CC-BY-SA-3.0".to_owned(),
                attribution: "Julian Onions and CSL style contributors".to_owned(),
            },
        ],
        locales: [
            (LOCALE_EN_US, "English (United States)"),
            (LOCALE_ZH_CN, "Chinese (China)"),
            (LOCALE_AR, "Arabic"),
        ]
        .into_iter()
        .map(|(id, label)| CitationPresentationAsset {
            id: id.to_owned(),
            label: label.to_owned(),
            license: "CC-BY-SA-3.0".to_owned(),
            attribution: "Citation Style Language locale contributors".to_owned(),
        })
        .collect(),
        isolation: CitationPresenterIsolation::OfflineDataOnly,
        asset_loading: CitationAssetLoadingPolicy::PackagedAllowlist,
        reference_record_writes_available: crate::reference_record_writes_available(),
        reference_record_writes_reason: crate::REFERENCE_RECORD_WRITES_RETIREMENT.to_owned(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentation {
    pub provider_id: String,
    pub provider_version: String,
    pub profile: CitationPresentationProfile,
    pub components: Vec<CitationComponentPresentation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationComponentPresentation {
    pub component_node_id: NodeId,
    pub revision: DocumentRevision,
    pub citations: Vec<PresentedCitation>,
    pub bibliography: Option<PresentedBibliography>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentedCitation {
    pub source_range: Range<u64>,
    pub form: CitationForm,
    pub note_number: Option<usize>,
    pub reference_node_ids: Vec<NodeId>,
    pub content: CitationRichText,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentedBibliography {
    pub source_range: Range<u64>,
    pub hanging_indent: bool,
    pub second_field_align: Option<CitationSecondFieldAlign>,
    pub line_spacing: i16,
    pub entry_spacing: i16,
    pub entries: Vec<PresentedBibliographyEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationSecondFieldAlign {
    Margin,
    Flush,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentedBibliographyEntry {
    pub reference_node_id: NodeId,
    pub first_field: Option<CitationRichText>,
    pub content: CitationRichText,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationRichText {
    pub runs: Vec<CitationRichTextRun>,
}

impl CitationRichText {
    #[must_use]
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationRichTextRun {
    pub text: String,
    pub style: CitationTextStyle,
    pub link: Option<String>,
    pub reference_node_id: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationTextStyle {
    pub italic: bool,
    pub small_caps: bool,
    pub weight: CitationFontWeight,
    pub underline: bool,
    pub vertical_align: CitationVerticalAlign,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationFontWeight {
    #[default]
    Normal,
    Bold,
    Light,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationVerticalAlign {
    #[default]
    None,
    Baseline,
    Superscript,
    Subscript,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationPresentationDiagnosticCode {
    UnavailableStyle,
    UnavailableLocale,
    UnresolvedCompilation,
    RequestLimitExceeded,
    DuplicateComponent,
    ConflictingReference,
    UnsupportedReferenceData,
    UnsupportedLocator,
    UnsupportedNarrative,
    MissingBibliography,
    MalformedProviderOutput,
    ProviderFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentationDiagnostic {
    pub code: CitationPresentationDiagnosticCode,
    pub message: String,
    pub component_node_id: Option<NodeId>,
    pub reference_node_id: Option<NodeId>,
    pub source_range: Option<Range<u64>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationPresentationFailure {
    pub diagnostics: Vec<CitationPresentationDiagnostic>,
}

impl fmt::Display for CitationPresentationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(diagnostic) = self.diagnostics.first() {
            formatter.write_str(&diagnostic.message)
        } else {
            formatter.write_str("citation presentation failed")
        }
    }
}

impl std::error::Error for CitationPresentationFailure {}

/// Presents an already resolved citation compilation without workspace or network access.
///
/// The provider is selected only from packaged, reviewed assets. Provider panics and malformed
/// rich output are contained and converted into fail-closed diagnostics.
///
/// # Errors
///
/// Returns precise diagnostics for unavailable assets, unresolved or inconsistent inputs,
/// unsupported provider data, request limits, provider failures, and malformed provider output.
pub fn present_citations(
    request: &CitationPresentationRequest,
) -> Result<CitationPresentation, CitationPresentationFailure> {
    let guarded = catch_unwind(AssertUnwindSafe(|| present_citations_inner(request)));
    match guarded {
        Ok(result) => result,
        Err(_) => Err(failure(diagnostic(
            CitationPresentationDiagnosticCode::ProviderFailure,
            "the packaged CSL provider failed while rendering".to_owned(),
            None,
            None,
            None,
        ))),
    }
}

fn present_citations_inner(
    request: &CitationPresentationRequest,
) -> Result<CitationPresentation, CitationPresentationFailure> {
    validate_request(request)?;
    let style = packaged_style(&request.profile.style_id)?;
    let locales = packaged_locales(&request.profile.locale)?;
    let locale = LocaleCode(request.profile.locale.clone());
    let mut components = Vec::with_capacity(request.compilation.components.len());
    for component in &request.compilation.components {
        components.push(present_component(component, &style, &locales, &locale)?);
    }
    Ok(CitationPresentation {
        provider_id: CITATION_PRESENTER_ID.to_owned(),
        provider_version: CITATION_PRESENTER_VERSION.to_owned(),
        profile: request.profile.clone(),
        components,
    })
}

fn validate_request(
    request: &CitationPresentationRequest,
) -> Result<(), CitationPresentationFailure> {
    if request.compilation.components.len() > MAX_COMPONENTS {
        return Err(limit_failure("components", MAX_COMPONENTS, None));
    }
    if !request.compilation.diagnostics.is_empty() {
        return Err(CitationPresentationFailure {
            diagnostics: request
                .compilation
                .diagnostics
                .iter()
                .map(|source| {
                    diagnostic(
                        CitationPresentationDiagnosticCode::UnresolvedCompilation,
                        source.message.clone(),
                        source.component_node_id,
                        source.reference_node_ids.first().copied(),
                        source.range.clone(),
                    )
                })
                .collect(),
        });
    }
    let mut component_ids = BTreeSet::new();
    for component in &request.compilation.components {
        if !component_ids.insert(component.component_node_id) {
            return Err(failure(diagnostic(
                CitationPresentationDiagnosticCode::DuplicateComponent,
                format!(
                    "component {} occurs more than once in the presentation request",
                    component.component_node_id
                ),
                Some(component.component_node_id),
                None,
                None,
            )));
        }
        if component.clusters.len() > MAX_CLUSTERS_PER_COMPONENT {
            return Err(limit_failure(
                "citation clusters per component",
                MAX_CLUSTERS_PER_COMPONENT,
                Some(component.component_node_id),
            ));
        }
        if component.references.len() > MAX_REFERENCES_PER_COMPONENT {
            return Err(limit_failure(
                "references per component",
                MAX_REFERENCES_PER_COMPONENT,
                Some(component.component_node_id),
            ));
        }
        if let Some(cluster) = component
            .clusters
            .iter()
            .find(|cluster| cluster.items.len() > MAX_ITEMS_PER_CLUSTER)
        {
            return Err(failure(diagnostic(
                CitationPresentationDiagnosticCode::RequestLimitExceeded,
                format!("citation cluster exceeds the {MAX_ITEMS_PER_CLUSTER} item limit"),
                Some(component.component_node_id),
                None,
                Some(cluster.range.clone()),
            )));
        }
    }
    Ok(())
}

fn packaged_style(style_id: &str) -> Result<IndependentStyle, CitationPresentationFailure> {
    let archived = match style_id {
        STYLE_APA => ArchivedStyle::AmericanPsychologicalAssociation,
        STYLE_VANCOUVER => ArchivedStyle::Vancouver,
        STYLE_CHICAGO_NOTES => ArchivedStyle::ChicagoNotes,
        _ => {
            return Err(failure(diagnostic(
                CitationPresentationDiagnosticCode::UnavailableStyle,
                format!("citation style `{style_id}` is not a packaged Weftext asset"),
                None,
                None,
                None,
            )));
        }
    };
    match archived.get() {
        Style::Independent(style) => Ok(style),
        Style::Dependent(_) => Err(failure(diagnostic(
            CitationPresentationDiagnosticCode::MalformedProviderOutput,
            format!("packaged citation style `{style_id}` is not independent"),
            None,
            None,
            None,
        ))),
    }
}

fn packaged_locales(locale_id: &str) -> Result<Vec<Locale>, CitationPresentationFailure> {
    if !matches!(locale_id, LOCALE_EN_US | LOCALE_ZH_CN | LOCALE_AR) {
        return Err(failure(diagnostic(
            CitationPresentationDiagnosticCode::UnavailableLocale,
            format!("citation locale `{locale_id}` is not a packaged Weftext asset"),
            None,
            None,
            None,
        )));
    }
    let requested = LocaleCode(locale_id.to_owned());
    let english = LocaleCode::en_us();
    let mut found_requested = false;
    let mut found_english = false;
    let locales = archive::locales()
        .into_iter()
        .filter(|locale| {
            let keep =
                locale.lang.as_ref() == Some(&requested) || locale.lang.as_ref() == Some(&english);
            found_requested |= locale.lang.as_ref() == Some(&requested);
            found_english |= locale.lang.as_ref() == Some(&english);
            keep
        })
        .collect::<Vec<_>>();
    if !found_requested || !found_english {
        return Err(failure(diagnostic(
            CitationPresentationDiagnosticCode::UnavailableLocale,
            format!("citation locale `{locale_id}` is missing from the packaged archive"),
            None,
            None,
            None,
        )));
    }
    Ok(locales)
}

fn present_component(
    component: &BibliographyComponentInput,
    style: &IndependentStyle,
    locales: &[Locale],
    locale: &LocaleCode,
) -> Result<CitationComponentPresentation, CitationPresentationFailure> {
    let references = prepare_references(component)?;
    let rendered = render_provider_component(component, style, locales, locale, &references)?;
    let mut citations = Vec::with_capacity(component.clusters.len());
    for (cluster, provider) in component.clusters.iter().zip(rendered.citations.iter()) {
        citations.push(present_cluster(
            component.component_node_id,
            cluster,
            provider,
        )?);
    }

    let bibliography = match (&component.placement, rendered.bibliography) {
        (None, _) => None,
        (Some(placement), None) => {
            return Err(failure(diagnostic(
                CitationPresentationDiagnosticCode::MissingBibliography,
                "the selected style did not return a bibliography".to_owned(),
                Some(component.component_node_id),
                None,
                Some(placement.range.clone()),
            )));
        }
        (Some(placement), Some(provider)) => Some(present_bibliography(
            component,
            placement.range.clone(),
            provider,
            &references,
        )?),
    };

    Ok(CitationComponentPresentation {
        component_node_id: component.component_node_id,
        revision: component.revision.clone(),
        citations,
        bibliography,
    })
}

fn render_provider_component<'a>(
    component: &'a BibliographyComponentInput,
    style: &'a IndependentStyle,
    locales: &'a [Locale],
    locale: &LocaleCode,
    references: &'a BTreeMap<NodeId, ProviderReference>,
) -> Result<hayagriva::Rendered, CitationPresentationFailure> {
    let mut driver = BibliographyDriver::new();
    let mut cited_node_ids = BTreeSet::new();

    for (cluster_index, cluster) in component.clusters.iter().enumerate() {
        let mut items = Vec::with_capacity(cluster.items.len());
        for item in &cluster.items {
            cited_node_ids.insert(item.reference.node_id);
            let provider_reference = references
                .get(&item.reference.node_id)
                .expect("validated reference must exist");
            let locator = item
                .locator
                .as_deref()
                .map(|value| {
                    Locator::from_str(&item.label)
                        .map(|label| SpecificLocator(label, LocatorPayload::Str(value)))
                        .map_err(|()| {
                            failure(diagnostic(
                                CitationPresentationDiagnosticCode::UnsupportedLocator,
                                format!("locator label `{}` is not supported", item.label),
                                Some(component.component_node_id),
                                Some(item.reference.node_id),
                                Some(item.range.clone()),
                            ))
                        })
                })
                .transpose()?;
            let purpose = (cluster.form == CitationForm::Narrative).then_some(CitePurpose::Prose);
            items.push(ProviderCitationItem::new(
                &provider_reference.item,
                locator,
                None,
                false,
                purpose,
            ));
        }
        driver.citation(CitationRequest::new(
            items,
            style,
            Some(locale.clone()),
            locales,
            Some(cluster_index + 1),
        ));
    }

    let hidden_request_count = if component.placement.is_some() {
        let hidden_items = component
            .references
            .iter()
            .filter(|reference| !cited_node_ids.contains(&reference.node_id))
            .map(|reference| {
                ProviderCitationItem::new(
                    &references
                        .get(&reference.node_id)
                        .expect("validated bibliography reference must exist")
                        .item,
                    None,
                    None,
                    true,
                    None,
                )
            })
            .collect::<Vec<_>>();
        if hidden_items.is_empty() {
            0
        } else {
            driver.citation(CitationRequest::new(
                hidden_items,
                style,
                Some(locale.clone()),
                locales,
                None,
            ));
            1
        }
    } else {
        0
    };

    let rendered = driver.finish(BibliographyRequest::new(
        style,
        Some(locale.clone()),
        locales,
    ));
    let expected_citations = component.clusters.len() + hidden_request_count;
    if rendered.citations.len() != expected_citations {
        return Err(malformed_component(
            component,
            format!(
                "provider returned {} citations for {expected_citations} requests",
                rendered.citations.len()
            ),
        ));
    }

    Ok(rendered)
}

#[derive(Debug)]
struct ProviderReference {
    item: CslItem,
}

fn prepare_references(
    component: &BibliographyComponentInput,
) -> Result<BTreeMap<NodeId, ProviderReference>, CitationPresentationFailure> {
    let mut data_by_node = BTreeMap::<NodeId, CitationData>::new();
    let mut node_by_key = BTreeMap::<String, NodeId>::new();
    for reference in component
        .references
        .iter()
        .map(|reference| (reference.node_id, &reference.citation_data, None))
        .chain(component.clusters.iter().flat_map(|cluster| {
            cluster.items.iter().map(|item| {
                (
                    item.reference.node_id,
                    &item.reference.citation_data,
                    Some(item.range.clone()),
                )
            })
        }))
    {
        let (node_id, citation_data, source_range) = reference;
        if let Some(existing) = data_by_node.get(&node_id) {
            if existing != citation_data {
                return Err(failure(diagnostic(
                    CitationPresentationDiagnosticCode::ConflictingReference,
                    format!("reference {node_id} has conflicting Citation Data"),
                    Some(component.component_node_id),
                    Some(node_id),
                    source_range,
                )));
            }
        } else {
            data_by_node.insert(node_id, citation_data.clone());
        }
        if let Some(existing_node_id) = node_by_key.insert(citation_data.key.clone(), node_id)
            && existing_node_id != node_id
        {
            return Err(failure(diagnostic(
                CitationPresentationDiagnosticCode::ConflictingReference,
                format!(
                    "citation key `{}` maps to multiple UUIDs",
                    citation_data.key
                ),
                Some(component.component_node_id),
                None,
                source_range,
            )));
        }
    }

    data_by_node
        .into_iter()
        .map(|(node_id, citation_data)| {
            let item = to_provider_item(component.component_node_id, node_id, &citation_data)?;
            Ok((node_id, ProviderReference { item }))
        })
        .collect()
}

fn to_provider_item(
    component_node_id: NodeId,
    node_id: NodeId,
    data: &CitationData,
) -> Result<CslItem, CitationPresentationFailure> {
    if data.key.is_empty() || data.title.is_empty() || data.item_type.is_empty() {
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            "required Citation Data is empty",
        ));
    }
    if Kind::from_str(&data.item_type).is_err() {
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            format!("item type `{}` is unsupported", data.item_type),
        ));
    }
    ensure_plain_provider_text(component_node_id, node_id, "reference.title", &data.title)?;

    let mut fields = BTreeMap::new();
    fields.insert("id".to_owned(), CslValue::String(node_id.to_string()));
    fields.insert("type".to_owned(), CslValue::String(data.item_type.clone()));
    fields.insert("title".to_owned(), CslValue::String(data.title.clone()));
    for (name, value) in &data.fields {
        if matches!(name.as_str(), "key" | "type" | "title") {
            continue;
        }
        if name == "note" {
            return Err(unsupported_reference(
                component_node_id,
                node_id,
                "reference.note is unavailable because this provider interprets it as control syntax",
            ));
        }
        let value = match value {
            ReferenceValue::Text(value) => {
                ensure_plain_provider_text(component_node_id, node_id, name, value)?;
                CslValue::String(value.clone())
            }
            ReferenceValue::Names(names) => CslValue::Names(
                names
                    .iter()
                    .map(|name_value| {
                        to_provider_name(component_node_id, node_id, name, name_value)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ReferenceValue::Date(reference_date) => CslValue::Date(to_provider_date(
                component_node_id,
                node_id,
                name,
                reference_date,
            )?),
        };
        fields.insert(name.clone(), value);
    }
    Ok(CslItem(fields))
}

fn to_provider_name(
    component_node_id: NodeId,
    node_id: NodeId,
    field: &str,
    name: &ReferenceName,
) -> Result<NameValue, CitationPresentationFailure> {
    if let Some(literal) = &name.literal {
        ensure_plain_provider_text(component_node_id, node_id, field, literal)?;
        return Ok(NameValue::Literal(LiteralName {
            literal: literal.clone(),
        }));
    }
    for value in [
        name.family.as_deref(),
        name.given.as_deref(),
        name.non_dropping_particle.as_deref(),
        name.dropping_particle.as_deref(),
        name.suffix.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        ensure_plain_provider_text(component_node_id, node_id, field, value)?;
    }
    Ok(NameValue::Item(NameItem {
        family: name.family.clone().unwrap_or_default(),
        given: name.given.clone(),
        non_dropping_particle: name.non_dropping_particle.clone(),
        dropping_particle: name.dropping_particle.clone(),
        suffix: name.suffix.clone(),
    }))
}

fn to_provider_date(
    component_node_id: NodeId,
    node_id: NodeId,
    field: &str,
    date: &ReferenceDate,
) -> Result<DateValue, CitationPresentationFailure> {
    if date.circa == Some(true) {
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            format!("{field}.circa is not supported by the pinned CSL provider"),
        ));
    }
    if let Some(literal) = &date.literal {
        ensure_plain_provider_text(component_node_id, node_id, field, literal)?;
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            format!("{field}.literal is not supported by the pinned CSL provider"),
        ));
    }
    let Some(parts) = &date.date_parts else {
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            format!("{field} has no provider-compatible date-parts"),
        ));
    };
    let mut provider_parts = Vec::with_capacity(parts.len());
    for part in parts {
        let converted = part
            .iter()
            .copied()
            .map(i16::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                unsupported_reference(
                    component_node_id,
                    node_id,
                    format!("{field}.date-parts exceeds the provider integer range"),
                )
            })?;
        provider_parts.push(VecDate(converted));
    }
    if let Some(season) = &date.season {
        ensure_plain_provider_text(component_node_id, node_id, field, season)?;
    }
    Ok(DateValue::DateParts {
        date_parts: VecDateRange(provider_parts),
        literal: None,
        season: date.season.clone(),
    })
}

fn ensure_plain_provider_text(
    component_node_id: NodeId,
    node_id: NodeId,
    field: &str,
    value: &str,
) -> Result<(), CitationPresentationFailure> {
    if value.len() > MAX_PROVIDER_TEXT_BYTES {
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            format!("{field} exceeds the provider text limit"),
        ));
    }
    if value.contains('<') {
        return Err(unsupported_reference(
            component_node_id,
            node_id,
            format!("{field} contains markup-like text that cannot be passed as plain text"),
        ));
    }
    Ok(())
}

fn present_cluster(
    component_node_id: NodeId,
    cluster: &ResolvedCitationCluster,
    provider: &hayagriva::RenderedCitation,
) -> Result<PresentedCitation, CitationPresentationFailure> {
    let associations = cluster
        .items
        .iter()
        .map(|item| item.reference.node_id)
        .collect::<Vec<_>>();
    let affixes = cluster
        .items
        .iter()
        .map(|item| (item.prefix.as_deref(), item.suffix.as_deref()))
        .collect::<Vec<_>>();
    let mut seen_affixes = vec![0_usize; affixes.len()];
    let content = convert_children(
        &provider.citation,
        None,
        Some((&associations, &affixes, &mut seen_affixes)),
    )?;
    if affixes
        .iter()
        .zip(seen_affixes.iter())
        .any(|((prefix, suffix), count)| (prefix.is_some() || suffix.is_some()) && *count != 1)
    {
        return Err(failure(diagnostic(
            CitationPresentationDiagnosticCode::MalformedProviderOutput,
            "the provider did not preserve a unique entry boundary for citation affixes".to_owned(),
            Some(component_node_id),
            None,
            Some(cluster.range.clone()),
        )));
    }
    if content.plain_text().trim().is_empty() {
        let code = if cluster.form == CitationForm::Narrative {
            CitationPresentationDiagnosticCode::UnsupportedNarrative
        } else {
            CitationPresentationDiagnosticCode::MalformedProviderOutput
        };
        return Err(failure(diagnostic(
            code,
            "the selected style returned an empty citation".to_owned(),
            Some(component_node_id),
            None,
            Some(cluster.range.clone()),
        )));
    }
    Ok(PresentedCitation {
        source_range: cluster.range.clone(),
        form: cluster.form,
        note_number: provider.note_number,
        reference_node_ids: associations,
        content,
    })
}

fn present_bibliography(
    component: &BibliographyComponentInput,
    source_range: Range<u64>,
    provider: hayagriva::RenderedBibliography,
    references: &BTreeMap<NodeId, ProviderReference>,
) -> Result<PresentedBibliography, CitationPresentationFailure> {
    let expected = component
        .references
        .iter()
        .map(|reference| reference.node_id)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(provider.items.len());
    for item in provider.items {
        let node_id = NodeId::from_str(&item.key).map_err(|_| {
            malformed_component(
                component,
                format!("provider returned non-UUID bibliography key `{}`", item.key),
            )
        })?;
        if !expected.contains(&node_id)
            || !references.contains_key(&node_id)
            || !seen.insert(node_id)
        {
            return Err(malformed_component(
                component,
                format!("provider returned unexpected bibliography reference {node_id}"),
            ));
        }
        let first_field = item
            .first_field
            .as_ref()
            .map(|field| convert_child(field, Some(node_id), None))
            .transpose()?;
        let content = convert_children(&item.content, Some(node_id), None)?;
        entries.push(PresentedBibliographyEntry {
            reference_node_id: node_id,
            first_field,
            content,
        });
    }
    if seen != expected {
        return Err(malformed_component(
            component,
            "provider bibliography omitted one or more requested references".to_owned(),
        ));
    }
    Ok(PresentedBibliography {
        source_range,
        hanging_indent: provider.hanging_indent,
        second_field_align: provider.second_field_align.map(|align| match align {
            SecondFieldAlign::Margin => CitationSecondFieldAlign::Margin,
            SecondFieldAlign::Flush => CitationSecondFieldAlign::Flush,
        }),
        line_spacing: provider.line_spacing.get(),
        entry_spacing: provider.entry_spacing,
        entries,
    })
}

type AffixContext<'a> = (
    &'a [NodeId],
    &'a [(Option<&'a str>, Option<&'a str>)],
    &'a mut [usize],
);

fn convert_children(
    children: &ElemChildren,
    association: Option<NodeId>,
    mut affix_context: Option<AffixContext<'_>>,
) -> Result<CitationRichText, CitationPresentationFailure> {
    let mut result = CitationRichText::default();
    for child in &children.0 {
        append_child(&mut result, child, association, affix_context.as_mut())?;
    }
    Ok(result)
}

fn convert_child(
    child: &ElemChild,
    association: Option<NodeId>,
    mut affix_context: Option<AffixContext<'_>>,
) -> Result<CitationRichText, CitationPresentationFailure> {
    let mut result = CitationRichText::default();
    append_child(&mut result, child, association, affix_context.as_mut())?;
    Ok(result)
}

fn append_child(
    result: &mut CitationRichText,
    child: &ElemChild,
    association: Option<NodeId>,
    affix_context: Option<&mut AffixContext<'_>>,
) -> Result<(), CitationPresentationFailure> {
    match child {
        ElemChild::Text(text) => push_run(
            result,
            text.text.clone(),
            provider_style(text.formatting),
            None,
            association,
        ),
        ElemChild::Link { text, url } => {
            validate_provider_link(url)?;
            push_run(
                result,
                text.text.clone(),
                provider_style(text.formatting),
                Some(url.clone()),
                association,
            );
        }
        ElemChild::Elem(element) => append_element(result, element, association, affix_context)?,
        ElemChild::Markup(_) => {
            return Err(malformed_output(
                "provider returned executable or markup-rich content",
            ));
        }
        ElemChild::Transparent { .. } => {
            return Err(malformed_output(
                "provider returned an unresolved transparent element",
            ));
        }
    }
    Ok(())
}

fn append_element(
    result: &mut CitationRichText,
    element: &Elem,
    association: Option<NodeId>,
    mut affix_context: Option<&mut AffixContext<'_>>,
) -> Result<(), CitationPresentationFailure> {
    let block = element.display == Some(Display::Block);
    if block {
        push_run(
            result,
            "\n".to_owned(),
            CitationTextStyle::default(),
            None,
            association,
        );
    }
    if let Some(ElemMeta::Entry(index)) = element.meta {
        let Some((associations, affixes, seen)) = affix_context.as_deref_mut() else {
            return Err(malformed_output(
                "provider returned an unexpected citation entry association",
            ));
        };
        let Some(node_id) = associations.get(index).copied() else {
            return Err(malformed_output(
                "provider returned an out-of-range citation entry association",
            ));
        };
        let Some((prefix, suffix)) = affixes.get(index).copied() else {
            return Err(malformed_output(
                "provider returned an out-of-range citation affix association",
            ));
        };
        seen[index] += 1;
        if let Some(prefix) = prefix {
            push_run(
                result,
                prefix.to_owned(),
                CitationTextStyle::default(),
                None,
                Some(node_id),
            );
        }
        for child in &element.children.0 {
            append_child(
                result,
                child,
                Some(node_id),
                Some(&mut **affix_context.as_mut().unwrap()),
            )?;
        }
        if let Some(suffix) = suffix {
            push_run(
                result,
                suffix.to_owned(),
                CitationTextStyle::default(),
                None,
                Some(node_id),
            );
        }
    } else {
        for child in &element.children.0 {
            append_child(result, child, association, affix_context.as_deref_mut())?;
        }
    }
    if block {
        push_run(
            result,
            "\n".to_owned(),
            CitationTextStyle::default(),
            None,
            association,
        );
    }
    Ok(())
}

fn provider_style(formatting: ProviderFormatting) -> CitationTextStyle {
    CitationTextStyle {
        italic: formatting.font_style == FontStyle::Italic,
        small_caps: formatting.font_variant == FontVariant::SmallCaps,
        weight: match formatting.font_weight {
            FontWeight::Normal => CitationFontWeight::Normal,
            FontWeight::Bold => CitationFontWeight::Bold,
            FontWeight::Light => CitationFontWeight::Light,
        },
        underline: formatting.text_decoration == TextDecoration::Underline,
        vertical_align: match formatting.vertical_align {
            VerticalAlign::None => CitationVerticalAlign::None,
            VerticalAlign::Baseline => CitationVerticalAlign::Baseline,
            VerticalAlign::Sup => CitationVerticalAlign::Superscript,
            VerticalAlign::Sub => CitationVerticalAlign::Subscript,
        },
    }
}

fn push_run(
    result: &mut CitationRichText,
    text: String,
    style: CitationTextStyle,
    link: Option<String>,
    reference_node_id: Option<NodeId>,
) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = result.runs.last_mut()
        && last.style == style
        && last.link == link
        && last.reference_node_id == reference_node_id
    {
        last.text.push_str(&text);
        return;
    }
    result.runs.push(CitationRichTextRun {
        text,
        style,
        link,
        reference_node_id,
    });
}

fn validate_provider_link(url: &str) -> Result<(), CitationPresentationFailure> {
    let safe_scheme = url
        .split_once(':')
        .is_some_and(|(scheme, _)| matches!(scheme, "http" | "https"));
    if safe_scheme && !url.chars().any(char::is_control) {
        Ok(())
    } else {
        Err(malformed_output(
            "provider returned a link outside the http/https allowlist",
        ))
    }
}

fn limit_failure(
    dimension: &str,
    limit: usize,
    component_node_id: Option<NodeId>,
) -> CitationPresentationFailure {
    failure(diagnostic(
        CitationPresentationDiagnosticCode::RequestLimitExceeded,
        format!("presentation request exceeds the {limit} {dimension} limit"),
        component_node_id,
        None,
        None,
    ))
}

fn unsupported_reference(
    component_node_id: NodeId,
    reference_node_id: NodeId,
    message: impl Into<String>,
) -> CitationPresentationFailure {
    failure(diagnostic(
        CitationPresentationDiagnosticCode::UnsupportedReferenceData,
        message.into(),
        Some(component_node_id),
        Some(reference_node_id),
        None,
    ))
}

fn malformed_component(
    component: &BibliographyComponentInput,
    message: String,
) -> CitationPresentationFailure {
    failure(diagnostic(
        CitationPresentationDiagnosticCode::MalformedProviderOutput,
        message,
        Some(component.component_node_id),
        None,
        component
            .placement
            .as_ref()
            .map(|placement| placement.range.clone()),
    ))
}

fn malformed_output(message: &str) -> CitationPresentationFailure {
    failure(diagnostic(
        CitationPresentationDiagnosticCode::MalformedProviderOutput,
        message.to_owned(),
        None,
        None,
        None,
    ))
}

fn diagnostic(
    code: CitationPresentationDiagnosticCode,
    message: String,
    component_node_id: Option<NodeId>,
    reference_node_id: Option<NodeId>,
    source_range: Option<Range<u64>>,
) -> CitationPresentationDiagnostic {
    CitationPresentationDiagnostic {
        code,
        message,
        component_node_id,
        reference_node_id,
        source_range,
    }
}

fn failure(diagnostic: CitationPresentationDiagnostic) -> CitationPresentationFailure {
    CitationPresentationFailure {
        diagnostics: vec![diagnostic],
    }
}
