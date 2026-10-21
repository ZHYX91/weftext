//! Process-isolated, offline recognition jobs and reviewed Core import.
//! Private job files are execution provenance, never a second author store.
use super::*;
use axum::extract::DefaultBodyLimit;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;
use weftext_core::{ActorFeatureStatus, ConfigRole, OperationIntent, OperationRequest};
use weftext_export::recognition::RecognitionReview;

pub(crate) const MAX_INPUT: usize = 16 * 1024 * 1024;
const MAX_RESULT: usize = 32 * 1024 * 1024;
const TTL: u64 = 900_000;
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "recognitionChanged",
        "原件、设置、授权或导入计划已变化；保留原输入并核对原操作。",
    )
}
fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "recognitionUnavailable",
        "独立识别运行时不可用；仍可保留原件，请检查执行端依赖。",
    )
}
fn io_error(_: std::io::Error) -> ApiError {
    ApiError::internal("识别作业资料无法读取或写入；原件与已有结果保留。")
}
pub(crate) fn uuid(value: &str) -> Result<(), ApiError> {
    if uuid::Uuid::parse_str(value)
        .is_ok_and(|v| v.to_string() == value && v.get_version_num() == 4)
    {
        Ok(())
    } else {
        Err(ApiError::bad("操作编号须为准确 UUID v4"))
    }
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ApiError> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io_error)?;
    f.write_all(bytes).map_err(io_error)?;
    f.sync_all().map_err(io_error)
}
fn read_json(path: &Path, limit: usize) -> Result<Value, ApiError> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(io_error)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(changed());
    }
    serde_json::from_slice(&bytes).map_err(|_| changed())
}
fn immutable(path: &Path, bytes: &[u8]) -> Result<(), ApiError> {
    match fs::read(path) {
        Ok(old) if old == bytes => Ok(()),
        Ok(_) => Err(changed()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => write_new(path, bytes),
        Err(e) => Err(io_error(e)),
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Target {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) slot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
}
fn same_target(a: &Target, b: &Target) -> bool {
    if a.slot.is_some() || b.slot.is_some() {
        a == b
    } else {
        a.parent == b.parent && a.name == b.name
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Start {
    pub(crate) job_id: String,
    pub(crate) target: Target,
    pub(crate) format: String,
    pub(crate) bytes_base64: String,
    pub(crate) options: Value,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct JobRequest {
    pub(crate) job_id: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Prepare {
    pub(crate) job_id: String,
    pub(crate) operation_id: String,
    pub(crate) target: Target,
    pub(crate) review: RecognitionReview,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Operation {
    pub(crate) target: Target,
    pub(crate) operation_id: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Commit {
    pub(crate) target: Target,
    pub(crate) operation_id: String,
    pub(crate) plan_id: String,
    pub(crate) accept_losses: bool,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Preview {
    pub(crate) job_id: String,
    pub(crate) page: u64,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Fields {
    pub(crate) target: Target,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Context {
    pub(crate) scope: String,
    pub(crate) resource: String,
    pub(crate) root: PathBuf,
    pub(crate) parent: PathBuf,
    pub(crate) name: String,
    pub(crate) epoch: u64,
    pub(crate) binding: String,
    pub(crate) target: Target,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Claim {
    version: u32,
    pub(crate) request: Start,
    pub(crate) context: Context,
    pub(crate) input_sha256: String,
    pub(crate) options_digest: String,
    implementation_digest: String,
    runtime_digest: Option<String>,
    pub(crate) born: u64,
}
#[derive(Default)]
pub(crate) struct Store {
    pub(crate) root: Option<PathBuf>,
    runtime: Option<PathBuf>,
    active: BTreeMap<String, Arc<AtomicBool>>,
}
impl Store {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            runtime: std::env::var_os("WEFTEXT_RECOGNITION_RUNTIME").map(PathBuf::from),
            ..Default::default()
        }
    }
    pub(crate) fn runtime(&self) -> Option<PathBuf> {
        self.runtime.clone().filter(|p| {
            p.is_absolute()
                && p.join("venv/Scripts/python.exe").is_file()
                && p.join("installation.json").is_file()
        })
    }
    fn key(scope: &str, id: &str) -> String {
        format!("{}-{id}", hash(scope.as_bytes()))
    }
    pub(crate) fn directory(&self, scope: &str, id: &str) -> Result<PathBuf, ApiError> {
        uuid(id)?;
        Ok(self
            .root
            .as_ref()
            .ok_or_else(unavailable)?
            .join(hash(scope.as_bytes()))
            .join(id))
    }
    pub(crate) fn claim(&self, scope: &str, id: &str) -> Result<Claim, ApiError> {
        let p = self.directory(scope, id)?.join("claim.json");
        if !p.is_file() {
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "recognitionUnavailable",
                "识别作业当前不可用。",
            ));
        }
        serde_json::from_value(read_json(&p, 24 * 1024 * 1024)?).map_err(|_| changed())
    }
    pub(crate) fn cancel(&mut self, scope: &str, id: &str) -> Result<(), ApiError> {
        let p = self.directory(scope, id)?;
        let _ = self.claim(scope, id)?;
        immutable(&p.join("cancelled"), b"cancelled")?;
        if let Some(c) = self.active.get(&Self::key(scope, id)) {
            c.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
    pub(crate) fn reply(&self, claim: &Claim) -> Result<Value, ApiError> {
        let dir = self.directory(&claim.context.scope, &claim.request.job_id)?;
        // A retained result never authorizes reading a substituted original.
        let _ = original(self, claim)?;
        let mut status = if dir.join("cancelled").exists() {
            "cancelled"
        } else if self
            .active
            .contains_key(&Self::key(&claim.context.scope, &claim.request.job_id))
        {
            "running"
        } else {
            "interrupted"
        }
        .to_string();
        let mut reply = json!({"version":1,"jobId":claim.request.job_id,"inputSha256":claim.input_sha256,"optionsDigest":claim.options_digest,"expiresAt":claim.born.saturating_add(TTL),"confirmationExpired":claim.born.saturating_add(TTL)<now()});
        if dir.join("result.json").exists() && !dir.join("cancelled").exists() {
            let value = read_json(&dir.join("result.json"), MAX_RESULT)?;
            status = value["result"]["status"]
                .as_str()
                .filter(|s| matches!(*s, "complete" | "partial" | "failed"))
                .unwrap_or("failed")
                .to_string();
            if value["result"]["workerFinalValidation"] == false {
                status = "interrupted".into();
            }
            reply["resultDigest"] =
                json!(hash(&fs::read(dir.join("result.json")).map_err(io_error)?));
            reply["result"] = value["result"].clone();
            reply["scheme"] = value["scheme"].clone();
        } else if dir.join("progress.json").exists()
            && !dir.join("cancelled").exists()
            && !self
                .active
                .contains_key(&Self::key(&claim.context.scope, &claim.request.job_id))
        {
            let value = checkpoint(claim, &dir)?;
            reply["result"] = value["result"].clone();
            reply["scheme"] = Value::Null;
            reply["resultDigest"] =
                json!(hash(&serde_json::to_vec(&value).map_err(|_| changed())?));
        }
        reply["status"] = json!(status);
        Ok(reply)
    }
    pub(crate) fn begin(
        &mut self,
        context: Context,
        req: Start,
    ) -> Result<(Claim, Option<Run>), ApiError> {
        uuid(&req.job_id)?;
        let bytes = upload(&req.bytes_base64)?;
        validate_options(&req)?;
        let dir = self.directory(&context.scope, &req.job_id)?;
        let implementation = implementation_digest();
        let input_sha256 = hash(&bytes);
        let options_digest = hash(&serde_json::to_vec(&req.options).map_err(|_| changed())?);
        let mut frozen = req.clone();
        frozen.bytes_base64 = String::new();
        let runtime_digest = if req.options["purpose"] == "resource" {
            None
        } else {
            Some(runtime_fingerprint(
                self.runtime().as_deref().ok_or_else(unavailable)?,
                &job_languages(&req.options),
            )?)
        };
        let claim = Claim {
            version: 1,
            request: frozen,
            context,
            input_sha256,
            options_digest,
            implementation_digest: implementation,
            runtime_digest,
            born: now(),
        };
        if dir.join("claim.json").exists() {
            let old = self.claim(&claim.context.scope, &req.job_id)?;
            if old.input_sha256 != claim.input_sha256
                || old.options_digest != claim.options_digest
                || old.request.format != claim.request.format
                || old.context.binding != claim.context.binding
                || !same_target(&old.request.target, &claim.request.target)
                || old.context.epoch != claim.context.epoch
            {
                return Err(changed());
            }
            check_claim(&old, &claim.context)?;
            check_live(&old)?;
            return Ok((old, None));
        }
        if self.active.len() >= 2 {
            return Err(ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "recognitionBusy",
                "识别任务已达并行预算，请等待或取消原任务。",
            ));
        }
        check_store_budget(self.root.as_ref().ok_or_else(unavailable)?)?;
        let runtime = self.runtime();
        if runtime.is_none() && req.options["purpose"] != "resource" && req.format != "docx" {
            return Err(unavailable());
        }
        fs::create_dir_all(&dir).map_err(io_error)?;
        immutable(&dir.join("input.bin"), &bytes)?;
        let claim_bytes = serde_json::to_vec(&claim).map_err(|_| changed())?;
        immutable(&dir.join("claim.json"), &claim_bytes)?;
        let implementation = materialize(self.root.as_ref().unwrap())?;
        let python = runtime.as_ref().map(|r| r.join("venv/Scripts/python.exe"));
        let cancel = Arc::new(AtomicBool::new(false));
        self.active
            .insert(Self::key(&claim.context.scope, &req.job_id), cancel.clone());
        Ok((
            claim.clone(),
            Some(Run {
                claim,
                dir,
                implementation,
                runtime,
                python,
                cancel,
            }),
        ))
    }
    pub(crate) fn finish(&mut self, claim: &Claim) {
        self.active
            .remove(&Self::key(&claim.context.scope, &claim.request.job_id));
    }
}
pub(crate) struct Run {
    claim: Claim,
    dir: PathBuf,
    implementation: PathBuf,
    runtime: Option<PathBuf>,
    python: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
}
pub(crate) fn upload(value: &str) -> Result<Vec<u8>, ApiError> {
    if value.len() > MAX_INPUT.div_ceil(3) * 4 {
        return Err(ApiError::bad("识别原件上限为16MiB"));
    }
    let b = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| ApiError::bad("原件编码无效"))?;
    if b.is_empty() || b.len() > MAX_INPUT {
        return Err(ApiError::bad("原件须为1字节至16MiB"));
    }
    Ok(b)
}
fn validate_options(req: &Start) -> Result<(), ApiError> {
    if !["pdf", "png", "jpeg", "docx"].contains(&req.format.as_str())
        || !req.options.is_object()
        || serde_json::to_vec(&req.options)
            .map_err(|_| changed())?
            .len()
            > 128 * 1024
    {
        return Err(ApiError::bad("识别格式或设置无效"));
    }
    let allowed = [
        "purpose",
        "operation",
        "languages",
        "pages",
        "dpi",
        "scheme",
        "language",
        "script",
        "region",
        "numbering",
        "selectedStructureIds",
    ];
    if req
        .options
        .as_object()
        .unwrap()
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
        || !matches!(
            req.options["purpose"].as_str(),
            Some("resource" | "text" | "note")
        )
    {
        return Err(ApiError::bad("识别目的或设置无效"));
    }
    if req.format == "docx"
        && req.options["purpose"] != "resource"
        && req.options["scheme"] != "official-document-cn-mainland-hans@1"
    {
        return Err(ApiError::bad("DOCX识别仅支持已声明公文方案"));
    }
    Ok(())
}
const SCRIPTS: [(&str, &str); 8] = [
    (
        "scripts/recognition/worker.py",
        include_str!("../../../scripts/recognition/worker.py"),
    ),
    (
        "scripts/recognition/engine.py",
        include_str!("../../../scripts/recognition/engine.py"),
    ),
    (
        "scripts/recognition/install.py",
        include_str!("../../../scripts/recognition/install.py"),
    ),
    (
        "scripts/recognition/schemes.py",
        include_str!("../../../scripts/recognition/schemes.py"),
    ),
    (
        "scripts/recognition/docx_reader.py",
        include_str!("../../../scripts/recognition/docx_reader.py"),
    ),
    (
        "scripts/recognition/product_worker.py",
        include_str!("../../../scripts/recognition/product_worker.py"),
    ),
    (
        "resources/recognition/runtime-lock.json",
        include_str!("../../../resources/recognition/runtime-lock.json"),
    ),
    (
        "resources/recognition/request.schema.json",
        include_str!("../../../resources/recognition/request.schema.json"),
    ),
];
pub(crate) fn implementation_digest() -> String {
    let mut data = vec![];
    for (name, source) in SCRIPTS {
        data.extend_from_slice(name.as_bytes());
        data.push(0);
        data.extend_from_slice(source.as_bytes());
        data.push(0);
    }
    hash(&data)
}
fn materialize(root: &Path) -> Result<PathBuf, ApiError> {
    let dir = root.join("implementations").join(implementation_digest());
    for (name, source) in SCRIPTS {
        let p = dir.join(name);
        fs::create_dir_all(p.parent().unwrap()).map_err(io_error)?;
        immutable(&p, source.as_bytes())?;
    }
    Ok(dir)
}
fn media(format: &str) -> &'static str {
    match format {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpeg" => "image/jpeg",
        _ => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    }
}
fn job_languages(options: &Value) -> Vec<String> {
    if options["operation"] == "inspect" {
        vec![]
    } else {
        options["languages"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_else(|| vec!["eng".into()])
    }
}
fn runtime_fingerprint(root: &Path, languages: &[String]) -> Result<String, ApiError> {
    let manifest = read_json(&root.join("installation.json"), 256 * 1024)?;
    let lock: Value = serde_json::from_str(include_str!(
        "../../../resources/recognition/runtime-lock.json"
    ))
    .map_err(|_| unavailable())?;
    if manifest["version"] != 1
        || manifest["runtimeLockSha256"]
            != hash(include_bytes!(
                "../../../resources/recognition/runtime-lock.json"
            ))
    {
        return Err(unavailable());
    }
    let mut proofs = vec![];
    let mut check = |path: PathBuf, expected: &Value| -> Result<(), ApiError> {
        let mut file = fs::File::open(&path).map_err(|_| unavailable())?;
        if file.metadata().map_err(|_| unavailable())?.len() > 64 * 1024 * 1024 {
            return Err(unavailable());
        }
        let mut h = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer).map_err(|_| unavailable())?;
            if n == 0 {
                break;
            }
            h.update(&buffer[..n]);
        }
        let value = format!("{:x}", h.finalize());
        if expected.as_str() != Some(&value) {
            return Err(unavailable());
        }
        proofs.push(value);
        Ok(())
    };
    for p in lock["packages"].as_array().ok_or_else(unavailable)? {
        for (name, entry) in p["runtimeFiles"].as_object().ok_or_else(unavailable)? {
            check(
                root.join("venv/Lib/site-packages").join(name),
                &entry["sha256"],
            )?;
        }
    }
    for (name, entry) in lock["tesseract"]["files"]
        .as_object()
        .ok_or_else(unavailable)?
    {
        check(root.join("tesseract").join(name), &entry["sha256"])?;
    }
    for language in languages {
        let entry = &lock["models"]["files"][language];
        if entry.is_null() {
            return Err(unavailable());
        }
        check(
            root.join("tessdata")
                .join(format!("{language}.traineddata")),
            &entry["sha256"],
        )?;
    }
    Ok(hash(
        &serde_json::to_vec(&proofs).map_err(|_| unavailable())?,
    ))
}
fn current_runtime(claim: &Claim) -> Result<(), ApiError> {
    if let Some(expected) = &claim.runtime_digest {
        let root = std::env::var_os("WEFTEXT_RECOGNITION_RUNTIME")
            .map(PathBuf::from)
            .ok_or_else(unavailable)?;
        if runtime_fingerprint(&root, &job_languages(&claim.request.options))? != *expected {
            return Err(changed());
        }
    }
    Ok(())
}
fn check_store_budget(root: &Path) -> Result<(), ApiError> {
    fn walk(path: &Path, depth: u8, count: &mut usize, bytes: &mut u64) -> Result<(), ApiError> {
        if depth > 4 {
            return Err(changed());
        }
        let entries = match fs::read_dir(path) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(io_error(e)),
        };
        for entry in entries {
            let entry = entry.map_err(io_error)?;
            *count += 1;
            if *count > 10000 {
                return Err(ApiError::bad("识别保留资料数量已达预算"));
            }
            let m = entry.file_type().map_err(io_error)?;
            if m.is_symlink() {
                return Err(changed());
            }
            if m.is_dir() {
                if entry.file_name() == "implementations" {
                    continue;
                }
                walk(&entry.path(), depth + 1, count, bytes)?;
            } else {
                *bytes += entry.metadata().map_err(io_error)?.len();
                if *bytes > 256 * 1024 * 1024 {
                    return Err(ApiError::bad(
                        "识别保留资料已达256MiB预算；请由所有者核对保留范围。",
                    ));
                }
            }
        }
        Ok(())
    }
    walk(root, 0, &mut 0, &mut 0)
}
fn envelope(run: &Run, operation: Option<(&str, u64)>) -> Value {
    let o = &run.claim.request.options;
    let mut request = json!({"version":1,"inputPath":run.dir.join("input.bin"),"inputSha256":run.claim.input_sha256,"mediaType":media(&run.claim.request.format),"languages":o.get("languages").cloned().unwrap_or_else(||json!(["eng"]))});
    let op = operation
        .map(|x| x.0)
        .unwrap_or_else(|| o["operation"].as_str().unwrap_or("recognize"));
    request["operation"] = json!(op);
    if op != "inspect" {
        request["pages"] = operation
            .map(|(_, p)| json!([{"page":p}]))
            .unwrap_or_else(|| {
                o.get("pages")
                    .cloned()
                    .unwrap_or_else(|| json!([{"page":1}]))
            });
    }
    request["dpi"] = if op == "preview" {
        json!(96)
    } else {
        o.get("dpi").cloned().unwrap_or_else(|| json!(300))
    };
    json!({"request":request,"format":run.claim.request.format,"purpose":if op=="preview"{"note"}else{o["purpose"].as_str().unwrap_or("note")},"scheme":if op=="recognize"{o["scheme"].as_str().unwrap_or("none")}else{"none"},"schemeOptions":{"language":o["language"],"script":o["script"],"region":o["region"],"numbering":o["numbering"].as_str().unwrap_or("keep"),"selectedStructureIds":o.get("selectedStructureIds").cloned().unwrap_or_else(||json!([]))}})
}
async fn execute(run: &Run, operation: Option<(&str, u64)>) -> Result<Value, ApiError> {
    if run.cancel.load(Ordering::SeqCst) {
        return Err(changed());
    }
    if run.claim.request.options["purpose"] == "resource" && operation.is_none() {
        return Ok(
            json!({"result":{"version":1,"status":"complete","retentionOnly":true,"input":{"sha256":run.claim.input_sha256,"mediaType":media(&run.claim.request.format)},"pages":[]},"scheme":null}),
        );
    }
    static EXECUTIONS: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> =
        std::sync::OnceLock::new();
    let _permit = EXECUTIONS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(2)))
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "recognitionBusy",
                "识别及原页预览已达并行预算，请稍后重试。",
            )
        })?;
    let python = run.python.as_ref().ok_or_else(unavailable)?;
    let runtime = run.runtime.as_ref().ok_or_else(unavailable)?;
    let tag = uuid::Uuid::new_v4().to_string();
    let output = run.dir.join(format!("output-{tag}.json"));
    let diagnostics = run.dir.join(format!("diagnostics-{tag}.txt"));
    let out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(io_error)?;
    let err = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&diagnostics)
        .map_err(io_error)?;
    let mut command = tokio::process::Command::new(python);
    command
        .args(["-I"])
        .arg(
            run.implementation
                .join("scripts/recognition/product_worker.py"),
        )
        .arg("--runtime")
        .arg(runtime)
        .current_dir(runtime)
        .env_clear()
        .env(
            "SystemRoot",
            std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
        )
        .env("TEMP", &run.dir)
        .env("TMP", &run.dir)
        .env("PYTHONUTF8", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::from(out))
        .stderr(std::process::Stdio::from(err))
        .kill_on_drop(true);
    if operation.is_none() {
        command.arg("--progress").arg(run.dir.join("progress.json"));
    }
    #[cfg(windows)]
    command.creation_flags(0x08000000 | 0x00000004);
    let mut child = command.spawn().map_err(io_error)?;
    #[cfg(windows)]
    let guard = process_job::attach_resume(&child).map_err(io_error)?;
    let request = serde_json::to_vec(&envelope(run, operation)).map_err(|_| changed())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&request).await.map_err(io_error)?;
        stdin.shutdown().await.map_err(io_error)?;
    }
    let start = std::time::Instant::now();
    let status = loop {
        if run.cancel.load(Ordering::SeqCst)
            || run.dir.join("cancelled").exists()
            || start.elapsed() > Duration::from_secs(900)
            || fs::metadata(&output).map_or(0, |m| m.len()) > MAX_RESULT as u64
            || fs::metadata(&diagnostics).map_or(0, |m| m.len()) > 1024 * 1024
        {
            #[cfg(windows)]
            drop(guard);
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(changed());
        }
        if let Some(s) = child.try_wait().map_err(io_error)? {
            break s;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    if !status.success() {
        return Err(changed());
    }
    let result = read_json(&output, MAX_RESULT)?;
    if hash(&fs::read(run.dir.join("input.bin")).map_err(io_error)?) != run.claim.input_sha256 {
        return Err(changed());
    }
    Ok(result)
}
fn checkpoint(claim: &Claim, dir: &Path) -> Result<Value, ApiError> {
    let mut value = read_json(&dir.join("progress.json"), MAX_RESULT)?;
    let pages = value["result"]["pages"]
        .as_array_mut()
        .ok_or_else(changed)?;
    for selection in claim.request.options["pages"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if !pages.iter().any(|p| p["page"] == selection["page"]) {
            pages.push(json!({"page":selection["page"],"status":"failed","regions":[],"losses":["unprocessed-after-interruption"]}));
        }
    }
    value["result"]["status"] = json!("partial");
    value["result"]["workerFinalValidation"] = json!(false);
    value["result"]["losses"] = json!(["host-execution-interrupted"]);
    value["scheme"] = Value::Null;
    Ok(value)
}
pub(crate) async fn run_job(run: Run) {
    let reply = match execute(&run, None).await {
        Ok(reply) => reply,
        Err(_) => {
            if current_runtime(&run.claim).is_ok()
                && hash(&fs::read(run.dir.join("input.bin")).unwrap_or_default())
                    == run.claim.input_sha256
            {
                checkpoint(&run.claim,&run.dir).unwrap_or_else(|_|json!({"result":{"version":1,"status":"failed","error":"execution-failed","pages":run.claim.request.options["pages"].as_array().map(|p|p.iter().map(|s|json!({"page":s["page"],"status":"failed","regions":[],"losses":["unprocessed-after-interruption"]})).collect::<Vec<_>>()).unwrap_or_default()},"scheme":null}))
            } else {
                json!({"result":{"version":1,"status":"failed","error":"input-or-runtime-changed","pages":[]},"scheme":null})
            }
        }
    };
    if !run.dir.join("cancelled").exists() {
        let _ = write_new(
            &run.dir.join("result.json"),
            &serde_json::to_vec(&reply).unwrap_or_default(),
        );
    }
}
pub(crate) async fn preview_job(
    store_root: PathBuf,
    runtime: Option<PathBuf>,
    claim: Claim,
    page: u64,
) -> Result<Value, ApiError> {
    check_live(&claim)?;
    if page == 0
        || page > 100
        || claim.request.options["purpose"] == "resource"
        || claim.request.format == "docx"
    {
        return Err(ApiError::bad("当前原件没有可用页面预览"));
    }
    let store = Store {
        root: Some(store_root.clone()),
        runtime: runtime.clone(),
        ..Default::default()
    };
    let dir = store.directory(&claim.context.scope, &claim.request.job_id)?;
    let _ = original(&store, &claim)?;
    if dir.join("cancelled").exists() {
        return Err(changed());
    }
    let cache = dir.join(format!("preview-{page}.json"));
    if cache.is_file() {
        return read_json(&cache, MAX_RESULT);
    }
    check_store_budget(&store_root)?;
    let run = Run {
        claim: claim.clone(),
        dir,
        implementation: materialize(&store_root)?,
        python: runtime.as_ref().map(|p| p.join("venv/Scripts/python.exe")),
        runtime,
        cancel: Arc::new(AtomicBool::new(false)),
    };
    let reply = execute(&run, Some(("preview", page))).await?;
    let p = &reply["result"]["pages"][0]["preview"];
    if p["mediaType"] != "image/png" {
        return Err(changed());
    }
    let value = json!({"jobId":claim.request.job_id,"page":page,"mediaType":p["mediaType"],"dataBase64":p["dataBase64"],"width":p["width"],"height":p["height"]});
    immutable(&cache, &serde_json::to_vec(&value).map_err(|_| changed())?)?;
    Ok(value)
}
pub(crate) fn capabilities(store: &Store, executor: &str) -> Value {
    json!({"version":1,"available":store.runtime().is_some(),"executor":executor,"formats":["pdf","png","jpeg","docx"],"models":[{"id":"eng","languages":["eng"]},{"id":"chi_sim","languages":["chi_sim"]},{"id":"chi_tra","languages":["chi_tra"]},{"id":"chi_sim+eng","languages":["chi_sim","eng"]},{"id":"chi_tra+eng","languages":["chi_tra","eng"]}],"schemes":[{"id":"official-document-cn-mainland-hans@1","title":"中国大陆简体公文"},{"id":"invoice-cn-mainland-digital-ordinary-single-page@1","title":"中国大陆电子发票（普通发票）标准单页"}],"limits":{"inputBytes":MAX_INPUT,"pages":100,"resultBytes":MAX_RESULT,"jobSeconds":900,"pageSeconds":60,"concurrentExecutions":2,"retainedBytes":268435456},"implementationDigest":implementation_digest()})
}
pub(crate) fn qualified_capabilities(store: &Store, executor: &str) -> Value {
    let mut value = capabilities(store, executor);
    let runtime = store.runtime();
    if let Some(models) = value["models"].as_array_mut() {
        models.retain(|model| {
            let languages = model["languages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect::<Vec<_>>();
            runtime
                .as_ref()
                .is_some_and(|r| runtime_fingerprint(r, &languages).is_ok())
        });
    }
    value["available"] = json!(!value["models"].as_array().unwrap().is_empty());
    value
}
pub(crate) fn check_claim(claim: &Claim, context: &Context) -> Result<(), ApiError> {
    if claim.context.scope != context.scope
        || claim.context.resource != context.resource
        || claim.context.binding != context.binding
        || claim.context.epoch != context.epoch
        || !same_target(&claim.request.target, &context.target)
        || claim.implementation_digest != implementation_digest()
    {
        Err(changed())
    } else {
        current_runtime(claim)
    }
}
fn check_live(claim: &Claim) -> Result<(), ApiError> {
    if claim.born.saturating_add(TTL) < now() {
        Err(changed())
    } else {
        Ok(())
    }
}
pub(crate) fn evidence(store: &Store, claim: &Claim) -> Result<Value, ApiError> {
    check_live(claim)?;
    let reply = store.reply(claim)?;
    if !matches!(reply["status"].as_str(), Some("complete" | "partial"))
        || reply["result"]["workerFinalValidation"] == false
    {
        return Err(changed());
    }
    Ok(
        json!({"version":1,"inputSha256":claim.input_sha256,"options":claim.request.options,"optionsDigest":claim.options_digest,"implementationDigest":claim.implementation_digest,"resultDigest":reply["resultDigest"],"result":reply["result"],"scheme":reply["scheme"]}),
    )
}
pub(crate) fn original(store: &Store, claim: &Claim) -> Result<Vec<u8>, ApiError> {
    let b = fs::read(
        store
            .directory(&claim.context.scope, &claim.request.job_id)?
            .join("input.bin"),
    )
    .map_err(io_error)?;
    if b.len() > MAX_INPUT || hash(&b) != claim.input_sha256 {
        return Err(changed());
    }
    Ok(b)
}
pub(crate) fn schema_fields(core: &mut Core, root: &Path) -> Result<Value, ApiError> {
    let config = core
        .read_config(root, ConfigRole::Schema)
        .map_err(|_| changed())?;
    let text = config
        .bytes
        .map(String::from_utf8)
        .transpose()
        .map_err(|_| changed())?
        .unwrap_or_else(|| {
            include_str!("../../../docs/design/examples/schema.authoring.json").into()
        });
    let schema = weftext_data::Schema::from_author_json(&text).map_err(|_| changed())?;
    let _ = core.take_snapshot_witness(&config.base_id);
    Ok(json!({"fields":weftext_export::recognition::mapping_fields(&schema)}))
}
pub(crate) fn fingerprint(
    claim: &Claim,
    review: &RecognitionReview,
    operation: &str,
) -> Result<String, ApiError> {
    uuid(operation)?;
    check_live(claim)?;
    if claim.request.options["operation"] == "inspect"
        || claim.request.options["purpose"] != review.purpose
    {
        return Err(changed());
    }
    Ok(hash(&serde_json::to_vec(&json!({"jobId":claim.request.job_id,"inputSha256":claim.input_sha256,"optionsDigest":claim.options_digest,"implementationDigest":claim.implementation_digest,"targetBinding":claim.context.binding,"review":review})).map_err(|_|changed())?))
}
pub(crate) fn intent(
    context: &Context,
    claim: &Claim,
    bytes: Vec<u8>,
    evidence: Value,
    review: RecognitionReview,
    operation: &str,
) -> OperationRequest {
    OperationRequest {
        operation_id: operation.into(),
        root: context.root.to_string_lossy().into_owned(),
        intent: OperationIntent::ImportRecognized {
            parent: context.parent.to_string_lossy().into_owned(),
            name: context.name.clone(),
            format: claim.request.format.clone(),
            bytes,
            evidence,
            review,
            allow_losses: true,
        },
    }
}
pub(crate) fn receipt(status: ActorFeatureStatus, current: bool) -> Value {
    let allowed = current && status.status == "prepared";
    json!({"operationId":status.operation_id,"status":status.status,"planId":status.plan_id.filter(|_|allowed),"commitAllowed":allowed,"preview":if current{status.preview}else{json!({})},"nodeRefs":if current{status.node_refs}else{vec![]},"resourceRefs":if current{status.resource_refs}else{vec![]}})
}
fn local_context(state: &AppState, target: &Target) -> Result<Context, ApiError> {
    if target.slot.is_some() {
        return Err(ApiError::bad("本地导入不能选择托管槽位"));
    }
    let loc = location(state, target.session_id.as_deref().ok_or_else(changed)?)?;
    let parent = target
        .parent
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| loc.root.clone());
    let parent = std::path::absolute(parent).map_err(io_error)?;
    if !parent.starts_with(&loc.root) {
        return Err(ApiError::bad("导入目的须在当前库内"));
    }
    let name = target.name.clone().unwrap_or_else(|| "Imported".into());
    let binding = hash(
        &serde_json::to_vec(&json!({"root":loc.root,"parent":parent,"name":name}))
            .map_err(|_| changed())?,
    );
    Ok(Context {
        scope: format!(
            "local-recognition:{}",
            hash(loc.root.to_string_lossy().as_bytes())
        ),
        resource: "recognition".into(),
        root: loc.root,
        parent,
        name,
        epoch: 0,
        binding,
        target: target.clone(),
    })
}
fn lock_store(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, export_api::ExportStore>, ApiError> {
    state.exports.lock().map_err(|_| changed())
}
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/recognition/capabilities", get(local_capabilities))
        .route("/api/recognition/fields", post(local_fields))
        .route("/api/recognition/start", post(local_start))
        .route("/api/recognition/status", post(local_status))
        .route("/api/recognition/cancel", post(local_cancel))
        .route("/api/recognition/preview", post(local_preview))
        .route("/api/recognition/download", post(local_download))
        .route("/api/recognition/prepare", post(local_prepare))
        .route("/api/recognition/commit", post(local_commit))
        .route("/api/recognition/operation", post(local_operation))
        .route("/api/recognition/open", post(local_open_result))
        .route("/api/recognition/settle", post(local_settle))
        .route("/api/recognition/cancel-import", post(local_settle))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}
async fn local_capabilities(State(state): State<AppState>) -> ApiResult {
    Ok(Json(qualified_capabilities(
        &lock_store(&state)?.recognition,
        "local",
    )))
}
async fn local_fields(State(state): State<AppState>, Json(req): Json<Fields>) -> ApiResult {
    let ctx = local_context(&state, &req.target)?;
    let v = core_call(&state, move |core| {
        schema_fields(core, &ctx.root)
            .map_err(|_| CoreError::new("recognitionFields", "识别字段不可用"))
    })
    .await?;
    Ok(Json(v))
}
async fn local_start(State(state): State<AppState>, Json(req): Json<Start>) -> ApiResult {
    let ctx = local_context(&state, &req.target)?;
    let (claim, run) = { lock_store(&state)?.recognition.begin(ctx, req)? };
    let reply = lock_store(&state)?.recognition.reply(&claim)?;
    if let Some(run) = run {
        let finished = claim.clone();
        tokio::spawn(async move {
            run_job(run).await;
            if let Ok(mut store) = state.exports.lock() {
                store.recognition.finish(&finished);
            }
        });
    }
    Ok(Json(reply))
}
fn local_claim(state: &AppState, id: &str) -> Result<(Claim, Context), ApiError> {
    // Scope is recovered only from this process's already-open library roots;
    // arbitrary private-job identifiers are never filesystem locators.
    let locations = state
        .locations
        .lock()
        .map_err(|_| changed())?
        .iter()
        .map(|(id, loc)| (id.clone(), loc.clone()))
        .collect::<Vec<_>>();
    let store = lock_store(state)?;
    for (session_id, loc) in locations {
        let scope = format!(
            "local-recognition:{}",
            hash(loc.root.to_string_lossy().as_bytes())
        );
        if let Ok(claim) = store.recognition.claim(&scope, id) {
            let mut target = claim.request.target.clone();
            target.session_id = Some(session_id);
            let ctx = local_context(state, &target)?;
            check_claim(&claim, &ctx)?;
            return Ok((claim, ctx));
        }
    }
    Err(changed())
}
async fn local_status(State(state): State<AppState>, Json(req): Json<JobRequest>) -> ApiResult {
    let (claim, _) = local_claim(&state, &req.job_id)?;
    Ok(Json(lock_store(&state)?.recognition.reply(&claim)?))
}
async fn local_cancel(State(state): State<AppState>, Json(req): Json<JobRequest>) -> ApiResult {
    let (claim, ctx) = local_claim(&state, &req.job_id)?;
    let mut store = lock_store(&state)?;
    store.recognition.cancel(&ctx.scope, &req.job_id)?;
    Ok(Json(store.recognition.reply(&claim)?))
}
async fn local_preview(State(state): State<AppState>, Json(req): Json<Preview>) -> ApiResult {
    let (claim, _) = local_claim(&state, &req.job_id)?;
    let (root, runtime) = {
        let store = lock_store(&state)?;
        (
            store.recognition.root.clone().ok_or_else(unavailable)?,
            store.recognition.runtime(),
        )
    };
    let value = preview_job(root, runtime, claim.clone(), req.page).await?;
    let _ = local_claim(&state, &req.job_id)?;
    Ok(Json(value))
}
pub(crate) fn download(store: &Store, claim: &Claim) -> Result<Value, ApiError> {
    if claim.request.options["purpose"] == "resource"
        || claim.request.options["operation"] == "inspect"
    {
        return Err(ApiError::bad("本次作业未执行可交付文字提取。"));
    }
    let e = evidence(store, claim)?;
    let mut text = String::new();
    for p in e["result"]["pages"].as_array().into_iter().flatten() {
        for r in p["regions"].as_array().into_iter().flatten() {
            for b in r["blocks"].as_array().into_iter().flatten() {
                if let Some(s) = b["text"].as_str() {
                    text.push_str(s);
                    text.push('\n');
                }
            }
        }
    }
    for b in e["result"]["docxBlocks"].as_array().into_iter().flatten() {
        if let Some(s) = b["text"].as_str() {
            text.push_str(s);
            text.push('\n');
        }
    }
    let pages=e["result"]["pages"].as_array().map(|pages|pages.iter().map(|p|json!({"page":p["page"],"status":p["status"],"losses":p["losses"],"regions":p["regions"].as_array().map(|regions|regions.iter().map(|r|json!({"id":r["id"],"rect":r["rect"],"method":r["method"],"status":r["status"],"losses":r["losses"]})).collect::<Vec<_>>()).unwrap_or_default()})).collect::<Vec<_>>()).unwrap_or_default();
    let provenance = json!({"version":1,"authority":"readOnlyRecognitionProvenance","text":"rawRecognizedTextBeforeAuthorReview","textSha256":hash(text.as_bytes()),"inputSha256":claim.input_sha256,"options":claim.request.options,"optionsDigest":claim.options_digest,"implementationDigest":claim.implementation_digest,"runtimeDigest":claim.runtime_digest,"resultDigest":e["resultDigest"],"sourceScope":"selectedPagesAndRegions","status":e["result"]["status"],"pageCount":e["result"]["pageCount"],"selection":e["result"]["selection"],"pages":pages,"runtime":e["result"]["runtime"],"losses":e["result"]["losses"],"docxIssues":e["result"]["issues"],"structureRecovery":"basicRuledRectangularTablesOnly"});
    let provenance = serde_json::to_vec_pretty(&provenance).map_err(|_| changed())?;
    Ok(
        json!({"jobId":claim.request.job_id,"fileName":"recognized.txt","mediaType":"text/plain","sha256":hash(text.as_bytes()),"dataBase64":base64::engine::general_purpose::STANDARD.encode(text.as_bytes()),"provenance":{"fileName":"recognized-provenance.json","mediaType":"application/json","sha256":hash(&provenance),"dataBase64":base64::engine::general_purpose::STANDARD.encode(provenance)}}),
    )
}
async fn local_download(State(state): State<AppState>, Json(req): Json<JobRequest>) -> ApiResult {
    let (claim, _) = local_claim(&state, &req.job_id)?;
    Ok(Json(download(&lock_store(&state)?.recognition, &claim)?))
}
async fn local_prepare(State(state): State<AppState>, Json(req): Json<Prepare>) -> ApiResult {
    let ctx = local_context(&state, &req.target)?;
    let (claim, evidence, bytes) = {
        let store = lock_store(&state)?;
        let c = store.recognition.claim(&ctx.scope, &req.job_id)?;
        check_claim(&c, &ctx)?;
        let e = evidence(&store.recognition, &c)?;
        let b = original(&store.recognition, &c)?;
        (c, e, b)
    };
    let fp = fingerprint(&claim, &req.review, &req.operation_id)?;
    let request = intent(
        &ctx,
        &claim,
        bytes,
        evidence,
        req.review.clone(),
        &req.operation_id,
    );
    let preview = json!({"summary":"保留准确原件及只读来源证据，创建可编辑原生笔记；映射字段写入已安装 Schema。","_recognition":decision(&claim,&ctx),"fields":req.review.fields});
    let v = core_call(&state, move |core| {
        core.with_actor_for_root(&ctx.scope, &ctx.root, |c| {
            c.actor_feature_prepare(&ctx.resource, &req.operation_id, &fp, request, preview, &[])
        })
    })
    .await?;
    Ok(Json(receipt(v, true)))
}
async fn local_operation(State(state): State<AppState>, Json(req): Json<Operation>) -> ApiResult {
    local_action(state, req, None, false).await
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenResult {
    target: Target,
    operation_id: String,
    node: weftext_knowledge::NodeRef,
}
async fn local_open_result(
    State(state): State<AppState>,
    Json(req): Json<OpenResult>,
) -> ApiResult {
    uuid(&req.operation_id)?;
    let context = local_context(&state, &req.target)?;
    let document = core_call(&state, move |core| {
        core.with_actor_for_root(&context.scope, &context.root, |core| {
            core.actor_feature_open_original_node(&context.resource, &req.operation_id, &req.node)
        })
    })
    .await?;
    let document = value(document)?;
    remember(&state, &document)?;
    Ok(Json(document))
}
async fn local_settle(State(state): State<AppState>, Json(req): Json<Operation>) -> ApiResult {
    local_action(state, req, None, true).await
}
async fn local_commit(State(state): State<AppState>, Json(req): Json<Commit>) -> ApiResult {
    local_action(
        state,
        Operation {
            target: req.target,
            operation_id: req.operation_id,
        },
        Some((req.plan_id, req.accept_losses)),
        false,
    )
    .await
}
async fn local_action(
    state: AppState,
    req: Operation,
    commit: Option<(String, bool)>,
    settle: bool,
) -> ApiResult {
    uuid(&req.operation_id)?;
    let ctx = local_context(&state, &req.target)?;
    let v=core_call(&state,move|core|core.with_actor_for_root(&ctx.scope,&ctx.root,|c|{
        let old=c.actor_feature_status(&ctx.resource,&req.operation_id)?;
        if let Some((plan,accept))=commit {let s=old.ok_or_else(||CoreError::new("recognitionOperation","原导入计划不可用"))?;if s.status=="prepared"&&!decision_current(&s.preview,&ctx){return Err(CoreError::new("recognitionChanged","原识别设置或计划失效"));}let s=c.actor_feature_commit_with_losses(&ctx.resource,&req.operation_id,&plan,accept)?;let same=decision_current(&s.preview,&ctx);return Ok(receipt(s,same));}
        let s=if settle{Some(c.actor_feature_settle(&ctx.resource,&req.operation_id)?)}else{old};Ok(s.map(|s|{let same=decision_current(&s.preview,&ctx);receipt(s,same)}).unwrap_or_else(||json!({"operationId":req.operation_id,"status":"notRegistered","commitAllowed":false,"preview":{},"nodeRefs":[]})))
    })).await?;
    Ok(Json(v))
}
pub(crate) fn decision(claim: &Claim, ctx: &Context) -> Value {
    json!({"epoch":ctx.epoch,"binding":ctx.binding,"expires":claim.born.saturating_add(TTL),"implementationDigest":implementation_digest(),"runtimeDigest":claim.runtime_digest,"languages":job_languages(&claim.request.options)})
}
pub(crate) fn decision_current(preview: &Value, ctx: &Context) -> bool {
    let d = &preview["_recognition"];
    let runtime_current = d["runtimeDigest"].is_null()
        || std::env::var_os("WEFTEXT_RECOGNITION_RUNTIME")
            .map(PathBuf::from)
            .is_some_and(|root| {
                let languages = d["languages"]
                    .as_array()
                    .map(|v| {
                        v.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                runtime_fingerprint(&root, &languages).is_ok_and(|s| d["runtimeDigest"] == s)
            });
    d["epoch"] == ctx.epoch
        && d["binding"] == ctx.binding
        && d["implementationDigest"] == implementation_digest()
        && d["expires"].as_u64().is_some_and(|e| e > now())
        && runtime_current
}

#[cfg(windows)]
mod process_job {
    use std::io;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::*,
    };
    pub(super) struct Guard(HANDLE);
    unsafe impl Send for Guard {}
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtResumeProcess(process: HANDLE) -> i32;
    }
    pub(super) fn attach_resume(child: &tokio::process::Child) -> io::Result<Guard> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let guard = Guard(handle);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            limits.ProcessMemoryLimit = 1024 * 1024 * 1024;
            let process = child
                .raw_handle()
                .ok_or_else(|| io::Error::other("missing process"))?
                as HANDLE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, process) == 0
                || NtResumeProcess(process) < 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(guard)
        }
    }
}
