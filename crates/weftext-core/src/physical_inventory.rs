//! Fail-closed inventories of the exact Weftext path tree.
//!
//! This module inventories relative UTF-8 paths, regular-file bytes, and empty
//! directories. It intentionally does not claim to preserve filesystem-image
//! metadata such as ACLs, extended attributes, alternate data streams, sparse
//! layout, timestamps, or hard-link topology.
//!
//! External-tree proofs also require the workspace and external directory and
//! regular-file object-identity sets to be disjoint. Unix uses device/inode
//! identity; supported Windows hosts require high-resolution `FILE_ID_INFO`
//! volume serial and 128-bit file ID without a low-resolution fallback. If
//! identity cannot be obtained, the proof fails closed. Hard-link topology
//! within either tree is still outside the inventory contract.
//!
//! A stable capture performs two complete walks and rejects a mismatch. That
//! detects ordinary concurrent filesystem changes, but a hostile local writer
//! capable of an ABA replacement remains outside the cooperative workspace
//! lease threat model until traversal is made handle-relative on every host.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::content_boundary::{linked_or_reparse, reject_linked_existing_ancestors};
use crate::workspace_revision::WORKSPACE_TRANSACTION_PREFIX;
use crate::{
    WORKSPACE_TRANSACTION_LEASE_FILE_NAME, WorkspaceTransactionError, WorkspaceTransactionLease,
};

/// Schema carried by a path-free complete physical-tree binding.
pub const PHYSICAL_TREE_INVENTORY_SCHEMA: &str = "weftext.physical-tree-inventory.v1";
pub const PHYSICAL_ROOT_IDENTITY_SCHEMA: &str = "weftext.physical-root-identity.v1";
/// Maximum number of entries accepted in one complete physical-tree inventory.
pub const PHYSICAL_TREE_MAX_ENTRIES: usize = 1_000_000;
/// Maximum UTF-8 byte length of one relative physical locator.
pub const PHYSICAL_TREE_MAX_LOCATOR_BYTES: usize = 32_768;

const PHYSICAL_TREE_DIGEST_DOMAIN: &[u8] = b"weftext.physical-tree-inventory.v1\0";
const HASH_BUFFER_BYTES: usize = 128 * 1024;

/// Physical entry kind included in an exact path-tree inventory.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalEntryKind {
    /// One directory, including an empty directory.
    Directory,
    /// One regular file and its default-stream bytes.
    RegularFile,
}

/// Canonical UTF-8, `/`-separated path relative to an inventory root.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct PhysicalLocator(String);

impl PhysicalLocator {
    /// Parses one safe non-empty relative locator.
    ///
    /// # Errors
    ///
    /// Rejects absolute/prefixed paths, backslashes, NUL, empty, `.` or `..`
    /// components, and locators above the fixed inventory limit.
    pub fn parse(value: &str) -> Result<Self, PhysicalInventoryError> {
        if value.is_empty()
            || value.len() > PHYSICAL_TREE_MAX_LOCATOR_BYTES
            || value.starts_with('/')
            || value.ends_with('/')
            || value.contains(['\\', '\0'])
        {
            return Err(PhysicalInventoryError::InvalidLocator);
        }
        if value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
        {
            return Err(PhysicalInventoryError::InvalidLocator);
        }
        let path = Path::new(value);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(PhysicalInventoryError::InvalidLocator);
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical relative locator text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PhysicalLocator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PhysicalLocator")
            .field(&self.0)
            .finish()
    }
}

/// Raw SHA-256 value for one regular file.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct PhysicalSha256([u8; 32]);

impl PhysicalSha256 {
    /// Returns canonical lowercase hexadecimal text.
    #[must_use]
    pub fn to_hex(self) -> String {
        hex_sha256(&self.0)
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PhysicalSha256 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

/// One canonical entry in an exact physical-tree inventory.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PhysicalInventoryEntry {
    locator: PhysicalLocator,
    kind: PhysicalEntryKind,
    byte_length: u64,
    sha256: Option<PhysicalSha256>,
}

/// Closed serializable record for journal-bound complete-tree recovery evidence.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PhysicalInventoryRecord {
    pub locator: String,
    pub kind: PhysicalEntryKind,
    pub byte_length: u64,
    pub sha256: Option<String>,
}

impl PhysicalInventoryEntry {
    /// Returns the relative path of this entry.
    #[must_use]
    pub const fn locator(&self) -> &PhysicalLocator {
        &self.locator
    }

    /// Returns whether this entry is a directory or regular file.
    #[must_use]
    pub const fn kind(&self) -> PhysicalEntryKind {
        self.kind
    }

    /// Returns the regular-file byte length, or zero for a directory.
    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Returns the regular-file SHA-256, or `None` for a directory.
    #[must_use]
    pub const fn sha256(&self) -> Option<PhysicalSha256> {
        self.sha256
    }
}

/// Serializable, path-free binding to one complete physical-tree inventory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PhysicalInventoryBinding {
    /// Binding schema.
    pub schema: String,
    /// Aggregate inventory SHA-256.
    pub sha256: String,
    /// Number of directory and regular-file entries.
    pub entry_count: u64,
    /// Checked sum of every regular-file length.
    pub file_bytes: u64,
}

/// Path-free binding to one local filesystem object identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PhysicalRootIdentityBinding {
    pub schema: String,
    pub sha256: String,
}

impl PhysicalRootIdentityBinding {
    pub fn validate(&self) -> Result<(), PhysicalInventoryError> {
        if self.schema == PHYSICAL_ROOT_IDENTITY_SCHEMA && valid_sha256(&self.sha256) {
            Ok(())
        } else {
            Err(PhysicalInventoryError::InvalidBinding)
        }
    }
}

impl PhysicalInventoryBinding {
    /// Validates the closed binding shape.
    ///
    /// This validates transport syntax, not proof that the represented tree
    /// exists. Capture or external-tree verification establishes that proof.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicalInventoryError::InvalidBinding`] for an unknown schema,
    /// malformed digest, or out-of-range entry count.
    pub fn validate(&self) -> Result<(), PhysicalInventoryError> {
        if self.schema != PHYSICAL_TREE_INVENTORY_SCHEMA
            || !valid_sha256(&self.sha256)
            || self.entry_count > PHYSICAL_TREE_MAX_ENTRIES as u64
        {
            return Err(PhysicalInventoryError::InvalidBinding);
        }
        Ok(())
    }
}

