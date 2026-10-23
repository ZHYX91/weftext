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
    pub(super) tag_witness: Option<(String, weftext_core::SnapshotWitness)>,
    pub(super) actor: String,
    pub(super) epoch: u64,
    pub(super) resources: BTreeSet<String>,
    pub(super) authorities: BTreeMap<String, BTreeSet<String>>,
    pub(super) config_hash: String,
    pub(super) result: weftext_data::QueryResult,
    pub(super) columns: Vec<String>,
    pub(super) schema: Schema,
    pub(super) query_authority: query::Authority,
    pub(super) embedded: Option<export_api::EmbeddedQueryBasis>,
    size_bytes: usize,
    expires: Instant,
}
fn retained_query<'a>(
    store: &'a Store,
    actor: &str,
    epoch: u64,
    id: &str,
) -> Result<&'a FrozenQuery, ApiError> {
    store
        .queries
        .get(id)
        .filter(|q| q.actor == actor && q.epoch == epoch && q.expires > Instant::now())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::GONE,
                "queryExpired",
                UiMessage::error_api_hosted_fields_original_query_expired(),
            )
        })
}
pub(super) fn frozen_query(
    inner: &HostedInner,
    actor: &str,
    id: &str,
) -> Result<FrozenQuery, ApiError> {
    let query = retained_query(&inner.fields, actor, inner.actor_generation(actor), id)?;
    query::authorize_authority(inner, actor, &query.query_authority)?;
    for (resource, fields) in &query.authorities {
        for field in fields {
            rights(inner, actor, resource, field, false)?;
        }
    }
    if office::binding_hash(inner, &query.resources)? != query.config_hash {
        return Err(conflict());
    }
    let mut query = query.clone();
    if let Some((path, witness)) = &query.tag_witness
        && inner
            .core
            .validate_snapshot_witness(&inner.config.root, path, witness)
            .is_err()
    {
        query.result.freshness = "stale".into();
        query
            .result
            .diagnostics
            .push(UiMessage::error_api_hosted_fields_tag_structure_changed().into());
    }
    Ok(query)
}
pub(super) fn query_catalog(inner: &HostedInner, actor: &str) -> Vec<Value> {
    inner.fields.queries.keys().filter_map(|id|frozen_query(inner,actor,id).ok().map(|q|json!({"queryId":id,"columns":q.columns,"rowCount":q.result.rows.len(),"coverage":q.result.coverage,"freshness":q.result.freshness}))).collect()
}

pub(super) fn bind_embedded_query(
    inner: &mut HostedInner,
    actor: &str,
    id: &str,
    basis: export_api::EmbeddedQueryBasis,
) -> Result<(), ApiError> {
    let original = frozen_query(inner, actor, id)?;
    if let Some(bound) = original.embedded {
        return if bound == basis {
            Ok(())
        } else {
            Err(conflict())
        };
    }
    let extra = basis
        .session_id
        .len()
        .checked_add(basis.source_hash.len())
        .and_then(|n| n.checked_add(basis.occurrence_id.len()))
        .and_then(|n| n.checked_add(128))
        .ok_or_else(conflict)?;
    let total = inner
        .fields
        .queries
        .values()
        .try_fold(extra, |n, q| n.checked_add(q.size_bytes))
        .ok_or_else(conflict)?;
    if total > 32 * 1024 * 1024 {
        return Err(conflict());
    }
    let current = inner.fields.queries.get_mut(id).ok_or_else(conflict)?;
    current.size_bytes = current.size_bytes.checked_add(extra).ok_or_else(conflict)?;
    current.embedded = Some(basis);
    Ok(())
}

