use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use weftext_agent_dsh::{DSH_RUNTIME_NAME, DshClient, DshCompatibilityPolicy, DshInitialize};
use weftext_core::{
    AdjacentHeadingBody, CalendarDate, ChildSort, ChronoPeriod, ChronoPlan, CitationAccessScope,
    CitationEditTarget, CitationMacroIntent, CitationPresentationProfile,
    CitationPresentationRequest, CitationWorkspaceIndex, DocumentEdit, DocumentRevision, NodeId,
    QueryAccessScope, QueryEvaluationContext, QueryWorkspaceIndex, SortDirection, SortMode,
    SyncDisposition, TaskEditIntent, TaskEditTarget, TaskId, TaskRecurrenceCompletionContext,
    TaskWorkspaceIndex, TrashItemId, TrashResourceSelection, TrashRestoreMode,
    TrashReviewedReplanAuthorization, TrashReviewedRequest, WorkspaceTransactionPlan,
    analyze_citation_authoring_source, build_workspace_link_index,
    citation_presentation_capabilities, classify_sync_state, commit_document_edit,
    commit_task_dependency_transaction, commit_task_edit_transaction,
    commit_task_recurrence_transaction, commit_workspace_transaction,
    confirm_permanent_delete_trash_items, create_workspace, load_legacy_trash_migration_backup,
    plan_adjacent_heading_body_setting, plan_citation_macro_edit, plan_copy_node,
    plan_create_child_node, plan_document_edit, plan_migrate_legacy_workspace_trash_with_backup,
    plan_move_node, plan_node_aliases_setting, plan_node_child_sort_setting,
    plan_node_sibling_rank_setting, plan_permanently_delete_trash_items, plan_rename_node,
    plan_restore_node, plan_restore_trash_item, plan_task_dependency_transaction,
    plan_task_edit_transaction, plan_task_recurrence_transaction, plan_trash_node,
    plan_trash_resources, prepare_legacy_trash_migration_backup, present_citations,
    preview_permanent_delete_trash_items, project_workspace_trash_state, read_node_document,
    read_workspace_revision, recover_workspace_transactions, replan_reviewed_trash_request,
    scan_workspace,
};

mod backup_command;
mod export_command;
mod import_command;
mod prototype_bridge;

