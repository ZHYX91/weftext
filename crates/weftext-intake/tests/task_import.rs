use weftext_core::{
    TaskImportDialect, TaskImportDocumentInput, TaskImportSettings, TaskImportStatusMapping,
    TaskImportStatusType, WorkspaceImportAuthority, WorkspaceImportNode,
    commit_workspace_transaction_retaining_journal, create_child_node, create_workspace,
    plan_import_tree, read_workspace_revision, scan_workspace,
};
use weftext_import::{CancellationToken, ImportTempRoot};
use weftext_intake::{
    IntakeErrorCode, TaskImportPreviewBundle, TaskImportRecovery, TaskImportReview,
    commit_previewed_task_import, preview_task_import, recover_previewed_task_import,
    validate_task_import_preview,
};

fn status(symbol: char, name: &str, status_type: TaskImportStatusType) -> TaskImportStatusMapping {
    TaskImportStatusMapping {
        symbol,
        name: name.to_owned(),
        status_type,
    }
}

fn obsidian_settings() -> TaskImportSettings {
    TaskImportSettings {
        dialect: TaskImportDialect::ObsidianTasksEmojiV1,
        plugin_version: Some("8.2.0".to_owned()),
        global_filter: Some("#task".to_owned()),
        indentation_width: 4,
        statuses: vec![
            status(' ', "Todo", TaskImportStatusType::Todo),
            status('x', "Done", TaskImportStatusType::Done),
            status('/', "In Progress", TaskImportStatusType::InProgress),
            status('-', "Cancelled", TaskImportStatusType::Cancelled),
        ],
    }
}

fn assert_exact_task_nodes(workspace: &std::path::Path, preview: &TaskImportPreviewBundle) {
    for node in &preview.nodes {
        let path = workspace
            .join(
                node.destination_locator
                    .replace('/', std::path::MAIN_SEPARATOR_STR),
            )
            .join(&node.document_file);
        assert_eq!(
            std::fs::read_to_string(path).expect("committed node"),
            node.exact_asciidoc
        );
    }
    assert!(scan_workspace(workspace).is_valid());
}

fn assert_legacy_receipt_wire_and_id(
    workspace: &std::path::Path,
    preview: &TaskImportPreviewBundle,
    receipt_path: &std::path::Path,
    receipt_id: &str,
) {
    let legacy_shaped_receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt_path).expect("durable task receipt bytes"))
            .expect("durable task receipt JSON");
    assert!(
        legacy_shaped_receipt["transaction"]
            .get("promotionSummary")
            .is_none(),
        "legacy non-promotion transaction wire must not gain a null promotion field"
    );
    let recovered = recover_previewed_task_import(
        workspace,
        preview,
        &TaskImportReview::from_preview(preview),
        receipt_path,
        "2026-08-24T12:01:00+08:00",
    )
    .expect("legacy-shaped durable receipt remains valid");
    let TaskImportRecovery::AlreadyFinalized {
        committed: recovered,
        ..
    } = recovered
    else {
        panic!("completed import should validate as already finalized");
    };
    assert_eq!(recovered.receipt.receipt_id, receipt_id);
}

