//! Recognition producer grants never imply future author/resource/field rights.
use super::*;
use crate::recognition_api as api;
use axum::extract::DefaultBodyLimit;

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) run: bool,
    #[serde(default)]
    pub(super) read: bool,
    #[serde(default)]
    pub(super) r#import: bool,
    #[serde(default)]
    pub(super) download: bool,
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:recognition", inner.actor_scope(actor))
}
fn access(inner: &HostedInner, actor: &str, slot: &str, action: &str) -> Result<(), ApiError> {
    let r = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.content_imports.get(slot))
        .ok_or_else(unavailable)?;
    let r = &r.recognition;
    if !inner.config.content_import_slots.contains_key(slot)
        || !match action {
            "run" => r.run && r.read,
            "read" => r.read,
            "import" => r.r#import && r.read,
            "download" => r.download && r.read,
            _ => false,
        }
    {
        return Err(unavailable());
    }
    Ok(())
}
fn context(
    inner: &HostedInner,
    actor: &str,
    target: &api::Target,
    action: &str,
) -> Result<api::Context, ApiError> {
    let slot = target.slot.as_deref().ok_or_else(unavailable)?;
    if target.session_id.is_some() || target.parent.is_some() || target.name.is_some() {
        return Err(unavailable());
    }
    access(inner, actor, slot, action)?;
    let s = &inner.config.content_import_slots[slot];
    let p = &inner.config.directories[&s.parent];
    let binding = api::hash(
        &serde_json::to_vec(&json!({"root":inner.config.root,"slot":s,"parent":p}))
            .map_err(|_| api::changed())?,
    );
    Ok(api::Context {
        scope: scope(inner, actor),
        resource: slot.into(),
        root: inner.config.root.clone(),
        parent: inner.config.root.join(&p.path),
        name: s.name.clone(),
        epoch: inner.actor_generation(actor),
        binding,
        target: target.clone(),
    })
}
fn claim(
    inner: &HostedInner,
    actor: &str,
    id: &str,
    action: &str,
) -> Result<(api::Claim, api::Context), ApiError> {
    let c = inner.office.recognition.claim(&scope(inner, actor), id)?;
    let ctx = context(inner, actor, &c.request.target, action)?;
    api::check_claim(&c, &ctx)?;
    Ok((c, ctx))
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/recognition/capabilities", get(capabilities))
        .route("/api/hosted/recognition/fields", post(fields))
        .route("/api/hosted/recognition/start", post(start))
        .route("/api/hosted/recognition/status", post(status))
        .route("/api/hosted/recognition/cancel", post(cancel))
        .route("/api/hosted/recognition/preview", post(preview))
        .route("/api/hosted/recognition/download", post(download))
        .route("/api/hosted/recognition/prepare", post(prepare))
        .route("/api/hosted/recognition/commit", post(commit))
        .route("/api/hosted/recognition/operation", post(operation))
        .route("/api/hosted/recognition/settle", post(settle))
        .route("/api/hosted/recognition/cancel-import", post(settle))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}
