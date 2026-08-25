//! Narrow loopback bridge for the Stage 1B interaction prototype.
//!
//! This is deliberately not the Weftext Server. It exposes one explicitly
//! selected local workspace, binds only to loopback, requires an unguessable
//! bearer token, and delegates every read, preview, and commit to `weftext-core`.

use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
use weftext_core::{
    AdjacentHeadingBody, AnnotationAction, AnnotationAppearance, AnnotationColor, AnnotationKind,
    AnnotationMark, AnnotationResourceMediaKind, AnnotationResourceRegion, AnnotationTargetIntent,
    CURRENT_WORKSPACE_DOCUMENT_FORMAT, CalendarDate, ChildSort, ChronoPeriod, CitationAccessScope,
    CitationEditTarget, CitationMacroIntent, CitationPresentationProfile,
    CitationPresentationRequest, CitationWorkspaceIndex, DocumentEdit, DocumentError,
    DocumentFormatCommand, DocumentProfileId, DocumentRevision, NodeId, QueryAccessScope,
    QueryEvaluationContext, QueryWorkspaceIndex, ResourceImportPlan, SortDirection, SortMode,
    TaskDependencyTransactionPlan, TaskEditIntent, TaskEditTarget, TaskEditTransactionPlan, TaskId,
    TaskRecurrenceCompletionContext, TaskRecurrenceTransactionPlan, TaskWorkspaceIndex,
    TrashItemId, TrashResourceSelection, TrashRestoreMode, WorkspaceRevision,
    WorkspaceTargetResolution, WorkspaceTransactionPlan, analyze_citation_authoring_source,
    analyze_document_for_profile, analyze_document_header_properties,
    bind_workspace_transaction_target_resolution, build_workspace_link_index,
    citation_presentation_capabilities, commit_document_edit, commit_import_resource,
    commit_task_dependency_transaction, commit_task_edit_transaction,
    commit_task_recurrence_transaction, commit_workspace_transaction,
    confirm_permanent_delete_trash_items, patch_document_header_property,
    plan_adjacent_heading_body_setting, plan_citation_macro_edit, plan_copy_node,
    plan_create_child_node, plan_document_edit, plan_document_format, plan_move_node,
    plan_node_aliases_setting, plan_node_child_sort_setting, plan_node_icon_setting,
    plan_node_sibling_rank_setting, plan_permanently_delete_trash_items, plan_rename_node,
    plan_restore_node, plan_restore_trash_item, plan_task_dependency_transaction,
    plan_task_edit_transaction, plan_task_recurrence_transaction, plan_trash_node,
    plan_trash_node_at, plan_trash_resources_at, present_citations,
    preview_permanent_delete_trash_items, project_node_metadata, project_workspace_trash_state,
    read_node_document, read_workspace_revision, recover_workspace_transactions,
    refresh_workspace_search_index, refresh_workspace_search_index_invalidating,
    resolve_node_icon_from_source, scan_workspace, search_workspace_index,
    workspace_document_format,
};