const SCHEMA: &str = "weftext.cli.v1";

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if matches!(
        arguments.as_slice(),
        [scope, protocol, command, _]
            if scope == "agent" && protocol == "mcp" && command == "serve"
    ) {
        let workspace = &arguments[3];
        return match weftext_agent_mcp::serve_stdio(workspace) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(2)
            }
        };
    }
    match run(&arguments) {
        Ok(value) => {
            if value.get("ok").and_then(serde_json::Value::as_bool) == Some(false) {
                eprintln!("{value}");
                ExitCode::from(2)
            } else {
                println!("{value}");
                ExitCode::SUCCESS
            }
        }
        Err(message) => {
            eprintln!(
                "{}",
                json!({"schema": SCHEMA, "ok": false, "error": message})
            );
            ExitCode::from(2)
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run(arguments: &[String]) -> Result<serde_json::Value, String> {
    if arguments.first().is_some_and(|scope| scope == "document") {
        return run_document(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "agent") {
        return run_agent(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "node") {
        return run_node(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "citation") {
        return run_citation(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "task") {
        return run_task(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "query") {
        return run_query(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "trash") {
        return run_trash(arguments);
    }
    if arguments.first().is_some_and(|scope| scope == "backup") {
        return backup_command::run(arguments, SCHEMA);
    }
    if arguments.first().is_some_and(|scope| scope == "import") {
        return import_command::run(arguments, SCHEMA);
    }
    if arguments.first().is_some_and(|scope| scope == "export") {
        return export_command::run(arguments, SCHEMA);
    }
    match arguments {
        [command] if command == "version" => Ok(json!({
            "schema": SCHEMA,
            "ok": true,
            "product": "weftext",
            "version": env!("CARGO_PKG_VERSION"),
            "storage": weftext_core::MANAGED_DOCUMENT_PROFILE_ID,
        })),
        [scope, command, root] if scope == "workspace" && command == "create" => {
            let created =
                create_workspace(PathBuf::from(root)).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "node": {"id": created.id, "name": created.path.file_name()},
            }))
        }
        [scope, command, root] if scope == "workspace" && command == "inventory" => {
            let inventory = scan_workspace(root);
            let disposition = classify_sync_state(&inventory);
            let trash_state = project_workspace_trash_state(root).ok();
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "workspace": {
                    "valid": inventory.is_valid(),
                    "syncDisposition": disposition_name(&disposition),
                    "nodes": inventory.nodes.iter().map(|node| json!({
                        "id": node.id,
                        "name": node.name,
                        "path": safe_locator(&inventory.root, &node.path),
                        "documentPath": safe_locator(&inventory.root, &node.document_path),
                        "parentId": node.parent_id,
                        "metadataDiagnostics": node.metadata_diagnostics,
                    })).collect::<Vec<_>>(),
                    "content": inventory.content.iter().map(|entry| content_entry_json(&inventory, entry)).collect::<Vec<_>>(),
                    "trashItems": trash_state.as_ref().map_or_else(Vec::new, |state| state.items.clone()),
                    "trashState": trash_state.as_ref().map(|state| state.state),
                    "trashReconciliationRequired": trash_state.as_ref().is_some_and(|state| state.reconciliation_required),
                    "legacyTrashMigrationRequired": trash_state.as_ref().is_some_and(|state| state.legacy_migration_required),
                    "issues": inventory.issues.iter().map(|issue| json!({
                        "code": format!("{:?}", issue.code),
                        "path": safe_locator(&inventory.root, &issue.path),
                        "message": issue.message,
                    })).collect::<Vec<_>>(),
                }
            }))
        }
        [scope, command, root] if scope == "workspace" && command == "recover" => {
            let report = recover_workspace_transactions(root).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "recovery": report,
            }))
        }
        [scope, command, root] if scope == "workspace" && command == "links" => {
            let index = build_workspace_link_index(root).map_err(|error| error.to_string())?;
            Ok(json!({"schema": SCHEMA, "ok": true, "links": index}))
        }
        [scope, command, root, value]
            if scope == "workspace"
                && matches!(command.as_str(), "presentation-preview" | "presentation") =>
        {
            let value = match value.as_str() {
                "separate" => AdjacentHeadingBody::Separate,
                "run_in" => AdjacentHeadingBody::RunIn,
                _ => return Err("presentation value must be separate or run_in".to_owned()),
            };
            let plan = plan_adjacent_heading_body_setting(root, value)
                .map_err(|error| error.to_string())?;
            if command == "presentation" {
                let committed =
                    commit_workspace_transaction(&plan).map_err(|error| error.to_string())?;
                Ok(json!({"schema": SCHEMA, "ok": true, "commit": committed}))
            } else {
                Ok(json!({"schema": SCHEMA, "ok": true, "plan": transaction_plan_json(&plan)}))
            }
        }
        [scope, command, date] if scope == "chrono" && command == "plan" => {
            let date = parse_date(date)?;
            let plan = ChronoPlan::build(
                date,
                &[
                    ChronoPeriod::Year,
                    ChronoPeriod::Quarter,
                    ChronoPeriod::Month,
                    ChronoPeriod::Week,
                    ChronoPeriod::Day,
                ],
            );
            Ok(json!({"schema": SCHEMA, "ok": true, "plan": plan}))
        }
        [scope, command, node] if scope == "prototype" && command == "serve" => {
            prototype_bridge::serve(node, 32_171)
        }
        [scope, command, node, port] if scope == "prototype" && command == "serve" => {
            let port = port
                .parse::<u16>()
                .map_err(|_| "prototype bridge port must be between 0 and 65535".to_owned())?;
            prototype_bridge::serve(node, port)
        }
        _ => Err(
            "usage: weftext <version|workspace <create|inventory|recover|links|presentation|presentation-preview>|backup ...|node ...|chrono plan|citation ...|task ...|query execute|document <read|preview|commit>|import <fake-preview|markdown-preview|pdf-capability|pdf-preview|agent-prepare|agent-export-evidence|agent-apply|commit|recover|task-preview|task-commit|task-recover>|export <markdown-preview|commit>|prototype serve|agent <mcp serve|dsh support|dsh probe>>"
                .to_owned(),
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn run_node(arguments: &[String]) -> Result<serde_json::Value, String> {
    let (plan, commit) = match arguments {
        [scope, command, root, parent, name]
            if scope == "node" && matches!(command.as_str(), "create-preview" | "create") =>
        {
            (
                plan_create_child_node(root, parse_node_id(parent)?, name)
                    .map_err(|error| error.to_string())?,
                command == "create",
            )
        }
        [scope, command, root, node, name]
            if scope == "node" && matches!(command.as_str(), "rename-preview" | "rename") =>
        {
            (
                plan_rename_node(root, parse_node_id(node)?, name)
                    .map_err(|error| error.to_string())?,
                command == "rename",
            )
        }
        [scope, command, root, node, parent]
            if scope == "node" && matches!(command.as_str(), "move-preview" | "move") =>
        {
            let node_id = parse_node_id(node)?;
            let source_name = managed_node_name(root, node_id)?;
            (
                plan_move_node(root, node_id, parse_node_id(parent)?, &source_name)
                    .map_err(|error| error.to_string())?,
                command == "move",
            )
        }
        [scope, command, root, node, parent, name]
            if scope == "node" && matches!(command.as_str(), "copy-preview" | "copy") =>
        {
            (
                plan_copy_node(root, parse_node_id(node)?, parse_node_id(parent)?, name)
                    .map_err(|error| error.to_string())?,
                command == "copy",
            )
        }
        [scope, command, root, node]
            if scope == "node" && matches!(command.as_str(), "trash-preview" | "trash") =>
        {
            (
                plan_trash_node(root, parse_node_id(node)?).map_err(|error| error.to_string())?,
                command == "trash",
            )
        }
        [scope, command, root, node, parent, name]
            if scope == "node" && matches!(command.as_str(), "restore-preview" | "restore") =>
        {
            (
                plan_restore_node(root, parse_node_id(node)?, parse_node_id(parent)?, name)
                    .map_err(|error| error.to_string())?,
                command == "restore",
            )
        }
        [scope, command, root, node, revision, aliases]
            if scope == "node" && matches!(command.as_str(), "aliases-preview" | "aliases") =>
        {
            let aliases = parse_json_argument::<Vec<String>>(aliases, "node aliases")?;
            (
                plan_node_aliases_setting(
                    root,
                    parse_node_id(node)?,
                    &parse_document_revision(revision)?,
                    &aliases,
                )
                .map_err(|error| error.to_string())?,
                command == "aliases",
            )
        }
        [scope, command, root, node, revision, value]
            if scope == "node"
                && matches!(command.as_str(), "child-sort-preview" | "child-sort") =>
        {
            let child_sort = match value.as_str() {
                "name:ascending" => ChildSort::default(),
                "name:descending" => ChildSort {
                    mode: SortMode::Name,
                    direction: SortDirection::Descending,
                },
                "manual" => ChildSort {
                    mode: SortMode::Manual,
                    direction: SortDirection::Ascending,
                },
                _ => {
                    return Err(
                        "child sort must be name:ascending, name:descending, or manual".to_owned(),
                    );
                }
            };
            (
                plan_node_child_sort_setting(
                    root,
                    parse_node_id(node)?,
                    &parse_document_revision(revision)?,
                    child_sort,
                )
                .map_err(|error| error.to_string())?,
                command == "child-sort",
            )
        }
        [scope, command, root, node, revision, value]
            if scope == "node"
                && matches!(command.as_str(), "sibling-rank-preview" | "sibling-rank") =>
        {
            let sibling_rank =
                if value == "none" {
                    None
                } else {
                    Some(value.parse::<u64>().map_err(|_| {
                        "sibling rank must be a positive integer or none".to_owned()
                    })?)
                };
            (
                plan_node_sibling_rank_setting(
                    root,
                    parse_node_id(node)?,
                    &parse_document_revision(revision)?,
                    sibling_rank,
                )
                .map_err(|error| error.to_string())?,
                command == "sibling-rank",
            )
        }
        _ => {
            return Err(
                "usage: weftext node <create[-preview]|rename[-preview]|move[-preview]|copy[-preview]|trash[-preview]|restore[-preview]|aliases[-preview]|child-sort[-preview]|sibling-rank[-preview]> ..."
                    .to_owned(),
            );
        }
    };
    if commit {
        let committed = commit_workspace_transaction(&plan).map_err(|error| error.to_string())?;
        Ok(json!({"schema": SCHEMA, "ok": true, "commit": committed}))
    } else {
        Ok(json!({"schema": SCHEMA, "ok": true, "plan": transaction_plan_json(&plan)}))
    }
}

fn transaction_plan_json(plan: &WorkspaceTransactionPlan) -> serde_json::Value {
    if !plan.trash_item_changes().is_empty() {
        return trash_transaction_plan_json(plan);
    }
    json!({
        "planId": plan.plan_id,
        "action": plan.action,
        "baseRevision": plan.base_revision,
        "pathChanges": plan.path_changes,
        "documentChanges": plan.document_changes,
        "generatedNodeIds": plan.generated_node_ids,
        "scopeSummary": plan.scope_summary,
        "identityMap": plan.identity_map,
        "capturedTarget": plan.captured_target,
        "targetNodeIds": plan.target_node_ids,
        "draftSensitiveNodeIds": plan.draft_sensitive_node_ids,
        "trashItemChanges": plan.trash_item_changes(),
    })
}

fn trash_transaction_plan_json(plan: &WorkspaceTransactionPlan) -> serde_json::Value {
    json!({
        "planId": plan.plan_id,
        "action": plan.action,
        "baseRevision": plan.base_revision,
        "pathChanges": [],
        "documentChanges": [],
        "generatedNodeIds": [],
        "scopeSummary": plan.scope_summary,
        "identityMap": plan.identity_map,
        "capturedTarget": plan.captured_target,
        "targetNodeIds": plan.target_node_ids,
        "draftSensitiveNodeIds": plan.draft_sensitive_node_ids,
        "trashItemChanges": plan.trash_item_changes(),
    })
}

fn managed_node_name(root: &str, node_id: NodeId) -> Result<String, String> {
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err("workspace must be valid before moving a node".to_owned());
    }
    inventory
        .nodes
        .iter()
        .find(|node| node.id == Some(node_id))
        .map(|node| node.name.clone())
        .ok_or_else(|| format!("node {node_id} was not found"))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TrashPermanentDeleteEvidence {
    trash_item_id: TrashItemId,
    payload_sha256: String,
    payload_byte_length: u64,
}

#[allow(clippy::too_many_lines)]
fn run_trash(arguments: &[String]) -> Result<serde_json::Value, String> {
    match arguments {
        [scope, command, root] if scope == "trash" && command == "inventory" => {
            let state = project_workspace_trash_state(root).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "trash": {
                    "workspaceRevision": read_workspace_revision(root).map_err(|error| error.to_string())?,
                    "state": state.state,
                    "items": state.items,
                    "reconciliation": {
                        "required": state.reconciliation_required,
                        "issueCount": state.diagnostic_count,
                    },
                    "legacyMigrationRequired": state.legacy_migration_required,
                },
            }))
        }
        [scope, command, root, node] if scope == "trash" && command == "node-preview" => {
            let plan =
                plan_trash_node(root, parse_node_id(node)?).map_err(|error| error.to_string())?;
            reviewed_trash_plan_response(&plan)
        }
        [scope, command, root, resources] if scope == "trash" && command == "resources-preview" => {
            let resources = parse_json_argument::<Vec<TrashResourceSelection>>(
                resources,
                "Trash resource selections",
            )?;
            let plan = plan_trash_resources(root, resources).map_err(|error| error.to_string())?;
            reviewed_trash_plan_response(&plan)
        }
        [scope, command, root, item]
            if scope == "trash"
                && matches!(
                    command.as_str(),
                    "restore-original-preview" | "restore-with-ancestors-preview"
                ) =>
        {
            let item = parse_trash_item_id(item)?;
            let mode = if command == "restore-original-preview" {
                TrashRestoreMode::Original
            } else {
                TrashRestoreMode::WithAncestors
            };
            let plan =
                plan_restore_trash_item(root, item, mode).map_err(|error| error.to_string())?;
            reviewed_trash_plan_response(&plan)
        }
        [scope, command, root, item, target, name]
            if scope == "trash" && command == "restore-existing-target-preview" =>
        {
            let plan = plan_restore_trash_item(
                root,
                parse_trash_item_id(item)?,
                TrashRestoreMode::ExistingTarget {
                    target_node_id: parse_node_id(target)?,
                    name: name.clone(),
                },
            )
            .map_err(|error| error.to_string())?;
            reviewed_trash_plan_response(&plan)
        }
        [scope, command, root, snapshot_parent]
            if scope == "trash" && command == "migration-preview" =>
        {
            let backup = prepare_legacy_trash_migration_backup(root, snapshot_parent)
                .map_err(|error| error.to_string())?;
            let plan = plan_migrate_legacy_workspace_trash_with_backup(root, &backup)
                .map_err(|error| error.to_string())?;
            let mut response = reviewed_trash_plan_response(&plan)?;
            response["migrationBackup"] = json!({
                "snapshotDirectory": backup.snapshot_directory(),
                "authority": backup.authority(),
            });
            Ok(response)
        }
        [scope, command, root, item_ids]
            if scope == "trash" && command == "permanent-delete-preview" =>
        {
            let ids = parse_json_argument::<Vec<TrashItemId>>(item_ids, "Trash item IDs")?;
            let preview = preview_permanent_delete_trash_items(root, ids)
                .map_err(|error| error.to_string())?;
            let confirmation_items = permanent_delete_evidence(&preview);
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "permanentDeletePreview": preview,
                "confirmationItems": confirmation_items,
            }))
        }
        [scope, command, root, evidence, phrase]
            if scope == "trash" && command == "permanent-delete-review" =>
        {
            let supplied = parse_json_argument::<Vec<TrashPermanentDeleteEvidence>>(
                evidence,
                "exact Trash permanent-delete evidence",
            )?;
            let ids = supplied.iter().map(|item| item.trash_item_id).collect();
            let preview = preview_permanent_delete_trash_items(root, ids)
                .map_err(|error| error.to_string())?;
            require_exact_permanent_delete_evidence(&preview, supplied)?;
            let confirmation = confirm_permanent_delete_trash_items(preview, true, phrase)
                .map_err(|error| error.to_string())?;
            let plan = plan_permanently_delete_trash_items(root, &confirmation)
                .map_err(|error| error.to_string())?;
            reviewed_trash_plan_response(&plan)
        }
        [scope, command, root, reviewed] if scope == "trash" && command == "commit" => {
            commit_reviewed_trash_request(
                root,
                reviewed,
                TrashReviewedReplanAuthorization::Ordinary,
            )
        }
        [scope, command, root, reviewed, snapshot_directory]
            if scope == "trash" && command == "migration-commit" =>
        {
            let backup = load_legacy_trash_migration_backup(root, snapshot_directory)
                .map_err(|error| error.to_string())?;
            commit_reviewed_trash_request(
                root,
                reviewed,
                TrashReviewedReplanAuthorization::LegacyMigration { backup },
            )
        }
        [scope, command, root, reviewed, phrase]
            if scope == "trash" && command == "permanent-delete-commit" =>
        {
            commit_reviewed_trash_request(
                root,
                reviewed,
                TrashReviewedReplanAuthorization::PermanentDelete {
                    higher_permission_granted: true,
                    exact_phrase: phrase.clone(),
                },
            )
        }
        [scope, command, root] if scope == "trash" && command == "recover" => {
            let recovery =
                recover_workspace_transactions(root).map_err(|error| error.to_string())?;
            Ok(json!({"schema": SCHEMA, "ok": true, "recovery": recovery}))
        }
        _ => Err(concat!(
            "usage: weftext trash <inventory ROOT|node-preview ROOT NODE_ID|",
            "resources-preview ROOT SELECTIONS_JSON|restore-original-preview ROOT ITEM_ID|",
            "restore-with-ancestors-preview ROOT ITEM_ID|",
            "restore-existing-target-preview ROOT ITEM_ID TARGET_NODE_ID NAME|",
            "migration-preview ROOT SNAPSHOT_PARENT|permanent-delete-preview ROOT ITEM_IDS_JSON|",
            "permanent-delete-review ROOT EVIDENCE_JSON EXACT_PHRASE|",
            "commit ROOT REVIEWED_REQUEST_JSON|",
            "migration-commit ROOT REVIEWED_REQUEST_JSON SNAPSHOT_DIRECTORY|",
            "permanent-delete-commit ROOT REVIEWED_REQUEST_JSON EXACT_PHRASE|recover ROOT>"
        )
        .to_owned()),
    }
}

