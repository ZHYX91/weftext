//! Field-only deliveries never expose the Core session, source, full revision,
//! internal schema, offsets, or history. No route accepts client source bytes.
use super::*;
use weftext_data::{NativeValue, Schema, narrow_profile, narrow_write_fields, patch_narrow_field};

#[derive(Default)]
pub(super) struct Store {
    bases: HashMap<String, Basis>,
    queries: HashMap<String, FrozenQuery>,
}
#[derive(Clone)]
pub(super) struct FrozenQuery {
    pub(super) actor: String,
    pub(super) epoch: u64,
    pub(super) resources: BTreeSet<String>,
    pub(super) authorities: BTreeMap<String, BTreeSet<String>>,
    pub(super) config_hash: String,
    pub(super) result: weftext_data::QueryResult,
    pub(super) columns: Vec<String>,
    pub(super) schema: Schema,
    expires: Instant,
}
pub(super) fn frozen_query(
    inner: &HostedInner,
    actor: &str,
    id: &str,
) -> Result<FrozenQuery, ApiError> {
    let query = inner
        .fields
        .queries
        .get(id)
        .filter(|q| {
            q.actor == actor
                && q.epoch == inner.actor_generation(actor)
                && q.expires > Instant::now()
        })
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::GONE,
                "queryExpired",
                "原查询结果已失效，请明确重新执行查询",
            )
        })?;
    for (resource, fields) in &query.authorities {
        for field in fields {
            rights(inner, actor, resource, field, false)?;
        }
    }
    if office::binding_hash(inner, &query.resources)? != query.config_hash {
        return Err(conflict());
    }
    Ok(query.clone())
}
pub(super) fn query_catalog(inner: &HostedInner, actor: &str) -> Vec<Value> {
    inner.fields.queries.iter().filter_map(|(id,q)|frozen_query(inner,actor,id).ok().map(|_|json!({"queryId":id,"columns":q.columns,"rowCount":q.result.rows.len(),"coverage":q.result.coverage}))).collect()
}
#[derive(Clone)]
struct Basis {
    actor: String,
    epoch: u64,
    resource: String,
    field: String,
    fields: BTreeSet<String>,
    witness: String,
    versions: BTreeSet<String>,
    expires: Instant,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Select {
    resource: String,
    field_id: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Write {
    resource: String,
    field_id: String,
    basis: String,
    value: NativeValue,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    resource: String,
    field_id: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Query {
    fields: BTreeSet<String>,
    resources: Option<BTreeSet<String>>,
    plan: weftext_data::QueryPlan,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/fields", get(list))
        .route("/api/hosted/fields/read", post(read))
        .route("/api/hosted/fields/write", post(write))
        .route("/api/hosted/fields/operation", post(operation))
        .route("/api/hosted/fields/settle", post(settle))
        .route("/api/hosted/query", post(query))
}

async fn query(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(mut req): Json<Query>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            if req.fields.is_empty()
                || req.fields.len() > 32
                || req.plan.advanced.is_some()
                || req.plan.scope.directory.is_some()
                || !matches!(req.plan.scope.lifecycle.as_str(), "" | "live")
                || !req.plan.types.is_empty()
                || req.plan.take.is_some_and(|n| n > 500)
            {
                return Err(ApiError::bad(
                    "请选择至多 32 个获权字段；当前托管查询支持当前范围及至多 500 行",
                ));
            }
            // Even an empty row scope must not turn the shared schema into an
            // oracle for ungranted field definitions or existence.
            for field in &req.fields {
                if !inner.policy.users[&actor]
                    .grants
                    .keys()
                    .any(|resource| rights(inner, &actor, resource, field, false).is_ok())
                {
                    return Err(unavailable());
                }
            }
            let resources = match req.resources {
                Some(resources) => resources,
                None => inner.policy.users[&actor]
                    .grants
                    .keys()
                    .filter(|r| {
                        req.fields
                            .iter()
                            .all(|f| rights(inner, &actor, r, f, false).is_ok())
                    })
                    .cloned()
                    .collect(),
            };
            if resources.len() > 64 {
                return Err(ApiError::bad("查询范围上限为 64 个获权对象"));
            }
            for resource in &resources {
                for field in &req.fields {
                    rights(inner, &actor, resource, field, false)?;
                }
            }
            let schema = data_api::hosted_schema(&mut inner.core, &inner.config.root)
                .map_err(|_| unavailable())?;
            let restricted = weftext_data::narrow_query_schema(&schema, &req.fields)
                .map_err(|_| unavailable())?;
            if req.plan.select.is_empty() {
                req.plan.select = restricted
                    .fields
                    .iter()
                    .map(|f| weftext_data::QueryColumn {
                        name: f.key.clone(),
                        expr: format!("field({})", serde_json::to_string(&f.id).unwrap()),
                    })
                    .collect();
            }
            req.plan.take.get_or_insert(500);
            let cut = uuid::Uuid::new_v4().to_string();
            let mut notes = vec![];
            let mut witnesses = BTreeMap::new();
            let mut complete = true;
            for resource in &resources {
                let profile = with_source(
                    inner,
                    &actor,
                    resource,
                    None,
                    |_, doc, current_schema, history, _| {
                        let current_shape =
                            weftext_data::narrow_query_schema(current_schema, &req.fields)
                                .map_err(|_| conflict())?;
                        if value(&current_shape)? != value(&restricted)? {
                            return Err(conflict());
                        }
                        let p = narrow_profile(&doc.source, current_schema, &req.fields, None)
                            .map_err(|_| unavailable())?;
                        if p.node_id != history.node_id {
                            return Err(unavailable());
                        }
                        Ok(p)
                    },
                );
                let Ok(profile) = profile else {
                    complete = false;
                    continue;
                };
                witnesses.insert(resource.clone(), profile.witness);
                notes.push(weftext_data::FrozenNote {
                    entity_key: resource.clone(),
                    path: format!("resource:{resource}"),
                    source: String::new(),
                    revision: cut.clone(),
                    lifecycle: "live".into(),
                    title: None,
                    body: None,
                    products: None,
                    authorized_fields: Some(
                        profile
                            .fields
                            .into_iter()
                            .map(|f| (f.field_id, f.cell))
                            .collect(),
                    ),
                });
            }
            let input = weftext_data::FrozenQueryInput {
                cut_id: cut.clone(),
                schema_revision: cut,
                coverage: if complete { "complete" } else { "partial" }.into(),
                freshness: "current".into(),
                notes,
                advanced: None,
            };
            let result = weftext_data::execute_query(&input, &req.plan, &restricted)
                .map_err(|_| ApiError::bad("查询不可用：请检查获权字段、CEL 表达式和预算"))?;
            // Current policy is serialized by this lock; external source/config
            // changes still need requalification before the result is delivered.
            for (resource, witness) in witnesses {
                with_source(inner, &actor, &resource, None, |_, doc, schema, _, _| {
                    let current = narrow_profile(&doc.source, schema, &req.fields, None)
                        .map_err(|_| conflict())?;
                    if current.witness != witness {
                        return Err(conflict());
                    }
                    Ok(())
                })?;
            }
            expire_revoked(inner);
            let size = |q:&FrozenQuery| serde_json::to_vec(&(&q.result,&q.schema)).map_or(32*1024*1024,|b|b.len());
            let mut authorities:BTreeMap<String,BTreeSet<String>>=resources.iter().map(|r|(r.clone(),req.fields.clone())).collect();
            if authorities.is_empty() { for field in &req.fields { let resource=inner.policy.users[&actor].grants.keys().find(|r|rights(inner,&actor,r,field,false).is_ok()).ok_or_else(unavailable)?;authorities.entry(resource.clone()).or_default().insert(field.clone()); } }
            let qualified_resources=authorities.keys().cloned().collect();
            let query=FrozenQuery { actor:actor.clone(), epoch:inner.actor_generation(&actor),config_hash:office::binding_hash(inner,&qualified_resources)?,resources:qualified_resources,authorities,result:result.clone(),columns:req.plan.select.iter().map(|c|c.name.clone()).collect(),schema:restricted,expires:Instant::now()+Duration::from_secs(900) };
            if inner.fields.queries.len() >= 32 || inner.fields.queries.values().filter(|q|q.actor==actor).count() >= 8 || inner.fields.queries.values().map(size).sum::<usize>().saturating_add(size(&query)) > 32*1024*1024 {
                return Err(ApiError::bad("冻结查询结果缓存已满，请稍后再试"));
            }
            let query_id=uuid::Uuid::new_v4().to_string();
            inner.fields.queries.insert(query_id.clone(),query);
            Ok(json!({"result":result,"queryId":query_id,"columns":req.plan.select.iter().map(|c|&c.name).collect::<Vec<_>>(),"scope":"authorizedFieldIntersection","readOnly":true}))
        })
        .await?,
    ))
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    inner.fields.queries.retain(|_, q| {
        q.expires > Instant::now()
            && inner
                .policy
                .users
                .get(&q.actor)
                .is_some_and(|u| u.enabled && u.epoch == q.epoch)
    });
    inner.fields.bases.retain(|_, basis| {
        basis.expires > Instant::now()
            && inner
                .policy
                .users
                .get(&basis.actor)
                .is_some_and(|u| u.enabled && u.epoch == basis.epoch)
    });
}
fn rights(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
    field: &str,
    write: bool,
) -> Result<FieldRights, ApiError> {
    inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.grants.get(resource))
        .and_then(|r| r.fields.get(field))
        .filter(|r| r.read && (!write || r.write))
        .cloned()
        .ok_or_else(unavailable)
}
fn conflict() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "fieldBasisChanged",
        "字段或校验依据已变化，请保留输入并重新读取核对。",
    )
}
fn profile_unavailable() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "fieldUnavailable",
        "字段当前无法限定，输入已保留；请核对当前文档、Schema 及校验依赖。",
    )
}
fn operation_error(_: CoreError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "fieldOperation",
        "原字段操作未完成或编号已绑定其他提案；请核对原操作状态。",
    )
}

