//! Bibliography exchange is a frozen projection of typed Work publication
//! values. This module does not resolve identities, open files, or write a Bib DB.
use crate::{ExportError, ExportLoss, Result};
use biblatex::{Bibliography, Chunk, ChunksExt};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use weftext_data::{NativeMember, NativeValue};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_RECORDS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BibliographyFormat {
    Bibtex,
    CslJson,
    Ris,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenWork {
    pub entity_ref: String,
    pub source_basis: String,
    pub publication: NativeValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationWork {
    pub key: String,
    pub work: FrozenWork,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyDependency {
    pub key: String,
    pub entity_ref: String,
    pub source_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographyProjection {
    pub bytes: Vec<u8>,
    pub format: BibliographyFormat,
    pub losses: Vec<ExportLoss>,
    pub dependencies: Vec<BibliographyDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSource {
    pub path: String,
    pub original_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPublication {
    pub suggested_key: Option<String>,
    pub publication: NativeValue,
    pub original_record: Value,
    pub unknown_fields: Value,
    pub sources: Vec<FieldSource>,
    pub conflicts: Vec<PublicationConflict>,
}

/// A proposal retains exact upload bytes before mapping. Identity is deliberately
/// absent: DOI/title/key matching never creates an automatic upsert instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBibliographyProposal {
    pub format: BibliographyFormat,
    pub original: Vec<u8>,
    pub records: Vec<ImportedPublication>,
    pub losses: Vec<ExportLoss>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationConflict {
    pub key: String,
    pub current: NativeValue,
    pub imported: NativeValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationMergeProposal {
    pub publication: NativeValue,
    pub conflicts: Vec<PublicationConflict>,
}

/// Existing authored values win in the provisional image. Every differing
/// imported member is returned for an explicit choice; arrays are not merged by
/// names, DOI, or ordinal. Unknown authored members also survive.
pub fn propose_publication_merge(
    current: &NativeValue,
    imported: &NativeValue,
) -> Result<PublicationMergeProposal> {
    let mut members = current
        .record()
        .ok_or_else(|| {
            fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_current_publication_not_record(),
            ))
        })?
        .to_vec();
    let incoming = imported.record().ok_or_else(|| {
        fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_imported_publication_not_record(
            ),
        ))
    })?;
    let mut conflicts = vec![];
    for member in incoming {
        if let Some(existing) = members.iter().find(|m| m.key == member.key) {
            if existing.value != member.value {
                conflicts.push(PublicationConflict {
                    key: member.key.clone(),
                    current: existing.value.clone(),
                    imported: member.value.clone(),
                });
            }
        } else {
            members.push(member.clone());
        }
    }
    Ok(PublicationMergeProposal {
        publication: NativeValue::Record(members),
        conflicts,
    })
}

pub fn project_bibliography(
    works: &[CitationWork],
    format: BibliographyFormat,
) -> Result<BibliographyProjection> {
    if works.len() > MAX_RECORDS {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_record_budget_exceeded(),
        )));
    }
    let mut keys = BTreeSet::new();
    let mut losses = vec![];
    let mut rows = vec![];
    for work in works {
        if !citation_key(&work.key) || !keys.insert(work.key.clone()) {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_citation_key_invalid_or_duplicate(),
            )));
        }
        if !weftext_data::scalar_valid("node_ref", &NativeValue::Text(work.work.entity_ref.clone()))
            || work.work.source_basis.is_empty()
        {
            return Err(fail(
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_stable_work_and_frozen_basis_required()),
            ));
        }
        work.work
            .publication
            .to_typst()
            .map_err(ExportError::from_data)?;
        rows.push(project_work(work, &mut losses)?);
    }
    let bytes = match format {
        BibliographyFormat::CslJson => serde_json::to_vec_pretty(&rows)?,
        BibliographyFormat::Bibtex => rows
            .iter()
            .map(csl_to_bib)
            .collect::<Result<Vec<_>>>()?
            .join("\n")
            .into_bytes(),
        BibliographyFormat::Ris => rows
            .iter()
            .map(csl_to_ris)
            .collect::<Result<Vec<_>>>()?
            .join("\n")
            .into_bytes(),
    };
    if bytes.len() > MAX_BYTES {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_generated_bytes_exceeded(),
        )));
    }
    Ok(BibliographyProjection {
        bytes,
        format,
        losses,
        dependencies: works
            .iter()
            .map(|w| BibliographyDependency {
                key: w.key.clone(),
                entity_ref: w.work.entity_ref.clone(),
                source_basis: w.work.source_basis.clone(),
            })
            .collect(),
    })
}

fn project_work(work: &CitationWork, losses: &mut Vec<ExportLoss>) -> Result<Value> {
    let p = &work.work.publication;
    let members = p.record().ok_or_else(|| {
        fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_publication_not_record(
                weftext_messages::params::ErrorExportBibliographyPublicationNotRecord {
                    work_key: (work.key).to_string(),
                },
            ),
        ))
    })?;
    let required = |key| {
        text(p, key).filter(|s| !s.is_empty()).ok_or_else(|| {
            fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_required_publication_member(
                    weftext_messages::params::ErrorExportBibliographyRequiredPublicationMember {
                        work_key: (work.key).to_string(),
                        member: (key).to_string(),
                    },
                ),
            ))
        })
    };
    let title = required("title")?;
    let kind = required("kind")?;
    let csl_kind = match kind {
        "article" => "article-journal",
        "book" => "book",
        "report" => "report",
        "standard" => "standard",
        "dataset" => "dataset",
        "chapter" => "chapter",
        "thesis" => "thesis",
        "web" => "webpage",
        _ => {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_unsupported_work_kind(
                    weftext_messages::params::ErrorExportBibliographyUnsupportedWorkKind {
                        work_key: (work.key).to_string(),
                        kind: (kind).to_string(),
                    },
                ),
            )));
        }
    };
    let mut row = Map::new();
    row.insert("id".into(), json!(work.key));
    row.insert("type".into(), json!(csl_kind));
    row.insert("title".into(), json!(title));
    row.extend(project_contributors(p, &work.key, losses, true)?);
    for (native, csl) in [
        ("container", "container-title"),
        ("volume", "volume"),
        ("issue", "issue"),
        ("pages", "page"),
        ("abstract", "abstract"),
        ("url", "URL"),
        ("language", "language"),
    ] {
        if let Some(value) = text(p, native) {
            row.insert(csl.into(), json!(value));
        }
    }
    if let Some(keywords) = project_keywords(p)? {
        row.insert("keyword".into(), keywords);
    }
    if let Some(issued) = p.member("issued") {
        match date_parts(issued) {
            Some(parts) => {
                row.insert("issued".into(), json!({"date-parts":[parts]}));
            }
            None => loss(
                losses,
                "datePrecision",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_issued_date_not_gregorian_representable()),
                format!("{}.issued", work.key),
            ),
        }
    }
    row.extend(project_identifiers(p, &work.key, losses)?);
    for member in members {
        if !matches!(
            member.key.as_str(),
            "kind"
                | "title"
                | "contributors"
                | "container"
                | "volume"
                | "issue"
                | "pages"
                | "abstract"
                | "url"
                | "language"
                | "keywords"
                | "issued"
                | "identifiers"
                | "citation_key"
        ) {
            loss(
                losses,
                "publicationExtension",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_publication_member_extension_only()),
                format!("{}.{}", work.key, member.key),
            );
        }
    }
    // Exact tagged values preserve ordered records, unknown members and numeric
    // spelling. The association is provenance, never an import identity command.
    row.insert("weftext".into(), json!({"version":1,"work":work.work.entity_ref,"sourceBasis":work.work.source_basis,"publication":p}));
    Ok(Value::Object(row))
}