fn permanent_delete_evidence(
    preview: &weftext_core::TrashPermanentDeletePreview,
) -> Vec<TrashPermanentDeleteEvidence> {
    preview
        .items
        .iter()
        .map(|item| TrashPermanentDeleteEvidence {
            trash_item_id: item.trash_item_id,
            payload_sha256: item.payload_sha256.clone(),
            payload_byte_length: item.payload_byte_length,
        })
        .collect()
}

fn require_exact_permanent_delete_evidence(
    preview: &weftext_core::TrashPermanentDeletePreview,
    mut supplied: Vec<TrashPermanentDeleteEvidence>,
) -> Result<(), String> {
    let mut expected = permanent_delete_evidence(preview);
    supplied.sort_by_key(|item| item.trash_item_id);
    expected.sort_by_key(|item| item.trash_item_id);
    if supplied == expected {
        Ok(())
    } else {
        Err("permanent deletion evidence must exactly match item IDs, payload digests, and byte lengths".to_owned())
    }
}

fn parse_trash_item_id(value: &str) -> Result<TrashItemId, String> {
    value
        .parse()
        .map_err(|error: weftext_core::TrashIdError| error.to_string())
}

fn reviewed_trash_plan_response(
    plan: &WorkspaceTransactionPlan,
) -> Result<serde_json::Value, String> {
    let reviewed = plan
        .reviewed_trash_request()
        .ok_or("Core Trash plan did not include a reviewed request")?;
    Ok(json!({
        "schema": SCHEMA,
        "ok": true,
        "plan": transaction_plan_json(plan),
        "reviewedRequest": reviewed,
    }))
}

