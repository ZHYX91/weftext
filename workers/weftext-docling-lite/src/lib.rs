use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use docling::{DocumentConverter, InputFormat, SourceDocument};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

pub const WORKER_PROTOCOL_VERSION: &str = "weftext.docling-lite-worker-json.v1";
pub const WORKER_REQUEST_VERSION: &str = "weftext.import-worker-request.v1";
pub const WORKER_RESPONSE_VERSION: &str = "weftext.import-worker-response.v1";
pub const WORKER_ID: &str = "weftext.docling-lite-worker";
pub const DOCLING_RELEASE_TAG: &str = "v0.52.2";
pub const DOCLING_RELEASE_COMMIT: &str = "ca9fe7a543b55a540dfa18b88f4f44591b5a928e";
pub const DOCLING_DOCUMENT_SCHEMA_NAME: &str = "DoclingDocument";
pub const DOCLING_DOCUMENT_SCHEMA_VERSION: &str = "1.10.0";

const SOURCE_ARTIFACT_VERSION: &str = "weftext.import.source-artifact.v1";
const IMPORT_PLAN_VERSION: &str = "weftext.import.plan.v1";
const ADAPTER_ID: &str = "weftext.pdf-docling-lite-adapter";
const INPUT_LOCATOR: &str = "input/source.pdf";
const OUTPUT_LOCATOR: &str = "output/docling-document.json";
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;
const MIN_RESPONSE_BYTES: u64 = 4096;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 1024;
const REQUIRED_COMPONENTS: [&str; 6] = [
    "docling-rs",
    "pdfium",
    "onnx-runtime",
    "layout-int8",
    "pp-ocr",
    "ocr-dictionary",
];

/// Runs the closed worker process protocol over standard streams.
#[must_use]
pub fn run_process() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let Ok(current_directory) = std::env::current_dir() else {
        return emit_fatal_to_stderr(
            "session_unavailable",
            "the worker session directory is unavailable",
        );
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    run_with_streams(
        &arguments,
        stdin.lock(),
        stdout.lock(),
        stderr.lock(),
        &current_directory,
    )
}

fn run_with_streams(
    arguments: &[OsString],
    mut input: impl Read,
    mut output: impl Write,
    mut errors: impl Write,
    session_root: &Path,
) -> ExitCode {
    if !arguments.is_empty() {
        return emit_fatal(
            &mut errors,
            "arguments_forbidden",
            "the worker accepts no command-line arguments",
        );
    }

    let request_bytes = match read_bounded(&mut input, MAX_REQUEST_BYTES) {
        Ok(bytes) => bytes,
        Err(failure) => return emit_protocol_failure(&mut errors, &failure),
    };
    let request = match parse_request(&request_bytes) {
        Ok(request) => request,
        Err(failure) => return emit_protocol_failure(&mut errors, &failure),
    };

    let response = match panic::catch_unwind(AssertUnwindSafe(|| {
        convert_valid_request(&request, session_root)
    })) {
        Ok(Ok(document)) => WorkerOutput::Completed(document),
        Ok(Err(failure)) => WorkerOutput::Failed(failure),
        Err(_) => WorkerOutput::Failed(WorkerFailure::new(
            "docling_worker_panicked",
            "the pinned Docling conversion failed without producing partial output",
        )),
    };

    let response_bytes = match bounded_response_bytes(&response, &request) {
        Ok(bytes) => bytes,
        Err(failure) => {
            return emit_protocol_failure(&mut errors, &failure);
        }
    };
    if output
        .write_all(&response_bytes)
        .and_then(|()| output.flush())
        .is_err()
    {
        return emit_fatal(
            &mut errors,
            "response_write_failed",
            "the worker response stream could not be completed",
        );
    }
    ExitCode::SUCCESS
}

fn parse_request(bytes: &[u8]) -> Result<WorkerRequest, ProtocolFailure> {
    let request: WorkerRequest = serde_json::from_slice(bytes).map_err(|_| {
        ProtocolFailure::new(
            "invalid_request_json",
            "standard input must contain one closed Weftext worker request JSON object",
        )
    })?;
    request.validate()?;
    Ok(request)
}

fn convert_valid_request(
    request: &WorkerRequest,
    session_root: &Path,
) -> Result<Value, WorkerFailure> {
    let source_bytes = read_source(request, session_root)?;
    let asset_root = find_asset_root()?;
    verify_runtime_assets(request, &asset_root)?;
    configure_fixed_runtime(&asset_root);
    validate_pdf_page_count(&source_bytes, request.page_limit)?;

    let source = SourceDocument::from_bytes("source.pdf", InputFormat::Pdf, source_bytes);
    let converter = DocumentConverter::with_allowed_formats([InputFormat::Pdf])
        .no_table_former(true)
        .ocr_lang("en")
        .force_full_page_ocr(request.plan.local_ocr_policy == LocalOcrPolicy::Always);
    let result = converter.convert(source).map_err(|_| {
        WorkerFailure::new(
            "docling_conversion_failed",
            "the pinned local PDF conversion failed without producing partial output",
        )
    })?;
    let document: Value =
        serde_json::from_str(&result.document.export_to_json()).map_err(|_| {
            WorkerFailure::new(
                "docling_document_serialization_failed",
                "Docling did not produce a valid JSON document",
            )
        })?;
    validate_docling_document(&document)?;
    Ok(document)
}