fn project_contributors(
    p: &NativeValue,
    key: &str,
    losses: &mut Vec<ExportLoss>,
    complete: bool,
) -> Result<Map<String, Value>> {
    let mut row = Map::new();
    let contributors = p
        .member("contributors")
        .and_then(NativeValue::array)
        .ok_or_else(|| {
            fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_contributors_unavailable(
                    weftext_messages::params::ErrorExportBibliographyContributorsUnavailable {
                        work_key: (key).to_string(),
                    },
                ),
            ))
        })?;
    if complete && contributors.is_empty() {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_contributors_unavailable(
                weftext_messages::params::ErrorExportBibliographyContributorsUnavailable {
                    work_key: (key).to_string(),
                },
            ),
        )));
    }
    let mut authors = vec![];
    let mut editors = vec![];
    for (index, contributor) in contributors.iter().enumerate() {
        let agent = contributor.member("agent").ok_or_else(|| {
            fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_contributor_agent_unavailable(weftext_messages::params::ErrorExportBibliographyContributorAgentUnavailable {
                    work_key: (key).to_string(),
                    index: weftext_messages::UiInteger::new(
                        i64::try_from(index).expect("bounded product count"),
                    )
                    .expect("bounded product count"),
                }),
            ))
        })?;
        let agent_ref = agent.member("ref");
        let agent_text = agent.member("text");
        if agent.record().is_none()
            || agent_ref.is_some() == agent_text.is_some()
            || agent_ref.is_some_and(|value| !weftext_data::scalar_valid("node_ref", value))
            || agent_text.is_some_and(|value| !weftext_data::scalar_valid("nonempty_text", value))
        {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_contributor_agent_branch(
                    weftext_messages::params::ErrorExportBibliographyContributorAgentBranch {
                        work_key: (key).to_string(),
                        index: weftext_messages::UiInteger::new(
                            i64::try_from(index).expect("bounded product count"),
                        )
                        .expect("bounded product count"),
                    },
                ),
            )));
        }
        let family = text(contributor, "family").filter(|s| !s.is_empty());
        let given = text(contributor, "given").filter(|s| !s.is_empty());
        let literal = text(contributor, "credited_as")
            .filter(|s| !s.is_empty())
            .or_else(|| text(agent, "text").filter(|s| !s.is_empty()));
        let mut person = Map::new();
        if let Some(family) = family {
            person.insert("family".into(), json!(family));
            if let Some(given) = given {
                person.insert("given".into(), json!(given));
            }
        } else if let Some(literal) = literal {
            person.insert("literal".into(), json!(literal));
        } else {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_contributor_name_missing(
                    weftext_messages::params::ErrorExportBibliographyContributorNameMissing {
                        work_key: (key).to_string(),
                        index: weftext_messages::UiInteger::new(
                            i64::try_from(index).expect("bounded product count"),
                        )
                        .expect("bounded product count"),
                    },
                ),
            )));
        }
        let roles = contributor
            .member("roles")
            .and_then(NativeValue::array)
            .ok_or_else(|| {
                fail(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_contributor_role_unknown(
                        weftext_messages::params::ErrorExportBibliographyContributorRoleUnknown {
                            work_key: (key).to_string(),
                            index: weftext_messages::UiInteger::new(
                                i64::try_from(index).expect("bounded product count"),
                            )
                            .expect("bounded product count"),
                        },
                    ),
                ))
            })?;
        let mut mapped = false;
        let mut seen_roles = BTreeSet::new();
        for role in roles {
            if let Some(role) = role.text()
                && !seen_roles.insert(role)
            {
                loss(
                    losses,
                    "duplicateContributorRole",
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_repeated_role_extension_only()),
                    format!("{}.contributors[{index}].roles", key),
                );
                continue;
            }
            match role.text() {
                Some("author") => {
                    authors.push(Value::Object(person.clone()));
                    mapped = true;
                }
                Some("editor") => {
                    editors.push(Value::Object(person.clone()));
                    mapped = true;
                }
                Some(role) => loss(
                    losses,
                    "contributorRole",
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_role_retained_in_extension(weftext_messages::params::ErrorExportBibliographyRoleRetainedInExtension { role: (role).to_string() })),
                    format!("{}.contributors[{index}].roles", key),
                ),
                None => return Err(fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_contributor_role_not_text(weftext_messages::params::ErrorExportBibliographyContributorRoleNotText { work_key: (key).to_string() })))),
            }
        }
        if !mapped {
            loss(
                losses,
                "contributorNotInStandardCitation",
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_contributor_extension_only(),
                ),
                format!("{}.contributors[{index}]", key),
            );
        }
        if agent_ref.is_some()
            || contributor.record().is_some_and(|members| {
                members.iter().any(|member| {
                    !matches!(
                        member.key.as_str(),
                        "agent" | "credited_as" | "family" | "given" | "roles"
                    )
                })
            })
        {
            loss(
                losses,
                "contributorExtension",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_contributor_details_extension_only()),
                format!("{}.contributors[{index}]", key),
            );
        }
    }
    if complete && authors.is_empty() && editors.is_empty() {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_author_editor_missing(
                weftext_messages::params::ErrorExportBibliographyAuthorEditorMissing {
                    work_key: (key).to_string(),
                },
            ),
        )));
    }
    if !complete || !authors.is_empty() {
        row.insert("author".into(), Value::Array(authors));
    }
    if !complete || !editors.is_empty() {
        row.insert("editor".into(), Value::Array(editors));
    }
    Ok(row)
}

fn project_identifiers(
    p: &NativeValue,
    key: &str,
    losses: &mut Vec<ExportLoss>,
) -> Result<Map<String, Value>> {
    let mut row = Map::new();
    let mut schemes = BTreeSet::new();
    if let Some(identifiers) = p.member("identifiers").and_then(NativeValue::array) {
        for (index, identifier) in identifiers.iter().enumerate() {
            let scheme =
                text(identifier, "scheme").ok_or_else(|| fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_identifier_scheme_unavailable())))?;
            let value =
                text(identifier, "value").ok_or_else(|| fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_identifier_value_unavailable())))?;
            let csl = match scheme {
                "doi" => Some("DOI"),
                "isbn" => Some("ISBN"),
                "issn" => Some("ISSN"),
                _ => None,
            };
            if let Some(csl) = csl.filter(|_| schemes.insert(scheme.to_owned())) {
                row.insert(csl.into(), json!(value));
            } else {
                loss(
                    losses,
                    "identifier",
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_identifier_details_extension_only()),
                    format!("{}.identifiers[{index}]", key),
                );
            }
        }
    }
    Ok(row)
}

fn project_keywords(p: &NativeValue) -> Result<Option<Value>> {
    p.member("keywords")
        .and_then(NativeValue::array)
        .map(|values| {
            values
                .iter()
                .map(NativeValue::text)
                .collect::<Option<Vec<_>>>()
                .map(|values| json!(values.join(", ")))
                .ok_or_else(|| fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_publication_keyword_not_text())))
        })
        .transpose()
}

fn csl_to_bib(row: &Value) -> Result<String> {
    let kind = match row["type"].as_str().unwrap_or("") {
        "article-journal" => "article",
        "book" => "book",
        "chapter" => "incollection",
        "report" => "report",
        "standard" => "standard",
        "dataset" => "dataset",
        "thesis" => "thesis",
        "webpage" => "online",
        _ => {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_csl_type_unsupported(),
            )));
        }
    };
    let mut fields = vec![];
    for (csl, bib) in [
        ("title", "title"),
        ("volume", "volume"),
        ("issue", "number"),
        ("page", "pages"),
        ("abstract", "abstract"),
        ("URL", "url"),
        ("language", "langid"),
        ("keyword", "keywords"),
        ("DOI", "doi"),
        ("ISBN", "isbn"),
        ("ISSN", "issn"),
    ] {
        if let Some(value) = row[csl].as_str() {
            fields.push(format!("  {bib} = {{{}}}", bib_text(value)));
        }
    }
    if let Some(container) = row["container-title"].as_str() {
        fields.push(format!(
            "  {} = {{{}}}",
            if kind == "article" {
                "journal"
            } else {
                "booktitle"
            },
            bib_text(container)
        ));
    }
    for role in ["author", "editor"] {
        if let Some(names) = row[role].as_array() {
            let names = names
                .iter()
                .map(|name| {
                    if let Some(literal) = name["literal"].as_str() {
                        format!("{{{}}}", bib_text(literal))
                    } else {
                        let family = bib_text(name["family"].as_str().unwrap_or(""));
                        match name["given"].as_str() {
                            Some(given) => format!("{{{family}}}, {{{}}}", bib_text(given)),
                            None => format!("{{{family}}}"),
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" and ");
            fields.push(format!("  {role} = {{{names}}}"));
        }
    }
    if let Some(parts) = row["issued"]["date-parts"][0].as_array() {
        let date = parts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == 0 {
                    p.to_string()
                } else {
                    format!("{:02}", p.as_i64().unwrap())
                }
            })
            .collect::<Vec<_>>()
            .join("-");
        fields.push(format!("  date = {{{date}}}"));
        fields.push(format!("  year = {{{}}}", parts[0]));
    }
    fields.push(format!(
        "  weftextpublication = {{{}}}",
        hex(&serde_json::to_vec(&row["weftext"])?)
    ));
    Ok(format!(
        "@{kind}{{{},\n{}\n}}\n",
        row["id"].as_str().unwrap(),
        fields.join(",\n")
    ))
}