fn read_reviewed_trash_request(argument: &str) -> Result<TrashReviewedRequest, String> {
    let bytes = if let Some(path) = argument.strip_prefix('@') {
        fs::read(path).map_err(|error| format!("could not read Trash reviewed request: {error}"))?
    } else {
        argument.as_bytes().to_vec()
    };
    TrashReviewedRequest::from_json_bytes(&bytes)
}

fn commit_reviewed_trash_request(
    root: &str,
    argument: &str,
    authorization: TrashReviewedReplanAuthorization,
) -> Result<serde_json::Value, String> {
    let reviewed = read_reviewed_trash_request(argument)?;
    let plan = replan_reviewed_trash_request(root, &reviewed, authorization)
        .map_err(|error| error.to_string())?;
    let commit = commit_workspace_transaction(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": SCHEMA,
        "ok": true,
        "reviewId": reviewed.review_id,
        "commit": commit,
        "trashItemChanges": plan.trash_item_changes(),
    }))
}

#[allow(clippy::too_many_lines)]
fn run_task(arguments: &[String]) -> Result<serde_json::Value, String> {
    match arguments {
        [scope, command, root] if scope == "task" && command == "validate" => {
            let index = TaskWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": index.diagnostics().is_empty(),
                "validation": {
                    "valid": index.diagnostics().is_empty(),
                    "generation": index.generation(),
                    "occurrences": index.occurrences(),
                    "diagnostics": index.diagnostics(),
                },
                "error": (!index.diagnostics().is_empty()).then(|| json!({
                    "code": "task_validation_failed",
                    "message": "task validation found one or more diagnostics",
                })),
            }))
        }
        [scope, command, root, node] if scope == "task" && command == "inspect" => {
            let node = parse_node_id(node)?;
            let index = TaskWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
            let occurrences = index.occurrences_for_node(node).collect::<Vec<_>>();
            if occurrences.is_empty()
                && !scan_workspace(root)
                    .nodes
                    .iter()
                    .any(|record| record.id == Some(node))
            {
                return Err("task node is unavailable".to_owned());
            }
            let diagnostics = index
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.node_id == node)
                .collect::<Vec<_>>();
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "nodeId": node,
                "occurrences": occurrences,
                "diagnostics": diagnostics,
            }))
        }
        [scope, command, root, node, revision, target, intent]
            if scope == "task" && matches!(command.as_str(), "edit-preview" | "edit") =>
        {
            let node = parse_node_id(node)?;
            let revision = parse_document_revision(revision)?;
            let target = parse_json_argument::<TaskEditTarget>(target, "task edit target")?;
            let intent = parse_json_argument::<TaskEditIntent>(intent, "task edit intent")?;
            let plan = plan_task_edit_transaction(root, node, &revision, &target, &intent)
                .map_err(|error| error.to_string())?;
            if command == "edit" {
                let committed =
                    commit_task_edit_transaction(&plan).map_err(|error| error.to_string())?;
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "commit": committed,
                    "task": plan.authoring.target,
                    "assignedId": plan.authoring.assigned_id,
                }))
            } else {
                let transaction = transaction_plan_json(plan.workspace_transaction());
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "plan": {
                        "authoring": plan.authoring,
                        "transaction": transaction,
                    },
                }))
            }
        }
        [scope, command, root, node, revision, target, context]
            if scope == "task"
                && matches!(command.as_str(), "recurrence-preview" | "recurrence") =>
        {
            let node = parse_node_id(node)?;
            let revision = parse_document_revision(revision)?;
            let target = parse_json_argument::<TaskEditTarget>(target, "task edit target")?;
            let context = parse_json_argument::<TaskRecurrenceCompletionContext>(
                context,
                "task recurrence context",
            )?;
            let plan = plan_task_recurrence_transaction(root, node, &revision, &target, &context)
                .map_err(|error| error.to_string())?;
            if command == "recurrence" {
                let committed =
                    commit_task_recurrence_transaction(&plan).map_err(|error| error.to_string())?;
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "commit": committed,
                    "completedTask": plan.completion.completed_task,
                    "nextTask": plan.completion.next_task,
                    "nextTaskId": plan.completion.next_task_id,
                    "stopped": plan.completion.stopped,
                }))
            } else {
                let transaction = transaction_plan_json(plan.workspace_transaction());
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "plan": {
                        "completion": plan.completion,
                        "transaction": transaction,
                    },
                }))
            }
        }
        [scope, command, root, node, revision, target, dependencies]
            if scope == "task"
                && matches!(command.as_str(), "dependencies-preview" | "dependencies") =>
        {
            let node = parse_node_id(node)?;
            let revision = parse_document_revision(revision)?;
            let target = parse_json_argument::<TaskEditTarget>(target, "task edit target")?;
            let dependencies =
                parse_json_argument::<Vec<TaskId>>(dependencies, "task dependencies")?;
            let plan =
                plan_task_dependency_transaction(root, node, &revision, &target, &dependencies)
                    .map_err(|error| error.to_string())?;
            if command == "dependencies" {
                let committed =
                    commit_task_dependency_transaction(&plan).map_err(|error| error.to_string())?;
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "commit": committed,
                    "task": plan.authoring.target,
                    "assignedId": plan.authoring.assigned_id,
                    "dependencies": plan.dependencies,
                }))
            } else {
                let transaction = transaction_plan_json(plan.workspace_transaction());
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "plan": {
                        "authoring": plan.authoring,
                        "dependencies": plan.dependencies,
                        "transaction": transaction,
                    },
                }))
            }
        }
        [scope, command, root] if scope == "task" && command == "recover" => {
            let recovery =
                recover_workspace_transactions(root).map_err(|error| error.to_string())?;
            Ok(json!({"schema": SCHEMA, "ok": true, "recovery": recovery}))
        }
        _ => Err(concat!(
            "usage: weftext task <validate ROOT|inspect ROOT NODE_ID|",
            "edit[-preview] ROOT NODE_ID REVISION TARGET_JSON INTENT_JSON|",
            "recurrence[-preview] ROOT NODE_ID REVISION TARGET_JSON CONTEXT_JSON|",
            "dependencies[-preview] ROOT NODE_ID REVISION TARGET_JSON IDS_JSON|recover ROOT>"
        )
        .to_owned()),
    }
}