fn read_source(request: &WorkerRequest, session_root: &Path) -> Result<Vec<u8>, WorkerFailure> {
    let canonical_root = session_root.canonicalize().map_err(|_| {
        WorkerFailure::new(
            "session_unavailable",
            "the fixed worker session directory is unavailable",
        )
    })?;
    let input_path = session_root.join("input").join("source.pdf");
    let metadata = fs::symlink_metadata(&input_path).map_err(|_| {
        WorkerFailure::new("source_unavailable", "the fixed PDF input is unavailable")
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(WorkerFailure::new(
            "source_not_regular",
            "the fixed PDF input must be a regular non-link file",
        ));
    }
    let canonical_input = input_path.canonicalize().map_err(|_| {
        WorkerFailure::new(
            "source_unavailable",
            "the fixed PDF input cannot be resolved",
        )
    })?;
    let expected_parent = canonical_root.join("input");
    if canonical_input.parent() != Some(expected_parent.as_path()) {
        return Err(WorkerFailure::new(
            "source_outside_session",
            "the fixed PDF input resolves outside the worker session",
        ));
    }
    if metadata.len() != request.source.byte_length
        || metadata.len() > request.plan.limits.max_source_bytes
    {
        return Err(WorkerFailure::new(
            "source_length_mismatch",
            "the fixed PDF input length differs from the reviewed request",
        ));
    }

    let mut file = open_source_exclusive(&input_path).map_err(|_| {
        WorkerFailure::new(
            "source_open_failed",
            "the fixed PDF input could not be opened safely",
        )
    })?;
    let bytes = read_bounded(&mut file, request.plan.limits.max_source_bytes).map_err(|_| {
        WorkerFailure::new(
            "source_read_failed",
            "the fixed PDF input could not be read within its reviewed limit",
        )
    })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != metadata.len()
        || sha256_bytes(&bytes) != request.source.sha256
        || !bytes.starts_with(b"%PDF-")
    {
        return Err(WorkerFailure::new(
            "source_evidence_mismatch",
            "the fixed PDF bytes differ from the reviewed source evidence",
        ));
    }
    Ok(bytes)
}

fn open_source_exclusive(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        options.share_mode(0);
    }
    options.open(path)
}

fn validate_pdf_page_count(bytes: &[u8], page_limit: u32) -> Result<(), WorkerFailure> {
    let page_count = docling_pdf::pdfium_backend::page_count(bytes, None).map_err(|_| {
        WorkerFailure::new(
            "pdf_structure_invalid",
            "PDFium could not validate the PDF before local conversion",
        )
    })?;
    if page_count == 0 || page_count > page_limit as usize {
        return Err(WorkerFailure::new(
            "pdf_page_limit_exceeded",
            "the PDF page count is empty or exceeds the reviewed request limit",
        ));
    }
    Ok(())
}

fn find_asset_root() -> Result<PathBuf, WorkerFailure> {
    let executable = std::env::current_exe()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| {
            WorkerFailure::new(
                "worker_installation_unavailable",
                "the fixed worker installation cannot be resolved",
            )
        })?;
    let executable_directory = executable.parent().ok_or_else(|| {
        WorkerFailure::new(
            "worker_installation_unavailable",
            "the fixed worker installation has no asset root",
        )
    })?;
    let mut candidates = vec![executable_directory.to_path_buf()];
    if let Some(parent) = executable_directory.parent() {
        candidates.push(parent.to_path_buf());
    }
    let matches = candidates
        .into_iter()
        .filter(|candidate| {
            required_asset_paths(candidate)
                .iter()
                .all(|path| path.exists())
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(WorkerFailure::new(
            "worker_assets_unavailable",
            "one unambiguous fixed Docling Lite asset root is required",
        ));
    }
    matches.into_iter().next().ok_or_else(|| {
        WorkerFailure::new(
            "worker_assets_unavailable",
            "the fixed Docling Lite assets are unavailable",
        )
    })
}

fn required_asset_paths(root: &Path) -> [PathBuf; 5] {
    [
        root.join("models").join("layout_heron_int8.onnx"),
        root.join("models").join("ocr_rec_en.onnx"),
        root.join("models").join("en_dict.txt"),
        root.join(".pdfium").join("lib").join(pdfium_library_name()),
        root.join(onnx_runtime_library_name()),
    ]
}