// Each original field save retains its own private recovery proposal; it
// cannot overwrite the actor's independently edited whole-source draft.
fn field_scope(inner: &HostedInner, actor: &str, operation: Option<&str>) -> String {
    format!(
        "{}:fields:{}",
        inner.actor_scope(actor),
        operation.unwrap_or("read")
    )
}

// Source is held only inside this trusted boundary. The Core session always
// closes, including qualification errors; source/draft APIs cannot address it.
fn with_source<T>(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    operation: Option<&str>,
    action: impl FnOnce(
        &mut Core,
        &weftext_core::OpenedDocument,
        &Schema,
        &weftext_core::HistoryTimeline,
        bool,
    ) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let file = inner
        .config
        .files
        .get(resource)
        .filter(|f| f.editable)
        .ok_or_else(profile_unavailable)?;
    let path = inner.config.root.join(&file.path);
    let root = inner.config.root.clone();
    let scope = field_scope(inner, actor, operation);
    // Keep API failures free of Core paths, full currentSource and details.
    inner
        .core
        .with_actor(&scope, |core| {
            let doc = core.open(path, Some(root.clone()))?;
            let result = (|| {
                let config = core
                    .read_config(&root, weftext_core::ConfigRole::Schema)
                    .map_err(|_| profile_unavailable())?;
                let schema = match &config.bytes {
                    Some(bytes) => Schema::from_author_json(
                        std::str::from_utf8(bytes).map_err(|_| profile_unavailable())?,
                    ),
                    None => Schema::bundled(),
                }
                .map_err(|_| profile_unavailable())?;
                let history = core
                    .list_history(&doc.session_id)
                    .map_err(|_| profile_unavailable())?;
                let current = history
                    .current_saved_version_id
                    .as_ref()
                    .ok_or_else(profile_unavailable)?;
                let checked = core
                    .compare_history(&doc.session_id, current)
                    .map_err(|_| profile_unavailable())?;
                if checked.current.source != doc.source
                    || history.currentness != "saved"
                    || history.versions.len() > 2000
                {
                    return Err(profile_unavailable());
                }
                let can_write_schema = config.bytes.is_some() && config.revision.is_some();
                if operation.is_some() {
                    core.with_hosted_schema(&root, &config, |core| {
                        Ok(action(core, &doc, &schema, &history, true))
                    })
                    .map_err(|_| conflict())?
                } else {
                    action(core, &doc, &schema, &history, can_write_schema)
                }
            })();
            core.close_session(&doc.session_id)?;
            Ok(result)
        })
        .map_err(|_| profile_unavailable())?
}