/// Opaque canonical inventory. Its `Debug` representation omits entry paths.
#[derive(Clone, Eq, PartialEq)]
pub struct PhysicalTreeInventory {
    entries: Vec<PhysicalInventoryEntry>,
    binding: PhysicalInventoryBinding,
    root_identity: PhysicalRootIdentityBinding,
    directory_identities: BTreeSet<file_id::FileId>,
    file_identities: BTreeSet<file_id::FileId>,
}

impl PhysicalTreeInventory {
    /// Returns the path-free binding suitable for a reviewed plan or receipt.
    #[must_use]
    pub const fn binding(&self) -> &PhysicalInventoryBinding {
        &self.binding
    }

    /// Returns canonically sorted entries for exact copy/verification adapters.
    #[must_use]
    pub fn entries(&self) -> &[PhysicalInventoryEntry] {
        &self.entries
    }

    #[must_use]
    pub fn records(&self) -> Vec<PhysicalInventoryRecord> {
        self.entries.iter().map(physical_entry_record).collect()
    }

    /// Returns a path-free binding to the captured root directory object.
    #[must_use]
    pub const fn root_identity(&self) -> &PhysicalRootIdentityBinding {
        &self.root_identity
    }
}

#[allow(
    dead_code,
    reason = "used by the crate-private pre-release task rebaseline authority"
)]
#[derive(Clone, Debug)]
pub(crate) enum PhysicalInventoryProjectionChange {
    CreateDirectory {
        locator: String,
    },
    CreateRegularFile {
        locator: String,
        bytes: Vec<u8>,
    },
    ReplaceRegularFile {
        locator: String,
        expected_bytes: Vec<u8>,
        next_bytes: Vec<u8>,
    },
}

/// Computes the complete expected post-state binding from one captured tree and a closed set of
/// exact touched-entry changes. This does not touch the filesystem.
#[allow(
    dead_code,
    reason = "used by the crate-private pre-release task rebaseline authority"
)]
pub(crate) fn project_physical_inventory_binding(
    base: &PhysicalTreeInventory,
    changes: &[PhysicalInventoryProjectionChange],
) -> Result<PhysicalInventoryBinding, PhysicalInventoryError> {
    let entries = projected_entries(base, changes)?;
    physical_inventory_binding_from_entries(&entries)
}

#[allow(
    dead_code,
    reason = "used by the crate-private pre-release task rebaseline authority"
)]
pub(crate) fn project_physical_inventory_records(
    base: &PhysicalTreeInventory,
    changes: &[PhysicalInventoryProjectionChange],
) -> Result<Vec<PhysicalInventoryRecord>, PhysicalInventoryError> {
    projected_entries(base, changes).map(|entries| physical_records(&entries))
}

pub(crate) fn project_physical_inventory_records_from_records(
    base: &[PhysicalInventoryRecord],
    changes: &[PhysicalInventoryProjectionChange],
) -> Result<Vec<PhysicalInventoryRecord>, PhysicalInventoryError> {
    let entries = physical_inventory_entries_from_records(base)?;
    projected_entries_from_owned(entries, changes).map(|entries| physical_records(&entries))
}

pub(crate) fn physical_inventory_binding_from_records(
    records: &[PhysicalInventoryRecord],
) -> Result<PhysicalInventoryBinding, PhysicalInventoryError> {
    let entries = physical_inventory_entries_from_records(records)?;
    physical_inventory_binding_from_entries(&entries)
}

fn physical_inventory_entries_from_records(
    records: &[PhysicalInventoryRecord],
) -> Result<Vec<PhysicalInventoryEntry>, PhysicalInventoryError> {
    if records.len() > PHYSICAL_TREE_MAX_ENTRIES {
        return Err(PhysicalInventoryError::EntryLimitExceeded);
    }
    let mut entries = Vec::with_capacity(records.len());
    for record in records {
        let locator = PhysicalLocator::parse(&record.locator)?;
        let (byte_length, sha256) = match (record.kind, record.byte_length, &record.sha256) {
            (PhysicalEntryKind::Directory, 0, None) => (0, None),
            (PhysicalEntryKind::RegularFile, byte_length, Some(sha256)) => {
                (byte_length, Some(parse_physical_sha256(sha256)?))
            }
            _ => return Err(PhysicalInventoryError::InvalidBinding),
        };
        entries.push(PhysicalInventoryEntry {
            locator,
            kind: record.kind,
            byte_length,
            sha256,
        });
    }
    validate_physical_entry_topology(&entries)?;
    Ok(entries)
}

fn projected_entries(
    base: &PhysicalTreeInventory,
    changes: &[PhysicalInventoryProjectionChange],
) -> Result<Vec<PhysicalInventoryEntry>, PhysicalInventoryError> {
    projected_entries_from_owned(base.entries.clone(), changes)
}

fn projected_entries_from_owned(
    base: Vec<PhysicalInventoryEntry>,
    changes: &[PhysicalInventoryProjectionChange],
) -> Result<Vec<PhysicalInventoryEntry>, PhysicalInventoryError> {
    let mut entries = base
        .into_iter()
        .map(|entry| (entry.locator.as_str().to_owned(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut projected_locators = BTreeSet::new();
    for change in changes {
        let locator_text = match change {
            PhysicalInventoryProjectionChange::CreateDirectory { locator }
            | PhysicalInventoryProjectionChange::CreateRegularFile { locator, .. }
            | PhysicalInventoryProjectionChange::ReplaceRegularFile { locator, .. } => locator,
        };
        let locator = PhysicalLocator::parse(locator_text)?;
        if !projected_locators.insert(locator.clone()) {
            return Err(PhysicalInventoryError::InvalidBinding);
        }
        match change {
            PhysicalInventoryProjectionChange::CreateDirectory { .. } => {
                if entries.contains_key(locator.as_str()) {
                    return Err(PhysicalInventoryError::BindingMismatch);
                }
                entries.insert(
                    locator.as_str().to_owned(),
                    PhysicalInventoryEntry {
                        locator,
                        kind: PhysicalEntryKind::Directory,
                        byte_length: 0,
                        sha256: None,
                    },
                );
            }
            PhysicalInventoryProjectionChange::CreateRegularFile { bytes, .. } => {
                if entries.contains_key(locator.as_str()) {
                    return Err(PhysicalInventoryError::BindingMismatch);
                }
                entries.insert(
                    locator.as_str().to_owned(),
                    regular_file_entry(locator, bytes)?,
                );
            }
            PhysicalInventoryProjectionChange::ReplaceRegularFile {
                expected_bytes,
                next_bytes,
                ..
            } => {
                let expected = regular_file_entry(locator.clone(), expected_bytes)?;
                if entries.get(locator.as_str()) != Some(&expected) {
                    return Err(PhysicalInventoryError::BindingMismatch);
                }
                entries.insert(
                    locator.as_str().to_owned(),
                    regular_file_entry(locator, next_bytes)?,
                );
            }
        }
    }
    if entries.len() > PHYSICAL_TREE_MAX_ENTRIES {
        return Err(PhysicalInventoryError::EntryLimitExceeded);
    }
    let entries = entries.into_values().collect::<Vec<_>>();
    validate_physical_entry_topology(&entries)?;
    Ok(entries)
}

fn validate_physical_entry_topology(
    entries: &[PhysicalInventoryEntry],
) -> Result<(), PhysicalInventoryError> {
    if entries
        .windows(2)
        .any(|pair| pair[0].locator >= pair[1].locator)
    {
        return Err(PhysicalInventoryError::InvalidBinding);
    }
    for entry in entries {
        let Some((parent, _)) = entry.locator.as_str().rsplit_once('/') else {
            continue;
        };
        let parent_index = entries
            .binary_search_by(|candidate| candidate.locator.as_str().cmp(parent))
            .map_err(|_| PhysicalInventoryError::InvalidBinding)?;
        if entries[parent_index].kind != PhysicalEntryKind::Directory {
            return Err(PhysicalInventoryError::InvalidBinding);
        }
    }
    Ok(())
}

fn physical_inventory_binding_from_entries(
    entries: &[PhysicalInventoryEntry],
) -> Result<PhysicalInventoryBinding, PhysicalInventoryError> {
    let file_bytes = entries.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.byte_length)
            .ok_or(PhysicalInventoryError::FileByteCountOverflow)
    })?;
    Ok(PhysicalInventoryBinding {
        schema: PHYSICAL_TREE_INVENTORY_SCHEMA.to_owned(),
        sha256: inventory_digest(entries)?,
        entry_count: u64::try_from(entries.len())
            .map_err(|_| PhysicalInventoryError::EntryLimitExceeded)?,
        file_bytes,
    })
}

