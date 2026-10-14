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
        .ok_or_else(|| fail("Current publication is not a record"))?
        .to_vec();
    let incoming = imported
        .record()
        .ok_or_else(|| fail("Imported publication is not a record"))?;
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
        return Err(fail("Bibliography record budget exceeded"));
    }
    let mut keys = BTreeSet::new();
    let mut losses = vec![];
    let mut rows = vec![];
    for work in works {
        if !citation_key(&work.key) || !keys.insert(work.key.clone()) {
            return Err(fail(
                "Citation key is invalid or repeated; no bibliography generated",
            ));
        }
        if !weftext_data::scalar_valid("node_ref", &NativeValue::Text(work.work.entity_ref.clone()))
            || work.work.source_basis.is_empty()
        {
            return Err(fail(
                "Bibliography requires a stable Work reference and frozen source basis",
            ));
        }
        work.work
            .publication
            .to_typst()
            .map_err(|e| fail(e.to_string()))?;
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
        return Err(fail("Generated bibliography exceeds byte budget"));
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
    let members = p
        .record()
        .ok_or_else(|| fail(format!("{}: publication is not a record", work.key)))?;
    let required = |key| {
        text(p, key).filter(|s| !s.is_empty()).ok_or_else(|| {
            fail(format!(
                "{}: complete citation needs publication.{key}",
                work.key
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
            return Err(fail(format!(
                "{}: unsupported Work kind {kind}; original publication remains available",
                work.key
            )));
        }
    };
    let mut row = Map::new();
    row.insert("id".into(), json!(work.key));
    row.insert("type".into(), json!(csl_kind));
    row.insert("title".into(), json!(title));
    let contributors = p
        .member("contributors")
        .and_then(NativeValue::array)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| fail(format!("{}: ordered contributors unavailable", work.key)))?;
    let mut authors = vec![];
    let mut editors = vec![];
    for (index, contributor) in contributors.iter().enumerate() {
        let agent = contributor.member("agent").ok_or_else(|| {
            fail(format!(
                "{}: contributor {index} agent unavailable",
                work.key
            ))
        })?;
        let agent_ref = agent.member("ref");
        let agent_text = agent.member("text");
        if agent.record().is_none()
            || agent_ref.is_some() == agent_text.is_some()
            || agent_ref.is_some_and(|value| !weftext_data::scalar_valid("node_ref", value))
            || agent_text.is_some_and(|value| !weftext_data::scalar_valid("nonempty_text", value))
        {
            return Err(fail(format!(
                "{}: contributor {index} needs exactly one valid ref/text Agent branch",
                work.key
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
            return Err(fail(format!(
                "{}: contributor {index} has no credited or structured name; referenced Agent is not guessed",
                work.key
            )));
        }
        let roles = contributor
            .member("roles")
            .and_then(NativeValue::array)
            .ok_or_else(|| fail(format!("{}: contributor {index} role is unknown", work.key)))?;
        let mut mapped = false;
        let mut seen_roles = BTreeSet::new();
        for role in roles {
            if let Some(role) = role.text()
                && !seen_roles.insert(role)
            {
                loss(
                    losses,
                    "duplicateContributorRole",
                    "Repeated role retained in the exact extension; standard citation lists the contributor once for this role",
                    format!("{}.contributors[{index}].roles", work.key),
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
                    format!(
                        "Role {role} is retained in the exact extension; standard author/editor arrays do not express it"
                    ),
                    format!("{}.contributors[{index}].roles", work.key),
                ),
                None => return Err(fail(format!("{}: contributor role is not text", work.key))),
            }
        }
        if !mapped {
            loss(
                losses,
                "contributorNotInStandardCitation",
                "Contributor retained only in the exact publication extension",
                format!("{}.contributors[{index}]", work.key),
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
                "Agent references, affiliations and additional contributor members are retained exactly; standard bibliography consumers only receive the rendered name and author/editor role",
                format!("{}.contributors[{index}]", work.key),
            );
        }
    }
    if authors.is_empty() && editors.is_empty() {
        return Err(fail(format!(
            "{}: no author/editor is declared; complete citation unavailable",
            work.key
        )));
    }
    if !authors.is_empty() {
        row.insert("author".into(), Value::Array(authors));
    }
    if !editors.is_empty() {
        row.insert("editor".into(), Value::Array(editors));
    }
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
    if let Some(keywords) = p.member("keywords").and_then(NativeValue::array) {
        let values = keywords
            .iter()
            .map(|v| v.text().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| fail("Publication keyword is not text"))?;
        row.insert("keyword".into(), json!(values.join(", ")));
    }
    if let Some(issued) = p.member("issued") {
        match date_parts(issued) {
            Some(parts) => {
                row.insert("issued".into(), json!({"date-parts":[parts]}));
            }
            None => loss(
                losses,
                "datePrecision",
                "Issued calendar/instant cannot be expressed as a Gregorian CSL date; preserved exactly without inventing a day",
                format!("{}.issued", work.key),
            ),
        }
    }
    let mut schemes = BTreeSet::new();
    if let Some(identifiers) = p.member("identifiers").and_then(NativeValue::array) {
        for (index, identifier) in identifiers.iter().enumerate() {
            let scheme =
                text(identifier, "scheme").ok_or_else(|| fail("Identifier scheme unavailable"))?;
            let value =
                text(identifier, "value").ok_or_else(|| fail("Identifier value unavailable"))?;
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
                    "Identifier scheme or additional value retained only in the exact publication extension",
                    format!("{}.identifiers[{index}]", work.key),
                );
            }
        }
    }
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
                "Publication member retained in the exact extension; standard bibliography consumers may ignore it",
                format!("{}.{}", work.key, member.key),
            );
        }
    }
    // Exact tagged values preserve ordered records, unknown members and numeric
    // spelling. The association is provenance, never an import identity command.
    row.insert("weftext".into(), json!({"version":1,"work":work.work.entity_ref,"sourceBasis":work.work.source_basis,"publication":p}));
    Ok(Value::Object(row))
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
        _ => return Err(fail("Unsupported CSL type")),
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
        _ => return Err(fail("Unsupported CSL type")),
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
        return Err(fail("Bibliography import exceeds byte budget"));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|_| fail("Bibliography import must be UTF-8; original bytes were not changed"))?;
    let mut losses = vec![];
    let records = match format {
        BibliographyFormat::CslJson => import_csl(source, &mut losses)?,
        BibliographyFormat::Bibtex => import_bib(source, &mut losses)?,
        BibliographyFormat::Ris => import_ris(source, &mut losses)?,
    };
    if records.len() > MAX_RECORDS {
        return Err(fail("Bibliography import record budget exceeded"));
    }
    for record in &records {
        record
            .publication
            .to_typst()
            .map_err(|e| fail(e.to_string()))?;
    }
    Ok(ImportBibliographyProposal {
        format,
        original: bytes.to_vec(),
        records,
        losses,
    })
}

