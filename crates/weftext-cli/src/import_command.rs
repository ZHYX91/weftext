use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use weftext_core::{
    NodeId, TASK_IMPORT_PROFILE_ID, TaskImportDocumentInput, TaskImportSettings,
    recover_workspace_transactions,
};
use weftext_import::{
    CancellationToken, ImportTempRoot, OriginClass, PortablePath, Sha256Digest, SourceFormat,
};
use weftext_intake::{
    FAKE_SOURCE_BYTE_LIMIT, MARKDOWN_SOURCE_BYTE_LIMIT, PDF_SOURCE_BYTE_LIMIT, TaskImportReview,
    apply_approved_agent_patch, commit_previewed_import, commit_previewed_task_import,
    docling_lite_capability, prepare_agent_enhancement, preview_docling_pdf_import,
    preview_fake_import, preview_markdown_import, preview_task_import,
    read_agent_enhancement_preview, read_agent_evidence_selection, read_agent_import_patch,
    read_preview_bundle, read_regular_file_bounded, read_task_import_bundle,
    recover_previewed_task_import, rfc3339_utc_now, write_agent_enhancement_preview,
    write_agent_import_evidence, write_preview_bundle, write_task_import_bundle,
};

const TASK_REQUEST_BYTE_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskImportPreviewRequest {
    profile: String,
    destination_parent_id: NodeId,
    destination_name: String,
    settings: TaskImportSettings,
    documents: Vec<TaskImportSourceRequest>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskImportSourceRequest {
    locator: String,
    source_path: PathBuf,
}

pub(crate) fn run(arguments: &[String], schema: &str) -> Result<Value, String> {
    if let Some(result) = run_task_source_set_command(arguments, schema) {
        return result;
    }
    match arguments {
        [scope, command, workspace, source, destination, bundle]
            if scope == "import" && command == "fake-preview" =>
        {
            preview_fake(workspace, source, destination, bundle, schema)
        }
        [scope, command, workspace, source, destination, bundle]
            if scope == "import" && command == "markdown-preview" =>
        {
            preview_markdown(
                workspace,
                source,
                destination,
                bundle,
                false,
                schema,
            )
        }
        [scope, command, workspace, source, destination, bundle, retain_original]
            if scope == "import"
                && command == "markdown-preview"
                && retain_original == "--retain-original" =>
        {
            preview_markdown(workspace, source, destination, bundle, true, schema)
        }
        [scope, command, installation]
            if scope == "import" && command == "pdf-capability" =>
        {
            Ok(json!({
                "schema": schema,
                "ok": true,
                "import": {
                    "stage": "capability",
                    "adapter": "docling_lite",
                    "capability": docling_lite_capability(installation),
                }
            }))
        }
        [scope, command, workspace, source, destination, bundle, installation]
            if scope == "import" && command == "pdf-preview" =>
        {
            preview_pdf(
                workspace,
                source,
                destination,
                bundle,
                installation,
                schema,
            )
        }
        [scope, command, workspace, bundle] if scope == "import" && command == "commit" => {
            commit_bundle(workspace, bundle, schema)
        }
        [scope, command, workspace, bundle, selection, review]
            if scope == "import" && command == "agent-prepare" =>
        {
            prepare_agent_review(workspace, bundle, selection, review, schema)
        }
        [scope, command, workspace, review, evidence, approval]
            if scope == "import" && command == "agent-export-evidence" =>
        {
            export_agent_evidence(workspace, review, evidence, approval, schema)
        }
        [scope, command, workspace, review, patch, bundle, approval]
            if scope == "import" && command == "agent-apply" =>
        {
            apply_agent_reviewed_patch(workspace, review, patch, bundle, approval, schema)
        }
        [scope, command, workspace] if scope == "import" && command == "recover" => {
            let report = recover_workspace_transactions(workspace).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": schema,
                "ok": true,
                "import": {
                    "stage": "recovered",
                    "recovery": report,
                }
            }))
        }
        _ => Err(
            "usage: weftext import fake-preview <workspace> <source.fake> <destination> <bundle.json> | weftext import markdown-preview <workspace> <source.md> <destination> <bundle.json> [--retain-original] | weftext import pdf-capability <installation> | weftext import pdf-preview <workspace> <source.pdf> <destination> <bundle.json> <installation> | weftext import agent-prepare <workspace> <bundle.json> <selection.json> <review.json> | weftext import agent-export-evidence <workspace> <review.json> <evidence.json> --approve-exact-egress | weftext import agent-apply <workspace> <review.json> <patch.json> <bundle.json> --approve-exact-egress | weftext import commit <workspace> <bundle.json> | weftext import task-preview <workspace> <request.json> <bundle.json> | weftext import task-commit <workspace> <bundle.json> <receipt.json> <proposal-id> <proposal-digest> <bundle-digest> | weftext import task-recover <workspace> <bundle.json> <receipt.json> <proposal-id> <proposal-digest> <bundle-digest> | weftext import recover <workspace>"
                .to_owned(),
        ),
    }
}