#[test]
fn complete_source_set_commits_exact_cjk_nested_tasks_and_query_once() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let workspace = temporary.path().join("任务空间");
    create_workspace(&workspace).expect("workspace");
    let root_id = scan_workspace(&workspace).nodes[0].id.expect("root id");
    let base_revision = read_workspace_revision(&workspace).expect("base revision");
    let documents = vec![
        TaskImportDocumentInput {
            locator: "项目.md".to_owned(),
            source: concat!(
                "# 项目 😀\r\n",
                "- [ ] #task 普通任务\r\n",
                "    - [/] #task 编写文档 📅 2026-09-05 🆔 write\n",
                "```tasks\n",
                "not done\n",
                "due before tomorrow\n",
                "```\n",
            )
            .to_owned(),
        },
        TaskImportDocumentInput {
            locator: "项目/完成.md".to_owned(),
            source: "- [-] #task 已取消 ❌ 2026-09-01 ⛔ write 🆔 cancelled\n".to_owned(),
        },
    ];
    let preview = preview_task_import(
        &workspace,
        &ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("temp root"),
        root_id,
        "导入",
        documents,
        obsidian_settings(),
        "2026-08-24T12:00:00+08:00",
        &CancellationToken::default(),
    )
    .expect("task preview");
    assert_eq!(
        read_workspace_revision(&workspace).expect("preview revision"),
        base_revision
    );
    assert!(!workspace.join("导入").exists());
    assert!(preview.task_plan.is_committable());
    assert_eq!(preview.evidence.len(), 2);
    assert_eq!(preview.task_plan.identities.len(), 2);
    validate_task_import_preview(&preview).expect("frozen preview");
    let project = preview
        .nodes
        .iter()
        .find(|node| node.source_locator.as_deref() == Some("项目.md"))
        .expect("project node");
    assert!(project.exact_asciidoc.contains("* [ ] 普通任务\r\n"));
    assert!(project.exact_asciidoc.contains("** [ ] 编写文档 task:[id="));
    assert!(project.exact_asciidoc.contains("phase=in-progress"));
    assert!(
        project
            .exact_asciidoc
            .contains("[.weftext-query,version=1,view=task-list]\n....\n")
    );
    assert!(!project.exact_asciidoc.contains("WEFTEXT_TASK_PATCH_"));

    let receipt_path = temporary.path().join("task-receipt.json");
    let mut wrong_review = TaskImportReview::from_preview(&preview);
    wrong_review.proposal_id.push_str("-changed");
    let review_error = commit_previewed_task_import(
        &workspace,
        &preview,
        &wrong_review,
        &receipt_path,
        "2026-08-24T12:00:30+08:00",
    )
    .expect_err("mismatched explicit review");
    assert_eq!(review_error.code, IntakeErrorCode::InvalidBundle);
    assert!(!workspace.join("导入").exists());

    let committed = commit_previewed_task_import(
        &workspace,
        &preview,
        &TaskImportReview::from_preview(&preview),
        &receipt_path,
        "2026-08-24T12:00:00+08:00",
    )
    .expect("commit exact task preview");
    assert_eq!(committed.proposal_digest, preview.proposal_digest);
    assert_eq!(committed.receipt.common_receipts.len(), 2);
    assert!(receipt_path.is_file());
    assert_legacy_receipt_wire_and_id(
        &workspace,
        &preview,
        &receipt_path,
        &committed.receipt.receipt_id,
    );
    assert_eq!(
        committed
            .transaction
            .import_authority
            .as_ref()
            .expect("authority")
            .proposal_digest,
        preview.bundle_digest.to_string()
    );
    assert_exact_task_nodes(&workspace, &preview);
}

#[test]
fn protected_ranges_do_not_become_tasks_and_stale_preview_does_not_write() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");
    let root_id = scan_workspace(&workspace).nodes[0].id.expect("root id");
    let documents = vec![TaskImportDocumentInput {
        locator: "mixed.md".to_owned(),
        source: concat!(
            "- [ ] 中文\r\n",
            "  * [X] العربية\n",
            "```text\n",
            "- [ ] literal\n",
            "```\n",
            "<!--\n",
            "- [ ] comment\n",
            "-->\n",
            "> - [ ] quote\n",
        )
        .to_owned(),
    }];
    let preview = preview_task_import(
        &workspace,
        &ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("temp root"),
        root_id,
        "Imported",
        documents,
        TaskImportSettings::markdown_checklist_v1(2),
        "2026-08-24T00:00:00Z",
        &CancellationToken::default(),
    )
    .expect("protected preview");
    let source = &preview
        .nodes
        .iter()
        .find(|node| node.source_locator.is_some())
        .expect("source node")
        .exact_asciidoc;
    assert!(source.contains("* [ ] 中文\r\n"));
    assert!(source.contains("** [x] العربية\n"));
    assert!(source.contains("- [ ] literal"));
    assert!(source.contains("\\[ ] comment"));
    assert!(source.contains("> - [ ] quote"));

    create_child_node(&workspace, "Concurrent").expect("concurrent change");
    let error = commit_previewed_task_import(
        &workspace,
        &preview,
        &TaskImportReview::from_preview(&preview),
        temporary.path().join("stale-receipt.json"),
        "2026-08-24T00:01:00Z",
    )
    .expect_err("stale preview");
    assert_eq!(error.code, IntakeErrorCode::StalePreview);
    assert!(!workspace.join("Imported").exists());
}

