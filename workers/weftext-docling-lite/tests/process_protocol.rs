use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

const BINARY: &str = env!("CARGO_BIN_EXE_weftext-docling-lite");

#[test]
fn process_rejects_every_command_line_argument_as_structured_json() {
    let output = Command::new(BINARY)
        .arg("--help")
        .output()
        .expect("run worker");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("fatal JSON");
    assert_eq!(
        error.get("code").and_then(Value::as_str),
        Some("arguments_forbidden")
    );
}

#[test]
fn process_rejects_malformed_request_without_free_text_output() {
    let mut child = Command::new(BINARY)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start worker");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"{}")
        .expect("write request");
    let output = child.wait_with_output().expect("worker output");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("fatal JSON");
    assert_eq!(
        error.get("errorVersion").and_then(Value::as_str),
        Some("weftext.docling-lite-error.v1")
    );
    assert_eq!(
        error.get("code").and_then(Value::as_str),
        Some("invalid_request_json")
    );
}

#[test]
fn valid_request_returns_a_typed_failed_response_when_fixed_source_is_absent() {
    let mut child = Command::new(BINARY)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start worker");
    let request = serde_json::to_vec(&fixture_request()).expect("request JSON");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&request)
        .expect("write request");
    let output = child.wait_with_output().expect("worker output");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).expect("response JSON");
    assert_eq!(
        response.get("contractVersion").and_then(Value::as_str),
        Some("weftext.import-worker-response.v1")
    );
    assert_eq!(
        response.pointer("/payload/status").and_then(Value::as_str),
        Some("failed")
    );
    assert_eq!(
        response
            .pointer("/diagnostics/0/code")
            .and_then(Value::as_str),
        Some("source_unavailable")
    );
    assert_eq!(
        response
            .get("components")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(6)
    );
}

#[allow(clippy::too_many_lines)]
fn fixture_request() -> Value {
    let digest = "a".repeat(64);
    let target = if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        "x86_64-apple-darwin"
    } else {
        "aarch64-apple-darwin"
    };
    let limits = json!({
        "maxSourceBytes": 1024,
        "maxProbeBytes": 512,
        "maxPages": 10,
        "maxContainerEntries": 10,
        "maxIrNodes": 100,
        "maxIrDepth": 10,
        "maxTextBytes": 4096,
        "maxResourceCount": 10,
        "maxResourceBytes": 4096,
        "maxTotalOutputBytes": 65536,
        "maxDiagnostics": 10,
        "maxAgentSelectedNodes": 10,
        "maxAgentOperations": 10,
        "maxAgentOutputBytes": 4096,
        "workerMemoryBytes": 536_870_912,
        "workerTimeoutMs": 60000,
        "cancellationGraceMs": 1000
    });
    let components = [
        "docling-rs",
        "pdfium",
        "onnx-runtime",
        "layout-int8",
        "pp-ocr",
        "ocr-dictionary",
    ]
    .map(|component| {
        json!({
            "component": component,
            "version": "fixture-v1",
            "sha256": digest,
            "noticeId": format!("notice-{component}")
        })
    });
    let command = json!({
        "protocolVersion": "weftext.docling-lite-worker-json.v1",
        "requestId": "request-fixture",
        "sourceDigest": digest,
        "planId": "plan-fixture",
        "inputLocator": "input/source.pdf",
        "outputLocator": "output/docling-document.json",
        "doclingReleaseTag": "v0.52.2",
        "doclingReleaseCommit": "ca9fe7a543b55a540dfa18b88f4f44591b5a928e",
        "documentSchemaName": "DoclingDocument",
        "documentSchemaVersion": "1.10.0",
        "target": target,
        "localOcrPolicy": "automatic",
        "ocrLanguage": "en",
        "layoutPrecision": "int8",
        "noTableFormer": true,
        "network": "denied",
        "pageLimit": 10,
        "memoryLimitBytes": 536_870_912,
        "outputByteLimit": 65536,
        "modelPins": components
    });
    json!({
        "contractVersion": "weftext.import-worker-request.v1",
        "requestId": "request-fixture",
        "workerId": "weftext.docling-lite-worker",
        "workerProtocolVersion": "weftext.docling-lite-worker-json.v1",
        "source": {
            "contractVersion": "weftext.import.source-artifact.v1",
            "sourceId": "source-fixture",
            "displayName": "fixture.pdf",
            "origin": "test_fixture",
            "byteLength": 12,
            "sha256": digest,
            "extensionHint": "pdf",
            "detectedFormat": "pdf",
            "mismatchEvidence": []
        },
        "sourceLocator": "input/source.pdf",
        "plan": {
            "contractVersion": "weftext.import.plan.v1",
            "planId": "plan-fixture",
            "proposedRootId": "550e8400-e29b-41d4-a716-446655440000",
            "sourceDigest": digest,
            "probeDigest": "b".repeat(64),
            "route": {
                "adapter": {
                    "adapterId": "weftext.pdf-docling-lite-adapter",
                    "adapterVersion": "0.52.2-lock-fixture",
                    "supportedFormat": "pdf"
                },
                "workerId": "weftext.docling-lite-worker",
                "workerProtocolVersion": "weftext.docling-lite-worker-json.v1"
            },
            "destination": "Imported",
            "splitPolicy": "single_node",
            "resourcePolicy": "extract_referenced",
            "localOcrPolicy": "automatic",
            "agentEnhancement": { "mode": "disabled" },
            "limits": limits,
            "egress": { "mode": "none" }
        },
        "network": "denied",
        "memoryLimitBytes": 536_870_912,
        "pageLimit": 10,
        "entryLimit": 10,
        "outputByteLimit": 65536,
        "formatOptions": command
    })
}
