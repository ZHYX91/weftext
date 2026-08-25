use std::fs;

use tempfile::TempDir;
use weftext_core::{
    DocumentEdit, commit_document_edit, create_workspace, plan_document_edit, read_node_document,
    read_workspace_revision,
};
use weftext_export::{
    ExportDiagnosticSeverity, ExportErrorCode, MarkdownMetadataPolicy, commit_markdown_export,
    preview_markdown_export, read_markdown_export_bundle, write_markdown_export_bundle,
};

struct Fixture {
    _temporary: TempDir,
    workspace: std::path::PathBuf,
    destination: std::path::PathBuf,
    bundle: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary export fixture");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("canonical workspace");
        let snapshot = read_node_document(&workspace).expect("root snapshot");
        let source = format!(
            concat!(
                "---\n",
                "weftext:\n",
                "  id: \"{}\"\n",
                "  icon: \"😀\"\n",
                "  aliases:\n",
                "    - \"文缕\"\n",
                "  adjacent_heading_body: separate\n",
                "---\n",
                "= 导出标题: 副标题\n",
                "Author Name <author@example.test>\n",
                ":status: draft\n",
                ":lang: zh-CN\n",
                "\n",
                "== 第一节\n",
                "正文 *粗体* 与 link:https://example.test[站点]，以及 node:{}[本节点]。\n",
                "\n",
                "======== 深层 H7\n",
                "> > 嵌套引用\n",
                "\n",
                "* [x] 完成\n",
                "* [ ] 待办\n",
                "\n",
                "[cols=\"2*\",options=\"header\"]\n",
                "|===\n",
                "|名称\n",
                "|值\n",
                "\n",
                "|中文\n",
                "|✅\n",
                "|===\n",
                "\n",
                "[source,rust]\n",
                "----\n",
                "fn main() {{}}\n",
                "----\n",
                "\n",
                "image::diagram.png[架构图]\n"
            ),
            snapshot.node_id, snapshot.node_id
        );
        let plan = plan_document_edit(
            &workspace,
            &snapshot.revision,
            [DocumentEdit {
                start: 0,
                end: u64::try_from(snapshot.source.len()).expect("source length"),
                replacement: source,
            }],
        )
        .expect("source edit plan");
        commit_document_edit(&plan).expect("source edit commit");
        let destination = temporary.path().join("exported.md");
        let bundle = temporary.path().join("export-plan.json");
        Self {
            _temporary: temporary,
            workspace,
            destination,
            bundle,
        }
    }
}

#[test]
fn preview_freezes_a_report_and_commit_publishes_exact_external_bytes_only() {
    let fixture = Fixture::new();
    let before_revision = read_workspace_revision(&fixture.workspace).expect("workspace revision");
    let before_source = read_node_document(&fixture.workspace).expect("source snapshot");

    let plan = preview_markdown_export(
        &fixture.workspace,
        before_source.node_id,
        &fixture.destination,
        MarkdownMetadataPolicy::PreserveWeftext,
    )
    .expect("Markdown export preview");
    assert!(!fixture.destination.exists());
    assert_eq!(
        read_workspace_revision(&fixture.workspace).expect("preview workspace revision"),
        before_revision
    );
    assert!(plan.artifact.starts_with("---\nweftext:\n"));
    assert!(plan.artifact.contains("# 导出标题"));
    assert!(plan.artifact.contains("####### 深层 H7"));
    assert!(plan.artifact.contains("**粗体**"));
    assert!(plan.artifact.contains("[站点](https://example.test)"));
    assert!(plan.artifact.contains("weftext://node/"));
    assert!(plan.artifact.contains("```asciidoc"));
    assert!(plan.artifact.contains("```rust"));
    assert!(
        plan.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "markdown_extended_heading")
    );
    assert!(
        plan.diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "external_image_reference_not_copied" })
    );

    write_markdown_export_bundle(&fixture.workspace, &fixture.bundle, &plan)
        .expect("immutable export bundle");
    let reloaded = read_markdown_export_bundle(&fixture.bundle).expect("validated export bundle");
    assert_eq!(reloaded, plan);
    let bundle_bytes = fs::read(&fixture.bundle).expect("bundle bytes");

    let receipt =
        commit_markdown_export(&fixture.workspace, &reloaded, "2026-08-24T12:00:00+08:00")
            .expect("export commit");
    assert_eq!(receipt.status, "committed");
    assert_eq!(
        fs::read_to_string(&fixture.destination).unwrap(),
        plan.artifact
    );
    assert_eq!(fs::read(&fixture.bundle).unwrap(), bundle_bytes);
    assert_eq!(
        read_workspace_revision(&fixture.workspace).expect("commit workspace revision"),
        before_revision
    );
    assert_eq!(
        read_node_document(&fixture.workspace)
            .expect("unchanged managed source")
            .source,
        before_source.source
    );
    assert_eq!(
        commit_markdown_export(&fixture.workspace, &reloaded, "2026-08-24T12:00:01+08:00")
            .expect_err("create-new destination cannot be replayed")
            .code(),
        ExportErrorCode::DestinationExists
    );
}

