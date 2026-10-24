//! Frozen Office consumers. Source, field, target, output and delivery rights
//! remain separate. A durable request claim never authorizes regeneration.
use super::*;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use weftext_export::{ExportArtifact, docx, xlsx};
use weftext_knowledge as k;
use weftext_messages::UiInteger;

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
    #[serde(default)]
    pub(super) doc: LegacyFormatRights,
    #[serde(default)]
    pub(super) wps: LegacyFormatRights,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LegacyFormatRights {
    #[serde(default)]
    pub(super) export: bool,
    #[serde(default)]
    pub(super) download: bool,
    #[serde(default)]
    pub(super) r#import: bool,
}
pub(super) fn valid_legacy_rights(rights: &OfficeRights) -> bool {
    (!rights.doc.download || rights.doc.export) && (!rights.wps.download || rights.wps.export)
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
    pub(super) recognition: crate::recognition_api::Store,
    jobs: HashMap<(String, String), Job>,
    pub(super) legacy_root: Option<PathBuf>,
    active: HashMap<(String, String), LegacyImportRun>,
    pub(super) import_active: HashMap<(String, String), LegacyImportRun>,
    legacy_capabilities: Option<Value>,
    pub(super) closing: bool,
}
impl Store {
    pub(super) fn close(&mut self) {
        self.closing = true;
        self.recognition.close();
        for run in self.active.values().chain(self.import_active.values()) {
            run.cancel.store(true, Ordering::Release);
        }
    }
    pub(super) fn with_legacy_root(root: PathBuf) -> Self {
        Self {
            recognition: crate::recognition_api::Store::new(root.with_file_name("recognition")),
            legacy_root: Some(root),
            ..Self::default()
        }
    }
}
pub(super) struct LegacyImportRun {
    pub(super) actor_epoch: u64,
    pub(super) cancel: Arc<AtomicBool>,
}
#[derive(Clone, Serialize, Deserialize)]
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
    output_locale: weftext_messages::UiLocale,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_backend: Option<String>,
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
        .route(
            "/api/hosted/office/legacy-capabilities",
            get(legacy_capabilities),
        )
        .route("/api/hosted/office/export/cancel", post(legacy_cancel))
        .route("/api/hosted/export", get(catalog))
        .route("/api/hosted/export/prepare", post(prepare))
        .route("/api/hosted/export/status", post(status))
        .route("/api/hosted/export/confirm", post(confirm))
        .route("/api/hosted/export/chunk", post(chunk))
        .route("/api/hosted/export/cancel", post(legacy_cancel))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    for runs in [&mut inner.office.active, &mut inner.office.import_active] {
        runs.retain(|(actor, _), run| {
            let keep = inner
                .policy
                .users
                .get(actor)
                .is_some_and(|u| u.enabled && u.epoch == run.actor_epoch);
            if !keep {
                run.cancel.store(true, Ordering::Release);
            }
            keep
        });
    }
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
        UiMessage::error_api_hosted_office_output_basis_changed(),
    )
}
fn id(value: &str) -> Result<(), ApiError> {
    uuid::Uuid::parse_str(value).map_err(|_| {
        ApiError::bad(UiMessage::error_api_hosted_office_output_request_id_invalid())
    })?;
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
        "doc" => grant.office.doc.export && (!download || grant.office.doc.download),
        "wps" => grant.office.wps.export && (!download || grant.office.wps.download),
        "pdf" => grant.outputs.pdf.export && (!download || grant.outputs.pdf.download),
        "bibtex" | "csl-json" | "ris" => {
            grant.outputs.bibliography.export && (!download || grant.outputs.bibliography.download)
        }
        _ => grant.office.export && (!download || grant.office.download),
    }
}
fn legacy_format(format: &str) -> bool {
    matches!(format, "doc" | "wps")
}
fn docx_family(format: &str) -> bool {
    matches!(format, "docx" | "doc" | "wps")
}
fn may_probe_legacy(inner: &HostedInner, actor: &str) -> bool {
    inner.policy.users.get(actor).is_some_and(|u| {
        u.enabled
            && u.grants.values().any(|g| {
                g.office.export
                    || g.office.download
                    || g.office.r#import
                    || g.office.doc.export
                    || g.office.doc.download
                    || g.office.doc.r#import
                    || g.office.wps.export
                    || g.office.wps.download
                    || g.office.wps.r#import
            })
    })
}
async fn server_legacy_capabilities(state: &HostedState, auth: &Auth) -> Result<Value, ApiError> {
    let check = auth.clone();
    let persistent = locked(state, move |inner| {
        let actor = inner.actor(&check)?;
        if !may_probe_legacy(inner, &actor) {
            return Err(unavailable());
        }
        Ok(inner.office.legacy_root.is_some())
    })
    .await?;
    let mut capabilities = value(super::super::legacy_office::probe().await)?;
    capabilities["executionLocation"] = json!("server");
    capabilities["persistentStorageConfigured"] = json!(persistent);
    let unavailable_description = value(ErrorDescription::new(
        UiMessage::error_api_hosted_office_durable_conversion_directory_required(),
    ))?;
    if !persistent && let Some(backends) = capabilities["backends"].as_array_mut() {
        for backend in backends {
            if let Some(directions) = backend["directions"].as_array_mut() {
                for direction in directions {
                    direction["available"] = json!(false);
                    direction.as_object_mut().ok_or_else(failed)?.extend(
                        unavailable_description
                            .as_object()
                            .ok_or_else(failed)?
                            .clone(),
                    );
                }
            }
        }
    }
    let check = auth.clone();
    let cached = capabilities.clone();
    locked(state, move |inner| {
        let actor = inner.actor(&check)?;
        if !may_probe_legacy(inner, &actor) {
            return Err(unavailable());
        }
        inner.office.legacy_capabilities = Some(cached);
        Ok(())
    })
    .await?;
    Ok(capabilities)
}
async fn legacy_capabilities(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
) -> ApiResult {
    Ok(Json(server_legacy_capabilities(&state, &auth).await?))
}
async fn legacy_cancel(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<RequestId>,
) -> ApiResult {
    id(&req.request_id)?;
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        expire_revoked(inner);
        let active = inner.office.active.get(&(actor.clone(), req.request_id.clone()));
        if let Some(run) = active { run.cancel.store(true, Ordering::Release); }
        Ok(Json(json!({"requestId":req.request_id,"status":if active.is_some(){"cancelling"}else{"notRunning"},"executionLocation":"server"})))
    }).await
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
        ApiError::bad(UiMessage::error_api_hosted_office_exact_draft_output_required())
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
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_office_duplicate_embedded_query_binding(),
            ));
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
                UiMessage::error_api_hosted_office_embedded_query_result_mismatch(),
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
        value["engineDiagnostics"] = json!([]);
        value["productErrors"] = json!([]);
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
    if docx_family(&req.format) {
        cut.worker = Some(WorkerRequest {
            source: String::new(),
            path: ".hosted-field-export.typ".into(),
            root: inner.config.root.clone(),
            paged: false,
            style: None,
            frozen_files: BTreeMap::new(),
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
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_field_output_input_budget(),
        ));
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
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_office_native_library_package_budget(),
            ));
        }
        let (artifact, preview, citations) = native_artifact(inner, actor, export.bytes)?;
        let size = size.saturating_add(artifact.bytes.len());
        if size > MAX_ARTIFACT {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_office_native_offline_bibliography_budget(),
            ));
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
        citations::freeze(inner, actor, r, source)?.ok_or_else(|| {
            ApiError::bad(
                UiMessage::error_api_hosted_office_native_bibliography_declaration_required(),
            )
        })?
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
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_bibliography_output_budget(),
        ));
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
            description: l.description.clone(),
            product_diagnostics: l.product_diagnostics.clone(),
            location: Some(l.path.clone()),
        })
        .collect::<Vec<_>>();
    losses.push(super::super::export_conversions::report(
        "native_not_recovery_backup",
        UiMessage::output_hosted_office_not_recovery_backup(),
    ));
    let preview = json!({"format":"native","scope":"savedWholeLibrary","workspaceId":package.manifest.workspace_id,"fileCount":package.files.len(),"files":package.manifest.entries});
    Ok((
        ExportArtifact {
            bytes,
            content_type: "application/zip".into(),
            extension: "weftext.zip".into(),
            losses,
            summary: UiMessage::output_hosted_office_native_library_summary(
                weftext_messages::params::OutputHostedOfficeNativeLibrarySummary {
                    file_count: i64::try_from(package.files.len())
                        .ok()
                        .and_then(UiInteger::new)
                        .ok_or_else(failed)?,
                },
            ),
        },
        preview,
        authority,
    ))
}
fn decode(encoded: &str) -> Result<Vec<u8>, ApiError> {
    if encoded.len() > MAX_TEMPLATE * 4 / 3 + 4 {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_office_template_size_limit(),
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            ApiError::bad(UiMessage::error_api_hosted_office_office_file_encoding_invalid())
        })?;
    if bytes.len() > MAX_TEMPLATE {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_office_template_size_limit(),
        ));
    }
    Ok(bytes)
}
async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;
        expire_revoked(inner);
        let legacy=inner.office.legacy_capabilities.clone();
        let grants=inner.grants_view(&actor)?;
        let resources:Vec<_>=grants.iter().filter_map(|(r,g)|{
            let f=inner.file(r).ok()?;
            let eligible=author_file(inner,r).is_ok();
            let mut formats=vec![];let mut downloads=vec![];
            for format in ["docx","doc","wps","xlsx","pdf","bibtex","csl-json","ris"] {if eligible && format_access(g,format,false) {formats.push(format);if format_access(g,format,true) {downloads.push(format);}}}
            (!formats.is_empty()).then(||json!({"resource":r,"title":f.title,"formats":formats,"downloads":downloads,"export":true,"download":g.office.download}))
        }).collect();
        let native=&inner.policy.users[&actor].native_whole_library;
        let jobs=inner.office.jobs.iter().filter(|((a,_),j)|a==&actor && access(inner,&actor,&j.needs,false).is_ok()).map(|((_,id),j)|response(id,j)).collect::<Vec<_>>();
        Ok(Json(json!({"formats":["docx","doc","wps","xlsx","pdf","bibtex","csl-json","ris","native"],"legacy":legacy,"resources":resources,"queries":fields::query_catalog(inner,&actor),"jobs":jobs,"native":{"enabled":inner.config.native_whole_library_export,"read":native.read,"export":native.export,"download":native.download,"scope":"savedWholeLibrary"}})))
    }).await
}
async fn inspect(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Inspect>,
) -> ApiResult {
    let check = auth.clone();
    let format = req.format.clone();
    let inspect_format = format.clone();
    let (actor, epoch) = locked(&state, move |inner| {
        let a = inner.actor(&check)?;
        inspect_access(inner, &a, &inspect_format)?;
        Ok((a.clone(), inner.actor_generation(&a)))
    })
    .await?;
    let bytes = decode(&req.bytes_base64)?;
    let _permit = state
        .render_gate
        .try_acquire()
        .map_err(|_| ApiError::bad(UiMessage::error_api_hosted_office_office_conversion_busy()))?;
    let result = crate::work::spawn_blocking(move || {
        super::super::export_conversions::template_slots(
            if legacy_format(&req.format) {
                "docx"
            } else {
                &req.format
            },
            &bytes,
        )
    })
    .await
    .map_err(|_| failed())?;
    locked(&state, move |inner| {
        if inner.actor(&auth)? != actor || inner.actor_generation(&actor) != epoch {
            return Err(expired());
        }
        inspect_access(inner, &actor, &format)?;
        Ok(Json(result?))
    })
    .await
}
fn inspect_access(inner: &HostedInner, actor: &str, format: &str) -> Result<(), ApiError> {
    if !matches!(format, "docx" | "doc" | "wps" | "xlsx") {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_office_template_invalid(),
        ));
    }
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(unavailable)?;
    if user.grants.values().any(|g| {
        if docx_family(format) {
            g.office.export || g.office.doc.export || g.office.wps.export
        } else {
            g.office.export
        }
    }) {
        Ok(())
    } else {
        Err(unavailable())
    }
}
fn validate(req: &Prepare) -> Result<(), ApiError> {
    id(&req.request_id)?;
    let bib = matches!(req.format.as_str(), "bibtex" | "csl-json" | "ris");
    if !matches!(
        req.format.as_str(),
        "docx" | "doc" | "wps" | "xlsx" | "pdf" | "bibtex" | "csl-json" | "ris" | "native"
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
        || (docx_family(&req.format) && req.resource.is_none())
        || (!legacy_format(&req.format) && req.legacy_backend.is_some())
        || req
            .legacy_backend
            .as_ref()
            .is_some_and(|b| !matches!(b.as_str(), "libreoffice" | "configured"))
        || (req.resource.is_none()
            && (req.source.is_some()
                || req.session_id.is_some()
                || req.base_id.is_some()
                || req.serial.is_some()
                || req.source_hash.is_some()))
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
        || (!req.embedded_queries.is_empty()
            && !matches!(req.format.as_str(), "docx" | "doc" | "wps" | "pdf"))
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_office_output_request_invalid(),
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
                UiMessage::error_api_hosted_office_template_slot_binding_invalid(),
            ));
        }
    }
    if docx_family(&req.format)
        && req.query_id.is_some()
        && !req.bindings.values().any(|b| b.kind == "query")
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_docx_query_slot_required(),
        ));
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
        || (docx_family(&req.format)
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
                    UiMessage::error_api_hosted_office_saved_body_required_for_review_output(),
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
        preview["summary"] = json!(UiMessage::error_api_export_frozen_query_preview());
    }
    let embedded = freeze_embedded(inner, actor, req, &mut needs)?;
    if !embedded.is_empty() {
        preview["embeddedQueries"] = super::super::embedded_output::preview(&embedded)?;
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
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_office_input_budget(),
        ));
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
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_annotation_output_budget(),
        ));
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
        if let Some(s)=suggestion {body.push('\n'); body.push_str(&UiMessage::error_api_export_conversion_review_suggestion_appendix(weftext_messages::params::ErrorApiExportConversionReviewSuggestionAppendix { state:s.state.clone(),replacement:s.replacement.clone() }).render(req.output_locale));}
        let downgrade_reason=(req.review_mode=="changes" && r.suggestion.as_ref().is_some_and(|s|s.state=="pending") && replacement.is_none()).then(||ErrorDescription::new(UiMessage::error_api_export_conversion_review_suggestion_downgrade()));
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
            summary: UiMessage::output_hosted_office_bibliography_summary(),
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
            summary: UiMessage::output_hosted_office_pdf_summary(
                weftext_messages::params::OutputHostedOfficePdfSummary {
                    page_count: i64::try_from(output.page_count)
                        .ok()
                        .and_then(UiInteger::new)
                        .ok_or_else(failed)?,
                },
            ),
        }
    } else if req.format == "docx" {
        let mut render: weftext_engine::RenderOutput =
            serde_json::from_value(render.ok_or_else(failed)?).map_err(|_| failed())?;
        let embedded_losses = super::super::embedded_output::materialize(
            render.tree.as_mut().ok_or_else(failed)?,
            &cut.embedded,
            req.output_locale,
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
                name: UiMessage::error_api_export_conversion_data_sheet_name()
                    .render(req.output_locale),
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
                UiMessage::error_api_export_conversion_xlsx_source_style_unmapped(),
            ));
        }
        a
    };
    if let Some(q) = &cut.query {
        artifact.losses.extend(c::query_losses(&q.result));
    }
    artifact.losses.extend(cut.additional_losses);
    if artifact.bytes.len() > MAX_ARTIFACT {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_office_artifact_budget(),
        ));
    }
    Ok(artifact)
}
fn response(request: &str, job: &Job) -> Value {
    let Some(a) = &job.artifact else {
        return json!({"requestId":request,"status":job.status,"preview":job.preview});
    };
    json!({"requestId":request,"status":job.status,"filename":format!("weftext.{}",a.extension),"contentType":a.content_type,"length":a.bytes.len(),"digest":job.digest,"losses":a.losses,"summary":a.summary,"preview":job.preview})
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyStage {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    conversion_layout: Option<String>,
    actor: String,
    epoch: u64,
    fingerprint: String,
    expires_at: u64,
    request: Prepare,
    needs: Needs,
    preview: Value,
    docx_sha256: String,
    docx_length: usize,
    losses: Vec<weftext_export::ExportLoss>,
    summary: UiMessage,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyReady {
    fingerprint: String,
    output_sha256: String,
    output_length: usize,
    preview: Value,
    losses: Vec<weftext_export::ExportLoss>,
    summary: UiMessage,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyPublication {
    fingerprint: String,
    ready_sha256: String,
}
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyConfirmed {
    fingerprint: String,
    digest: String,
    accepted_losses: bool,
}
struct LegacyEvidence {
    witnesses: Vec<(String, weftext_core::SnapshotWitness)>,
    environment: Option<library::Environment>,
    citations: Option<citations::Capture>,
    epoch: u64,
}
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn legacy_location(
    inner: &HostedInner,
    actor: &str,
    request: &str,
) -> Result<(PathBuf, PathBuf), ApiError> {
    let uuid = uuid::Uuid::parse_str(request).map_err(|_| {
        ApiError::bad(UiMessage::error_api_hosted_office_legacy_output_uuid_required())
    })?;
    if uuid.get_version_num() != 4 || uuid.to_string() != request {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_legacy_output_uuid_required(),
        ));
    }
    let root = inner.office.legacy_root.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::CONFLICT,
            "legacyStorageUnavailable",
            UiMessage::error_api_hosted_office_durable_conversion_directory_required(),
        )
    })?;
    let namespace = digest(&serde_json::to_vec(&json!({"scope":scope(inner,actor),"root":inner.config.root,"deployment":inner.config.deployment})).map_err(|_| failed())?);
    let owned = root
        .join(namespace)
        .join(format!("epoch-{}", inner.actor_generation(actor)));
    Ok((
        owned.join("exports").join(request),
        owned.join("conversions"),
    ))
}
fn flat_legacy_location(
    inner: &HostedInner,
    actor: &str,
    epoch: u64,
    request: &str,
) -> Result<super::super::legacy_office::ConversionLocation, ApiError> {
    super::super::legacy_office::scoped_location(
        inner.office.legacy_root.as_deref().ok_or_else(failed)?,
        &inner.config.deployment,
        &inner.config.root,
        actor,
        epoch,
        request,
        "export",
    )
    .map_err(|_| failed())
}
fn durable_write(path: &Path, bytes: &[u8]) -> Result<(), ApiError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| failed())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| failed())
}
fn reparse(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
fn stage_root(dir: &Path) -> Result<&Path, ApiError> {
    dir.ancestors().nth(4).ok_or_else(failed)
}
pub(super) fn ensure_owned(root: &Path, path: &Path, create: bool) -> Result<(), ApiError> {
    if !root.is_absolute() {
        return Err(failed());
    }
    let relative = path.strip_prefix(root).map_err(|_| failed())?;
    if create {
        std::fs::create_dir_all(root).map_err(|_| failed())?;
    }
    let metadata = std::fs::symlink_metadata(root).map_err(|_| failed())?;
    if !metadata.is_dir() || reparse(&metadata) {
        return Err(failed());
    }
    let canonical_root = root.canonicalize().map_err(|_| failed())?;
    let mut current = root.to_owned();
    let mut expected = canonical_root;
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(failed());
        }
        current.push(component.as_os_str());
        expected.push(component.as_os_str());
        if create {
            match std::fs::create_dir(&current) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(failed()),
            }
        }
        let metadata = std::fs::symlink_metadata(&current).map_err(|_| failed())?;
        if !metadata.is_dir()
            || reparse(&metadata)
            || current.canonicalize().map_err(|_| failed())? != expected
        {
            return Err(failed());
        }
    }
    Ok(())
}
fn bounded_read(dir: &Path, name: &str, max: usize) -> Result<Vec<u8>, ApiError> {
    use std::io::Read;
    let path = dir.join(name);
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| failed())?;
    if !metadata.is_file() || reparse(&metadata) || metadata.len() > max as u64 {
        return Err(failed());
    }
    let canonical = path.canonicalize().map_err(|_| failed())?;
    let parent = dir.canonicalize().map_err(|_| failed())?;
    if canonical.parent() != Some(parent.as_path()) {
        return Err(failed());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .map_err(|_| failed())?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failed())?;
    if bytes.len() > max {
        return Err(failed());
    }
    Ok(bytes)
}
fn write_stage(dir: &Path, stage: &LegacyStage, bytes: &[u8]) -> Result<(), ApiError> {
    ensure_owned(stage_root(dir)?, dir, true)?;
    let metadata = serde_json::to_vec(stage).map_err(|_| failed())?;
    if metadata.len() > MAX_STORE {
        return Err(failed());
    }
    durable_write(&dir.join("frozen.docx"), bytes)?;
    durable_write(&dir.join("stage.json"), &metadata)?;
    durable_write(&dir.join("stage.sha256"), digest(&metadata).as_bytes())
}
fn read_stage(dir: &Path) -> Result<(LegacyStage, Vec<u8>), ApiError> {
    ensure_owned(stage_root(dir)?, dir, false)?;
    let metadata = std::fs::symlink_metadata(dir).map_err(|_| failed())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(failed());
    }
    let bytes = bounded_read(dir, "stage.json", MAX_STORE)?;
    let seal = bounded_read(dir, "stage.sha256", 64)?;
    if seal != digest(&bytes).as_bytes() {
        return Err(failed());
    }
    let stage: LegacyStage = serde_json::from_slice(&bytes).map_err(|_| failed())?;
    let docx = bounded_read(dir, "frozen.docx", MAX_ARTIFACT)?;
    if stage.schema_version != 1
        || stage.docx_length != docx.len()
        || stage.docx_sha256 != digest(&docx)
        || !legacy_format(&stage.request.format)
        || stage.needs.format != stage.request.format
        || stage.fingerprint != digest(&serde_json::to_vec(&stage.request).map_err(|_| failed())?)
    {
        return Err(failed());
    }
    Ok((stage, docx))
}
fn legacy_request(req: &Prepare, bytes: Vec<u8>) -> super::super::legacy_office::ConversionRequest {
    super::super::legacy_office::ConversionRequest {
        backend: req
            .legacy_backend
            .clone()
            .unwrap_or_else(|| "libreoffice".into()),
        direction: if req.format == "doc" {
            super::super::legacy_office::Direction::DocxToDoc
        } else {
            super::super::legacy_office::Direction::DocxToWps
        },
        bytes,
    }
}
fn unknown_answer(request: &str, code: &str) -> Value {
    let description = ErrorDescription::new(UiMessage::error_api_hosted_office_result_unknown());
    json!({"requestId":request,"status":"unknown","code":code,
        "explanation":description.explanation,"diagnostics":description.diagnostics,"causes":description.causes,
        "executionLocation":"server"})
}
struct LegacyRunGuard {
    state: HostedState,
    key: Arc<Mutex<Option<(String, String)>>>,
    cancel: Arc<AtomicBool>,
}
impl Drop for LegacyRunGuard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        let Some(key) = self.key.lock().ok().and_then(|g| g.clone()) else {
            return;
        };
        if let Ok(mut inner) = self.state.inner.lock() {
            if inner
                .office
                .active
                .get(&key)
                .is_some_and(|r| Arc::ptr_eq(&r.cancel, &self.cancel))
            {
                inner.office.active.remove(&key);
            }
            if let Some(job) = inner.office.jobs.get_mut(&key)
                && job.status == "pending"
            {
                job.status = "unknown";
                job.reserved = 0;
                job.preview = unknown_answer(&key.1, "requestInterrupted");
            }
        }
    }
}
async fn recheck_legacy(
    state: &HostedState,
    auth: &Auth,
    actor: &str,
    req: &Prepare,
    evidence: Arc<LegacyEvidence>,
) -> Result<Job, ApiError> {
    let auth = auth.clone();
    let actor = actor.to_owned();
    let req = req.clone();
    locked(state, move |inner| {
        if inner.actor(&auth)? != actor || inner.actor_generation(&actor) != evidence.epoch {
            return Err(expired());
        }
        let job = lookup(inner, &actor, &req.request_id, false)?
            .filter(|j| j.status == "pending")
            .ok_or_else(failed)?;
        witness_check(inner, &evidence.witnesses)?;
        if let Some(c) = &evidence.citations {
            c.validate(inner, &actor)?;
        }
        if let Some(source) = &req.source {
            check_draft(inner, &actor, &req, source)?;
        }
        if let Some(env) = &evidence.environment {
            env.recheck(inner, &actor)?;
        }
        Ok(job)
    })
    .await
}
// A retry can deliver only a previously published, sealed artifact. Incomplete
// stages never compile source or start an external program during recovery.
async fn recover_legacy(
    state: &HostedState,
    auth: &Auth,
    request: &str,
    expected: Option<String>,
) -> Result<Option<Value>, ApiError> {
    id(request)?;
    let check = auth.clone();
    let request_owned = request.to_owned();
    let context = locked(state, move |inner| {
        let actor = inner.actor(&check)?;
        expire_revoked(inner);
        let claim = inner
            .core
            .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
                core.actor_output_fingerprint(&request_owned)
            })
            .map_err(HostedInner::core_error)?;
        let Some(claim) = claim else { return Ok(None) };
        if expected.as_ref().is_some_and(|e| e != &claim) {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "operationConflict",
                UiMessage::error_api_hosted_office_output_request_input_conflict(),
            ));
        }
        let Ok((dir, conversions)) = legacy_location(inner, &actor, &request_owned) else {
            return Ok(None);
        };
        let flat = flat_legacy_location(
            inner,
            &actor,
            inner.actor_generation(&actor),
            &request_owned,
        )?;
        Ok(Some((
            actor,
            inner.actor_generation(&inner.actor(&check)?),
            claim,
            dir,
            conversions,
            flat,
        )))
    })
    .await?;
    let Some((actor, epoch, claim, dir, old_conversions, flat)) = context else {
        return Ok(None);
    };
    match std::fs::symlink_metadata(&dir) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Ok(Some(unknown_answer(request, "stageUnknown"))),
    }
    let read_dir = dir.clone();
    let loaded = crate::work::spawn_blocking(move || read_stage(&read_dir))
        .await
        .map_err(|_| failed())?;
    let (stage, bytes) = match loaded {
        Ok(v) => v,
        Err(_) => return Ok(Some(unknown_answer(request, "stageUnknown"))),
    };
    if stage.actor != actor
        || stage.epoch != epoch
        || stage.request.request_id != request
        || stage.fingerprint != claim
    {
        return Ok(Some(unknown_answer(request, "stageMismatch")));
    }
    let (conversions, conversion_id) = match stage.conversion_layout.as_deref() {
        None => (old_conversions, request.to_owned()),
        Some("flatV1") => (flat.root, flat.request_id),
        _ => return Ok(Some(unknown_answer(request, "stageMismatch"))),
    };
    let check = auth.clone();
    let needs = stage.needs.clone();
    let expected_actor = actor.clone();
    locked(state, move |inner| {
        if inner.actor(&check)? != expected_actor
            || inner.actor_generation(&expected_actor) != epoch
        {
            return Err(expired());
        }
        access(inner, &expected_actor, &needs, false)
    })
    .await?;
    if stage.expires_at <= unix_now() {
        return Ok(Some(json!({"requestId":request,"status":"expired"})));
    }
    if ensure_owned(stage_root(&dir)?, &conversions, false).is_err() {
        return Ok(Some(unknown_answer(request, "conversionUnknown")));
    }
    let result = super::super::legacy_office::recover_once(
        &conversion_id,
        legacy_request(&stage.request, bytes),
        &conversions,
    )
    .await;
    let result = match result {
        Ok(r) => r,
        Err(_) => return Ok(Some(unknown_answer(request, "conversionUnknown"))),
    };
    if result.status != "succeeded" {
        let status = match result.status.as_str() {
            "cancelled" => "cancelled",
            "failed" | "unavailable" => "failed",
            _ => "unknown",
        };
        let check = auth.clone();
        let needs = stage.needs.clone();
        let actor = actor.clone();
        locked(state, move |inner| {
            if inner.actor(&check)? != actor || inner.actor_generation(&actor) != epoch {
                return Err(expired());
            }
            access(inner, &actor, &needs, false)
        })
        .await?;
        let mut metadata = result.metadata();
        metadata["requestId"] = json!(request);
        return Ok(Some(
            json!({"requestId":request,"status":status,"preview":{"legacy":metadata},"executionLocation":"server"}),
        ));
    }
    let generated = match &result.generated {
        Some(a) => a,
        None => return Ok(Some(unknown_answer(request, "artifactUnknown"))),
    };
    let ready = bounded_read(&dir, "ready.json", MAX_STORE).and_then(|b| {
        let published: LegacyPublication =
            serde_json::from_slice(&bounded_read(&dir, "published.json", 1024)?)
                .map_err(|_| failed())?;
        if published.fingerprint != claim || published.ready_sha256 != digest(&b) {
            return Err(failed());
        }
        serde_json::from_slice::<LegacyReady>(&b).map_err(|_| failed())
    });
    let ready = match ready {
        Ok(r)
            if r.fingerprint == claim
                && r.output_sha256 == generated.sha256
                && r.output_length == generated.bytes.len() =>
        {
            r
        }
        _ => return Ok(Some(unknown_answer(request, "publicationUnknown"))),
    };
    let confirmed = if dir.join("confirmed.json").try_exists().unwrap_or(true) {
        let confirmation = bounded_read(&dir, "confirmed.json", 1024)
            .and_then(|b| serde_json::from_slice::<LegacyConfirmed>(&b).map_err(|_| failed()));
        match confirmation {
            Ok(c)
                if c.fingerprint == claim
                    && c.digest == generated.sha256
                    && (ready.losses.is_empty() || c.accepted_losses) =>
            {
                true
            }
            _ => return Ok(Some(unknown_answer(request, "confirmationUnknown"))),
        }
    } else {
        false
    };
    let artifact = ExportArtifact {
        bytes: generated.bytes.clone(),
        content_type: if stage.request.format == "doc" {
            "application/msword"
        } else {
            "application/octet-stream"
        }
        .into(),
        extension: stage.request.format.clone(),
        losses: ready.losses,
        summary: ready.summary,
    };
    let job = Job {
        actor: actor.clone(),
        epoch,
        needs: stage.needs,
        expires: Instant::now()
            + Duration::from_secs(
                stage
                    .expires_at
                    .saturating_sub(unix_now())
                    .min(TTL.as_secs()),
            ),
        status: if confirmed { "confirmed" } else { "prepared" },
        digest: Some(generated.sha256.clone()),
        reserved: artifact.bytes.len()
            + serde_json::to_vec(&ready.preview).map_or(MAX_STORE, |b| b.len()),
        preview: ready.preview,
        artifact: Some(Arc::new(artifact)),
    };
    let check = auth.clone();
    let request = request.to_owned();
    locked(state, move |inner| {
        if inner.actor(&check)? != actor || inner.actor_generation(&actor) != epoch {
            return Err(expired());
        }
        access(inner, &actor, &job.needs, false)?;
        let replaces = inner
            .office
            .jobs
            .contains_key(&(actor.clone(), request.clone()));
        if !replaces
            && (inner.office.jobs.len() >= 32
                || inner
                    .office
                    .jobs
                    .values()
                    .filter(|j| j.actor == actor)
                    .count()
                    >= 8)
        {
            return Err(failed());
        }
        let used = inner
            .office
            .jobs
            .iter()
            .filter(|(k, _)| *k != &(actor.clone(), request.clone()))
            .map(|(_, j)| j.reserved)
            .sum::<usize>();
        if used.saturating_add(job.reserved) > MAX_STORE {
            return Err(failed());
        }
        let answer = response(&request, &job);
        inner.office.jobs.insert((actor, request), job);
        Ok(Some(answer))
    })
    .await
}
async fn deliver_legacy_result<T>(
    state: &HostedState,
    auth: &Auth,
    actor: &str,
    req: &Prepare,
    evidence: Arc<LegacyEvidence>,
    result: Result<T, ApiError>,
) -> Result<(Job, T), ApiError> {
    let job = recheck_legacy(state, auth, actor, req, evidence).await?;
    Ok((job, result?))
}
async fn prepare_legacy(
    state: HostedState,
    auth: Auth,
    req: Prepare,
    _allow_start: bool,
) -> ApiResult {
    let cancel = Arc::new(AtomicBool::new(false));
    let registration = Arc::new(Mutex::new(None));
    let _guard = LegacyRunGuard {
        state: state.clone(),
        key: registration.clone(),
        cancel: cancel.clone(),
    };
    let registered = registration.clone();
    let initial_cancel = cancel.clone();
    let fingerprint = digest(&serde_json::to_vec(&req).map_err(|_| failed())?);
    let check = auth.clone();
    let captured = req.clone();
    let fp = fingerprint.clone();
    let (actor, cut, location, flat, answer) = locked(&state, move |inner| {
        let actor = inner.actor(&check)?;
        expire_revoked(inner);
        let location = legacy_location(inner, &actor, &captured.request_id)?;
        let flat=flat_legacy_location(inner,&actor,inner.actor_generation(&actor),&captured.request_id)?;
        let old = inner
            .core
            .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
                core.actor_output_fingerprint(&captured.request_id)
            })
            .map_err(HostedInner::core_error)?;
        if let Some(old) = old {
            if old != fp {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "operationConflict",
                    UiMessage::error_api_hosted_office_output_request_input_conflict(),
                ));
            }
            let answer = lookup(inner, &actor, &captured.request_id, false)?
                .filter(|j| j.status != "unknown")
                .map(|j| response(&captured.request_id, &j));
            return Ok((actor, None, location, flat, answer));
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
            return Err(ApiError::bad(UiMessage::error_api_hosted_office_office_artifact_cache_capacity()));
        }
        let mut registered = registered.lock().map_err(|_| failed())?;
        if inner.office.closing || initial_cancel.load(Ordering::Acquire) {
            return Err(failed());
        }
        if !inner
            .core
            .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
                core.actor_output_claim(&captured.request_id, &fp)
            })
            .map_err(HostedInner::core_error)?
        {
            return Err(failed());
        }
        ensure_owned(stage_root(&location.0)?,&location.0,true)?;
        durable_write(&location.0.join("claim.json"),&serde_json::to_vec(&json!({"requestId":captured.request_id,"actor":actor,"epoch":cut.job.epoch,"fingerprint":fp,"format":captured.format})).map_err(|_|failed())?)?;
        inner.office.jobs.insert(
            (actor.clone(), captured.request_id.clone()),
            cut.job.clone(),
        );
        let key = (actor.clone(), captured.request_id.clone());
        inner.office.active.insert(
            key.clone(),
            LegacyImportRun {
                actor_epoch: cut.job.epoch,
                cancel: initial_cancel,
            },
        );
        *registered = Some(key);
        Ok((actor, Some(cut), location, flat, None))
    })
    .await?;
    if let Some(answer) = answer {
        return Ok(Json(answer));
    }
    let Some(mut cut) = cut else {
        return Ok(Json(
            recover_legacy(&state, &auth, &req.request_id, Some(fingerprint))
                .await?
                .unwrap_or_else(|| unknown_answer(&req.request_id, "stageNotRegistered")),
        ));
    };
    let key = (actor.clone(), req.request_id.clone());
    let epoch = cut.job.epoch;
    let render = if let Some(worker) = cut.worker.take() {
        Some(editor::compile_frozen(&state, worker).await)
    } else {
        None
    };
    let evidence = Arc::new(LegacyEvidence {
        witnesses: std::mem::take(&mut cut.witnesses),
        environment: cut.environment.take(),
        citations: cut.citations.clone(),
        epoch,
    });
    let job = cut.job.clone();
    let mut docx_req = req.clone();
    docx_req.format = "docx".into();
    let docx: Result<_, ApiError> = async {
        let render = render.transpose()?;
        crate::work::spawn_blocking(move || convert(docx_req, cut, render))
            .await
            .map_err(|_| failed())?
    }
    .await;
    let (_, docx) =
        deliver_legacy_result(&state, &auth, &actor, &req, evidence.clone(), docx).await?;
    let stage = LegacyStage {
        schema_version: 1,
        conversion_layout: Some("flatV1".into()),
        actor: actor.clone(),
        epoch,
        fingerprint: fingerprint.clone(),
        expires_at: unix_now()
            + job
                .expires
                .saturating_duration_since(Instant::now())
                .as_secs(),
        request: req.clone(),
        needs: job.needs.clone(),
        preview: job.preview.clone(),
        docx_sha256: digest(&docx.bytes),
        docx_length: docx.bytes.len(),
        losses: docx.losses.clone(),
        summary: docx.summary.clone(),
    };
    let (dir, _) = location;
    let conversions = flat.root;
    let save_dir = dir.clone();
    let save_stage = stage.clone();
    let save_bytes = docx.bytes.clone();
    let staged =
        crate::work::spawn_blocking(move || write_stage(&save_dir, &save_stage, &save_bytes))
            .await
            .map_err(|_| failed())
            .and_then(|result| result);
    deliver_legacy_result(&state, &auth, &actor, &req, evidence.clone(), staged).await?;
    ensure_owned(stage_root(&dir)?, &conversions, true)?;
    let result = super::super::legacy_office::convert_once(
        &flat.request_id,
        legacy_request(&req, docx.bytes),
        &conversions,
        cancel.clone(),
    )
    .await
    .map_err(|_| failed());
    let (mut job, result) =
        deliver_legacy_result(&state, &auth, &actor, &req, evidence.clone(), result).await?;
    let mut metadata = result.metadata();
    metadata["requestId"] = json!(req.request_id);
    job.preview["legacy"] = metadata;
    job.preview["executionLocation"] = json!("server");
    if result.status != "succeeded" {
        job.status = match result.status.as_str() {
            "cancelled" => "cancelled",
            "failed" | "unavailable" => "failed",
            _ => "unknown",
        };
        job.reserved = 0;
        return locked(&state, move |inner| {
            if inner.actor(&auth)? != actor || inner.actor_generation(&actor) != epoch {
                return Err(expired());
            }
            access(inner, &actor, &job.needs, false)?;
            witness_check(inner, &evidence.witnesses)?;
            if let Some(c) = &evidence.citations {
                c.validate(inner, &actor)?
            }
            if let Some(env) = &evidence.environment {
                env.recheck(inner, &actor)?
            }
            if let Some(source) = &req.source {
                check_draft(inner, &actor, &req, source)?
            }
            let answer = response(&key.1, &job);
            inner.office.jobs.insert(key, job);
            Ok(Json(answer))
        })
        .await;
    }
    let generated = result.generated.as_ref().ok_or_else(failed)?;
    let mut losses = stage.losses;
    losses.extend(
        result
            .public_losses()
            .into_iter()
            .map(|l| weftext_export::ExportLoss {
                code: l.code,
                description: l.description,
                product_diagnostics: l.product_diagnostics,
                location: l.location,
            }),
    );
    // Original output meaning stays typed. The existing conversion metadata
    // independently carries the exact backend, format and typed result.
    let summary = stage.summary;
    let artifact = ExportArtifact {
        bytes: generated.bytes.clone(),
        content_type: if req.format == "doc" {
            "application/msword"
        } else {
            "application/octet-stream"
        }
        .into(),
        extension: req.format.clone(),
        losses: losses.clone(),
        summary: summary.clone(),
    };
    job.reserved =
        artifact.bytes.len() + serde_json::to_vec(&job.preview).map_or(MAX_STORE, |b| b.len());
    job.digest = Some(generated.sha256.clone());
    job.artifact = Some(Arc::new(artifact));
    job.status = "prepared";
    let ready = LegacyReady {
        fingerprint,
        output_sha256: generated.sha256.clone(),
        output_length: generated.bytes.len(),
        preview: job.preview.clone(),
        losses,
        summary,
    };
    let ready_bytes = serde_json::to_vec(&ready).map_err(|_| failed())?;
    if ready_bytes.len() > MAX_STORE {
        return Err(failed());
    }
    let publication = serde_json::to_vec(&LegacyPublication {
        fingerprint: ready.fingerprint.clone(),
        ready_sha256: digest(&ready_bytes),
    })
    .map_err(|_| failed())?;
    let metadata_dir = dir.clone();
    crate::work::spawn_blocking(move || {
        ensure_owned(stage_root(&metadata_dir)?, &metadata_dir, false)?;
        durable_write(&metadata_dir.join("ready.json"), &ready_bytes)
    })
    .await
    .map_err(|_| failed())??;
    // The small publication seal is written only while the original authority
    // and freeze witnesses are still validated under the policy lock.
    locked(&state, move |inner| {
        if inner.actor(&auth)? != actor || inner.actor_generation(&actor) != epoch {
            return Err(expired());
        }
        access(inner, &actor, &job.needs, false)?;
        witness_check(inner, &evidence.witnesses)?;
        if let Some(c) = &evidence.citations {
            c.validate(inner, &actor)?
        }
        if let Some(env) = &evidence.environment {
            env.recheck(inner, &actor)?
        }
        if let Some(source) = &req.source {
            check_draft(inner, &actor, &req, source)?
        }
        if cancel.load(Ordering::Acquire) {
            return Err(failed());
        }
        let used = inner
            .office
            .jobs
            .iter()
            .filter(|(k, _)| *k != &key)
            .map(|(_, j)| j.reserved)
            .sum::<usize>();
        if used.saturating_add(job.reserved) > MAX_STORE {
            return Err(failed());
        }
        ensure_owned(stage_root(&dir)?, &dir, false)?;
        durable_write(&dir.join("published.json"), &publication)?;
        let answer = response(&key.1, &job);
        inner.office.jobs.insert(key, job);
        Ok(Json(answer))
    })
    .await
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
async fn restore_legacy_if_needed(
    state: &HostedState,
    auth: &Auth,
    request: &str,
) -> Result<Option<Value>, ApiError> {
    let check = auth.clone();
    let request_owned = request.to_owned();
    let needed = locked(state, move |inner| {
        let actor = inner.actor(&check)?;
        Ok(lookup(inner, &actor, &request_owned, false)?.is_none_or(|j| j.status == "unknown"))
    })
    .await?;
    if needed {
        recover_legacy(state, auth, request, None).await
    } else {
        Ok(None)
    }
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
struct ExportDelivery {
    witnesses: Vec<(String, weftext_core::SnapshotWitness)>,
    environment: Option<library::Environment>,
    citations: Option<citations::Capture>,
    native_witness: Option<Arc<weftext_core::NativeExportWitness>>,
}
fn deliver_export_result<T>(
    inner: &mut HostedInner,
    auth: &Auth,
    actor: &str,
    key: &(String, String),
    req: &Prepare,
    evidence: &ExportDelivery,
    converted: Result<T, ApiError>,
) -> Result<(Job, T), ApiError> {
    if inner.actor(auth)? != actor {
        return Err(expired());
    }
    let job = lookup(inner, actor, &key.1, false)?
        .filter(|j| j.status == "pending")
        .ok_or_else(failed)?;
    witness_check(inner, &evidence.witnesses)?;
    if let Some(citations) = &evidence.citations {
        citations.validate(inner, actor)?;
    }
    if let Some(w) = &evidence.native_witness {
        inner
            .core
            .validate_native_export_witness(&inner.config.root, w)
            .map_err(|_| failed())?;
    }
    if let Some(source) = &req.source {
        check_draft(inner, actor, req, source)?;
    }
    if let Some(environment) = &evidence.environment {
        environment.recheck(inner, actor)?;
    }
    Ok((job, converted?))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Prepare>,
) -> ApiResult {
    validate(&req)?;
    if legacy_format(&req.format) {
        return prepare_legacy(state, auth, req, true).await;
    }
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
                    UiMessage::error_api_hosted_office_output_request_input_conflict(),
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
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_office_office_artifact_cache_capacity(),
            ));
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
    let evidence = ExportDelivery {
        witnesses: std::mem::take(&mut cut.witnesses),
        environment: cut.environment.clone(),
        citations: cut.citations.clone(),
        native_witness: cut.native_witness.clone(),
    };
    let delivery_req = req.clone();
    let converted: Result<_, ApiError> = async {
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
                    req.output_locale,
                )?;
                cut.additional_losses.extend(losses);
                if source != worker.source {
                    worker.source = source;
                    worker.frozen_files.insert(
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
        crate::work::spawn_blocking(move || convert(req, cut, render))
            .await
            .map_err(|_| failed())?
    }
    .await;
    locked(&state, move |inner| {
        let (mut job, artifact) = deliver_export_result(
            inner,
            &auth,
            &actor,
            &key,
            &delivery_req,
            &evidence,
            converted,
        )?;
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
    RequestJson(req): RequestJson<RequestId>,
) -> ApiResult {
    if let Some(answer) = restore_legacy_if_needed(&state, &auth, &req.request_id).await? {
        return Ok(Json(answer));
    }
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
    RequestJson(req): RequestJson<Confirm>,
) -> ApiResult {
    restore_legacy_if_needed(&state, &auth, &req.request_id).await?;
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
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_office_exact_artifact_confirmation_required(),
            ));
        }
        if legacy_format(&job.needs.format) {
            let (dir, _) = legacy_location(inner, &actor, &req.request_id)?;
            ensure_owned(stage_root(&dir)?, &dir, false)?;
            let fingerprint = inner
                .core
                .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
                    core.actor_output_fingerprint(&req.request_id)
                })
                .map_err(HostedInner::core_error)?
                .ok_or_else(failed)?;
            let confirmation = LegacyConfirmed {
                fingerprint,
                digest: req.digest.clone(),
                accepted_losses: req.accept_losses,
            };
            let path = dir.join("confirmed.json");
            if path.try_exists().unwrap_or(true) {
                let old: LegacyConfirmed =
                    serde_json::from_slice(&bounded_read(&dir, "confirmed.json", 1024)?)
                        .map_err(|_| failed())?;
                if old.fingerprint != confirmation.fingerprint
                    || old.digest != confirmation.digest
                    || (!artifact.losses.is_empty() && !old.accepted_losses)
                {
                    return Err(failed());
                }
            } else {
                durable_write(
                    &path,
                    &serde_json::to_vec(&confirmation).map_err(|_| failed())?,
                )?;
            }
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
    RequestJson(req): RequestJson<OutputChunk>,
) -> ApiResult {
    restore_legacy_if_needed(&state, &auth, &req.request_id).await?;
    locked(&state,move|inner|{
        let actor=inner.actor(&auth)?;
        let job=lookup(inner,&actor,&req.request_id,true)?.filter(|j|j.status=="confirmed").ok_or_else(failed)?;
        let artifact=job.artifact.ok_or_else(failed)?;
        if req.offset>artifact.bytes.len() {return Err(ApiError::bad(UiMessage::error_api_hosted_office_artifact_download_offset_invalid()));}
        let end=req.offset.saturating_add(64*1024).min(artifact.bytes.len());let bytes=&artifact.bytes[req.offset..end];
        Ok(Json(json!({"requestId":req.request_id,"bytesBase64":base64::engine::general_purpose::STANDARD.encode(bytes),"offset":req.offset,"length":bytes.len(),"total":artifact.bytes.len(),"sha256":job.digest,"done":end==artifact.bytes.len()})))
    }).await
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn request(format: &str) -> Prepare {
        serde_json::from_value(json!({"requestId":uuid::Uuid::new_v4(),"outputLocale":"zh-CN","format":format,"reviewMode":"none","includeResolved":false,"stylePolicy":"template","bindings":{}})).unwrap()
    }
    fn seed_legacy(
        state: &HostedState,
        publish: bool,
    ) -> (Prepare, PathBuf, PathBuf, LegacyStage, Vec<u8>) {
        let mut inner = state.inner.lock().unwrap();
        let owned = inner.config.root.join(".legacy-test-output");
        inner.office = Store::with_legacy_root(owned);
        let grant = inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap();
        grant.office.export = false;
        grant.office.download = false;
        grant.office.doc = LegacyFormatRights {
            export: true,
            download: true,
            r#import: false,
        };
        let mut req = request("doc");
        req.resource = Some("Shared".into());
        let cut = capture(&mut inner, "alice", &req).unwrap();
        let fingerprint = digest(&serde_json::to_vec(&req).unwrap());
        let actor_scope = scope(&inner, "alice");
        let root = inner.config.root.clone();
        inner
            .core
            .with_actor_for_root(&actor_scope, &root, |core| {
                core.actor_output_claim(&req.request_id, &fingerprint)
            })
            .unwrap();
        let (dir, conversions) = legacy_location(&inner, "alice", &req.request_id).unwrap();
        let bytes = b"PK\x03\x04frozen test input, no external program".to_vec();
        let stage = LegacyStage {
            schema_version: 1,
            conversion_layout: None,
            actor: "alice".into(),
            epoch: inner.actor_generation("alice"),
            fingerprint,
            expires_at: unix_now() + TTL.as_secs(),
            request: req.clone(),
            needs: cut.job.needs,
            preview: json!({"format":"doc","frozenSource":"exact frozen body"}),
            docx_sha256: digest(&bytes),
            docx_length: bytes.len(),
            losses: vec![weftext_export::ExportLoss {
                code: "frozenLoss".into(),
                description: ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: req.request_id.clone(),
                    },
                )),
                product_diagnostics: vec![],
                location: None,
            }],
            summary: UiMessage::common_operation(weftext_messages::params::CommonOperation {
                operation_id: req.request_id.clone(),
            }),
        };
        write_stage(&dir, &stage, &bytes).unwrap();
        let once = conversions.join(&req.request_id);
        std::fs::create_dir_all(&once).unwrap();
        let once = once.canonicalize().unwrap();
        let original = once.join("original.docx");
        std::fs::write(&original, &bytes).unwrap();
        let generated = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1exact synthetic generated DOC".to_vec();
        let output = once.join("generated.doc");
        std::fs::write(&output, &generated).unwrap();
        let make_artifact = |filename: &str, path: PathBuf, data: &[u8]| {
            super::super::super::legacy_office::Artifact {
                filename: filename.into(),
                sha256: digest(data),
                length: data.len(),
                retained_path: path,
                bytes: vec![],
            }
        };
        let result = super::super::super::legacy_office::ConversionResult {
            request_id: req.request_id.clone(),
            status: "succeeded".into(),
            code: "converted".into(),
            description: ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: req.request_id.clone(),
                },
            )),
            backend: "libreoffice".into(),
            details: None,
            backend_version: Some("fixture".into()),
            direction: super::super::super::legacy_office::Direction::DocxToDoc,
            original: make_artifact("original.docx", original.clone(), &bytes),
            intermediate_docx: Some(make_artifact("original.docx", original, &bytes)),
            generated: Some(make_artifact("generated.doc", output, &generated)),
            features: vec![],
            workspace: once.clone(),
        };
        let conversion_fingerprint=digest(json!({"version":1,"backend":"libreoffice","direction":"docxToDoc","sha256":digest(&bytes),"length":bytes.len()}).to_string().as_bytes());
        std::fs::write(once.join("request.json"),serde_json::to_vec(&json!({"schemaVersion":1,"requestId":req.request_id,"fingerprint":conversion_fingerprint,"backend":"libreoffice","direction":"docxToDoc","inputSha256":digest(&bytes),"inputLength":bytes.len()})).unwrap()).unwrap();
        std::fs::write(
            once.join("completed.json"),
            serde_json::to_vec(
                &json!({"schemaVersion":1,"fingerprint":conversion_fingerprint,"result":result}),
            )
            .unwrap(),
        )
        .unwrap();
        if publish {
            let ready = LegacyReady {
                fingerprint: stage.fingerprint.clone(),
                output_sha256: digest(&generated),
                output_length: generated.len(),
                preview: stage.preview.clone(),
                losses: stage.losses.clone(),
                summary: UiMessage::common_operation(weftext_messages::params::CommonOperation {
                    operation_id: req.request_id.clone(),
                }),
            };
            durable_write(
                &dir.join("ready.json"),
                &serde_json::to_vec(&ready).unwrap(),
            )
            .unwrap();
            durable_write(
                &dir.join("published.json"),
                &serde_json::to_vec(&LegacyPublication {
                    fingerprint: ready.fingerprint.clone(),
                    ready_sha256: digest(&serde_json::to_vec(&ready).unwrap()),
                })
                .unwrap(),
            )
            .unwrap();
        }
        (req, dir, conversions, stage, generated)
    }
    #[test]
    fn export_success_and_failure_share_actual_grant_source_and_epoch_rechecks() {
        for failure in [false, true] {
            for mutation in [0, 1, 2] {
                let (_temp, state, auth, _, _, _) = history_api::test_fixture();
                let mut inner = state.inner.lock().unwrap();
                inner
                    .policy
                    .users
                    .get_mut("alice")
                    .unwrap()
                    .grants
                    .get_mut("Shared")
                    .unwrap()
                    .office
                    .export = true;
                let mut req = request("docx");
                req.resource = Some("Shared".into());
                let mut cut = capture(&mut inner, "alice", &req).unwrap();
                let key = ("alice".into(), req.request_id.clone());
                inner.office.jobs.insert(key.clone(), cut.job);
                let evidence = ExportDelivery {
                    witnesses: std::mem::take(&mut cut.witnesses),
                    environment: cut.environment,
                    citations: cut.citations,
                    native_witness: cut.native_witness,
                };
                let result = || {
                    if failure {
                        Err(crate::worker_failure::diagnostics_tests::original_failure(
                            crate::worker_failure::Kind::OutputRejected,
                        ))
                    } else {
                        Ok(json!({"value":"protected"}))
                    }
                };
                assert_eq!(
                    deliver_export_result(
                        &mut inner,
                        &auth,
                        "alice",
                        &key,
                        &req,
                        &evidence,
                        result()
                    )
                    .is_err(),
                    failure
                );
                match mutation {
                    0 => {
                        inner
                            .policy
                            .users
                            .get_mut("alice")
                            .unwrap()
                            .grants
                            .get_mut("Shared")
                            .unwrap()
                            .office
                            .export = false
                    }
                    1 => std::fs::write(
                        inner.config.root.join("Shared/Shared.typ"),
                        "changed saved source",
                    )
                    .unwrap(),
                    _ => inner.policy.users.get_mut("alice").unwrap().epoch += 1,
                }
                let error = deliver_export_result(
                    &mut inner,
                    &auth,
                    "alice",
                    &key,
                    &req,
                    &evidence,
                    result(),
                )
                .err()
                .unwrap();
                assert!(error.private_worker_failure.is_none());
            }
        }
    }
    #[tokio::test]
    async fn legacy_worker_success_and_failure_share_the_original_grant_recheck() {
        for failure in [false, true] {
            let (_temp, state, auth, _, _, _) = history_api::test_fixture();
            let (req, _, _, _, _) = seed_legacy(&state, false);
            let evidence = {
                let mut inner = state.inner.lock().unwrap();
                let mut cut = capture(&mut inner, "alice", &req).unwrap();
                let epoch = cut.job.epoch;
                inner
                    .office
                    .jobs
                    .insert(("alice".into(), req.request_id.clone()), cut.job);
                Arc::new(LegacyEvidence {
                    witnesses: std::mem::take(&mut cut.witnesses),
                    environment: cut.environment,
                    citations: cut.citations,
                    epoch,
                })
            };
            let result = || {
                if failure {
                    Err(crate::worker_failure::diagnostics_tests::original_failure(
                        crate::worker_failure::Kind::OutputRejected,
                    ))
                } else {
                    Ok(json!({"value":"protected"}))
                }
            };
            assert_eq!(
                deliver_legacy_result(&state, &auth, "alice", &req, evidence.clone(), result())
                    .await
                    .is_err(),
                failure
            );
            state
                .inner
                .lock()
                .unwrap()
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap()
                .office
                .doc
                .export = false;
            let error = deliver_legacy_result(&state, &auth, "alice", &req, evidence, result())
                .await
                .err()
                .unwrap();
            assert!(error.private_worker_failure.is_none());
        }
    }
    #[tokio::test]
    async fn legacy_actual_stage_io_failure_still_rechecks_the_frozen_grant() {
        let (_temp, state, auth, _, _, _) = history_api::test_fixture();
        let (req, dir, _, stage, _generated) = seed_legacy(&state, false);
        let evidence = {
            let mut inner = state.inner.lock().unwrap();
            let mut cut = capture(&mut inner, "alice", &req).unwrap();
            let epoch = cut.job.epoch;
            inner
                .office
                .jobs
                .insert(("alice".into(), req.request_id.clone()), cut.job);
            Arc::new(LegacyEvidence {
                witnesses: std::mem::take(&mut cut.witnesses),
                environment: cut.environment,
                citations: cut.citations,
                epoch,
            })
        };
        // The owned create-only stage already exists. This is an actual disk
        // publication failure, with original frozen bytes retained.
        let before = std::fs::read(dir.join("frozen.docx")).unwrap();
        let failure = write_stage(&dir, &stage, &before);
        assert!(failure.is_err());
        state
            .inner
            .lock()
            .unwrap()
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .office
            .doc
            .export = false;
        let error = deliver_legacy_result(&state, &auth, "alice", &req, evidence, failure)
            .await
            .err()
            .unwrap();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(std::fs::read(dir.join("frozen.docx")).unwrap(), before);
    }
    #[test]
    fn legacy_formats_require_separate_grants_and_uploaded_docx_inspection_does_not_grant_docx_output()
     {
        let (_temp, state, _auth, _doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let grant = inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap();
        grant.office = OfficeRights {
            doc: LegacyFormatRights {
                export: true,
                download: false,
                r#import: false,
            },
            ..Default::default()
        };
        assert!(format_access(grant, "doc", false));
        assert!(!format_access(grant, "doc", true));
        assert!(!format_access(grant, "wps", false));
        assert!(!format_access(grant, "docx", false));
        assert!(inspect_access(&inner, "alice", "docx").is_ok());
        let mut req = request("doc");
        req.resource = Some("Shared".into());
        assert!(capture(&mut inner, "alice", &req).is_ok());
        req.format = "docx".into();
        assert!(capture(&mut inner, "alice", &req).is_err());
        assert!(!valid_legacy_rights(&OfficeRights {
            doc: LegacyFormatRights {
                download: true,
                ..Default::default()
            },
            ..Default::default()
        }));
    }
    #[test]
    fn legacy_stages_are_actor_generation_scoped_and_reject_changed_bytes_or_partial_seals() {
        let (_temp, state, _auth, _doc, _, _) = history_api::test_fixture();
        let (req, dir, _conversions, _stage, _generated) = seed_legacy(&state, false);
        assert!(read_stage(&dir).is_ok());
        let mut inner = state.inner.lock().unwrap();
        assert_ne!(
            legacy_location(&inner, "alice", &req.request_id).unwrap().0,
            legacy_location(&inner, "bob", &req.request_id).unwrap().0
        );
        inner.policy.users.get_mut("alice").unwrap().epoch += 1;
        assert_ne!(
            dir,
            legacy_location(&inner, "alice", &req.request_id).unwrap().0
        );
        drop(inner);
        std::fs::write(dir.join("frozen.docx"), b"PK\x03\x04changed").unwrap();
        assert!(read_stage(&dir).is_err());
        std::fs::write(dir.join("stage.sha256"), b"partial").unwrap();
        assert!(read_stage(&dir).is_err());
    }
    #[tokio::test]
    async fn legacy_published_recovery_uses_exact_artifact_without_old_sessions_and_current_download_permission()
     {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, _dir, _conversions, stage, generated) = seed_legacy(&state, true);
        {
            let mut inner = state.inner.lock().unwrap();
            inner.sessions.clear();
            std::fs::write(
                inner.config.root.join("Shared/Shared.typ"),
                "later source body",
            )
            .unwrap();
        }
        let answer = recover_legacy(
            &state,
            &auth,
            &req.request_id,
            Some(stage.fingerprint.clone()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(answer["status"], "prepared");
        assert_eq!(answer["digest"], digest(&generated));
        assert_eq!(answer["losses"][0]["code"], "frozenLoss");
        assert!(!answer.to_string().contains(".legacy-test-output"));
        let Json(_) = confirm(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Confirm {
                request_id: req.request_id.clone(),
                digest: digest(&generated),
                accept_losses: true,
            }),
        )
        .await
        .unwrap();
        state.inner.lock().unwrap().office.jobs.clear();
        let Json(restored) = status(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(RequestId {
                request_id: req.request_id.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(restored["status"], "confirmed");
        let Json(bytes) = chunk(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(OutputChunk {
                request_id: req.request_id.clone(),
                offset: 0,
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(bytes["bytesBase64"].as_str().unwrap())
                .unwrap(),
            generated
        );
        {
            let mut inner = state.inner.lock().unwrap();
            let restored = inner.office.jobs[&("alice".into(), req.request_id.clone())].clone();
            assert!(
                restored.expires
                    <= Instant::now() + Duration::from_secs(stage.expires_at - unix_now() + 1)
            );
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap()
                .office
                .doc
                .download = false;
        }
        assert!(
            chunk(
                State(state),
                Extension(auth),
                crate::RequestJson(OutputChunk {
                    request_id: req.request_id,
                    offset: 0
                })
            )
            .await
            .is_err()
        );
    }
    #[tokio::test]
    async fn legacy_flat_layout_uses_scoped_wrapper_and_never_falls_back_to_old_execution() {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, dir, old_root, mut stage, generated) = seed_legacy(&state, true);
        let flat = {
            let inner = state.inner.lock().unwrap();
            let flat = flat_legacy_location(&inner, "alice", stage.epoch, &req.request_id).unwrap();
            let bob = flat_legacy_location(&inner, "bob", stage.epoch, &req.request_id).unwrap();
            assert_eq!(flat.root, bob.root);
            assert_ne!(flat.request_id, bob.request_id);
            flat
        };
        stage.conversion_layout = Some("flatV1".into());
        let encoded = serde_json::to_vec(&stage).unwrap();
        std::fs::write(dir.join("stage.json"), &encoded).unwrap();
        std::fs::write(dir.join("stage.sha256"), digest(&encoded)).unwrap();
        let unknown = recover_legacy(&state, &auth, &req.request_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unknown["status"], "unknown");
        assert!(!flat.root.exists());
        ensure_owned(stage_root(&dir).unwrap(), &flat.root, true).unwrap();
        let wrapper = flat.root.join(&flat.request_id);
        std::fs::create_dir(&wrapper).unwrap();
        let wrapper = wrapper.canonicalize().unwrap();
        let old_wrapper = old_root.join(&req.request_id);
        let mut request_record: Value =
            serde_json::from_slice(&std::fs::read(old_wrapper.join("request.json")).unwrap())
                .unwrap();
        request_record["requestId"] = json!(flat.request_id);
        let mut completed: Value =
            serde_json::from_slice(&std::fs::read(old_wrapper.join("completed.json")).unwrap())
                .unwrap();
        completed["result"]["requestId"] = json!(flat.request_id);
        completed["result"]["workspace"] = json!(wrapper);
        for key in ["original", "intermediateDocx", "generated"] {
            let old_path =
                PathBuf::from(completed["result"][key]["retainedPath"].as_str().unwrap());
            let new_path = wrapper.join(old_path.file_name().unwrap());
            if !new_path.exists() {
                std::fs::write(&new_path, std::fs::read(old_path).unwrap()).unwrap();
            }
            completed["result"][key]["retainedPath"] = json!(new_path);
        }
        std::fs::write(
            wrapper.join("request.json"),
            serde_json::to_vec(&request_record).unwrap(),
        )
        .unwrap();
        std::fs::write(
            wrapper.join("completed.json"),
            serde_json::to_vec(&completed).unwrap(),
        )
        .unwrap();
        let recovered = recover_legacy(&state, &auth, &req.request_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered["status"], "prepared");
        assert_eq!(recovered["requestId"], req.request_id);
        assert_eq!(recovered["digest"], digest(&generated));
        assert!(old_wrapper.join("completed.json").is_file());
        assert!(!recovered.to_string().contains(&flat.request_id));
    }
    #[tokio::test]
    async fn legacy_unpublished_completion_and_missing_worker_receipt_remain_unknown_without_execution()
     {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, dir, conversions, _stage, _generated) = seed_legacy(&state, true);
        std::fs::remove_file(dir.join("published.json")).unwrap();
        let answer = recover_legacy(&state, &auth, &req.request_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(answer["status"], "unknown");
        assert_eq!(answer["code"], "publicationUnknown");
        assert!(dir.join("ready.json").exists());
        assert!(!dir.join("published.json").exists());
        std::fs::remove_file(conversions.join(&req.request_id).join("completed.json")).unwrap();
        let before = std::fs::read(conversions.join(&req.request_id).join("request.json")).unwrap();
        let answer = recover_legacy(&state, &auth, &req.request_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(answer["status"], "unknown");
        assert_eq!(
            before,
            std::fs::read(conversions.join(&req.request_id).join("request.json")).unwrap()
        );
        let mut changed = req.clone();
        changed.source = Some("another exact input".into());
        assert!(prepare_legacy(state, auth, changed, true).await.is_err());
    }
    #[tokio::test]
    async fn legacy_expired_stage_never_extends_lifetime_or_restarts_worker() {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, dir, conversions, mut stage, _generated) = seed_legacy(&state, true);
        stage.expires_at = unix_now() - 1;
        let encoded = serde_json::to_vec(&stage).unwrap();
        std::fs::write(dir.join("stage.json"), &encoded).unwrap();
        std::fs::write(dir.join("stage.sha256"), digest(&encoded)).unwrap();
        std::fs::remove_file(conversions.join(&req.request_id).join("completed.json")).unwrap();
        let answer = recover_legacy(&state, &auth, &req.request_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(answer["status"], "expired");
        assert!(
            !conversions
                .join(&req.request_id)
                .join("completed.json")
                .exists()
        );
    }
    #[tokio::test]
    async fn legacy_large_accepted_preview_remains_recoverable() {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, dir, _conversions, stage, generated) = seed_legacy(&state, false);
        let ready = LegacyReady {
            fingerprint: stage.fingerprint,
            output_sha256: digest(&generated),
            output_length: generated.len(),
            preview: json!({"frozenSource":"s".repeat(5*1024*1024)}),
            losses: stage.losses,
            summary: UiMessage::common_operation(weftext_messages::params::CommonOperation {
                operation_id: req.request_id.clone(),
            }),
        };
        durable_write(
            &dir.join("ready.json"),
            &serde_json::to_vec(&ready).unwrap(),
        )
        .unwrap();
        durable_write(
            &dir.join("published.json"),
            &serde_json::to_vec(&LegacyPublication {
                fingerprint: ready.fingerprint.clone(),
                ready_sha256: digest(&serde_json::to_vec(&ready).unwrap()),
            })
            .unwrap(),
        )
        .unwrap();
        let answer = recover_legacy(&state, &auth, &req.request_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(answer["status"], "prepared");
        assert_eq!(
            answer["preview"]["frozenSource"].as_str().unwrap().len(),
            5 * 1024 * 1024
        );
    }
    #[test]
    fn legacy_stage_rejects_ancestor_junction_without_touching_target() {
        use std::os::windows::process::CommandExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("owned");
        let outside = temp.path().join("outside");
        let namespace = root.join("a".repeat(64)).join("epoch-1");
        std::fs::create_dir_all(&namespace).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let link = namespace.join("exports");
        let result=std::process::Command::new("powershell.exe").args(["-NoLogo","-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:WEFTEXT_TEST_LINK -Target $env:WEFTEXT_TEST_TARGET | Out-Null"]).env("WEFTEXT_TEST_LINK",&link).env("WEFTEXT_TEST_TARGET",&outside).creation_flags(0x08000000).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(ensure_owned(&root, &link, false).is_err());
        assert!(ensure_owned(&root, &link.join(uuid::Uuid::new_v4().to_string()), true).is_err());
        assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
    }
    #[test]
    fn revoked_epoch_cancels_owned_export_and_import_workers() {
        let (_temp, state, _auth, _doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let epoch = inner.actor_generation("alice");
        let export = Arc::new(AtomicBool::new(false));
        let import = Arc::new(AtomicBool::new(false));
        inner.office.active.insert(
            ("alice".into(), uuid::Uuid::new_v4().to_string()),
            LegacyImportRun {
                actor_epoch: epoch,
                cancel: export.clone(),
            },
        );
        inner.office.import_active.insert(
            ("alice".into(), uuid::Uuid::new_v4().to_string()),
            LegacyImportRun {
                actor_epoch: epoch,
                cancel: import.clone(),
            },
        );
        inner.policy.users.get_mut("alice").unwrap().epoch += 1;
        expire_revoked(&mut inner);
        assert!(export.load(Ordering::Acquire));
        assert!(import.load(Ordering::Acquire));
        assert!(inner.office.active.is_empty());
        assert!(inner.office.import_active.is_empty());
    }
    #[tokio::test]
    async fn legacy_live_worker_delivery_rechecks_saved_source_and_current_grants() {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, _dir, _conversions, _stage, _generated) = seed_legacy(&state, false);
        let evidence = {
            let mut inner = state.inner.lock().unwrap();
            let mut cut = capture(&mut inner, "alice", &req).unwrap();
            let epoch = cut.job.epoch;
            inner
                .office
                .jobs
                .insert(("alice".into(), req.request_id.clone()), cut.job);
            Arc::new(LegacyEvidence {
                witnesses: std::mem::take(&mut cut.witnesses),
                environment: cut.environment,
                citations: cut.citations,
                epoch,
            })
        };
        assert!(
            recheck_legacy(&state, &auth, "alice", &req, evidence.clone())
                .await
                .is_ok()
        );
        {
            let mut inner = state.inner.lock().unwrap();
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap()
                .office
                .doc
                .export = false;
        }
        assert!(
            recheck_legacy(&state, &auth, "alice", &req, evidence.clone())
                .await
                .is_err()
        );
        {
            let mut inner = state.inner.lock().unwrap();
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap()
                .office
                .doc
                .export = true;
            std::fs::write(
                inner.config.root.join("Shared/Shared.typ"),
                "a late saved edit",
            )
            .unwrap();
        }
        assert!(
            recheck_legacy(&state, &auth, "alice", &req, evidence)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn legacy_cancel_and_request_drop_signal_only_owned_run_and_leave_unknown() {
        let (_temp, state, auth, _doc, _, _) = history_api::test_fixture();
        let (req, _dir, _conversions, _stage, _generated) = seed_legacy(&state, false);
        let cancel = Arc::new(AtomicBool::new(false));
        let key = ("alice".into(), req.request_id.clone());
        {
            let mut inner = state.inner.lock().unwrap();
            let cut = capture(&mut inner, "alice", &req).unwrap();
            let epoch = cut.job.epoch;
            inner.office.jobs.insert(key.clone(), cut.job);
            inner.office.active.insert(
                key.clone(),
                LegacyImportRun {
                    actor_epoch: epoch,
                    cancel: cancel.clone(),
                },
            );
        }
        let Json(answer) = legacy_cancel(
            State(state.clone()),
            Extension(auth),
            crate::RequestJson(RequestId {
                request_id: req.request_id.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(answer["status"], "cancelling");
        assert!(cancel.load(Ordering::Acquire));
        drop(LegacyRunGuard {
            state: state.clone(),
            key: Arc::new(Mutex::new(Some(key.clone()))),
            cancel,
        });
        let inner = state.inner.lock().unwrap();
        assert_eq!(inner.office.jobs[&key].status, "unknown");
        assert!(inner.office.active.is_empty());
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
                    ..Default::default()
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
        // The selected output language is an explicit part of the exact request,
        // independent of UI preferences; omission is not an implicit default.
        let frozen = json!({"requestId":uuid::Uuid::new_v4().to_string(),"outputLocale":"zh-CN","resource":null,"source":null,"format":"xlsx","reviewMode":"none","includeResolved":false,"queryId":"originalQuery","template":null,"stylePolicy":"template","bindings":{"fixed":{"kind":"text","text":"exact"}}});
        let parsed: Prepare = serde_json::from_value(frozen.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), frozen);
        let mut missing = frozen.clone();
        missing.as_object_mut().unwrap().remove("outputLocale");
        assert!(serde_json::from_value::<Prepare>(missing).is_err());
        let mut selected = frozen.clone();
        selected["outputLocale"] = json!("en-US");
        assert_ne!(
            digest(&serde_json::to_vec(&frozen).unwrap()),
            digest(&serde_json::to_vec(&selected).unwrap())
        );
        selected["outputLocale"] = json!("en");
        assert!(serde_json::from_value::<Prepare>(selected).is_err());
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

    include!("office_query_input_tests.rs");
}