fn csl_to_ris(row: &Value) -> Result<String> {
    let kind = match row["type"].as_str().unwrap_or("") {
        "article-journal" => "JOUR",
        "book" => "BOOK",
        "chapter" => "CHAP",
        "report" => "RPRT",
        "standard" => "STAND",
        "dataset" => "DATA",
        "thesis" => "THES",
        "webpage" => "ELEC",
        _ => {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_csl_type_unsupported(),
            )));
        }
    };
    let mut lines = vec![
        format!("TY  - {kind}"),
        format!("ID  - {}", row["id"].as_str().unwrap()),
    ];
    for (csl, tag) in [
        ("title", "TI"),
        ("container-title", "T2"),
        ("volume", "VL"),
        ("issue", "IS"),
        ("page", "SP"),
        ("abstract", "AB"),
        ("URL", "UR"),
        ("language", "LA"),
        ("keyword", "KW"),
        ("DOI", "DO"),
    ] {
        if let Some(value) = row[csl].as_str() {
            ris_line(&mut lines, tag, value);
        }
    }
    if let Some(value) = row["ISBN"].as_str().or_else(|| row["ISSN"].as_str()) {
        ris_line(&mut lines, "SN", value);
    }
    for (role, tag) in [("author", "AU"), ("editor", "ED")] {
        if let Some(names) = row[role].as_array() {
            for name in names {
                let name =
                    name["literal"]
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| match name["given"].as_str() {
                            Some(given) => {
                                format!("{}, {given}", name["family"].as_str().unwrap_or(""))
                            }
                            None => name["family"].as_str().unwrap_or("").to_owned(),
                        });
                ris_line(&mut lines, tag, &name);
            }
        }
    }
    if let Some(parts) = row["issued"]["date-parts"][0].as_array() {
        let date = parts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == 0 {
                    p.to_string()
                } else {
                    format!("{:02}", p.as_i64().unwrap())
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        ris_line(&mut lines, "PY", &date);
    }
    lines.push(format!(
        "C1  - weftext-v1:{}",
        hex(&serde_json::to_vec(&row["weftext"])?)
    ));
    lines.push("ER  - ".into());
    Ok(format!("{}\n", lines.join("\n")))
}

pub fn import_bibliography(
    format: BibliographyFormat,
    bytes: &[u8],
) -> Result<ImportBibliographyProposal> {
    if bytes.len() > MAX_BYTES {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_bytes_exceeded(),
        )));
    }
    let source = std::str::from_utf8(bytes).map_err(|_| {
        fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_utf_eight_required(),
        ))
    })?;
    let mut losses = vec![];
    let records = match format {
        BibliographyFormat::CslJson => import_csl(source, &mut losses)?,
        BibliographyFormat::Bibtex => import_bib(source, &mut losses)?,
        BibliographyFormat::Ris => import_ris(source, &mut losses)?,
    };
    if records.len() > MAX_RECORDS {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_records_exceeded(),
        )));
    }
    for record in &records {
        record
            .publication
            .to_typst()
            .map_err(ExportError::from_data)?;
    }
    Ok(ImportBibliographyProposal {
        format,
        original: bytes.to_vec(),
        records,
        losses,
    })
}

fn import_csl(source: &str, losses: &mut Vec<ExportLoss>) -> Result<Vec<ImportedPublication>> {
    let value = weftext_data::parse_unique_json(source.as_bytes()).map_err(|error| {
        let mut error = ExportError::from_data(error);
        error.description.explanation =
            weftext_messages::UiMessage::error_export_bibliography_invalid_csl_json();
        error
    })?;
    let rows = value.as_array().ok_or_else(|| {
        fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_csl_record_array_required(),
        ))
    })?;
    if rows.len() > MAX_RECORDS {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_records_exceeded(),
        )));
    }
    rows.iter()
        .enumerate()
        .map(|(index, row)| map_csl(row, index, losses))
        .collect()
}

fn map_csl(row: &Value, index: usize, losses: &mut Vec<ExportLoss>) -> Result<ImportedPublication> {
    let object = row.as_object().ok_or_else(|| {
        fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_csl_item_not_record(),
        ))
    })?;
    let suggested_key = row["id"].as_str().map(str::to_owned);
    let mut fields = vec![];
    let mut sources = vec![];
    let mut ordered_sources = vec![];
    let mut known = BTreeSet::from(["id", "type", "author", "editor", "issued", "weftext"]);
    let mut add = |key: &str, value: NativeValue, original: &str| {
        fields.push(NativeMember {
            key: key.into(),
            value,
        });
        sources.push(FieldSource {
            path: format!("publication.{key}"),
            original_field: original.into(),
        });
    };
    if let Some(kind) = row["type"].as_str() {
        let kind = match kind {
            "article-journal" | "article" | "article-magazine" | "article-newspaper" => "article",
            "book" => "book",
            "chapter" | "paper-conference" => "chapter",
            "report" => "report",
            "standard" => "standard",
            "dataset" => "dataset",
            "thesis" => "thesis",
            "webpage" | "post-weblog" => "web",
            other => other,
        };
        add("kind", NativeValue::Text(kind.into()), "type");
    }
    for (csl, native) in [
        ("title", "title"),
        ("container-title", "container"),
        ("volume", "volume"),
        ("issue", "issue"),
        ("page", "pages"),
        ("abstract", "abstract"),
        ("URL", "url"),
        ("language", "language"),
    ] {
        known.insert(csl);
        if let Some(value) = row[csl].as_str() {
            add(native, NativeValue::Text(value.into()), csl);
        } else if object.contains_key(csl) {
            loss(
                losses,
                "fieldType",
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_non_text_csl_field_preserved(),
                ),
                format!("records[{index}].{csl}"),
            );
        }
    }
    known.extend(["DOI", "ISBN", "ISSN", "keyword"]);
    let mut identifiers = vec![];
    for (csl, scheme) in [("DOI", "doi"), ("ISBN", "isbn"), ("ISSN", "issn")] {
        if let Some(value) = row[csl].as_str() {
            ordered_sources.push(FieldSource {
                path: format!("publication.identifiers[{}]", identifiers.len()),
                original_field: csl.into(),
            });
            identifiers.push(record([("scheme", txt(scheme)), ("value", txt(value))]));
        }
    }
    if !identifiers.is_empty() {
        add(
            "identifiers",
            NativeValue::Array(identifiers),
            "DOI/ISBN/ISSN",
        );
    }
    if let Some(keyword) = row["keyword"].as_str() {
        add(
            "keywords",
            NativeValue::Array(vec![txt(keyword)]),
            "keyword",
        );
    }
    let mut contributors = vec![];
    for role in ["author", "editor"] {
        if let Some(names) = row[role].as_array() {
            for (ordinal, name) in names.iter().enumerate() {
                let Some(name) = name.as_object() else {
                    return Err(fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_csl_contributor_not_record())));
                };
                let family = name.get("family").and_then(Value::as_str);
                let given = name.get("given").and_then(Value::as_str);
                let literal = name.get("literal").and_then(Value::as_str);
                let credited = literal.map(str::to_owned).or_else(|| {
                    family.map(|f| given.map_or_else(|| f.into(), |g| format!("{g} {f}")))
                });
                let Some(credited) = credited.filter(|v| !v.is_empty()) else {
                    return Err(fail(
                        weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_csl_contributor_name_unsupported()),
                    ));
                };
                let mut members = vec![
                    NativeMember {
                        key: "agent".into(),
                        value: record([("text", txt(&credited))]),
                    },
                    NativeMember {
                        key: "credited_as".into(),
                        value: txt(&credited),
                    },
                    NativeMember {
                        key: "roles".into(),
                        value: NativeValue::Array(vec![txt(role)]),
                    },
                ];
                if let Some(family) = family {
                    members.push(NativeMember {
                        key: "family".into(),
                        value: txt(family),
                    });
                }
                if let Some(given) = given {
                    members.push(NativeMember {
                        key: "given".into(),
                        value: txt(given),
                    });
                }
                if name
                    .keys()
                    .any(|k| !matches!(k.as_str(), "family" | "given" | "literal"))
                {
                    loss(
                        losses,
                        "nameParts",
                        weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_additional_csl_name_parts_preserved()),
                        format!("records[{index}].{role}[{ordinal}]"),
                    );
                }
                ordered_sources.push(FieldSource {
                    path: format!("publication.contributors[{}]", contributors.len()),
                    original_field: format!("{role}[{ordinal}]"),
                });
                contributors.push(NativeValue::Record(members));
            }
        } else if object.contains_key(role) {
            return Err(fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_csl_contributors_ordered_array_required())));
        }
    }
    if !contributors.is_empty() {
        add(
            "contributors",
            NativeValue::Array(contributors),
            "author/editor",
        );
    }
    if let Some(parts) = row["issued"]["date-parts"]
        .as_array()
        .filter(|v| v.len() == 1)
        .and_then(|v| v[0].as_array())
    {
        if let Some(date) = parts
            .iter()
            .map(Value::as_i64)
            .collect::<Option<Vec<_>>>()
            .and_then(|v| precision_date(&v))
        {
            add("issued", date, "issued.date-parts");
        } else {
            loss(
                losses,
                "datePrecision",
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_csl_date_unsupported(),
                ),
                format!("records[{index}].issued"),
            );
        }
    } else if object.contains_key("issued") {
        loss(
            losses,
            "datePrecision",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_csl_imprecise_date_preserved(
                ),
            ),
            format!("records[{index}].issued"),
        );
    }
    if let Some(key) = &suggested_key {
        add("citation_key", txt(key), "id");
    }
    sources.extend(ordered_sources);
    let unknown_fields = Value::Object(
        object
            .iter()
            .filter(|(k, _)| !known.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    if unknown_fields.as_object().is_some_and(|v| !v.is_empty()) {
        loss(
            losses,
            "unknownProviderFields",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_unmapped_csl_members_preserved(),
            ),
            format!("records[{index}]"),
        );
    }
    let mapped = NativeValue::Record(fields);
    let mut publication = mapped.clone();
    let mut conflicts = vec![];
    if let Some(extension) = object.get("weftext") {
        publication = exact_extension(extension, index, losses)?;
        // Extensions preserve exact values, but an external editor can change
        // the standard fields. Surface those edits as choices instead of
        // silently restoring an older embedded title or contributor list.
        conflicts = exact_standard_conflicts(&publication, &mapped, row, index)?;
        sources.push(FieldSource {
            path: "publication".into(),
            original_field:
                "weftext.publication (exact preserved values; conflicts require a choice)".into(),
        });
    }
    Ok(ImportedPublication {
        suggested_key,
        publication,
        original_record: row.clone(),
        unknown_fields,
        sources,
        conflicts,
    })
}

