//! Frozen, explicitly executed local operations. A journal decision provides
//! logical atomicity to this Core; ordinary external tools can observe partial
//! installs. No recovery path retries an unknown namespace operation.
use super::*;
use backend::NamespaceGuard;
use std::collections::{BTreeMap, BTreeSet};
use weftext_messages::UiMessage;

#[cfg(all(test, windows))]
#[path = "definition_admission_tests.rs"]
mod definition_admission_tests;
mod directory;
mod owner_backup;
mod pending;
#[cfg(all(test, windows))]
mod pending_tests;
mod trash;

const MAX_OPERATION_BYTES: usize = 64 * 1024 * 1024;
const MAX_OPERATION_FILES: usize = 5000;
#[path = "operations/compound.rs"]
pub(crate) mod compound;

/// One original namespace participant. Physical evidence belongs to this Plan,
/// while portable object versions remain in its original SyncApplication.
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum RelocationParent {
    /// Original platform-tagged physical evidence for an existing parent.
    Existing(FileIdentity),
    Created {
        created_directory: String,
    },
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct RelocationStep {
    pub(crate) source: String,
    pub(crate) target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parking: Option<String>,
    pub(crate) source_identity: FileIdentity,
    pub(crate) source_parent_identity: FileIdentity,
    pub(crate) target_parent_identity: RelocationParent,
    pub(crate) old_leaf: String,
    pub(crate) new_leaf: String,
    pub(crate) leaf_identity: FileIdentity,
    pub(crate) members: Vec<RelocationMember>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct RetiredMember {
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) identity: FileIdentity,
    pub(crate) parent: FileIdentity,
    pub(crate) attributes: backend::EntryMetadata,
    pub(crate) object: sync::ObjectKey,
    pub(crate) relative_path: String,
    pub(crate) resource_id: String,
    pub(crate) digest: String,
    pub(crate) file_revision: String,
}
pub(crate) struct RelocationRef<'a> {
    pub(crate) source: &'a str,
    pub(crate) target: &'a str,
    pub(crate) source_identity: &'a FileIdentity,
    pub(crate) old_leaf: &'a str,
    pub(crate) new_leaf: &'a str,
    pub(crate) leaf_identity: &'a FileIdentity,
    pub(crate) members: &'a [RelocationMember],
}
impl RelocationRef<'_> {
    pub(crate) fn map(&self, path: &Path) -> Option<PathBuf> {
        let relative = crate::history::history_relative_path(path, Path::new(self.source))?;
        Some(
            Path::new(self.target).join(if relative == Path::new(self.old_leaf) {
                Path::new(self.new_leaf)
            } else {
                &relative
            }),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRequest {
    pub operation_id: String,
    pub root: String,
    pub intent: OperationIntent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OperationIntent {
    /// Frozen by the dedicated create-only library API, never generic prepare.
    CreateLibrary {
        options: crate::LibraryCreateOptions,
    },
    /// Exact observed content adoption, not an edited-after repair path.
    AdoptSyncExternal {
        identity: crate::sync::SyncIdentity,
        expected_installed: Vec<crate::sync::Dependency>,
        changes: Vec<FileChange>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        removed_members: Vec<crate::sync::SyncRemovedMember>,
    },
    ApplySync {
        identity: crate::sync::SyncIdentity,
        objects: BTreeSet<crate::sync::ObjectKey>,
    },
    MergeSync {
        identity: crate::sync::SyncIdentity,
        resolutions: Vec<crate::sync::SyncResolution>,
        blobs: BTreeMap<String, Vec<u8>>,
    },
    RevalidateSync {
        identity: crate::sync::SyncIdentity,
        expected_installed: Vec<crate::sync::Dependency>,
        reviewed_versions: Vec<crate::sync::Dependency>,
        resolutions: Vec<crate::sync::SyncRevalidation>,
        blobs: BTreeMap<String, Vec<u8>>,
    },
    CloneSync {
        identity: crate::sync::SyncIdentity,
        objects: BTreeSet<crate::sync::ObjectKey>,
    },
    RenameNode {
        source: String,
        name: String,
    },
    MoveNode {
        source: String,
        parent: String,
    },
    RelocateDirectory {
        source: String,
        parent: String,
        name: String,
    },
    CopyNode {
        source: String,
        parent: String,
        name: String,
        #[serde(default)]
        allow_losses: bool,
    },
    TemplateNode {
        source: String,
        parent: String,
        name: String,
        instantiated_source: String,
        source_base_id: String,
    },
    TrashNode {
        source: String,
    },
    RestoreNode {
        trash_operation_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        parent: String,
        name: String,
    },
    NewNode {
        parent: String,
        name: String,
        source: Option<String>,
    },
    UpdateFiles {
        changes: Vec<FileChange>,
    },
    BackupLibrary {
        destination: String,
    },
    ConfigureStyle {
        base_id: String,
        source: String,
        enabled: bool,
    },
    ImportResource {
        node: String,
        name: String,
        bytes: Vec<u8>,
        manifest_base_id: String,
        resource_base_id: String,
    },
    ImportNative {
        parent: String,
        name: String,
        bytes: Vec<u8>,
        #[serde(default)]
        allow_losses: bool,
    },
    /// One original, immutable recognition provenance and reviewed native author
    /// proposal share the same create-only transaction and sync publication.
    ImportRecognized {
        parent: String,
        name: String,
        format: String,
        bytes: Vec<u8>,
        evidence: serde_json::Value,
        review: weftext_export::recognition::RecognitionReview,
        #[serde(default)]
        allow_losses: bool,
    },
    ImportBibliography {
        parent: String,
        name: String,
        format: String,
        bytes: Vec<u8>,
        #[serde(default)]
        allow_losses: bool,
    },
    ImportDocx {
        parent: String,
        name: String,
        bytes: Vec<u8>,
        #[serde(default)]
        allow_losses: bool,
    },
    /// Exact DOCX/options and any legacy originals share one durable Core plan.
    ImportDocxWithOptions {
        parent: String,
        name: String,
        bytes: Vec<u8>,
        #[serde(default)]
        allow_losses: bool,
        options: weftext_export::docx::DocxImportOptions,
        #[serde(default)]
        originals: Vec<weftext_export::docx::OriginalImportAsset>,
        #[serde(default)]
        conversion_losses: Vec<weftext_export::exchange::NativeLoss>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileChange {
    pub path: String,
    #[serde(default)]
    pub base_id: Option<String>,
    /// The caller's exact base is compared to the guarded actual file. None
    /// means create-only, not permission to overwrite an existing file.
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationChange {
    pub path: String,
    pub action: String,
    pub bytes: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationPreview {
    pub operation_id: String,
    pub plan_id: String,
    pub kind: String,
    pub status: String,
    pub changes: Vec<OperationChange>,
    pub node_map: BTreeMap<String, String>,
    pub new_node_id: Option<String>,
    pub revisions: BTreeMap<String, String>,
    pub resource_map: BTreeMap<String, String>,
    pub losses: Vec<weftext_knowledge::Loss>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationReceipt {
    pub operation_id: String,
    pub plan_id: String,
    pub status: String,
    pub path: String,
    pub node_ids: Vec<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigRole {
    Schema,
    WorkspaceSettings,
    Rules,
}
impl ConfigRole {
    fn filename(self) -> &'static str {
        match self {
            Self::Schema => ".weftext-schema.json",
            Self::WorkspaceSettings => ".weftext-settings.json",
            Self::Rules => ".weftext-rules",
        }
    }
    fn role(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::WorkspaceSettings => "workspace-settings",
            Self::Rules => "rules",
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDocument {
    pub role_ref: String,
    pub path: String,
    pub bytes: Option<Vec<u8>>,
    pub revision: Option<String>,
    pub base_id: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStyleDocument {
    pub source: String,
    pub base_id: String,
    pub path: String,
    pub enabled: bool,
    pub resource_ref: Option<String>,
    pub entry_function: String,
    pub installed: bool,
}
#[derive(Clone)]
pub(crate) struct StyleBasis {
    root: String,
    settings: SnapshotBinding,
    style: Option<SnapshotBinding>,
    manifest: Option<SnapshotBinding>,
    owner: Option<SnapshotBinding>,
    source: String,
    resource_ref: Option<String>,
}
pub struct LibraryStyleWitness {
    basis: StyleBasis,
}
impl LibraryStyleWitness {
    pub fn retained_bytes(&self) -> usize {
        style_basis_retained_bytes(&self.basis)
    }
}
fn style_basis_retained_bytes(basis: &StyleBasis) -> usize {
    std::mem::size_of::<LibraryStyleWitness>()
        + basis.root.capacity()
        + basis.source.capacity()
        + basis.resource_ref.as_ref().map_or(0, String::capacity)
        + [
            Some(&basis.settings),
            basis.style.as_ref(),
            basis.manifest.as_ref(),
            basis.owner.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|b| {
            b.path.capacity()
                + b.captured
                    .as_ref()
                    .map_or(0, |f| f.path.capacity() + f.bytes.capacity())
        })
        .sum::<usize>()
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    pub operation_id: String,
    pub node_id: String,
    pub name: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedResource {
    pub resource_id: String,
    pub resource_ref: String,
    pub node_id: String,
    pub path: String,
    pub bytes: Vec<u8>,
    pub base_id: String,
    pub revision: Option<String>,
    pub identity_status: String,
    pub manifest_path: String,
    pub manifest_base_id: String,
    pub manifest_bytes: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
struct ResourceBirth {
    resource_id: String,
    workspace_id: String,
    node_id: String,
    identity: FileIdentity,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionReservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library: Option<hosted::ActorLibraryBinding>,
    #[serde(
        default,
        rename = "operationId",
        skip_serializing_if = "Option::is_none"
    )]
    operation_id: Option<String>,
    owner_id: String,
    source: RevisionReservation,
    key: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFile {
    pub path: String,
    pub bytes: Option<Vec<u8>>,
    pub base_id: String,
    pub node_id: Option<String>,
    pub kind: String,
    pub currentness: String,
    pub revision: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSet {
    pub files: Vec<SnapshotFile>,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmittedSnapshot {
    pub cut_id: String,
    pub membership_ids: Vec<String>,
    pub inventory: Inventory,
    pub files: Vec<SnapshotFile>,
    pub schema: Option<SnapshotFile>,
    pub tags: Option<SnapshotFile>,
    pub complete: bool,
    pub warnings: Vec<CoreDescription>,
    pub raw_complete: bool,
    pub raw_pending: usize,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmittedCompileSnapshot {
    pub snapshot: AdmittedSnapshot,
    pub files: BTreeMap<String, Vec<u8>>,
    pub style: Option<weftext_engine::StyleModule>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedOperationSource {
    pub receipt: Option<Receipt>,
    pub source: String,
    pub definition_key: Option<String>,
    pub status: String,
    pub root: Option<String>,
    pub path: String,
}
#[derive(Clone)]
pub(crate) struct SnapshotBinding {
    pub(crate) path: String,
    pub(crate) captured: Option<CapturedFile>,
    pub(crate) parent: FileIdentity,
}
/// An owned read witness. It cannot be deserialized or used as a write basis;
/// dropping it releases the captured bytes without retaining them in Core.
#[derive(Clone)]
pub struct SnapshotWitness {
    pub(crate) binding: SnapshotBinding,
}
impl SnapshotWitness {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.binding.path.capacity()
            + self
                .binding
                .captured
                .as_ref()
                .map_or(0, |f| f.bytes.capacity() + f.path.capacity())
    }
}
/// One explicitly selected, live canonical node. This is a read witness, not
/// a writer or a claim that an external source has a committed revision.
pub struct QualifiedNode {
    pub owner: weftext_knowledge::NodeRef,
    pub revision: Option<String>,
    pub source_hash: String,
    pub witness: SnapshotWitness,
    pub(crate) identity: FileIdentity,
}
impl QualifiedNode {
    pub(crate) fn captured_file(&self) -> &CapturedFile {
        self.witness.binding.captured.as_ref().unwrap()
    }
}
fn same_snapshot_dependency(original: &SnapshotBinding, incoming: &SnapshotBinding) -> bool {
    Path::new(&original.path) == Path::new(&incoming.path)
        || matches!((&original.captured, &incoming.captured), (Some(a), Some(b))
            if a.stamp.identity == b.stamp.identity)
}
fn verify_duplicate_snapshot(original: &SnapshotBinding, incoming: &SnapshotBinding) -> Result<()> {
    if !crate::history::history_relative_path(Path::new(&original.path), Path::new(&incoming.path))
        .is_some_and(|relative| relative.as_os_str().is_empty())
        || original.parent != incoming.parent
        || original.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
            != incoming.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
    {
        return Err(CoreError::new(
            "staleBase",
            UiMessage::error_core_operations_competing_read_witnesses_for_one_dependency(),
        ));
    }
    Ok(())
}
#[derive(Clone)]
pub(crate) struct AdmittedBasis {
    pub(crate) root: String,
    pub(crate) lifecycle: String,
    members: BTreeSet<String>,
    pub(crate) bases: Vec<String>,
    tag_base: Option<String>,
    complete: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct RevisionFact {
    pub(crate) path: String,
    pub(crate) stamp: FileStamp,
    pub(crate) hash: String,
    pub(crate) revision: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevisionReservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) library: Option<hosted::ActorLibraryBinding>,
    #[serde(
        default,
        rename = "operationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) operation_id: Option<String>,
    pub(crate) path: String,
    pub(crate) stamp: FileStamp,
    pub(crate) bytes: Vec<u8>,
    pub(crate) revision: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct CapturedFile {
    pub(crate) path: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) stamp: FileStamp,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct CapturedDirectory {
    pub(crate) path: String,
    pub(crate) identity: FileIdentity,
    pub(crate) entries: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct AbsentDependency {
    pub(crate) path: String,
    pub(crate) parent: FileIdentity,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct NewBirth {
    pub(crate) path: String,
    pub(crate) node_id: String,
}
/// Physical placement evidence only. Source bytes are deliberately not frozen:
/// ordinary moves carry late edits and late children with the original tree.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct RelocationMember {
    pub(crate) relative_path: String,
    pub(crate) identity: FileIdentity,
    pub(crate) parent: FileIdentity,
    pub(crate) node_id: Option<String>,
    pub(crate) directory_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum Mutation {
    Compound {
        relocations: Vec<RelocationStep>,
        updates: Vec<FileChange>,
        #[serde(default)]
        created_files: Vec<FileChange>,
        #[serde(default)]
        created_directories: Vec<String>,
        #[serde(default)]
        births: Vec<NewBirth>,
        #[serde(default)]
        retired_members: Vec<RetiredMember>,
    },
    Relocate {
        source: String,
        target: String,
        source_identity: FileIdentity,
        target_parent_identity: FileIdentity,
        create_trash_parent: bool,
        old_leaf: String,
        new_leaf: String,
        leaf_identity: FileIdentity,
        #[serde(default)]
        members: Vec<RelocationMember>,
        /// Exact content participants at their final paths, in the same decision.
        #[serde(default)]
        updates: Vec<FileChange>,
    },
    CreateTree {
        target: String,
        /// Hydration creates only absent descendants of a separately guarded
        /// existing workspace root. Ordinary creation still requires absence.
        #[serde(default)]
        existing_target: bool,
        directories: Vec<String>,
        files: Vec<FileChange>,
        births: Vec<NewBirth>,
        updates: Vec<FileChange>,
    },
    Update {
        files: Vec<FileChange>,
    },
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct FrozenOperation {
    pub(crate) request: OperationRequest,
    pub(crate) preview: OperationPreview,
    pub(crate) files: Vec<CapturedFile>,
    pub(crate) directories: Vec<CapturedDirectory>,
    pub(crate) mutation: Mutation,
    pub(crate) receipt: OperationReceipt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) hosted_destination: Option<HostedDestination>,
    #[serde(default)]
    pub(crate) expected_membership: Option<BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) expected_trash_membership: Option<BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) backup_private_digest: Option<String>,
    #[serde(default)]
    pub(crate) absent_dependencies: Vec<AbsentDependency>,
    #[serde(default)]
    pub(crate) write_parents: Vec<(String, FileIdentity)>,
}
impl FrozenOperation {
    pub(crate) fn retired_members(&self) -> &[RetiredMember] {
        match &self.mutation {
            Mutation::Compound {
                retired_members, ..
            } => retired_members,
            _ => &[],
        }
    }
    pub(crate) fn is_retired_member(&self, path: &str) -> bool {
        self.retired_members()
            .iter()
            .any(|member| locator_equal(&member.source, path))
    }
    pub(crate) fn births(&self) -> &[NewBirth] {
        match &self.mutation {
            Mutation::CreateTree { births, .. } | Mutation::Compound { births, .. } => births,
            _ => &[],
        }
    }
    pub(crate) fn created_files(&self) -> &[FileChange] {
        match &self.mutation {
            Mutation::CreateTree { files, .. } => files,
            Mutation::Compound { created_files, .. } => created_files,
            _ => &[],
        }
    }
    pub(crate) fn relocations(&self) -> Vec<RelocationRef<'_>> {
        match &self.mutation {
            Mutation::Relocate {
                source,
                target,
                source_identity,
                old_leaf,
                new_leaf,
                leaf_identity,
                members,
                ..
            } => vec![RelocationRef {
                source,
                target,
                source_identity,
                old_leaf,
                new_leaf,
                leaf_identity,
                members,
            }],
            Mutation::Compound { relocations, .. } => relocations
                .iter()
                .map(|s| RelocationRef {
                    source: &s.source,
                    target: &s.target,
                    source_identity: &s.source_identity,
                    old_leaf: &s.old_leaf,
                    new_leaf: &s.new_leaf,
                    leaf_identity: &s.leaf_identity,
                    members: &s.members,
                })
                .collect(),
            _ => vec![],
        }
    }
    pub(crate) fn is_trash(&self) -> bool {
        matches!(&self.mutation, Mutation::Relocate { source, target, .. }
            if source != target && Path::new(target).parent() == Some(Path::new(&self.request.root).join(".weftext-trash").as_path()))
    }

    pub(crate) fn relocated_path(&self, path: &str) -> String {
        self.relocations()
            .iter()
            .filter_map(|step| {
                step.map(Path::new(path))
                    .map(|mapped| (Path::new(step.source).components().count(), mapped))
            })
            .max_by_key(|(depth, _)| *depth)
            .map(|(_, mapped)| mapped)
            .map(|path| display(&path))
            .unwrap_or_else(|| path.into())
    }
    pub(crate) fn relocated_files(&self) -> Vec<FileChange> {
        let updates = match &self.mutation {
            Mutation::Relocate { updates, .. } | Mutation::Compound { updates, .. } => updates,
            _ => return vec![],
        };
        let mut files: BTreeMap<_, _> = self
            .files
            .iter()
            .filter(|file| !self.is_retired_member(&file.path))
            .map(|f| {
                let path = self.relocated_path(&f.path);
                (
                    path.clone(),
                    FileChange {
                        path,
                        base_id: None,
                        before: Some(f.bytes.clone()),
                        after: f.bytes.clone(),
                    },
                )
            })
            .collect();
        files.extend(updates.iter().map(|f| (f.path.clone(), f.clone())));
        if let Mutation::Compound { created_files, .. } = &self.mutation {
            files.extend(
                created_files
                    .iter()
                    .map(|file| (file.path.clone(), file.clone())),
            );
        }
        files.into_values().collect()
    }
    fn updated_resource_owner(
        &self,
        path: &Path,
        application: Option<&sync::apply::SyncApplication>,
    ) -> Option<String> {
        if let Some(application) = application {
            // Receiver ownership comes from the reviewed content role and
            // placement, never a metadata claim in arbitrary resource bytes.
            return application
                .after
                .iter()
                .filter_map(|o| match &o.object {
                    sync::ObjectKey::Node(id) if path.starts_with(&o.directory) => Some((o, id)),
                    _ => None,
                })
                .max_by_key(|(o, _)| Path::new(&o.directory).components().count())
                .map(|(_, id)| id.clone());
        }
        // The ordinary resource/style planners each have exactly one owner.
        (self.preview.node_map.len() == 1)
            .then(|| self.preview.node_map.values().next().unwrap().clone())
    }
    pub(crate) fn affects(&self, path: &Path) -> bool {
        let target = display(path).to_lowercase();
        let within = |p: &str| {
            let p = p.to_lowercase();
            target == p || target.starts_with(&(p + "\\"))
        };
        match &self.mutation {
            Mutation::Compound {
                relocations,
                updates,
                created_files,
                created_directories,
                retired_members,
                ..
            } => {
                relocations.iter().any(|step| {
                    within(&step.source)
                        || within(&step.target)
                        || step.parking.as_ref().is_some_and(|path| within(path))
                }) || updates
                    .iter()
                    .chain(created_files)
                    .any(|change| Path::new(&change.path) == path)
                    || retired_members.iter().any(|member| {
                        locator_equal(&member.source, &display(path))
                            || locator_equal(&member.target, &display(path))
                    })
                    || created_directories
                        .iter()
                        .any(|directory| within(directory))
            }
            Mutation::Relocate {
                source,
                target,
                updates,
                ..
            } => {
                within(source)
                    || within(target)
                    || updates
                        .iter()
                        .any(|c| c.before.as_ref() != Some(&c.after) && Path::new(&c.path) == path)
            }
            Mutation::CreateTree {
                target,
                existing_target,
                directories,
                files,
                updates,
                ..
            } => {
                (!existing_target && within(target))
                    || (*existing_target
                        && directories
                            .iter()
                            .any(|d| within(&display(&Path::new(target).join(d)))))
                    || files
                        .iter()
                        .any(|f| f.path.eq_ignore_ascii_case(&display(path)))
                    || updates
                        .iter()
                        .any(|f| f.path.eq_ignore_ascii_case(&display(path)))
            }
            Mutation::Update { files } => files
                .iter()
                .any(|f| f.path.eq_ignore_ascii_case(&display(path))),
        }
    }
    pub(crate) fn has_identity(&self, identity: &FileIdentity) -> bool {
        self.files.iter().any(|f| &f.stamp.identity == identity)
            || self.relocations().iter().any(|step| {
                step.source_identity == identity
                    || step.leaf_identity == identity
                    || step
                        .members
                        .iter()
                        .any(|member| &member.identity == identity || &member.parent == identity)
            })
    }
}
impl Core {
    /// Qualify only an operator-selected source; never discover ambient nodes.
    pub fn qualified_node_at(&self, root: &Path, path: &Path) -> Result<QualifiedNode> {
        let root = absolute_path(root)?;
        let path = absolute_path(path)?;
        let workspace = self.require_workspace(&root)?;
        if !path.starts_with(&root)
            || !is_canonical(&path)
            || path.components().any(|c| {
                c.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(".weftext-trash")
            })
        {
            return Err(CoreError::new(
                "unsupportedRole",
                UiMessage::error_core_operations_select_exact_live_canonical_node_source(),
            ));
        }
        self.check_pending(&path)?;
        self.check_discovery_boundary(&root, path.parent().unwrap())?;
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let mut file = guard.open_file(false, false)?;
        let actual = stamp(&file)?;
        self.check_pending_identity(&actual.identity)?;
        let bytes = read_source(&mut file)?;
        let birth = self.admitted_binding(&actual.identity)?.ok_or_else(|| {
            CoreError::new(
                "identityPending",
                UiMessage::error_core_operations_selected_object_has_no_committed_node_admission(),
            )
        })?;
        if birth.workspace_id != workspace.workspace_id
            || weftext_engine::inspect_metadata(&decode(&bytes)?)
                .id
                .as_ref()
                != Some(&birth.node_id)
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_operations_current_declaration_workspace_and_object_admission_differ(),
            ));
        }
        self.check_history_placement(&birth, &path)?;
        let captured = CapturedFile {
            path: display(&path),
            bytes,
            stamp: actual,
        };
        Ok(QualifiedNode {
            owner: weftext_knowledge::NodeRef {
                workspace_id: birth.workspace_id,
                node_id: birth.node_id,
            },
            revision: self.accepted_revision(&captured)?,
            source_hash: hash(&captured.bytes),
            identity: captured.stamp.identity.clone(),
            witness: SnapshotWitness {
                binding: SnapshotBinding {
                    path: captured.path.clone(),
                    captured: Some(captured),
                    parent: guard.directory_identity()?,
                },
            },
        })
    }
    pub fn qualified_node_for_session_at(
        &self,
        root: &Path,
        path: &Path,
        session_id: &str,
    ) -> Result<QualifiedNode> {
        self.backend.check_path(&self.data_dir)?;
        self.check_actor_session(session_id)?;
        let session = &self.sessions[session_id];
        if session.path != absolute_path(path)? {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_operations_opened_and_configured_owner_paths_differ(),
            ));
        }
        let current = self.qualified_node_at(root, path)?;
        let original = self
            .admitted_binding(&session.stamp.identity)?
            .ok_or_else(|| {
                CoreError::new(
                    "identityPending",
                    UiMessage::error_core_operations_original_opened_node_admission_unavailable(),
                )
            })?;
        if session.node_id.as_ref() != Some(&current.owner.node_id)
            || session.stamp.identity != current.identity
            || original.workspace_id != current.owner.workspace_id
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_operations_current_object_differs_from_original_opened_owner(
                ),
            ));
        }
        Ok(current)
    }
    pub(crate) fn validate_source_reservation(
        &self,
        id: &str,
        suffix: &str,
        bytes: &[u8],
    ) -> Result<hosted::ActorLibraryBinding> {
        let invalid = || {
            CoreError::new(
                "resolutionUnavailable",
                UiMessage::error_core_operations_original_source_reservation_cannot_qualified_to_attempt_and_owner(),
            )
        };
        let (library, original, source, owner, key) = if suffix == "definition-reservation" {
            let record: DefinitionReservation =
                serde_json::from_slice(bytes).map_err(|_| invalid())?;
            (
                record.library,
                record.operation_id,
                record.source,
                Some(record.owner_id),
                Some(record.key),
            )
        } else if suffix == "revision-reservation" {
            let record: RevisionReservation =
                serde_json::from_slice(bytes).map_err(|_| invalid())?;
            (
                record.library.clone(),
                record.operation_id.clone(),
                record,
                None,
                None,
            )
        } else {
            return Err(invalid());
        };
        let binding = library.ok_or_else(invalid)?;
        let original_root = self.validate_original_reservation_library(&binding)?;
        if original.as_deref() != Some(id)
            || !Path::new(&source.path).starts_with(&original_root)
            || source.bytes.len() > MAX_SOURCE_BYTES
            || source.stamp.size != source.bytes.len() as u64
        {
            return Err(invalid());
        }
        valid_operation_id(&source.revision)?;
        let birth = self
            .admitted_binding(&source.stamp.identity)?
            .ok_or_else(invalid)?;
        if binding.workspace_id.as_ref() != Some(&birth.workspace_id)
            || owner.is_some_and(|owner| owner != birth.node_id)
        {
            return Err(invalid());
        }
        if let Some(key) = key {
            valid_operation_id(&key)?;
        }
        Ok(binding)
    }
    pub fn saved_operation_source(
        &self,
        operation_id: &str,
    ) -> Result<Option<SavedOperationSource>> {
        self.backend.check_path(&self.data_dir)?;
        valid_operation_id(operation_id)?;
        let Some(plan) = self.operations.get(operation_id) else {
            return Ok(None);
        };
        if plan.kind != "save" || plan.transaction.is_some() {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_operations_operation_not_original_source_save(),
            ));
        }
        let definition_key = match self
            .backend
            .read_bytes(self.journal_path(operation_id, "definition-reservation"))
        {
            Ok(bytes) => Some(serde_json::from_slice::<DefinitionReservation>(&bytes)?.key),
            Err(e) if e.code == "missing" => None,
            Err(e) => return Err(e),
        };
        let committed = self.committed(operation_id)?;
        Ok(Some(SavedOperationSource {
            root: match &plan.file_save.as_ref().unwrap().scope {
                QualifiedInputScope::Library { binding } => Some(binding.root.clone()),
                QualifiedInputScope::DeviceUnassigned => None,
            },
            path: plan.path.clone(),
            receipt: if committed {
                plan.receipt.clone()
            } else {
                None
            },
            source: decode(&plan.after)?,
            definition_key,
            status: if committed {
                "committed"
            } else if self.resolved(operation_id)? {
                "resolved"
            } else {
                "recoveryPending"
            }
            .into(),
        }))
    }
    pub(crate) fn read_observation_facts(&self, operation_id: &str) -> Result<Vec<RevisionFact>> {
        let mut facts = Vec::new();
        let mut record_found = false;
        for entry in self.backend.read_dir(self.data_dir.join("journal"))? {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == format!("{operation_id}.observations.json")
                || (name.starts_with(&format!("{operation_id}.recovery-observations-"))
                    && name.ends_with(".json"))
            {
                record_found = true;
                facts.extend(serde_json::from_slice::<Vec<RevisionFact>>(
                    &self.backend.read_bytes(path)?,
                )?);
            }
        }
        if !record_found
            && self.operations.get(operation_id).is_some_and(|plan| {
                plan.content_revision.is_some()
                    || plan
                        .transaction
                        .as_ref()
                        .is_some_and(|t| !t.preview.revisions.is_empty())
            })
            && self.committed(operation_id)?
        {
            return Err(CoreError::new(
                "identityPending",
                weftext_messages::UiMessage::error_core_backend_committed_observations_missing(),
            ));
        }
        Ok(facts)
    }
    pub(crate) fn validate_recovery_scope(&self, t: &FrozenOperation) -> Result<()> {
        self.validate_resource_collection_initialization(t)?;
        if t.expected_membership.is_none() && t.expected_trash_membership.is_none() {
            return Ok(());
        }
        let root = Path::new(&t.request.root);
        let trash = root.join(".weftext-trash");
        let workspace = self.require_workspace(root)?;
        let failure = || {
            CoreError::new(
                "scopeChanged",
                UiMessage::error_core_operations_original_lifecycle_membership_no_longer_has_exact_captured_physical_objects(),
            )
        };
        let in_trash = |path: &Path| -> Result<bool> {
            Ok(backend::namespace_relative_path(path, &trash)?.is_some())
        };
        let mut members = BTreeSet::new();
        for plan in self.committed_plans()?.into_iter() {
            for birth in self.read_births(&plan.operation_id)? {
                if birth.workspace_id == workspace.workspace_id {
                    members.insert(birth.node_id);
                }
            }
        }
        let mut trash_members = self
            .admitted_trash_files(root)?
            .iter()
            .map(|file| self.admitted_binding(&file.stamp.identity))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .filter(|birth| birth.workspace_id == workspace.workspace_id)
            .map(|birth| birth.node_id)
            .collect::<BTreeSet<_>>();

        // Project only this original transaction's captured Node objects back
        // to their before lifecycle. Foreign late members remain in the set.
        let mut positions = match &t.mutation {
            Mutation::Compound { .. } => compound::compound_membership_positions(&self.backend, t)?,
            _ => BTreeMap::new(),
        };
        let relocations = t.relocations();
        let mut held_roots = Vec::new();
        for step in &relocations {
            if !matches!(t.mutation, Mutation::Compound { .. }) {
                for candidate in BTreeSet::from([step.source, step.target]) {
                    if !Path::new(candidate).try_exists()? {
                        continue;
                    }
                    let guard = ParentGuard::acquire(
                        &self.backend,
                        &Path::new(candidate).join(".recovery-membership"),
                    )?;
                    if guard.directory_identity()? == *step.source_identity {
                        if positions
                            .insert(step.source.into(), PathBuf::from(candidate))
                            .is_some()
                        {
                            return Err(failure());
                        }
                        held_roots.push(guard);
                    }
                }
            }
            let current = positions.get(step.source).ok_or_else(failure)?;
            let guard = ParentGuard::acquire(&self.backend, &current.join(".recovery-membership"))?;
            if guard.directory_identity()? != *step.source_identity {
                return Err(failure());
            }
            held_roots.push(guard);
        }
        let mut held_files = Vec::new();
        for original in &t.files {
            let Some(birth) = self
                .admitted_binding(&original.stamp.identity)?
                .filter(|birth| birth.workspace_id == workspace.workspace_id)
            else {
                continue;
            };
            let mut owner = None;
            for step in &relocations {
                if let Some(relative) = backend::namespace_relative_path(
                    Path::new(&original.path),
                    Path::new(step.source),
                )? {
                    let depth = Path::new(step.source).components().count();
                    if owner.as_ref().is_none_or(|(prior, _, _)| depth > *prior) {
                        owner = Some((depth, step, relative));
                    }
                }
            }
            let Some((_, step, relative)) = owner else {
                continue;
            };
            let current = &positions[step.source];
            let expected_parent = t
                .directories
                .iter()
                .find(|directory| {
                    locator_equal(
                        &directory.path,
                        &display(Path::new(&original.path).parent().unwrap()),
                    )
                })
                .map(|directory| &directory.identity)
                .or_else(|| {
                    step.members
                        .iter()
                        .find(|member| member.identity == original.stamp.identity)
                        .map(|member| &member.parent)
                })
                .or_else(|| {
                    (Path::new(&original.path).parent() == Some(Path::new(step.source)))
                        .then_some(step.source_identity)
                })
                .ok_or_else(failure)?;
            let mut candidates = BTreeSet::from([current.join(&relative)]);
            if relative == Path::new(step.old_leaf) {
                candidates.insert(current.join(step.new_leaf));
            }
            let mut found = false;
            for candidate in candidates {
                if !candidate.try_exists()? {
                    continue;
                }
                let guard = ParentGuard::acquire(&self.backend, &candidate)?;
                let file = guard.open_file(false, false)?;
                if stamp(&file)?.identity != original.stamp.identity {
                    continue;
                }
                if found || guard.directory_identity()? != *expected_parent {
                    return Err(failure());
                }
                found = true;
                held_roots.push(guard);
                held_files.push(file);
            }
            if !found {
                return Err(failure());
            }
            if in_trash(Path::new(&original.path))? {
                trash_members.insert(birth.node_id);
            } else {
                trash_members.remove(&birth.node_id);
            }
        }

        // Post-commit inspection also compares against the original before
        // cut. Only exact births materialized by this same Plan are removed.
        for birth in self.read_births(&t.request.operation_id)? {
            let Some(planned) = t
                .births()
                .iter()
                .find(|planned| planned.node_id == birth.node_id)
            else {
                continue;
            };
            if birth.workspace_id != workspace.workspace_id
                || self
                    .materialized_identity(&t.request.operation_id, Path::new(&planned.path))?
                    .as_ref()
                    != Some(&birth.identity)
            {
                return Err(failure());
            }
            let guard = ParentGuard::acquire(&self.backend, Path::new(&planned.path))?;
            let file = guard.open_file(false, false)?;
            if stamp(&file)?.identity != birth.identity {
                return Err(failure());
            }
            held_roots.push(guard);
            held_files.push(file);
            members.remove(&birth.node_id);
            trash_members.remove(&birth.node_id);
        }
        members.retain(|id| !trash_members.contains(id));
        if t.expected_membership
            .as_ref()
            .is_some_and(|expected| &members != expected)
            || t.expected_trash_membership
                .as_ref()
                .is_some_and(|expected| &trash_members != expected)
        {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_operations_original_strong_operation_s_admitted_membership_changed_no_replay_or_barrier_release(),
            ));
        }
        Ok(())
    }
    pub(crate) fn publish_recovery_facts(
        &self,
        plan: &journal::Plan,
        materialized: &[(String, FileStamp)],
    ) -> Result<()> {
        let t = plan.transaction.as_ref().ok_or_else(|| {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_operations_recovery_needs_original_transaction(),
            )
        })?;
        let stamps = materialized.iter().cloned().collect::<BTreeMap<_, _>>();
        let relocated = t.relocated_files();
        let output = match &t.mutation {
            Mutation::Update { files } => files.iter().collect::<Vec<_>>(),
            Mutation::CreateTree { files, updates, .. } => files.iter().chain(updates).collect(),
            Mutation::Relocate { .. } | Mutation::Compound { .. } => relocated.iter().collect(),
        };
        let facts = output
            .iter()
            .filter_map(|c| t.preview.revisions.get(&c.path).map(|r| (c, r)))
            .map(|(c, r)| {
                Ok(RevisionFact {
                    path: c.path.clone(),
                    stamp: stamps
                        .get(&c.path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "recoveryUnavailable",
                                UiMessage::error_core_operations_recovery_member_stamp_missing(),
                            )
                        })?
                        .clone(),
                    hash: hash(&c.after),
                    revision: r.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let record = self.journal_path(&plan.operation_id, "observations");
        if self.backend.entry_exists(&record)? {
            journal::private_create(
                &self.backend,
                &self.journal_path(
                    &plan.operation_id,
                    &format!("recovery-observations-{}", fresh()),
                ),
                &serde_json::to_vec(&facts)?,
            )?;
        } else {
            journal::private_create(&self.backend, &record, &serde_json::to_vec(&facts)?)?;
        }
        if !t.births().is_empty() {
            let births = t.births();
            let mut actual = Vec::new();
            for birth in births {
                let observed = stamps.get(&birth.path).ok_or_else(|| {
                    CoreError::new(
                        "recoveryUnavailable",
                        UiMessage::error_core_operations_birth_member_missing(),
                    )
                })?;
                if self
                    .materialized_identity(&plan.operation_id, Path::new(&birth.path))?
                    .as_ref()
                    != Some(&observed.identity)
                {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_proposed_new_node_not_original_materialized_object(),
                    ));
                }
                actual.push(Birth {
                    node_id: birth.node_id.clone(),
                    workspace_id: plan.workspace_id.clone().unwrap(),
                    identity: observed.identity.clone(),
                });
            }
            let record = self.journal_path(&plan.operation_id, "births");
            if self.backend.entry_exists(&record)? {
                let old: Vec<Birth> = serde_json::from_slice(&self.backend.read_bytes(&record)?)?;
                if old.len() != actual.len()
                    || old.iter().any(|b| {
                        !actual.iter().any(|a| {
                            a.node_id == b.node_id
                                && a.workspace_id == b.workspace_id
                                && a.identity == b.identity
                        })
                    })
                {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_retained_birth_binding_disagrees_with_original_materialized_objects(),
                    ));
                }
            } else if !actual.is_empty() {
                journal::private_create(&self.backend, &record, &serde_json::to_vec(&actual)?)?;
            }
        }
        let mut resources = Vec::new();
        for (path, reference) in &t.preview.resource_map {
            let stamp = stamps.get(path).ok_or_else(|| {
                CoreError::new(
                    "recoveryUnavailable",
                    UiMessage::error_core_operations_fresh_resource_member_missing(),
                )
            })?;
            if self
                .materialized_identity(&plan.operation_id, Path::new(path))?
                .as_ref()
                != Some(&stamp.identity)
            {
                return Err(CoreError::new(
                    "thirdState",
                    UiMessage::error_core_operations_resource_not_original_materialized_object(),
                ));
            }
            let node_id = match &t.mutation {
                Mutation::CreateTree { .. } if plan.sync_application.is_some() => {
                    t.updated_resource_owner(Path::new(path), plan.sync_application.as_ref())
                }
                Mutation::CreateTree { births, .. } => births
                    .iter()
                    .filter(|b| Path::new(path).starts_with(Path::new(&b.path).parent().unwrap()))
                    .max_by_key(|b| Path::new(&b.path).components().count())
                    .map(|b| b.node_id.clone()),
                Mutation::Update { .. } | Mutation::Relocate { .. } | Mutation::Compound { .. } => {
                    t.updated_resource_owner(Path::new(path), plan.sync_application.as_ref())
                }
            }
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_operations_resource_owner_missing(),
                )
            })?;
            resources.push(ResourceBirth {
                resource_id: reference.rsplit('/').next().unwrap().into(),
                workspace_id: plan.workspace_id.clone().unwrap(),
                node_id,
                identity: stamp.identity.clone(),
            });
        }
        let record = self.journal_path(&plan.operation_id, "resource-births");
        if !resources.is_empty() {
            if self.backend.entry_exists(&record)? {
                let old: Vec<ResourceBirth> =
                    serde_json::from_slice(&self.backend.read_bytes(&record)?)?;
                if old.len() != resources.len()
                    || old.iter().any(|b| {
                        !resources.iter().any(|a| {
                            a.resource_id == b.resource_id
                                && a.node_id == b.node_id
                                && a.identity == b.identity
                        })
                    })
                {
                    return Err(CoreError::new(
                        "thirdState",
                        UiMessage::error_core_operations_retained_resource_bindings_differ(),
                    ));
                }
            } else {
                journal::private_create(&self.backend, &record, &serde_json::to_vec(&resources)?)?;
            }
        }
        Ok(())
    }
    pub(crate) fn record_materialized(
        &self,
        operation_id: &str,
        path: &Path,
        file: &backend::BoundFile,
    ) -> Result<()> {
        journal::private_create(
            &self.backend,
            &self.journal_path(
                operation_id,
                &format!("object-{}", hash(display(path).as_bytes())),
            ),
            &serde_json::to_vec(
                &serde_json::json!({"path":display(path),"identity":stamp(file)?.identity}),
            )?,
        )
    }
    pub(crate) fn materialized_identity(
        &self,
        operation_id: &str,
        path: &Path,
    ) -> Result<Option<FileIdentity>> {
        match self.backend.read_bytes(self.journal_path(
            operation_id,
            &format!("object-{}", hash(display(path).as_bytes())),
        )) {
            Ok(bytes) => {
                let v: serde_json::Value = serde_json::from_slice(&bytes)?;
                if v["path"] != display(path) {
                    return Err(CoreError::new(
                        "invalidState",
                        UiMessage::error_core_operations_materialized_object_record_path_mismatch(),
                    ));
                }
                Ok(Some(serde_json::from_value(v["identity"].clone())?))
            }
            Err(e) if e.code == "missing" => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn record_observation(
        &self,
        operation_id: &str,
        path: &Path,
        stamp: &FileStamp,
        bytes: &[u8],
        revision: &str,
    ) -> Result<()> {
        journal::private_create(
            &self.backend,
            &self.journal_path(operation_id, "observations"),
            &serde_json::to_vec(&vec![RevisionFact {
                path: display(path),
                stamp: stamp.clone(),
                hash: hash(bytes),
                revision: revision.into(),
            }])?,
        )
    }
    pub fn prepared_operation_files(&self, operation_id: &str) -> Result<Vec<FileChange>> {
        self.backend.check_path(&self.data_dir)?;
        self.check_sync_operation_access(operation_id)?;
        let t = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    UiMessage::error_core_operations_no_frozen_operation_has_i_d(),
                )
            })?;
        Ok(match &t.mutation {
            Mutation::Update { files } => files.clone(),
            Mutation::CreateTree { files, updates, .. } => {
                files.iter().chain(updates).cloned().collect()
            }
            Mutation::Relocate { updates, .. } => updates.clone(),
            Mutation::Compound {
                updates,
                created_files,
                ..
            } => updates.iter().chain(created_files).cloned().collect(),
        })
    }
    pub fn committed_operation_receipt(
        &self,
        operation_id: &str,
    ) -> Result<Option<OperationReceipt>> {
        self.backend.check_path(&self.data_dir)?;
        let p = self.operations.get(operation_id).ok_or_else(|| {
            CoreError::new(
                "operationMissing",
                UiMessage::error_core_operations_no_original_operation_has_i_d(),
            )
        })?;
        let t = p.transaction.as_ref().ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                UiMessage::error_core_operations_i_d_belongs_to_legacy_operation(),
            )
        })?;
        Ok(self.committed(operation_id)?.then(|| t.receipt.clone()))
    }
    pub fn reserve_definition_key(
        &mut self,
        root: impl AsRef<Path>,
        owner_path: impl AsRef<Path>,
        base_id: &str,
        operation_id: &str,
    ) -> Result<String> {
        valid_operation_id(operation_id)?;
        self.check_attempt_not_cancelled(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(owner_path.as_ref())?;
        let current = self.managed_node(
            &root,
            path.parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_missing_source_parent(),
                )
            })?,
            false,
        )?;
        if current.path != display(&path) {
            return Err(CoreError::new(
                "unsupportedRole",
                UiMessage::error_core_operations_definitions_require_canonical_node_source(),
            ));
        }
        let owner = self.admitted_binding(&current.stamp.identity)?.unwrap();
        let record = self.journal_path(operation_id, "definition-reservation");
        if let Ok(bytes) = self.backend.read_bytes(&record) {
            let old: DefinitionReservation = serde_json::from_slice(&bytes)?;
            if old.owner_id == owner.node_id && self.committed(operation_id)? {
                return Ok(old.key);
            }
            if old.owner_id != owner.node_id
                || old.source.stamp != current.stamp
                || old.source.bytes != current.bytes
            {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_operations_definition_reservation_already_belongs_to_different_source_basis(),
                ));
            }
            return Ok(old.key);
        }
        self.verify_snapshot(&path, base_id, Some(&current))?;
        let key = fresh();
        let revision = self.accepted_revision(&current)?.unwrap_or_else(fresh);
        journal::private_create(
            &self.backend,
            &record,
            &serde_json::to_vec(&DefinitionReservation {
                library: self.draft_library_for_root(&root)?,
                operation_id: Some(operation_id.into()),
                owner_id: owner.node_id,
                source: RevisionReservation {
                    library: None,
                    operation_id: None,
                    path: current.path,
                    stamp: current.stamp,
                    bytes: current.bytes,
                    revision,
                },
                key: key.clone(),
            })?,
        )?;
        Ok(key)
    }
    /// Recover the original local reservation from its actual owner and key.
    /// This does not admit a definition or grant a writer to a matching title.
    pub fn pending_definition_operation(
        &self,
        root: &Path,
        path: &Path,
        key: &str,
    ) -> Result<Option<String>> {
        let node = self.managed_node(
            root,
            path.parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_operations_missing_owner(),
                )
            })?,
            false,
        )?;
        if node.path != display(path) {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_not_canonical_owner(),
            ));
        }
        let owner = self
            .admitted_binding(&node.stamp.identity)?
            .ok_or_else(|| {
                CoreError::new(
                    "identityConflict",
                    weftext_messages::UiMessage::error_core_operations_owner_not_admitted(),
                )
            })?;
        let directory = self.journal_path("probe", "definition-reservation");
        let mut found = None;
        for entry in self.backend.read_dir(directory.parent().unwrap())? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(id) = name.strip_suffix(".definition-reservation.json") else {
                continue;
            };
            let reservation: DefinitionReservation =
                serde_json::from_slice(&self.backend.read_bytes(entry.path())?)?;
            if reservation.owner_id == owner.node_id
                && reservation.source.stamp.identity == node.stamp.identity
                && reservation.key == key
                && !self.committed(id)?
                && !self.resolved(id)?
            {
                if found.is_some() {
                    return Err(CoreError::new(
                        "operationConflict",
                        UiMessage::error_core_operations_ambiguous_definition_reservation(),
                    ));
                }
                found = Some(id.to_string());
            }
        }
        Ok(found)
    }
    pub fn is_definition_key_admitted(
        &self,
        root: impl AsRef<Path>,
        owner_id: &str,
        key: &str,
    ) -> Result<bool> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let owner = weftext_knowledge::NodeRef {
            workspace_id: workspace.workspace_id,
            node_id: owner_id.into(),
        };
        for p in self.committed_plans()?.into_iter() {
            if self.hosted_definition_key_admitted(p, &owner, key)? {
                return Ok(true);
            }
            if p.workspace_id.as_deref() == Some(owner.workspace_id.as_str())
                && let Some(application) = &p.sync_application
                && let Some(evidence) = &application.semantic_evidence
                && let Some(after) = application
                    .after
                    .iter()
                    .find(|after| after.object == sync::ObjectKey::Node(owner.node_id.clone()))
            {
                let name = format!("{}.typ", after.state.name);
                let mut sources = Vec::new();
                if let (Some(revision), Some(digest)) = (
                    after.state.file_revisions.get(&name),
                    after.state.files.get(&name),
                ) {
                    sources.push((revision.clone(), digest.clone()));
                }
                for value in &evidence.historical_inputs {
                    let input: sync::revalidation::HistoricalSemanticInput =
                        serde_json::from_value(value.clone())?;
                    if input.object == after.object {
                        sources.push((input.file_revision, input.digest));
                    }
                }
                for (revision, digest) in sources {
                    let proof = serde_json::json!({"kind":"definition","owner":owner,"key":key,"sourceRevision":revision});
                    if evidence
                        .discharged
                        .iter()
                        .any(|discharged| discharged == &proof)
                        && application.identity.workspace_id == owner.workspace_id
                        && let Some(bytes) = application.blobs.get(&digest)
                        && hash(bytes) == digest
                        && std::str::from_utf8(bytes).is_ok_and(|source| {
                            weftext_engine::inspect_metadata(source).id.as_deref() == Some(owner_id)
                                && weftext_knowledge::definition_slots(source, &owner).is_ok_and(
                                    |slots| {
                                        slots.iter().filter(|slot| slot.key == key).count() == 1
                                    },
                                )
                        })
                    {
                        return Ok(true);
                    }
                }
            }
            let reservation = self.backend.read_optional_json::<DefinitionReservation>(
                self.journal_path(&p.operation_id, "definition-reservation"),
            )?;
            if let Some(r) = reservation
                && r.owner_id == owner_id
                && r.key == key
            {
                let after =
                    if p.kind == "save" && p.identity.as_ref() == Some(&r.source.stamp.identity) {
                        Some(&p.after)
                    } else {
                        p.transaction.as_ref().and_then(|t| match &t.mutation {
                            Mutation::Update { files } => files
                                .iter()
                                .find(|f| f.path == r.source.path)
                                .map(|f| &f.after),
                            _ => None,
                        })
                    };
                if after
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .is_some_and(|s| {
                        weftext_knowledge::definition_slots(s, &owner)
                            .is_ok_and(|slots| slots.iter().any(|slot| slot.key == key))
                    })
                {
                    return Ok(true);
                }
            }
            if p.workspace_id.as_deref() == Some(owner.workspace_id.as_str())
                && let Some(t) = &p.transaction
                && t.preview.node_map.values().any(|n| n == owner_id)
            {
                // Only the original canonical birth source can establish a
                // definition. Resources and configuration may contain arbitrary
                // UTF-8, including another copy of the same Node declaration.
                let mut births = t.births().iter().filter(|birth| birth.node_id == owner_id);
                let Some(birth) = births.next() else { continue };
                if births.next().is_some() {
                    continue;
                }
                let mut files = t
                    .created_files()
                    .iter()
                    .filter(|file| Path::new(&file.path) == Path::new(&birth.path));
                let Some(file) = files.next() else { continue };
                if files.next().is_none()
                    && std::str::from_utf8(&file.after).is_ok_and(|s| {
                        weftext_engine::inspect_metadata(s).id.as_deref() == Some(owner_id)
                            && weftext_knowledge::definition_slots(s, &owner)
                                .is_ok_and(|slots| slots.iter().any(|slot| slot.key == key))
                    })
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub fn read_source_revision(
        &self,
        root: impl AsRef<Path>,
        owner_id: &str,
        revision: &str,
    ) -> Result<Vec<u8>> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let inventory = self.inventory(&root)?;
        let current = inventory
            .entries
            .iter()
            .filter(|e| e.node_id.as_deref() == Some(owner_id) && e.identity_status == "admitted")
            .collect::<Vec<_>>();
        if current.len() != 1 {
            return Err(CoreError::new(
                "revisionUnavailable",
                UiMessage::error_core_operations_current_owner_missing_duplicated_or_unqualified(),
            ));
        }
        self.retained_source_revision(&workspace.workspace_id, owner_id, revision)
    }

    /// Read one configured owner's exact retained revision without enumerating
    /// the library. Hosted callers supply the operator-bound canonical path and
    /// authorize definition/history access before calling this method.
    pub fn read_source_revision_at(
        &self,
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
        owner_id: &str,
        revision: &str,
    ) -> Result<Vec<u8>> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let current = self.managed_node(
            &root,
            path.parent().ok_or_else(|| {
                CoreError::new(
                    "revisionUnavailable",
                    UiMessage::error_core_operations_configured_owner_has_no_parent(),
                )
            })?,
            false,
        )?;
        if Path::new(&current.path) != path
            || self
                .admitted_binding(&current.stamp.identity)?
                .is_none_or(|birth| birth.node_id != owner_id)
            || self.accepted_revision(&current)?.is_none()
        {
            return Err(CoreError::new(
                "revisionUnavailable",
                UiMessage::error_core_operations_exact_configured_owner_not_currently_admitted(),
            ));
        }
        self.retained_source_revision(&workspace.workspace_id, owner_id, revision)
    }

    fn retained_source_revision(
        &self,
        workspace_id: &str,
        owner_id: &str,
        revision: &str,
    ) -> Result<Vec<u8>> {
        for p in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && (p.workspace_id.as_deref() == Some(workspace_id)
                                || (p.kind == "save"
                                    && p.identity
                                        .as_ref()
                                        .map(|id| self.admitted_binding(id))
                                        .transpose()?
                                        .flatten()
                                        .is_some_and(|birth| {
                                            birth.workspace_id == workspace_id
                                                && birth.node_id == owner_id
                                        })))
                    } {
                        Some(p)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            if let Ok(bytes) = self.revision_payload(p, revision)
                && weftext_engine::inspect_metadata(&decode(&bytes)?)
                    .id
                    .as_deref()
                    == Some(owner_id)
            {
                return Ok(bytes);
            }
            if let Ok(bytes) = self
                .backend
                .read_bytes(self.journal_path(&p.operation_id, "revision-reservation"))
            {
                let r: RevisionReservation = serde_json::from_slice(&bytes)?;
                if r.revision == revision
                    && weftext_engine::inspect_metadata(&decode(&r.bytes)?)
                        .id
                        .as_deref()
                        == Some(owner_id)
                    && p.transaction.as_ref().is_some_and(|t| {
                        t.files
                            .iter()
                            .any(|f| f.stamp == r.stamp && f.bytes == r.bytes)
                    })
                {
                    return Ok(r.bytes);
                }
            }
        }
        Err(CoreError::new(
            "revisionUnavailable",
            UiMessage::error_core_operations_no_committed_retained_exact_source_revision_has_i_d_latest_was_not_substituted(),
        ))
    }
    pub fn read_config_revision(
        &self,
        root: impl AsRef<Path>,
        role: ConfigRole,
        revision: &str,
    ) -> Result<Vec<u8>> {
        self.backend.check_path(&self.data_dir)?;
        let root = absolute_path(root.as_ref())?;
        self.require_workspace(&root)?;
        let path = display(&root.join(role.filename()));
        for p in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if self.committed(&p.operation_id)? && p.workspace_id.is_some() {
                        Some(p)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            if p.transaction.as_ref().is_some_and(|t| {
                t.request.root == display(&root)
                    && t.preview
                        .revisions
                        .get(&path)
                        .is_some_and(|r| r == revision)
            }) {
                return self.revision_payload(p, revision);
            }
        }
        Err(CoreError::new(
            "revisionUnavailable",
            UiMessage::error_core_operations_no_retained_exact_config_revision_has_i_d(),
        ))
    }
    fn revision_payload(&self, p: &journal::Plan, revision: &str) -> Result<Vec<u8>> {
        if matches!(p.kind.as_str(), "save" | "saveAs") {
            self.validate_file_save_plan(p)?;
        }
        let facts = self.read_observation_facts(&p.operation_id)?;
        let fact = facts
            .iter()
            .find(|f| f.revision == revision)
            .ok_or_else(|| {
                CoreError::new(
                    "revisionUnavailable",
                    UiMessage::error_core_operations_no_decision_witness_for_revision(),
                )
            })?;
        let relocated = p
            .transaction
            .as_ref()
            .map(|t| t.relocated_files())
            .unwrap_or_default();
        let bytes = if matches!(p.kind.as_str(), "save" | "createNote") {
            Some(&p.after)
        } else {
            p.transaction.as_ref().and_then(|t| match &t.mutation {
                Mutation::Update { files } => {
                    files.iter().find(|f| f.path == fact.path).map(|f| &f.after)
                }
                Mutation::CreateTree { files, updates, .. } => files
                    .iter()
                    .chain(updates)
                    .find(|f| f.path == fact.path)
                    .map(|f| &f.after),
                Mutation::Relocate { .. } | Mutation::Compound { .. } => relocated
                    .iter()
                    .find(|f| f.path == fact.path)
                    .map(|f| &f.after),
            })
        };
        bytes
            .filter(|b| hash(b) == fact.hash)
            .cloned()
            .ok_or_else(|| {
                CoreError::new(
                    "revisionUnavailable",
                    UiMessage::error_core_operations_retained_exact_bytes_unavailable(),
                )
            })
    }

    pub(crate) fn admitted_resource(
        &self,
        id: &str,
        workspace: &str,
        node: &str,
        identity: &FileIdentity,
    ) -> Result<bool> {
        for plan in self.committed_plans()? {
            if self.resource_births(plan)?.iter().any(|birth| {
                birth.resource_id == id
                    && birth.workspace_id == workspace
                    && birth.node_id == node
                    && &birth.identity == identity
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn resource_births(&self, plan: &journal::Plan) -> Result<Vec<ResourceBirth>> {
        let bytes = match self
            .backend
            .read_bytes(self.journal_path(&plan.operation_id, "resource-births"))
        {
            Ok(bytes) => bytes,
            Err(error)
                if error.code == "missing"
                    && plan
                        .transaction
                        .as_ref()
                        .is_none_or(|t| t.preview.resource_map.is_empty()) =>
            {
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };
        let births: Vec<ResourceBirth> = serde_json::from_slice(&bytes)?;
        if births.iter().any(|birth| {
            birth.resource_id.is_empty()
                || birth.node_id.is_empty()
                || birth.workspace_id.is_empty()
        }) || plan
            .transaction
            .as_ref()
            .is_some_and(|t| !t.preview.resource_map.is_empty() && births.is_empty())
        {
            return Err(CoreError::new(
                "identityPending",
                weftext_messages::UiMessage::error_core_backend_resource_birth_incomplete(),
            ));
        }
        Ok(births)
    }
    pub(crate) fn resource_owner_born(
        &self,
        id: &str,
        owner: &weftext_knowledge::NodeRef,
    ) -> Result<bool> {
        for plan in self.committed_plans()? {
            if self.resource_births(plan)?.iter().any(|birth| {
                birth.resource_id == id
                    && birth.workspace_id == owner.workspace_id
                    && birth.node_id == owner.node_id
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn managed_resource_snapshot(
        &mut self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
    ) -> Result<Vec<ManagedResource>> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(node_source.as_ref())?;
        let directory = path.parent().ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                UiMessage::error_core_operations_missing_node_parent(),
            )
        })?;
        let in_trash = directory.strip_prefix(&root).is_ok_and(|relative| {
            relative.components().next().is_some_and(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(".weftext-trash")
            })
        });
        let node = if in_trash {
            let original = capture_optional(&self.backend, &path)?.ok_or_else(|| {
                CoreError::new("missingNode", weftext_messages::UiMessage::error_core_operations_retained_canonical_source_missing())
            })?;
            let birth = self
                .admitted_binding(&original.stamp.identity)?
                .ok_or_else(|| {
                    CoreError::new(
                        "identityPending",
                        UiMessage::error_core_operations_retained_source_has_no_original_admission(
                        ),
                    )
                })?;
            let qualified = self.qualified_query_node_at(
                &root,
                &path,
                &weftext_knowledge::NodeRef {
                    workspace_id: birth.workspace_id,
                    node_id: birth.node_id,
                },
                "trash",
            )?;
            let mut witnesses = vec![];
            Self::merge_query_node_feature_witness(&mut witnesses, &qualified.witness)?;
            let current = witnesses
                .into_iter()
                .find(|witness| locator_equal(&witness.binding.path, &display(&path)))
                .and_then(|witness| witness.binding.captured)
                .ok_or_else(|| {
                    CoreError::new(
                        "identityPending",
                        UiMessage::error_core_operations_missing_original_source_witness(),
                    )
                })?;
            if current.stamp != original.stamp || current.bytes != original.bytes {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_retained_source_changed_during_capture(),
                ));
            }
            current
        } else {
            self.managed_node(&root, directory, false)?
        };
        let birth = self.admitted_binding(&node.stamp.identity)?.unwrap();
        let manifest_path = directory.join("weftext.resources.json");
        let manifest = self
            .read_snapshot(&root, &[display(&manifest_path)])?
            .files
            .remove(0);
        let Some(bytes) = manifest.bytes.clone() else {
            return Ok(Vec::new());
        };
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        if value["version"] != 1 {
            return Err(CoreError::new(
                "resourceManifest",
                UiMessage::error_core_operations_unsupported_resource_manifest_version(),
            ));
        }
        let resources = value["resources"].as_array().ok_or_else(|| {
            CoreError::new(
                "resourceManifest",
                UiMessage::error_core_operations_missing_resources_array(),
            )
        })?;
        if resources.len() > 64 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_operations_at_most64_registered_resources_per_snapshot(),
            ));
        }
        let mut paths = Vec::new();
        let mut ids = BTreeSet::new();
        for r in resources {
            let id = r["resourceId"].as_str().ok_or_else(|| {
                CoreError::new(
                    "resourceManifest",
                    UiMessage::error_core_operations_missing_resource_i_d(),
                )
            })?;
            valid_operation_id(id)?;
            if !ids.insert(id) {
                return Err(CoreError::new(
                    "resourceManifest",
                    UiMessage::error_core_operations_duplicate_resource_i_d(),
                ));
            }
            let name = r["path"].as_str().ok_or_else(|| {
                CoreError::new(
                    "resourceManifest",
                    UiMessage::error_core_operations_missing_resource_path(),
                )
            })?;
            paths.push(display(&resource_path(directory, name)?));
        }
        if paths.is_empty() {
            return Ok(vec![]);
        }
        paths.push(display(&manifest_path));
        let cut = self.read_snapshot(&root, &paths)?;
        let current_manifest = cut.files.last().unwrap();
        if current_manifest.bytes != manifest.bytes {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_operations_resource_manifest_changed_while_preparing_exact_cut(),
            ));
        }
        let mut result = Vec::new();
        for (r, f) in resources.iter().zip(&cut.files) {
            let id = r["resourceId"].as_str().unwrap();
            let Some(bytes) = f.bytes.clone() else {
                return Err(CoreError::new(
                    "resourceMissing",
                    UiMessage::error_core_operations_registered_resource_file_absent(),
                ));
            };
            let capture = self.snapshots[&f.base_id].captured.as_ref().unwrap();
            let admitted = self.admitted_resource(
                id,
                &birth.workspace_id,
                &birth.node_id,
                &capture.stamp.identity,
            )?;
            result.push(ManagedResource {
                resource_id: id.into(),
                resource_ref: format!("weftext-resource://{}/{id}", birth.workspace_id),
                node_id: birth.node_id.clone(),
                path: f.path.clone(),
                bytes,
                base_id: f.base_id.clone(),
                revision: if admitted { f.revision.clone() } else { None },
                identity_status: if admitted { "admitted" } else { "unadmitted" }.into(),
                manifest_path: current_manifest.path.clone(),
                manifest_base_id: current_manifest.base_id.clone(),
                manifest_bytes: current_manifest.bytes.clone().unwrap(),
            });
        }
        Ok(result)
    }
    pub fn prepare_import_resource(
        &mut self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
        name: &str,
        bytes: Vec<u8>,
        operation_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(node_source.as_ref())?;
        valid_name(name)?;
        if let Some(t) = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
        {
            if t.request.root == display(&root)
                && matches!(&t.request.intent,OperationIntent::ImportResource{node,name:old_name,bytes:old_bytes,..}if node==&display(&path)&&old_name==name&&old_bytes==&bytes)
            {
                return self.operation_details(operation_id);
            }
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_operations_operation_id_belongs_to_different_import_request(),
            ));
        }
        let directory = path.parent().ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                UiMessage::error_core_operations_missing_resource_owner(),
            )
        })?;
        self.managed_node(&root, directory, false)?;
        let manifest = directory.join("weftext.resources.json");
        let target = directory.join(name);
        let cut = self.read_snapshot(&root, &[display(&manifest), display(&target)])?;
        self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(&root),
            intent: OperationIntent::ImportResource {
                node: display(&path),
                name: name.into(),
                bytes,
                manifest_base_id: cut.files[0].base_id.clone(),
                resource_base_id: cut.files[1].base_id.clone(),
            },
        })
    }
    pub fn has_library(&self, root: impl AsRef<Path>) -> Result<bool> {
        let root = absolute_path(root.as_ref())?;
        if self.workspace(&root)?.is_some() {
            return Ok(true);
        }
        if self.backend.entry_exists(root.join(".weftext-format"))?
            || self
                .backend
                .entry_exists(root.join(".weftext-workspace.json"))?
        {
            return Err(CoreError::new(
                "unadmittedLibrary",
                UiMessage::error_core_operations_existing_library_controls_incomplete_or_lack_their_original_private_journal(),
            ));
        }
        Ok(false)
    }
    pub fn validate_snapshot(
        &self,
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
        base_id: &str,
    ) -> Result<()> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        self.require_workspace(&root)?;
        if !path.starts_with(&root) {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_snapshot_belongs_outside_library(),
            ));
        }
        self.check_pending(&path)?;
        self.verify_snapshot(
            &path,
            base_id,
            capture_optional(&self.backend, &path)?.as_ref(),
        )
    }
    pub fn take_snapshot_witness(&mut self, base_id: &str) -> Result<SnapshotWitness> {
        self.snapshots
            .remove(base_id)
            .map(|binding| SnapshotWitness { binding })
            .ok_or_else(|| {
                CoreError::new(
                    "basisRequired",
                    UiMessage::error_core_operations_original_snapshot_unavailable(),
                )
            })
    }
    /// Retain the same original read evidence for another bounded consumer.
    /// This does not observe current bytes or grant a write capability.
    pub fn snapshot_witness(&self, base_id: &str) -> Result<SnapshotWitness> {
        self.backend.check_path(&self.data_dir)?;
        self.snapshots
            .get(base_id)
            .cloned()
            .map(|binding| SnapshotWitness { binding })
            .ok_or_else(|| {
                CoreError::new(
                    "basisRequired",
                    UiMessage::error_core_operations_original_snapshot_unavailable(),
                )
            })
    }
    pub fn validate_snapshot_witness(
        &self,
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
        witness: &SnapshotWitness,
    ) -> Result<()> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        self.require_workspace(&root)?;
        if crate::history::history_relative_path(&path, &root).is_none() {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_snapshot_belongs_outside_library(),
            ));
        }
        self.check_pending(&path)?;
        self.verify_snapshot_binding(
            &path,
            &witness.binding,
            capture_optional(&self.backend, &path)?.as_ref(),
        )
    }
    /// Consume each validation's owned witnesses into one bounded exact pool.
    /// Duplicates are dropped only after their complete versions agree. An
    /// error empties the pool; the caller must discard the failed save request.
    pub fn merge_snapshot_witnesses(
        pool: &mut Vec<SnapshotWitness>,
        incoming: Vec<SnapshotWitness>,
    ) -> Result<()> {
        Self::merge_dependency_witnesses(pool, incoming, false)
    }
    /// Feature plans preserve original absent dependencies too. This does not
    /// turn an absent observation into a continuously held positive guard.
    pub fn merge_feature_witnesses(
        pool: &mut Vec<SnapshotWitness>,
        incoming: Vec<SnapshotWitness>,
    ) -> Result<()> {
        Self::merge_dependency_witnesses(pool, incoming, true)
    }
    fn merge_dependency_witnesses(
        pool: &mut Vec<SnapshotWitness>,
        incoming: Vec<SnapshotWitness>,
        allow_absent: bool,
    ) -> Result<()> {
        let original = std::mem::take(pool);
        if original.len() > 2048 || incoming.len() > 2048 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_operations_at_most2048_original_read_witnesses_per_batch(),
            ));
        }
        let mut merged = Vec::<SnapshotWitness>::new();
        let mut total = 0usize;
        for witness in original.into_iter().chain(incoming) {
            let basis = &witness.binding;
            if let Some(existing) = merged
                .iter()
                .find(|existing| same_snapshot_dependency(&existing.binding, basis))
            {
                verify_duplicate_snapshot(&existing.binding, basis)?;
                continue;
            }
            if basis.captured.is_none() && !allow_absent {
                return Err(CoreError::new(
                    "basisRequired",
                    UiMessage::error_core_operations_guarded_definition_dependency_must_exist(),
                ));
            }
            total = total
                .checked_add(basis.captured.as_ref().map_or(0, |f| f.bytes.len()))
                .ok_or_else(|| {
                    CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_guarded_read_dependencies_exceed_their_byte_budget(),
                    )
                })?;
            if merged.len() == 64 || total > MAX_OPERATION_BYTES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_merged_read_dependencies_exceed64_files_or64_mi_b(),
                ));
            }
            merged.push(witness);
        }
        *pool = merged;
        Ok(())
    }
    /// Protect every actual read dependency through the action's durable write.
    /// The owner is excluded before opening: save acquires its own share-zero
    /// write handle and validates the current actor session through that handle.
    pub fn with_guarded_snapshot_witnesses<T>(
        &mut self,
        root: &Path,
        witnesses: &[&SnapshotWitness],
        write_owner_path: &Path,
        action: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let root = absolute_path(root)?;
        let owner = absolute_path(write_owner_path)?;
        self.require_workspace(&root)?;
        if crate::history::history_relative_path(&owner, &root).is_none() {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_writer_belongs_outside_library(),
            ));
        }
        if witnesses.len() > 2048 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_operations_at_most2048_original_read_witnesses(),
            ));
        }
        let owner_identity = {
            let file = ParentGuard::acquire(&self.backend, &owner)?.open_placement_file()?;
            stamp(&file)?.identity
        };
        let mut selected = Vec::<&SnapshotBinding>::new();
        let mut total = 0usize;
        for witness in witnesses {
            let basis = &witness.binding;
            let path = Path::new(&basis.path);
            if crate::history::history_relative_path(path, &root).is_none() {
                return Err(CoreError::new(
                    "outsideRoot",
                    UiMessage::error_core_operations_read_witness_belongs_outside_library(),
                ));
            }
            if let Some(original) = selected
                .iter()
                .find(|original| same_snapshot_dependency(original, basis))
            {
                verify_duplicate_snapshot(original, basis)?;
                continue;
            }
            let captured = basis.captured.as_ref().ok_or_else(|| {
                CoreError::new(
                    "basisRequired",
                    UiMessage::error_core_operations_guarded_definition_dependency_must_exist(),
                )
            })?;
            total = total.checked_add(captured.bytes.len()).ok_or_else(|| {
                CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_guarded_read_dependencies_exceed_their_byte_budget(),
                )
            })?;
            if total > MAX_OPERATION_BYTES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_guarded_read_dependencies_exceed64_mi_b(),
                ));
            }
            if selected.len() == 64 {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_at_most64_unique_guarded_read_dependencies(),
                ));
            }
            selected.push(basis);
        }
        let mut guards = vec![];
        let mut handles = vec![];
        for basis in selected {
            let path = Path::new(&basis.path);
            self.check_pending(path)?;
            let guard = ParentGuard::acquire(&self.backend, path)?;
            if basis
                .captured
                .as_ref()
                .is_some_and(|f| f.stamp.identity == owner_identity)
            {
                if guard.directory_identity()? != basis.parent {
                    return Err(CoreError::new(
                        "staleBase",
                        UiMessage::error_core_operations_guarded_writer_s_original_parent_changed(),
                    ));
                }
                guards.push(guard);
                continue;
            }
            let control = path.parent() == Some(root.as_path())
                && path.file_name().is_some_and(|name| {
                    matches!(
                        name.to_str(),
                        Some(
                            ".weftext-workspace.json"
                                | ".weftext-format"
                                | ".weftext-mvp-commit.json"
                                | ".weftext-rules"
                        )
                    )
                });
            let mut handle = if control {
                guard.open_read_guard_file()?
            } else {
                guard.open_file(false, false)?
            };
            let actual_stamp = stamp(&handle)?;
            self.check_pending_identity(&actual_stamp.identity)?;
            let bytes = read_blob(&mut handle)?;
            let expected = basis.captured.as_ref().unwrap();
            if guard.directory_identity()? != basis.parent
                || actual_stamp != expected.stamp
                || bytes != expected.bytes
            {
                return Err(CoreError::new(
                    "staleBase",
                    UiMessage::error_core_operations_guarded_read_dependency_changed_after_validation(),
                ));
            }
            guards.push(guard);
            handles.push(handle);
        }
        action(self)
    }
    pub fn validate_library_style(&self, root: impl AsRef<Path>, base_id: &str) -> Result<()> {
        self.backend.check_path(&self.data_dir)?;
        let root = absolute_path(root.as_ref())?;
        let b = self.style_bases.get(base_id).ok_or_else(|| {
            CoreError::new(
                "staleBase",
                UiMessage::error_core_operations_style_basis_unavailable(),
            )
        })?;
        self.validate_style_basis(&root, b)
    }
    fn validate_style_basis(&self, root: &Path, b: &StyleBasis) -> Result<()> {
        self.require_workspace(root)?;
        if b.root != display(root) {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_style_basis_belongs_to_different_library(),
            ));
        }
        if b.style.is_some() {
            self.managed_node(root, &root.join("Styles"), false)?;
        }
        for basis in [
            &b.settings,
            b.style.as_ref().unwrap_or(&b.settings),
            b.manifest.as_ref().unwrap_or(&b.settings),
            b.owner.as_ref().unwrap_or(&b.settings),
        ] {
            self.check_pending(Path::new(&basis.path))?;
            let current = capture_optional(&self.backend, Path::new(&basis.path))?;
            let parent = ParentGuard::acquire(&self.backend, Path::new(&basis.path))?;
            if basis.parent != parent.directory_identity()?
                || basis.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
                    != current.as_ref().map(|f| (&f.stamp, &f.bytes))
            {
                return Err(CoreError::new(
                    "staleBase",
                    UiMessage::error_core_operations_style_environment_changed_after_it_was_frozen(
                    ),
                ));
            }
        }
        Ok(())
    }
    pub fn take_library_style_witness(&mut self, base_id: &str) -> Result<LibraryStyleWitness> {
        self.style_bases
            .remove(base_id)
            .map(|basis| LibraryStyleWitness { basis })
            .ok_or_else(|| {
                CoreError::new(
                    "basisRequired",
                    UiMessage::error_core_operations_original_style_basis_unavailable(),
                )
            })
    }
    pub fn validate_library_style_witness(
        &self,
        root: &Path,
        witness: &LibraryStyleWitness,
    ) -> Result<()> {
        self.validate_style_basis(&absolute_path(root)?, &witness.basis)
    }
    /// Copy the original read-only style dependencies into a bounded Plan pool.
    /// Absent settings are negative observations: the original Plan rechecks
    /// absence before its decision; this does not lock a nonexistent file.
    /// Such a pool belongs to actor_feature_prepare_with_witnesses, not to the
    /// sustained positive-file guard used for saved definitions.
    pub fn merge_library_style_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &LibraryStyleWitness,
    ) -> Result<()> {
        let basis = &witness.basis;
        let incoming = [
            Some(&basis.settings),
            basis.style.as_ref(),
            basis.manifest.as_ref(),
            basis.owner.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|binding| SnapshotWitness {
            binding: binding.clone(),
        })
        .collect();
        Self::merge_dependency_witnesses(pool, incoming, true)
    }
    pub fn library_style_basis_retained_bytes(&self, base_id: &str) -> Result<usize> {
        self.backend.check_path(&self.data_dir)?;
        let basis = self.style_bases.get(base_id).ok_or_else(|| {
            CoreError::new(
                "basisRequired",
                UiMessage::error_core_operations_original_style_basis_unavailable(),
            )
        })?;
        Ok(style_basis_retained_bytes(basis))
    }
    pub fn discard_library_style_basis(&mut self, base_id: &str) -> Result<()> {
        valid_operation_id(base_id)?;
        self.style_bases.remove(base_id);
        Ok(())
    }
    pub fn library_style_basis(
        &self,
        root: impl AsRef<Path>,
        base_id: &str,
    ) -> Result<LibraryStyleDocument> {
        self.backend.check_path(&self.data_dir)?;
        let root = absolute_path(root.as_ref())?;
        self.validate_library_style(&root, base_id)?;
        let b = &self.style_bases[base_id];
        Ok(LibraryStyleDocument {
            source: b.source.clone(),
            base_id: base_id.into(),
            path: display(&root.join("Styles/library-style.typ")),
            enabled: value_enabled(&b.settings.captured.as_ref().map(|f| f.bytes.clone()))?,
            resource_ref: b.resource_ref.clone(),
            entry_function: "library-style".into(),
            installed: b.style.is_some(),
        })
    }
    pub fn trash_entries(&self, root: impl AsRef<Path>) -> Result<Vec<TrashEntry>> {
        let root = absolute_path(root.as_ref())?;
        self.require_workspace(&root)?;
        let mut entries = Vec::new();
        for entry in self.original_trash_roots(&root)? {
            let Some(node_id) = &entry.node_id else {
                continue;
            };
            if !entry.path.try_exists()? {
                continue;
            }
            self.capture_original_trash_node(&entry)?;
            entries.push(TrashEntry {
                operation_id: entry.operation_id,
                node_id: node_id.clone(),
                name: entry.leaf.trim_end_matches(".typ").into(),
                path: display(&entry.path),
            });
        }
        entries.sort_by(|a, b| (&a.operation_id, &a.node_id).cmp(&(&b.operation_id, &b.node_id)));
        Ok(entries)
    }
    pub fn read_library_style(&mut self, root: impl AsRef<Path>) -> Result<LibraryStyleDocument> {
        let root = absolute_path(root.as_ref())?;
        let mut temporary = Vec::new();
        let result = self.read_library_style_inner(&root, &mut temporary);
        for base in temporary {
            self.snapshots.remove(&base);
        }
        result
    }
    fn read_library_style_inner(
        &mut self,
        root: &Path,
        temporary: &mut Vec<String>,
    ) -> Result<LibraryStyleDocument> {
        let workspace = self.require_workspace(root)?;
        let settings_path = root.join(ConfigRole::WorkspaceSettings.filename());
        let settings = self
            .read_snapshot(root, &[display(&settings_path)])?
            .files
            .remove(0);
        temporary.push(settings.base_id.clone());
        let settings_binding = self.snapshots[&settings.base_id].clone();
        let value: serde_json::Value = settings
            .bytes
            .as_ref()
            .map(|b| serde_json::from_slice(b))
            .transpose()?
            .unwrap_or_else(|| serde_json::json!({"version":1}));
        let resource_ref = value
            .get("defaultStyle")
            .and_then(|v| v.get("resource"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if value
            .get("defaultStyle")
            .and_then(|v| v.get("entryFunction"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s != "library-style")
        {
            return Err(CoreError::new(
                "styleSelection",
                UiMessage::error_core_operations_style_profile_supports_exact_library_style_entry_function_only(),
            ));
        }
        let path = root.join("Styles").join("library-style.typ");
        let mut style_binding = None;
        let mut manifest_binding = None;
        let mut owner_binding = None;
        let (source, enabled, installed) = if let Some(selected) = &resource_ref {
            let prefix = format!("weftext-resource://{}/", workspace.workspace_id);
            let resource_id = selected.strip_prefix(&prefix).ok_or_else(|| {
                CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_default_style_reference_belongs_to_different_workspace(),
                )
            })?;
            valid_operation_id(resource_id)?;
            self.managed_node(root, &root.join("Styles"), false)?;
            let mut snapshots = SnapshotSet {
                files: Vec::new(),
                complete: true,
            };
            for path in [
                &path,
                &root.join("Styles/weftext.resources.json"),
                &root.join("Styles/Styles.typ"),
            ] {
                let snapshot = self.read_snapshot(root, &[display(path)])?.files.remove(0);
                temporary.push(snapshot.base_id.clone());
                snapshots.files.push(snapshot);
            }
            let module = &snapshots.files[0];
            let manifest = &snapshots.files[1];
            let value: serde_json::Value =
                serde_json::from_slice(manifest.bytes.as_ref().ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_style_resource_binding_missing(),
                    )
                })?)?;
            if value["version"] != 1
                || !value["resources"].as_array().is_some_and(|a| {
                    let selected = a
                        .iter()
                        .filter(|r| r["resourceId"] == resource_id)
                        .collect::<Vec<_>>();
                    selected.len() == 1 && selected[0]["path"] == "library-style.typ"
                })
            {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_selected_resource_not_bound_to_exact_style_module(),
                ));
            }
            let source = decode(module.bytes.as_ref().ok_or_else(|| {
                CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_selected_module_unavailable(),
                )
            })?)?;
            let owner_capture = self.snapshots[&snapshots.files[2].base_id]
                .captured
                .as_ref()
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_style_owner_source_missing(),
                    )
                })?;
            let owner = self
                .admitted_binding(&owner_capture.stamp.identity)?
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_style_owner_source_unadmitted(),
                    )
                })?;
            if weftext_engine::inspect_metadata(&decode(&owner_capture.bytes)?)
                .id
                .as_ref()
                != Some(&owner.node_id)
            {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_style_owner_declaration_damaged(),
                ));
            }
            let binding = self.snapshots[&module.base_id].captured.as_ref().unwrap();
            if !self.admitted_resource(
                resource_id,
                &workspace.workspace_id,
                &owner.node_id,
                &binding.stamp.identity,
            )? {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_selected_style_resource_object_has_no_committed_core_birth_binding(),
                ));
            }
            style_binding = Some(self.snapshots[&module.base_id].clone());
            manifest_binding = Some(self.snapshots[&manifest.base_id].clone());
            owner_binding = Some(self.snapshots[&snapshots.files[2].base_id].clone());
            (source, value_enabled(&settings.bytes)?, true)
        } else {
            (default_style_source().into(), false, false)
        };
        let base_id = fresh();
        self.style_bases.insert(
            base_id.clone(),
            StyleBasis {
                root: display(root),
                settings: settings_binding,
                style: style_binding,
                manifest: manifest_binding,
                owner: owner_binding,
                source: source.clone(),
                resource_ref: resource_ref.clone(),
            },
        );
        Ok(LibraryStyleDocument {
            source,
            base_id,
            path: display(&path),
            enabled,
            resource_ref,
            entry_function: "library-style".into(),
            installed,
        })
    }
    pub fn prepare_library_style(
        &mut self,
        root: impl AsRef<Path>,
        base_id: &str,
        source: &str,
        enabled: bool,
        operation_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(&root),
            intent: OperationIntent::ConfigureStyle {
                base_id: base_id.into(),
                source: source.into(),
                enabled,
            },
        })
    }

    pub(crate) fn accepted_revision(&self, captured: &CapturedFile) -> Result<Option<String>> {
        let digest = hash(&captured.bytes);
        let mut candidates = Vec::new();
        for plan in self.committed_plans()? {
            if let Some(fact) = self
                .read_observation_facts(&plan.operation_id)?
                .into_iter()
                .find(|fact| fact.stamp == captured.stamp && fact.hash == digest)
            {
                candidates.push((self.commit_sequence(&plan.operation_id)?, fact.revision));
            }
        }
        candidates.sort_by_key(|value| value.0);
        let Some((sequence, revision)) = candidates.last() else {
            return Ok(None);
        };
        if candidates
            .iter()
            .any(|(s, r)| s == sequence && r != revision)
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_backend_ambiguous_revision_facts(),
            ));
        }
        Ok(Some(revision.clone()))
    }
    pub fn reserve_source_revision(
        &mut self,
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
        base_id: &str,
        operation_id: &str,
    ) -> Result<String> {
        valid_operation_id(operation_id)?;
        self.check_attempt_not_cancelled(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        let current = self.managed_node(
            &root,
            path.parent().ok_or_else(|| {
                CoreError::new(
                    "invalidPath",
                    UiMessage::error_core_backend_missing_source_parent(),
                )
            })?,
            false,
        )?;
        if current.path != display(&path) {
            return Err(CoreError::new(
                "unsupportedRole",
                UiMessage::error_core_operations_only_canonical_managed_source_has_source_revision(
                ),
            ));
        }
        self.verify_snapshot(&path, base_id, Some(&current))?;
        if let Some(revision) = self.accepted_revision(&current)? {
            return Ok(revision);
        }
        let reservation_path = self.journal_path(operation_id, "revision-reservation");
        if let Ok(bytes) = self.backend.read_bytes(&reservation_path) {
            let old: RevisionReservation = serde_json::from_slice(&bytes)?;
            if old.path != current.path || old.stamp != current.stamp || old.bytes != current.bytes
            {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_operations_revision_reservation_belongs_to_different_source_basis(),
                ));
            }
            return Ok(old.revision);
        }
        let revision = fresh();
        journal::private_create(
            &self.backend,
            &reservation_path,
            &serde_json::to_vec(&RevisionReservation {
                library: self.draft_library_for_root(&root)?,
                operation_id: Some(operation_id.into()),
                path: current.path,
                stamp: current.stamp,
                bytes: current.bytes,
                revision: revision.clone(),
            })?,
        )?;
        Ok(revision)
    }
    pub(crate) fn verify_snapshot(
        &self,
        path: &Path,
        base_id: &str,
        current: Option<&CapturedFile>,
    ) -> Result<()> {
        let basis = self.snapshots.get(base_id).ok_or_else(|| {
            CoreError::new(
                "basisRequired",
                UiMessage::error_core_operations_read_exact_core_snapshot_before_preparing_updates(
                ),
            )
        })?;
        self.verify_snapshot_binding(path, basis, current)
    }
    fn verify_snapshot_binding(
        &self,
        path: &Path,
        basis: &SnapshotBinding,
        current: Option<&CapturedFile>,
    ) -> Result<()> {
        let parent_guard = ParentGuard::acquire(&self.backend, path)?;
        if basis.path != display(path)
            || basis.parent != parent_guard.directory_identity()?
            || basis.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
                != current.map(|f| (&f.stamp, &f.bytes))
        {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_core_operations_snapshot_object_bytes_or_parent_changed_after_review(),
            ));
        }
        Ok(())
    }
    pub fn read_config(
        &mut self,
        root: impl AsRef<Path>,
        role: ConfigRole,
    ) -> Result<ConfigDocument> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let path = root.join(role.filename());
        // Keep external bytes inspectable; semantic consumers and writers validate their own use.
        let snapshot = self
            .read_snapshot(&root, &[display(&path)])?
            .files
            .remove(0);
        if matches!(role, ConfigRole::Rules)
            && snapshot.bytes.is_none()
            && self.query_rules_ever_committed(&root)?
        {
            self.snapshots.remove(&snapshot.base_id);
            return Err(CoreError::new(
                "rulesUnavailable",
                UiMessage::error_core_operations_previously_committed_rules_missing_no_default_or_implicit_reinstallation_available(),
            ));
        }
        Ok(ConfigDocument {
            role_ref: format!(
                "weftext-config://{}/{}",
                workspace.workspace_id,
                role.role()
            ),
            path: display(&path),
            bytes: snapshot.bytes,
            revision: snapshot.revision,
            base_id: snapshot.base_id,
        })
    }
    pub fn prepare_config_save(
        &mut self,
        root: impl AsRef<Path>,
        role: ConfigRole,
        before: Option<Vec<u8>>,
        after: Vec<u8>,
        operation_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        if matches!(role, ConfigRole::Rules) {
            crate::validate_library_rules(&after)?;
        }
        if let Some(t) = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
        {
            if t.request.root == display(&root)
                && matches!(&t.request.intent,OperationIntent::UpdateFiles{changes}if changes.len()==1&&changes[0].path==display(&root.join(role.filename()))&&changes[0].before==before&&changes[0].after==after)
            {
                return self.operation_details(operation_id);
            }
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_operations_operation_id_belongs_to_different_config_input(),
            ));
        }
        let current = self.read_config(&root, role)?;
        self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(&root),
            intent: OperationIntent::UpdateFiles {
                changes: vec![FileChange {
                    path: display(&root.join(role.filename())),
                    base_id: Some(current.base_id),
                    before,
                    after,
                }],
            },
        })
    }
    pub fn read_snapshot(
        &mut self,
        root: impl AsRef<Path>,
        paths: &[String],
    ) -> Result<SnapshotSet> {
        self.read_bounded_snapshot(root.as_ref(), paths, 64)
    }
    pub(crate) fn read_bounded_snapshot(
        &mut self,
        root: &Path,
        paths: &[String],
        limit: usize,
    ) -> Result<SnapshotSet> {
        let root = absolute_path(root)?;
        self.require_workspace(&root)?;
        if paths.is_empty() || paths.len() > limit {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_snapshot_file_count(
                    weftext_messages::params::ErrorCoreSnapshotFileCount {
                        limit: weftext_messages::UiInteger::try_from(limit).map_err(|_| {
                            CoreError::new(
                                "budget",
                                UiMessage::error_core_snapshot_limit_unsupported(),
                            )
                        })?,
                    },
                ),
            ));
        }
        let mut guards = Vec::new();
        let mut handles = Vec::new();
        let mut seen = BTreeSet::new();
        for p in paths {
            let path = absolute_path(Path::new(p))?;
            if !path.starts_with(&root) || !seen.insert(display(&path).to_lowercase()) {
                return Err(CoreError::new(
                    "outsideRoot",
                    UiMessage::error_core_operations_snapshot_paths_must_unique_and_inside_library(
                    ),
                ));
            }
            self.check_pending(&path)?;
            let guard = ParentGuard::acquire(&self.backend, &path)?;
            let handle = match guard.open_file(false, false) {
                Ok(f) => Some(f),
                Err(e) if e.code == "missing" => None,
                Err(e) => return Err(e),
            };
            if let Some(f) = &handle {
                self.check_pending_identity(&stamp(f)?.identity)?;
            }
            guards.push((path, guard));
            handles.push(handle);
        }
        let mut output = Vec::new();
        let mut bindings = Vec::new();
        let mut total = 0usize;
        for ((path, guard), handle) in guards.iter().zip(&mut handles) {
            let captured = match handle {
                Some(f) => Some(CapturedFile {
                    path: display(path),
                    stamp: stamp(f)?,
                    bytes: read_blob(f)?,
                }),
                None => None,
            };
            total += captured.as_ref().map_or(0, |f| f.bytes.len());
            if total > MAX_OPERATION_BYTES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_selected_snapshot_exceeds64_mi_b_no_partial_snapshot_published(),
                ));
            }
            let birth = captured
                .as_ref()
                .map(|f| self.admitted_binding(&f.stamp.identity))
                .transpose()?
                .flatten();
            let base_id = fresh();
            let revision = captured
                .as_ref()
                .map(|f| self.accepted_revision(f))
                .transpose()?
                .flatten();
            output.push(SnapshotFile {
                path: display(path),
                bytes: captured.as_ref().map(|f| f.bytes.clone()),
                base_id: base_id.clone(),
                node_id: birth.as_ref().map(|b| b.node_id.clone()),
                kind: if birth.is_some() { "managed" } else { "file" }.into(),
                currentness: if revision.is_some() {
                    "accepted"
                } else {
                    "observed"
                }
                .into(),
                revision,
            });
            bindings.push((
                base_id,
                SnapshotBinding {
                    path: display(path),
                    captured,
                    parent: guard.directory_identity()?,
                },
            ));
        }
        self.snapshots.extend(bindings);
        Ok(SnapshotSet {
            files: output,
            complete: true,
        })
    }
    /// Original committed relocation or Clone birth evidence locates Trash
    /// trees; neither a path name nor a matching declaration creates a member.
    fn admitted_trash_files(&self, root: &Path) -> Result<Vec<CapturedFile>> {
        let mut files = Vec::new();
        for entry in self.original_trash_roots(root)? {
            let path = entry.path;
            if !self.backend.entry_exists(&path)? {
                continue;
            }
            let guard =
                ParentGuard::acquire(&self.backend, &Path::new(&path).join(".trash-membership"))?;
            if guard.directory_identity()? != entry.identity {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_committed_trash_directory_was_replaced(),
                ));
            }
            let mut captured = Vec::new();
            let mut directories = Vec::new();
            snapshot_tree(
                &self.backend,
                Path::new(&path),
                &mut captured,
                &mut directories,
                false,
            )?;
            for file in captured {
                if self.admitted_binding(&file.stamp.identity)?.is_some()
                    && !files
                        .iter()
                        .any(|old: &CapturedFile| locator_equal(&old.path, &file.path))
                {
                    files.push(file);
                }
            }
            if files.len() > MAX_OPERATION_FILES
                || files.iter().map(|file| file.bytes.len()).sum::<usize>() > MAX_OPERATION_BYTES
            {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_admitted_trash_scope_exceeds_original_transaction_budget(),
                ));
            }
        }
        Ok(files)
    }

    pub fn admitted_snapshot(&mut self, root: impl AsRef<Path>) -> Result<AdmittedSnapshot> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        // The universe is the finite set of committed birth facts in this one
        // private authority, not all raw directories that resemble Nodes.
        let mut expected = BTreeMap::new();
        let mut trashed = BTreeSet::new();
        for p in self.committed_plans()?.into_iter() {
            for b in self.read_births(&p.operation_id)? {
                if b.workspace_id == workspace.workspace_id {
                    expected.insert(b.node_id.clone(), b);
                }
            }
        }
        for file in self.admitted_trash_files(&root)? {
            if let Some(birth) = self.admitted_binding(&file.stamp.identity)?
                && birth.workspace_id == workspace.workspace_id
            {
                trashed.insert(birth.node_id);
            }
        }
        expected.retain(|id, _| !trashed.contains(id));
        let mut inventory = self.inventory(&root)?;
        let raw_complete = inventory.complete;
        let raw_pending = inventory
            .entries
            .iter()
            .filter(|e| e.kind == "node" && e.identity_status != "admitted")
            .count();
        let mut paths = Vec::new();
        let mut warnings = Vec::new();
        let mut complete = true;
        let mut members = Vec::new();
        for (id, birth) in &expected {
            let candidates = inventory
                .entries
                .iter()
                .filter(|e| e.node_id.as_ref() == Some(id) && e.identity_status == "admitted")
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                complete = false;
                warnings.push(
                    UiMessage::error_core_operations_admitted_node_unavailable(
                        weftext_messages::params::ErrorCoreOperationsAdmittedNodeUnavailable {
                            node_id: id.clone(),
                        },
                    )
                    .into(),
                );
                continue;
            }
            let candidate = candidates[0];
            let observed = capture_optional(&self.backend, Path::new(&candidate.path))?;
            if observed
                .as_ref()
                .is_none_or(|f| f.stamp.identity != birth.identity)
            {
                complete = false;
                warnings.push(
                    UiMessage::error_core_operations_admitted_source_object_missing(
                        weftext_messages::params::ErrorCoreOperationsAdmittedSourceObjectMissing {
                            node_id: id.clone(),
                        },
                    )
                    .into(),
                );
                continue;
            }
            paths.push(candidate.path.clone());
            members.push(id.clone());
        }
        if expected.len() > MAX_OPERATION_FILES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_operations_admitted_set_exceeds5000_nodes(),
            ));
        }
        let schema_path = display(&root.join(ConfigRole::Schema.filename()));
        paths.push(schema_path.clone());
        paths.push(display(&root.join(".weftext-tags.json")));
        // All selected share-zero source/config handles coexist during capture.
        // Core commit cannot run between members because this authority is mutably
        // borrowed; new raw files cannot fabricate committed birth membership.
        let (mut cut, tags) =
            match self.read_bounded_snapshot(&root, &paths, MAX_OPERATION_FILES + 2) {
                Ok(mut cut) => {
                    let tags = cut.files.pop();
                    (cut, tags)
                }
                Err(_) => {
                    // Tags are an optional structural effect. A pending/busy tree
                    // must not block an unrelated exact query or ordinary capture.
                    // No failed cut is published; retry the source/schema set.
                    paths.pop();
                    (
                        self.read_bounded_snapshot(&root, &paths, MAX_OPERATION_FILES + 1)?,
                        None,
                    )
                }
            };
        let schema = cut.files.pop().unwrap();
        for f in &cut.files {
            let expected_node = inventory
                .entries
                .iter()
                .find(|e| e.path == f.path)
                .and_then(|e| e.node_id.clone());
            if f.node_id != expected_node
                || f.bytes.as_ref().is_none_or(|b| {
                    decode(b)
                        .ok()
                        .is_none_or(|s| weftext_engine::inspect_metadata(&s).id != expected_node)
                })
            {
                complete = false;
                warnings.push(
                    UiMessage::error_core_operations_source_admission_changed_during_capture(weftext_messages::params::ErrorCoreOperationsSourceAdmissionChangedDuringCapture {
                        path: f.path.clone(),
                    })
                    .into(),
                );
            }
            if let Some(entry) = inventory.entries.iter_mut().find(|e| e.path == f.path) {
                entry.source = f.bytes.as_ref().and_then(|b| decode(b).ok());
            }
        }
        // Schema is frozen author input; absence is explicit and the caller can
        // choose its bundled schema for an unconfigured library.
        if !raw_complete {
            warnings.extend(inventory.warnings.iter().cloned());
        }
        let cut_id = fresh();
        self.admitted_cuts.insert(
            cut_id.clone(),
            AdmittedBasis {
                root: display(&root),
                lifecycle: "live".into(),
                members: members.iter().cloned().collect(),
                bases: cut
                    .files
                    .iter()
                    .chain(std::iter::once(&schema))
                    .map(|f| f.base_id.clone())
                    .collect(),
                tag_base: tags.as_ref().map(|file| file.base_id.clone()),
                complete,
            },
        );
        Ok(AdmittedSnapshot {
            cut_id,
            membership_ids: members,
            inventory,
            files: cut.files,
            schema: Some(schema),
            tags,
            complete,
            warnings,
            raw_complete,
            raw_pending,
        })
    }
    /// Release this cut's private read tokens only, including optional captures.
    /// Owned witnesses and prepared Plans remain valid; this never changes
    /// author files, operations, admission facts or other independently read cuts.
    pub fn release_admitted_cut(&mut self, cut_id: &str) {
        self.compile_dependency_cuts.remove(cut_id);
        if let Some(cut) = self.admitted_cuts.remove(cut_id) {
            // Optional Tags capture is owned by the cut even when no structural
            // Query binds it into the operation's actual dependency read set.
            for base in cut.bases.into_iter().chain(cut.tag_base) {
                self.snapshots.remove(&base);
            }
        }
    }

    /// Bind only the Tags file captured with this admitted cut. Unrelated
    /// operations keep their original read set when they never use structure.
    pub fn bind_admitted_tag_structure(
        &mut self,
        root: impl AsRef<Path>,
        cut_id: &str,
        file: &SnapshotFile,
    ) -> Result<weftext_data::TagDocument> {
        let root = absolute_path(root.as_ref())?;
        let basis = self.admitted_cuts.get(cut_id).ok_or_else(|| {
            CoreError::new(
                "staleBase",
                UiMessage::error_core_operations_original_admitted_cut_expired(),
            )
        })?;
        if basis.root != display(&root) || basis.tag_base.as_ref() != Some(&file.base_id) {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_core_operations_tag_structure_does_not_belong_to_original_admitted_cut(),
            ));
        }
        let document = self.qualified_tag_document_from_snapshot(&root, file)?;
        let basis = self.admitted_cuts.get_mut(cut_id).unwrap();
        if !basis.bases.contains(&file.base_id) {
            basis.bases.push(file.base_id.clone());
        }
        Ok(document)
    }
    pub fn trash_admitted_snapshot(&mut self, root: impl AsRef<Path>) -> Result<AdmittedSnapshot> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let mut source_files = BTreeMap::new();
        let mut warnings = Vec::new();
        let mut complete = true;
        let mut entries = Vec::new();
        for file in self.admitted_trash_files(&root)? {
            let Some(birth) = self.admitted_binding(&file.stamp.identity)? else {
                continue;
            };
            if birth.workspace_id != workspace.workspace_id {
                continue;
            }
            let source = decode(&file.bytes)?;
            let info = weftext_engine::inspect_metadata(&source);
            if info.id.as_deref() != Some(&birth.node_id) {
                complete = false;
                warnings.push(
                    UiMessage::error_core_operations_trashed_node_declaration_damaged(weftext_messages::params::ErrorCoreOperationsTrashedNodeDeclarationDamaged {
                        node_id: birth.node_id.clone(),
                    })
                    .into(),
                );
                continue;
            }
            if source_files
                .insert(birth.node_id.clone(), file.path.clone())
                .is_some()
            {
                complete = false;
                warnings.push(
                    UiMessage::error_core_operations_trashed_node_identity_duplicate(
                        weftext_messages::params::ErrorCoreOperationsTrashedNodeIdentityDuplicate {
                            node_id: birth.node_id.clone(),
                        },
                    )
                    .into(),
                );
            }
            entries.push(InventoryEntry {
                path: file.path.clone(),
                relative_path: Path::new(&file.path)
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                name: Path::new(&file.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into(),
                parent_path: display(Path::new(&file.path).parent().unwrap()),
                kind: "node".into(),
                node_id: Some(birth.node_id),
                identity_status: "admitted".into(),
                title: info.title,
                excerpt: None,
                source: Some(source),
            });
        }
        let mut paths = source_files.values().cloned().collect::<Vec<_>>();
        paths.push(display(&root.join(ConfigRole::Schema.filename())));
        paths.push(display(&root.join(".weftext-tags.json")));
        let (mut snapshot, tags) =
            match self.read_bounded_snapshot(&root, &paths, MAX_OPERATION_FILES + 2) {
                Ok(mut snapshot) => {
                    let tags = snapshot.files.pop();
                    (snapshot, tags)
                }
                Err(_) => {
                    paths.pop();
                    (
                        self.read_bounded_snapshot(&root, &paths, MAX_OPERATION_FILES + 1)?,
                        None,
                    )
                }
            };
        let schema = snapshot.files.pop().unwrap();
        for f in &snapshot.files {
            if let Some(entry) = entries.iter_mut().find(|e| e.path == f.path) {
                entry.source = f.bytes.as_ref().and_then(|b| decode(b).ok());
                if f.node_id != entry.node_id {
                    complete = false;
                    warnings.push(
                        UiMessage::error_core_operations_trash_admission_changed_during_capture(weftext_messages::params::ErrorCoreOperationsTrashAdmissionChangedDuringCapture {
                            path: f.path.clone(),
                        })
                        .into(),
                    );
                }
            }
        }
        let inventory = Inventory {
            root: display(&root),
            workspace_id: Some(workspace.workspace_id),
            status: "trashReadOnly".into(),
            entries,
            warnings: warnings.clone(),
            complete,
        };
        let cut_id = fresh();
        let members = source_files.keys().cloned().collect::<Vec<_>>();
        self.admitted_cuts.insert(
            cut_id.clone(),
            AdmittedBasis {
                root: display(&root),
                lifecycle: "trash".into(),
                members: members.iter().cloned().collect(),
                bases: snapshot
                    .files
                    .iter()
                    .chain(std::iter::once(&schema))
                    .map(|file| file.base_id.clone())
                    .collect(),
                tag_base: tags.as_ref().map(|file| file.base_id.clone()),
                complete,
            },
        );
        Ok(AdmittedSnapshot {
            cut_id,
            membership_ids: members,
            inventory,
            files: snapshot.files,
            schema: Some(schema),
            tags,
            complete,
            warnings,
            raw_complete: complete,
            raw_pending: 0,
        })
    }
    pub fn prepare_operation(&mut self, request: OperationRequest) -> Result<OperationPreview> {
        if matches!(request.intent, OperationIntent::AdoptSyncExternal { .. }) {
            return self.prepare_sync_adoption(request);
        }
        if matches!(
            request.intent,
            OperationIntent::ApplySync { .. }
                | OperationIntent::MergeSync { .. }
                | OperationIntent::RevalidateSync { .. }
        ) {
            return self.prepare_sync_application(request);
        }
        if matches!(request.intent, OperationIntent::CloneSync { .. }) {
            return self.prepare_sync_clone(request);
        }
        self.prepare_internal(request, None, None, &[], &[], None, &[], None)
    }
    /// Freeze the finite admitted source set and all bounded native filesystem
    /// inputs together. The engine may only read this map; absent inputs remain
    /// unavailable. This is an immutable local input cut, not a currentness claim.
    pub fn admitted_compile_snapshot(
        &mut self,
        root: impl AsRef<Path>,
        lifecycle: &str,
    ) -> Result<AdmittedCompileSnapshot> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let snapshot = match lifecycle {
            "live" => self.admitted_snapshot(&root)?,
            "trash" => self.trash_admitted_snapshot(&root)?,
            _ => {
                return Err(CoreError::new(
                    "invalidScope",
                    UiMessage::error_core_operations_choose_live_or_trash_admitted_scope(),
                ));
            }
        };
        let cut_id = snapshot.cut_id.clone();
        let result =
            self.capture_admitted_compile_snapshot(&root, &workspace.workspace_id, snapshot);
        if result.is_err() {
            self.release_admitted_cut(&cut_id);
        }
        result
    }

    /// Pure preview input from the actual opened document's qualified root.
    /// It does not create a write basis or keep another author/Draft record.
    pub fn preview_compile_snapshot(
        &mut self,
        session_id: &str,
        source: &str,
    ) -> Result<weftext_engine::FrozenRenderRequest> {
        self.check_actor_session(session_id)?;
        check_size(source.as_bytes())?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_source()))?
            .clone();
        self.check_pending(&session.path)?;
        let main_guard = ParentGuard::acquire(&self.backend, &session.path)?;
        let main = main_guard.open_file(false, false)?;
        if stamp(&main)?.identity != session.stamp.identity {
            return Err(CoreError::new(
                "scopeChanged",
                weftext_messages::UiMessage::error_core_operations_the_opened_source_object_was_replaced(),
            ));
        }
        drop(main);
        let (mut files, style) = if self.has_library(&session.root)? {
            let captured = self.admitted_compile_snapshot(&session.root, "live")?;
            // Read-only preview retains the bytes, not a write/dependency cut.
            self.release_compile_dependency_cut(&captured.snapshot.cut_id);
            (captured.files, captured.style)
        } else {
            let mut captured = Vec::new();
            let mut directories = Vec::new();
            snapshot_tree(
                &self.backend,
                &session.root,
                &mut captured,
                &mut directories,
                false,
            )?;
            let (_guards, _handles) = self.hold_compile_tree(&captured, &directories)?;
            let mut files = BTreeMap::new();
            for file in captured {
                let relative = Path::new(&file.path)
                    .strip_prefix(&session.root)
                    .map_err(|_| {
                        CoreError::new("outsideRoot", weftext_messages::UiMessage::error_core_operations_preview_input_is_outside_its_actual_root())
                    })?
                    .to_str()
                    .ok_or_else(|| CoreError::new("invalidPath", weftext_messages::UiMessage::error_core_operations_preview_path_is_not_unicode()))?
                    .replace('\\', "/");
                if files.insert(relative, file.bytes).is_some() {
                    return Err(CoreError::new(
                        "scopeChanged",
                        weftext_messages::UiMessage::error_core_operations_preview_input_paths_are_not_unique(),
                    ));
                }
            }
            (files, None)
        };
        let relative = session
            .path
            .strip_prefix(&session.root)
            .map_err(|_| {
                CoreError::new(
                    "outsideRoot",
                    weftext_messages::UiMessage::error_core_operations_the_opened_source_is_outside_its_actual_root(),
                )
            })?
            .to_str()
            .ok_or_else(|| CoreError::new("invalidPath", weftext_messages::UiMessage::error_core_operations_preview_path_is_not_unicode()))?
            .replace('\\', "/");
        // The only replaced bytes are this caller's explicit current source.
        // Every dependency stays from the one qualified immutable capture.
        files.insert(relative, source.as_bytes().to_vec());
        if files
            .values()
            .try_fold(0usize, |total, bytes| total.checked_add(bytes.len()))
            .is_none_or(|total| total > MAX_OPERATION_BYTES)
        {
            return Err(CoreError::new(
                "budget",
                weftext_messages::UiMessage::error_core_operations_preview_input_exceeds_the_complete64_mi_b_budget(),
            ));
        }
        let main = main_guard.open_file(false, false)?;
        if stamp(&main)?.identity != session.stamp.identity {
            return Err(CoreError::new(
                "scopeChanged",
                weftext_messages::UiMessage::error_core_operations_the_opened_source_object_changed_during_capture(),
            ));
        }
        Ok(weftext_engine::FrozenRenderRequest {
            source: source.into(),
            path: session.path,
            root: session.root,
            files,
            style,
        })
    }

    fn hold_compile_tree(
        &self,
        captured: &[CapturedFile],
        directories: &[CapturedDirectory],
    ) -> Result<(Vec<ParentGuard>, Vec<backend::BoundFile>)> {
        let mut guards = Vec::new();
        let mut handles = Vec::new();
        for file in captured {
            self.check_pending(Path::new(&file.path))?;
            let guard = ParentGuard::acquire(&self.backend, Path::new(&file.path))?;
            let mut handle = guard.open_file(false, false)?;
            if stamp(&handle)? != file.stamp
                || read_blob(&mut handle)? != file.bytes
                || stamp(&handle)? != file.stamp
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_compile_input_changed_while_acquiring_shared_cut(),
                ));
            }
            guards.push(guard);
            handles.push(handle);
        }
        for directory in directories {
            let guard = ParentGuard::acquire(
                &self.backend,
                &Path::new(&directory.path).join(".compile-membership"),
            )?;
            let (entries, complete) = guard.directory_entries(MAX_OPERATION_FILES)?;
            if !complete
                || guard.directory_identity()? != directory.identity
                || sorted_entries(entries, Path::new(&directory.path)) != directory.entries
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_compile_input_membership_changed(),
                ));
            }
            guards.push(guard);
        }
        Ok((guards, handles))
    }

    fn capture_admitted_compile_snapshot(
        &mut self,
        root: &Path,
        workspace_id: &str,
        snapshot: AdmittedSnapshot,
    ) -> Result<AdmittedCompileSnapshot> {
        let mut captured = Vec::new();
        let mut directories = Vec::new();
        snapshot_tree(&self.backend, root, &mut captured, &mut directories, false)?;
        let (mut guards, handles) = self.hold_compile_tree(&captured, &directories)?;
        for original in snapshot
            .files
            .iter()
            .chain(snapshot.schema.iter())
            .chain(snapshot.tags.iter())
        {
            let basis = self.snapshots.get(&original.base_id).ok_or_else(|| {
                CoreError::new(
                    "staleBase",
                    UiMessage::error_core_operations_selected_source_basis_expired(),
                )
            })?;
            let current = captured
                .iter()
                .find(|f| locator_equal(&f.path, &original.path));
            if basis.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
                != current.map(|f| (&f.stamp, &f.bytes))
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_selected_admitted_source_schema_changed_before_compile_cut(),
                ));
            }
            if current.is_none() {
                let guard = ParentGuard::acquire(&self.backend, Path::new(&basis.path))?;
                ensure_absent(&self.backend, Path::new(&basis.path))?;
                if guard.directory_identity()? != basis.parent {
                    return Err(CoreError::new(
                        "scopeChanged",
                        UiMessage::error_core_operations_negative_schema_basis_changed(),
                    ));
                }
                guards.push(guard);
            }
        }
        let settings_path = root.join(ConfigRole::WorkspaceSettings.filename());
        let settings = captured
            .iter()
            .find(|f| locator_equal(&f.path, &display(&settings_path)));
        let settings_value: serde_json::Value = settings
            .map(|f| serde_json::from_slice(&f.bytes))
            .transpose()?
            .unwrap_or_else(|| serde_json::json!({"version":1}));
        let style = if settings_value["defaultStyle"]["enabled"] == true {
            let selected = settings_value["defaultStyle"]["resource"]
                .as_str()
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_enabled_default_style_has_no_exact_resource_selector(),
                    )
                })?;
            let resource_id = selected
                .strip_prefix(&format!("weftext-resource://{workspace_id}/"))
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_style_belongs_to_another_workspace(),
                    )
                })?;
            if settings_value["defaultStyle"]["entryFunction"]
                .as_str()
                .is_some_and(|f| f != "library-style")
            {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_unsupported_explicit_style_entry_function(),
                ));
            }
            let module = captured
                .iter()
                .find(|f| locator_equal(&f.path, &display(&root.join("Styles/library-style.typ"))))
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_selected_style_module_unavailable(),
                    )
                })?;
            let manifest = captured
                .iter()
                .find(|f| {
                    locator_equal(
                        &f.path,
                        &display(&root.join("Styles/weftext.resources.json")),
                    )
                })
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_selected_style_binding_unavailable(),
                    )
                })?;
            let value: serde_json::Value = serde_json::from_slice(&manifest.bytes)?;
            if value["version"] != 1
                || !value["resources"].as_array().is_some_and(|a| {
                    let v = a
                        .iter()
                        .filter(|r| r["resourceId"] == resource_id)
                        .collect::<Vec<_>>();
                    v.len() == 1 && v[0]["path"] == "library-style.typ"
                })
            {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_selected_style_resource_binding_ambiguous(),
                ));
            }
            let owner = captured
                .iter()
                .find(|f| locator_equal(&f.path, &display(&root.join("Styles/Styles.typ"))))
                .map(|f| self.admitted_binding(&f.stamp.identity))
                .transpose()?
                .flatten()
                .ok_or_else(|| {
                    CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_selected_style_owner_unavailable(),
                    )
                })?;
            if !self.admitted_resource(
                resource_id,
                workspace_id,
                &owner.node_id,
                &module.stamp.identity,
            )? {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_operations_selected_style_not_admitted_resource_object(),
                ));
            }
            Some(weftext_engine::StyleModule {
                path: Path::new(&module.path)
                    .strip_prefix(root)
                    .map_err(|_| {
                        CoreError::new(
                            "outsideRoot",
                            UiMessage::error_core_operations_selected_style_outside_selected_library(),
                        )
                    })?
                    .to_string_lossy()
                    .replace('\\', "/"),
                source: decode(&module.bytes)?,
                enabled: true,
                entry_function: "library-style".into(),
            })
        } else {
            None
        };
        let mut files = BTreeMap::new();
        let mut bases = Vec::new();
        let mut bindings = Vec::new();
        for f in captured {
            let path = Path::new(&f.path);
            let base_id = fresh();
            let parent = ParentGuard::acquire(&self.backend, path)?.directory_identity()?;
            files.insert(
                path.strip_prefix(root)
                    .map_err(|_| {
                        CoreError::new(
                            "outsideRoot",
                            UiMessage::error_core_operations_compile_input_outside_selected_library(
                            ),
                        )
                    })?
                    .to_string_lossy()
                    .replace('\\', "/"),
                f.bytes.clone(),
            );
            bindings.push((
                base_id.clone(),
                SnapshotBinding {
                    path: f.path.clone(),
                    captured: Some(f),
                    parent,
                },
            ));
            bases.push(base_id);
        }
        if let Some(cut) = self.admitted_cuts.get_mut(&snapshot.cut_id) {
            cut.bases.extend(bases);
        }
        self.snapshots.extend(bindings);
        self.retain_compile_dependency_cut(root, &snapshot.cut_id, &directories)?;
        drop(handles);
        drop(guards);
        Ok(AdmittedCompileSnapshot {
            snapshot,
            files,
            style,
        })
    }
    pub fn feature_operation_status(
        &self,
        operation_id: &str,
        canonical_author_intent: &serde_json::Value,
    ) -> Result<Option<OperationPreview>> {
        self.backend.check_path(&self.data_dir)?;
        valid_operation_id(operation_id)?;
        let fingerprint = hash(&canonical_json_bytes(canonical_author_intent)?);
        if let Some(p) = self.operations.get(operation_id) {
            if p.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_operations_operation_id_belongs_to_different_canonical_author_intent(),
                ));
            }
            return self.operation_details(operation_id).map(Some);
        }
        if self.resolved(operation_id)? {
            return Err(CoreError::new(
                "operationCancelled",
                UiMessage::error_core_journal_original_operation_was_cancelled_before_preparation(),
            ));
        }
        Ok(None)
    }
    pub fn prepare_feature_operation(
        &mut self,
        request: OperationRequest,
        canonical_author_intent: &serde_json::Value,
    ) -> Result<OperationPreview> {
        self.prepare_internal(
            request,
            None,
            Some(hash(&canonical_json_bytes(canonical_author_intent)?)),
            &[],
            &[],
            None,
            &[],
            None,
        )
    }
    pub fn prepare_dependent_feature_operation(
        &mut self,
        request: OperationRequest,
        canonical_author_intent: &serde_json::Value,
        base_ids: &[String],
    ) -> Result<OperationPreview> {
        self.prepare_internal(
            request,
            None,
            Some(hash(&canonical_json_bytes(canonical_author_intent)?)),
            base_ids,
            &[],
            None,
            &[],
            None,
        )
    }
    pub(crate) fn prepare_dependent_feature_operation_with_witnesses(
        &mut self,
        request: OperationRequest,
        canonical_author_intent: &serde_json::Value,
        base_ids: &[String],
        witnesses: &[&SnapshotWitness],
    ) -> Result<OperationPreview> {
        self.prepare_dependent_feature_operation_with_tree(
            request,
            canonical_author_intent,
            base_ids,
            witnesses,
            None,
            &[],
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_dependent_feature_operation_with_tree(
        &mut self,
        request: OperationRequest,
        canonical_author_intent: &serde_json::Value,
        base_ids: &[String],
        witnesses: &[&SnapshotWitness],
        tree: Option<&FeatureTreeWitness>,
        transfer_witnesses: &[&SnapshotWitness],
        destination_slot: Option<&HostedDestination>,
    ) -> Result<OperationPreview> {
        self.prepare_internal(
            request,
            None,
            Some(hash(&canonical_json_bytes(canonical_author_intent)?)),
            base_ids,
            witnesses,
            tree,
            transfer_witnesses,
            destination_slot,
        )
    }
    /// Strong product actions freeze the complete admitted-set membership and
    /// every supplied cut source/schema basis as dependencies of the same plan.
    pub fn prepare_admitted_operation(
        &mut self,
        request: OperationRequest,
        cut_id: &str,
    ) -> Result<OperationPreview> {
        self.prepare_admitted_internal(request, cut_id, None)
    }
    pub fn prepare_admitted_feature_operation(
        &mut self,
        request: OperationRequest,
        cut_id: &str,
        canonical_author_intent: &serde_json::Value,
    ) -> Result<OperationPreview> {
        self.prepare_admitted_internal(
            request,
            cut_id,
            Some(hash(&canonical_json_bytes(canonical_author_intent)?)),
        )
    }
    fn prepare_admitted_internal(
        &mut self,
        request: OperationRequest,
        cut_id: &str,
        fingerprint: Option<String>,
    ) -> Result<OperationPreview> {
        let cut = self.admitted_cuts.get(cut_id).cloned().ok_or_else(|| {
            CoreError::new(
                "cutExpired",
                UiMessage::error_core_operations_read_fresh_admitted_set_snapshot_before_preparing(
                ),
            )
        })?;
        if cut.root != request.root || !cut.complete {
            return Err(CoreError::new(
                "coverageIncomplete",
                UiMessage::error_core_operations_strong_action_requires_complete_admitted_set_cut(),
            ));
        }
        if cut.lifecycle != "live" {
            return Err(CoreError::new(
                "readOnly",
                UiMessage::error_core_operations_trash_read_cut_cannot_authorize_new_generic_author_operation(),
            ));
        }
        self.prepare_internal(request, Some(cut), fingerprint, &[], &[], None, &[], None)
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_internal(
        &mut self,
        request: OperationRequest,
        cut: Option<AdmittedBasis>,
        author_fingerprint: Option<String>,
        dependencies: &[String],
        witnesses: &[&SnapshotWitness],
        tree: Option<&FeatureTreeWitness>,
        transfer_witnesses: &[&SnapshotWitness],
        destination_slot: Option<&HostedDestination>,
    ) -> Result<OperationPreview> {
        valid_operation_id(&request.operation_id)?;
        let fingerprint = author_fingerprint.unwrap_or(hash(&serde_json::to_vec(&request)?));
        if let Some(old) = self.operations.get(&request.operation_id) {
            if old.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_operations_operation_id_belongs_to_different_input(),
                ));
            }
            return self.operation_details(&request.operation_id);
        }
        if self.resolved(&request.operation_id)? {
            return Err(CoreError::new(
                "operationCancelled",
                UiMessage::error_core_journal_original_operation_was_cancelled_before_preparation(),
            ));
        }
        let root = absolute_path(Path::new(&request.root))?;
        let workspace = self.require_workspace(&root)?;
        if let Some(destination) = destination_slot {
            self.validate_hosted_destination(&root, &request.intent, destination, None)?;
        }
        let plan_id = fresh();
        let mut preview = OperationPreview {
            operation_id: request.operation_id.clone(),
            plan_id: plan_id.clone(),
            kind: intent_kind(&request.intent).into(),
            status: "prepared".into(),
            changes: Vec::new(),
            node_map: BTreeMap::new(),
            new_node_id: None,
            revisions: BTreeMap::new(),
            resource_map: BTreeMap::new(),
            losses: Vec::new(),
        };
        let mut files = Vec::new();
        let mut directories = Vec::new();
        let mut backup_private_digest = None;
        let mut absent_dependencies = Vec::new();
        let mut dependency_pool = Vec::new();
        let mut incoming = Vec::with_capacity(dependencies.len());
        for id in dependencies {
            let basis = self.snapshots.get(id).cloned().ok_or_else(|| {
                CoreError::new(
                    "staleBase",
                    UiMessage::error_core_operations_dependency_basis_unavailable(),
                )
            })?;
            incoming.push(SnapshotWitness { binding: basis });
        }
        Self::merge_dependency_witnesses(&mut dependency_pool, incoming, true)?;
        Self::merge_dependency_witnesses(
            &mut dependency_pool,
            witnesses.iter().map(|w| (*w).clone()).collect(),
            true,
        )?;
        Self::merge_dependency_witnesses(
            &mut dependency_pool,
            transfer_witnesses.iter().map(|w| (*w).clone()).collect(),
            true,
        )?;
        if let Some(tree) = tree {
            self.validate_feature_tree_witness(&root, tree)?;
            Self::merge_dependency_witnesses(
                &mut dependency_pool,
                tree.dependencies
                    .iter()
                    .chain(std::iter::once(&tree.schema))
                    .cloned()
                    .collect(),
                true,
            )?;
        }
        for witness in dependency_pool {
            let basis = witness.binding;
            if !locator_within(Path::new(&basis.path), &root) {
                return Err(CoreError::new(
                    "outsideRoot",
                    UiMessage::error_core_operations_dependency_belongs_to_another_library(),
                ));
            }
            self.check_pending(Path::new(&basis.path))?;
            let current = capture_optional(&self.backend, Path::new(&basis.path))?;
            self.verify_snapshot_binding(Path::new(&basis.path), &basis, current.as_ref())?;
            if let Some(f) = current {
                files.push(f);
            } else {
                absent_dependencies.push(AbsentDependency {
                    path: basis.path,
                    parent: basis.parent,
                });
            }
        }
        if let Some(cut) = &cut {
            let current = self.admitted_snapshot(&root)?;
            let same = current.complete
                && current
                    .membership_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    == cut.members;
            self.release_admitted_cut(&current.cut_id);
            if !same {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_admitted_set_membership_changed_before_planning(),
                ));
            }
            for id in &cut.bases {
                let basis = self.snapshots.get(id).cloned().ok_or_else(|| {
                    CoreError::new(
                        "staleBase",
                        UiMessage::error_core_operations_frozen_cut_basis_unavailable(),
                    )
                })?;
                let current = capture_optional(&self.backend, Path::new(&basis.path))?;
                self.verify_snapshot(Path::new(&basis.path), id, current.as_ref())?;
                if let Some(f) = current {
                    files.push(f);
                } else {
                    absent_dependencies.push(AbsentDependency {
                        path: basis.path,
                        parent: basis.parent,
                    });
                }
            }
        }
        let mutation = match &request.intent {
            OperationIntent::CreateLibrary { .. } => {
                return Err(CoreError::new(
                    "invalidIntent",
                    UiMessage::error_core_operations_library_creation_requires_dedicated_create_only_planner(),
                ));
            }
            OperationIntent::CloneSync { .. }
            | OperationIntent::ApplySync { .. }
            | OperationIntent::MergeSync { .. }
            | OperationIntent::RevalidateSync { .. }
            | OperationIntent::AdoptSyncExternal { .. } => {
                return Err(CoreError::new(
                    "invalidIntent",
                    UiMessage::error_core_operations_replica_intents_require_their_dedicated_guarded_planner(),
                ));
            }
            OperationIntent::ImportDocx {
                parent,
                name,
                bytes,
                allow_losses,
            } => self.plan_docx_import(
                &root,
                Path::new(parent),
                name,
                bytes,
                *allow_losses,
                None,
                &[],
                &[],
                &mut preview,
                &mut files,
                &mut absent_dependencies,
            )?,
            OperationIntent::ImportDocxWithOptions {
                parent,
                name,
                bytes,
                allow_losses,
                options,
                originals,
                conversion_losses,
            } => self.plan_docx_import(
                &root,
                Path::new(parent),
                name,
                bytes,
                *allow_losses,
                Some(options),
                originals,
                conversion_losses,
                &mut preview,
                &mut files,
                &mut absent_dependencies,
            )?,
            OperationIntent::ImportNative {
                parent,
                name,
                bytes,
                allow_losses,
            } => self.plan_native_import(
                &root,
                Path::new(parent),
                name,
                bytes,
                *allow_losses,
                &fingerprint,
                &mut preview,
                &mut files,
                &mut absent_dependencies,
            )?,
            OperationIntent::ImportRecognized {
                parent,
                name,
                format,
                bytes,
                evidence,
                review,
                allow_losses,
            } => self.plan_recognition_import(
                &root,
                Path::new(parent),
                name,
                format,
                bytes,
                evidence,
                review,
                *allow_losses,
                &mut preview,
                &mut files,
                &mut absent_dependencies,
            )?,
            OperationIntent::ImportBibliography {
                parent,
                name,
                format,
                bytes,
                allow_losses,
            } => self.plan_bibliography_import(
                &root,
                Path::new(parent),
                name,
                format,
                bytes,
                *allow_losses,
                &mut preview,
                &mut files,
                &mut absent_dependencies,
            )?,
            OperationIntent::RenameNode { source, name } => {
                valid_name(name)?;
                let source = absolute_path(Path::new(source))?;
                let parent = source.parent().ok_or_else(|| {
                    CoreError::new(
                        "invalidPath",
                        UiMessage::error_core_operations_node_has_no_parent(),
                    )
                })?;
                self.prepare_relocation(
                    &root,
                    &source,
                    &parent.join(name),
                    false,
                    &mut files,
                    &mut preview,
                )?
            }
            OperationIntent::MoveNode { source, parent } => {
                let source = absolute_path(Path::new(source))?;
                let parent = absolute_path(Path::new(parent))?;
                let name = node_name(&source)?;
                self.prepare_relocation(
                    &root,
                    &source,
                    &parent.join(name),
                    false,
                    &mut files,
                    &mut preview,
                )?
            }
            OperationIntent::RelocateDirectory {
                source,
                parent,
                name,
            } => {
                let source = absolute_path(Path::new(source))?;
                let parent = absolute_path(Path::new(parent))?;
                self.prepare_directory_relocation(
                    &root,
                    &source,
                    &parent.join(name),
                    &mut files,
                    &mut preview,
                )?
            }
            OperationIntent::TrashNode { source } => {
                let source = absolute_path(Path::new(source))?;
                let trash = root.join(".weftext-trash");
                let target = trash.join(&request.operation_id);
                self.prepare_relocation(&root, &source, &target, true, &mut files, &mut preview)?
            }
            OperationIntent::RestoreNode {
                trash_operation_id,
                node_id,
                parent,
                name,
            } => {
                valid_name(name)?;
                valid_operation_id(trash_operation_id)?;
                let selected =
                    self.select_original_trash_node(&root, trash_operation_id, node_id.as_deref())?;
                let source = selected.path;
                let parent = absolute_path(Path::new(parent))?;
                self.prepare_relocation(
                    &root,
                    &source,
                    &parent.join(name),
                    true,
                    &mut files,
                    &mut preview,
                )?
            }
            OperationIntent::NewNode {
                parent,
                name,
                source,
            } => {
                valid_name(name)?;
                let parent = absolute_path(Path::new(parent))?;
                self.owned_parent(&root, &parent)?;
                let target = parent.join(name);
                ensure_absent(&self.backend, &target)?;
                let node_id = fresh();
                let source = match source {
                    None => default_note_source(&node_id, name)?,
                    Some(s) => initialize_source(s, &node_id)?,
                };
                check_size(source.as_bytes())?;
                let path = target.join(format!("{name}.typ"));
                preview.new_node_id = Some(node_id.clone());
                preview.revisions.insert(display(&path), fresh());
                preview.changes.push(change(&path, "create", source.len()));
                Mutation::CreateTree {
                    existing_target: false,
                    target: display(&target),
                    directories: Vec::new(),
                    files: vec![FileChange {
                        path: display(&path),
                        base_id: None,
                        before: None,
                        after: source.into_bytes(),
                    }],
                    births: vec![NewBirth {
                        path: display(&path),
                        node_id,
                    }],
                    updates: Vec::new(),
                }
            }
            OperationIntent::UpdateFiles { changes } => {
                if changes.is_empty() || changes.len() > 64 {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_bounded_update_requires_between1_and64_files(),
                    ));
                }
                let mut seen = BTreeSet::new();
                for c in changes {
                    let path = absolute_path(Path::new(&c.path))?;
                    if !seen.insert(display(&path).to_lowercase()) {
                        return Err(CoreError::new(
                            "duplicatePath",
                            UiMessage::error_core_operations_one_operation_cannot_write_same_path_twice(),
                        ));
                    }
                    self.validate_update(&root, &path, &c.after)?;
                    self.validate_navigation_order_update(&root, c)?;
                    self.capture_navigation_order_dependencies(
                        &root,
                        c,
                        &mut files,
                        &mut directories,
                    )?;
                    if path
                        .file_name()
                        .is_some_and(|n| n == "weftext.directory.json")
                    {
                        self.validate_directory_plan(
                            &request.operation_id,
                            &root,
                            &path,
                            &c.after,
                        )?;
                    }
                    self.check_pending(&path)?;
                    if is_canonical(&path) {
                        check_size(&c.after)?;
                    } else if c.after.len() > MAX_OPERATION_BYTES {
                        return Err(CoreError::new(
                            "budget",
                            UiMessage::error_core_operations_file_exceeds64_mi_b(),
                        ));
                    }
                    let current = capture_optional(&self.backend, &path)?;
                    if current.as_ref().map(|f| &f.bytes) != c.before.as_ref() {
                        return Err(CoreError::new(
                            "conflict",
                            UiMessage::error_core_operations_exact_current_bytes_differ_from_supplied_base(),
                        ));
                    }
                    let base_id = c.base_id.as_deref().ok_or_else(|| CoreError::new("basisRequired", UiMessage::error_core_operations_updates_require_exact_core_snapshot_base_id_including_create_only_targets()))?;
                    self.verify_snapshot(&path, base_id, current.as_ref())?;
                    let revision = current
                        .as_ref()
                        .filter(|f| f.bytes == c.after)
                        .map(|f| self.accepted_revision(f))
                        .transpose()?
                        .flatten();
                    let reservation = self
                        .backend
                        .read_optional_json::<RevisionReservation>(
                            self.journal_path(&request.operation_id, "revision-reservation"),
                        )?
                        .filter(|r| {
                            r.path == c.path
                                && current
                                    .as_ref()
                                    .is_some_and(|f| f.stamp == r.stamp && f.bytes == r.bytes)
                                && r.bytes == c.after
                        });
                    preview.revisions.insert(
                        c.path.clone(),
                        revision
                            .or_else(|| reservation.map(|r| r.revision))
                            .unwrap_or_else(fresh),
                    );
                    if let Some(f) = current
                        && !files.iter().any(|old| old.path == f.path)
                    {
                        files.push(f);
                    }
                    let action = if c.before.as_ref() == Some(&c.after) {
                        if preview
                            .revisions
                            .get(&c.path)
                            .map(|r| -> Result<bool> {
                                Ok(files
                                    .iter()
                                    .find(|f| f.path == c.path)
                                    .map(|f| self.accepted_revision(f))
                                    .transpose()?
                                    .flatten()
                                    .as_ref()
                                    != Some(r))
                            })
                            .transpose()?
                            .unwrap_or(false)
                        {
                            "admitObservedSource"
                        } else {
                            "readDependency"
                        }
                    } else if c.before.is_some() {
                        "write"
                    } else {
                        "create"
                    };
                    preview.changes.push(change(&path, action, c.after.len()));
                }
                Mutation::Update {
                    files: changes.clone(),
                }
            }
            OperationIntent::CopyNode {
                source,
                parent,
                name,
                allow_losses,
            } => {
                let requested_source = absolute_path(Path::new(source))?;
                let source = tree
                    .map(|tree| tree.source.clone())
                    .unwrap_or_else(|| requested_source.clone());
                let parent = absolute_path(Path::new(parent))?;
                valid_name(name)?;
                self.managed_node(&root, &source, false)?;
                self.owned_parent(&root, &parent)?;
                self.check_subtree_boundary(&root, &source)?;
                let target = parent.join(name);
                ensure_absent(&self.backend, &target)?;
                let mut members = Vec::new();
                if let Some(tree) = tree {
                    if !crate::history::history_relative_path(&requested_source, &tree.source)
                        .is_some_and(|p| p.as_os_str().is_empty())
                    {
                        return Err(CoreError::new(
                            "operationConflict",
                            UiMessage::error_core_operations_copy_source_differs_from_original_authorized_tree(),
                        ));
                    }
                    members.extend(tree.files.clone());
                    directories.extend(tree.directories.clone());
                } else {
                    snapshot_tree(
                        &self.backend,
                        &source,
                        &mut members,
                        &mut directories,
                        false,
                    )?;
                }
                files.extend(members.clone());
                let schema_path = root.join(ConfigRole::Schema.filename());
                let schema = if let Some(tree) = tree {
                    tree.schema.binding.captured.clone()
                } else {
                    capture_optional(&self.backend, &schema_path)?
                };
                let mut context = members.clone();
                for witness in transfer_witnesses {
                    context.push(witness.binding.captured.clone().ok_or_else(|| {
                        CoreError::new(
                            "basisRequired",
                            UiMessage::error_core_operations_transfer_sources_require_original_present_bytes(),
                        )
                    })?);
                }
                let inventory = tree
                    .map(|_| self.feature_copy_inventory(&root, &context))
                    .transpose()?;
                let mutation = self.plan_copy(
                    &root,
                    &source,
                    &target,
                    &members,
                    &directories,
                    &mut preview,
                    *allow_losses,
                    schema.as_ref(),
                    inventory
                        .as_ref()
                        .map(|inventory| (inventory, context.as_slice())),
                )?;
                if let Some(config) = schema {
                    files.push(config);
                } else {
                    absent_dependencies.push(AbsentDependency {
                        path: display(&schema_path),
                        parent: ParentGuard::acquire(&self.backend, &schema_path)?
                            .directory_identity()?,
                    });
                }
                mutation
            }
            OperationIntent::TemplateNode {
                source,
                parent,
                name,
                instantiated_source,
                source_base_id,
            } => {
                let requested_canonical = absolute_path(Path::new(source))?;
                let canonical = tree
                    .map(|tree| {
                        tree.source.join(format!(
                            "{}.typ",
                            tree.source.file_name().unwrap().to_string_lossy()
                        ))
                    })
                    .unwrap_or_else(|| requested_canonical.clone());
                let source = canonical.parent().ok_or_else(|| {
                    CoreError::new(
                        "templateSource",
                        UiMessage::error_core_operations_template_source_has_no_node_parent(),
                    )
                })?;
                let original = self.managed_node(&root, source, false)?;
                if original.path != display(&canonical) {
                    return Err(CoreError::new(
                        "templateSource",
                        UiMessage::error_core_operations_templates_require_exact_admitted_canonical_source(),
                    ));
                }
                // The original token binds the caller's exact locator spelling;
                // the authorized tree uses the same guarded physical entry's
                // actual relative names. Do not replace the token or weaken its
                // file/parent/byte comparison to normalize a mixed slash input.
                self.verify_snapshot(&requested_canonical, source_base_id, Some(&original))?;
                let parent = absolute_path(Path::new(parent))?;
                valid_name(name)?;
                self.owned_parent(&root, &parent)?;
                self.check_subtree_boundary(&root, source)?;
                let target = parent.join(name);
                ensure_absent(&self.backend, &target)?;
                let mut originals = Vec::new();
                let mut original_dirs = Vec::new();
                if let Some(tree) = tree {
                    if !crate::history::history_relative_path(&requested_canonical, &canonical)
                        .is_some_and(|p| p.as_os_str().is_empty())
                    {
                        return Err(CoreError::new(
                            "operationConflict",
                            UiMessage::error_core_operations_template_source_differs_from_original_authorized_tree(),
                        ));
                    }
                    originals.extend(tree.files.clone());
                    original_dirs.extend(tree.directories.clone());
                } else {
                    snapshot_tree(
                        &self.backend,
                        source,
                        &mut originals,
                        &mut original_dirs,
                        false,
                    )?;
                }
                let mut instantiated = originals.clone();
                let owner = self
                    .admitted_binding(&original.stamp.identity)?
                    .ok_or_else(|| {
                        CoreError::new(
                            "templateSource",
                            UiMessage::error_core_operations_template_owner_unadmitted(),
                        )
                    })?;
                instantiated
                    .iter_mut()
                    .find(|f| f.path == original.path)
                    .unwrap()
                    .bytes = initialize_source(instantiated_source, &owner.node_id)?.into_bytes();
                let schema_path = root.join(ConfigRole::Schema.filename());
                let schema = if let Some(tree) = tree {
                    tree.schema.binding.captured.clone()
                } else {
                    capture_optional(&self.backend, &schema_path)?
                };
                let mut context = instantiated.clone();
                for witness in transfer_witnesses {
                    let captured = witness.binding.captured.clone().ok_or_else(|| {
                        CoreError::new(
                            "basisRequired",
                            UiMessage::error_core_operations_transfer_sources_require_original_present_bytes(),
                        )
                    })?;
                    // The template author proposal replaces only its own
                    // source in the transfer view, never the guarded original.
                    if !originals.iter().any(|f| {
                        crate::history::history_relative_path(
                            Path::new(&f.path),
                            Path::new(&captured.path),
                        )
                        .is_some_and(|p| p.as_os_str().is_empty())
                    }) {
                        context.push(captured);
                    }
                }
                let inventory = tree
                    .map(|_| self.feature_copy_inventory(&root, &context))
                    .transpose()?;
                let mutation = self.plan_copy(
                    &root,
                    source,
                    &target,
                    &instantiated,
                    &original_dirs,
                    &mut preview,
                    tree.is_some(),
                    schema.as_ref(),
                    inventory
                        .as_ref()
                        .map(|inventory| (inventory, context.as_slice())),
                )?;
                preview.new_node_id = preview.node_map.get(&owner.node_id).cloned();
                files.extend(originals);
                directories.extend(original_dirs);
                if let Some(config) = schema {
                    files.push(config);
                } else {
                    absent_dependencies.push(AbsentDependency {
                        path: display(&schema_path),
                        parent: ParentGuard::acquire(&self.backend, &schema_path)?
                            .directory_identity()?,
                    });
                }
                mutation
            }
            OperationIntent::BackupLibrary { destination } => {
                let target = absolute_path(Path::new(destination))?;
                if target.starts_with(&root)
                    || target.starts_with(&self.data_dir)
                    || root.starts_with(&target)
                    || self.data_dir.starts_with(&target)
                {
                    return Err(CoreError::new(
                        "recursiveBackup",
                        UiMessage::error_core_operations_backup_destination_must_outside_library_and_private_data_directory(),
                    ));
                }
                ensure_absent(&self.backend, &target)?;
                snapshot_tree(&self.backend, &root, &mut files, &mut directories, false)?;
                let library_count = files.len();
                let library_dirs = directories.len();
                let private = self.capture_owner_backup_private(
                    &root,
                    &workspace.workspace_id,
                    &mut files,
                    &mut directories,
                )?;
                backup_private_digest = Some(private.digest);
                let mut output = Vec::new();
                let mut dirs = vec![
                    "library".into(),
                    "private".into(),
                    "private/journal".into(),
                    "private/drafts".into(),
                    "private/draft-history".into(),
                    "private/capture-drafts".into(),
                ];
                for (index, d) in directories.iter().enumerate() {
                    let (base, prefix) = if index < library_dirs {
                        (&root, "library")
                    } else {
                        (&self.data_dir, "private")
                    };
                    let relative = Path::new(&d.path).strip_prefix(base).unwrap();
                    if !relative.as_os_str().is_empty() {
                        dirs.push(display(&Path::new(prefix).join(relative)));
                    }
                }
                for (index, f) in files.iter().enumerate() {
                    let (base, prefix) = if index < library_count {
                        (&root, "library")
                    } else {
                        (&self.data_dir, "private")
                    };
                    output.push(FileChange {
                        path: display(
                            &target
                                .join(prefix)
                                .join(Path::new(&f.path).strip_prefix(base).unwrap()),
                        ),
                        base_id: None,
                        before: None,
                        after: f.bytes.clone(),
                    });
                }
                // Selected private sync content has deeper directories than
                // journal/drafts. Include every actual output parent, without
                // enumerating other workspaces' private transport histories.
                for file in &output {
                    let mut parent = Path::new(&file.path).parent().unwrap();
                    while parent != target {
                        dirs.push(display(parent.strip_prefix(&target).unwrap()));
                        parent = parent.parent().unwrap();
                    }
                }
                let manifest = serde_json::to_vec_pretty(
                    &serde_json::json!({"version":1,"kind":"weftext.local-backup.v1","workspaceId":workspace.workspace_id,"sourceRoot":display(&root),"privateDataRoot":display(&self.data_dir),"operationId":request.operation_id,"storageProfile":STORAGE_PROFILE,"files":output.iter().map(|f| serde_json::json!({"path":Path::new(&f.path).strip_prefix(&target).unwrap().to_string_lossy(),"sha256":hash(&f.after),"bytes":f.after.len()})).collect::<Vec<_>>(),"restore":"Preserve originals. Physical object bindings require explicit restore/re-admission; do not reset private state or replace a live library."}),
                )?;
                output.push(FileChange {
                    path: display(&target.join("backup.json")),
                    base_id: None,
                    before: None,
                    after: manifest,
                });
                dirs.sort();
                dirs.dedup();
                if output.len().saturating_add(dirs.len()) > MAX_OPERATION_FILES
                    || output
                        .iter()
                        .try_fold(0usize, |total, f| total.checked_add(f.after.len()))
                        .is_none_or(|total| total > MAX_OPERATION_BYTES)
                {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_complete_backup_including_manifest_and_directories_exceeds64_mi_b_or5000_members(),
                    ));
                }
                // Use the same pure encoder as delivery, including its 80 MiB
                // ZIP/path/manifest budget, before staging or any user effect.
                let relative = output
                    .iter()
                    .map(|f| display(Path::new(&f.path).strip_prefix(&target).unwrap()))
                    .collect::<Vec<_>>();
                weftext_export::recovery_backup::encode_recovery_backup(
                    &dirs,
                    relative
                        .iter()
                        .zip(&output)
                        .map(|(path, f)| (path.as_str(), f.after.as_slice())),
                )
                .map_err(|_| {
                    CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_original_backup_outside_complete_bounded_z_i_p_delivery_profile(),
                    )
                })?;
                preview.changes.extend(
                    output
                        .iter()
                        .map(|f| change(Path::new(&f.path), "backup", f.after.len())),
                );
                Mutation::CreateTree {
                    existing_target: false,
                    target: display(&target),
                    directories: dirs,
                    files: output,
                    births: Vec::new(),
                    updates: Vec::new(),
                }
            }
            OperationIntent::ConfigureStyle {
                base_id,
                source,
                enabled,
            } => {
                check_size(source.as_bytes())?;
                let basis = self.style_bases.get(base_id).cloned().ok_or_else(|| {
                    CoreError::new(
                        "staleBase",
                        UiMessage::error_core_operations_read_style_again_before_preparing(),
                    )
                })?;
                if basis.root != display(&root) {
                    return Err(CoreError::new(
                        "outsideRoot",
                        UiMessage::error_core_operations_style_basis_belongs_to_another_library(),
                    ));
                }
                for b in [
                    &basis.settings,
                    basis.style.as_ref().unwrap_or(&basis.settings),
                    basis.manifest.as_ref().unwrap_or(&basis.settings),
                    basis.owner.as_ref().unwrap_or(&basis.settings),
                ] {
                    let current = capture_optional(&self.backend, Path::new(&b.path))?;
                    if b.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
                        != current.as_ref().map(|f| (&f.stamp, &f.bytes))
                    {
                        return Err(CoreError::new(
                            "staleBase",
                            UiMessage::error_core_operations_style_module_manifest_or_settings_changed_after_review(),
                        ));
                    }
                    if let Some(f) = current
                        && !files.iter().any(|old: &CapturedFile| old.path == f.path)
                    {
                        files.push(f);
                    }
                }
                let mut settings: serde_json::Value = basis
                    .settings
                    .captured
                    .as_ref()
                    .map(|f| serde_json::from_slice(&f.bytes))
                    .transpose()?
                    .unwrap_or_else(|| serde_json::json!({"version":1}));
                if !settings.is_object() {
                    return Err(CoreError::new(
                        "invalidState",
                        UiMessage::error_core_operations_workspace_settings_must_object(),
                    ));
                }
                let target = root.join("Styles");
                let module_path = target.join("library-style.typ");
                let resource_ref = basis.resource_ref.unwrap_or_else(|| {
                    format!("weftext-resource://{}/{}", workspace.workspace_id, fresh())
                });
                if settings.get("defaultStyle").is_none() {
                    settings["defaultStyle"] = serde_json::json!({});
                }
                if !settings["defaultStyle"].is_object() {
                    return Err(CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_operations_unknown_default_style_selector_cannot_overwritten(),
                    ));
                }
                settings["defaultStyle"]["enabled"] = (*enabled).into();
                settings["defaultStyle"]["resource"] = resource_ref.clone().into();
                settings["defaultStyle"]["entryFunction"] = "library-style".into();
                let settings_change = FileChange {
                    path: basis.settings.path,
                    base_id: None,
                    before: basis.settings.captured.map(|f| f.bytes),
                    after: serde_json::to_vec_pretty(&settings)?,
                };
                preview.changes.push(change(
                    Path::new(&settings_change.path),
                    "configure",
                    settings_change.after.len(),
                ));
                preview
                    .revisions
                    .insert(settings_change.path.clone(), fresh());
                preview
                    .changes
                    .push(change(&module_path, "style", source.len()));
                preview.revisions.insert(display(&module_path), fresh());
                if let Some(style) = basis.style {
                    Mutation::Update {
                        files: vec![
                            FileChange {
                                path: style.path,
                                base_id: None,
                                before: style.captured.map(|f| f.bytes),
                                after: source.as_bytes().to_vec(),
                            },
                            settings_change,
                        ],
                    }
                } else {
                    ensure_absent(&self.backend, &target)?;
                    self.owned_parent(&root, &root)?;
                    let node_id = fresh();
                    let node_path = target.join("Styles.typ");
                    let declaration = format!(
                        "#metadata((id: \"{node_id}\", types: ())) <weftext>\n= 库默认样式\n"
                    );
                    let resource_id = resource_ref.rsplit('/').next().unwrap();
                    preview
                        .resource_map
                        .insert(display(&module_path), resource_ref.clone());
                    let manifest = serde_json::to_vec_pretty(
                        &serde_json::json!({"version":1,"resources":[{"resourceId":resource_id,"path":"library-style.typ"}]}),
                    )?;
                    preview.new_node_id = Some(node_id.clone());
                    preview.revisions.insert(display(&node_path), fresh());
                    let output = vec![
                        FileChange {
                            path: display(&node_path),
                            base_id: None,
                            before: None,
                            after: declaration.into_bytes(),
                        },
                        FileChange {
                            path: display(&module_path),
                            base_id: None,
                            before: None,
                            after: source.as_bytes().to_vec(),
                        },
                        FileChange {
                            path: display(&target.join("weftext.resources.json")),
                            base_id: None,
                            before: None,
                            after: manifest,
                        },
                    ];
                    preview.changes.extend(
                        output
                            .iter()
                            .filter(|f| f.path != display(&module_path))
                            .map(|f| change(Path::new(&f.path), "create", f.after.len())),
                    );
                    Mutation::CreateTree {
                        existing_target: false,
                        target: display(&target),
                        directories: vec![],
                        files: output,
                        births: vec![NewBirth {
                            path: display(&node_path),
                            node_id,
                        }],
                        updates: vec![settings_change],
                    }
                }
            }
            OperationIntent::ImportResource {
                node,
                name,
                bytes,
                manifest_base_id,
                resource_base_id,
            } => {
                valid_name(name)?;
                if bytes.len() > MAX_OPERATION_BYTES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_resource_exceeds64_mi_b(),
                    ));
                }
                let node_path = absolute_path(Path::new(node))?;
                let directory = node_path.parent().ok_or_else(|| {
                    CoreError::new(
                        "invalidPath",
                        UiMessage::error_core_operations_missing_resource_owner(),
                    )
                })?;
                let canonical = self.managed_node(&root, directory, false)?;
                if Path::new(&canonical.path) != node_path {
                    return Err(CoreError::new(
                        "unsupportedRole",
                        UiMessage::error_core_operations_select_canonical_admitted_node_source(),
                    ));
                }
                let owner = self.admitted_binding(&canonical.stamp.identity)?.unwrap();
                let target = directory.join(name);
                let manifest_path = directory.join("weftext.resources.json");
                if target == node_path
                    || target == manifest_path
                    || name == "weftext.annotations.json"
                {
                    return Err(CoreError::new(
                        "resourceConflict",
                        UiMessage::error_core_operations_resource_cannot_replace_node_control_file(
                        ),
                    ));
                }
                let manifest = capture_optional(&self.backend, &manifest_path)?;
                self.verify_snapshot(&manifest_path, manifest_base_id, manifest.as_ref())?;
                if manifest
                    .as_ref()
                    .is_some_and(|file| file.bytes.len() > MAX_SOURCE_BYTES)
                {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_owned_resource_manifest_exceeds2_mi_b(),
                    ));
                }
                if manifest.is_none() {
                    self.require_resource_collection_uninitialized(&owner)?;
                }
                let current = capture_optional(&self.backend, &target)?;
                self.verify_snapshot(&target, resource_base_id, current.as_ref())?;
                if current.is_some() {
                    return Err(CoreError::new(
                        "exists",
                        UiMessage::error_core_operations_resource_import_create_only(),
                    ));
                }
                let mut value: serde_json::Value = manifest
                    .as_ref()
                    .map(|f| serde_json::from_slice(&f.bytes))
                    .transpose()?
                    .unwrap_or_else(|| serde_json::json!({"version":1,"resources":[]}));
                if value["version"] != 1 || !value["resources"].is_array() {
                    return Err(CoreError::new(
                        "resourceManifest",
                        UiMessage::error_core_operations_unsupported_resource_manifest(),
                    ));
                }
                if value["resources"].as_array().unwrap().len() >= 64 {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_at_most64_registered_resource_descriptors(
                        ),
                    ));
                }
                if value["resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["path"] == name.as_str())
                {
                    return Err(CoreError::new(
                        "resourceConflict",
                        UiMessage::error_core_operations_manifest_already_has_path(),
                    ));
                }
                let resource_id = fresh();
                let resource_ref = format!(
                    "weftext-resource://{}/{resource_id}",
                    workspace.workspace_id
                );
                value["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({"resourceId":resource_id,"path":name}));
                let manifest_after = serde_json::to_vec_pretty(&value)?;
                if manifest_after.len() > MAX_SOURCE_BYTES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_owned_resource_manifest_exceeds2_mi_b(),
                    ));
                }
                let changes = vec![
                    FileChange {
                        path: display(&target),
                        base_id: Some(resource_base_id.clone()),
                        before: None,
                        after: bytes.clone(),
                    },
                    FileChange {
                        path: display(&manifest_path),
                        base_id: Some(manifest_base_id.clone()),
                        before: manifest.as_ref().map(|f| f.bytes.clone()),
                        after: manifest_after,
                    },
                ];
                if let Some(f) = manifest {
                    files.push(f);
                }
                files.push(canonical);
                for c in &changes {
                    preview
                        .changes
                        .push(change(Path::new(&c.path), "import", c.after.len()));
                    preview.revisions.insert(c.path.clone(), fresh());
                }
                preview
                    .node_map
                    .insert(owner.node_id.clone(), owner.node_id);
                preview.resource_map.insert(display(&target), resource_ref);
                Mutation::Update { files: changes }
            }
        };
        self.validate_tag_mutation_changes(
            &root,
            &request.operation_id,
            &fingerprint,
            &mutation,
            false,
        )?;
        // Owned membership is an original committed dependency too. Reserve
        // its version in this same Plan before installation, so both normal
        // execution and original-plan recovery publish its exact observation.
        if let Mutation::CreateTree {
            files: output,
            births,
            ..
        } = &mutation
        {
            for file in output.iter().filter(|file| {
                Path::new(&file.path).file_name()
                    == Some(std::ffi::OsStr::new("weftext.resources.json"))
                    && births.iter().any(|birth| {
                        Path::new(&birth.path).parent() == Path::new(&file.path).parent()
                    })
            }) {
                preview
                    .revisions
                    .entry(file.path.clone())
                    .or_insert_with(fresh);
            }
        }
        let result_path = match &mutation {
            Mutation::Compound { .. } => {
                return Err(CoreError::new(
                    "invalidIntent",
                    UiMessage::error_core_operations_compound_must_originate_from_original_sync_planner(),
                ));
            }
            Mutation::Relocate { target, .. } | Mutation::CreateTree { target, .. } => {
                target.clone()
            }
            Mutation::Update { files } => files[0].path.clone(),
        };
        let receipt = OperationReceipt {
            operation_id: request.operation_id.clone(),
            plan_id,
            status: "committed".into(),
            path: result_path.clone(),
            node_ids: preview
                .node_map
                .values()
                .cloned()
                .chain(preview.new_node_id.clone())
                .collect(),
        };
        let mut plan = journal::Plan::file(
            request.operation_id.clone(),
            fingerprint,
            Path::new(&result_path),
            None,
            Vec::new(),
            None,
            &preview.kind,
        );
        plan.workspace_id = Some(workspace.workspace_id);
        plan.node_id = preview.new_node_id.clone();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let mut required_paths = match &mutation {
            Mutation::Update { files } => files
                .iter()
                .map(|file| file.path.clone())
                .collect::<BTreeSet<_>>(),
            Mutation::CreateTree { updates, .. }
            | Mutation::Relocate { updates, .. }
            | Mutation::Compound { updates, .. } => {
                updates.iter().map(|file| file.path.clone()).collect()
            }
        };
        if let Mutation::Relocate {
            source, old_leaf, ..
        } = &mutation
        {
            required_paths.insert(display(&Path::new(source).join(old_leaf)));
        }
        let mut unique = Vec::<CapturedFile>::new();
        let mut identities = BTreeMap::new();
        for file in files {
            let identity = &file.stamp.identity;
            let key = identity.clone();
            if let Some(&index) = identities.get(&key) {
                let original: &mut CapturedFile = &mut unique[index];
                // Dedup only a proven spelling alias of the same directory
                // entry. Different hardlink leaves are separate namespace
                // dependencies and cannot be folded into one read handle.
                if file.stamp != original.stamp
                    || file.bytes != original.bytes
                    || !crate::history::history_relative_path(
                        Path::new(&file.path),
                        Path::new(&original.path),
                    )
                    .is_some_and(|relative| relative.as_os_str().is_empty())
                    || !backend::same_directory(
                        &self.backend,
                        Path::new(&file.path).parent().unwrap(),
                        Path::new(&original.path).parent().unwrap(),
                    )?
                {
                    return Err(CoreError::new(
                        "staleBase",
                        UiMessage::error_core_operations_competing_frozen_versions_or_locators_for_one_physical_dependency(),
                    ));
                }
                if required_paths.contains(&file.path) {
                    if required_paths.contains(&original.path) && file.path != original.path {
                        return Err(CoreError::new(
                            "invalidIntent",
                            UiMessage::error_core_operations_one_physical_writer_has_multiple_distinct_requested_spellings(),
                        ));
                    }
                    *original = file;
                }
            } else {
                identities.insert(key, unique.len());
                unique.push(file);
            }
        }
        let files = unique;
        let mut write_parents = Vec::new();
        let writes = match &mutation {
            Mutation::Update { files } => files.iter().collect::<Vec<_>>(),
            Mutation::CreateTree { updates, .. } => updates.iter().collect(),
            _ => vec![],
        };
        for c in writes {
            write_parents.push((
                c.path.clone(),
                ParentGuard::acquire(&self.backend, Path::new(&c.path))?.directory_identity()?,
            ));
        }
        if let Mutation::CreateTree { target, .. } = &mutation {
            write_parents.push((
                target.clone(),
                ParentGuard::acquire(&self.backend, Path::new(target))?.directory_identity()?,
            ));
        }
        if let Some(destination) = destination_slot {
            self.validate_hosted_destination(&root, &request.intent, destination, Some(&mutation))?;
        }
        plan.transaction = Some(FrozenOperation {
            request,
            preview: preview.clone(),
            files,
            directories,
            mutation,
            receipt,
            hosted_destination: destination_slot.cloned(),
            expected_membership: cut.map(|c| c.members),
            expected_trash_membership: None,
            backup_private_digest,
            absent_dependencies,
            write_parents,
        });
        self.stage(plan)?;
        Ok(preview)
    }
    pub fn operation_details(&self, operation_id: &str) -> Result<OperationPreview> {
        self.backend.check_path(&self.data_dir)?;
        self.check_sync_operation_access(operation_id)?;
        let p = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    UiMessage::error_core_operations_no_frozen_operation_has_i_d(),
                )
            })?;
        let mut preview = p.preview.clone();
        preview.status = if self.committed(operation_id)? {
            "committed"
        } else if self.resolved(operation_id)? {
            "resolved"
        } else if self
            .backend
            .entry_exists(self.journal_path(operation_id, "install"))?
        {
            "recoveryPending"
        } else {
            "prepared"
        }
        .into();
        Ok(preview)
    }
    pub fn execute_operation(
        &mut self,
        operation_id: &str,
        plan_id: &str,
    ) -> Result<OperationReceipt> {
        // Reading a damaged decision must fail before a mutation, never become
        // a guessed absence inside the error-classification closure below.
        let resolved = if self.committed(operation_id)? {
            false
        } else {
            self.resolved(operation_id)?
        };
        match self.execute_operation_inner(operation_id, plan_id) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                let started = self
                    .backend
                    .entry_exists(self.journal_path(operation_id, "install"))?;
                if started && !self.committed(operation_id)? && !resolved {
                    Err(CoreError::new(
                        "recoveryPending",
                        UiMessage::error_core_recovery_installation_unresolved(
                            weftext_messages::params::ErrorCoreRecoveryInstallationUnresolved {
                                operation_id: operation_id.into(),
                            },
                        ),
                    )
                    .caused_by(error))
                } else {
                    Err(error)
                }
            }
        }
    }

    fn execute_operation_inner(
        &mut self,
        operation_id: &str,
        plan_id: &str,
    ) -> Result<OperationReceipt> {
        let plan = self.operations.get(operation_id).cloned().ok_or_else(|| {
            CoreError::new(
                "operationMissing",
                UiMessage::error_core_operations_prepare_operation_first(),
            )
        })?;
        let t = plan.transaction.as_ref().ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                UiMessage::error_core_operations_i_d_belongs_to_legacy_operation(),
            )
        })?;
        if matches!(
            t.request.intent,
            OperationIntent::CloneSync { .. }
                | OperationIntent::ApplySync { .. }
                | OperationIntent::MergeSync { .. }
                | OperationIntent::RevalidateSync { .. }
                | OperationIntent::AdoptSyncExternal { .. }
        ) {
            self.sync_local_owner()?;
        }
        if t.preview.plan_id != plan_id {
            return Err(CoreError::new(
                "planConflict",
                UiMessage::error_core_operations_execute_exact_reviewed_frozen_plan_i_d(),
            ));
        }
        if self.committed(operation_id)? {
            return Ok(t.receipt.clone());
        }
        if self.resolved(operation_id)?
            || self
                .backend
                .entry_exists(self.journal_path(operation_id, "install"))?
        {
            return Err(CoreError::new(
                "recoveryPending",
                UiMessage::error_core_operations_original_operation_may_have_effects_inspect_it_and_do_not_execute_again(),
            ));
        }
        self.validate_operation_pending(&plan)?;
        self.validate_sync_application(&plan)?;
        let root = Path::new(&t.request.root);
        if let OperationIntent::CloneSync { identity, .. } = &t.request.intent
            && self
                .sync_setups()?
                .iter()
                .any(|s| s.identity.workspace_id == identity.workspace_id)
        {
            return Err(CoreError::new(
                "syncReplicaExists",
                UiMessage::error_core_operations_another_replica_of_workspace_was_committed_in_private_state(),
            ));
        }
        if !matches!(t.request.intent, OperationIntent::CloneSync { .. }) {
            self.require_workspace(root)?;
        }
        if let Some(destination) = &t.hosted_destination {
            self.validate_hosted_destination(
                root,
                &t.request.intent,
                destination,
                Some(&t.mutation),
            )?;
        }
        self.validate_resource_collection_initialization(t)?;
        self.validate_tag_plan_changes(&plan)?;
        if let Some(expected) = &t.expected_membership {
            let current = self.admitted_snapshot(root)?;
            let same = current.complete
                && current
                    .membership_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    == *expected;
            self.release_admitted_cut(&current.cut_id);
            if !same {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_admitted_set_membership_changed_after_reviewed_strong_plan(),
                ));
            }
        }
        if let Some(expected) = &t.expected_trash_membership {
            let current = self.trash_admitted_snapshot(root)?;
            let same = current.complete
                && current
                    .membership_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    == *expected;
            self.release_admitted_cut(&current.cut_id);
            if !same {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_admitted_trash_membership_changed_after_reviewed_original_plan(),
                ));
            }
        }
        // ImportResource and ConfigureStyle build their closed write set in
        // Core's role-specific planner. Generic UpdateFiles only permits
        // unchanged resource dependencies. Both retain every original file,
        // owner, manifest and parent guard through the same decision below.
        if let Mutation::Update { files: changes } = &t.mutation
            && !matches!(
                t.request.intent,
                OperationIntent::ImportResource { .. }
                    | OperationIntent::ConfigureStyle { .. }
                    | OperationIntent::ApplySync { .. }
                    | OperationIntent::MergeSync { .. }
                    | OperationIntent::RevalidateSync { .. }
                    | OperationIntent::AdoptSyncExternal { .. }
            )
        {
            for c in changes {
                self.validate_update(root, Path::new(&c.path), &c.after)?;
                self.validate_navigation_order_update(root, c)?;
                if Path::new(&c.path)
                    .file_name()
                    .is_some_and(|n| n == "weftext.directory.json")
                {
                    self.validate_directory_plan(operation_id, root, Path::new(&c.path), &c.after)?;
                }
            }
        }
        if matches!(t.mutation, Mutation::Compound { .. }) {
            self.execute_compound(&plan)?;
            for session in self
                .sessions
                .values_mut()
                .filter(|session| t.affects(&session.path))
            {
                session.base_id = fresh();
            }
            return Ok(t.receipt.clone());
        }
        // All existing files are guarded together before the first write. This
        // gives a consistent bounded source set and prevents current reads of a
        // partially written annotation/source pair.
        let mut guards = Vec::new();
        let mut handles = Vec::new();
        let mut observations = Vec::new();
        let _private_capture = self.validate_owner_backup_private(&plan, false)?;
        for (path, parent) in &t.write_parents {
            let guard = ParentGuard::acquire(&self.backend, Path::new(path))?;
            if guard.directory_identity()? != *parent {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_frozen_write_parent_was_replaced(),
                ));
            }
            guards.push(guard);
        }
        for absent in &t.absent_dependencies {
            let guard = ParentGuard::acquire(&self.backend, Path::new(&absent.path))?;
            if guard.directory_identity()? != absent.parent {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_operations_absent_dependency_s_original_parent_was_replaced(),
                ));
            }
            match guard.open_file(false, false) {
                Err(e) if e.code == "missing" => {}
                Ok(_) => {
                    return Err(CoreError::new(
                        "scopeChanged",
                        UiMessage::error_core_operations_original_absent_source_config_dependency_now_exists(),
                    ));
                }
                Err(e) => return Err(e),
            }
            guards.push(guard);
        }
        if !matches!(t.mutation, Mutation::Relocate { .. }) {
            for f in &t.files {
                if !matches!(t.request.intent, OperationIntent::BackupLibrary { .. }) {
                    self.check_pending(Path::new(&f.path))?;
                }
                let guard = ParentGuard::acquire(&self.backend, Path::new(&f.path))?;
                let write = matches!(&t.mutation, Mutation::Update {files} if files.iter().any(|c|c.path==f.path && c.before.as_ref()!=Some(&c.after)))
                    || matches!(&t.mutation, Mutation::CreateTree {updates,..} if updates.iter().any(|c|c.path == f.path && c.before.as_ref()!=Some(&c.after)));
                let mut handle = if !write
                    && (matches!(t.request.intent, OperationIntent::BackupLibrary { .. })
                        || operation_read_control(root, Path::new(&f.path)))
                {
                    guard.open_read_guard_file()?
                } else {
                    guard.open_file(write, false)?
                };
                if stamp(&handle)? != f.stamp || read_blob(&mut handle)? != f.bytes {
                    return Err(CoreError::new(
                        "conflict",
                        UiMessage::error_core_operations_frozen_source_object_or_bytes_changed_prepare_new_operation(),
                    ));
                }
                guards.push(guard);
                handles.push(handle);
            }
            for d in &t.directories {
                let guard = ParentGuard::acquire(
                    &self.backend,
                    &Path::new(&d.path).join(".snapshot-prospective"),
                )?;
                let (entries, complete) = guard.directory_entries(MAX_OPERATION_FILES)?;
                let names = sorted_entries(entries, Path::new(&d.path));
                if !complete
                    || guard.directory_identity()? != d.identity
                    || crate::author_order::navigation_order_directory_entries(
                        t,
                        Path::new(&d.path),
                        names,
                    ) != crate::author_order::navigation_order_directory_entries(
                        t,
                        Path::new(&d.path),
                        d.entries.clone(),
                    )
                {
                    return Err(CoreError::new(
                        "scopeChanged",
                        UiMessage::error_core_operations_frozen_directory_membership_changed_or_became_incomplete(),
                    ));
                }
                guards.push(guard);
            }
        }
        match &t.mutation {
            Mutation::Compound { .. } => {
                unreachable!("compound has one dedicated original execution above")
            }
            Mutation::Relocate {
                source,
                target,
                source_identity,
                target_parent_identity,
                create_trash_parent,
                old_leaf,
                new_leaf,
                leaf_identity,
                updates,
                members,
            } => {
                let source = Path::new(source);
                let target = Path::new(target);
                if !source
                    .components()
                    .any(|c| c.as_os_str() == ".weftext-trash")
                {
                    self.check_subtree_boundary(root, source)?;
                }
                if !target
                    .components()
                    .any(|c| c.as_os_str() == ".weftext-trash")
                {
                    self.owned_parent(root, target.parent().unwrap())?;
                }
                for file in t
                    .files
                    .iter()
                    .filter(|f| !Path::new(&f.path).starts_with(source))
                {
                    self.check_pending(Path::new(&file.path))?;
                    let guard = ParentGuard::acquire(&self.backend, Path::new(&file.path))?;
                    let write = updates
                        .iter()
                        .any(|c| c.path == file.path && c.before.as_ref() != Some(&c.after));
                    let mut handle =
                        if !write && operation_read_control(root, Path::new(&file.path)) {
                            guard.open_read_guard_file()?
                        } else {
                            guard.open_file(write, false)?
                        };
                    if stamp(&handle)? != file.stamp || read_blob(&mut handle)? != file.bytes {
                        return Err(CoreError::new(
                            "conflict",
                            UiMessage::error_core_operations_relocation_prerequisite_changed_after_review(),
                        ));
                    }
                    guards.push(guard);
                    handles.push(handle);
                }
                if plan.sync_application.is_some() {
                    for file in &t.files {
                        self.check_pending(Path::new(&file.path))?;
                    }
                    let mut preliminary = NamespaceGuard::acquire(
                        &self.backend,
                        source,
                        if *create_trash_parent {
                            root
                        } else {
                            target.parent().unwrap()
                        },
                    )?;
                    if preliminary.source_identity()? != *source_identity
                        || preliminary.parent_identity()? != *target_parent_identity
                    {
                        return Err(CoreError::new(
                            "conflict",
                            UiMessage::error_core_operations_original_relocation_closure_changed(),
                        ));
                    }
                    let held = validate_relocation_tree(&mut preliminary, t, false)?;
                    drop(held);
                    validate_relocation_creations(&self.backend, &mut preliminary, t)?;
                }
                let mut trash_guard = None;
                let mut trash_directory = None;
                if *create_trash_parent {
                    let guard = ParentGuard::acquire(&self.backend, target.parent().unwrap())?;
                    if guard.directory_identity()? != *target_parent_identity {
                        return Err(CoreError::new(
                            "conflict",
                            UiMessage::error_core_operations_trash_parent_closure_changed(),
                        ));
                    }
                    ensure_absent(&self.backend, target.parent().unwrap())?;
                    self.install_record(&plan)?;
                    let directory = guard.create_directory()?;
                    self.record_materialized(operation_id, target.parent().unwrap(), &directory)?;
                    trash_directory = Some(directory);
                    trash_guard = Some(guard);
                }
                let mut namespace =
                    NamespaceGuard::acquire(&self.backend, source, target.parent().unwrap())?;
                if namespace.source_identity()? != *source_identity
                    || (!create_trash_parent
                        && namespace.parent_identity()? != *target_parent_identity)
                {
                    return Err(CoreError::new(
                        "conflict",
                        UiMessage::error_core_operations_source_directory_or_target_parent_was_replaced(),
                    ));
                }
                if plan.sync_application.is_some() {
                    let frozen = validate_relocation_tree(&mut namespace, t, false)?;
                    drop(frozen);
                    namespace.release_source_descendants();
                }
                let member_handles = validate_relocation_members(&mut namespace, members)?;
                drop(member_handles);
                namespace.release_source_descendants();
                let mut leaf = namespace.open_source_file(old_leaf, old_leaf != new_leaf)?;
                let captured = t
                    .files
                    .iter()
                    .find(|f| f.path == display(&source.join(old_leaf)))
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_missing_canonical_preimage(),
                        )
                    })?;
                if stamp(&leaf)?.identity != *leaf_identity
                    || read_source(&mut leaf)? != captured.bytes
                {
                    return Err(CoreError::new(
                        "conflict",
                        UiMessage::error_core_operations_canonical_source_changed_after_preview(),
                    ));
                }
                if source == target {
                    self.install_record(&plan)?;
                    self.commit(&plan)?;
                    return Ok(t.receipt.clone());
                }
                ensure_absent(&self.backend, target)?;
                if !create_trash_parent {
                    self.install_record(&plan)?;
                }
                if old_leaf != new_leaf {
                    namespace.rename_leaf(&leaf, new_leaf)?;
                }
                drop(leaf);
                namespace.move_source(target.file_name().unwrap().to_str().ok_or_else(
                    || {
                        CoreError::new(
                            "invalidName",
                            UiMessage::error_core_operations_non_u_t_f8_target_name(),
                        )
                    },
                )?)?;
                // Windows denies renaming an ancestor while its members have
                // no-delete handles. The original namespace closure remains
                // held across that single step; reacquire and verify members
                // from that same physical root before any commit.
                let member_handles = validate_relocation_members(&mut namespace, members)?;
                let reopened = namespace.open_moved_file(new_leaf)?;
                if stamp(&reopened)?.identity != *leaf_identity {
                    return Err(CoreError::new(
                        "recoveryPending",
                        UiMessage::error_core_operations_canonical_object_binding_changed_during_rename(),
                    ));
                }
                drop(reopened);
                let mut retained = if plan.sync_application.is_some() {
                    validate_relocation_tree(&mut namespace, t, true)?
                } else {
                    vec![]
                };
                let mut created = Vec::new();
                if plan.sync_application.is_some() {
                    // All existing participants are retained at the verified new
                    // placement before any content write or create-only output.
                    for change in updates.iter().filter(|c| c.before.is_none()) {
                        let path = Path::new(&change.path);
                        let file = if let Ok(relative) = path.strip_prefix(target) {
                            namespace.open_source_output(relative, true, true)?
                        } else {
                            let guard = ParentGuard::acquire(&self.backend, path)?;
                            let file = guard.open_file(true, true)?;
                            guards.push(guard);
                            file
                        };
                        self.record_materialized(operation_id, path, &file)?;
                        created.push((change.path.clone(), file));
                    }
                    let changes = t.relocated_files();
                    let mut participant_paths = Vec::new();
                    let mut participants = Vec::new();
                    for (file, handle) in t
                        .files
                        .iter()
                        .filter(|f| Path::new(&f.path).starts_with(source))
                        .zip(&mut retained)
                    {
                        participant_paths.push(t.relocated_path(&file.path));
                        participants.push(handle);
                    }
                    for (file, handle) in t
                        .files
                        .iter()
                        .filter(|f| !Path::new(&f.path).starts_with(source))
                        .zip(&mut handles)
                    {
                        participant_paths.push(file.path.clone());
                        participants.push(handle);
                    }
                    for (path, file) in &mut created {
                        participant_paths.push(path.clone());
                        participants.push(file);
                    }
                    let mut stamps = Vec::new();
                    for (path, handle) in participant_paths.iter().zip(participants) {
                        let change = changes.iter().find(|c| &c.path == path).ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                UiMessage::error_core_operations_missing_compound_participant(),
                            )
                        })?;
                        if change.before.as_ref() != Some(&change.after) {
                            write_blob(handle, &change.after)?;
                        }
                        if read_blob(handle)? != change.after {
                            return Err(CoreError::new(
                                "recoveryPending",
                                UiMessage::error_core_operations_compound_afterimage_verification_failed(),
                            ));
                        }
                        stamps.push((path.clone(), stamp(handle)?));
                    }
                    validate_relocation_membership(&mut namespace, t, true, true)?;
                    self.publish_recovery_facts(&plan, &stamps)?;
                }
                self.commit(&plan)?;
                drop(member_handles);
                drop(retained);
                drop(created);
                drop(trash_directory);
                drop(trash_guard);
            }
            Mutation::Update { files: changes } => {
                let mut created = Vec::new();
                for c in changes.iter().filter(|c| c.before.is_none()) {
                    let guard = ParentGuard::acquire(&self.backend, Path::new(&c.path))?;
                    ensure_absent(&self.backend, Path::new(&c.path))?;
                    created.push((guard, c));
                }
                self.install_record(&plan)?;
                for c in changes.iter().filter(|c| c.before.is_some()) {
                    let index = t.files.iter().position(|f| f.path == c.path).unwrap();
                    if c.before.as_ref() != Some(&c.after) {
                        write_blob(&mut handles[index], &c.after)?;
                    }
                    observations.push(RevisionFact {
                        path: c.path.clone(),
                        stamp: stamp(&handles[index])?,
                        hash: hash(&c.after),
                        revision: t.preview.revisions[&c.path].clone(),
                    });
                }
                let mut resource_births = Vec::new();
                for (guard, c) in created {
                    let mut file = guard.open_file(true, true)?;
                    self.record_materialized(operation_id, Path::new(&c.path), &file)?;
                    write_blob(&mut file, &c.after)?;
                    observations.push(RevisionFact {
                        path: c.path.clone(),
                        stamp: stamp(&file)?,
                        hash: hash(&c.after),
                        revision: t.preview.revisions[&c.path].clone(),
                    });
                    if let Some(reference) = t.preview.resource_map.get(&c.path) {
                        resource_births.push(ResourceBirth {
                            resource_id: reference.rsplit('/').next().unwrap().into(),
                            workspace_id: plan.workspace_id.clone().unwrap(),
                            node_id: t
                                .updated_resource_owner(
                                    Path::new(&c.path),
                                    plan.sync_application.as_ref(),
                                )
                                .ok_or_else(|| {
                                    CoreError::new(
                                        "invalidState",
                                        UiMessage::error_core_operations_resource_owner_missing(),
                                    )
                                })?,
                            identity: stamp(&file)?.identity,
                        });
                    }
                    handles.push(file);
                    guards.push(guard);
                }
                if !resource_births.is_empty() {
                    journal::private_create(
                        &self.backend,
                        &self.journal_path(operation_id, "resource-births"),
                        &serde_json::to_vec(&resource_births)?,
                    )?;
                }
                journal::private_create(
                    &self.backend,
                    &self.journal_path(operation_id, "observations"),
                    &serde_json::to_vec(&observations)?,
                )?;
                self.commit(&plan)?;
            }
            Mutation::CreateTree {
                target,
                existing_target,
                directories,
                files: output,
                births,
                updates,
            } => {
                if *existing_target {
                    if !matches!(
                        t.request.intent,
                        OperationIntent::ApplySync { .. }
                            | OperationIntent::MergeSync { .. }
                            | OperationIntent::RevalidateSync { .. }
                    ) || Path::new(target) != root
                    {
                        return Err(CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_existing_tree_creation_requires_original_sync_workspace(),
                        ));
                    }
                } else if !matches!(
                    t.request.intent,
                    OperationIntent::BackupLibrary { .. } | OperationIntent::CloneSync { .. }
                ) {
                    self.owned_parent(root, Path::new(target).parent().unwrap())?;
                }
                let guard_path = if *existing_target {
                    root.join(".sync-root")
                } else {
                    PathBuf::from(target)
                };
                let guard = ParentGuard::acquire(&self.backend, &guard_path)?;
                if !existing_target {
                    ensure_absent(&self.backend, Path::new(target))?;
                }
                for relative in directories {
                    ensure_absent(&self.backend, &Path::new(target).join(relative))?;
                }
                for c in output {
                    ensure_absent(&self.backend, Path::new(&c.path))?;
                }
                let mut update_creates = Vec::new();
                for c in updates.iter().filter(|c| c.before.is_none()) {
                    let g = ParentGuard::acquire(&self.backend, Path::new(&c.path))?;
                    ensure_absent(&self.backend, Path::new(&c.path))?;
                    update_creates.push((g, c));
                }
                self.install_record(&plan)?;
                let mut dir_handles = BTreeMap::new();
                if !existing_target {
                    let directory = guard.create_directory()?;
                    self.record_materialized(operation_id, Path::new(target), &directory)?;
                    dir_handles.insert(PathBuf::new(), directory);
                }
                let mut dirs = directories.iter().map(PathBuf::from).collect::<Vec<_>>();
                dirs.sort_by(|a, b| {
                    a.components()
                        .count()
                        .cmp(&b.components().count())
                        .then_with(|| a.cmp(b))
                });
                dirs.dedup();
                for relative in dirs {
                    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
                    let handle = if let Some(parent) = dir_handles.get(parent) {
                        ParentGuard::child_directory(
                            parent,
                            relative.file_name().unwrap().to_str().unwrap(),
                        )?
                    } else if *existing_target {
                        let parent_guard = ParentGuard::acquire(
                            &self.backend,
                            &Path::new(target).join(&relative),
                        )?;
                        let handle = parent_guard.create_directory()?;
                        guards.push(parent_guard);
                        handle
                    } else {
                        return Err(CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_missing_planned_parent_directory(),
                        ));
                    };
                    self.record_materialized(
                        operation_id,
                        &Path::new(target).join(&relative),
                        &handle,
                    )?;
                    dir_handles.insert(relative, handle);
                }
                let mut new_births = Vec::new();
                let mut resource_births = Vec::new();
                for c in output {
                    let relative = Path::new(&c.path).strip_prefix(target).map_err(|_| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_planned_file_escaped_target(),
                        )
                    })?;
                    let mut file = if let Some(parent) =
                        dir_handles.get(relative.parent().unwrap_or_else(|| Path::new("")))
                    {
                        ParentGuard::child_file(
                            parent,
                            relative.file_name().unwrap().to_str().unwrap(),
                        )?
                    } else if *existing_target {
                        let parent_guard = ParentGuard::acquire(&self.backend, Path::new(&c.path))?;
                        let file = parent_guard.open_file(true, true)?;
                        guards.push(parent_guard);
                        file
                    } else {
                        return Err(CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_missing_planned_file_parent(),
                        ));
                    };
                    self.record_materialized(operation_id, Path::new(&c.path), &file)?;
                    write_blob(&mut file, &c.after)?;
                    if let Some(revision) = t.preview.revisions.get(&c.path) {
                        observations.push(RevisionFact {
                            path: c.path.clone(),
                            stamp: stamp(&file)?,
                            hash: hash(&c.after),
                            revision: revision.clone(),
                        });
                    }
                    if let Some(b) = births.iter().find(|b| b.path == c.path) {
                        new_births.push(Birth {
                            node_id: b.node_id.clone(),
                            workspace_id: plan.workspace_id.clone().unwrap(),
                            identity: stamp(&file)?.identity,
                        });
                    }
                    if let Some(reference) = t.preview.resource_map.get(&c.path) {
                        let owner = if plan.sync_application.is_some() {
                            t.updated_resource_owner(
                                Path::new(&c.path),
                                plan.sync_application.as_ref(),
                            )
                        } else {
                            births
                                .iter()
                                .filter(|b| {
                                    Path::new(&c.path)
                                        .starts_with(Path::new(&b.path).parent().unwrap())
                                })
                                .max_by_key(|b| Path::new(&b.path).components().count())
                                .map(|b| b.node_id.clone())
                        }
                        .ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                UiMessage::error_core_operations_fresh_resource_lacks_reviewed_owner(),
                            )
                        })?;
                        resource_births.push(ResourceBirth {
                            resource_id: reference.rsplit('/').next().unwrap().into(),
                            workspace_id: plan.workspace_id.clone().unwrap(),
                            node_id: owner,
                            identity: stamp(&file)?.identity,
                        });
                    }
                    handles.push(file);
                }
                for c in updates.iter().filter(|c| c.before.is_some()) {
                    let index = t
                        .files
                        .iter()
                        .position(|f| f.path == c.path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "invalidPlan",
                                UiMessage::error_core_operations_missing_config_preimage(),
                            )
                        })?;
                    if c.before.as_ref() != Some(&c.after) {
                        write_blob(&mut handles[index], &c.after)?;
                    }
                    observations.push(RevisionFact {
                        path: c.path.clone(),
                        stamp: stamp(&handles[index])?,
                        hash: hash(&c.after),
                        revision: t.preview.revisions[&c.path].clone(),
                    });
                }
                for (g, c) in update_creates {
                    let mut f = g.open_file(true, true)?;
                    self.record_materialized(operation_id, Path::new(&c.path), &f)?;
                    write_blob(&mut f, &c.after)?;
                    observations.push(RevisionFact {
                        path: c.path.clone(),
                        stamp: stamp(&f)?,
                        hash: hash(&c.after),
                        revision: t.preview.revisions[&c.path].clone(),
                    });
                    handles.push(f);
                    guards.push(g);
                }
                if !new_births.is_empty() {
                    journal::private_create(
                        &self.backend,
                        &self.journal_path(operation_id, "births"),
                        &serde_json::to_vec(&new_births)?,
                    )?;
                }
                if !resource_births.is_empty() {
                    journal::private_create(
                        &self.backend,
                        &self.journal_path(operation_id, "resource-births"),
                        &serde_json::to_vec(&resource_births)?,
                    )?;
                }
                journal::private_create(
                    &self.backend,
                    &self.journal_path(operation_id, "observations"),
                    &serde_json::to_vec(&observations)?,
                )?;
                let _private_final = self.validate_owner_backup_private(&plan, true)?;
                self.commit(&plan)?;
            }
        }
        // Existing UI sessions retain their drafts but their bases become stale.
        for session in self.sessions.values_mut().filter(|s| t.affects(&s.path)) {
            session.base_id = fresh();
        }
        Ok(t.receipt.clone())
    }
    pub(crate) fn install_record(&self, plan: &journal::Plan) -> Result<()> {
        journal::private_create(
            &self.backend,
            &self.journal_path(&plan.operation_id, "install"),
            &serde_json::to_vec(
                &serde_json::json!({"operationId":plan.operation_id,"fingerprint":plan.fingerprint}),
            )?,
        )
    }
    pub(crate) fn require_workspace(&self, root: &Path) -> Result<WorkspaceConfig> {
        self.workspace(root)?.ok_or_else(|| {
            CoreError::new(
                "unadmittedLibrary",
                UiMessage::error_core_operations_library_needs_original_committed_private_identity_journal(),
            )
        })
    }
    pub(crate) fn owned_parent(&self, root: &Path, parent: &Path) -> Result<()> {
        if !parent.starts_with(root)
            || parent
                .strip_prefix(root)
                .unwrap()
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with(".weftext-"))
        {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_target_outside_live_managed_library(),
            ));
        }
        self.check_discovery_boundary(root, parent)?;
        let _ = ParentGuard::acquire(&self.backend, &parent.join(".parent-prospective"))?;
        Ok(())
    }
    pub(crate) fn managed_node(
        &self,
        root: &Path,
        directory: &Path,
        allow_trash: bool,
    ) -> Result<CapturedFile> {
        if !directory.starts_with(root) {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_node_outside_library(),
            ));
        }
        if !allow_trash {
            self.check_subtree_boundary(root, directory)?;
        }
        let name = node_name(directory)?;
        let path = directory.join(format!("{name}.typ"));
        let file = capture_optional(&self.backend, &path)?.ok_or_else(|| {
            CoreError::new(
                "missingNode",
                weftext_messages::UiMessage::error_core_operations_canonical_source_missing(),
            )
        })?;
        self.check_pending(&path)?;
        let birth = self
            .admitted_binding(&file.stamp.identity)?
            .ok_or_else(|| {
                CoreError::new(
                    "identityConflict",
                    weftext_messages::UiMessage::error_core_operations_source_object_has_no_committed_admission(),
                )
            })?;
        let workspace = self.require_workspace(root)?;
        if birth.workspace_id != workspace.workspace_id
            || weftext_engine::inspect(&decode(&file.bytes)?).id.as_ref() != Some(&birth.node_id)
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_canonical_source_declaration_and_admission_differ(
                ),
            ));
        }
        Ok(file)
    }
    pub(crate) fn prepare_relocation(
        &self,
        root: &Path,
        source: &Path,
        target: &Path,
        trash: bool,
        files: &mut Vec<CapturedFile>,
        preview: &mut OperationPreview,
    ) -> Result<Mutation> {
        self.prepare_relocation_with_target(root, source, target, trash, files, preview, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_relocation_with_target(
        &self,
        root: &Path,
        source: &Path,
        target: &Path,
        trash: bool,
        files: &mut Vec<CapturedFile>,
        preview: &mut OperationPreview,
        occupant: Option<&FileIdentity>,
    ) -> Result<Mutation> {
        // Trash folders use operation UUID names but keep the original canonical
        // basename. Restore obtains that name from the original frozen record.
        let trash_origin = if source
            .components()
            .any(|c| c.as_os_str() == ".weftext-trash")
        {
            Some(
                self.original_trash_roots(root)?
                    .into_iter()
                    .find(|entry| {
                        entry.node_id.is_some()
                            && locator_equal(&display(&entry.path), &display(source))
                    })
                    .ok_or_else(|| {
                        CoreError::new(
                            "trashMissing",
                            UiMessage::error_core_operations_no_committed_record_binds_trash_directory(),
                        )
                    })?,
            )
        } else {
            None
        };
        let mut file = if let Some(entry) = &trash_origin {
            self.capture_original_trash_node(entry)?
        } else {
            self.managed_node(root, source, false)?
        };
        self.check_pending(Path::new(&file.path))?;
        let birth = self
            .admitted_binding(&file.stamp.identity)?
            .ok_or_else(|| {
                CoreError::new(
                    "identityConflict",
                    weftext_messages::UiMessage::error_core_operations_original_admitted_node_object_was_replaced(),
                )
            })?;
        let workspace = self.require_workspace(root)?;
        if birth.workspace_id != workspace.workspace_id
            || weftext_engine::inspect(&decode(&file.bytes)?).id.as_ref() != Some(&birth.node_id)
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_node_declaration_or_workspace_changed(),
            ));
        }
        if !trash
            || !target
                .components()
                .any(|c| c.as_os_str() == ".weftext-trash")
        {
            self.owned_parent(root, target.parent().unwrap())?;
        }
        let create_trash_parent = target
            .parent()
            .map(|p| -> Result<bool> {
                Ok(p == root.join(".weftext-trash") && !self.backend.entry_exists(p)?)
            })
            .transpose()?
            .unwrap_or(false);
        let mut namespace = NamespaceGuard::acquire(
            &self.backend,
            source,
            if create_trash_parent {
                root
            } else {
                target.parent().unwrap()
            },
        )?;
        let old_leaf = Path::new(&file.path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let new_leaf = if target
            .components()
            .any(|c| c.as_os_str() == ".weftext-trash")
        {
            old_leaf.clone()
        } else {
            format!("{}.typ", node_name(target)?)
        };
        if source != target {
            qualify_relocation_target(&self.backend, target, occupant)?;
        }
        if old_leaf != new_leaf && namespace.source_entry_exists(Path::new(&new_leaf))? {
            return Err(CoreError::new(
                "exists",
                weftext_messages::UiMessage::error_core_operations_create_only_target_occupied_no_overwrite_allowed(),
            ));
        }
        let source_identity = namespace.source_identity()?;
        if trash_origin
            .as_ref()
            .is_some_and(|entry| entry.identity != source_identity)
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_operations_original_committed_trash_directory_was_replaced(),
            ));
        }
        let target_parent_identity = namespace.parent_identity()?;
        // Compare the exact canonical file through the namespace's actual source handle.
        let mut current = namespace.open_source_file(&old_leaf, false)?;
        if stamp(&current)? != file.stamp || read_source(&mut current)? != file.bytes {
            return Err(CoreError::new(
                "conflict",
                UiMessage::error_core_operations_canonical_object_changed_during_planning(),
            ));
        }
        file.path = display(&source.join(&old_leaf));
        preview
            .changes
            .push(change(source, "relocateFrom", file.bytes.len()));
        preview
            .changes
            .push(change(target, "relocateTo", file.bytes.len()));
        preview
            .node_map
            .insert(birth.node_id.clone(), birth.node_id);
        let leaf_identity = file.stamp.identity.clone();
        files.push(file);
        let members = if preview.kind == "renameNode"
            || preview.kind == "moveNode"
            || preview.kind == "trashNode"
            || preview.kind == "restoreNode"
        {
            self.capture_relocation_members(
                &mut namespace,
                source,
                &old_leaf,
                &workspace.workspace_id,
            )?
        } else {
            vec![]
        };
        Ok(Mutation::Relocate {
            source: display(source),
            target: display(target),
            source_identity,
            target_parent_identity,
            create_trash_parent,
            old_leaf,
            new_leaf,
            leaf_identity,
            members,
            updates: vec![],
        })
    }
    fn capture_relocation_members(
        &self,
        namespace: &mut NamespaceGuard,
        source: &Path,
        old_leaf: &str,
        workspace: &str,
    ) -> Result<Vec<RelocationMember>> {
        let mut candidates = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for p in self.committed_plans()?.into_iter() {
            for birth in self
                .read_births(&p.operation_id)?
                .into_iter()
                .filter(|b| b.workspace_id == workspace)
            {
                if !seen.insert(birth.node_id.clone()) {
                    continue;
                }
                for path in self.node_placement_paths(&birth)? {
                    if let Some(relative) =
                        crate::history::history_relative_path(Path::new(&path), source)
                    {
                        candidates.push((
                            relative,
                            Some(birth.node_id.clone()),
                            None,
                            birth.identity.clone(),
                        ));
                    }
                }
            }
        }
        for (_, id, identity) in self.directory_admission_candidates(workspace)? {
            for path in self.query_directory_placements(workspace, &id, &identity)? {
                if let Some(relative) =
                    crate::history::history_relative_path(Path::new(&path), source)
                {
                    candidates.push((
                        relative.join("weftext.directory.json"),
                        None,
                        Some(id.clone()),
                        identity.clone(),
                    ));
                }
            }
        }
        let mut members = Vec::<RelocationMember>::new();
        for (relative, node_id, directory_id, expected) in candidates {
            if relative == Path::new(old_leaf) {
                continue;
            }
            if members
                .iter()
                .any(|m| Path::new(&m.relative_path) == relative)
            {
                continue;
            }
            // A moved-out/replaced or inaccessible member does not stop an
            // ordinary whole-tree move. It simply gains no placement proof.
            let Ok(handle) = namespace.open_source_placement(&relative) else {
                continue;
            };
            let actual = stamp(&handle)?.identity;
            let parent = namespace.source_parent_identity(&relative)?;
            if (node_id.is_some() && actual != expected)
                || (directory_id.is_some() && parent != expected)
            {
                continue;
            }
            if members.len() == MAX_OPERATION_FILES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_operations_placement_member_evidence_exceeds_bounded_operation_budget(),
                ));
            }
            members.push(RelocationMember {
                relative_path: display(&relative),
                identity: actual,
                parent,
                node_id,
                directory_id,
            });
        }
        namespace.release_source_descendants();
        Ok(members)
    }
    pub(crate) fn validate_update(&self, root: &Path, path: &Path, after: &[u8]) -> Result<()> {
        if !path.starts_with(root) || path.components().any(|c| c.as_os_str() == ".weftext-trash") {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_operations_updates_require_live_files_inside_library(),
            ));
        }
        let name = path.file_name().unwrap_or_default();
        if path.parent() == Some(root) && name == ".weftext-rules" {
            return crate::validate_library_rules(after);
        }
        if path.parent() == Some(root) && name == ".weftext-settings.json" {
            return crate::author_order::validate_workspace_settings_scope(
                after,
                &self.require_workspace(root)?.workspace_id,
            );
        }
        if name == "weftext.directory.json" {
            return crate::directory_identity::validate_directory_marker(after).map(|_| ());
        }
        if path.parent() == Some(root)
            && [
                ".weftext-schema.json",
                ".weftext-settings.json",
                ".weftext-tags.json",
                ".weftext-rules",
            ]
            .iter()
            .any(|n| name == *n)
        {
            let _: serde_json::Value = serde_json::from_slice(after)?;
            return Ok(());
        }
        let parent = path.parent().unwrap();
        self.check_discovery_boundary(root, parent)?;
        let canonical = self.managed_node(root, parent, false)?;
        if Path::new(&canonical.path) == path {
            let before_id = weftext_engine::inspect(&decode(&canonical.bytes)?).id;
            if weftext_engine::inspect(&decode(after)?).id != before_id {
                return Err(CoreError::new(
                    "identityConflict",
                    UiMessage::error_core_operations_source_update_must_preserve_admitted_node_i_d(
                    ),
                ));
            }
        } else if name == "weftext.annotations.json" || name == "weftext.resources.json" {
            let _: serde_json::Value = serde_json::from_slice(after)?;
        } else {
            let manifest = capture_optional(&self.backend, &parent.join("weftext.resources.json"))?
                .ok_or_else(|| {
                    CoreError::new(
                        "unsupportedRole",
                        UiMessage::error_core_operations_resource_updates_require_existing_owned_resource_manifest(),
                    )
                })?;
            let value: serde_json::Value = serde_json::from_slice(&manifest.bytes)?;
            if value["version"] != 1
                || !value["resources"].as_array().is_some_and(|a| {
                    let records = a
                        .iter()
                        .filter(|r| r["path"].as_str() == name.to_str())
                        .collect::<Vec<_>>();
                    records.len() == 1
                        && records.iter().any(|r| {
                            r["path"].as_str() == name.to_str()
                                && r["resourceId"]
                                    .as_str()
                                    .is_some_and(|id| Uuid::parse_str(id).is_ok())
                        })
                })
            {
                return Err(CoreError::new(
                    "unsupportedRole",
                    UiMessage::error_core_operations_file_not_registered_as_node_owned_resource(),
                ));
            }
            let file = capture_optional(&self.backend, path)?.ok_or_else(|| {
                CoreError::new(
                    "resourceMissing",
                    weftext_messages::UiMessage::error_core_operations_dependency_resource_missing(
                    ),
                )
            })?;
            let owner = self.admitted_binding(&canonical.stamp.identity)?.unwrap();
            let resource_id = value["resources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["path"].as_str() == name.to_str())
                .unwrap()["resourceId"]
                .as_str()
                .unwrap();
            if file.bytes != after
                || !self.admitted_resource(
                    resource_id,
                    &owner.workspace_id,
                    &owner.node_id,
                    &file.stamp.identity,
                )?
            {
                return Err(CoreError::new(
                    "unsupportedRole",
                    UiMessage::error_core_operations_update_files_may_only_freeze_unchanged_admitted_resource_dependency(),
                ));
            }
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)] // One frozen source/schema/tree scope plus its output preview.
    fn plan_copy(
        &self,
        root: &Path,
        source: &Path,
        target: &Path,
        files: &[CapturedFile],
        dirs: &[CapturedDirectory],
        preview: &mut OperationPreview,
        allow_losses: bool,
        schema_config: Option<&CapturedFile>,
        authorized_inventory: Option<(&Inventory, &[CapturedFile])>,
    ) -> Result<Mutation> {
        use weftext_knowledge as knowledge;
        let workspace = self.require_workspace(root)?;
        let schema = if let Some(config) = schema_config {
            weftext_data::Schema::from_author_json(&decode(&config.bytes)?)
        } else {
            weftext_data::Schema::bundled()
        }
        .map_err(|e| CoreError::from_data("schemaUnavailable", e))?;
        let inventory = match authorized_inventory {
            Some((inventory, _)) => inventory.clone(),
            None => self.inventory(root)?,
        };
        if !inventory.complete {
            return Err(CoreError::new(
                "coverageIncomplete",
                UiMessage::error_core_operations_copy_requires_complete_bounded_inventory_observation(),
            ));
        }
        let mut nodes = Vec::new();
        let mut node_map = BTreeMap::new();
        let mut births = Vec::new();
        let mut revisions = BTreeMap::new();
        let mut definitions = BTreeMap::new();
        let mut owners = BTreeMap::new();
        let mut source_targets = BTreeMap::new();
        for item in inventory.entries.iter().filter(|e| e.kind == "node") {
            if item.identity_status != "admitted" {
                if Path::new(&item.path).starts_with(source) {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_core_operations_copy_subtree_has_unadmitted_duplicate_or_damaged_node(),
                    ));
                }
                continue;
            }
            let id = item.node_id.clone().unwrap();
            let owner = knowledge::NodeRef {
                workspace_id: workspace.workspace_id.clone(),
                node_id: id.clone(),
            };
            let observed = if let Some((_, context)) = authorized_inventory {
                context
                    .iter()
                    .find(|f| {
                        crate::history::history_relative_path(
                            Path::new(&f.path),
                            Path::new(&item.path),
                        )
                        .is_some_and(|p| p.as_os_str().is_empty())
                    })
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::new(
                            "scopeChanged",
                            UiMessage::error_core_operations_original_authorized_transfer_source_absent(),
                        )
                    })?
            } else if let Some(f) = files.iter().find(|f| f.path == item.path) {
                f.clone()
            } else {
                capture_optional(&self.backend, Path::new(&item.path))?.ok_or_else(|| {
                    CoreError::new(
                        "scopeChanged",
                        weftext_messages::UiMessage::error_core_operations_link_target_disappeared(
                        ),
                    )
                })?
            };
            let source_text = decode(&observed.bytes)?;
            let info = weftext_engine::inspect_metadata(&source_text);
            let revision = self.accepted_revision(&observed)?.unwrap_or_default();
            nodes.push(knowledge::NodeSnapshot {
                owner: owner.clone(),
                path: file_url(Path::new(&item.path))?,
                library_path: Path::new(&item.path)
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                revision_id: revision,
                source: source_text.clone(),
                title: info.title,
                aliases: vec![],
                labels: vec![],
                labels_complete: false,
            });
            if Path::new(&item.path).starts_with(source) {
                let new = knowledge::NodeRef {
                    workspace_id: workspace.workspace_id.clone(),
                    node_id: fresh(),
                };
                let after_path = copy_path(source, target, Path::new(&item.path))?;
                preview.node_map.insert(id.clone(), new.node_id.clone());
                node_map.insert(owner.clone(), new.clone());
                let revision = fresh();
                preview
                    .revisions
                    .insert(display(&after_path), revision.clone());
                revisions.insert(owner.clone(), revision);
                births.push(NewBirth {
                    path: display(&after_path),
                    node_id: new.node_id,
                });
                owners.insert(item.path.clone(), owner.clone());
                source_targets.insert(item.path.clone(), display(&after_path));
                for slot in knowledge::definition_slots(&source_text, &owner)
                    .map_err(|e| CoreError::from_knowledge(&e.code.clone(), e))?
                {
                    if !self.is_definition_key_admitted(root, &owner.node_id, &slot.key)? {
                        return Err(CoreError::new(
                            "definitionUnadmitted",
                            UiMessage::error_core_operations_authored_definition_key_has_no_committed_core_allocation_fact(),
                        ));
                    }
                    definitions.insert(slot.identity, fresh());
                }
            }
        }
        let mut resource_map = BTreeMap::new();
        let mut resource_revisions = BTreeMap::new();
        let mut resources = Vec::new();
        let mut manifests = BTreeMap::new();
        let mut annotations = BTreeMap::new();
        for file in files {
            let path = Path::new(&file.path);
            if path
                .file_name()
                .is_some_and(|n| n == "weftext.directory.json")
            {
                manifests.insert(
                    file.path.clone(),
                    self.copy_directory_marker(root, path, &file.bytes)?,
                );
            } else if path
                .file_name()
                .is_some_and(|n| n == "weftext.resources.json")
            {
                let canonical = path
                    .parent()
                    .unwrap()
                    .join(format!("{}.typ", node_name(path.parent().unwrap())?));
                let owner = owners.get(&display(&canonical)).ok_or_else(|| {
                    CoreError::new(
                        "resourceOwner",
                        UiMessage::error_core_operations_resource_manifest_not_inside_copied_admitted_node(),
                    )
                })?;
                let mut value: serde_json::Value = serde_json::from_slice(&file.bytes)?;
                if value["version"] != 1 || !value["resources"].is_array() {
                    return Err(CoreError::new(
                        "resourceManifest",
                        UiMessage::error_core_operations_unsupported_resource_manifest_format(),
                    ));
                }
                for record in value["resources"].as_array_mut().unwrap() {
                    let old_id = record["resourceId"]
                        .as_str()
                        .ok_or_else(|| {
                            CoreError::new(
                                "resourceManifest",
                                UiMessage::error_core_operations_missing_resource_i_d(),
                            )
                        })?
                        .to_string();
                    valid_operation_id(&old_id)?;
                    let name = record["path"].as_str().ok_or_else(|| {
                        CoreError::new(
                            "resourceManifest",
                            UiMessage::error_core_operations_missing_resource_path(),
                        )
                    })?;
                    let resource_path = resource_path(path.parent().unwrap(), name)?;
                    let resource = files
                        .iter()
                        .find(|f| f.path == display(&resource_path))
                        .ok_or_else(|| {
                            CoreError::new(
                                "resourceMissing",
                                UiMessage::error_core_operations_copy_snapshot_omits_registered_resource(),
                            )
                        })?;
                    if !self.admitted_resource(
                        &old_id,
                        &workspace.workspace_id,
                        &owner.node_id,
                        &resource.stamp.identity,
                    )? {
                        return Err(CoreError::new(
                            "resourceUnadmitted",
                            UiMessage::error_core_operations_manifest_u_u_i_d_alone_does_not_establish_resource_identity(),
                        ));
                    }
                    let new_id = fresh();
                    let old_ref = format!("weftext-resource://{}/{old_id}", workspace.workspace_id);
                    let new_ref = format!("weftext-resource://{}/{new_id}", workspace.workspace_id);
                    if resource_map
                        .insert(old_ref.clone(), new_ref.clone())
                        .is_some()
                    {
                        return Err(CoreError::new(
                            "duplicateResource",
                            UiMessage::error_core_operations_duplicate_resource_i_ds_in_copy_snapshot(),
                        ));
                    }
                    let after_path = copy_path(source, target, &resource_path)?;
                    let revision = fresh();
                    preview
                        .revisions
                        .insert(display(&after_path), revision.clone());
                    resource_revisions.insert(old_ref.clone(), revision);
                    preview.resource_map.insert(display(&after_path), new_ref);
                    resources.push(knowledge::ResourceSnapshot {
                        key: old_ref,
                        path: file_url(&resource_path)?,
                        revision_id: self.accepted_revision(resource)?.unwrap_or_default(),
                    });
                    record["resourceId"] = new_id.into();
                }
                manifests.insert(file.path.clone(), serde_json::to_vec_pretty(&value)?);
            } else if path
                .file_name()
                .is_some_and(|n| n == "weftext.annotations.json")
            {
                let doc = knowledge::decode_annotations(&file.bytes)
                    .map_err(|e| CoreError::from_knowledge(&e.code.clone(), e))?;
                for record in doc.records {
                    if annotations.insert(record.annotation_id, fresh()).is_some() {
                        return Err(CoreError::new(
                            "duplicateAnnotation",
                            UiMessage::error_core_operations_duplicate_annotation_i_ds_in_copy_snapshot(),
                        ));
                    }
                }
            }
        }
        let mut transfer = Vec::new();
        for file in files {
            let owner = owners.get(&file.path).cloned();
            let kind = if owner.is_some() {
                knowledge::CopyFileKind::Source
            } else if Path::new(&file.path)
                .file_name()
                .is_some_and(|n| n == "weftext.annotations.json")
            {
                knowledge::CopyFileKind::Annotations
            } else {
                knowledge::CopyFileKind::Resource
            };
            let mut typed_refs = vec![];
            let mut definition_references = vec![];
            let mut complete = true;
            if let Some(owner) = &owner {
                match weftext_data::typed_ref_closure(&decode(&file.bytes)?, &schema) {
                    Ok(closure) => {
                        complete = closure.complete;
                        typed_refs = closure
                            .slots
                            .into_iter()
                            .map(|s| knowledge::TypedRefSlot {
                                from: s.from,
                                to: s.to,
                                target: s.target,
                                kind: s.kind,
                            })
                            .collect();
                    }
                    Err(_) => complete = false,
                }
                match knowledge::saved_query_reference_slots(&decode(&file.bytes)?, &schema, owner)
                {
                    Ok(slots) => {
                        typed_refs.extend(slots.typed_refs);
                        definition_references = slots.definition_references;
                    }
                    Err(_) => complete = false,
                }
                match weftext_engine::citation_work_slots(&decode(&file.bytes)?) {
                    Ok(slots) => {
                        typed_refs.extend(slots.into_iter().map(|slot| knowledge::TypedRefSlot {
                            from: slot.range.from,
                            to: slot.range.to,
                            target: slot.work,
                            kind: "node_ref".into(),
                        }))
                    }
                    Err(_) => complete = false,
                }
            }
            transfer.push(knowledge::CopyFile {
                source_path: file_url(Path::new(&file.path))?,
                target_path: display(&copy_path(source, target, Path::new(&file.path))?),
                before: file.bytes.clone(),
                kind,
                owner,
                typed_refs,
                definition_references,
                typed_refs_complete: complete,
            });
        }
        let plan = knowledge::plan_copy(knowledge::CopyInput {
            files: transfer,
            node_map,
            resource_map,
            tag_map: BTreeMap::new(),
            schema_map: BTreeMap::new(),
            annotation_map: annotations,
            definition_map: definitions,
            revision_map: revisions,
            resource_revision_map: resource_revisions,
            inventory: knowledge::InventorySnapshot {
                generation: preview.plan_id.clone(),
                coverage: knowledge::Coverage::Complete,
                nodes,
                resources,
            },
            allow_losses,
        })
        .map_err(|e| CoreError::from_knowledge(&e.code.clone(), e))?;
        preview.losses = plan.losses;
        let mut output = plan
            .files
            .into_iter()
            .map(|f| FileChange {
                path: f.path,
                base_id: None,
                before: None,
                after: f.after,
            })
            .collect::<Vec<_>>();
        for (original, after) in manifests {
            let target_path = display(&copy_path(source, target, Path::new(&original))?);
            output
                .iter_mut()
                .find(|f| f.path == target_path)
                .unwrap()
                .after = after;
        }
        let mut directories = dirs
            .iter()
            .filter_map(|d| Path::new(&d.path).strip_prefix(source).ok())
            .filter(|p| !p.as_os_str().is_empty())
            .map(display)
            .collect::<Vec<_>>();
        if !plan.preserved_originals.is_empty() {
            directories.push(".weftext-copy-originals".into());
            for original in plan.preserved_originals {
                let original_path = url::Url::parse(&original.source_path)
                    .ok()
                    .and_then(|u| u.to_file_path().ok())
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidPlan",
                            UiMessage::error_core_operations_invalid_preserved_source_u_r_l(),
                        )
                    })?;
                let name = hash(display(&original_path).as_bytes());
                output.push(FileChange {
                    path: display(&target.join(".weftext-copy-originals").join(name)),
                    base_id: None,
                    before: None,
                    after: original.bytes,
                });
            }
        }
        preview.changes.extend(
            output
                .iter()
                .map(|f| change(Path::new(&f.path), "copy", f.after.len())),
        );
        Ok(Mutation::CreateTree {
            existing_target: false,
            target: display(target),
            directories,
            files: output,
            births,
            updates: vec![],
        })
    }
}
pub(crate) fn intent_kind(intent: &OperationIntent) -> &'static str {
    match intent {
        OperationIntent::CreateLibrary { .. } => "createLibrary",
        OperationIntent::AdoptSyncExternal { .. } => "adoptSyncExternal",
        OperationIntent::CloneSync { .. } => "cloneSync",
        OperationIntent::ApplySync { .. } => "applySync",
        OperationIntent::MergeSync { .. } => "mergeSync",
        OperationIntent::RevalidateSync { .. } => "revalidateSync",
        OperationIntent::RenameNode { .. } => "renameNode",
        OperationIntent::MoveNode { .. } => "moveNode",
        OperationIntent::RelocateDirectory { .. } => "relocateDirectory",
        OperationIntent::CopyNode { .. } => "copyNode",
        OperationIntent::TemplateNode { .. } => "templateNode",
        OperationIntent::TrashNode { .. } => "trashNode",
        OperationIntent::RestoreNode { .. } => "restoreNode",
        OperationIntent::NewNode { .. } => "newNode",
        OperationIntent::UpdateFiles { .. } => "updateFiles",
        OperationIntent::BackupLibrary { .. } => "backupLibrary",
        OperationIntent::ConfigureStyle { .. } => "configureStyle",
        OperationIntent::ImportResource { .. } => "importResource",
        OperationIntent::ImportNative { .. } => "importNative",
        OperationIntent::ImportRecognized { .. } => "importRecognized",
        OperationIntent::ImportBibliography { .. } => "importBibliography",
        OperationIntent::ImportDocx { .. } | OperationIntent::ImportDocxWithOptions { .. } => {
            "importDocx"
        }
    }
}
pub(crate) fn canonical_json_bytes(value: &serde_json::Value) -> Result<Vec<u8>> {
    fn sorted(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => {
                let mut keys = object.keys().collect::<Vec<_>>();
                keys.sort();
                let mut result = serde_json::Map::new();
                for key in keys {
                    result.insert(key.clone(), sorted(&object[key]));
                }
                serde_json::Value::Object(result)
            }
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(sorted).collect())
            }
            other => other.clone(),
        }
    }
    Ok(serde_json::to_vec(&sorted(value))?)
}
fn value_enabled(bytes: &Option<Vec<u8>>) -> Result<bool> {
    Ok(bytes
        .as_ref()
        .map(|b| serde_json::from_slice::<serde_json::Value>(b))
        .transpose()?
        .and_then(|v| v["defaultStyle"]["enabled"].as_bool())
        .unwrap_or(false))
}
fn default_style_source() -> &'static str {
    weftext_engine::DEFAULT_LIBRARY_STYLE
}
pub(crate) fn resource_path(directory: &Path, name: &str) -> Result<PathBuf> {
    let relative = PathBuf::from(name);
    if relative.is_absolute()
        || relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(CoreError::new(
            "resourcePath",
            UiMessage::error_core_operations_resources_require_canonical_relative_file_path(),
        ));
    }
    let path = absolute_path(&directory.join(relative))?;
    if is_canonical(&path) {
        return Err(CoreError::new(
            "resourcePath",
            UiMessage::error_core_operations_canonical_node_source_cannot_registered_as_resource(),
        ));
    }
    Ok(path)
}
pub(crate) fn file_url(path: &Path) -> Result<String> {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .map_err(|_| {
            CoreError::new(
                "invalidPath",
                UiMessage::error_core_operations_cannot_form_canonical_local_file_u_r_l(),
            )
        })
}
fn copy_path(source: &Path, target: &Path, path: &Path) -> Result<PathBuf> {
    let relative = path.strip_prefix(source).map_err(|_| {
        CoreError::new(
            "invalidPlan",
            UiMessage::error_core_operations_copy_member_escaped_selected_source(),
        )
    })?;
    if relative == Path::new(&format!("{}.typ", node_name(source)?)) {
        Ok(target.join(format!("{}.typ", node_name(target)?)))
    } else {
        Ok(target.join(relative))
    }
}
pub(crate) fn read_blob(file: &mut backend::BoundFile) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.take(MAX_OPERATION_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_OPERATION_BYTES {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_operations_file_exceeds64_mi_b(),
        ));
    }
    Ok(bytes)
}
pub(crate) fn write_blob(file: &mut backend::BoundFile, bytes: &[u8]) -> Result<()> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(bytes)?;
    file.set_len(bytes.len() as u64)?;
    file.sync_all()?;
    if read_blob(file)? != bytes {
        return Err(CoreError::new(
            "verificationFailed",
            UiMessage::error_core_operations_same_handle_blob_readback_differs_original_operation_retained(),
        ));
    }
    Ok(())
}
pub(crate) fn change(path: &Path, action: &str, bytes: usize) -> OperationChange {
    OperationChange {
        path: display(path),
        action: action.into(),
        bytes,
    }
}
pub(crate) fn node_name(directory: &Path) -> Result<String> {
    let name = directory
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            CoreError::new(
                "invalidName",
                UiMessage::error_core_operations_node_name_must_u_t_f8(),
            )
        })?;
    valid_name(name)?;
    Ok(name.into())
}
fn qualify_relocation_target(
    backend: &crate::BackendContext,
    path: &Path,
    occupant: Option<&FileIdentity>,
) -> Result<()> {
    if let Some(expected) = occupant {
        let guard = ParentGuard::acquire(backend, &path.join(".original-participant-proof"))?;
        if guard.directory_identity()? != *expected {
            return Err(CoreError::new(
                "conflict",
                UiMessage::error_core_operations_destination_not_original_moving_participant(),
            ));
        }
        Ok(())
    } else {
        ensure_absent(backend, path)
    }
}
pub(crate) fn ensure_absent(backend: &crate::BackendContext, path: &Path) -> Result<()> {
    if backend.entry_exists(path)? {
        Err(CoreError::new(
            "exists",
            weftext_messages::UiMessage::error_core_operations_create_only_target_occupied_no_overwrite_allowed(),
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn capture_optional(
    backend: &crate::BackendContext,
    path: &Path,
) -> Result<Option<CapturedFile>> {
    let guard = ParentGuard::acquire(backend, path)?;
    match guard.open_file(false, false) {
        Ok(mut file) => {
            let bytes = read_blob(&mut file)?;
            Ok(Some(CapturedFile {
                path: display(path),
                bytes,
                stamp: stamp(&file)?,
            }))
        }
        Err(e) if e.code == "missing" => Ok(None),
        Err(e) => Err(e),
    }
}
fn locator_within(path: &Path, root: &Path) -> bool {
    let path = display(path).replace('/', "\\").to_lowercase();
    let root = display(root)
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase();
    path == root || path.starts_with(&(root + "\\"))
}
fn locator_equal(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .eq_ignore_ascii_case(&b.replace('/', "\\"))
}
/// Read-only root controls remain write/delete protected while Core may read
/// their metadata/rules again inside the same decision. Author leaves retain
/// their original share-zero writer/read behavior.
pub(crate) fn operation_read_control(root: &Path, path: &Path) -> bool {
    path.parent().is_some_and(|parent| {
        crate::history::history_relative_path(parent, root)
            .is_some_and(|p| p.as_os_str().is_empty())
    }) && path.file_name().is_some_and(|name| {
        matches!(
            name.to_str(),
            Some(
                ".weftext-workspace.json"
                    | ".weftext-format"
                    | ".weftext-mvp-commit.json"
                    | ".weftext-rules"
            )
        )
    })
}
pub(crate) fn sorted_entries(
    entries: Vec<(std::ffi::OsString, backend::EntryMetadata)>,
    _: &Path,
) -> Vec<String> {
    let mut names = entries
        .into_iter()
        .map(|(name, attrs)| format!("{}:{attrs}", name.to_string_lossy()))
        .collect::<Vec<_>>();
    names.sort();
    names
}
fn validate_relocation_membership(
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
    after: bool,
    include_creations: bool,
) -> Result<()> {
    let Mutation::Relocate {
        source,
        target,
        old_leaf,
        new_leaf,
        ..
    } = &t.mutation
    else {
        unreachable!()
    };
    let target = Path::new(target);
    for directory in &t.directories {
        let relative = Path::new(&directory.path)
            .strip_prefix(source)
            .map_err(|_| {
                CoreError::new(
                    "invalidPlan",
                    UiMessage::error_core_operations_directory_outside_relocation(),
                )
            })?;
        let (identity, mut entries, complete) =
            namespace.source_directory_snapshot(relative, MAX_OPERATION_FILES)?;
        let mut expected = directory.entries.clone();
        if after && relative.as_os_str().is_empty() && old_leaf != new_leaf {
            for entry in &mut expected {
                if let Some(attrs) = entry.strip_prefix(&format!("{old_leaf}:")) {
                    *entry = format!("{new_leaf}:{attrs}");
                }
            }
            expected.sort();
        }
        if include_creations
            && after
            && let Mutation::Relocate { updates, .. } = &t.mutation
        {
            let final_directory = if relative.as_os_str().is_empty() {
                target.to_path_buf()
            } else {
                target.join(relative)
            };
            for change in updates.iter().filter(|c| {
                c.before.is_none() && Path::new(&c.path).parent() == Some(final_directory.as_path())
            }) {
                let name = Path::new(&change.path).file_name().unwrap();
                if !entries.iter().any(|(actual, _)| actual == name) {
                    return Err(CoreError::new(
                        "scopeChanged",
                        UiMessage::error_core_operations_newly_created_relocation_participant_missing(),
                    ));
                }
                entries.retain(|(actual, _)| actual != name);
            }
        }
        if !complete
            || identity != directory.identity
            || sorted_entries(entries, Path::new(&directory.path)) != expected
        {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_operations_moved_directory_membership_or_identity_differs_from_frozen_tree(),
            ));
        }
    }
    Ok(())
}
fn validate_relocation_members(
    namespace: &mut NamespaceGuard,
    members: &[RelocationMember],
) -> Result<Vec<backend::BoundFile>> {
    let mut held = Vec::new();
    for member in members {
        let relative = Path::new(&member.relative_path);
        let handle = namespace.open_source_placement(relative)?;
        if stamp(&handle)?.identity != member.identity
            || namespace.source_parent_identity(relative)? != member.parent
        {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_operations_placement_only_member_object_or_parent_changed_source_bytes_not_move_precondition(),
            ));
        }
        held.push(handle);
    }
    Ok(held)
}
fn validate_relocation_creations(
    backend: &crate::BackendContext,
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
) -> Result<()> {
    let Mutation::Relocate {
        target,
        old_leaf,
        new_leaf,
        updates,
        ..
    } = &t.mutation
    else {
        unreachable!()
    };
    if old_leaf != new_leaf && namespace.source_entry_exists(Path::new(new_leaf))? {
        return Err(CoreError::new(
            "exists",
            UiMessage::error_core_operations_renamed_canonical_filename_already_occupied(),
        ));
    }
    for change in updates.iter().filter(|c| c.before.is_none()) {
        if let Ok(relative) = Path::new(&change.path).strip_prefix(target) {
            match namespace.open_source_descendant(relative) {
                Err(e) if e.code == "missing" => {}
                Err(e) => return Err(e),
                Ok(_) => {
                    return Err(CoreError::new(
                        "exists",
                        UiMessage::error_core_operations_incoming_created_participant_already_occupied(),
                    ));
                }
            }
        } else {
            ensure_absent(backend, Path::new(&change.path))?;
        }
    }
    Ok(())
}
fn validate_relocation_tree(
    namespace: &mut NamespaceGuard,
    t: &FrozenOperation,
    after: bool,
) -> Result<Vec<backend::BoundFile>> {
    let Mutation::Relocate {
        source,
        old_leaf,
        new_leaf,
        ..
    } = &t.mutation
    else {
        unreachable!()
    };
    validate_relocation_membership(namespace, t, after, false)?;
    let mut held = Vec::new();
    for file in t
        .files
        .iter()
        .filter(|f| Path::new(&f.path).starts_with(source))
    {
        let relative = Path::new(&file.path).strip_prefix(source).map_err(|_| {
            CoreError::new(
                "invalidPlan",
                UiMessage::error_core_operations_file_outside_relocation(),
            )
        })?;
        let relative = if after && relative == Path::new(old_leaf) {
            Path::new(new_leaf)
        } else {
            relative
        };
        let write = if let Mutation::Relocate { updates, .. } = &t.mutation {
            let final_path = t.relocated_path(&file.path);
            updates
                .iter()
                .any(|c| c.path == final_path && c.before.as_ref() != Some(&c.after))
        } else {
            false
        };
        let mut handle = namespace.open_source_output(relative, write, false)?;
        if stamp(&handle)? != file.stamp || read_blob(&mut handle)? != file.bytes {
            return Err(CoreError::new(
                "conflict",
                UiMessage::error_core_operations_moved_file_no_longer_matches_frozen_identity_and_bytes(),
            ));
        }
        held.push(handle);
    }
    Ok(held)
}
pub(crate) fn snapshot_tree(
    backend: &crate::BackendContext,
    root: &Path,
    files: &mut Vec<CapturedFile>,
    dirs: &mut Vec<CapturedDirectory>,
    skip_instance_lock: bool,
) -> Result<()> {
    let mut queue = vec![root.to_path_buf()];
    let mut total = files.iter().map(|f| f.bytes.len()).sum::<usize>();
    while let Some(path) = queue.pop() {
        if dirs.len() >= MAX_OPERATION_FILES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_operations_snapshot_exceeds5000_directories_no_partial_success(),
            ));
        }
        let guard = ParentGuard::acquire(backend, &path.join(".snapshot-prospective"))?;
        let (entries, complete) = guard.directory_entries(MAX_OPERATION_FILES)?;
        if !complete {
            return Err(CoreError::new(
                "coverageIncomplete",
                UiMessage::error_core_operations_directory_exceeds_snapshot_entry_budget(),
            ));
        }
        dirs.push(CapturedDirectory {
            path: display(&path),
            identity: guard.directory_identity()?,
            entries: sorted_entries(entries.clone(), &path),
        });
        for (name, attrs) in entries {
            if skip_instance_lock && path == root && name == "instance.lock" {
                continue;
            }
            if attrs.is_unsupported() {
                return Err(CoreError::new(
                    "unsupportedPath",
                    UiMessage::error_core_operations_snapshot_member_reparse_offline_or_cloud_placeholder(),
                ));
            }
            let child = path.join(name);
            if attrs.is_directory() {
                if dirs.len() + queue.len() >= MAX_OPERATION_FILES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_captured_and_queued_snapshot_directories_exceed5000(),
                    ));
                }
                queue.push(child);
            } else {
                let captured = capture_optional(backend, &child)?
                    .ok_or_else(|| CoreError::new("scopeChanged", weftext_messages::UiMessage::error_core_operations_snapshot_file_disappeared()))?;
                total += captured.bytes.len();
                files.push(captured);
                if total > MAX_OPERATION_BYTES || files.len() > MAX_OPERATION_FILES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_operations_snapshot_exceeds64_mi_b_or5000_files_no_partial_success(),
                    ));
                }
            }
        }
    }
    Ok(())
}
fn initialize_source(source: &str, node_id: &str) -> Result<String> {
    let info = weftext_engine::inspect_metadata(source);
    if let Some(metadata) = info.metadata {
        let Some(id) = metadata.fields.iter().find(|f| f.key == "id") else {
            use typst_syntax::{
                LinkedNode, Source,
                ast::{self, AstNode},
            };
            let parsed = Source::detached(source);
            let root = LinkedNode::new(parsed.root());
            let call = root
                .children()
                .find(|n| {
                    n.range().start == metadata.from && n.get().cast::<ast::FuncCall>().is_some()
                })
                .ok_or_else(|| {
                    CoreError::new(
                        "templateIdentity",
                        UiMessage::error_core_operations_exact_metadata_call_unavailable(),
                    )
                })?;
            let call_ast = call.get().cast::<ast::FuncCall>().unwrap();
            let dict = call_ast
                .args()
                .items()
                .find_map(|arg| {
                    if let ast::Arg::Pos(e) = arg {
                        e.to_untyped().cast::<ast::Dict>()
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    CoreError::new(
                        "templateIdentity",
                        UiMessage::error_core_operations_exact_metadata_dictionary_unavailable(),
                    )
                })?;
            let node = parsed.find(dict.to_untyped().span()).ok_or_else(|| {
                CoreError::new(
                    "templateIdentity",
                    UiMessage::error_core_operations_exact_dictionary_span_unavailable(),
                )
            })?;
            let range = node.range();
            let mut after = source.to_string();
            if source[range.start + 1..range.end - 1].trim() == ":" {
                after.replace_range(range, &format!("(id: \"{node_id}\")"));
            } else {
                after.insert_str(range.start + 1, &format!("id: \"{node_id}\", "));
            }
            if weftext_engine::inspect_metadata(&after).id.as_deref() != Some(node_id) {
                return Err(CoreError::new(
                    "templateIdentity",
                    UiMessage::error_core_operations_new_metadata_id_did_not_reparse(),
                ));
            }
            return Ok(after);
        };
        if !id.writable
            || id
                .value
                .as_ref()
                .and_then(serde_json::Value::as_str)
                .is_none()
        {
            return Err(CoreError::new(
                "templateIdentity",
                UiMessage::error_core_operations_computed_template_identity_cannot_rewritten_safely(
                ),
            ));
        }
        let mut after = source.to_string();
        after.replace_range(id.from..id.to, &format!("\"{node_id}\""));
        if weftext_engine::inspect_metadata(&after).id.as_deref() != Some(node_id) {
            return Err(CoreError::new(
                "templateIdentity",
                UiMessage::error_core_operations_rewritten_declaration_not_one_exact_new_identity(),
            ));
        }
        Ok(after)
    } else if info.identity_status == "absent" {
        Ok(format!(
            "#metadata((id: \"{node_id}\", types: ())) <weftext>\n\n{source}"
        ))
    } else {
        Err(CoreError::new(
            "templateIdentity",
            UiMessage::error_core_operations_unknown_or_ambiguous_template_identity_preserved_and_cannot_initialized(),
        ))
    }
}

#[cfg(all(test, windows))]
mod typed_snapshot_delivery_tests {
    use super::*;

    #[test]
    fn missing_admitted_source_has_typed_exact_identity_and_retains_original_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let note = core
            .create_note_fixture(&root, &root, "Exact author title 😀")
            .unwrap();
        let owner = note.node_id.clone().unwrap();
        let original = fs::read(&note.path).unwrap();
        let retained = temp
            .path()
            .join("original-source-outside-selected-library.typ");
        fs::rename(&note.path, &retained).unwrap();
        let cut = core.admitted_snapshot(&root).unwrap();
        assert!(!cut.complete);
        assert!(!cut.membership_ids.contains(&owner));
        let warning = cut
            .warnings
            .iter()
            .find(|warning| {
                warning.explanation
                    == UiMessage::error_core_operations_admitted_node_unavailable(
                        weftext_messages::params::ErrorCoreOperationsAdmittedNodeUnavailable {
                            node_id: owner.clone(),
                        },
                    )
            })
            .expect("the exact missing admitted identity is disclosed");
        assert!(warning.diagnostics.is_empty());
        assert!(warning.causes.as_slice().is_empty());
        assert!(warning.details.is_none());
        let wire = serde_json::to_value(warning).unwrap();
        assert_eq!(wire["explanation"]["params"]["nodeId"], owner);
        assert_eq!(wire["diagnostics"], serde_json::json!([]));
        assert_eq!(wire["causes"], serde_json::json!([]));
        assert!(wire.get("message").is_none());
        assert_eq!(fs::read(&retained).unwrap(), original);
        core.release_admitted_cut(&cut.cut_id);
    }

    #[test]
    fn transaction_receipt_contains_only_original_facts_and_ignores_historical_extra_text() {
        let original = br#"{"operationId":"original-operation","planId":"original-plan","status":"committed","path":"exact/original.typ","nodeIds":["original-node"],"message":"historical text is not new delivery authority"}"#;
        let parsed: OperationReceipt = serde_json::from_slice(original).unwrap();
        assert_eq!(parsed.operation_id, "original-operation");
        assert_eq!(parsed.plan_id, "original-plan");
        assert_eq!(parsed.status, "committed");
        assert_eq!(parsed.path, "exact/original.typ");
        assert_eq!(parsed.node_ids, vec!["original-node"]);
        let wire = serde_json::to_value(parsed).unwrap();
        assert_eq!(wire.as_object().unwrap().len(), 5);
        assert!(wire.get("message").is_none());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(original).unwrap()["message"],
            "historical text is not new delivery authority"
        );
    }
}
