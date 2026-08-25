use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;
use weftext_core::{create_child_node, create_workspace, read_workspace_revision};
use weftext_import::{AgentImportPatch, AgentPatchOperation, ImportNodeKind, sha256_bytes};
use weftext_intake::{read_agent_enhancement_preview, read_preview_bundle};

struct Fixture {
    temporary: TempDir,
    workspace: PathBuf,
    source: PathBuf,
    bundle: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary fixture root");
        let workspace = temporary.path().join("验收空间");
        create_workspace(&workspace).expect("canonical workspace");
        let source = temporary.path().join("输入.fake");
        std::fs::write(
            &source,
            concat!(
                "WEFTEXT-FAKE/1\n",
                "导入标题 😀\n",
                "第一段中文。\n",
                "مرحبا بالعالم\n"
            ),
        )
        .expect("fake source");
        let bundle = temporary.path().join("preview-bundle.json");
        Self {
            temporary,
            workspace,
            source,
            bundle,
        }
    }

    fn preview(&self) -> Output {
        preview_command(&self.workspace, &self.source, "导入节点", &self.bundle)
    }

    fn commit(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_weftext"))
            .args(["import", "commit"])
            .arg(&self.workspace)
            .arg(&self.bundle)
            .output()
            .expect("run fake import commit")
    }
}

