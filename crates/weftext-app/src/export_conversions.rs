//! Pure converter adapters receive exact Core cuts and cached Query results.
//! The job lifecycle owns confirmation/delivery; these adapters never save an
//! author document, reread a Query, or fetch an external dependency.
use super::export_api::{Binding, PrepareRequest};
use super::*;
use base64::Engine as _;
use std::{collections::BTreeMap, path::Path};
use weftext_data::{
    Cell, NativeValue, NoteProjection, QueryResult, QueryRow, QueryValueType, Schema,
};
use weftext_engine::{FrozenRenderRequest, RenderNode, RenderOutput, StyleModule};
use weftext_export::bibliography::{
    BibliographyFormat, CitationWork, FrozenWork, project_bibliography,
};
use weftext_export::docx::{DocxBinding, DocxInput, DocxOptions};
use weftext_export::xlsx::{
    XlsxFieldSlot, XlsxQuerySlot, XlsxRequest, XlsxSheetInput, XlsxTemplate,
};
use weftext_export::{ExportArtifact, ExportLoss};

fn unavailable(description: impl Into<ErrorDescription>) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "conversionUnavailable",
        description,
    )
}
fn convert_error(error: weftext_export::ExportError) -> ApiError {
    let (description, diagnostics) = error.into_parts();
    ApiError::from_product(
        StatusCode::UNPROCESSABLE_ENTITY,
        "conversionUnavailable",
        None,
        description,
        diagnostics,
    )
}
fn native_error(error: weftext_export::exchange::NativeError) -> CoreError {
    let code = error.code.clone();
    let (description, diagnostics) = error.into_parts();
    CoreError::from_product(&code, Some(code.clone()), description, diagnostics)
}
fn knowledge_error(error: weftext_knowledge::KnowledgeError) -> CoreError {
    let code = error.code.clone();
    let (description, diagnostics) = error.into_parts();
    CoreError::from_product(&code, Some(code.clone()), description, diagnostics)
}
pub(crate) fn report(code: &str, description: impl Into<ErrorDescription>) -> ExportLoss {
    ExportLoss {
        code: code.into(),
        description: description.into(),
        product_diagnostics: vec![],
        location: None,
    }
}
fn decode(value: &str) -> Result<Vec<u8>, ApiError> {
    if value.len() > 64 * 1024 * 1024 * 4 / 3 + 4 {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_template_budget(),
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| ApiError::bad(UiMessage::error_api_export_conversion_template_encoding()))?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_template_budget(),
        ));
    }
    Ok(bytes)
}

pub(crate) fn template_slots(format: &str, bytes: &[u8]) -> Result<Value, ApiError> {
    let slots=match format {
        "docx"=>weftext_export::docx::list_template_controls(bytes).map_err(convert_error)?.into_iter().map(|c|{
            let supported=c.placement!="unsupported";
            json!({"name":c.tag,"kind":"content","label":c.alias.unwrap_or_else(||c.tag.clone()),"supported":supported,
                "placement":c.placement,"description":null,"reason":if supported{None}else{Some(weftext_export::ExportError::new(UiMessage::error_api_export_conversion_unsupported_content_control()))}})
        }).collect::<Vec<_>>(),
        "xlsx"=>weftext_export::xlsx::inspect_xlsx_template(bytes).map_err(convert_error)?.anchors.into_iter().map(|a|{
            let kind=if a.kind=="table"{"table"}else if a.rows==1&&a.columns==1{"cell"}else{"range"};
            let description=a.sheet.as_ref().map(|sheet| UiMessage::error_api_export_conversion_xlsx_anchor_description(weftext_messages::params::ErrorApiExportConversionXlsxAnchorDescription {
                name:a.name.clone(),sheet:sheet.clone(),columns:weftext_messages::UiInteger::new(i64::try_from(a.columns).expect("template column count fits i64")).expect("template column budget fits safe integer"),capacity:weftext_messages::UiInteger::new(i64::try_from(a.capacity).expect("template row count fits i64")).expect("template row budget fits safe integer")}));
            let label=a.name.clone();
            json!({"name":a.name,"kind":kind,"label":label,"description":description,"supported":a.supported,"reason":a.reason,"rows":a.rows,"columns":a.columns,"capacity":a.capacity})
        }).collect::<Vec<_>>(),
        _=>return Err(ApiError::bad(UiMessage::error_api_export_conversion_office_template_unsupported())),
    };
    Ok(json!({"slots":slots,"warnings":[]}))
}

