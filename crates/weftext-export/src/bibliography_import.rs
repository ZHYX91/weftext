//! Pure producer for bibliography import. Core owns the destination schema cut,
//! fresh admitted identities, actual namespace Plan, and explicit commit.
use crate::bibliography::{BibliographyFormat, import_bibliography};
use crate::exchange::{
    MAX_PACKAGE_FILES, NATIVE_FORMAT, NATIVE_VERSION, NativeEntry, NativeFileRole, NativeLoss,
    NativeManifest, NativePackage, digest, validate_native_package,
};
use crate::{ExportError, Result};
use serde_json::json;
use std::collections::BTreeMap;
use weftext_data::{NativeMember, NativeValue, ResourceMetadata, Schema, ValidationContext};

pub fn parse_bibliography_format(format: &str) -> Result<BibliographyFormat> {
    match format {
        "bibtex" | "bib" => Ok(BibliographyFormat::Bibtex),
        "cslJson" | "csl-json" | "csl" => Ok(BibliographyFormat::CslJson),
        "ris" => Ok(BibliographyFormat::Ris),
        _ => Err(ExportError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_format_unsupported(),
        ))),
    }
}

/// This is an internal typed content proposal, not an uploaded/retained native
/// ZIP. It carries the true bibliography upload and exact per-record mappings
/// as registered resources owned by the new paper Works. No existing Work is
/// selected or updated. Provisional IDs only link this one proposal's members;
/// Core replaces them with fresh admitted identities when it freezes the Plan.
pub fn build_bibliography_native_package(
    format: &str,
    bytes: &[u8],
    destination_schema_author_json: &str,
    operation_id: &str,
) -> Result<NativePackage> {
    if !weftext_data::uuid_v4(operation_id) {
        return Err(ExportError::new(
            weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_import_canonical_operation_uuid_required()),
        ));
    }
    let format = parse_bibliography_format(format)?;
    let proposal = import_bibliography(format, bytes)?;
    if proposal.records.is_empty() {
        return Err(ExportError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_importable_records_missing(),
        )));
    }
    if proposal.records.len().saturating_mul(3).saturating_add(2) > MAX_PACKAGE_FILES {
        return Err(ExportError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_atomic_file_count_exceeded(),
        )));
    }
    if proposal
        .records
        .iter()
        .any(|record| !record.conflicts.is_empty())
    {
        return Err(ExportError::new(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_import_exact_publication_fields_conflict())));
    }
    let schema =
        Schema::from_author_json(destination_schema_author_json).map_err(ExportError::from_data)?;
    if !schema
        .types
        .iter()
        .any(|kind| kind.id == "library/paper" && kind.requires.iter().any(|r| r == "library/work"))
    {
        return Err(ExportError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_paper_work_type_unavailable(),
        )));
    }
    let publication_field = schema
        .fields
        .iter()
        .find(|field| field.id == "library/publication")
        .ok_or_else(|| ExportError::new(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_import_publication_field_unavailable())))?;
    let workspace = provisional_id(operation_id, "workspace");
    let mut package = NativePackage {
        manifest: NativeManifest {
            format: NATIVE_FORMAT.into(),
            version: NATIVE_VERSION,
            package_id: provisional_id(operation_id, "package"),
            workspace_id: workspace.clone(),
            scope: "library".into(),
            schema_path: ".weftext-schema.json".into(),
            entries: BTreeMap::new(),
            losses: proposal
                .losses
                .iter()
                .map(|loss| NativeLoss {
                    path: loss
                        .location
                        .clone()
                        .unwrap_or_else(|| "bibliography".into()),
                    code: loss.code.clone(),
                    description: loss.description.clone(),

                    product_diagnostics: loss.product_diagnostics.clone(),
                })
                .collect(),
        },
        files: BTreeMap::new(),
    };
    insert(
        &mut package,
        ".weftext-schema.json".into(),
        destination_schema_author_json.as_bytes().to_vec(),
        NativeFileRole::Schema,
        None,
        None,
    )?;
    for (ordinal, imported) in proposal.records.iter().enumerate() {
        let directory = format!("paper-{:04}", ordinal + 1);
        let node_id = provisional_id(operation_id, &format!("node:{ordinal}"));
        let owner_ref = format!("weftext-node://{workspace}/{node_id}");
        let mut publication = imported.publication.clone();
        if publication
            .member("identifiers")
            .and_then(NativeValue::array)
            .is_some_and(|identifiers| {
                identifiers.iter().any(|identifier| {
                    identifier.member("scheme").and_then(NativeValue::text) == Some("doi")
                        && identifier
                            .member("value")
                            .is_none_or(|value| !weftext_data::scalar_valid("doi", value))
                })
            })
        {
            return Err(ExportError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_import_malformed_doi(
                    weftext_messages::params::ErrorExportBibliographyImportMalformedDoi {
                        record: weftext_messages::UiInteger::new(
                            i64::try_from(ordinal + 1).expect("bounded product count"),
                        )
                        .expect("bounded product count"),
                    },
                ),
            )));
        }
        let validation = schema.validate(&publication_field.shape, &publication);
        if !validation.is_valid() {
            return Err(ExportError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_import_publication_shape_invalid(weftext_messages::params::ErrorExportBibliographyImportPublicationShapeInvalid {
                    record: weftext_messages::UiInteger::new(
                        i64::try_from(ordinal + 1).expect("bounded product count"),
                    )
                    .expect("bounded product count"),
                }),
            ))
            .with_diagnostics(
                validation
                    .diagnostics
                    .into_iter()
                    .chain(validation.unavailable)
                    .collect(),
            ));
        }
        // Unknown schema members are preserved author bytes; only proved typed
        // reference slots require explicit mapping before a new-note import.
        let mut authored = metadata(&node_id, &publication_field.key, &publication)?;
        let closure =
            weftext_data::typed_ref_closure(&authored, &schema).map_err(ExportError::from_data)?;
        if !closure.complete || !closure.slots.is_empty() {
            return Err(ExportError::new(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_import_reference_mapping_unavailable(weftext_messages::params::ErrorExportBibliographyImportReferenceMappingUnavailable { record: weftext_messages::UiInteger::new(i64::try_from(ordinal + 1).expect("bounded product count")).expect("bounded product count") }))));
        }
        let mut resource_metadata = vec![];
        let mut registered = vec![];
        let mut publication_resources = vec![];
        let record_id = provisional_id(operation_id, &format!("record-resource:{ordinal}"));
        let original_record = serde_json::to_vec_pretty(
            &json!({"format":format,"ordinal":ordinal,"record":imported}),
        )?;
        add_resource(
            &mut package,
            &directory,
            &node_id,
            &workspace,
            &record_id,
            "original-record.json",
            original_record,
            &mut resource_metadata,
            &mut registered,
            &mut publication_resources,
            "bibliography-record",
        )?;
        if ordinal == 0 {
            let original_name = match format {
                BibliographyFormat::Bibtex => "original.bib",
                BibliographyFormat::CslJson => "original.csl.json",
                BibliographyFormat::Ris => "original.ris",
            };
            let original_id = provisional_id(operation_id, "upload-resource");
            add_resource(
                &mut package,
                &directory,
                &node_id,
                &workspace,
                &original_id,
                original_name,
                proposal.original.clone(),
                &mut resource_metadata,
                &mut registered,
                &mut publication_resources,
                "bibliography-upload",
            )?;
        }
        let members = match &mut publication {
            NativeValue::Record(members) => members,
            _ => unreachable!(),
        };
        if let Some(resources) = members.iter_mut().find(|member| member.key == "resources") {
            let NativeValue::Array(existing) = &mut resources.value else {
                return Err(ExportError::new(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_import_publication_resources_invalid())));
            };
            existing.extend(publication_resources);
        } else {
            members.push(NativeMember {
                key: "resources".into(),
                value: NativeValue::Array(publication_resources),
            });
        }
        authored = metadata(&node_id, &publication_field.key, &publication)?;
        let validation = weftext_data::validate_note(
            &authored,
            &schema,
            &ValidationContext {
                owner: Some(owner_ref),
                resources: resource_metadata,
                resources_complete: true,
            },
        );
        if !validation.is_valid() {
            return Err(ExportError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_bibliography_import_paper_work_invalid(
                    weftext_messages::params::ErrorExportBibliographyImportPaperWorkInvalid {
                        record: weftext_messages::UiInteger::new(
                            i64::try_from(ordinal + 1).expect("bounded product count"),
                        )
                        .expect("bounded product count"),
                    },
                ),
            ))
            .with_diagnostics(
                validation
                    .diagnostics
                    .into_iter()
                    .chain(validation.unavailable)
                    .collect(),
            ));
        }
        let projection = weftext_data::project(&authored, &schema);
        if projection.qualification != "qualified"
            || !projection
                .resolved_types
                .iter()
                .any(|t| t == "library/work")
            || !projection
                .resolved_types
                .iter()
                .any(|t| t == "library/paper")
        {
            return Err(ExportError::new(
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_bibliography_import_imported_paper_work_unqualified()),
            ));
        }
        // The displayed title derives from the formal publication field once.
        let field_key = weftext_engine::encode_string(&publication_field.key)
            .map_err(ExportError::from_engine)?;
        authored.push_str(&format!("\n#context {{\n  let publication = query(label(\"weftext\")).first().value.at({field_key})\n  if \"title\" in publication {{ title(publication.title) }}\n}}\n"));
        insert(
            &mut package,
            format!("{directory}/{directory}.typ"),
            authored.into_bytes(),
            NativeFileRole::Source,
            Some(&node_id),
            None,
        )?;
        insert(
            &mut package,
            format!("{directory}/weftext.resources.json"),
            serde_json::to_vec_pretty(&json!({"version":1,"resources":registered}))?,
            NativeFileRole::ResourceManifest,
            Some(&node_id),
            None,
        )?;
    }
    validate_native_package(&package).map_err(ExportError::from_native)?;
    Ok(package)
}