#[test]
fn fake_preview_is_workspace_read_only_and_exact_bundle_commits_with_bound_receipt() {
    let fixture = Fixture::new();
    let base_revision = read_workspace_revision(&fixture.workspace).expect("base revision");

    let preview = fixture.preview();
    assert_success(&preview);
    assert_eq!(
        read_workspace_revision(&fixture.workspace).expect("revision after preview"),
        base_revision
    );
    assert!(!fixture.workspace.join("导入节点").exists());
    assert!(fixture.bundle.is_file());

    let bundle_bytes = std::fs::read(&fixture.bundle).expect("exact bundle bytes");
    let bundle: serde_json::Value =
        serde_json::from_slice(&bundle_bytes).expect("preview bundle JSON");
    assert_eq!(
        bundle["contractVersion"],
        "weftext.intake-preview-bundle.v1"
    );
    assert_eq!(bundle["baseWorkspaceRevision"], base_revision.as_str());
    assert_eq!(
        bundle["previewReceipt"]["commitResult"]["status"],
        "preview_only"
    );
    assert!(bundle.get("workspaceRoot").is_none());
    assert!(bundle.get("workspaceHandle").is_none());

    let preview_stdout = output_json(&preview);
    assert_eq!(preview_stdout["import"]["stage"], "preview");
    assert_eq!(preview_stdout["import"]["adapter"], "fake");
    assert_eq!(preview_stdout["import"]["bundle"], bundle);

    let commit = fixture.commit();
    assert_success(&commit);
    assert_eq!(
        std::fs::read(&fixture.bundle).expect("bundle remains immutable"),
        bundle_bytes
    );
    let committed = output_json(&commit);
    assert_eq!(committed["import"]["stage"], "committed");
    assert_eq!(committed["import"]["adapter"], "fake");
    assert_eq!(
        committed["import"]["receipt"]["commitResult"]["status"],
        "committed"
    );
    assert_eq!(
        committed["import"]["proposalDigest"],
        bundle["proposalDigest"]
    );
    assert_eq!(
        committed["import"]["receipt"]["proposalDigest"],
        bundle["proposalDigest"]
    );
    assert_eq!(
        committed["import"]["transaction"]["importAuthority"]["proposalId"],
        bundle["proposal"]["proposalId"]
    );
    assert_eq!(
        committed["import"]["transaction"]["importAuthority"]["proposalDigest"],
        bundle["proposalDigest"]
    );
    assert_eq!(
        committed["import"]["receipt"]["commitResult"]["transaction_id"],
        committed["import"]["transaction"]["planId"]
    );
    assert_eq!(
        committed["import"]["receipt"]["commitResult"]["workspace_revision"],
        committed["import"]["transaction"]["revision"]
    );

    let exact_source = bundle["proposal"]["nodes"][0]["exactAsciidoc"]
        .as_str()
        .expect("exact proposal source");
    let committed_source =
        std::fs::read_to_string(fixture.workspace.join("导入节点/导入节点.adoc"))
            .expect("committed canonical source");
    assert_eq!(committed_source, exact_source);
    assert!(committed_source.contains("导入标题 😀"));
    assert!(committed_source.contains("第一段中文。"));
    assert!(committed_source.contains("مرحبا بالعالم"));
    assert_eq!(
        read_workspace_revision(&fixture.workspace)
            .expect("committed revision")
            .as_str(),
        committed["import"]["transaction"]["revision"]
            .as_str()
            .expect("transaction revision")
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the end-to-end agent import test keeps both reviewed commits and every persisted handoff artifact in one linear scenario"
)]
fn agent_cli_exports_only_reviewed_ir_and_requires_two_explicit_commits() {
    let fixture = Fixture::new();
    let base_revision = read_workspace_revision(&fixture.workspace).expect("base revision");
    assert_success(&fixture.preview());
    let local = read_preview_bundle(&fixture.bundle).expect("local preview bundle");
    let target = local.document.nodes[0].id.clone();
    let selection_path = fixture.temporary.path().join("agent-selection.json");
    std::fs::write(
        &selection_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "provider": "reviewed-provider",
            "selectedNodeIds": [target],
            "retention": "delete-after-call",
            "redaction": "selected-ir-only"
        }))
        .expect("selection JSON"),
    )
    .expect("selection file");
    let review_path = fixture.temporary.path().join("agent-review.json");
    let prepare = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "agent-prepare"])
        .arg(&fixture.workspace)
        .arg(&fixture.bundle)
        .arg(&selection_path)
        .arg(&review_path)
        .output()
        .expect("prepare agent review");
    assert_success(&prepare);
    assert_eq!(
        read_workspace_revision(&fixture.workspace).expect("revision after review"),
        base_revision
    );
    let prepared = output_json(&prepare);
    assert_eq!(
        prepared["agentEnhancement"]["networkExecuted"],
        serde_json::json!(false)
    );
    assert!(
        prepared["agentEnhancement"]
            .get("authorizedBundle")
            .is_none()
    );
    assert!(prepared["agentEnhancement"].get("sourceBytes").is_none());
    let review = read_agent_enhancement_preview(&review_path).expect("exact agent review");

    let evidence_path = fixture.temporary.path().join("agent-evidence.json");
    let denied_export = agent_evidence_command(
        &fixture.workspace,
        &review_path,
        &evidence_path,
        "--approval-not-granted",
    );
    assert!(!denied_export.status.success());
    assert!(String::from_utf8_lossy(&denied_export.stderr).contains("--approve-exact-egress"));
    assert!(!evidence_path.exists());
    let export = agent_evidence_command(
        &fixture.workspace,
        &review_path,
        &evidence_path,
        "--approve-exact-egress",
    );
    assert_success(&export);
    assert_eq!(
        std::fs::read(&evidence_path).expect("exported evidence"),
        review.evidence.to_bytes().expect("exact evidence bytes")
    );
    assert_eq!(
        output_json(&export)["agentEnhancement"]["networkExecuted"],
        serde_json::json!(false)
    );

    let ImportNodeKind::Paragraph { text } = &review.authorized_bundle.document.nodes[0].kind
    else {
        panic!("fake adapter first node must remain a paragraph");
    };
    let replacement = "Agent reviewed typed correction";
    let patch = AgentImportPatch::create(
        review.authorized_bundle.document.revision.clone(),
        vec![review.authorized_bundle.document.nodes[0].id.clone()],
        vec![AgentPatchOperation::CorrectText {
            node_id: review.authorized_bundle.document.nodes[0].id.clone(),
            expected_text_digest: sha256_bytes(text.as_bytes()),
            replacement: replacement.to_owned(),
        }],
        review.selection.provider.clone(),
        "reviewed-model",
        review.authorized_bundle.plan.egress.clone(),
    )
    .expect("typed patch");
    let patch_path = fixture.temporary.path().join("agent-patch.json");
    std::fs::write(
        &patch_path,
        serde_json::to_vec_pretty(&patch).expect("typed patch JSON"),
    )
    .expect("typed patch file");
    let enhanced_path = fixture.temporary.path().join("agent-enhanced.json");
    let denied_apply = agent_apply_command(
        &fixture.workspace,
        &review_path,
        &patch_path,
        &enhanced_path,
        "--approval-not-granted",
    );
    assert!(!denied_apply.status.success());
    assert!(!enhanced_path.exists());
    let apply = agent_apply_command(
        &fixture.workspace,
        &review_path,
        &patch_path,
        &enhanced_path,
        "--approve-exact-egress",
    );
    assert_success(&apply);
    assert_eq!(
        read_workspace_revision(&fixture.workspace).expect("revision after typed patch"),
        base_revision
    );
    let enhanced = read_preview_bundle(&enhanced_path).expect("enhanced preview bundle");
    assert_eq!(enhanced.preview_receipt.agent_provenance.len(), 1);
    assert!(
        enhanced.proposal.nodes[0]
            .exact_asciidoc
            .contains(replacement)
    );
    assert_eq!(
        output_json(&apply)["import"]["requiresFinalCommitApproval"],
        serde_json::json!(true)
    );

    let commit = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "commit"])
        .arg(&fixture.workspace)
        .arg(&enhanced_path)
        .output()
        .expect("commit enhanced preview");
    assert_success(&commit);
    let committed = std::fs::read_to_string(fixture.workspace.join("导入节点/导入节点.adoc"))
        .expect("canonical imported AsciiDoc");
    assert!(committed.contains(replacement));
    assert!(!fixture.workspace.join("导入节点/导入节点.md").exists());
}