async fn list(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let fields = inner.policy.users[&actor]
                .grants
                .iter()
                .flat_map(|(resource, rights)| {
                    rights.fields.iter().filter(|(_, r)| r.read).map(
                        move |(id, r)| json!({"resource":resource,"fieldId":id,"write":r.write}),
                    )
                })
                .collect::<Vec<_>>();
            Ok(json!({"fields":fields,"coverage":"explicitAuthorizedFields"}))
        })
        .await?,
    ))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Select>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let access = rights(inner, &actor, &req.resource, &req.field_id, false)?;
        expire_revoked(inner);
        let granted = inner.policy.users[&actor].grants[&req.resource].fields.iter()
            .filter(|(_, r)| r.read).map(|(id, _)| id.clone()).collect::<BTreeSet<_>>();
        let (mut field, writable) = with_source(inner, &actor, &req.resource, None, |_, doc, schema, history, can_write_schema| {
            let profile = narrow_profile(&doc.source, schema, &BTreeSet::from([req.field_id.clone()]), None)
                .map_err(|_| profile_unavailable())?;
            if profile.node_id != history.node_id { return Err(profile_unavailable()); }
            let field = profile.fields.into_iter().next().ok_or_else(unavailable)?;
            let writable = if access.write && field.writable && can_write_schema {
                narrow_write_fields(&doc.source, schema, &req.field_id).ok()
                    .filter(|fields| fields.is_subset(&granted))
                    .and_then(|fields| narrow_profile(&doc.source, schema, &fields, Some(&req.field_id)).ok()
                        .map(|p| (fields, p.witness, history.versions.iter().map(|v| v.version_id.clone()).collect())))
            } else { None };
            Ok((field, writable))
        })?;
        let basis = if let Some((fields, witness, versions)) = writable {
            if inner.fields.bases.len() >= 1024 || inner.fields.bases.values().filter(|b| b.actor == actor).count() >= 128 {
                return Err(ApiError::bad("字段依据已达上限，请稍后重新读取"));
            }
            let id = uuid::Uuid::new_v4().to_string();
            inner.fields.bases.insert(id.clone(), Basis {
                actor: actor.clone(), epoch: inner.actor_generation(&actor), resource: req.resource.clone(),
                field: req.field_id.clone(), fields, witness, versions, expires: Instant::now() + Duration::from_secs(900),
            });
            Some(id)
        } else { None };
        field.writable = basis.is_some();
        Ok(json!({"resource":req.resource,"field":field,"basis":basis,"qualification":"staticAuthorizedField"}))
    }).await?))
}