const SCHEMA: &str = "weftext.prototype.bridge.v1";
const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_PENDING_WORKSPACE_PLANS: usize = 64;
const MAX_PENDING_TASK_PLANS: usize = 64;
const HOSTED_PROTOTYPE_ORIGIN: &str = "https://weftext-webui-prototype.zhengyx91.chatgpt.site";

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditRequest {
    #[serde(default)]
    node_id: Option<String>,
    revision: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct ParseRequest {
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormatRequest {
    source: String,
    start: u64,
    end: u64,
    command: DocumentFormatCommand,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PropertyPatchRequest {
    source: String,
    key: String,
    value: Option<String>,
    node_id: Option<String>,
    revision: Option<String>,
    #[serde(default)]
    remove: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceActionRequest {
    action: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    resolved_by: Option<WorkspaceTargetResolution>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct NodeMetadataPreviewRequest {
    action: String,
    node_id: String,
    revision: String,
    icon: Option<String>,
    aliases: Option<Vec<String>>,
    mode: Option<SortMode>,
    direction: Option<SortDirection>,
    sibling_rank: Option<u64>,
    #[serde(default)]
    remove: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionCommitRequest {
    plan_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TrashNodePreviewRequest {
    node_id: String,
    base_workspace_revision: String,
    trashed_at: String,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TrashResourcesPreviewRequest {
    base_workspace_revision: String,
    trashed_at: String,
    resources: Vec<TrashResourceSelection>,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum TrashRestorePreviewRequest {
    Original {
        #[serde(rename = "trashItemId")]
        trash_item_id: String,
        #[serde(rename = "baseWorkspaceRevision")]
        base_workspace_revision: String,
        #[serde(default = "caller_explicit_target_resolution", rename = "resolvedBy")]
        resolved_by: WorkspaceTargetResolution,
    },
    WithAncestors {
        #[serde(rename = "trashItemId")]
        trash_item_id: String,
        #[serde(rename = "baseWorkspaceRevision")]
        base_workspace_revision: String,
        #[serde(default = "caller_explicit_target_resolution", rename = "resolvedBy")]
        resolved_by: WorkspaceTargetResolution,
    },
    ExistingTarget {
        #[serde(rename = "trashItemId")]
        trash_item_id: String,
        #[serde(rename = "baseWorkspaceRevision")]
        base_workspace_revision: String,
        #[serde(rename = "targetNodeId")]
        target_node_id: String,
        name: String,
        #[serde(default = "caller_explicit_target_resolution", rename = "resolvedBy")]
        resolved_by: WorkspaceTargetResolution,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TrashPermanentDeleteEvidence {
    trash_item_id: TrashItemId,
    payload_sha256: String,
    payload_byte_length: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TrashPermanentDeletePreviewRequest {
    base_workspace_revision: String,
    items: Vec<TrashPermanentDeleteEvidence>,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

const fn caller_explicit_target_resolution() -> WorkspaceTargetResolution {
    WorkspaceTargetResolution::CallerExplicit
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourcePreviewRequest {
    node_id: String,
    name: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnnotationPreviewRequest {
    action: String,
    node_id: String,
    annotation_id: Option<String>,
    message_id: Option<String>,
    kind: Option<AnnotationKind>,
    target: Option<AnnotationTargetRequest>,
    appearance: Option<AnnotationAppearanceRequest>,
    body_source: Option<String>,
    suggested_source: Option<String>,
    labels: Option<Vec<String>>,
    author_id: Option<String>,
    author_name: Option<String>,
    timestamp: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnnotationAppearanceRequest {
    mark: AnnotationMark,
    theme: Option<AnnotationColor>,
}

#[derive(Debug, Deserialize)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum AnnotationTargetRequest {
    Document {},
    TextRange {
        start: u64,
        end: u64,
    },
    InsertionPoint {
        position: u64,
    },
    BlockAt {
        source_offset: u64,
    },
    ResourceRegion {
        resource_locator: String,
        resource_digest: String,
        media_kind: AnnotationResourceMediaKind,
        region: AnnotationResourceRegionRequest,
    },
}

#[derive(Debug, Deserialize)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum AnnotationResourceRegionRequest {
    Rect {
        page: Option<u32>,
        x_millionths: u32,
        y_millionths: u32,
        width_millionths: u32,
        height_millionths: u32,
    },
    TimeRange {
        start_milliseconds: u64,
        end_milliseconds: u64,
    },
}

impl From<AnnotationTargetRequest> for AnnotationTargetIntent {
    fn from(value: AnnotationTargetRequest) -> Self {
        match value {
            AnnotationTargetRequest::Document {} => Self::Document,
            AnnotationTargetRequest::TextRange { start, end } => Self::TextRange { start, end },
            AnnotationTargetRequest::InsertionPoint { position } => {
                Self::InsertionPoint { position }
            }
            AnnotationTargetRequest::BlockAt { source_offset } => Self::BlockAt { source_offset },
            AnnotationTargetRequest::ResourceRegion {
                resource_locator,
                resource_digest,
                media_kind,
                region,
            } => Self::ResourceRegion {
                resource_locator,
                resource_digest,
                media_kind,
                region: region.into(),
            },
        }
    }
}

impl From<AnnotationResourceRegionRequest> for AnnotationResourceRegion {
    fn from(value: AnnotationResourceRegionRequest) -> Self {
        match value {
            AnnotationResourceRegionRequest::Rect {
                page,
                x_millionths,
                y_millionths,
                width_millionths,
                height_millionths,
            } => Self::Rect {
                page,
                x_millionths,
                y_millionths,
                width_millionths,
                height_millionths,
            },
            AnnotationResourceRegionRequest::TimeRange {
                start_milliseconds,
                end_milliseconds,
            } => Self::TimeRange {
                start_milliseconds,
                end_milliseconds,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChronoPreviewRequest {
    chrono_root_id: String,
    year: i32,
    month: u8,
    day: u8,
    periods: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CitationAnalyzeRequest {
    node_id: String,
    source: String,
    style_id: String,
    locale: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CitationMacroEditRequest {
    node_id: String,
    source: String,
    target: CitationEditTarget,
    intent: CitationMacroIntent,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskEditPreviewRequest {
    node_id: String,
    base_workspace_revision: String,
    base_revision: String,
    target: TaskEditTarget,
    intent: TaskEditIntent,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskRecurrencePreviewRequest {
    node_id: String,
    base_workspace_revision: String,
    base_revision: String,
    target: TaskEditTarget,
    context: TaskRecurrenceCompletionContext,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskDependenciesPreviewRequest {
    node_id: String,
    base_workspace_revision: String,
    base_revision: String,
    target: TaskEditTarget,
    dependencies: Vec<TaskId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QueryExecuteRequest {
    source: String,
    block_index: usize,
    context: QueryEvaluationContext,
}

enum TaskWorkspacePlan {
    Edit(Box<TaskEditTransactionPlan>),
    Recurrence(Box<TaskRecurrenceTransactionPlan>),
    Dependencies(Box<TaskDependencyTransactionPlan>),
}

struct BridgeState {
    workspace_root: PathBuf,
    search_index_path: PathBuf,
    search_index: Value,
    search_index_warning: Value,
    plans: BTreeMap<String, WorkspaceTransactionPlan>,
    resource_plans: BTreeMap<String, ResourceImportPlan>,
    task_plans: BTreeMap<String, TaskWorkspacePlan>,
}

pub(crate) fn serve(workspace_directory: &str, port: u16) -> Result<Value, String> {
    let workspace_root = PathBuf::from(workspace_directory);
    recover_workspace_transactions(&workspace_root).map_err(|error| error.to_string())?;
    let snapshot = read_node_document(&workspace_root).map_err(|error| error.to_string())?;
    let inventory = scan_workspace(&workspace_root);
    project_workspace_trash_state(&workspace_root)
        .map_err(|_| "prototype bridge requires a valid Weftext workspace root".to_owned())?;
    weftext_core::build_workspace_navigation(&inventory)
        .map_err(|_| "prototype bridge requires a valid Weftext workspace root".to_owned())?;
    let root_id = inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id)
        .ok_or("prototype bridge requires a root node identity")?;
    let search_index_path = std::env::temp_dir()
        .join("weftext-search-indexes")
        .join(format!("{root_id}.json"));
    let (search_index, search_index_warning) = derived_index_open_result(
        refresh_workspace_search_index(&workspace_root, &search_index_path),
    );
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
        .map_err(|error| format!("prototype bridge could not bind to loopback: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("prototype bridge could not inspect its address: {error}"))?;
    let token = Uuid::new_v4().simple().to_string();
    let endpoint = format!("http://127.0.0.1:{}", address.port());
    let open_url = format!("{HOSTED_PROTOTYPE_ORIGIN}/#core={endpoint}&token={token}");
    println!(
        "{}",
        json!({
            "schema": SCHEMA,
            "ok": true,
            "state": "ready",
            "nodeId": snapshot.node_id,
            "nodeName": node_name(&workspace_root),
            "endpoint": endpoint,
            "openUrl": open_url,
            "scope": "workspace",
            "server": false,
            "searchIndex": search_index.clone(),
            "searchIndexWarning": search_index_warning.clone(),
        })
    );
    std::io::stdout()
        .flush()
        .map_err(|error| format!("prototype bridge could not flush readiness: {error}"))?;

    let mut state = BridgeState {
        workspace_root,
        search_index_path,
        search_index,
        search_index_warning,
        plans: BTreeMap::new(),
        resource_plans: BTreeMap::new(),
        task_plans: BTreeMap::new(),
    };
    for incoming in listener.incoming() {
        match incoming {
            Ok(mut stream) => {
                if let Err(error) = handle_connection(&mut stream, &mut state, &token) {
                    let _ = write_response(
                        &mut stream,
                        400,
                        None,
                        json!({"schema": SCHEMA, "ok": false, "error": error}),
                    );
                }
            }
            Err(error) => return Err(format!("prototype bridge connection failed: {error}")),
        }
    }
    Err("prototype bridge stopped unexpectedly".to_owned())
}

fn handle_connection(
    stream: &mut TcpStream,
    state: &mut BridgeState,
    token: &str,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    let request = read_request(stream)?;
    let origin = request.headers.get("origin").map(String::as_str);

    if request.method == "OPTIONS" {
        let allowed_origin = require_allowed_origin(
            origin.ok_or("prototype bridge browser requests require an Origin")?,
        )?;
        return write_empty_response(stream, 204, Some(allowed_origin));
    }

    let allowed_origin = origin.map(require_allowed_origin).transpose()?;
    if require_authorization(&request, token).is_err() {
        return write_response(
            stream,
            401,
            allowed_origin,
            json!({"schema": SCHEMA, "ok": false, "error": "prototype bridge authorization failed"}),
        );
    }
    handle_authorized_request(stream, state, allowed_origin, &request)
}

#[allow(clippy::too_many_lines)]
fn handle_authorized_request(
    stream: &mut TcpStream,
    state: &mut BridgeState,
    allowed_origin: Option<&str>,
    request: &Request,
) -> Result<(), String> {
    let route = request.path.split('?').next().unwrap_or(&request.path);
    if request.method == "POST" && route.ends_with("/commit") {
        let allow_legacy_migration = route == "/api/workspace/action/commit"
            && serde_json::from_slice::<TransactionCommitRequest>(&request.body)
                .ok()
                .and_then(|commit| state.plans.get(&commit.plan_id))
                .is_some_and(|plan| plan.action == weftext_core::StructuralAction::TrashMigration);
        if let Err(error) =
            require_bridge_workspace_writable(&state.workspace_root, allow_legacy_migration)
        {
            return write_response(
                stream,
                409,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": false, "error": error}),
            );
        }
    }
    match (request.method.as_str(), route) {
        ("GET", "/api/workspace") => write_response(
            stream,
            200,
            allowed_origin,
            workspace_response(
                &state.workspace_root,
                &state.search_index,
                &state.search_index_warning,
            )?,
        ),
        ("GET", "/api/document") => {
            let node_directory = requested_node_directory(&state.workspace_root, &request.path)?;
            let snapshot =
                read_node_document(&node_directory).map_err(|error| error.to_string())?;
            let analysis = analyze_document_for_profile(
                snapshot.profile,
                &snapshot.source,
                workspace_presentation(&state.workspace_root),
            );
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "document": {
                        "nodeId": snapshot.node_id,
                        "name": node_name(&node_directory),
                        "revision": snapshot.revision,
                        "length": snapshot.source.len(),
                        "source": snapshot.source,
                        "profile": analysis.descriptor,
                        "model": analysis.model,
                        "view": analysis.view,
                        "metadata": project_node_metadata(
                            &snapshot.source,
                            if node_directory == state.workspace_root {
                                weftext_core::NodeMetadataScope::WorkspaceRoot
                            } else {
                                weftext_core::NodeMetadataScope::Node
                            },
                        ).map_err(|error| error.to_string())?,
                        "properties": analyze_document_header_properties(&snapshot.source),
                    }
                }),
            )
        }
        ("GET", "/api/search") => {
            let query = request_query_value(&request.path, "q").unwrap_or_default();
            let index =
                refresh_workspace_search_index(&state.workspace_root, &state.search_index_path)
                    .map_err(|error| error.to_string())?;
            let results = search_workspace_index(&state.search_index_path, &query)
                .map_err(|error| error.to_string())?;
            state.search_index =
                serde_json::to_value(&index).expect("search index statistics are serializable");
            state.search_index_warning = Value::Null;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "results": results, "index": index}),
            )
        }
        ("POST", "/api/document/model") => {
            let draft: ParseRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid draft parse request: {error}"))?;
            let analysis = analyze_document_for_profile(
                workspace_document_profile(&state.workspace_root)?,
                &draft.source,
                workspace_presentation(&state.workspace_root),
            );
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "profile": analysis.descriptor,
                    "model": analysis.model,
                    "view": analysis.view,
                    "properties": analyze_document_header_properties(&draft.source),
                }),
            )
        }
        ("POST", "/api/document/format") => {
            let format: FormatRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid document format request: {error}"))?;
            let plan = plan_document_format(
                workspace_document_profile(&state.workspace_root)?,
                &format.source,
                format.start,
                format.end,
                format.command,
            )
            .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "plan": plan}),
            )
        }
        ("POST", "/api/document/property") => {
            let patch: PropertyPatchRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid user property patch: {error}"))?;
            let value = (!patch.remove).then_some(patch.value.as_deref().unwrap_or_default());
            let result: Result<String, String> =
                patch_document_header_property(&patch.source, &patch.key, value)
                    .map_err(|error| error.to_string());
            match result {
                Ok(source) => {
                    let validation = match (patch.node_id.as_deref(), patch.revision.as_deref()) {
                        (Some(node_id), Some(revision)) => {
                            let edit = EditRequest {
                                node_id: Some(node_id.to_owned()),
                                revision: revision.to_owned(),
                                source: source.clone(),
                            };
                            let directory = edit_node_directory(&state.workspace_root, &edit)?;
                            let plan =
                                build_plan(&directory, &edit).map_err(|error| error.to_string())?;
                            Some(json!({
                                "baseRevision": plan.base_revision,
                                "nextRevision": plan.next_revision,
                                "changed": plan.changed,
                            }))
                        }
                        (None, None) => None,
                        _ => {
                            return Err(
                                "property patch requires both nodeId and revision".to_owned()
                            );
                        }
                    };
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({"schema": SCHEMA, "ok": true, "source": source, "validation": validation}),
                    )
                }
                Err(error) => write_response(
                    stream,
                    422,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error}),
                ),
            }
        }
        ("GET", "/api/citation/capabilities") => write_response(
            stream,
            200,
            allowed_origin,
            json!({
                "schema": SCHEMA,
                "ok": true,
                "capabilities": citation_presentation_capabilities(),
            }),
        ),
        ("GET", "/api/citation/validate") => {
            let index = citation_index(&state.workspace_root)?;
            let access = local_citation_scope(&index);
            let inventory = scan_workspace(&state.workspace_root);
            let mut components = Vec::with_capacity(inventory.nodes.len());
            let mut component_diagnostic_count = 0_usize;
            for node in &inventory.nodes {
                let node_id = node.id.ok_or("workspace node has no identity")?;
                let analysis = index
                    .analyze_component(node_id, &access)
                    .map_err(|error| error.to_string())?;
                component_diagnostic_count += analysis.diagnostics.len();
                components.push(analysis);
            }
            let valid = index.diagnostics().is_empty() && component_diagnostic_count == 0;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "validation": {
                        "valid": valid,
                        "generation": index.generation(),
                        "referenceDiagnostics": index.diagnostics(),
                        "components": components,
                    },
                }),
            )
        }
        ("GET", "/api/citation/search") => {
            let query = request_query_value(&request.path, "q").unwrap_or_default();
            let limit =
                request_query_value(&request.path, "limit").map_or(Ok(25_usize), |value| {
                    value
                        .parse::<usize>()
                        .map_err(|_| "citation search limit must be an unsigned integer".to_owned())
                })?;
            let index = citation_index(&state.workspace_root)?;
            let references = index
                .search_references(&query, &local_citation_scope(&index), limit)
                .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "query": query, "references": references}),
            )
        }
        ("POST", "/api/citation/analyze") => {
            let draft: CitationAnalyzeRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid citation draft request: {error}"))?;
            let node_id = draft
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            let index = citation_index(&state.workspace_root)?;
            let access = local_citation_scope(&index);
            let analysis = index
                .analyze_component_source(node_id, &draft.source, &access)
                .map_err(|error| error.to_string())?;
            let compilation = index
                .collect_bibliography_input_for_source(node_id, &draft.source, &access)
                .map_err(|error| error.to_string())?;
            let presentation = present_citations(&CitationPresentationRequest::new(
                CitationPresentationProfile::new(draft.style_id, draft.locale),
                compilation,
            ));
            let (presentation, presentation_failure) = match presentation {
                Ok(presentation) => (Some(presentation), None),
                Err(failure) => (None, Some(failure)),
            };
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "authoring": analyze_citation_authoring_source(&draft.source),
                    "analysis": analysis,
                    "presentation": presentation,
                    "presentationFailure": presentation_failure,
                }),
            )
        }
        ("POST", "/api/citation/macro-edit-preview") => {
            let preview: CitationMacroEditRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid citation edit preview: {error}"))?;
            let node_id = preview
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            let index = citation_index(&state.workspace_root)?;
            let plan = plan_citation_macro_edit(
                &index,
                node_id,
                &preview.source,
                &local_citation_scope(&index),
                &preview.target,
                &preview.intent,
            )
            .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "plan": plan}),
            )
        }
        ("POST", "/api/citation/recover" | "/api/citation/rollback") => {
            let recovery = recover_workspace_transactions(&state.workspace_root)
                .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "operation": if route.ends_with("rollback") { "rollback" } else { "recover" },
                    "recovery": recovery,
                    "workspace": workspace_payload(&state.workspace_root)?,
                }),
            )
        }
        ("POST", "/api/query/execute") => {
            let query: QueryExecuteRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid query execution request: {error}"))?;
            let index = QueryWorkspaceIndex::rebuild(&state.workspace_root)
                .map_err(|error| error.to_string())?;
            let access = QueryAccessScope::complete(index.node_ids());
            let execution = index
                .execute_source(&query.source, query.block_index, &access, &query.context)
                .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "valid": execution.result.is_some(),
                    "execution": execution,
                }),
            )
        }
        ("GET", "/api/task/validate") => {
            let index = TaskWorkspaceIndex::rebuild(&state.workspace_root)
                .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "validation": {
                        "valid": index.diagnostics().is_empty(),
                        "generation": index.generation(),
                        "occurrences": index.occurrences(),
                        "diagnostics": index.diagnostics(),
                    },
                }),
            )
        }
        ("GET", "/api/task/inspect") => {
            let node_id = request_query_value(&request.path, "nodeId")
                .ok_or("task inspection requires nodeId")?
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            node_directory_for_id(&state.workspace_root, &node_id.to_string())?;
            let index = TaskWorkspaceIndex::rebuild(&state.workspace_root)
                .map_err(|error| error.to_string())?;
            let occurrences = index.occurrences_for_node(node_id).collect::<Vec<_>>();
            let diagnostics = index
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.node_id == node_id)
                .collect::<Vec<_>>();
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "nodeId": node_id,
                    "occurrences": occurrences,
                    "diagnostics": diagnostics,
                }),
            )
        }
        ("POST", "/api/task/edit-preview") => {
            let preview: TaskEditPreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid task edit preview: {error}"))?;
            require_current_workspace_revision(
                &state.workspace_root,
                &preview.base_workspace_revision,
            )?;
            let node_id = preview
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            node_directory_for_id(&state.workspace_root, &preview.node_id)?;
            let base_revision = DocumentRevision::parse(&preview.base_revision)
                .map_err(|error| error.to_string())?;
            let plan = plan_task_edit_transaction(
                &state.workspace_root,
                node_id,
                &base_revision,
                &preview.target,
                &preview.intent,
            )
            .map_err(|error| error.to_string())?;
            let plan_id = plan.workspace_transaction().plan_id.clone();
            let payload = json!({
                "planId": plan_id,
                "kind": "edit",
                "baseWorkspaceRevision": plan.workspace_transaction().base_revision,
                "nodeId": node_id,
                "authoring": &plan.authoring,
                "documentChanges": plan.workspace_transaction().document_changes,
            });
            stage_task_plan(state, plan_id, TaskWorkspacePlan::Edit(Box::new(plan)))?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "plan": payload}),
            )
        }
        ("POST", "/api/task/recurrence-preview") => {
            let preview: TaskRecurrencePreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid task recurrence preview: {error}"))?;
            require_current_workspace_revision(
                &state.workspace_root,
                &preview.base_workspace_revision,
            )?;
            let node_id = preview
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            node_directory_for_id(&state.workspace_root, &preview.node_id)?;
            let base_revision = DocumentRevision::parse(&preview.base_revision)
                .map_err(|error| error.to_string())?;
            let plan = plan_task_recurrence_transaction(
                &state.workspace_root,
                node_id,
                &base_revision,
                &preview.target,
                &preview.context,
            )
            .map_err(|error| error.to_string())?;
            let plan_id = plan.workspace_transaction().plan_id.clone();
            let payload = json!({
                "planId": plan_id,
                "kind": "recurrence",
                "baseWorkspaceRevision": plan.workspace_transaction().base_revision,
                "nodeId": node_id,
                "completion": &plan.completion,
                "documentChanges": plan.workspace_transaction().document_changes,
            });
            stage_task_plan(
                state,
                plan_id,
                TaskWorkspacePlan::Recurrence(Box::new(plan)),
            )?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "plan": payload}),
            )
        }
        ("POST", "/api/task/dependencies-preview") => {
            let preview: TaskDependenciesPreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid task dependency preview: {error}"))?;
            require_current_workspace_revision(
                &state.workspace_root,
                &preview.base_workspace_revision,
            )?;
            let node_id = preview
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            node_directory_for_id(&state.workspace_root, &preview.node_id)?;
            let base_revision = DocumentRevision::parse(&preview.base_revision)
                .map_err(|error| error.to_string())?;
            let plan = plan_task_dependency_transaction(
                &state.workspace_root,
                node_id,
                &base_revision,
                &preview.target,
                &preview.dependencies,
            )
            .map_err(|error| error.to_string())?;
            let plan_id = plan.workspace_transaction().plan_id.clone();
            let payload = json!({
                "planId": plan_id,
                "kind": "dependencies",
                "baseWorkspaceRevision": plan.workspace_transaction().base_revision,
                "nodeId": node_id,
                "dependencies": &plan.dependencies,
                "authoring": &plan.authoring,
                "documentChanges": plan.workspace_transaction().document_changes,
            });
            stage_task_plan(
                state,
                plan_id,
                TaskWorkspacePlan::Dependencies(Box::new(plan)),
            )?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "plan": payload}),
            )
        }
        ("POST", "/api/task/transaction/commit") => {
            let commit: TransactionCommitRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid task transaction commit: {error}"))?;
            let plan = state
                .task_plans
                .remove(&commit.plan_id)
                .ok_or("task transaction preview has expired")?;
            let (committed, node_id, result) = match plan {
                TaskWorkspacePlan::Edit(plan) => {
                    let node_id = plan.node_id;
                    let result = json!({
                        "task": &plan.authoring.target,
                        "assignedId": plan.authoring.assigned_id,
                    });
                    let committed =
                        commit_task_edit_transaction(&plan).map_err(|error| error.to_string())?;
                    (committed, node_id, result)
                }
                TaskWorkspacePlan::Recurrence(plan) => {
                    let node_id = plan.node_id;
                    let result = json!({
                        "completedTask": &plan.completion.completed_task,
                        "nextTask": &plan.completion.next_task,
                        "nextTaskId": plan.completion.next_task_id,
                        "stopped": plan.completion.stopped,
                    });
                    let committed = commit_task_recurrence_transaction(&plan)
                        .map_err(|error| error.to_string())?;
                    (committed, node_id, result)
                }
                TaskWorkspacePlan::Dependencies(plan) => {
                    let node_id = plan.node_id;
                    let result = json!({
                        "task": &plan.authoring.target,
                        "assignedId": plan.authoring.assigned_id,
                        "dependencies": &plan.dependencies,
                    });
                    let committed = commit_task_dependency_transaction(&plan)
                        .map_err(|error| error.to_string())?;
                    (committed, node_id, result)
                }
            };
            let (index, warning) =
                derived_index_result(refresh_workspace_search_index_invalidating(
                    &state.workspace_root,
                    &state.search_index_path,
                    std::iter::once(node_id),
                ));
            state.search_index = index;
            state.search_index_warning = warning;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "nodeId": node_id,
                    "commit": committed,
                    "result": result,
                    "workspace": workspace_payload(&state.workspace_root)?,
                    "searchIndex": state.search_index.clone(),
                    "searchIndexWarning": state.search_index_warning.clone(),
                }),
            )
        }
        ("POST", "/api/task/recover") => {
            let recovery = recover_workspace_transactions(&state.workspace_root)
                .map_err(|error| error.to_string())?;
            state.task_plans.clear();
            write_response(
                stream,
                200,
                allowed_origin,
                json!({
                    "schema": SCHEMA,
                    "ok": true,
                    "recovery": recovery,
                    "workspace": workspace_payload(&state.workspace_root)?,
                }),
            )
        }
        ("GET", "/api/annotations") => {
            let node_id = request_query_value(&request.path, "nodeId")
                .ok_or("annotation read requires nodeId")?
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            let annotations = weftext_core::read_node_annotations(
                &state.workspace_root,
                node_id,
                weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
            )
            .map_err(|error| error.to_string())?;
            write_response(
                stream,
                200,
                allowed_origin,
                json!({"schema": SCHEMA, "ok": true, "annotations": annotations}),
            )
        }
        ("POST", "/api/annotation/preview") => {
            let preview: AnnotationPreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid annotation preview: {error}"))?;
            let node_id = preview
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            let sidecar_snapshot = weftext_core::capture_annotation_sidecar_snapshot(
                &state.workspace_root,
                node_id,
                weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
            )
            .map_err(|error| error.to_string())?;
            match weftext_core::plan_annotation_action(
                &state.workspace_root,
                &sidecar_snapshot,
                annotation_action(preview)?,
            ) {
                Ok(plan) => {
                    let payload = json!({
                        "planId": plan.plan_id,
                        "baseRevision": plan.base_revision,
                        "action": "annotation",
                    });
                    state.plans.insert(plan.plan_id.clone(), plan);
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({"schema": SCHEMA, "ok": true, "plan": payload}),
                    )
                }
                Err(error) => write_response(
                    stream,
                    422,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("POST", "/api/annotation/commit") => {
            let commit: TransactionCommitRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid annotation commit: {error}"))?;
            let Some(plan) = state.plans.remove(&commit.plan_id) else {
                return write_response(
                    stream,
                    409,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": "annotation preview is unavailable; retry"}),
                );
            };
            let node_id = request_query_value(&request.path, "nodeId")
                .ok_or("annotation commit requires nodeId")?
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            match commit_workspace_transaction(&plan) {
                Ok(committed) => {
                    let annotations = weftext_core::read_node_annotations(
                        &state.workspace_root,
                        node_id,
                        weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
                    )
                    .map_err(|error| error.to_string())?;
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({
                            "schema": SCHEMA,
                            "ok": true,
                            "commit": committed,
                            "annotations": annotations,
                            "workspace": workspace_payload(&state.workspace_root)?,
                        }),
                    )
                }
                Err(error) => write_response(
                    stream,
                    409,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("POST", "/api/chrono/preview") => {
            let preview: ChronoPreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid Chrono preview: {error}"))?;
            let date = CalendarDate::new(preview.year, preview.month, preview.day)
                .map_err(|error| error.to_string())?;
            let periods = parse_chrono_periods(&preview.periods)?;
            let chrono_root_id = preview
                .chrono_root_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            match weftext_core::plan_chrono_nodes(
                &state.workspace_root,
                chrono_root_id,
                date,
                &periods,
            ) {
                Ok(plan) => {
                    let response = transaction_plan_json(&plan);
                    state.plans.insert(plan.plan_id.clone(), plan);
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({"schema": SCHEMA, "ok": true, "plan": response}),
                    )
                }
                Err(error) => write_response(
                    stream,
                    422,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("POST", "/api/document/preview") => {
            let edit = parse_edit_request(&request.body)?;
            let node_directory = edit_node_directory(&state.workspace_root, &edit)?;
            match build_plan(&node_directory, &edit) {
                Ok(plan) => write_response(
                    stream,
                    200,
                    allowed_origin,
                    json!({
                        "schema": SCHEMA,
                        "ok": true,
                        "plan": {
                            "action": "document.edit",
                            "nodeId": plan.node_id,
                            "baseRevision": plan.base_revision,
                            "nextRevision": plan.next_revision,
                            "oldLength": plan.old_length,
                            "newLength": plan.new_length,
                            "changed": plan.changed,
                        }
                    }),
                ),
                Err(error) => write_response(
                    stream,
                    error_status(&error),
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("POST", "/api/document/commit") => {
            let edit = parse_edit_request(&request.body)?;
            let node_directory = edit_node_directory(&state.workspace_root, &edit)?;
            let resolved_icon = resolve_node_icon_from_source(&edit.source);
            match build_plan(&node_directory, &edit).and_then(|plan| {
                let base_revision = plan.base_revision.clone();
                commit_document_edit(&plan).map(|committed| (base_revision, committed))
            }) {
                Ok((base_revision, committed)) => {
                    let (index, index_warning) =
                        derived_index_result(refresh_workspace_search_index_invalidating(
                            &state.workspace_root,
                            &state.search_index_path,
                            std::iter::once(committed.node_id),
                        ));
                    state.search_index = index.clone();
                    state.search_index_warning = index_warning.clone();
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({
                            "schema": SCHEMA,
                            "ok": true,
                            "commit": {
                                "action": "document.edit",
                                "nodeId": committed.node_id,
                                "baseRevision": base_revision,
                                "revision": committed.revision,
                                "length": committed.length,
                            },
                            "searchIndex": index,
                            "searchIndexWarning": index_warning,
                            "icon": resolved_icon,
                        }),
                    )
                }
                Err(error) => write_response(
                    stream,
                    error_status(&error),
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("GET", "/api/trash") => write_response(
            stream,
            200,
            allowed_origin,
            json!({
                "schema": SCHEMA,
                "ok": true,
                "trash": trash_inventory_payload(&state.workspace_root)?,
            }),
        ),
        ("POST", "/api/trash/node/preview") => {
            let result = (|| {
                let request: TrashNodePreviewRequest = serde_json::from_slice(&request.body)
                    .map_err(|error| format!("invalid node Trash preview: {error}"))?;
                require_workspace_base_revision(
                    &state.workspace_root,
                    &request.base_workspace_revision,
                )?;
                let mut plan = plan_trash_node_at(
                    &state.workspace_root,
                    parse_action_id(Some(&request.node_id), "nodeId")?,
                    &request.trashed_at,
                )
                .map_err(|error| error.to_string())?;
                bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
                    .map_err(|error| error.to_string())?;
                stage_workspace_plan(state, plan)
            })();
            write_workspace_plan_preview(stream, allowed_origin, result)
        }
        ("POST", "/api/trash/resources/preview") => {
            let result = (|| {
                let request: TrashResourcesPreviewRequest =
                    serde_json::from_slice(&request.body)
                        .map_err(|error| format!("invalid resource Trash preview: {error}"))?;
                require_workspace_base_revision(
                    &state.workspace_root,
                    &request.base_workspace_revision,
                )?;
                let mut plan = plan_trash_resources_at(
                    &state.workspace_root,
                    request.resources,
                    &request.trashed_at,
                )
                .map_err(|error| error.to_string())?;
                if plan.captured_target.is_some() {
                    bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
                        .map_err(|error| error.to_string())?;
                }
                stage_workspace_plan(state, plan)
            })();
            write_workspace_plan_preview(stream, allowed_origin, result)
        }
        ("POST", "/api/trash/restore/preview") => {
            let result = (|| {
                let request: TrashRestorePreviewRequest = serde_json::from_slice(&request.body)
                    .map_err(|error| format!("invalid Trash restore preview: {error}"))?;
                let (item_id, base_revision, mode, resolved_by) = match request {
                    TrashRestorePreviewRequest::Original {
                        trash_item_id,
                        base_workspace_revision,
                        resolved_by,
                    } => (
                        parse_trash_item_id(&trash_item_id)?,
                        base_workspace_revision,
                        TrashRestoreMode::Original,
                        resolved_by,
                    ),
                    TrashRestorePreviewRequest::WithAncestors {
                        trash_item_id,
                        base_workspace_revision,
                        resolved_by,
                    } => (
                        parse_trash_item_id(&trash_item_id)?,
                        base_workspace_revision,
                        TrashRestoreMode::WithAncestors,
                        resolved_by,
                    ),
                    TrashRestorePreviewRequest::ExistingTarget {
                        trash_item_id,
                        base_workspace_revision,
                        target_node_id,
                        name,
                        resolved_by,
                    } => (
                        parse_trash_item_id(&trash_item_id)?,
                        base_workspace_revision,
                        TrashRestoreMode::ExistingTarget {
                            target_node_id: parse_action_id(Some(&target_node_id), "targetNodeId")?,
                            name,
                        },
                        resolved_by,
                    ),
                };
                require_workspace_base_revision(&state.workspace_root, &base_revision)?;
                let mut plan = plan_restore_trash_item(&state.workspace_root, item_id, mode)
                    .map_err(|error| error.to_string())?;
                bind_workspace_transaction_target_resolution(&mut plan, resolved_by)
                    .map_err(|error| error.to_string())?;
                stage_workspace_plan(state, plan)
            })();
            write_workspace_plan_preview(stream, allowed_origin, result)
        }
        ("POST", "/api/trash/permanent-delete/preview") => {
            let result = (|| {
                let request: TrashPermanentDeletePreviewRequest =
                    serde_json::from_slice(&request.body)
                        .map_err(|error| format!("invalid permanent-delete preview: {error}"))?;
                require_workspace_base_revision(
                    &state.workspace_root,
                    &request.base_workspace_revision,
                )?;
                let preview = preview_permanent_delete_trash_items(
                    &state.workspace_root,
                    request
                        .items
                        .iter()
                        .map(|item| item.trash_item_id)
                        .collect(),
                )
                .map_err(|error| error.to_string())?;
                require_exact_permanent_delete_evidence(&preview, request.items)?;
                let confirmation = confirm_permanent_delete_trash_items(
                    preview,
                    true,
                    weftext_core::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE,
                )
                .map_err(|error| error.to_string())?;
                let mut plan =
                    plan_permanently_delete_trash_items(&state.workspace_root, &confirmation)
                        .map_err(|error| error.to_string())?;
                if plan.captured_target.is_some() {
                    bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
                        .map_err(|error| error.to_string())?;
                }
                stage_workspace_plan(state, plan)
            })();
            write_workspace_plan_preview(stream, allowed_origin, result)
        }
        ("POST", "/api/trash/migrate-legacy/preview") => write_response(
            stream,
            422,
            allowed_origin,
            json!({
                "schema": SCHEMA,
                "ok": false,
                "error": "the prototype bridge does not accept host snapshot paths; migrate with the Desktop directory capability or the CLI"
            }),
        ),
        ("POST", "/api/workspace/action/preview") => {
            let action: WorkspaceActionRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid workspace action: {error}"))?;
            match build_workspace_plan(&state.workspace_root, &action).and_then(|mut plan| {
                if let Some(resolution) = action.resolved_by {
                    bind_workspace_transaction_target_resolution(&mut plan, resolution)
                        .map_err(|error| error.to_string())?;
                }
                Ok(plan)
            }) {
                Ok(plan) => {
                    let response = stage_workspace_plan(state, plan);
                    write_workspace_plan_preview(stream, allowed_origin, response)
                }
                Err(error) => write_response(
                    stream,
                    422,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error}),
                ),
            }
        }
        ("POST", "/api/node/metadata/preview") => {
            let request: NodeMetadataPreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid node metadata preview: {error}"))?;
            match build_node_metadata_plan(&state.workspace_root, request) {
                Ok(plan) => {
                    let response = transaction_plan_json(&plan);
                    state.plans.insert(plan.plan_id.clone(), plan);
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({"schema": SCHEMA, "ok": true, "plan": response}),
                    )
                }
                Err(error) => write_response(
                    stream,
                    422,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error}),
                ),
            }
        }
        ("POST", "/api/workspace/action/commit") => {
            let commit: TransactionCommitRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid workspace commit: {error}"))?;
            let Some(plan) = state.plans.remove(&commit.plan_id) else {
                return write_response(
                    stream,
                    409,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": "workspace plan is unavailable; preview again"}),
                );
            };
            let invalidated = plan
                .document_changes
                .iter()
                .map(|change| change.node_id)
                .chain(plan.generated_node_ids.iter().copied())
                .collect::<Vec<_>>();
            match commit_workspace_transaction(&plan) {
                Ok(committed) => {
                    let (index, index_warning) =
                        derived_index_result(refresh_workspace_search_index_invalidating(
                            &state.workspace_root,
                            &state.search_index_path,
                            invalidated,
                        ));
                    state.search_index = index.clone();
                    state.search_index_warning = index_warning.clone();
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({
                            "schema": SCHEMA,
                            "ok": true,
                            "commit": committed,
                            "workspace": workspace_payload(&state.workspace_root)?,
                            "searchIndex": index,
                            "searchIndexWarning": index_warning,
                        }),
                    )
                }
                Err(error) => write_response(
                    stream,
                    409,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("POST", "/api/resource/preview") => {
            let preview: ResourcePreviewRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid resource preview: {error}"))?;
            let node_id = preview
                .node_id
                .parse::<NodeId>()
                .map_err(|error| error.to_string())?;
            match weftext_core::plan_import_resource(
                &state.workspace_root,
                node_id,
                &preview.name,
                preview.bytes,
            ) {
                Ok(plan) => {
                    let payload = json!({
                        "planId": plan.plan_id,
                        "nodeId": plan.node_id,
                        "name": plan.name,
                        "byteLength": plan.byte_length,
                        "baseRevision": plan.base_revision,
                    });
                    state.resource_plans.insert(plan.plan_id.clone(), plan);
                    write_response(
                        stream,
                        200,
                        allowed_origin,
                        json!({"schema": SCHEMA, "ok": true, "plan": payload}),
                    )
                }
                Err(error) => write_response(
                    stream,
                    422,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        ("POST", "/api/resource/commit") => {
            let commit: TransactionCommitRequest = serde_json::from_slice(&request.body)
                .map_err(|error| format!("invalid resource commit: {error}"))?;
            let Some(plan) = state.resource_plans.remove(&commit.plan_id) else {
                return write_response(
                    stream,
                    409,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": "resource preview is unavailable; choose the file again"}),
                );
            };
            match commit_import_resource(plan) {
                Ok(committed) => write_response(
                    stream,
                    200,
                    allowed_origin,
                    json!({
                        "schema": SCHEMA,
                        "ok": true,
                        "resource": {
                            "nodeId": committed.node_id,
                            "name": committed.name,
                            "byteLength": committed.byte_length,
                            "workspaceRevision": committed.workspace_revision,
                        },
                        "workspace": workspace_payload(&state.workspace_root)?,
                    }),
                ),
                Err(error) => write_response(
                    stream,
                    409,
                    allowed_origin,
                    json!({"schema": SCHEMA, "ok": false, "error": error.to_string()}),
                ),
            }
        }
        _ => write_response(
            stream,
            404,
            allowed_origin,
            json!({"schema": SCHEMA, "ok": false, "error": "unknown prototype bridge route"}),
        ),
    }
}

fn build_plan(
    node_directory: &Path,
    edit: &EditRequest,
) -> Result<weftext_core::DocumentEditPlan, DocumentError> {
    let revision = DocumentRevision::parse(&edit.revision)?;
    let snapshot = read_node_document(node_directory)?;
    let end = u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX);
    plan_document_edit(
        node_directory,
        &revision,
        [DocumentEdit {
            start: 0,
            end,
            replacement: edit.source.clone(),
        }],
    )
}

fn workspace_response(
    root: &Path,
    search_index: &Value,
    search_index_warning: &Value,
) -> Result<Value, String> {
    Ok(json!({
        "schema": SCHEMA,
        "ok": true,
        "workspace": workspace_payload(root)?,
        "searchIndex": search_index,
        "searchIndexWarning": search_index_warning,
    }))
}

fn trash_inventory_payload(root: &Path) -> Result<Value, String> {
    let state = project_workspace_trash_state(root).map_err(|error| error.to_string())?;
    Ok(json!({
        "workspaceRevision": read_workspace_revision(root).map_err(|error| error.to_string())?,
        "state": state.state,
        "items": state.items,
        "reconciliation": {
            "required": state.reconciliation_required,
            "issueCount": state.diagnostic_count,
        },
        "legacyMigrationRequired": state.legacy_migration_required,
    }))
}

fn require_bridge_workspace_writable(
    root: &Path,
    allow_legacy_migration: bool,
) -> Result<(), String> {
    let state = project_workspace_trash_state(root).map_err(|error| error.to_string())?;
    if state.reconciliation_required {
        return Err(
            "Workspace Trash requires reconciliation; the workspace is read-only".to_owned(),
        );
    }
    if state.legacy_migration_required && !allow_legacy_migration {
        return Err(
            "legacy Workspace Trash requires explicit migration; the workspace is read-only"
                .to_owned(),
        );
    }
    Ok(())
}

fn workspace_payload(root: &Path) -> Result<Value, String> {
    let inventory = scan_workspace(root);
    let trash_state = project_workspace_trash_state(root).map_err(|error| error.to_string())?;
    let navigation =
        weftext_core::build_workspace_navigation(&inventory).map_err(|error| error.to_string())?;
    let revision = read_workspace_revision(root).map_err(|error| error.to_string())?;
    let degraded = trash_state.reconciliation_required || trash_state.legacy_migration_required;
    let links = match build_workspace_link_index(root) {
        Ok(links) => serde_json::to_value(links).map_err(|error| error.to_string())?,
        Err(_) if degraded => json!({
            "revision": revision,
            "nodes": [],
            "outgoing": [],
            "backlinks": [],
            "potentialMentions": [],
        }),
        Err(error) => return Err(error.to_string()),
    };
    let root_setting = workspace_presentation(root);
    let nodes = navigation
        .hierarchy
        .iter()
        .map(|node| {
            let icon = match &node.display_icon {
                weftext_core::WorkspaceItemIcon::ExplicitNode(icon) => Some(icon.clone()),
                _ => None,
            };
            json!({
                "id": node.node_id,
                "name": node.name,
                "parentId": node.parent_node_id,
                "path": node.locator,
                "icon": icon,
                "displayIcon": node.display_icon,
            })
        })
        .collect::<Vec<_>>();
    let content = navigation
        .contents
        .iter()
        .map(|entry| {
            json!({
                "kind": entry.kind,
                "name": entry.name,
                "path": entry.locator,
                "parentPath": entry.parent_locator,
                "nodeId": entry.node_id,
                "ownerNodeId": entry.owner_node_id,
                "displayIcon": entry.display_icon,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "rootNodeId": inventory.nodes.iter().find(|node| node.parent_id.is_none()).and_then(|node| node.id),
        "revision": revision,
        "documentFormat": weftext_core::workspace_document_format(root),
        "presentation": {
            "adjacentHeadingBody": match root_setting {
                AdjacentHeadingBody::Separate => "separate",
                AdjacentHeadingBody::RunIn => "run_in",
            }
        },
        "nodes": nodes,
        "trashItems": trash_state.items,
        "trashReconciliation": {
            "required": trash_state.reconciliation_required,
            "issueCount": trash_state.diagnostic_count,
        },
        "trashLegacyMigrationRequired": trash_state.legacy_migration_required,
        "content": content,
        "navigation": navigation,
        "links": links,
        "iconCatalog": weftext_core::built_in_node_icons(),
    }))
}

fn citation_index(root: &Path) -> Result<CitationWorkspaceIndex, String> {
    CitationWorkspaceIndex::rebuild(root).map_err(|error| error.to_string())
}

fn local_citation_scope(index: &CitationWorkspaceIndex) -> CitationAccessScope {
    CitationAccessScope::complete(index.reference_node_ids())
}

fn stage_task_plan(
    state: &mut BridgeState,
    plan_id: String,
    plan: TaskWorkspacePlan,
) -> Result<(), String> {
    if state.task_plans.len() >= MAX_PENDING_TASK_PLANS {
        return Err("too many task transaction previews are pending".to_owned());
    }
    state.task_plans.insert(plan_id, plan);
    Ok(())
}

fn require_current_workspace_revision(root: &Path, expected: &str) -> Result<(), String> {
    let expected = WorkspaceRevision::parse(expected).map_err(|error| error.to_string())?;
    let actual = read_workspace_revision(root).map_err(|error| error.to_string())?;
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "workspace changed after task baseline {expected}; current revision is {actual}"
        ))
    }
}

fn workspace_presentation(root: &Path) -> AdjacentHeadingBody {
    scan_workspace(root)
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.metadata)
        .map_or(AdjacentHeadingBody::Separate, |metadata| {
            metadata.presentation.adjacent_heading_body
        })
}

fn workspace_document_profile(root: &Path) -> Result<DocumentProfileId, String> {
    if workspace_document_format(root).generation == CURRENT_WORKSPACE_DOCUMENT_FORMAT.generation {
        Ok(DocumentProfileId::AsciiDocV1)
    } else {
        Err("prototype bridge requires the exact weftext.asciidoc.v1 workspace marker".to_owned())
    }
}

fn requested_node_directory(root: &Path, request_path: &str) -> Result<PathBuf, String> {
    let Some((_, query)) = request_path.split_once('?') else {
        return Ok(root.to_path_buf());
    };
    let node_id = query
        .split('&')
        .find_map(|piece| piece.strip_prefix("nodeId="))
        .ok_or("document request is missing nodeId")?;
    let decoded = percent_decode(node_id)?;
    node_directory_for_id(root, &decoded)
}

fn request_query_value(request_path: &str, name: &str) -> Option<String> {
    let (_, query) = request_path.split_once('?')?;
    query
        .split('&')
        .find_map(|piece| piece.split_once('=').filter(|(key, _)| *key == name))
        .and_then(|(_, value)| percent_decode(value).ok())
}

fn edit_node_directory(root: &Path, edit: &EditRequest) -> Result<PathBuf, String> {
    edit.node_id.as_deref().map_or_else(
        || Ok(root.to_path_buf()),
        |id| node_directory_for_id(root, id),
    )
}

fn node_directory_for_id(root: &Path, value: &str) -> Result<PathBuf, String> {
    let id = value.parse::<NodeId>().map_err(|error| error.to_string())?;
    let inventory = scan_workspace(root);
    inventory
        .nodes
        .iter()
        .find(|node| node.id == Some(id))
        .map(|node| node.path.clone())
        .ok_or_else(|| "workspace node is unavailable".to_owned())
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'%' {
            if cursor + 2 >= bytes.len() {
                return Err("invalid percent encoding".to_owned());
            }
            let hex = std::str::from_utf8(&bytes[cursor + 1..cursor + 3])
                .map_err(|_| "invalid percent encoding".to_owned())?;
            decoded.push(
                u8::from_str_radix(hex, 16).map_err(|_| "invalid percent encoding".to_owned())?,
            );
            cursor += 3;
        } else {
            decoded.push(bytes[cursor]);
            cursor += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| "nodeId is not UTF-8".to_owned())
}

fn build_node_metadata_plan(
    root: &Path,
    request: NodeMetadataPreviewRequest,
) -> Result<WorkspaceTransactionPlan, String> {
    let node_id = parse_action_id(Some(&request.node_id), "nodeId")?;
    let revision = DocumentRevision::parse(&request.revision).map_err(|error| error.to_string())?;
    let plan = match request.action.as_str() {
        "aliases" => {
            if request.icon.is_some()
                || request.mode.is_some()
                || request.direction.is_some()
                || request.sibling_rank.is_some()
                || request.remove
            {
                return Err(
                    "aliases preview contains fields for another metadata action".to_owned(),
                );
            }
            let aliases = request.aliases.ok_or("aliases preview requires aliases")?;
            plan_node_aliases_setting(root, node_id, &revision, &aliases)
        }
        "icon" => {
            if request.aliases.is_some()
                || request.mode.is_some()
                || request.direction.is_some()
                || request.sibling_rank.is_some()
            {
                return Err("icon preview contains fields for another metadata action".to_owned());
            }
            let icon = match (request.remove, request.icon.as_deref()) {
                (true, None) => None,
                (false, Some(icon)) => Some(icon),
                (true, Some(_)) => {
                    return Err("icon removal cannot include a value".to_owned());
                }
                (false, None) => return Err("icon preview requires icon or remove".to_owned()),
            };
            plan_node_icon_setting(root, node_id, &revision, icon)
        }
        "child_sort" => {
            if request.icon.is_some()
                || request.aliases.is_some()
                || request.sibling_rank.is_some()
                || request.remove
            {
                return Err(
                    "child_sort preview contains fields for another metadata action".to_owned(),
                );
            }
            let mode = request.mode.ok_or("child_sort preview requires mode")?;
            let direction = match (mode, request.direction) {
                (SortMode::Name, direction) => direction.unwrap_or_default(),
                (SortMode::Manual, None) => SortDirection::Ascending,
                (SortMode::Manual, Some(_)) => {
                    return Err("manual child_sort does not accept direction".to_owned());
                }
            };
            plan_node_child_sort_setting(root, node_id, &revision, ChildSort { mode, direction })
        }
        "sibling_rank" => {
            if request.icon.is_some()
                || request.aliases.is_some()
                || request.mode.is_some()
                || request.direction.is_some()
            {
                return Err(
                    "sibling_rank preview contains fields for another metadata action".to_owned(),
                );
            }
            let rank = match (request.remove, request.sibling_rank) {
                (true, None) => None,
                (false, Some(rank)) => Some(rank),
                (true, Some(_)) => {
                    return Err("sibling_rank removal cannot include a rank".to_owned());
                }
                (false, None) => {
                    return Err("sibling_rank preview requires siblingRank or remove".to_owned());
                }
            };
            plan_node_sibling_rank_setting(root, node_id, &revision, rank)
        }
        _ => return Err("unknown node metadata action".to_owned()),
    };
    plan.map_err(|error| error.to_string())
}

fn build_workspace_plan(
    root: &Path,
    action: &WorkspaceActionRequest,
) -> Result<WorkspaceTransactionPlan, String> {
    let node_id = || parse_action_id(action.node_id.as_deref(), "nodeId");
    let parent_id = || parse_action_id(action.parent_id.as_deref(), "parentId");
    let name = || {
        action
            .name
            .as_deref()
            .ok_or_else(|| "workspace action requires name".to_owned())
    };
    match action.action.as_str() {
        "create" => plan_create_child_node(root, parent_id()?, name()?),
        "rename" => plan_rename_node(root, node_id()?, name()?),
        "move" => plan_move_node(root, node_id()?, parent_id()?, name()?),
        "copy" => plan_copy_node(root, node_id()?, parent_id()?, name()?),
        "trash" => plan_trash_node(root, node_id()?),
        "restore" => plan_restore_node(root, node_id()?, parent_id()?, name()?),
        "presentation" => {
            let value = match action.value.as_deref() {
                Some("separate") => AdjacentHeadingBody::Separate,
                Some("run_in") => AdjacentHeadingBody::RunIn,
                _ => return Err("presentation value must be separate or run_in".to_owned()),
            };
            plan_adjacent_heading_body_setting(root, value)
        }
        _ => return Err("unknown workspace action".to_owned()),
    }
    .map_err(|error| error.to_string())
}

fn annotation_action(request: AnnotationPreviewRequest) -> Result<AnnotationAction, String> {
    match request.action.as_str() {
        "create" => create_annotation_action(request),
        "reply" => reply_annotation_action(request),
        "edit_message" => edit_annotation_message_action(request),
        "set_appearance" => set_annotation_appearance_action(request),
        "set_labels" => set_annotation_labels_action(request),
        "resolve" => simple_annotation_action(request, SimpleAnnotationAction::Resolve),
        "reopen" => simple_annotation_action(request, SimpleAnnotationAction::Reopen),
        "reanchor" => simple_annotation_action(request, SimpleAnnotationAction::Reanchor),
        "accept_suggestion" => {
            simple_annotation_action(request, SimpleAnnotationAction::AcceptSuggestion)
        }
        "reject_suggestion" => {
            simple_annotation_action(request, SimpleAnnotationAction::RejectSuggestion)
        }
        _ => Err("unknown annotation action".to_owned()),
    }
}

fn create_annotation_action(
    mut request: AnnotationPreviewRequest,
) -> Result<AnnotationAction, String> {
    if request.annotation_id.is_some() || request.message_id.is_some() {
        return Err("annotation create cannot include annotationId or messageId".to_owned());
    }
    let kind = request
        .kind
        .ok_or_else(|| "annotation create requires kind".to_owned())?;
    let target = annotation_target(request.target.take())?;
    let appearance = create_annotation_appearance(request.appearance.take())?;
    let body_source = request.body_source.take();
    validate_optional_annotation_body(body_source.as_deref())?;
    validate_annotation_create_combination(
        kind,
        &target,
        appearance,
        body_source.as_deref(),
        request.suggested_source.as_deref(),
    )?;
    let (author_id, author_name) = annotation_actor(&request)?;
    Ok(AnnotationAction::Create {
        kind,
        target,
        appearance,
        labels: request.labels.unwrap_or_default(),
        body_source,
        suggested_source: request.suggested_source,
        author_id,
        author_name,
        timestamp: request.timestamp,
    })
}

fn reply_annotation_action(
    mut request: AnnotationPreviewRequest,
) -> Result<AnnotationAction, String> {
    if request.message_id.is_some()
        || request.kind.is_some()
        || request.target.is_some()
        || request.appearance.is_some()
        || request.suggested_source.is_some()
        || request.labels.is_some()
    {
        return Err("annotation reply contains fields that do not apply to reply".to_owned());
    }
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    let body_source = required_annotation_body(request.body_source.take())?;
    let (author_id, author_name) = annotation_actor(&request)?;
    Ok(AnnotationAction::Reply {
        annotation_id,
        body_source,
        author_id,
        author_name,
        timestamp: request.timestamp,
    })
}

fn edit_annotation_message_action(
    mut request: AnnotationPreviewRequest,
) -> Result<AnnotationAction, String> {
    ensure_annotation_fields_absent(&request, true, "edit_message")?;
    if request.author_name.is_some() {
        return Err(
            "edit_message does not accept authorName; ownership is checked by authorId".to_owned(),
        );
    }
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    let message_id = annotation_request_id(request.message_id.as_deref(), "messageId")?;
    let body_source = required_annotation_body(request.body_source.take())?;
    let author_id = annotation_request_id(request.author_id.as_deref(), "authorId")?;
    Ok(AnnotationAction::EditMessage {
        annotation_id,
        message_id,
        body_source,
        author_id,
        timestamp: request.timestamp,
    })
}

fn set_annotation_appearance_action(
    request: AnnotationPreviewRequest,
) -> Result<AnnotationAction, String> {
    if request.message_id.is_some()
        || request.kind.is_some()
        || request.target.is_some()
        || request.body_source.is_some()
        || request.suggested_source.is_some()
        || request.labels.is_some()
        || request.author_id.is_some()
        || request.author_name.is_some()
    {
        return Err(
            "annotation appearance update contains fields that do not apply to set_appearance"
                .to_owned(),
        );
    }
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    let appearance = request
        .appearance
        .ok_or_else(|| "set_appearance requires appearance".to_owned())?;
    let appearance = annotation_appearance(appearance)?;
    Ok(AnnotationAction::SetAppearance {
        annotation_id,
        appearance,
        timestamp: request.timestamp,
    })
}

fn set_annotation_labels_action(
    request: AnnotationPreviewRequest,
) -> Result<AnnotationAction, String> {
    if request.message_id.is_some()
        || request.kind.is_some()
        || request.target.is_some()
        || request.appearance.is_some()
        || request.body_source.is_some()
        || request.suggested_source.is_some()
        || request.author_id.is_some()
        || request.author_name.is_some()
    {
        return Err(
            "annotation label update contains fields that do not apply to set_labels".to_owned(),
        );
    }
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    let labels = request
        .labels
        .ok_or_else(|| "set_labels requires labels".to_owned())?;
    Ok(AnnotationAction::SetLabels {
        annotation_id,
        labels,
        timestamp: request.timestamp,
    })
}

#[derive(Clone, Copy)]
enum SimpleAnnotationAction {
    Resolve,
    Reopen,
    Reanchor,
    AcceptSuggestion,
    RejectSuggestion,
}

fn simple_annotation_action(
    request: AnnotationPreviewRequest,
    action: SimpleAnnotationAction,
) -> Result<AnnotationAction, String> {
    if request.message_id.is_some() || request.body_source.is_some() {
        return Err("annotation state actions cannot include messageId or bodySource".to_owned());
    }
    ensure_annotation_fields_absent(&request, false, "annotation state action")?;
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    Ok(match action {
        SimpleAnnotationAction::Resolve => AnnotationAction::SetResolved {
            annotation_id,
            resolved: true,
            timestamp: request.timestamp,
        },
        SimpleAnnotationAction::Reopen => AnnotationAction::SetResolved {
            annotation_id,
            resolved: false,
            timestamp: request.timestamp,
        },
        SimpleAnnotationAction::Reanchor => AnnotationAction::Reanchor {
            annotation_id,
            timestamp: request.timestamp,
        },
        SimpleAnnotationAction::AcceptSuggestion => AnnotationAction::AcceptSuggestion {
            annotation_id,
            timestamp: request.timestamp,
        },
        SimpleAnnotationAction::RejectSuggestion => AnnotationAction::RejectSuggestion {
            annotation_id,
            timestamp: request.timestamp,
        },
    })
}

fn ensure_annotation_fields_absent(
    request: &AnnotationPreviewRequest,
    allow_author_id: bool,
    action: &str,
) -> Result<(), String> {
    if request.kind.is_some()
        || request.target.is_some()
        || request.appearance.is_some()
        || request.suggested_source.is_some()
        || request.labels.is_some()
        || request.author_name.is_some()
        || (!allow_author_id && request.author_id.is_some())
    {
        return Err(format!(
            "{action} contains annotation fields that do not apply"
        ));
    }
    Ok(())
}

fn annotation_target(
    target: Option<AnnotationTargetRequest>,
) -> Result<AnnotationTargetIntent, String> {
    target
        .map(AnnotationTargetIntent::from)
        .ok_or_else(|| "annotation create requires target".to_owned())
}

fn create_annotation_appearance(
    appearance: Option<AnnotationAppearanceRequest>,
) -> Result<Option<AnnotationAppearance>, String> {
    match appearance {
        Some(appearance) if appearance.mark == AnnotationMark::None => {
            Err("appearance mark none is reserved for set_appearance clearing".to_owned())
        }
        Some(appearance) => annotation_appearance(appearance),
        None => Ok(None),
    }
}

fn annotation_appearance(
    appearance: AnnotationAppearanceRequest,
) -> Result<Option<AnnotationAppearance>, String> {
    if appearance.mark == AnnotationMark::None {
        if appearance.theme.is_some() {
            return Err("clearing annotation appearance cannot include theme".to_owned());
        }
        return Ok(None);
    }
    let color = appearance
        .theme
        .ok_or_else(|| "annotation appearance requires theme".to_owned())?;
    Ok(Some(AnnotationAppearance {
        mark: appearance.mark,
        color,
    }))
}

fn validate_annotation_create_combination(
    kind: AnnotationKind,
    target: &AnnotationTargetIntent,
    appearance: Option<AnnotationAppearance>,
    body_source: Option<&str>,
    suggested_source: Option<&str>,
) -> Result<(), String> {
    match kind {
        AnnotationKind::Comment if body_source.is_none() || suggested_source.is_some() => {
            Err("comment annotations require bodySource and forbid suggestedSource".to_owned())
        }
        AnnotationKind::Mark if appearance.is_none() || suggested_source.is_some() => {
            Err("mark annotations require appearance and forbid suggestedSource".to_owned())
        }
        AnnotationKind::SuggestionInsert
            if !matches!(target, AnnotationTargetIntent::InsertionPoint { .. })
                || suggested_source.is_none_or(str::is_empty) =>
        {
            Err(
                "suggestion_insert requires an insertion_point target and suggestedSource"
                    .to_owned(),
            )
        }
        AnnotationKind::SuggestionDelete
            if !matches!(target, AnnotationTargetIntent::TextRange { .. })
                || suggested_source.is_some() =>
        {
            Err(
                "suggestion_delete requires a text_range target and forbids suggestedSource"
                    .to_owned(),
            )
        }
        AnnotationKind::Comment
        | AnnotationKind::Mark
        | AnnotationKind::SuggestionInsert
        | AnnotationKind::SuggestionDelete => Ok(()),
    }
}

fn required_annotation_body(value: Option<String>) -> Result<String, String> {
    let value = value.ok_or_else(|| "annotation action requires bodySource".to_owned())?;
    validate_optional_annotation_body(Some(&value))?;
    Ok(value)
}

fn validate_optional_annotation_body(value: Option<&str>) -> Result<(), String> {
    if value.is_some_and(|body| body.trim().is_empty()) {
        Err("annotation bodySource cannot be empty".to_owned())
    } else {
        Ok(())
    }
}

fn annotation_actor(request: &AnnotationPreviewRequest) -> Result<(Uuid, String), String> {
    let author_id = annotation_request_id(request.author_id.as_deref(), "authorId")?;
    let author_name = request
        .author_name
        .as_deref()
        .ok_or_else(|| "annotation action requires authorName".to_owned())?
        .trim()
        .to_owned();
    if author_name.is_empty() {
        return Err("annotation authorName cannot be empty".to_owned());
    }
    Ok((author_id, author_name))
}

fn annotation_request_id(value: Option<&str>, field: &str) -> Result<Uuid, String> {
    let value = value.ok_or_else(|| format!("annotation action requires {field}"))?;
    let parsed = value
        .parse::<Uuid>()
        .map_err(|error| format!("invalid annotation {field}: {error}"))?;
    if value != parsed.to_string()
        || parsed.get_version_num() != 4
        || parsed.get_variant() != uuid::Variant::RFC4122
    {
        return Err(format!(
            "annotation {field} must be a lowercase RFC 4122 UUIDv4"
        ));
    }
    Ok(parsed)
}

fn parse_chrono_periods(values: &[String]) -> Result<Vec<ChronoPeriod>, String> {
    values
        .iter()
        .map(|value| match value.as_str() {
            "year" => Ok(ChronoPeriod::Year),
            "quarter" => Ok(ChronoPeriod::Quarter),
            "month" => Ok(ChronoPeriod::Month),
            "week" => Ok(ChronoPeriod::Week),
            "day" => Ok(ChronoPeriod::Day),
            _ => Err("invalid Chrono period".to_owned()),
        })
        .collect()
}

fn parse_action_id(value: Option<&str>, field: &str) -> Result<NodeId, String> {
    value
        .ok_or_else(|| format!("workspace action requires {field}"))?
        .parse()
        .map_err(|error: weftext_core::NodeIdError| error.to_string())
}

fn parse_trash_item_id(value: &str) -> Result<TrashItemId, String> {
    value
        .parse()
        .map_err(|error: weftext_core::TrashIdError| error.to_string())
}

fn require_workspace_base_revision(root: &Path, expected: &str) -> Result<(), String> {
    let expected = WorkspaceRevision::parse(expected).map_err(|error| error.to_string())?;
    let actual = read_workspace_revision(root).map_err(|error| error.to_string())?;
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "stale Trash preview: expected workspace revision {expected}, actual {actual}"
        ))
    }
}

fn require_exact_permanent_delete_evidence(
    preview: &weftext_core::TrashPermanentDeletePreview,
    mut supplied: Vec<TrashPermanentDeleteEvidence>,
) -> Result<(), String> {
    let mut expected = preview
        .items
        .iter()
        .map(|item| TrashPermanentDeleteEvidence {
            trash_item_id: item.trash_item_id,
            payload_sha256: item.payload_sha256.clone(),
            payload_byte_length: item.payload_byte_length,
        })
        .collect::<Vec<_>>();
    supplied.sort_by_key(|item| item.trash_item_id);
    expected.sort_by_key(|item| item.trash_item_id);
    if supplied == expected {
        Ok(())
    } else {
        Err("permanent deletion evidence does not match the current item IDs, payload digests, and byte lengths".to_owned())
    }
}

fn stage_workspace_plan(
    state: &mut BridgeState,
    plan: WorkspaceTransactionPlan,
) -> Result<Value, String> {
    if state.plans.len() >= MAX_PENDING_WORKSPACE_PLANS {
        return Err(
            "too many pending workspace plans; commit or reconnect before previewing again"
                .to_owned(),
        );
    }
    let response = transaction_plan_json(&plan);
    state.plans.insert(plan.plan_id.clone(), plan);
    Ok(response)
}

fn write_workspace_plan_preview(
    stream: &mut TcpStream,
    allowed_origin: Option<&str>,
    result: Result<Value, String>,
) -> Result<(), String> {
    match result {
        Ok(plan) => write_response(
            stream,
            200,
            allowed_origin,
            json!({"schema": SCHEMA, "ok": true, "plan": plan}),
        ),
        Err(error) => write_response(
            stream,
            422,
            allowed_origin,
            json!({"schema": SCHEMA, "ok": false, "error": error}),
        ),
    }
}

fn transaction_plan_json(plan: &WorkspaceTransactionPlan) -> Value {
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

fn trash_transaction_plan_json(plan: &WorkspaceTransactionPlan) -> Value {
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

fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| error.to_string())?);
    let mut header_bytes = 0_usize;
    let mut request_line = String::new();
    read_header_line(&mut reader, &mut request_line, &mut header_bytes)?;
    let mut pieces = request_line.split_whitespace();
    let method = pieces.next().ok_or("missing HTTP method")?.to_owned();
    let path = pieces.next().ok_or("missing HTTP path")?.to_owned();
    let version = pieces.next().ok_or("missing HTTP version")?;
    if pieces.next().is_some() || !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err("unsupported HTTP request line".to_owned());
    }

    let mut headers = BTreeMap::new();
    loop {
        let mut line = String::new();
        read_header_line(&mut reader, &mut line, &mut header_bytes)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        let (name, value) = line.split_once(':').ok_or("invalid HTTP header")?;
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty() || headers.contains_key(&name) {
            return Err("duplicate or empty HTTP header".to_owned());
        }
        headers.insert(name, value.trim().to_owned());
    }
    if headers.contains_key("transfer-encoding") {
        return Err("chunked prototype bridge requests are unsupported".to_owned());
    }
    let content_length = headers.get("content-length").map_or(Ok(0_usize), |value| {
        value
            .parse::<usize>()
            .map_err(|_| "invalid Content-Length".to_owned())
    })?;
    if content_length > MAX_BODY_BYTES {
        return Err("prototype bridge request body exceeds 8 MiB".to_owned());
    }
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("incomplete HTTP request body: {error}"))?;
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