#[test]
fn commit_rejects_unknown_fields_and_tampered_authority_without_writing() {
    let fixture = Fixture::new();
    let preview = fixture.preview();
    assert_success(&preview);
    let exact = std::fs::read(&fixture.bundle).expect("exact preview bundle");
    let mut bundle: serde_json::Value =
        serde_json::from_slice(&exact).expect("preview bundle JSON");

    bundle
        .as_object_mut()
        .expect("bundle object")
        .insert("workspaceHandle".to_owned(), serde_json::json!("forged"));
    std::fs::write(
        &fixture.bundle,
        serde_json::to_vec_pretty(&bundle).expect("unknown-field bundle"),
    )
    .expect("write unknown-field bundle");
    let unknown = fixture.commit();
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown field"));
    assert!(!fixture.workspace.join("导入节点").exists());

    let mut bundle: serde_json::Value =
        serde_json::from_slice(&exact).expect("restore exact preview bundle");
    bundle["proposal"]["nodes"][0]["exactAsciidoc"] =
        serde_json::json!("---\nweftext:\n  id: \"forged\"\n---\n= forged\n");
    std::fs::write(
        &fixture.bundle,
        serde_json::to_vec_pretty(&bundle).expect("tampered bundle"),
    )
    .expect("write tampered bundle");
    let tampered = fixture.commit();
    assert!(!tampered.status.success());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("bundle digest"));
    assert!(!fixture.workspace.join("导入节点").exists());
}