#[test]
fn plain_option_removes_only_weftext_envelope_and_stale_preview_cannot_write() {
    let fixture = Fixture::new();
    let snapshot = read_node_document(&fixture.workspace).expect("source snapshot");
    let plan = preview_markdown_export(
        &fixture.workspace,
        snapshot.node_id,
        &fixture.destination,
        MarkdownMetadataPolicy::RemoveWeftext,
    )
    .expect("plain Markdown preview");
    assert!(!plan.artifact.starts_with("---\nweftext:"));
    assert!(plan.artifact.contains("# 导出标题"));
    assert!(plan.artifact.contains(":status: draft"));
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "weftext_metadata_removed"
            && diagnostic.severity == ExportDiagnosticSeverity::Omission
    }));

    let edit = plan_document_edit(
        &fixture.workspace,
        &snapshot.revision,
        [DocumentEdit {
            start: u64::try_from(snapshot.source.len()).unwrap(),
            end: u64::try_from(snapshot.source.len()).unwrap(),
            replacement: "\nChanged after preview.\n".to_owned(),
        }],
    )
    .expect("concurrent edit plan");
    commit_document_edit(&edit).expect("concurrent edit");
    let error = commit_markdown_export(&fixture.workspace, &plan, "2026-08-24T12:00:00+08:00")
        .expect_err("stale export");
    assert_eq!(error.code(), ExportErrorCode::StalePlan);
    assert!(!fixture.destination.exists());
}

#[test]
fn export_rejects_workspace_destinations_and_keeps_disabled_effects_inert() {
    let fixture = Fixture::new();
    let snapshot = read_node_document(&fixture.workspace).expect("source snapshot");
    let inside = fixture.workspace.join("inside.md");
    assert_eq!(
        preview_markdown_export(
            &fixture.workspace,
            snapshot.node_id,
            &inside,
            MarkdownMetadataPolicy::PreserveWeftext,
        )
        .expect_err("in-workspace export must fail")
        .code(),
        ExportErrorCode::UnsafeDestination
    );

    let active_source = format!(
        "{}\ninclude::https://example.test/secret[]\n",
        snapshot.source
    );
    let edit = plan_document_edit(
        &fixture.workspace,
        &snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: u64::try_from(snapshot.source.len()).unwrap(),
            replacement: active_source,
        }],
    )
    .expect("active-source edit plan");
    commit_document_edit(&edit).expect("active-source commit");
    let current = read_node_document(&fixture.workspace).expect("active source");
    let preview = preview_markdown_export(
        &fixture.workspace,
        current.node_id,
        &fixture.destination,
        MarkdownMetadataPolicy::PreserveWeftext,
    )
    .expect("disabled effect is exported inertly");
    assert!(preview.artifact.contains("```asciidoc"));
    assert!(
        preview
            .artifact
            .contains("include::https://example.test/secret[]")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "unsupported_asciidoc_preserved_literal" })
    );
    assert!(!fixture.destination.exists());
}