fn read_header_line(
    reader: &mut BufReader<TcpStream>,
    line: &mut String,
    total: &mut usize,
) -> Result<(), String> {
    let read = reader
        .read_line(line)
        .map_err(|error| format!("could not read HTTP header: {error}"))?;
    if read == 0 {
        return Err("unexpected end of HTTP headers".to_owned());
    }
    *total = total.saturating_add(read);
    if *total > MAX_HEADER_BYTES {
        return Err("prototype bridge headers exceed 32 KiB".to_owned());
    }
    Ok(())
}

fn parse_edit_request(body: &[u8]) -> Result<EditRequest, String> {
    serde_json::from_slice(body).map_err(|error| format!("invalid edit request: {error}"))
}

fn require_authorization(request: &Request, token: &str) -> Result<(), String> {
    let expected = format!("Bearer {token}");
    match request.headers.get("authorization") {
        Some(value) if value == &expected => Ok(()),
        _ => Err("prototype bridge authorization failed".to_owned()),
    }
}

fn require_allowed_origin(origin: &str) -> Result<&str, String> {
    if origin == HOSTED_PROTOTYPE_ORIGIN || is_loopback_origin(origin) {
        Ok(origin)
    } else {
        Err("prototype bridge rejected the browser origin".to_owned())
    }
}