fn physical_entry_record(entry: &PhysicalInventoryEntry) -> PhysicalInventoryRecord {
    PhysicalInventoryRecord {
        locator: entry.locator.as_str().to_owned(),
        kind: entry.kind,
        byte_length: entry.byte_length,
        sha256: entry.sha256.map(PhysicalSha256::to_hex),
    }
}

fn physical_records(entries: &[PhysicalInventoryEntry]) -> Vec<PhysicalInventoryRecord> {
    entries.iter().map(physical_entry_record).collect()
}

fn parse_physical_sha256(value: &str) -> Result<PhysicalSha256, PhysicalInventoryError> {
    if !valid_sha256(value) {
        return Err(PhysicalInventoryError::InvalidBinding);
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = hex_nibble(pair[0]).ok_or(PhysicalInventoryError::InvalidBinding)?;
        let low = hex_nibble(pair[1]).ok_or(PhysicalInventoryError::InvalidBinding)?;
        bytes[index] = (high << 4) | low;
    }
    Ok(PhysicalSha256(bytes))
}

const fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn regular_file_entry(
    locator: PhysicalLocator,
    bytes: &[u8],
) -> Result<PhysicalInventoryEntry, PhysicalInventoryError> {
    Ok(PhysicalInventoryEntry {
        locator,
        kind: PhysicalEntryKind::RegularFile,
        byte_length: u64::try_from(bytes.len())
            .map_err(|_| PhysicalInventoryError::FileByteCountOverflow)?,
        sha256: Some(PhysicalSha256(Sha256::digest(bytes).into())),
    })
}

impl fmt::Debug for PhysicalTreeInventory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PhysicalTreeInventory")
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}

/// Opaque proof that a disjoint external tree matched one reviewed binding.
#[derive(Clone)]
pub struct VerifiedExternalPhysicalTree {
    canonical_root: PathBuf,
    binding: PhysicalInventoryBinding,
    root_identity: PhysicalRootIdentityBinding,
}

impl VerifiedExternalPhysicalTree {
    /// Returns the path-free physical binding proven by this observation.
    #[must_use]
    pub const fn binding(&self) -> &PhysicalInventoryBinding {
        &self.binding
    }

    #[must_use]
    pub const fn root_identity(&self) -> &PhysicalRootIdentityBinding {
        &self.root_identity
    }

    pub(crate) fn journal_authority_root(&self) -> &Path {
        &self.canonical_root
    }

    /// Reopens the external tree and proves it remains disjoint and byte-exact.
    ///
    /// # Errors
    ///
    /// Returns a physical inventory error when the workspace/root relationship
    /// or any external entry differs from the original proof.
    pub fn revalidate(
        &self,
        workspace_lease: &WorkspaceTransactionLease,
    ) -> Result<(), PhysicalInventoryError> {
        let current =
            capture_disjoint_external_physical_tree(workspace_lease, &self.canonical_root)?;
        if current.binding == self.binding && current.root_identity == self.root_identity {
            Ok(())
        } else {
            Err(PhysicalInventoryError::BindingMismatch)
        }
    }

    pub(crate) fn revalidate_excluding_transaction(
        &self,
        workspace_lease: &WorkspaceTransactionLease,
        transaction: &Path,
        transaction_identity: &PhysicalRootIdentityBinding,
    ) -> Result<(), PhysicalInventoryError> {
        verify_disjoint_external_physical_tree_excluding_transaction(
            workspace_lease,
            &self.canonical_root,
            &self.binding,
            &self.root_identity,
            transaction,
            transaction_identity,
        )
    }
}

impl fmt::Debug for VerifiedExternalPhysicalTree {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedExternalPhysicalTree")
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}

/// Captures a stable complete workspace path tree under Core's exclusive lease.
///
/// `.git`, ignored/unmanaged bytes, Trash, sidecars, empty directories, and
/// zero-length files all participate. The only omitted path is the exact valid
/// root transaction lease anchor. Every other root transaction-shaped entry
/// fails closed; an identically named nested entry remains ordinary payload.
///
/// # Errors
///
/// Returns a typed error for an unsafe root/entry, unreadable content, resource
/// ceiling, unfinished transaction evidence, or concurrent change.
pub fn capture_stable_workspace_physical_inventory(
    lease: &WorkspaceTransactionLease,
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    capture_stable_workspace_physical_inventory_with_final_probe(lease, || {})
}

fn capture_stable_workspace_physical_inventory_with_final_probe(
    lease: &WorkspaceTransactionLease,
    final_probe: impl FnOnce(),
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    require_lease_anchor_identity(lease)?;
    let root = canonical_non_linked_directory(lease.physical_inventory_root())?;
    if root != lease.physical_inventory_root() {
        return Err(PhysicalInventoryError::PathEscape(None));
    }
    let inventory = capture_stable_canonical_root(&root, CaptureMode::Workspace)?;
    final_probe();
    require_lease_anchor_identity(lease)?;
    Ok(inventory)
}