#[test]
fn commit_rejects_a_stale_workspace_revision_without_rerunning_import() {
    let fixture = Fixture::new();
    let preview = fixture.preview();
    assert_success(&preview);
    let bundle_bytes = std::fs::read(&fixture.bundle).expect("preview bundle");

    create_child_node(&fixture.workspace, "Concurrent").expect("concurrent workspace change");
    let commit = fixture.commit();
    assert!(!commit.status.success());
    assert!(String::from_utf8_lossy(&commit.stderr).contains("stale import preview"));
    assert!(!fixture.workspace.join("导入节点").exists());
    assert_eq!(
        std::fs::read(&fixture.bundle).expect("bundle unchanged"),
        bundle_bytes
    );
}

#[test]
fn preview_refuses_to_place_its_bundle_inside_the_workspace() {
    let fixture = Fixture::new();
    let base_revision = read_workspace_revision(&fixture.workspace).expect("base revision");
    let inside = fixture.workspace.join("preview.json");

    let preview = preview_command(&fixture.workspace, &fixture.source, "导入节点", &inside);
    assert!(!preview.status.success());
    assert!(String::from_utf8_lossy(&preview.stderr).contains("outside the workspace"));
    assert!(!inside.exists());
    assert!(!fixture.workspace.join("导入节点").exists());
    assert_eq!(
        read_workspace_revision(&fixture.workspace).expect("unchanged revision"),
        base_revision
    );
}

#[test]
fn explicit_markdown_preview_is_read_only_and_commits_only_canonical_asciidoc() {
    let temporary = tempfile::tempdir().expect("Markdown fixture root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("canonical workspace");
    let source = temporary.path().join("输入.md");
    let source_bytes = concat!(
        "---\r\n",
        "status: draft\r\n",
        "---\r\n",
        "# 导入标题 😀\r\n",
        "正文 **加粗** 与 [链接](https://example.test)。\r\n\r\n",
        "> 引用\r\n",
        "```rust\r\n",
        "fn main() {}\r\n",
        "```\r\n",
    )
    .as_bytes();
    std::fs::write(&source, source_bytes).expect("Markdown source");
    let bundle_path = temporary.path().join("markdown-preview.json");
    let base_revision = read_workspace_revision(&workspace).expect("base revision");

    let preview = markdown_preview_command(&workspace, &source, "Imported", &bundle_path, true);
    assert_success(&preview);
    assert_eq!(
        read_workspace_revision(&workspace).expect("preview revision"),
        base_revision
    );
    assert!(!workspace.join("Imported").exists());

    let preview_json = output_json(&preview);
    assert_eq!(preview_json["import"]["adapter"], "markdown_compatibility");
    let bundle = &preview_json["import"]["bundle"];
    assert_eq!(
        bundle["plan"]["route"]["adapter"]["adapterId"],
        "weftext.markdown-compatibility"
    );
    assert_eq!(
        bundle["plan"]["resourcePolicy"],
        "extract_and_retain_original"
    );
    assert_eq!(
        bundle["proposal"]["nodes"][0]["resources"][0]["locator"],
        "original-输入.md.source"
    );
    let exact = bundle["proposal"]["nodes"][0]["exactAsciidoc"]
        .as_str()
        .expect("canonical proposal source");
    assert!(exact.starts_with("---\nweftext:\n  id: \""));
    assert_eq!(exact.matches("\nweftext:\n").count(), 1);
    assert!(exact.contains("= 导入标题 😀"));
    assert!(exact.contains("正文 *加粗* 与 链接 (https://example.test)"));
    assert!(exact.contains("[source,yaml]"));
    assert!(!exact.contains("\nstatus: draft\n"));

    let commit = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "commit"])
        .arg(&workspace)
        .arg(&bundle_path)
        .output()
        .expect("commit Markdown import");
    assert_success(&commit);
    assert_eq!(
        output_json(&commit)["import"]["adapter"],
        "markdown_compatibility"
    );
    assert_eq!(
        std::fs::read(workspace.join(".weftext-format")).expect("exact profile marker"),
        b"weftext.asciidoc.v1\n"
    );
    assert_eq!(
        std::fs::read_to_string(workspace.join("Imported/Imported.adoc"))
            .expect("managed AsciiDoc"),
        exact
    );
    assert!(!workspace.join("Imported/Imported.md").exists());
    assert_eq!(
        std::fs::read(workspace.join("Imported/original-输入.md.source"))
            .expect("retained non-managed original"),
        source_bytes
    );
}