fn run_task_source_set_command(
    arguments: &[String],
    schema: &str,
) -> Option<Result<Value, String>> {
    match arguments {
        [scope, command, workspace, request, bundle]
            if scope == "import" && command == "task-preview" =>
        {
            Some(preview_task_source_set(workspace, request, bundle, schema))
        }
        [
            scope,
            command,
            workspace,
            bundle,
            receipt,
            proposal_id,
            proposal_digest,
            bundle_digest,
        ] if scope == "import" && command == "task-commit" => Some(commit_task_source_set(
            workspace,
            bundle,
            receipt,
            proposal_id,
            proposal_digest,
            bundle_digest,
            schema,
        )),
        [
            scope,
            command,
            workspace,
            bundle,
            receipt,
            proposal_id,
            proposal_digest,
            bundle_digest,
        ] if scope == "import" && command == "task-recover" => Some(recover_task_source_set(
            workspace,
            bundle,
            receipt,
            proposal_id,
            proposal_digest,
            bundle_digest,
            schema,
        )),
        _ => None,
    }
}

fn preview_task_source_set(
    workspace: &str,
    request_path: &str,
    bundle_path: &str,
    schema: &str,
) -> Result<Value, String> {
    let request_path = Path::new(request_path);
    let request_bytes = read_regular_file_bounded(request_path, TASK_REQUEST_BYTE_LIMIT)
        .map_err(|error| error.to_string())?;
    let request_value: Value =
        serde_json::from_slice(&request_bytes).map_err(|error| error.to_string())?;
    let request: TaskImportPreviewRequest =
        serde_json::from_value(request_value.clone()).map_err(|error| error.to_string())?;
    if serde_json::to_value(&request).map_err(|error| error.to_string())? != request_value {
        return Err("task import request contains fields outside its exact contract".to_owned());
    }
    if request.profile != TASK_IMPORT_PROFILE_ID {
        return Err("task import request must pin profile weftext.task-import.v1".to_owned());
    }
    let request_parent = request_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut source_identities = BTreeSet::new();
    let mut documents = Vec::with_capacity(request.documents.len());
    for document in request.documents {
        let source_path = if document.source_path.is_absolute() {
            document.source_path
        } else {
            request_parent.join(document.source_path)
        };
        let canonical = std::fs::canonicalize(&source_path).map_err(|error| {
            format!(
                "resolve explicit task import source {}: {error}",
                source_path.display()
            )
        })?;
        if !source_identities.insert(canonical) {
            return Err(
                "task import request selects the same source file more than once".to_owned(),
            );
        }
        let source_bytes = read_regular_file_bounded(&source_path, MARKDOWN_SOURCE_BYTE_LIMIT)
            .map_err(|error| error.to_string())?;
        let source = String::from_utf8(source_bytes)
            .map_err(|_| format!("task import source is not UTF-8: {}", source_path.display()))?;
        documents.push(TaskImportDocumentInput {
            locator: document.locator,
            source,
        });
    }
    let temp_root = ImportTempRoot::initialize(std::env::temp_dir().join("weftext-intake-v1"))
        .map_err(|error| error.to_string())?;
    let cancellation = CancellationToken::default();
    let bundle = preview_task_import(
        workspace,
        &temp_root,
        request.destination_parent_id,
        request.destination_name,
        documents,
        request.settings,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
        &cancellation,
    )
    .map_err(|error| error.to_string())?;
    write_task_import_bundle(workspace, bundle_path, &bundle).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "preview",
            "adapter": "task_source_set",
            "bundlePath": bundle_path,
            "committable": bundle.task_plan.is_committable(),
            "review": TaskImportReview::from_preview(&bundle),
            "bundle": bundle,
        }
    }))
}