fn run_query(arguments: &[String]) -> Result<serde_json::Value, String> {
    let [scope, command, root, source_path, block_index, context] = arguments else {
        return Err(
            "usage: weftext query execute ROOT SOURCE_FILE BLOCK_INDEX CONTEXT_JSON".to_owned(),
        );
    };
    if scope != "query" || command != "execute" {
        return Err(
            "usage: weftext query execute ROOT SOURCE_FILE BLOCK_INDEX CONTEXT_JSON".to_owned(),
        );
    }
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("could not read query source: {error}"))?;
    let block_index = block_index
        .parse::<usize>()
        .map_err(|_| "query block index must be a non-negative integer".to_owned())?;
    let context = parse_json_argument::<QueryEvaluationContext>(context, "query context")?;
    let index = QueryWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
    let access = QueryAccessScope::complete(index.node_ids());
    let execution = index
        .execute_source(&source, block_index, &access, &context)
        .map_err(|error| error.to_string())?;
    let valid = execution.result.is_some();
    Ok(json!({
        "schema": SCHEMA,
        "ok": valid,
        "execution": execution,
        "error": (!valid).then(|| json!({
            "code": "query_source_invalid",
            "message": "the selected canonical query block is missing or invalid",
        })),
    }))
}

fn safe_locator(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| "<outside-workspace>".to_owned(),
        |relative| relative.to_string_lossy().replace('\\', "/"),
    )
}