#[test]
fn malformed_task_source_remains_a_read_only_conflicted_preview() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");
    let root_id = scan_workspace(&workspace).nodes[0].id.expect("root id");
    let documents = vec![TaskImportDocumentInput {
        locator: "blocked.md".to_owned(),
        source: "- [ ] #task Impossible 📅 2026-02-30\n".to_owned(),
    }];
    let preview = preview_task_import(
        &workspace,
        &ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("temp root"),
        root_id,
        "Blocked",
        documents,
        obsidian_settings(),
        "2026-08-24T00:00:00Z",
        &CancellationToken::default(),
    )
    .expect("diagnostic preview");
    assert!(!preview.task_plan.is_committable());
    assert!(!workspace.join("Blocked").exists());
    let error = commit_previewed_task_import(
        &workspace,
        &preview,
        &TaskImportReview::from_preview(&preview),
        temporary.path().join("blocked-receipt.json"),
        "2026-08-24T00:01:00Z",
    )
    .expect_err("blocking diagnostics");
    assert_eq!(error.code, IntakeErrorCode::ProposalConflict);
    assert!(!workspace.join("Blocked").exists());
}

#[test]
fn committed_but_unreceipted_import_recovers_receipt_before_finalizing_journal() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");
    let root_id = scan_workspace(&workspace).nodes[0].id.expect("root id");
    let preview = preview_task_import(
        &workspace,
        &ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("temp root"),
        root_id,
        "Recovered",
        vec![TaskImportDocumentInput {
            locator: "tasks.md".to_owned(),
            source: "- [ ] 中文 task\n".to_owned(),
        }],
        TaskImportSettings::markdown_checklist_v1(4),
        "2026-08-24T00:00:00Z",
        &CancellationToken::default(),
    )
    .expect("preview");
    let review = TaskImportReview::from_preview(&preview);
    let authority = WorkspaceImportAuthority {
        proposal_id: preview.proposal_id.clone(),
        proposal_digest: preview.bundle_digest.to_string(),
    };
    let nodes = preview
        .nodes
        .iter()
        .map(|node| WorkspaceImportNode {
            locator: node.destination_locator.clone(),
            node_id: node.node_id,
            document_file: node.document_file.clone(),
            exact_source: node.exact_asciidoc.clone(),
            document_sha256: node.document_digest.to_string(),
            resources: Vec::new(),
        })
        .collect();
    let plan = plan_import_tree(
        &workspace,
        &preview.base_workspace_revision,
        authority,
        nodes,
    )
    .expect("reviewed Core import plan");
    commit_workspace_transaction_retaining_journal(
        &plan,
        temporary.path().join("recovered-receipt.json"),
    )
    .expect("simulate crash after committed marker and before receipt");
    assert!(workspace.join("Recovered/tasks/tasks.adoc").is_file());

    let receipt = temporary.path().join("recovered-receipt.json");
    let recovery = recover_previewed_task_import(
        &workspace,
        &preview,
        &review,
        &receipt,
        "2026-08-24T00:00:00Z",
    )
    .expect("recover committed receipt");
    assert!(matches!(
        recovery,
        TaskImportRecovery::ReceiptRecovered { .. }
    ));
    assert!(receipt.is_file());
    create_child_node(&workspace, "AfterRecovery").expect("journal finalized after receipt");
    let idempotent = recover_previewed_task_import(
        &workspace,
        &preview,
        &review,
        &receipt,
        "2026-08-24T00:03:00Z",
    )
    .expect("idempotent finalized recovery");
    assert!(matches!(
        idempotent,
        TaskImportRecovery::AlreadyFinalized { .. }
    ));
}