fn is_loopback_origin(origin: &str) -> bool {
    [
        "http://localhost",
        "http://127.0.0.1",
        "https://localhost",
        "https://127.0.0.1",
    ]
    .iter()
    .any(|prefix| {
        origin == *prefix
            || origin.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.starts_with(':')
                    && suffix.len() > 1
                    && suffix[1..].bytes().all(|byte| byte.is_ascii_digit())
            })
    })
}

const fn error_status(error: &DocumentError) -> u16 {
    match error {
        DocumentError::StaleRevision { .. } | DocumentError::IdentityChanged { .. } => 409,
        DocumentError::InvalidRevision(_)
        | DocumentError::InvalidMetadata(_)
        | DocumentError::MissingIdentity
        | DocumentError::InvalidWorkspaceFormat(_)
        | DocumentError::InvalidEditRange { .. }
        | DocumentError::NonCharacterBoundary { .. }
        | DocumentError::OverlappingEdits
        | DocumentError::ContentBoundary(_) => 422,
        DocumentError::InvalidNodePath(_)
        | DocumentError::AmbiguousDocumentGeneration(_)
        | DocumentError::SymlinkUnsupported(_)
        | DocumentError::InvalidUtf8(_)
        | DocumentError::VerificationFailed { .. }
        | DocumentError::Io(_)
        | DocumentError::Persist(_) => 500,
    }
}

