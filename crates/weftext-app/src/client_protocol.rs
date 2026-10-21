//! The fixed business call registry shared by the console and the local owner.
//! Callers choose an operation, never an arbitrary URI, method, header or host.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const WIRE_VERSION: u32 = 1;
pub const MAX_WIRE_BYTES: usize = 96 * 1024 * 1024;
pub const MAX_REPLY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallKind {
    Capabilities,
    Documents,
    Open,
    EditorSnapshot,
    LibraryInspect,
    LibraryCreate,
    NoteCreate,
    Save,
    SaveAs,
    QueryCatalog,
    QueryRun,
    OperationPrepare,
    OperationCommit,
    OperationStatus,
    OperationSettle,
    StructureStatus,
    StructureSettle,
    RecoveryList,
    RecoveryInspect,
    RecoveryResolve,
    HistoryList,
    HistoryRead,
    HistoryCompare,
    HistoryRestore,
    HistoryStatus,
    HistorySettle,
    ExportPrepare,
    ExportStatus,
    ExportConfirm,
    ExportDownload,
    ImportInspect,
    ImportPrepare,
    ImportCommit,
    ImportStatus,
    ImportSettle,
    OfficeImportInspect,
    OfficeImportPrepare,
    OfficeImportCommit,
    OfficeImportStatus,
    OfficeImportSettle,
    SyncBinding,
    SyncEnable,
    SyncStatus,
    SyncPublish,
    SyncReceive,
    SyncExport,
    SyncHistory,
    SyncOperation,
    SyncCancel,
    SyncAdoptionCut,
    SyncRevalidationCut,
    SyncRevalidationReview,
    RemoteSyncInspect,
    RemoteSyncStart,
    RemoteSyncJob,
    RemoteSyncStop,
}

impl CallKind {
    /// Both the broker and console use this exact registry. No fallback route.
    pub fn local_route(self) -> Option<(LocalMethod, &'static str)> {
        use CallKind::*;
        use LocalMethod::*;
        Some(match self {
            Capabilities => (Get, "/api/bootstrap"),
            Documents => return None,
            Open => (Post, "/api/open"),
            EditorSnapshot => (Post, "/api/editor-map"),
            LibraryInspect => (Post, "/api/library/open"),
            LibraryCreate => (Post, "/api/library/create"),
            NoteCreate => (Post, "/api/note/create"),
            Save => (Post, "/api/save"),
            SaveAs => (Post, "/api/save-as"),
            QueryCatalog => (Post, "/api/data/schema"),
            QueryRun => (Post, "/api/query/run"),
            OperationPrepare => (Post, "/api/operations/prepare"),
            OperationCommit => (Post, "/api/operations/commit"),
            OperationStatus => (Post, "/api/operations/status"),
            OperationSettle | StructureSettle => (Post, "/api/operations/settle"),
            StructureStatus => (Post, "/api/operations/status"),
            RecoveryList => (Get, "/api/recovery"),
            RecoveryInspect => (Post, "/api/recovery/inspect"),
            RecoveryResolve => (Post, "/api/recovery/resolve"),
            HistoryList => (Post, "/api/history/list"),
            HistoryRead => (Post, "/api/history/read"),
            HistoryCompare => (Post, "/api/history/compare"),
            HistoryRestore => (Post, "/api/history/restore"),
            HistoryStatus => (Post, "/api/history/status"),
            HistorySettle => return None,
            ExportPrepare => (Post, "/api/export/prepare"),
            ExportStatus => (Post, "/api/export/status"),
            ExportConfirm => (Post, "/api/export/confirm"),
            ExportDownload => (Post, "/api/export/download"),
            ImportInspect | OfficeImportInspect => (Post, "/api/exchange/inspect"),
            ImportPrepare | OfficeImportPrepare => (Post, "/api/exchange/prepare"),
            ImportCommit | OfficeImportCommit => (Post, "/api/exchange/commit"),
            ImportStatus | OfficeImportStatus => (Post, "/api/operations/status"),
            ImportSettle | OfficeImportSettle => (Post, "/api/operations/settle"),
            SyncBinding => (Post, "/api/sync/binding"),
            SyncEnable => (Post, "/api/sync/enable"),
            SyncStatus => (Post, "/api/sync/status"),
            SyncPublish => (Post, "/api/sync/publish"),
            SyncReceive => (Post, "/api/sync/receive"),
            SyncExport => (Post, "/api/sync/export"),
            SyncHistory => (Post, "/api/sync/history"),
            SyncOperation => (Post, "/api/sync/operation"),
            SyncCancel => (Post, "/api/sync/cancel-prepared"),
            SyncAdoptionCut => (Post, "/api/sync/adoption-cut"),
            SyncRevalidationCut => (Post, "/api/sync/revalidation-cut"),
            SyncRevalidationReview => (Post, "/api/sync/revalidation-review"),
            RemoteSyncInspect => (Post, "/api/sync/remote/inspect"),
            RemoteSyncStart => (Post, "/api/sync/remote/start"),
            RemoteSyncJob => (Post, "/api/sync/remote/job"),
            RemoteSyncStop => (Post, "/api/sync/remote/stop"),
        })
    }