/// Captures a stable exact physical path tree without workspace exclusions.
///
/// # Errors
///
/// Returns a typed error for an unsafe root/entry, unreadable content, resource
/// ceiling, or concurrent change.
pub fn capture_stable_physical_tree(
    root: impl AsRef<Path>,
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    let root = canonical_non_linked_directory(root.as_ref())?;
    capture_stable_canonical_root(&root, CaptureMode::Exact)
}

/// Verifies that an external exact tree is disjoint from the leased workspace
/// and matches one path-free reviewed binding.
///
/// The returned value proves current physical equality only. Backup manifests,
/// durability markers, retention, and restore policy remain outside Core.
///
/// # Errors
///
/// Returns an error for an invalid binding, aliased/nested roots, unsafe tree,
/// concurrent change, or physical mismatch.
pub fn verify_disjoint_external_physical_tree(
    workspace_lease: &WorkspaceTransactionLease,
    external_root: impl AsRef<Path>,
    expected: &PhysicalInventoryBinding,
) -> Result<VerifiedExternalPhysicalTree, PhysicalInventoryError> {
    expected.validate()?;
    let verified = capture_disjoint_external_physical_tree(workspace_lease, external_root)?;
    if verified.binding != *expected {
        return Err(PhysicalInventoryError::BindingMismatch);
    }
    Ok(verified)
}

/// Captures an exact external tree after proving it is path-disjoint from the
/// leased workspace.
///
/// # Errors
///
/// Returns an error for aliased/nested roots, an unsafe tree, resource limits,
/// or concurrent change.
pub fn capture_disjoint_external_physical_tree(
    workspace_lease: &WorkspaceTransactionLease,
    external_root: impl AsRef<Path>,
) -> Result<VerifiedExternalPhysicalTree, PhysicalInventoryError> {
    capture_disjoint_external_physical_tree_with_final_probe(
        workspace_lease,
        external_root.as_ref(),
        || {},
    )
}

