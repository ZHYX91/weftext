use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tempfile::Builder;
use uuid::Uuid;
use weftext_backup::{
    commit_alternate_restore, commit_full_workspace_backup, commit_restore_drill,
    commit_scoped_restore, commit_snapshot_retention, plan_alternate_restore,
    plan_full_workspace_backup, plan_restore_drill, plan_single_node_restore,
    plan_snapshot_retention, plan_subtree_restore, protect_full_workspace_snapshot,
    recover_snapshot_retention, verify_full_workspace_snapshot, AlternateRestorePlan,
    FullWorkspaceBackupPlan, RestoreDrillPlan, ScopedRestorePlan, SnapshotRetentionPlan,
    SnapshotRetentionPolicy,
};
use weftext_core::{
    analyze_citation_authoring_source, analyze_document_for_profile,
    analyze_document_header_properties, bind_workspace_transaction_target_resolution,
    build_workspace_link_index, citation_presentation_capabilities, commit_document_edit,
    commit_import_resource, commit_task_dependency_transaction, commit_task_edit_transaction,
    commit_task_recurrence_transaction, commit_workspace_transaction,
    commit_workspace_transaction_with_draft_gate, confirm_permanent_delete_trash_items,
    patch_document_header_property, plan_adjacent_heading_body_setting, plan_citation_macro_edit,
    plan_copy_node, plan_create_child_node, plan_document_edit, plan_document_format,
    plan_migrate_legacy_workspace_trash_at_with_backup, plan_move_node, plan_node_aliases_setting,
    plan_node_child_sort_setting, plan_node_icon_setting, plan_node_sibling_rank_setting,
    plan_permanently_delete_trash_items, plan_rename_node, plan_restore_node,
    plan_restore_trash_item, plan_task_dependency_transaction, plan_task_edit_transaction,
    plan_task_recurrence_transaction, plan_trash_node, plan_trash_node_at, plan_trash_resources_at,
    prepare_legacy_trash_migration_backup, present_citations, preview_permanent_delete_trash_items,
    preview_workspace_transaction_draft_gate, project_node_metadata, project_workspace_trash_state,
    read_node_document, read_workspace_revision, recover_workspace_transactions,
    refresh_workspace_search_index, refresh_workspace_search_index_invalidating,
    resolve_node_icon_from_source, scan_workspace, search_workspace_index,
    workspace_document_format, AdjacentHeadingBody, AnnotationAction, AnnotationAppearance,
    AnnotationColor, AnnotationKind, AnnotationMark, AnnotationResourceMediaKind,
    AnnotationResourceRegion, AnnotationTargetIntent, CalendarDate, ChildSort, ChronoPeriod,
    CitationAccessScope, CitationEditTarget, CitationMacroIntent, CitationPresentationProfile,
    CitationPresentationRequest, CitationWorkspaceIndex, DocumentEdit, DocumentEditPlan,
    DocumentFormatCommand, DocumentProfileId, DocumentRevision, DocumentSnapshot, NodeId,
    QueryAccessScope, QueryEvaluationContext, QueryWorkspaceIndex, ResourceImportPlan,
    SortDirection, SortMode, TaskDependencyTransactionPlan, TaskEditIntent, TaskEditTarget,
    TaskEditTransactionPlan, TaskId, TaskImportDocumentInput, TaskImportSettings,
    TaskRecurrenceCompletionContext, TaskRecurrenceTransactionPlan, TaskWorkspaceIndex,
    TrashItemId, TrashResourceSelection, TrashRestoreMode, WorkspaceDocumentGeneration,
    WorkspaceDraftGateToken, WorkspaceDraftRegistryView, WorkspaceRevision,
    WorkspaceTargetResolution, WorkspaceTransactionError, WorkspaceTransactionPlan,
    TASK_IMPORT_PROFILE_ID,
};
use weftext_export::{
    commit_markdown_export, preview_markdown_export, MarkdownExportPlan, MarkdownMetadataPolicy,
};
use weftext_import::{
    AgentImportPatch, CancellationToken, ImportTempRoot, OriginClass, PortablePath,
};
use weftext_intake::{
    apply_approved_agent_patch, commit_previewed_import, commit_previewed_task_import,
    docling_lite_capability, prepare_agent_enhancement, preview_docling_pdf_import,
    preview_fake_import, preview_markdown_import, preview_task_import,
    recover_previewed_task_import, rfc3339_utc_now, validate_preview_bundle,
    AgentEnhancementPreview, AgentEvidenceSelection, ImportPreviewBundle, TaskImportPreviewBundle,
    TaskImportReview,
};

use crate::agent_lifecycle::DesktopAgentLifecycle;
use crate::draft_store::{DraftInventory, DraftStore, StoredDraft};

