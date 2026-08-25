//! Weftext's headless domain core.

mod annotations;
mod asciidoc_adapter;
mod checklist;
mod checklist_action;
mod chrono;
mod citation;
mod citation_authoring;
mod citation_presentation;
mod citation_workspace;
mod content_boundary;
mod document;
mod document_envelope;
mod document_format;
mod document_formatting;
mod document_model;
mod document_properties;
mod frontmatter;
mod icon;
mod identity;
mod links;
mod navigation;
mod node;
mod physical_inventory;
mod portable_name;
mod property;
mod query;
mod query_workspace;
mod reference;
mod resource;
mod search;
mod search_index;
mod sorting;
mod source_lexing;
mod sync;
mod task;
mod task_action_transaction;
mod task_authoring;
mod task_dependency_graph;
mod task_dependency_transaction;
mod task_import;
mod task_node;
mod task_node_action;
mod task_projection;
mod task_promotion_transaction;
mod task_rebaseline;
mod task_rebaseline_transaction;
mod task_recurrence_authoring;
mod task_transaction;
mod task_workspace;
mod temporal;
mod workspace;
mod workspace_revision;
mod workspace_scope;
mod workspace_transaction;
mod workspace_trash;

pub use annotations::{
    ANNOTATION_STORE_VERSION, Anchor, Annotation, AnnotationAction, AnnotationAppearance,
    AnnotationBody, AnnotationBodyFormat, AnnotationColor, AnnotationKind, AnnotationMark,
    AnnotationReanchorOutcome, AnnotationResolution, AnnotationResourceMediaKind,
    AnnotationResourceRegion, AnnotationState, AnnotationStore, AnnotationSuggestionEdit,
    AnnotationTargetIntent, AnnotationValidationError, MAX_ANNOTATION_BODY_BYTES,
    MAX_ANNOTATION_CONTEXT_BYTES, MAX_ANNOTATION_LABEL_BYTES, MAX_ANNOTATION_LABELS,
    MAX_ANNOTATION_STORE_BYTES, MAX_ANNOTATIONS, MAX_MESSAGES_PER_ANNOTATION,
    MAX_SUGGESTED_SOURCE_BYTES, MAX_TOTAL_MESSAGES, ThreadMessage, annotation_suggestion_edit,
    build_annotation_target, reanchor_annotation,
};
pub use checklist::{
    ChecklistAnalysis, ChecklistDiagnostic, ChecklistDiagnosticCode, ChecklistEvidence,
    ChecklistMarker, ChecklistParserOccurrence, ChecklistState, analyze_checklist_source,
};
pub use checklist_action::{
    ChecklistToggleError, ChecklistToggleEvidence, ChecklistToggleSourcePlan,
    ChecklistToggleSummary, plan_checklist_toggle_source,
};
pub use chrono::{CalendarDate, ChronoNodePlan, ChronoPeriod, ChronoPlan};
pub use citation::{
    BibliographyInclusion, BibliographyOccurrence, CitationAttribute, CitationCluster,
    CitationDiagnostic, CitationDiagnosticCode, CitationForm, CitationItem, CitationKeyOccurrence,
    CitationSourceAnalysis, NoCiteOccurrence, analyze_citation_source,
};
pub use citation_authoring::{
    CitationAuthoringAnalysis, CitationAuthoringDiagnosticCode, CitationAuthoringFailure,
    CitationAuthoringPlan, CitationClusterIntent, CitationEditTarget, CitationItemIntent,
    CitationMacroIntent, REFERENCE_RECORD_WRITES_RETIREMENT, analyze_citation_authoring_source,
    plan_citation_macro_edit, reference_record_writes_available,
};
pub use citation_presentation::{
    CITATION_PRESENTER_ID, CITATION_PRESENTER_VERSION, CitationAssetLoadingPolicy,
    CitationComponentPresentation, CitationFontWeight, CitationPresentation,
    CitationPresentationAsset, CitationPresentationCapabilities, CitationPresentationDiagnostic,
    CitationPresentationDiagnosticCode, CitationPresentationFailure, CitationPresentationProfile,
    CitationPresentationRequest, CitationPresenterIsolation, CitationRichText, CitationRichTextRun,
    CitationSecondFieldAlign, CitationTextStyle, CitationVerticalAlign, PresentedBibliography,
    PresentedBibliographyEntry, PresentedCitation, citation_presentation_capabilities,
    present_citations,
};
pub use citation_workspace::{
    BibliographyCompilation, BibliographyComponentInput, BibliographyReferenceInput,
    CitationAccessScope, CitationComponentAnalysis, CitationWorkspaceDiagnostic,
    CitationWorkspaceDiagnosticCode, CitationWorkspaceError, CitationWorkspaceIndex,
    ReferenceDeclaration, ReferenceSearchError, ReferenceSearchField, ReferenceSearchHit,
    ResolvedCitationCluster, ResolvedCitationItem, ResolvedNoCiteOccurrence, ResolvedReference,
};
pub use content_boundary::CONTENT_RULES_FILE_NAME;
pub use document::{
    CommittedDocument, DocumentEdit, DocumentEditPlan, DocumentError, DocumentRevision,
    DocumentSnapshot, ExactSourceDocumentSnapshot, Utf8SourceEdit, Utf8SourcePatchPlan,
    commit_document_edit, plan_document_edit, read_node_document,
};
pub use document_envelope::{DocumentEnvelope, DocumentEnvelopeState, probe_document_envelope};
pub use document_format::{
    ASCIIDOC_V1_MARKER, ASCIIDOC_WORKSPACE_DOCUMENT_FORMAT, CURRENT_WORKSPACE_DOCUMENT_FORMAT,
    UNSUPPORTED_WORKSPACE_DOCUMENT_FORMAT, WORKSPACE_FORMAT_MARKER_FILE, WorkspaceDocumentFormat,
    WorkspaceDocumentGeneration, canonical_document_file_name, canonical_document_file_name_for,
    canonical_document_locator, canonical_document_locator_for, canonical_document_path,
    canonical_document_path_for, is_unmanaged_markdown_path, strip_optional_canonical_extension,
    workspace_document_format,
};
pub use document_formatting::{
    DocumentFormatCommand, DocumentFormatError, DocumentFormatPlan, plan_document_format,
};
pub use document_model::{
    DOCUMENT_CONTRACT_VERSION, DocumentAdjacentHeadingBodyEligibility,
    DocumentAdjacentHeadingBodyPresentation, DocumentAdjacentHeadingBodyResolution,
    DocumentAdjacentHeadingBodyRule, DocumentAnalysis, DocumentAnalysisStatus, DocumentBlock,
    DocumentBlockKind, DocumentBlockSemantic, DocumentCapabilities, DocumentDegradation,
    DocumentDiagnostic, DocumentDiagnosticCode, DocumentEffectCapability, DocumentEffectDecision,
    DocumentEffectEvidence, DocumentEffectOrigin, DocumentFormatAdapter, DocumentInlineKind,
    DocumentInlineSemantic, DocumentLinkKind, DocumentLinkOccurrence, DocumentListItem,
    DocumentListKind, DocumentListModel, DocumentMathNotation, DocumentModel,
    DocumentProfileDescriptor, DocumentProfileId, DocumentSourceOccurrences, DocumentTableCell,
    DocumentTableCellStyle, DocumentTableModel, DocumentTableRow, DocumentViewModel, RunInGroup,
    active_document_adapter, active_document_profile, analyze_document,
    analyze_document_for_profile, analyze_document_with_adapter, document_adapter_for_profile,
    extract_document_occurrences, parse_document, searchable_document_text,
    searchable_document_text_for_profile,
};
pub use document_properties::{
    DocumentProperty, DocumentPropertyAnalysis, DocumentPropertyDiagnostic,
    DocumentPropertyDiagnosticCode, DocumentPropertyKind, DocumentPropertyPatchError,
    analyze_document_header_properties, patch_document_header_property,
};
pub use frontmatter::{
    AdjacentHeadingBody, FrontmatterDiagnostic, FrontmatterDiagnosticCode, FrontmatterError,
    NODE_METADATA_PROJECTION_SCHEMA, NodeMetadata, NodeMetadataProjection, NodeMetadataScope,
    PresentationSettings, parse_node_metadata, parse_node_metadata_with_diagnostics,
    project_node_metadata,
};
pub use icon::{
    BuiltInNodeIcon, NodeIconKind, ResolvedNodeIcon, WorkspaceItemIcon, WorkspaceItemIconFallback,
    built_in_node_icons, derive_workspace_item_icon, patch_node_icon_property,
    read_node_icon_from_source, resolve_node_icon, resolve_node_icon_from_source,
};
pub use identity::{NodeId, NodeIdError};
pub use links::{
    Backlink, InternalLinkKind, LinkIndexError, LinkMatchQuality, NodeLinkEntry, OutgoingLink,
    PotentialMention, WorkspaceLinkIndex, build_workspace_link_index,
};
pub use navigation::{
    NAVIGATION_PROJECTION_VERSION, NavigationContentItem, NavigationNode,
    NavigationProjectionError, WorkspaceNavigationProjection, build_workspace_navigation,
};
pub use node::{
    InventoryIssue, InventoryIssueCode, NodeRecord, WorkspaceContentEntry, WorkspaceContentKind,
    WorkspaceIndex, WorkspaceInventory, scan_workspace,
};
pub use physical_inventory::{
    PHYSICAL_TREE_INVENTORY_SCHEMA, PHYSICAL_TREE_MAX_ENTRIES, PHYSICAL_TREE_MAX_LOCATOR_BYTES,
    PhysicalEntryKind, PhysicalInventoryBinding, PhysicalInventoryEntry, PhysicalInventoryError,
    PhysicalLocator, PhysicalSha256, PhysicalTreeInventory, VerifiedExternalPhysicalTree,
    capture_disjoint_external_physical_tree, capture_stable_physical_tree,
    capture_stable_workspace_physical_inventory, verify_disjoint_external_physical_tree,
};
pub use portable_name::{MAX_PORTABLE_NODE_NAME_BYTES, suggest_portable_node_name};
pub use property::{
    PROPERTY_VALUE_PROFILE_ID, PropertyScalarStyle, PropertyScalarValue, PropertyTypingError,
    classify_property_scalar,
};
pub use query::{
    QUERY_DEFAULT_LIMIT, QUERY_EXPRESSION_CAPABILITY_ID, QUERY_MAX_ALIAS_BYTES,
    QUERY_MAX_BODY_BYTES, QUERY_MAX_CONTEXT_TEXT_BYTES, QUERY_MAX_EVALUATION_STEPS,
    QUERY_MAX_EXPRESSION_NODES, QUERY_MAX_IN_VALUES, QUERY_MAX_LIMIT, QUERY_MAX_NESTING,
    QUERY_MAX_OUTPUT_NAME_BYTES, QUERY_MAX_PROJECTION_FIELDS, QUERY_MAX_RESULT_BYTES,
    QUERY_MAX_SORT_FIELDS, QUERY_MAX_STRING_LITERAL_BYTES, QUERY_MAX_TOKENS, QUERY_PROFILE_ID,
    QueryBlock, QueryBlockContext, QueryComparisonOperator, QueryContextReference, QueryDiagnostic,
    QueryDiagnosticCode, QueryDirection, QueryDocumentContext, QueryExpression,
    QueryExpressionKind, QueryField, QueryFieldReference, QueryGroup, QueryHeadingContext,
    QueryHeadingReference, QueryLexicalContext, QueryLiteral, QueryNodeContext, QueryNullPlacement,
    QueryPlan, QueryProjection, QueryScope, QuerySort, QuerySource, QuerySourceAnalysis,
    QueryValueExpression, QueryValueExpressionKind, QueryValueType, QueryView,
    analyze_query_source,
};
pub use query_workspace::{
    QueryAccessScope, QueryCellValue, QueryColumnIdentity, QueryEvaluationContext,
    QueryEvaluationContextError, QueryExecutionBinding, QueryExecutionError, QueryResult,
    QueryResultCell, QueryResultGroup, QueryResultRow, QueryRowIdentity, QuerySourceExecution,
    QueryWorkspaceError, QueryWorkspaceIndex, query_result_csv,
};
pub use reference::{
    CITATION_DATA_PROFILE_ID, CitationData, ReferenceAnalysis, ReferenceDate, ReferenceDiagnostic,
    ReferenceDiagnosticCode, ReferenceFieldRange, ReferenceName, ReferenceValue,
    analyze_reference_metadata,
};
pub use resource::{
    ImportedResource, ResourceImportError, ResourceImportPlan, commit_import_resource,
    plan_import_resource,
};
pub use search::{
    WorkspaceSearchError, WorkspaceSearchResult, search_workspace, search_workspace_scoped,
};
pub use search_index::{
    SearchIndexError, SearchIndexStats, rebuild_workspace_search_index,
    refresh_workspace_search_index, refresh_workspace_search_index_invalidating,
    search_workspace_index,
};
pub use sorting::{ChildSort, SiblingOrder, SortDirection, SortMode};
pub use sync::{AnnotationReplicaCompleteness, SyncDisposition, classify_sync_state};
pub use task::{
    TASK_METADATA_FIELDS, TASK_PROFILE_ID, TaskAttribute, TaskDateTime, TaskDiagnostic,
    TaskDiagnosticCode, TaskId, TaskIdError, TaskMetadata, TaskOccurrence, TaskPhase, TaskPriority,
    TaskRecurrence, TaskRecurrenceFrequency, TaskRepeatFrom, TaskResolution, TaskSourceAnalysis,
    TaskState, analyze_task_source,
};
pub use task_action_transaction::{
    ChecklistToggleTransactionPlan, CommittedChecklistToggle, CommittedTaskNodeEdit,
    TaskActionTransactionError, TaskNodeEditTransactionPlan, commit_checklist_toggle_transaction,
    commit_checklist_toggle_transaction_scoped, commit_task_node_edit_transaction,
    commit_task_node_edit_transaction_scoped, plan_checklist_toggle_transaction,
    plan_checklist_toggle_transaction_scoped, plan_task_node_edit_transaction,
    plan_task_node_edit_transaction_scoped,
};
pub use task_authoring::{
    TaskAuthoringDiagnosticCode, TaskAuthoringFailure, TaskAuthoringPlan, TaskDateField,
    TaskEditIntent, TaskEditTarget, plan_task_edit,
};
pub use task_dependency_transaction::{
    CommittedTaskNodeDependencyReplacement, TaskNodeDependencyReplacementDiagnostic,
    TaskNodeDependencyReplacementDiagnosticCode, TaskNodeDependencyReplacementError,
    TaskNodeDependencyReplacementPlan, TaskNodeDependencyReplacementRequest,
    TaskNodeDependencyReplacementSummary, commit_task_node_dependency_replacement_transaction,
    commit_task_node_dependency_replacement_transaction_scoped,
    commit_task_node_dependency_replacement_transaction_scoped_with_draft_gate,
    commit_task_node_dependency_replacement_transaction_with_draft_gate,
    plan_task_node_dependency_replacement_transaction,
    plan_task_node_dependency_replacement_transaction_scoped,
};
pub use task_import::{
    TASK_IMPORT_PROFILE_ID, TaskImportDiagnostic, TaskImportDiagnosticCode, TaskImportDialect,
    TaskImportDocumentInput, TaskImportDocumentPlan, TaskImportEdit, TaskImportEditKind,
    TaskImportIdentityMapping, TaskImportPlan, TaskImportSettings, TaskImportSettingsError,
    TaskImportStatusMapping, TaskImportStatusType, plan_task_import, validate_task_import_plan,
};
pub use task_node::{
    TASK_NODE_PROFILE_MARKER, TASK_NODE_PROFILE_VERSION, TaskNodeAttributeEvidence,
    TaskNodeAttributeForm, TaskNodeAttributeKind, TaskNodeDiagnostic, TaskNodeDiagnosticCode,
    TaskNodePriority, TaskNodeProfile, TaskNodeProfileAnalysis, TaskNodeProfileVersion,
    TaskNodeState, TaskNodeTitleEvidence, analyze_task_node_profile,
};
pub use task_node_action::{
    TaskNodeActionEvidence, TaskNodeClosedEdit, TaskNodeEditError, TaskNodeEditIntent,
    TaskNodeEditRequest, TaskNodeEditSummary, TaskNodeSourceEditPlan, TaskNodeTemporalField,
    plan_task_node_source_edit,
};
pub use task_projection::{
    TaskRow, TaskRowEvidence, TaskRowKind, TaskWorkspaceProjection,
    TaskWorkspaceProjectionDiagnostic, TaskWorkspaceProjectionDiagnosticCode,
    TaskWorkspaceProjectionError,
};
pub use task_promotion_transaction::{
    ChecklistPromotionEvidence, CommittedTaskPromotion, TaskPromotionAnnotationBlocker,
    TaskPromotionAnnotationBlockerReason, TaskPromotionAnnotationDisposition,
    TaskPromotionAnnotationDispositionRecord, TaskPromotionAnnotationSummary, TaskPromotionError,
    TaskPromotionPlan, TaskPromotionRequest, TaskPromotionSummary,
    commit_task_promotion_transaction, commit_task_promotion_transaction_scoped,
    commit_task_promotion_transaction_scoped_with_draft_gate,
    commit_task_promotion_transaction_with_draft_gate, plan_task_promotion_transaction,
    plan_task_promotion_transaction_scoped,
};
#[cfg(debug_assertions)]
pub use task_promotion_transaction::{
    commit_task_promotion_with_injected_failure_for_recovery_fixture,
    commit_task_promotion_with_injected_verification_failure_for_recovery_fixture,
    prepare_task_promotion_applying_recovery_fixture,
    prepare_task_promotion_committed_recovery_fixture, prepare_task_promotion_recovery_fixture,
};
pub use task_rebaseline::{
    LocalTaskRebaselineAuthority, TASK_REBASELINE_MAX_BLOCKERS, TASK_REBASELINE_MAX_DOCUMENT_BYTES,
    TASK_REBASELINE_MAX_DOCUMENTS, TASK_REBASELINE_MAX_OCCURRENCES,
    TASK_REBASELINE_MAX_PLAN_JSON_BYTES, TASK_REBASELINE_MAX_QUERIES,
    TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES, TASK_REBASELINE_MAX_TOTAL_PREVIEW_BYTES,
    TASK_REBASELINE_MAX_TOTAL_SOURCE_BYTES, TASK_REBASELINE_SCHEMA,
    TaskRebaselineAnnotationInventory, TaskRebaselineBlocker, TaskRebaselineBlockerCode,
    TaskRebaselineError, TaskRebaselineExternalSnapshotBinding, TaskRebaselineIdentityMapping,
    TaskRebaselineOccurrenceDisposition, TaskRebaselineOccurrenceInventory,
    TaskRebaselinePhysicalInventoryBinding, TaskRebaselinePlan,
    TaskRebaselinePortableInventoryBinding, TaskRebaselinePreStateBinding, TaskRebaselineProposal,
    TaskRebaselineQueryDisposition, TaskRebaselineQueryInventory, TaskRebaselineScope,
    TaskRebaselineSourceKind, TaskRebaselineSourcePreview, TaskRebaselineTaskFields,
    capture_local_task_rebaseline_authority, decode_task_rebaseline_plan_json,
    plan_task_rebaseline, revalidate_task_rebaseline_plan, validate_task_rebaseline_plan,
};
pub use task_recurrence_authoring::{
    TaskRecurrenceCompletionContext, TaskRecurrenceCompletionDiagnosticCode,
    TaskRecurrenceCompletionFailure, TaskRecurrenceCompletionPlan, plan_task_recurrence_completion,
};
pub use task_transaction::{
    TaskDependencyTransactionPlan, TaskEditTransactionPlan, TaskRecurrenceTransactionPlan,
    TaskTransactionError, commit_task_dependency_transaction, commit_task_edit_transaction,
    commit_task_recurrence_transaction, plan_task_dependency_transaction,
    plan_task_dependency_transaction_scoped, plan_task_edit_transaction,
    plan_task_edit_transaction_scoped, plan_task_recurrence_transaction,
    plan_task_recurrence_transaction_scoped,
};
pub use task_workspace::{
    TaskWorkspaceDiagnostic, TaskWorkspaceDiagnosticCode, TaskWorkspaceError, TaskWorkspaceIndex,
    TaskWorkspaceOccurrence,
};
pub use temporal::{TaskNodeTemporal, TaskNodeTemporalError};
pub use weftext_asciidoc::{PROFILE_ID as MANAGED_DOCUMENT_PROFILE_ID, SourceEdit};
pub use workspace::{CreatedNode, WorkspaceError, create_child_node, create_workspace};
pub use workspace_revision::{WorkspaceRevision, WorkspaceRevisionError, read_workspace_revision};
pub use workspace_scope::{
    WorkspaceNodeProjection, WorkspaceReadScope, WorkspaceScopeError, WorkspaceScopeInventoryError,
};
#[cfg(debug_assertions)]
#[doc(hidden)]
pub use workspace_transaction::prepare_workspace_transaction_recovery_fixture;
pub use workspace_transaction::{
    AnnotationSidecarExpectedState, AnnotationSidecarPlanAuthority, AnnotationSidecarSnapshot,
    CommittedWorkspaceTransaction, RecoveryReport, StructuralAction, TRASH_NODE_NAME,
    WORKSPACE_TRANSACTION_LEASE_FILE_NAME, WorkspaceCapturedTarget, WorkspaceDocumentChange,
    WorkspaceDraftGatePreview, WorkspaceDraftGateToken, WorkspaceDraftRegistryView,
    WorkspaceIdentityMapEntry, WorkspaceIdentityPolicy, WorkspaceImportAuthority,
    WorkspaceImportNode, WorkspaceImportResource, WorkspaceImportTransactionState,
    WorkspacePathChange, WorkspaceRestoreAnnotationSidecar, WorkspaceRestoreTreeNode,
    WorkspaceScopeRootNode, WorkspaceTargetResolution, WorkspaceTransactionError,
    WorkspaceTransactionLease, WorkspaceTransactionPlan, WorkspaceTransactionReceiptHandoff,
    WorkspaceTransactionScopeSummary, acquire_workspace_transaction_lease,
    bind_workspace_transaction_target_resolution, capture_annotation_sidecar_snapshot,
    commit_workspace_transaction, commit_workspace_transaction_retaining_journal,
    commit_workspace_transaction_retaining_journal_with_draft_gate,
    commit_workspace_transaction_with_draft_gate, confirm_permanent_delete_trash_items,
    finalize_committed_workspace_transaction, has_unfinished_workspace_transaction,
    inspect_workspace_import_transaction, load_legacy_trash_migration_backup,
    plan_adjacent_heading_body_setting, plan_annotation_action, plan_chrono_nodes, plan_copy_node,
    plan_create_child_node, plan_import_node, plan_import_tree,
    plan_migrate_legacy_workspace_trash, plan_migrate_legacy_workspace_trash_at,
    plan_migrate_legacy_workspace_trash_at_with_backup,
    plan_migrate_legacy_workspace_trash_with_backup, plan_move_node, plan_node_aliases_setting,
    plan_node_child_sort_setting, plan_node_icon_setting, plan_node_sibling_rank_setting,
    plan_permanently_delete_trash_items, plan_rename_node, plan_restore_node,
    plan_restore_snapshot_tree, plan_restore_trash_item, plan_trash_node, plan_trash_node_at,
    plan_trash_resource, plan_trash_resources, plan_trash_resources_at,
    prepare_legacy_trash_migration_backup, preview_permanent_delete_trash_items,
    preview_workspace_transaction_draft_gate, project_workspace_trash_items,
    project_workspace_trash_state, publish_committed_workspace_transaction_receipt,
    read_committed_workspace_transaction_receipt_handoff, read_node_annotations,
    read_node_annotations_at_node_path, recover_workspace_import_transaction,
    recover_workspace_transaction_for_plan, recover_workspace_transactions,
    recover_workspace_transactions_retaining_committed, replan_reviewed_trash_request,
};
pub use workspace_trash::{
    LEGACY_TRASH_MIGRATION_BACKUP_SCHEMA, LegacyTrashMigrationBackup,
    LegacyTrashMigrationBackupAuthority, TRASH_DIRECTORY_NAME, TRASH_ITEM_MANIFEST_FILE_NAME,
    TRASH_ITEM_PAYLOAD_DIRECTORY_NAME, TRASH_ITEM_SCHEMA, TRASH_ITEMS_DIRECTORY_NAME,
    TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE, TRASH_REVIEWED_REQUEST_SCHEMA, TrashIdError,
    TrashItemId, TrashItemKind, TrashItemManifest, TrashItemRestoreAvailability, TrashOperationId,
    TrashOriginResolution, TrashOriginStatus, TrashPermanentDeleteConfirmation,
    TrashPermanentDeleteItemPreview, TrashPermanentDeletePreview, TrashPlanDisposition,
    TrashResourceSelection, TrashRestoreBlockedReason, TrashRestoreMode, TrashReviewId,
    TrashReviewedAction, TrashReviewedReplanAuthorization, TrashReviewedRequest,
    WorkspaceTrashItem, WorkspaceTrashItemProjection, WorkspaceTrashPlanItemChange,
    WorkspaceTrashState, WorkspaceTrashStateProjection,
};

/// Fixed annotation sidecar name for every node.
pub const ANNOTATIONS_FILE_NAME: &str = "weftext.annotations.json";