#[test]
fn markdown_preview_rejects_active_content_before_creating_authority() {
    let temporary = tempfile::tempdir().expect("unsafe Markdown fixture root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("canonical workspace");
    let source = temporary.path().join("unsafe.md");
    std::fs::write(&source, "# Unsafe\n<script>alert(1)</script>\n")
        .expect("unsafe Markdown source");
    let bundle = temporary.path().join("unsafe-preview.json");
    let base_revision = read_workspace_revision(&workspace).expect("base revision");

    let preview = markdown_preview_command(&workspace, &source, "Unsafe", &bundle, false);
    assert!(!preview.status.success());
    assert!(
        String::from_utf8_lossy(&preview.stderr)
            .contains("bounded format probe did not authorize conversion planning")
    );
    assert!(!bundle.exists());
    assert!(!workspace.join("Unsafe").exists());
    assert_eq!(
        read_workspace_revision(&workspace).expect("unchanged revision"),
        base_revision
    );
}

#[test]
fn pdf_capability_and_preview_expose_the_honest_closed_gate() {
    let temporary = tempfile::tempdir().expect("PDF fixture root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");
    let installation = temporary.path().join("docling-lite");
    std::fs::create_dir(&installation).expect("installation");
    let source = temporary.path().join("input.pdf");
    std::fs::write(&source, b"%PDF-1.7\n").expect("PDF source");
    let bundle = temporary.path().join("pdf-preview.json");

    let capability = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "pdf-capability"])
        .arg(&installation)
        .output()
        .expect("PDF capability");
    assert_success(&capability);
    let capability = output_json(&capability);
    assert_eq!(capability["import"]["adapter"], "docling_lite");
    assert_eq!(capability["import"]["capability"]["available"], false);
    assert_eq!(
        capability["import"]["capability"]["ambientNetworkAllowed"],
        false
    );

    let preview = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "pdf-preview"])
        .arg(&workspace)
        .arg(&source)
        .arg("Imported")
        .arg(&bundle)
        .arg(&installation)
        .output()
        .expect("PDF preview");
    assert!(!preview.status.success());
    assert!(!bundle.exists());
    assert!(!workspace.join("Imported").exists());
}

