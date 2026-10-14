//! Frozen Office consumers. Source, field, target, output and delivery rights
//! remain separate. A durable request claim never authorizes regeneration.
use super::*;
use std::path::Path;
use weftext_core::SnapshotFile;
use weftext_export::{ExportArtifact, docx, xlsx};
use weftext_knowledge as k;

const TTL: Duration = Duration::from_secs(900);
const MAX_TEMPLATE: usize = 16 * 1024 * 1024;
const MAX_ARTIFACT: usize = 32 * 1024 * 1024;
const MAX_STORE: usize = 64 * 1024 * 1024;

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OfficeRights {
    #[serde(default)]
    pub(super) export: bool,
    #[serde(default)]
    pub(super) download: bool,
    #[serde(default)]
    pub(super) r#import: bool,
}
#[derive(Default)]
pub(super) struct Store {
    jobs: HashMap<(String, String), Job>,
}
#[derive(Clone)]
struct Needs {
    execution: library::ExecutionAuthority,
    sources: BTreeSet<String>,
    fields: BTreeMap<String, BTreeSet<String>>,
    query_authority: Option<query::Authority>,
    review: Option<String>,
    resources: BTreeSet<String>,
    binding: String,
}
#[derive(Clone)]
struct Job {
    actor: String,
    epoch: u64,
    needs: Needs,
    expires: Instant,
    status: &'static str,
    artifact: Option<Arc<ExportArtifact>>,
    preview: Value,
    digest: Option<String>,
    reserved: usize,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Template {
    name: String,
    bytes_base64: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    kind: String,
    text: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    request_id: String,
    resource: Option<String>,
    source: Option<String>,
    format: String,
    review_mode: String,
    include_resolved: bool,
    query_id: Option<String>,
    template: Option<Template>,
    style_policy: String,
    bindings: BTreeMap<String, Binding>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestId {
    request_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Confirm {
    request_id: String,
    digest: String,
    accept_losses: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputChunk {
    request_id: String,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inspect {
    format: String,
    bytes_base64: String,
}
struct Cut {
    worker: Option<WorkerRequest>,
    environment: Option<library::Environment>,
    snapshots: Vec<SnapshotFile>,
    query: Option<fields::FrozenQuery>,
    reviews: Vec<docx::DocxReview>,
    template: Option<Vec<u8>>,
    job: Job,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/office", get(catalog))
        .route("/api/hosted/office/template", post(inspect))
        .route("/api/hosted/office/export/prepare", post(prepare))
        .route("/api/hosted/office/export/status", post(status))
        .route("/api/hosted/office/export/confirm", post(confirm))
        .route("/api/hosted/office/export/chunk", post(chunk))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    inner.office.jobs.retain(|_, j| {
        j.expires > Instant::now()
            && inner
                .policy
                .users
                .get(&j.actor)
                .is_some_and(|u| u.enabled && u.epoch == j.epoch)
    });
}
pub(super) fn binding_hash(
    inner: &HostedInner,
    resources: &BTreeSet<String>,
) -> Result<String, ApiError> {
    inputs::binding_hash(inner, resources)
}
fn failed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "officeUnavailable",
        "原输出未完成或冻结依据已变化；请核对原请求，输入保留。",
    )
}
fn id(value: &str) -> Result<(), ApiError> {
    uuid::Uuid::parse_str(value).map_err(|_| ApiError::bad("输出请求编号无效"))?;
    Ok(())
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:office-output", inner.actor_scope(actor))
}
fn access(inner: &HostedInner, actor: &str, needs: &Needs, download: bool) -> Result<(), ApiError> {
    library::authorize_execution(inner, actor, &needs.execution)?;
    if let Some(authority) = &needs.query_authority {
        query::authorize_authority(inner, actor, authority)?;
    }
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?;
    let style_inputs = needs.execution.inputs.iter().chain(
        needs
            .query_authority
            .iter()
            .flat_map(|q| &q.execution.inputs),
    );
    for r in style_inputs {
        if media::valid_input(&inner.config, r) {
            media::input_access(inner, actor, r, true)?;
            continue;
        }
        let grant = user.grants.get(r).ok_or_else(unavailable)?;
        if !grant.read || !grant.office.export || (download && !grant.office.download) {
            return Err(unavailable());
        }
    }
    for r in &needs.resources {
        if media::valid_input(&inner.config, r) {
            media::input_access(inner, actor, r, true)?;
            continue;
        }
        let grant = user.grants.get(r).ok_or_else(unavailable)?;
        if !grant.office.export
            || (download && !grant.office.download)
            || (needs.sources.contains(r) && !grant.read)
            || needs.fields.get(r).is_some_and(|fs| {
                fs.iter()
                    .any(|f| !grant.fields.get(f).is_some_and(|f| f.read))
            })
            || (needs.review.as_ref() == Some(r)
                && (!grant.annotations.read || !grant.annotations.target))
        {
            return Err(unavailable());
        }
    }
    if binding_hash(inner, &needs.resources)? != needs.binding {
        return Err(failed());
    }
    Ok(())
}
fn snapshot_check(inner: &HostedInner, snapshots: &[SnapshotFile]) -> Result<(), ApiError> {
    for f in snapshots {
        inner
            .core
            .validate_snapshot(&inner.config.root, &f.path, &f.base_id)
            .map_err(|_| failed())?;
    }
    Ok(())
}
fn decode(encoded: &str) -> Result<Vec<u8>, ApiError> {
    if encoded.len() > MAX_TEMPLATE * 4 / 3 + 4 {
        return Err(ApiError::bad("Office 模板上限为 16 MiB"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| ApiError::bad("Office 文件编码无效"))?;
    if bytes.len() > MAX_TEMPLATE {
        return Err(ApiError::bad("Office 模板上限为 16 MiB"));
    }
    Ok(bytes)
}
async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;
        expire_revoked(inner);
        let resources:Vec<_>=inner.policy.users[&actor].grants.iter().filter_map(|(r,g)|{
            let f=inner.config.files.get(r)?;
            (g.read && g.office.export && f.editable).then(||json!({"resource":r,"title":f.title,"export":true,"download":g.office.download}))
        }).collect();
        Ok(Json(json!({"resources":resources,"queries":fields::query_catalog(inner,&actor)})))
    }).await
}
async fn inspect(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Inspect>,
) -> ApiResult {
    let check = auth.clone();
    let (actor, epoch) = locked(&state, move |inner| {
        let a = inner.actor(&check)?;
        Ok((a.clone(), inner.actor_generation(&a)))
    })
    .await?;
    let bytes = decode(&req.bytes_base64)?;
    let _permit = state
        .render_gate
        .try_acquire()
        .map_err(|_| ApiError::bad("Office 转换正在运行，请稍后重试"))?;
    let result = tokio::task::spawn_blocking(move || {
        super::super::export_conversions::template_slots(&req.format, &bytes)
    })
    .await
    .map_err(|_| failed())?;
    locked(&state, move |inner| {
        if inner.actor(&auth)? != actor || inner.actor_generation(&actor) != epoch {
            return Err(expired());
        }
        Ok(Json(result?))
    })
    .await
}
fn validate(req: &Prepare) -> Result<(), ApiError> {
    id(&req.request_id)?;
    if !matches!(req.format.as_str(), "docx" | "xlsx")
        || !matches!(req.review_mode.as_str(), "none" | "comments" | "changes")
        || !matches!(req.style_policy.as_str(), "template" | "source")
        || req.bindings.len() > 128
        || (req.template.is_none() && !req.bindings.is_empty())
        || (req.format == "xlsx"
            && (req.query_id.is_none()
                || req.resource.is_some()
                || req.source.is_some()
                || req.review_mode != "none"
                || req.include_resolved))
        || (req.format == "docx" && req.resource.is_none())
    {
        return Err(ApiError::bad(
            "请选择有效 Office 输出、准确正文或已有查询结果；未使用的参数不能静默忽略",
        ));
    }
    for (slot, b) in &req.bindings {
        if slot.is_empty()
            || slot.len() > 512
            || !matches!(b.kind.as_str(), "body" | "title" | "text" | "query")
            || (b.kind == "text" && b.text.is_none())
            || (b.kind != "text" && b.text.is_some())
            || b.text.as_ref().is_some_and(|t| t.len() > 1024 * 1024)
            || (b.kind == "query" && req.query_id.is_none())
            || (req.format == "xlsx" && matches!(b.kind.as_str(), "body" | "title"))
        {
            return Err(ApiError::bad(
                "模板槽位绑定无效；XLSX 支持明确文本和已显示的查询结果",
            ));
        }
    }
    if req.format == "docx"
        && req.query_id.is_some()
        && !req.bindings.values().any(|b| b.kind == "query")
    {
        return Err(ApiError::bad("DOCX 查询结果须明确绑定到模板槽位"));
    }
    if let Some(s) = &req.source {
        writing_api::validate_source(s, None)?;
    }
    Ok(())
}
fn capture(inner: &mut HostedInner, actor: &str, req: &Prepare) -> Result<Cut, ApiError> {
    let query = req
        .query_id
        .as_ref()
        .map(|q| fields::frozen_query(inner, actor, q))
        .transpose()?;
    let mut sources = BTreeSet::new();
    let mut captured_dependencies = None;
    if let Some(r) = &req.resource {
        let f = inner
            .config
            .files
            .get(r)
            .filter(|f| f.editable)
            .ok_or_else(unavailable)?;
        let selected = f.inputs.clone();
        let captured = inputs::capture(inner, actor, &selected)?;
        sources.insert(r.clone());
        sources.extend(captured.ids().iter().cloned());
        captured_dependencies = Some(captured);
    }
    let fields = query
        .as_ref()
        .map(|q| q.authorities.clone())
        .unwrap_or_default();
    let resources = sources
        .iter()
        .cloned()
        .chain(fields.keys().cloned())
        .collect();
    let mut needs = Needs {
        execution: library::ExecutionAuthority::default(),
        binding: binding_hash(inner, &resources)?,
        resources,
        sources,
        fields,
        query_authority: query.as_ref().map(|q| q.query_authority.clone()),
        review: (req.review_mode != "none").then(|| req.resource.clone().unwrap()),
    };
    access(inner, actor, &needs, false)?;
    let template = req
        .template
        .as_ref()
        .map(|t| decode(&t.bytes_base64))
        .transpose()?;
    let mut snapshots = vec![];
    let mut reviews = vec![];
    let mut preview =
        json!({"format":req.format,"stylePolicy":req.style_policy,"reviewMode":req.review_mode});
    let mut environment = None;
    let worker = if let Some(r) = &req.resource {
        let main = inner.config.root.join(&inner.config.files[r].path);
        let mut paths: Vec<_> = needs
            .sources
            .iter()
            .map(|r| {
                Ok(inner
                    .config
                    .root
                    .join(inputs::path(inner, actor, r)?)
                    .to_string_lossy()
                    .into_owned())
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        if needs.review.is_some() {
            paths.push(
                main.parent()
                    .ok_or_else(unavailable)?
                    .join("weftext.annotations.json")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        snapshots = inner
            .core
            .read_snapshot(&inner.config.root, &paths)
            .map_err(|_| failed())?
            .files;
        let saved = snapshots
            .iter()
            .find(|f| Path::new(&f.path) == main)
            .ok_or_else(failed)?;
        let bytes = saved.bytes.as_deref().ok_or_else(failed)?;
        let source = req
            .source
            .clone()
            .unwrap_or(String::from_utf8(bytes.to_vec()).map_err(|_| failed())?);
        if needs.review.is_some() {
            if source.as_bytes() != bytes {
                return Err(ApiError::bad(
                    "带批注或修订输出需要已保存的准确正文；请先保存并核对",
                ));
            }
            let root = inner.config.root.clone();
            let current = inner
                .core
                .with_actor(&scope(inner, actor), |core| {
                    let doc = core.open(&main, Some(root))?;
                    let result = doc
                        .workspace_id
                        .clone()
                        .zip(doc.node_id.clone())
                        .filter(|(_, n)| Some(n) == saved.node_id.as_ref())
                        .map(|(workspace_id, node_id)| k::NodeSnapshot {
                            owner: k::NodeRef {
                                workspace_id,
                                node_id,
                            },
                            path: saved.path.clone(),
                            library_path: String::new(),
                            revision_id: saved.revision.clone().unwrap_or_default(),
                            source: source.clone(),
                            title: None,
                            aliases: vec![],
                            labels: vec![],
                            labels_complete: false,
                        });
                    core.close_session(&doc.session_id)?;
                    Ok(result)
                })
                .map_err(|_| failed())?
                .ok_or_else(failed)?;
            let sidecar = main.parent().unwrap().join("weftext.annotations.json");
            let annotations = snapshots
                .iter()
                .find(|f| Path::new(&f.path) == sidecar)
                .and_then(|f| f.bytes.as_deref())
                .unwrap_or(b"{\"version\":1,\"annotations\":[]}");
            reviews = freeze_reviews(annotations, &current, req)?;
        }
        let captured = captured_dependencies.take().ok_or_else(failed)?;
        let (worker, dependencies, captured_environment) =
            editor::freeze_with_inputs(inner, actor, r, &source, captured)?;
        needs.execution = captured_environment.authority.clone();
        access(inner, actor, &needs, false)?;
        environment = Some(captured_environment);
        for (p, h) in dependencies {
            if !snapshots.iter().any(|f| {
                Path::new(&f.path) == inner.config.root.join(&p)
                    && f.bytes.as_ref().is_some_and(|b| digest(b) == h)
            }) {
                return Err(failed());
            }
        }
        snapshot_check(inner, &snapshots)?;
        preview["document"] = json!({"resource":r,"sourceHash":digest(source.as_bytes()),"scope":if req.review_mode=="none"{"currentCapturedDraft"}else{"exactSavedSourceAndReviews"}});
        preview["source"] = json!(source);
        Some(worker)
    } else {
        None
    };
    if let Some(q) = &query {
        preview["query"] = json!({"queryId":req.query_id,"scope":"previouslyDisplayedFrozenResult","columns":q.columns,"rowCount":q.result.rows.len(),"coverage":q.result.coverage,"freshness":q.result.freshness,"limited":q.result.limited});
        preview["columns"] = json!(q.columns);
        preview["rows"] = json!(q.result.rows.iter().take(12).collect::<Vec<_>>());
        preview["summary"] =
            json!("预览显示至多 12 行；工件使用全部原查询行及其准确状态，不重新执行查询。");
    }
    let reserved = template.as_ref().map_or(0, Vec::len)
        + environment
            .as_ref()
            .map_or(0, library::Environment::retained_bytes)
        + serde_json::to_vec(&preview).map_or(MAX_STORE, |b| b.len())
        + snapshots
            .iter()
            .map(|s| s.bytes.as_ref().map_or(0, Vec::len))
            .sum::<usize>()
        + req.source.as_ref().map_or(0, String::len)
        + query.as_ref().map_or(0, |q| {
            serde_json::to_vec(&(&q.result, &q.schema)).map_or(MAX_STORE, |b| b.len())
        });
    if reserved > MAX_ARTIFACT {
        return Err(ApiError::bad("本次 Office 输入超过 32 MiB 预算"));
    }
    Ok(Cut {
        worker,
        environment,
        snapshots,
        query,
        reviews,
        template,
        job: Job {
            actor: actor.into(),
            epoch: inner.actor_generation(actor),
            needs,
            expires: Instant::now() + TTL,
            status: "pending",
            artifact: None,
            preview,
            digest: None,
            reserved: reserved.max(1024 * 1024),
        },
    })
}
fn freeze_reviews(
    bytes: &[u8],
    current: &k::NodeSnapshot,
    req: &Prepare,
) -> Result<Vec<docx::DocxReview>, ApiError> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(ApiError::bad("批注超过输出预算"));
    }
    let doc = k::decode_annotations(bytes).map_err(|_| failed())?;
    let mut selected: BTreeSet<_> = doc
        .records
        .iter()
        .filter(|r| r.reply_to.is_none() && (req.include_resolved || r.review_state == "open"))
        .map(|r| r.annotation_id.clone())
        .collect();
    loop {
        let before = selected.len();
        for r in &doc.records {
            if r.reply_to.as_ref().is_some_and(|p| selected.contains(p)) {
                selected.insert(r.annotation_id.clone());
            }
        }
        if before == selected.len() {
            break;
        }
    }
    Ok(doc.records.iter().filter(|r|selected.contains(&r.annotation_id)).map(|r|{
        let owned=r.target.anchor().is_some_and(|a|a.node==current.owner) || matches!(&r.target,k::AnnotationTarget::WholeDocument{node}if node==&current.owner);
        let range=r.target.anchor().filter(|_|owned).and_then(|a|k::resolve_anchor(a,Some(current),&[]).current_range);
        let suggestion=r.suggestion.as_ref().filter(|_|owned);
        let replacement=if req.review_mode=="changes" {suggestion.filter(|s|s.state=="pending" && !s.requires_reconfirmation && s.confirmed_revision==current.revision_id && range.as_ref().is_some_and(|r|k::writable_suggestion_range(&current.source,r))).map(|s|s.replacement.clone())}else{None};
        let mut body=r.body.clone();
        if let Some(s)=suggestion {body.push_str(&format!("\n建议状态：{}；拟替换为：{}",s.state,s.replacement));}
        let downgrade_reason=(req.review_mode=="changes" && r.suggestion.as_ref().is_some_and(|s|s.state=="pending") && replacement.is_none()).then(||"该建议不能按准确文字修订输出；保留获权讨论并报告未定位目标。".into());
        let empty_body_point=range.as_ref().is_some_and(|r|r.from==r.to && r.from==current.source.len() && k::writable_suggestion_range(&current.source,r));
        docx::DocxReview{id:r.annotation_id.clone(),parent:r.reply_to.clone(),author:r.attribution.clone(),date:docx::word_comment_date(&r.created_at),body,resolved:r.review_state=="resolved",range:range.map(|r|(r.from,r.to)),whole_document:matches!(&r.target,k::AnnotationTarget::WholeDocument{node}if node==&current.owner),replacement,downgrade_reason,empty_body_point}
    }).collect())
}
fn convert(req: Prepare, cut: &Cut, render: Option<Value>) -> Result<ExportArtifact, ApiError> {
    use super::super::export_conversions as c;
    let mut artifact = if req.format == "docx" {
        let render: weftext_engine::RenderOutput =
            serde_json::from_value(render.ok_or_else(failed)?).map_err(|_| failed())?;
        let mut bindings = BTreeMap::new();
        for (name, b) in &req.bindings {
            bindings.insert(
                name.clone(),
                match b.kind.as_str() {
                    "body" => docx::DocxBinding::Body,
                    "title" => docx::DocxBinding::Title,
                    "text" => docx::DocxBinding::Text {
                        value: b.text.clone().unwrap(),
                    },
                    "query" => {
                        let q = cut.query.as_ref().ok_or_else(failed)?;
                        docx::DocxBinding::Nodes {
                            nodes: vec![c::query_table(&q.result, &q.columns)?],
                        }
                    }
                    _ => return Err(failed()),
                },
            );
        }
        docx::export_reviewed_docx(
            docx::DocxInput {
                render: &render,
                template: cut.template.as_deref(),
                bindings: &bindings,
                options: docx::DocxOptions {
                    preserve_source_styles: req.style_policy == "source",
                    reject_losses: false,
                },
            },
            &cut.reviews,
        )
        .map_err(|_| failed())?
    } else {
        let q = cut.query.as_ref().ok_or_else(failed)?;
        let mut fields = vec![];
        let mut queries = vec![];
        for (anchor, b) in &req.bindings {
            if b.kind == "query" {
                queries.push(xlsx::XlsxQuerySlot {
                    anchor: anchor.clone(),
                    sheet: 0,
                });
            } else {
                fields.push(xlsx::XlsxFieldSlot {
                    anchor: anchor.clone(),
                    cell: weftext_data::Cell::value(weftext_data::NativeValue::Text(
                        b.text.clone().ok_or_else(failed)?,
                    )),
                });
            }
        }
        let request = xlsx::XlsxRequest {
            sheets: vec![xlsx::XlsxSheetInput {
                name: "资料".into(),
                columns: q.columns.clone(),
                result: q.result.clone(),
                schema: Some(
                    serde_json::to_value(query_providers::delivery_schema(&q.schema))
                        .map_err(|_| failed())?,
                ),
            }],
            template: cut.template.as_ref().map(|bytes| xlsx::XlsxTemplate {
                bytes: bytes.clone(),
                fields,
                queries,
            }),
        };
        let mut a = xlsx::export_xlsx(&request).map_err(|_| failed())?;
        if req.style_policy == "source" {
            a.losses.push(c::report(
                "xlsx_source_style_unmapped",
                "XLSX 输出冻结字段数据；原生正文排版样式没有映射为 Excel 样式。",
            ));
        }
        a
    };
    if let Some(q) = &cut.query {
        artifact.losses.extend(c::query_losses(&q.result));
    }
    if artifact.bytes.len() > MAX_ARTIFACT {
        return Err(ApiError::bad("Office 工件超过 32 MiB 预算"));
    }
    Ok(artifact)
}
fn response(request: &str, job: &Job) -> Value {
    let Some(a) = &job.artifact else {
        return json!({"requestId":request,"status":job.status});
    };
    json!({"requestId":request,"status":job.status,"filename":format!("weftext.{}",a.extension),"contentType":a.content_type,"length":a.bytes.len(),"digest":job.digest,"losses":a.losses,"summary":a.summary,"preview":job.preview})
}
fn lookup(
    inner: &mut HostedInner,
    actor: &str,
    request: &str,
    download: bool,
) -> Result<Option<Job>, ApiError> {
    id(request)?;
    expire_revoked(inner);
    let Some(job) = inner
        .office
        .jobs
        .get(&(actor.into(), request.into()))
        .cloned()
    else {
        return Ok(None);
    };
    access(inner, actor, &job.needs, download)?;
    Ok(Some(job))
}
fn missing(inner: &mut HostedInner, actor: &str, request: &str) -> Result<Value, ApiError> {
    let exists = inner
        .core
        .with_actor(&scope(inner, actor), |core| {
            core.actor_output_fingerprint(request)
        })
        .map_err(HostedInner::core_error)?
        .is_some();
    Ok(json!({"requestId":request,"status":if exists{"expired"}else{"notRegistered"}}))
}
// Cancelling an HTTP request or a compiler panic must not leave a job that can
// later be mistaken for a prepared artifact. Core's claim stays irrevocable.
struct PendingGuard {
    state: HostedState,
    key: (String, String),
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.state.inner.lock()
            && let Some(job) = inner.office.jobs.get_mut(&self.key)
            && job.status == "pending"
        {
            job.status = "failed";
            job.reserved = 0;
            job.preview = Value::Null;
        }
    }
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    validate(&req)?;
    let fingerprint = digest(&serde_json::to_vec(&req).map_err(|_| failed())?);
    let check = auth.clone();
    let captured = req.clone();
    let (actor, cut, existing) = locked(&state, move |inner| {
        let actor = inner.actor(&check)?;
        expire_revoked(inner);
        let old = inner
            .core
            .with_actor(&scope(inner, &actor), |core| {
                core.actor_output_fingerprint(&captured.request_id)
            })
            .map_err(HostedInner::core_error)?;
        if let Some(old) = old {
            if old != fingerprint {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "operationConflict",
                    "同一输出编号不能用于不同内容",
                ));
            }
            let answer = if let Some(job) = lookup(inner, &actor, &captured.request_id, false)? {
                response(&captured.request_id, &job)
            } else {
                json!({"requestId":captured.request_id,"status":"expired"})
            };
            return Ok((actor, None, Some(answer)));
        }
        let cut = capture(inner, &actor, &captured)?;
        if inner.office.jobs.len() >= 32
            || inner
                .office
                .jobs
                .values()
                .filter(|j| j.actor == actor)
                .count()
                >= 8
            || inner
                .office
                .jobs
                .values()
                .map(|j| j.reserved)
                .sum::<usize>()
                .saturating_add(cut.job.reserved)
                > MAX_STORE
        {
            return Err(ApiError::bad("Office 工件缓存已满，请稍后重试"));
        }
        if !inner
            .core
            .with_actor(&scope(inner, &actor), |core| {
                core.actor_output_claim(&captured.request_id, &fingerprint)
            })
            .map_err(HostedInner::core_error)?
        {
            return Err(failed());
        }
        inner.office.jobs.insert(
            (actor.clone(), captured.request_id.clone()),
            cut.job.clone(),
        );
        Ok((actor, Some(cut), None))
    })
    .await?;
    if let Some(answer) = existing {
        return Ok(Json(answer));
    }
    let key = (actor.clone(), req.request_id.clone());
    let _guard = PendingGuard {
        state: state.clone(),
        key: key.clone(),
    };
    let mut cut = cut.ok_or_else(failed)?;
    let render = if let Some(worker) = cut.worker.take() {
        Some(editor::compile_frozen(&state, worker).await?)
    } else {
        None
    };
    let snapshots = std::mem::take(&mut cut.snapshots);
    let environment = cut.environment.take();
    let converted = tokio::task::spawn_blocking(move || convert(req, &cut, render))
        .await
        .map_err(|_| failed())?;
    locked(&state, move |inner| {
        if inner.actor(&auth)? != actor {
            return Err(expired());
        }
        let mut job = lookup(inner, &actor, &key.1, false)?
            .filter(|j| j.status == "pending")
            .ok_or_else(failed)?;
        snapshot_check(inner, &snapshots)?;
        if let Some(environment) = &environment {
            environment.recheck(inner, &actor)?;
        }
        let artifact = converted?;
        let used = inner
            .office
            .jobs
            .iter()
            .filter(|(k, _)| *k != &key)
            .map(|(_, j)| j.reserved)
            .sum::<usize>();
        let retained =
            artifact.bytes.len() + serde_json::to_vec(&job.preview).map_or(MAX_STORE, |b| b.len());
        if used.saturating_add(retained) > MAX_STORE {
            return Err(failed());
        }
        job.reserved = retained;
        job.digest = Some(digest(&artifact.bytes));
        job.artifact = Some(Arc::new(artifact));
        job.status = "prepared";
        let answer = response(&key.1, &job);
        inner.office.jobs.insert(key, job);
        Ok(Json(answer))
    })
    .await
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<RequestId>,
) -> ApiResult {
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        Ok(Json(match lookup(inner, &actor, &req.request_id, false)? {
            Some(job) => response(&req.request_id, &job),
            None => missing(inner, &actor, &req.request_id)?,
        }))
    })
    .await
}
async fn confirm(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Confirm>,
) -> ApiResult {
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let Some(mut job) = lookup(inner, &actor, &req.request_id, true)? else {
            return Ok(Json(missing(inner, &actor, &req.request_id)?));
        };
        let artifact = job.artifact.as_ref().ok_or_else(failed)?;
        if !matches!(job.status, "prepared" | "confirmed")
            || job.digest.as_deref() != Some(&req.digest)
            || (!artifact.losses.is_empty() && !req.accept_losses)
        {
            return Err(ApiError::bad("请确认这份准确工件摘要及其转换损失"));
        }
        job.status = "confirmed";
        let answer = response(&req.request_id, &job);
        inner.office.jobs.insert((actor, req.request_id), job);
        Ok(Json(answer))
    })
    .await
}
async fn chunk(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<OutputChunk>,
) -> ApiResult {
    locked(&state,move|inner|{
        let actor=inner.actor(&auth)?;
        let job=lookup(inner,&actor,&req.request_id,true)?.filter(|j|j.status=="confirmed").ok_or_else(failed)?;
        let artifact=job.artifact.ok_or_else(failed)?;
        if req.offset>artifact.bytes.len() {return Err(ApiError::bad("下载偏移超出原工件"));}
        let end=req.offset.saturating_add(64*1024).min(artifact.bytes.len());let bytes=&artifact.bytes[req.offset..end];
        Ok(Json(json!({"requestId":req.request_id,"bytesBase64":base64::engine::general_purpose::STANDARD.encode(bytes),"offset":req.offset,"length":bytes.len(),"total":artifact.bytes.len(),"sha256":job.digest,"done":end==artifact.bytes.len()})))
    }).await
}
