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
    closing: bool,
}
impl Store {
    pub(crate) fn close(&mut self) {
        self.closing = true;
        for job in self.jobs.values() {
            job.stop.store(true, Ordering::Release);
        }
    }
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
    message: String,
    identity: SyncIdentity,
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
        let endpoint = url::Url::parse(&connection.endpoint)
            .map_err(|_| ApiError::bad("请输入准确服务地址"))?;
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
                "使用 HTTPS 服务根地址（本机回环可用 HTTP），凭据单独填写",
            ));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        let mut token =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", connection.token))
                .map_err(|_| ApiError::bad("凭据格式无效"))?;
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
            .map_err(|_| ApiError::internal("同步网络客户端初始化失败"))?;
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
            .map_err(|_| ApiError::bad("服务地址无效"))?;
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
                    "远端连接未完成；已接收的完整记录保留，可重新开始传输",
                )
            })?;
        if !response.status().is_success() {
            return Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "syncRemoteRejected",
                format!(
                    "远端拒绝继续传输（HTTP {}）；核对配对和当前同步授权",
                    response.status().as_u16()
                ),
            ));
        }
        if response
            .content_length()
            .is_some_and(|len| len > limit as u64)
        {
            return Err(ApiError::bad("远端响应超过同步预算"));
        }
        let mut bytes = Vec::new();
        while let Some(part) = response
            .chunk()
            .await
            .map_err(|_| ApiError::bad("远端响应中断，完整记录仍保留"))?
        {
            if part.len() > limit.saturating_sub(bytes.len()) {
                return Err(ApiError::bad("远端响应超过同步预算"));
            }
            bytes.extend_from_slice(&part);
        }
        serde_json::from_slice(&bytes).map_err(|_| ApiError::bad("远端响应格式无效"))
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
async fn inspect(Json(req): Json<Connection>) -> ApiResult {
    let client = Client::new(&req)?;
    let result: Value = client
        .call("status", json!({"identity":req.identity}), 16 * 1024 * 1024)
        .await?;
    if result["identity"] != value(&req.identity)? {
        return Err(ApiError::bad("远端配对身份与请求不一致"));
    }
    Ok(Json(result))
}
async fn start(State(state): State<AppState>, Json(req): Json<Start>) -> ApiResult {
    if uuid::Uuid::parse_str(&req.request_id).is_err() {
        return Err(ApiError::bad("传输编号无效"));
    }
    let client = Client::new(&req.connection)?;
    weftext_core::sync::ReplicaGraph::new(
        &req.connection.identity.workspace_id,
        &req.connection.identity.genesis,
    )?;
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&req).map_err(|_| ApiError::internal("同步记录编码失败"))?
        )
    );
    let stop = Arc::new(AtomicBool::new(false));
    let info = Progress {
        request_id: req.request_id.clone(),
        status: "running".into(),
        records_done: 0,
        records_total: 0,
        bytes_done: 0,
        bytes_total: 0,
        message: "正在配对并冻结准确传输记录".into(),
        identity: req.connection.identity.clone(),
    };
    {
        let mut store = state
            .sync_remote
            .lock()
            .map_err(|_| ApiError::internal("同步状态不可用"))?;
        if let Some(original) = store.jobs.get(&req.request_id) {
            if original.fingerprint != fingerprint {
                return Err(ApiError::bad("同一传输编号不能用于不同请求"));
            }
            return Ok(Json(value(&original.info)?));
        }
        if store.closing {
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "hostClosing",
                "The owner is closing; no new transfer was started",
            ));
        }
        if store.jobs.values().any(|v| v.info.status == "running") {
            return Err(ApiError::bad("已有远程传输进行中，可查看或停止原传输"));
        }
        store
            .jobs
            .retain(|_, v| v.created.elapsed() < Duration::from_secs(3600));
        if store.jobs.len() >= 64 {
            return Err(ApiError::bad("本机传输记录已达预算，旧记录一小时后过期"));
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
    crate::work::spawn(async move {
        let id = req.request_id.clone();
        let result = tokio::time::timeout(
            Duration::from_secs(15 * 60),
            transfer(&state, req, client, stop.clone()),
        )
        .await;
        update(&state, &id, |p| match result {
            Ok(Ok(())) => {
                p.status = "complete".into();
                p.message = "传输完成；接收内容仍须预览后应用到资料库".into();
            }
            Ok(Err(error)) => {
                p.status = if stop.load(Ordering::Relaxed) {
                    "stopped"
                } else {
                    "failed"
                }
                .into();
                p.message = error.message;
            }
            Err(_) => {
                p.status = "failed".into();
                p.message = "传输超时；已接收完整记录保留，重新连接可复用".into();
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
            "传输已停止；已接收完整记录保留，未写入资料库",
        ))
    } else {
        Ok(())
    }
}
async fn job(State(state): State<AppState>, Json(req): Json<JobRequest>) -> ApiResult {
    let store = state
        .sync_remote
        .lock()
        .map_err(|_| ApiError::internal("同步状态不可用"))?;
    Ok(Json(match store.jobs.get(&req.request_id) {
        Some(job) => value(&job.info)?,
        None => {
            json!({"requestId":req.request_id,"status":"unknown","message":"原传输记录已过期或进程已重启。已接收的完整记录保留，可明确开始新传输"})
        }
    }))
}
async fn stop(State(state): State<AppState>, Json(req): Json<JobRequest>) -> ApiResult {
    let mut store = state
        .sync_remote
        .lock()
        .map_err(|_| ApiError::internal("同步状态不可用"))?;
    let job = store
        .jobs
        .get_mut(&req.request_id)
        .ok_or_else(|| ApiError::bad("没有进行中的原传输"))?;
    if job.info.status == "running" {
        job.stop.store(true, Ordering::Relaxed);
        job.info.message = "正在停止；当前块可能已经接收，之后不再请求新块".into();
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
            let root = req
                .root
                .ok_or_else(|| ApiError::bad("上传须选择本机已配对资料库"))?;
            let requested = req.objects;
            let paired = identity.clone();
            let records = core_call(state, move |core| {
                if core.sync_binding_info(&root)?.identity != paired {
                    return Err(CoreError::new(
                        "syncIdentityMismatch",
                        "Local replica pairing differs",
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
                return Err(ApiError::bad("本机传输记录超过预算"));
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
                    return Err(ApiError::bad("远端上传进度无效"));
                }
                if !begun.complete {
                    let ticket = begun
                        .ticket
                        .ok_or_else(|| ApiError::bad("远端未返回上传票据"))?;
                    let result = async {
                        let mut offset = begun.offset;
                        loop {
                            check_stop(&stop)?;
                            let end = offset.saturating_add(CHUNK).min(bytes.len());
                            let written: Written = client.call("append", json!({"ticket":ticket,"offset":offset,"bytes":base64::engine::general_purpose::STANDARD.encode(&bytes[offset..end])}), 4096).await?;
                            if written.offset != end || written.complete != (end == bytes.len()) { return Err(ApiError::bad("远端没有确认准确同步块")); }
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
                    p.message = "原始不可变记录已由远端确认".into();
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
                return Err(ApiError::bad("远端配对或传输预算不符"));
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
                        return Err(ApiError::bad("远端目录分页不连续"));
                    }
                    records.extend(page.records);
                }
                if !sync_wire::check_catalog(&records)
                    || records.iter().map(|r| r.bytes).sum::<usize>() != prepared.bytes
                {
                    return Err(ApiError::bad("远端目录不完整或超过预算"));
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
                            return Err(ApiError::bad("已接收记录与远端长度不同"));
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
                                .map_err(|_| ApiError::bad("远端同步块编码无效"))?;
                            let end = bytes.len().saturating_add(CHUNK).min(record.bytes);
                            if part.record != record
                                || part.offset != bytes.len()
                                || part.next != end
                                || data.len() != end - bytes.len()
                                || part.complete != (end == record.bytes)
                            {
                                return Err(ApiError::bad("远端同步块与冻结记录不一致"));
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
                        p.message = "完整记录已通过摘要检查并保留，资料库尚未改变".into();
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