/// Compare only the submitted standard fields. A partial exact record is valid
/// author data even when it cannot generate a complete citation. Complex groups
/// reuse the real forward projection; unsupported shapes remain undecidable.
fn exact_standard_conflicts(
    publication: &NativeValue,
    mapped: &NativeValue,
    row: &Value,
    index: usize,
) -> Result<Vec<PublicationConflict>> {
    let object = row.as_object().unwrap();
    let incomparable = |field: &str| {
        fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_standard_field_comparison_unavailable(weftext_messages::params::ErrorExportBibliographyStandardFieldComparisonUnavailable { index: weftext_messages::UiInteger::new(i64::try_from(index).expect("bounded product count")).expect("bounded product count"), field: (field).to_string() })))
    };
    let mut conflicts = vec![];
    let mut compare = |key: &str, imported: NativeValue| {
        conflicts.push(PublicationConflict {
            key: key.into(),
            current: publication
                .member(key)
                .cloned()
                .unwrap_or(NativeValue::None),
            imported,
        });
    };
    for (csl, native) in [
        ("type", "kind"),
        ("title", "title"),
        ("container-title", "container"),
        ("volume", "volume"),
        ("issue", "issue"),
        ("page", "pages"),
        ("abstract", "abstract"),
        ("URL", "url"),
        ("language", "language"),
    ] {
        if !object.contains_key(csl) {
            continue;
        }
        let imported = mapped.member(native).ok_or_else(|| incomparable(csl))?;
        let current = publication.member(native);
        if current.is_some_and(|v| !matches!(v, NativeValue::Text(_) | NativeValue::None)) {
            return Err(incomparable(csl));
        }
        if native == "kind" {
            let supported = |v: &NativeValue| {
                matches!(
                    v.text(),
                    Some(
                        "article"
                            | "book"
                            | "report"
                            | "standard"
                            | "dataset"
                            | "chapter"
                            | "thesis"
                            | "web"
                    )
                )
            };
            if !supported(imported)
                || current.is_some_and(|v| *v != NativeValue::None && !supported(v))
            {
                return Err(incomparable(csl));
            }
        }
        if current != Some(imported) {
            compare(native, imported.clone());
        }
    }
    if object.contains_key("issued") {
        let imported = mapped
            .member("issued")
            .ok_or_else(|| incomparable("issued"))?;
        if !row["issued"]
            .as_object()
            .is_some_and(|v| v.len() == 1 && v.contains_key("date-parts"))
        {
            return Err(incomparable("issued"));
        }
        let expected = match publication.member("issued") {
            None | Some(NativeValue::None) => Value::Null,
            Some(value) => {
                json!({"date-parts":[date_parts(value).ok_or_else(|| incomparable("issued"))?]})
            }
        };
        if row["issued"] != expected {
            compare("issued", imported.clone());
        }
    }
    if object.contains_key("keyword") {
        let imported = mapped
            .member("keywords")
            .ok_or_else(|| incomparable("keyword"))?;
        let expected = match publication.member("keywords") {
            None | Some(NativeValue::None) => Value::Null,
            Some(value) => {
                value.array().ok_or_else(|| incomparable("keyword"))?;
                project_keywords(publication)
                    .map_err(|_| incomparable("keyword"))?
                    .ok_or_else(|| incomparable("keyword"))?
            }
        };
        if row["keyword"] != expected {
            compare("keywords", imported.clone());
        }
    }
    if ["author", "editor"]
        .iter()
        .any(|role| object.contains_key(*role))
    {
        // Additional CSL name parts are retained by the ordinary importer, but
        // cannot be flattened into a proven native contributor replacement.
        for role in ["author", "editor"]
            .into_iter()
            .filter(|role| object.contains_key(*role))
        {
            let names = row[role].as_array().ok_or_else(|| incomparable(role))?;
            if names.iter().any(|name| {
                !name.as_object().is_some_and(|parts| {
                    parts.iter().all(|(key, value)| {
                        matches!(key.as_str(), "family" | "given" | "literal") && value.is_string()
                    })
                })
            }) {
                return Err(incomparable(role));
            }
        }
        let expected = match publication.member("contributors") {
            None | Some(NativeValue::None) => {
                Map::from_iter([("author".into(), json!([])), ("editor".into(), json!([]))])
            }
            Some(_) => project_contributors(publication, "exact", &mut vec![], false)
                .map_err(|_| incomparable("author/editor"))?,
        };
        if ["author", "editor"]
            .iter()
            .any(|role| object.contains_key(*role) && row[*role] != expected[*role])
        {
            compare(
                "contributors",
                mapped
                    .member("contributors")
                    .cloned()
                    .unwrap_or_else(|| NativeValue::Array(vec![])),
            );
        }
    }
    if ["DOI", "ISBN", "ISSN"]
        .iter()
        .any(|field| object.contains_key(*field))
    {
        for field in ["DOI", "ISBN", "ISSN"]
            .into_iter()
            .filter(|field| object.contains_key(*field))
        {
            if !row[field].is_string() {
                return Err(incomparable(field));
            }
        }
        if publication
            .member("identifiers")
            .is_some_and(|v| v.array().is_none() && *v != NativeValue::None)
        {
            return Err(incomparable("DOI/ISBN/ISSN"));
        }
        let expected = project_identifiers(publication, "exact", &mut vec![])
            .map_err(|_| incomparable("DOI/ISBN/ISSN"))?;
        if ["DOI", "ISBN", "ISSN"].iter().any(|field| {
            object.contains_key(*field)
                && row[*field] != expected.get(*field).cloned().unwrap_or(Value::Null)
        }) {
            compare(
                "identifiers",
                mapped
                    .member("identifiers")
                    .cloned()
                    .ok_or_else(|| incomparable("DOI/ISBN/ISSN"))?,
            );
        }
    }
    Ok(conflicts)
}