pub(super) fn retain_query(
    inner: &mut HostedInner,
    actor: &str,
    result: weftext_data::QueryResult,
    columns: Vec<String>,
    schema: Schema,
    authority: query::Authority,
    tag_witness: Option<(String, weftext_core::SnapshotWitness)>,
) -> Result<String, ApiError> {
    query::authorize_authority(inner, actor, &authority)?;
    expire_revoked(inner);
    let mut authorities = authority.fields.clone();
    for resource in authority
        .products
        .keys()
        .chain(authority.definitions.keys())
        .chain(authority.relations.keys())
        .chain(&authority.inputs)
    {
        authorities.entry(resource.clone()).or_default();
    }
    let resources = authorities.keys().cloned().collect();
    let size_bytes = query::serialized_query_bytes(&(&result, &schema, &columns, &authority))?
        .checked_add(tag_witness.as_ref().map_or(0, |(_, w)| w.retained_bytes()))
        .ok_or_else(conflict)?;
    let retained = FrozenQuery {
        tag_witness,
        actor: actor.into(),
        epoch: inner.actor_generation(actor),
        config_hash: office::binding_hash(inner, &resources)?,
        resources,
        authorities,
        result,
        columns,
        schema,
        query_authority: authority,
        embedded: None,
        size_bytes,
        expires: Instant::now() + Duration::from_secs(900),
    };
    retain_in_store(&mut inner.fields, retained)
}
fn cache_evictions(store: &Store, actor: &str, new_bytes: usize) -> Result<Vec<String>, ApiError> {
    const MAX_BYTES: usize = 32 * 1024 * 1024;
    let full = || ApiError::bad(UiMessage::error_api_hosted_fields_frozen_query_cache_budget());
    if new_bytes > MAX_BYTES {
        return Err(full());
    }
    let mut bytes = store
        .queries
        .values()
        .try_fold(new_bytes, |n, q| n.checked_add(q.size_bytes))
        .ok_or_else(full)?;
    let mut count = store.queries.len() + 1;
    let mut own = store
        .queries
        .iter()
        .filter(|(_, q)| q.actor == actor)
        .collect::<Vec<_>>();
    own.sort_by(|(aid, a), (bid, b)| a.expires.cmp(&b.expires).then(aid.cmp(bid)));
    let mut actor_count = own.len() + 1;
    let mut removed = vec![];
    for (id, q) in own {
        if count <= 32 && actor_count <= 8 && bytes <= MAX_BYTES {
            return Ok(removed);
        }
        removed.push(id.clone());
        count -= 1;
        actor_count -= 1;
        bytes -= q.size_bytes;
    }
    if count <= 32 && actor_count <= 8 && bytes <= MAX_BYTES {
        Ok(removed)
    } else {
        Err(full())
    }
}
fn retain_in_store(store: &mut Store, retained: FrozenQuery) -> Result<String, ApiError> {
    // Plan the complete eviction before mutating. Other actors' cached results
    // cannot be displaced, and an unretainable new result preserves old IDs.
    let evictions = cache_evictions(store, &retained.actor, retained.size_bytes)?;
    for id in evictions {
        store.queries.remove(&id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    store.queries.insert(id.clone(), retained);
    Ok(id)
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

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TagAssignmentTarget {
    pub(super) resource: String,
    pub(super) field_id: String,
    basis: String,
}

pub(super) fn authorize_tag_assignment(
    inner: &HostedInner,
    actor: &str,
    target: &TagAssignmentTarget,
) -> Result<(), ApiError> {
    rights(inner, actor, &target.resource, &target.field_id, true)?;
    structure::guard_slot(inner, &target.resource)?;
    if !inner.file(&target.resource)?.editable {
        return Err(unavailable());
    }
    Ok(())
}

/// The caller's opaque field basis proves only granted fields. The trusted
/// source and exact Core snapshot remain inside the server boundary.
pub(super) fn prepare_tag_assignment(
    inner: &mut HostedInner,
    actor: &str,
    target: &TagAssignmentTarget,
    operation: &str,
) -> Result<weftext_core::TagAssignment, ApiError> {
    authorize_tag_assignment(inner, actor, target)?;
    expire_revoked(inner);
    let basis = inner
        .fields
        .bases
        .get(&target.basis)
        .filter(|b| {
            b.actor == actor
                && b.epoch == inner.actor_generation(actor)
                && b.resource == target.resource
                && b.field == target.field_id
                && b.expires > Instant::now()
        })
        .cloned()
        .ok_or_else(conflict)?;
    for field in &basis.fields {
        rights(inner, actor, &target.resource, field, false)?;
    }
    let root = inner.config.root.clone();
    let path = inner.config.root.join(&inner.file(&target.resource)?.path);
    with_source(
        inner,
        actor,
        &target.resource,
        Some(operation),
        |core, doc, schema, history, _| {
            let current =
                narrow_profile(&doc.source, schema, &basis.fields, Some(&target.field_id))
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
            for version in &history.versions {
                if !basis.versions.contains(&version.version_id) {
                    let old = core
                        .read_history(&doc.session_id, &version.version_id)
                        .map_err(|_| conflict())?;
                    if narrow_profile(&old.source, schema, &basis.fields, Some(&target.field_id))
                        .map_err(|_| conflict())?
                        .witness
                        != basis.witness
                    {
                        return Err(conflict());
                    }
                }
            }
            let file = core
                .read_snapshot(&root, &[path.to_string_lossy().into_owned()])
                .map_err(|_| conflict())?
                .files
                .remove(0);
            if file.bytes.as_deref() != Some(doc.source.as_bytes()) {
                return Err(conflict());
            }
            Ok(weftext_core::TagAssignment {
                path: file.path,
                base_id: file.base_id,
                field_id: target.field_id.clone(),
            })
        },
    )
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
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/fields", get(list))
        .route("/api/hosted/fields/read", post(read))
        .route("/api/hosted/fields/write", post(write))
        .route("/api/hosted/fields/operation", post(operation))
        .route("/api/hosted/fields/settle", post(settle))
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
        .grant(actor, resource)?
        .fields
        .get(field)
        .filter(|r| {
            r.read && (!write || r.write && inner.file(resource).ok().is_some_and(|f| f.editable))
        })
        .cloned()
        .ok_or_else(unavailable)
}
fn conflict() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "fieldBasisChanged",
        UiMessage::error_api_hosted_fields_field_basis_changed(),
    )
}
fn profile_unavailable() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "fieldUnavailable",
        UiMessage::error_api_hosted_fields_field_qualification_unavailable(),
    )
}
fn operation_error(_: CoreError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "fieldOperation",
        UiMessage::error_api_hosted_fields_field_operation_unresolved(),
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
    structure::guard_slot(inner, resource)?;
    let file = author_file(inner, resource).map_err(|_| profile_unavailable())?;
    let path = inner.config.root.join(&file.path);
    let root = inner.config.root.clone();
    let scope = field_scope(inner, actor, operation);
    // Keep API failures free of Core paths, full currentSource and details.
    inner
        .core
        .with_actor_for_root(&scope, &inner.config.root, |core| {
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
            let available_grants = inner.grants_view(&actor)?;
            let fields = available_grants
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
    RequestJson(req): RequestJson<Select>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let access = rights(inner, &actor, &req.resource, &req.field_id, false)?;
        expire_revoked(inner);
        let grant=inner.grant(&actor,&req.resource)?;
        let granted = grant.fields.iter()
            .filter(|(_, r)| r.read).map(|(id, _)| id.clone()).collect::<BTreeSet<_>>();
        let file_writable = inner.file(&req.resource).ok().is_some_and(|f| f.editable);
        let (mut field, writable) = with_source(inner, &actor, &req.resource, None, |_, doc, schema, history, can_write_schema| {
            let profile = narrow_profile(&doc.source, schema, &BTreeSet::from([req.field_id.clone()]), None)
                .map_err(|_| profile_unavailable())?;
            if profile.node_id != history.node_id { return Err(profile_unavailable()); }
            let field = profile.fields.into_iter().next().ok_or_else(unavailable)?;
            let writable = if access.write && file_writable && field.writable && can_write_schema {
                narrow_write_fields(&doc.source, schema, &req.field_id).ok()
                    .filter(|fields| fields.is_subset(&granted))
                    .and_then(|fields| narrow_profile(&doc.source, schema, &fields, Some(&req.field_id)).ok()
                        .map(|p| (fields, p.witness, history.versions.iter().map(|v| v.version_id.clone()).collect())))
            } else { None };
            Ok((field, writable))
        })?;
        let basis = if let Some((fields, witness, versions)) = writable {
            if inner.fields.bases.len() >= 1024 || inner.fields.bases.values().filter(|b| b.actor == actor).count() >= 128 {
                return Err(ApiError::bad(UiMessage::error_api_hosted_fields_field_basis_capacity()));
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

#[cfg(all(test, windows))]
pub(super) async fn test_read_assignment_field(
    state: HostedState,
    auth: Auth,
    resource: &str,
    field_id: &str,
) -> ApiResult {
    read(
        State(state),
        Extension(auth),
        crate::RequestJson(Select {
            resource: resource.into(),
            field_id: field_id.into(),
        }),
    )
    .await
}

async fn write(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Write>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            rights(inner, &actor, &req.resource, &req.field_id, true)?;
            let scope = field_scope(inner, &actor, Some(&req.operation_id));
            let fingerprint = digest(&serde_json::to_vec(&req).map_err(|_| conflict())?);
            if let Some(status) = inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |core| {
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
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        rights(inner, &actor, &req.resource, &req.field_id, false)?;
        let result = inner.core.with_actor_for_root(&field_scope(inner, &actor, Some(&req.operation_id)), &inner.config.root, |core|
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
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            rights(inner, &actor, &req.resource, &req.field_id, true)?;
            value(
                inner
                    .core
                    .with_actor_for_root(
                        &field_scope(inner, &actor, Some(&req.operation_id)),
                        &inner.config.root,
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

#[cfg(test)]
mod cache_tests {
    use super::*;

    fn sample(actor: &str, order: u64) -> FrozenQuery {
        let result = weftext_data::QueryResult {
            cut_id: format!("cut:{order}"),
            schema_revision: "schema".into(),
            rows: vec![weftext_data::QueryRow {
                key: "row".into(),
                entity_key: Some("one".into()),
                path: Some("resource:one".into()),
                revision: Some("cut".into()),
                cells: BTreeMap::from([(
                    "text".into(),
                    weftext_data::Cell::value(NativeValue::Text(format!("original:{order}"))),
                )]),
                read_only: true,
                lifecycle: Some("live".into()),
            }],
            coverage: "complete".into(),
            freshness: "current".into(),
            diagnostics: vec![],
            total_known: 1,
            limited: false,
            aggregate_exact: true,
            runtime: "native".into(),
            terminal_schema: BTreeMap::new(),
        };
        let schema = Schema::bundled().unwrap();
        let size_bytes = query::serialized_query_bytes(&(&result, &schema)).unwrap();
        FrozenQuery {
            tag_witness: None,
            actor: actor.into(),
            epoch: 1,
            resources: BTreeSet::new(),
            authorities: BTreeMap::new(),
            config_hash: "binding".into(),
            result,
            columns: vec!["text".into()],
            schema,
            query_authority: query::Authority::default(),
            embedded: None,
            size_bytes,
            expires: Instant::now() + Duration::from_secs(900 + order),
        }
    }

    #[test]
    fn ninth_result_expires_only_oldest_own_id_and_preserves_office_snapshot() {
        let mut store = Store::default();
        let old = retain_in_store(&mut store, sample("alice", 0)).unwrap();
        // Office prepare owns a clone rather than resolving the ID again.
        let office_snapshot = retained_query(&store, "alice", 1, &old).unwrap().clone();
        let other = retain_in_store(&mut store, sample("bob", 0)).unwrap();
        for n in 1..=8 {
            retain_in_store(&mut store, sample("alice", n)).unwrap();
        }
        let error = retained_query(&store, "alice", 1, &old).err().unwrap();
        assert_eq!(error.status, StatusCode::GONE);
        assert_eq!(error.code, "queryExpired");
        assert!(retained_query(&store, "bob", 1, &other).is_ok());
        assert_eq!(
            store
                .queries
                .values()
                .filter(|q| q.actor == "alice")
                .count(),
            8
        );
        assert_eq!(
            office_snapshot.result.rows[0].cells["text"].native(),
            Some(&NativeValue::Text("original:0".into()))
        );
    }

    #[test]
    fn global_count_pressure_evicts_only_requesting_actor() {
        let mut store = Store::default();
        let own = retain_in_store(&mut store, sample("alice", 0)).unwrap();
        let mut other_ids = vec![];
        for n in 0..31 {
            other_ids.push(retain_in_store(&mut store, sample(&format!("actor:{n}"), 0)).unwrap());
        }
        retain_in_store(&mut store, sample("alice", 1)).unwrap();
        assert!(!store.queries.contains_key(&own));
        assert!(other_ids.iter().all(|id| store.queries.contains_key(id)));
        assert_eq!(store.queries.len(), 32);
    }

    #[test]
    fn unretainable_result_does_not_remove_existing_ids() {
        let mut store = Store::default();
        let own = retain_in_store(&mut store, sample("alice", 0)).unwrap();
        let mut other = sample("bob", 0);
        // Simulate a measured retained result occupying nearly all bytes.
        other.size_bytes = 32 * 1024 * 1024 - store.queries[&own].size_bytes;
        let other = retain_in_store(&mut store, other).unwrap();
        let keys = store.queries.keys().cloned().collect::<BTreeSet<_>>();
        let mut replacement = sample("alice", 1);
        replacement.size_bytes = store.queries[&own].size_bytes + 1;
        assert!(retain_in_store(&mut store, replacement).is_err());
        assert_eq!(store.queries.keys().cloned().collect::<BTreeSet<_>>(), keys);
        assert!(retained_query(&store, "alice", 1, &own).is_ok());
        assert!(retained_query(&store, "bob", 1, &other).is_ok());
        let mut oversized = sample("alice", 2);
        oversized.size_bytes = 32 * 1024 * 1024 + 1;
        assert!(retain_in_store(&mut store, oversized).is_err());
        assert_eq!(store.queries.keys().cloned().collect::<BTreeSet<_>>(), keys);
    }

    #[test]
    fn byte_pressure_plans_multiple_oldest_evictions() {
        let mut store = Store::default();
        let mut ids = vec![];
        for n in 0..8 {
            let mut q = sample("alice", n);
            q.size_bytes = 4 * 1024 * 1024;
            ids.push(retain_in_store(&mut store, q).unwrap());
        }
        let mut replacement = sample("alice", 9);
        replacement.size_bytes = 9 * 1024 * 1024;
        retain_in_store(&mut store, replacement).unwrap();
        assert!(ids[..3].iter().all(|id| !store.queries.contains_key(id)));
        assert!(ids[3..].iter().all(|id| store.queries.contains_key(id)));
    }
}