struct DocumentCut {
    reviews: Vec<weftext_export::docx::DocxReview>,
    files: BTreeMap<String, Vec<u8>>,
    style: Option<StyleModule>,
    schema: Schema,
    schema_json: Value,
    cut_id: String,
    workspace_id: Option<String>,
    owner_id: Option<String>,
    citations: Option<super::citation_api::FrozenCitations>,
    warnings: Vec<weftext_core::CoreDescription>,
    complete: bool,
}
fn relative(root: &Path, path: &Path) -> Result<String, CoreError> {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_api_export_conversion_source_outside_frozen_root(),
            )
        })
}
fn capture(
    core: &mut Core,
    loc: &Location,
    req: &PrepareRequest,
) -> Result<DocumentCut, CoreError> {
    if !matches!(
        req.review_mode.as_str(),
        "" | "none" | "comments" | "changes"
    ) {
        return Err(CoreError::new(
            "reviewMode",
            UiMessage::error_api_export_conversion_review_mode_required(),
        ));
    }
    core.check_session_base(&req.session_id, &req.base_id)?;
    let inventory = core.inventory(&loc.root)?;
    let key = relative(&loc.root, &loc.path)?;
    if inventory.workspace_id.is_some() {
        let mut frozen = core.admitted_compile_snapshot(&loc.root, "live")?;
        let reviews = freeze_reviews(&frozen, loc, req)?;
        let owner_id = frozen
            .snapshot
            .files
            .iter()
            .find(|f| Path::new(&f.path) == loc.path)
            .and_then(|f| f.node_id.clone());
        frozen.files.insert(key, req.source.as_bytes().to_vec());
        if let Some(file) = frozen
            .snapshot
            .files
            .iter_mut()
            .find(|f| Path::new(&f.path) == loc.path)
        {
            file.bytes = Some(req.source.as_bytes().to_vec());
        }
        let schema_file = frozen.snapshot.schema.as_ref().ok_or_else(|| {
            CoreError::new(
                "schemaUnavailable",
                UiMessage::error_api_export_conversion_frozen_schema_unavailable(),
            )
        })?;
        let (schema, _) =
            super::data_api::knowledge_schema_from_snapshot(schema_file).map_err(|mut error| {
                error.code = "schemaUnavailable".into();
                error
            })?;
        let schema_json = serde_json::to_value(&schema)?;
        let citations = super::citation_api::freeze_citations(&mut frozen, &req.source, &loc.path)?;
        core.check_session_base(&req.session_id, &req.base_id)?;
        Ok(DocumentCut {
            reviews,
            files: frozen.files,
            style: frozen.style,
            schema,
            schema_json,
            cut_id: frozen.snapshot.cut_id,
            workspace_id: frozen.snapshot.inventory.workspace_id,
            owner_id,
            citations,
            warnings: frozen.snapshot.warnings,
            complete: frozen.snapshot.complete,
        })
    } else {
        if matches!(req.review_mode.as_str(), "comments" | "changes") && req.format == "docx" {
            return Err(CoreError::new(
                "reviewUnavailable",
                UiMessage::error_api_export_conversion_ordinary_file_has_no_review(),
            ));
        }
        if inventory.status != "ordinary" {
            return Err(CoreError::new(
                "unadmittedLibrary",
                UiMessage::error_api_export_conversion_library_cannot_fall_back_to_ordinary(),
            ));
        }
        let mut files = core.ordinary_export_files(&req.session_id, &req.base_id, &loc.root)?;
        if files.keys().any(|p| {
            matches!(
                p.rsplit('/').next().unwrap_or(p),
                ".weftext-workspace.json" | ".weftext-format" | ".weftext-mvp-commit.json"
            )
        }) {
            return Err(CoreError::new(
                "unadmittedLibrary",
                UiMessage::error_api_export_conversion_library_admission_required(),
            ));
        }
        if weftext_engine::inspect_citation_declaration(&req.source)
            .map_err(|e| CoreError::from_engine("citationUnavailable", e))?
            .is_some()
        {
            return Err(CoreError::new(
                "citationUnavailable",
                UiMessage::error_api_export_conversion_citations_need_managed_cut(),
            ));
        }
        files.insert(key, req.source.as_bytes().to_vec());
        let schema =
            Schema::bundled().map_err(|error| CoreError::from_data("schemaUnavailable", error))?;
        let schema_json = serde_json::to_value(&schema)?;
        core.check_session_base(&req.session_id, &req.base_id)?;
        Ok(DocumentCut {
            reviews: vec![],
            files,
            style: None,
            schema,
            schema_json,
            cut_id: format!(
                "ordinary:{}:{}",
                req.base_id,
                weftext_engine::source_hash(&req.source)
            ),
            workspace_id: None,
            owner_id: None,
            citations: None,
            warnings: vec![],
            complete: true,
        })
    }
}
fn freeze_reviews(
    frozen: &weftext_core::AdmittedCompileSnapshot,
    loc: &Location,
    req: &PrepareRequest,
) -> Result<Vec<weftext_export::docx::DocxReview>, CoreError> {
    use weftext_knowledge as k;
    if req.format != "docx" || !matches!(req.review_mode.as_str(), "comments" | "changes") {
        return Ok(vec![]);
    }
    let source = frozen
        .snapshot
        .files
        .iter()
        .find(|f| Path::new(&f.path) == loc.path)
        .ok_or_else(|| {
            CoreError::new(
                "reviewSource",
                UiMessage::error_api_export_conversion_review_source_missing(),
            )
        })?;
    if source.bytes.as_deref() != Some(req.source.as_bytes()) {
        return Err(CoreError::new(
            "reviewDraft",
            UiMessage::error_api_export_conversion_saved_review_source_required(),
        ));
    }
    let key = relative(
        &loc.root,
        &loc.path
            .parent()
            .ok_or_else(|| {
                CoreError::new(
                    "reviewSource",
                    UiMessage::error_api_export_conversion_note_directory_missing(),
                )
            })?
            .join("weftext.annotations.json"),
    )?;
    let Some(bytes) = frozen.files.get(&key) else {
        return Ok(vec![]);
    };
    let annotations = k::decode_annotations(bytes).map_err(knowledge_error)?;
    let owner = k::NodeRef {
        workspace_id: frozen
            .snapshot
            .inventory
            .workspace_id
            .clone()
            .unwrap_or_default(),
        node_id: source.node_id.clone().unwrap_or_default(),
    };
    let current = k::NodeSnapshot {
        owner: owner.clone(),
        path: source.path.clone(),
        library_path: relative(&loc.root, &loc.path)?,
        revision_id: source.revision.clone().unwrap_or_default(),
        source: req.source.clone(),
        title: None,
        aliases: vec![],
        labels: vec![],
        labels_complete: false,
    };

    let mut selected: std::collections::BTreeSet<String> = annotations
        .records
        .iter()
        .filter(|r| r.reply_to.is_none() && (req.include_resolved || r.review_state == "open"))
        .map(|r| r.annotation_id.clone())
        .collect();
    loop {
        let n = selected.len();
        for r in &annotations.records {
            if r.reply_to.as_ref().is_some_and(|p| selected.contains(p)) {
                selected.insert(r.annotation_id.clone());
            }
        }
        if n == selected.len() {
            break;
        }
    }
    Ok(annotations.records.iter().filter(|r|selected.contains(&r.annotation_id)).map(|r| {
        let range=r.target.anchor().and_then(|a|k::resolve_anchor(a,Some(&current),&[]).current_range);
        let replacement=if req.review_mode=="changes" {r.suggestion.as_ref().filter(|s|s.state=="pending" && !s.requires_reconfirmation && s.confirmed_revision==current.revision_id && range.as_ref().is_some_and(|range|k::writable_suggestion_range(&req.source,range))).map(|s|s.replacement.clone())}else{None};
        let mut body=r.body.clone(); if let Some(s)=&r.suggestion {body.push('\n'); body.push_str(&UiMessage::error_api_export_conversion_review_suggestion_appendix(weftext_messages::params::ErrorApiExportConversionReviewSuggestionAppendix { state:s.state.clone(),replacement:s.replacement.clone() }).render(req.output_locale));if req.review_mode=="changes" && s.state=="pending" && replacement.is_none(){body.push('\n');body.push_str(&UiMessage::error_api_export_conversion_review_suggestion_comment_only().render(req.output_locale));}}
        let empty_body_point = range.as_ref().is_some_and(|r|r.from==r.to && r.from==req.source.len() && k::writable_suggestion_range(&req.source,r));
        let downgrade_reason = (req.review_mode=="changes" && r.suggestion.as_ref().is_some_and(|s|s.state=="pending") && replacement.is_none()).then(||ErrorDescription::new(UiMessage::error_api_export_conversion_review_suggestion_downgrade()));
        weftext_export::docx::DocxReview{id:r.annotation_id.clone(),parent:r.reply_to.clone(),author:r.attribution.clone(),date:weftext_export::docx::word_comment_date(&r.created_at),body,resolved:r.review_state=="resolved",range:range.map(|r|(r.from,r.to)),whole_document:matches!(&r.target,k::AnnotationTarget::WholeDocument{node}if node==&owner),replacement,downgrade_reason,empty_body_point}
    }).collect())
}

