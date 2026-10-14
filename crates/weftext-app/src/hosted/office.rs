//! Frozen Office consumers. Source, field, target, output and delivery rights
//! remain separate. A durable request claim never authorizes regeneration.
use super::*;
use std::path::Path;
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
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FormatRights {
    #[serde(default)]
    pub(super) export: bool,
    #[serde(default)]
    pub(super) download: bool,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OutputRights {
    #[serde(default)]
    pub(super) pdf: FormatRights,
    #[serde(default)]
    pub(super) bibliography: FormatRights,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct NativeRights {
    #[serde(default)]
    pub(super) read: bool,
    #[serde(default)]
    pub(super) export: bool,
    #[serde(default)]
    pub(super) download: bool,
}
pub(super) fn valid_native_rights(r: &NativeRights) -> bool {
    (!r.export || r.read) && (!r.download || (r.read && r.export))
}
#[derive(Default)]
pub(super) struct Store {
    jobs: HashMap<(String, String), Job>,
}
#[derive(Clone)]
struct Needs {
    format: String,
    citations: citations::Authority,
    embedded_authorities: Vec<query::Authority>,
    native_binding: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    field: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    serial: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bibliography_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    works: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    embedded_queries: Vec<export_api::EmbeddedQuerySelection>,
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
    additional_losses: Vec<weftext_export::ExportLoss>,
    field_cells: BTreeMap<String, weftext_data::Cell>,
    citations: Option<citations::Capture>,
    native_witness: Option<Arc<weftext_core::NativeExportWitness>>,
    artifact: Option<ExportArtifact>,
    embedded: Vec<super::super::embedded_output::FrozenEmbedded>,
    witnesses: Vec<(String, weftext_core::SnapshotWitness)>,
    worker: Option<WorkerRequest>,
    environment: Option<library::Environment>,
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
        .route("/api/hosted/export", get(catalog))
        .route("/api/hosted/export/prepare", post(prepare))
        .route("/api/hosted/export/status", post(status))
        .route("/api/hosted/export/confirm", post(confirm))
        .route("/api/hosted/export/chunk", post(chunk))
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
fn native_binding(inner: &HostedInner) -> Result<String, ApiError> {
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"deployment":inner.config.deployment,"enabled":inner.config.native_whole_library_export})).map_err(|_|failed())?))
}
fn format_access(grant: &Rights, format: &str, download: bool) -> bool {
    match format {
        "pdf" => grant.outputs.pdf.export && (!download || grant.outputs.pdf.download),
        "bibtex" | "csl-json" | "ris" => {
            grant.outputs.bibliography.export && (!download || grant.outputs.bibliography.download)
        }
        _ => grant.office.export && (!download || grant.office.download),
    }
}
fn access(inner: &HostedInner, actor: &str, needs: &Needs, download: bool) -> Result<(), ApiError> {
    library::authorize_execution(inner, actor, &needs.execution)?;
    citations::authorize(inner, actor, &needs.citations, true, download)?;
    citations::authorize(inner, actor, &needs.execution.citations, true, download)?;
    if let Some(authority) = &needs.query_authority {
        query::authorize_authority(inner, actor, authority)?;
        citations::authorize(inner, actor, &authority.execution.citations, true, download)?;
    }
    for authority in &needs.embedded_authorities {
        query::authorize_authority(inner, actor, authority)?;
        citations::authorize(inner, actor, &authority.execution.citations, true, download)?;
    }
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?;
    if let Some(binding) = &needs.native_binding {
        let r = &user.native_whole_library;
        if !inner.config.native_whole_library_export
            || !r.read
            || !r.export
            || (download && !r.download)
        {
            return Err(unavailable());
        }
        if native_binding(inner)? != *binding {
            return Err(failed());
        }
    }
    let style_inputs = needs
        .execution
        .inputs
        .iter()
        .chain(
            needs
                .query_authority
                .iter()
                .flat_map(|q| &q.execution.inputs),
        )
        .chain(
            needs
                .embedded_authorities
                .iter()
                .flat_map(|q| &q.execution.inputs),
        );
    for r in style_inputs {
        if media::valid_input(&inner.config, r) {
            media::input_access(inner, actor, r, true)?;
            continue;
        }
        let grant = &inner.grant(actor, r)?;
        if !grant.read || !format_access(grant, &needs.format, download) {
            return Err(unavailable());
        }
    }
    for r in &needs.resources {
        if media::valid_input(&inner.config, r) {
            media::input_access(inner, actor, r, true)?;
            continue;
        }
        let grant = &inner.grant(actor, r)?;
        if !format_access(grant, &needs.format, download)
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
fn witness_check(
    inner: &HostedInner,
    witnesses: &[(String, weftext_core::SnapshotWitness)],
) -> Result<(), ApiError> {
    for (path, w) in witnesses {
        inner
            .core
            .validate_snapshot_witness(&inner.config.root, path, w)
            .map_err(|_| failed())?;
    }
    Ok(())
}
fn check_draft(
    inner: &mut HostedInner,
    actor: &str,
    req: &Prepare,
    source: &str,
) -> Result<(), ApiError> {
    let required = req.format == "pdf" || !req.embedded_queries.is_empty();
    let any = req.session_id.is_some()
        || req.base_id.is_some()
        || req.serial.is_some()
        || req.source_hash.is_some();
    if !required && !any {
        return Ok(());
    }
    let session = req.session_id.as_ref().ok_or_else(|| {
        ApiError::bad("准确草稿输出需要 sessionId、baseId、source、serial 与 sourceHash")
    })?;
    let opened = inner
        .sessions
        .get(session)
        .filter(|s| {
            s.actor == actor
                && s.generation == inner.actor_generation(actor)
                && Some(&s.resource) == req.resource.as_ref()
        })
        .ok_or_else(unavailable)?;
    inner.rights(actor, &opened.resource)?;
    writing_api::validate_source(source, Some(req.source_hash.as_deref().ok_or_else(failed)?))?;
    writing_api::exact_version(
        &inner.editor_versions,
        session,
        source,
        req.serial.ok_or_else(failed)?,
    )?;
    let actor_scope = inner.actor_scope(actor);
    let base = req.base_id.as_deref().ok_or_else(failed)?;
    inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
            core.check_session_base(session, base)
        })
        .map_err(|_| failed())?;
    Ok(())
}
fn freeze_fields(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source: &str,
    req: &Prepare,
    witnesses: &mut Vec<(String, weftext_core::SnapshotWitness)>,
) -> Result<BTreeMap<String, weftext_data::Cell>, ApiError> {
    let ids = req
        .bindings
        .values()
        .filter_map(|b| b.field.clone())
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rights = inner.grant(actor, resource)?;
    if ids
        .iter()
        .any(|id| !rights.fields.get(id).is_some_and(|f| f.read))
    {
        return Err(unavailable());
    }
    let config = inner
        .core
        .read_config(&inner.config.root, weftext_core::ConfigRole::Schema)
        .map_err(|_| failed())?;
    let witness = inner
        .core
        .take_snapshot_witness(&config.base_id)
        .map_err(|_| failed())?;
    if config.bytes.is_some() && config.revision.is_none() {
        return Err(failed());
    }
    let schema = match &config.bytes {
        Some(b) => {
            weftext_data::Schema::from_author_json(std::str::from_utf8(b).map_err(|_| failed())?)
        }
        None => weftext_data::Schema::bundled(),
    }
    .map_err(|_| failed())?;
    let profile =
        weftext_data::narrow_profile(source, &schema, &ids, None).map_err(|_| failed())?;
    let file = author_file(inner, resource)?;
    let qualified = inner
        .core
        .qualified_node_at(&inner.config.root, &inner.config.root.join(file.path))
        .map_err(|_| failed())?;
    if req.source.is_none() && qualified.revision.is_none() {
        return Err(failed());
    }
    if profile.node_id != qualified.owner.node_id {
        return Err(failed());
    }
    witnesses.push((config.path, witness));
    witnesses.push((
        inner
            .config
            .root
            .join(&inner.file(resource)?.path)
            .to_string_lossy()
            .into_owned(),
        qualified.witness,
    ));
    Ok(profile
        .fields
        .into_iter()
        .map(|f| (f.field_id, f.cell))
        .collect())
}
fn freeze_embedded(
    inner: &HostedInner,
    actor: &str,
    req: &Prepare,
    needs: &mut Needs,
) -> Result<Vec<super::super::embedded_output::FrozenEmbedded>, ApiError> {
    let mut items = vec![];
    let mut seen = BTreeSet::new();
    for s in &req.embedded_queries {
        if !seen.insert(&s.occurrence_id) {
            return Err(ApiError::bad("同一正文查询不能绑定两份结果"));
        }
        let source = req.source.as_deref().ok_or_else(failed)?;
        let occurrence =
            weftext_data::embedded_query(source, &s.occurrence_id).map_err(|_| failed())?;
        if !occurrence.runnable {
            return Err(failed());
        }
        let q = fields::frozen_query(inner, actor, &s.query_id)?;
        let basis = export_api::EmbeddedQueryBasis {
            session_id: req.session_id.clone().ok_or_else(failed)?,
            source_hash: weftext_engine::source_hash(source),
            serial: req.serial.ok_or_else(failed)?,
            occurrence_id: s.occurrence_id.clone(),
        };
        if q.embedded.as_ref() != Some(&basis) {
            return Err(ApiError::bad(
                "查询结果不属于本篇准确草稿和正文位置，未重跑或借用其他结果",
            ));
        }
        for (r, ids) in &q.authorities {
            needs
                .fields
                .entry(r.clone())
                .or_default()
                .extend(ids.iter().cloned());
        }
        needs.resources.extend(q.resources.iter().cloned());
        needs.embedded_authorities.push(q.query_authority.clone());
        let mut value = serde_json::to_value(&q.result).map_err(|_| failed())?;
        value["queryId"] = json!(s.query_id);
        items.push(super::super::embedded_output::FrozenEmbedded {
            occurrence,
            result: q.result.clone(),
            frozen: export_api::FrozenQueryOutput {
                root: String::new(),
                value,
                columns: q.columns,
                schema: Value::Null,
                embedded: Some(basis),
                born: Instant::now(),
            },
        });
    }
    needs.binding = binding_hash(inner, &needs.resources)?;
    Ok(items)
}
fn empty_cut(inner: &HostedInner, actor: &str, req: &Prepare, needs: Needs) -> Cut {
    Cut {
        additional_losses: vec![],
        worker: None,
        environment: None,
        query: None,
        reviews: vec![],
        template: None,
        field_cells: BTreeMap::new(),
        citations: None,
        native_witness: None,
        artifact: None,
        embedded: vec![],
        witnesses: vec![],
        job: Job {
            actor: actor.into(),
            epoch: inner.actor_generation(actor),
            needs,
            expires: Instant::now() + TTL,
            status: "pending",
            artifact: None,
            preview: json!({"format":req.format,"scope":"frozenAuthorizedInput"}),
            digest: None,
            reserved: 1024 * 1024,
        },
    }
}
fn capture_fields_only(
    inner: &mut HostedInner,
    actor: &str,
    req: &Prepare,
    query: Option<fields::FrozenQuery>,
) -> Result<Cut, ApiError> {
    let r = req.resource.as_ref().ok_or_else(failed)?;
    let file = author_file(inner, r)?;
    let mut fields = query
        .as_ref()
        .map(|q| q.authorities.clone())
        .unwrap_or_default();
    fields
        .entry(r.clone())
        .or_default()
        .extend(req.bindings.values().filter_map(|b| b.field.clone()));
    let mut resources = fields.keys().cloned().collect::<BTreeSet<_>>();
    resources.insert(r.clone());
    let needs = Needs {
        format: req.format.clone(),
        citations: Default::default(),
        native_binding: None,
        embedded_authorities: vec![],
        execution: Default::default(),
        sources: if req.source.is_some() {
            BTreeSet::from([r.clone()])
        } else {
            BTreeSet::new()
        },
        fields,
        query_authority: query.as_ref().map(|q| q.query_authority.clone()),
        review: None,
        binding: binding_hash(inner, &resources)?,
        resources,
    };
    access(inner, actor, &needs, false)?;
    let mut cut = empty_cut(inner, actor, req, needs);
    cut.query = query;
    cut.template = req
        .template
        .as_ref()
        .map(|t| decode(&t.bytes_base64))
        .transpose()?;
    let path = inner.config.root.join(&file.path);
    let saved = inner
        .core
        .read_snapshot(&inner.config.root, &[path.to_string_lossy().into_owned()])
        .map_err(|_| failed())?
        .files
        .into_iter()
        .next()
        .ok_or_else(failed)?;
    cut.witnesses.push((
        saved.path.clone(),
        inner
            .core
            .take_snapshot_witness(&saved.base_id)
            .map_err(|_| failed())?,
    ));
    let source = req.source.as_deref().unwrap_or(
        std::str::from_utf8(saved.bytes.as_deref().ok_or_else(failed)?).map_err(|_| failed())?,
    );
    check_draft(inner, actor, req, source)?;
    cut.field_cells = freeze_fields(inner, actor, r, source, req, &mut cut.witnesses)?;
    cut.job.preview["fields"] = json!(cut.field_cells);
    if req.format == "docx" {
        cut.worker = Some(WorkerRequest {
            source: String::new(),
            path: ".hosted-field-export.typ".into(),
            root: inner.config.root.clone(),
            paged: false,
            style: None,
            frozen_files: Some(BTreeMap::new()),
        });
    }
    let size = cut
        .witnesses
        .iter()
        .map(|(_, w)| w.retained_bytes())
        .sum::<usize>()
        + cut.template.as_ref().map_or(0, Vec::len)
        + serde_json::to_vec(&(&cut.field_cells, &cut.job.preview))
            .map_err(|_| failed())?
            .len()
        + cut.query.as_ref().map_or(0, |q| {
            serde_json::to_vec(&q.result).map_or(MAX_STORE, |b| b.len())
        });
    if size > MAX_ARTIFACT {
        return Err(ApiError::bad("本次字段输出输入超过 32 MiB 预算"));
    }
    cut.job.reserved = size.max(1024 * 1024);
    witness_check(inner, &cut.witnesses)?;
    Ok(cut)
}
fn capture_exchange(inner: &mut HostedInner, actor: &str, req: &Prepare) -> Result<Cut, ApiError> {
    let native = req.format == "native";
    let resources = if native {
        BTreeSet::new()
    } else if req.bibliography_scope.as_deref() == Some("selectedWorks") {
        req.works.iter().cloned().collect()
    } else {
        BTreeSet::from([req.resource.clone().ok_or_else(failed)?])
    };
    let needs = Needs {
        format: req.format.clone(),
        citations: Default::default(),
        embedded_authorities: vec![],
        native_binding: if native {
            Some(native_binding(inner)?)
        } else {
            None
        },
        execution: Default::default(),
        sources: if req.bibliography_scope.as_deref() == Some("citationWorks") {
            resources.clone()
        } else {
            BTreeSet::new()
        },
        fields: BTreeMap::new(),
        query_authority: None,
        review: None,
        binding: binding_hash(inner, &resources)?,
        resources,
    };
    access(inner, actor, &needs, false)?;
    let mut cut = empty_cut(inner, actor, req, needs);
    if native {
        let export = inner
            .core
            .capture_native_export(&inner.config.root)
            .map_err(|_| failed())?;
        let size = export
            .bytes
            .len()
            .saturating_add(export.witness.retained_bytes())
            .saturating_add(
                serde_json::to_vec(&export.manifest)
                    .map_err(|_| failed())?
                    .len(),
            );
        if size > MAX_ARTIFACT {
            return Err(ApiError::bad("原生整库资料包与准确输入超过 32 MiB 预算"));
        }
        let (artifact, preview, citations) = native_artifact(inner, actor, export.bytes)?;
        let size = size.saturating_add(artifact.bytes.len());
        if size > MAX_ARTIFACT {
            return Err(ApiError::bad("原生资料包及离线文献输入超过 32 MiB 预算"));
        }
        cut.job.needs.citations = citations;
        access(inner, actor, &cut.job.needs, false)?;
        cut.native_witness = Some(Arc::new(export.witness));
        cut.artifact = Some(artifact);
        cut.job.preview = preview;
        cut.job.reserved = size.max(1024 * 1024);
        return Ok(cut);
    }
    let capture = if req.bibliography_scope.as_deref() == Some("selectedWorks") {
        citations::selected(inner, actor, &req.works)?
    } else {
        let r = req.resource.as_ref().ok_or_else(failed)?;
        let file = author_file(inner, r)?;
        let path = inner.config.root.join(&file.path);
        let saved = inner
            .core
            .read_snapshot(&inner.config.root, &[path.to_string_lossy().into_owned()])
            .map_err(|_| failed())?
            .files
            .into_iter()
            .next()
            .ok_or_else(failed)?;
        cut.witnesses.push((
            saved.path.clone(),
            inner
                .core
                .take_snapshot_witness(&saved.base_id)
                .map_err(|_| failed())?,
        ));
        let source = req.source.as_deref().unwrap_or(
            std::str::from_utf8(saved.bytes.as_deref().ok_or_else(failed)?)
                .map_err(|_| failed())?,
        );
        check_draft(inner, actor, req, source)?;
        citations::freeze(inner, actor, r, source)?
            .ok_or_else(|| ApiError::bad("所选准确正文没有原生文献声明"))?
    };
    cut.job.needs.citations = capture.authority.clone();
    access(inner, actor, &cut.job.needs, false)?;
    cut.job.preview["works"] = json!(capture.works);
    cut.job.preview["scope"] = json!(req.bibliography_scope);
    let size = capture.retained_bytes()
        + cut
            .witnesses
            .iter()
            .map(|(_, w)| w.retained_bytes())
            .sum::<usize>();
    if size > MAX_ARTIFACT {
        return Err(ApiError::bad("本次文献输出输入超过 32 MiB 预算"));
    }
    cut.job.reserved = size.max(1024 * 1024);
    cut.citations = Some(capture);
    Ok(cut)
}
fn native_artifact(
    inner: &HostedInner,
    actor: &str,
    bytes: Vec<u8>,
) -> Result<(ExportArtifact, Value, citations::Authority), ApiError> {
    let mut package =
        weftext_export::exchange::read_native_package(&bytes).map_err(|_| failed())?;
    let authority = citations::native_inputs(inner, actor, &mut package)?;
    let bytes = weftext_export::exchange::write_native_package(&package).map_err(|_| failed())?;
    let mut losses = package
        .manifest
        .losses
        .iter()
        .map(|l| weftext_export::ExportLoss {
            code: l.code.clone(),
            message: l.detail.clone(),
            location: Some(l.path.clone()),
        })
        .collect::<Vec<_>>();
    losses.push(super::super::export_conversions::report("native_not_recovery_backup","资料包包含已保存整库纳管内容与依赖；不包含私有历史、个人草稿或执行日志，不能替代恢复备份。"));
    let preview = json!({"format":"native","scope":"savedWholeLibrary","workspaceId":package.manifest.workspace_id,"fileCount":package.files.len(),"files":package.manifest.entries});
    Ok((
        ExportArtifact {
            bytes,
            content_type: "application/zip".into(),
            extension: "weftext.zip".into(),
            losses,
            summary: format!(
                "已保存的整个资料库：{} 个纳管文件；不是仅当前笔记或恢复备份。",
                package.files.len()
            ),
        },
        preview,
        authority,
    ))
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
        let grants=inner.grants_view(&actor)?;
        let resources:Vec<_>=grants.iter().filter_map(|(r,g)|{
            let f=inner.file(r).ok()?;
            let eligible=author_file(inner,r).is_ok();
            let mut formats=vec![];let mut downloads=vec![];
            for format in ["docx","xlsx","pdf","bibtex","csl-json","ris"] {if eligible && format_access(g,format,false) {formats.push(format);if format_access(g,format,true) {downloads.push(format);}}}
            (!formats.is_empty()).then(||json!({"resource":r,"title":f.title,"formats":formats,"downloads":downloads,"export":true,"download":g.office.download}))
        }).collect();
        let native=&inner.policy.users[&actor].native_whole_library;
        let jobs=inner.office.jobs.iter().filter(|((a,_),j)|a==&actor && access(inner,&actor,&j.needs,false).is_ok()).map(|((_,id),j)|response(id,j)).collect::<Vec<_>>();
        Ok(Json(json!({"formats":["docx","xlsx","pdf","bibtex","csl-json","ris","native"],"resources":resources,"queries":fields::query_catalog(inner,&actor),"jobs":jobs,"native":{"enabled":inner.config.native_whole_library_export,"read":native.read,"export":native.export,"download":native.download,"scope":"savedWholeLibrary"}})))
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
    let bib = matches!(req.format.as_str(), "bibtex" | "csl-json" | "ris");
    if !matches!(
        req.format.as_str(),
        "docx" | "xlsx" | "pdf" | "bibtex" | "csl-json" | "ris" | "native"
    ) || !matches!(req.review_mode.as_str(), "none" | "comments" | "changes")
        || !matches!(req.style_policy.as_str(), "template" | "source")
        || req.bindings.len() > 128
        || (req.template.is_none()
            && !req.bindings.is_empty()
            && !(req.format == "xlsx" && req.bindings.values().all(|b| b.kind == "field")))
        || (req.format == "xlsx"
            && ((req.query_id.is_none()
                && (req.resource.is_none() || !req.bindings.values().any(|b| b.kind == "field")))
                || req.review_mode != "none"
                || req.include_resolved))
        || (req.format == "docx" && req.resource.is_none())
        || (req.format == "pdf"
            && (req.resource.is_none()
                || req.source.is_none()
                || req.session_id.is_none()
                || req.base_id.is_none()
                || req.serial.is_none()
                || req.source_hash.is_none()
                || req.template.is_some()
                || !req.bindings.is_empty()
                || req.review_mode != "none"
                || req.include_resolved))
        || (bib
            && (!matches!(
                req.bibliography_scope.as_deref(),
                Some("citationWorks" | "selectedWorks")
            ) || req.template.is_some()
                || !req.bindings.is_empty()
                || req.query_id.is_some()
                || !req.embedded_queries.is_empty()
                || req.review_mode != "none"
                || req.include_resolved))
        || (!bib && (req.bibliography_scope.is_some() || !req.works.is_empty()))
        || (bib
            && req.bibliography_scope.as_deref() == Some("selectedWorks")
            && (req.works.is_empty()
                || req.resource.is_some()
                || req.source.is_some()
                || req.session_id.is_some()
                || req.base_id.is_some()
                || req.serial.is_some()
                || req.source_hash.is_some()))
        || (bib
            && req.bibliography_scope.as_deref() == Some("citationWorks")
            && (req.resource.is_none() || !req.works.is_empty()))
        || (req.format == "native"
            && (req.resource.is_some()
                || req.source.is_some()
                || req.session_id.is_some()
                || req.base_id.is_some()
                || req.serial.is_some()
                || req.source_hash.is_some()
                || req.query_id.is_some()
                || req.template.is_some()
                || !req.bindings.is_empty()
                || !req.embedded_queries.is_empty()
                || req.review_mode != "none"
                || req.include_resolved))
        || req.embedded_queries.len() > 32
        || req.works.len() > 64
        || (!req.embedded_queries.is_empty() && !matches!(req.format.as_str(), "docx" | "pdf"))
    {
        return Err(ApiError::bad(
            "请选择有效 Office 输出、准确正文或已有查询结果；未使用的参数不能静默忽略",
        ));
    }
    for (slot, b) in &req.bindings {
        if slot.is_empty()
            || slot.len() > 512
            || !matches!(
                b.kind.as_str(),
                "body" | "title" | "text" | "query" | "field"
            )
            || (b.kind == "text" && b.text.is_none())
            || (b.kind != "text" && b.text.is_some())
            || (b.kind == "field"
                && (b
                    .field
                    .as_ref()
                    .is_none_or(|f| f.is_empty() || f.len() > 256)
                    || req.resource.is_none()))
            || (b.kind != "field" && b.field.is_some())
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
        writing_api::validate_source(s, req.source_hash.as_deref())?;
    }
    Ok(())
}
fn capture(inner: &mut HostedInner, actor: &str, req: &Prepare) -> Result<Cut, ApiError> {
    if req.format == "native" || matches!(req.format.as_str(), "bibtex" | "csl-json" | "ris") {
        return capture_exchange(inner, actor, req);
    }
    let query = req
        .query_id
        .as_ref()
        .map(|q| fields::frozen_query(inner, actor, q))
        .transpose()?;
    let body = req.format == "pdf"
        || (req.format == "docx"
            && (req.template.is_none()
                || req
                    .bindings
                    .values()
                    .any(|b| matches!(b.kind.as_str(), "body" | "title"))))
        || req.review_mode != "none";
    if req.resource.is_some() && !body {
        return capture_fields_only(inner, actor, req, query);
    }
    let mut sources = BTreeSet::new();
    let mut captured_dependencies = None;
    if let Some(r) = &req.resource {
        let f = author_file(inner, r)?;
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
    let mut fields = fields;
    if let Some(resource) = &req.resource {
        fields
            .entry(resource.clone())
            .or_default()
            .extend(req.bindings.values().filter_map(|b| b.field.clone()));
    }
    let resources = sources
        .iter()
        .cloned()
        .chain(fields.keys().cloned())
        .collect();
    let mut needs = Needs {
        format: req.format.clone(),
        citations: Default::default(),
        embedded_authorities: vec![],
        native_binding: None,
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
    let mut witnesses = vec![];
    let mut field_cells = BTreeMap::new();
    let worker = if let Some(r) = &req.resource {
        let main = inner.config.root.join(&inner.file(r)?.path);
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
        for file in &snapshots {
            witnesses.push((
                file.path.clone(),
                inner
                    .core
                    .take_snapshot_witness(&file.base_id)
                    .map_err(|_| failed())?,
            ));
        }
        let saved = snapshots
            .iter()
            .find(|f| Path::new(&f.path) == main)
            .ok_or_else(failed)?;
        let bytes = saved.bytes.as_deref().ok_or_else(failed)?;
        let source = req
            .source
            .clone()
            .unwrap_or(String::from_utf8(bytes.to_vec()).map_err(|_| failed())?);
        check_draft(inner, actor, req, &source)?;
        field_cells = freeze_fields(inner, actor, r, &source, req, &mut witnesses)?;
        if needs.review.is_some() {
            if source.as_bytes() != bytes {
                return Err(ApiError::bad(
                    "带批注或修订输出需要已保存的准确正文；请先保存并核对",
                ));
            }
            let root = inner.config.root.clone();
            let current = inner
                .core
                .with_actor_for_root(&scope(inner, actor), &inner.config.root, |core| {
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
        let (mut worker, dependencies, captured_environment) =
            editor::freeze_with_inputs(inner, actor, r, &source, captured)?;
        worker.paged = req.format == "pdf";
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
        witness_check(inner, &witnesses)?;
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
    let embedded = freeze_embedded(inner, actor, req, &mut needs)?;
    if !embedded.is_empty() {
        preview["embeddedQueries"] = super::super::embedded_output::preview(&embedded);
    }
    access(inner, actor, &needs, false)?;
    let reserved = template.as_ref().map_or(0, Vec::len)
        + environment
            .as_ref()
            .map_or(0, library::Environment::retained_bytes)
        + serde_json::to_vec(&preview).map_or(MAX_STORE, |b| b.len())
        + snapshots
            .iter()
            .map(|s| s.bytes.as_ref().map_or(0, Vec::len))
            .sum::<usize>()
        + witnesses
            .iter()
            .map(|(_, w)| w.retained_bytes())
            .sum::<usize>()
        + embedded
            .iter()
            .map(|e| serde_json::to_vec(&e.result).map_or(MAX_STORE, |b| b.len()))
            .sum::<usize>()
        + req.source.as_ref().map_or(0, String::len)
        + query.as_ref().map_or(0, |q| {
            serde_json::to_vec(&(&q.result, &q.schema)).map_or(MAX_STORE, |b| b.len())
        });
    if reserved > MAX_ARTIFACT {
        return Err(ApiError::bad("本次 Office 输入超过 32 MiB 预算"));
    }
    Ok(Cut {
        additional_losses: vec![],
        field_cells,
        citations: None,
        native_witness: None,
        artifact: None,
        embedded,
        witnesses,
        worker,
        environment,
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
fn convert(req: Prepare, mut cut: Cut, render: Option<Value>) -> Result<ExportArtifact, ApiError> {
    use super::super::export_conversions as c;
    let mut artifact = if let Some(artifact) = cut.artifact.take() {
        artifact
    } else if matches!(req.format.as_str(), "bibtex" | "csl-json" | "ris") {
        use weftext_export::bibliography::{BibliographyFormat, project_bibliography};
        let format = match req.format.as_str() {
            "bibtex" => BibliographyFormat::Bibtex,
            "csl-json" => BibliographyFormat::CslJson,
            _ => BibliographyFormat::Ris,
        };
        let result =
            project_bibliography(&cut.citations.as_ref().ok_or_else(failed)?.works, format)
                .map_err(|_| failed())?;
        ExportArtifact {
            bytes: result.bytes,
            content_type: match format {
                BibliographyFormat::Bibtex => "application/x-bibtex",
                BibliographyFormat::CslJson => "application/vnd.citationstyles.csl+json",
                BibliographyFormat::Ris => "application/x-research-info-systems",
            }
            .into(),
            extension: match format {
                BibliographyFormat::Bibtex => "bib",
                BibliographyFormat::CslJson => "json",
                BibliographyFormat::Ris => "ris",
            }
            .into(),
            losses: result.losses,
            summary: "获权出版字段的准确冻结文献；未读取署名对象或全文用于补值。".into(),
        }
    } else if req.format == "pdf" {
        let output: weftext_engine::PagedOutput =
            serde_json::from_value(render.ok_or_else(failed)?).map_err(|_| failed())?;
        if !output.ok {
            return Err(failed());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(output.pdf_base64.ok_or_else(failed)?)
            .map_err(|_| failed())?;
        ExportArtifact {
            bytes,
            content_type: "application/pdf".into(),
            extension: "pdf".into(),
            losses: vec![],
            summary: format!(
                "原生 PDF：{} 页；准确草稿与选中查询结果的冻结输出。",
                output.page_count
            ),
        }
    } else if req.format == "docx" {
        let mut render: weftext_engine::RenderOutput =
            serde_json::from_value(render.ok_or_else(failed)?).map_err(|_| failed())?;
        let embedded_losses = super::super::embedded_output::materialize(
            render.tree.as_mut().ok_or_else(failed)?,
            &cut.embedded,
        )?;
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
                    "field" => docx::DocxBinding::Text {
                        value: c::cell_text(
                            cut.field_cells
                                .get(b.field.as_ref().ok_or_else(failed)?)
                                .ok_or_else(failed)?,
                        )?,
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
        let mut artifact = docx::export_reviewed_docx(
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
        .map_err(|_| failed())?;
        artifact.losses.extend(embedded_losses);
        artifact
    } else {
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
                    cell: if b.kind == "field" {
                        cut.field_cells
                            .get(b.field.as_ref().ok_or_else(failed)?)
                            .ok_or_else(failed)?
                            .clone()
                    } else {
                        weftext_data::Cell::value(weftext_data::NativeValue::Text(
                            b.text.clone().ok_or_else(failed)?,
                        ))
                    },
                });
            }
        }
        let (columns, result, schema) = if let Some(q) = &cut.query {
            (
                q.columns.clone(),
                q.result.clone(),
                Some(
                    serde_json::to_value(query_providers::delivery_schema(&q.schema))
                        .map_err(|_| failed())?,
                ),
            )
        } else {
            let cells = req
                .bindings
                .iter()
                .filter(|(_, b)| b.kind == "field")
                .map(|(name, b)| {
                    Ok((
                        name.clone(),
                        cut.field_cells
                            .get(b.field.as_ref().ok_or_else(failed)?)
                            .ok_or_else(failed)?
                            .clone(),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, ApiError>>()?;
            let columns = cells.keys().cloned().collect();
            let result = weftext_data::QueryResult {
                cut_id: uuid::Uuid::new_v4().to_string(),
                schema_revision: "authorizedFieldSlice".into(),
                rows: vec![weftext_data::QueryRow {
                    key: "authorizedFields".into(),
                    entity_key: req.resource.clone(),
                    path: None,
                    revision: None,
                    cells,
                    read_only: true,
                    lifecycle: Some("live".into()),
                }],
                coverage: "complete".into(),
                freshness: "current".into(),
                diagnostics: vec![],
                total_known: 1,
                limited: false,
                aggregate_exact: true,
                runtime: "frozen-field-projection".into(),
                terminal_schema: BTreeMap::new(),
            };
            (columns, result, None)
        };
        let request = xlsx::XlsxRequest {
            sheets: vec![xlsx::XlsxSheetInput {
                name: "资料".into(),
                columns,
                result,
                schema,
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
    artifact.losses.extend(cut.additional_losses);
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
        .with_actor_for_root(&scope(inner, actor), &inner.config.root, |core| {
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
            .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
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
            .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
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
    let render = if let Some(mut worker) = cut.worker.take() {
        let mut render = editor::compile_frozen(&state, worker.clone()).await?;
        if req.format == "pdf" {
            let output: weftext_engine::PagedOutput =
                serde_json::from_value(render.clone()).map_err(|_| failed())?;
            if !output.ok {
                return Err(failed());
            }
            let (source, losses) = super::super::embedded_output::paged_source(
                &worker.source,
                &cut.embedded,
                &output.embedded_queries,
            )?;
            cut.additional_losses.extend(losses);
            if source != worker.source {
                worker.source = source;
                worker
                    .frozen_files
                    .get_or_insert_with(BTreeMap::new)
                    .insert(
                        worker.path.to_string_lossy().replace('\\', "/"),
                        base64::engine::general_purpose::STANDARD.encode(worker.source.as_bytes()),
                    );
                if let Some(environment) = &cut.environment {
                    environment.apply(&mut worker)?;
                }
                render = editor::compile_frozen(&state, worker).await?;
            }
        }
        Some(render)
    } else {
        None
    };
    let witnesses = std::mem::take(&mut cut.witnesses);
    let environment = cut.environment.take();
    let citations = cut.citations.clone();
    let native_witness = cut.native_witness.clone();
    let delivery_req = req.clone();
    let converted = tokio::task::spawn_blocking(move || convert(req, cut, render))
        .await
        .map_err(|_| failed())?;
    locked(&state, move |inner| {
        if inner.actor(&auth)? != actor {
            return Err(expired());
        }
        let mut job = lookup(inner, &actor, &key.1, false)?
            .filter(|j| j.status == "pending")
            .ok_or_else(failed)?;
        witness_check(inner, &witnesses)?;
        if let Some(citations) = &citations {
            citations.validate(inner, &actor)?;
        }
        if let Some(w) = &native_witness {
            inner
                .core
                .validate_native_export_witness(&inner.config.root, w)
                .map_err(|_| failed())?;
        }
        if let Some(source) = &delivery_req.source {
            check_draft(inner, &actor, &delivery_req, source)?;
        }
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
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn request(format: &str) -> Prepare {
        serde_json::from_value(json!({"requestId":uuid::Uuid::new_v4(),"format":format,"reviewMode":"none","includeResolved":false,"stylePolicy":"template","bindings":{}})).unwrap()
    }
    #[test]
    fn current_field_export_requires_only_actual_field_and_output_grants() {
        let (_temp, state, _auth, _doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let doc = inner
            .core
            .open(root.join("Shared/Shared.typ"), Some(root.clone()))
            .unwrap();
        let id = doc.node_id.unwrap();
        let source = format!(
            "#metadata((id: \"{id}\", types: (\"task\",), status: \"open\")) <weftext>\nPRIVATE BODY NOT EXPORTED\n"
        );
        inner
            .core
            .save(weftext_core::SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source,
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        inner.core.close_session(&doc.session_id).unwrap();
        inner.policy.users.get_mut("alice").unwrap().grants.insert(
            "Shared".into(),
            Rights {
                office: OfficeRights {
                    export: true,
                    download: true,
                    r#import: false,
                },
                fields: BTreeMap::from([(
                    "tasks/status".into(),
                    FieldRights {
                        read: true,
                        write: false,
                    },
                )]),
                ..Default::default()
            },
        );
        let mut req = request("xlsx");
        req.resource = Some("Shared".into());
        req.bindings.insert(
            "status".into(),
            Binding {
                kind: "field".into(),
                field: Some("tasks/status".into()),
                text: None,
            },
        );
        validate(&req).unwrap();
        assert!(inner.rights("alice", "Shared").is_err());
        let cut = capture(&mut inner, "alice", &req).unwrap();
        assert!(cut.worker.is_none());
        assert!(cut.job.needs.sources.is_empty());
        assert!(
            !serde_json::to_string(&cut.job.preview)
                .unwrap()
                .contains("PRIVATE BODY NOT EXPORTED")
        );
        let artifact = convert(req.clone(), cut, None).unwrap();
        assert_eq!(artifact.extension, "xlsx");
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .fields
            .clear();
        assert!(capture(&mut inner, "alice", &req).is_err());
    }
    #[test]
    fn pdf_requires_exact_registered_draft_and_unused_inputs_are_rejected() {
        let req = request("pdf");
        assert!(validate(&req).is_err());
        let mut native = request("native");
        validate(&native).unwrap();
        native.source = Some("unrelated draft".into());
        assert!(validate(&native).is_err());
        let mut bib = request("bibtex");
        bib.bibliography_scope = Some("selectedWorks".into());
        bib.works = vec!["Work".into()];
        validate(&bib).unwrap();
        bib.source = Some("private body".into());
        assert!(validate(&bib).is_err());
        // New absent inputs cannot change the durable fingerprint of a prior
        // Office request after a server upgrade.
        let legacy = json!({"requestId":uuid::Uuid::new_v4().to_string(),"resource":null,"source":null,"format":"xlsx","reviewMode":"none","includeResolved":false,"queryId":"originalQuery","template":null,"stylePolicy":"template","bindings":{"fixed":{"kind":"text","text":"exact"}}});
        let parsed: Prepare = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), legacy);
    }
    #[test]
    fn exact_output_uses_original_hosted_source_actor_and_rechecks_session_and_draft() {
        let (_temp, state, _auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let source = doc["source"].as_str().unwrap().to_owned();
        let mut req = request("pdf");
        req.resource = Some("Shared".into());
        req.session_id = doc["sessionId"].as_str().map(str::to_owned);
        req.base_id = doc["baseId"].as_str().map(str::to_owned);
        req.source = Some(source.clone());
        req.source_hash = Some(weftext_engine::source_hash(&source));
        req.serial = Some(1);
        validate(&req).unwrap();
        // Unlike local/unscoped fixtures, the original Hosted open registered
        // this Core session to the source actor, not the output-fence actor.
        assert!(
            inner
                .core
                .check_session_base(
                    req.session_id.as_deref().unwrap(),
                    req.base_id.as_deref().unwrap()
                )
                .is_err()
        );
        let output_scope = scope(&inner, "alice");
        assert!(
            inner
                .core
                .with_actor(&output_scope, |core| core.check_session_base(
                    req.session_id.as_deref().unwrap(),
                    req.base_id.as_deref().unwrap()
                ))
                .is_err()
        );
        check_draft(&mut inner, "alice", &req, &source).unwrap();
        assert!(check_draft(&mut inner, "bob", &req, &source).is_err());
        writing_api::observe_version(
            &mut inner.editor_versions,
            req.session_id.as_deref().unwrap(),
            &(source.clone() + "Changed"),
            2,
        )
        .unwrap();
        writing_api::observe_version(
            &mut inner.editor_versions,
            req.session_id.as_deref().unwrap(),
            &source,
            3,
        )
        .unwrap();
        assert!(check_draft(&mut inner, "alice", &req, &source).is_err());
        req.serial = Some(3);
        check_draft(&mut inner, "alice", &req, &source).unwrap();
        let path = inner.config.root.join("Shared/Shared.typ");
        std::fs::write(path, source.clone() + "\nExternal body change").unwrap();
        assert!(check_draft(&mut inner, "alice", &req, &source).is_err());
    }
    #[test]
    fn durable_output_namespaces_stay_shared_and_whole_library_has_its_own_permission() {
        let (_temp, state, _auth, _doc, _, _) = history_api::test_fixture();
        let inner = state.inner.lock().unwrap();
        assert!(scope(&inner, "alice").ends_with(":office-output"));
        let resources = BTreeSet::new();
        let needs = Needs {
            format: "native".into(),
            citations: Default::default(),
            embedded_authorities: vec![],
            native_binding: Some(native_binding(&inner).unwrap()),
            execution: Default::default(),
            sources: BTreeSet::new(),
            fields: BTreeMap::new(),
            query_authority: None,
            review: None,
            binding: binding_hash(&inner, &resources).unwrap(),
            resources,
        };
        assert!(inner.rights("alice", "Shared").is_ok());
        assert!(access(&inner, "alice", &needs, false).is_err());
        assert!(!valid_native_rights(&NativeRights {
            read: false,
            export: true,
            download: true
        }));
    }
}