fn capture_disjoint_external_physical_tree_with_final_probe(
    workspace_lease: &WorkspaceTransactionLease,
    external_root: &Path,
    final_probe: impl FnOnce(),
) -> Result<VerifiedExternalPhysicalTree, PhysicalInventoryError> {
    let external_root = canonical_non_linked_directory(external_root)?;
    require_disjoint(workspace_lease.physical_inventory_root(), &external_root)?;
    let workspace_inventory = capture_stable_workspace_physical_inventory(workspace_lease)?;
    let inventory = capture_stable_canonical_root(&external_root, CaptureMode::Exact)?;
    require_directory_identity_disjoint(&workspace_inventory, &inventory)?;
    final_probe();
    let final_workspace_inventory = capture_stable_workspace_physical_inventory(workspace_lease)?;
    if final_workspace_inventory != workspace_inventory {
        return Err(PhysicalInventoryError::ConcurrentChange);
    }
    require_lease_anchor_identity(workspace_lease)?;
    Ok(VerifiedExternalPhysicalTree {
        canonical_root: external_root,
        binding: inventory.binding,
        root_identity: inventory.root_identity,
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CaptureMode {
    Exact,
    Workspace,
}

fn capture_stable_canonical_root(
    root: &Path,
    mode: CaptureMode,
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    capture_stable_canonical_root_with_exclusion(root, mode, None)
}

/// Captures the workspace payload while excluding exactly one already-validated current journal
/// directory. Every other transaction-shaped root entry still fails closed.
pub(crate) fn capture_stable_workspace_physical_inventory_excluding_transaction(
    lease: &WorkspaceTransactionLease,
    transaction: &Path,
    transaction_identity: &PhysicalRootIdentityBinding,
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    require_lease_anchor_identity(lease)?;
    transaction_identity.validate()?;
    let root = canonical_non_linked_directory(lease.physical_inventory_root())?;
    let canonical_transaction = canonical_non_linked_directory(transaction)?;
    if canonical_transaction.parent() != Some(root.as_path())
        || canonical_transaction
            .file_name()
            .and_then(|name| name.to_str())
            != Some(".__weftext-transaction-workspace-current")
        || physical_root_identity_binding(physical_object_identity(&canonical_transaction)?)
            != *transaction_identity
    {
        return Err(PhysicalInventoryError::UnfinishedTransaction(
            PhysicalLocator::parse(".__weftext-transaction-workspace-current")?,
        ));
    }
    let inventory = capture_stable_canonical_root_with_exclusion(
        &root,
        CaptureMode::Workspace,
        Some(transaction_identity),
    )?;
    if physical_root_identity_binding(physical_object_identity(&canonical_transaction)?)
        != *transaction_identity
    {
        return Err(PhysicalInventoryError::ConcurrentChange);
    }
    require_lease_anchor_identity(lease)?;
    Ok(inventory)
}

pub(crate) fn physical_root_identity_at(
    path: &Path,
) -> Result<PhysicalRootIdentityBinding, PhysicalInventoryError> {
    let root = canonical_non_linked_directory(path)?;
    Ok(physical_root_identity_binding(physical_object_identity(
        &root,
    )?))
}

pub(crate) fn verify_disjoint_external_physical_tree_excluding_transaction(
    workspace_lease: &WorkspaceTransactionLease,
    external_root: &Path,
    expected_binding: &PhysicalInventoryBinding,
    expected_root_identity: &PhysicalRootIdentityBinding,
    transaction: &Path,
    transaction_identity: &PhysicalRootIdentityBinding,
) -> Result<(), PhysicalInventoryError> {
    verify_disjoint_external_physical_tree_excluding_transaction_with_final_probe(
        workspace_lease,
        external_root,
        expected_binding,
        expected_root_identity,
        transaction,
        transaction_identity,
        || {},
    )
}

fn verify_disjoint_external_physical_tree_excluding_transaction_with_final_probe(
    workspace_lease: &WorkspaceTransactionLease,
    external_root: &Path,
    expected_binding: &PhysicalInventoryBinding,
    expected_root_identity: &PhysicalRootIdentityBinding,
    transaction: &Path,
    transaction_identity: &PhysicalRootIdentityBinding,
    final_probe: impl FnOnce(),
) -> Result<(), PhysicalInventoryError> {
    expected_binding.validate()?;
    expected_root_identity.validate()?;
    let external_root = canonical_non_linked_directory(external_root)?;
    require_disjoint(workspace_lease.physical_inventory_root(), &external_root)?;
    let workspace_inventory = capture_stable_workspace_physical_inventory_excluding_transaction(
        workspace_lease,
        transaction,
        transaction_identity,
    )?;
    let external_inventory = capture_stable_canonical_root(&external_root, CaptureMode::Exact)?;
    require_directory_identity_disjoint(&workspace_inventory, &external_inventory)?;
    if external_inventory.binding != *expected_binding
        || external_inventory.root_identity != *expected_root_identity
    {
        return Err(PhysicalInventoryError::BindingMismatch);
    }
    final_probe();
    let final_workspace_inventory =
        capture_stable_workspace_physical_inventory_excluding_transaction(
            workspace_lease,
            transaction,
            transaction_identity,
        )?;
    if final_workspace_inventory != workspace_inventory {
        return Err(PhysicalInventoryError::ConcurrentChange);
    }
    require_lease_anchor_identity(workspace_lease)
}

fn capture_stable_canonical_root_with_exclusion(
    root: &Path,
    mode: CaptureMode,
    excluded_transaction_identity: Option<&PhysicalRootIdentityBinding>,
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    let first = capture_once(root, mode, excluded_transaction_identity)?;
    let second = capture_once(root, mode, excluded_transaction_identity)?;
    if first == second {
        Ok(first)
    } else {
        Err(PhysicalInventoryError::ConcurrentChange)
    }
}

fn capture_once(
    root: &Path,
    mode: CaptureMode,
    excluded_transaction_identity: Option<&PhysicalRootIdentityBinding>,
) -> Result<PhysicalTreeInventory, PhysicalInventoryError> {
    let mut entries = Vec::new();
    let mut directory_identities = BTreeSet::new();
    let mut file_identities = BTreeSet::new();
    let mut file_bytes = 0_u64;
    let root_object_identity = physical_object_identity(root)?;
    collect_directory(
        root,
        root,
        mode,
        excluded_transaction_identity,
        &mut entries,
        &mut directory_identities,
        &mut file_identities,
        &mut file_bytes,
    )?;
    entries.sort();
    if entries
        .windows(2)
        .any(|pair| pair[0].locator == pair[1].locator)
    {
        return Err(PhysicalInventoryError::ConcurrentChange);
    }
    let entry_count =
        u64::try_from(entries.len()).map_err(|_| PhysicalInventoryError::EntryLimitExceeded)?;
    let sha256 = inventory_digest(&entries)?;
    Ok(PhysicalTreeInventory {
        entries,
        binding: PhysicalInventoryBinding {
            schema: PHYSICAL_TREE_INVENTORY_SCHEMA.to_owned(),
            sha256,
            entry_count,
            file_bytes,
        },
        root_identity: physical_root_identity_binding(root_object_identity),
        directory_identities,
        file_identities,
    })
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the recursive walker carries one closed accumulator set and keeps all entry-type validation in one reviewable boundary"
)]
fn collect_directory(
    root: &Path,
    directory: &Path,
    mode: CaptureMode,
    excluded_transaction_identity: Option<&PhysicalRootIdentityBinding>,
    entries: &mut Vec<PhysicalInventoryEntry>,
    directory_identities: &mut BTreeSet<file_id::FileId>,
    file_identities: &mut BTreeSet<file_id::FileId>,
    file_bytes: &mut u64,
) -> Result<(), PhysicalInventoryError> {
    let directory_identity = validate_directory(root, directory)?;
    if !directory_identities.insert(directory_identity) {
        return Err(PhysicalInventoryError::DirectoryIdentityAlias(
            optional_locator(root, directory),
        ));
    }
    let mut children = Vec::new();
    for child in fs::read_dir(directory)
        .map_err(|source| io_error("enumerate physical directory", source))?
    {
        let pending = entries
            .len()
            .checked_add(children.len())
            .ok_or(PhysicalInventoryError::EntryLimitExceeded)?;
        if pending >= PHYSICAL_TREE_MAX_ENTRIES {
            return Err(PhysicalInventoryError::EntryLimitExceeded);
        }
        children
            .push(child.map_err(|source| io_error("enumerate physical directory entry", source))?);
    }
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
        let path = child.path();
        let name = child
            .file_name()
            .into_string()
            .map_err(|_| PhysicalInventoryError::NonUtf8Path)?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|source| io_error("inspect physical entry", source))?;
        if mode == CaptureMode::Workspace && directory == root {
            if name == WORKSPACE_TRANSACTION_LEASE_FILE_NAME {
                let locator = locator_from_name(&name)?;
                if linked_or_reparse(&metadata) {
                    return Err(PhysicalInventoryError::LinkedOrReparse(Some(locator)));
                }
                if !metadata.is_file() || metadata.len() != 0 {
                    return Err(PhysicalInventoryError::UnfinishedTransaction(locator));
                }
                ensure_resolved_inside(&path, root)?;
                continue;
            }
            if name == ".__weftext-transaction-workspace-current"
                && let Some(expected) = excluded_transaction_identity
            {
                if linked_or_reparse(&metadata)
                    || !metadata.is_dir()
                    || physical_root_identity_binding(physical_object_identity(&path)?) != *expected
                {
                    return Err(PhysicalInventoryError::UnfinishedTransaction(
                        locator_from_name(&name)?,
                    ));
                }
                continue;
            }
            if name
                .to_ascii_lowercase()
                .starts_with(&WORKSPACE_TRANSACTION_PREFIX.to_ascii_lowercase())
            {
                return Err(PhysicalInventoryError::UnfinishedTransaction(
                    locator_from_name(&name)?,
                ));
            }
        }
        let locator = locator_from_path(root, &path)?;
        if linked_or_reparse(&metadata) {
            return Err(PhysicalInventoryError::LinkedOrReparse(Some(locator)));
        }
        ensure_resolved_inside(&path, root)?;
        if metadata.is_dir() {
            push_entry(
                entries,
                PhysicalInventoryEntry {
                    locator,
                    kind: PhysicalEntryKind::Directory,
                    byte_length: 0,
                    sha256: None,
                },
            )?;
            collect_directory(
                root,
                &path,
                mode,
                excluded_transaction_identity,
                entries,
                directory_identities,
                file_identities,
                file_bytes,
            )?;
        } else if metadata.is_file() {
            let (byte_length, sha256, identity) = digest_regular_file(root, &path)?;
            file_identities.insert(identity);
            *file_bytes = file_bytes
                .checked_add(byte_length)
                .ok_or(PhysicalInventoryError::FileByteCountOverflow)?;
            push_entry(
                entries,
                PhysicalInventoryEntry {
                    locator,
                    kind: PhysicalEntryKind::RegularFile,
                    byte_length,
                    sha256: Some(sha256),
                },
            )?;
        } else {
            return Err(PhysicalInventoryError::UnsupportedEntry(Some(locator)));
        }
    }
    if validate_directory(root, directory)? == directory_identity {
        Ok(())
    } else {
        Err(PhysicalInventoryError::ConcurrentChange)
    }
}