fn commit_task_source_set(
    workspace: &str,
    bundle_path: &str,
    receipt_path: &str,
    proposal_id: &str,
    proposal_digest: &str,
    bundle_digest: &str,
    schema: &str,
) -> Result<Value, String> {
    let bundle = read_task_import_bundle(bundle_path).map_err(|error| error.to_string())?;
    let review = TaskImportReview {
        proposal_id: proposal_id.to_owned(),
        proposal_digest: Sha256Digest::parse(proposal_digest).map_err(|error| error.to_string())?,
        bundle_digest: Sha256Digest::parse(bundle_digest).map_err(|error| error.to_string())?,
    };
    let committed = commit_previewed_task_import(
        workspace,
        &bundle,
        &review,
        receipt_path,
        bundle.preview_created_at.clone(),
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "committed",
            "adapter": "task_source_set",
            "proposalId": committed.proposal_id,
            "proposalDigest": committed.proposal_digest,
            "receiptPath": receipt_path,
            "transaction": committed.transaction,
            "receipt": committed.receipt,
        }
    }))
}

fn recover_task_source_set(
    workspace: &str,
    bundle_path: &str,
    receipt_path: &str,
    proposal_id: &str,
    proposal_digest: &str,
    bundle_digest: &str,
    schema: &str,
) -> Result<Value, String> {
    let bundle = read_task_import_bundle(bundle_path).map_err(|error| error.to_string())?;
    let review = TaskImportReview {
        proposal_id: proposal_id.to_owned(),
        proposal_digest: Sha256Digest::parse(proposal_digest).map_err(|error| error.to_string())?,
        bundle_digest: Sha256Digest::parse(bundle_digest).map_err(|error| error.to_string())?,
    };
    let recovery = recover_previewed_task_import(
        workspace,
        &bundle,
        &review,
        receipt_path,
        bundle.preview_created_at.clone(),
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "task_recovered",
            "adapter": "task_source_set",
            "receiptPath": receipt_path,
            "recovery": recovery,
        }
    }))
}

fn preview_markdown(
    workspace: &str,
    source_path: &str,
    destination: &str,
    bundle_path: &str,
    retain_original: bool,
    schema: &str,
) -> Result<Value, String> {
    let source_path = Path::new(source_path);
    let source_bytes = read_regular_file_bounded(source_path, MARKDOWN_SOURCE_BYTE_LIMIT)
        .map_err(|error| error.to_string())?;
    let display_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Markdown import source must have a UTF-8 file name".to_owned())?;
    let destination = PortablePath::parse(destination).map_err(|error| error.to_string())?;
    let temp_root = ImportTempRoot::initialize(std::env::temp_dir().join("weftext-intake-v1"))
        .map_err(|error| error.to_string())?;
    let bundle = preview_markdown_import(
        workspace,
        temp_root,
        display_name,
        OriginClass::LocalFile,
        source_bytes,
        destination,
        retain_original,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
        CancellationToken::default(),
    )
    .map_err(|error| error.to_string())?;
    write_preview_bundle(workspace, bundle_path, &bundle).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "preview",
            "adapter": "markdown_compatibility",
            "bundlePath": bundle_path,
            "bundle": bundle,
        }
    }))
}

fn preview_pdf(
    workspace: &str,
    source_path: &str,
    destination: &str,
    bundle_path: &str,
    installation: &str,
    schema: &str,
) -> Result<Value, String> {
    let source_path = Path::new(source_path);
    let source_bytes = read_regular_file_bounded(source_path, PDF_SOURCE_BYTE_LIMIT)
        .map_err(|error| error.to_string())?;
    let display_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "PDF import source must have a UTF-8 file name".to_owned())?;
    let destination = PortablePath::parse(destination).map_err(|error| error.to_string())?;
    let temp_root = ImportTempRoot::initialize(std::env::temp_dir().join("weftext-intake-v1"))
        .map_err(|error| error.to_string())?;
    let bundle = preview_docling_pdf_import(
        workspace,
        temp_root,
        installation,
        display_name,
        OriginClass::LocalFile,
        source_bytes,
        destination,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
        CancellationToken::default(),
    )
    .map_err(|error| error.to_string())?;
    write_preview_bundle(workspace, bundle_path, &bundle).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "preview",
            "adapter": "docling_lite",
            "bundlePath": bundle_path,
            "bundle": bundle,
        }
    }))
}

