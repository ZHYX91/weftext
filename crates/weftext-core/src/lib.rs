//! Local MVP persistence; this is deliberately not the full Core sync/schema protocol.
pub(crate) mod author_order;
mod backend;
pub use author_order::{
    NavigationOrderMember, NavigationOrderSnapshot, navigation_orders_changed,
    validate_workspace_settings,
};
mod capture_draft;
mod compile_dependencies;
pub use capture_draft::{
    CaptureAuthorInput, CaptureAuthorRawInput, CaptureAuthorRecovery, CaptureAuthorTexts,
    CaptureBodyMode, CaptureCreationRequest, CaptureDetachedAuthorInput, CaptureDetachedRawInput,
    CaptureDraft, CaptureFieldInput, CaptureFieldValueState, CaptureFlushReceipt, CaptureFormBasis,
    CaptureInput, CaptureInputCut, CaptureMutation, CaptureMutationCommand, CaptureMutationKind,
    CaptureMutationLookup, CaptureMutationTarget, CaptureParent, CapturePendingIdentity,
    CaptureRawAuthor, CaptureRawBinding, CaptureRawInput, CaptureRawProvenance,
    CaptureRawRetirement, CaptureResolvedTarget, CaptureStorageStatus, CaptureTarget, CaptureText,
    FrozenCaptureRequest, MAX_CAPTURE_DOMAIN_BYTES, MAX_CAPTURE_RECORD_BYTES, MAX_CAPTURE_RECORDS,
};
pub use compile_dependencies::{
    CompileDependency, CompileDependencyKind, CompileDependencyStatus, CompileDependencyWitness,
};
mod device_state;
mod directory_identity;
pub use device_state::DeviceIdentity;
mod profile_identity;
pub use profile_identity::{QualifiedProfile, qualify_existing_profile, qualify_profile};
mod client_preferences;
pub use client_preferences::{ClientPreferences, ClientPreferencesSnapshot};
mod desktop_preferences;
pub use desktop_preferences::{DesktopPreferences, DesktopPreferencesSnapshot};
#[cfg(all(test, windows))]
mod compile_input_tests;
#[cfg(all(test, windows))]
mod device_state_tests;
mod draft_recovery;
mod exchange;
mod export_read;
mod file_save_scope;
use file_save_scope::{FileSaveData, QualifiedInputScope};
mod history;
mod hosted;
mod hosted_destination;
pub use hosted_destination::{HostedDestination, HostedUniqueChild, HostedUniqueMember};
mod recognition;
pub mod reminder_device_state;
mod reminder_schedule;
#[cfg(all(test, windows))]
mod tag_write_guard_tests;
pub use hosted::{
    ActorContentImportReceipt, ActorDefinitionProposal, ActorDefinitionStatus, ActorFeatureStatus,
    ActorFieldStatus, ActorHistoryRestoreStatus, ActorOwnerBackupStatus, ActorSaveStatus,
    ActorStructureNode, ActorStructureReceipt, ContentImportMember, FeatureTreeWitness,
    HostedDefinitionReservationRequest, HostedNavigationIntent, HostedSaveRequest,
    OwnerBackupArtifact,
};
#[cfg(all(test, windows))]
mod history_tests;
mod inventory;
mod journal;
mod library_creation;
mod local_operation;
mod original_delivery;
pub use library_creation::{
    InitialLibraryStyle, InitialRootNote, LibraryCreateOptions, root_query_presets,
};
mod operations;
mod query_scope;
mod resource;
mod resource_collection;
mod resource_diagnostics;
pub use resource::{
    MissingResourceWitness, QualifiedResource, QualifiedResourceInput, ResourceWitness,
};
pub use resource_collection::{
    QualifiedResourceCollection, ResourceCollectionEntry, ResourceCollectionWitness,
};
#[cfg(all(test, windows))]
mod library_resource_tests;
#[cfg(all(test, windows))]
mod query_scope_tests;
pub use query_scope::{
    QualifiedQueryNode, QualifiedQueryScope, QueryNodeWitness, QueryScopeWitness,
};
pub mod sync;
mod transaction_recovery;
pub use directory_identity::{
    NavigationPreparation, NavigationTarget, TagAssignment, TagRecord, TagRegistrySnapshot,
};
pub use draft_recovery::{
    DocumentDraftFlush, DocumentDraftReceipt, DraftAlternative, ResumeDraftRequest,
};
pub use exchange::{NativeExport, NativeExportWitness};
pub use history::{
    HistoryComparison, HistoryRead, HistoryRestoreRequest, HistoryRestoreResult,
    HistoryRestoreStatus, HistoryRetention, HistoryTimeline, HistoryVersion,
};
#[cfg(test)]
mod error_contract_tests;
#[cfg(all(test, windows))]
mod recovery_tests;