    pub(crate) fn remote_route(self) -> Option<(LocalMethod, &'static str)> {
        use CallKind::*;
        use LocalMethod::*;
        Some(match self {
            Capabilities => (Get, "/api/hosted/me"),
            Documents => (Get, "/api/hosted/documents"),
            Open => (Post, "/api/hosted/open"),
            EditorSnapshot => (Post, "/api/hosted/editor/map"),
            Save => (Post, "/api/hosted/save"),
            QueryCatalog => (Get, "/api/hosted/query/catalog"),
            QueryRun => (Post, "/api/hosted/query"),
            OperationPrepare => (Post, "/api/hosted/structure/prepare"),
            OperationCommit => (Post, "/api/hosted/structure/commit"),
            OperationStatus => (Post, "/api/hosted/operation"),
            OperationSettle => (Post, "/api/hosted/operation/settle"),
            StructureStatus => (Post, "/api/hosted/structure/operation"),
            StructureSettle => (Post, "/api/hosted/structure/settle"),
            HistoryList => (Post, "/api/hosted/history/list"),
            HistoryRead => (Post, "/api/hosted/history/read"),
            HistoryCompare => (Post, "/api/hosted/history/compare"),
            HistoryRestore => (Post, "/api/hosted/history/restore"),
            HistoryStatus => (Post, "/api/hosted/history/status"),
            HistorySettle => (Post, "/api/hosted/history/settle"),
            ExportPrepare => (Post, "/api/hosted/export/prepare"),
            ExportStatus => (Post, "/api/hosted/export/status"),
            ExportConfirm => (Post, "/api/hosted/export/confirm"),
            ExportDownload => (Post, "/api/hosted/export/chunk"),
            ImportInspect => (Post, "/api/hosted/import/inspect"),
            ImportPrepare => (Post, "/api/hosted/import/prepare"),
            ImportCommit => (Post, "/api/hosted/import/commit"),
            ImportStatus => (Post, "/api/hosted/import/operation"),
            ImportSettle => (Post, "/api/hosted/import/settle"),
            OfficeImportInspect => (Post, "/api/hosted/office/import/inspect"),
            OfficeImportPrepare => (Post, "/api/hosted/office/import/prepare"),
            OfficeImportCommit => (Post, "/api/hosted/office/import/commit"),
            OfficeImportStatus => (Post, "/api/hosted/office/import/operation"),
            OfficeImportSettle => (Post, "/api/hosted/office/import/settle"),
            SyncStatus => (Post, "/api/hosted/sync/status"),
            _ => return None,
        })
    }

    pub(crate) fn is_effect(self) -> bool {
        use CallKind::*;
        matches!(
            self,
            LibraryCreate
                | NoteCreate
                | Save
                | SaveAs
                | OperationPrepare
                | OperationCommit
                | OperationSettle
                | StructureSettle
                | RecoveryResolve
                | HistoryRestore
                | HistorySettle
                | ExportPrepare
                | ExportConfirm
                | ImportPrepare
                | ImportCommit
                | ImportSettle
                | OfficeImportPrepare
                | OfficeImportCommit
                | OfficeImportSettle
                | SyncEnable
                | SyncPublish
                | SyncReceive
                | SyncExport
                | SyncCancel
                | RemoteSyncStart
                | RemoteSyncStop
        )
    }

