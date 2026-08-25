use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use uuid::{Uuid, Version};

use crate::content_boundary::{ContentRules, linked_or_reparse, portable_path};
use crate::{NodeId, WorkspaceRevision, parse_node_metadata, read_node_document};

pub const TRASH_DIRECTORY_NAME: &str = ".weftext-trash";
pub const TRASH_ITEMS_DIRECTORY_NAME: &str = "_weftext.items";
pub const TRASH_ITEM_MANIFEST_FILE_NAME: &str = "_weftext.trash-item.json";
pub const TRASH_ITEM_PAYLOAD_DIRECTORY_NAME: &str = "payload";
pub const TRASH_ITEM_SCHEMA: &str = "weftext.trash-item/v1";
pub const TRASH_REVIEWED_REQUEST_SCHEMA: &str = "weftext.trash-reviewed-request/v1";
pub const LEGACY_TRASH_MIGRATION_BACKUP_SCHEMA: &str = "weftext.legacy-trash-migration-backup/v1";
pub const TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE: &str =
    "PERMANENTLY DELETE SELECTED TRASH ITEMS";

const MAX_TRASH_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_TRASH_REVIEWED_REQUEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_TRASH_PAYLOAD_ENTRIES: usize = 100_000;
const MAX_TRASH_PAYLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub(crate) fn is_trash_storage_path(root: &Path, path: &Path) -> bool {
    let trash = root.join(TRASH_DIRECTORY_NAME);
    path == trash || path.starts_with(trash)
}