fn metadata(node_id: &str, field_key: &str, publication: &NativeValue) -> Result<String> {
    let record = NativeValue::Record(vec![
        NativeMember {
            key: "id".into(),
            value: NativeValue::Text(node_id.into()),
        },
        NativeMember {
            key: "types".into(),
            value: NativeValue::Array(vec![NativeValue::Text("library/paper".into())]),
        },
        NativeMember {
            key: field_key.into(),
            value: publication.clone(),
        },
    ]);
    Ok(format!(
        "#metadata({}) <weftext>\n",
        record.to_typst().map_err(ExportError::from_data)?
    ))
}

fn provisional_id(operation_id: &str, role: &str) -> String {
    let digest = digest(format!("{operation_id}:{role}").as_bytes());
    let mut value = u128::from_str_radix(&digest[..32], 16)
        .unwrap()
        .to_be_bytes();
    value[6] = (value[6] & 15) | 0x40;
    value[8] = (value[8] & 63) | 0x80;
    uuid::Uuid::from_bytes(value).to_string()
}

fn insert(
    package: &mut NativePackage,
    path: String,
    bytes: Vec<u8>,
    role: NativeFileRole,
    owner: Option<&str>,
    resource: Option<&str>,
) -> Result<()> {
    let mut entry = NativeEntry::new(role.clone(), &bytes);
    if role == NativeFileRole::Source {
        entry.node_id = owner.map(str::to_owned);
    } else {
        entry.owner_node_id = owner.map(str::to_owned);
    }
    entry.resource_id = resource.map(str::to_owned);
    if package.files.contains_key(&path) {
        return Err(ExportError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_bibliography_import_resource_path_collision(),
        )));
    }
    package.manifest.entries.insert(path.clone(), entry);
    package.files.insert(path, bytes);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_resource(
    package: &mut NativePackage,
    directory: &str,
    node_id: &str,
    workspace: &str,
    id: &str,
    name: &str,
    bytes: Vec<u8>,
    metadata: &mut Vec<ResourceMetadata>,
    registered: &mut Vec<serde_json::Value>,
    resources: &mut Vec<NativeValue>,
    role: &str,
) -> Result<()> {
    insert(
        package,
        format!("{directory}/{name}"),
        bytes,
        NativeFileRole::Resource,
        Some(node_id),
        Some(id),
    )?;
    let reference = format!("weftext-resource://{workspace}/{id}");
    metadata.push(ResourceMetadata {
        resource_ref: reference.clone(),
        owner: format!("weftext-node://{workspace}/{node_id}"),
        lifecycle: "live".into(),
    });
    registered.push(json!({"resourceId":id,"path":name}));
    resources.push(NativeValue::Record(vec![
        NativeMember {
            key: "resource".into(),
            value: NativeValue::Text(reference),
        },
        NativeMember {
            key: "role".into(),
            value: NativeValue::Text(role.into()),
        },
    ]));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    const OP: &str = "e5fc8f57-7791-4943-867f-e834d46d0e09";
    const SCHEMA: &str = include_str!("../../../docs/design/examples/schema.authoring.json");
    #[test]
    fn actual_paper_sources_owned_originals_and_source_mappings_are_frozen() {
        let upload=br#"[{"type":"article-journal","id":"sameDOI","title":"First","DOI":"10.1234/example","author":[{"literal":"One"},{"literal":"Two"}],"issued":{"date-parts":[[2026,9]]},"provider-data":{"exact":"12345678901234567890"}},{"type":"article-journal","id":"sameDOI","title":"Second","DOI":"10.1234/example","author":[{"literal":"Three"}]}]"#;
        let package = build_bibliography_native_package("cslJson", upload, SCHEMA, OP).unwrap();
        assert_eq!(package.files["paper-0001/original.csl.json"], upload);
        let sources = package
            .manifest
            .entries
            .iter()
            .filter(|(_, entry)| entry.role == NativeFileRole::Source)
            .collect::<Vec<_>>();
        assert_eq!(sources.len(), 2);
        assert_ne!(sources[0].1.node_id, sources[1].1.node_id);
        let schema = Schema::from_author_json(SCHEMA).unwrap();
        let source = std::str::from_utf8(&package.files["paper-0001/paper-0001.typ"]).unwrap();
        let projection = weftext_data::project(source, &schema);
        assert!(projection.resolved_types.contains(&"library/paper".into()));
        let publication = projection
            .fields
            .iter()
            .find(|f| f.field_id == "library/publication")
            .unwrap()
            .cell
            .native()
            .unwrap();
        assert_eq!(
            publication
                .member("issued")
                .unwrap()
                .member("precision")
                .unwrap()
                .text(),
            Some("month")
        );
        let original: serde_json::Value =
            serde_json::from_slice(&package.files["paper-0001/original-record.json"]).unwrap();
        assert!(
            original["record"]["sources"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
        );
        assert!(
            original["record"]["unknownFields"]
                .get("provider-data")
                .is_some()
        );
        let repeat = build_bibliography_native_package("cslJson", upload, SCHEMA, OP).unwrap();
        assert_eq!(package.files, repeat.files);
        let ids = sources
            .iter()
            .map(|(_, e)| e.node_id.clone().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 2);
        let root = tempfile::TempDir::new().unwrap();
        let rendered = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render_frozen(&weftext_engine::FrozenRenderRequest {
                source: source.into(),
                path: root.path().join("paper-0001/paper-0001.typ"),
                root: root.path().into(),
                files: package.files.clone(),
                style: None,
            })
            .unwrap();
        assert!(rendered.ok, "{:?}", rendered.diagnostics);
        assert_eq!(
            rendered.products.as_ref().unwrap().native_title_bodies,
            ["First"]
        );
    }
    #[test]
    fn invalid_schema_and_unresolved_native_identity_cannot_create_fake_work() {
        assert!(
            build_bibliography_native_package(
                "cslJson",
                br#"[{"title":"Bad DOI","DOI":"fake"}]"#,
                SCHEMA,
                OP
            )
            .is_err()
        );
        assert!(build_bibliography_native_package("cslJson", b"[]", SCHEMA, OP).is_err());
        assert!(
            build_bibliography_native_package("cslJson", b"[]", SCHEMA, "bad-operation").is_err()
        );
        assert!(build_bibliography_native_package("unsupported", b"[]", SCHEMA, OP).is_err());
    }
}