fn preview_fake(
    workspace: &str,
    source_path: &str,
    destination: &str,
    bundle_path: &str,
    schema: &str,
) -> Result<Value, String> {
    let source_path = Path::new(source_path);
    let source_bytes = read_regular_file_bounded(source_path, FAKE_SOURCE_BYTE_LIMIT)
        .map_err(|error| error.to_string())?;
    let display_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "fake import source must have a UTF-8 file name".to_owned())?;
    let destination = PortablePath::parse(destination).map_err(|error| error.to_string())?;
    let temp_root = ImportTempRoot::initialize(std::env::temp_dir().join("weftext-intake-v1"))
        .map_err(|error| error.to_string())?;
    let bundle = preview_fake_import(
        workspace,
        temp_root,
        display_name,
        OriginClass::LocalFile,
        source_bytes,
        destination,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    write_preview_bundle(workspace, bundle_path, &bundle).map_err(|error| error.to_string())?;

    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "preview",
            "adapter": "fake",
            "bundlePath": bundle_path,
            "bundle": bundle,
        }
    }))
}

fn prepare_agent_review(
    workspace: &str,
    bundle_path: &str,
    selection_path: &str,
    review_path: &str,
    schema: &str,
) -> Result<Value, String> {
    let bundle = read_preview_bundle(bundle_path).map_err(|error| error.to_string())?;
    let selection =
        read_agent_evidence_selection(selection_path).map_err(|error| error.to_string())?;
    let review = prepare_agent_enhancement(
        &bundle,
        selection,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let evidence_byte_length = review
        .evidence
        .to_bytes()
        .map_err(|error| error.to_string())?
        .len();
    write_agent_enhancement_preview(workspace, review_path, &review)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "agentEnhancement": {
            "stage": "review_prepared",
            "reviewPath": review_path,
            "previewDigest": review.preview_digest,
            "baseBundleDigest": review.base_bundle_digest,
            "selection": review.selection,
            "evidenceDigest": review.evidence.evidence_digest,
            "evidenceByteLength": evidence_byte_length,
            "networkExecuted": false,
            "requiresExplicitEgressApproval": true,
        }
    }))
}

fn export_agent_evidence(
    workspace: &str,
    review_path: &str,
    evidence_path: &str,
    approval: &str,
    schema: &str,
) -> Result<Value, String> {
    require_exact_egress_approval(approval)?;
    let review = read_agent_enhancement_preview(review_path).map_err(|error| error.to_string())?;
    let evidence_byte_length = review
        .evidence
        .to_bytes()
        .map_err(|error| error.to_string())?
        .len();
    write_agent_import_evidence(workspace, evidence_path, &review)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "agentEnhancement": {
            "stage": "evidence_exported",
            "reviewPath": review_path,
            "evidencePath": evidence_path,
            "previewDigest": review.preview_digest,
            "evidenceDigest": review.evidence.evidence_digest,
            "evidenceByteLength": evidence_byte_length,
            "provider": review.selection.provider,
            "networkExecuted": false,
        }
    }))
}

fn apply_agent_reviewed_patch(
    workspace: &str,
    review_path: &str,
    patch_path: &str,
    bundle_path: &str,
    approval: &str,
    schema: &str,
) -> Result<Value, String> {
    require_exact_egress_approval(approval)?;
    let review = read_agent_enhancement_preview(review_path).map_err(|error| error.to_string())?;
    let patch = read_agent_import_patch(patch_path).map_err(|error| error.to_string())?;
    let bundle = apply_approved_agent_patch(
        &review,
        &patch,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    write_preview_bundle(workspace, bundle_path, &bundle).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "agent_patch_preview",
            "bundlePath": bundle_path,
            "bundleDigest": bundle.bundle_digest,
            "proposalDigest": bundle.proposal_digest,
            "proposal": bundle.proposal,
            "receipt": bundle.preview_receipt,
            "requiresFinalCommitApproval": true,
            "networkExecuted": false,
        }
    }))
}

fn require_exact_egress_approval(approval: &str) -> Result<(), String> {
    if approval != "--approve-exact-egress" {
        return Err(
            "agent evidence export and patch application require --approve-exact-egress".to_owned(),
        );
    }
    Ok(())
}

fn commit_bundle(workspace: &str, bundle_path: &str, schema: &str) -> Result<Value, String> {
    let bundle = read_preview_bundle(bundle_path).map_err(|error| error.to_string())?;
    let adapter = match bundle.source.detected_format {
        SourceFormat::Pdf => "docling_lite",
        SourceFormat::Markdown => "markdown_compatibility",
        _ => "fake",
    };
    let committed = commit_previewed_import(
        workspace,
        &bundle,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "import": {
            "stage": "committed",
            "adapter": adapter,
            "proposalId": committed.proposal_id,
            "proposalDigest": committed.proposal_digest,
            "transaction": committed.transaction,
            "receipt": committed.receipt,
        }
    }))
}