const SETTINGS_FILE: &str = "desktop-session.json";
const MAX_RECENT_WORKSPACES: usize = 10;
const MAX_PENDING_WORKSPACE_PLANS: usize = 64;
const MAX_PENDING_TASK_PLANS: usize = 64;
const MAX_PENDING_IMPORT_PLANS: usize = 8;
const MAX_PENDING_AGENT_IMPORT_PLANS: usize = 8;
const MAX_PENDING_TASK_IMPORT_PLANS: usize = 8;
const MAX_PENDING_EXPORT_PLANS: usize = 8;
const MAX_PENDING_BACKUP_PLANS: usize = 16;
const MAX_TASK_IMPORT_DOCUMENTS: usize = 2_048;
const MAX_TASK_IMPORT_SOURCE_SET_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct DesktopPreferences {
    active_workspace: Option<String>,
    recent_workspaces: Vec<RecentWorkspace>,
    safe_mode: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentWorkspace {
    path: String,
    last_node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditRequest {
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
struct DraftSaveRequest {
    node_id: String,
    revision: String,
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftDiscardRequest {
    node_id: String,
}

#[derive(Debug, Deserialize)]
struct SafeModeRequest {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceActionRequest {
    action: String,
    node_id: Option<String>,
    parent_id: Option<String>,
    name: Option<String>,
    value: Option<String>,
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TrashLegacyMigrationPreviewRequest {
    base_workspace_revision: String,
    trashed_at: String,
    backup_parent_capability: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourcePreviewRequest {
    node_id: String,
    name: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ImportPreviewRequest {
    display_name: String,
    bytes: Vec<u8>,
    destination: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MarkdownImportPreviewRequest {
    display_name: String,
    bytes: Vec<u8>,
    destination: String,
    retain_original: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ImportCommitRequest {
    bundle_digest: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskImportSourceDocumentRequest {
    locator: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskImportPreviewRequest {
    profile: String,
    destination_parent_id: NodeId,
    destination_name: String,
    settings: TaskImportSettings,
    documents: Vec<TaskImportSourceDocumentRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskImportCommitRequest {
    review: TaskImportReview,
    receipt_destination_capability: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskImportRecoverRequest {
    review: TaskImportReview,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AgentEnhancementPrepareRequest {
    bundle_digest: String,
    provider: String,
    selected_node_ids: Vec<String>,
    retention: String,
    redaction: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AgentPatchApplyRequest {
    preview_digest: String,
    egress_approved: bool,
    patch: AgentImportPatch,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MarkdownExportPreviewRequest {
    node_id: String,
    destination_capability: String,
    metadata_policy: MarkdownMetadataPolicy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MarkdownExportCommitRequest {
    plan_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BackupPathCapabilityKind {
    BackupParent,
    Snapshot,
    RestoreParent,
    DrillParent,
    DrillResultsParent,
}

impl BackupPathCapabilityKind {
    const fn token_prefix(self) -> &'static str {
        match self {
            Self::BackupParent => "backup-parent",
            Self::Snapshot => "backup-snapshot",
            Self::RestoreParent => "backup-restore-parent",
            Self::DrillParent => "backup-drill-parent",
            Self::DrillResultsParent => "backup-drill-results-parent",
        }
    }
}

#[derive(Debug)]
struct BackupPathCapability {
    kind: BackupPathCapabilityKind,
    path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupPreviewRequest {
    backup_parent_capability: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupPlanCommitRequest {
    plan_digest: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupSnapshotRequest {
    snapshot_capability: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupProtectRequest {
    snapshot_capability: String,
    label: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupRetentionPreviewRequest {
    backup_parent_capability: String,
    keep_latest_unprotected: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupRetentionRecoverRequest {
    backup_parent_capability: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupAlternateRestorePreviewRequest {
    snapshot_capability: String,
    destination_parent_capability: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BackupScopedRestorePreviewRequest {
    snapshot_capability: String,
    source_node_id: String,
    destination_parent_id: String,
    destination_name: String,
    scope: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[expect(
    clippy::struct_field_names,
    reason = "the suffix makes each distinct, typed directory capability explicit on the wire"
)]
struct BackupDrillPreviewRequest {
    snapshot_capability: String,
    drill_parent_capability: String,
    results_parent_capability: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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

struct PendingAnnotationPlan {
    node_id: NodeId,
    plan: WorkspaceTransactionPlan,
}

#[derive(Clone)]
struct PendingTaskImportPlan {
    bundle: TaskImportPreviewBundle,
    review: TaskImportReview,
    receipt_path: Option<PathBuf>,
}

pub(crate) struct DesktopBackend {
    config_dir: PathBuf,
    docling_installation_root: PathBuf,
    agents: DesktopAgentLifecycle,
    drafts: DraftStore,
    preferences: DesktopPreferences,
    workspace_root: Option<PathBuf>,
    plans: HashMap<String, WorkspaceTransactionPlan>,
    workspace_draft_gate_tokens: HashMap<String, WorkspaceDraftGateToken>,
    annotation_plans: HashMap<String, PendingAnnotationPlan>,
    resource_plans: HashMap<String, ResourceImportPlan>,
    task_plans: HashMap<String, TaskWorkspacePlan>,
    import_temp_root: Option<ImportTempRoot>,
    import_plans: HashMap<String, ImportPreviewBundle>,
    agent_import_plans: HashMap<String, AgentEnhancementPreview>,
    task_import_plans: HashMap<String, PendingTaskImportPlan>,
    task_import_receipt_destinations: HashMap<String, PathBuf>,
    export_destinations: HashMap<String, PathBuf>,
    export_plans: HashMap<String, MarkdownExportPlan>,
    backup_path_capabilities: HashMap<String, BackupPathCapability>,
    backup_plans: HashMap<String, FullWorkspaceBackupPlan>,
    alternate_restore_plans: HashMap<String, AlternateRestorePlan>,
    scoped_restore_plans: HashMap<String, ScopedRestorePlan>,
    restore_drill_plans: HashMap<String, RestoreDrillPlan>,
    retention_plans: HashMap<String, SnapshotRetentionPlan>,
}

impl DesktopBackend {
    #[cfg(test)]
    pub(crate) fn new(config_dir: PathBuf) -> Self {
        let docling_installation_root = config_dir.join("docling-lite-not-installed");
        Self::new_with_docling_installation(config_dir, docling_installation_root)
    }

    pub(crate) fn new_with_docling_installation(
        config_dir: PathBuf,
        docling_installation_root: PathBuf,
    ) -> Self {
        let preferences = read_preferences(&config_dir).unwrap_or_default();
        let drafts = DraftStore::new(&config_dir);
        let agents = DesktopAgentLifecycle::new(config_dir.clone());
        Self {
            config_dir,
            docling_installation_root,
            agents,
            drafts,
            preferences,
            workspace_root: None,
            plans: HashMap::new(),
            workspace_draft_gate_tokens: HashMap::new(),
            annotation_plans: HashMap::new(),
            resource_plans: HashMap::new(),
            task_plans: HashMap::new(),
            import_temp_root: None,
            import_plans: HashMap::new(),
            agent_import_plans: HashMap::new(),
            task_import_plans: HashMap::new(),
            task_import_receipt_destinations: HashMap::new(),
            export_destinations: HashMap::new(),
            export_plans: HashMap::new(),
            backup_path_capabilities: HashMap::new(),
            backup_plans: HashMap::new(),
            alternate_restore_plans: HashMap::new(),
            scoped_restore_plans: HashMap::new(),
            restore_drill_plans: HashMap::new(),
            retention_plans: HashMap::new(),
        }
    }

    pub(crate) fn restore_workspace(&mut self) -> Result<Value, String> {
        let Some(active) = self.preferences.active_workspace.clone() else {
            return Ok(json!({
                "ok": true,
                "opened": false,
                "recents": self.preferences.recent_workspaces,
            }));
        };
        match self.open_workspace(Path::new(&active)) {
            Ok(mut payload) => {
                payload["restored"] = Value::Bool(true);
                Ok(payload)
            }
            Err(error) => {
                self.preferences.active_workspace = None;
                self.persist_preferences()?;
                Ok(json!({
                    "ok": true,
                    "opened": false,
                    "recents": self.preferences.recent_workspaces,
                    "restoreError": error,
                }))
            }
        }
    }

    pub(crate) fn open_workspace(&mut self, root: &Path) -> Result<Value, String> {
        let root =
            fs::canonicalize(root).map_err(|error| format!("无法打开所选工作区：{error}"))?;
        workspace_document_profile(&root)?;
        recover_workspace_transactions(&root).map_err(|error| error.to_string())?;
        let inventory = scan_workspace(&root);
        project_workspace_trash_state(&root)
            .map_err(|_| "所选目录不是有效的 Weftext 工作区".to_owned())?;
        weftext_core::build_workspace_navigation(&inventory)
            .map_err(|_| "所选目录不是有效的 Weftext 工作区".to_owned())?;
        let root_id = inventory
            .nodes
            .iter()
            .find(|node| node.parent_id.is_none())
            .and_then(|node| node.id)
            .ok_or("工作区根节点缺少身份")?;
        let path_text = root.to_string_lossy().into_owned();
        let remembered = self
            .preferences
            .recent_workspaces
            .iter()
            .find(|recent| recent.path == path_text)
            .and_then(|recent| recent.last_node_id.as_deref())
            .and_then(|value| value.parse::<NodeId>().ok())
            .filter(|id| inventory.nodes.iter().any(|node| node.id == Some(*id)))
            .unwrap_or(root_id);

        let import_temp_root = ImportTempRoot::initialize(self.config_dir.join("import-temp-v1"))
            .map_err(|error| format!("无法初始化导入临时区：{error}"))?;
        let import_temp_recovery = import_temp_root
            .recover_abandoned()
            .map_err(|error| format!("无法安全清理遗留导入会话：{error}"))?;

        self.agents.close_for_workspace_change()?;
        self.workspace_root = Some(root.clone());
        self.plans.clear();
        self.annotation_plans.clear();
        self.resource_plans.clear();
        self.task_plans.clear();
        self.import_temp_root = Some(import_temp_root);
        self.import_plans.clear();
        self.agent_import_plans.clear();
        self.task_import_plans.clear();
        self.task_import_receipt_destinations.clear();
        self.export_destinations.clear();
        self.export_plans.clear();
        self.backup_path_capabilities.clear();
        self.backup_plans.clear();
        self.alternate_restore_plans.clear();
        self.scoped_restore_plans.clear();
        self.restore_drill_plans.clear();
        self.retention_plans.clear();
        self.remember_workspace(&root, remembered);
        self.persist_preferences()?;
        let (search_index, search_index_warning) =
            derived_index_open_result(self.refresh_search_index(&root));

        let document = self.document_payload_with_draft(&root, remembered)?;
        let recovery = self.draft_inventory_payload(&root)?;
        Ok(json!({
            "ok": true,
            "opened": true,
            "workspacePath": path_text,
            "workspace": workspace_payload(&root)?,
            "document": document,
            "draftRecovery": recovery,
            "safeMode": self.preferences.safe_mode,
            "searchIndex": search_index,
            "searchIndexWarning": search_index_warning,
            "importTempRecovery": {
                "removedCount": import_temp_recovery.removed.len(),
                "skippedCount": import_temp_recovery.skipped.len(),
            },
            "importCapabilities": {
                "doclingLite": docling_lite_capability(&self.docling_installation_root),
            },
            "agentCapability": self.agents.capability(&root),
            "iconCatalog": weftext_core::built_in_node_icons(),
            "recents": self.preferences.recent_workspaces,
        }))
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn request(&mut self, path: &str, body: Option<Value>) -> Result<Value, String> {
        self.request_with_import_cancellation(path, body, CancellationToken::default())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn request_with_import_cancellation(
        &mut self,
        path: &str,
        body: Option<Value>,
        import_cancellation: CancellationToken,
    ) -> Result<Value, String> {
        let root = self
            .workspace_root
            .clone()
            .ok_or("请先选择一个 Weftext 工作区")?;
        let route = path.split('?').next().unwrap_or(path);
        if route.starts_with("/api/agent/") {
            return self.agents.request(route, body, &root);
        }
        match route {
            "/api/workspace" => Ok(json!({
                "ok": true,
                "workspace": workspace_payload(&root)?,
                "draftRecovery": self.draft_inventory_payload(&root)?,
            })),
            "/api/document" => {
                let node_id = query_value(path, "nodeId")
                    .map_or_else(|| root_node_id(&root), |value| parse_node_id(&value))?;
                let document = self.document_payload_with_draft(&root, node_id)?;
                if query_value(path, "remember").as_deref() != Some("false") {
                    self.remember_workspace(&root, node_id);
                    self.persist_preferences()?;
                }
                Ok(json!({"ok": true, "document": document}))
            }
            "/api/search" => {
                let query = query_value(path, "q").unwrap_or_default();
                let index = self.refresh_search_index(&root)?;
                let results = search_workspace_index(&self.search_index_path(&root)?, &query)
                    .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "results": results, "index": index}))
            }
            "/api/document/preview" => {
                let edit: EditRequest = from_body(body, "文档保存预览")?;
                let node_directory = edit_node_directory(&root, edit.node_id.as_deref())?;
                let plan = build_document_plan(&node_directory, &edit)?;
                Ok(json!({"ok": true, "plan": document_plan_payload(&plan)}))
            }
            "/api/document/commit" => {
                self.require_workspace_writes()?;
                let edit: EditRequest = from_body(body, "文档保存")?;
                let node_directory = edit_node_directory(&root, edit.node_id.as_deref())?;
                let plan = build_document_plan(&node_directory, &edit)?;
                let base_revision = plan.base_revision.clone();
                let committed = commit_document_edit(&plan).map_err(|error| error.to_string())?;
                let (search_index, search_index_warning) =
                    derived_index_result(self.refresh_search_index_invalidating(
                        &root,
                        std::iter::once(committed.node_id),
                    ));
                let draft_warning = self
                    .drafts
                    .discard(&root, root_node_id(&root)?, committed.node_id)
                    .err();
                let draft_recovery = self.draft_inventory_payload(&root).unwrap_or_else(|error| {
                    json!({
                        "drafts": [],
                        "issues": [error],
                    })
                });
                Ok(json!({
                    "ok": true,
                    "commit": {
                        "action": "document.edit",
                        "nodeId": committed.node_id,
                        "baseRevision": base_revision,
                        "revision": committed.revision,
                        "length": committed.length,
                    },
                    "draftWarning": draft_warning,
                    "draftRecovery": draft_recovery,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                    "icon": resolve_node_icon_from_source(&edit.source),
                }))
            }
            "/api/document/model" => {
                let draft: ParseRequest = from_body(body, "文档草稿解析")?;
                let analysis = analyze_document_for_profile(
                    workspace_document_profile(&root)?,
                    &draft.source,
                    workspace_presentation(&root),
                );
                Ok(json!({
                    "ok": true,
                    "profile": analysis.descriptor,
                    "model": analysis.model,
                    "view": analysis.view,
                    "properties": analyze_document_header_properties(&draft.source),
                }))
            }
            "/api/document/format" => {
                let request: FormatRequest = from_body(body, "文档格式命令")?;
                let plan = plan_document_format(
                    workspace_document_profile(&root)?,
                    &request.source,
                    request.start,
                    request.end,
                    request.command,
                )
                .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "plan": plan}))
            }
            "/api/document/property" => {
                let request: PropertyPatchRequest = from_body(body, "用户属性窄补丁")?;
                let value =
                    (!request.remove).then_some(request.value.as_deref().unwrap_or_default());
                let source = patch_document_header_property(&request.source, &request.key, value)
                    .map_err(|error| error.to_string())?;
                let validation = match (request.node_id.as_deref(), request.revision.as_deref()) {
                    (Some(node_id), Some(revision)) => {
                        let edit = EditRequest {
                            node_id: Some(node_id.to_owned()),
                            revision: revision.to_owned(),
                            source: source.clone(),
                        };
                        let directory = edit_node_directory(&root, edit.node_id.as_deref())?;
                        Some(document_plan_payload(&build_document_plan(
                            &directory, &edit,
                        )?))
                    }
                    (None, None) => None,
                    _ => return Err("用户属性补丁必须同时提供 nodeId 和 revision".to_owned()),
                };
                Ok(json!({"ok": true, "source": source, "validation": validation}))
            }
            "/api/citation/capabilities" => Ok(json!({
                "ok": true,
                "capabilities": citation_presentation_capabilities(),
            })),
            "/api/citation/validate" => {
                let index = citation_index(&root)?;
                let access = local_citation_scope(&index);
                let inventory = scan_workspace(&root);
                let mut components = Vec::with_capacity(inventory.nodes.len());
                let mut component_diagnostic_count = 0_usize;
                for node in &inventory.nodes {
                    let node_id = node.id.ok_or("工作区节点缺少身份")?;
                    let analysis = index
                        .analyze_component(node_id, &access)
                        .map_err(|error| error.to_string())?;
                    component_diagnostic_count += analysis.diagnostics.len();
                    components.push(analysis);
                }
                let valid = index.diagnostics().is_empty() && component_diagnostic_count == 0;
                Ok(json!({
                    "ok": true,
                    "validation": {
                        "valid": valid,
                        "generation": index.generation(),
                        "referenceDiagnostics": index.diagnostics(),
                        "components": components,
                    },
                }))
            }
            "/api/citation/search" => {
                let query = query_value(path, "q").unwrap_or_default();
                let limit = query_value(path, "limit").map_or(Ok(25_usize), |value| {
                    value
                        .parse::<usize>()
                        .map_err(|_| "引用检索数量必须是无符号整数".to_owned())
                })?;
                let index = citation_index(&root)?;
                let references = index
                    .search_references(&query, &local_citation_scope(&index), limit)
                    .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "query": query, "references": references}))
            }
            "/api/citation/analyze" => {
                let request: CitationAnalyzeRequest = from_body(body, "引用草稿分析")?;
                let node_id = parse_node_id(&request.node_id)?;
                let index = citation_index(&root)?;
                let access = local_citation_scope(&index);
                let analysis = index
                    .analyze_component_source(node_id, &request.source, &access)
                    .map_err(|error| error.to_string())?;
                let compilation = index
                    .collect_bibliography_input_for_source(node_id, &request.source, &access)
                    .map_err(|error| error.to_string())?;
                let presentation = present_citations(&CitationPresentationRequest::new(
                    CitationPresentationProfile::new(request.style_id, request.locale),
                    compilation,
                ));
                let (presentation, presentation_failure) = match presentation {
                    Ok(presentation) => (Some(presentation), None),
                    Err(failure) => (None, Some(failure)),
                };
                Ok(json!({
                    "ok": true,
                    "authoring": analyze_citation_authoring_source(&request.source),
                    "analysis": analysis,
                    "presentation": presentation,
                    "presentationFailure": presentation_failure,
                }))
            }
            "/api/citation/macro-edit-preview" => {
                let request: CitationMacroEditRequest = from_body(body, "引用编辑预览")?;
                let node_id = parse_node_id(&request.node_id)?;
                let index = citation_index(&root)?;
                let plan = plan_citation_macro_edit(
                    &index,
                    node_id,
                    &request.source,
                    &local_citation_scope(&index),
                    &request.target,
                    &request.intent,
                )
                .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "plan": plan}))
            }
            "/api/query/execute" => {
                let request: QueryExecuteRequest = from_body(body, "查询执行")?;
                let index =
                    QueryWorkspaceIndex::rebuild(&root).map_err(|error| error.to_string())?;
                let access = QueryAccessScope::complete(index.node_ids());
                let execution = index
                    .execute_source(
                        &request.source,
                        request.block_index,
                        &access,
                        &request.context,
                    )
                    .map_err(|error| error.to_string())?;
                Ok(json!({
                    "ok": true,
                    "valid": execution.result.is_some(),
                    "execution": execution,
                }))
            }
            "/api/task/validate" => {
                let index =
                    TaskWorkspaceIndex::rebuild(&root).map_err(|error| error.to_string())?;
                Ok(json!({
                    "ok": true,
                    "validation": {
                        "valid": index.diagnostics().is_empty(),
                        "generation": index.generation(),
                        "occurrences": index.occurrences(),
                        "diagnostics": index.diagnostics(),
                    },
                }))
            }
            "/api/task/inspect" => {
                let node_id = query_value(path, "nodeId")
                    .ok_or_else(|| "任务检查缺少 nodeId".to_owned())
                    .and_then(|value| parse_node_id(&value))?;
                node_directory_for_id(&root, node_id)?;
                let index =
                    TaskWorkspaceIndex::rebuild(&root).map_err(|error| error.to_string())?;
                let occurrences = index.occurrences_for_node(node_id).collect::<Vec<_>>();
                let diagnostics = index
                    .diagnostics()
                    .iter()
                    .filter(|diagnostic| diagnostic.node_id == node_id)
                    .collect::<Vec<_>>();
                Ok(json!({
                    "ok": true,
                    "nodeId": node_id,
                    "occurrences": occurrences,
                    "diagnostics": diagnostics,
                }))
            }
            "/api/task/edit-preview" => {
                let request: TaskEditPreviewRequest = from_body(body, "任务编辑预览")?;
                self.require_task_plan_ready(&root, &request.base_workspace_revision)?;
                let node_id = parse_node_id(&request.node_id)?;
                node_directory_for_id(&root, node_id)?;
                let base_revision = DocumentRevision::parse(&request.base_revision)
                    .map_err(|error| error.to_string())?;
                let plan = plan_task_edit_transaction(
                    &root,
                    node_id,
                    &base_revision,
                    &request.target,
                    &request.intent,
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
                self.store_task_plan(plan_id, TaskWorkspacePlan::Edit(Box::new(plan)))?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/task/recurrence-preview" => {
                let request: TaskRecurrencePreviewRequest = from_body(body, "重复任务完成预览")?;
                self.require_task_plan_ready(&root, &request.base_workspace_revision)?;
                let node_id = parse_node_id(&request.node_id)?;
                node_directory_for_id(&root, node_id)?;
                let base_revision = DocumentRevision::parse(&request.base_revision)
                    .map_err(|error| error.to_string())?;
                let plan = plan_task_recurrence_transaction(
                    &root,
                    node_id,
                    &base_revision,
                    &request.target,
                    &request.context,
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
                self.store_task_plan(plan_id, TaskWorkspacePlan::Recurrence(Box::new(plan)))?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/task/dependencies-preview" => {
                let request: TaskDependenciesPreviewRequest = from_body(body, "任务依赖替换预览")?;
                self.require_task_plan_ready(&root, &request.base_workspace_revision)?;
                let node_id = parse_node_id(&request.node_id)?;
                node_directory_for_id(&root, node_id)?;
                let base_revision = DocumentRevision::parse(&request.base_revision)
                    .map_err(|error| error.to_string())?;
                let plan = plan_task_dependency_transaction(
                    &root,
                    node_id,
                    &base_revision,
                    &request.target,
                    &request.dependencies,
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
                self.store_task_plan(plan_id, TaskWorkspacePlan::Dependencies(Box::new(plan)))?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/task/transaction/commit" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "任务操作")?;
                let request: TransactionCommitRequest = from_body(body, "任务事务提交")?;
                let plan = self
                    .task_plans
                    .remove(&request.plan_id)
                    .ok_or("任务事务预览已失效，请重新预览")?;
                let (committed, node_id, result) = match plan {
                    TaskWorkspacePlan::Edit(plan) => {
                        let node_id = plan.node_id;
                        let result = json!({
                            "task": &plan.authoring.target,
                            "assignedId": plan.authoring.assigned_id,
                        });
                        let committed = commit_task_edit_transaction(&plan)
                            .map_err(|error| error.to_string())?;
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
                let (search_index, search_index_warning) = derived_index_result(
                    self.refresh_search_index_invalidating(&root, std::iter::once(node_id)),
                );
                Ok(json!({
                    "ok": true,
                    "nodeId": node_id,
                    "commit": committed,
                    "result": result,
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/task/recover" => {
                self.require_workspace_writes()?;
                let recovery =
                    recover_workspace_transactions(&root).map_err(|error| error.to_string())?;
                self.task_plans.clear();
                Ok(json!({
                    "ok": true,
                    "recovery": recovery,
                    "workspace": workspace_payload(&root)?,
                }))
            }
            "/api/annotations" => {
                let node_id = query_value(path, "nodeId")
                    .map_or_else(|| root_node_id(&root), |value| parse_node_id(&value))?;
                let annotations = weftext_core::read_node_annotations(
                    &root,
                    node_id,
                    weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
                )
                .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "annotations": annotations}))
            }
            "/api/annotation/preview" => {
                let request: AnnotationPreviewRequest = from_body(body, "批注事务预览")?;
                let node_id = parse_node_id(&request.node_id)?;
                self.require_clean_saved_workspace(&root, "批注操作")?;
                let sidecar_snapshot = weftext_core::capture_annotation_sidecar_snapshot(
                    &root,
                    node_id,
                    weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
                )
                .map_err(|error| error.to_string())?;
                let plan = weftext_core::plan_annotation_action(
                    &root,
                    &sidecar_snapshot,
                    annotation_action(request)?,
                )
                .map_err(|error| error.to_string())?;
                let payload = json!({
                    "planId": plan.plan_id,
                    "baseRevision": plan.base_revision,
                    "action": "annotation",
                });
                self.annotation_plans.insert(
                    plan.plan_id.clone(),
                    PendingAnnotationPlan { node_id, plan },
                );
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/annotation/commit" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "批注操作")?;
                let commit: TransactionCommitRequest = from_body(body, "批注事务提交")?;
                let requested_node_id = query_value(path, "nodeId")
                    .map(|value| parse_node_id(&value))
                    .transpose()?;
                let pending = self
                    .annotation_plans
                    .get(&commit.plan_id)
                    .ok_or("批注预览已失效，请重试")?;
                if requested_node_id.is_some_and(|node_id| node_id != pending.node_id) {
                    return Err("批注提交节点与预览节点不一致".to_owned());
                }
                let pending = self
                    .annotation_plans
                    .remove(&commit.plan_id)
                    .ok_or("批注预览已失效，请重试")?;
                let invalidated = pending
                    .plan
                    .document_changes
                    .iter()
                    .map(|change| change.node_id)
                    .collect::<Vec<_>>();
                let document_changed = !invalidated.is_empty();
                let committed = commit_workspace_transaction(&pending.plan)
                    .map_err(|error| error.to_string())?;
                let annotations = weftext_core::read_node_annotations(
                    &root,
                    pending.node_id,
                    weftext_core::AnnotationReplicaCompleteness::CompleteLocalWorkspace,
                )
                .map_err(|error| error.to_string())?;
                let document = document_changed
                    .then(|| document_payload(&root, pending.node_id))
                    .transpose()?;
                let (search_index, search_index_warning) = if document_changed {
                    derived_index_result(self.refresh_search_index_invalidating(&root, invalidated))
                } else {
                    (Value::Null, Value::Null)
                };
                Ok(json!({
                    "ok": true,
                    "nodeId": pending.node_id,
                    "commit": committed,
                    "annotations": annotations,
                    "document": document,
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/chrono/preview" => {
                let request: ChronoPreviewRequest = from_body(body, "Chrono 事务预览")?;
                let date = CalendarDate::new(request.year, request.month, request.day)
                    .map_err(|error| error.to_string())?;
                let periods = parse_chrono_periods(&request.periods)?;
                let plan = weftext_core::plan_chrono_nodes(
                    &root,
                    parse_node_id(&request.chrono_root_id)?,
                    date,
                    &periods,
                )
                .map_err(|error| error.to_string())?;
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/drafts" => Ok(json!({
                "ok": true,
                "draftRecovery": self.draft_inventory_payload(&root)?,
            })),
            "/api/draft/save" => {
                let request: DraftSaveRequest = from_body(body, "恢复草稿保存")?;
                self.save_draft(&root, request)
            }
            "/api/draft/discard" => {
                let request: DraftDiscardRequest = from_body(body, "恢复草稿放弃")?;
                self.discard_draft(&root, &request.node_id)
            }
            "/api/diagnostics" => Ok(json!({
                "ok": true,
                "diagnostics": self.diagnostics_payload(&root)?,
            })),
            "/api/safe-mode" => {
                let request: SafeModeRequest = from_body(body, "安全模式设置")?;
                self.preferences.safe_mode = request.enabled;
                self.persist_preferences()?;
                Ok(json!({
                    "ok": true,
                    "safeMode": self.preferences.safe_mode,
                    "diagnostics": self.diagnostics_payload(&root)?,
                }))
            }
            "/api/backup/capabilities" => {
                let recovery = self.draft_inventory_payload(&root)?;
                let draft_count = recovery["drafts"].as_array().map_or(0, Vec::len);
                let issue_count = recovery["issues"].as_array().map_or(0, Vec::len);
                Ok(json!({
                    "ok": true,
                    "backup": {
                        "schema": "weftext.desktop-backup-capabilities.v1",
                        "documentProfile": workspace_document_profile(&root)?,
                        "managedShape": "X/X.adoc",
                        "annotations": "node_local_weftext.annotations.json",
                        "fullWorkspace": true,
                        "verify": true,
                        "protect": true,
                        "retention": true,
                        "alternateRestore": true,
                        "singleNodeRestore": true,
                        "subtreeRestore": true,
                        "restoreDrill": true,
                        "targetAuthority": "native_directory_capability",
                        "safeMode": self.preferences.safe_mode,
                        "workspaceMutationAllowed": !self.preferences.safe_mode,
                        "savedSourceSetReady": draft_count == 0 && issue_count == 0,
                        "draftCount": draft_count,
                        "recoveryIssueCount": issue_count,
                    }
                }))
            }
            "/api/backup/preview" => {
                self.require_clean_saved_workspace(&root, "完整工作区备份预览")?;
                let request: BackupPreviewRequest = from_exact_body(body, "完整工作区备份预览")?;
                let backup_parent = self.consume_backup_path_capability(
                    &request.backup_parent_capability,
                    BackupPathCapabilityKind::BackupParent,
                )?;
                let plan = plan_full_workspace_backup(&root, backup_parent)
                    .map_err(|error| error.to_string())?;
                let key = plan.plan_digest.clone();
                let payload = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
                self.store_backup_plan(key, plan)?;
                Ok(json!({"ok": true, "backup": {"stage": "preview", "plan": payload}}))
            }
            "/api/backup/commit" => {
                self.require_clean_saved_workspace(&root, "完整工作区备份提交")?;
                let request: BackupPlanCommitRequest = from_exact_body(body, "完整工作区备份提交")?;
                let plan = self
                    .backup_plans
                    .remove(&request.plan_digest)
                    .ok_or("备份预览已失效，请重新预览")?;
                let receipt =
                    commit_full_workspace_backup(&plan).map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "backup": {"stage": "committed", "receipt": receipt}}))
            }
            "/api/backup/verify" => {
                let request: BackupSnapshotRequest = from_exact_body(body, "备份校验")?;
                let snapshot = self.consume_backup_path_capability(
                    &request.snapshot_capability,
                    BackupPathCapabilityKind::Snapshot,
                )?;
                let verification =
                    verify_full_workspace_snapshot(snapshot).map_err(|error| error.to_string())?;
                Ok(
                    json!({"ok": true, "backup": {"stage": "verified", "verification": verification}}),
                )
            }
            "/api/backup/protect" => {
                let request: BackupProtectRequest = from_exact_body(body, "备份保护点")?;
                let snapshot = self.consume_backup_path_capability(
                    &request.snapshot_capability,
                    BackupPathCapabilityKind::Snapshot,
                )?;
                let protection = protect_full_workspace_snapshot(snapshot, request.label)
                    .map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "backup": {"stage": "protected", "protection": protection}}))
            }
            "/api/backup/retention/preview" => {
                let request: BackupRetentionPreviewRequest =
                    from_exact_body(body, "备份保留策略预览")?;
                let backup_parent = self.consume_backup_path_capability(
                    &request.backup_parent_capability,
                    BackupPathCapabilityKind::BackupParent,
                )?;
                let plan = plan_snapshot_retention(
                    backup_parent,
                    SnapshotRetentionPolicy {
                        keep_latest_unprotected: request.keep_latest_unprotected,
                    },
                )
                .map_err(|error| error.to_string())?;
                let key = plan.plan_digest.clone();
                let payload = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
                self.store_retention_plan(key, plan)?;
                Ok(json!({"ok": true, "backup": {"stage": "retention_preview", "plan": payload}}))
            }
            "/api/backup/retention/commit" => {
                let request: BackupPlanCommitRequest = from_exact_body(body, "备份保留策略提交")?;
                let plan = self
                    .retention_plans
                    .remove(&request.plan_digest)
                    .ok_or("保留策略预览已失效，请重新预览")?;
                let receipt =
                    commit_snapshot_retention(&plan).map_err(|error| error.to_string())?;
                Ok(
                    json!({"ok": true, "backup": {"stage": "retention_committed", "receipt": receipt}}),
                )
            }
            "/api/backup/retention/recover" => {
                let request: BackupRetentionRecoverRequest =
                    from_exact_body(body, "备份保留策略恢复")?;
                let backup_parent = self.consume_backup_path_capability(
                    &request.backup_parent_capability,
                    BackupPathCapabilityKind::BackupParent,
                )?;
                let recovery =
                    recover_snapshot_retention(backup_parent).map_err(|error| error.to_string())?;
                Ok(
                    json!({"ok": true, "backup": {"stage": "retention_recovered", "recovery": recovery}}),
                )
            }
            "/api/backup/restore/preview" => {
                let request: BackupAlternateRestorePreviewRequest =
                    from_exact_body(body, "备份 alternate restore 预览")?;
                let snapshot = self.consume_backup_path_capability(
                    &request.snapshot_capability,
                    BackupPathCapabilityKind::Snapshot,
                )?;
                let destination_parent = self.consume_backup_path_capability(
                    &request.destination_parent_capability,
                    BackupPathCapabilityKind::RestoreParent,
                )?;
                let workspace_name = root.file_name().ok_or("当前工作区根名称无效")?;
                let plan =
                    plan_alternate_restore(snapshot, destination_parent.join(workspace_name))
                        .map_err(|error| error.to_string())?;
                let key = plan.plan_digest.clone();
                let payload = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
                self.store_alternate_restore_plan(key, plan)?;
                Ok(json!({"ok": true, "backup": {"stage": "restore_preview", "plan": payload}}))
            }
            "/api/backup/restore/commit" => {
                let request: BackupPlanCommitRequest =
                    from_exact_body(body, "备份 alternate restore 提交")?;
                let plan = self
                    .alternate_restore_plans
                    .remove(&request.plan_digest)
                    .ok_or("alternate restore 预览已失效，请重新预览")?;
                let receipt = commit_alternate_restore(&plan).map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "backup": {"stage": "restored", "receipt": receipt}}))
            }
            "/api/backup/scoped-restore/preview" => {
                self.require_clean_saved_workspace(&root, "范围恢复预览")?;
                let request: BackupScopedRestorePreviewRequest =
                    from_exact_body(body, "范围恢复预览")?;
                let snapshot = self.consume_backup_path_capability(
                    &request.snapshot_capability,
                    BackupPathCapabilityKind::Snapshot,
                )?;
                let source_node_id = parse_node_id(&request.source_node_id)?;
                let destination_parent_id = parse_node_id(&request.destination_parent_id)?;
                let plan = match request.scope.as_str() {
                    "single_node" => plan_single_node_restore(
                        snapshot,
                        &root,
                        source_node_id,
                        destination_parent_id,
                        &request.destination_name,
                    ),
                    "subtree" => plan_subtree_restore(
                        snapshot,
                        &root,
                        source_node_id,
                        destination_parent_id,
                        &request.destination_name,
                    ),
                    _ => return Err("范围恢复 scope 必须是 single_node 或 subtree".to_owned()),
                }
                .map_err(|error| error.to_string())?;
                let key = plan.plan_digest.clone();
                let payload = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
                self.store_scoped_restore_plan(key, plan)?;
                Ok(
                    json!({"ok": true, "backup": {"stage": "scoped_restore_preview", "plan": payload}}),
                )
            }
            "/api/backup/scoped-restore/commit" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "范围恢复提交")?;
                let request: BackupPlanCommitRequest = from_exact_body(body, "范围恢复提交")?;
                let plan = self
                    .scoped_restore_plans
                    .remove(&request.plan_digest)
                    .ok_or("范围恢复预览已失效，请重新预览")?;
                let receipt = commit_scoped_restore(&plan).map_err(|error| error.to_string())?;
                let (search_index, search_index_warning) =
                    derived_index_result(self.refresh_search_index(&root));
                Ok(json!({
                    "ok": true,
                    "backup": {"stage": "scoped_restored", "receipt": receipt},
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/backup/drill/preview" => {
                let request: BackupDrillPreviewRequest = from_exact_body(body, "备份恢复演练预览")?;
                let snapshot = self.consume_backup_path_capability(
                    &request.snapshot_capability,
                    BackupPathCapabilityKind::Snapshot,
                )?;
                let drill_parent = self.consume_backup_path_capability(
                    &request.drill_parent_capability,
                    BackupPathCapabilityKind::DrillParent,
                )?;
                let results_parent = self.consume_backup_path_capability(
                    &request.results_parent_capability,
                    BackupPathCapabilityKind::DrillResultsParent,
                )?;
                let plan = plan_restore_drill(snapshot, drill_parent, results_parent)
                    .map_err(|error| error.to_string())?;
                let key = plan.plan_digest.clone();
                let payload = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
                self.store_restore_drill_plan(key, plan)?;
                Ok(json!({"ok": true, "backup": {"stage": "drill_preview", "plan": payload}}))
            }
            "/api/backup/drill/commit" => {
                let request: BackupPlanCommitRequest = from_exact_body(body, "备份恢复演练提交")?;
                let plan = self
                    .restore_drill_plans
                    .remove(&request.plan_digest)
                    .ok_or("恢复演练预览已失效，请重新预览")?;
                let receipt = commit_restore_drill(&plan).map_err(|error| error.to_string())?;
                Ok(json!({"ok": true, "backup": {"stage": "drill_completed", "receipt": receipt}}))
            }
            "/api/trash" => Ok(json!({
                "ok": true,
                "trash": trash_inventory_payload(&root)?,
            })),
            "/api/trash/node/preview" => {
                let request: TrashNodePreviewRequest = from_exact_body(body, "节点 Trash 预览")?;
                require_workspace_base_revision(&root, &request.base_workspace_revision)?;
                let mut plan = plan_trash_node_at(
                    &root,
                    parse_node_id(&request.node_id)?,
                    &request.trashed_at,
                )
                .map_err(|error| error.to_string())?;
                bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
                    .map_err(|error| error.to_string())?;
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/trash/resources/preview" => {
                let request: TrashResourcesPreviewRequest =
                    from_exact_body(body, "资源 Trash 批量预览")?;
                require_workspace_base_revision(&root, &request.base_workspace_revision)?;
                let mut plan =
                    plan_trash_resources_at(&root, request.resources, &request.trashed_at)
                        .map_err(|error| error.to_string())?;
                if plan.captured_target.is_some() {
                    bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
                        .map_err(|error| error.to_string())?;
                }
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/trash/restore/preview" => {
                let request: TrashRestorePreviewRequest = from_exact_body(body, "Trash 恢复预览")?;
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
                            target_node_id: parse_node_id(&target_node_id)?,
                            name,
                        },
                        resolved_by,
                    ),
                };
                require_workspace_base_revision(&root, &base_revision)?;
                let mut plan = plan_restore_trash_item(&root, item_id, mode)
                    .map_err(|error| error.to_string())?;
                bind_workspace_transaction_target_resolution(&mut plan, resolved_by)
                    .map_err(|error| error.to_string())?;
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/trash/permanent-delete/preview" => {
                let request: TrashPermanentDeletePreviewRequest =
                    from_exact_body(body, "Trash 永久删除预览")?;
                require_workspace_base_revision(&root, &request.base_workspace_revision)?;
                let preview = preview_permanent_delete_trash_items(
                    &root,
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
                let mut plan = plan_permanently_delete_trash_items(&root, &confirmation)
                    .map_err(|error| error.to_string())?;
                if plan.captured_target.is_some() {
                    bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
                        .map_err(|error| error.to_string())?;
                }
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/trash/migrate-legacy/preview" => {
                let request: TrashLegacyMigrationPreviewRequest =
                    from_exact_body(body, "旧 Trash 迁移预览")?;
                require_workspace_base_revision(&root, &request.base_workspace_revision)?;
                let backup_parent = self.consume_backup_path_capability(
                    &request.backup_parent_capability,
                    BackupPathCapabilityKind::BackupParent,
                )?;
                let backup = prepare_legacy_trash_migration_backup(&root, backup_parent)
                    .map_err(|error| error.to_string())?;
                let plan = plan_migrate_legacy_workspace_trash_at_with_backup(
                    &root,
                    &request.trashed_at,
                    &backup,
                )
                .map_err(|error| error.to_string())?;
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/workspace/action/preview" => {
                let action: WorkspaceActionRequest = from_body(body, "工作区事务预览")?;
                let mut plan = build_workspace_plan(&root, &action)?;
                if let Some(resolution) = action.resolved_by {
                    bind_workspace_transaction_target_resolution(&mut plan, resolution)
                        .map_err(|error| error.to_string())?;
                }
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/workspace/action/commit" => {
                let commit: TransactionCommitRequest = from_body(body, "工作区事务提交")?;
                let legacy_migration = self.plans.get(&commit.plan_id).is_some_and(|plan| {
                    plan.action == weftext_core::StructuralAction::TrashMigration
                });
                self.require_workspace_writes_for(legacy_migration)?;
                let plan = self
                    .plans
                    .remove(&commit.plan_id)
                    .ok_or("事务预览已失效，请重新预览")?;
                let token = self
                    .workspace_draft_gate_tokens
                    .remove(&commit.plan_id)
                    .ok_or("事务草稿授权已失效，请重新预览")?;
                let registry = self.workspace_draft_registry_view(&root)?;
                let invalidated = plan
                    .document_changes
                    .iter()
                    .map(|change| change.node_id)
                    .chain(plan.generated_node_ids.iter().copied())
                    .collect::<Vec<_>>();
                let committed =
                    commit_workspace_transaction_with_draft_gate(&plan, &token, &registry)
                        .map_err(|error| match error {
                            WorkspaceTransactionError::DraftGateBlocked(node_ids) => format!(
                                "操作范围命中 {} 个设备草稿；请保存或明确放弃后重新预览",
                                node_ids.len()
                            ),
                            other => other.to_string(),
                        })?;
                let (search_index, search_index_warning) = derived_index_result(
                    self.refresh_search_index_invalidating(&root, invalidated),
                );
                Ok(json!({
                    "ok": true,
                    "commit": committed,
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/node/metadata/preview" => {
                self.require_clean_saved_workspace(&root, "节点元数据操作")?;
                let request: NodeMetadataPreviewRequest = from_body(body, "节点元数据预览")?;
                let plan = build_node_metadata_plan(&root, request)?;
                let payload = self.store_workspace_plan(&root, plan)?;
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/resource/preview" => {
                let request: ResourcePreviewRequest = from_body(body, "资源导入预览")?;
                let plan = weftext_core::plan_import_resource(
                    &root,
                    parse_node_id(&request.node_id)?,
                    &request.name,
                    request.bytes,
                )
                .map_err(|error| error.to_string())?;
                let payload = json!({
                    "planId": plan.plan_id,
                    "nodeId": plan.node_id,
                    "name": plan.name,
                    "byteLength": plan.byte_length,
                    "baseRevision": plan.base_revision,
                });
                self.resource_plans.insert(plan.plan_id.clone(), plan);
                Ok(json!({"ok": true, "plan": payload}))
            }
            "/api/resource/commit" => {
                self.require_workspace_writes()?;
                let commit: TransactionCommitRequest = from_body(body, "资源导入提交")?;
                let plan = self
                    .resource_plans
                    .remove(&commit.plan_id)
                    .ok_or("资源预览已失效，请重新选择文件")?;
                let committed = commit_import_resource(plan).map_err(|error| error.to_string())?;
                Ok(json!({
                    "ok": true,
                    "resource": {
                        "nodeId": committed.node_id,
                        "name": committed.name,
                        "byteLength": committed.byte_length,
                        "workspaceRevision": committed.workspace_revision,
                    },
                    "workspace": workspace_payload(&root)?,
                }))
            }
            "/api/import/fake-preview" => {
                let request: ImportPreviewRequest = from_body(body, "测试导入预览")?;
                let destination =
                    PortablePath::parse(&request.destination).map_err(|error| error.to_string())?;
                let temp_root = self
                    .import_temp_root
                    .clone()
                    .ok_or("导入临时区尚未初始化，请重新打开工作区")?;
                let bundle = preview_fake_import(
                    &root,
                    temp_root,
                    request.display_name,
                    OriginClass::LocalFile,
                    request.bytes,
                    destination,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let bundle_digest = bundle.bundle_digest.to_string();
                let payload = json!({
                    "bundleDigest": bundle.bundle_digest,
                    "baseWorkspaceRevision": bundle.base_workspace_revision,
                    "source": bundle.source,
                    "probe": bundle.probe,
                    "plan": bundle.plan,
                    "document": bundle.document,
                    "proposal": bundle.proposal,
                    "proposalDigest": bundle.proposal_digest,
                    "components": bundle.components,
                    "receipt": bundle.preview_receipt,
                });
                self.store_import_plan(bundle_digest, bundle)?;
                Ok(json!({"ok": true, "import": payload}))
            }
            "/api/import/pdf-capability" => Ok(json!({
                "ok": true,
                "import": {
                    "adapter": "docling_lite",
                    "capability": docling_lite_capability(&self.docling_installation_root),
                }
            })),
            "/api/import/pdf-preview" => {
                let request: ImportPreviewRequest = from_body(body, "PDF 导入预览")?;
                let destination =
                    PortablePath::parse(&request.destination).map_err(|error| error.to_string())?;
                let temp_root = self
                    .import_temp_root
                    .clone()
                    .ok_or("导入临时区尚未初始化，请重新打开工作区")?;
                let bundle = preview_docling_pdf_import(
                    &root,
                    temp_root,
                    &self.docling_installation_root,
                    request.display_name,
                    OriginClass::LocalFile,
                    request.bytes,
                    destination,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                    import_cancellation,
                )
                .map_err(|error| error.to_string())?;
                let bundle_digest = bundle.bundle_digest.to_string();
                let payload = json!({
                    "bundleDigest": bundle.bundle_digest,
                    "baseWorkspaceRevision": bundle.base_workspace_revision,
                    "source": bundle.source,
                    "probe": bundle.probe,
                    "plan": bundle.plan,
                    "document": bundle.document,
                    "proposal": bundle.proposal,
                    "proposalDigest": bundle.proposal_digest,
                    "components": bundle.components,
                    "receipt": bundle.preview_receipt,
                });
                self.store_import_plan(bundle_digest, bundle)?;
                Ok(json!({"ok": true, "import": payload}))
            }
            "/api/import/markdown/preview" => {
                let request: MarkdownImportPreviewRequest = from_body(body, "Markdown 导入预览")?;
                let destination =
                    PortablePath::parse(&request.destination).map_err(|error| error.to_string())?;
                let temp_root = self
                    .import_temp_root
                    .clone()
                    .ok_or("导入临时区尚未初始化，请重新打开工作区")?;
                let bundle = preview_markdown_import(
                    &root,
                    temp_root,
                    request.display_name,
                    OriginClass::LocalFile,
                    request.bytes,
                    destination,
                    request.retain_original,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                    import_cancellation,
                )
                .map_err(|error| error.to_string())?;
                let bundle_digest = bundle.bundle_digest.to_string();
                let payload = json!({
                    "bundleDigest": bundle.bundle_digest,
                    "baseWorkspaceRevision": bundle.base_workspace_revision,
                    "source": bundle.source,
                    "probe": bundle.probe,
                    "plan": bundle.plan,
                    "document": bundle.document,
                    "proposal": bundle.proposal,
                    "proposalDigest": bundle.proposal_digest,
                    "components": bundle.components,
                    "receipt": bundle.preview_receipt,
                });
                self.store_import_plan(bundle_digest, bundle)?;
                Ok(json!({"ok": true, "import": payload}))
            }
            "/api/import/task/preview" => {
                if self.task_import_plans.len() >= MAX_PENDING_TASK_IMPORT_PLANS {
                    return Err(
                        "待确认的任务源集合预览过多；请先确认、恢复或重新打开工作区".to_owned()
                    );
                }
                let request: TaskImportPreviewRequest =
                    from_exact_body(body, "任务源集合导入预览")?;
                if request.profile != TASK_IMPORT_PROFILE_ID {
                    return Err("任务源集合导入必须固定 profile weftext.task-import.v1".to_owned());
                }
                if request.documents.is_empty()
                    || request.documents.len() > MAX_TASK_IMPORT_DOCUMENTS
                {
                    return Err(format!(
                        "任务源集合必须包含 1..={MAX_TASK_IMPORT_DOCUMENTS} 个文档"
                    ));
                }
                let mut total_bytes = 0_usize;
                let mut documents = Vec::with_capacity(request.documents.len());
                for document in request.documents {
                    total_bytes = total_bytes
                        .checked_add(document.bytes.len())
                        .ok_or("任务源集合字节数溢出")?;
                    if total_bytes > MAX_TASK_IMPORT_SOURCE_SET_BYTES {
                        return Err(format!(
                            "任务源集合超过 {MAX_TASK_IMPORT_SOURCE_SET_BYTES} 字节"
                        ));
                    }
                    let source = String::from_utf8(document.bytes)
                        .map_err(|_| format!("任务源文档不是 UTF-8：{}", document.locator))?;
                    documents.push(TaskImportDocumentInput {
                        locator: document.locator,
                        source,
                    });
                }
                let temp_root = self
                    .import_temp_root
                    .as_ref()
                    .ok_or("导入临时区尚未初始化，请重新打开工作区")?;
                let bundle = preview_task_import(
                    &root,
                    temp_root,
                    request.destination_parent_id,
                    request.destination_name,
                    documents,
                    request.settings,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                    &import_cancellation,
                )
                .map_err(|error| error.to_string())?;
                let review = TaskImportReview::from_preview(&bundle);
                let key = review.bundle_digest.to_string();
                let payload = json!({
                    "stage": "preview",
                    "adapter": "task_source_set",
                    "committable": bundle.task_plan.is_committable(),
                    "review": &review,
                    "bundle": &bundle,
                });
                self.store_task_import_plan(
                    key,
                    PendingTaskImportPlan {
                        bundle,
                        review,
                        receipt_path: None,
                    },
                )?;
                Ok(json!({"ok": true, "import": payload}))
            }
            "/api/import/task/commit" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "任务源集合导入提交")?;
                let request: TaskImportCommitRequest = from_body(body, "任务源集合导入提交")?;
                let key = request.review.bundle_digest.to_string();
                let pending = self
                    .task_import_plans
                    .get(&key)
                    .cloned()
                    .ok_or("任务源集合预览已失效，请重新预览")?;
                if pending.review != request.review {
                    return Err("提交确认与已存的精确任务源集合 review 不一致".to_owned());
                }
                Self::require_task_import_revision(&root, &pending.bundle)?;
                if pending.receipt_path.is_some() {
                    return Err(
                        "该任务源集合提交已经开始；请使用恢复操作完成或回滚固定计划".to_owned()
                    );
                }
                let receipt_path = self
                    .task_import_receipt_destinations
                    .remove(&request.receipt_destination_capability)
                    .ok_or("任务导入 receipt 目标授权无效或已使用；请重新通过系统选择器授权")?;
                self.task_import_plans
                    .get_mut(&key)
                    .ok_or("任务源集合预览已失效，请重新预览")?
                    .receipt_path = Some(receipt_path.clone());
                let committed = commit_previewed_task_import(
                    &root,
                    &pending.bundle,
                    &pending.review,
                    &receipt_path,
                    pending.bundle.preview_created_at.clone(),
                )
                .map_err(|error| {
                    format!("{error}；精确任务源集合 bundle/review 已保留，可尝试恢复")
                })?;
                self.task_import_plans.remove(&key);
                let invalidated = committed
                    .transaction
                    .path_changes
                    .iter()
                    .map(|change| change.node_id)
                    .collect::<Vec<_>>();
                let (search_index, search_index_warning) = derived_index_result(
                    self.refresh_search_index_invalidating(&root, invalidated),
                );
                Ok(json!({
                    "ok": true,
                    "import": {
                        "stage": "committed",
                        "adapter": "task_source_set",
                        "proposalId": committed.proposal_id,
                        "proposalDigest": committed.proposal_digest,
                        "transaction": committed.transaction,
                        "receipt": committed.receipt,
                    },
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/import/task/recover" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "任务源集合导入恢复")?;
                let request: TaskImportRecoverRequest = from_body(body, "任务源集合导入恢复")?;
                let key = request.review.bundle_digest.to_string();
                let pending = self
                    .task_import_plans
                    .get(&key)
                    .cloned()
                    .ok_or("没有可恢复的已存任务源集合计划")?;
                if pending.review != request.review {
                    return Err("恢复确认与已存的精确任务源集合 review 不一致".to_owned());
                }
                let receipt_path = pending
                    .receipt_path
                    .as_ref()
                    .ok_or("任务源集合提交尚未绑定系统选择的 receipt 目标")?;
                let recovery = recover_previewed_task_import(
                    &root,
                    &pending.bundle,
                    &pending.review,
                    receipt_path,
                    pending.bundle.preview_created_at.clone(),
                )
                .map_err(|error| error.to_string())?;
                self.task_import_plans.remove(&key);
                let invalidated = pending
                    .bundle
                    .nodes
                    .iter()
                    .map(|node| node.node_id)
                    .collect::<Vec<_>>();
                let (search_index, search_index_warning) = derived_index_result(
                    self.refresh_search_index_invalidating(&root, invalidated),
                );
                Ok(json!({
                    "ok": true,
                    "import": {
                        "stage": "task_recovered",
                        "adapter": "task_source_set",
                        "review": pending.review,
                        "recovery": recovery,
                    },
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/import/agent/prepare" => {
                let request: AgentEnhancementPrepareRequest =
                    from_body(body, "Agent 导入增强范围预览")?;
                let local_bundle = self
                    .import_plans
                    .get(&request.bundle_digest)
                    .cloned()
                    .ok_or("本地导入预览已失效，请重新提取")?;
                let preview = prepare_agent_enhancement(
                    &local_bundle,
                    AgentEvidenceSelection {
                        provider: request.provider,
                        selected_node_ids: request.selected_node_ids,
                        retention: request.retention,
                        redaction: request.redaction,
                    },
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let preview_digest = preview.preview_digest.to_string();
                let evidence_bytes = preview
                    .evidence
                    .to_bytes()
                    .map_err(|error| error.to_string())?;
                let payload = json!({
                    "previewDigest": preview.preview_digest,
                    "baseBundleDigest": preview.base_bundle_digest,
                    "selection": preview.selection,
                    "evidenceDigest": preview.evidence.evidence_digest,
                    "evidenceByteLength": evidence_bytes.len(),
                    "evidence": preview.evidence,
                    "authorizedPlan": preview.authorized_bundle.plan,
                    "proposal": preview.authorized_bundle.proposal,
                    "receipt": preview.authorized_bundle.preview_receipt,
                    "networkExecuted": false,
                    "requiresExplicitEgressApproval": true,
                });
                self.store_agent_import_plan(preview_digest, preview)?;
                Ok(json!({"ok": true, "agentEnhancement": payload}))
            }
            "/api/import/agent/apply-approved-patch" => {
                let request: AgentPatchApplyRequest = from_body(body, "Agent typed IR patch 应用")?;
                if !request.egress_approved {
                    return Err("Agent 增强需要本次显式出站审批；预览仍可重新确认".to_owned());
                }
                let approved = self
                    .agent_import_plans
                    .get(&request.preview_digest)
                    .cloned()
                    .ok_or("Agent 增强范围预览已失效，请重新选择证据")?;
                let enhanced = apply_approved_agent_patch(
                    &approved,
                    &request.patch,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let base_digest = approved.base_bundle_digest.to_string();
                if !self.import_plans.contains_key(&base_digest)
                    && self.import_plans.len() >= MAX_PENDING_IMPORT_PLANS
                {
                    return Err("待确认的导入预览过多；请先确认或重新打开工作区".to_owned());
                }
                let bundle_digest = enhanced.bundle_digest.to_string();
                let payload = json!({
                    "bundleDigest": enhanced.bundle_digest,
                    "baseWorkspaceRevision": enhanced.base_workspace_revision,
                    "source": enhanced.source,
                    "probe": enhanced.probe,
                    "plan": enhanced.plan,
                    "document": enhanced.document,
                    "proposal": enhanced.proposal,
                    "proposalDigest": enhanced.proposal_digest,
                    "components": enhanced.components,
                    "receipt": enhanced.preview_receipt,
                    "requiresFinalCommitApproval": true,
                });
                self.agent_import_plans.remove(&request.preview_digest);
                self.import_plans.remove(&base_digest);
                self.import_plans.insert(bundle_digest, enhanced);
                Ok(json!({"ok": true, "import": payload}))
            }
            "/api/import/commit" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "导入提交")?;
                let request: ImportCommitRequest = from_body(body, "导入提交")?;
                let bundle = self
                    .import_plans
                    .remove(&request.bundle_digest)
                    .ok_or("导入预览已失效，请重新预览")?;
                let committed = commit_previewed_import(
                    &root,
                    &bundle,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let invalidated = committed
                    .transaction
                    .path_changes
                    .iter()
                    .map(|change| change.node_id)
                    .collect::<Vec<_>>();
                let (search_index, search_index_warning) = derived_index_result(
                    self.refresh_search_index_invalidating(&root, invalidated),
                );
                Ok(json!({
                    "ok": true,
                    "import": {
                        "proposalId": committed.proposal_id,
                        "proposalDigest": committed.proposal_digest,
                        "transaction": committed.transaction,
                        "receipt": committed.receipt,
                    },
                    "workspace": workspace_payload(&root)?,
                    "searchIndex": search_index,
                    "searchIndexWarning": search_index_warning,
                }))
            }
            "/api/export/markdown/preview" => {
                self.require_clean_saved_workspace(&root, "Markdown 导出预览")?;
                let request: MarkdownExportPreviewRequest = from_body(body, "Markdown 导出预览")?;
                let destination = self
                    .export_destinations
                    .remove(&request.destination_capability)
                    .ok_or("Markdown 导出目标授权无效或已使用；请重新通过系统选择器授权")?;
                let plan = preview_markdown_export(
                    &root,
                    parse_node_id(&request.node_id)?,
                    destination,
                    request.metadata_policy,
                )
                .map_err(|error| error.to_string())?;
                let payload = json!({
                    "stage": "preview",
                    "format": "markdown_compatibility",
                    "plan": plan,
                });
                self.store_export_plan(plan.plan_id.clone(), plan)?;
                Ok(json!({"ok": true, "export": payload}))
            }
            "/api/export/commit" => {
                self.require_workspace_writes()?;
                self.require_clean_saved_workspace(&root, "Markdown 导出提交")?;
                let request: MarkdownExportCommitRequest = from_body(body, "Markdown 导出提交")?;
                let plan = self
                    .export_plans
                    .remove(&request.plan_id)
                    .ok_or("Markdown 导出预览已失效，请重新预览")?;
                let receipt = commit_markdown_export(
                    &root,
                    &plan,
                    rfc3339_utc_now().map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                Ok(json!({
                    "ok": true,
                    "export": {
                        "stage": "committed",
                        "format": "markdown_compatibility",
                        "receipt": receipt,
                    }
                }))
            }
            _ => Err("未知的 Desktop Core 请求".to_owned()),
        }
    }

    fn document_payload_with_draft(&self, root: &Path, node_id: NodeId) -> Result<Value, String> {
        let mut payload = document_payload(root, node_id)?;
        let workspace_id = root_node_id(root)?;
        match self.drafts.load(root, workspace_id, node_id) {
            Ok(Some(draft)) => {
                let snapshot = read_snapshot_for_id(root, node_id)?;
                if draft.source == snapshot.source {
                    self.drafts.discard(root, workspace_id, node_id)?;
                } else {
                    payload["recoveryDraft"] = draft_payload(
                        &draft,
                        &snapshot,
                        &node_name(&snapshot.node_directory),
                        true,
                    );
                }
            }
            Ok(None) => {}
            Err(error) => payload["recoveryIssue"] = Value::String(error),
        }
        Ok(payload)
    }

    fn draft_inventory_payload(&self, root: &Path) -> Result<Value, String> {
        let workspace_id = root_node_id(root)?;
        let DraftInventory { drafts, mut issues } = self.drafts.list(root, workspace_id)?;
        let inventory = scan_workspace(root);
        let mut summaries = Vec::new();
        for draft in drafts {
            let Some(node) = inventory
                .nodes
                .iter()
                .find(|node| node.id == Some(draft.node_id))
            else {
                issues.push(format!(
                    "节点 {} 已不在当前工作区，恢复草稿仍被保留",
                    draft.node_id
                ));
                continue;
            };
            let snapshot = match read_node_document(&node.path) {
                Ok(value) => value,
                Err(error) => {
                    issues.push(format!(
                        "节点 {} 的恢复草稿无法核对：{error}",
                        draft.node_id
                    ));
                    continue;
                }
            };
            if draft.source == snapshot.source {
                self.drafts.discard(root, workspace_id, draft.node_id)?;
                continue;
            }
            summaries.push(draft_payload(&draft, &snapshot, &node.name, false));
        }
        Ok(json!({
            "drafts": summaries,
            "issues": issues,
        }))
    }

    fn save_draft(&self, root: &Path, request: DraftSaveRequest) -> Result<Value, String> {
        let node_id = parse_node_id(&request.node_id)?;
        let base_revision =
            DocumentRevision::parse(&request.revision).map_err(|error| error.to_string())?;
        let snapshot = read_snapshot_for_id(root, node_id)?;
        let workspace_id = root_node_id(root)?;
        if request.source == snapshot.source {
            self.drafts.discard(root, workspace_id, node_id)?;
            return Ok(json!({
                "ok": true,
                "clean": true,
                "draftRecovery": self.draft_inventory_payload(root)?,
            }));
        }
        let draft = self
            .drafts
            .save(root, workspace_id, node_id, base_revision, request.source)?;
        Ok(json!({
            "ok": true,
            "clean": false,
            "draft": draft_payload(
                &draft,
                &snapshot,
                &node_name(&snapshot.node_directory),
                true,
            ),
            "draftRecovery": self.draft_inventory_payload(root)?,
        }))
    }

    fn discard_draft(&self, root: &Path, node_id: &str) -> Result<Value, String> {
        let node_id = parse_node_id(node_id)?;
        let workspace_id = root_node_id(root)?;
        let removed = self.drafts.discard(root, workspace_id, node_id)?;
        Ok(json!({
            "ok": true,
            "removed": removed,
            "draftRecovery": self.draft_inventory_payload(root)?,
        }))
    }

    fn require_workspace_writes(&self) -> Result<(), String> {
        self.require_workspace_writes_for(false)
    }

    fn require_workspace_writes_for(
        &self,
        allow_legacy_trash_migration: bool,
    ) -> Result<(), String> {
        if self.preferences.safe_mode {
            return Err("安全模式已启用；工作区提交已暂停，设备恢复草稿仍会保留".to_owned());
        }
        if let Some(root) = &self.workspace_root {
            let state = project_workspace_trash_state(root).map_err(|error| error.to_string())?;
            if state.reconciliation_required {
                return Err("Trash 需要协调；当前工作区仅可只读使用".to_owned());
            }
            if state.legacy_migration_required && !allow_legacy_trash_migration {
                return Err("旧 Trash 格式必须先完成显式迁移；当前工作区仅可只读使用".to_owned());
            }
        }
        Ok(())
    }

    fn store_workspace_plan(
        &mut self,
        root: &Path,
        plan: WorkspaceTransactionPlan,
    ) -> Result<Value, String> {
        if self.plans.len() >= MAX_PENDING_WORKSPACE_PLANS {
            return Err("待确认的工作区事务预览过多；请先确认或重新打开工作区".to_owned());
        }
        let registry = self.workspace_draft_registry_view(root)?;
        let gate = preview_workspace_transaction_draft_gate(&plan, &registry)
            .map_err(|error| error.to_string())?;
        if !gate.blocking_dirty_node_ids.is_empty() {
            return Err(format!(
                "操作范围命中 {} 个设备草稿；请保存或明确放弃后重新预览",
                gate.blocking_dirty_node_ids.len()
            ));
        }
        let token = gate
            .executable_token
            .ok_or("Core 未签发草稿门禁提交授权；请重新预览")?;
        let payload = transaction_plan_payload(&plan);
        self.workspace_draft_gate_tokens
            .insert(plan.plan_id.clone(), token);
        self.plans.insert(plan.plan_id.clone(), plan);
        Ok(payload)
    }

    fn store_task_plan(&mut self, plan_id: String, plan: TaskWorkspacePlan) -> Result<(), String> {
        if self.task_plans.len() >= MAX_PENDING_TASK_PLANS {
            return Err("待确认的任务预览过多；请先确认、恢复或重新打开工作区".to_owned());
        }
        self.task_plans.insert(plan_id, plan);
        Ok(())
    }

    fn store_import_plan(
        &mut self,
        bundle_digest: String,
        bundle: ImportPreviewBundle,
    ) -> Result<(), String> {
        validate_preview_bundle(&bundle).map_err(|error| error.to_string())?;
        if self.import_plans.len() >= MAX_PENDING_IMPORT_PLANS {
            return Err("待确认的导入预览过多；请先确认或重新打开工作区".to_owned());
        }
        self.import_plans.insert(bundle_digest, bundle);
        Ok(())
    }

    fn store_agent_import_plan(
        &mut self,
        preview_digest: String,
        preview: AgentEnhancementPreview,
    ) -> Result<(), String> {
        if self.agent_import_plans.len() >= MAX_PENDING_AGENT_IMPORT_PLANS {
            return Err("待确认的 Agent 证据范围过多；请先确认或重新打开工作区".to_owned());
        }
        self.agent_import_plans.insert(preview_digest, preview);
        Ok(())
    }

    fn store_task_import_plan(
        &mut self,
        bundle_digest: String,
        plan: PendingTaskImportPlan,
    ) -> Result<(), String> {
        if self.task_import_plans.len() >= MAX_PENDING_TASK_IMPORT_PLANS {
            return Err("待确认的任务源集合预览过多；请先确认、恢复或重新打开工作区".to_owned());
        }
        self.task_import_plans.insert(bundle_digest, plan);
        Ok(())
    }

    fn store_export_plan(
        &mut self,
        plan_id: String,
        plan: MarkdownExportPlan,
    ) -> Result<(), String> {
        if self.export_plans.len() >= MAX_PENDING_EXPORT_PLANS {
            return Err("待确认的导出预览过多；请先确认或重新打开工作区".to_owned());
        }
        self.export_plans.insert(plan_id, plan);
        Ok(())
    }

    fn require_backup_plan_capacity(&self) -> Result<(), String> {
        let pending = self
            .backup_plans
            .len()
            .saturating_add(self.alternate_restore_plans.len())
            .saturating_add(self.scoped_restore_plans.len())
            .saturating_add(self.restore_drill_plans.len())
            .saturating_add(self.retention_plans.len());
        if pending >= MAX_PENDING_BACKUP_PLANS {
            Err("待确认的备份或恢复预览过多；请先确认或重新打开工作区".to_owned())
        } else {
            Ok(())
        }
    }

    fn store_backup_plan(
        &mut self,
        plan_digest: String,
        plan: FullWorkspaceBackupPlan,
    ) -> Result<(), String> {
        self.require_backup_plan_capacity()?;
        self.backup_plans.insert(plan_digest, plan);
        Ok(())
    }

    fn store_alternate_restore_plan(
        &mut self,
        plan_digest: String,
        plan: AlternateRestorePlan,
    ) -> Result<(), String> {
        self.require_backup_plan_capacity()?;
        self.alternate_restore_plans.insert(plan_digest, plan);
        Ok(())
    }

    fn store_scoped_restore_plan(
        &mut self,
        plan_digest: String,
        plan: ScopedRestorePlan,
    ) -> Result<(), String> {
        self.require_backup_plan_capacity()?;
        self.scoped_restore_plans.insert(plan_digest, plan);
        Ok(())
    }

    fn store_restore_drill_plan(
        &mut self,
        plan_digest: String,
        plan: RestoreDrillPlan,
    ) -> Result<(), String> {
        self.require_backup_plan_capacity()?;
        self.restore_drill_plans.insert(plan_digest, plan);
        Ok(())
    }

    fn store_retention_plan(
        &mut self,
        plan_digest: String,
        plan: SnapshotRetentionPlan,
    ) -> Result<(), String> {
        self.require_backup_plan_capacity()?;
        self.retention_plans.insert(plan_digest, plan);
        Ok(())
    }

    fn consume_backup_path_capability(
        &mut self,
        capability: &str,
        expected_kind: BackupPathCapabilityKind,
    ) -> Result<PathBuf, String> {
        let selected = self
            .backup_path_capabilities
            .remove(capability)
            .ok_or("备份目标授权无效或已使用；请重新通过系统选择器授权")?;
        if selected.kind != expected_kind {
            return Err("备份目标授权类型与本次操作不匹配".to_owned());
        }
        Ok(selected.path)
    }

    pub(crate) fn register_backup_path_capability(
        &mut self,
        kind: BackupPathCapabilityKind,
        selected: &Path,
    ) -> Result<Value, String> {
        if self.workspace_root.is_none() {
            return Err("请先选择一个 Weftext 工作区".to_owned());
        }
        if !selected.is_absolute() {
            return Err("系统选择器没有返回绝对备份目录".to_owned());
        }
        let metadata = fs::symlink_metadata(selected)
            .map_err(|error| format!("无法检查所选备份目录：{error}"))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("备份目录授权必须指向真实的现有目录".to_owned());
        }
        let selected =
            fs::canonicalize(selected).map_err(|error| format!("无法解析所选备份目录：{error}"))?;
        let capability = format!("{}-{}", kind.token_prefix(), Uuid::new_v4());
        let display_path = selected.to_string_lossy().into_owned();
        if self.backup_path_capabilities.len() >= MAX_PENDING_BACKUP_PLANS {
            return Err("未使用的备份目录授权过多；请重新打开工作区后再选择".to_owned());
        }
        self.backup_path_capabilities.insert(
            capability.clone(),
            BackupPathCapability {
                kind,
                path: selected,
            },
        );
        Ok(json!({
            "capability": capability,
            "kind": kind,
            "displayPath": display_path,
        }))
    }

    pub(crate) fn register_markdown_export_destination(
        &mut self,
        destination: PathBuf,
    ) -> Result<Value, String> {
        if self.workspace_root.is_none() {
            return Err("请先选择一个 Weftext 工作区".to_owned());
        }
        if !destination.is_absolute() {
            return Err("系统选择器没有返回绝对 Markdown 目标路径".to_owned());
        }
        let capability = format!("markdown-export-destination-{}", uuid::Uuid::new_v4());
        let display_path = destination.to_string_lossy().into_owned();
        self.export_destinations.clear();
        self.export_destinations
            .insert(capability.clone(), destination);
        Ok(json!({
            "capability": capability,
            "displayPath": display_path,
        }))
    }

    pub(crate) fn register_task_import_receipt_destination(
        &mut self,
        destination: PathBuf,
    ) -> Result<Value, String> {
        if self.workspace_root.is_none() {
            return Err("请先选择一个 Weftext 工作区".to_owned());
        }
        if !destination.is_absolute() {
            return Err("系统选择器没有返回绝对 task import receipt 目标路径".to_owned());
        }
        if destination
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("json"))
        {
            return Err("task import receipt 必须是新的 .json 文件".to_owned());
        }
        match fs::symlink_metadata(&destination) {
            Ok(_) => return Err("task import receipt 目标已经存在；请选择新的文件".to_owned()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("无法检查 task import receipt 目标：{error}")),
        }
        let capability = format!("task-import-receipt-destination-{}", uuid::Uuid::new_v4());
        let display_path = destination.to_string_lossy().into_owned();
        self.task_import_receipt_destinations.clear();
        self.task_import_receipt_destinations
            .insert(capability.clone(), destination);
        Ok(json!({
            "capability": capability,
            "displayPath": display_path,
        }))
    }

    fn require_clean_saved_workspace(&self, root: &Path, operation: &str) -> Result<(), String> {
        let recovery = self.draft_inventory_payload(root)?;
        let has_drafts = recovery["drafts"]
            .as_array()
            .is_some_and(|drafts| !drafts.is_empty());
        let has_issues = recovery["issues"]
            .as_array()
            .is_some_and(|issues| !issues.is_empty());
        if has_drafts || has_issues {
            Err(format!(
                "{operation}需要完整已保存源集；请先保存、放弃或恢复所有设备草稿"
            ))
        } else {
            Ok(())
        }
    }

    fn workspace_draft_registry_view(
        &self,
        root: &Path,
    ) -> Result<WorkspaceDraftRegistryView, String> {
        let recovery = self.draft_inventory_payload(root)?;
        if recovery["issues"]
            .as_array()
            .is_some_and(|issues| !issues.is_empty())
        {
            return Err(
                "无法核对完整设备草稿登记；请先处理草稿恢复中心的问题并重新预览".to_owned(),
            );
        }
        let dirty_node_ids = recovery["drafts"]
            .as_array()
            .ok_or("设备草稿登记返回了无效 drafts 集合")?
            .iter()
            .map(|draft| {
                draft["nodeId"]
                    .as_str()
                    .ok_or_else(|| "设备草稿登记缺少 nodeId".to_owned())
                    .and_then(parse_node_id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        WorkspaceDraftRegistryView::new(
            format!("desktop-draft-store:{}", Uuid::new_v4()),
            dirty_node_ids,
        )
        .map_err(|error| error.to_string())
    }

    fn require_task_plan_ready(
        &self,
        root: &Path,
        expected_workspace_revision: &str,
    ) -> Result<(), String> {
        self.require_clean_saved_workspace(root, "任务操作")?;
        let expected = WorkspaceRevision::parse(expected_workspace_revision)
            .map_err(|error| error.to_string())?;
        let actual = read_workspace_revision(root).map_err(|error| error.to_string())?;
        if expected == actual {
            Ok(())
        } else {
            Err(format!(
                "工作区已更改；任务预览基线为 {expected}，当前修订为 {actual}"
            ))
        }
    }

    fn require_task_import_revision(
        root: &Path,
        bundle: &TaskImportPreviewBundle,
    ) -> Result<(), String> {
        let actual = read_workspace_revision(root).map_err(|error| error.to_string())?;
        if actual == bundle.base_workspace_revision {
            Ok(())
        } else {
            Err(format!(
                "工作区已更改；任务源集合预览基线为 {}，当前修订为 {actual}",
                bundle.base_workspace_revision
            ))
        }
    }

    fn diagnostics_payload(&self, root: &Path) -> Result<Value, String> {
        let inventory = scan_workspace(root);
        let recovery = self.draft_inventory_payload(root)?;
        let issue_codes = inventory
            .issues
            .iter()
            .map(|issue| format!("{:?}", issue.code))
            .collect::<Vec<_>>();
        Ok(json!({
            "safeMode": self.preferences.safe_mode,
            "workspaceValid": inventory.is_valid(),
            "nodeCount": inventory.nodes.len(),
            "inventoryIssueCodes": issue_codes,
            "recoveryDraftCount": recovery["drafts"].as_array().map_or(0, Vec::len),
            "recoveryIssueCount": recovery["issues"].as_array().map_or(0, Vec::len),
            "index": "external-incremental-v1",
            "pathsRedacted": true,
            "documentBodiesIncluded": false,
        }))
    }

    fn remember_workspace(&mut self, root: &Path, selected: NodeId) {
        let path = root.to_string_lossy().into_owned();
        self.preferences
            .recent_workspaces
            .retain(|recent| recent.path != path);
        self.preferences.recent_workspaces.insert(
            0,
            RecentWorkspace {
                path: path.clone(),
                last_node_id: Some(selected.to_string()),
            },
        );
        self.preferences
            .recent_workspaces
            .truncate(MAX_RECENT_WORKSPACES);
        self.preferences.active_workspace = Some(path);
    }

    fn persist_preferences(&self) -> Result<(), String> {
        fs::create_dir_all(&self.config_dir)
            .map_err(|error| format!("无法创建桌面设置目录：{error}"))?;
        let bytes = serde_json::to_vec_pretty(&self.preferences)
            .map_err(|error| format!("无法保存桌面会话：{error}"))?;
        let mut staged = Builder::new()
            .prefix(".weftext-desktop-")
            .tempfile_in(&self.config_dir)
            .map_err(|error| format!("无法暂存桌面会话：{error}"))?;
        staged
            .write_all(&bytes)
            .and_then(|()| staged.flush())
            .map_err(|error| format!("无法写入桌面会话：{error}"))?;
        staged
            .persist(self.config_dir.join(SETTINGS_FILE))
            .map_err(|error| format!("无法提交桌面会话：{}", error.error))?;
        Ok(())
    }

    fn search_index_path(&self, root: &Path) -> Result<PathBuf, String> {
        let workspace_id = root_node_id(root)?;
        Ok(self
            .config_dir
            .join("search-indexes")
            .join(format!("{workspace_id}.json")))
    }

    fn refresh_search_index(&self, root: &Path) -> Result<Value, String> {
        let stats = refresh_workspace_search_index(root, &self.search_index_path(root)?)
            .map_err(|error| error.to_string())?;
        serde_json::to_value(stats).map_err(|error| error.to_string())
    }

    fn refresh_search_index_invalidating(
        &self,
        root: &Path,
        node_ids: impl IntoIterator<Item = NodeId>,
    ) -> Result<Value, String> {
        let stats = refresh_workspace_search_index_invalidating(
            root,
            &self.search_index_path(root)?,
            node_ids,
        )
        .map_err(|error| error.to_string())?;
        serde_json::to_value(stats).map_err(|error| error.to_string())
    }
}

fn read_preferences(config_dir: &Path) -> Result<DesktopPreferences, String> {
    let path = config_dir.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(DesktopPreferences::default());
    }
    let bytes = fs::read(&path).map_err(|error| format!("无法读取桌面会话：{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("桌面会话格式无效：{error}"))
}

fn from_body<T>(body: Option<Value>, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(body.ok_or_else(|| format!("{label}缺少请求内容"))?)
        .map_err(|error| format!("{label}内容无效：{error}"))
}

fn from_exact_body<T>(body: Option<Value>, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let value = body.ok_or_else(|| format!("{label}缺少请求内容"))?;
    let parsed: T = serde_json::from_value(value.clone())
        .map_err(|error| format!("{label}内容无效：{error}"))?;
    let normalized = serde_json::to_value(&parsed)
        .map_err(|error| format!("{label}无法固定 typed 内容：{error}"))?;
    if normalized != value {
        return Err(format!("{label}包含 exact typed 契约之外的字段或值"));
    }
    Ok(parsed)
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

fn workspace_payload(root: &Path) -> Result<Value, String> {
    workspace_document_profile(root)?;
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
        "rootNodeId": root_node_id(root)?,
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

fn document_payload(root: &Path, node_id: NodeId) -> Result<Value, String> {
    let directory = node_directory_for_id(root, node_id)?;
    let snapshot = read_node_document(&directory).map_err(|error| error.to_string())?;
    let analysis = analyze_document_for_profile(
        snapshot.profile,
        &snapshot.source,
        workspace_presentation(root),
    );
    let metadata_scope = if directory == root {
        weftext_core::NodeMetadataScope::WorkspaceRoot
    } else {
        weftext_core::NodeMetadataScope::Node
    };
    Ok(json!({
        "nodeId": snapshot.node_id,
        "name": node_name(&directory),
        "revision": snapshot.revision,
        "length": snapshot.source.len(),
        "source": snapshot.source,
        "profile": analysis.descriptor,
        "model": analysis.model,
        "view": analysis.view,
        "metadata": project_node_metadata(&snapshot.source, metadata_scope)
            .map_err(|error| error.to_string())?,
        "properties": analyze_document_header_properties(&snapshot.source),
    }))
}

fn read_snapshot_for_id(root: &Path, node_id: NodeId) -> Result<DocumentSnapshot, String> {
    let directory = node_directory_for_id(root, node_id)?;
    read_node_document(&directory).map_err(|error| error.to_string())
}

fn draft_payload(
    draft: &StoredDraft,
    snapshot: &DocumentSnapshot,
    name: &str,
    include_source: bool,
) -> Value {
    let mut payload = json!({
        "nodeId": draft.node_id,
        "name": name,
        "baseRevision": draft.base_revision,
        "currentRevision": snapshot.revision,
        "profile": draft.document_profile,
        "stale": draft.base_revision != snapshot.revision,
        "length": draft.source.len(),
        "updatedAtUnixMs": draft.updated_at_unix_ms,
    });
    if include_source {
        payload["source"] = Value::String(draft.source.clone());
    }
    payload
}

fn document_plan_payload(plan: &DocumentEditPlan) -> Value {
    json!({
        "action": "document.edit",
        "nodeId": plan.node_id,
        "baseRevision": plan.base_revision,
        "nextRevision": plan.next_revision,
        "oldLength": plan.old_length,
        "newLength": plan.new_length,
        "changed": plan.changed,
    })
}

fn derived_index_result(result: Result<Value, String>) -> (Value, Value) {
    match result {
        Ok(index) => (index, Value::Null),
        Err(message) => (
            Value::Null,
            json!({
                "code": "derived_search_index_refresh_failed",
                "message": message,
                "rebuildRequired": true,
                "authoritativeCommitSucceeded": true,
            }),
        ),
    }
}

fn derived_index_open_result(result: Result<Value, String>) -> (Value, Value) {
    match result {
        Ok(index) => (index, Value::Null),
        Err(message) => (
            Value::Null,
            json!({
                "code": "derived_search_index_refresh_failed",
                "message": message,
                "rebuildRequired": true,
                "workspaceOpenSucceeded": true,
            }),
        ),
    }
}

fn build_document_plan(
    node_directory: &Path,
    edit: &EditRequest,
) -> Result<DocumentEditPlan, String> {
    let revision = DocumentRevision::parse(&edit.revision).map_err(|error| error.to_string())?;
    let snapshot = read_node_document(node_directory).map_err(|error| error.to_string())?;
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
    .map_err(|error| error.to_string())
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
    if workspace_document_format(root).generation == WorkspaceDocumentGeneration::AsciiDocV1 {
        Ok(DocumentProfileId::AsciiDocV1)
    } else {
        Err("工作区必须包含精确的 weftext.asciidoc.v1 格式标记".to_owned())
    }
}

fn root_node_id(root: &Path) -> Result<NodeId, String> {
    scan_workspace(root)
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id)
        .ok_or_else(|| "工作区根节点不可用".to_owned())
}

fn node_directory_for_id(root: &Path, node_id: NodeId) -> Result<PathBuf, String> {
    scan_workspace(root)
        .nodes
        .into_iter()
        .find(|node| node.id == Some(node_id))
        .map(|node| node.path)
        .ok_or_else(|| "工作区节点不可用".to_owned())
}

fn edit_node_directory(root: &Path, value: Option<&str>) -> Result<PathBuf, String> {
    value.map_or_else(
        || Ok(root.to_path_buf()),
        |raw| node_directory_for_id(root, parse_node_id(raw)?),
    )
}

fn parse_node_id(value: &str) -> Result<NodeId, String> {
    value
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
            "工作区已更改；Trash 预览基线为 {expected}，当前修订为 {actual}"
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
        Err("永久删除确认与当前 item ID、payload 摘要或字节数不一致".to_owned())
    }
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
        _ => Err("未知的批注事务".to_owned()),
    }
}

fn create_annotation_action(
    mut request: AnnotationPreviewRequest,
) -> Result<AnnotationAction, String> {
    if request.annotation_id.is_some() || request.message_id.is_some() {
        return Err("创建批注不能包含 annotationId 或 messageId".to_owned());
    }
    let kind = request.kind.ok_or_else(|| "创建批注缺少 kind".to_owned())?;
    let target = annotation_target(request.target.take())?;
    if request
        .appearance
        .as_ref()
        .is_some_and(|appearance| appearance.mark == AnnotationMark::None)
    {
        return Err("创建批注不能使用 appearance.mark=none".to_owned());
    }
    let appearance = annotation_appearance(request.appearance.take())?;
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
        return Err("回复批注包含不适用于 reply 的字段".to_owned());
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
        return Err("edit_message 不接受 authorName；消息作者以 authorId 校验".to_owned());
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
        return Err("外观更新包含不适用于 set_appearance 的字段".to_owned());
    }
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    let appearance = request
        .appearance
        .ok_or_else(|| "set_appearance 缺少 appearance".to_owned())?;
    let appearance = annotation_appearance(Some(appearance))?;
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
        return Err("标签更新包含不适用于 set_labels 的字段".to_owned());
    }
    let annotation_id = annotation_request_id(request.annotation_id.as_deref(), "annotationId")?;
    let labels = request
        .labels
        .ok_or_else(|| "set_labels 缺少 labels".to_owned())?;
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
        return Err("该批注动作不能包含 messageId 或 bodySource".to_owned());
    }
    ensure_annotation_fields_absent(&request, false, "该批注动作")?;
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
        return Err(format!("{action} 包含不适用的批注字段"));
    }
    Ok(())
}

fn annotation_target(
    target: Option<AnnotationTargetRequest>,
) -> Result<AnnotationTargetIntent, String> {
    target
        .map(AnnotationTargetIntent::from)
        .ok_or_else(|| "创建批注缺少 target".to_owned())
}

fn annotation_appearance(
    appearance: Option<AnnotationAppearanceRequest>,
) -> Result<Option<AnnotationAppearance>, String> {
    if let Some(appearance) = appearance {
        if appearance.mark == AnnotationMark::None {
            if appearance.theme.is_some() {
                return Err("清除批注外观时不能同时提供 theme".to_owned());
            }
            return Ok(None);
        }
        let color = appearance
            .theme
            .ok_or_else(|| "批注 appearance 缺少 theme".to_owned())?;
        return Ok(Some(AnnotationAppearance {
            mark: appearance.mark,
            color,
        }));
    }
    Ok(None)
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
            Err("comment 批注需要正文且不能包含 suggestedSource".to_owned())
        }
        AnnotationKind::Mark if appearance.is_none() || suggested_source.is_some() => {
            Err("mark 批注需要外观且不能包含 suggestedSource".to_owned())
        }
        AnnotationKind::SuggestionInsert
            if !matches!(target, AnnotationTargetIntent::InsertionPoint { .. })
                || suggested_source.is_none_or(str::is_empty) =>
        {
            Err("suggestion_insert 需要 insertion_point target 和 suggestedSource".to_owned())
        }
        AnnotationKind::SuggestionDelete
            if !matches!(target, AnnotationTargetIntent::TextRange { .. })
                || suggested_source.is_some() =>
        {
            Err("suggestion_delete 需要 text_range target 且不能包含 suggestedSource".to_owned())
        }
        AnnotationKind::Comment
        | AnnotationKind::Mark
        | AnnotationKind::SuggestionInsert
        | AnnotationKind::SuggestionDelete => Ok(()),
    }
}

fn required_annotation_body(value: Option<String>) -> Result<String, String> {
    let value = value.ok_or_else(|| "批注事务缺少 bodySource".to_owned())?;
    validate_optional_annotation_body(Some(&value))?;
    Ok(value)
}

fn validate_optional_annotation_body(value: Option<&str>) -> Result<(), String> {
    if value.is_some_and(|body| body.trim().is_empty()) {
        Err("批注 bodySource 不能为空".to_owned())
    } else {
        Ok(())
    }
}

fn annotation_actor(request: &AnnotationPreviewRequest) -> Result<(uuid::Uuid, String), String> {
    let author_id = annotation_request_id(request.author_id.as_deref(), "authorId")?;
    let author_name = request
        .author_name
        .as_deref()
        .ok_or_else(|| "批注事务缺少 authorName".to_owned())?
        .trim()
        .to_owned();
    if author_name.is_empty() {
        return Err("批注 authorName 不能为空".to_owned());
    }
    Ok((author_id, author_name))
}

fn annotation_request_id(value: Option<&str>, field: &str) -> Result<uuid::Uuid, String> {
    let value = value.ok_or_else(|| format!("批注事务缺少 {field}"))?;
    let parsed = value
        .parse::<uuid::Uuid>()
        .map_err(|error| format!("批注 {field} 无效：{error}"))?;
    if parsed.to_string() != value
        || parsed.get_version_num() != 4
        || parsed.get_variant() != uuid::Variant::RFC4122
    {
        return Err(format!("批注 {field} 必须是小写 RFC 4122 UUIDv4"));
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
            _ => Err("Chrono period 无效".to_owned()),
        })
        .collect()
}

fn build_node_metadata_plan(
    root: &Path,
    request: NodeMetadataPreviewRequest,
) -> Result<WorkspaceTransactionPlan, String> {
    let node_id = parse_node_id(&request.node_id)?;
    let revision = DocumentRevision::parse(&request.revision).map_err(|error| error.to_string())?;
    let plan = match request.action.as_str() {
        "aliases" => {
            if request.icon.is_some()
                || request.mode.is_some()
                || request.direction.is_some()
                || request.sibling_rank.is_some()
                || request.remove
            {
                return Err("别名预览包含了不适用于 aliases 的字段".to_owned());
            }
            let aliases = request.aliases.ok_or("别名预览缺少 aliases")?;
            plan_node_aliases_setting(root, node_id, &revision, &aliases)
        }
        "icon" => {
            if request.aliases.is_some()
                || request.mode.is_some()
                || request.direction.is_some()
                || request.sibling_rank.is_some()
            {
                return Err("图标预览包含了不适用于 icon 的字段".to_owned());
            }
            let icon = match (request.remove, request.icon.as_deref()) {
                (true, None) => None,
                (false, Some(icon)) => Some(icon),
                (true, Some(_)) => return Err("移除 icon 时不能同时提供值".to_owned()),
                (false, None) => return Err("图标预览缺少 icon 或 remove".to_owned()),
            };
            plan_node_icon_setting(root, node_id, &revision, icon)
        }
        "child_sort" => {
            if request.icon.is_some()
                || request.aliases.is_some()
                || request.sibling_rank.is_some()
                || request.remove
            {
                return Err("子节点排序预览包含了不适用于 child_sort 的字段".to_owned());
            }
            let mode = request.mode.ok_or("子节点排序预览缺少 mode")?;
            let direction = match (mode, request.direction) {
                (SortMode::Name, direction) => direction.unwrap_or_default(),
                (SortMode::Manual, None) => SortDirection::Ascending,
                (SortMode::Manual, Some(_)) => {
                    return Err("manual 排序不接受 direction".to_owned());
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
                return Err("同级排序预览包含了不适用于 sibling_rank 的字段".to_owned());
            }
            let rank = match (request.remove, request.sibling_rank) {
                (true, None) => None,
                (false, Some(rank)) => Some(rank),
                (true, Some(_)) => return Err("移除 sibling_rank 时不能同时提供值".to_owned()),
                (false, None) => return Err("同级排序预览缺少 siblingRank 或 remove".to_owned()),
            };
            plan_node_sibling_rank_setting(root, node_id, &revision, rank)
        }
        _ => return Err("未知的节点元数据操作".to_owned()),
    };
    plan.map_err(|error| error.to_string())
}

fn build_workspace_plan(
    root: &Path,
    action: &WorkspaceActionRequest,
) -> Result<WorkspaceTransactionPlan, String> {
    let node_id = || parse_required_id(action.node_id.as_deref(), "nodeId");
    let parent_id = || parse_required_id(action.parent_id.as_deref(), "parentId");
    let name = || {
        action
            .name
            .as_deref()
            .ok_or_else(|| "工作区事务缺少节点名称".to_owned())
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
                _ => return Err("混排设置必须是 separate 或 run_in".to_owned()),
            };
            plan_adjacent_heading_body_setting(root, value)
        }
        _ => return Err("未知的工作区事务".to_owned()),
    }
    .map_err(|error| error.to_string())
}

fn parse_required_id(value: Option<&str>, field: &str) -> Result<NodeId, String> {
    parse_node_id(value.ok_or_else(|| format!("工作区事务缺少 {field}"))?)
}

fn transaction_plan_payload(plan: &WorkspaceTransactionPlan) -> Value {
    if !plan.trash_item_changes().is_empty() {
        return trash_transaction_plan_payload(plan);
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

fn trash_transaction_plan_payload(plan: &WorkspaceTransactionPlan) -> Value {
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

fn query_value(path: &str, name: &str) -> Option<String> {
    let (_, query) = path.split_once('?')?;
    query
        .split('&')
        .find_map(|piece| piece.split_once('=').filter(|(key, _)| *key == name))
        .and_then(|(_, value)| percent_decode(value).ok())
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'%' {
            if cursor + 2 >= bytes.len() {
                return Err("URL 编码不完整".to_owned());
            }
            let hex = std::str::from_utf8(&bytes[cursor + 1..cursor + 3])
                .map_err(|_| "URL 编码无效".to_owned())?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|_| "URL 编码无效".to_owned())?);
            cursor += 3;
        } else if bytes[cursor] == b'+' {
            decoded.push(b' ');
            cursor += 1;
        } else {
            decoded.push(bytes[cursor]);
            cursor += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| "URL 内容不是 UTF-8".to_owned())
}

fn node_name(node_directory: &Path) -> String {
    node_directory.file_name().map_or_else(
        || "文档".to_owned(),
        |value| value.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use weftext_core::create_workspace;

    const REVIEWER_ID: &str = "70d407dd-8538-45da-bb3d-d2eb4baa8539";
    const SECOND_REVIEWER_ID: &str = "a3cab95a-1aa0-48e7-ad41-699dd42a40aa";

    fn replace_root_document(root: &Path, source: String) -> DocumentSnapshot {
        let snapshot = read_node_document(root).expect("read document before replacement");
        let plan = plan_document_edit(
            root,
            &snapshot.revision,
            [DocumentEdit {
                start: 0,
                end: u64::try_from(snapshot.source.len()).expect("document length"),
                replacement: source,
            }],
        )
        .expect("plan document replacement");
        commit_document_edit(&plan).expect("commit document replacement");
        read_node_document(root).expect("read replaced document")
    }

    fn annotation_action_from_json(request: Value) -> Result<AnnotationAction, String> {
        let request = serde_json::from_value(request)
            .map_err(|error| format!("invalid annotation request: {error}"))?;
        annotation_action(request)
    }

    fn commit_annotation_request(
        backend: &mut DesktopBackend,
        node_id: NodeId,
        request: Value,
    ) -> Value {
        let preview = backend
            .request("/api/annotation/preview", Some(request))
            .expect("annotation preview");
        let plan_id = preview["plan"]["planId"]
            .as_str()
            .expect("annotation plan ID")
            .to_owned();
        backend
            .request(
                &format!("/api/annotation/commit?nodeId={node_id}"),
                Some(json!({"planId": plan_id})),
            )
            .expect("annotation commit")
    }

    fn backup_path_capability(
        backend: &mut DesktopBackend,
        kind: BackupPathCapabilityKind,
        path: &Path,
    ) -> String {
        backend
            .register_backup_path_capability(kind, path)
            .expect("backup path capability")["capability"]
            .as_str()
            .expect("backup capability token")
            .to_owned()
    }

    #[test]
    fn desktop_payload_uses_the_shared_content_boundary_contract() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/content-boundary-v02");
        let payload = workspace_payload(&root).expect("workspace payload");
        assert_eq!(payload["nodes"].as_array().map(Vec::len), Some(2));
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
        let content = payload["content"].as_array().expect("content");
        assert!(content.iter().any(|entry| {
            entry["kind"] == "managed_node"
                && entry["path"] == "Managed"
                && entry["nodeId"].is_string()
        }));
        assert!(content.iter().any(|entry| {
            entry["kind"] == "unmanaged_markdown"
                && entry["path"] == "loose.md"
                && entry["displayIcon"]["kind"] == "markdown_file"
        }));
        assert!(content.iter().all(|entry| {
            entry["path"] != "Managed/Managed.adoc"
                && !entry["path"]
                    .as_str()
                    .is_some_and(|path| path.starts_with("ignored"))
        }));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one product-level test proves item inventory, batch resources, drafts, Safe Mode, and one-use plans together"
    )]
    fn desktop_trash_routes_use_core_items_and_fail_closed() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        let root_node = create_workspace(&workspace).expect("workspace");
        let child = weftext_core::create_child_node(&workspace, "Child").expect("child");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let opened = backend.open_workspace(&workspace).expect("open workspace");
        assert!(opened["workspace"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .all(|node| node.get("trashed").is_none()));

        let base = read_workspace_revision(&workspace)
            .expect("workspace revision")
            .to_string();
        let preview = backend
            .request(
                "/api/trash/node/preview",
                Some(json!({
                    "nodeId": child.id,
                    "baseWorkspaceRevision": base,
                    "trashedAt": "2026-08-24T12:00:00+08:00",
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect("node Trash preview");
        assert!(child.path.is_dir(), "preview remains read-only");
        assert_eq!(
            preview["plan"]["trashItemChanges"][0]["disposition"],
            "stored"
        );
        assert!(preview["plan"]["trashItemChanges"][0]
            .get("itemPath")
            .is_none());
        assert_eq!(preview["plan"]["pathChanges"], json!([]));
        assert_eq!(preview["plan"]["documentChanges"], json!([]));
        assert_eq!(preview["plan"]["generatedNodeIds"], json!([]));
        assert!(!serde_json::to_string(&preview["plan"])
            .expect("serialize path-free Trash plan")
            .contains(".weftext-trash"));
        let plan_id = preview["plan"]["planId"]
            .as_str()
            .expect("plan ID")
            .to_owned();
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": plan_id})),
            )
            .expect("node Trash commit");
        assert!(!child.path.exists());
        let replay = backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": plan_id})),
            )
            .expect_err("plan is single-use");
        assert!(replay.contains("失效"));

        let inventory = backend
            .request("/api/trash", None)
            .expect("Trash inventory");
        let item = &inventory["trash"]["items"][0];
        assert_eq!(item["restore"]["originResolution"], "active");
        assert!(item.get("payloadPath").is_none());
        let item_id = item["manifest"]["trashItemId"]
            .as_str()
            .expect("item ID")
            .to_owned();
        let restore = backend
            .request(
                "/api/trash/restore/preview",
                Some(json!({
                    "mode": "original",
                    "trashItemId": item_id,
                    "baseWorkspaceRevision": inventory["trash"]["workspaceRevision"],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect("restore preview");
        let restore_plan = restore["plan"]["planId"]
            .as_str()
            .expect("restore plan")
            .to_owned();
        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable Safe Mode");
        let safe_mode = backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": restore_plan})),
            )
            .expect_err("Safe Mode blocks restore commit");
        assert!(safe_mode.contains("安全模式"));
        backend
            .request("/api/safe-mode", Some(json!({"enabled": false})))
            .expect("disable Safe Mode");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": restore_plan})),
            )
            .expect("same stored restore plan remains available");
        assert!(child.path.join("Child.adoc").is_file());

        fs::write(child.path.join("one.bin"), b"one").expect("first resource");
        fs::write(child.path.join("two.bin"), b"two-two").expect("second resource");
        let root_document = read_node_document(&workspace).expect("root document");
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": root_node.id,
                    "revision": root_document.revision,
                    "source": format!("{}\nunsaved\n", root_document.source),
                })),
            )
            .expect("device draft");
        let unrelated = backend
            .request(
                "/api/trash/resources/preview",
                Some(json!({
                    "baseWorkspaceRevision": read_workspace_revision(&workspace).expect("revision"),
                    "trashedAt": "2026-08-24T12:01:00+08:00",
                    "resources": [{"ownerNodeId": child.id, "name": "one.bin"}],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect("an unrelated root draft does not block child resources");
        assert!(unrelated["plan"]["planId"].is_string());
        backend
            .request("/api/draft/discard", Some(json!({"nodeId": root_node.id})))
            .expect("discard draft");

        let child_document = read_node_document(&child.path).expect("child document");
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": child.id,
                    "revision": child_document.revision,
                    "source": format!("{}\nunsaved\n", child_document.source),
                })),
            )
            .expect("owner draft");
        let blocked = backend
            .request(
                "/api/trash/resources/preview",
                Some(json!({
                    "baseWorkspaceRevision": read_workspace_revision(&workspace).expect("revision"),
                    "trashedAt": "2026-08-24T12:01:00+08:00",
                    "resources": [{"ownerNodeId": child.id, "name": "one.bin"}],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect_err("the owning node draft blocks resource Trash");
        assert!(blocked.contains("操作范围命中 1 个设备草稿"));
        backend
            .request("/api/draft/discard", Some(json!({"nodeId": child.id})))
            .expect("discard owner draft");

        let resources = backend
            .request(
                "/api/trash/resources/preview",
                Some(json!({
                    "baseWorkspaceRevision": read_workspace_revision(&workspace).expect("revision"),
                    "trashedAt": "2026-08-24T12:01:00+08:00",
                    "resources": [
                        {"ownerNodeId": child.id, "name": "one.bin"},
                        {"ownerNodeId": child.id, "name": "two.bin"},
                    ],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect("resource batch preview");
        let changes = resources["plan"]["trashItemChanges"]
            .as_array()
            .expect("resource item changes");
        assert_eq!(changes.len(), 2);
        assert_eq!(
            changes[0]["manifest"]["operationId"],
            changes[1]["manifest"]["operationId"]
        );
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": resources["plan"]["planId"]})),
            )
            .expect("resource batch commit");
        assert!(!child.path.join("one.bin").exists());
        assert!(!child.path.join("two.bin").exists());

        let inventory = backend
            .request("/api/trash", None)
            .expect("resource inventory");
        let one = inventory["trash"]["items"]
            .as_array()
            .expect("items")
            .iter()
            .find(|item| item["manifest"]["originalName"] == "one.bin")
            .expect("one.bin item");
        let evidence = json!({
            "trashItemId": one["manifest"]["trashItemId"],
            "payloadSha256": one["manifest"]["sha256"],
            "payloadByteLength": one["manifest"]["byteLength"],
        });
        let mismatch = backend
            .request(
                "/api/trash/permanent-delete/preview",
                Some(json!({
                    "baseWorkspaceRevision": inventory["trash"]["workspaceRevision"],
                    "items": [{
                        "trashItemId": evidence["trashItemId"],
                        "payloadSha256": "0".repeat(64),
                        "payloadByteLength": evidence["payloadByteLength"],
                    }],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect_err("changed digest is rejected");
        assert!(mismatch.contains("不一致"));
        let permanent = backend
            .request(
                "/api/trash/permanent-delete/preview",
                Some(json!({
                    "baseWorkspaceRevision": inventory["trash"]["workspaceRevision"],
                    "items": [evidence],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect("exact permanent-delete preview");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": permanent["plan"]["planId"]})),
            )
            .expect("permanent delete commit");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one end-to-end case keeps legacy migration capability, Safe Mode, and explicit recovery semantics auditable"
    )]
    fn desktop_legacy_trash_migration_requires_typed_external_snapshot_capability() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Legacy workspace");
        let root_node = create_workspace(&workspace).expect("workspace");
        let child = weftext_core::create_child_node(&workspace, "Legacy").expect("child");
        commit_workspace_transaction(
            &plan_trash_node_at(&workspace, child.id, "2026-08-24T12:00:00+08:00")
                .expect("Trash setup plan"),
        )
        .expect("Trash setup commit");
        let item = scan_workspace(&workspace).trash_items.remove(0);
        fs::rename(&item.payload_path, workspace.join(".weftext-trash/Legacy"))
            .expect("historical direct entry");
        fs::remove_dir_all(
            workspace
                .join(".weftext-trash")
                .join(weftext_core::TRASH_ITEMS_DIRECTORY_NAME),
        )
        .expect("remove item-store authority");
        let snapshots = temp.path().join("migration snapshots");
        fs::create_dir(&snapshots).expect("snapshot parent");

        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let opened = backend
            .open_workspace(&workspace)
            .expect("read-only legacy open");
        assert_eq!(opened["workspace"]["trashLegacyMigrationRequired"], true);
        assert!(opened["workspace"]["trashItems"]
            .as_array()
            .expect("trusted items")
            .is_empty());

        let wrong_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::RestoreParent,
            &snapshots,
        );
        let wrong = backend
            .request(
                "/api/trash/migrate-legacy/preview",
                Some(json!({
                    "baseWorkspaceRevision": read_workspace_revision(&workspace).expect("revision"),
                    "trashedAt": "2026-08-24T12:30:00+08:00",
                    "backupParentCapability": wrong_capability,
                })),
            )
            .expect_err("wrong directory capability kind");
        assert!(wrong.contains("授权类型"));

        let backup_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::BackupParent,
            &snapshots,
        );
        let preview = backend
            .request(
                "/api/trash/migrate-legacy/preview",
                Some(json!({
                    "baseWorkspaceRevision": read_workspace_revision(&workspace).expect("revision"),
                    "trashedAt": "2026-08-24T12:30:00+08:00",
                    "backupParentCapability": backup_capability,
                })),
            )
            .expect("snapshot-backed migration preview");
        assert_eq!(
            preview["plan"]["trashItemChanges"][0]["manifest"]["originStatus"],
            "unknown"
        );
        assert_eq!(
            fs::read_dir(&snapshots).expect("snapshots").count(),
            1,
            "Core creates one external exact snapshot"
        );
        let plan_id = preview["plan"]["planId"]
            .as_str()
            .expect("plan ID")
            .to_owned();
        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable Safe Mode");
        let safe_mode = backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": plan_id})),
            )
            .expect_err("Safe Mode blocks migration commit");
        assert!(safe_mode.contains("安全模式"));
        backend
            .request("/api/safe-mode", Some(json!({"enabled": false})))
            .expect("disable Safe Mode");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": plan_id})),
            )
            .expect("migration commit");

        let inventory = backend.request("/api/trash", None).expect("item inventory");
        assert_eq!(inventory["trash"]["state"], "ready");
        let item = &inventory["trash"]["items"][0];
        assert_eq!(item["manifest"]["originStatus"], "unknown");
        let original = backend
            .request(
                "/api/trash/restore/preview",
                Some(json!({
                    "mode": "original",
                    "trashItemId": item["manifest"]["trashItemId"],
                    "baseWorkspaceRevision": inventory["trash"]["workspaceRevision"],
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect_err("unknown origin cannot restore implicitly");
        assert!(!original.is_empty());
        let restore = backend
            .request(
                "/api/trash/restore/preview",
                Some(json!({
                    "mode": "existing_target",
                    "trashItemId": item["manifest"]["trashItemId"],
                    "baseWorkspaceRevision": inventory["trash"]["workspaceRevision"],
                    "targetNodeId": root_node.id,
                    "name": "Recovered legacy",
                    "resolvedBy": "caller_explicit",
                })),
            )
            .expect("explicit target restore preview");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": restore["plan"]["planId"]})),
            )
            .expect("explicit target restore commit");
        assert!(workspace
            .join("Recovered legacy/Recovered legacy.adoc")
            .is_file());
    }

    #[test]
    fn opens_and_restores_last_workspace() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let config = temp.path().join("config");

        let mut first = DesktopBackend::new(config.clone());
        let opened = first.open_workspace(&workspace).expect("open");
        assert_eq!(opened["opened"], true);
        assert_eq!(
            opened["workspace"]["nodes"].as_array().map(Vec::len),
            Some(1)
        );

        let mut restarted = DesktopBackend::new(config);
        let restored = restarted.restore_workspace().expect("restore");
        assert_eq!(restored["opened"], true);
        assert_eq!(restored["restored"], true);
    }

    #[test]
    fn desktop_fails_closed_without_the_exact_asciidoc_marker() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        fs::write(
            workspace.join(".weftext-format"),
            b"weftext.asciidoc.v1\r\n",
        )
        .expect("malformed marker");

        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let error = backend
            .open_workspace(&workspace)
            .expect_err("malformed marker must fail closed");
        assert!(error.contains("精确的 weftext.asciidoc.v1 格式标记"));
        assert!(backend.workspace_root.is_none());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one product-level test proves the typed target, draft, exact-plan, and safe-mode contracts together"
    )]
    fn desktop_backup_controls_are_typed_exact_and_fail_closed() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        let source_root = create_workspace(&workspace).expect("source workspace");
        let source_child =
            weftext_core::create_child_node(&workspace, "Source").expect("source child");
        let trashable =
            weftext_core::create_child_node(&workspace, "TrashBytes").expect("trash child");
        fs::write(
            trashable.path.join("asset.bin"),
            b"Desktop backup carries Core Trash bytes\0\xff",
        )
        .expect("Trash payload bytes");
        commit_workspace_transaction(
            &plan_trash_node_at(&workspace, trashable.id, "2026-08-24T12:00:00+08:00")
                .expect("Trash plan"),
        )
        .expect("Trash commit");
        let trash_payload_locator = scan_workspace(&workspace).trash_items[0]
            .payload_path
            .strip_prefix(&workspace)
            .expect("Trash payload locator")
            .to_path_buf();
        let backup_parent = temp.path().join("backups");
        fs::create_dir(&backup_parent).expect("backup parent");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open source");

        let capabilities = backend
            .request("/api/backup/capabilities", None)
            .expect("backup capabilities");
        assert_eq!(capabilities["backup"]["managedShape"], "X/X.adoc");
        assert_eq!(
            capabilities["backup"]["annotations"],
            "node_local_weftext.annotations.json"
        );
        assert_eq!(capabilities["backup"]["savedSourceSetReady"], true);

        let wrong = backend
            .register_backup_path_capability(
                BackupPathCapabilityKind::RestoreParent,
                &backup_parent,
            )
            .expect("wrong typed capability");
        let wrong_error = backend
            .request(
                "/api/backup/preview",
                Some(json!({"backupParentCapability": wrong["capability"]})),
            )
            .expect_err("restore capability cannot authorize backup output");
        assert!(wrong_error.contains("授权类型"));

        let backup_capability = backend
            .register_backup_path_capability(BackupPathCapabilityKind::BackupParent, &backup_parent)
            .expect("backup capability");
        let source_document = read_node_document(&workspace).expect("source document");
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": source_root.id,
                    "revision": source_document.revision,
                    "source": format!("{}\nunsaved\n", source_document.source),
                })),
            )
            .expect("save device draft");
        let draft_blocked = backend
            .request(
                "/api/backup/preview",
                Some(json!({
                    "backupParentCapability": backup_capability["capability"],
                })),
            )
            .expect_err("device draft blocks authoritative backup preview");
        assert!(draft_blocked.contains("完整已保存源集"));
        backend
            .request(
                "/api/draft/discard",
                Some(json!({"nodeId": source_root.id})),
            )
            .expect("discard draft");

        let preview = backend
            .request(
                "/api/backup/preview",
                Some(json!({
                    "backupParentCapability": backup_capability["capability"],
                })),
            )
            .expect("backup preview");
        let plan_digest = preview["backup"]["plan"]["planDigest"]
            .as_str()
            .expect("plan digest")
            .to_owned();
        let snapshot = PathBuf::from(
            preview["backup"]["plan"]["snapshotDirectory"]
                .as_str()
                .expect("snapshot directory"),
        );
        let preview_locators = preview["backup"]["plan"]["entries"]
            .as_array()
            .expect("physical backup entries")
            .iter()
            .filter_map(|entry| entry["locator"].as_str())
            .collect::<Vec<_>>();
        assert!(preview_locators.iter().any(|locator| {
            locator.starts_with(".weftext-trash/_weftext.items/")
                && locator.ends_with("/_weftext.trash-item.json")
        }));
        assert!(preview_locators.iter().any(|locator| {
            locator.starts_with(".weftext-trash/_weftext.items/")
                && locator.ends_with("/payload/TrashBytes/asset.bin")
        }));
        assert!(!snapshot.exists(), "preview must remain read-only");
        let consumed = backend
            .request(
                "/api/backup/preview",
                Some(json!({
                    "backupParentCapability": backup_capability["capability"],
                })),
            )
            .expect_err("directory capability is one-use");
        assert!(consumed.contains("无效或已使用"));

        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable safe mode");
        let committed = backend
            .request(
                "/api/backup/commit",
                Some(json!({"planDigest": plan_digest})),
            )
            .expect("safe mode permits external backup commit");
        assert_eq!(committed["backup"]["receipt"]["verified"], true);
        assert!(snapshot.exists());
        assert_eq!(
            fs::read(
                snapshot
                    .join("content/Workspace")
                    .join(&trash_payload_locator)
                    .join("asset.bin")
            )
            .expect("snapshot Trash payload bytes"),
            b"Desktop backup carries Core Trash bytes\0\xff"
        );

        let target = temp.path().join("Target");
        let target_root = create_workspace(&target).expect("target workspace");
        backend.open_workspace(&target).expect("open target");
        let snapshot_capability = backend
            .register_backup_path_capability(BackupPathCapabilityKind::Snapshot, &snapshot)
            .expect("snapshot capability");
        let scoped = backend
            .request(
                "/api/backup/scoped-restore/preview",
                Some(json!({
                    "snapshotCapability": snapshot_capability["capability"],
                    "sourceNodeId": source_child.id,
                    "destinationParentId": target_root.id,
                    "destinationName": "Recovered",
                    "scope": "single_node",
                })),
            )
            .expect("scoped restore preview");
        let scoped_digest = scoped["backup"]["plan"]["planDigest"]
            .as_str()
            .expect("scoped plan digest");
        let safe_blocked = backend
            .request(
                "/api/backup/scoped-restore/commit",
                Some(json!({"planDigest": scoped_digest})),
            )
            .expect_err("safe mode blocks current-workspace restore commit");
        assert!(safe_blocked.contains("安全模式"));
        backend
            .request("/api/safe-mode", Some(json!({"enabled": false})))
            .expect("disable safe mode");
        let restored = backend
            .request(
                "/api/backup/scoped-restore/commit",
                Some(json!({"planDigest": scoped_digest})),
            )
            .expect("commit preserved scoped plan");
        assert_eq!(restored["backup"]["receipt"]["exactBytesVerified"], true);
        assert!(target.join("Recovered/Recovered.adoc").is_file());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one route-level test proves every external backup maintenance control in Safe Mode"
    )]
    fn desktop_backup_maintenance_restore_and_drill_controls_are_real() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("source workspace");
        let backup_parent = temp.path().join("backups");
        let restore_parent = temp.path().join("alternate");
        let drill_parent = temp.path().join("drills");
        let drill_results_parent = temp.path().join("drill-results");
        for directory in [
            &backup_parent,
            &restore_parent,
            &drill_parent,
            &drill_results_parent,
        ] {
            fs::create_dir(directory).expect("backup control directory");
        }

        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open source");
        let backup_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::BackupParent,
            &backup_parent,
        );
        let preview = backend
            .request(
                "/api/backup/preview",
                Some(json!({"backupParentCapability": backup_capability})),
            )
            .expect("full backup preview");
        let backup_digest = preview["backup"]["plan"]["planDigest"]
            .as_str()
            .expect("backup digest")
            .to_owned();
        let snapshot = PathBuf::from(
            preview["backup"]["plan"]["snapshotDirectory"]
                .as_str()
                .expect("snapshot path"),
        );
        backend
            .request(
                "/api/backup/commit",
                Some(json!({"planDigest": backup_digest})),
            )
            .expect("full backup commit");
        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable Safe Mode");

        let verify_capability =
            backup_path_capability(&mut backend, BackupPathCapabilityKind::Snapshot, &snapshot);
        let verified = backend
            .request(
                "/api/backup/verify",
                Some(json!({"snapshotCapability": verify_capability})),
            )
            .expect("Safe Mode snapshot verification");
        assert_eq!(verified["backup"]["verification"]["complete"], true);

        let protect_capability =
            backup_path_capability(&mut backend, BackupPathCapabilityKind::Snapshot, &snapshot);
        let protected = backend
            .request(
                "/api/backup/protect",
                Some(json!({
                    "snapshotCapability": protect_capability,
                    "label": "产品恢复基线",
                })),
            )
            .expect("Safe Mode restore-point protection");
        assert_eq!(protected["backup"]["protection"]["label"], "产品恢复基线");

        let retention_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::BackupParent,
            &backup_parent,
        );
        let retention = backend
            .request(
                "/api/backup/retention/preview",
                Some(json!({
                    "backupParentCapability": retention_capability,
                    "keepLatestUnprotected": 0,
                })),
            )
            .expect("Safe Mode retention preview");
        let retention_digest = retention["backup"]["plan"]["planDigest"]
            .as_str()
            .expect("retention digest")
            .to_owned();
        let retained = backend
            .request(
                "/api/backup/retention/commit",
                Some(json!({"planDigest": retention_digest})),
            )
            .expect("Safe Mode retention commit");
        assert_eq!(retained["backup"]["stage"], "retention_committed");
        assert!(snapshot.is_dir(), "protected snapshot survives retention");

        let recovery_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::BackupParent,
            &backup_parent,
        );
        let recovered = backend
            .request(
                "/api/backup/retention/recover",
                Some(json!({"backupParentCapability": recovery_capability})),
            )
            .expect("Safe Mode retention recovery");
        assert_eq!(recovered["backup"]["stage"], "retention_recovered");

        let restore_snapshot_capability =
            backup_path_capability(&mut backend, BackupPathCapabilityKind::Snapshot, &snapshot);
        let restore_parent_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::RestoreParent,
            &restore_parent,
        );
        let restore = backend
            .request(
                "/api/backup/restore/preview",
                Some(json!({
                    "snapshotCapability": restore_snapshot_capability,
                    "destinationParentCapability": restore_parent_capability,
                })),
            )
            .expect("Safe Mode alternate restore preview");
        let restore_digest = restore["backup"]["plan"]["planDigest"]
            .as_str()
            .expect("restore digest")
            .to_owned();
        let restored = backend
            .request(
                "/api/backup/restore/commit",
                Some(json!({"planDigest": restore_digest})),
            )
            .expect("Safe Mode clean alternate restore");
        assert_eq!(restored["backup"]["receipt"]["bytewiseVerified"], true);
        assert!(restore_parent.join("Workspace/Workspace.adoc").is_file());

        let drill_snapshot_capability =
            backup_path_capability(&mut backend, BackupPathCapabilityKind::Snapshot, &snapshot);
        let drill_parent_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::DrillParent,
            &drill_parent,
        );
        let drill_results_capability = backup_path_capability(
            &mut backend,
            BackupPathCapabilityKind::DrillResultsParent,
            &drill_results_parent,
        );
        let drill = backend
            .request(
                "/api/backup/drill/preview",
                Some(json!({
                    "snapshotCapability": drill_snapshot_capability,
                    "drillParentCapability": drill_parent_capability,
                    "resultsParentCapability": drill_results_capability,
                })),
            )
            .expect("Safe Mode drill preview");
        let drill_digest = drill["backup"]["plan"]["planDigest"]
            .as_str()
            .expect("drill digest")
            .to_owned();
        let drilled = backend
            .request(
                "/api/backup/drill/commit",
                Some(json!({"planDigest": drill_digest})),
            )
            .expect("Safe Mode drill commit");
        assert_eq!(drilled["backup"]["receipt"]["openedClean"], true);
        assert_eq!(drilled["backup"]["receipt"]["bytewiseVerified"], true);
    }

    #[test]
    fn desktop_boundary_projects_and_commits_one_canonical_scalar_icon_plan() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read");
        let source =
            snapshot
                .source
                .replacen("weftext:\n", "weftext:\n  icon: weftext:project\n", 1);
        fs::write(&snapshot.document_path, &source).expect("write scalar icon");
        let snapshot = read_node_document(&workspace).expect("read scalar icon");

        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let opened = backend.open_workspace(&workspace).expect("open workspace");
        assert_eq!(
            opened["workspace"]["nodes"][0]["icon"]["value"],
            "weftext:project"
        );
        assert_eq!(opened["workspace"]["nodes"][0]["icon"]["glyph"], "项");
        assert_eq!(
            opened["document"]["metadata"]["schema"],
            "weftext.node-metadata.v1"
        );
        assert_eq!(opened["document"]["metadata"]["icon"], "weftext:project");
        assert_eq!(opened["document"]["metadata"]["aliases"], json!([]));
        assert_eq!(
            opened["document"]["metadata"]["adjacentHeadingBody"],
            "separate"
        );

        let preview = backend
            .request(
                "/api/node/metadata/preview",
                Some(json!({
                    "action": "icon",
                    "icon": "weftext:book",
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                })),
            )
            .expect("preview scalar through metadata boundary");
        assert_eq!(preview["plan"]["action"], "node_metadata");
        assert_eq!(
            read_node_document(&workspace)
                .expect("preview read only")
                .source,
            snapshot.source
        );

        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": preview["plan"]["planId"]})),
            )
            .expect("commit selected icon");
        let committed = backend
            .request(&format!("/api/document?nodeId={}", snapshot.node_id), None)
            .expect("read committed projection");
        assert_eq!(committed["document"]["metadata"]["icon"], "weftext:book");
        assert_eq!(
            committed["document"]["metadata"]["resolvedIcon"]["glyph"],
            "书"
        );
        assert!(read_node_document(&workspace)
            .expect("committed source")
            .source
            .contains("weftext:\n  icon: \"weftext:book\"\n"));
        assert!(backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": preview["plan"]["planId"]})),
            )
            .expect_err("stored preview is single use")
            .contains("失效"));
    }

    #[test]
    fn opens_authoritative_workspace_when_initial_derived_index_refresh_fails() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let config = temp.path().join("config");
        fs::create_dir_all(&config).expect("config directory");
        fs::write(config.join("search-indexes"), b"failure injection")
            .expect("block derived index directory creation");

        let mut backend = DesktopBackend::new(config);
        let opened = backend
            .open_workspace(&workspace)
            .expect("authoritative workspace still opens");
        assert_eq!(opened["opened"], true);
        assert_eq!(
            opened["document"]["nodeId"],
            opened["workspace"]["rootNodeId"]
        );
        assert!(opened["searchIndex"].is_null());
        assert_eq!(
            opened["searchIndexWarning"]["code"],
            "derived_search_index_refresh_failed"
        );
        assert_eq!(opened["searchIndexWarning"]["rebuildRequired"], true);
        assert_eq!(opened["searchIndexWarning"]["workspaceOpenSucceeded"], true);

        let search_error = backend
            .request("/api/search?q=Workspace", None)
            .expect_err("search must report the unavailable derived index");
        assert!(!search_error.is_empty());
        assert_eq!(
            read_node_document(&workspace)
                .expect("authoritative document remains readable")
                .node_id
                .to_string(),
            opened["document"]["nodeId"].as_str().expect("node id")
        );
    }

    #[test]
    fn searches_managed_asciidoc_without_exposing_the_system_envelope() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read");
        let source = snapshot.source.clone()
            + "= Workspace\n:keywords: alpha-search-tag\n\n== 路线\n桌面日用闭环\n";
        let plan = plan_document_edit(
            &workspace,
            &snapshot.revision,
            [DocumentEdit {
                start: 0,
                end: u64::try_from(snapshot.source.len()).expect("length"),
                replacement: source,
            }],
        )
        .expect("plan");
        commit_document_edit(&plan).expect("commit");

        let config = temp.path().join("config");
        let mut backend = DesktopBackend::new(config.clone());
        let opened = backend.open_workspace(&workspace).expect("open");
        assert_eq!(opened["searchIndex"]["reparsedDocuments"], 1);

        let visible = backend
            .request("/api/search?q=日用闭环", None)
            .expect("search");
        assert_eq!(visible["results"].as_array().map(Vec::len), Some(1));
        let hidden = backend
            .request("/api/search?q=weftext", None)
            .expect("search");
        assert_eq!(hidden["results"], json!([]));
        let tag = backend
            .request("/api/search?q=alpha-search-tag", None)
            .expect("search");
        assert_eq!(tag["results"].as_array().map(Vec::len), Some(1));
        assert!(config.join("search-indexes").is_dir());
        assert!(!workspace.join("search-indexes").exists());

        let root_id = root_node_id(&workspace).expect("root id");
        let create =
            plan_create_child_node(&workspace, root_id, "ArchivedSearchNode").expect("create plan");
        commit_workspace_transaction(&create).expect("create");
        let archived_id = scan_workspace(&workspace)
            .nodes
            .iter()
            .find(|node| node.name == "ArchivedSearchNode")
            .and_then(|node| node.id)
            .expect("created node");
        let trash = plan_trash_node(&workspace, archived_id).expect("trash plan");
        commit_workspace_transaction(&trash).expect("trash");
        let archived = backend
            .request("/api/search?q=ArchivedSearchNode", None)
            .expect("search");
        assert_eq!(archived["results"], json!([]));
    }

    #[test]
    fn parses_an_unsaved_draft_through_the_desktop_boundary() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let opened = backend.open_workspace(&workspace).expect("open");
        assert_eq!(
            opened["workspace"]["documentFormat"]["generation"],
            "ascii_doc_v1"
        );
        assert_eq!(
            opened["workspace"]["documentFormat"]["canonicalExtension"],
            "adoc"
        );
        let snapshot = read_node_document(&workspace).expect("snapshot");
        let source = snapshot.source + "= 标题\n\n== 小节\n紧邻正文\n";

        let parsed = backend
            .request("/api/document/model", Some(json!({"source": source})))
            .expect("parse draft");
        let blocks = parsed["model"]["blocks"]
            .as_array()
            .expect("AsciiDoc blocks");
        assert!(blocks.iter().any(|block| block["text"] == "小节"));
        assert!(blocks.iter().any(|block| block["text"] == "紧邻正文"));
        assert_eq!(parsed["profile"]["profile"], "ascii_doc_v1");
        assert_eq!(parsed["model"]["blocks"], parsed["view"]["blocks"]);
    }

    #[test]
    fn annotation_bridge_rejects_the_retired_markdown_body_field() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let node_id = read_node_document(&workspace).expect("read").node_id;
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");

        let error = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "create",
                    "nodeId": node_id,
                    "kind": "comment",
                    "target": {"kind": "document"},
                    "bodyMarkdown": "旧批注正文",
                    "labels": [],
                    "authorId": "70d407dd-8538-45da-bb3d-d2eb4baa8539",
                    "authorName": "Desktop reviewer",
                    "timestamp": "2026-08-21T12:00:00Z",
                })),
            )
            .expect_err("legacy Markdown body field must be rejected");
        assert!(error.contains("bodyMarkdown"));
    }

    #[test]
    fn annotation_bridge_rejects_retired_compatibility_fields_and_action_aliases() {
        for retired_field in [
            json!({"sourceOffset": 42}),
            json!({"mark": "highlight"}),
            json!({"color": "yellow"}),
            json!({"resolved": true}),
        ] {
            let mut request = json!({
                "action": "create",
                "nodeId": "11111111-1111-4111-8111-111111111111",
                "kind": "comment",
                "target": {"kind": "document"},
                "bodySource": "严格 v3 正文",
                "authorId": REVIEWER_ID,
                "authorName": "Desktop reviewer",
                "timestamp": "2026-08-24T08:00:00+08:00"
            });
            request
                .as_object_mut()
                .expect("request object")
                .extend(retired_field.as_object().expect("retired field").clone());
            assert!(annotation_action_from_json(request).is_err());
        }

        let camel_case_action = annotation_action_from_json(json!({
            "action": "acceptSuggestion",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "timestamp": "2026-08-24T08:01:00+08:00"
        }));
        assert!(camel_case_action.is_err());

        let type_tag = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {"type": "document"},
            "bodySource": "严格 v3 正文",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T08:02:00+08:00"
        }));
        assert!(type_tag.is_err());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn annotation_bridge_maps_typed_v3_actions_without_losing_target_data() {
        let resource = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {
                "kind": "resource_region",
                "resourceLocator": "figures/review.pdf",
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
            },
            "bodySource": "资源区域批注",
            "labels": ["figure"],
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T09:00:00+08:00"
        }))
        .expect("resource target mapping");
        assert!(matches!(
            resource,
            AnnotationAction::Create {
                target: AnnotationTargetIntent::ResourceRegion {
                    ref resource_locator,
                    media_kind: AnnotationResourceMediaKind::Pdf,
                    region: AnnotationResourceRegion::Rect { page: Some(2), .. },
                    ..
                },
                ..
            } if resource_locator == "figures/review.pdf"
        ));
        let timed_resource = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {
                "kind": "resource_region",
                "resourceLocator": "media/review.mp3",
                "resourceDigest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "mediaKind": "audio",
                "region": {
                    "kind": "time_range",
                    "startMilliseconds": 1250,
                    "endMilliseconds": 2750
                }
            },
            "bodySource": "音频时间区间",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T09:00:30+08:00"
        }))
        .expect("timed resource target mapping");
        assert!(matches!(
            timed_resource,
            AnnotationAction::Create {
                target: AnnotationTargetIntent::ResourceRegion {
                    media_kind: AnnotationResourceMediaKind::Audio,
                    region: AnnotationResourceRegion::TimeRange {
                        start_milliseconds: 1250,
                        end_milliseconds: 2750,
                    },
                    ..
                },
                ..
            }
        ));

        let insert = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "suggestion_insert",
            "target": {"kind": "insertion_point", "position": 12},
            "bodySource": "可选说明",
            "suggestedSource": "插入内容",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T09:01:00+08:00"
        }))
        .expect("insert suggestion mapping");
        assert!(matches!(
            insert,
            AnnotationAction::Create {
                kind: AnnotationKind::SuggestionInsert,
                target: AnnotationTargetIntent::InsertionPoint { position: 12 },
                body_source: Some(ref body),
                suggested_source: Some(ref suggested),
                ..
            } if body == "可选说明" && suggested == "插入内容"
        ));

        let edit = annotation_action_from_json(json!({
            "action": "edit_message",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "messageId": "44444444-4444-4444-8444-444444444444",
            "bodySource": "已编辑的正文",
            "authorId": REVIEWER_ID,
            "timestamp": "2026-08-24T09:02:00+08:00"
        }))
        .expect("message edit mapping");
        assert!(matches!(edit, AnnotationAction::EditMessage { .. }));

        let appearance = annotation_action_from_json(json!({
            "action": "set_appearance",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "appearance": {"mark": "underline", "theme": "blue"},
            "timestamp": "2026-08-24T09:03:00+08:00"
        }))
        .expect("appearance mapping");
        assert!(matches!(
            appearance,
            AnnotationAction::SetAppearance {
                appearance: Some(AnnotationAppearance {
                    mark: AnnotationMark::Underline,
                    color: AnnotationColor::Blue,
                }),
                ..
            }
        ));
        let cleared = annotation_action_from_json(json!({
            "action": "set_appearance",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "appearance": {"mark": "none"},
            "timestamp": "2026-08-24T09:03:30+08:00"
        }))
        .expect("appearance clear mapping");
        assert!(matches!(
            cleared,
            AnnotationAction::SetAppearance {
                appearance: None,
                ..
            }
        ));

        let labels = annotation_action_from_json(json!({
            "action": "set_labels",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "labels": ["review", "urgent"],
            "timestamp": "2026-08-24T09:04:00+08:00"
        }))
        .expect("labels mapping");
        assert!(matches!(
            labels,
            AnnotationAction::SetLabels { labels, .. } if labels == ["review", "urgent"]
        ));

        for (action_name, expected) in [
            ("reopen", "reopen"),
            ("reanchor", "reanchor"),
            ("accept_suggestion", "accept"),
            ("reject_suggestion", "reject"),
        ] {
            let action = annotation_action_from_json(json!({
                "action": action_name,
                "nodeId": "11111111-1111-4111-8111-111111111111",
                "annotationId": "33333333-3333-4333-8333-333333333333",
                "timestamp": "2026-08-24T09:05:00+08:00"
            }))
            .expect("simple v3 action mapping");
            assert!(match expected {
                "reopen" => matches!(
                    action,
                    AnnotationAction::SetResolved {
                        resolved: false,
                        ..
                    }
                ),
                "reanchor" => matches!(action, AnnotationAction::Reanchor { .. }),
                "accept" => matches!(action, AnnotationAction::AcceptSuggestion { .. }),
                "reject" => matches!(action, AnnotationAction::RejectSuggestion { .. }),
                _ => false,
            });
        }
    }

    #[test]
    fn annotation_bridge_rejects_unknown_fields_and_illegal_v3_combinations() {
        let unknown = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {"kind": "document"},
            "bodySource": "正文",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T10:00:00+08:00",
            "unexpected": true
        }));
        assert!(unknown.is_err());

        let unknown_document_target_field = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {"kind": "document", "unexpected": true},
            "bodySource": "正文",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T10:00:30+08:00"
        }));
        assert!(unknown_document_target_field.is_err());

        let wrong_insert_target = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "suggestion_insert",
            "target": {"kind": "document"},
            "suggestedSource": "插入内容",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T10:01:00+08:00"
        }));
        assert!(wrong_insert_target.is_err());

        let invisible_create_appearance = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {"kind": "document"},
            "appearance": {"mark": "none"},
            "bodySource": "正文",
            "authorId": REVIEWER_ID,
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T10:02:00+08:00"
        }));
        assert!(invisible_create_appearance.is_err());

        let missing_labels = annotation_action_from_json(json!({
            "action": "set_labels",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "timestamp": "2026-08-24T10:03:00+08:00"
        }));
        assert!(missing_labels.is_err());

        let foreign_edit_fields = annotation_action_from_json(json!({
            "action": "edit_message",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "annotationId": "33333333-3333-4333-8333-333333333333",
            "messageId": "44444444-4444-4444-8444-444444444444",
            "bodySource": "编辑",
            "authorId": REVIEWER_ID,
            "authorName": "不能伪造显示名",
            "timestamp": "2026-08-24T10:04:00+08:00"
        }));
        assert!(foreign_edit_fields.is_err());

        let uppercase_uuid = annotation_action_from_json(json!({
            "action": "create",
            "nodeId": "11111111-1111-4111-8111-111111111111",
            "kind": "comment",
            "target": {"kind": "document"},
            "bodySource": "正文",
            "authorId": "70D407DD-8538-45DA-BB3D-D2EB4BAA8539",
            "authorName": "Desktop reviewer",
            "timestamp": "2026-08-24T10:05:00+08:00"
        }));
        assert!(uppercase_uuid.is_err());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn annotation_bridge_commits_strict_v3_requests_and_supports_thread_actions() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let initial = read_node_document(&workspace).expect("initial document");
        let seeded = replace_root_document(
            &workspace,
            initial.source + "= Workspace\n\n== Review\nAlpha target Omega.\n",
        );
        let node_id = seeded.node_id;
        let source_offset = seeded.source.find("Alpha target").expect("review block");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");

        let created = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "create",
                "nodeId": node_id,
                "kind": "comment",
                "target": {"kind": "block_at", "sourceOffset": source_offset},
                "appearance": {"mark": "highlight", "theme": "yellow"},
                "bodySource": "请核对这个段落",
                "labels": ["verify"],
                "authorId": REVIEWER_ID,
                "authorName": "Desktop reviewer",
                "timestamp": "2026-08-24T11:00:00+08:00"
            }),
        );
        let annotation_id = created["annotations"]["annotations"][0]["id"]
            .as_str()
            .expect("annotation ID")
            .to_owned();
        let message_id = created["annotations"]["annotations"][0]["thread"][0]["id"]
            .as_str()
            .expect("message ID")
            .to_owned();
        assert_eq!(created["annotations"]["version"], 3);
        assert_eq!(
            created["annotations"]["annotations"][0]["target"]["kind"],
            "block"
        );
        assert_eq!(
            created["annotations"]["annotations"][0]["thread"][0]["body"]["format"],
            "weftext.asciidoc.inline.v1"
        );
        assert!(created["document"].is_null());

        let replied = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "reply",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "bodySource": "严格 v3 回复",
                "authorId": SECOND_REVIEWER_ID,
                "authorName": "Second reviewer",
                "timestamp": "2026-08-24T11:01:00+08:00"
            }),
        );
        assert_eq!(
            replied["annotations"]["annotations"][0]["thread"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );

        let edited = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "edit_message",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "messageId": message_id,
                "bodySource": "已核对并修订",
                "authorId": REVIEWER_ID,
                "timestamp": "2026-08-24T11:02:00+08:00"
            }),
        );
        assert_eq!(
            edited["annotations"]["annotations"][0]["thread"][0]["body"]["source"],
            "已核对并修订"
        );
        let forged_edit = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "edit_message",
                    "nodeId": node_id,
                    "annotationId": annotation_id,
                    "messageId": message_id,
                    "bodySource": "伪造编辑",
                    "authorId": SECOND_REVIEWER_ID,
                    "timestamp": "2026-08-24T11:03:00+08:00"
                })),
            )
            .expect_err("only the message author may edit");
        assert!(forged_edit.contains("only the message author"));

        let appeared = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "set_appearance",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "appearance": {"mark": "underline", "theme": "blue"},
                "timestamp": "2026-08-24T11:04:00+08:00"
            }),
        );
        assert_eq!(
            appeared["annotations"]["annotations"][0]["appearance"],
            json!({"mark": "underline", "theme": "blue"})
        );
        let labelled = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "set_labels",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "labels": ["review", "edited"],
                "timestamp": "2026-08-24T11:05:00+08:00"
            }),
        );
        assert_eq!(
            labelled["annotations"]["annotations"][0]["labels"],
            json!(["review", "edited"])
        );

        let resolved = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "resolve",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "timestamp": "2026-08-24T11:06:00+08:00"
            }),
        );
        assert_eq!(
            resolved["annotations"]["annotations"][0]["resolution"],
            "resolved"
        );
        let reopened = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "reopen",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "timestamp": "2026-08-24T11:07:00+08:00"
            }),
        );
        assert_eq!(reopened["annotations"]["annotations"][0]["state"], "open");
        assert!(reopened["annotations"]["annotations"][0]["resolution"].is_null());

        let before_shift = read_node_document(&workspace).expect("document before shift");
        let shifted_source =
            before_shift
                .source
                .replacen("== Review", "Intro paragraph.\n\n== Review", 1);
        let shifted = replace_root_document(&workspace, shifted_source);
        let reanchored = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "reanchor",
                "nodeId": node_id,
                "annotationId": annotation_id,
                "timestamp": "2026-08-24T11:08:00+08:00"
            }),
        );
        assert_eq!(
            reanchored["annotations"]["annotations"][0]["target"]["base_revision"],
            shifted.revision.as_str()
        );

        let sidecar = fs::read_to_string(workspace.join("weftext.annotations.json"))
            .expect("annotation sidecar");
        assert!(sidecar.contains("weftext.asciidoc.inline.v1"));
        assert!(!sidecar.contains("weftext.markdown.inline.v1"));
        assert!(!workspace.join("_weftext.annotations.json").exists());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn annotation_bridge_accepts_and_rejects_suggestions_with_authoritative_results() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let initial = read_node_document(&workspace).expect("initial document");
        let seeded = replace_root_document(
            &workspace,
            initial.source + "= Workspace\n\n== Review\nAlpha target Omega.\n",
        );
        let node_id = seeded.node_id;
        let insertion = seeded.source.find("Omega").expect("insertion point");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");

        let inserted = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "create",
                "nodeId": node_id,
                "kind": "suggestion_insert",
                "target": {"kind": "insertion_point", "position": insertion},
                "suggestedSource": "reviewed ",
                "authorId": REVIEWER_ID,
                "authorName": "Desktop reviewer",
                "timestamp": "2026-08-24T12:00:00+08:00"
            }),
        );
        let insert_id = inserted["annotations"]["annotations"][0]["id"]
            .as_str()
            .expect("insert suggestion ID")
            .to_owned();
        let accepted = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "accept_suggestion",
                "nodeId": node_id,
                "annotationId": insert_id,
                "timestamp": "2026-08-24T12:01:00+08:00"
            }),
        );
        let mut expected_source = seeded.source.clone();
        expected_source.insert_str(insertion, "reviewed ");
        assert_eq!(accepted["document"]["source"], expected_source);
        assert_eq!(accepted["document"]["nodeId"], node_id.to_string());
        assert!(accepted["searchIndex"].is_object());
        assert_eq!(
            accepted["annotations"]["annotations"][0]["resolution"],
            "accepted"
        );
        assert_eq!(
            read_node_document(&workspace)
                .expect("accepted source")
                .source,
            expected_source
        );

        let before_delete = read_node_document(&workspace).expect("source before deletion");
        let delete_start = before_delete.source.find("Alpha ").expect("delete target");
        let delete_end = delete_start + "Alpha ".len();
        let deletion = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "create",
                "nodeId": node_id,
                "kind": "suggestion_delete",
                "target": {
                    "kind": "text_range",
                    "start": delete_start,
                    "end": delete_end
                },
                "authorId": REVIEWER_ID,
                "authorName": "Desktop reviewer",
                "timestamp": "2026-08-24T12:02:00+08:00"
            }),
        );
        let delete_id = deletion["annotations"]["annotations"]
            .as_array()
            .expect("annotations")
            .iter()
            .find(|annotation| annotation["kind"] == "suggestion_delete")
            .and_then(|annotation| annotation["id"].as_str())
            .expect("delete suggestion ID")
            .to_owned();
        let rejected = commit_annotation_request(
            &mut backend,
            node_id,
            json!({
                "action": "reject_suggestion",
                "nodeId": node_id,
                "annotationId": delete_id,
                "timestamp": "2026-08-24T12:03:00+08:00"
            }),
        );
        assert!(rejected["document"].is_null());
        assert!(rejected["searchIndex"].is_null());
        assert_eq!(
            rejected["annotations"]["annotations"]
                .as_array()
                .expect("annotations")
                .iter()
                .find(|annotation| annotation["kind"] == "suggestion_delete")
                .expect("delete suggestion")["resolution"],
            "rejected"
        );
        assert_eq!(
            read_node_document(&workspace)
                .expect("source after rejection")
                .source,
            before_delete.source
        );
    }

    #[test]
    fn annotation_commit_is_bound_to_its_previewed_node() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let node_id = read_node_document(&workspace).expect("document").node_id;
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        let preview = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "create",
                    "nodeId": node_id,
                    "kind": "comment",
                    "target": {"kind": "document"},
                    "bodySource": "节点绑定",
                    "authorId": REVIEWER_ID,
                    "authorName": "Desktop reviewer",
                    "timestamp": "2026-08-24T13:00:00+08:00"
                })),
            )
            .expect("annotation preview");
        let plan_id = preview["plan"]["planId"]
            .as_str()
            .expect("plan ID")
            .to_owned();

        let wrong_node = "99999999-9999-4999-8999-999999999999";
        let mismatch = backend
            .request(
                &format!("/api/annotation/commit?nodeId={wrong_node}"),
                Some(json!({"planId": plan_id})),
            )
            .expect_err("mismatched node");
        assert!(mismatch.contains("节点与预览节点不一致"));

        let committed = backend
            .request(
                &format!("/api/annotation/commit?nodeId={node_id}"),
                Some(json!({"planId": plan_id})),
            )
            .expect("plan remains available for the bound node");
        assert_eq!(committed["nodeId"], node_id.to_string());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn annotation_preview_and_commit_fail_closed_on_drafts_and_recovery_issues() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let config = temp.path().join("config");
        let snapshot = read_node_document(&workspace).expect("document");
        let node_id = snapshot.node_id;
        let mut backend = DesktopBackend::new(config.clone());
        backend.open_workspace(&workspace).expect("open");

        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": node_id,
                    "revision": snapshot.revision,
                    "source": format!("{}\n未保存批注上下文\n", snapshot.source),
                })),
            )
            .expect("save device draft");
        let blocked_preview = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "create",
                    "nodeId": node_id,
                    "kind": "comment",
                    "target": {"kind": "document"},
                    "bodySource": "必须等待草稿处理",
                    "authorId": REVIEWER_ID,
                    "authorName": "Desktop reviewer",
                    "timestamp": "2026-08-24T14:00:00+08:00"
                })),
            )
            .expect_err("device draft blocks annotation preview");
        assert!(blocked_preview.contains("完整已保存源集"));
        backend
            .request("/api/draft/discard", Some(json!({"nodeId": node_id})))
            .expect("discard draft");

        let preview = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "create",
                    "nodeId": node_id,
                    "kind": "comment",
                    "target": {"kind": "document"},
                    "bodySource": "提交前再次检查草稿",
                    "authorId": REVIEWER_ID,
                    "authorName": "Desktop reviewer",
                    "timestamp": "2026-08-24T14:01:00+08:00"
                })),
            )
            .expect("clean annotation preview");
        let plan_id = preview["plan"]["planId"]
            .as_str()
            .expect("plan ID")
            .to_owned();
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": node_id,
                    "revision": snapshot.revision,
                    "source": format!("{}\n预览后出现的草稿\n", snapshot.source),
                })),
            )
            .expect("save draft after preview");
        let blocked_commit = backend
            .request(
                &format!("/api/annotation/commit?nodeId={node_id}"),
                Some(json!({"planId": plan_id})),
            )
            .expect_err("device draft blocks annotation commit");
        assert!(blocked_commit.contains("完整已保存源集"));
        backend
            .request("/api/draft/discard", Some(json!({"nodeId": node_id})))
            .expect("discard post-preview draft");
        backend
            .request(
                &format!("/api/annotation/commit?nodeId={node_id}"),
                Some(json!({"planId": plan_id})),
            )
            .expect("blocked commit preserves its plan");

        let current = read_node_document(&workspace).expect("current document");
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": node_id,
                    "revision": current.revision,
                    "source": format!("{}\n待损坏的草稿\n", current.source),
                })),
            )
            .expect("save draft for recovery issue");
        let scope = fs::read_dir(config.join("drafts"))
            .expect("draft scopes")
            .next()
            .expect("draft scope")
            .expect("draft scope entry")
            .path();
        let draft_path = fs::read_dir(scope)
            .expect("draft records")
            .find_map(|entry| {
                let entry = entry.ok()?;
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json")
                    .then(|| entry.path())
            })
            .expect("draft record");
        fs::write(draft_path, b"{").expect("corrupt draft record");
        let recovery_block = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "create",
                    "nodeId": node_id,
                    "kind": "comment",
                    "target": {"kind": "document"},
                    "bodySource": "恢复问题必须先解决",
                    "authorId": REVIEWER_ID,
                    "authorName": "Desktop reviewer",
                    "timestamp": "2026-08-24T14:02:00+08:00"
                })),
            )
            .expect_err("recovery issue blocks annotation preview");
        assert!(recovery_block.contains("完整已保存源集"));
    }

    #[test]
    fn desktop_boundary_exercises_properties_annotations_chrono_and_resources() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read");
        let node_id = snapshot.node_id.to_string();
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");

        let property = backend
            .request(
                "/api/document/property",
                Some(json!({
                    "source": snapshot.source,
                    "key": "status",
                    "value": "review",
                })),
            )
            .expect("property patch");
        assert!(property["source"]
            .as_str()
            .is_some_and(|source| source.contains(":status: review")));

        let annotation = backend
            .request(
                "/api/annotation/preview",
                Some(json!({
                    "action": "create",
                    "nodeId": &node_id,
                    "kind": "comment",
                    "target": {"kind": "document"},
                    "appearance": {"mark": "highlight", "theme": "yellow"},
                    "bodySource": "请核对",
                    "labels": ["verify"],
                    "authorId": "70d407dd-8538-45da-bb3d-d2eb4baa8539",
                    "authorName": "Desktop reviewer",
                    "timestamp": "2026-08-21T12:00:00Z",
                })),
            )
            .expect("annotation preview");
        let annotation_commit = backend
            .request(
                &format!("/api/annotation/commit?nodeId={node_id}"),
                Some(json!({"planId": annotation["plan"]["planId"]})),
            )
            .expect("annotation commit");
        assert_eq!(
            annotation_commit["annotations"]["annotations"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        let annotation_sidecar = workspace.join("weftext.annotations.json");
        assert!(annotation_sidecar.is_file());
        assert!(!workspace.join("_weftext.annotations.json").exists());
        let annotation_json = fs::read_to_string(annotation_sidecar).expect("annotation sidecar");
        assert!(annotation_json.contains("weftext.asciidoc.inline.v1"));
        assert!(annotation_json.contains("Desktop reviewer"));

        let chrono = backend
            .request(
                "/api/chrono/preview",
                Some(json!({
                    "chronoRootId": node_id,
                    "year": 2026,
                    "month": 8,
                    "day": 21,
                    "periods": ["day"],
                })),
            )
            .expect("Chrono preview");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": chrono["plan"]["planId"]})),
            )
            .expect("Chrono commit");
        assert!(workspace.join("2026/2026-08-21/2026-08-21.adoc").is_file());

        let resource = backend
            .request(
                "/api/resource/preview",
                Some(json!({
                    "nodeId": node_id,
                    "name": "diagram.png",
                    "bytes": [137, 80, 78, 71],
                })),
            )
            .expect("resource preview");
        backend
            .request(
                "/api/resource/commit",
                Some(json!({"planId": resource["plan"]["planId"]})),
            )
            .expect("resource commit");
        assert_eq!(
            fs::read(workspace.join("diagram.png")).unwrap(),
            [137, 80, 78, 71]
        );
    }

    #[test]
    fn desktop_import_commits_the_exact_preview_once_and_obeys_write_guards() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read root");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let opened = backend.open_workspace(&workspace).expect("open");
        assert_eq!(opened["importTempRecovery"]["removedCount"], 0);
        assert_eq!(opened["importTempRecovery"]["skippedCount"], 0);

        let preview = backend
            .request(
                "/api/import/fake-preview",
                Some(json!({
                    "displayName": "acceptance.fake",
                    "bytes": b"WEFTEXT-FAKE/1\nImported\nDesktop intake\n".to_vec(),
                    "destination": "Imported",
                })),
            )
            .expect("import preview");
        let digest = preview["import"]["bundleDigest"]
            .as_str()
            .expect("bundle digest")
            .to_owned();
        assert_eq!(
            preview["import"]["proposal"]["nodes"][0]["locator"],
            "Imported"
        );
        assert!(preview["import"].get("sourceBytes").is_none());
        assert!(!workspace.join("Imported").exists());

        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": format!("{}\n未保存的导入前编辑\n", snapshot.source),
                })),
            )
            .expect("save draft");
        let draft_block = backend
            .request("/api/import/commit", Some(json!({"bundleDigest": digest})))
            .expect_err("draft blocks import commit");
        assert!(draft_block.contains("完整已保存源集"));
        backend
            .request(
                "/api/draft/discard",
                Some(json!({"nodeId": snapshot.node_id})),
            )
            .expect("discard draft");

        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable safe mode");
        let safe_mode_block = backend
            .request("/api/import/commit", Some(json!({"bundleDigest": digest})))
            .expect_err("safe mode blocks import commit");
        assert!(safe_mode_block.contains("安全模式"));
        backend
            .request("/api/safe-mode", Some(json!({"enabled": false})))
            .expect("disable safe mode");

        let committed = backend
            .request("/api/import/commit", Some(json!({"bundleDigest": digest})))
            .expect("commit exact import preview");
        assert_eq!(
            committed["import"]["proposalDigest"],
            preview["import"]["proposalDigest"]
        );
        assert_eq!(committed["import"]["transaction"]["action"], "import");
        assert!(workspace.join("Imported/Imported.adoc").is_file());
        assert_eq!(
            fs::read(workspace.join(".weftext-format")).unwrap(),
            b"weftext.asciidoc.v1\n"
        );

        let consumed = backend
            .request("/api/import/commit", Some(json!({"bundleDigest": digest})))
            .expect_err("preview is single-use");
        assert!(consumed.contains("预览已失效"));
    }

    #[test]
    fn desktop_replays_source_evidence_before_storing_an_import_bundle() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let temp_root =
            ImportTempRoot::initialize(temp.path().join("intake-temp")).expect("intake temp root");
        let mut bundle = preview_fake_import(
            &workspace,
            temp_root,
            "stored.fake",
            OriginClass::TestFixture,
            b"WEFTEXT-FAKE/1\nStored\noriginal\n".to_vec(),
            PortablePath::parse("Stored").expect("destination"),
            "2026-08-24T00:00:00Z",
        )
        .expect("preview");
        bundle.source_bytes = b"WEFTEXT-FAKE/1\nStored\nchanged after preview\n".to_vec();
        let material = serde_json::to_vec(&(
            &bundle.contract_version,
            &bundle.source_bytes,
            &bundle.source,
            &bundle.probe,
            &bundle.plan,
            &bundle.document,
            &bundle.proposal,
            &bundle.proposal_digest,
            &bundle.components,
            &bundle.base_workspace_revision,
            &bundle.preview_receipt,
        ))
        .expect("bundle authority");
        bundle.bundle_digest = weftext_import::sha256_bytes(&material);

        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let error = backend
            .store_import_plan(bundle.bundle_digest.to_string(), bundle)
            .expect_err("stored bundle source bytes must be replayed");
        assert!(error.contains("source artifact differs"));
        assert!(backend.import_plans.is_empty());
    }

    fn task_import_preview_request(parent_id: NodeId, destination_name: &str) -> Value {
        json!({
            "profile": "weftext.task-import.v1",
            "destinationParentId": parent_id,
            "destinationName": destination_name,
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
            "documents": [
                {"locator": "项目.md", "bytes": b"# Project\r\n- [ ] first task\r\n".to_vec()},
                {"locator": "子目录/完成.md", "bytes": b"- [x] completed task\n".to_vec()}
            ]
        })
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn desktop_task_source_set_uses_exact_review_opaque_receipt_capability_and_recovery() {
        let temporary = tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let root_id = root_node_id(&workspace).expect("root ID");
        let mut backend = DesktopBackend::new(temporary.path().join("config"));
        backend.open_workspace(&workspace).expect("open workspace");

        let mut unexpected_settings = task_import_preview_request(root_id, "RejectedTasks");
        unexpected_settings["settings"]["browserPath"] = json!("C:\\raw\\vault");
        assert!(backend
            .request("/api/import/task/preview", Some(unexpected_settings))
            .expect_err("nested settings are exact typed input")
            .contains("exact typed 契约"));
        assert!(backend.task_import_plans.is_empty());

        let preview = backend
            .request(
                "/api/import/task/preview",
                Some(task_import_preview_request(root_id, "ImportedTasks")),
            )
            .expect("task source-set preview");
        assert_eq!(preview["import"]["adapter"], "task_source_set");
        assert_eq!(preview["import"]["committable"], true);
        assert_eq!(
            preview["import"]["bundle"]["taskPlan"]["settings"]["dialect"],
            "markdown_checklist_v1"
        );
        assert_eq!(
            preview["import"]["bundle"]["sourceDocuments"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert!(preview["import"]["bundle"]["sourceSetDigest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64));
        assert!(!workspace.join("ImportedTasks").exists());

        let review = preview["import"]["review"].clone();
        let bundle_key = review["bundleDigest"]
            .as_str()
            .expect("bundle digest")
            .to_owned();
        let retained = backend
            .task_import_plans
            .get(&bundle_key)
            .cloned()
            .expect("stored exact pending plan");
        let receipt_path = temporary.path().join("task-import-receipt.json");
        let destination = backend
            .register_task_import_receipt_destination(receipt_path.clone())
            .expect("system-selected receipt destination");
        let capability = destination["capability"]
            .as_str()
            .expect("opaque capability")
            .to_owned();

        let raw_path = backend
            .request(
                "/api/import/task/commit",
                Some(json!({
                    "review": review,
                    "receiptDestinationCapability": capability,
                    "receiptPath": receipt_path,
                })),
            )
            .expect_err("browser-native path field is outside the typed contract");
        assert!(raw_path.contains("未知字段") || raw_path.contains("unknown field"));
        assert!(backend
            .task_import_receipt_destinations
            .contains_key(&capability));

        let committed = backend
            .request(
                "/api/import/task/commit",
                Some(json!({
                    "review": review,
                    "receiptDestinationCapability": capability,
                })),
            )
            .expect("commit exact task source-set review");
        assert_eq!(committed["import"]["stage"], "committed");
        assert_eq!(
            committed["import"]["receipt"]["contractVersion"],
            "weftext.task-import-receipt.v1"
        );
        assert!(receipt_path.is_file());
        assert!(workspace.join("ImportedTasks/项目/项目.adoc").is_file());
        assert!(workspace
            .join("ImportedTasks/子目录/完成/完成.adoc")
            .is_file());

        let replay = backend
            .request(
                "/api/import/task/commit",
                Some(json!({
                    "review": review,
                    "receiptDestinationCapability": "already-consumed",
                })),
            )
            .expect_err("stored review is single-use");
        assert!(replay.contains("预览已失效"));

        backend.task_import_plans.insert(
            bundle_key,
            PendingTaskImportPlan {
                receipt_path: Some(receipt_path),
                ..retained
            },
        );
        let recovered = backend
            .request("/api/import/task/recover", Some(json!({"review": review})))
            .expect("idempotent exact receipt recovery");
        assert_eq!(
            recovered["import"]["recovery"]["status"],
            "already_finalized"
        );
        assert!(backend.task_import_plans.is_empty());
    }

    #[test]
    fn desktop_task_source_set_pending_plans_obey_draft_safe_mode_stale_switch_and_capacity_guards()
    {
        let temporary = tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("root document");
        let root_id = snapshot.node_id;
        let mut backend = DesktopBackend::new(temporary.path().join("config"));
        backend.open_workspace(&workspace).expect("open workspace");
        let preview = backend
            .request(
                "/api/import/task/preview",
                Some(task_import_preview_request(root_id, "GuardedTasks")),
            )
            .expect("task preview");
        let review = preview["import"]["review"].clone();
        let receipt = temporary.path().join("guarded-receipt.json");
        let capability = backend
            .register_task_import_receipt_destination(receipt)
            .expect("receipt destination")["capability"]
            .as_str()
            .expect("capability")
            .to_owned();
        let commit_body = || {
            json!({
                "review": review,
                "receiptDestinationCapability": capability,
            })
        };

        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": format!("{}\nunsaved\n", snapshot.source),
                })),
            )
            .expect("device draft");
        assert!(backend
            .request("/api/import/task/commit", Some(commit_body()))
            .expect_err("draft guard")
            .contains("完整已保存"));
        backend
            .request(
                "/api/draft/discard",
                Some(json!({"nodeId": snapshot.node_id})),
            )
            .expect("discard draft");
        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("safe mode");
        assert!(backend
            .request("/api/import/task/commit", Some(commit_body()))
            .expect_err("safe mode guard")
            .contains("安全模式"));
        backend
            .request("/api/safe-mode", Some(json!({"enabled": false})))
            .expect("disable safe mode");

        let change = plan_create_child_node(&workspace, root_id, "ExternalChange")
            .expect("external workspace plan");
        commit_workspace_transaction(&change).expect("external workspace change");
        assert!(backend
            .request("/api/import/task/commit", Some(commit_body()))
            .expect_err("stale revision guard")
            .contains("工作区已更改"));

        let second = temporary.path().join("SecondWorkspace");
        create_workspace(&second).expect("second workspace");
        backend.open_workspace(&second).expect("switch workspace");
        assert!(backend.task_import_plans.is_empty());
        assert!(backend.task_import_receipt_destinations.is_empty());

        let second_root = root_node_id(&second).expect("second root");
        let first = backend
            .request(
                "/api/import/task/preview",
                Some(task_import_preview_request(second_root, "CapacityOne")),
            )
            .expect("one pending preview");
        let first_key = first["import"]["review"]["bundleDigest"]
            .as_str()
            .expect("first bundle key")
            .to_owned();
        let stored = backend
            .task_import_plans
            .get(&first_key)
            .cloned()
            .expect("stored pending preview");
        while backend.task_import_plans.len() < MAX_PENDING_TASK_IMPORT_PLANS {
            let key = format!("capacity-{}", backend.task_import_plans.len());
            backend.task_import_plans.insert(key, stored.clone());
        }
        let capacity = backend
            .request(
                "/api/import/task/preview",
                Some(task_import_preview_request(second_root, "CapacityBlocked")),
            )
            .expect_err("bounded pending plan cache");
        assert!(capacity.contains("预览过多"));
    }

    #[test]
    fn desktop_agent_import_discloses_selected_ir_and_accepts_only_typed_patch() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        let local = backend
            .request(
                "/api/import/fake-preview",
                Some(json!({
                    "displayName": "agent.fake",
                    "bytes": b"WEFTEXT-FAKE/1\nAgent import\nuncertain text\n".to_vec(),
                    "destination": "AgentImported",
                })),
            )
            .expect("local extraction");
        let base_bundle_digest = local["import"]["bundleDigest"]
            .as_str()
            .expect("base bundle digest")
            .to_owned();
        let node_id = local["import"]["document"]["nodes"][0]["id"]
            .as_str()
            .expect("IR node id")
            .to_owned();
        let selection = backend
            .request(
                "/api/import/agent/prepare",
                Some(json!({
                    "bundleDigest": base_bundle_digest,
                    "provider": "reviewed-provider",
                    "selectedNodeIds": [node_id],
                    "retention": "delete-after-call",
                    "redaction": "none",
                })),
            )
            .expect("egress selection preview");
        assert_eq!(selection["agentEnhancement"]["networkExecuted"], false);
        assert_eq!(
            selection["agentEnhancement"]["requiresExplicitEgressApproval"],
            true
        );
        assert_eq!(
            selection["agentEnhancement"]["evidence"]["selectedNodeIds"],
            json!([node_id])
        );
        assert!(selection["agentEnhancement"]["evidence"]
            .get("sourceBytes")
            .is_none());
        let preview_digest = selection["agentEnhancement"]["previewDigest"]
            .as_str()
            .expect("preview digest")
            .to_owned();
        let patch = agent_text_patch(&local, &selection, node_id);
        let denied = apply_agent_patch(&mut backend, &preview_digest, false, &patch)
            .expect_err("egress approval is explicit and fail closed");
        assert!(denied.contains("显式出站审批"));
        let enhanced = apply_agent_patch(&mut backend, &preview_digest, true, &patch)
            .expect("apply approved typed patch");
        assert_eq!(enhanced["import"]["requiresFinalCommitApproval"], true);
        assert!(enhanced["import"]["proposal"]["nodes"][0]["exactAsciidoc"]
            .as_str()
            .expect("proposed source")
            .contains("verified correction"));
        assert_eq!(
            enhanced["import"]["receipt"]["agentProvenance"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        let replay = apply_agent_patch(&mut backend, &preview_digest, true, &patch)
            .expect_err("approved selection is single-use");
        assert!(replay.contains("已失效"));

        let enhanced_digest = enhanced["import"]["bundleDigest"]
            .as_str()
            .expect("enhanced bundle digest");
        backend
            .request(
                "/api/import/commit",
                Some(json!({"bundleDigest": enhanced_digest})),
            )
            .expect("final approved Core commit");
        let source = fs::read_to_string(workspace.join("AgentImported/AgentImported.adoc"))
            .expect("committed canonical source");
        assert!(source.contains("verified correction"));
    }

    fn apply_agent_patch(
        backend: &mut DesktopBackend,
        preview_digest: &str,
        egress_approved: bool,
        patch: &weftext_import::AgentImportPatch,
    ) -> Result<Value, String> {
        backend.request(
            "/api/import/agent/apply-approved-patch",
            Some(json!({
                "previewDigest": preview_digest,
                "egressApproved": egress_approved,
                "patch": patch,
            })),
        )
    }

    fn agent_text_patch(
        local: &Value,
        selection: &Value,
        node_id: String,
    ) -> weftext_import::AgentImportPatch {
        let base_ir_revision = weftext_import::Sha256Digest::parse(
            local["import"]["document"]["revision"]
                .as_str()
                .expect("IR revision"),
        )
        .expect("digest");
        let egress = serde_json::from_value(
            selection["agentEnhancement"]["authorizedPlan"]["egress"].clone(),
        )
        .expect("typed egress");
        weftext_import::AgentImportPatch::create(
            base_ir_revision,
            vec![node_id.clone()],
            vec![weftext_import::AgentPatchOperation::CorrectText {
                node_id,
                expected_text_digest: weftext_import::sha256_bytes(b"uncertain text"),
                replacement: "verified correction".to_owned(),
            }],
            "reviewed-provider",
            "reviewed-model",
            egress,
        )
        .expect("typed patch")
    }

    #[test]
    fn desktop_markdown_import_is_explicit_preview_only_and_rejects_active_content() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        let markdown = concat!(
            "---\r\n",
            "status: draft\r\n",
            "---\r\n",
            "# 导入标题 😀\r\n",
            "正文 **加粗** 与 [链接](https://example.test)。\r\n",
        )
        .as_bytes()
        .to_vec();
        let base_revision = read_workspace_revision(&workspace).expect("base revision");

        let preview = backend
            .request(
                "/api/import/markdown/preview",
                Some(json!({
                    "displayName": "输入.md",
                    "bytes": markdown,
                    "destination": "ImportedMarkdown",
                    "retainOriginal": true,
                })),
            )
            .expect("Markdown preview");
        let digest = preview["import"]["bundleDigest"]
            .as_str()
            .expect("bundle digest")
            .to_owned();
        assert_eq!(
            preview["import"]["plan"]["route"]["adapter"]["adapterId"],
            "weftext.markdown-compatibility"
        );
        assert_eq!(
            preview["import"]["plan"]["resourcePolicy"],
            "extract_and_retain_original"
        );
        assert_eq!(
            preview["import"]["proposal"]["nodes"][0]["resources"][0]["locator"],
            "original-输入.md.source"
        );
        assert_eq!(
            read_workspace_revision(&workspace).expect("preview revision"),
            base_revision
        );
        assert!(!workspace.join("ImportedMarkdown").exists());

        backend
            .request("/api/import/commit", Some(json!({"bundleDigest": digest})))
            .expect("commit reviewed Markdown bundle");
        let imported = workspace.join("ImportedMarkdown");
        let exact =
            fs::read_to_string(imported.join("ImportedMarkdown.adoc")).expect("canonical AsciiDoc");
        assert!(exact.contains("= 导入标题 😀"));
        assert!(!imported.join("ImportedMarkdown.md").exists());
        assert_eq!(
            fs::read(imported.join("original-输入.md.source")).expect("retained source"),
            concat!(
                "---\r\n",
                "status: draft\r\n",
                "---\r\n",
                "# 导入标题 😀\r\n",
                "正文 **加粗** 与 [链接](https://example.test)。\r\n",
            )
            .as_bytes()
        );

        let before_unsafe = read_workspace_revision(&workspace).expect("revision before reject");
        let error = backend
            .request(
                "/api/import/markdown/preview",
                Some(json!({
                    "displayName": "unsafe.md",
                    "bytes": b"# Unsafe\n<script>alert(1)</script>\n".to_vec(),
                    "destination": "Unsafe",
                    "retainOriginal": false,
                })),
            )
            .expect_err("active Markdown is rejected before authority is cached");
        assert!(error.contains("did not authorize") || error.contains("active"));
        assert!(backend.import_plans.is_empty());
        assert!(!workspace.join("Unsafe").exists());
        assert_eq!(
            read_workspace_revision(&workspace).expect("revision after reject"),
            before_unsafe
        );
    }

    #[test]
    fn desktop_markdown_export_commits_exact_external_bytes_once() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read root");
        let base_workspace_revision =
            read_workspace_revision(&workspace).expect("base workspace revision");
        let destination = temp.path().join("导出-保留.md");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        let destination_capability = backend
            .register_markdown_export_destination(destination.clone())
            .expect("system-selected export destination")["capability"]
            .as_str()
            .expect("opaque destination capability")
            .to_owned();

        let preview = backend
            .request(
                "/api/export/markdown/preview",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "destinationCapability": destination_capability.clone(),
                    "metadataPolicy": "preserve_weftext",
                })),
            )
            .expect("Markdown export preview");
        let plan_id = preview["export"]["plan"]["planId"]
            .as_str()
            .expect("plan ID")
            .to_owned();
        let reviewed_artifact = preview["export"]["plan"]["artifact"]
            .as_str()
            .expect("reviewed artifact")
            .as_bytes()
            .to_vec();
        assert_eq!(preview["export"]["stage"], "preview");
        assert_eq!(
            preview["export"]["plan"]["metadataPolicy"],
            "preserve_weftext"
        );
        assert!(!destination.exists());
        assert_eq!(
            read_workspace_revision(&workspace).expect("revision after preview"),
            base_workspace_revision
        );
        assert_eq!(
            read_node_document(&workspace)
                .expect("source after preview")
                .source,
            snapshot.source
        );
        let reused_destination = backend
            .request(
                "/api/export/markdown/preview",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "destinationCapability": destination_capability,
                    "metadataPolicy": "preserve_weftext",
                })),
            )
            .expect_err("destination capability is single-use");
        assert!(reused_destination.contains("授权无效或已使用"));

        let raw_path = backend
            .request(
                "/api/export/markdown/preview",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "destination": temp.path().join("bypass.md"),
                    "metadataPolicy": "preserve_weftext",
                })),
            )
            .expect_err("raw WebView paths are not export authority");
        assert!(raw_path.contains("字段无效") || raw_path.contains("unknown field"));

        let committed = backend
            .request(
                "/api/export/commit",
                Some(json!({"planId": plan_id.clone()})),
            )
            .expect("commit reviewed Markdown artifact");
        assert_eq!(committed["export"]["stage"], "committed");
        assert_eq!(committed["export"]["receipt"]["planId"], plan_id);
        assert_eq!(
            fs::read(&destination).expect("published artifact"),
            reviewed_artifact
        );
        assert_eq!(
            read_workspace_revision(&workspace).expect("revision after export"),
            base_workspace_revision
        );
        assert_eq!(
            read_node_document(&workspace)
                .expect("source after export")
                .source,
            snapshot.source
        );

        let replay = backend
            .request("/api/export/commit", Some(json!({"planId": plan_id})))
            .expect_err("export preview is single-use");
        assert!(replay.contains("预览已失效"));
    }

    #[test]
    fn desktop_markdown_export_commit_obeys_draft_and_safe_mode_guards() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read root");
        let destination = temp.path().join("导出-受保护.md");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        let destination_capability = backend
            .register_markdown_export_destination(destination.clone())
            .expect("system-selected export destination")["capability"]
            .as_str()
            .expect("opaque destination capability")
            .to_owned();
        let preview = backend
            .request(
                "/api/export/markdown/preview",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "destinationCapability": destination_capability,
                    "metadataPolicy": "preserve_weftext",
                })),
            )
            .expect("Markdown export preview");
        let plan_id = preview["export"]["plan"]["planId"]
            .as_str()
            .expect("plan ID")
            .to_owned();

        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": format!("{}\n未保存的导出前编辑\n", snapshot.source),
                })),
            )
            .expect("save draft");
        let draft_block = backend
            .request(
                "/api/export/commit",
                Some(json!({"planId": plan_id.clone()})),
            )
            .expect_err("draft blocks export commit");
        assert!(draft_block.contains("完整已保存源集"));
        backend
            .request(
                "/api/draft/discard",
                Some(json!({"nodeId": snapshot.node_id})),
            )
            .expect("discard draft");

        backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable safe mode");
        let safe_mode_block = backend
            .request(
                "/api/export/commit",
                Some(json!({"planId": plan_id.clone()})),
            )
            .expect_err("safe mode blocks export commit");
        assert!(safe_mode_block.contains("安全模式"));
        assert!(!destination.exists());
        backend
            .request("/api/safe-mode", Some(json!({"enabled": false})))
            .expect("disable safe mode");
        backend
            .request("/api/export/commit", Some(json!({"planId": plan_id})))
            .expect("guards preserve the reviewed plan");
        assert!(destination.is_file());
    }

    #[test]
    fn desktop_markdown_export_remove_policy_is_explicit_and_preview_only() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read root");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        let plain_destination = temp.path().join("导出-纯文本.md");
        let destination_capability = backend
            .register_markdown_export_destination(plain_destination.clone())
            .expect("system-selected export destination")["capability"]
            .as_str()
            .expect("opaque destination capability")
            .to_owned();
        let plain = backend
            .request(
                "/api/export/markdown/preview",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "destinationCapability": destination_capability,
                    "metadataPolicy": "remove_weftext",
                })),
            )
            .expect("plain Markdown preview");
        assert!(!plain_destination.exists());
        assert!(plain["export"]["plan"]["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "weftext_metadata_removed"));
        assert!(!plain["export"]["plan"]["artifact"]
            .as_str()
            .expect("plain artifact")
            .contains("weftext:"));
    }

    #[test]
    fn desktop_pdf_capability_and_preview_fail_closed_without_packaged_sandbox() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let installation = temp.path().join("packaged-docling-lite");
        fs::create_dir(&installation).expect("installation directory");
        let mut backend =
            DesktopBackend::new_with_docling_installation(temp.path().join("config"), installation);
        let opened = backend.open_workspace(&workspace).expect("open");
        assert_eq!(
            opened["importCapabilities"]["doclingLite"]["available"],
            false
        );

        let capability = backend
            .request("/api/import/pdf-capability", None)
            .expect("PDF capability");
        assert_eq!(capability["import"]["adapter"], "docling_lite");
        assert_eq!(capability["import"]["capability"]["available"], false);
        assert_eq!(
            capability["import"]["capability"]["ambientNetworkAllowed"],
            false
        );
        let error = backend
            .request(
                "/api/import/pdf-preview",
                Some(json!({
                    "displayName": "input.pdf",
                    "bytes": b"%PDF-1.7\n".to_vec(),
                    "destination": "Imported",
                })),
            )
            .expect_err("PDF preview remains gated");
        assert!(error.contains("docling") || error.contains("Docling"));
        assert!(!workspace.join("Imported").exists());
    }

    #[test]
    fn desktop_node_metadata_uses_stored_narrow_plans_and_draft_guards() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let child = weftext_core::create_child_node(&workspace, "Child").expect("child");
        let before = read_node_document(&child.path).expect("child source");
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");

        let preview = backend
            .request(
                "/api/node/metadata/preview",
                Some(json!({
                    "action": "aliases",
                    "nodeId": child.id,
                    "revision": before.revision,
                    "aliases": ["别名", "Alias"],
                })),
            )
            .expect("aliases preview");
        assert_eq!(preview["plan"]["action"], "node_metadata");
        assert_eq!(
            read_node_document(&child.path)
                .expect("preview is read-only")
                .source,
            before.source
        );

        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": child.id,
                    "revision": before.revision,
                    "source": format!("{}\n未保存\n", before.source),
                })),
            )
            .expect("save draft");
        let blocked = backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": preview["plan"]["planId"]})),
            )
            .expect_err("draft blocks metadata commit");
        assert!(blocked.contains("操作范围命中 1 个设备草稿"));
        backend
            .request("/api/draft/discard", Some(json!({"nodeId": child.id})))
            .expect("discard draft");
        let refreshed = backend
            .request(
                "/api/node/metadata/preview",
                Some(json!({
                    "action": "aliases",
                    "nodeId": child.id,
                    "revision": before.revision,
                    "aliases": ["别名", "Alias"],
                })),
            )
            .expect("refresh preview after resolving draft");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": refreshed["plan"]["planId"]})),
            )
            .expect("commit refreshed plan");
        let aliased = read_node_document(&child.path).expect("aliased source");
        assert!(aliased
            .source
            .contains("  aliases:\n    - \"别名\"\n    - \"Alias\"\n"));

        let rank = backend
            .request(
                "/api/node/metadata/preview",
                Some(json!({
                    "action": "sibling_rank",
                    "nodeId": child.id,
                    "revision": aliased.revision,
                    "siblingRank": 1024,
                })),
            )
            .expect("rank preview");
        backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": rank["plan"]["planId"]})),
            )
            .expect("rank commit");
        assert!(read_node_document(&child.path)
            .expect("ranked source")
            .source
            .contains("  sibling_rank: 1024\n"));

        let malformed = backend
            .request(
                "/api/node/metadata/preview",
                Some(json!({
                    "action": "child_sort",
                    "nodeId": child.id,
                    "revision": read_node_document(&child.path).unwrap().revision,
                    "mode": "manual",
                    "direction": "descending",
                })),
            )
            .expect_err("manual direction is ambiguous");
        assert!(malformed.contains("manual 排序不接受 direction"));
    }

    #[test]
    fn restores_a_durable_draft_and_detects_an_external_revision() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let config = temp.path().join("config");
        let snapshot = read_node_document(&workspace).expect("read");

        let mut first = DesktopBackend::new(config.clone());
        first.open_workspace(&workspace).expect("open");
        let saved = first
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": format!("{}\n未提交但可恢复", snapshot.source),
                })),
            )
            .expect("save draft");
        assert_eq!(saved["clean"], false);

        let external_source = format!("{}\n外部修改", snapshot.source);
        let external = plan_document_edit(
            &workspace,
            &snapshot.revision,
            [DocumentEdit {
                start: 0,
                end: u64::try_from(snapshot.source.len()).expect("length"),
                replacement: external_source.clone(),
            }],
        )
        .expect("external plan");
        commit_document_edit(&external).expect("external commit");

        let mut restarted = DesktopBackend::new(config);
        let restored = restarted.restore_workspace().expect("restore");
        assert_eq!(restored["opened"], true);
        assert_eq!(restored["document"]["source"], external_source);
        assert_eq!(restored["document"]["recoveryDraft"]["stale"], true);
        assert!(restored["document"]["recoveryDraft"]["source"]
            .as_str()
            .is_some_and(|source| source.contains("未提交但可恢复")));
        assert_eq!(
            restored["draftRecovery"]["drafts"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            read_node_document(&workspace)
                .expect("read after restore")
                .source,
            external_source
        );
    }

    #[test]
    fn desktop_citations_are_read_only_until_typed_reference_data_is_accepted() {
        let temporary = tempdir().expect("citation workspace parent");
        let root = temporary.path().join("Citations");
        let root_id = "11111111-1111-4111-8111-111111111111";
        let component_id = "22222222-2222-4222-8222-222222222222";
        fs::create_dir_all(root.join("Component")).expect("component directory");
        fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
        fs::write(
            root.join("Citations.adoc"),
            format!("---\nweftext:\n  id: \"{root_id}\"\n---\n= Citations\n"),
        )
        .expect("root source");
        fs::write(
            root.join("Component/Component.adoc"),
            format!(
                "---\nweftext:\n  id: \"{component_id}\"\n---\n= Component\n\nEvidence without a reference record.\n\nbibliography::[]\n"
            ),
        )
        .expect("component source");

        let mut backend = DesktopBackend::new(temporary.path().join("config"));
        backend.open_workspace(&root).expect("open citations");
        let capabilities = backend
            .request("/api/citation/capabilities", None)
            .expect("capabilities");
        assert_eq!(
            capabilities["capabilities"]["providerId"],
            "weftext.hayagriva"
        );
        assert_eq!(
            capabilities["capabilities"]["referenceRecordWritesAvailable"],
            false
        );
        assert!(capabilities["capabilities"]["referenceRecordWritesReason"].is_string());

        let validation = backend
            .request("/api/citation/validate", None)
            .expect("validate");
        assert_eq!(validation["validation"]["valid"], true);
        let search = backend
            .request("/api/citation/search?q=anything&limit=10", None)
            .expect("search references");
        assert_eq!(search["references"], json!([]));

        let component = read_node_document(root.join("Component")).expect("component");
        let analyze = backend
            .request(
                "/api/citation/analyze",
                Some(json!({
                    "nodeId": component_id,
                    "source": component.source,
                    "styleId": "apa",
                    "locale": "en-US",
                })),
            )
            .expect("analyze empty bibliography");
        assert_eq!(analyze["analysis"]["clusters"], json!([]));
        assert!(analyze["presentation"].is_object());

        for route in [
            "/api/citation/reference/create-preview",
            "/api/citation/reference/edit-preview",
            "/api/citation/rename-preview",
            "/api/citation/transaction/commit",
            "/api/citation/rollback",
        ] {
            assert_eq!(
                backend.request(route, None).expect_err("retired route"),
                "未知的 Desktop Core 请求"
            );
        }
        assert!(!root.join("Reference").exists());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn desktop_task_routes_store_exact_plans_and_block_device_drafts() {
        const ROOT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
        const CHILD_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
        const TASK_A: &str = "11111111-1111-4111-8111-111111111111";
        const TASK_B: &str = "22222222-2222-4222-8222-222222222222";
        const TASK_R: &str = "33333333-3333-4333-8333-333333333333";

        let temporary = tempdir().expect("task workspace parent");
        let root = temporary.path().join("Tasks");
        fs::create_dir_all(root.join("Child")).expect("child directory");
        fs::write(root.join(".weftext-format"), "weftext.asciidoc.v1\n").expect("format marker");
        fs::write(
            root.join("Tasks.adoc"),
            format!(
                concat!(
                    "---\nweftext:\n  id: \"{}\"\n---\n= Tasks\n\n",
                    "* [ ] Editable task:[id={}]\n",
                    "* [ ] Repeat task:[id={},due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=2\",repeat-from=due]\n"
                ),
                ROOT_ID,
                TASK_A,
                TASK_R,
            ),
        )
        .expect("root task source");
        fs::write(
            root.join("Child/Child.adoc"),
            format!(
                "---\nweftext:\n  id: \"{CHILD_ID}\"\n---\n= Child\n\n* [ ] Dependency task:[id={TASK_B}]\n"
            ),
        )
        .expect("child task source");

        let mut backend = DesktopBackend::new(temporary.path().join("config"));
        let opened = backend.open_workspace(&root).expect("open tasks");
        let validation = backend
            .request("/api/task/validate", None)
            .expect("validate tasks");
        assert_eq!(validation["validation"]["valid"], true);
        assert_eq!(
            validation["validation"]["occurrences"]
                .as_array()
                .map(Vec::len),
            Some(3)
        );
        let inspection = backend
            .request(&format!("/api/task/inspect?nodeId={ROOT_ID}"), None)
            .expect("inspect tasks");
        assert_eq!(inspection["occurrences"].as_array().map(Vec::len), Some(2));
        let query = backend
            .request(
                "/api/query/execute",
                Some(json!({
                    "source": "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
                    "blockIndex": 0,
                    "context": {
                        "today": {"year": 2026, "month": 8, "day": 24},
                        "now": "2026-08-24T12:00:00+08:00",
                        "timezone": "Asia/Shanghai",
                        "locale": "zh-CN",
                        "binding": {"nodeId": ROOT_ID, "heading": null},
                    },
                })),
            )
            .expect("execute node query");
        assert_eq!(query["valid"], true);
        assert_eq!(query["execution"]["result"]["totalBeforeLimit"], 2);
        assert_eq!(
            query["execution"]["result"]["columns"]
                .as_array()
                .expect("query columns")
                .iter()
                .map(|column| column["path"].as_str().expect("column path"))
                .collect::<Vec<_>>(),
            vec!["name", "path"]
        );
        assert_eq!(
            query["execution"]["csv"],
            "name,path\r\nTasks,/\r\nChild,/Child\r\n"
        );
        assert_eq!(
            query["execution"]["result"]["rows"][1]["cells"][1]["value"]["value"],
            "/Child"
        );

        let document = read_node_document(&root).expect("root snapshot");
        let edit = backend
            .request(
                "/api/task/edit-preview",
                Some(json!({
                    "nodeId": ROOT_ID,
                    "baseWorkspaceRevision": opened["workspace"]["revision"],
                    "baseRevision": document.revision,
                    "target": {"kind": "id", "id": TASK_A},
                    "intent": {"kind": "set_priority", "priority": "high"},
                })),
            )
            .expect("task edit preview");
        assert!(edit["plan"]["authoring"]["proposedSource"]
            .as_str()
            .expect("proposed source")
            .contains("priority=high"));
        assert!(!fs::read_to_string(root.join("Tasks.adoc"))
            .expect("unchanged preview source")
            .contains("priority=high"));
        let edit_plan_id = edit["plan"]["planId"]
            .as_str()
            .expect("edit plan ID")
            .to_owned();
        let edited = backend
            .request(
                "/api/task/transaction/commit",
                Some(json!({"planId": edit_plan_id})),
            )
            .expect("task edit commit");
        assert_eq!(edited["result"]["task"]["metadata"]["priority"], "high");
        assert!(backend
            .request(
                "/api/task/transaction/commit",
                Some(json!({"planId": edit_plan_id})),
            )
            .is_err());

        let document = read_node_document(&root).expect("dependency snapshot");
        let workspace = workspace_payload(&root).expect("dependency workspace");
        let dependencies = backend
            .request(
                "/api/task/dependencies-preview",
                Some(json!({
                    "nodeId": ROOT_ID,
                    "baseWorkspaceRevision": workspace["revision"],
                    "baseRevision": document.revision,
                    "target": {"kind": "id", "id": TASK_A},
                    "dependencies": [TASK_B],
                })),
            )
            .expect("dependency preview");
        let depended = backend
            .request(
                "/api/task/transaction/commit",
                Some(json!({"planId": dependencies["plan"]["planId"]})),
            )
            .expect("dependency commit");
        assert_eq!(depended["result"]["dependencies"][0], TASK_B);

        let document = read_node_document(&root).expect("recurrence snapshot");
        let workspace = workspace_payload(&root).expect("recurrence workspace");
        let recurrence = backend
            .request(
                "/api/task/recurrence-preview",
                Some(json!({
                    "nodeId": ROOT_ID,
                    "baseWorkspaceRevision": workspace["revision"],
                    "baseRevision": document.revision,
                    "target": {"kind": "id", "id": TASK_R},
                    "context": {
                        "completedAt": {"kind": "date", "value": "2026-08-24"},
                        "utcOffsetMinutes": 480,
                    },
                })),
            )
            .expect("recurrence preview");
        let next_id = recurrence["plan"]["completion"]["nextTaskId"].clone();
        let recurred = backend
            .request(
                "/api/task/transaction/commit",
                Some(json!({"planId": recurrence["plan"]["planId"]})),
            )
            .expect("recurrence commit");
        assert_eq!(recurred["result"]["nextTaskId"], next_id);

        let document = read_node_document(&root).expect("draft snapshot");
        let draft_source = format!("{}\nUnsaved task context.\n", document.source);
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": ROOT_ID,
                    "revision": document.revision,
                    "source": draft_source,
                })),
            )
            .expect("save device draft");
        let workspace = workspace_payload(&root).expect("draft workspace");
        let blocked = backend
            .request(
                "/api/task/edit-preview",
                Some(json!({
                    "nodeId": ROOT_ID,
                    "baseWorkspaceRevision": workspace["revision"],
                    "baseRevision": document.revision,
                    "target": {"kind": "id", "id": TASK_A},
                    "intent": {"kind": "set_priority", "priority": "low"},
                })),
            )
            .expect_err("device draft must block task plan");
        assert!(blocked.contains("完整已保存源集"));
        backend
            .request("/api/draft/discard", Some(json!({"nodeId": ROOT_ID})))
            .expect("discard device draft");
        let recovery = backend
            .request("/api/task/recover", None)
            .expect("task recovery");
        assert_eq!(recovery["recovery"]["applyingRolledBack"], 0);
    }

    #[test]
    fn committed_document_edit_clears_the_device_draft() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let snapshot = read_node_document(&workspace).expect("read");
        let source = format!("{}\n准备提交", snapshot.source);
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        backend.open_workspace(&workspace).expect("open");
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": source,
                })),
            )
            .expect("draft");
        let committed = backend
            .request(
                "/api/document/commit",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": source,
                })),
            )
            .expect("commit");
        assert_eq!(committed["draftRecovery"]["drafts"], json!([]));
        let drafts = backend.request("/api/drafts", None).expect("drafts");
        assert_eq!(drafts["draftRecovery"]["drafts"], json!([]));
    }

    #[test]
    fn authoritative_commits_succeed_when_derived_index_refresh_fails() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let config = temp.path().join("config");
        let mut backend = DesktopBackend::new(config.clone());
        backend.open_workspace(&workspace).expect("open");

        let index_directory = config.join("search-indexes");
        fs::remove_dir_all(&index_directory).expect("remove derived index directory");
        fs::write(&index_directory, b"failure injection").expect("block index directory creation");

        let snapshot = read_node_document(&workspace).expect("read");
        let source = format!("{}\n权威提交不能被索引失败反转", snapshot.source);
        let committed = backend
            .request(
                "/api/document/commit",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": source,
                })),
            )
            .expect("document commit remains successful");
        assert_eq!(committed["ok"], true);
        assert!(committed["searchIndex"].is_null());
        assert_eq!(
            committed["searchIndexWarning"]["code"],
            "derived_search_index_refresh_failed"
        );
        assert_eq!(
            committed["searchIndexWarning"]["authoritativeCommitSucceeded"],
            true
        );
        assert_eq!(
            read_node_document(&workspace)
                .expect("committed source")
                .source,
            source
        );

        let root_id = root_node_id(&workspace).expect("root id");
        let preview = backend
            .request(
                "/api/workspace/action/preview",
                Some(json!({
                    "action": "create",
                    "parentId": root_id,
                    "name": "CommittedDespiteIndexFailure",
                })),
            )
            .expect("structure preview");
        let structure = backend
            .request(
                "/api/workspace/action/commit",
                Some(json!({"planId": preview["plan"]["planId"]})),
            )
            .expect("structure commit remains successful");
        assert_eq!(structure["ok"], true);
        assert_eq!(
            structure["searchIndexWarning"]["code"],
            "derived_search_index_refresh_failed"
        );
        assert!(workspace
            .join("CommittedDespiteIndexFailure")
            .join("CommittedDespiteIndexFailure.adoc")
            .is_file());
    }

    #[test]
    fn safe_mode_persists_and_blocks_workspace_commits() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let config = temp.path().join("config");
        let snapshot = read_node_document(&workspace).expect("read");
        let source = format!("{}\n安全模式草稿", snapshot.source);
        let mut backend = DesktopBackend::new(config.clone());
        backend.open_workspace(&workspace).expect("open");
        let enabled = backend
            .request("/api/safe-mode", Some(json!({"enabled": true})))
            .expect("enable");
        assert_eq!(enabled["safeMode"], true);
        backend
            .request(
                "/api/draft/save",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": source,
                })),
            )
            .expect("device draft remains available");
        let error = backend
            .request(
                "/api/document/commit",
                Some(json!({
                    "nodeId": snapshot.node_id,
                    "revision": snapshot.revision,
                    "source": source,
                })),
            )
            .expect_err("commit blocked");
        assert!(error.contains("安全模式"));
        let recovery_error = backend
            .request("/api/task/recover", None)
            .expect_err("task recovery blocked");
        assert!(recovery_error.contains("安全模式"));

        let mut restarted = DesktopBackend::new(config);
        let restored = restarted.restore_workspace().expect("restore");
        assert_eq!(restored["safeMode"], true);
        assert_eq!(
            restored["draftRecovery"]["drafts"].as_array().map(Vec::len),
            Some(1)
        );
        let diagnostics = restarted
            .request("/api/diagnostics", None)
            .expect("diagnostics");
        assert_eq!(diagnostics["diagnostics"]["pathsRedacted"], true);
        assert_eq!(diagnostics["diagnostics"]["documentBodiesIncluded"], false);
    }

    #[test]
    fn desktop_routes_own_the_private_agent_session_and_revoke_it_on_workspace_switch() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        let second = temp.path().join("Second");
        create_workspace(&workspace).expect("workspace");
        create_workspace(&second).expect("second workspace");
        let root_id = read_node_document(&workspace)
            .expect("root document")
            .node_id;
        let mut backend = DesktopBackend::new(temp.path().join("config"));
        let opened = backend.open_workspace(&workspace).expect("open");
        assert_eq!(opened["agentCapability"]["agentExecutionAvailable"], false);
        assert_eq!(
            opened["agentCapability"]["approvalAndCommitAgentCallable"],
            false
        );

        let started = backend
            .request(
                "/api/agent/session/start",
                Some(json!({
                    "scopeNodeIds": [root_id],
                    "delegatedCapabilities": [
                        "read_workspace",
                        "search_workspace"
                    ],
                    "probeDshRuntime": false,
                })),
            )
            .expect("start private session");
        assert_eq!(started["session"]["scopeNodeIds"], json!([root_id]));
        assert_eq!(started["session"]["revoked"], false);
        assert!(backend
            .request("/api/agent/capability", Some(json!({})))
            .is_err());

        let switched = backend.open_workspace(&second).expect("switch workspace");
        assert!(switched["agentCapability"]["activeSession"].is_null());
        assert!(backend
            .request("/api/agent/session/recovery", None)
            .is_err());
    }
}