fn content_entry_json(
    inventory: &weftext_core::WorkspaceInventory,
    entry: &weftext_core::WorkspaceContentEntry,
) -> serde_json::Value {
    let node = entry
        .node_id
        .and_then(|id| inventory.nodes.iter().find(|node| node.id == Some(id)));
    let fallback = match entry.kind {
        weftext_core::WorkspaceContentKind::ManagedNode
            if node.is_some_and(|node| node.path == inventory.root) =>
        {
            weftext_core::WorkspaceItemIconFallback::WorkspaceRoot
        }
        weftext_core::WorkspaceContentKind::ManagedNode
            if node.is_some_and(|node| {
                node.path == inventory.root.join(weftext_core::TRASH_NODE_NAME)
            }) =>
        {
            weftext_core::WorkspaceItemIconFallback::Trash
        }
        weftext_core::WorkspaceContentKind::ManagedNode => {
            weftext_core::WorkspaceItemIconFallback::ManagedNode
        }
        weftext_core::WorkspaceContentKind::UnmanagedDirectory => {
            weftext_core::WorkspaceItemIconFallback::UnmanagedFolder
        }
        weftext_core::WorkspaceContentKind::UnmanagedMarkdown => {
            weftext_core::WorkspaceItemIconFallback::UnmanagedMarkdown
        }
        weftext_core::WorkspaceContentKind::Resource => {
            weftext_core::WorkspaceItemIconFallback::OrdinaryFile
        }
    };
    let explicit = node
        .and_then(|node| std::fs::read_to_string(&node.document_path).ok())
        .and_then(|source| weftext_core::resolve_node_icon_from_source(&source));
    json!({
        "kind": entry.kind,
        "name": entry.name,
        "path": entry.relative_path,
        "parentPath": entry.parent_relative_path,
        "nodeId": entry.node_id,
        "ownerNodeId": entry.owner_node_id,
        "displayIcon": weftext_core::derive_workspace_item_icon(explicit, fallback),
    })
}

fn parse_node_id(value: &str) -> Result<weftext_core::NodeId, String> {
    value
        .parse()
        .map_err(|error: weftext_core::NodeIdError| error.to_string())
}

fn run_document(arguments: &[String]) -> Result<serde_json::Value, String> {
    match arguments {
        [scope, command, node] if scope == "document" && command == "read" => {
            let snapshot = read_node_document(node).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "document": {
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "length": snapshot.source.len(),
                    "source": snapshot.source,
                }
            }))
        }
        [scope, command, node, revision, start, end, replacement]
            if scope == "document" && command == "preview" =>
        {
            let plan = plan_cli_edit(node, revision, start, end, replacement)?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "plan": {
                    "action": "document.edit",
                    "nodeId": plan.node_id,
                    "baseRevision": &plan.base_revision,
                    "nextRevision": &plan.next_revision,
                    "oldLength": plan.old_length,
                    "newLength": plan.new_length,
                    "changed": plan.changed,
                    "edits": &plan.edits,
                    "nextSource": plan.next_source(),
                }
            }))
        }
        [scope, command, node, revision, start, end, replacement]
            if scope == "document" && command == "commit" =>
        {
            let plan = plan_cli_edit(node, revision, start, end, replacement)?;
            let committed = commit_document_edit(&plan).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "commit": {
                    "action": "document.edit",
                    "nodeId": committed.node_id,
                    "baseRevision": &plan.base_revision,
                    "revision": committed.revision,
                    "length": committed.length,
                }
            }))
        }
        _ => Err("usage: weftext <document read|document preview|document commit>".to_owned()),
    }
}