#[test]
#[allow(clippy::too_many_lines)]
fn task_source_set_preview_commits_frozen_cross_document_mapping_and_receipt() {
    let temporary = tempfile::tempdir().expect("task import fixture root");
    let workspace = temporary.path().join("任务空间");
    create_workspace(&workspace).expect("canonical workspace");
    let root_id = weftext_core::scan_workspace(&workspace).nodes[0]
        .id
        .expect("root ID");
    let project = temporary.path().join("项目.md");
    let done = temporary.path().join("完成.md");
    std::fs::write(
        &project,
        concat!(
            "# 项目\r\n",
            "- [/] #task 编写 📅 2026-09-05 🆔 write\r\n",
            "```tasks\n",
            "not done\n",
            "```\n",
        ),
    )
    .expect("project source");
    std::fs::write(
        &done,
        "- [-] #task 取消 ❌ 2026-09-01 ⛔ write 🆔 cancelled\n",
    )
    .expect("done source");
    let request = temporary.path().join("task-request.json");
    std::fs::write(
        &request,
        serde_json::to_vec_pretty(&serde_json::json!({
            "profile": "weftext.task-import.v1",
            "destinationParentId": root_id,
            "destinationName": "导入",
            "settings": {
                "dialect": "obsidian_tasks_emoji_v1",
                "pluginVersion": "8.2.0",
                "globalFilter": "#task",
                "indentationWidth": 4,
                "statuses": [
                    {"symbol": " ", "name": "Todo", "statusType": "TODO"},
                    {"symbol": "x", "name": "Done", "statusType": "DONE"},
                    {"symbol": "/", "name": "In Progress", "statusType": "IN_PROGRESS"},
                    {"symbol": "-", "name": "Cancelled", "statusType": "CANCELLED"}
                ]
            },
            "documents": [
                {"locator": "项目.md", "sourcePath": project},
                {"locator": "项目/完成.md", "sourcePath": done}
            ]
        }))
        .expect("request JSON"),
    )
    .expect("task request");
    let bundle = temporary.path().join("task-preview.json");
    let base_revision = read_workspace_revision(&workspace).expect("base revision");
    let preview = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "task-preview"])
        .arg(&workspace)
        .arg(&request)
        .arg(&bundle)
        .output()
        .expect("task preview");
    assert_success(&preview);
    assert_eq!(
        read_workspace_revision(&workspace).expect("preview revision"),
        base_revision
    );
    assert!(!workspace.join("导入").exists());
    let preview_json = output_json(&preview);
    assert_eq!(preview_json["import"]["adapter"], "task_source_set");
    assert_eq!(preview_json["import"]["committable"], true);
    let reviewed = &preview_json["import"]["bundle"];
    assert_eq!(reviewed["contractVersion"], "weftext.task-import-bundle.v1");
    assert_eq!(
        reviewed["taskPlan"]["identities"].as_array().unwrap().len(),
        2
    );
    assert_eq!(reviewed["evidence"].as_array().unwrap().len(), 2);
    assert!(reviewed["nodes"].as_array().unwrap().iter().any(|node| {
        node["exactAsciidoc"]
            .as_str()
            .is_some_and(|source| source.contains("[.weftext-query,version=1,view=task-list]"))
    }));

    let immutable_bundle = std::fs::read(&bundle).expect("immutable task bundle");
    let receipt = temporary.path().join("task-receipt.json");
    let mut commit_command = Command::new(env!("CARGO_BIN_EXE_weftext"));
    commit_command
        .args(["import", "task-commit"])
        .arg(&workspace)
        .arg(&bundle)
        .arg(&receipt)
        .arg(
            preview_json["import"]["review"]["proposalId"]
                .as_str()
                .expect("reviewed proposal ID"),
        )
        .arg(
            preview_json["import"]["review"]["proposalDigest"]
                .as_str()
                .expect("reviewed proposal digest"),
        )
        .arg(
            preview_json["import"]["review"]["bundleDigest"]
                .as_str()
                .expect("reviewed bundle digest"),
        );
    let commit = commit_command.output().expect("task commit");
    assert_success(&commit);
    assert_eq!(
        std::fs::read(&bundle).expect("bundle remains exact"),
        immutable_bundle
    );
    assert!(receipt.is_file());
    let committed = output_json(&commit);
    assert_eq!(committed["import"]["adapter"], "task_source_set");
    assert_eq!(
        committed["import"]["proposalDigest"],
        reviewed["proposalDigest"]
    );
    assert_eq!(
        committed["import"]["receipt"]["contractVersion"],
        "weftext.task-import-receipt.v1"
    );
    assert_eq!(
        committed["import"]["receipt"]["commonReceipts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        committed["import"]["transaction"]["importAuthority"]["proposalDigest"],
        reviewed["bundleDigest"]
    );
    assert!(workspace.join("导入/项目/项目.adoc").is_file());
    assert!(workspace.join("导入/项目/完成/完成.adoc").is_file());
    assert!(!workspace.join("导入/项目/项目.md").exists());
    assert!(weftext_core::scan_workspace(&workspace).is_valid());

    let mut task_recover = Command::new(env!("CARGO_BIN_EXE_weftext"));
    task_recover
        .args(["import", "task-recover"])
        .arg(&workspace)
        .arg(&bundle)
        .arg(&receipt)
        .arg(
            preview_json["import"]["review"]["proposalId"]
                .as_str()
                .expect("reviewed proposal ID"),
        )
        .arg(
            preview_json["import"]["review"]["proposalDigest"]
                .as_str()
                .expect("reviewed proposal digest"),
        )
        .arg(
            preview_json["import"]["review"]["bundleDigest"]
                .as_str()
                .expect("reviewed bundle digest"),
        );
    let task_recover = task_recover.output().expect("idempotent task recovery");
    assert_success(&task_recover);
    assert_eq!(
        output_json(&task_recover)["import"]["recovery"]["status"],
        "already_finalized"
    );

    let recovery = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "recover"])
        .arg(&workspace)
        .output()
        .expect("task import recovery");
    assert_success(&recovery);
    assert_eq!(output_json(&recovery)["import"]["stage"], "recovered");
}