fn node_name(node_directory: &Path) -> String {
    node_directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Weftext node")
        .to_owned()
}

fn derived_index_result<T, E>(result: Result<T, E>) -> (Value, Value)
where
    T: Serialize,
    E: std::fmt::Display,
{
    match result {
        Ok(index) => (
            serde_json::to_value(index).expect("search index statistics are serializable"),
            Value::Null,
        ),
        Err(error) => (
            Value::Null,
            json!({
                "code": "derived_search_index_refresh_failed",
                "message": error.to_string(),
                "rebuildRequired": true,
                "authoritativeCommitSucceeded": true,
            }),
        ),
    }
}

fn derived_index_open_result<T, E>(result: Result<T, E>) -> (Value, Value)
where
    T: Serialize,
    E: std::fmt::Display,
{
    match result {
        Ok(index) => (
            serde_json::to_value(index).expect("search index statistics are serializable"),
            Value::Null,
        ),
        Err(error) => (
            Value::Null,
            json!({
                "code": "derived_search_index_refresh_failed",
                "message": error.to_string(),
                "rebuildRequired": true,
                "workspaceOpenSucceeded": true,
            }),
        ),
    }
}

fn write_empty_response(
    stream: &mut TcpStream,
    status: u16,
    allowed_origin: Option<&str>,
) -> Result<(), String> {
    write_raw_response(stream, status, allowed_origin, &[])
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    allowed_origin: Option<&str>,
    body: Value,
) -> Result<(), String> {
    let serialized = serde_json::to_vec(&body).map_err(|error| error.to_string())?;
    drop(body);
    write_raw_response(stream, status, allowed_origin, &serialized)
}