async fn write(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Write>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            rights(inner, &actor, &req.resource, &req.field_id, true)?;
            let scope = field_scope(inner, &actor, Some(&req.operation_id));
            let fingerprint = digest(&serde_json::to_vec(&req).map_err(|_| conflict())?);
            if let Some(status) = inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_field_retry(
                        &req.resource,
                        &req.field_id,
                        &req.operation_id,
                        &fingerprint,
                    )
                })
                .map_err(operation_error)?
            {
                return value(status);
            }
            expire_revoked(inner);
            let basis = inner
                .fields
                .bases
                .get(&req.basis)
                .filter(|b| {
                    b.actor == actor
                        && b.resource == req.resource
                        && b.field == req.field_id
                        && b.epoch == inner.actor_generation(&actor)
                })
                .cloned()
                .ok_or_else(conflict)?;
            for field in &basis.fields {
                rights(inner, &actor, &req.resource, field, false)?;
            }
            with_source(
                inner,
                &actor,
                &req.resource,
                Some(&req.operation_id.clone()),
                |core, doc, schema, history, _| {
                    let current =
                        narrow_profile(&doc.source, schema, &basis.fields, Some(&req.field_id))
                            .map_err(|_| conflict())?;
                    if current.node_id != history.node_id || current.witness != basis.witness {
                        return Err(conflict());
                    }
                    let versions = history
                        .versions
                        .iter()
                        .map(|v| v.version_id.clone())
                        .collect::<BTreeSet<_>>();
                    if !basis.versions.is_subset(&versions) {
                        return Err(conflict());
                    }
                    // Retained original records catch A→B→A. Missing witnesses cannot
                    // silently turn equal current bytes into a valid old basis.
                    for version in &history.versions {
                        if !basis.versions.contains(&version.version_id) {
                            let old = core
                                .read_history(&doc.session_id, &version.version_id)
                                .map_err(|_| conflict())?;
                            let profile = narrow_profile(
                                &old.source,
                                schema,
                                &basis.fields,
                                Some(&req.field_id),
                            )
                            .map_err(|_| conflict())?;
                            if profile.witness != basis.witness {
                                return Err(conflict());
                            }
                        }
                    }
                    let patch = patch_narrow_field(
                        &doc.source,
                        schema,
                        &basis.fields,
                        &req.field_id,
                        &req.value,
                    )
                    .map_err(|_| conflict())?;
                    let after =
                        narrow_profile(&patch.source, schema, &basis.fields, Some(&req.field_id))
                            .map_err(|_| conflict())?;
                    let cell = after
                        .fields
                        .iter()
                        .find(|f| f.field_id == req.field_id)
                        .ok_or_else(conflict)?;
                    let status = core
                        .actor_field_save(
                            &req.resource,
                            &req.field_id,
                            &fingerprint,
                            value(&cell.cell)?,
                            weftext_core::HostedSaveRequest {
                                session_id: doc.session_id.clone(),
                                base_id: doc.base_id.clone(),
                                source: patch.source,
                                operation_id: req.operation_id,
                            },
                        )
                        .map_err(operation_error)?;
                    value(status)
                },
            )
        })
        .await?,
    ))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        rights(inner, &actor, &req.resource, &req.field_id, false)?;
        let result = inner.core.with_actor(&field_scope(inner, &actor, Some(&req.operation_id)), |core|
            core.actor_field_status(&req.resource, &req.field_id, &req.operation_id)).map_err(operation_error)?;
        match result {
            Some(status) => value(status),
            None => Ok(json!({"operationId":req.operation_id,"resource":req.resource,"fieldId":req.field_id,"status":"notRegistered","cell":null})),
        }
    }).await?))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            rights(inner, &actor, &req.resource, &req.field_id, true)?;
            value(
                inner
                    .core
                    .with_actor(
                        &field_scope(inner, &actor, Some(&req.operation_id)),
                        |core| {
                            core.actor_field_settle(&req.resource, &req.field_id, &req.operation_id)
                        },
                    )
                    .map_err(operation_error)?,
            )
        })
        .await?,
    ))
}
