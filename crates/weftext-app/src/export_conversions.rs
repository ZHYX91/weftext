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

fn unavailable(message: impl Into<String>) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "conversionUnavailable",
        message,
    )
}
fn convert_error(error: impl std::fmt::Display) -> ApiError {
    unavailable(error.to_string())
}
pub(crate) fn report(code: &str, message: &str) -> ExportLoss {
    ExportLoss {
        code: code.into(),
        message: message.into(),
        location: None,
    }
}
fn decode(value: &str) -> Result<Vec<u8>, ApiError> {
    if value.len() > 64 * 1024 * 1024 * 4 / 3 + 4 {
        return Err(ApiError::bad("模板超过64 MiB"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| ApiError::bad("模板编码无效"))?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(ApiError::bad("模板超过64 MiB"));
    }
    Ok(bytes)
}

pub(crate) fn template_slots(format: &str, bytes: &[u8]) -> Result<Value, ApiError> {
    let slots=match format {
        "docx"=>weftext_export::docx::list_template_controls(bytes).map_err(convert_error)?.into_iter().map(|c|{
            let supported=c.placement!="unsupported";
            json!({"name":c.tag,"kind":"content","label":c.alias.unwrap_or_else(||c.tag.clone()),"supported":supported,
                "placement":c.placement,"reason":if supported{None}else{Some("内容控件的结构位置暂不支持")}})
        }).collect::<Vec<_>>(),
        "xlsx"=>weftext_export::xlsx::inspect_xlsx_template(bytes).map_err(convert_error)?.anchors.into_iter().map(|a|{
            let kind=if a.kind=="table"{"table"}else if a.rows==1&&a.columns==1{"cell"}else{"range"};
            let label=if let Some(sheet)=&a.sheet {format!("{} · {} · {}列/{}数据行",a.name,sheet,a.columns,a.capacity)}else{a.name.clone()};
            json!({"name":a.name,"kind":kind,"label":label,"supported":a.supported,"reason":a.reason,"rows":a.rows,"columns":a.columns,"capacity":a.capacity})
        }).collect::<Vec<_>>(),
        _=>return Err(ApiError::bad("该输出格式不支持Office模板")),
    };
    Ok(json!({"slots":slots,"warnings":[]}))
}

struct DocumentCut {
    files: BTreeMap<String, Vec<u8>>,
    style: Option<StyleModule>,
    schema: Schema,
    schema_json: Value,
    cut_id: String,
    workspace_id: Option<String>,
    owner_id: Option<String>,
    citations: Option<super::citation_api::FrozenCitations>,
    warnings: Vec<String>,
    complete: bool,
}
fn relative(root: &Path, path: &Path) -> Result<String, CoreError> {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .map_err(|_| CoreError::new("outsideRoot", "Export source is outside its frozen root"))
}
fn capture(
    core: &mut Core,
    loc: &Location,
    req: &PrepareRequest,
) -> Result<DocumentCut, CoreError> {
    core.check_session_base(&req.session_id, &req.base_id)?;
    let inventory = core.inventory(&loc.root)?;
    let key = relative(&loc.root, &loc.path)?;
    if inventory.workspace_id.is_some() {
        let mut frozen = core.admitted_compile_snapshot(&loc.root, "live")?;
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
            CoreError::new("schemaUnavailable", "Frozen schema role is unavailable")
        })?;
        let (schema, _) = super::data_api::knowledge_schema_from_snapshot(schema_file)
            .map_err(|e| CoreError::new("schemaUnavailable", e.message))?;
        let schema_json = serde_json::to_value(&schema)?;
        let citations = super::citation_api::freeze_citations(&mut frozen, &req.source, &loc.path)?;
        core.check_session_base(&req.session_id, &req.base_id)?;
        Ok(DocumentCut {
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
        if inventory.status != "ordinary" {
            return Err(CoreError::new(
                "unadmittedLibrary",
                "Incomplete or unadmitted library cannot fall back to ordinary export",
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
                "Managed library control files exist without qualified admission; choose an ordinary source root instead",
            ));
        }
        if weftext_engine::inspect_citation_declaration(&req.source)
            .map_err(|e| CoreError::new("citationUnavailable", e.to_string()))?
            .is_some()
        {
            return Err(CoreError::new(
                "citationUnavailable",
                "Stable citation Work declarations need a managed frozen library cut",
            ));
        }
        files.insert(key, req.source.as_bytes().to_vec());
        let schema =
            Schema::bundled().map_err(|e| CoreError::new("schemaUnavailable", e.to_string()))?;
        let schema_json = serde_json::to_value(&schema)?;
        core.check_session_base(&req.session_id, &req.base_id)?;
        Ok(DocumentCut {
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

pub(crate) async fn generate(
    state: &AppState,
    req: &PrepareRequest,
) -> Result<(ExportArtifact, Value), ApiError> {
    let loc = location(state, &req.session_id)?;
    super::writing_api::observe_snapshot(state, &req.session_id, &req.source, req.serial, None)?;
    let sid = req.session_id.clone();
    let base = req.base_id.clone();
    core_call(state, move |core| core.check_session_base(&sid, &base)).await?;
    let mut preview = json!({"format":req.format,"document":{"sessionId":req.session_id,"baseId":req.base_id,"serial":req.serial,"sourceHash":weftext_engine::source_hash(&req.source),"scope":"currentDraft"},"stylePolicy":req.style_policy});
    if !req.embedded_queries.is_empty() && !matches!(req.format.as_str(), "docx" | "pdf") {
        return Err(ApiError::bad("正文查询结果只适用于 DOCX/PDF 随文输出"));
    }
    if !matches!(req.format.as_str(), "docx" | "xlsx")
        && (req.template.is_some() || !req.bindings.is_empty())
    {
        return Err(ApiError::bad(
            "该格式不支持模板或槽位绑定；没有忽略上传内容",
        ));
    }
    if req.template.is_none() && !req.bindings.is_empty() {
        return Err(ApiError::bad("没有模板时不能传入槽位绑定"));
    }
    let (mut artifact, more) = match req.format.as_str() {
        "source" => (
            ExportArtifact {
                bytes: req.source.as_bytes().to_vec(),
                content_type: "text/plain; charset=utf-8".into(),
                extension: "typ".into(),
                losses: vec![],
                summary: "当前草稿的准确原生Typst字节；不包含外部依赖或库内其他资料".into(),
            },
            json!({"source":req.source,"sourceBytes":req.source.len()}),
        ),
        "native" => native(state, req, &loc).await?,
        "pdf" => pdf(state, req, &loc).await?,
        "docx" | "xlsx" | "bibtex" | "cslJson" | "csl-json" | "ris" => {
            let embedded = super::embedded_output::freeze(state, req, &loc.root.to_string_lossy())?;
            preview["embeddedQueries"] = super::embedded_output::preview(&embedded);
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
            tokio::task::spawn_blocking(move || {
                let projection = weftext_data::project(&req_owned.source, &cut.schema);
                let template = req_owned
                    .template
                    .as_ref()
                    .map(|t| decode(&t.bytes_base64))
                    .transpose()?;
                let render = render
                    .map(|output| {
                        serde_json::from_value::<RenderOutput>(output)
                            .map_err(|_| ApiError::internal("冻结HTML编译合同无效"))
                    })
                    .transpose()?;
                let frozen_result = query
                    .as_ref()
                    .map(|q| {
                        serde_json::from_value::<QueryResult>(q.value.clone())
                            .map_err(|_| ApiError::internal("冻结Query合同不可读取"))
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
                        &embedded,
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
            .map_err(|_| ApiError::internal("输出转换没有完成"))??
        }
        _ => return Err(ApiError::bad("未知输出格式")),
    };
    if !matches!(req.format.as_str(), "docx" | "xlsx") && req.query_id.is_some() {
        preview["query"] = json!({"queryId":req.query_id,"included":false,"scope":"outsideSelectedFormat","reason":"该格式使用当前草稿、已保存整库或明确文献Work"});
        artifact.losses.push(report(
            "query_not_in_format_scope",
            "该格式输出当前草稿、已保存整库或当前文献Work；本次显示的Query结果不在其内容范围。",
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
        let output: weftext_engine::PagedOutput =
            serde_json::from_value(value).map_err(|_| ApiError::internal("冻结分页合同无效"))?;
        if !output.ok {
            return Err(unavailable(format!(
                "PDF 编译失败：{}",
                output
                    .diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("；")
            )));
        }
        Ok(output)
    };
    let original = compile(render_frozen_target_worker(state, input.clone(), true).await?)?;
    let (source, losses) =
        super::embedded_output::paged_source(&req.source, &embedded, &original.embedded_queries)?;
    let output = if source == req.source {
        original
    } else {
        input.source = source;
        let key = relative(&loc.root, &loc.path).map_err(ApiError::from)?;
        input.files.insert(key, input.source.as_bytes().to_vec());
        compile(render_frozen_target_worker(state, input, true).await?)?
    };
    let bytes = decode(
        output
            .pdf_base64
            .as_deref()
            .ok_or_else(|| unavailable("PDF 没有生成文件"))?,
    )?;
    let preview = json!({"target":"paged","scope":"frozenDraftWithSelectedQueryResults","cutId":cut.cut_id,"complete":cut.complete,"warnings":cut.warnings,"pageCount":output.page_count,"embeddedQueries":super::embedded_output::preview(&embedded),"diagnostics":output.diagnostics});
    Ok((
        ExportArtifact {
            bytes,
            content_type: "application/pdf".into(),
            extension: "pdf".into(),
            losses,
            summary: format!(
                "原生 PDF：{} 页，保留已选择正文查询的冻结行和原运行状态",
                output.page_count
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
        .map_err(|_| ApiError::internal("文档会话不可用"))?
        .clone();
    let versions = state
        .editor_versions
        .lock()
        .map_err(|_| ApiError::internal("编辑会话不可用"))?
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
    let (bytes,package)=core_call(state,move|core|{
        core.check_session_base(&request.session_id,&request.base_id)?;
        let bytes=core.export_native_package(&location.root)?;
        let package=weftext_export::exchange::read_native_package(&bytes).map_err(|e|CoreError::new(&e.code,e.message))?;
        let key=relative(&location.root,&location.path)?;
        if package.files.get(&key).map(Vec::as_slice)!=Some(request.source.as_bytes()){return Err(CoreError::new("dirtyDraft","Native package exports the saved whole library; save the current draft first"));}
        for(path,source)in drafts{let key=relative(&location.root,&path)?;if package.files.get(&key).is_some_and(|saved|saved.as_slice()!=source.as_bytes()){return Err(CoreError::new("dirtyDraft","Another open library document has an unsaved draft; save it before whole-library export"));}}
        core.check_session_base(&request.session_id,&request.base_id)?;Ok((bytes,package))
    }).await?;
    let losses=package.manifest.losses.iter().map(|l|ExportLoss{code:l.code.clone(),message:l.detail.clone(),location:Some(l.path.clone())}).chain(std::iter::once(report("native_not_recovery_backup","该原生资料包包含已保存的整库纳管内容与可交付依赖；不包含本机历史、执行日志或未登记原始文件，不能替代恢复备份。"))).collect();
    let preview = json!({"scope":"savedWholeLibrary","workspaceId":package.manifest.workspace_id,"fileCount":package.files.len(),"files":package.manifest.entries,"summary":"已保存的整个资料库；不是仅当前笔记"});
    Ok((
        ExportArtifact {
            bytes,
            content_type: "application/zip".into(),
            extension: "weftext.zip".into(),
            losses,
            summary: format!(
                "已保存的整个资料库：{}个纳管资料/依赖文件；不是仅当前笔记，也不是恢复备份",
                package.files.len()
            ),
        },
        preview,
    ))
}

fn field_cell(projection: &NoteProjection, name: &str) -> Result<Cell, ApiError> {
    if projection.qualification != "qualified" {
        return Err(unavailable("当前草稿的typed属性投影不可用"));
    }
    let matches = projection
        .fields
        .iter()
        .filter(|f| f.key == name || f.field_id == name)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(unavailable(format!(
            "属性‘{name}’不在当前草稿的唯一typed字段合同中"
        )));
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
            NativeValue::None => "〈null〉".into(),
            NativeValue::Bool(b) => b.to_string(),
            NativeValue::Array(_) | NativeValue::Record(_) => {
                serde_json::to_string_pretty(value)
                    .map_err(|e| ApiError::internal(e.to_string()))?
            }
        },
        Cell::Absent => "〈缺值 absent〉".into(),
        Cell::Null => "〈null〉".into(),
        Cell::Invalid { diagnostics, value } => format!(
            "〈invalid：{}〉{}",
            diagnostics.join("；"),
            value
                .as_ref()
                .map(|v| cell_text(&Cell::value(v.clone())))
                .transpose()?
                .map(|s| format!("\n{s}"))
                .unwrap_or_default()
        ),
        Cell::Unavailable { reason } => format!("〈unavailable：{reason}〉"),
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
            binding
                .field
                .as_deref()
                .ok_or_else(|| ApiError::bad("属性绑定缺少字段"))?,
        ),
        "text" => Ok(Cell::value(NativeValue::Text(
            binding
                .text
                .clone()
                .ok_or_else(|| ApiError::bad("固定文字绑定缺少内容"))?,
        ))),
        "title" => render
            .and_then(|r| r.products.as_ref())
            .and_then(|p| p.title.clone())
            .map(|s| Cell::value(NativeValue::Text(s)))
            .ok_or_else(|| unavailable("同次冻结HTML产品没有唯一笔记标题；没有从文件名发明标题")),
        _ => Err(ApiError::bad("单元格槽位仅支持typed属性、标题或固定文字")),
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
        return Err(unavailable("DOCX Query表格超过行列预算；没有截断行"));
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
            "DOCX列顺序遗漏冻结Query单元格或重复列；没有静默丢弃内容",
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
        "DOCX中的Query表保留列顺序、行顺序、精确文字与各Cell状态；机器重入需要原生/完整XLSX结构输出。",
    )];
    if result.coverage != "complete" || result.limited || !result.aggregate_exact {
        losses.push(report("query_incomplete","导出的是已显示的冻结Query行；部分覆盖、显示上限或非精确聚合保持原状态，不是完整精确总数。"));
    }
    losses
}
fn docx(
    req: &PrepareRequest,
    mut render: RenderOutput,
    projection: &NoteProjection,
    template: Option<Vec<u8>>,
    query: Option<&QueryResult>,
    columns: Option<&[String]>,
    embedded: &[super::embedded_output::FrozenEmbedded],
) -> Result<(ExportArtifact, Value), ApiError> {
    let mut bindings = BTreeMap::new();
    let mut losses = vec![];
    let mut query_bound = false;
    if template.is_none() || req.bindings.values().any(|binding| binding.kind == "body") {
        if let Some(tree) = render.tree.as_mut() {
            losses.extend(super::embedded_output::materialize(tree, embedded)?);
        }
    } else if !embedded.is_empty() {
        losses.push(report(
            "embedded_query_body_not_selected",
            "本次模板未绑定正文；已选择的正文查询结果不在输出范围。",
        ));
    }
    for (name, binding) in &req.bindings {
        let mapped = match binding.kind.as_str() {
            "body" => DocxBinding::Body,
            "title" => DocxBinding::Title,
            "text" => DocxBinding::Text {
                value: binding
                    .text
                    .clone()
                    .ok_or_else(|| ApiError::bad("固定文字缺失"))?,
            },
            "field" => {
                let cell = field_cell(
                    projection,
                    binding
                        .field
                        .as_deref()
                        .ok_or_else(|| ApiError::bad("属性字段缺失"))?,
                )?;
                losses.push(report("docx_field_structure","属性槽位保留精确可读文字与状态；DOCX不携带完整typed字段结构，重入请使用原生或XLSX输出。"));
                DocxBinding::Text {
                    value: cell_text(&cell)?,
                }
            }
            "query" => {
                let query = query.ok_or_else(|| ApiError::bad("Query绑定缺少冻结结果标识"))?;
                query_bound = true;
                DocxBinding::Nodes {
                    nodes: vec![query_table(query, columns.unwrap())?],
                }
            }
            _ => return Err(ApiError::bad(format!("未知DOCX绑定类型：{}", binding.kind))),
        };
        bindings.insert(name.clone(), mapped);
    }
    if let Some(query) = query {
        if template.is_none() {
            let tree = render
                .tree
                .take()
                .ok_or_else(|| unavailable("冻结HTML编译缺少正文树"))?;
            render.tree = Some(node(
                "div",
                "document-with-frozen-query".into(),
                vec![tree, query_table(query, columns.unwrap())?],
            ));
            query_bound = true;
        }
        if !query_bound {
            return Err(ApiError::bad(
                "已选择Query结果，但模板没有Query槽位绑定；请绑定或明确去除Query输出选择",
            ));
        }
        losses.extend(query_losses(query));
    }
    let artifact = weftext_export::docx::export_docx(DocxInput {
        render: &render,
        template: template.as_deref(),
        bindings: &bindings,
        options: DocxOptions {
            preserve_source_styles: req.style_policy == "source",
            reject_losses: false,
        },
    })
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
            "当前笔记没有可导出的已限定typed字段；请选择冻结Query或原生源码输出",
        ));
    }
    if cut.owner_id.is_some() && projection.id != cut.owner_id {
        return Err(unavailable(
            "草稿中的笔记 ID 与已纳管身份不一致；不能将其字段导出为原笔记",
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
                diagnostics: vec![format!("Unknown field preserved; source: {}", field.raw)],
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
        return Err(ApiError::bad("Query区域绑定需要实际缓存Query结果标识"));
    }
    let request = XlsxRequest {
        sheets: result_and_schema
            .as_ref()
            .map(|(result, columns, schema)| {
                vec![XlsxSheetInput {
                    name: "资料".into(),
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
        artifact.losses.push(report("xlsx_source_style_unmapped","XLSX只映射typed数据；原生Typst排版样式未映射为Excel单元格样式，当前模板/工作簿样式保留。"));
    }
    let preview = if let Some((result, columns, _)) = result_and_schema {
        json!({"sheets":[{"name":"资料","columns":columns,"rowCount":result.rows.len(),"coverage":result.coverage,"freshness":result.freshness,"limited":result.limited}],"rows":result.rows.iter().take(12).collect::<Vec<_>>(),"previewRowsShown":result.rows.len().min(12),"summary":"预览仅显示前12行；工件包含所有冻结行，超预算明确拒绝"})
    } else {
        json!({"scope":"typedTemplateFields","fields":request.template.as_ref().map(|t|&t.fields),"summary":"仅填入明确选择的单元格字段；精确Cell状态在结构附表"})
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
        _ => return Err(ApiError::bad("未知文献格式")),
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
            return Err(unavailable("当前笔记不是typed文献Work，且没有明确文献声明"));
        }
        let owner = cut
            .owner_id
            .as_ref()
            .filter(|id| projection.id.as_ref() == Some(id))
            .ok_or_else(|| unavailable("当前文献没有同次纳管Work身份"))?;
        let workspace = cut
            .workspace_id
            .as_ref()
            .ok_or_else(|| unavailable("文献输出需要纳管库身份"))?;
        let publication = projection
            .fields
            .iter()
            .find(|f| f.field_id == "library/publication")
            .and_then(|f| f.cell.native())
            .cloned()
            .ok_or_else(|| unavailable("文献publication字段缺席、无效或不可用"))?;
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
            summary: format!(
                "{}篇按明确Work身份冻结的文献；没有按DOI、标题或建议键猜测关联",
                works.len()
            ),
        },
        json!({"scope":"frozenCitationWorks","works":works,"dependencies":projection.dependencies}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readable_typed_slots_keep_exact_text_and_distinct_states() {
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
                diagnostics: vec!["bad".into()],
                value: None,
            },
            Cell::Unavailable {
                reason: "missing".into(),
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