fn push_entry(
    entries: &mut Vec<PhysicalInventoryEntry>,
    entry: PhysicalInventoryEntry,
) -> Result<(), PhysicalInventoryError> {
    if entries.len() >= PHYSICAL_TREE_MAX_ENTRIES {
        return Err(PhysicalInventoryError::EntryLimitExceeded);
    }
    entries.push(entry);
    Ok(())
}

fn digest_regular_file(
    root: &Path,
    path: &Path,
) -> Result<(u64, PhysicalSha256, file_id::FileId), PhysicalInventoryError> {
    let path_before =
        fs::symlink_metadata(path).map_err(|source| io_error("inspect physical file", source))?;
    let locator = locator_from_path(root, path)?;
    if linked_or_reparse(&path_before) || !path_before.is_file() {
        return Err(PhysicalInventoryError::LinkedOrReparse(Some(locator)));
    }
    ensure_resolved_inside(path, root)?;
    let mut file = File::open(path).map_err(|source| io_error("open physical file", source))?;
    let before = file
        .metadata()
        .map_err(|source| io_error("inspect open physical file", source))?;
    if !before.is_file() {
        return Err(PhysicalInventoryError::UnsupportedEntry(Some(locator)));
    }
    let before_modified = before.modified().ok();
    let before_identity = physical_object_identity(path)?;
    let mut byte_length = 0_u64;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| io_error("read physical file", source))?;
        if read == 0 {
            break;
        }
        byte_length = byte_length
            .checked_add(read as u64)
            .ok_or(PhysicalInventoryError::FileByteCountOverflow)?;
        hasher.update(&buffer[..read]);
    }
    let after = file
        .metadata()
        .map_err(|source| io_error("reinspect open physical file", source))?;
    let path_after = fs::symlink_metadata(path)
        .map_err(|source| io_error("reinspect physical file path", source))?;
    let after_identity = physical_object_identity(path)?;
    if linked_or_reparse(&path_after)
        || !path_after.is_file()
        || before.len() != byte_length
        || after.len() != byte_length
        || (before_modified.is_some() && before_modified != after.modified().ok())
        || before_identity != after_identity
    {
        return Err(PhysicalInventoryError::ConcurrentChange);
    }
    ensure_resolved_inside(path, root)?;
    Ok((
        byte_length,
        PhysicalSha256(hasher.finalize().into()),
        before_identity,
    ))
}

fn physical_root_identity_binding(identity: file_id::FileId) -> PhysicalRootIdentityBinding {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.physical-root-identity.v1\0");
    match identity {
        file_id::FileId::Inode {
            device_id,
            inode_number,
        } => {
            hasher.update(b"inode\0");
            hasher.update(device_id.to_be_bytes());
            hasher.update(inode_number.to_be_bytes());
        }
        file_id::FileId::LowRes {
            volume_serial_number,
            file_index,
        } => {
            hasher.update(b"lowres\0");
            hasher.update(volume_serial_number.to_be_bytes());
            hasher.update(file_index.to_be_bytes());
        }
        file_id::FileId::HighRes {
            volume_serial_number,
            file_id,
        } => {
            hasher.update(b"highres\0");
            hasher.update(volume_serial_number.to_be_bytes());
            hasher.update(file_id.to_be_bytes());
        }
    }
    PhysicalRootIdentityBinding {
        schema: PHYSICAL_ROOT_IDENTITY_SCHEMA.to_owned(),
        sha256: hex_sha256(&hasher.finalize()),
    }
}

fn inventory_digest(entries: &[PhysicalInventoryEntry]) -> Result<String, PhysicalInventoryError> {
    let mut hasher = Sha256::new();
    hasher.update(PHYSICAL_TREE_DIGEST_DOMAIN);
    for entry in entries {
        hasher.update([match entry.kind {
            PhysicalEntryKind::Directory => b'D',
            PhysicalEntryKind::RegularFile => b'F',
        }]);
        let locator = entry.locator.as_str().as_bytes();
        let locator_length = u64::try_from(locator.len())
            .map_err(|_| PhysicalInventoryError::LocatorLimitExceeded)?;
        hasher.update(locator_length.to_be_bytes());
        hasher.update(locator);
        hasher.update(entry.byte_length.to_be_bytes());
        match entry.sha256 {
            Some(sha256) => hasher.update(sha256.as_bytes()),
            None => hasher.update([0_u8; 32]),
        }
    }
    Ok(hex_sha256(&hasher.finalize()))
}

fn canonical_non_linked_directory(path: &Path) -> Result<PathBuf, PhysicalInventoryError> {
    reject_linked_existing_ancestors(path).map_err(map_ancestor_error)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| io_error("inspect physical inventory root", source))?;
    if linked_or_reparse(&metadata) {
        return Err(PhysicalInventoryError::LinkedOrReparse(None));
    }
    if !metadata.is_dir() {
        return Err(PhysicalInventoryError::RootNotDirectory);
    }
    let canonical = fs::canonicalize(path)
        .map_err(|source| io_error("resolve physical inventory root", source))?;
    reject_linked_existing_ancestors(&canonical).map_err(map_ancestor_error)?;
    Ok(canonical)
}

fn validate_directory(root: &Path, path: &Path) -> Result<file_id::FileId, PhysicalInventoryError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| io_error("inspect physical directory", source))?;
    let locator = optional_locator(root, path);
    if linked_or_reparse(&metadata) {
        return Err(PhysicalInventoryError::LinkedOrReparse(locator));
    }
    if !metadata.is_dir() {
        return Err(PhysicalInventoryError::UnsupportedEntry(locator));
    }
    ensure_resolved_inside(path, root)?;
    physical_object_identity(path)
}

#[cfg(unix)]
fn physical_object_identity(path: &Path) -> Result<file_id::FileId, PhysicalInventoryError> {
    file_id::get_file_id(path).map_err(|source| PhysicalInventoryError::IdentityUnavailable {
        operation: "identify physical directory",
        source,
    })
}

#[cfg(windows)]
fn physical_object_identity(path: &Path) -> Result<file_id::FileId, PhysicalInventoryError> {
    file_id::get_high_res_file_id(path).map_err(|source| {
        PhysicalInventoryError::IdentityUnavailable {
            operation: "identify physical directory",
            source,
        }
    })
}