    pub(crate) fn identifier_field(self) -> Option<&'static str> {
        use CallKind::*;
        match self {
            ExportPrepare | ExportStatus | ExportConfirm | ExportDownload | RemoteSyncStart
            | RemoteSyncJob | RemoteSyncStop => Some("requestId"),
            LibraryCreate
            | NoteCreate
            | Save
            | SaveAs
            | OperationPrepare
            | OperationCommit
            | OperationStatus
            | OperationSettle
            | StructureStatus
            | StructureSettle
            | RecoveryInspect
            | RecoveryResolve
            | HistoryRestore
            | HistoryStatus
            | HistorySettle
            | ImportPrepare
            | ImportCommit
            | ImportStatus
            | ImportSettle
            | OfficeImportPrepare
            | OfficeImportCommit
            | OfficeImportStatus
            | OfficeImportSettle
            | SyncEnable
            | SyncOperation
            | SyncCancel
            | SyncRevalidationReview => Some("operationId"),
            _ => None,
        }
    }

    /// Only true original-result endpoints. A collection status is never a receipt.
    pub(crate) fn original_status(self, remote: bool) -> Option<Self> {
        use CallKind::*;
        match self {
            Save | SaveAs if !remote => Some(OperationStatus),
            Save | OperationSettle if remote => Some(OperationStatus),
            OperationSettle => Some(OperationStatus),
            StructureSettle => Some(if remote {
                StructureStatus
            } else {
                OperationStatus
            }),
            ExportPrepare | ExportConfirm => Some(ExportStatus),
            RemoteSyncStart | RemoteSyncStop => Some(RemoteSyncJob),
            ImportPrepare | ImportCommit | ImportSettle => Some(ImportStatus),
            OfficeImportPrepare | OfficeImportCommit | OfficeImportSettle => {
                Some(OfficeImportStatus)
            }
            HistoryRestore | HistorySettle => Some(HistoryStatus),
            OperationPrepare | OperationCommit if remote => Some(StructureStatus),
            LibraryCreate | NoteCreate | OperationPrepare | OperationCommit => {
                Some(OperationStatus)
            }
            _ => None,
        }
    }

    pub(crate) fn fence_kind(self) -> Option<&'static str> {
        match self {
            Self::Save => Some("save"),
            Self::SaveAs => Some("saveAs"),
            Self::HistoryRestore => Some("save"),
            Self::LibraryCreate => Some("createLibrary"),
            Self::NoteCreate => Some("newNode"),
            Self::OperationPrepare
            | Self::OperationCommit
            | Self::ImportPrepare
            | Self::ImportCommit
            | Self::OfficeImportPrepare
            | Self::OfficeImportCommit => Some("operation"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BusinessCall {
    pub kind: CallKind,
    pub body: Value,
}

impl BusinessCall {
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if !self.body.is_object() {
            return Err("inputShape");
        }
        for source in [
            self.body.get("source"),
            self.body.get("draftSource"),
            self.body
                .get("context")
                .and_then(|context| context.get("source")),
        ]
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        {
            if source.len() > MAX_SOURCE_BYTES {
                return Err("inputBudget");
            }
        }
        let bytes = serde_json::to_vec(&self.body).map_err(|_| "inputJson")?;
        if bytes.len() > MAX_WIRE_BYTES {
            return Err("inputBudget");
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_calls_cannot_be_deserialized_into_a_route() {
        for value in ["/api/admin", "http://evil.test/api/open", "open?path=x"] {
            assert!(serde_json::from_value::<CallKind>(serde_json::json!(value)).is_err());
        }
        assert_eq!(
            CallKind::Save.local_route(),
            Some((LocalMethod::Post, "/api/save"))
        );
        assert!(CallKind::LibraryCreate.remote_route().is_none());
    }

    #[test]
    fn p0_unified_lookup_preserves_save_and_separate_hosted_families() {
        assert_eq!(
            CallKind::Save.original_status(false),
            Some(CallKind::OperationStatus)
        );
        assert_eq!(
            CallKind::Save.original_status(true),
            Some(CallKind::OperationStatus)
        );
        assert_eq!(
            CallKind::OperationPrepare.original_status(true),
            Some(CallKind::StructureStatus)
        );
        assert_eq!(CallKind::SyncPublish.original_status(false), None);
        assert!(!CallKind::SyncRevalidationReview.is_effect());
        assert_eq!(
            CallKind::RemoteSyncStart.original_status(false),
            Some(CallKind::RemoteSyncJob)
        );
        assert!(CallKind::RemoteSyncStart.remote_route().is_none());
        assert_eq!(
            CallKind::EditorSnapshot.local_route(),
            Some((LocalMethod::Post, "/api/editor-map"))
        );
    }
}