fn import_bib(source: &str, losses: &mut Vec<ExportLoss>) -> Result<Vec<ImportedPublication>> {
    let raw = biblatex::RawBibliography::parse(source).map_err(|e| {
        fail(
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_invalid_bibtex(),
            )
            .diagnostic(
                weftext_messages::DiagnosticOrigin::Dependency,
                e.to_string(),
            ),
        )
    })?;
    let mut abbreviations = BTreeSet::new();
    for abbreviation in &raw.abbreviations {
        if !abbreviations.insert(abbreviation.key.v.to_ascii_lowercase()) {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_bibtex_abbreviation_duplicate(),
            )));
        }
    }
    for entry in &raw.entries {
        let mut fields = BTreeSet::new();
        for field in &entry.v.fields {
            if !fields.insert(field.key.v.to_ascii_lowercase()) {
                return Err(fail(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_bibtex_field_duplicate(),
                )));
            }
        }
    }
    let parsed = Bibliography::from_raw(raw).map_err(|e| {
        fail(
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_invalid_bibtex(),
            )
            .diagnostic(
                weftext_messages::DiagnosticOrigin::Dependency,
                e.to_string(),
            ),
        )
    })?;
    if parsed.len() > MAX_RECORDS {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_records_exceeded(),
        )));
    }
    let mut result = vec![];
    for (index, entry) in parsed.iter().enumerate() {
        let get = |key: &str| entry.get(key).map(|v| v.format_verbatim());
        let bib_kind = entry.entry_type.to_string();
        let csl_kind = match bib_kind.as_str() {
            "article" => "article-journal",
            "book" | "mvbook" => "book",
            "inbook" | "incollection" | "inproceedings" => "chapter",
            "report" | "techreport" => "report",
            "standard" => "standard",
            "dataset" => "dataset",
            "thesis" | "phdthesis" | "mastersthesis" => "thesis",
            "online" | "electronic" | "www" => "webpage",
            _ => bib_kind.as_str(),
        };
        let mut csl = json!({"id":entry.key,"type":csl_kind});
        let known = BTreeSet::from([
            "title",
            "journal",
            "journaltitle",
            "booktitle",
            "volume",
            "number",
            "pages",
            "abstract",
            "url",
            "langid",
            "keywords",
            "doi",
            "isbn",
            "issn",
            "author",
            "editor",
            "year",
            "month",
            "date",
            "weftextpublication",
        ]);
        for (bib, name) in [
            ("title", "title"),
            ("volume", "volume"),
            ("number", "issue"),
            ("pages", "page"),
            ("abstract", "abstract"),
            ("url", "URL"),
            ("langid", "language"),
            ("keywords", "keyword"),
            ("doi", "DOI"),
            ("isbn", "ISBN"),
            ("issn", "ISSN"),
        ] {
            if let Some(value) = get(bib) {
                csl[name] = json!(value);
            }
        }
        if let Some(value) = get("journaltitle")
            .or_else(|| get("journal"))
            .or_else(|| get("booktitle"))
        {
            csl["container-title"] = json!(value);
        }
        for role in ["author", "editor"] {
            if let Some(chunks) = entry.get(role) {
                let names = chunks.parse::<Vec<biblatex::Person>>().map_err(|e| {
                    fail(
                        weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_bibliography_invalid_bibtex_role(weftext_messages::params::ErrorExportBibliographyInvalidBibtexRole {
                                role: (role).to_string(),
                            }),
                        )
                        .diagnostic(
                            weftext_messages::DiagnosticOrigin::Dependency,
                            e.to_string(),
                        ),
                    )
                })?;
                csl[role] = Value::Array(
                    names
                        .into_iter()
                        .map(|name| {
                            if !name.prefix.is_empty() || !name.suffix.is_empty() {
                                loss(
                                    losses,
                                    "nameParts",
                                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_bibtex_name_affixes_preserved()),
                                    format!("records[{index}].{role}"),
                                );
                            }
                            let family = if name.prefix.is_empty() {
                                name.name
                            } else {
                                format!("{} {}", name.prefix, name.name)
                            };
                            if name.given_name.is_empty() {
                                json!({"literal":family})
                            } else {
                                json!({"family":family,"given":name.given_name})
                            }
                        })
                        .collect(),
                );
            }
        }
        let date = get("date").or_else(|| {
            get("year").map(|year| {
                get("month").map_or_else(|| year.clone(), |month| format!("{year}-{month}"))
            })
        });
        if let Some(date) = date {
            if let Some(parts) = parse_date(&date) {
                csl["issued"] = json!({"date-parts":[parts]});
            } else {
                loss(
                    losses,
                    "datePrecision",
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_bibtex_imprecise_date_preserved()),
                    format!("records[{index}].date"),
                );
            }
        }
        if let Some(extension) = get("weftextpublication") {
            csl["weftext"] = decode_extension(&extension)?;
        }
        let mut proposal = map_csl(&csl, index, losses)?;
        proposal.original_record = json!({"key":entry.key,"type":bib_kind,"fields":entry.fields.iter().map(|(key,v)|(key.clone(),v.format_verbatim())).collect::<BTreeMap<_,_>>()});
        proposal.unknown_fields = json!(
            entry
                .fields
                .iter()
                .filter(|(key, _)| !known.contains(key.as_str()))
                .map(|(key, v)| (key.clone(), v.format_verbatim()))
                .collect::<BTreeMap<_, _>>()
        );
        for source in &mut proposal.sources {
            let actual = match source.original_field.as_str() {
                "id" => "entry.key",
                "type" => "entry.type",
                "container-title" => {
                    if entry.get("journaltitle").is_some() {
                        "journaltitle"
                    } else if entry.get("journal").is_some() {
                        "journal"
                    } else {
                        "booktitle"
                    }
                }
                "issue" => "number",
                "page" => "pages",
                "URL" => "url",
                "language" => "langid",
                "keyword" => "keywords",
                "DOI" => "doi",
                "ISBN" => "isbn",
                "ISSN" => "issn",
                "issued.date-parts" => {
                    if entry.get("date").is_some() {
                        "date"
                    } else if entry.get("month").is_some() {
                        "year/month"
                    } else {
                        "year"
                    }
                }
                other => other,
            };
            source.original_field = format!("BibTeX:{actual}");
        }
        if proposal
            .unknown_fields
            .as_object()
            .is_some_and(|v| !v.is_empty())
        {
            loss(
                losses,
                "unknownProviderFields",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_unmapped_bibtex_fields_preserved()),
                format!("records[{index}]"),
            );
        }
        // Parsing resolves TeX/macros; exact upload remains available even where
        // text projection does not preserve presentation or inherited fields.
        loss(
            losses,
            "bibtexTextProjection",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_bibtex_markup_projected_as_text(),
            ),
            format!("records[{index}]"),
        );
        result.push(proposal);
    }
    Ok(result)
}

fn import_ris(source: &str, losses: &mut Vec<ExportLoss>) -> Result<Vec<ImportedPublication>> {
    let mut records = vec![];
    let mut fields: Vec<(String, String)> = vec![];
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let bytes = line.as_bytes();
        let is_tag = bytes.len() >= 6
            && bytes[0].is_ascii_uppercase()
            && bytes[1].is_ascii_alphanumeric()
            && &bytes[2..6] == b"  - ";
        if !is_tag {
            if line.starts_with([' ', '\t']) && !fields.is_empty() {
                let last = fields.last_mut().unwrap();
                last.1.push('\n');
                last.1.push_str(line.trim_start());
                continue;
            }
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_malformed_ris_line(
                    weftext_messages::params::ErrorExportBibliographyMalformedRisLine {
                        line: weftext_messages::UiInteger::new(
                            i64::try_from(line_no + 1).expect("bounded product count"),
                        )
                        .expect("bounded product count"),
                    },
                ),
            )));
        }
        let key = &line[..2];
        let value = &line[6..];
        if key == "TY" {
            if !fields.is_empty() {
                return Err(fail(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_ris_record_missing_end_before_next())));
            }
        } else if fields.is_empty() {
            return Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_ris_record_start_required(),
            )));
        }
        fields.push((key.into(), value.into()));
        if key == "ER" {
            if records.len() >= MAX_RECORDS {
                return Err(fail(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_import_records_exceeded(
                    ),
                )));
            }
            records.push(map_ris(&fields, records.len(), losses)?);
            fields.clear();
        }
    }
    if !fields.is_empty() {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_ris_record_end_required(),
        )));
    }
    Ok(records)
}

