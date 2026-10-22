//! Safe, original-preserving recognition producer. Raw evidence is historical;
//! only the generated native author source is subsequently editable authority.
use crate::exchange::{self, NativeEntry, NativeFileRole as Role, NativeManifest, NativePackage};
use crate::{ExportError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use weftext_data::{NativeMember, NativeValue, Schema, Shape};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewBlock {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub heading_level: Option<u8>,
    #[serde(default)]
    pub table: Option<Vec<Vec<String>>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewField {
    pub key: String,
    pub value: String,
    pub confirmed: bool,
    #[serde(default)]
    pub target_field: Option<String>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub candidate_index: Option<usize>,
    #[serde(default)]
    pub manual_correction: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognitionReview {
    pub purpose: String,
    pub title: String,
    pub blocks: Vec<ReviewBlock>,
    pub fields: Vec<ReviewField>,
    pub accept_partial: bool,
    #[serde(default)]
    pub numbering: Option<String>,
    #[serde(default)]
    pub selected_structure_ids: Vec<String>,
}
fn error(message: impl Into<weftext_messages::ErrorDescription>) -> ExportError {
    ExportError::new(message)
}
fn quoted(value: &str) -> Result<String> {
    weftext_engine::encode_string(value).map_err(ExportError::from_engine)
}
fn insert(
    p: &mut NativePackage,
    path: &str,
    bytes: Vec<u8>,
    role: Role,
    owner: Option<&str>,
    resource: Option<&str>,
) {
    let mut entry = NativeEntry::new(role.clone(), &bytes);
    if role == Role::Source {
        entry.node_id = owner.map(str::to_owned);
    } else {
        entry.owner_node_id = owner.map(str::to_owned);
    }
    entry.resource_id = resource.map(str::to_owned);
    p.manifest.entries.insert(path.into(), entry);
    p.files.insert(path.into(), bytes);
}
pub fn mapping_fields(schema: &Schema) -> Vec<Value> {
    let mut result = vec![];
    for t in &schema.types {
        let (types, valid) = schema.expand_types(std::slice::from_ref(&t.id));
        if !valid.is_valid()
            || schema
                .types
                .iter()
                .any(|kind| types.contains(&kind.id) && !kind.constraints.is_empty())
        {
            continue;
        }
        let fields = schema.applicable_fields(&types);
        if fields.iter().any(|(_, required)| *required) {
            continue;
        }
        for (field, _) in fields {
            if matches!(schema.resolve(&field.shape), Shape::Scalar { semantic } if matches!(semantic.as_str(), "text"|"nonempty_text"|"decimal"|"exact_decimal"|"date"))
            {
                result.push(json!({"id":field.id,"key":field.key,"title":field.key,"shape":schema.resolve(&field.shape),"typeId":t.id}));
            }
        }
    }
    result
}
fn mapped_value(schema: &Schema, mapping: &ReviewField) -> Result<NativeValue> {
    let field = schema
        .fields
        .iter()
        .find(|f| Some(&f.id) == mapping.target_field.as_ref())
        .ok_or_else(|| {
            error(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_recognition_mapped_field_not_installed(),
            ))
        })?;
    let kind = mapping.target_type.as_ref().ok_or_else(|| {
        error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_mapped_type_required(),
        ))
    })?;
    if !mapping_fields(schema)
        .iter()
        .any(|v| v["id"] == field.id && v["typeId"] == *kind)
    {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_scalar_mapping_not_applicable(),
        )));
    }
    let value = match schema.resolve(&field.shape) {
        Shape::Scalar { semantic } => match semantic.as_str() {
            "decimal" => NativeValue::Decimal(mapping.value.clone()),
            "date" => NativeValue::Date(mapping.value.clone()),
            // exact_decimal's native codec is textual, retaining all digits.
            "text" | "nonempty_text" | "exact_decimal" => NativeValue::Text(mapping.value.clone()),
            _ => {
                return Err(error(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_recognition_scalar_codec_unsupported(
                    ),
                )));
            }
        },
        _ => {
            return Err(error(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_recognition_field_shape_unsupported(),
            )));
        }
    };
    if !schema.validate(&field.shape, &value).is_valid() {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_confirmed_value_shape_mismatch(),
        )));
    }
    value.to_typst().map_err(ExportError::from_data)?;
    Ok(value)
}
/// Evidence contains exact worker result, scheme proposal, settings and digests.
/// This trusted producer never executes the recognized text as native markup.
pub fn build_recognition_package(
    format: &str,
    original: &[u8],
    evidence: &Value,
    review: &RecognitionReview,
    schema_json: &str,
    operation: &str,
) -> Result<NativePackage> {
    let ext = match format {
        "pdf" => "pdf",
        "png" => "png",
        "jpeg" => "jpg",
        "docx" => "docx",
        _ => {
            return Err(error(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_recognition_input_unsupported(),
            )));
        }
    };
    if original.is_empty()
        || original.len() > 16 * 1024 * 1024
        || serde_json::to_vec(evidence)?.len() > 32 * 1024 * 1024
        || serde_json::to_vec(review)?.len() > 2 * 1024 * 1024
        || !weftext_data::uuid_v4(operation)
    {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_input_review_or_evidence_exceeded(
            ),
        )));
    }
    if !matches!(review.purpose.as_str(), "resource" | "text" | "note")
        || review.title.is_empty()
        || review.title.len() > 1000
        || review.blocks.len() > 20_000
        || review.fields.len() > 128
    {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_review_invalid(),
        )));
    }
    if evidence["inputSha256"] != exchange::digest(original) {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_original_digest_mismatch(),
        )));
    }
    if evidence["result"]["workerFinalValidation"] == false
        || (review.purpose != "resource"
            && !matches!(
                evidence["result"]["status"].as_str(),
                Some("complete" | "partial")
            ))
    {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_new_job_required_after_failure(),
        )));
    }
    if review.purpose != "resource"
        && evidence["result"]["status"] != "complete"
        && !review.accept_partial
    {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_partial_acceptance_required(),
        )));
    }
    let schema = Schema::from_author_json(schema_json).map_err(ExportError::from_data)?;
    let id = |role: &str| {
        let hash = exchange::digest(format!("{operation}:{role}").as_bytes());
        let mut bytes = u128::from_str_radix(&hash[..32], 16).unwrap().to_be_bytes();
        bytes[6] = (bytes[6] & 15) | 0x40;
        bytes[8] = (bytes[8] & 63) | 0x80;
        uuid::Uuid::from_bytes(bytes).to_string()
    };
    let workspace = id("workspace");
    let node = id("node");
    let original_id = id("original");
    let evidence_id = id("evidence");
    let mut package = NativePackage {
        manifest: NativeManifest {
            format: exchange::NATIVE_FORMAT.into(),
            version: exchange::NATIVE_VERSION,
            package_id: id("package"),
            workspace_id: workspace.clone(),
            scope: "library".into(),
            schema_path: ".weftext-schema.json".into(),
            entries: BTreeMap::new(),
            losses: vec![],
        },
        files: BTreeMap::new(),
    };
    insert(
        &mut package,
        ".weftext-schema.json",
        schema_json.as_bytes().to_vec(),
        Role::Schema,
        None,
        None,
    );
    let mut members = vec![NativeMember {
        key: "id".into(),
        value: NativeValue::Text(node.clone()),
    }];
    let mut types = BTreeSet::new();
    let mut fields = BTreeSet::new();
    let candidates = evidence["scheme"]["fields"].as_object();
    let mut confirmed_keys = BTreeSet::new();
    for field in review.fields.iter().filter(|f| f.confirmed) {
        if field.key.is_empty()
            || field.value.len() > 65536
            || !candidates.is_some_and(|a| a.contains_key(&field.key))
        {
            return Err(error(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_recognition_confirmed_field_candidate_missing(),
            )));
        }
        if evidence["scheme"]["applicability"] != "supported" || !confirmed_keys.insert(&field.key)
        {
            return Err(error(
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_confirmation_scheme_or_keys_invalid()),
            ));
        }
        let candidate = field
            .candidate_index
            .and_then(|index| candidates.unwrap()[&field.key]["candidates"].get(index));
        if !field.manual_correction
            && !candidate.is_some_and(|c| {
                c["parseStatus"] == "parsed"
                    && c["parsed"] == field.value
                    && c["sources"].as_array().is_some_and(|s| !s.is_empty())
            })
        {
            return Err(error(
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_candidate_or_manual_correction_required()),
            ));
        }
        if let Some(target) = &field.target_field {
            if !fields.insert(target.clone()) {
                return Err(error(
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_mapped_field_candidate_conflict()),
                ));
            }
            let value = mapped_value(&schema, field)?;
            let f = schema.fields.iter().find(|f| &f.id == target).unwrap();
            let expected = candidates.unwrap()[&field.key]["valueKind"]
                .as_str()
                .unwrap_or("text");
            let actual = match schema.resolve(&f.shape) {
                Shape::Scalar { semantic }
                    if matches!(semantic.as_str(), "decimal" | "exact_decimal") =>
                {
                    "decimal"
                }
                Shape::Scalar { semantic } if semantic == "date" => "date",
                _ => "text",
            };
            if expected != actual {
                return Err(error(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_recognition_mapped_codec_kind_mismatch(),
                )));
            }
            members.push(NativeMember {
                key: f.key.clone(),
                value,
            });
            types.insert(field.target_type.clone().unwrap());
        }
    }
    members.push(NativeMember {
        key: "types".into(),
        value: NativeValue::Array(types.into_iter().map(NativeValue::Text).collect()),
    });
    let mut source = format!(
        "#metadata({}) <weftext>\n#heading(level: 1, text({}))\n\n",
        NativeValue::Record(members)
            .to_typst()
            .map_err(ExportError::from_data)?,
        quoted(&review.title)?
    );
    let original_name = format!("original.{ext}");
    source.push_str(&format!(
        "#link({}, text(\"保留原件\"))\n\n",
        quoted(&format!("weftext-resource://{workspace}/{original_id}"))?
    ));
    let mut known = BTreeSet::new();
    let mut tables = BTreeSet::new();
    if let Some(pages) = evidence["result"]["pages"].as_array() {
        for p in pages {
            if let Some(regions) = p["regions"].as_array() {
                for r in regions {
                    for b in r["blocks"].as_array().into_iter().flatten() {
                        if let Some(id) = b["id"].as_str() {
                            known.insert(id.to_string());
                        }
                    }
                    for (index, t) in r["tables"].as_array().into_iter().flatten().enumerate() {
                        let id = t["id"].as_str().map(str::to_owned).unwrap_or_else(|| {
                            format!(
                                "table:{}:{}:{index}",
                                p["page"],
                                r["id"].as_str().unwrap_or("")
                            )
                        });
                        known.insert(id.clone());
                        tables.insert(id);
                    }
                }
            }
        }
    }
    for b in evidence["result"]["docxBlocks"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(id) = b["id"].as_str() {
            known.insert(id.to_string());
        }
    }
    let structures = evidence["scheme"]["structures"].as_array();
    let mut selected = BTreeSet::new();
    for id in &review.selected_structure_ids {
        if !selected.insert(id) || !structures.is_some_and(|a| a.iter().any(|s| s["id"] == *id)) {
            return Err(error(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_recognition_structure_duplicate_or_unfrozen(),
            )));
        }
    }
    let numbering = review.numbering.as_deref().unwrap_or("keep");
    if !matches!(numbering, "keep" | "remove" | "rebuild") {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_numbering_choice_unsupported(),
        )));
    }
    if numbering == "rebuild" && !selected.is_empty() {
        source.push_str("#counter(heading).update(0)\n");
    }
    let mut seen = BTreeSet::new();
    let mut native_counters = BTreeMap::<String, u64>::new();
    if review.purpose != "resource" {
        for block in &review.blocks {
            if !known.contains(&block.id)
                || !seen.insert(&block.id)
                || block.text.len() > 2 * 1024 * 1024
            {
                return Err(error(
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_block_duplicate_or_provenance_missing()),
                ));
            }
            let structure = structures.and_then(|a| {
                a.iter().find(|s| {
                    s["id"]
                        .as_str()
                        .is_some_and(|id| selected.contains(&id.to_string()))
                        && s["sources"]
                            .as_array()
                            .is_some_and(|sources| sources.iter().any(|s| s["blockId"] == block.id))
                })
            });
            let mut authored_text = block.text.clone();
            if let Some(s) = structure {
                if matches!(s["kind"].as_str(), Some("title" | "title-candidate")) {
                    source.push_str(&format!(
                        "#text(weight: \"bold\", {})\n\n",
                        quoted(&authored_text)?
                    ));
                    continue;
                }
                if numbering != "keep" && s["nativeNumbering"].is_null() {
                    if s["raw"] != block.text {
                        return Err(error(
                            weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_numbering_offsets_invalid_after_manual_edit()),
                        ));
                    }
                    authored_text = s["content"]
                        .as_str()
                        .ok_or_else(|| error(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_numbering_content_range_missing())))?
                        .to_string();
                }
                if numbering == "rebuild" {
                    let level = s["level"]
                        .as_u64()
                        .filter(|level| (1..=6).contains(level))
                        .ok_or_else(|| {
                            error(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_heading_level_rebuild_unsupported()))
                        })?;
                    source.push_str(&format!(
                        "#heading(level: {level}, numbering: \"1.1.1.1.1.1\", text({}))\n\n",
                        quoted(&authored_text)?
                    ));
                    continue;
                }
                if numbering == "keep" && !s["nativeNumbering"].is_null() {
                    let n = &s["nativeNumbering"];
                    if n["proven"] != true
                        || n["format"] != "decimal"
                        || n["level"].as_u64().unwrap_or(0) > 1
                    {
                        return Err(error(
                            weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_counter_proposal_required()),
                        ));
                    }
                    let key = n["numId"].to_string();
                    let current = native_counters.entry(key).or_insert_with(|| {
                        n["startOverride"]
                            .as_u64()
                            .or_else(|| n["start"].as_u64())
                            .unwrap_or(1)
                    });
                    let pattern = n["pattern"].as_str().unwrap_or("%1.");
                    if pattern.matches("%1").count() != 1 || pattern.contains("%2") {
                        return Err(error(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_counter_pattern_unsupported())));
                    }
                    let pattern = pattern.replace("%1", "1");
                    source.push_str(&format!(
                        "#enum(start: {}, numbering: {}, [#text({})])\n\n",
                        *current,
                        quoted(&pattern)?,
                        quoted(&authored_text)?
                    ));
                    *current += 1;
                    continue;
                }
            }
            match block.kind.as_deref().unwrap_or("paragraph") {
                "paragraph" => source.push_str(&format!("#text({})\n\n", quoted(&authored_text)?)),
                "heading" => source.push_str(&format!(
                    "#heading(level: {}, text({}))\n\n",
                    block
                        .heading_level
                        .filter(|l| (1..=6).contains(l))
                        .ok_or_else(|| error(weftext_messages::UiMessage::error_export_recognition_heading_level_rebuild_unsupported()))?,
                    quoted(&authored_text)?
                )),
                "table" => {
                    if !tables.contains(&block.id) {
                        return Err(error(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_frozen_grid_candidate_required())));
                    }
                    let rows = block
                        .table
                        .as_ref()
                        .ok_or_else(|| error(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_confirmed_table_cells_missing())))?;
                    let cols = rows.first().map_or(0, Vec::len);
                    if cols == 0
                        || cols > 64
                        || rows.len() > 500
                        || rows.iter().any(|r| r.len() != cols)
                    {
                        return Err(error(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_bounded_rectangular_table_required())));
                    }
                    source.push_str(&format!("#table(columns: {cols},\n"));
                    for cell in rows.iter().flatten() {
                        source.push_str(&format!("  text({}),\n", quoted(cell)?));
                    }
                    source.push_str(")\n\n");
                }
                _ => {
                    return Err(error(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_recognition_block_kind_unsupported(),
                    )));
                }
            }
        }
    }
    let confirmed = review
        .fields
        .iter()
        .filter(|f| f.confirmed && f.target_field.is_none())
        .collect::<Vec<_>>();
    if !confirmed.is_empty() {
        source.push_str(
            "#heading(level: 2, text(\"未映射字段（可编辑正文）\"))\n#table(columns: 2,\n",
        );
        for f in &confirmed {
            source.push_str(&format!(
                "  text({}), text({}),\n",
                quoted(&f.key)?,
                quoted(&f.value)?
            ));
        }
        source.push_str(")\n");
    }
    if source.len() > 2 * 1024 * 1024 {
        return Err(error(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_generated_source_exceeded(),
        )));
    }
    let validation = weftext_data::validate_note(&source, &schema, &Default::default());
    if !validation.is_valid() {
        return Err(ExportError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_recognition_author_fields_invalid(),
        ))
        .with_diagnostics(
            validation
                .diagnostics
                .into_iter()
                .chain(validation.unavailable)
                .collect(),
        ));
    }
    let original_ref = format!("weftext-resource://{workspace}/{original_id}");
    let retained = json!({"version":1,"authority":"historicalAcceptedAtImportOnly","inputSha256":exchange::digest(original),"originalResource":original_ref,"schemaSha256":exchange::digest(schema_json.as_bytes()),"evidence":evidence,"review":review});
    insert(
        &mut package,
        "document/document.typ",
        source.into_bytes(),
        Role::Source,
        Some(&node),
        None,
    );
    insert(
        &mut package,
        &format!("document/{original_name}"),
        original.to_vec(),
        Role::Resource,
        Some(&node),
        Some(&original_id),
    );
    insert(
        &mut package,
        "document/recognition-evidence.json",
        serde_json::to_vec_pretty(&retained)?,
        Role::Resource,
        Some(&node),
        Some(&evidence_id),
    );
    insert(
        &mut package,
        "document/weftext.resources.json",
        serde_json::to_vec_pretty(
            &json!({"version":1,"resources":[{"resourceId":original_id,"path":original_name},{"resourceId":evidence_id,"path":"recognition-evidence.json"}]}),
        )?,
        Role::ResourceManifest,
        Some(&node),
        None,
    );
    package.manifest.losses.push(exchange::NativeLoss{path:"recognition".into(),code:"recognition-layout".into(),description:weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_recognition_editable_recognition_with_original_retained()),
product_diagnostics: Vec::new(),
});
    exchange::validate_native_package(&package).map_err(ExportError::from_native)?;
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    const OP: &str = "a095b011-c3b9-4bda-8634-5f40f24b1c1a";
    fn fixture() -> (Vec<u8>, Value, RecognitionReview, String) {
        let original = b"%PDF TEST producer bytes".to_vec();
        let evidence = json!({"inputSha256":exchange::digest(&original),"result":{"status":"complete","pages":[{"page":1,"regions":[{"id":"r1","blocks":[{"id":"p1b1","text":"#read(\"secret\") <injected>"}],"tables":[]}]}]},"scheme":{"applicability":"supported","fields":{"invoiceNumber":{"valueKind":"text","status":"present","value":"00123456789012345678","candidates":[{"parseStatus":"parsed","parsed":"00123456789012345678","sources":[{"blockId":"p1b1","quote":"00123456789012345678"}]}]},"amount":{"valueKind":"decimal","status":"present","candidates":[{"parseStatus":"parsed","parsed":"12345678901234567890.01","sources":[{"blockId":"p1b1"}]}]}}}});
        let review = RecognitionReview {
            purpose: "note".into(),
            title: "TEST #include".into(),
            blocks: vec![ReviewBlock {
                id: "p1b1".into(),
                text: "#read(\"secret\") <injected>".into(),
                kind: None,
                heading_level: None,
                table: None,
            }],
            fields: vec![],
            accept_partial: false,
            numbering: None,
            selected_structure_ids: vec![],
        };
        (
            original,
            evidence,
            review,
            include_str!("../../../docs/design/examples/schema.authoring.json").into(),
        )
    }
    fn field(key: &str, value: &str, target: Option<&str>) -> ReviewField {
        ReviewField {
            key: key.into(),
            value: value.into(),
            confirmed: true,
            target_field: target.map(str::to_owned),
            target_type: target.map(|_| "test/invoice".into()),
            candidate_index: Some(0),
            manual_correction: false,
        }
    }
    #[test]
    fn original_and_historical_evidence_are_registered_and_markup_is_inert() {
        let (o, e, r, s) = fixture();
        let p = build_recognition_package("pdf", &o, &e, &r, &s, OP).unwrap();
        assert_eq!(p.files["document/original.pdf"], o);
        assert_eq!(
            p.manifest
                .entries
                .values()
                .filter(|e| e.role == Role::Resource)
                .count(),
            2
        );
        let source = std::str::from_utf8(&p.files["document/document.typ"]).unwrap();
        assert!(source.contains("#text(\"#read(\\\"secret\\\") <injected>\")"));
        let retained: Value =
            serde_json::from_slice(&p.files["document/recognition-evidence.json"]).unwrap();
        assert_eq!(retained["authority"], "historicalAcceptedAtImportOnly");
        assert_eq!(retained["evidence"], e);
    }
    #[test]
    fn fake_table_stale_block_partial_and_original_substitution_fail() {
        let (o, mut e, mut r, s) = fixture();
        r.blocks[0].kind = Some("table".into());
        r.blocks[0].table = Some(vec![vec!["fake".into()]]);
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        r.blocks[0].kind = None;
        r.blocks[0].id = "missing".into();
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        r.blocks[0].id = "p1b1".into();
        e["result"]["status"] = json!("partial");
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        r.accept_partial = true;
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_ok());
        e["result"]["workerFinalValidation"] = json!(false);
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        e["result"]
            .as_object_mut()
            .unwrap()
            .remove("workerFinalValidation");
        e["result"]["status"] = json!("failed");
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        e["result"]["status"] = json!("partial");
        assert!(build_recognition_package("pdf", b"changed", &e, &r, &s, OP).is_err());
    }
    #[test]
    fn negative_field_needs_explicit_manual_correction_and_duplicate_keys_fail() {
        let (o, mut e, mut r, s) = fixture();
        r.fields
            .push(field("invoiceNumber", "00123456789012345678", None));
        e["scheme"]["fields"]["invoiceNumber"]["candidates"] = json!([]);
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        r.fields[0].manual_correction = true;
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_ok());
        r.fields.push(r.fields[0].clone());
        assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
    }
    #[test]
    fn rebuilding_never_silently_clamps_a_proven_heading_level() {
        let (o, mut e, mut r, s) = fixture();
        e["scheme"]["structures"] = json!([{"id":"structure:p1b1","kind":"heading","raw":r.blocks[0].text,"content":r.blocks[0].text,"level":7,"sources":[{"blockId":"p1b1"}]}]);
        r.numbering = Some("rebuild".into());
        r.selected_structure_ids = vec!["structure:p1b1".into()];
        for level in [0, 7, 8, 9] {
            e["scheme"]["structures"][0]["level"] = json!(level);
            assert!(build_recognition_package("pdf", &o, &e, &r, &s, OP).is_err());
        }
        for level in [1, 3, 6] {
            e["scheme"]["structures"][0]["level"] = json!(level);
            let package = build_recognition_package("pdf", &o, &e, &r, &s, OP).unwrap();
            let source = std::str::from_utf8(&package.files["document/document.typ"]).unwrap();
            assert!(source.contains(&format!("#heading(level: {level}, numbering:")));
        }
    }
    #[test]
    fn typed_mapping_preserves_leading_zero_and_exact_decimal_once() {
        let (o, e, mut r, s) = fixture();
        let mut schema: Value = serde_json::from_str(&s).unwrap();
        for (id, key, semantic) in [
            ("test/number", "invoice_number", "text"),
            ("test/amount", "invoice_amount", "decimal"),
        ] {
            schema["fields"].as_array_mut().unwrap().push(json!({"id":id,"key":key,"shape":{"kind":"scalar","semantic":semantic},"nullable":false,"default":{"kind":"absent"}}));
        }
        schema["types"].as_array_mut().unwrap().push(json!({"id":"test/invoice","fields":[{"id":"test/number","required":false},{"id":"test/amount","required":false}],"requires":[],"conflicts":[],"constraints":[]}));
        r.fields = vec![
            field("invoiceNumber", "00123456789012345678", Some("test/number")),
            field("amount", "12345678901234567890.01", Some("test/amount")),
        ];
        let schema = serde_json::to_string(&schema).unwrap();
        let p = build_recognition_package("pdf", &o, &e, &r, &schema, OP).unwrap();
        let source = std::str::from_utf8(&p.files["document/document.typ"]).unwrap();
        let schema = Schema::from_author_json(&schema).unwrap();
        let projection = weftext_data::project(source, &schema);
        let fields = projection.fields;
        assert_eq!(
            fields
                .iter()
                .find(|f| f.field_id == "test/number")
                .unwrap()
                .cell
                .native(),
            Some(&NativeValue::Text("00123456789012345678".into()))
        );
        assert_eq!(
            fields
                .iter()
                .find(|f| f.field_id == "test/amount")
                .unwrap()
                .cell
                .native(),
            Some(&NativeValue::Decimal("12345678901234567890.01".into()))
        );
        assert_eq!(source.matches("00123456789012345678").count(), 1);
        r.fields[0].target_field = Some("test/amount".into());
        assert!(
            build_recognition_package(
                "pdf",
                &o,
                &e,
                &r,
                &serde_json::to_string(&schema).unwrap(),
                OP
            )
            .is_err()
        );
    }
}