fn import_csl(source: &str, losses: &mut Vec<ExportLoss>) -> Result<Vec<ImportedPublication>> {
    let mut decoder = serde_json::Deserializer::from_str(source);
    let value = UniqueJson::deserialize(&mut decoder)
        .map_err(|e| fail(format!("Invalid CSL JSON: {e}")))?
        .0;
    decoder.end()?;
    let rows = value
        .as_array()
        .ok_or_else(|| fail("CSL JSON bibliography must be an array of records"))?;
    if rows.len() > MAX_RECORDS {
        return Err(fail("Bibliography import record budget exceeded"));
    }
    rows.iter()
        .enumerate()
        .map(|(index, row)| map_csl(row, index, losses))
        .collect()
}

fn map_csl(row: &Value, index: usize, losses: &mut Vec<ExportLoss>) -> Result<ImportedPublication> {
    let object = row
        .as_object()
        .ok_or_else(|| fail("CSL item is not a record"))?;
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
                "Non-text CSL field preserved in original; no rounded/stringified replacement proposed",
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
                    return Err(fail("CSL contributor is not a record"));
                };
                let family = name.get("family").and_then(Value::as_str);
                let given = name.get("given").and_then(Value::as_str);
                let literal = name.get("literal").and_then(Value::as_str);
                let credited = literal.map(str::to_owned).or_else(|| {
                    family.map(|f| given.map_or_else(|| f.into(), |g| format!("{g} {f}")))
                });
                let Some(credited) = credited.filter(|v| !v.is_empty()) else {
                    return Err(fail(
                        "CSL contributor has no supported literal/structured name",
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
                        "Additional CSL name parts preserved in original; no forced Person is created",
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
            return Err(fail("CSL author/editor must be an ordered array"));
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
                "Unsupported CSL date remains in original; no day 1 default is introduced",
                format!("records[{index}].issued"),
            );
        }
    } else if object.contains_key("issued") {
        loss(
            losses,
            "datePrecision",
            "CSL literal/range/unknown date retained in original without inventing an exact date",
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
            "Unmapped CSL members retained in the original record",
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
        let work = CitationWork {
            key: suggested_key
                .clone()
                .filter(|k| citation_key(k))
                .unwrap_or_else(|| "imported".into()),
            work: FrozenWork {
                entity_ref: extension["work"].as_str().unwrap_or("").into(),
                source_basis: extension["sourceBasis"].as_str().unwrap_or("").into(),
                publication: publication.clone(),
            },
        };
        if let Ok(expected) = project_work(&work, &mut vec![]) {
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
                ("issued", "issued"),
                ("keyword", "keywords"),
            ] {
                if object.contains_key(csl)
                    && row[csl] != expected[csl]
                    && let Some(imported) = mapped.member(native)
                {
                    conflicts.push(PublicationConflict {
                        key: native.into(),
                        current: publication
                            .member(native)
                            .cloned()
                            .unwrap_or(NativeValue::None),
                        imported: imported.clone(),
                    });
                }
            }
            if ["author", "editor"]
                .iter()
                .any(|role| object.contains_key(*role) && row[*role] != expected[*role])
                && let Some(imported) = mapped.member("contributors")
            {
                conflicts.push(PublicationConflict {
                    key: "contributors".into(),
                    current: publication
                        .member("contributors")
                        .cloned()
                        .unwrap_or(NativeValue::None),
                    imported: imported.clone(),
                });
            }
            if ["DOI", "ISBN", "ISSN"]
                .iter()
                .any(|key| object.contains_key(*key) && row[*key] != expected[*key])
                && let Some(imported) = mapped.member("identifiers")
            {
                conflicts.push(PublicationConflict {
                    key: "identifiers".into(),
                    current: publication
                        .member("identifiers")
                        .cloned()
                        .unwrap_or(NativeValue::None),
                    imported: imported.clone(),
                });
            }
        }
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

fn import_bib(source: &str, losses: &mut Vec<ExportLoss>) -> Result<Vec<ImportedPublication>> {
    let raw = biblatex::RawBibliography::parse(source)
        .map_err(|e| fail(format!("Invalid BibTeX: {e}")))?;
    let mut abbreviations = BTreeSet::new();
    for abbreviation in &raw.abbreviations {
        if !abbreviations.insert(abbreviation.key.v.to_ascii_lowercase()) {
            return Err(fail(
                "Duplicate BibTeX abbreviation; explicit repair required",
            ));
        }
    }
    for entry in &raw.entries {
        let mut fields = BTreeSet::new();
        for field in &entry.v.fields {
            if !fields.insert(field.key.v.to_ascii_lowercase()) {
                return Err(fail("Duplicate BibTeX field; explicit repair required"));
            }
        }
    }
    let parsed = Bibliography::from_raw(raw).map_err(|e| fail(format!("Invalid BibTeX: {e}")))?;
    if parsed.len() > MAX_RECORDS {
        return Err(fail("Bibliography import record budget exceeded"));
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
                let names = chunks
                    .parse::<Vec<biblatex::Person>>()
                    .map_err(|e| fail(format!("Invalid BibTeX {role}: {e}")))?;
                csl[role] = Value::Array(
                    names
                        .into_iter()
                        .map(|name| {
                            if !name.prefix.is_empty() || !name.suffix.is_empty() {
                                loss(
                                    losses,
                                    "nameParts",
                                    "BibTeX name prefix/suffix retained in original record",
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
                    "BibTeX date retained in original; range/uncertainty/month macro is not assigned an exact day",
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
                "Unmapped BibTeX fields retained in original record",
                format!("records[{index}]"),
            );
        }
        // Parsing resolves TeX/macros; exact upload remains available even where
        // text projection does not preserve presentation or inherited fields.
        loss(
            losses,
            "bibtexTextProjection",
            "BibTeX TeX markup, macros and inheritance project to text; complete source bytes are retained",
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
            return Err(fail(format!("Malformed RIS line {}", line_no + 1)));
        }
        let key = &line[..2];
        let value = &line[6..];
        if key == "TY" {
            if !fields.is_empty() {
                return Err(fail("RIS record missing ER before next TY"));
            }
        } else if fields.is_empty() {
            return Err(fail("RIS record must start with TY"));
        }
        fields.push((key.into(), value.into()));
        if key == "ER" {
            if records.len() >= MAX_RECORDS {
                return Err(fail("Bibliography import record budget exceeded"));
            }
            records.push(map_ris(&fields, records.len(), losses)?);
            fields.clear();
        }
    }
    if !fields.is_empty() {
        return Err(fail("RIS record missing closing ER"));
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
            _ => Err(fail(format!(
                "RIS repeated singleton field {tag}; explicit repair required"
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
                "RIS date retained in original without inferred day",
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
        return Err(fail("Repeated exact RIS publication extension"));
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
            "Unmapped RIS tags retained in original record",
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
        return Err(fail("Unsupported exact publication extension version"));
    }
    let publication: NativeValue = serde_json::from_value(value["publication"].clone())?;
    publication
        .record()
        .ok_or_else(|| fail("Exact publication extension must contain a native record"))?;
    publication.to_typst().map_err(|e| fail(e.to_string()))?;
    loss(
        losses,
        "exactExtension",
        "Exact publication extension is proposed as author data; original Work reference/basis is provenance and is not reused as import identity",
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
fn fail(message: impl Into<String>) -> ExportError {
    ExportError(message.into())
}
fn loss(losses: &mut Vec<ExportLoss>, code: &str, message: impl Into<String>, location: String) {
    losses.push(ExportLoss {
        code: code.into(),
        message: message.into(),
        location: Some(location),
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
        return Err(fail("Invalid exact publication extension"));
    }
    let bytes = value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| {
            let digit = |c: u8| (c as char).to_digit(16).map(|v| v as u8);
            Ok(
                digit(p[0]).ok_or_else(|| fail("Invalid extension hex"))? * 16
                    + digit(p[1]).ok_or_else(|| fail("Invalid extension hex"))?,
            )
        })
        .collect::<Result<Vec<u8>>>()?;
    let mut decoder = serde_json::Deserializer::from_slice(&bytes);
    let value = UniqueJson::deserialize(&mut decoder)?.0;
    decoder.end()?;
    Ok(value)
}

/// Duplicate JSON members are invalid rather than last-write-wins imports.
struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON value with unique members")
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut access: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut rows = vec![];
                while let Some(value) = access.next_element::<UniqueJson>()? {
                    rows.push(value.0);
                }
                Ok(UniqueJson(Value::Array(rows)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!(
                            "Duplicate JSON member {key}"
                        )));
                    }
                    values.insert(key, access.next_value::<UniqueJson>()?.0);
                }
                Ok(UniqueJson(Value::Object(values)))
            }
        }
        de.deserialize_any(Visitor)
    }
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
        }
        let output = project_bibliography(&[work], BibliographyFormat::CslJson).unwrap();
        let rows: Value = serde_json::from_slice(&output.bytes).unwrap();
        assert_eq!(rows[0]["issued"]["date-parts"], json!([[2026, 9]]));
        assert_eq!(rows[0]["author"][0]["family"], "陈");
        assert_eq!(rows[0]["author"][1]["literal"], "Native Research Group");
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
            d.message
                .contains("both in the document and a bibliography")
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
}