#[cfg(not(any(unix, windows)))]
fn physical_object_identity(_path: &Path) -> Result<file_id::FileId, PhysicalInventoryError> {
    Err(PhysicalInventoryError::IdentityUnsupported)
}

fn ensure_resolved_inside(path: &Path, root: &Path) -> Result<(), PhysicalInventoryError> {
    let canonical =
        fs::canonicalize(path).map_err(|source| io_error("resolve physical entry", source))?;
    if canonical.starts_with(root) {
        Ok(())
    } else {
        Err(PhysicalInventoryError::PathEscape(optional_locator(
            root, path,
        )))
    }
}

fn locator_from_name(name: &str) -> Result<PhysicalLocator, PhysicalInventoryError> {
    PhysicalLocator::parse(name)
}

fn locator_from_path(root: &Path, path: &Path) -> Result<PhysicalLocator, PhysicalInventoryError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| PhysicalInventoryError::PathEscape(None))?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        let Component::Normal(value) = component else {
            return Err(PhysicalInventoryError::PathEscape(None));
        };
        pieces.push(value.to_str().ok_or(PhysicalInventoryError::NonUtf8Path)?);
    }
    PhysicalLocator::parse(&pieces.join("/"))
}

fn optional_locator(root: &Path, path: &Path) -> Option<PhysicalLocator> {
    (root != path)
        .then(|| locator_from_path(root, path).ok())
        .flatten()
}

fn require_disjoint(workspace: &Path, external: &Path) -> Result<(), PhysicalInventoryError> {
    if external.starts_with(workspace) || workspace.starts_with(external) {
        Err(PhysicalInventoryError::ExternalTreeNotDisjoint)
    } else {
        Ok(())
    }
}

fn require_directory_identity_disjoint(
    workspace: &PhysicalTreeInventory,
    external: &PhysicalTreeInventory,
) -> Result<(), PhysicalInventoryError> {
    if workspace
        .directory_identities
        .is_disjoint(&external.directory_identities)
        && workspace
            .file_identities
            .is_disjoint(&external.file_identities)
    {
        Ok(())
    } else {
        Err(PhysicalInventoryError::ExternalTreeNotDisjoint)
    }
}

fn require_lease_anchor_identity(
    lease: &WorkspaceTransactionLease,
) -> Result<(), PhysicalInventoryError> {
    match lease.validate_anchor_identity() {
        Ok(()) => Ok(()),
        Err(WorkspaceTransactionError::RecoveryRequired(_)) => {
            Err(PhysicalInventoryError::LeaseAnchorMismatch)
        }
        Err(WorkspaceTransactionError::Io(source)) => {
            Err(PhysicalInventoryError::IdentityUnavailable {
                operation: "identify workspace transaction lease anchor",
                source,
            })
        }
        Err(_) => Err(PhysicalInventoryError::IdentityUnavailable {
            operation: "identify workspace transaction lease anchor",
            source: io::Error::other("workspace lease identity validation failed"),
        }),
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn io_error(operation: &'static str, source: io::Error) -> PhysicalInventoryError {
    PhysicalInventoryError::Io { operation, source }
}

fn map_ancestor_error(error: crate::content_boundary::ContentRulesError) -> PhysicalInventoryError {
    match error {
        crate::content_boundary::ContentRulesError::Io(source) => {
            io_error("inspect physical path ancestor", source)
        }
        _ => PhysicalInventoryError::LinkedOrReparse(None),
    }
}

/// Fail-closed physical-tree inventory error without absolute path disclosure.
#[derive(Debug)]
pub enum PhysicalInventoryError {
    /// The selected root is not a directory.
    RootNotDirectory,
    /// A root/entry crosses a symlink, junction, or other reparse point.
    LinkedOrReparse(Option<PhysicalLocator>),
    /// A resolved entry escaped the selected root.
    PathEscape(Option<PhysicalLocator>),
    /// A path cannot be represented as canonical UTF-8.
    NonUtf8Path,
    /// A path is neither a directory nor regular file.
    UnsupportedEntry(Option<PhysicalLocator>),
    /// Root-level workspace transaction evidence requires recovery.
    UnfinishedTransaction(PhysicalLocator),
    /// The root lease path no longer names the file held by the lease.
    LeaseAnchorMismatch,
    /// Two directory locators resolved to the same filesystem object.
    DirectoryIdentityAlias(Option<PhysicalLocator>),
    /// This platform cannot provide a stable physical object identity.
    IdentityUnsupported,
    /// A filesystem object identity could not be read.
    IdentityUnavailable {
        /// Operation label without a host path.
        operation: &'static str,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// Two complete captures or one open-file observation differed.
    ConcurrentChange,
    /// The fixed complete-tree entry ceiling was exceeded.
    EntryLimitExceeded,
    /// One locator exceeded the fixed byte ceiling.
    LocatorLimitExceeded,
    /// The aggregate regular-file byte count overflowed.
    FileByteCountOverflow,
    /// A supplied relative locator is not canonical/safe.
    InvalidLocator,
    /// A serialized path-free binding has invalid syntax or schema.
    InvalidBinding,
    /// An external tree aliases, contains, or is contained by the workspace.
    ExternalTreeNotDisjoint,
    /// An external tree differs from the reviewed physical binding.
    BindingMismatch,
    /// A filesystem operation failed.
    Io {
        /// Operation label without a host path.
        operation: &'static str,
        /// Underlying I/O error.
        source: io::Error,
    },
}

impl fmt::Display for PhysicalInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotDirectory => {
                formatter.write_str("physical inventory root is not a directory")
            }
            Self::LinkedOrReparse(locator) => write!(
                formatter,
                "physical inventory refuses a link or reparse point at {}",
                locator_text(locator.as_ref())
            ),
            Self::PathEscape(locator) => write!(
                formatter,
                "physical inventory path escaped its root at {}",
                locator_text(locator.as_ref())
            ),
            Self::NonUtf8Path => formatter.write_str("physical inventory path is not UTF-8"),
            Self::UnsupportedEntry(locator) => write!(
                formatter,
                "physical inventory entry is not a directory or regular file at {}",
                locator_text(locator.as_ref())
            ),
            Self::UnfinishedTransaction(locator) => write!(
                formatter,
                "unfinished workspace transaction blocks physical inventory at {}",
                locator.as_str()
            ),
            Self::LeaseAnchorMismatch => formatter.write_str(
                "workspace transaction lease path no longer names the held lease anchor",
            ),
            Self::DirectoryIdentityAlias(locator) => write!(
                formatter,
                "physical inventory directory identity is aliased at {}",
                locator_text(locator.as_ref())
            ),
            Self::IdentityUnsupported => formatter
                .write_str("stable physical object identity is unsupported on this platform"),
            Self::IdentityUnavailable { operation, source } => {
                write!(formatter, "{operation}: {source}")
            }
            Self::ConcurrentChange => {
                formatter.write_str("physical tree changed during stable inventory")
            }
            Self::EntryLimitExceeded => write!(
                formatter,
                "physical inventory exceeds {PHYSICAL_TREE_MAX_ENTRIES} entries"
            ),
            Self::LocatorLimitExceeded => write!(
                formatter,
                "physical inventory locator exceeds {PHYSICAL_TREE_MAX_LOCATOR_BYTES} bytes"
            ),
            Self::FileByteCountOverflow => {
                formatter.write_str("physical inventory byte count overflowed")
            }
            Self::InvalidLocator => formatter.write_str("invalid physical inventory locator"),
            Self::InvalidBinding => formatter.write_str("invalid physical inventory binding"),
            Self::ExternalTreeNotDisjoint => {
                formatter.write_str("external physical tree is not disjoint from the workspace")
            }
            Self::BindingMismatch => {
                formatter.write_str("external physical tree differs from the reviewed binding")
            }
            Self::Io { operation, source } => write!(formatter, "{operation}: {source}"),
        }
    }
}

