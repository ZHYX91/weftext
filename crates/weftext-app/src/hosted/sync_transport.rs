//! Explicit owner capability: full immutable history, never author-tree writes.
use super::*;
use crate::sync_wire::{self, CHUNK, MAX_BYTES, Record};
use weftext_core::sync::{ObjectKey, SyncIdentity};
const TTL: Duration = Duration::from_secs(15 * 60);
#[derive(Default)]
pub(super) struct Store {
    downloads: HashMap<String, Snapshot>,
    uploads: HashMap<String, Upload>,
}
struct Lease {
    actor: String,
    epoch: u64,
    expires: Instant,
}
struct Snapshot {
    lease: Lease,
    records: Vec<(Record, Vec<u8>)>,
}
struct Upload {
    lease: Lease,
    record: Record,
    bytes: Vec<u8>,
    complete: bool,
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    let valid = |lease: &Lease| {
        lease.expires > Instant::now()
            && inner
                .policy
                .users
                .get(&lease.actor)
                .is_some_and(|u| u.enabled && u.sync_owner && u.epoch == lease.epoch)
    };
    inner.sync.downloads.retain(|_, v| valid(&v.lease));
    inner.sync.uploads.retain(|_, v| valid(&v.lease));
}
fn owner(
    inner: &HostedInner,
    auth: &Auth,
    identity: Option<&SyncIdentity>,
) -> Result<(String, SyncIdentity), ApiError> {
    let actor = inner.actor(auth)?;
    let configured = inner
        .config
        .sync_identity
        .as_ref()
        .filter(|configured| identity.is_none_or(|v| v == *configured))
        .ok_or_else(unavailable)?;
    if !inner.policy.users[&actor].sync_owner {
        return Err(unavailable());
    }
    Ok((actor, configured.clone()))
}
fn lease(inner: &HostedInner, actor: String) -> Lease {
    Lease {
        epoch: inner.actor_generation(&actor),
        actor,
        expires: Instant::now() + TTL,
    }
}
fn current(inner: &HostedInner, actor: &str, value: &Lease) -> bool {
    value.actor == actor
        && value.epoch == inner.actor_generation(actor)
        && value.expires > Instant::now()
}
fn sync_error(error: CoreError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "syncRecordRejected",
        UiMessage::error_api_hosted_sync_transport_record_rejected(
            weftext_messages::params::ErrorApiHostedSyncTransportRecordRejected {
                code: error.code,
            },
        ),
    )
}
fn publish_bound(inner: &HostedInner, identity: &SyncIdentity) -> Result<(), ApiError> {
    match inner.core.sync_binding_info(&inner.config.root) {
        Ok(binding) if binding.identity == *identity => {
            inner
                .core
                .publish_sync(&inner.config.root)
                .map_err(sync_error)?;
        }
        Err(error) if error.code != "syncNotEnabled" => return Err(sync_error(error)),
        _ => {}
    }
    Ok(())
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/sync/status", post(status))
        .route("/api/hosted/sync/prepare", post(prepare))
        .route("/api/hosted/sync/catalog", post(catalog))
        .route("/api/hosted/sync/chunk", post(chunk))
        .route("/api/hosted/sync/upload", post(upload))
        .route("/api/hosted/sync/append", post(append))
        .route("/api/hosted/sync/close", post(close))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pair {
    identity: SyncIdentity,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    identity: SyncIdentity,
    objects: Option<BTreeSet<ObjectKey>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ticket {
    ticket: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    ticket: String,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadChunk {
    ticket: String,
    record: usize,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Begin {
    identity: SyncIdentity,
    record: Record,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Append {
    ticket: String,
    offset: usize,
    bytes: String,
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Pair>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (_, identity) = owner(inner, &auth, Some(&req.identity))?;
            publish_bound(inner, &identity)?;
            value(inner.core.sync_status(&identity).map_err(sync_error)?)
        })
        .await?,
    ))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let (actor, identity) = owner(inner, &auth, Some(&req.identity))?;
        publish_bound(inner, &identity)?;
        expire_revoked(inner);
        if inner.sync.downloads.len() >= 4 { return Err(ApiError::bad(UiMessage::error_api_hosted_sync_transport_sync_download_capacity())); }
        let bundle = match req.objects {
            Some(objects) => inner.core.export_sync_objects(&identity, &objects),
            None => inner.core.export_sync_history(&identity),
        }.map_err(sync_error)?;
        let records = sync_wire::records(bundle).map_err(|_| ApiError::internal(UiMessage::error_api_hosted_sync_transport_sync_record_encoding_failed()))?;
        let bytes: usize = records.iter().map(|(r, _)| r.bytes).sum();
        let retained: usize = inner.sync.downloads.values().flat_map(|v| &v.records).map(|(r, _)| r.bytes).sum();
        if records.len() > sync_wire::MAX_RECORDS || bytes > MAX_BYTES.saturating_sub(retained) { return Err(ApiError::bad(UiMessage::error_api_hosted_sync_transport_sync_download_budget())); }
        let ticket = uuid::Uuid::new_v4().to_string();
        let result = json!({"ticket":ticket,"identity":identity,"records":records.len(),"bytes":bytes,"chunkBytes":CHUNK});
        inner.sync.downloads.insert(ticket, Snapshot { lease: lease(inner, actor), records });
        Ok(result)
    }).await?))
}
async fn catalog(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Catalog>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let (actor, _) = owner(inner, &auth, None)?;
        let snapshot = inner.sync.downloads.get(&req.ticket).filter(|v| current(inner, &actor, &v.lease)).ok_or_else(unavailable)?;
        if req.offset > snapshot.records.len() { return Err(unavailable()); }
        let end = req.offset.saturating_add(128).min(snapshot.records.len());
        Ok(json!({"offset":req.offset,"next":end,"complete":end == snapshot.records.len(),"records":snapshot.records[req.offset..end].iter().map(|(r, _)|r).collect::<Vec<_>>()}))
    }).await?))
}
async fn chunk(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<ReadChunk>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let (actor, _) = owner(inner, &auth, None)?;
        let snapshot = inner.sync.downloads.get(&req.ticket).filter(|v| current(inner, &actor, &v.lease)).ok_or_else(unavailable)?;
        let (record, bytes) = snapshot.records.get(req.record).ok_or_else(unavailable)?;
        if req.offset > bytes.len() || !req.offset.is_multiple_of(CHUNK) { return Err(unavailable()); }
        let end = req.offset.saturating_add(CHUNK).min(bytes.len());
        Ok(json!({"record":record,"offset":req.offset,"next":end,"complete":end==bytes.len(),"bytes":base64::engine::general_purpose::STANDARD.encode(&bytes[req.offset..end])}))
    }).await?))
}
async fn upload(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Begin>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, identity) = owner(inner, &auth, Some(&req.identity))?;
            if !req.record.validate() {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_sync_transport_sync_record_metadata_invalid(),
                ));
            }
            if let Some(bytes) = inner
                .core
                .sync_record(&identity, req.record.manifest, &req.record.id)
                .map_err(sync_error)?
            {
                if bytes.len() != req.record.bytes {
                    return Err(ApiError::bad(
                        UiMessage::error_api_hosted_sync_transport_sync_record_length_conflict(),
                    ));
                }
                return Ok(json!({"ticket":null,"offset":bytes.len(),"complete":true}));
            }
            expire_revoked(inner);
            // An interrupted HTTP client reconnects to the same actor/epoch buffer.
            if let Some((ticket, value)) = inner
                .sync
                .uploads
                .iter()
                .find(|(_, v)| v.record == req.record && current(inner, &actor, &v.lease))
            {
                return Ok(
                    json!({"ticket":ticket,"offset":value.bytes.len(),"complete":value.complete}),
                );
            }
            let reserved: usize = inner.sync.uploads.values().map(|v| v.record.bytes).sum();
            if inner.sync.uploads.len() >= 32
                || req.record.bytes > MAX_BYTES.saturating_sub(reserved)
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_sync_transport_sync_upload_budget(),
                ));
            }
            let ticket = uuid::Uuid::new_v4().to_string();
            let lease = lease(inner, actor);
            inner.sync.uploads.insert(
                ticket.clone(),
                Upload {
                    lease,
                    record: req.record,
                    bytes: Vec::new(),
                    complete: false,
                },
            );
            Ok(json!({"ticket":ticket,"offset":0,"complete":false}))
        })
        .await?,
    ))
}
async fn append(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Append>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, identity) = owner(inner, &auth, None)?;
            let upload = inner
                .sync
                .uploads
                .get(&req.ticket)
                .filter(|v| current(inner, &actor, &v.lease))
                .ok_or_else(unavailable)?;
            if req.bytes.len() > CHUNK * 2 {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_sync_transport_sync_chunk_budget(),
                ));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&req.bytes)
                .map_err(|_| {
                    ApiError::bad(
                        UiMessage::error_api_hosted_sync_transport_sync_chunk_encoding_invalid(),
                    )
                })?;
            if bytes.len() > CHUNK
                || !req.offset.is_multiple_of(CHUNK)
                || req.offset > upload.bytes.len()
                || bytes.len() != CHUNK.min(upload.record.bytes.saturating_sub(req.offset))
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_sync_transport_sync_chunk_position_invalid(),
                ));
            }
            if upload.complete {
                if upload.bytes[req.offset..req.offset + bytes.len()] != bytes {
                    return Err(ApiError::bad(
                        UiMessage::error_api_hosted_sync_transport_sync_duplicate_chunk_conflict(),
                    ));
                }
                return Ok(json!({"offset":upload.record.bytes,"complete":true}));
            }
            let end = req.offset + bytes.len();
            if req.offset < upload.bytes.len() {
                if end > upload.bytes.len() || upload.bytes[req.offset..end] != bytes {
                    return Err(ApiError::bad(
                        UiMessage::error_api_hosted_sync_transport_sync_duplicate_chunk_conflict(),
                    ));
                }
                return Ok(json!({"offset":upload.bytes.len(),"complete":false}));
            }
            let upload = inner.sync.uploads.get_mut(&req.ticket).unwrap();
            upload.bytes.extend(bytes);
            if upload.bytes.len() == upload.record.bytes {
                if let Err(error) = inner.core.receive_sync_record(
                    &identity,
                    upload.record.manifest,
                    &upload.record.id,
                    upload.bytes.clone(),
                ) {
                    inner.sync.uploads.remove(&req.ticket);
                    return Err(sync_error(error));
                }
                upload.complete = true;
            }
            Ok(json!({"offset":upload.bytes.len(),"complete":upload.complete}))
        })
        .await?,
    ))
}
async fn close(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Ticket>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, _) = owner(inner, &auth, None)?;
            if inner
                .sync
                .downloads
                .get(&req.ticket)
                .is_some_and(|v| current(inner, &actor, &v.lease))
            {
                inner.sync.downloads.remove(&req.ticket);
            }
            if inner
                .sync
                .uploads
                .get(&req.ticket)
                .is_some_and(|v| current(inner, &actor, &v.lease))
            {
                inner.sync.uploads.remove(&req.ticket);
            }
            Ok(json!({"status":"closed"}))
        })
        .await?,
    ))
}