pub(crate) async fn generate(
    state: &AppState,
    req: &PrepareRequest,
) -> Result<(ExportArtifact, Value), ApiError> {
    if matches!(req.format.as_str(), "doc" | "wps") {
        super::export_api::claim_legacy_request(state, &req.request_id, req)?;
        let mut intermediate = req.clone();
        intermediate.format = "docx".into();
        intermediate.legacy_backend = None;
        let frozen = super::export_api::frozen_legacy_output_stage(state, &req.request_id)?;
        let restored = frozen.is_some();
        let (mut artifact, mut preview) = if let Some(frozen) = frozen {
            frozen
        } else {
            let frozen = Box::pin(generate(state, &intermediate)).await?;
            super::export_api::retain_legacy_output_stage(
                state,
                &req.request_id,
                &frozen.0,
                &frozen.1,
            )?;
            frozen
        };
        let direction = if req.format == "doc" {
            super::legacy_office::Direction::DocxToDoc
        } else {
            super::legacy_office::Direction::DocxToWps
        };
        let converted = super::export_api::legacy_convert(
            state,
            &req.request_id,
            super::legacy_office::ConversionRequest {
                backend: req
                    .legacy_backend
                    .clone()
                    .unwrap_or_else(|| "libreoffice".into()),
                direction,
                bytes: artifact.bytes.clone(),
            },
        )
        .await?;
        let generated = converted.generated.as_ref().ok_or_else(|| {
            unavailable(UiMessage::error_api_export_conversion_external_artifact_missing())
        })?;
        let has_session = state
            .locations
            .lock()
            .map_err(|_| {
                ApiError::internal(
                    UiMessage::error_api_writing_editor_session_temporarily_unavailable(),
                )
            })?
            .contains_key(&req.session_id);
        if !restored || has_session {
            super::writing_api::check_snapshot(
                state,
                &req.session_id,
                &req.source,
                req.serial,
                None,
            )?;
            let sid = req.session_id.clone();
            let base = req.base_id.clone();
            core_call(state, move |core| core.check_session_base(&sid, &base)).await?;
        } else {
            preview["document"]["scope"] = json!("recoveredFrozenOutput");
            preview["recoveryNotice"] =
                json!(UiMessage::error_api_export_conversion_original_frozen_stage_recovered());
        }
        artifact.bytes = generated.bytes.clone();
        artifact.extension = req.format.clone();
        artifact.content_type = if req.format == "doc" {
            "application/msword"
        } else {
            "application/octet-stream"
        }
        .into();
        artifact
            .losses
            .extend(converted.losses().into_iter().map(|loss| ExportLoss {
                code: loss.code,
                description: loss.description,
                product_diagnostics: loss.product_diagnostics,
                location: loss.location,
            }));
        // Keep the original typed stage summary. The separate conversion
        // metadata describes its actual backend and unverified losses.
        preview["format"] = json!(req.format);
        preview["legacyConversion"] = converted.metadata();
        preview["executionLocation"] = json!("currentBackend");
        return Ok((artifact, preview));
    }
    if req.legacy_backend.is_some() {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_external_backend_unused(),
        ));
    }
    let loc = location(state, &req.session_id)?;
    super::writing_api::observe_snapshot(state, &req.session_id, &req.source, req.serial, None)?;
    let sid = req.session_id.clone();
    let base = req.base_id.clone();
    core_call(state, move |core| core.check_session_base(&sid, &base)).await?;
    let mut preview = json!({"format":req.format,"document":{"sessionId":req.session_id,"baseId":req.base_id,"serial":req.serial,"sourceHash":weftext_engine::source_hash(&req.source),"scope":"currentDraft"},"stylePolicy":req.style_policy});
    if !req.embedded_queries.is_empty() && !matches!(req.format.as_str(), "docx" | "pdf") {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_embedded_query_format(),
        ));
    }
    if !matches!(req.format.as_str(), "docx" | "xlsx")
        && (req.template.is_some() || !req.bindings.is_empty())
    {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_template_or_bindings_unsupported(),
        ));
    }
    if req.template.is_none() && !req.bindings.is_empty() {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_bindings_require_template(),
        ));
    }
    let (mut artifact, more) = match req.format.as_str() {
        "source" => (
            ExportArtifact {
                bytes: req.source.as_bytes().to_vec(),
                content_type: "text/plain; charset=utf-8".into(),
                extension: "typ".into(),
                losses: vec![],
                summary: UiMessage::error_api_export_conversion_native_source_summary(),
            },
            json!({"source":req.source,"sourceBytes":req.source.len()}),
        ),
        "native" => native(state, req, &loc).await?,
        "pdf" => pdf(state, req, &loc).await?,
        "docx" | "xlsx" | "bibtex" | "cslJson" | "csl-json" | "ris" => {
            let embedded = super::embedded_output::freeze(state, req, &loc.root.to_string_lossy())?;
            preview["embeddedQueries"] = super::embedded_output::preview(&embedded)?;
            let cut_req = req.clone();
            let cut_loc = loc.clone();
            let mut cut = core_call(state, move |core| capture(core, &cut_loc, &cut_req)).await?;
            preview["document"]["cutId"] = json!(cut.cut_id);
            preview["document"]["complete"] = json!(cut.complete);
            preview["document"]["warnings"] = json!(cut.warnings);
            let query = if let Some(id) = &req.query_id {
                if matches!(req.format.as_str(), "docx" | "xlsx") {
                    Some(super::export_api::frozen_query(
                        state,
                        id,
                        &loc.root.to_string_lossy(),
                    )?)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(q) = &query {
                preview["query"] = json!({"queryId":req.query_id,"scope":"previouslyDisplayedFrozenResult","cutId":q.value["cutId"],"schemaRevision":q.value["schemaRevision"],"rowCount":q.value["rows"].as_array().map(Vec::len),"columns":q.columns,"coverage":q.value["coverage"],"freshness":q.value["freshness"],"limited":q.value["limited"],"aggregateExact":q.value["aggregateExact"],"inputVersions":q.value["inputVersions"]});
            }
            let needs_render =
                req.format == "docx" || req.bindings.values().any(|b| b.kind == "title");
            let render = if needs_render {
                let output = render_frozen_worker(
                    state,
                    FrozenRenderRequest {
                        source: req.source.clone(),
                        path: loc.path.clone(),
                        root: loc.root.clone(),
                        files: std::mem::take(&mut cut.files),
                        style: cut.style.clone(),
                    },
                )
                .await?;
                Some(output)
            } else {
                None
            };
            let req_owned = req.clone();
            crate::work::spawn_blocking(move || {
                let projection = weftext_data::project(&req_owned.source, &cut.schema);
                let template = req_owned
                    .template
                    .as_ref()
                    .map(|t| decode(&t.bytes_base64))
                    .transpose()?;
                let render = render
                    .map(|output| {
                        serde_json::from_value::<RenderOutput>(output)
                            .map_err(|_| ApiError::internal(UiMessage::error_api_export_conversion_invalid_frozen_html_contract()))
                    })
                    .transpose()?;
                let frozen_result = query
                    .as_ref()
                    .map(|q| {
                        serde_json::from_value::<QueryResult>(q.value.clone())
                            .map_err(|_| ApiError::internal(UiMessage::error_api_export_conversion_frozen_query_contract_unreadable()))
                    })
                    .transpose()?;
                match req_owned.format.as_str() {
                    "docx" => docx(
                        &req_owned,
                        render.unwrap(),
                        &projection,
                        template,
                        frozen_result.as_ref(),
                        query.as_ref().map(|q| q.columns.as_slice()),
                        DocxExtras {
                            embedded: &embedded,
                            reviews: &cut.reviews,
                        },
                    ),
                    "xlsx" => xlsx(
                        &req_owned,
                        render.as_ref(),
                        &projection,
                        template,
                        frozen_result,
                        query.as_ref().map(|q| (&q.columns, &q.schema)),
                        &cut,
                    ),
                    _ => bibliography(&req_owned, &projection, &cut),
                }
            })
            .await
            .map_err(|_| ApiError::internal(UiMessage::error_api_export_conversion_conversion_incomplete()))??
        }
        _ => {
            return Err(ApiError::bad(
                UiMessage::error_api_export_conversion_unknown_format(),
            ));
        }
    };
    if !matches!(req.format.as_str(), "docx" | "xlsx") && req.query_id.is_some() {
        preview["query"] = json!({"queryId":req.query_id,"included":false,"scope":"outsideSelectedFormat","reason":UiMessage::error_api_export_conversion_query_outside_format_scope()});
        artifact.losses.push(report(
            "query_not_in_format_scope",
            UiMessage::error_api_export_conversion_query_outside_format(),
        ));
    }
    if let Some(object) = more.as_object() {
        for (key, val) in object {
            preview[key] = val.clone();
        }
    }
    super::writing_api::check_snapshot(state, &req.session_id, &req.source, req.serial, None)?;
    let sid = req.session_id.clone();
    let base = req.base_id.clone();
    core_call(state, move |core| core.check_session_base(&sid, &base)).await?;
    Ok((artifact, preview))
}

async fn pdf(
    state: &AppState,
    req: &PrepareRequest,
    loc: &Location,
) -> Result<(ExportArtifact, Value), ApiError> {
    let embedded = super::embedded_output::freeze(state, req, &loc.root.to_string_lossy())?;
    let cut_req = req.clone();
    let cut_loc = loc.clone();
    let cut = core_call(state, move |core| capture(core, &cut_loc, &cut_req)).await?;
    let mut input = FrozenRenderRequest {
        source: req.source.clone(),
        path: loc.path.clone(),
        root: loc.root.clone(),
        files: cut.files,
        style: cut.style,
    };
    let compile = |value: Value| -> Result<weftext_engine::PagedOutput, ApiError> {
        let output: weftext_engine::PagedOutput = serde_json::from_value(value).map_err(|_| {
            ApiError::internal(
                UiMessage::error_api_export_conversion_invalid_frozen_paged_contract(),
            )
        })?;
        if !output.ok {
            let mut error =
                unavailable(UiMessage::error_api_export_conversion_pdf_compile_failed());
            // Compiler diagnostics are a collection with source locations,
            // not a causal error chain. Keep the complete typed collection.
            error.details = Some(Box::new(ApiErrorDetails::EngineDiagnostics {
                diagnostics: output.diagnostics,
            }));
            return Err(error);
        }
        Ok(output)
    };
    let original = compile(render_frozen_target_worker(state, input.clone(), true).await?)?;
    let (source, losses) = super::embedded_output::paged_source(
        &req.source,
        &embedded,
        &original.embedded_queries,
        req.output_locale,
    )?;
    let output = if source == req.source {
        original
    } else {
        input.source = source;
        let key = relative(&loc.root, &loc.path).map_err(ApiError::from)?;
        input.files.insert(key, input.source.as_bytes().to_vec());
        compile(render_frozen_target_worker(state, input, true).await?)?
    };
    let bytes =
        decode(output.pdf_base64.as_deref().ok_or_else(|| {
            unavailable(UiMessage::error_api_export_conversion_pdf_file_missing())
        })?)?;
    let preview = json!({"target":"paged","scope":"frozenDraftWithSelectedQueryResults","cutId":cut.cut_id,"complete":cut.complete,"warnings":cut.warnings,"pageCount":output.page_count,"embeddedQueries":super::embedded_output::preview(&embedded)?,"diagnostics":output.diagnostics});
    Ok((
        ExportArtifact {
            bytes,
            content_type: "application/pdf".into(),
            extension: "pdf".into(),
            losses,
            summary: UiMessage::error_api_export_conversion_pdf_summary(
                weftext_messages::params::ErrorApiExportConversionPdfSummary {
                    pages: weftext_messages::UiInteger::new(
                        i64::try_from(output.page_count).expect("page count is bounded"),
                    )
                    .expect("page count is a safe integer"),
                },
            ),
        },
        preview,
    ))
}

async fn native(
    state: &AppState,
    req: &PrepareRequest,
    loc: &Location,
) -> Result<(ExportArtifact, Value), ApiError> {
    let locations = state
        .locations
        .lock()
        .map_err(|_| {
            ApiError::internal(UiMessage::error_api_writing_document_session_unavailable())
        })?
        .clone();
    let versions = state
        .editor_versions
        .lock()
        .map_err(|_| ApiError::internal(UiMessage::error_api_writing_editor_session_unavailable()))?
        .clone();
    let drafts = versions
        .into_iter()
        .filter_map(|(id, version)| {
            locations
                .get(&id)
                .filter(|l| l.root == loc.root)
                .map(|l| (l.path.clone(), version.source))
        })
        .collect::<Vec<_>>();
    let request = req.clone();
    let location = loc.clone();
    let (bytes, package) = core_call(state, move |core| {
        core.check_session_base(&request.session_id, &request.base_id)?;
        let bytes = core.export_native_package(&location.root)?;
        let mut package =
            weftext_export::exchange::read_native_package(&bytes).map_err(native_error)?;
        let key = relative(&location.root, &location.path)?;
        if package.files.get(&key).map(Vec::as_slice) != Some(request.source.as_bytes()) {
            return Err(CoreError::new(
                "dirtyDraft",
                UiMessage::error_api_export_conversion_save_current_draft_before_native_export(),
            ));
        }
        for (path, source) in drafts {
            let key = relative(&location.root, &path)?;
            if package
                .files
                .get(&key)
                .is_some_and(|saved| saved.as_slice() != source.as_bytes())
            {
                return Err(CoreError::new(
                    "dirtyDraft",
                    UiMessage::error_api_export_conversion_save_other_draft_before_native_export(),
                ));
            }
        }
        complete_native_bibliographies(&mut package)?;
        let bytes =
            weftext_export::exchange::write_native_package(&package).map_err(native_error)?;
        core.check_session_base(&request.session_id, &request.base_id)?;
        Ok((bytes, package))
    })
    .await?;
    let losses = package
        .manifest
        .losses
        .iter()
        .map(|loss| ExportLoss {
            code: loss.code.clone(),
            description: loss.description.clone(),
            product_diagnostics: loss.product_diagnostics.clone(),
            location: Some(loss.path.clone()),
        })
        .chain(std::iter::once(report(
            "native_not_recovery_backup",
            UiMessage::error_api_export_conversion_native_package_not_recovery_backup(),
        )))
        .collect();
    let preview = json!({"scope":"savedWholeLibrary","workspaceId":package.manifest.workspace_id,"fileCount":package.files.len(),"files":package.manifest.entries,"summary":UiMessage::error_api_export_conversion_whole_library_scope()});
    Ok((
        ExportArtifact {
            bytes,
            content_type: "application/zip".into(),
            extension: "weftext.zip".into(),
            losses,
            summary: UiMessage::error_api_export_conversion_native_package_summary(
                weftext_messages::params::ErrorApiExportConversionNativePackageSummary {
                    files: weftext_messages::UiInteger::new(package.files.len() as i64)
                        .expect("native package file count is bounded"),
                },
            ),
        },
        preview,
    ))
}

/// The saved whole-library package is the sole authority for these derived
/// inputs. Never substitute the current Draft or reread a Work from disk.
fn complete_native_bibliographies(
    package: &mut weftext_export::exchange::NativePackage,
) -> Result<(), CoreError> {
    use weftext_export::exchange::NativeFileRole;
    let unavailable = |message| CoreError::new("citationWorkUnavailable", message);
    let sources = package
        .manifest
        .entries
        .iter()
        .filter(|(_, entry)| entry.role == NativeFileRole::Source)
        .collect::<Vec<_>>();
    let mut generated = vec![];
    for (path, owner) in &sources {
        let source = std::str::from_utf8(&package.files[*path]).map_err(|_| {
            unavailable(UiMessage::error_api_export_conversion_saved_citation_owner_encoding())
        })?;
        let Some(declaration) = weftext_engine::inspect_citation_declaration(source)
            .map_err(|e| CoreError::from_engine("citationDeclarationUnavailable", e))?
        else {
            continue;
        };
        if weftext_engine::inspect(source).id.as_deref() != owner.node_id.as_deref()
            || !weftext_engine::native_citation_bibliography_input(source)
        {
            return Err(CoreError::new(
                "citationDeclarationUnavailable",
                UiMessage::error_api_export_conversion_citation_owner_ambiguous(),
            ));
        }
        let schema_bytes = &package.files[&package.manifest.schema_path];
        let schema = std::str::from_utf8(schema_bytes)
            .map_err(|_| {
                CoreError::new(
                    "citationSchemaUnavailable",
                    UiMessage::error_api_export_conversion_frozen_schema_encoding(),
                )
            })
            .and_then(|source| {
                Schema::from_author_json(source)
                    .map_err(|error| CoreError::from_data("citationSchemaUnavailable", error))
            })?;
        let mut works = vec![];
        for entry in declaration.entries {
            let prefix = format!("weftext-node://{}/", package.manifest.workspace_id);
            let node = entry.work.strip_prefix(&prefix).ok_or_else(|| {
                unavailable(UiMessage::error_api_export_conversion_work_outside_saved_cut())
            })?;
            let matches = sources
                .iter()
                .filter(|(_, source)| source.node_id.as_deref() == Some(node))
                .collect::<Vec<_>>();
            let [(work_path, work_entry)] = matches.as_slice() else {
                return Err(unavailable(
                    UiMessage::error_api_export_conversion_saved_work_ambiguous(),
                ));
            };
            let source = std::str::from_utf8(&package.files[*work_path]).map_err(|_| {
                unavailable(UiMessage::error_api_export_conversion_saved_work_encoding())
            })?;
            let projection = weftext_data::project(source, &schema);
            if projection.qualification != "qualified"
                || projection.id.as_deref() != Some(node)
                || !projection
                    .resolved_types
                    .iter()
                    .any(|t| t == "library/work")
            {
                return Err(CoreError::new(
                    "citationWorkType",
                    UiMessage::error_api_export_conversion_saved_work_type_unavailable(),
                ));
            }
            let publication = projection
                .fields
                .iter()
                .find(|field| field.field_id == "library/publication")
                .and_then(|field| field.cell.native())
                .cloned()
                .ok_or_else(|| {
                    CoreError::new(
                        "citationPublicationUnavailable",
                        UiMessage::error_api_export_conversion_saved_publication_unavailable(),
                    )
                })?;
            works.push(CitationWork {
                key: entry.key,
                work: FrozenWork {
                    entity_ref: entry.work,
                    source_basis: format!(
                        "native-package:{}:revision:{}:hash:{}",
                        package.manifest.package_id,
                        work_entry.revision.as_deref().unwrap_or("unavailable"),
                        weftext_engine::source_hash(source)
                    ),
                    publication,
                },
            });
        }
        let bibliography =
            project_bibliography(&works, BibliographyFormat::Bibtex).map_err(|error| {
                let (description, diagnostics) = error.into_parts();
                CoreError::from_product(
                    "citationBibliographyUnavailable",
                    None,
                    description,
                    diagnostics,
                )
            })?;
        generated.push((owner.node_id.clone().unwrap(), bibliography.bytes));
    }
    // Retain all originals if any generated path collides or exceeds budget.
    let mut completed = package.clone();
    for (owner, bytes) in generated {
        weftext_export::exchange::append_generated_bibliography(&mut completed, &owner, bytes)
            .map_err(native_error)?;
    }
    *package = completed;
    Ok(())
}

fn field_cell(projection: &NoteProjection, name: &str) -> Result<Cell, ApiError> {
    if projection.qualification != "qualified" {
        return Err(unavailable(
            UiMessage::error_api_export_conversion_draft_projection_unavailable(),
        ));
    }
    let matches = projection
        .fields
        .iter()
        .filter(|f| f.key == name || f.field_id == name)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(unavailable(
            UiMessage::error_api_export_conversion_field_not_unique(
                weftext_messages::params::ErrorApiExportConversionFieldNotUnique {
                    name: name.into(),
                },
            ),
        ));
    }
    Ok(matches[0].cell.clone())
}
pub(crate) fn cell_text(cell: &Cell) -> Result<String, ApiError> {
    Ok(match cell {
        Cell::Value { value } => match value {
            NativeValue::Text(s)
            | NativeValue::Int(s)
            | NativeValue::Decimal(s)
            | NativeValue::Float(s)
            | NativeValue::Date(s)
            | NativeValue::Time(s)
            | NativeValue::LocalDatetime(s)
            | NativeValue::Duration(s) => s.clone(),
            NativeValue::None => "<null>".into(),
            NativeValue::Bool(b) => b.to_string(),
            NativeValue::Array(_) | NativeValue::Record(_) => serde_json::to_string_pretty(value)
                .map_err(|e| {
                ApiError::internal(UiMessage::error_api_export_conversion_cell_encoding_failed())
                    .diagnostic(DiagnosticOrigin::Json, e.to_string())
            })?,
        },
        Cell::Absent => "<absent>".into(),
        Cell::Null => "<null>".into(),
        // Error cells use the same closed machine representation as CSV/TSV.
        // UI language never becomes author data or discards parallel diagnostics.
        Cell::Invalid { .. } | Cell::Unavailable { .. } => {
            serde_json::to_string(cell).map_err(|error| {
                ApiError::internal(UiMessage::error_api_export_conversion_cell_encoding_failed())
                    .diagnostic(DiagnosticOrigin::Json, error.to_string())
            })?
        }
    })
}
fn binding_cell(
    binding: &Binding,
    projection: &NoteProjection,
    render: Option<&RenderOutput>,
) -> Result<Cell, ApiError> {
    match binding.kind.as_str() {
        "field" => field_cell(
            projection,
            binding.field.as_deref().ok_or_else(|| {
                ApiError::bad(UiMessage::error_api_export_conversion_field_binding_missing_field())
            })?,
        ),
        "text" => Ok(Cell::value(NativeValue::Text(
            binding.text.clone().ok_or_else(|| {
                ApiError::bad(UiMessage::error_api_export_conversion_literal_binding_missing_text())
            })?,
        ))),
        "title" => render
            .and_then(|r| r.products.as_ref())
            .and_then(|p| p.title.clone())
            .map(|s| Cell::value(NativeValue::Text(s)))
            .ok_or_else(|| {
                unavailable(UiMessage::error_api_export_conversion_unique_title_unavailable())
            }),
        _ => Err(ApiError::bad(
            UiMessage::error_api_export_conversion_unsupported_cell_binding(),
        )),
    }
}
pub(crate) fn node(tag: &str, key: String, children: Vec<RenderNode>) -> RenderNode {
    RenderNode {
        key,
        kind: "element".into(),
        tag: Some(tag.into()),
        text: None,
        attrs: BTreeMap::new(),
        children,
        origin: None,
        writable: false,
    }
}
pub(crate) fn text_node(text: String, key: String) -> RenderNode {
    RenderNode {
        key,
        kind: "text".into(),
        tag: None,
        text: Some(text),
        attrs: BTreeMap::new(),
        children: vec![],
        origin: None,
        writable: false,
    }
}
pub(crate) fn query_table(
    result: &QueryResult,
    columns: &[String],
) -> Result<RenderNode, ApiError> {
    if columns.is_empty() || columns.len() > 512 || result.rows.len() > 4999 {
        return Err(unavailable(
            UiMessage::error_api_export_conversion_query_table_budget(),
        ));
    }
    let names: std::collections::BTreeSet<_> = columns.iter().collect();
    if names.len() != columns.len()
        || result
            .terminal_schema
            .keys()
            .any(|key| !names.contains(key))
        || result
            .rows
            .iter()
            .any(|row| row.cells.keys().any(|key| !names.contains(key)))
    {
        return Err(unavailable(
            UiMessage::error_api_export_conversion_query_column_mismatch(),
        ));
    }
    let mut rows = vec![node(
        "tr",
        "query-header".into(),
        columns
            .iter()
            .enumerate()
            .map(|(c, name)| {
                node(
                    "th",
                    format!("query-h-{c}"),
                    vec![node(
                        "p",
                        format!("query-hp-{c}"),
                        vec![text_node(name.clone(), format!("query-ht-{c}"))],
                    )],
                )
            })
            .collect(),
    )];
    for (r, row) in result.rows.iter().enumerate() {
        let mut cells = vec![];
        for (c, name) in columns.iter().enumerate() {
            cells.push(node(
                "td",
                format!("query-cell-{r}-{c}"),
                vec![node(
                    "p",
                    format!("query-p-{r}-{c}"),
                    vec![text_node(
                        cell_text(row.cells.get(name).unwrap_or(&Cell::Absent))?,
                        format!("query-text-{r}-{c}"),
                    )],
                )],
            ));
        }
        rows.push(node("tr", format!("query-row-{r}"), cells));
    }
    Ok(node("table", "frozen-query-table".into(), rows))
}
pub(crate) fn query_losses(result: &QueryResult) -> Vec<ExportLoss> {
    let mut losses = vec![report(
        "docx_query_structure",
        UiMessage::error_api_export_conversion_docx_query_structure(),
    )];
    if result.coverage != "complete" || result.limited || !result.aggregate_exact {
        losses.push(report(
            "query_incomplete",
            UiMessage::error_api_export_conversion_query_incomplete(),
        ));
    }
    losses
}
struct DocxExtras<'a> {
    embedded: &'a [super::embedded_output::FrozenEmbedded],
    reviews: &'a [weftext_export::docx::DocxReview],
}
fn docx(
    req: &PrepareRequest,
    mut render: RenderOutput,
    projection: &NoteProjection,
    template: Option<Vec<u8>>,
    query: Option<&QueryResult>,
    columns: Option<&[String]>,
    extras: DocxExtras<'_>,
) -> Result<(ExportArtifact, Value), ApiError> {
    let DocxExtras { embedded, reviews } = extras;
    let mut bindings = BTreeMap::new();
    let mut losses = vec![];
    let mut query_bound = false;
    if template.is_none() || req.bindings.values().any(|binding| binding.kind == "body") {
        if let Some(tree) = render.tree.as_mut() {
            losses.extend(super::embedded_output::materialize(
                tree,
                embedded,
                req.output_locale,
            )?);
        }
    } else if !embedded.is_empty() {
        losses.push(report(
            "embedded_query_body_not_selected",
            UiMessage::error_api_export_conversion_embedded_body_not_selected(),
        ));
    }
    for (name, binding) in &req.bindings {
        let mapped = match binding.kind.as_str() {
            "body" => DocxBinding::Body,
            "title" => DocxBinding::Title,
            "text" => DocxBinding::Text {
                value: binding.text.clone().ok_or_else(|| {
                    ApiError::bad(UiMessage::error_api_export_conversion_literal_missing())
                })?,
            },
            "field" => {
                let cell = field_cell(
                    projection,
                    binding.field.as_deref().ok_or_else(|| {
                        ApiError::bad(UiMessage::error_api_export_conversion_field_missing())
                    })?,
                )?;
                losses.push(report(
                    "docx_field_structure",
                    UiMessage::error_api_export_conversion_docx_field_structure(),
                ));
                DocxBinding::Text {
                    value: cell_text(&cell)?,
                }
            }
            "query" => {
                let query = query.ok_or_else(|| {
                    ApiError::bad(
                        UiMessage::error_api_export_conversion_query_binding_missing_result(),
                    )
                })?;
                query_bound = true;
                DocxBinding::Nodes {
                    nodes: vec![query_table(query, columns.unwrap())?],
                }
            }
            _ => {
                return Err(ApiError::bad(
                    UiMessage::error_api_export_conversion_unknown_docx_binding(
                        weftext_messages::params::ErrorApiExportConversionUnknownDocxBinding {
                            kind: binding.kind.clone(),
                        },
                    ),
                ));
            }
        };
        bindings.insert(name.clone(), mapped);
    }
    if let Some(query) = query {
        if template.is_none() {
            let tree = render.tree.take().ok_or_else(|| {
                unavailable(UiMessage::error_api_export_conversion_frozen_html_body_missing())
            })?;
            render.tree = Some(node(
                "div",
                "document-with-frozen-query".into(),
                vec![tree, query_table(query, columns.unwrap())?],
            ));
            query_bound = true;
        }
        if !query_bound {
            return Err(ApiError::bad(
                UiMessage::error_api_export_conversion_query_selection_not_bound(),
            ));
        }
        losses.extend(query_losses(query));
    }
    let artifact = weftext_export::docx::export_reviewed_docx(
        DocxInput {
            render: &render,
            template: template.as_deref(),
            bindings: &bindings,
            options: DocxOptions {
                preserve_source_styles: req.style_policy == "source",
                reject_losses: false,
            },
        },
        reviews,
    )
    .map_err(convert_error)?;
    let preview = json!({"target":"html","title":render.products.as_ref().and_then(|p|p.title.clone()),"body":render.products.as_ref().and_then(|p|p.body.clone()),"bindings":req.bindings,"rowCount":query.map(|q|q.rows.len())});
    let mut artifact = artifact;
    artifact.losses.extend(losses);
    Ok((artifact, preview))
}
fn note_result(
    projection: &NoteProjection,
    cut: &DocumentCut,
) -> Result<(QueryResult, Vec<String>), ApiError> {
    if projection.qualification != "qualified" || projection.fields.is_empty() {
        return Err(unavailable(
            UiMessage::error_api_export_conversion_qualified_fields_unavailable(),
        ));
    }
    if cut.owner_id.is_some() && projection.id != cut.owner_id {
        return Err(unavailable(
            UiMessage::error_api_export_conversion_draft_owner_mismatch(),
        ));
    }
    let mut cells = BTreeMap::new();
    let mut columns = vec![];
    for field in &projection.fields {
        columns.push(field.key.clone());
        cells.insert(field.key.clone(), field.cell.clone());
    }
    for field in &projection.unknown_fields {
        let key = format!("unknown:{}", field.key);
        columns.push(key.clone());
        cells.insert(
            key,
            Cell::Invalid {
                diagnostics: vec![ErrorDescription::new(
                    UiMessage::error_api_export_conversion_unknown_field_preserved(
                        weftext_messages::params::ErrorApiExportConversionUnknownFieldPreserved {
                            source: field.raw.clone(),
                        },
                    ),
                )],
                value: field.value.clone(),
            },
        );
    }
    let result = QueryResult {
        cut_id: cut.cut_id.clone(),
        schema_revision: weftext_engine::source_hash(&cut.schema_json.to_string()),
        rows: vec![QueryRow {
            key: "current-draft-fields".into(),
            entity_key: cut
                .workspace_id
                .as_ref()
                .zip(cut.owner_id.as_ref())
                .map(|(workspace, owner)| format!("weftext-node://{workspace}/{owner}")),
            path: None,
            revision: None,
            cells,
            read_only: true,
            lifecycle: Some("live".into()),
        }],
        coverage: if cut.complete { "complete" } else { "partial" }.into(),
        freshness: "current".into(),
        diagnostics: projection.diagnostics.clone(),
        total_known: 1,
        limited: false,
        aggregate_exact: true,
        runtime: "frozenTypedProjection".into(),
        terminal_schema: columns
            .iter()
            .map(|c| (c.clone(), QueryValueType::Any))
            .collect(),
    };
    Ok((result, columns))
}
fn xlsx(
    req: &PrepareRequest,
    render: Option<&RenderOutput>,
    projection: &NoteProjection,
    template: Option<Vec<u8>>,
    query: Option<QueryResult>,
    query_context: Option<(&Vec<String>, &Value)>,
    cut: &DocumentCut,
) -> Result<(ExportArtifact, Value), ApiError> {
    let field_template = template.is_some() && query.is_none();
    let result_and_schema = if let Some(query) = query {
        let (columns, schema) = query_context.unwrap();
        Some((query, columns.clone(), schema.clone()))
    } else if field_template {
        None
    } else {
        let (result, columns) = note_result(projection, cut)?;
        Some((result, columns, cut.schema_json.clone()))
    };
    let mut fields = vec![];
    let mut queries = vec![];
    for (anchor, binding) in &req.bindings {
        if binding.kind == "query" {
            queries.push(XlsxQuerySlot {
                anchor: anchor.clone(),
                sheet: 0,
            });
        } else {
            fields.push(XlsxFieldSlot {
                anchor: anchor.clone(),
                cell: binding_cell(binding, projection, render)?,
            });
        }
    }
    if req.query_id.is_none() && !queries.is_empty() {
        return Err(ApiError::bad(
            UiMessage::error_api_export_conversion_query_region_needs_frozen_result(),
        ));
    }
    let request = XlsxRequest {
        sheets: result_and_schema
            .as_ref()
            .map(|(result, columns, schema)| {
                vec![XlsxSheetInput {
                    name: UiMessage::error_api_export_conversion_data_sheet_name()
                        .render(req.output_locale),
                    columns: columns.clone(),
                    result: result.clone(),
                    schema: Some(schema.clone()),
                }]
            })
            .unwrap_or_default(),
        template: template.map(|bytes| XlsxTemplate {
            bytes,
            fields,
            queries,
        }),
    };
    let mut artifact = weftext_export::xlsx::export_xlsx(&request).map_err(convert_error)?;
    if req.style_policy == "source" {
        artifact.losses.push(report(
            "xlsx_source_style_unmapped",
            UiMessage::error_api_export_conversion_xlsx_source_style_unmapped(),
        ));
    }
    let preview = if let Some((result, columns, _)) = result_and_schema {
        json!({"sheets":[{"name":UiMessage::error_api_export_conversion_data_sheet_name().render(req.output_locale),"columns":columns,"rowCount":result.rows.len(),"coverage":result.coverage,"freshness":result.freshness,"limited":result.limited}],"rows":result.rows.iter().take(12).collect::<Vec<_>>(),"previewRowsShown":result.rows.len().min(12),"summary":UiMessage::error_api_export_conversion_complete_frozen_rows_preview()})
    } else {
        json!({"scope":"typedTemplateFields","fields":request.template.as_ref().map(|t|&t.fields),"summary":UiMessage::error_api_export_conversion_selected_template_fields_preview()})
    };
    Ok((artifact, preview))
}
fn bibliography(
    req: &PrepareRequest,
    projection: &NoteProjection,
    cut: &DocumentCut,
) -> Result<(ExportArtifact, Value), ApiError> {
    let format = match req.format.as_str() {
        "bibtex" => BibliographyFormat::Bibtex,
        "cslJson" | "csl-json" => BibliographyFormat::CslJson,
        "ris" => BibliographyFormat::Ris,
        _ => {
            return Err(ApiError::bad(
                UiMessage::error_api_export_conversion_unknown_bibliography_format(),
            ));
        }
    };
    let works = if let Some(citations) = &cut.citations {
        citations.works.clone()
    } else {
        if projection.qualification != "qualified"
            || !projection
                .resolved_types
                .iter()
                .any(|t| t == "library/work")
        {
            return Err(unavailable(
                UiMessage::error_api_export_conversion_work_declaration_required(),
            ));
        }
        let owner = cut
            .owner_id
            .as_ref()
            .filter(|id| projection.id.as_ref() == Some(id))
            .ok_or_else(|| {
                unavailable(UiMessage::error_api_export_conversion_work_identity_missing())
            })?;
        let workspace = cut.workspace_id.as_ref().ok_or_else(|| {
            unavailable(UiMessage::error_api_export_conversion_work_library_required())
        })?;
        let publication = projection
            .fields
            .iter()
            .find(|f| f.field_id == "library/publication")
            .and_then(|f| f.cell.native())
            .cloned()
            .ok_or_else(|| {
                unavailable(UiMessage::error_api_export_conversion_publication_unavailable())
            })?;
        vec![CitationWork {
            key: format!("work_{}", owner.replace('-', "")),
            work: FrozenWork {
                entity_ref: format!("weftext-node://{workspace}/{owner}"),
                source_basis: format!(
                    "draft-cut:{}:hash:{}",
                    cut.cut_id,
                    weftext_engine::source_hash(&req.source)
                ),
                publication,
            },
        }]
    };
    let projection = project_bibliography(&works, format).map_err(convert_error)?;
    let (content_type, extension) = match format {
        BibliographyFormat::Bibtex => ("application/x-bibtex; charset=utf-8", "bib"),
        BibliographyFormat::CslJson => ("application/json; charset=utf-8", "json"),
        BibliographyFormat::Ris => ("application/x-research-info-systems; charset=utf-8", "ris"),
    };
    Ok((
        ExportArtifact {
            bytes: projection.bytes,
            content_type: content_type.into(),
            extension: extension.into(),
            losses: projection.losses,
            summary: UiMessage::error_api_export_conversion_bibliography_summary(
                weftext_messages::params::ErrorApiExportConversionBibliographySummary {
                    works: weftext_messages::UiInteger::new(works.len() as i64)
                        .expect("bibliography work count is bounded"),
                },
            ),
        },
        json!({"scope":"frozenCitationWorks","works":works,"dependencies":projection.dependencies}),
    ))
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn captured_document_warnings_keep_core_descriptions_and_exact_missing_identity() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let selected = core.create_note_fixture(&root, &root, "Selected").unwrap();
        let missing = core.create_note_fixture(&root, &root, "Missing").unwrap();
        let original = std::fs::read(&missing.path).unwrap();
        let retained = temp.path().join("original-missing-source.typ");
        std::fs::rename(&missing.path, &retained).unwrap();
        let request:PrepareRequest=serde_json::from_value(json!({"requestId":uuid::Uuid::new_v4(),"sessionId":selected.session_id,"source":selected.source,"serial":1,"baseId":selected.base_id,"format":"docx","outputLocale":"zh-CN"})).unwrap();
        let location = Location {
            path: selected.path.into(),
            root: root.clone(),
        };
        let cut = capture(&mut core, &location, &request).unwrap();
        assert!(!cut.complete);
        let owner = missing.node_id.unwrap();
        let warning = cut
            .warnings
            .iter()
            .find(|warning| {
                warning.explanation.key() == "error.core.operations.admittedNodeUnavailable"
            })
            .unwrap();
        let wire = serde_json::to_value(warning).unwrap();
        assert_eq!(wire["explanation"]["params"]["nodeId"], owner);
        let decoded: weftext_core::CoreDescription = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(&decoded, warning);
        assert!(wire.get("message").is_none());
        assert_eq!(std::fs::read(&retained).unwrap(), original);
        core.release_compile_dependency_cut(&cut.cut_id);
    }

    use super::*;
    use weftext_export::exchange::{
        NATIVE_FORMAT, NATIVE_VERSION, NativeEntry, NativeFileRole, NativeManifest, NativePackage,
    };
    fn native_citation_fixture() -> NativePackage {
        let owner = "00000000-0000-4000-8000-000000000001";
        let work = "00000000-0000-4000-8000-000000000002";
        let workspace = "00000000-0000-4000-8000-000000000003";
        let source = format!(
            "#metadata((id: \"{owner}\", types: (\"note\",))) <weftext>\n\
             #metadata((kind: \"weftext.citations\", entries: ((key: \"exact\", work: \"weftext-node://{workspace}/{work}\"),))) <weftext-citations>\n\
             @exact\n#bibliography(\"weftext.citations.bib\")\n"
        );
        let work_source = format!(
            "#metadata((id: \"{work}\", types: (\"paper\",), publication: (kind: \"article\", title: \"Exact saved publication\", contributors: ((agent: (text: \"Author\"), roles: (\"author\",)),)))) <weftext>"
        );
        let files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
            ("Owner/Owner.typ".into(), source.into_bytes()),
            ("Work/Work.typ".into(), work_source.into_bytes()),
            (
                ".weftext-schema.json".into(),
                include_bytes!("../../../docs/design/examples/schema.authoring.json").to_vec(),
            ),
        ]);
        let entries = files
            .iter()
            .map(|(path, bytes)| {
                let role = if path == ".weftext-schema.json" {
                    NativeFileRole::Schema
                } else {
                    NativeFileRole::Source
                };
                let mut entry = NativeEntry::new(role, bytes);
                if path == "Owner/Owner.typ" {
                    entry.node_id = Some(owner.into());
                } else if path == "Work/Work.typ" {
                    entry.node_id = Some(work.into());
                }
                (path.clone(), entry)
            })
            .collect();
        NativePackage {
            manifest: NativeManifest {
                format: NATIVE_FORMAT.into(),
                version: NATIVE_VERSION,
                package_id: "00000000-0000-4000-8000-000000000004".into(),
                workspace_id: workspace.into(),
                scope: "library".into(),
                schema_path: ".weftext-schema.json".into(),
                entries,
                losses: vec![],
            },
            files,
        }
    }
    fn replace_native_source(package: &mut NativePackage, path: &str, source: String) {
        let entry = package.manifest.entries.get_mut(path).unwrap();
        entry.bytes = source.len();
        entry.sha256 = weftext_export::exchange::digest(source.as_bytes());
        package.files.insert(path.into(), source.into_bytes());
    }
    #[test]
    fn native_bibliographies_use_saved_package_work_and_preserve_all_original_members() {
        let mut package = native_citation_fixture();
        let originals = package.files.clone();
        complete_native_bibliographies(&mut package).unwrap();
        for (path, original) in originals {
            assert_eq!(package.files[&path], original);
        }
        let generated = &package.files["Owner/weftext.citations.bib"];
        assert!(
            std::str::from_utf8(generated)
                .unwrap()
                .contains("Exact saved publication")
        );
        assert_eq!(
            package.manifest.entries["Owner/weftext.citations.bib"].role,
            NativeFileRole::GeneratedBibliography
        );
        let bytes = weftext_export::exchange::write_native_package(&package).unwrap();
        let reloaded = weftext_export::exchange::read_native_package(&bytes).unwrap();
        assert_eq!(reloaded.files, package.files);
    }
    #[test]
    fn native_bibliographies_reject_bad_work_schema_ownership_and_occupied_paths_atomically() {
        for (path, from, to, expected) in [
            (
                "Work/Work.typ",
                "types: (\"paper\",)",
                "types: (\"note\",)",
                "citationWorkType",
            ),
            (
                "Work/Work.typ",
                "00000000-0000-4000-8000-000000000002",
                "00000000-0000-4000-8000-000000000005",
                "citationWorkType",
            ),
            (
                "Work/Work.typ",
                "publication:",
                "unavailable_publication:",
                "citationPublicationUnavailable",
            ),
            (
                "Owner/Owner.typ",
                "weftext-node://00000000-0000-4000-8000-000000000003/",
                "weftext-node://00000000-0000-4000-8000-000000000006/",
                "citationWorkUnavailable",
            ),
            (
                "Owner/Owner.typ",
                "#bibliography(\"weftext.citations.bib\")",
                "#bibliography(\"weftext.citations.bib\")\n#bibliography(\"external.bib\")",
                "citationDeclarationUnavailable",
            ),
            (
                "Owner/Owner.typ",
                "#bibliography(\"weftext.citations.bib\")",
                "#bibliography(\"external.bib\")",
                "citationDeclarationUnavailable",
            ),
            (
                ".weftext-schema.json",
                "{",
                "invalid schema {",
                "citationSchemaUnavailable",
            ),
        ] {
            let mut package = native_citation_fixture();
            let source = std::str::from_utf8(&package.files[path])
                .unwrap()
                .replacen(from, to, 1);
            replace_native_source(&mut package, path, source);
            let originals = package.files.clone();
            assert_eq!(
                complete_native_bibliographies(&mut package)
                    .unwrap_err()
                    .code,
                expected
            );
            assert_eq!(package.files, originals);
        }
        let mut package = native_citation_fixture();
        package
            .manifest
            .losses
            .push(weftext_export::exchange::NativeLoss {
                path: "Owner/WEFTEXT.CITATIONS.BIB".into(),
                code: "unregisteredFileOmitted".into(),
                description: UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: "Original remains outside the package".into(),
                    },
                )
                .into(),
                product_diagnostics: vec![],
            });
        let originals = package.files.clone();
        assert_eq!(
            complete_native_bibliographies(&mut package)
                .unwrap_err()
                .code,
            "packagePath"
        );
        assert_eq!(package.files, originals);
    }
    #[test]
    fn readable_typed_slots_keep_exact_text_and_distinct_states() {
        let issue = ErrorDescription::new(UiMessage::common_operation(
            weftext_messages::params::CommonOperation {
                operation_id: "original diagnostic 😀".into(),
            },
        ));
        assert_eq!(
            cell_text(&Cell::value(NativeValue::Int("9223372036854775807".into()))).unwrap(),
            "9223372036854775807"
        );
        assert_eq!(
            cell_text(&Cell::value(NativeValue::Decimal(
                "0.10000000000000000001".into()
            )))
            .unwrap(),
            "0.10000000000000000001"
        );
        let states = [
            Cell::Absent,
            Cell::Null,
            Cell::Invalid {
                diagnostics: vec![issue.clone()],
                value: None,
            },
            Cell::Unavailable {
                kind: weftext_data::DataErrorKind::Unavailable,
                reason: issue.clone(),
                diagnostics: vec![],
            },
        ]
        .map(|c| cell_text(&c).unwrap());
        assert_eq!(
            states
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        assert_eq!(
            cell_text(&Cell::value(NativeValue::Text("=1+1".into()))).unwrap(),
            "=1+1"
        );
        let invalid = Cell::Invalid {
            diagnostics: vec![issue.clone(); 40],
            value: Some(NativeValue::Text(
                "Exact invalid author value 😀\r\n".into(),
            )),
        };
        let exported: Value = serde_json::from_str(&cell_text(&invalid).unwrap()).unwrap();
        assert_eq!(exported["state"], "invalid");
        assert_eq!(exported["diagnostics"].as_array().unwrap().len(), 40);
        assert_eq!(
            exported["value"],
            value(NativeValue::Text(
                "Exact invalid author value 😀\r\n".into()
            ))
            .unwrap()
        );
        assert_eq!(exported, value(&invalid).unwrap());
    }
    #[test]
    fn unsupported_binding_and_template_formats_refuse() {
        let projection = NoteProjection {
            qualification: "qualified".into(),
            id: None,
            types: vec![],
            resolved_types: vec![],
            fields: vec![],
            unknown_fields: vec![],
            diagnostics: vec![],
        };
        let binding = Binding {
            kind: "body".into(),
            field: None,
            text: None,
        };
        assert!(binding_cell(&binding, &projection, None).is_err());
        assert!(field_cell(&projection, "missing").is_err());
        assert!(template_slots("native", b"unused").is_err());
    }
    #[test]
    fn note_workbook_uses_qualified_entity_ref_and_rejects_draft_identity_change() {
        let schema = Schema::bundled().unwrap();
        let owner = "12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
        let workspace = "58c47256-bfce-4d29-a037-b83a3a644fb0";
        let source = format!(
            "#metadata((id: \"{owner}\", types: (\"paper\",), publication: (kind: \"article\", title: \"Original\"))) <weftext>"
        );
        let projection = weftext_data::project(&source, &schema);
        let cut = DocumentCut {
            reviews: vec![],
            files: BTreeMap::new(),
            style: None,
            schema_json: serde_json::to_value(&schema).unwrap(),
            schema,
            cut_id: "cut-one".into(),
            workspace_id: Some(workspace.into()),
            owner_id: Some(owner.into()),
            citations: None,
            warnings: vec![],
            complete: true,
        };
        let (result, _) = note_result(&projection, &cut).unwrap();
        assert_eq!(
            result.rows[0].entity_key.as_deref(),
            Some(format!("weftext-node://{workspace}/{owner}").as_str())
        );
        let changed = source.replace(owner, "8aa10c08-3e7d-4d10-9488-92bc7f393050");
        let other = weftext_data::project(&changed, &cut.schema);
        assert!(note_result(&other, &cut).is_err());
    }
    #[test]
    fn frozen_query_table_keeps_supplied_column_and_row_order_and_refuses_omissions() {
        let row = |key: &str, name: &str| QueryRow {
            key: key.into(),
            entity_key: None,
            path: None,
            revision: None,
            cells: BTreeMap::from([
                ("name".into(), Cell::value(NativeValue::Text(name.into()))),
                (
                    "phone".into(),
                    Cell::value(NativeValue::Text("00123".into())),
                ),
            ]),
            read_only: true,
            lifecycle: None,
        };
        let result = QueryResult {
            cut_id: "older-query-cut".into(),
            schema_revision: "older-schema".into(),
            rows: vec![row("b", "乙"), row("a", "甲"), row("repeat", "乙")],
            coverage: "partial".into(),
            freshness: "stale".into(),
            diagnostics: vec![],
            total_known: 3,
            limited: false,
            aggregate_exact: false,
            runtime: "frozen".into(),
            terminal_schema: BTreeMap::new(),
        };
        let columns = vec!["phone".into(), "name".into()];
        let tree = query_table(&result, &columns).unwrap();
        let rendered = |row: usize, col: usize| {
            tree.children[row].children[col].children[0].children[0]
                .text
                .as_deref()
                .unwrap()
        };
        assert_eq!(rendered(0, 0), "phone");
        assert_eq!(rendered(1, 0), "00123");
        assert_eq!(rendered(1, 1), "乙");
        assert_eq!(rendered(2, 1), "甲");
        assert_eq!(rendered(3, 1), "乙");
        assert!(query_table(&result, &["name".into()]).is_err());
        assert!(
            query_losses(&result)
                .iter()
                .any(|l| l.code == "query_incomplete")
        );
    }
}