#[test]
fn task_import_cli_rejects_unknown_request_fields_before_preview() {
    let temporary = tempfile::tempdir().expect("task import fixture root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");
    let root_id = weftext_core::scan_workspace(&workspace).nodes[0]
        .id
        .expect("root ID");
    let source = temporary.path().join("input.md");
    std::fs::write(&source, "- [ ] task\n").expect("source");
    let request = temporary.path().join("bad-request.json");
    std::fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({
            "profile": "weftext.task-import.v1",
            "destinationParentId": root_id,
            "destinationName": "Imported",
            "settings": {
                "dialect": "markdown_checklist_v1",
                "pluginVersion": null,
                "globalFilter": null,
                "indentationWidth": 4,
                "statuses": [
                    {"symbol": " ", "name": "Open", "statusType": "TODO"},
                    {"symbol": "x", "name": "Closed", "statusType": "DONE"},
                    {"symbol": "X", "name": "Closed", "statusType": "DONE"}
                ]
            },
            "documents": [{"locator": "input.md", "sourcePath": source}],
            "rediscoverDirectory": true
        }))
        .expect("request"),
    )
    .expect("request file");
    let bundle = temporary.path().join("must-not-exist.json");
    let preview = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "task-preview"])
        .arg(&workspace)
        .arg(&request)
        .arg(&bundle)
        .output()
        .expect("bad task preview");
    assert!(!preview.status.success());
    assert!(String::from_utf8_lossy(&preview.stderr).contains("unknown field"));
    assert!(!bundle.exists());
    assert!(!workspace.join("Imported").exists());
}

fn preview_command(workspace: &Path, source: &Path, destination: &str, bundle: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "fake-preview"])
        .arg(workspace)
        .arg(source)
        .arg(destination)
        .arg(bundle)
        .output()
        .expect("run fake import preview")
}

fn markdown_preview_command(
    workspace: &Path,
    source: &Path,
    destination: &str,
    bundle: &Path,
    retain_original: bool,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_weftext"));
    command
        .args(["import", "markdown-preview"])
        .arg(workspace)
        .arg(source)
        .arg(destination)
        .arg(bundle);
    if retain_original {
        command.arg("--retain-original");
    }
    command.output().expect("run Markdown import preview")
}

fn agent_evidence_command(
    workspace: &Path,
    review: &Path,
    evidence: &Path,
    approval: &str,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "agent-export-evidence"])
        .arg(workspace)
        .arg(review)
        .arg(evidence)
        .arg(approval)
        .output()
        .expect("export selected agent evidence")
}

fn agent_apply_command(
    workspace: &Path,
    review: &Path,
    patch: &Path,
    bundle: &Path,
    approval: &str,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["import", "agent-apply"])
        .arg(workspace)
        .arg(review)
        .arg(patch)
        .arg(bundle)
        .arg(approval)
        .output()
        .expect("apply reviewed typed agent patch")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("CLI JSON stdout")
}