macro_rules! temporary_uuid_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new_v4() -> Self {
                Self(Uuid::new_v4())
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = TrashIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let parsed =
                    Uuid::parse_str(value).map_err(|_| TrashIdError::InvalidUuid($label))?;
                if parsed.get_version() != Some(Version::Random) {
                    return Err(TrashIdError::NotVersionFour($label));
                }
                if parsed.hyphenated().to_string() != value {
                    return Err(TrashIdError::NotCanonicalLowercase($label));
                }
                Ok(Self(parsed))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

temporary_uuid_id!(TrashItemId, "trash item ID");
temporary_uuid_id!(TrashOperationId, "trash operation ID");
temporary_uuid_id!(TrashReviewId, "Trash review ID");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrashIdError {
    InvalidUuid(&'static str),
    NotVersionFour(&'static str),
    NotCanonicalLowercase(&'static str),
}

impl fmt::Display for TrashIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (label, reason) = match self {
            Self::InvalidUuid(label) => (*label, "is not a UUID"),
            Self::NotVersionFour(label) => (*label, "is not UUIDv4"),
            Self::NotCanonicalLowercase(label) => (*label, "is not canonical lowercase UUID text"),
        };
        write!(formatter, "{label} {reason}")
    }
}

impl std::error::Error for TrashIdError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashItemKind {
    Node,
    Resource,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashOriginStatus {
    Known,
    Unknown,
}

/// Portable authority for one independently recoverable Workspace Trash item.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrashItemManifest {
    Node {
        schema: String,
        #[serde(rename = "trashItemId")]
        trash_item_id: TrashItemId,
        #[serde(rename = "operationId")]
        operation_id: TrashOperationId,
        #[serde(rename = "trashedAt")]
        trashed_at: String,
        #[serde(rename = "originStatus")]
        origin_status: TrashOriginStatus,
        #[serde(rename = "nodeId")]
        node_id: NodeId,
        #[serde(rename = "originalParentNodeId")]
        original_parent_node_id: Option<NodeId>,
        #[serde(rename = "originalName")]
        original_name: String,
        #[serde(rename = "ancestorNodeIds")]
        ancestor_node_ids: Vec<NodeId>,
        #[serde(rename = "payloadSha256")]
        payload_sha256: String,
        #[serde(rename = "payloadByteLength")]
        payload_byte_length: u64,
        #[serde(rename = "payloadEntryCount")]
        payload_entry_count: u64,
    },
    Resource {
        schema: String,
        #[serde(rename = "trashItemId")]
        trash_item_id: TrashItemId,
        #[serde(rename = "operationId")]
        operation_id: TrashOperationId,
        #[serde(rename = "trashedAt")]
        trashed_at: String,
        #[serde(rename = "originStatus")]
        origin_status: TrashOriginStatus,
        #[serde(rename = "originalOwnerNodeId")]
        original_owner_node_id: Option<NodeId>,
        #[serde(rename = "originalName")]
        original_name: String,
        #[serde(rename = "sha256")]
        sha256: String,
        #[serde(rename = "byteLength")]
        byte_length: u64,
    },
}

impl TrashItemManifest {
    /// Parses and semantically validates one closed v1 manifest, including duplicate-key
    /// rejection. This is inspection only; mutation plans never accept caller-authored manifests.
    ///
    /// # Errors
    ///
    /// Returns an error for oversized, malformed, duplicate-key, or semantically invalid input.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, String> {
        crate::workspace_transaction::reject_duplicate_json_keys(bytes)
            .map_err(|error| error.to_string())?;
        let manifest: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        validate_manifest(&manifest, Path::new(TRASH_ITEM_MANIFEST_FILE_NAME))
            .map_err(|issue| issue.message)?;
        Ok(manifest)
    }

    /// Returns Core's one canonical UTF-8 serialization for the manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest does not satisfy the closed v1 contract.
    pub fn to_canonical_json_bytes(&self) -> Result<Vec<u8>, String> {
        validate_manifest(self, Path::new(TRASH_ITEM_MANIFEST_FILE_NAME))
            .map_err(|issue| issue.message)?;
        manifest_bytes(self).map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_node(
        trash_item_id: TrashItemId,
        operation_id: TrashOperationId,
        trashed_at: String,
        node_id: NodeId,
        original_parent_node_id: Option<NodeId>,
        original_name: String,
        ancestor_node_ids: Vec<NodeId>,
        summary: &TrashPayloadSummary,
    ) -> Result<Self, String> {
        let manifest = Self::Node {
            schema: TRASH_ITEM_SCHEMA.to_owned(),
            trash_item_id,
            operation_id,
            trashed_at,
            origin_status: if original_parent_node_id.is_some() {
                TrashOriginStatus::Known
            } else {
                TrashOriginStatus::Unknown
            },
            node_id,
            original_parent_node_id,
            original_name,
            ancestor_node_ids,
            payload_sha256: summary.sha256.clone(),
            payload_byte_length: summary.byte_length,
            payload_entry_count: summary.entry_count,
        };
        validate_manifest(&manifest, Path::new(TRASH_ITEM_MANIFEST_FILE_NAME))
            .map_err(|issue| issue.message)?;
        Ok(manifest)
    }

    pub(crate) fn new_resource(
        trash_item_id: TrashItemId,
        operation_id: TrashOperationId,
        trashed_at: String,
        original_owner_node_id: Option<NodeId>,
        original_name: String,
        bytes: &[u8],
    ) -> Result<Self, String> {
        let manifest = Self::Resource {
            schema: TRASH_ITEM_SCHEMA.to_owned(),
            trash_item_id,
            operation_id,
            trashed_at,
            origin_status: if original_owner_node_id.is_some() {
                TrashOriginStatus::Known
            } else {
                TrashOriginStatus::Unknown
            },
            original_owner_node_id,
            original_name,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            byte_length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        };
        validate_manifest(&manifest, Path::new(TRASH_ITEM_MANIFEST_FILE_NAME))
            .map_err(|issue| issue.message)?;
        Ok(manifest)
    }

    #[must_use]
    pub const fn kind(&self) -> TrashItemKind {
        match self {
            Self::Node { .. } => TrashItemKind::Node,
            Self::Resource { .. } => TrashItemKind::Resource,
        }
    }

    #[must_use]
    pub const fn trash_item_id(&self) -> TrashItemId {
        match self {
            Self::Node { trash_item_id, .. } | Self::Resource { trash_item_id, .. } => {
                *trash_item_id
            }
        }
    }

    #[must_use]
    pub const fn operation_id(&self) -> TrashOperationId {
        match self {
            Self::Node { operation_id, .. } | Self::Resource { operation_id, .. } => *operation_id,
        }
    }

    #[must_use]
    pub fn original_name(&self) -> &str {
        match self {
            Self::Node { original_name, .. } | Self::Resource { original_name, .. } => {
                original_name
            }
        }
    }

    #[must_use]
    pub const fn node_id(&self) -> Option<NodeId> {
        match self {
            Self::Node { node_id, .. } => Some(*node_id),
            Self::Resource { .. } => None,
        }
    }

    #[must_use]
    pub const fn original_parent_node_id(&self) -> Option<NodeId> {
        match self {
            Self::Node {
                original_parent_node_id,
                ..
            } => *original_parent_node_id,
            Self::Resource { .. } => None,
        }
    }

    #[must_use]
    pub const fn original_owner_node_id(&self) -> Option<NodeId> {
        match self {
            Self::Resource {
                original_owner_node_id,
                ..
            } => *original_owner_node_id,
            Self::Node { .. } => None,
        }
    }

    #[must_use]
    pub const fn origin_status(&self) -> TrashOriginStatus {
        match self {
            Self::Node { origin_status, .. } | Self::Resource { origin_status, .. } => {
                *origin_status
            }
        }
    }

    #[must_use]
    pub fn trashed_at(&self) -> &str {
        match self {
            Self::Node { trashed_at, .. } | Self::Resource { trashed_at, .. } => trashed_at,
        }
    }

    #[must_use]
    pub fn payload_sha256(&self) -> &str {
        match self {
            Self::Node { payload_sha256, .. } => payload_sha256,
            Self::Resource { sha256, .. } => sha256,
        }
    }

    #[must_use]
    pub const fn payload_byte_length(&self) -> u64 {
        match self {
            Self::Node {
                payload_byte_length,
                ..
            } => *payload_byte_length,
            Self::Resource { byte_length, .. } => *byte_length,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrashItem {
    pub manifest: TrashItemManifest,
    pub item_path: PathBuf,
    pub payload_path: PathBuf,
    /// Every node identity carried by a node item, paired with its path relative
    /// to the item payload root. Resource items leave this empty.
    pub node_locators: BTreeMap<NodeId, String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashPlanDisposition {
    Stored,
    Restored,
    PermanentlyDeleted,
    Migrated,
}

/// Path-free reviewed evidence exposed by a structural Trash plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceTrashPlanItemChange {
    pub disposition: TrashPlanDisposition,
    pub manifest: TrashItemManifest,
    pub destination_node_id: Option<NodeId>,
    pub destination_name: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashOriginResolution {
    Active,
    InTrash,
    Missing,
    Unknown,
    ReconciliationRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashRestoreBlockedReason {
    OriginUnknown,
    OriginMissing,
    NameConflict,
    CaseFoldConflict,
    AncestorCycle,
    AncestorAmbiguous,
    ReconciliationRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrashItemRestoreAvailability {
    pub origin_resolution: TrashOriginResolution,
    pub original_available: bool,
    pub with_ancestors_available: bool,
    pub required_ancestor_item_ids: Vec<TrashItemId>,
    pub blocked_reason: Option<TrashRestoreBlockedReason>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceTrashItemProjection {
    pub manifest: TrashItemManifest,
    pub contained_node_ids: Vec<NodeId>,
    pub restore: TrashItemRestoreAvailability,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTrashState {
    Ready,
    LegacyMigrationRequired,
    ReconciliationRequired,
}

/// Path-free read-only state that remains available when Trash-only diagnostics freeze mutation.
/// `items` is populated only when the complete item store is trusted.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceTrashStateProjection {
    pub state: WorkspaceTrashState,
    pub items: Vec<WorkspaceTrashItemProjection>,
    pub legacy_migration_required: bool,
    pub reconciliation_required: bool,
    pub diagnostic_count: u64,
}

/// Path-free authority written beside an exact external legacy-Trash snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LegacyTrashMigrationBackupAuthority {
    pub schema: String,
    pub backup_id: TrashReviewId,
    pub workspace_root_sha256: String,
    pub base_revision: WorkspaceRevision,
    pub trash_tree_sha256: String,
    pub physical_entries: u64,
    pub physical_bytes: u64,
    pub authority_digest: String,
}

/// Opaque local proof that Core created and verified a disjoint exact snapshot before migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyTrashMigrationBackup {
    pub(crate) canonical_workspace_root: PathBuf,
    pub(crate) snapshot_directory: PathBuf,
    pub(crate) authority: LegacyTrashMigrationBackupAuthority,
}

impl LegacyTrashMigrationBackup {
    #[must_use]
    pub const fn authority(&self) -> &LegacyTrashMigrationBackupAuthority {
        &self.authority
    }

    #[must_use]
    pub fn snapshot_directory(&self) -> &Path {
        &self.snapshot_directory
    }
}

impl LegacyTrashMigrationBackupAuthority {
    /// Validates every closed field and the self-binding authority digest.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported, malformed, empty, or digest-mismatched authority.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != LEGACY_TRASH_MIGRATION_BACKUP_SCHEMA {
            return Err("unsupported legacy Trash migration backup schema".to_owned());
        }
        validate_sha256_text(&self.workspace_root_sha256, "backup workspace root digest")?;
        validate_sha256_text(&self.trash_tree_sha256, "backup Trash tree digest")?;
        validate_sha256_text(&self.authority_digest, "backup authority digest")?;
        WorkspaceRevision::parse(self.base_revision.as_str()).map_err(|error| error.to_string())?;
        if self.physical_entries == 0 {
            return Err("legacy Trash backup inventory is empty".to_owned());
        }
        if legacy_trash_backup_authority_digest(self)? != self.authority_digest {
            return Err(
                "legacy Trash backup authority digest does not match its fields".to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrashRestoreMode {
    Original,
    WithAncestors,
    ExistingTarget {
        target_node_id: NodeId,
        name: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrashResourceSelection {
    pub owner_node_id: NodeId,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrashPermanentDeleteItemPreview {
    pub trash_item_id: TrashItemId,
    pub kind: TrashItemKind,
    pub original_name: String,
    pub payload_sha256: String,
    pub payload_byte_length: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrashPermanentDeletePreview {
    pub base_revision: WorkspaceRevision,
    pub items: Vec<TrashPermanentDeleteItemPreview>,
    pub total_payload_bytes: u64,
}

/// Caller-visible selector and generated entropy required to reproduce one exact Trash preview.
/// Core owns construction and independently validates every field during replan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrashReviewedAction {
    StoreNode {
        #[serde(rename = "nodeId")]
        node_id: NodeId,
        #[serde(rename = "trashedAt")]
        trashed_at: String,
        #[serde(rename = "trashItemId")]
        trash_item_id: TrashItemId,
        #[serde(rename = "operationId")]
        operation_id: TrashOperationId,
        #[serde(rename = "trashNodeId")]
        trash_node_id: Option<NodeId>,
    },
    StoreResources {
        resources: Vec<TrashResourceSelection>,
        #[serde(rename = "trashedAt")]
        trashed_at: String,
        #[serde(rename = "trashItemIds")]
        trash_item_ids: Vec<TrashItemId>,
        #[serde(rename = "operationId")]
        operation_id: TrashOperationId,
        #[serde(rename = "trashNodeId")]
        trash_node_id: Option<NodeId>,
    },
    Restore {
        #[serde(rename = "trashItemId")]
        trash_item_id: TrashItemId,
        mode: TrashRestoreMode,
    },
    MigrateLegacy {
        #[serde(rename = "trashedAt")]
        trashed_at: String,
        #[serde(rename = "trashItemIds")]
        trash_item_ids: Vec<TrashItemId>,
        #[serde(rename = "operationId")]
        operation_id: TrashOperationId,
        backup: LegacyTrashMigrationBackupAuthority,
    },
    PermanentDelete {
        preview: TrashPermanentDeletePreview,
    },
}

/// Closed, path-free artifact that lets another process replan the exact reviewed Trash request.
/// It deliberately carries no executable journal steps or payload bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrashReviewedRequest {
    pub schema: String,
    pub review_id: TrashReviewId,
    pub workspace_root_sha256: String,
    pub base_revision: WorkspaceRevision,
    pub action: TrashReviewedAction,
    pub path_changes: Vec<crate::workspace_transaction::WorkspacePathChange>,
    pub generated_node_ids: Vec<NodeId>,
    pub scope_summary: Option<crate::workspace_transaction::WorkspaceTransactionScopeSummary>,
    pub identity_map: Vec<crate::workspace_transaction::WorkspaceIdentityMapEntry>,
    pub captured_target: Option<crate::workspace_transaction::WorkspaceCapturedTarget>,
    pub target_node_ids: Vec<NodeId>,
    pub draft_sensitive_node_ids: Vec<NodeId>,
    pub trash_item_changes: Vec<WorkspaceTrashPlanItemChange>,
    pub authority_digest: String,
}

impl TrashReviewedRequest {
    /// Parses one bounded, closed request and rejects duplicate JSON keys or changed authority.
    ///
    /// # Errors
    ///
    /// Returns an error for oversized, malformed, duplicate-key, or tampered input.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_TRASH_REVIEWED_REQUEST_BYTES {
            return Err("Trash reviewed request exceeds the 16 MiB safety ceiling".to_owned());
        }
        crate::workspace_transaction::reject_duplicate_json_keys(bytes)
            .map_err(|error| error.to_string())?;
        let request: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        request.validate()?;
        Ok(request)
    }

    /// Returns Core's canonical compact JSON plus one trailing newline.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails closed validation.
    pub fn to_canonical_json_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let mut bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Validates the artifact independently of any current workspace state.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid bounds, inconsistent previews, or changed authority.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != TRASH_REVIEWED_REQUEST_SCHEMA {
            return Err("unsupported Trash reviewed request schema".to_owned());
        }
        validate_sha256_text(&self.workspace_root_sha256, "workspace root digest")?;
        validate_sha256_text(&self.authority_digest, "reviewed request authority digest")?;
        WorkspaceRevision::parse(self.base_revision.as_str()).map_err(|error| error.to_string())?;
        if self.path_changes.len() > 100_000
            || self.generated_node_ids.len() > 1
            || self.target_node_ids.len() > 100_000
            || self.draft_sensitive_node_ids.len() > 100_000
            || !self.identity_map.is_empty()
            || self.trash_item_changes.is_empty()
            || self.trash_item_changes.len() > 10_000
        {
            return Err("Trash reviewed request exceeds its structural bounds".to_owned());
        }
        if self
            .path_changes
            .iter()
            .any(|change| change.new_path.is_empty() || change.new_path.len() > 4_096)
        {
            return Err("Trash reviewed request contains an invalid path preview".to_owned());
        }
        let generated = self
            .generated_node_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if generated.len() != self.generated_node_ids.len() {
            return Err("Trash reviewed request repeats a generated node ID".to_owned());
        }
        let target_node_ids = self
            .target_node_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let draft_sensitive_node_ids = self
            .draft_sensitive_node_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if target_node_ids.iter().copied().collect::<Vec<_>>() != self.target_node_ids
            || draft_sensitive_node_ids.iter().copied().collect::<Vec<_>>()
                != self.draft_sensitive_node_ids
        {
            return Err("Trash reviewed request identity sets are not canonical".to_owned());
        }
        if let Some(summary) = &self.scope_summary {
            crate::workspace_transaction::validate_scope_summary(summary)
                .map_err(|error| error.to_string())?;
        }
        for change in &self.trash_item_changes {
            change.manifest.to_canonical_json_bytes()?;
        }
        validate_reviewed_action(self)?;
        if reviewed_request_authority_digest(self)? != self.authority_digest {
            return Err(
                "Trash reviewed request authority digest does not match its fields".to_owned(),
            );
        }
        Ok(())
    }
}

/// Re-authorization supplied when replaying a reviewed request. Permanent deletion must cross the
/// higher-permission boundary and repeat the exact phrase; all other actions use `Ordinary`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrashReviewedReplanAuthorization {
    Ordinary,
    LegacyMigration {
        backup: LegacyTrashMigrationBackup,
    },
    PermanentDelete {
        higher_permission_granted: bool,
        exact_phrase: String,
    },
}

pub(crate) fn build_trash_reviewed_request(
    root: &Path,
    plan: &crate::workspace_transaction::WorkspaceTransactionPlan,
    action: TrashReviewedAction,
) -> Result<TrashReviewedRequest, String> {
    let review_id = plan
        .plan_id
        .parse::<TrashReviewId>()
        .map_err(|error| error.to_string())?;
    let mut request = TrashReviewedRequest {
        schema: TRASH_REVIEWED_REQUEST_SCHEMA.to_owned(),
        review_id,
        workspace_root_sha256: trash_reviewed_workspace_root_digest(root)
            .map_err(|error| error.to_string())?,
        base_revision: plan.base_revision.clone(),
        action,
        path_changes: plan.path_changes.clone(),
        generated_node_ids: plan.generated_node_ids.clone(),
        scope_summary: plan.scope_summary.clone(),
        identity_map: plan.identity_map.clone(),
        captured_target: plan.captured_target.clone(),
        target_node_ids: plan.target_node_ids.clone(),
        draft_sensitive_node_ids: plan.draft_sensitive_node_ids.clone(),
        trash_item_changes: plan.trash_item_changes().to_vec(),
        authority_digest: "0".repeat(64),
    };
    request.authority_digest = reviewed_request_authority_digest(&request)?;
    request.validate()?;
    Ok(request)
}

pub(crate) fn trash_reviewed_workspace_root_digest(root: &Path) -> std::io::Result<String> {
    let canonical = fs::canonicalize(root)?;
    let locator = canonical.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    let locator = locator.to_ascii_lowercase();
    let mut digest = Sha256::new();
    digest.update(b"weftext.trash-reviewed-workspace-root/v1\n");
    digest.update(
        u64::try_from(locator.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    digest.update(locator.as_bytes());
    Ok(format!("{:x}", digest.finalize()))
}

fn reviewed_request_authority_digest(request: &TrashReviewedRequest) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(
        "weftext.trash-reviewed-request-authority/v1",
        &request.schema,
        request.review_id,
        &request.workspace_root_sha256,
        &request.base_revision,
        &request.action,
        &request.path_changes,
        &request.generated_node_ids,
        &request.scope_summary,
        &request.identity_map,
        &request.captured_target,
        &request.target_node_ids,
        &request.draft_sensitive_node_ids,
        &request.trash_item_changes,
    ))
    .map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[allow(clippy::too_many_lines)]
fn validate_reviewed_action(request: &TrashReviewedRequest) -> Result<(), String> {
    match &request.action {
        TrashReviewedAction::StoreNode {
            node_id,
            trashed_at,
            trash_item_id,
            operation_id,
            trash_node_id,
        } => {
            let expected_generated = trash_node_id.iter().copied().collect::<Vec<_>>();
            if request.generated_node_ids != expected_generated
                || request.trash_item_changes.len() != 1
            {
                return Err("node Trash review has inconsistent generated authority".to_owned());
            }
            let change = &request.trash_item_changes[0];
            if change.disposition != TrashPlanDisposition::Stored
                || change.manifest.kind() != TrashItemKind::Node
                || change.manifest.node_id() != Some(*node_id)
                || change.manifest.trash_item_id() != *trash_item_id
                || change.manifest.operation_id() != *operation_id
                || change.manifest.trashed_at() != trashed_at
                || change.destination_node_id.is_some()
                || change.destination_name.is_some()
            {
                return Err("node Trash review differs from its manifest preview".to_owned());
            }
        }
        TrashReviewedAction::StoreResources {
            resources,
            trashed_at,
            trash_item_ids,
            operation_id,
            trash_node_id,
        } => {
            let expected_generated = trash_node_id.iter().copied().collect::<Vec<_>>();
            if resources.is_empty()
                || resources.len() != trash_item_ids.len()
                || resources.len() != request.trash_item_changes.len()
                || request.generated_node_ids != expected_generated
            {
                return Err("resource Trash review has inconsistent batch authority".to_owned());
            }
            for ((selection, item_id), change) in resources
                .iter()
                .zip(trash_item_ids)
                .zip(&request.trash_item_changes)
            {
                if change.disposition != TrashPlanDisposition::Stored
                    || change.manifest.kind() != TrashItemKind::Resource
                    || change.manifest.trash_item_id() != *item_id
                    || change.manifest.operation_id() != *operation_id
                    || change.manifest.trashed_at() != trashed_at
                    || change.manifest.original_owner_node_id() != Some(selection.owner_node_id)
                    || change.manifest.original_name() != selection.name
                    || change.destination_node_id.is_some()
                    || change.destination_name.is_some()
                {
                    return Err(
                        "resource Trash review differs from its batch manifest preview".to_owned(),
                    );
                }
            }
        }
        TrashReviewedAction::Restore { trash_item_id, .. } => {
            if !request.generated_node_ids.is_empty()
                || request
                    .trash_item_changes
                    .iter()
                    .any(|change| change.disposition != TrashPlanDisposition::Restored)
                || request
                    .trash_item_changes
                    .last()
                    .is_none_or(|change| change.manifest.trash_item_id() != *trash_item_id)
            {
                return Err("restore review differs from its item preview".to_owned());
            }
        }
        TrashReviewedAction::MigrateLegacy {
            trashed_at,
            trash_item_ids,
            operation_id,
            backup,
        } => {
            backup.validate()?;
            if trash_item_ids.is_empty()
                || !request.generated_node_ids.is_empty()
                || trash_item_ids.len() != request.trash_item_changes.len()
                || backup.base_revision != request.base_revision
                || backup.workspace_root_sha256 != request.workspace_root_sha256
            {
                return Err("legacy Trash migration review has inconsistent authority".to_owned());
            }
            for (item_id, change) in trash_item_ids.iter().zip(&request.trash_item_changes) {
                if change.disposition != TrashPlanDisposition::Migrated
                    || change.manifest.trash_item_id() != *item_id
                    || change.manifest.operation_id() != *operation_id
                    || change.manifest.trashed_at() != trashed_at
                    || change.manifest.origin_status() != TrashOriginStatus::Unknown
                {
                    return Err(
                        "legacy Trash migration review differs from its manifest preview"
                            .to_owned(),
                    );
                }
            }
        }
        TrashReviewedAction::PermanentDelete { preview } => {
            if preview.base_revision != request.base_revision
                || !request.generated_node_ids.is_empty()
                || !request.path_changes.is_empty()
                || preview.items.len() != request.trash_item_changes.len()
            {
                return Err("permanent deletion review has inconsistent authority".to_owned());
            }
            for (item, change) in preview.items.iter().zip(&request.trash_item_changes) {
                if change.disposition != TrashPlanDisposition::PermanentlyDeleted
                    || change.manifest.trash_item_id() != item.trash_item_id
                    || change.manifest.kind() != item.kind
                    || change.manifest.original_name() != item.original_name
                    || change.manifest.payload_sha256() != item.payload_sha256
                    || change.manifest.payload_byte_length() != item.payload_byte_length
                {
                    return Err(
                        "permanent deletion review differs from its item preview".to_owned()
                    );
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn legacy_trash_backup_authority_digest(
    authority: &LegacyTrashMigrationBackupAuthority,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(
        "weftext.legacy-trash-migration-backup-authority/v1",
        &authority.schema,
        authority.backup_id,
        &authority.workspace_root_sha256,
        &authority.base_revision,
        &authority.trash_tree_sha256,
        authority.physical_entries,
        authority.physical_bytes,
    ))
    .map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_sha256_text(value: &str, label: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!("{label} is not canonical lowercase SHA-256"))
    }
}

#[derive(Clone, Debug)]
pub struct TrashPermanentDeleteConfirmation {
    pub(crate) preview: TrashPermanentDeletePreview,
    pub(crate) higher_permission_granted: bool,
    pub(crate) authority_digest: String,
}

impl TrashPermanentDeleteConfirmation {
    #[must_use]
    pub const fn preview(&self) -> &TrashPermanentDeletePreview {
        &self.preview
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrashStoreIssue {
    pub(crate) path: PathBuf,
    pub(crate) message: String,
    pub(crate) duplicate_identity: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TrashStoreInspection {
    pub(crate) items: Vec<WorkspaceTrashItem>,
    pub(crate) issues: Vec<TrashStoreIssue>,
    pub(crate) legacy_format: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrashPayloadSummary {
    pub(crate) sha256: String,
    pub(crate) byte_length: u64,
    pub(crate) entry_count: u64,
}

struct TrashPayloadRecord {
    locator: Vec<u8>,
    file_length: u64,
    file_sha256: [u8; 32],
    is_file: bool,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn inspect_workspace_trash_store(
    root: &Path,
    rules: &ContentRules,
    active_node_ids: &BTreeMap<NodeId, PathBuf>,
) -> TrashStoreInspection {
    let trash = root.join(TRASH_DIRECTORY_NAME);
    let metadata = match fs::symlink_metadata(&trash) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return TrashStoreInspection::default();
        }
        Err(error) => {
            return inspection_issue(&trash, format!("cannot inspect Workspace Trash: {error}"));
        }
    };
    if linked_or_reparse(&metadata) || !metadata.is_dir() {
        return inspection_issue(
            &trash,
            "Workspace Trash must be a regular non-link directory".to_owned(),
        );
    }
    if rules.classify(TRASH_DIRECTORY_NAME, true).is_some() {
        return inspection_issue(
            &trash,
            "content rules cannot classify the Core-reserved Workspace Trash store".to_owned(),
        );
    }

    let canonical_document_name = format!("{TRASH_DIRECTORY_NAME}.adoc");
    let canonical_document = trash.join(&canonical_document_name);
    if !canonical_document.is_file() {
        return inspection_issue(
            &canonical_document,
            "Workspace Trash is missing its canonical .weftext-trash.adoc document".to_owned(),
        );
    }
    if let Err(issue) = reject_rule_classification(root, rules, &canonical_document, false) {
        return TrashStoreInspection {
            issues: vec![issue],
            ..TrashStoreInspection::default()
        };
    }
    let items_root = trash.join(TRASH_ITEMS_DIRECTORY_NAME);
    let mut inspection = TrashStoreInspection::default();
    let entries = match sorted_entries(&trash) {
        Ok(entries) => entries,
        Err(error) => {
            inspection.issues.push(store_issue(&trash, error));
            return inspection;
        }
    };
    let has_items_store = items_root.is_dir();
    let mut direct_legacy_entries = Vec::new();
    for entry in entries {
        let path = entry.path();
        let Ok(name) = entry.file_name().into_string() else {
            inspection.issues.push(store_issue(
                &path,
                "Workspace Trash entry name is not UTF-8".to_owned(),
            ));
            continue;
        };
        if name != TRASH_ITEMS_DIRECTORY_NAME && name != canonical_document_name {
            direct_legacy_entries.push(path);
        }
    }
    if !direct_legacy_entries.is_empty() {
        inspection.legacy_format = !has_items_store;
        let message = if has_items_store {
            "Workspace Trash mixes legacy direct entries with _weftext.items authority"
        } else {
            "legacy direct-entry Trash requires explicit migration to _weftext.items"
        };
        inspection.issues.extend(
            direct_legacy_entries
                .into_iter()
                .map(|path| store_issue(&path, message.to_owned())),
        );
        return inspection;
    }
    let items_metadata = match fs::symlink_metadata(&items_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // An empty historical Trash node is harmless. The first deletion
            // creates the item store and its first item in one transaction.
            return inspection;
        }
        Err(error) => {
            inspection.issues.push(store_issue(
                &items_root,
                format!("cannot inspect Trash item store: {error}"),
            ));
            return inspection;
        }
    };
    if linked_or_reparse(&items_metadata) || !items_metadata.is_dir() {
        inspection.issues.push(store_issue(
            &items_root,
            "_weftext.items must be a regular non-link directory".to_owned(),
        ));
        return inspection;
    }
    if rules
        .classify(
            &format!("{TRASH_DIRECTORY_NAME}/{TRASH_ITEMS_DIRECTORY_NAME}"),
            true,
        )
        .is_some()
    {
        inspection.issues.push(store_issue(
            &items_root,
            "content rules cannot classify the Core-reserved Trash item store".to_owned(),
        ));
    }

    let mut seen_item_ids = BTreeSet::new();
    let mut seen_node_ids = active_node_ids.clone();
    let entries = match sorted_entries(&items_root) {
        Ok(entries) => entries,
        Err(error) => {
            inspection.issues.push(store_issue(&items_root, error));
            return inspection;
        }
    };
    for entry in entries {
        let mut candidate_seen_node_ids = seen_node_ids.clone();
        match inspect_item(root, rules, &entry.path(), &mut candidate_seen_node_ids) {
            Ok(item) => {
                if seen_item_ids.insert(item.manifest.trash_item_id()) {
                    seen_node_ids = candidate_seen_node_ids;
                    inspection.items.push(item);
                } else {
                    inspection.issues.push(TrashStoreIssue {
                        path: entry.path(),
                        message: "duplicate Trash item ID".to_owned(),
                        duplicate_identity: true,
                    });
                }
            }
            Err(issue) => inspection.issues.push(issue),
        }
    }
    validate_resource_item_names(&inspection.items, &seen_node_ids, &mut inspection.issues);
    inspection
        .items
        .sort_by_key(|item| item.manifest.trash_item_id());
    inspection
}

#[allow(clippy::too_many_lines)]
fn inspect_item(
    root: &Path,
    rules: &ContentRules,
    item_path: &Path,
    seen_node_ids: &mut BTreeMap<NodeId, PathBuf>,
) -> Result<WorkspaceTrashItem, TrashStoreIssue> {
    let metadata = fs::symlink_metadata(item_path)
        .map_err(|error| store_issue(item_path, format!("cannot inspect Trash item: {error}")))?;
    if linked_or_reparse(&metadata) || !metadata.is_dir() {
        return Err(store_issue(
            item_path,
            "Trash item must be a regular non-link directory".to_owned(),
        ));
    }
    let directory_name = item_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| store_issue(item_path, "Trash item name is not UTF-8".to_owned()))?;
    let directory_id = directory_name
        .parse::<TrashItemId>()
        .map_err(|error| store_issue(item_path, error.to_string()))?;
    reject_rule_classification(root, rules, item_path, true)?;

    let entries = sorted_entries(item_path).map_err(|message| store_issue(item_path, message))?;
    let mut names = BTreeSet::new();
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| store_issue(&entry.path(), "Trash item entry is not UTF-8".to_owned()))?;
        names.insert(name);
    }
    let expected = BTreeSet::from([
        TRASH_ITEM_MANIFEST_FILE_NAME.to_owned(),
        TRASH_ITEM_PAYLOAD_DIRECTORY_NAME.to_owned(),
    ]);
    if names != expected {
        return Err(store_issue(
            item_path,
            "Trash item must contain exactly its manifest and payload directory".to_owned(),
        ));
    }

    let manifest_path = item_path.join(TRASH_ITEM_MANIFEST_FILE_NAME);
    reject_rule_classification(root, rules, &manifest_path, false)?;
    let manifest = read_manifest(&manifest_path)?;
    if manifest.trash_item_id() != directory_id {
        return Err(store_issue(
            &manifest_path,
            "Trash item directory ID differs from manifest trashItemId".to_owned(),
        ));
    }
    validate_manifest(&manifest, &manifest_path)?;

    let payload = item_path.join(TRASH_ITEM_PAYLOAD_DIRECTORY_NAME);
    reject_rule_classification_tree(root, rules, &payload)?;
    let payload_metadata = fs::symlink_metadata(&payload)
        .map_err(|error| store_issue(&payload, format!("cannot inspect item payload: {error}")))?;
    if linked_or_reparse(&payload_metadata) || !payload_metadata.is_dir() {
        return Err(store_issue(
            &payload,
            "Trash item payload must be a regular non-link directory".to_owned(),
        ));
    }
    let payload_entries =
        sorted_entries(&payload).map_err(|message| store_issue(&payload, message))?;
    if payload_entries.len() != 1 {
        return Err(store_issue(
            &payload,
            "Trash item payload must contain exactly one original-name entry".to_owned(),
        ));
    }
    let carried_path = payload_entries[0].path();
    let carried_name = carried_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| store_issue(&carried_path, "payload entry name is not UTF-8".to_owned()))?;
    if carried_name != manifest.original_name() {
        return Err(store_issue(
            &carried_path,
            "payload entry name differs from manifest originalName".to_owned(),
        ));
    }

    let mut node_locators = BTreeMap::new();
    match &manifest {
        TrashItemManifest::Node {
            node_id,
            payload_sha256,
            payload_byte_length,
            payload_entry_count,
            ..
        } => {
            let metadata = fs::symlink_metadata(&carried_path).map_err(|error| {
                store_issue(
                    &carried_path,
                    format!("cannot inspect node payload: {error}"),
                )
            })?;
            if linked_or_reparse(&metadata) || !metadata.is_dir() {
                return Err(store_issue(
                    &carried_path,
                    "node Trash item payload must be a directory".to_owned(),
                ));
            }
            let summary = trash_payload_summary(&carried_path)?;
            if &summary.sha256 != payload_sha256
                || summary.byte_length != *payload_byte_length
                || summary.entry_count != *payload_entry_count
            {
                return Err(store_issue(
                    &carried_path,
                    "node Trash payload differs from its manifest digest, size, or entry count"
                        .to_owned(),
                ));
            }
            inspect_node_payload(
                &carried_path,
                &carried_path,
                &mut node_locators,
                seen_node_ids,
            )?;
            if node_locators.get(node_id).map(String::as_str) != Some(manifest.original_name()) {
                return Err(store_issue(
                    &carried_path,
                    "node Trash payload root identity differs from manifest nodeId".to_owned(),
                ));
            }
        }
        TrashItemManifest::Resource {
            sha256,
            byte_length,
            ..
        } => {
            validate_resource_original_name(manifest.original_name(), &manifest_path)?;
            let metadata = fs::symlink_metadata(&carried_path).map_err(|error| {
                store_issue(
                    &carried_path,
                    format!("cannot inspect resource payload: {error}"),
                )
            })?;
            if linked_or_reparse(&metadata) || !metadata.is_file() {
                return Err(store_issue(
                    &carried_path,
                    "resource Trash item payload must be a regular file".to_owned(),
                ));
            }
            let bytes = read_bounded_file(&carried_path, MAX_TRASH_PAYLOAD_BYTES)?;
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != *byte_length
                || format!("{:x}", Sha256::digest(&bytes)) != *sha256
            {
                return Err(store_issue(
                    &carried_path,
                    "resource Trash payload differs from its manifest digest or size".to_owned(),
                ));
            }
        }
    }

    Ok(WorkspaceTrashItem {
        manifest,
        item_path: item_path.to_path_buf(),
        payload_path: carried_path,
        node_locators,
    })
}

fn validate_resource_original_name(name: &str, path: &Path) -> Result<(), TrashStoreIssue> {
    let folded = name.to_ascii_lowercase();
    if folded == ".git"
        || folded == crate::WORKSPACE_FORMAT_MARKER_FILE
        || folded == crate::content_boundary::CONTENT_RULES_FILE_NAME
        || folded == crate::ANNOTATIONS_FILE_NAME
        || folded == TRASH_ITEM_MANIFEST_FILE_NAME
        || folded == TRASH_ITEMS_DIRECTORY_NAME
        || folded.starts_with(".__weftext-transaction-")
        || folded.starts_with(".__weftext-resource-")
    {
        Err(store_issue(
            path,
            "resource Trash item claims a reserved source filename".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn validate_resource_item_names(
    items: &[WorkspaceTrashItem],
    node_paths: &BTreeMap<NodeId, PathBuf>,
    issues: &mut Vec<TrashStoreIssue>,
) {
    for item in items {
        let TrashItemManifest::Resource {
            original_owner_node_id: Some(owner_id),
            original_name,
            ..
        } = &item.manifest
        else {
            continue;
        };
        let Some(owner_path) = node_paths.get(owner_id) else {
            continue;
        };
        let Some(owner_name) = owner_path.file_name().and_then(|name| name.to_str()) else {
            issues.push(store_issue(
                &item.item_path,
                "resource Trash owner path is not UTF-8".to_owned(),
            ));
            continue;
        };
        if original_name.eq_ignore_ascii_case(&format!("{owner_name}.adoc")) {
            issues.push(store_issue(
                &item.item_path,
                "resource Trash item cannot carry a managed node's canonical document".to_owned(),
            ));
        }
    }
}

fn inspect_node_payload(
    payload_root: &Path,
    directory: &Path,
    node_locators: &mut BTreeMap<NodeId, String>,
    seen_node_ids: &mut BTreeMap<NodeId, PathBuf>,
) -> Result<(), TrashStoreIssue> {
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| store_issue(directory, "node payload name is not UTF-8".to_owned()))?;
    crate::workspace::validate_node_name(name, false)
        .map_err(|error| store_issue(directory, error.to_string()))?;
    let document = directory.join(format!("{name}.adoc"));
    let snapshot = read_node_document(directory).map_err(|error| {
        store_issue(&document, format!("invalid node payload document: {error}"))
    })?;
    let metadata = parse_node_metadata(&snapshot.source).map_err(|error| {
        store_issue(&document, format!("invalid node payload metadata: {error}"))
    })?;
    let id = metadata
        .id
        .ok_or_else(|| store_issue(&document, "node payload is missing weftext.id".to_owned()))?;
    let relative = directory
        .strip_prefix(payload_root.parent().unwrap_or(payload_root))
        .map_err(|_| store_issue(directory, "node payload path escaped its item".to_owned()))?;
    let locator = relative
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| store_issue(directory, "node payload path is not UTF-8".to_owned()))?
        .join("/");
    if let Some(first) = seen_node_ids.insert(id, directory.to_path_buf()) {
        return Err(TrashStoreIssue {
            path: directory.to_path_buf(),
            message: format!("node ID is already used by {}", first.display()),
            duplicate_identity: true,
        });
    }
    node_locators.insert(id, locator);

    for entry in sorted_entries(directory).map_err(|message| store_issue(directory, message))? {
        let path = entry.path();
        if path == document {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            store_issue(&path, format!("cannot inspect node payload entry: {error}"))
        })?;
        if linked_or_reparse(&metadata) {
            return Err(store_issue(
                &path,
                "Trash payload cannot contain links or reparse points".to_owned(),
            ));
        }
        if metadata.is_dir() {
            inspect_node_payload(payload_root, &path, node_locators, seen_node_ids)?;
        } else if !metadata.is_file() {
            return Err(store_issue(
                &path,
                "Trash payload contains an unsupported filesystem entry".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn trash_payload_summary(path: &Path) -> Result<TrashPayloadSummary, TrashStoreIssue> {
    let mut records = Vec::new();
    let mut byte_length = 0_u64;
    collect_payload_records(path, path, &mut records, &mut byte_length)?;
    records.sort_by(|left, right| left.locator.cmp(&right.locator));
    if records.len() > MAX_TRASH_PAYLOAD_ENTRIES {
        return Err(store_issue(
            path,
            format!("Trash payload exceeds {MAX_TRASH_PAYLOAD_ENTRIES} entries"),
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.trash-payload-tree/v1\n");
    for record in &records {
        hasher.update([if record.is_file { b'F' } else { b'D' }]);
        hasher.update(
            u64::try_from(record.locator.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        hasher.update(&record.locator);
        hasher.update(record.file_length.to_be_bytes());
        hasher.update(record.file_sha256);
    }
    Ok(TrashPayloadSummary {
        sha256: format!("{:x}", hasher.finalize()),
        byte_length,
        entry_count: u64::try_from(records.len()).unwrap_or(u64::MAX),
    })
}

pub(crate) fn inspect_legacy_node_payload(
    path: &Path,
    seen_node_ids: &mut BTreeMap<NodeId, PathBuf>,
) -> Result<(NodeId, TrashPayloadSummary), TrashStoreIssue> {
    let mut node_locators = BTreeMap::new();
    inspect_node_payload(path, path, &mut node_locators, seen_node_ids)?;
    let root_id = node_locators
        .iter()
        .find_map(|(id, locator)| (locator == path.file_name()?.to_str()?).then_some(*id))
        .ok_or_else(|| {
            store_issue(
                path,
                "legacy Trash node payload has no canonical root identity".to_owned(),
            )
        })?;
    Ok((root_id, trash_payload_summary(path)?))
}

fn collect_payload_records(
    root: &Path,
    current: &Path,
    records: &mut Vec<TrashPayloadRecord>,
    byte_length: &mut u64,
) -> Result<(), TrashStoreIssue> {
    let metadata = fs::symlink_metadata(current)
        .map_err(|error| store_issue(current, format!("cannot inspect Trash payload: {error}")))?;
    if linked_or_reparse(&metadata) {
        return Err(store_issue(
            current,
            "Trash payload cannot contain links or reparse points".to_owned(),
        ));
    }
    let relative = current
        .strip_prefix(root.parent().unwrap_or(root))
        .map_err(|_| store_issue(current, "Trash payload path escaped its root".to_owned()))?;
    let locator = relative
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| store_issue(current, "Trash payload path is not UTF-8".to_owned()))?
        .join("/");
    if metadata.is_dir() {
        records.push(TrashPayloadRecord {
            locator: locator.into_bytes(),
            file_length: 0,
            file_sha256: [0; 32],
            is_file: false,
        });
        for entry in sorted_entries(current).map_err(|message| store_issue(current, message))? {
            collect_payload_records(root, &entry.path(), records, byte_length)?;
        }
    } else if metadata.is_file() {
        let bytes = read_bounded_file(current, MAX_TRASH_PAYLOAD_BYTES)?;
        *byte_length = byte_length
            .checked_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                store_issue(current, "Trash payload byte count overflowed".to_owned())
            })?;
        if *byte_length > MAX_TRASH_PAYLOAD_BYTES {
            return Err(store_issue(
                current,
                format!("Trash payload exceeds {MAX_TRASH_PAYLOAD_BYTES} bytes"),
            ));
        }
        records.push(TrashPayloadRecord {
            locator: locator.into_bytes(),
            file_length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            file_sha256: Sha256::digest(&bytes).into(),
            is_file: true,
        });
    } else {
        return Err(store_issue(
            current,
            "Trash payload contains an unsupported filesystem entry".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn manifest_bytes(manifest: &TrashItemManifest) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec(manifest)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn read_manifest(path: &Path) -> Result<TrashItemManifest, TrashStoreIssue> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| store_issue(path, format!("cannot inspect Trash manifest: {error}")))?;
    if linked_or_reparse(&metadata) || !metadata.is_file() {
        return Err(store_issue(
            path,
            "Trash manifest must be a regular non-link file".to_owned(),
        ));
    }
    let bytes = read_bounded_file(path, MAX_TRASH_MANIFEST_BYTES)?;
    crate::workspace_transaction::reject_duplicate_json_keys(&bytes)
        .map_err(|error| store_issue(path, format!("invalid Trash manifest JSON: {error}")))?;
    let manifest: TrashItemManifest = serde_json::from_slice(&bytes)
        .map_err(|error| store_issue(path, format!("invalid Trash manifest JSON: {error}")))?;
    let canonical = manifest_bytes(&manifest).map_err(|error| {
        store_issue(path, format!("cannot canonicalize Trash manifest: {error}"))
    })?;
    if bytes != canonical {
        return Err(store_issue(
            path,
            "Trash manifest bytes are not Core's canonical v1 serialization".to_owned(),
        ));
    }
    Ok(manifest)
}

fn validate_manifest(manifest: &TrashItemManifest, path: &Path) -> Result<(), TrashStoreIssue> {
    let schema = match manifest {
        TrashItemManifest::Node { schema, .. } | TrashItemManifest::Resource { schema, .. } => {
            schema
        }
    };
    if schema != TRASH_ITEM_SCHEMA {
        return Err(store_issue(
            path,
            format!("unsupported Trash item schema: {schema}"),
        ));
    }
    match manifest {
        TrashItemManifest::Node {
            node_id,
            original_parent_node_id,
            ancestor_node_ids,
            origin_status,
            payload_sha256,
            payload_entry_count,
            ..
        } => {
            crate::workspace::validate_node_name(manifest.original_name(), false)
                .map_err(|error| store_issue(path, error.to_string()))?;
            validate_node_id(*node_id, path)?;
            if let Some(parent) = original_parent_node_id {
                validate_node_id(*parent, path)?;
            }
            for ancestor in ancestor_node_ids {
                validate_node_id(*ancestor, path)?;
            }
            validate_digest(payload_sha256, path)?;
            if *payload_entry_count == 0 {
                return Err(store_issue(
                    path,
                    "node Trash manifest has an empty payload entry count".to_owned(),
                ));
            }
            if (*origin_status == TrashOriginStatus::Unknown) != original_parent_node_id.is_none() {
                return Err(store_issue(
                    path,
                    "node Trash originStatus must exactly match originalParentNodeId nullability"
                        .to_owned(),
                ));
            }
            if original_parent_node_id == &Some(*node_id) || ancestor_node_ids.contains(node_id) {
                return Err(store_issue(
                    path,
                    "node Trash origin chain cannot contain its own nodeId".to_owned(),
                ));
            }
            let unique = ancestor_node_ids.iter().copied().collect::<BTreeSet<_>>();
            if unique.len() != ancestor_node_ids.len() {
                return Err(store_issue(
                    path,
                    "node Trash ancestorNodeIds contains duplicates".to_owned(),
                ));
            }
            if let Some(parent) = original_parent_node_id {
                if ancestor_node_ids.last() != Some(parent) {
                    return Err(store_issue(
                        path,
                        "node Trash ancestorNodeIds must end with originalParentNodeId".to_owned(),
                    ));
                }
            } else if !ancestor_node_ids.is_empty() {
                return Err(store_issue(
                    path,
                    "origin-unknown node Trash item cannot claim ancestorNodeIds".to_owned(),
                ));
            }
        }
        TrashItemManifest::Resource {
            original_owner_node_id,
            origin_status,
            sha256,
            ..
        } => {
            crate::workspace::validate_portable_path_component(manifest.original_name(), false)
                .map_err(|error| store_issue(path, error.to_string()))?;
            if let Some(owner) = original_owner_node_id {
                validate_node_id(*owner, path)?;
            }
            validate_digest(sha256, path)?;
            if (*origin_status == TrashOriginStatus::Unknown) != original_owner_node_id.is_none() {
                return Err(store_issue(
                    path,
                    "resource Trash originStatus must exactly match originalOwnerNodeId nullability"
                        .to_owned(),
                ));
            }
        }
    }
    if !valid_explicit_offset_timestamp(manifest.trashed_at()) {
        return Err(store_issue(
            path,
            "Trash manifest trashedAt is not a real explicit-offset RFC 3339 timestamp".to_owned(),
        ));
    }
    Ok(())
}

fn validate_node_id(id: NodeId, path: &Path) -> Result<(), TrashStoreIssue> {
    let text = id.to_string();
    if id.as_uuid().get_version() == Some(Version::Random)
        && text.parse::<NodeId>().is_ok_and(|parsed| parsed == id)
    {
        Ok(())
    } else {
        Err(store_issue(
            path,
            "Trash manifest node identity is not canonical lowercase UUIDv4".to_owned(),
        ))
    }
}

fn valid_explicit_offset_timestamp(value: &str) -> bool {
    if !value.is_ascii() || value.len() < 20 || value.as_bytes().get(10) != Some(&b'T') {
        return false;
    }
    let date = &value[..10];
    if date.as_bytes().get(4) != Some(&b'-') || date.as_bytes().get(7) != Some(&b'-') {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        date[..4].parse::<i32>(),
        date[5..7].parse::<u8>(),
        date[8..10].parse::<u8>(),
    ) else {
        return false;
    };
    if crate::CalendarDate::new(year, month, day).is_err() {
        return false;
    }
    let time = &value[11..];
    if time.len() < 9
        || time.as_bytes().get(2) != Some(&b':')
        || time.as_bytes().get(5) != Some(&b':')
    {
        return false;
    }
    let (Ok(hour), Ok(minute), Ok(second)) = (
        time[..2].parse::<u8>(),
        time[3..5].parse::<u8>(),
        time[6..8].parse::<u8>(),
    ) else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut offset_start = 8;
    if time.as_bytes().get(offset_start) == Some(&b'.') {
        offset_start += 1;
        let fraction_start = offset_start;
        while time
            .as_bytes()
            .get(offset_start)
            .is_some_and(u8::is_ascii_digit)
        {
            offset_start += 1;
        }
        if offset_start == fraction_start {
            return false;
        }
    }
    if time.get(offset_start..) == Some("Z") {
        return true;
    }
    let Some(offset) = time.get(offset_start..) else {
        return false;
    };
    if offset.len() != 6
        || !matches!(offset.as_bytes()[0], b'+' | b'-')
        || offset.as_bytes()[3] != b':'
    {
        return false;
    }
    let (Ok(offset_hour), Ok(offset_minute)) =
        (offset[1..3].parse::<u8>(), offset[4..6].parse::<u8>())
    else {
        return false;
    };
    offset_hour <= 23 && offset_minute <= 59
}

fn validate_digest(value: &str, path: &Path) -> Result<(), TrashStoreIssue> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(store_issue(
            path,
            "Trash manifest digest is not canonical lowercase SHA-256".to_owned(),
        ))
    }
}

fn reject_rule_classification(
    root: &Path,
    rules: &ContentRules,
    path: &Path,
    is_directory: bool,
) -> Result<(), TrashStoreIssue> {
    let relative =
        portable_path(root, path).map_err(|error| store_issue(path, error.to_string()))?;
    if rules.classify(&relative, is_directory).is_some() {
        Err(store_issue(
            path,
            "content rules cannot classify any byte in the Core-reserved Trash store".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn reject_rule_classification_tree(
    root: &Path,
    rules: &ContentRules,
    path: &Path,
) -> Result<(), TrashStoreIssue> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| store_issue(path, format!("cannot inspect Trash authority: {error}")))?;
    if linked_or_reparse(&metadata) {
        return Err(store_issue(
            path,
            "Trash authority cannot contain links or reparse points".to_owned(),
        ));
    }
    reject_rule_classification(root, rules, path, metadata.is_dir())?;
    if metadata.is_dir() {
        for entry in sorted_entries(path).map_err(|message| store_issue(path, message))? {
            reject_rule_classification_tree(root, rules, &entry.path())?;
        }
    } else if !metadata.is_file() {
        return Err(store_issue(
            path,
            "Trash authority contains an unsupported filesystem entry".to_owned(),
        ));
    }
    Ok(())
}

fn read_bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, TrashStoreIssue> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| store_issue(path, format!("cannot inspect file: {error}")))?;
    if linked_or_reparse(&metadata) || !metadata.is_file() || metadata.len() > limit {
        return Err(store_issue(
            path,
            format!("file is not a regular non-link file within {limit} bytes"),
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|error| store_issue(path, format!("cannot open file: {error}")))?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| store_issue(path, format!("cannot read file: {error}")))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(store_issue(path, format!("file grew beyond {limit} bytes")));
    }
    Ok(bytes)
}

fn sorted_entries(path: &Path) -> Result<Vec<fs::DirEntry>, String> {
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("cannot enumerate directory: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate directory: {error}"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

fn inspection_issue(path: &Path, message: String) -> TrashStoreInspection {
    TrashStoreInspection {
        issues: vec![store_issue(path, message)],
        ..TrashStoreInspection::default()
    }
}

fn store_issue(path: &Path, message: String) -> TrashStoreIssue {
    TrashStoreIssue {
        path: path.to_path_buf(),
        message,
        duplicate_identity: false,
    }
}