impl std::error::Error for PhysicalInventoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IdentityUnavailable { source, .. } | Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn locator_text(locator: Option<&PhysicalLocator>) -> &str {
    locator.map_or("<root>", PhysicalLocator::as_str)
}

#[cfg(test)]
mod lease_end_revalidation_tests {
    use super::*;
    use crate::acquire_workspace_transaction_lease;

    fn replace_anchor(root: &Path, displaced: &Path) {
        let anchor = root.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
        fs::rename(anchor, displaced).unwrap();
        fs::write(root.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME), []).unwrap();
    }

    #[test]
    fn workspace_capture_revalidates_anchor_after_the_complete_tree_walk() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Workspace");
        fs::create_dir(&workspace).unwrap();
        let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
        let displaced = temporary.path().join("held-workspace-anchor");

        let result = capture_stable_workspace_physical_inventory_with_final_probe(&lease, || {
            replace_anchor(&workspace, &displaced);
        });
        assert!(matches!(
            result,
            Err(PhysicalInventoryError::LeaseAnchorMismatch)
        ));
    }

    #[test]
    fn external_capture_revalidates_anchor_after_identity_comparison() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Workspace");
        let external = temporary.path().join("External");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&external).unwrap();
        let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
        let displaced = temporary.path().join("held-external-anchor");

        let result =
            capture_disjoint_external_physical_tree_with_final_probe(&lease, &external, || {
                replace_anchor(&workspace, &displaced);
            });
        assert!(matches!(
            result,
            Err(PhysicalInventoryError::LeaseAnchorMismatch)
        ));
    }

    #[test]
    fn external_capture_revalidates_the_complete_workspace_after_external_walk() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Workspace");
        let external = temporary.path().join("External");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&external).unwrap();
        let lease = acquire_workspace_transaction_lease(&workspace).unwrap();

        let result =
            capture_disjoint_external_physical_tree_with_final_probe(&lease, &external, || {
                fs::write(workspace.join("persistent-concurrent.bin"), b"changed").unwrap();
            });
        assert!(matches!(
            result,
            Err(PhysicalInventoryError::ConcurrentChange)
        ));
    }

    #[test]
    fn external_capture_with_journal_exclusion_revalidates_the_same_workspace_view() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Workspace");
        let external = temporary.path().join("External");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&external).unwrap();
        let lease = acquire_workspace_transaction_lease(&workspace).unwrap();
        let transaction = workspace.join(".__weftext-transaction-workspace-current");
        fs::create_dir(&transaction).unwrap();
        let transaction_identity = physical_root_identity_at(&transaction).unwrap();
        let external_inventory = capture_stable_physical_tree(&external).unwrap();

        let result = verify_disjoint_external_physical_tree_excluding_transaction_with_final_probe(
            &lease,
            &external,
            external_inventory.binding(),
            external_inventory.root_identity(),
            &transaction,
            &transaction_identity,
            || {
                fs::write(workspace.join("persistent-concurrent.bin"), b"changed").unwrap();
            },
        );
        assert!(matches!(
            result,
            Err(PhysicalInventoryError::ConcurrentChange)
        ));
    }

    #[test]
    fn record_validation_rejects_duplicate_locators_and_incomplete_parent_topology() {
        let directory = |locator: &str| PhysicalInventoryRecord {
            locator: locator.to_owned(),
            kind: PhysicalEntryKind::Directory,
            byte_length: 0,
            sha256: None,
        };
        let file = |locator: &str| PhysicalInventoryRecord {
            locator: locator.to_owned(),
            kind: PhysicalEntryKind::RegularFile,
            byte_length: 0,
            sha256: Some("0".repeat(64)),
        };

        assert!(matches!(
            physical_inventory_binding_from_records(&[directory("same"), file("same")]),
            Err(PhysicalInventoryError::InvalidBinding)
        ));
        assert!(matches!(
            physical_inventory_binding_from_records(&[file("missing/child.bin")]),
            Err(PhysicalInventoryError::InvalidBinding)
        ));
        assert!(matches!(
            physical_inventory_binding_from_records(&[file("parent"), file("parent/child.bin"),]),
            Err(PhysicalInventoryError::InvalidBinding)
        ));
    }

    #[test]
    fn projected_records_require_every_created_parent_directory() {
        let base = Vec::new();
        let missing_parent = vec![PhysicalInventoryProjectionChange::CreateRegularFile {
            locator: "missing/child.bin".to_owned(),
            bytes: b"bytes".to_vec(),
        }];
        assert!(matches!(
            projected_entries_from_owned(base.clone(), &missing_parent),
            Err(PhysicalInventoryError::InvalidBinding)
        ));

        let file_parent = vec![
            PhysicalInventoryProjectionChange::CreateRegularFile {
                locator: "parent".to_owned(),
                bytes: b"parent bytes".to_vec(),
            },
            PhysicalInventoryProjectionChange::CreateRegularFile {
                locator: "parent/child.bin".to_owned(),
                bytes: b"child bytes".to_vec(),
            },
        ];
        assert!(matches!(
            projected_entries_from_owned(base, &file_parent),
            Err(PhysicalInventoryError::InvalidBinding)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_physical_identity_uses_only_high_resolution_file_id_info() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(matches!(
            physical_object_identity(temporary.path()).unwrap(),
            file_id::FileId::HighRes { .. }
        ));
    }
}