fn verify_runtime_assets(request: &WorkerRequest, root: &Path) -> Result<(), WorkerFailure> {
    // The native runtime is a separately installed DLL, not an alias for the
    // Rust worker binary or the `ort` crate version.
    let pins = request
        .format_options
        .model_pins
        .iter()
        .map(|pin| (pin.component.as_str(), pin))
        .collect::<BTreeMap<_, _>>();
    let executable = std::env::current_exe().map_err(|_| {
        WorkerFailure::new(
            "worker_installation_unavailable",
            "the fixed worker binary cannot be inspected",
        )
    })?;
    let fixed_assets = [
        ("docling-rs", executable),
        ("onnx-runtime", required_asset_paths(root)[4].clone()),
        ("pdfium", required_asset_paths(root)[3].clone()),
        ("layout-int8", required_asset_paths(root)[0].clone()),
        ("pp-ocr", required_asset_paths(root)[1].clone()),
        ("ocr-dictionary", required_asset_paths(root)[2].clone()),
    ];
    for (component, path) in fixed_assets {
        let pin = pins.get(component).ok_or_else(|| {
            WorkerFailure::new(
                "component_pin_missing",
                "a required Docling Lite component pin is missing",
            )
        })?;
        verify_regular_file(&path)?;
        if sha256_file(&path)? != pin.sha256 {
            return Err(WorkerFailure::new(
                "component_digest_mismatch",
                "a fixed Docling Lite component differs from its reviewed digest",
            ));
        }
    }
    Ok(())
}

fn verify_regular_file(path: &Path) -> Result<(), WorkerFailure> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        WorkerFailure::new(
            "component_unavailable",
            "a fixed Docling Lite component is unavailable",
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(WorkerFailure::new(
            "component_not_regular",
            "every fixed Docling Lite component must be a regular non-link file",
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, WorkerFailure> {
    let mut file = File::open(path).map_err(|_| {
        WorkerFailure::new(
            "component_read_failed",
            "a fixed Docling Lite component could not be read",
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| {
            WorkerFailure::new(
                "component_read_failed",
                "a fixed Docling Lite component could not be read",
            )
        })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(lower_hex(digest.finalize().as_slice()))
}

fn configure_fixed_runtime(root: &Path) {
    for variable in [
        "DOCLING_RS_EP",
        "DOCLING_LAYOUT_ONNX",
        "DOCLING_RS_FP32",
        "DOCLING_RS_PDF_THREADS",
        "DOCLING_RS_PDF_INTRA",
        "DOCLING_RS_PDF_WORKERS",
        "DOCLING_RS_PDF_LAYOUT_BATCH",
        "DOCLING_RS_PDF_PARALLEL_MIN",
        "DOCLING_RS_OCR_LANG",
        "DOCLING_OCR_REC_ONNX",
        "DOCLING_OCR_DICT",
        "DOCLING_LEGACY_LINES",
        "DOCLING_PDFIUM_WORDS",
        "DOCLING_PDFIUM_TEXT",
        "DOCLING_RS_SLOW_RESIZE",
        "DOCLING_RS_DEBUG_REGIONS",
        "DOCLING_RS_TIMING",
        "DOCLING_TABLEFORMER_ENCODER",
        "DOCLING_TABLEFORMER_DECODER",
        "DOCLING_TABLEFORMER_BBOX",
        "DOCLING_RS_TF_SIMPLE_MATCH",
        "DOCLING_RS_TF_MATCH_DUMP",
        "DOCLING_CODE_FORMULA_DIR",
        "DOCLING_RS_ENRICH_DEBUG",
        "PDFIUM_DYNAMIC_LIB_PATH",
    ] {
        std::env::remove_var(variable);
    }
    std::env::set_var("DOCLING_RS_EP", "cpu");
    std::env::set_var("DOCLING_RS_PDF_THREADS", "1");
    std::env::set_var("DOCLING_RS_PDF_INTRA", "1");
    std::env::set_var("DOCLING_RS_PDF_WORKERS", "1");
    std::env::set_var("DOCLING_RS_PDF_LAYOUT_BATCH", "1");
    std::env::set_var("DOCLING_RS_OCR_LANG", "en");
    std::env::set_var(
        "DOCLING_LAYOUT_ONNX",
        root.join("models").join("layout_heron_int8.onnx"),
    );
    std::env::set_var(
        "DOCLING_OCR_REC_ONNX",
        root.join("models").join("ocr_rec_en.onnx"),
    );
    std::env::set_var("DOCLING_OCR_DICT", root.join("models").join("en_dict.txt"));
    std::env::set_var("PDFIUM_DYNAMIC_LIB_PATH", root.join(".pdfium").join("lib"));
}

fn validate_docling_document(document: &Value) -> Result<(), WorkerFailure> {
    let object = document.as_object().ok_or_else(|| {
        WorkerFailure::new(
            "docling_schema_mismatch",
            "Docling output is not a document object",
        )
    })?;
    if object.get("schema_name").and_then(Value::as_str) != Some(DOCLING_DOCUMENT_SCHEMA_NAME)
        || object.get("version").and_then(Value::as_str) != Some(DOCLING_DOCUMENT_SCHEMA_VERSION)
    {
        return Err(WorkerFailure::new(
            "docling_schema_mismatch",
            "Docling output does not match the reviewed DoclingDocument schema",
        ));
    }
    Ok(())
}

fn bounded_response_bytes(
    response: &WorkerOutput,
    request: &WorkerRequest,
) -> Result<Vec<u8>, ProtocolFailure> {
    let bytes = match response {
        WorkerOutput::Completed(document) => serde_json::to_vec(document),
        WorkerOutput::Failed(failure) => {
            serde_json::to_vec(&WorkerResponse::failed(request, *failure))
        }
    }
    .map_err(|_| {
        ProtocolFailure::new(
            "response_serialization_failed",
            "the worker response could not be serialized",
        )
    })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) <= request.output_byte_limit {
        return Ok(bytes);
    }
    let fallback = WorkerResponse::failed(
        request,
        WorkerFailure::new(
            "worker_output_limit_exceeded",
            "the Docling output exceeded the reviewed worker response limit",
        ),
    );
    let bytes = serde_json::to_vec(&fallback).map_err(|_| {
        ProtocolFailure::new(
            "response_serialization_failed",
            "the bounded worker failure response could not be serialized",
        )
    })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > request.output_byte_limit {
        return Err(ProtocolFailure::new(
            "response_limit_too_small",
            "the reviewed output limit cannot contain the closed worker response",
        ));
    }
    Ok(bytes)
}

fn read_bounded(reader: &mut impl Read, maximum: u64) -> Result<Vec<u8>, ProtocolFailure> {
    let bounded = maximum.checked_add(1).ok_or_else(|| {
        ProtocolFailure::new(
            "byte_limit_invalid",
            "a worker byte limit overflowed its closed representation",
        )
    })?;
    let mut bytes = Vec::new();
    reader.take(bounded).read_to_end(&mut bytes).map_err(|_| {
        ProtocolFailure::new(
            "input_read_failed",
            "the worker input stream could not be read",
        )
    })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(ProtocolFailure::new(
            "input_limit_exceeded",
            "the worker input exceeded its closed byte limit",
        ));
    }
    Ok(bytes)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    lower_hex(Sha256::digest(bytes).as_slice())
}

fn lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    value
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
const fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}

const fn pdfium_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    }
}