use backend::{FileIdentity, FileStamp, ParentGuard, stamp};
pub use inventory::validate_library_rules;
pub use inventory::{Inventory, InventoryEntry};
pub use journal::RecoveryInfo;
pub use operations::{
    AdmittedCompileSnapshot, AdmittedSnapshot, ConfigDocument, ConfigRole, FileChange,
    LibraryStyleDocument, LibraryStyleWitness, ManagedResource, OperationChange, OperationIntent,
    OperationPreview, OperationReceipt, OperationRequest, QualifiedNode, SavedOperationSource,
    SnapshotFile, SnapshotSet, SnapshotWitness, TrashEntry,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
pub use weftext_messages::{
    DiagnosticOrigin, ErrorCauses, ErrorDescription, RawDiagnostic, UiMessage,
};

pub type Result<T> = std::result::Result<T, CoreError>;
pub const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
pub const STORAGE_PROFILE: &str = "weftext.local-mvp.v1";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreError {
    pub code: String,
    pub explanation: UiMessage,
    pub diagnostics: Vec<RawDiagnostic>,
    pub causes: ErrorCauses,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<CoreErrorDetails>,
    #[serde(skip)]
    os_error: Option<i32>,
}
/// Independent problems stay independent of the bounded causal chain.
/// All groups are produced from already bounded Core/data inputs, never decoded as requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CoreErrorDetails {
    Aggregate { groups: Vec<ErrorDetailGroup> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ErrorDetailGroup {
    SemanticEvidence {
        gaps: Vec<sync::UnfulfilledEvidence>,
    },
    Data {
        classification: weftext_data::DataErrorKind,
        diagnostics: Vec<ErrorDescription>,
    },
    Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        diagnostics: Vec<ErrorDescription>,
    },
}
/// A computed product status or warning; recovery source remains in its original error channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoreDescription {
    pub explanation: UiMessage,
    pub diagnostics: Vec<RawDiagnostic>,
    pub causes: ErrorCauses,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<CoreErrorDetails>,
}
impl CoreDescription {
    pub fn new(explanation: UiMessage) -> Self {
        Self::from(ErrorDescription::new(explanation))
    }
    pub fn into_parts(self) -> (ErrorDescription, Option<CoreErrorDetails>) {
        (
            ErrorDescription {
                explanation: self.explanation,
                diagnostics: self.diagnostics,
                causes: self.causes,
            },
            self.details,
        )
    }
    pub fn into_error(self, code: &str) -> CoreError {
        let (description, details) = self.into_parts();
        let mut error = CoreError::new(code, description);
        error.details = details;
        error
    }
    pub fn diagnostic(mut self, origin: DiagnosticOrigin, text: impl Into<String>) -> Self {
        self.diagnostics.push(RawDiagnostic {
            origin,
            text: text.into(),
        });
        self
    }
}
impl From<UiMessage> for CoreDescription {
    fn from(explanation: UiMessage) -> Self {
        Self::new(explanation)
    }
}
impl From<ErrorDescription> for CoreDescription {
    fn from(description: ErrorDescription) -> Self {
        Self {
            explanation: description.explanation,
            diagnostics: description.diagnostics,
            causes: description.causes,
            details: None,
        }
    }
}
impl From<CoreError> for CoreDescription {
    fn from(error: CoreError) -> Self {
        let mut groups = match error.details {
            Some(CoreErrorDetails::Aggregate { groups }) => groups,
            None => Vec::new(),
        };
        groups.push(ErrorDetailGroup::Product {
            code: Some(error.code),
            diagnostics: Vec::new(),
        });
        Self {
            explanation: error.explanation,
            diagnostics: error.diagnostics,
            causes: error.causes,
            details: Some(CoreErrorDetails::Aggregate { groups }),
        }
    }
}
impl CoreError {
    pub fn new(code: &str, description: impl Into<ErrorDescription>) -> Self {
        let description = description.into();
        Self {
            code: code.into(),
            explanation: description.explanation,
            diagnostics: description.diagnostics,
            causes: description.causes,
            current_source: None,
            details: None,
            os_error: None,
        }
    }
    pub fn diagnostic(mut self, origin: DiagnosticOrigin, text: impl Into<String>) -> Self {
        self.diagnostics.push(RawDiagnostic {
            origin,
            text: text.into(),
        });
        self
    }
    fn into_description(self) -> ErrorDescription {
        ErrorDescription {
            explanation: self.explanation,
            diagnostics: self.diagnostics,
            causes: self.causes,
        }
    }
    pub fn from_data(code: &str, error: weftext_data::DataError) -> Self {
        let (classification, description, details) = error.into_parts();
        let mut error = Self::new(code, description);
        error.details = Some(CoreErrorDetails::Aggregate {
            groups: vec![ErrorDetailGroup::Data {
                classification,
                diagnostics: details
                    .map(weftext_data::DataErrorDetails::into_diagnostics)
                    .unwrap_or_default(),
            }],
        });
        error
    }
    pub fn from_export(code: &str, error: weftext_export::ExportError) -> Self {
        let (description, diagnostics) = error.into_parts();
        Self::from_product(code, None, description, diagnostics)
    }
    pub fn from_engine(code: &str, error: weftext_engine::EngineError) -> Self {
        let (description, diagnostics) = error.into_parts();
        Self::from_product(code, None, description, diagnostics)
    }
    pub fn from_native(error: weftext_export::exchange::NativeError) -> Self {
        let code = error.code.clone();
        let (description, diagnostics) = error.into_parts();
        Self::from_product(&code, None, description, diagnostics)
    }
    pub fn from_knowledge(code: &str, error: weftext_knowledge::KnowledgeError) -> Self {
        let source_code = error.code.clone();
        let (description, diagnostics) = error.into_parts();
        Self::from_product(code, Some(source_code), description, diagnostics)
    }
    pub fn from_product(
        code: &str,
        source_code: Option<String>,
        description: ErrorDescription,
        diagnostics: Vec<ErrorDescription>,
    ) -> Self {
        let mut error = Self::new(code, description);
        if source_code.is_some() || !diagnostics.is_empty() {
            error.details = Some(CoreErrorDetails::Aggregate {
                groups: vec![ErrorDetailGroup::Product {
                    code: source_code,
                    diagnostics,
                }],
            });
        }
        error
    }
    pub fn from_descriptions(
        code: &str,
        explanation: UiMessage,
        descriptions: Vec<CoreDescription>,
    ) -> Self {
        let mut error = Self::new(code, explanation);
        let mut diagnostics = Vec::with_capacity(descriptions.len());
        for description in descriptions {
            let (description, details) = description.into_parts();
            diagnostics.push(description);
            if let Some(CoreErrorDetails::Aggregate { groups }) = details {
                for group in groups {
                    error.add_detail_group(group);
                }
            }
        }
        if !diagnostics.is_empty() {
            error.add_detail_group(ErrorDetailGroup::Product {
                code: None,
                diagnostics,
            });
        }
        error
    }
    pub fn add_detail_group(&mut self, group: ErrorDetailGroup) {
        match &mut self.details {
            Some(CoreErrorDetails::Aggregate { groups }) => groups.push(group),
            None => {
                self.details = Some(CoreErrorDetails::Aggregate {
                    groups: vec![group],
                })
            }
        }
    }
    pub fn caused_by(mut self, mut cause: CoreError) -> Self {
        let code = cause.code.clone();
        if self.current_source.is_none() {
            self.current_source = cause.current_source.take();
        }
        let mut description = ErrorDescription::new(self.explanation.clone());
        description.causes = std::mem::take(&mut self.causes);
        if let Some(CoreErrorDetails::Aggregate { groups }) = cause.details.take() {
            for group in groups {
                self.add_detail_group(group);
            }
        }
        match description.try_caused_by(code, cause.into_description()) {
            Ok(description) => self.causes = description.causes,
            Err(boundary) => {
                if !boundary.outer.causes.as_slice().is_empty() {
                    self.add_detail_group(ErrorDetailGroup::Product {
                        code: Some(self.code.clone()),
                        diagnostics: vec![boundary.outer],
                    });
                }
                self.causes =
                    ErrorCauses::one(boundary.code.clone(), boundary.original.explanation.clone());
                self.add_detail_group(ErrorDetailGroup::Product {
                    code: Some(boundary.code),
                    diagnostics: vec![boundary.original],
                });
            }
        }
        self
    }
    pub(crate) fn os_error(&self) -> Option<i32> {
        self.os_error
    }
    fn conflict(explanation: UiMessage, source: &[u8]) -> Self {
        let mut error = Self::new("conflict", explanation);
        error.current_source = String::from_utf8(source.to_vec()).ok();
        error
    }
}
impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.code,
            self.explanation
                .render(weftext_messages::UiLocale::default())
        )?;
        for diagnostic in &self.diagnostics {
            write!(f, "\n{:?}: {}", diagnostic.origin, diagnostic.text)?;
        }
        Ok(())
    }
}
impl std::error::Error for CoreError {}
impl From<std::io::Error> for CoreError {
    fn from(value: std::io::Error) -> Self {
        if let Some(original) = value
            .get_ref()
            .and_then(|error| error.downcast_ref::<CoreError>())
        {
            return original.clone();
        }
        let code = match value.kind() {
            std::io::ErrorKind::AlreadyExists => "exists",
            std::io::ErrorKind::NotFound => "missing",
            std::io::ErrorKind::PermissionDenied => "permissionDenied",
            _ => "io",
        };
        if let Some(original) = value
            .get_ref()
            .and_then(|error| error.downcast_ref::<ErrorDescription>())
        {
            return Self::new(code, original.clone());
        }
        let os_error = value.raw_os_error();
        let mut error = Self::new(code, UiMessage::error_core_io_failure())
            .diagnostic(DiagnosticOrigin::Os, value.to_string());
        error.os_error = os_error;
        error
    }
}
impl From<serde_json::Error> for CoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::new("invalidState", UiMessage::error_core_json_failure())
            .diagnostic(DiagnosticOrigin::Json, value.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocument {
    pub session_id: String,
    pub path: String,
    pub root: String,
    pub source: String,
    pub base_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovered_draft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovered_draft_base_source: Option<String>,
    pub recovered_draft_conflict: bool,
    pub recovered_draft_variants: Vec<DraftAlternative>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovered_draft_witness: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveRequest {
    pub session_id: String,
    pub base_id: String,
    pub source: String,
    pub operation_id: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveAsRequest {
    pub session_id: String,
    pub path: String,
    pub source: String,
    pub operation_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<String>,
    pub operation_id: String,
    pub status: String,
    pub path: String,
    pub base_id: String,
}
/// The original journal stores only Receipt. Recovery delivery is transient.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOutcome {
    #[serde(flatten)]
    pub receipt: Receipt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_issue: Option<RecoveryIssue>,
}
pub type RecoveryIssue = CoreError;
impl SaveOutcome {
    fn committed(receipt: Receipt) -> Self {
        Self {
            receipt,
            recovery_issue: None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub operation_id: String,
    pub root: String,
    pub workspace_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_note_path: Option<String>,
}

#[derive(Clone)]
struct Session {
    root: PathBuf,
    actor: Option<String>,
    path: PathBuf,
    base_id: String,
    bytes: Vec<u8>,
    stamp: FileStamp,
    file_scope: Result<QualifiedInputScope>,
    node_id: Option<String>,
    conflicting_draft: Option<Draft>,
    draft_placement_pending: bool,
    // Generated by Core for this input producer, never selected by an HTTP
    // caller. A reopened document starts a new producer; it cannot silently
    // coalesce a different surface's already acknowledged proposal.
    draft_producer_id: String,
    draft_witness: Option<String>,
    draft_input: Option<DraftAlternative>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Draft {
    scope: QualifiedInputScope,
    identity: FileIdentity,
    path: String,
    base_source: String,
    source: String,
    #[serde(default = "known_base")]
    base_known: bool,
    #[serde(default)]
    alternatives: Vec<DraftAlternative>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    producer_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceConfig {
    version: u32,
    workspace_id: String,
    storage_profile: String,
    schema_support: String,
    sync_support: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Birth {
    node_id: String,
    workspace_id: String,
    identity: FileIdentity,
}

pub struct Core {
    actor_library: Option<hosted::ActorLibraryBinding>,
    actor_library_path: Option<PathBuf>,
    actor: Option<String>,
    _instance_lock: File,
    _data_guard: ParentGuard,
    _private_guards: Vec<ParentGuard>,
    data_dir: PathBuf,
    sessions: HashMap<String, Session>,
    operations: HashMap<String, journal::Plan>,
    snapshots: HashMap<String, operations::SnapshotBinding>,
    style_bases: HashMap<String, operations::StyleBasis>,
    admitted_cuts: HashMap<String, operations::AdmittedBasis>,
    compile_dependency_cuts: HashMap<String, compile_dependencies::CompileDependencyCut>,
}
impl Core {
    pub fn check_session_base(&self, session_id: &str, base_id: &str) -> Result<()> {
        self.check_actor_session(session_id)?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_source()))?;
        if session.base_id != base_id || session.conflicting_draft.is_some() {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_opened_source_basis_unqualified(),
            ));
        }
        self.check_pending(&session.path)?;
        let guard = ParentGuard::acquire(&session.path)?;
        let mut file = guard.open_file(false, false)?;
        if stamp(&file)? != session.stamp || read_source(&mut file)? != session.bytes {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_source_object_changed_after_open(),
            ));
        }
        Ok(())
    }
    pub fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = absolute_path(data_dir.as_ref())?;
        fs::create_dir_all(&data_dir)?;
        let data_guard = ParentGuard::acquire(&data_dir.join("instance.lock"))?;
        let instance_lock = instance_lock(&data_dir)?;
        fs::create_dir_all(data_dir.join("journal"))?;
        fs::create_dir_all(data_dir.join("drafts"))?;
        fs::create_dir_all(data_dir.join("draft-history"))?;
        fs::create_dir_all(data_dir.join(device_state::DEVICE_STATE_ROOT))?;
        fs::create_dir_all(data_dir.join(capture_draft::CAPTURE_DIRECTORY))?;
        fs::create_dir_all(data_dir.join(capture_draft::CAPTURE_STAGING_DIRECTORY))?;
        fs::create_dir_all(
            data_dir
                .join(capture_draft::CAPTURE_STAGING_DIRECTORY)
                .join("local"),
        )?;
        let private_guards = [
            "journal",
            "drafts",
            "draft-history",
            device_state::DEVICE_STATE_ROOT,
            capture_draft::CAPTURE_DIRECTORY,
            capture_draft::CAPTURE_STAGING_DIRECTORY,
            "capture-staging/local",
        ]
        .into_iter()
        .map(|name| ParentGuard::acquire(&data_dir.join(name).join(".private-prospective")))
        .collect::<Result<Vec<_>>>()?;
        let mut core = Self {
            actor_library: None,
            actor_library_path: None,
            actor: None,
            _instance_lock: instance_lock,
            _data_guard: data_guard,
            _private_guards: private_guards,
            data_dir,
            sessions: HashMap::new(),
            operations: HashMap::new(),
            snapshots: HashMap::new(),
            style_bases: HashMap::new(),
            admitted_cuts: HashMap::new(),
            compile_dependency_cuts: HashMap::new(),
        };
        core.load_journals()?;
        Ok(core)
    }
    pub fn open(
        &mut self,
        path: impl AsRef<Path>,
        root: Option<PathBuf>,
    ) -> Result<OpenedDocument> {
        let path = absolute_path(path.as_ref())?;
        check_typ(&path)?;
        self.check_pending(&path)?;
        let mut root =
            absolute_path(&root.unwrap_or_else(|| path.parent().unwrap().to_path_buf()))?;
        if !path.starts_with(&root) {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_source_outside_selected_root(),
            ));
        }
        self.actor_library_for_root(&root)?;
        let guard = ParentGuard::acquire(&path)?;
        let mut file = guard.open_file(false, false)?;
        let bytes = read_source(&mut file)?;
        let observed = stamp(&file)?;
        self.check_pending_identity(&observed.identity)?;
        let bound = self.admitted_binding(&observed.identity);
        if let Some(birth) = &bound
            && let Some(library) = self.operations.values().find(|p| {
                p.kind == "createLibrary"
                    && p.workspace_id.as_deref() == Some(&birth.workspace_id)
                    && self.committed(&p.operation_id)
            })
        {
            let library_root = PathBuf::from(&library.path);
            if path.starts_with(&library_root) {
                root = library_root;
            }
        }
        self.actor_library_for_root(&root)?;
        let source = decode(&bytes)?;
        let info = weftext_engine::inspect(&source);
        let workspace = self.workspace(&root)?;
        let candidate = is_canonical(&path);
        let admitted = bound.as_ref().map(|b| b.node_id.clone()).or_else(|| {
            info.id.as_ref().and_then(|id| {
                workspace.as_ref().and_then(|w| {
                    self.admitted(id, &w.workspace_id, &observed.identity)
                        .then_some(id.clone())
                })
            })
        });
        let kind = if admitted.is_some()
            && path.strip_prefix(&root).ok().is_some_and(|p| {
                p.components().next().is_some_and(|c| {
                    c.as_os_str()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(".weftext-trash")
                })
            }) {
            "managedTrash"
        } else if admitted.is_some() && (!candidate || info.id != admitted) {
            "managedInvalid"
        } else if candidate && admitted.is_some() {
            "managed"
        } else {
            "ordinary"
        };
        let session_id = fresh();
        let base_id = fresh();
        let file_scope = self.capture_file_save_scope(&path);
        let (draft, draft_placement_pending) = if file_scope.is_ok() {
            self.read_relocated_draft_with_identity(&root, &path, &observed.identity, &source)?
        } else {
            (None, true)
        };
        let draft_witness = draft.as_ref().map(Draft::witness).transpose()?;
        let draft = draft
            .filter(|d| d.source != source || !d.alternatives.is_empty())
            .filter(|d| {
                !(self.actor.is_some()
                    && d.base_known
                    && d.source == d.base_source
                    && d.alternatives.is_empty())
            });
        let recovered_draft_conflict = draft.as_ref().is_some_and(|d| {
            !d.base_known || d.base_source != source || !d.alternatives.is_empty()
        });
        let recovered_draft = draft.as_ref().map(|d| d.source.clone());
        let recovered_draft_base_source = draft.as_ref().map(|d| d.base_source.clone());
        let recovered_draft_variants = draft
            .as_ref()
            .map(|d| d.alternatives.clone())
            .unwrap_or_default();
        let recovered_draft_witness = draft.as_ref().map(Draft::witness).transpose()?;
        self.sessions.insert(
            session_id.clone(),
            Session {
                root: root.clone(),
                actor: self.actor.clone(),
                path: path.clone(),
                base_id: base_id.clone(),
                bytes,
                stamp: observed,
                file_scope,
                node_id: admitted.clone(),
                conflicting_draft: if recovered_draft_conflict {
                    draft
                } else {
                    None
                },
                draft_placement_pending,
                draft_producer_id: fresh(),
                draft_witness,
                draft_input: None,
            },
        );
        Ok(OpenedDocument {
            session_id,
            path: display(&path),
            root: display(&root),
            source,
            base_id,
            kind: kind.into(),
            node_id: admitted,
            workspace_id: workspace.map(|w| w.workspace_id),
            recovered_draft,
            recovered_draft_base_source,
            recovered_draft_conflict,
            recovered_draft_variants,
            recovered_draft_witness,
        })
    }
    pub fn persist_draft(&mut self, session_id: &str, source: &str) -> Result<()> {
        self.check_actor_session(session_id)?;
        check_size(source.as_bytes())?;
        let session = self.sessions.get(session_id).cloned().ok_or_else(|| {
            CoreError::new("sessionMissing", UiMessage::error_reopen_file_session())
        })?;
        let draft = Draft {
            scope: self.draft_scope_for_session(session_id, session.conflicting_draft.as_ref())?,
            path: display(&session.path),
            base_source: session
                .conflicting_draft
                .as_ref()
                .map_or_else(|| decode(&session.bytes), |d| Ok(d.base_source.clone()))?,
            source: source.into(),
            base_known: session
                .conflicting_draft
                .as_ref()
                .is_none_or(|d| d.base_known),
            identity: session.stamp.identity.clone(),
            alternatives: Vec::new(),
            producer_id: Some(session.draft_producer_id.clone()),
        };
        self.store_session_draft(session_id, draft).map(|_| ())
    }
    pub fn resolve_draft(
        &mut self,
        session_id: &str,
        action: &str,
        source: Option<&str>,
    ) -> Result<()> {
        self.check_actor_session(session_id)?;
        if !matches!(action, "discardRecovered" | "mergeRecovered") {
            return Err(CoreError::new(
                "invalidDraftAction",
                UiMessage::error_invalid_draft_resolution_action(),
            ));
        }
        let session = self
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_file()))?;
        if action == "mergeRecovered" && source.is_none() {
            return Err(CoreError::new(
                "mergeRequired",
                UiMessage::error_reviewed_merge_required(),
            ));
        }
        let latest_record = self.current_session_draft_record(session_id)?;
        let latest = latest_record.as_ref().map(|(draft, _)| draft);
        if latest.map(Draft::witness).transpose()? != session.draft_witness {
            return Err(CoreError::new(
                "draftChanged",
                "Another proposal arrived after this recovery view; reopen and compare the complete current record before resolving it",
            ));
        }
        let source = if action == "discardRecovered" {
            decode(&session.bytes)?
        } else {
            source.unwrap().to_owned()
        };
        check_size(source.as_bytes())?;
        let reviewed = Draft {
            scope: self.draft_scope_for_session(session_id, session.conflicting_draft.as_ref())?,
            path: display(&session.path),
            base_source: decode(&session.bytes)?,
            source,
            base_known: true,
            identity: session.stamp.identity.clone(),
            alternatives: Vec::new(),
            producer_id: Some(session.draft_producer_id.clone()),
        };
        // Validate the complete replacement before changing history or the
        // primary. Archive the latest complete record, not the open-time cache
        // or a source-only comparison that misses an ABA primary.
        let next_witness = reviewed.witness()?;
        if let Some((_, original_bytes)) = latest_record {
            journal::private_create(
                &self
                    .draft_directory("draft-history")
                    .join(format!("{}.json", fresh())),
                &original_bytes,
            )?;
        }
        self.store_draft(&reviewed)?;
        let session = self.sessions.get_mut(session_id).unwrap();
        session.conflicting_draft = None;
        session.draft_witness = Some(next_witness);
        session.draft_input = Some(reviewed.proposal());
        Ok(())
    }
    pub fn save(&mut self, request: SaveRequest) -> Result<SaveOutcome> {
        self.save_with_restore(request, None)
    }
    fn save_with_restore(
        &mut self,
        request: SaveRequest,
        restore: Option<history::RestoreBinding>,
    ) -> Result<SaveOutcome> {
        self.check_actor_session(&request.session_id)?;
        valid_operation_id(&request.operation_id)?;
        self.check_attempt_not_cancelled(&request.operation_id)?;
        check_size(request.source.as_bytes())?;
        let session = self
            .sessions
            .get(&request.session_id)
            .cloned()
            .ok_or_else(|| {
                CoreError::new("sessionMissing", UiMessage::error_reopen_file_session())
            })?;
        let (original_path, scope) = if let Some(plan) = self.operations.get(&request.operation_id)
        {
            self.validate_file_save_plan(plan)?;
            (
                PathBuf::from(&plan.path),
                plan.file_save.as_ref().unwrap().scope.clone(),
            )
        } else {
            (session.path.clone(), session.file_scope.clone()?)
        };
        let fingerprint = original_delivery::save_fingerprint(
            &request,
            &original_path,
            restore.as_ref(),
            scope.clone(),
        )?;
        if let Some(receipt) = self.retry(&request.operation_id, &fingerprint)? {
            return Ok(SaveOutcome::committed(receipt));
        }
        self.validate_current_file_save_scope(&scope, &session.path)?;
        if request.base_id != session.base_id {
            let draft = Draft {
                scope: self.draft_scope_for_session(
                    &request.session_id,
                    session.conflicting_draft.as_ref(),
                )?,
                path: display(&session.path),
                base_source: String::new(),
                source: request.source.clone(),
                base_known: false,
                identity: session.stamp.identity.clone(),
                alternatives: Vec::new(),
                producer_id: Some(session.draft_producer_id.clone()),
            };
            let draft = self.store_session_draft(&request.session_id, draft)?;
            self.sessions
                .get_mut(&request.session_id)
                .unwrap()
                .conflicting_draft = Some(draft);
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_save_base_unknown(),
            ));
        }
        self.persist_draft(&request.session_id, &request.source)?;
        if session.conflicting_draft.is_some() {
            return Err(CoreError::new(
                "draftConflict",
                UiMessage::error_recovered_draft_base_conflict(),
            ));
        }
        self.check_pending(&session.path)?;
        if session.path.components().any(|c| {
            c.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(".weftext-trash")
        }) {
            return Err(CoreError::new(
                "trashReadOnly",
                UiMessage::error_restore_before_source_edit(),
            ));
        }
        if let Some(expected) = &session.node_id {
            let metadata = weftext_engine::inspect(&request.source);
            if metadata.id.as_ref() != Some(expected) {
                return Err(CoreError::new(
                    "identityConflict",
                    UiMessage::error_admitted_node_identity_changed(),
                ));
            }
        }
        let guard = ParentGuard::acquire(&session.path)?;
        let mut file = guard.open_file(true, false)?;
        let before = read_source(&mut file)?;
        let current_stamp = stamp(&file)?;
        if current_stamp != session.stamp || before != session.bytes {
            return Err(CoreError::conflict(
                UiMessage::error_source_changed_outside_session(),
                &before,
            ));
        }
        let mut plan = journal::Plan::file(
            request.operation_id.clone(),
            fingerprint,
            &session.path,
            Some(before),
            request.source.as_bytes().to_vec(),
            Some(current_stamp.identity.clone()),
            "save",
        );
        plan.file_save = Some(FileSaveData::new(
            request.session_id.clone(),
            Some(request.base_id.clone()),
            scope,
        ));
        plan.receipt = Some(Receipt {
            revision_id: None,
            operation_id: request.operation_id,
            status: "saved".into(),
            path: display(&session.path),
            base_id: if request.source.as_bytes() == session.bytes {
                session.base_id.clone()
            } else {
                fresh()
            },
        });
        let content_revision = if plan.before.as_ref() == Some(&plan.after) {
            self.accepted_revision(&operations::CapturedFile {
                path: display(&session.path),
                bytes: plan.after.clone(),
                stamp: current_stamp.clone(),
            })
            .unwrap_or_else(fresh)
        } else {
            plan.receipt.as_ref().unwrap().base_id.clone()
        };
        plan.content_revision = Some(content_revision.clone());
        plan.receipt.as_mut().unwrap().revision_id = Some(content_revision);
        plan.history_restore = restore;
        self.validate_sync_source_stamp(&session.path, &current_stamp)?;
        self.stage(plan.clone())?;
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("saveStaged")?;
        if plan.before.as_ref() != Some(&plan.after) {
            write_source(&mut file, &plan.after)?;
        }
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("sourceWritten")?;
        let new_stamp = stamp(&file)?;
        self.record_observation(
            &plan.operation_id,
            &session.path,
            &new_stamp,
            &plan.after,
            plan.content_revision.as_ref().unwrap(),
        )?;
        self.commit(&plan)?;
        #[cfg(all(test, windows))]
        hosted::definition_test_failpoint("saveCommitted")?;
        let receipt = plan.receipt.clone().unwrap();
        self.sessions.insert(
            request.session_id.clone(),
            Session {
                base_id: receipt.base_id.clone(),
                bytes: plan.after,
                stamp: new_stamp,
                ..self.sessions.get(&request.session_id).cloned().unwrap()
            },
        );
        #[cfg(all(test, windows))]
        let draft_update = hosted::definition_test_failpoint("saveDraftUpdate")
            .and_then(|()| self.persist_draft(&request.session_id, &request.source));
        #[cfg(not(all(test, windows)))]
        let draft_update = self.persist_draft(&request.session_id, &request.source);
        let recovery_issue = draft_update.err().map(|error| {
            let code = error.code.clone();
            CoreError::new(
                &code,
                UiMessage::error_core_committed_draft_update_unavailable(),
            )
            .caused_by(error)
        });
        Ok(SaveOutcome {
            receipt,
            recovery_issue,
        })
    }
    pub fn save_as(&mut self, request: SaveAsRequest) -> Result<SaveOutcome> {
        self.check_actor_session(&request.session_id)?;
        valid_operation_id(&request.operation_id)?;
        self.check_attempt_not_cancelled(&request.operation_id)?;
        check_size(request.source.as_bytes())?;
        let session = self
            .sessions
            .get(&request.session_id)
            .cloned()
            .ok_or_else(|| {
                CoreError::new("sessionMissing", UiMessage::error_open_source_first())
            })?;
        let path = absolute_path(Path::new(&request.path))?;
        check_typ(&path)?;
        if let Some(plan) = self.operations.get(&request.operation_id) {
            self.validate_file_save_plan(plan)?;
            let data = FileSaveData::new(
                request.session_id.clone(),
                None,
                plan.file_save.as_ref().unwrap().scope.clone(),
            );
            let fingerprint =
                file_save_scope::fingerprint("saveAs", &path, &request.source, &data, None)?;
            if let Some(receipt) = self.retry(&request.operation_id, &fingerprint)? {
                return Ok(SaveOutcome::committed(receipt));
            }
        }
        self.persist_draft(&request.session_id, &request.source)?;
        self.check_pending(&path)?;
        if path == session.path {
            return Err(CoreError::new(
                "exists",
                UiMessage::error_save_as_target_must_be_absent(),
            ));
        }
        let guard = ParentGuard::acquire(&path)?;
        let data = FileSaveData::new(
            request.session_id.clone(),
            None,
            self.capture_file_save_scope(&path)?,
        );
        let fingerprint =
            file_save_scope::fingerprint("saveAs", &path, &request.source, &data, None)?;
        let mut plan = journal::Plan::file(
            request.operation_id.clone(),
            fingerprint,
            &path,
            None,
            request.source.as_bytes().to_vec(),
            None,
            "saveAs",
        );
        plan.file_save = Some(data);
        plan.receipt = Some(Receipt {
            revision_id: None,
            operation_id: request.operation_id,
            status: "saved".into(),
            path: display(&path),
            base_id: fresh(),
        });
        self.stage(plan.clone())?;
        let mut file = match guard.open_file(true, true) {
            Ok(file) => file,
            Err(e) => {
                self.resolve_record(&plan.operation_id, "createRejected")?;
                return Err(e);
            }
        };
        self.record_materialized(&plan.operation_id, &path, &file)?;
        write_source(&mut file, &plan.after)?;
        self.commit(&plan)?;
        Ok(SaveOutcome::committed(plan.receipt.unwrap()))
    }
    pub fn create_library(&mut self, path: impl AsRef<Path>) -> Result<LibraryInfo> {
        self.create_library_with_id(path, &fresh())
    }
    pub fn create_library_with_id(
        &mut self,
        path: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<LibraryInfo> {
        self.create_library_with_options_and_id(path, LibraryCreateOptions::default(), operation_id)
    }
    pub fn create_note(
        &mut self,
        root: impl AsRef<Path>,
        parent: impl AsRef<Path>,
        name: &str,
    ) -> Result<OperationReceipt> {
        self.create_note_with_id(root, parent, name, &fresh())
    }
    /// Creation commits through the shared NewNode transaction. Opening is a
    /// separate delivery step and cannot erase the original receipt.
    pub fn create_note_with_id(
        &mut self,
        root: impl AsRef<Path>,
        parent: impl AsRef<Path>,
        name: &str,
        operation_id: &str,
    ) -> Result<OperationReceipt> {
        valid_operation_id(operation_id)?;
        let preview = self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(root.as_ref()),
            intent: OperationIntent::NewNode {
                parent: display(parent.as_ref()),
                name: name.into(),
                source: None,
            },
        })?;
        self.execute_operation(operation_id, &preview.plan_id)
    }
    #[cfg(test)]
    fn create_note_fixture(
        &mut self,
        root: impl AsRef<Path>,
        parent: impl AsRef<Path>,
        name: &str,
    ) -> Result<OpenedDocument> {
        let receipt = self.create_note(root.as_ref(), parent, name)?;
        self.open(
            Path::new(&receipt.path).join(format!("{name}.typ")),
            Some(root.as_ref().to_path_buf()),
        )
    }
    fn workspace(&self, root: &Path) -> Result<Option<WorkspaceConfig>> {
        let config_path = root.join(".weftext-workspace.json");
        if !config_path.exists() {
            return Ok(None);
        }
        let read_control = |path: &Path| -> Result<Vec<u8>> {
            let guard = ParentGuard::acquire(path)?;
            let mut file = guard.open_read_guard_file()?;
            read_source(&mut file)
        };
        let config: WorkspaceConfig = match serde_json::from_slice(&read_control(&config_path)?) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        if config.version != 1 || config.storage_profile != STORAGE_PROFILE {
            return Ok(None);
        }
        if read_control(&root.join(".weftext-format"))? != b"weftext.typst.v1\n" {
            return Ok(None);
        }
        let activation: serde_json::Value =
            match read_control(&root.join(".weftext-mvp-commit.json")) {
                Ok(bytes) => serde_json::from_slice(&bytes)?,
                Err(_) => return Ok(None),
            };
        let operation_id = activation["operationId"].as_str().unwrap_or("");
        let owned = self.operations.get(operation_id).is_some_and(|p| {
            matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                && crate::history::history_relative_path(root, Path::new(&p.path))
                    .is_some_and(|p| p.as_os_str().is_empty())
                && backend::same_directory(Path::new(&p.path), root).unwrap_or(false)
                && self.library_creation_root_matches(p, root).unwrap_or(false)
                && p.workspace_id.as_ref() == Some(&config.workspace_id)
                && self.committed(operation_id)
        });
        Ok(owned.then_some(config))
    }
    fn admitted(&self, node_id: &str, workspace_id: &str, identity: &FileIdentity) -> bool {
        self.operations.values().any(|p| {
            p.workspace_id.as_deref() == Some(workspace_id)
                && self.committed(&p.operation_id)
                && self.read_births(&p.operation_id).ok().is_some_and(|bs| {
                    bs.iter().any(|b| {
                        b.node_id == node_id
                            && b.workspace_id == workspace_id
                            && &b.identity == identity
                    })
                })
        })
    }
    fn admitted_binding(&self, identity: &FileIdentity) -> Option<Birth> {
        self.operations
            .values()
            .filter(|p| self.committed(&p.operation_id))
            .find_map(|p| {
                self.read_births(&p.operation_id)
                    .inspect_err(|error| {
                        resource_diagnostics::error(
                            "admitted_binding.births",
                            &p.operation_id,
                            error,
                        )
                    })
                    .ok()?
                    .into_iter()
                    .find(|b| &b.identity == identity)
            })
    }
    #[cfg(test)]
    fn legacy_draft_path(&self, path: &Path) -> PathBuf {
        self.draft_directory("drafts").join(format!(
            "{}.json",
            hash(display(path).replace('/', "\\").to_lowercase().as_bytes())
        ))
    }
    fn read_draft(&self, path: &Path) -> Result<Option<Draft>> {
        self.read_relocated_draft(path)
    }
}