async fn capabilities(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{let actor=inner.actor(&auth)?;let mut v=api::qualified_capabilities(&inner.office.recognition,"selfHosted");let mut targets=vec![];for (id,s) in &inner.config.content_import_slots{if access(inner,&actor,id,"read").is_ok(){targets.push(json!({"id":id,"title":s.title,"available":access(inner,&actor,id,"run").is_ok()&&!inner.config.root.join(&inner.config.directories[&s.parent].path).join(&s.name).exists()}));}}v["targets"]=json!(targets);Ok(v)}).await?))
}
async fn fields(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Fields>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let ctx = context(inner, &actor, &req.target, "import")?;
            library::access(inner, &actor, "schema", false)?;
            let mut value = api::schema_fields(&mut inner.core, &ctx.root)?;
            let grants = &inner.policy.users[&actor].content_imports[&ctx.resource]
                .members
                .fields;
            if let Some(fields) = value["fields"].as_array_mut() {
                fields.retain(|f| {
                    f["id"]
                        .as_str()
                        .and_then(|id| grants.get(id))
                        .is_some_and(|g| g.write && g.read)
                });
            }
            Ok(value)
        })
        .await?,
    ))
}
async fn start(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Start>,
) -> ApiResult {
    let (c, run, reply) = locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let ctx = context(inner, &actor, &req.target, "run")?;
        let (c, run) = inner.office.recognition.begin(ctx, req)?;
        let reply = inner.office.recognition.reply(&c)?;
        Ok((c, run, reply))
    })
    .await?;
    if let Some(run) = run {
        tokio::spawn(async move {
            api::run_job(run).await;
            if let Ok(mut inner) = state.inner.lock() {
                inner.office.recognition.finish(&c);
            }
        });
    }
    Ok(Json(reply))
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::JobRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let (c, _) = claim(inner, &actor, &req.job_id, "read")?;
            inner.office.recognition.reply(&c)
        })
        .await?,
    ))
}
async fn cancel(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::JobRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let (c, ctx) = claim(inner, &actor, &req.job_id, "read")?;
            inner.office.recognition.cancel(&ctx.scope, &req.job_id)?;
            inner.office.recognition.reply(&c)
        })
        .await?,
    ))
}
async fn download(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::JobRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let (c, _) = claim(inner, &actor, &req.job_id, "download")?;
            api::download(&inner.office.recognition, &c)
        })
        .await?,
    ))
}
async fn preview(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Preview>,
) -> ApiResult {
    let id = req.job_id.clone();
    let auth2 = auth.clone();
    let (c, root, runtime) = locked(&state, move |inner| {
        let actor = inner.actor(&auth2)?;
        let (c, _) = claim(inner, &actor, &id, "read")?;
        Ok((
            c,
            inner
                .office
                .recognition
                .root
                .clone()
                .ok_or_else(api::changed)?,
            inner.office.recognition.runtime(),
        ))
    })
    .await?;
    let result = api::preview_job(root, runtime, c, req.page).await?;
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let _ = claim(inner, &actor, &req.job_id, "read")?;
        Ok(())
    })
    .await?;
    Ok(Json(result))
}
fn safe(
    inner: &HostedInner,
    actor: &str,
    ctx: &api::Context,
    s: weftext_core::ActorFeatureStatus,
) -> Value {
    let current = api::decision_current(&s.preview, ctx)
        && access(inner, actor, &ctx.resource, "import").is_ok();
    api::receipt(s, current)
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{
    let actor=inner.actor(&auth)?;let ctx=context(inner,&actor,&req.target,"import")?;let(c,_)=claim(inner,&actor,&req.job_id,"import")?;api::check_claim(&c,&ctx)?;library::access(inner,&actor,"schema",false)?;
    let field_rights=&inner.policy.users[&actor].content_imports[&ctx.resource].members.fields;for f in req.review.fields.iter().filter(|f|f.confirmed){if f.target_field.as_ref().is_some_and(|id|!field_rights.get(id).is_some_and(|r|r.read&&r.write)){return Err(unavailable());}}
    let fp=api::fingerprint(&c,&req.review,&req.operation_id)?;let evidence=api::evidence(&inner.office.recognition,&c)?;let bytes=api::original(&inner.office.recognition,&c)?;
    let request=api::intent(&ctx,&c,bytes,evidence,req.review.clone(),&req.operation_id);
    let p=inner.config.directories[&inner.config.content_import_slots[&ctx.resource].parent].clone();let q=inner.core.qualified_query_scope_at(&ctx.root,&ctx.parent,&p.reference,"live").map_err(|_|api::changed())?;let mut witnesses=vec![];Core::merge_query_scope_witness(&mut witnesses,&q.witness).map_err(|_|api::changed())?;let refs=witnesses.iter().collect::<Vec<_>>();
    let preview=json!({"summary":UiMessage::error_api_hosted_recognition_plan_summary(),"fields":req.review.fields,"_recognition":api::decision(&c,&ctx)});
    let s=inner.core.with_actor_for_root(&ctx.scope,&ctx.root,|core|core.actor_feature_prepare_content_import(&ctx.resource,&req.operation_id,&fp,request,preview,&refs,&ctx.resource)).map_err(|_|api::changed())?;Ok(safe(inner,&actor,&ctx,s))
}).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Operation>,
) -> ApiResult {
    action(state, auth, req, None, false).await
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Operation>,
) -> ApiResult {
    action(state, auth, req, None, true).await
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<api::Commit>,
) -> ApiResult {
    action(
        state,
        auth,
        api::Operation {
            target: req.target,
            operation_id: req.operation_id,
        },
        Some((req.plan_id, req.accept_losses)),
        false,
    )
    .await
}
async fn action(
    state: HostedState,
    auth: Auth,
    req: api::Operation,
    commit: Option<(String, bool)>,
    settle: bool,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{
    api::uuid(&req.operation_id)?;let actor=inner.actor(&auth)?;let ctx=context(inner,&actor,&req.target,"read")?;
    let old=inner.core.with_actor_for_root(&ctx.scope,&ctx.root,|c|c.actor_feature_status(&ctx.resource,&req.operation_id)).map_err(|_|api::changed())?;
    let s=if let Some((plan,losses))=commit{access(inner,&actor,&ctx.resource,"import")?;let old=old.ok_or_else(api::changed)?;if old.status=="prepared"&&!api::decision_current(&old.preview,&ctx){return Err(api::changed());}inner.core.with_actor_for_root(&ctx.scope,&ctx.root,|c|c.actor_feature_commit_with_losses(&ctx.resource,&req.operation_id,&plan,losses)).map_err(|_|api::changed())?}else if settle{inner.core.with_actor_for_root(&ctx.scope,&ctx.root,|c|c.actor_feature_settle(&ctx.resource,&req.operation_id)).map_err(|_|api::changed())?}else if let Some(old)=old{old}else{return Ok(json!({"operationId":req.operation_id,"status":"notRegistered","commitAllowed":false,"preview":{},"nodeRefs":[]}));};Ok(safe(inner,&actor,&ctx,s))
}).await?))
}