const fn onnx_runtime_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    }
}

const fn build_target() -> Option<&'static str> {
    if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

fn emit_fatal_to_stderr(code: &'static str, message: &'static str) -> ExitCode {
    emit_fatal(&mut std::io::stderr().lock(), code, message)
}

fn emit_protocol_failure(errors: &mut impl Write, failure: &ProtocolFailure) -> ExitCode {
    emit_fatal(errors, failure.code, failure.message)
}

fn emit_fatal(errors: &mut impl Write, code: &'static str, message: &'static str) -> ExitCode {
    let error = FatalError {
        error_version: "weftext.docling-lite-error.v1",
        code,
        message,
    };
    let _ = serde_json::to_writer(&mut *errors, &error);
    let _ = errors.write_all(b"\n");
    let _ = errors.flush();
    ExitCode::from(2)
}

#[derive(Clone, Copy, Debug)]
struct ProtocolFailure {
    code: &'static str,
    message: &'static str,
}

impl ProtocolFailure {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

#[derive(Clone, Copy, Debug)]
struct WorkerFailure {
    code: &'static str,
    message: &'static str,
}

enum WorkerOutput {
    Completed(Value),
    Failed(WorkerFailure),
}

impl WorkerFailure {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FatalError {
    error_version: &'static str,
    code: &'static str,
    message: &'static str,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum WorkerNetworkPolicy {
    Denied,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalOcrPolicy {
    Automatic,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SourceFormat {
    Unknown,
    FakeFixture,
    Pdf,
    Image,
    Html,
    Docx,
    Odt,
    Markdown,
    Tex,
    Csv,
    Xlsx,
    Ods,
    Epub,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OriginClass {
    LocalFile,
    Clipboard,
    Download,
    ServerUpload,
    AgentProvided,
    TestFixture,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SourceArtifact {
    contract_version: String,
    source_id: String,
    display_name: String,
    origin: OriginClass,
    byte_length: u64,
    sha256: String,
    extension_hint: Option<String>,
    detected_format: SourceFormat,
    mismatch_evidence: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AdapterDescriptor {
    adapter_id: String,
    adapter_version: String,
    supported_format: SourceFormat,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AdapterRoute {
    adapter: AdapterDescriptor,
    worker_id: String,
    worker_protocol_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ImportLimits {
    max_source_bytes: u64,
    max_probe_bytes: u64,
    max_pages: u32,
    max_container_entries: u32,
    max_ir_nodes: u32,
    max_ir_depth: u16,
    max_text_bytes: u64,
    max_resource_count: u32,
    max_resource_bytes: u64,
    max_total_output_bytes: u64,
    max_diagnostics: u32,
    max_agent_selected_nodes: u32,
    max_agent_operations: u32,
    max_agent_output_bytes: u64,
    worker_memory_bytes: u64,
    worker_timeout_ms: u64,
    cancellation_grace_ms: u64,
}

impl ImportLimits {
    fn validate(&self) -> Result<(), ProtocolFailure> {
        let nonzero = [
            self.max_source_bytes,
            self.max_probe_bytes,
            u64::from(self.max_pages),
            u64::from(self.max_container_entries),
            u64::from(self.max_ir_nodes),
            u64::from(self.max_ir_depth),
            self.max_text_bytes,
            u64::from(self.max_resource_count),
            self.max_resource_bytes,
            self.max_total_output_bytes,
            u64::from(self.max_diagnostics),
            u64::from(self.max_agent_selected_nodes),
            u64::from(self.max_agent_operations),
            self.max_agent_output_bytes,
            self.worker_memory_bytes,
            self.worker_timeout_ms,
            self.cancellation_grace_ms,
        ];
        if nonzero.contains(&0)
            || self.max_probe_bytes > self.max_source_bytes
            || self.max_resource_bytes > self.max_total_output_bytes
            || self.max_agent_output_bytes > self.max_total_output_bytes
            || self.max_total_output_bytes < MIN_RESPONSE_BYTES
        {
            return Err(ProtocolFailure::new(
                "invalid_import_limits",
                "the worker request contains invalid or contradictory resource limits",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ImportPlan {
    contract_version: String,
    plan_id: String,
    proposed_root_id: String,
    source_digest: String,
    probe_digest: String,
    route: AdapterRoute,
    destination: String,
    split_policy: Value,
    resource_policy: Value,
    local_ocr_policy: LocalOcrPolicy,
    agent_enhancement: Value,
    limits: ImportLimits,
    egress: Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DoclingModelPin {
    component: String,
    version: String,
    sha256: String,
    notice_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DoclingLiteWorkerCommand {
    protocol_version: String,
    request_id: String,
    source_digest: String,
    plan_id: String,
    input_locator: String,
    output_locator: String,
    docling_release_tag: String,
    docling_release_commit: String,
    document_schema_name: String,
    document_schema_version: String,
    target: String,
    local_ocr_policy: LocalOcrPolicy,
    ocr_language: String,
    layout_precision: String,
    no_table_former: bool,
    network: WorkerNetworkPolicy,
    page_limit: u32,
    memory_limit_bytes: u64,
    output_byte_limit: u64,
    model_pins: Vec<DoclingModelPin>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct WorkerRequest {
    contract_version: String,
    request_id: String,
    worker_id: String,
    worker_protocol_version: String,
    source: SourceArtifact,
    source_locator: String,
    plan: ImportPlan,
    network: WorkerNetworkPolicy,
    memory_limit_bytes: u64,
    page_limit: u32,
    entry_limit: u32,
    output_byte_limit: u64,
    format_options: DoclingLiteWorkerCommand,
}

impl WorkerRequest {
    fn validate(&self) -> Result<(), ProtocolFailure> {
        self.plan.limits.validate()?;
        let command = &self.format_options;
        if self.contract_version != WORKER_REQUEST_VERSION
            || self.worker_id != WORKER_ID
            || self.worker_protocol_version != WORKER_PROTOCOL_VERSION
            || !valid_identifier(&self.request_id)
            || self.source.contract_version != SOURCE_ARTIFACT_VERSION
            || !valid_identifier(&self.source.source_id)
            || !valid_source_name(&self.source.display_name)
            || self.source.detected_format != SourceFormat::Pdf
            || self.source.byte_length == 0
            || self.source.byte_length > self.plan.limits.max_source_bytes
            || !valid_digest(&self.source.sha256)
            || self.source.mismatch_evidence.len() > 32
            || self
                .source
                .mismatch_evidence
                .iter()
                .any(|evidence| evidence.len() > 1024)
            || self.plan.contract_version != IMPORT_PLAN_VERSION
            || !valid_identifier(&self.plan.plan_id)
            || !valid_identifier(&self.plan.proposed_root_id)
            || !valid_digest(&self.plan.source_digest)
            || !valid_digest(&self.plan.probe_digest)
            || self.plan.source_digest != self.source.sha256
            || self.plan.route.adapter.adapter_id != ADAPTER_ID
            || !valid_identifier(&self.plan.route.adapter.adapter_version)
            || self.plan.route.adapter.supported_format != SourceFormat::Pdf
            || self.plan.route.worker_id != WORKER_ID
            || self.plan.route.worker_protocol_version != WORKER_PROTOCOL_VERSION
            || !valid_portable_path(&self.plan.destination)
            || self.plan.split_policy != json!("single_node")
            || self.plan.resource_policy != json!("extract_referenced")
            || self.plan.agent_enhancement != json!({"mode": "disabled"})
            || self.plan.egress != json!({"mode": "none"})
            || self.source_locator != INPUT_LOCATOR
            || self.memory_limit_bytes != self.plan.limits.worker_memory_bytes
            || self.page_limit != self.plan.limits.max_pages
            || self.entry_limit != self.plan.limits.max_container_entries
            || self.output_byte_limit != self.plan.limits.max_total_output_bytes
            || command.protocol_version != WORKER_PROTOCOL_VERSION
            || command.request_id != self.request_id
            || command.source_digest != self.source.sha256
            || command.plan_id != self.plan.plan_id
            || command.input_locator != INPUT_LOCATOR
            || command.input_locator != self.source_locator
            || command.output_locator != OUTPUT_LOCATOR
            || command.docling_release_tag != DOCLING_RELEASE_TAG
            || command.docling_release_commit != DOCLING_RELEASE_COMMIT
            || command.document_schema_name != DOCLING_DOCUMENT_SCHEMA_NAME
            || command.document_schema_version != DOCLING_DOCUMENT_SCHEMA_VERSION
            || build_target() != Some(command.target.as_str())
            || command.local_ocr_policy != self.plan.local_ocr_policy
            || command.local_ocr_policy == LocalOcrPolicy::Never
            || command.ocr_language != "en"
            || command.layout_precision != "int8"
            || !command.no_table_former
            || command.network != WorkerNetworkPolicy::Denied
            || command.page_limit != self.page_limit
            || command.memory_limit_bytes != self.memory_limit_bytes
            || command.output_byte_limit != self.output_byte_limit
        {
            return Err(ProtocolFailure::new(
                "worker_request_mismatch",
                "the worker request differs from the reviewed Docling Lite PDF profile",
            ));
        }
        self.validate_pins()
    }

    fn validate_pins(&self) -> Result<(), ProtocolFailure> {
        if self.format_options.model_pins.len() < REQUIRED_COMPONENTS.len() {
            return Err(ProtocolFailure::new(
                "component_pin_missing",
                "the worker request is missing a required Docling Lite component pin",
            ));
        }
        if self.format_options.model_pins.len() > REQUIRED_COMPONENTS.len() {
            return Err(ProtocolFailure::new(
                "component_pins_invalid",
                "the worker component pin inventory must contain exactly six reviewed entries",
            ));
        }
        let mut components = BTreeSet::new();
        for pin in &self.format_options.model_pins {
            if !valid_identifier(&pin.component)
                || !valid_identifier(&pin.version)
                || !valid_digest(&pin.sha256)
                || !valid_identifier(&pin.notice_id)
                || !components.insert(pin.component.as_str())
            {
                return Err(ProtocolFailure::new(
                    "component_pins_invalid",
                    "the worker component pins must be unique and use bounded canonical values",
                ));
            }
        }
        if REQUIRED_COMPONENTS
            .iter()
            .any(|component| !components.contains(component))
        {
            return Err(ProtocolFailure::new(
                "component_pin_missing",
                "the worker request is missing a required Docling Lite component pin",
            ));
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentVersion<'a> {
    component_id: &'a str,
    version: &'a str,
    artifact_digest: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum DiagnosticSeverity {
    Blocking,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportDiagnostic<'a> {
    code: &'a str,
    severity: DiagnosticSeverity,
    message: &'a str,
    source_location: Option<Value>,
    ir_node_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkerResponse<'a> {
    contract_version: &'static str,
    request_id: &'a str,
    worker_id: &'static str,
    worker_protocol_version: &'static str,
    source_digest: &'a str,
    payload: Value,
    resources: Vec<Value>,
    diagnostics: Vec<ImportDiagnostic<'a>>,
    components: Vec<ComponentVersion<'a>>,
}

impl<'a> WorkerResponse<'a> {
    fn failed(request: &'a WorkerRequest, failure: WorkerFailure) -> Self {
        let message = if failure.message.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES {
            "the pinned local PDF conversion failed"
        } else {
            failure.message
        };
        Self {
            contract_version: WORKER_RESPONSE_VERSION,
            request_id: &request.request_id,
            worker_id: WORKER_ID,
            worker_protocol_version: WORKER_PROTOCOL_VERSION,
            source_digest: &request.source.sha256,
            payload: json!({
                "status": "failed",
                "doclingDocumentJson": null
            }),
            resources: Vec::new(),
            diagnostics: vec![ImportDiagnostic {
                code: failure.code,
                severity: DiagnosticSeverity::Blocking,
                message,
                source_location: None,
                ir_node_id: None,
            }],
            components: response_components(request),
        }
    }
}

fn response_components(request: &WorkerRequest) -> Vec<ComponentVersion<'_>> {
    request
        .format_options
        .model_pins
        .iter()
        .map(|pin| ComponentVersion {
            component_id: &pin.component,
            version: &pin.version,
            artifact_digest: Some(&pin.sha256),
        })
        .collect()
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
}

fn valid_source_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.contains(['/', '\\', '\0'])
        && !matches!(value, "." | "..")
        && !value.chars().any(char::is_control)
}

fn valid_portable_path(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 512
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains(['\\', '\0'])
        || has_windows_drive_prefix(value)
    {
        return false;
    }
    let mut component_count = 0_usize;
    for component in value.split('/') {
        component_count += 1;
        if component.is_empty()
            || matches!(component, "." | "..")
            || component.len() > 120
            || component.ends_with(' ')
            || component.ends_with('.')
            || component.chars().any(char::is_control)
            || component.contains([':', '*', '?', '"', '<', '>', '|'])
            || is_windows_device_name(component)
        {
            return false;
        }
    }
    component_count <= 32
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_windows_device_name(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit()
            && upper.as_bytes()[3] != b'0')
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_closed_reviewed_request() {
        let request = fixture_request(LocalOcrPolicy::Automatic);
        request.validate().expect("valid request");
    }

    #[test]
    fn accepts_force_full_page_ocr_but_rejects_no_ocr() {
        fixture_request(LocalOcrPolicy::Always)
            .validate()
            .expect("always OCR");
        let failure = fixture_request(LocalOcrPolicy::Never)
            .validate()
            .expect_err("no-OCR final route");
        assert_eq!(failure.code, "worker_request_mismatch");
    }

    #[test]
    fn rejects_unknown_fields_and_arbitrary_input_locators() {
        let request = fixture_request(LocalOcrPolicy::Automatic);
        let mut value = serde_json::to_value(request).expect("request JSON");
        value
            .as_object_mut()
            .expect("object")
            .insert("futureAuthority".to_owned(), Value::Bool(true));
        let bytes = serde_json::to_vec(&value).expect("JSON bytes");
        assert_eq!(
            parse_request(&bytes).expect_err("unknown field").code,
            "invalid_request_json"
        );

        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.source_locator = "../workspace/secret.pdf".to_owned();
        assert_eq!(
            request.validate().expect_err("unsafe locator").code,
            "worker_request_mismatch"
        );

        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.plan.destination = "../workspace".to_owned();
        assert_eq!(
            request.validate().expect_err("unsafe destination").code,
            "worker_request_mismatch"
        );

        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.plan.split_policy = json!({"top_level_sections": {"maximum_nodes": 2}});
        assert_eq!(
            request.validate().expect_err("non-Lite split policy").code,
            "worker_request_mismatch"
        );
    }

    #[test]
    fn rejects_pin_drift_duplicates_and_missing_components() {
        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.format_options.model_pins[0].sha256 = "A".repeat(64);
        assert_eq!(
            request.validate().expect_err("uppercase digest").code,
            "component_pins_invalid"
        );

        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.format_options.model_pins.pop();
        assert_eq!(
            request.validate().expect_err("missing component").code,
            "component_pin_missing"
        );

        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.format_options.model_pins[1].component = "docling-rs".to_owned();
        assert_eq!(
            request.validate().expect_err("duplicate component").code,
            "component_pins_invalid"
        );

        let mut request = fixture_request(LocalOcrPolicy::Automatic);
        request.format_options.model_pins.push(DoclingModelPin {
            component: "unexpected".to_owned(),
            version: "fixture-v1".to_owned(),
            sha256: "a".repeat(64),
            notice_id: "notice-unexpected".to_owned(),
        });
        assert_eq!(
            request.validate().expect_err("extra component").code,
            "component_pins_invalid"
        );
    }

    #[test]
    fn failed_response_is_structured_and_echoes_exact_component_pins() {
        let request = fixture_request(LocalOcrPolicy::Automatic);
        let response = WorkerResponse::failed(
            &request,
            WorkerFailure::new("fixture_failure", "fixture failed"),
        );
        let value = serde_json::to_value(response).expect("response JSON");
        assert_eq!(
            value.pointer("/payload/status").and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            value.pointer("/diagnostics/0/code").and_then(Value::as_str),
            Some("fixture_failure")
        );
        assert_eq!(
            value
                .pointer("/components/0/artifactDigest")
                .and_then(Value::as_str),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }

    #[test]
    fn completed_response_is_the_raw_docling_document() {
        let request = fixture_request(LocalOcrPolicy::Automatic);
        let document = json!({
            "schema_name": DOCLING_DOCUMENT_SCHEMA_NAME,
            "version": DOCLING_DOCUMENT_SCHEMA_VERSION,
            "name": "fixture"
        });
        let bytes = bounded_response_bytes(&WorkerOutput::Completed(document.clone()), &request)
            .expect("bounded response");
        let value: Value = serde_json::from_slice(&bytes).expect("response JSON");
        assert_eq!(value, document);
        assert!(value.get("contractVersion").is_none());
        assert!(value.get("payload").is_none());
    }

    #[test]
    fn docling_schema_gate_is_exact() {
        validate_docling_document(&json!({
            "schema_name": DOCLING_DOCUMENT_SCHEMA_NAME,
            "version": DOCLING_DOCUMENT_SCHEMA_VERSION
        }))
        .expect("reviewed schema");
        assert_eq!(
            validate_docling_document(&json!({
                "schema_name": DOCLING_DOCUMENT_SCHEMA_NAME,
                "version": "1.11.0"
            }))
            .expect_err("schema drift")
            .code,
            "docling_schema_mismatch"
        );
    }

    #[test]
    fn bounded_reader_detects_request_overrun() {
        let mut input = &b"12345"[..];
        assert_eq!(
            read_bounded(&mut input, 4).expect_err("overrun").code,
            "input_limit_exceeded"
        );
    }

    #[allow(clippy::too_many_lines)]
    fn fixture_request(local_ocr_policy: LocalOcrPolicy) -> WorkerRequest {
        let digest = "a".repeat(64);
        let limits = ImportLimits {
            max_source_bytes: 1024,
            max_probe_bytes: 512,
            max_pages: 10,
            max_container_entries: 10,
            max_ir_nodes: 100,
            max_ir_depth: 10,
            max_text_bytes: 4096,
            max_resource_count: 10,
            max_resource_bytes: 4096,
            max_total_output_bytes: 64 * 1024,
            max_diagnostics: 10,
            max_agent_selected_nodes: 10,
            max_agent_operations: 10,
            max_agent_output_bytes: 4096,
            worker_memory_bytes: 512 * 1024 * 1024,
            worker_timeout_ms: 60_000,
            cancellation_grace_ms: 1000,
        };
        let request_id = "request-fixture".to_owned();
        let plan_id = "plan-fixture".to_owned();
        let pins = REQUIRED_COMPONENTS
            .iter()
            .map(|component| DoclingModelPin {
                component: (*component).to_owned(),
                version: "fixture-v1".to_owned(),
                sha256: digest.clone(),
                notice_id: format!("notice-{component}"),
            })
            .collect();
        WorkerRequest {
            contract_version: WORKER_REQUEST_VERSION.to_owned(),
            request_id: request_id.clone(),
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
            source: SourceArtifact {
                contract_version: SOURCE_ARTIFACT_VERSION.to_owned(),
                source_id: "source-fixture".to_owned(),
                display_name: "fixture.pdf".to_owned(),
                origin: OriginClass::TestFixture,
                byte_length: 12,
                sha256: digest.clone(),
                extension_hint: Some("pdf".to_owned()),
                detected_format: SourceFormat::Pdf,
                mismatch_evidence: Vec::new(),
            },
            source_locator: INPUT_LOCATOR.to_owned(),
            plan: ImportPlan {
                contract_version: IMPORT_PLAN_VERSION.to_owned(),
                plan_id: plan_id.clone(),
                proposed_root_id: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
                source_digest: digest.clone(),
                probe_digest: "b".repeat(64),
                route: AdapterRoute {
                    adapter: AdapterDescriptor {
                        adapter_id: ADAPTER_ID.to_owned(),
                        adapter_version: "0.52.2-lock-fixture".to_owned(),
                        supported_format: SourceFormat::Pdf,
                    },
                    worker_id: WORKER_ID.to_owned(),
                    worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
                },
                destination: "Imported".to_owned(),
                split_policy: Value::String("single_node".to_owned()),
                resource_policy: Value::String("extract_referenced".to_owned()),
                local_ocr_policy,
                agent_enhancement: json!({"mode": "disabled"}),
                limits,
                egress: json!({"mode": "none"}),
            },
            network: WorkerNetworkPolicy::Denied,
            memory_limit_bytes: 512 * 1024 * 1024,
            page_limit: 10,
            entry_limit: 10,
            output_byte_limit: 64 * 1024,
            format_options: DoclingLiteWorkerCommand {
                protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
                request_id,
                source_digest: digest,
                plan_id,
                input_locator: INPUT_LOCATOR.to_owned(),
                output_locator: OUTPUT_LOCATOR.to_owned(),
                docling_release_tag: DOCLING_RELEASE_TAG.to_owned(),
                docling_release_commit: DOCLING_RELEASE_COMMIT.to_owned(),
                document_schema_name: DOCLING_DOCUMENT_SCHEMA_NAME.to_owned(),
                document_schema_version: DOCLING_DOCUMENT_SCHEMA_VERSION.to_owned(),
                target: build_target()
                    .unwrap_or("unsupported-test-target")
                    .to_owned(),
                local_ocr_policy,
                ocr_language: "en".to_owned(),
                layout_precision: "int8".to_owned(),
                no_table_former: true,
                network: WorkerNetworkPolicy::Denied,
                page_limit: 10,
                memory_limit_bytes: 512 * 1024 * 1024,
                output_byte_limit: 64 * 1024,
                model_pins: pins,
            },
        }
    }
}