#[allow(clippy::too_many_lines)]
fn run_citation(arguments: &[String]) -> Result<serde_json::Value, String> {
    match arguments {
        [scope, command] if scope == "citation" && command == "capabilities" => Ok(json!({
            "schema": SCHEMA,
            "ok": true,
            "capabilities": citation_presentation_capabilities(),
        })),
        [scope, command, root] if scope == "citation" && command == "validate" => {
            let index = CitationWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
            let access = complete_citation_scope(&index);
            let inventory = scan_workspace(root);
            let mut components = Vec::with_capacity(inventory.nodes.len());
            let mut component_diagnostic_count = 0_usize;
            for node in &inventory.nodes {
                let node_id = node
                    .id
                    .ok_or_else(|| "workspace node has no valid identity".to_owned())?;
                let analysis = index
                    .analyze_component(node_id, &access)
                    .map_err(|error| error.to_string())?;
                component_diagnostic_count += analysis.diagnostics.len();
                components.push(analysis);
            }
            let valid = index.diagnostics().is_empty() && component_diagnostic_count == 0;
            Ok(json!({
                "schema": SCHEMA,
                "ok": valid,
                "validation": {
                    "valid": valid,
                    "generation": index.generation(),
                    "referenceDiagnostics": index.diagnostics(),
                    "components": components,
                },
                "error": (!valid).then(|| json!({
                    "code": "citation_validation_failed",
                    "message": "citation validation found one or more diagnostics",
                })),
            }))
        }
        [scope, command, root, query] if scope == "citation" && command == "search" => {
            search_citations(root, query, "25")
        }
        [scope, command, root, query, limit] if scope == "citation" && command == "search" => {
            search_citations(root, query, limit)
        }
        [scope, command, root, component] if scope == "citation" && command == "inspect" => {
            let component = parse_node_id(component)?;
            let index = CitationWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
            let access = complete_citation_scope(&index);
            let analysis = index
                .analyze_component(component, &access)
                .map_err(|error| error.to_string())?;
            let node = citation_node_snapshot(root, component)?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "analysis": analysis,
                "authoring": analyze_citation_authoring_source(&node.source),
            }))
        }
        [scope, command, root, component, style, locale]
            if scope == "citation" && command == "render" =>
        {
            let component = parse_node_id(component)?;
            let index = CitationWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
            let access = complete_citation_scope(&index);
            let compilation = index
                .collect_bibliography_inputs(&[component], &access)
                .map_err(|error| error.to_string())?;
            let request = CitationPresentationRequest::new(
                CitationPresentationProfile::new(style, locale),
                compilation,
            );
            match present_citations(&request) {
                Ok(presentation) => Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "presentation": presentation,
                })),
                Err(failure) => Ok(json!({
                    "schema": SCHEMA,
                    "ok": false,
                    "error": {
                        "code": "citation_presentation_failed",
                        "message": failure.to_string(),
                        "diagnostics": failure.diagnostics,
                    },
                })),
            }
        }
        [scope, command, root]
            if scope == "citation" && matches!(command.as_str(), "recover" | "rollback") =>
        {
            let report = recover_workspace_transactions(root).map_err(|error| error.to_string())?;
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "operation": command,
                "recovery": report,
            }))
        }
        [scope, command, root, component, revision, target, intent]
            if scope == "citation"
                && matches!(command.as_str(), "macro-edit-preview" | "macro-edit") =>
        {
            let component = parse_node_id(component)?;
            let revision = parse_document_revision(revision)?;
            let snapshot = citation_node_snapshot(root, component)?;
            require_document_revision(&revision, &snapshot.revision)?;
            let target = parse_json_argument::<CitationEditTarget>(target, "citation edit target")?;
            let intent = parse_json_argument::<CitationMacroIntent>(intent, "citation intent")?;
            let index = CitationWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
            let access = complete_citation_scope(&index);
            let authoring = plan_citation_macro_edit(
                &index,
                component,
                &snapshot.source,
                &access,
                &target,
                &intent,
            )
            .map_err(|error| error.to_string())?;
            if command == "macro-edit" {
                let path = citation_node_path(root, component)?;
                let plan = plan_document_edit(path, &revision, [authoring.edit.clone()])
                    .map_err(|error| error.to_string())?;
                let committed = commit_document_edit(&plan).map_err(|error| error.to_string())?;
                Ok(json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "commit": committed_document_json(&committed),
                }))
            } else {
                Ok(json!({"schema": SCHEMA, "ok": true, "plan": authoring}))
            }
        }
        _ => Err(concat!(
            "usage: weftext citation <capabilities|validate ROOT|search ROOT QUERY [LIMIT]|",
            "inspect ROOT COMPONENT_ID|render ROOT COMPONENT_ID STYLE LOCALE|",
            "recover ROOT|rollback ROOT|",
            "macro-edit[-preview] ROOT COMPONENT_ID REVISION TARGET_JSON INTENT_JSON>"
        )
        .to_owned()),
    }
}

fn search_citations(root: &str, query: &str, limit: &str) -> Result<serde_json::Value, String> {
    let limit = limit
        .parse::<usize>()
        .map_err(|_| "citation search limit must be an unsigned integer".to_owned())?;
    let index = CitationWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())?;
    let access = complete_citation_scope(&index);
    let hits = index
        .search_references(query, &access, limit)
        .map_err(|error| error.to_string())?;
    Ok(json!({"schema": SCHEMA, "ok": true, "references": hits}))
}