fn map_ris(
    fields: &[(String, String)],
    index: usize,
    losses: &mut Vec<ExportLoss>,
) -> Result<ImportedPublication> {
    let single = |tag: &str| -> Result<Option<&str>> {
        let values = fields
            .iter()
            .filter(|(k, _)| k == tag)
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>();
        match values.as_slice() {
            [] => Ok(None),
            [value] => Ok(Some(*value)),
            _ => Err(fail(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_repeated_ris_singleton(
                    weftext_messages::params::ErrorExportBibliographyRepeatedRisSingleton {
                        tag: (tag).to_string(),
                    },
                ),
            ))),
        }
    };
    let kind = match single("TY")?.unwrap_or("") {
        "JOUR" | "JFULL" => "article-journal",
        "BOOK" => "book",
        "CHAP" | "CPAPER" => "chapter",
        "RPRT" => "report",
        "STAND" => "standard",
        "DATA" => "dataset",
        "THES" => "thesis",
        "ELEC" => "webpage",
        other => other,
    };
    let mut csl = json!({"type":kind});
    for (ris, name) in [
        ("ID", "id"),
        ("TI", "title"),
        ("T2", "container-title"),
        ("VL", "volume"),
        ("IS", "issue"),
        ("SP", "page"),
        ("AB", "abstract"),
        ("UR", "URL"),
        ("LA", "language"),
        ("DO", "DOI"),
    ] {
        if let Some(value) = single(ris)? {
            csl[name] = json!(value);
        }
    }
    if let Some(end) = single("EP")?
        && let Some(start) = csl["page"].as_str()
    {
        csl["page"] = json!(format!("{start}–{end}"));
    }
    for (tag, role) in [("AU", "author"), ("ED", "editor")] {
        let authors = fields
            .iter()
            .filter(|(k, _)| k == tag)
            .map(|(_, name)| {
                if let Some((family, given)) = name.split_once(',') {
                    json!({"family":family.trim(),"given":given.trim()})
                } else {
                    json!({"literal":name})
                }
            })
            .collect::<Vec<_>>();
        if !authors.is_empty() {
            csl[role] = Value::Array(authors);
        }
    }
    let keywords = fields
        .iter()
        .filter(|(k, _)| k == "KW")
        .map(|(_, v)| v.clone())
        .collect::<Vec<_>>();
    if !keywords.is_empty() {
        csl["keyword"] = json!(keywords.join(", "));
    }
    if let Some(date) = single("PY")?.or(single("Y1")?) {
        if let Some(parts) = parse_date(date.trim_end_matches('/')) {
            csl["issued"] = json!({"date-parts":[parts]});
        } else {
            loss(
                losses,
                "datePrecision",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_ris_date_preserved_without_day_inference()),
                format!("records[{index}].PY"),
            );
        }
    }
    if let Some(value) = single("SN")? {
        // RIS SN is type-dependent; keep it unknown where its scheme is not proved.
        if kind == "book" {
            csl["ISBN"] = json!(value);
        } else if kind == "article-journal" {
            csl["ISSN"] = json!(value);
        }
    }
    let extensions = fields
        .iter()
        .filter(|(k, _)| k == "C1")
        .filter_map(|(_, v)| v.strip_prefix("weftext-v1:"))
        .collect::<Vec<_>>();
    if extensions.len() > 1 {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_ris_exact_extension_duplicate(),
        )));
    }
    if let Some(extension) = extensions.first() {
        csl["weftext"] = decode_extension(extension)?;
    }
    let mut proposal = map_csl(&csl, index, losses)?;
    if extensions.is_empty()
        && !keywords.is_empty()
        && let NativeValue::Record(members) = &mut proposal.publication
        && let Some(field) = members.iter_mut().find(|field| field.key == "keywords")
    {
        field.value = NativeValue::Array(keywords.iter().map(|keyword| txt(keyword)).collect());
    }
    proposal.original_record = json!(fields);
    let known = BTreeSet::from([
        "TY", "ER", "ID", "TI", "T2", "VL", "IS", "SP", "EP", "AB", "UR", "LA", "DO", "AU", "ED",
        "KW", "PY", "Y1", "SN",
    ]);
    proposal.unknown_fields = json!(
        fields
            .iter()
            .filter(|(key, _)| !known.contains(key.as_str()))
            .collect::<Vec<_>>()
    );
    if proposal
        .unknown_fields
        .as_array()
        .is_some_and(|v| !v.is_empty())
    {
        loss(
            losses,
            "unknownProviderFields",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_unmapped_ris_tags_preserved(
                ),
            ),
            format!("records[{index}]"),
        );
    }
    for source in &mut proposal.sources {
        let actual = match source.original_field.as_str() {
            "id" => "ID",
            "type" => "TY",
            "title" => "TI",
            "container-title" => "T2",
            "volume" => "VL",
            "issue" => "IS",
            "page" => {
                if fields.iter().any(|(key, _)| key == "EP") {
                    "SP/EP"
                } else {
                    "SP"
                }
            }
            "abstract" => "AB",
            "URL" => "UR",
            "language" => "LA",
            "keyword" => "KW",
            "DOI" => "DO",
            "ISBN" | "ISSN" => "SN",
            "issued.date-parts" => {
                if fields.iter().any(|(key, _)| key == "PY") {
                    "PY"
                } else {
                    "Y1"
                }
            }
            "author/editor" => "AU/ED",
            other => other,
        };
        let actual = actual.replace("author[", "AU[").replace("editor[", "ED[");
        source.original_field = format!("RIS:{actual}");
    }
    Ok(proposal)
}

fn exact_extension(
    value: &Value,
    index: usize,
    losses: &mut Vec<ExportLoss>,
) -> Result<NativeValue> {
    if value["version"] != 1 {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_exact_extension_version_unsupported(),
        )));
    }
    let publication: NativeValue = serde_json::from_value(value["publication"].clone())?;
    publication.record().ok_or_else(|| {
        fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_exact_extension_record_required(
            ),
        ))
    })?;
    publication.to_typst().map_err(ExportError::from_data)?;
    loss(
        losses,
        "exactExtension",
        weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_exact_extension_identity_not_reused(),
        ),
        format!("records[{index}].weftext"),
    );
    Ok(publication)
}

fn date_parts(value: &NativeValue) -> Option<Vec<i64>> {
    match value {
        NativeValue::Date(value) => parse_date(value),
        NativeValue::Record(_) => {
            if text(value, "calendar") != Some("gregorian")
                || text(value, "version") != Some("proleptic-gregorian.v1")
                || value.member("leap_month") == Some(&NativeValue::Bool(true))
            {
                return None;
            }
            let precision = text(value, "precision")?;
            let year = value.member("year")?.integer()?;
            let month = value.member("month").and_then(NativeValue::integer);
            let day = value.member("day").and_then(NativeValue::integer);
            let parts = match (precision, month, day) {
                ("year", None, None) => vec![year],
                ("month", Some(month), None) => vec![year, month],
                ("day", Some(month), Some(day)) => vec![year, month, day],
                _ => return None,
            };
            precision_date(&parts).map(|_| parts)
        }
        _ => None,
    }
}