fn fresh() -> String {
    Uuid::new_v4().to_string()
}
fn default_note_source(node_id: &str, name: &str) -> Result<String> {
    let title = weftext_engine::encode_literal(name)
        .map_err(|e| CoreError::from_engine("invalidName", e))?;
    Ok(format!(
        "#metadata((id: \"{node_id}\", types: ())) <weftext>\n\n#set document(title: [{title}])\n#title()\n\n"
    ))
}
fn known_base() -> bool {
    true
}
fn instance_lock(data_dir: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0).custom_flags(0x00200000);
    }
    options.open(data_dir.join("instance.lock")).map_err(|e| {
        if matches!(e.raw_os_error(), Some(32 | 33)) {
            CoreError::new(
                "instanceBusy",
                UiMessage::error_private_core_instance_busy(),
            )
        } else {
            e.into()
        }
    })
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
fn decode(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec())
        .map_err(|_| CoreError::new("encoding", UiMessage::error_source_not_utf8()))
}
fn check_size(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_SOURCE_BYTES {
        Err(CoreError::new(
            "tooLarge",
            UiMessage::error_source_byte_limit(),
        ))
    } else {
        Ok(())
    }
}
fn read_source(file: &mut File) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    check_size(&bytes)?;
    Ok(bytes)
}
fn write_source(file: &mut File, bytes: &[u8]) -> Result<()> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(bytes)?;
    file.set_len(bytes.len() as u64)?;
    file.sync_all()?;
    if read_source(file)? != bytes {
        return Err(CoreError::new(
            "verificationFailed",
            UiMessage::error_source_readback_mismatch(),
        ));
    }
    Ok(())
}
fn check_typ(path: &Path) -> Result<()> {
    if path
        .extension()
        .is_none_or(|x| !x.eq_ignore_ascii_case("typ"))
    {
        return Err(CoreError::new(
            "unsupportedFile",
            UiMessage::error_native_source_file_required(),
        ));
    }
    Ok(())
}
fn valid_name(name: &str) -> Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty()
        || name.len() > 120
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*#[]".contains(c))
        || name.starts_with('.')
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        return Err(CoreError::new(
            "invalidName",
            UiMessage::error_invalid_note_name(),
        ));
    }
    Ok(())
}
fn valid_operation_id(id: &str) -> Result<()> {
    if !Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id) {
        return Err(CoreError::new(
            "invalidOperationId",
            UiMessage::error_invalid_operation_identifier(),
        ));
    }
    Ok(())
}
pub(crate) fn absolute_path(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(CoreError::new(
            "invalidPath",
            UiMessage::error_absolute_path_required(),
        ));
    }
    if path
        .components()
        .any(|p| matches!(p, Component::ParentDir | Component::CurDir))
    {
        return Err(CoreError::new(
            "invalidPath",
            UiMessage::error_dot_path_components_unsupported(),
        ));
    }
    if path.to_string_lossy().contains('\0') {
        return Err(CoreError::new(
            "invalidPath",
            UiMessage::error_path_contains_nul(),
        ));
    }
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if text.starts_with("\\\\") || text[2..].contains(':') {
            return Err(CoreError::new(
                "invalidPath",
                UiMessage::error_path_namespace_unsupported(),
            ));
        }
        for component in path.components().filter_map(|c| {
            if let Component::Normal(n) = c {
                Some(n)
            } else {
                None
            }
        }) {
            let name = component.to_string_lossy();
            if name.ends_with([' ', '.']) {
                return Err(CoreError::new(
                    "invalidPath",
                    UiMessage::error_path_alias_unsupported(),
                ));
            }
        }
    }
    Ok(path.to_path_buf())
}
fn is_canonical(path: &Path) -> bool {
    path.file_stem()
        .zip(path.parent().and_then(Path::file_name))
        .is_some_and(|(a, b)| a == b)
}
fn private_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("{}.tmp", fresh()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(all(test, windows))]
mod tests;