fn complete_citation_scope(index: &CitationWorkspaceIndex) -> CitationAccessScope {
    CitationAccessScope::complete(index.reference_node_ids())
}

fn citation_node_path(root: &str, node_id: NodeId) -> Result<PathBuf, String> {
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err(format!(
            "workspace inventory is invalid: {:?}",
            inventory.issues.first().map(|issue| issue.code)
        ));
    }
    inventory
        .nodes
        .into_iter()
        .find(|node| node.id == Some(node_id))
        .map(|node| node.path)
        .ok_or_else(|| format!("node {node_id} is not in the workspace"))
}

fn citation_node_snapshot(
    root: &str,
    node_id: NodeId,
) -> Result<weftext_core::DocumentSnapshot, String> {
    read_node_document(citation_node_path(root, node_id)?).map_err(|error| error.to_string())
}

fn parse_document_revision(value: &str) -> Result<DocumentRevision, String> {
    DocumentRevision::parse(value).map_err(|error| error.to_string())
}

fn require_document_revision(
    expected: &DocumentRevision,
    actual: &DocumentRevision,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "stale document revision: expected {expected}, actual {actual}"
        ))
    }
}

fn parse_json_argument<T: serde::de::DeserializeOwned>(
    argument: &str,
    label: &str,
) -> Result<T, String> {
    let source = if let Some(path) = argument.strip_prefix('@') {
        fs::read_to_string(path).map_err(|error| format!("could not read {label} file: {error}"))?
    } else {
        argument.to_owned()
    };
    serde_json::from_str(&source).map_err(|error| format!("invalid {label} JSON: {error}"))
}

fn committed_document_json(committed: &weftext_core::CommittedDocument) -> serde_json::Value {
    json!({
        "nodeId": committed.node_id,
        "revision": committed.revision,
        "length": committed.length,
    })
}

fn run_agent(arguments: &[String]) -> Result<serde_json::Value, String> {
    match arguments {
        [scope, harness, command]
            if scope == "agent" && harness == "dsh" && command == "support" =>
        {
            let policy = DshCompatibilityPolicy::default();
            Ok(json!({
                "schema": SCHEMA,
                "ok": true,
                "agent": {
                    "harness": "dsh",
                    "supportTier": "first_tier",
                    "implementationStage": "read_only_tools",
                    "ready": false,
                    "runtimeName": DSH_RUNTIME_NAME,
                    "supportedWireVersions": policy.supported_versions().collect::<Vec<_>>(),
                    "cancellation": "runtime_termination",
                    "approvalRequests": false,
                    "readOnlyMcpTools": true,
                    "mutationTools": false,
                }
            }))
        }
        [
            scope,
            harness,
            command,
            runtime,
            provider,
            model,
            cwd,
            runtime_arguments @ ..,
        ] if scope == "agent" && harness == "dsh" && command == "probe" => {
            probe_dsh(runtime, provider, model, cwd, runtime_arguments)
        }
        _ => Err("usage: weftext <agent mcp serve|agent dsh support|agent dsh probe>".to_owned()),
    }
}

fn plan_cli_edit(
    node: &str,
    revision: &str,
    start: &str,
    end: &str,
    replacement: &str,
) -> Result<weftext_core::DocumentEditPlan, String> {
    let revision = DocumentRevision::parse(revision).map_err(|error| error.to_string())?;
    let start = start
        .parse::<u64>()
        .map_err(|_| "document edit start must be an unsigned byte offset".to_owned())?;
    let end = end
        .parse::<u64>()
        .map_err(|_| "document edit end must be an unsigned byte offset".to_owned())?;
    plan_document_edit(
        node,
        &revision,
        [DocumentEdit {
            start,
            end,
            replacement: replacement.to_owned(),
        }],
    )
    .map_err(|error| error.to_string())
}

fn probe_dsh(
    runtime: &str,
    provider: &str,
    model: &str,
    cwd: &str,
    runtime_arguments: &[String],
) -> Result<serde_json::Value, String> {
    let mut command = Command::new(runtime);
    command.args(runtime_arguments);
    let mut client =
        DshClient::spawn(command, Duration::from_secs(30)).map_err(|error| error.to_string())?;
    let handshake = client
        .initialize(
            &DshInitialize {
                cwd: PathBuf::from(cwd),
                provider: provider.to_owned(),
                model: model.to_owned(),
                max_tokens: None,
            },
            &DshCompatibilityPolicy::default(),
        )
        .map_err(|error| error.to_string())?;
    client.shutdown().map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": SCHEMA,
        "ok": true,
        "agent": handshake,
    }))
}

fn parse_date(value: &str) -> Result<CalendarDate, String> {
    let pieces = value.split('-').collect::<Vec<_>>();
    if pieces.len() != 3 {
        return Err("date must use YYYY-MM-DD".to_owned());
    }
    let year = pieces[0].parse().map_err(|_| "invalid year")?;
    let month = pieces[1].parse().map_err(|_| "invalid month")?;
    let day = pieces[2].parse().map_err(|_| "invalid day")?;
    CalendarDate::new(year, month, day).map_err(|error| error.to_string())
}

const fn disposition_name(disposition: &SyncDisposition) -> &'static str {
    match disposition {
        SyncDisposition::Ready => "ready",
        SyncDisposition::WaitForMoreFiles { .. } => "wait_for_more_files",
        SyncDisposition::NeedsUserResolution { .. } => "needs_user_resolution",
    }
}