fn write_raw_response(
    stream: &mut TcpStream,
    status: u16,
    allowed_origin: Option<&str>,
    body: &[u8],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        410 => "Gone",
        413 => "Content Too Large",
        422 => "Unprocessable Content",
        _ => "Internal Server Error",
    };
    let mut headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(origin) = allowed_origin {
        FmtWrite::write_fmt(
            &mut headers,
            format_args!(
                "Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Authorization, Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\n"
            ),
        )
        .map_err(|error| format!("could not format HTTP response: {error}"))?;
    }
    headers.push_str("\r\n");
    stream
        .write_all(headers.as_bytes())
        .and_then(|()| stream.write_all(body))
        .and_then(|()| stream.flush())
        .map_err(|error| format!("could not write HTTP response: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{AnnotationPreviewRequest, annotation_action, workspace_payload};
    use std::path::Path;

    use serde_json::{Value, json};
    use tempfile::tempdir;
    use weftext_core::{
        AnnotationAction, AnnotationKind, AnnotationMark, AnnotationResourceMediaKind,
        AnnotationResourceRegion, AnnotationTargetIntent, commit_workspace_transaction,
        create_workspace, plan_annotation_action,
    };

    fn annotation_request(mut overrides: Value) -> AnnotationPreviewRequest {
        let mut value = json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": { "kind": "document" },
            "appearance": { "mark": "highlight", "theme": "yellow" },
            "bodySource": "Review this sentence.",
            "labels": ["review"],
            "authorId": "22222222-2222-4222-8222-222222222222",
            "authorName": "Reviewer",
            "timestamp": "2026-08-24T08:00:00+08:00"
        });
        let object = value.as_object_mut().expect("request object");
        for (key, replacement) in overrides.as_object_mut().expect("override object").iter() {
            if replacement.is_null() {
                object.remove(key);
            } else {
                object.insert(key.clone(), replacement.clone());
            }
        }
        serde_json::from_value(value).expect("annotation request")
    }

    #[test]
    fn bridge_payload_uses_the_versioned_core_navigation_projection() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/content-boundary-v02");
        let payload = workspace_payload(&root).expect("workspace payload");
        assert_eq!(payload["navigation"]["version"], 1);
        assert_eq!(
            payload["navigation"]["hierarchy"]
                .as_array()
                .expect("hierarchy")
                .iter()
                .map(|entry| entry["locator"].as_str().expect("locator"))
                .collect::<Vec<_>>(),
            vec!["", "Managed"]
        );
        assert_eq!(
            payload["navigation"]["contents"]
                .as_array()
                .expect("contents")
                .iter()
                .filter(|entry| entry["parentLocator"] == "")
                .map(|entry| entry["locator"].as_str().expect("locator"))
                .collect::<Vec<_>>(),
            vec!["Managed", "Files", "loose.md", "resource.bin"]
        );
    }

    #[test]
    fn canonical_annotation_create_maps_without_defaults_or_legacy_fields() {
        let action = annotation_action(annotation_request(json!({}))).expect("annotation action");
        assert!(matches!(
            action,
            AnnotationAction::Create {
                kind: AnnotationKind::Comment,
                target: AnnotationTargetIntent::Document,
                appearance: Some(appearance),
                body_source: Some(ref body),
                suggested_source: None,
                ..
            } if appearance.mark == AnnotationMark::Highlight && body == "Review this sentence."
        ));
    }

    #[test]
    fn v3_annotation_requests_cover_suggestions_and_thread_actions() {
        let insert = annotation_action(annotation_request(json!({
            "kind": "suggestion_insert",
            "target": { "kind": "insertion_point", "position": 12 },
            "suggestedSource": "inserted text",
            "appearance": null,
            "bodySource": null
        })))
        .expect("insert suggestion");
        assert!(matches!(
            insert,
            AnnotationAction::Create {
                kind: AnnotationKind::SuggestionInsert,
                target: AnnotationTargetIntent::InsertionPoint { position: 12 },
                suggested_source: Some(ref source),
                ..
            } if source == "inserted text"
        ));

        let edit = annotation_action(annotation_request(json!({
            "action": "edit_message",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "messageId": "44444444-4444-4444-8444-444444444444",
            "kind": null,
            "target": null,
            "appearance": null,
            "labels": null,
            "authorName": null
        })))
        .expect("message edit");
        assert!(matches!(edit, AnnotationAction::EditMessage { .. }));

        let reply = annotation_action(annotation_request(json!({
            "action": "reply",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "kind": null,
            "target": null,
            "appearance": null,
            "labels": null
        })))
        .expect("reply");
        assert!(matches!(reply, AnnotationAction::Reply { .. }));
    }

    #[test]
    fn v3_annotation_requests_map_delete_and_resource_targets_without_loss() {
        let deletion = annotation_action(annotation_request(json!({
            "kind": "suggestion_delete",
            "target": { "kind": "text_range", "start": 8, "end": 16 },
            "appearance": null,
            "bodySource": null
        })))
        .expect("delete suggestion");
        assert!(matches!(
            deletion,
            AnnotationAction::Create {
                kind: AnnotationKind::SuggestionDelete,
                target: AnnotationTargetIntent::TextRange { start: 8, end: 16 },
                suggested_source: None,
                ..
            }
        ));

        let resource = annotation_action(annotation_request(json!({
            "target": {
                "kind": "resource_region",
                "resourceLocator": "figure.pdf",
                "resourceDigest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "mediaKind": "pdf",
                "region": {
                    "kind": "rect",
                    "page": 2,
                    "xMillionths": 10,
                    "yMillionths": 20,
                    "widthMillionths": 30,
                    "heightMillionths": 40
                }
            }
        })))
        .expect("resource annotation");
        assert!(matches!(
            resource,
            AnnotationAction::Create {
                target: AnnotationTargetIntent::ResourceRegion {
                    media_kind: AnnotationResourceMediaKind::Pdf,
                    region: AnnotationResourceRegion::Rect { page: Some(2), .. },
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn v3_annotation_requests_cover_appearance_labels_and_resolution_actions() {
        let appearance = annotation_action(annotation_request(json!({
            "action": "set_appearance",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "kind": null,
            "target": null,
            "appearance": { "mark": "underline", "theme": "blue" },
            "bodySource": null,
            "labels": null,
            "authorId": null,
            "authorName": null
        })))
        .expect("appearance change");
        assert!(matches!(
            appearance,
            AnnotationAction::SetAppearance {
                appearance: Some(value),
                ..
            } if value.mark == AnnotationMark::Underline
        ));

        let cleared = annotation_action(annotation_request(json!({
            "action": "set_appearance",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "kind": null,
            "target": null,
            "appearance": { "mark": "none" },
            "bodySource": null,
            "labels": null,
            "authorId": null,
            "authorName": null
        })))
        .expect("appearance clear");
        assert!(matches!(
            cleared,
            AnnotationAction::SetAppearance {
                appearance: None,
                ..
            }
        ));

        let labels = annotation_action(annotation_request(json!({
            "action": "set_labels",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "kind": null,
            "target": null,
            "appearance": null,
            "bodySource": null,
            "labels": ["review", "urgent"],
            "authorId": null,
            "authorName": null
        })))
        .expect("label change");
        assert!(matches!(
            labels,
            AnnotationAction::SetLabels { labels, .. } if labels == ["review", "urgent"]
        ));
    }

    #[test]
    fn v3_annotation_requests_cover_all_simple_snake_case_actions() {
        for action_name in [
            "resolve",
            "reopen",
            "reanchor",
            "accept_suggestion",
            "reject_suggestion",
        ] {
            let action = annotation_action(annotation_request(json!({
                "action": action_name,
                "annotationId": "33333333-3333-4333-8333-333333333333",
                "kind": null,
                "target": null,
                "appearance": null,
                "bodySource": null,
                "labels": null,
                "authorId": null,
                "authorName": null
            })))
            .expect("state action");
            assert!(match action_name {
                "resolve" => matches!(action, AnnotationAction::SetResolved { resolved: true, .. }),
                "reopen" => matches!(
                    action,
                    AnnotationAction::SetResolved {
                        resolved: false,
                        ..
                    }
                ),
                "reanchor" => matches!(action, AnnotationAction::Reanchor { .. }),
                "accept_suggestion" => {
                    matches!(action, AnnotationAction::AcceptSuggestion { .. })
                }
                "reject_suggestion" => {
                    matches!(action, AnnotationAction::RejectSuggestion { .. })
                }
                _ => false,
            });
        }
    }

    #[test]
    fn annotation_bridge_rejects_every_retired_request_shape() {
        for retired in [
            json!({ "sourceOffset": 0 }),
            json!({ "mark": "highlight" }),
            json!({ "color": "yellow" }),
            json!({ "resolved": true }),
            json!({ "bodyMarkdown": "retired" }),
        ] {
            let mut request = json!({
                "action": "create",
                "nodeId": "11111111-1111-4111-8111-111111111111",
                "kind": "comment",
                "target": { "kind": "document" },
                "bodySource": "comment",
                "authorId": "22222222-2222-4222-8222-222222222222",
                "authorName": "Reviewer",
                "timestamp": "2026-08-24T08:00:00+08:00"
            });
            request
                .as_object_mut()
                .expect("request")
                .extend(retired.as_object().expect("retired field").clone());
            assert!(serde_json::from_value::<AnnotationPreviewRequest>(request).is_err());
        }

        for retired_action in [
            "editMessage",
            "setAppearance",
            "setLabels",
            "acceptSuggestion",
            "rejectSuggestion",
            "appearance",
            "labels",
            "accept",
            "reject",
        ] {
            let request = annotation_request(json!({ "action": retired_action }));
            assert!(annotation_action(request).is_err());
        }

        for retired_target in [
            json!({ "type": "document" }),
            json!({ "kind": "textRange", "start": 0, "end": 1 }),
            json!({ "kind": "block_at", "source_offset": 0 }),
            json!({ "kind": "document", "unexpected": true }),
        ] {
            let mut request = json!({
                "action": "create",
                "nodeId": "11111111-1111-4111-8111-111111111111",
                "kind": "comment",
                "bodySource": "comment",
                "authorId": "22222222-2222-4222-8222-222222222222",
                "authorName": "Reviewer",
                "timestamp": "2026-08-24T08:00:00+08:00"
            });
            request["target"] = retired_target.clone();
            assert!(
                serde_json::from_value::<AnnotationPreviewRequest>(request).is_err(),
                "retired target was accepted: {retired_target}"
            );
        }

        let retired_region = json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {
                "kind": "resource_region",
                "resourceLocator": "figure.pdf",
                "resourceDigest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "mediaKind": "pdf",
                "region": {
                    "type": "rect",
                    "xMillionths": 0,
                    "yMillionths": 0,
                    "widthMillionths": 1,
                    "heightMillionths": 1
                }
            },
            "bodySource": "comment",
            "authorId": "22222222-2222-4222-8222-222222222222",
            "authorName": "Reviewer",
            "timestamp": "2026-08-24T08:00:00+08:00"
        });
        assert!(serde_json::from_value::<AnnotationPreviewRequest>(retired_region).is_err());
    }

    #[test]
    fn annotation_bridge_rejects_invalid_canonical_combinations() {
        assert!(
            annotation_action(annotation_request(json!({
                "appearance": { "mark": "none" }
            })))
            .is_err()
        );

        assert!(
            annotation_action(annotation_request(json!({
                "action": "set_appearance",
                "annotationId": "33333333-3333-4333-8333-333333333333",
                "kind": null,
                "target": null,
                "appearance": { "mark": "none", "theme": "yellow" },
                "bodySource": null,
                "labels": null,
                "authorId": null,
                "authorName": null
            })))
            .is_err()
        );

        let wrong_target = annotation_action(annotation_request(json!({
            "kind": "suggestion_insert",
            "target": { "kind": "text_range", "start": 0, "end": 4 },
            "suggestedSource": "inserted text",
            "appearance": null,
            "bodySource": null
        })));
        assert!(wrong_target.is_err());

        let uppercase_actor = annotation_action(annotation_request(json!({
            "authorId": "22222222-2222-4222-8222-22222222222A"
        })));
        assert!(uppercase_actor.is_err());
    }

    #[test]
    fn annotation_nested_objects_reject_unknown_fields_and_camel_aliases() {
        let mut appearance_unknown = json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": { "kind": "document" },
            "appearance": { "mark": "highlight", "theme": "yellow", "color": "yellow" },
            "bodySource": "comment",
            "authorId": "22222222-2222-4222-8222-222222222222",
            "authorName": "Reviewer",
            "timestamp": "2026-08-24T08:00:00+08:00"
        });
        assert!(
            serde_json::from_value::<AnnotationPreviewRequest>(appearance_unknown.take()).is_err()
        );

        let camel_kind = annotation_request(json!({
            "kind": "suggestion_insert",
            "target": { "kind": "insertion_point", "position": 1 },
            "appearance": null,
            "bodySource": null,
            "suggestedSource": "x"
        }));
        assert!(matches!(
            annotation_action(camel_kind),
            Ok(AnnotationAction::Create {
                kind: AnnotationKind::SuggestionInsert,
                ..
            })
        ));
        assert!(
            serde_json::from_value::<AnnotationPreviewRequest>(json!({
                "action": "create",
                "nodeId": "11111111-1111-4111-8111-111111111111",
                "kind": "suggestionInsert",
                "target": { "kind": "insertion_point", "position": 1 },
                "suggestedSource": "x",
                "authorId": "22222222-2222-4222-8222-222222222222",
                "authorName": "Reviewer",
                "timestamp": "2026-08-24T08:00:00+08:00"
            }))
            .is_err()
        );

        let region_unknown = json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {
                "kind": "resource_region",
                "resourceLocator": "figure.pdf",
                "resourceDigest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "mediaKind": "pdf",
                "region": {
                    "kind": "rect",
                    "xMillionths": 0,
                    "yMillionths": 0,
                    "widthMillionths": 1,
                    "heightMillionths": 1,
                    "unexpected": true
                }
            },
            "bodySource": "comment",
            "authorId": "22222222-2222-4222-8222-222222222222",
            "authorName": "Reviewer",
            "timestamp": "2026-08-24T08:00:00+08:00"
        });
        assert!(serde_json::from_value::<AnnotationPreviewRequest>(region_unknown).is_err());
    }

    #[test]
    fn annotation_bridge_persists_only_v3_asciidoc_inline_messages() {
        let temporary = tempdir().expect("temporary directory");
        let root = temporary.path().join("Notes");
        let node = create_workspace(&root).expect("workspace");
        let request = annotation_request(json!({
            "nodeId": node.id.to_string()
        }));
        let action = annotation_action(request).expect("annotation action");
        let sidecar_snapshot = weftext_core::capture_annotation_sidecar_snapshot(
            &root,
            node.id,
            weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
        )
        .expect("annotation sidecar snapshot");
        let plan =
            plan_annotation_action(&root, &sidecar_snapshot, action).expect("annotation plan");
        commit_workspace_transaction(&plan).expect("annotation commit");

        let sidecar = std::fs::read_to_string(root.join("weftext.annotations.json"))
            .expect("annotation sidecar");
        let value: Value = serde_json::from_str(&sidecar).expect("annotation JSON");
        assert_eq!(value["version"], 3);
        assert_eq!(
            value["annotations"][0]["thread"][0]["body"]["format"],
            "weftext.asciidoc.inline.v1"
        );
        assert!(value["annotations"][0].get("anchor").is_none());
        assert!(value["annotations"][0].get("target").is_some());
        assert!(!sidecar.contains("bodyMarkdown"));
    }
}