fn parse_date(date: &str) -> Option<Vec<i64>> {
    let parts = date
        .split(['-', '/'])
        .map(str::parse::<i64>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    precision_date(&parts).map(|_| parts)
}

fn precision_date(parts: &[i64]) -> Option<NativeValue> {
    if !(1..=3).contains(&parts.len()) || !(1..=9999).contains(&parts[0]) {
        return None;
    }
    if parts.len() >= 2 && !(1..=12).contains(&parts[1]) {
        return None;
    }
    if parts.len() == 3 {
        let year = parts[0];
        let max = match parts[1] {
            4 | 6 | 9 | 11 => 30,
            2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
            2 => 28,
            _ => 31,
        };
        if !(1..=max).contains(&parts[2]) {
            return None;
        }
    }
    let mut members = vec![
        NativeMember {
            key: "calendar".into(),
            value: txt("gregorian"),
        },
        NativeMember {
            key: "version".into(),
            value: txt("proleptic-gregorian.v1"),
        },
        NativeMember {
            key: "precision".into(),
            value: txt(["year", "month", "day"][parts.len() - 1]),
        },
    ];
    for (key, value) in ["year", "month", "day"].iter().zip(parts) {
        members.push(NativeMember {
            key: (*key).into(),
            value: NativeValue::Int(value.to_string()),
        });
    }
    Some(NativeValue::Record(members))
}

fn record<const N: usize>(items: [(&str, NativeValue); N]) -> NativeValue {
    NativeValue::Record(
        items
            .into_iter()
            .map(|(key, value)| NativeMember {
                key: key.into(),
                value,
            })
            .collect(),
    )
}
fn txt(value: &str) -> NativeValue {
    NativeValue::Text(value.into())
}
fn text<'a>(value: &'a NativeValue, key: &str) -> Option<&'a str> {
    value.member(key).and_then(NativeValue::text)
}
fn citation_key(key: &str) -> bool {
    key.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
fn fail(message: impl Into<weftext_messages::ErrorDescription>) -> ExportError {
    ExportError::new(message)
}
fn loss(
    losses: &mut Vec<ExportLoss>,
    code: &str,
    message: impl Into<weftext_messages::ErrorDescription>,
    location: String,
) {
    losses.push(ExportLoss {
        code: code.into(),
        description: message.into(),
        location: Some(location),

        product_diagnostics: Vec::new(),
    });
}
fn bib_text(value: &str) -> String {
    Chunk::Normal(value.into()).to_biblatex_string(false)
}
fn ris_line(lines: &mut Vec<String>, tag: &str, value: &str) {
    let mut parts = value.split('\n');
    lines.push(format!(
        "{tag}  - {}",
        parts.next().unwrap_or("").replace('\r', " ")
    ));
    for part in parts {
        lines.push(format!("      {}", part.replace('\r', " ")));
    }
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| {
            [
                DIGITS[(b >> 4) as usize] as char,
                DIGITS[(b & 15) as usize] as char,
            ]
        })
        .collect()
}
fn decode_extension(value: &str) -> Result<Value> {
    if value.len() > MAX_BYTES || !value.len().is_multiple_of(2) {
        return Err(fail(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_exact_extension_invalid(),
        )));
    }
    let bytes = value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| {
            let digit = |c: u8| (c as char).to_digit(16).map(|v| v as u8);
            Ok(digit(p[0]).ok_or_else(|| {
                fail(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_extension_hex_invalid(),
                ))
            })? * 16
                + digit(p[1]).ok_or_else(|| {
                    fail(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_bibliography_extension_hex_invalid(),
                ))
                })?)
        })
        .collect::<Result<Vec<u8>>>()?;
    weftext_data::parse_unique_json(&bytes).map_err(ExportError::from_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn work() -> CitationWork {
        CitationWork { key:"boundKey".into(), work:FrozenWork { entity_ref:"weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/8aa10c08-3e7d-4d10-9488-92bc7f393050".into(), source_basis:"frozen-source-1".into(), publication:record([
            ("kind", txt("article")), ("title", txt("原生 & {literal} Title")), ("citation_key", txt("laterSuggestion")),
            ("contributors", NativeValue::Array(vec![record([("agent",record([("text",txt("陈明"))])),("family",txt("陈")),("given",txt("明")),("roles",NativeValue::Array(vec![txt("author")]))]), record([("agent",record([("text",txt("Native Research Group"))])),("roles",NativeValue::Array(vec![txt("author")]))])])),
            ("issued",precision_date(&[2026,9]).unwrap()), ("custom_precise", NativeValue::Decimal("123456789.123456789".into()))
        ]) } }
    }
    #[test]
    fn frozen_key_order_precision_and_unknown_values_survive_all_formats() {
        let work = work();
        for format in [
            BibliographyFormat::CslJson,
            BibliographyFormat::Bibtex,
            BibliographyFormat::Ris,
        ] {
            let output = project_bibliography(std::slice::from_ref(&work), format).unwrap();
            assert_eq!(output.dependencies[0].key, "boundKey");
            let imported = import_bibliography(format, &output.bytes).unwrap();
            assert_eq!(imported.original, output.bytes);
            assert_eq!(imported.records[0].publication, work.work.publication);
            assert!(
                imported.records[0].conflicts.is_empty(),
                "{format:?}: {:?}",
                imported.records[0].conflicts
            );
        }
        let output = project_bibliography(&[work], BibliographyFormat::CslJson).unwrap();
        let rows: Value = serde_json::from_slice(&output.bytes).unwrap();
        assert_eq!(rows[0]["issued"]["date-parts"], json!([[2026, 9]]));
        assert_eq!(rows[0]["author"][0]["family"], "陈");
        assert_eq!(rows[0]["author"][1]["literal"], "Native Research Group");
    }
    #[test]
    fn rich_workflow_same_name_roles_survive_import_reorder_and_all_citation_formats() {
        let original = br#"[{"type":"article-journal","title":"Owned paper","author":[{"literal":"Same Name"}],"editor":[{"literal":"Same Name"}]}]"#;
        let imported = import_bibliography(BibliographyFormat::CslJson, original).unwrap();
        assert_eq!(imported.original, original);
        let mut publication = imported.records[0].publication.clone();
        let contributors = publication.member("contributors").unwrap().array().unwrap();
        assert_eq!(contributors.len(), 2);
        assert_eq!(
            contributors[0].member("roles").unwrap().array().unwrap()[0].text(),
            Some("author")
        );
        assert_eq!(
            contributors[1].member("roles").unwrap().array().unwrap()[0].text(),
            Some("editor")
        );
        let mut reordered = vec![];
        for (index, role) in ["editor", "author"].into_iter().enumerate() {
            let id = if index == 0 {
                "12d7c4f3-9ee6-4c36-920d-ec6af27bc654"
            } else {
                "8aa10c08-3e7d-4d10-9488-92bc7f393050"
            };
            reordered.push(record([
                (
                    "agent",
                    record([(
                        "ref",
                        txt(&format!(
                            "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/{id}"
                        )),
                    )]),
                ),
                ("credited_as", txt("Same Name")),
                ("roles", NativeValue::Array(vec![txt(role)])),
            ]));
        }
        if let NativeValue::Record(members) = &mut publication {
            members
                .iter_mut()
                .find(|member| member.key == "contributors")
                .unwrap()
                .value = NativeValue::Array(reordered);
        }
        let mut work = work();
        work.work.publication = publication.clone();
        for format in [
            BibliographyFormat::CslJson,
            BibliographyFormat::Bibtex,
            BibliographyFormat::Ris,
        ] {
            let artifact = project_bibliography(std::slice::from_ref(&work), format).unwrap();
            assert_eq!(artifact.dependencies[0].key, "boundKey");
            let restored = import_bibliography(format, &artifact.bytes).unwrap();
            assert_eq!(restored.records[0].publication, publication, "{format:?}");
            assert!(restored.records[0].conflicts.is_empty());
        }
        let csl =
            project_bibliography(std::slice::from_ref(&work), BibliographyFormat::CslJson).unwrap();
        let rows: Value = serde_json::from_slice(&csl.bytes).unwrap();
        assert_eq!(rows[0]["author"][0]["literal"], "Same Name");
        assert_eq!(rows[0]["editor"][0]["literal"], "Same Name");
        // An unresolved double target is invalid, never silently mapped to either person.
        if let NativeValue::Record(members) = &mut work.work.publication {
            let values = &mut members
                .iter_mut()
                .find(|member| member.key == "contributors")
                .unwrap()
                .value;
            if let NativeValue::Array(contributors) = values
                && let NativeValue::Record(members) = &mut contributors[0]
            {
                members
                    .iter_mut()
                    .find(|member| member.key == "agent")
                    .unwrap()
                    .value = record([
                    (
                        "ref",
                        txt(
                            "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
                        ),
                    ),
                    ("text", txt("Same Name")),
                ]);
            }
        }
        assert!(project_bibliography(&[work], BibliographyFormat::CslJson).is_err());
    }
    #[test]
    fn collision_incomplete_and_unknown_role_cannot_fake_success() {
        let work = work();
        assert!(
            project_bibliography(&[work.clone(), work.clone()], BibliographyFormat::Bibtex)
                .is_err()
        );
        let mut empty = work;
        empty.work.publication = record([
            ("title", txt("No authored roles")),
            ("kind", txt("article")),
        ]);
        assert!(project_bibliography(&[empty], BibliographyFormat::CslJson).is_err());
        assert!(
            import_bibliography(
                BibliographyFormat::CslJson,
                br#"[{"title":"one","title":"two"}]"#
            )
            .is_err()
        );
        assert!(
            import_bibliography(BibliographyFormat::Ris, b"TY  - JOUR\nTI  - title\n").is_err()
        );
    }
    #[test]
    fn external_import_preserves_unknown_original_and_does_not_invent_day() {
        let source=br#"[{"id":"unsafe key","type":"article-journal","title":"Imported","author":[{"literal":"A"},{"family":"B","given":"C"}],"issued":{"date-parts":[[2026]]},"providerThing":{"x":9007199254740993}}]"#;
        let proposal = import_bibliography(BibliographyFormat::CslJson, source).unwrap();
        assert_eq!(proposal.original, source);
        assert_eq!(
            date_parts(proposal.records[0].publication.member("issued").unwrap()),
            Some(vec![2026])
        );
        assert_eq!(
            proposal.records[0].suggested_key.as_deref(),
            Some("unsafe key")
        );
        assert!(
            proposal.records[0]
                .unknown_fields
                .get("providerThing")
                .is_some()
        );
        let current = record([
            ("title", txt("Author edit")),
            ("unknown", NativeValue::Int("9007199254740993".into())),
        ]);
        let merged = propose_publication_merge(&current, &proposal.records[0].publication).unwrap();
        assert_eq!(text(&merged.publication, "title"), Some("Author edit"));
        assert_eq!(merged.conflicts.len(), 1);
        assert_eq!(
            merged.publication.member("unknown"),
            current.member("unknown")
        );
    }
    #[test]
    fn generated_bib_is_real_native_compile_input_and_heading_collision_fails() {
        let generated = project_bibliography(&[work()], BibliographyFormat::Bibtex).unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
        let mut input=weftext_engine::FrozenRenderRequest {
            source:"#set heading(numbering: \"1.\")\n= Internal heading <refs>\nSee @refs. Cite @boundKey; #cite(<boundKey>).\n#bibliography(\"weftext.citations.bib\")".into(),
            path:root.path().join("note.typ"),root:root.path().into(),
            files:BTreeMap::from([("weftext.citations.bib".into(),generated.bytes)]),style:None,
        };
        let output = engine.render_frozen(&input).unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        assert!(output.html.contains("Research Group"));
        assert!(
            output
                .dependencies
                .iter()
                .any(|d| d.path == "weftext.citations.bib" && d.status == "loaded")
        );
        input.source =
            "= Conflicting heading <boundKey>\n@boundKey\n#bibliography(\"weftext.citations.bib\")"
                .into();
        let output = engine.render_frozen(&input).unwrap();
        assert!(!output.ok);
        assert!(output.diagnostics.iter().any(|d| {
            d.diagnostics.iter().any(|raw| {
                raw.origin == weftext_messages::DiagnosticOrigin::Typst
                    && raw.text.contains("both in the document and a bibliography")
            })
        }));
    }
    #[test]
    fn externally_changed_standard_field_conflicts_with_exact_preserved_extension() {
        let output = project_bibliography(&[work()], BibliographyFormat::CslJson).unwrap();
        let mut rows: Value = serde_json::from_slice(&output.bytes).unwrap();
        rows[0]["title"] = json!("External editor title");
        let imported = import_bibliography(
            BibliographyFormat::CslJson,
            &serde_json::to_vec(&rows).unwrap(),
        )
        .unwrap();
        assert_eq!(imported.records[0].conflicts.len(), 1);
        assert_eq!(imported.records[0].conflicts[0].key, "title");
        assert_eq!(
            text(&imported.records[0].publication, "title"),
            Some("原生 & {literal} Title")
        );
        assert!(
            import_bibliography(
                BibliographyFormat::Bibtex,
                b"@article{a,title={A},title={B}}"
            )
            .is_err()
        );
    }

    #[test]
    fn partial_exact_title_and_kind_changes_are_reported_without_complete_citation() {
        let exact = record([
            ("kind", txt("article")),
            ("title", txt("Preserved exact title")),
        ]);
        for (kind, title, expected) in [
            ("article-journal", "Externally changed title", vec!["title"]),
            ("book", "Preserved exact title", vec!["kind"]),
            ("article-journal", "Preserved exact title", vec![]),
            ("article-magazine", "Preserved exact title", vec![]),
        ] {
            let rows = json!([{"id":"originalKey","type":kind,"title":title,"weftext":{"version":1,"publication":exact}}]);
            let bytes = serde_json::to_vec(&rows).unwrap();
            let imported = import_bibliography(BibliographyFormat::CslJson, &bytes).unwrap();
            assert_eq!(imported.original, bytes);
            assert_eq!(imported.records[0].original_record, rows[0]);
            assert_eq!(imported.records[0].publication, exact);
            assert_eq!(
                imported.records[0]
                    .conflicts
                    .iter()
                    .map(|v| v.key.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            if title != "Preserved exact title" {
                assert_eq!(
                    imported.records[0].conflicts[0].current,
                    txt("Preserved exact title")
                );
                assert_eq!(imported.records[0].conflicts[0].imported, txt(title));
            }
        }
        let mut incomplete = work();
        incomplete.work.publication = exact;
        assert!(project_bibliography(&[incomplete], BibliographyFormat::CslJson).is_err());
    }

    #[test]
    fn partial_exact_complex_groups_use_the_same_forward_projection_without_flattening() {
        let original = work();
        let exact = record([
            (
                "contributors",
                original
                    .work
                    .publication
                    .member("contributors")
                    .unwrap()
                    .clone(),
            ),
            ("issued", precision_date(&[2026, 9]).unwrap()),
            ("keywords", NativeValue::Array(vec![txt("one"), txt("two")])),
            (
                "identifiers",
                NativeValue::Array(vec![
                    record([("scheme", txt("doi")), ("value", txt("original"))]),
                    record([
                        ("scheme", txt("doi")),
                        ("value", txt("another retained identifier")),
                    ]),
                ]),
            ),
        ]);
        let mut rows = json!([{"author":[{"family":"陈","given":"明"},{"literal":"Native Research Group"}],"issued":{"date-parts":[[2026,9]]},"keyword":"one, two","DOI":"original","weftext":{"version":1,"publication":exact}}]);
        let unchanged = import_bibliography(
            BibliographyFormat::CslJson,
            &serde_json::to_vec(&rows).unwrap(),
        )
        .unwrap();
        assert!(unchanged.records[0].conflicts.is_empty());
        assert_eq!(unchanged.records[0].publication, exact);
        rows[0]["DOI"] = json!("external");
        let changed = import_bibliography(
            BibliographyFormat::CslJson,
            &serde_json::to_vec(&rows).unwrap(),
        )
        .unwrap();
        assert_eq!(changed.records[0].conflicts.len(), 1);
        assert_eq!(changed.records[0].conflicts[0].key, "identifiers");
        assert_eq!(changed.records[0].publication, exact);
        rows[0]["author"] = json!([]);
        let removed = import_bibliography(
            BibliographyFormat::CslJson,
            &serde_json::to_vec(&rows).unwrap(),
        )
        .unwrap();
        assert!(
            removed.records[0]
                .conflicts
                .iter()
                .any(|c| c.key == "contributors" && c.imported == NativeValue::Array(vec![]))
        );
    }

    #[test]
    fn partial_exact_unprovable_standard_groups_are_rejected_instead_of_guessed() {
        let unresolved = record([(
            "contributors",
            NativeValue::Array(vec![record([
                ("agent", record([("ref", txt(&work().work.entity_ref))])),
                ("roles", NativeValue::Array(vec![txt("author")])),
            ])]),
        )]);
        let rows = json!([{"author":[{"literal":"Do not guess this Agent name"}],"weftext":{"version":1,"publication":unresolved}}]);
        assert!(
            import_bibliography(
                BibliographyFormat::CslJson,
                &serde_json::to_vec(&rows).unwrap()
            )
            .unwrap_err()
            .description
            .explanation
            .render(weftext_messages::UiLocale::EnUs)
            .contains("cannot be compared accurately")
        );
        let exact = record([
            ("title", txt("Exact title")),
            ("issued", record([("calendar", txt("unknown"))])),
        ]);
        for field in [
            json!({"issued":{"date-parts":[[2026]]}}),
            json!({"author":[{"family":"Known","non-dropping-particle":"Not flattenable"}]}),
            json!({"title":123}),
        ] {
            let mut row = field;
            row["weftext"] = json!({"version":1,"publication":exact});
            let bytes = serde_json::to_vec(&json!([row])).unwrap();
            assert!(
                import_bibliography(BibliographyFormat::CslJson, &bytes)
                    .unwrap_err()
                    .description
                    .explanation
                    .render(weftext_messages::UiLocale::EnUs)
                    .contains("cannot be compared accurately")
            );
            assert_eq!(text(&exact, "title"), Some("Exact title"));
        }
    }
}
