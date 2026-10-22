//! Local-owner HTTP client. Network waits never hold the author Core mutex.
//! Jobs copy content-addressed records only; apply remains an explicit Plan.
use super::*;
use crate::sync_wire::{self, CHUNK, MAX_BYTES, Record};
use base64::Engine as _;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use weftext_core::sync::{ObjectKey, SyncBundle, SyncIdentity};

#[derive(Default)]
pub(crate) struct Store {
    jobs: HashMap<String, Job>,
}
struct Job {
    fingerprint: String,
    info: Progress,
    stop: Arc<AtomicBool>,
    created: Instant,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    request_id: String,
    status: String,
    records_done: usize,
    records_total: usize,
    bytes_done: usize,
    bytes_total: usize,
    #[serde(flatten)]
    description: ErrorDescription,
    identity: SyncIdentity,
    error: Option<ApiError>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Connection {
    endpoint: String,
    token: String,
    identity: SyncIdentity,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Start {
    request_id: String,
    connection: Connection,
    direction: Direction,
    root: Option<String>,
    objects: Option<BTreeSet<ObjectKey>>,
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Direction {
    Push,
    Pull,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JobRequest {
    request_id: String,
}
struct Client {
    http: reqwest::Client,
    endpoint: url::Url,
}
impl Client {
    fn new(connection: &Connection) -> Result<Self, ApiError> {
        let endpoint = url::Url::parse(&connection.endpoint).map_err(|_| {
            ApiError::bad(UiMessage::error_api_remote_sync_server_address_required())
        })?;
        let loopback = endpoint.host_str().is_some_and(|h| {
            h.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        if !(endpoint.scheme() == "https" || endpoint.scheme() == "http" && loopback)
            || endpoint.path() != "/"
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.host_str().is_none()
            || connection.token.len() < 32
            || connection.token.len() > 512
        {
            return Err(ApiError::bad(
                UiMessage::error_api_remote_sync_secure_root_address_required(),
            ));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        let mut token =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", connection.token))
                .map_err(
                    |_| ApiError::bad(UiMessage::error_api_remote_sync_invalid_credential()),
                )?;
        token.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, token);
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .map_err(|_| {
                ApiError::internal(UiMessage::error_api_remote_sync_client_initialization_failed())
            })?;
        Ok(Self { http, endpoint })
    }
    async fn call<T: DeserializeOwned>(
        &self,
        route: &str,
        request: Value,
        limit: usize,
    ) -> Result<T, ApiError> {
        let url = self
            .endpoint
            .join(&format!("api/hosted/sync/{route}"))
            .map_err(
                |_| ApiError::bad(UiMessage::error_api_remote_sync_invalid_server_address()),
            )?;
        let mut response = self
            .http
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|_| {
                ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "syncConnection",
                    UiMessage::error_api_remote_sync_connection_incomplete(),
                )
            })?;
        if !response.status().is_success() {
            return Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "syncRemoteRejected",
                UiMessage::error_api_remote_sync_remote_rejected(
                    weftext_messages::params::ErrorApiRemoteSyncRemoteRejected {
                        status: weftext_messages::UiInteger::new(i64::from(
                            response.status().as_u16(),
                        ))
                        .expect("HTTP status is a safe integer"),
                    },
                ),
            ));
        }
        if response
            .content_length()
            .is_some_and(|len| len > limit as u64)
        {
            return Err(ApiError::bad(
                UiMessage::error_api_remote_sync_response_budget(),
            ));
        }
        let mut bytes = Vec::new();
        while let Some(part) = response
            .chunk()
            .await
            .map_err(|_| ApiError::bad(UiMessage::error_api_remote_sync_response_interrupted()))?
        {
            if part.len() > limit.saturating_sub(bytes.len()) {
                return Err(ApiError::bad(
                    UiMessage::error_api_remote_sync_response_budget(),
                ));
            }
            bytes.extend_from_slice(&part);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| ApiError::bad(UiMessage::error_api_remote_sync_invalid_response()))
    }
    async fn close(&self, ticket: &str) {
        let _: Result<Value, _> = self.call("close", json!({"ticket":ticket}), 4096).await;
    }
}
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sync/remote/inspect", post(inspect))
        .route("/api/sync/remote/start", post(start))
        .route("/api/sync/remote/job", post(job))
        .route("/api/sync/remote/stop", post(stop))
}
async fn inspect(RequestJson(req): RequestJson<Connection>) -> ApiResult {
    let client = Client::new(&req)?;
    let result: Value = client
        .call("status", json!({"identity":req.identity}), 16 * 1024 * 1024)
        .await?;
    if result["identity"] != value(&req.identity)? {
        return Err(ApiError::bad(
            UiMessage::error_api_remote_sync_pairing_mismatch(),
        ));
    }
    Ok(Json(result))
}
async fn start(State(state): State<AppState>, RequestJson(req): RequestJson<Start>) -> ApiResult {
    if uuid::Uuid::parse_str(&req.request_id).is_err() {
        return Err(ApiError::bad(
            UiMessage::error_api_remote_sync_invalid_transfer_id(),
        ));
    }
    let client = Client::new(&req.connection)?;
    weftext_core::sync::ReplicaGraph::new(
        &req.connection.identity.workspace_id,
        &req.connection.identity.genesis,
    )?;
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&req).map_err(|_| ApiError::internal(
            UiMessage::error_api_remote_sync_transfer_encoding_failed()
        ))?)
    );
    let stop = Arc::new(AtomicBool::new(false));
    let info = Progress {
        request_id: req.request_id.clone(),
        status: "running".into(),
        records_done: 0,
        records_total: 0,
        bytes_done: 0,
        bytes_total: 0,
        description: UiMessage::error_api_remote_sync_pairing_and_freezing().into(),
        identity: req.connection.identity.clone(),
        error: None,
    };
    {
        let mut store = state.sync_remote.lock().map_err(|_| {
            ApiError::internal(UiMessage::error_api_remote_sync_state_unavailable())
        })?;
        if let Some(original) = store.jobs.get(&req.request_id) {
            if original.fingerprint != fingerprint {
                return Err(ApiError::bad(
                    UiMessage::error_api_remote_sync_transfer_request_reuse(),
                ));
            }
            return Ok(Json(value(&original.info)?));
        }
        if store.jobs.values().any(|v| v.info.status == "running") {
            return Err(ApiError::bad(
                UiMessage::error_api_remote_sync_transfer_already_running(),
            ));
        }
        store
            .jobs
            .retain(|_, v| v.created.elapsed() < Duration::from_secs(3600));
        if store.jobs.len() >= 64 {
            return Err(ApiError::bad(
                UiMessage::error_api_remote_sync_retained_transfer_budget(),
            ));
        }
        store.jobs.insert(
            req.request_id.clone(),
            Job {
                fingerprint,
                info: info.clone(),
                stop: stop.clone(),
                created: Instant::now(),
            },
        );
    }
    tokio::spawn(async move {
        let id = req.request_id.clone();
        let result = tokio::time::timeout(
            Duration::from_secs(15 * 60),
            transfer(&state, req, client, stop.clone()),
        )
        .await;
        update(&state, &id, |p| match result {
            Ok(Ok(())) => {
                p.status = "complete".into();
                p.description = UiMessage::error_api_remote_sync_transfer_complete().into();
            }
            Ok(Err(error)) => {
                p.status = if stop.load(Ordering::Relaxed) {
                    "stopped"
                } else {
                    "failed"
                }
                .into();
                p.description = UiMessage::error_api_remote_sync_transfer_failed().into();
                p.error = Some(error);
            }
            Err(_) => {
                p.status = "failed".into();
                p.description = UiMessage::error_api_remote_sync_transfer_timed_out().into();
            }
        });
    });
    Ok(Json(value(info)?))
}
fn update(state: &AppState, id: &str, f: impl FnOnce(&mut Progress)) {
    if let Ok(mut store) = state.sync_remote.lock()
        && let Some(job) = store.jobs.get_mut(id)
    {
        f(&mut job.info);
    }
}
fn check_stop(stop: &AtomicBool) -> Result<(), ApiError> {
    if stop.load(Ordering::Relaxed) {
        Err(ApiError::bad(
            UiMessage::error_api_remote_sync_transfer_stopped(),
        ))
    } else {
        Ok(())
    }
}
async fn job(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<JobRequest>,
) -> ApiResult {
    let store = state
        .sync_remote
        .lock()
        .map_err(|_| ApiError::internal(UiMessage::error_api_remote_sync_state_unavailable()))?;
    Ok(Json(match store.jobs.get(&req.request_id) {
        Some(job) => value(&job.info)?,
        None => {
            json!({"requestId":req.request_id,"status":"unknown","explanation":UiMessage::error_api_remote_sync_transfer_record_unavailable(),"diagnostics":[],"causes":[],"error":null})
        }
    }))
}
async fn stop(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<JobRequest>,
) -> ApiResult {
    let mut store = state
        .sync_remote
        .lock()
        .map_err(|_| ApiError::internal(UiMessage::error_api_remote_sync_state_unavailable()))?;
    let job = store.jobs.get_mut(&req.request_id).ok_or_else(|| {
        ApiError::bad(UiMessage::error_api_remote_sync_original_transfer_missing())
    })?;
    if job.info.status == "running" {
        job.stop.store(true, Ordering::Relaxed);
        job.info.description = UiMessage::error_api_remote_sync_stopping().into();
    }
    Ok(Json(value(&job.info)?))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepared {
    ticket: String,
    identity: SyncIdentity,
    records: usize,
    bytes: usize,
    chunk_bytes: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Page {
    offset: usize,
    next: usize,
    complete: bool,
    records: Vec<Record>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadChunk {
    record: Record,
    offset: usize,
    next: usize,
    complete: bool,
    bytes: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Begun {
    ticket: Option<String>,
    offset: usize,
    complete: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Written {
    offset: usize,
    complete: bool,
}

async fn transfer(
    state: &AppState,
    req: Start,
    client: Client,
    stop: Arc<AtomicBool>,
) -> Result<(), ApiError> {
    check_stop(&stop)?;
    let identity = req.connection.identity;
    match req.direction {
        Direction::Push => {
            let root = req.root.ok_or_else(|| {
                ApiError::bad(UiMessage::error_api_remote_sync_local_pairing_required())
            })?;
            let requested = req.objects;
            let paired = identity.clone();
            let records = core_call(state, move |core| {
                if core.sync_binding_info(&root)?.identity != paired {
                    return Err(CoreError::new(
                        "syncIdentityMismatch",
                        UiMessage::error_api_remote_sync_local_pairing_mismatch(),
                    ));
                }
                core.publish_sync(&root)?;
                let bundle = match requested {
                    Some(objects) => core.export_sync_objects(&paired, &objects)?,
                    None => core.export_sync_history(&paired)?,
                };
                Ok(sync_wire::records(bundle)?)
            })
            .await?;
            let metas: Vec<_> = records.iter().map(|(r, _)| r.clone()).collect();
            if !sync_wire::check_catalog(&metas) {
                return Err(ApiError::bad(
                    UiMessage::error_api_remote_sync_local_transfer_budget(),
                ));
            }
            update(state, &req.request_id, |p| {
                p.records_total = records.len();
                p.bytes_total = metas.iter().map(|r| r.bytes).sum();
            });
            for (record, bytes) in records {
                check_stop(&stop)?;
                let begun: Begun = client
                    .call("upload", json!({"identity":identity,"record":record}), 4096)
                    .await?;
                if begun.offset > record.bytes
                    || (!begun.complete && !begun.offset.is_multiple_of(CHUNK))
                    || (begun.complete && begun.offset != record.bytes)
                {
                    return Err(ApiError::bad(
                        UiMessage::error_api_remote_sync_invalid_upload_progress(),
                    ));
                }
                if !begun.complete {
                    let ticket = begun.ticket.ok_or_else(|| {
                        ApiError::bad(UiMessage::error_api_remote_sync_upload_ticket_missing())
                    })?;
                    let result = async {
                        let mut offset = begun.offset;
                        loop {
                            check_stop(&stop)?;
                            let end = offset.saturating_add(CHUNK).min(bytes.len());
                            let written: Written = client.call("append", json!({"ticket":ticket,"offset":offset,"bytes":base64::engine::general_purpose::STANDARD.encode(&bytes[offset..end])}), 4096).await?;
                            if written.offset != end || written.complete != (end == bytes.len()) { return Err(ApiError::bad(UiMessage::error_api_remote_sync_chunk_not_acknowledged())); }
                            if written.complete { break; } offset = end;
                        }
                        Ok(())
                    }.await;
                    client.close(&ticket).await;
                    result?;
                }
                update(state, &req.request_id, |p| {
                    p.records_done += 1;
                    p.bytes_done += record.bytes;
                    p.description = UiMessage::error_api_remote_sync_record_acknowledged().into();
                });
            }
        }
        Direction::Pull => {
            let prepared: Prepared = client
                .call(
                    "prepare",
                    json!({"identity":identity,"objects":req.objects}),
                    4096,
                )
                .await?;
            if prepared.identity != identity
                || prepared.records > sync_wire::MAX_RECORDS
                || prepared.bytes > MAX_BYTES
                || prepared.chunk_bytes != CHUNK
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_remote_sync_remote_pairing_or_budget_mismatch(),
                ));
            }
            let ticket = prepared.ticket;
            let result = async {
                let mut records = Vec::new();
                while records.len() < prepared.records {
                    check_stop(&stop)?;
                    let page: Page = client
                        .call(
                            "catalog",
                            json!({"ticket":ticket,"offset":records.len()}),
                            128 * 1024,
                        )
                        .await?;
                    if page.offset != records.len()
                        || page.records.is_empty()
                        || page.records.len() > 128
                        || page.next != records.len() + page.records.len()
                        || page.next > prepared.records
                        || page.complete != (page.next == prepared.records)
                    {
                        return Err(ApiError::bad(
                            UiMessage::error_api_remote_sync_catalog_page_discontinuous(),
                        ));
                    }
                    records.extend(page.records);
                }
                if !sync_wire::check_catalog(&records)
                    || records.iter().map(|r| r.bytes).sum::<usize>() != prepared.bytes
                {
                    return Err(ApiError::bad(
                        UiMessage::error_api_remote_sync_catalog_incomplete(),
                    ));
                }
                let paired = identity.clone();
                core_call(state, move |core| {
                    core.receive_sync(&paired, SyncBundle::default())
                })
                .await?;
                update(state, &req.request_id, |p| {
                    p.records_total = records.len();
                    p.bytes_total = prepared.bytes;
                });
                for (index, record) in records.into_iter().enumerate() {
                    check_stop(&stop)?;
                    let paired = identity.clone();
                    let meta = record.clone();
                    let present = core_call(state, move |core| {
                        Ok(core
                            .sync_record(&paired, meta.manifest, &meta.id)?
                            .map(|b| b.len()))
                    })
                    .await?;
                    if let Some(len) = present {
                        if len != record.bytes {
                            return Err(ApiError::bad(
                                UiMessage::error_api_remote_sync_record_length_mismatch(),
                            ));
                        }
                    } else {
                        let mut bytes = Vec::new();
                        loop {
                            check_stop(&stop)?;
                            let part: ReadChunk = client
                                .call(
                                    "chunk",
                                    json!({"ticket":ticket,"record":index,"offset":bytes.len()}),
                                    CHUNK * 2 + 4096,
                                )
                                .await?;
                            let data = base64::engine::general_purpose::STANDARD
                                .decode(part.bytes)
                                .map_err(|_| {
                                    ApiError::bad(UiMessage::error_api_remote_sync_chunk_encoding())
                                })?;
                            let end = bytes.len().saturating_add(CHUNK).min(record.bytes);
                            if part.record != record
                                || part.offset != bytes.len()
                                || part.next != end
                                || data.len() != end - bytes.len()
                                || part.complete != (end == record.bytes)
                            {
                                return Err(ApiError::bad(
                                    UiMessage::error_api_remote_sync_chunk_mismatch(),
                                ));
                            }
                            bytes.extend(data);
                            if part.complete {
                                break;
                            }
                        }
                        let paired = identity.clone();
                        let meta = record.clone();
                        core_call(state, move |core| {
                            core.receive_sync_record(&paired, meta.manifest, &meta.id, bytes)
                        })
                        .await?;
                    }
                    update(state, &req.request_id, |p| {
                        p.records_done += 1;
                        p.bytes_done += record.bytes;
                        p.description = UiMessage::error_api_remote_sync_record_retained().into();
                    });
                }
                Ok(())
            }
            .await;
            client.close(&ticket).await;
            result?;
        }
    }
    Ok(())
}
