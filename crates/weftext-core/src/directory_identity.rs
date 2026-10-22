//! Navigation identities are admitted by the original operation journal. A
//! portable UUID claim or a remembered pathname cannot establish continuity.
use super::*;
use std::collections::BTreeSet;

const DIRECTORY_MARKER: &str = "weftext.directory.json";
const TAG_REGISTRY: &str = ".weftext-tags.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationTarget {
    pub target_ref: Option<String>,
    pub path: Option<String>,
    pub kind: String,
    pub status: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationPreparation {
    pub preview: OperationPreview,
    pub target_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TagAssignment {
    pub path: String,
    pub base_id: String,
    pub field_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRecord {
    pub tag_ref: String,
    pub tag_id: String,
    pub display_name: String,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRegistrySnapshot {
    pub tags: Vec<TagRecord>,
    pub base_id: String,
    pub complete: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DirectoryMarker {
    directory_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DirectoryReservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library: Option<hosted::ActorLibraryBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    root: String,
    path: String,
    workspace_id: String,
    directory_id: String,
    identity: FileIdentity,
    fingerprint: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TagReservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library: Option<hosted::ActorLibraryBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    root: String,
    workspace_id: String,
    tag_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    additional_tag_ids: Vec<String>,
    registry_identity: Option<FileIdentity>,
    fingerprint: String,
}

use weftext_data::{TagDefinition, TagDocument};

struct TagWriteBasis {
    document: TagDocument,
    bytes: Option<Vec<u8>>,
    identity: Option<FileIdentity>,
}

fn tag_update_files(t: &operations::FrozenOperation) -> &[FileChange] {
    match &t.mutation {
        operations::Mutation::Update { files } => files,
        operations::Mutation::CreateTree { updates, .. }
        | operations::Mutation::Relocate { updates, .. }
        | operations::Mutation::Compound { updates, .. } => updates,
    }
}

pub(crate) fn validate_directory_marker(bytes: &[u8]) -> Result<()> {
    let marker: DirectoryMarker = serde_json::from_slice(bytes)?;
    valid_operation_id(&marker.directory_id)
}

fn tag_document(bytes: Option<&[u8]>) -> Result<TagDocument> {
    let Some(bytes) = bytes else {
        return Ok(TagDocument {
            version: 2,
            tags: vec![],
        });
    };
    // Historical v1 blobs remain immutable. Only the explicit Core write path
    // converts their in-memory all-root interpretation to current v2 bytes.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct LegacyTag {
        tag_id: String,
        display_name: String,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LegacyDocument {
        version: u32,
        tags: Vec<LegacyTag>,
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let doc = if value["version"] == 1 {
        let old: LegacyDocument = serde_json::from_slice(bytes)?;
        debug_assert_eq!(old.version, 1);
        TagDocument {
            version: 2,
            tags: old
                .tags
                .into_iter()
                .map(|tag| TagDefinition {
                    tag_id: tag.tag_id,
                    display_name: tag.display_name,
                    parent_tag_id: None,
                })
                .collect(),
        }
    } else {
        serde_json::from_slice(bytes)?
    };
    doc.forest()
        .map_err(|e| CoreError::new("tagRegistry", e.to_string()))?;
    Ok(doc)
}

pub(crate) fn validate_sync_tags(bytes: &[u8]) -> Result<()> {
    let doc: TagDocument = serde_json::from_slice(bytes)?;
    doc.forest()
        .map(|_| ())
        .map_err(|e| CoreError::new("tagRegistry", e.to_string()))
}
pub(crate) fn decode_historical_tag_document(bytes: &[u8]) -> Result<TagDocument> {
    tag_document(Some(bytes))
}

fn validate_tag_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().count() > 256 || name.contains('\0') {
        return Err(CoreError::new(
            "invalidTagName",
            "Tag display name must contain 1 to 256 characters",
        ));
    }
    Ok(())
}

fn target(
    kind: &str,
    status: &str,
    reference: Option<String>,
    path: Option<String>,
    label: String,
) -> NavigationTarget {
    NavigationTarget {
        target_ref: reference,
        path,
        kind: kind.into(),
        status: status.into(),
        label,
        display_name: None,
    }
}

fn entity_ref(kind: &str, workspace: &str, id: &str) -> String {
    format!("weftext-{kind}://{workspace}/{id}")
}

fn same_path(locator: &str, path: &Path) -> bool {
    #[cfg(windows)]
    {
        locator
            .replace('/', "\\")
            .eq_ignore_ascii_case(&display(path).replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        Path::new(locator) == path
    }
}
fn is_navigation_marker_output(
    plan: &journal::Plan,
    transaction: &operations::FrozenOperation,
    file: &operations::FileChange,
) -> bool {
    let eligible = Path::new(&file.path)
        .file_name()
        .is_some_and(|name| name == DIRECTORY_MARKER)
        && !transaction.preview.resource_map.keys().any(|path| {
            crate::history::history_relative_path(Path::new(path), Path::new(&file.path))
                .is_some_and(|relative| relative.as_os_str().is_empty())
        });
    if !eligible {
        return false;
    }
    let parent = Path::new(&file.path).parent().unwrap();
    // A canonical Node birth already assigns this container its role. An
    // unknown marker-looking member cannot assign another navigation identity.
    if transaction
        .births()
        .iter()
        .any(|birth| Path::new(&birth.path).parent() == Some(parent))
    {
        return false;
    }
    let typed_role = |object: &sync::replica::LocalObject| {
        matches!(&object.object,sync::ObjectKey::Directory(id)|sync::ObjectKey::RootDirectory(id)
            if serde_json::from_slice::<DirectoryMarker>(&file.after).is_ok_and(|marker|marker.directory_id==*id))
            && Path::new(&object.directory) == parent
            && object.state.files.get(DIRECTORY_MARKER) == Some(&hash(&file.after))
    };
    if let Some(application) = &plan.sync_application {
        return application.after.iter().any(typed_role);
    }
    if let Some(setup) = &plan.sync_setup {
        return setup.initial.iter().any(typed_role);
    }
    true
}

pub(crate) fn parse_ref(reference: &str) -> Result<(&str, &str, &str)> {
    let (prefix, rest) = reference.split_once("://").ok_or_else(|| {
        CoreError::new(
            "invalidReference",
            "A typed navigation reference is required",
        )
    })?;
    let kind = match prefix {
        "weftext-node" => "node",
        "weftext-directory" => "directory",
        "weftext-tag" => "tag",
        _ => {
            return Err(CoreError::new(
                "invalidReference",
                "Unsupported navigation reference kind",
            ));
        }
    };
    let (workspace, id) = rest.split_once('/').ok_or_else(|| {
        CoreError::new(
            "invalidReference",
            "Navigation reference needs WorkspaceId and object ID",
        )
    })?;
    if id.contains('/') || Uuid::parse_str(workspace).is_err() || Uuid::parse_str(id).is_err() {
        return Err(CoreError::new(
            "invalidReference",
            "Navigation reference IDs must be UUIDs",
        ));
    }
    Ok((kind, workspace, id))
}

fn marker_claim(
    backend: &crate::BackendContext,
    path: &Path,
) -> Result<Option<(DirectoryMarker, FileIdentity)>> {
    let marker_path = path.join(DIRECTORY_MARKER);
    let guard = ParentGuard::acquire(backend, &marker_path)?;
    let identity = guard.directory_identity()?;
    let mut file = match guard.open_file(false, false) {
        Ok(file) => file,
        Err(error) if error.code == "missing" => return Ok(None),
        Err(error) => return Err(error),
    };
    let bytes = read_source(&mut file)?;
    validate_directory_marker(&bytes)?;
    Ok(Some((serde_json::from_slice(&bytes)?, identity)))
}

impl Core {
    pub(crate) fn validate_navigation_reservation(
        &self,
        id: &str,
        suffix: &str,
        bytes: &[u8],
    ) -> Result<hosted::ActorLibraryBinding> {
        let invalid = || {
            CoreError::new(
                "resolutionUnavailable",
                "Original navigation reservation cannot be qualified to this attempt and library",
            )
        };
        let (library, original, root, workspace, ids, fingerprint) = match suffix {
            "tag-reservation" => {
                let r: TagReservation = serde_json::from_slice(bytes).map_err(|_| invalid())?;
                let mut ids = r.additional_tag_ids;
                ids.push(r.tag_id);
                (
                    r.library,
                    r.operation_id,
                    r.root,
                    r.workspace_id,
                    ids,
                    r.fingerprint,
                )
            }
            "directory-reservation" => {
                let r: DirectoryReservation =
                    serde_json::from_slice(bytes).map_err(|_| invalid())?;
                if !Path::new(&r.path).starts_with(&r.root) {
                    return Err(invalid());
                }
                (
                    r.library,
                    r.operation_id,
                    r.root,
                    r.workspace_id,
                    vec![r.directory_id],
                    r.fingerprint,
                )
            }
            _ => return Err(invalid()),
        };
        let binding = library.ok_or_else(invalid)?;
        let original_root = self.validate_original_reservation_library(&binding)?;
        if original.as_deref() != Some(id)
            || !root.eq_ignore_ascii_case(&original_root)
            || binding.workspace_id.as_deref() != Some(&workspace)
            || ids.len() > 5000
            || fingerprint.len() != 64
            || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid());
        }
        for id in ids {
            valid_operation_id(&id)?;
        }
        Ok(binding)
    }

    /// Copy consumes the admitted source identity, while the frozen copy plan
    /// allocates a fresh ID. Its retained directory creation handle supplies the
    /// physical birth evidence in the same original operation journal.
    pub(crate) fn copy_directory_marker(
        &self,
        root: &Path,
        source: &Path,
        before: &[u8],
    ) -> Result<Vec<u8>> {
        let marker: DirectoryMarker = serde_json::from_slice(before)?;
        let workspace = self.require_workspace(root)?;
        let reference = entity_ref("directory", &workspace.workspace_id, &marker.directory_id);
        let resolved = self.resolve_navigation_target(root, &reference)?;
        if resolved.status != "managed"
            || resolved.path.as_deref() != source.parent().map(display).as_deref()
        {
            return Err(CoreError::new(
                "identityConflict",
                "A copied directory marker must have one admitted original object binding",
            ));
        }
        Ok(serde_json::to_vec_pretty(&DirectoryMarker {
            directory_id: fresh(),
        })?)
    }

    /// Called by the single UpdateFiles planner and executor, before installing.
    /// Admission is create-only and is bound to the retained directory object.
    pub(crate) fn validate_directory_plan(
        &self,
        operation_id: &str,
        root: &Path,
        path: &Path,
        after: &[u8],
    ) -> Result<()> {
        let reservation: DirectoryReservation = self
            .read_navigation_record(operation_id, "directory-reservation")?
            .ok_or_else(|| {
                CoreError::new(
                    "identityRequired",
                    "Directory admission requires its original Core allocation",
                )
            })?;
        let marker: DirectoryMarker = serde_json::from_slice(after)?;
        let parent = path
            .parent()
            .ok_or_else(|| CoreError::new("invalidPath", "Directory marker has no parent"))?;
        let guard = ParentGuard::acquire(&self.backend, path)?;
        if reservation.root != display(root)
            || reservation.path != display(parent)
            || reservation.workspace_id != self.require_workspace(root)?.workspace_id
            || reservation.directory_id != marker.directory_id
            || reservation.identity != guard.directory_identity()?
            || path.file_name().is_none_or(|name| name != DIRECTORY_MARKER)
            || self
                .operations
                .get(operation_id)
                .is_some_and(|plan| plan.fingerprint != reservation.fingerprint)
        {
            return Err(CoreError::new(
                "identityConflict",
                "Directory allocation, parent object or marker differs from the original proposal",
            ));
        }
        match guard.open_file(false, false) {
            Err(error) if error.code == "missing" => Ok(()),
            Ok(_) => Err(CoreError::new(
                "exists",
                "Directory identity admission is create-only",
            )),
            Err(error) => Err(error),
        }
    }

    fn read_navigation_record<T: for<'de> Deserialize<'de>>(
        &self,
        operation_id: &str,
        suffix: &str,
    ) -> Result<Option<T>> {
        match self
            .backend
            .read_bytes(self.journal_path(operation_id, suffix))
        {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.code == "missing" => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn directory_admitted(
        &self,
        workspace: &str,
        id: &str,
        identity: &FileIdentity,
    ) -> Result<bool> {
        Ok(!self
            .directory_admission_paths(workspace, id, identity)?
            .is_empty())
    }
    pub(crate) fn directory_admission_paths(
        &self,
        workspace: &str,
        id: &str,
        identity: &FileIdentity,
    ) -> Result<Vec<(String, String)>> {
        let mut origins = vec![];
        for plan in self.committed_plans()?.into_iter() {
            if plan.workspace_id.as_deref() == Some(workspace)
                && let Some(transaction) = plan.transaction.as_ref()
                && matches!(
                    transaction.request.intent,
                    OperationIntent::CopyNode { .. }
                        | OperationIntent::TemplateNode { .. }
                        | OperationIntent::ImportNative { .. }
                        | OperationIntent::ImportBibliography { .. }
                        | OperationIntent::ImportRecognized { .. }
                        | OperationIntent::ImportDocx { .. }
                        | OperationIntent::ImportDocxWithOptions { .. }
                        | OperationIntent::CloneSync { .. }
                        | OperationIntent::ApplySync { .. }
                        | OperationIntent::MergeSync { .. }
                        | OperationIntent::RevalidateSync { .. }
                )
            {
                for file in transaction
                    .created_files()
                    .iter()
                    .filter(|file| is_navigation_marker_output(plan, transaction, file))
                {
                    if serde_json::from_slice::<DirectoryMarker>(&file.after)
                        .is_ok_and(|marker| marker.directory_id == id)
                        && self
                            .materialized_identity(
                                &plan.operation_id,
                                Path::new(&file.path).parent().unwrap(),
                            )?
                            .as_ref()
                            == Some(identity)
                    {
                        origins.push((
                            display(Path::new(&file.path).parent().unwrap()),
                            plan.operation_id.clone(),
                        ));
                    }
                }
            }
            if plan.workspace_id.as_deref() == Some(workspace)
                && let Some(application) = &plan.sync_application
                && let Some(transaction) = &plan.transaction
            {
                for object in &application.after {
                    if matches!(&object.object, sync::ObjectKey::RootDirectory(value) if value == id)
                        && transaction.write_parents.iter().any(|(p, bound)| {
                            Path::new(p).parent() == Some(Path::new(&object.directory))
                                && bound == identity
                        })
                    {
                        origins.push((object.directory.clone(), plan.operation_id.clone()));
                    }
                }
            }
            let Some(reservation) = self.read_navigation_record::<DirectoryReservation>(
                &plan.operation_id,
                "directory-reservation",
            )?
            else {
                continue;
            };
            if reservation.workspace_id != workspace
                || reservation.directory_id != id
                || &reservation.identity != identity
                || reservation.fingerprint != plan.fingerprint
            {
                continue;
            }
            let Some(transaction) = plan.transaction.as_ref() else {
                continue;
            };
            if let OperationIntent::UpdateFiles { changes } = &transaction.request.intent {
                let expected = Path::new(&reservation.path).join(DIRECTORY_MARKER);
                if changes.iter().any(|change| {
                    change.path == display(&expected)
                        && change.before.is_none()
                        && serde_json::from_slice::<DirectoryMarker>(&change.after)
                            .is_ok_and(|marker| marker.directory_id == id)
                }) {
                    origins.push((reservation.path.clone(), plan.operation_id.clone()));
                }
            }
        }
        Ok(origins)
    }
    /// Match only bounded, actually observed FIs against the same original
    /// positive directory roles. No workspace candidate collection is created.
    pub(crate) fn any_directory_identity_admitted(
        &self,
        workspace: &str,
        identities: &BTreeSet<String>,
    ) -> Result<bool> {
        let observed = |identity: &FileIdentity| -> Result<bool> {
            Ok(identities.contains(&serde_json::to_string(identity)?))
        };
        for plan in self.committed_plans()?.into_iter() {
            if plan.workspace_id.as_deref() == Some(workspace)
                && let Some(transaction) = &plan.transaction
            {
                if matches!(
                    transaction.request.intent,
                    OperationIntent::CopyNode { .. }
                        | OperationIntent::TemplateNode { .. }
                        | OperationIntent::ImportNative { .. }
                        | OperationIntent::ImportBibliography { .. }
                        | OperationIntent::ImportRecognized { .. }
                        | OperationIntent::ImportDocx { .. }
                        | OperationIntent::ImportDocxWithOptions { .. }
                        | OperationIntent::CloneSync { .. }
                        | OperationIntent::ApplySync { .. }
                        | OperationIntent::MergeSync { .. }
                        | OperationIntent::RevalidateSync { .. }
                ) {
                    for file in transaction
                        .created_files()
                        .iter()
                        .filter(|file| is_navigation_marker_output(plan, transaction, file))
                    {
                        if serde_json::from_slice::<DirectoryMarker>(&file.after).is_ok()
                            && let Some(identity) = self.materialized_identity(
                                &plan.operation_id,
                                Path::new(&file.path).parent().unwrap(),
                            )?
                            && observed(&identity)?
                        {
                            return Ok(true);
                        }
                    }
                }
                if let Some(application) = &plan.sync_application {
                    for object in &application.after {
                        if matches!(object.object, sync::ObjectKey::RootDirectory(_)) {
                            for (path, identity) in &transaction.write_parents {
                                if Path::new(path).parent() == Some(Path::new(&object.directory))
                                    && observed(identity)?
                                {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
            let Some(reservation) = self.read_navigation_record::<DirectoryReservation>(
                &plan.operation_id,
                "directory-reservation",
            )?
            else {
                continue;
            };
            if reservation.workspace_id != workspace
                || !observed(&reservation.identity)?
                || reservation.fingerprint != plan.fingerprint
            {
                continue;
            }
            if let Some(transaction) = &plan.transaction
                && let OperationIntent::UpdateFiles { changes } = &transaction.request.intent
            {
                let expected = Path::new(&reservation.path).join(DIRECTORY_MARKER);
                if changes.iter().any(|change| {
                    change.path == display(&expected)
                        && change.before.is_none()
                        && serde_json::from_slice::<DirectoryMarker>(&change.after)
                            .is_ok_and(|marker| marker.directory_id == reservation.directory_id)
                }) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub(crate) fn directory_admission_candidates(
        &self,
        workspace: &str,
    ) -> Result<Vec<(String, String, FileIdentity)>> {
        let mut candidates = Vec::new();
        for p in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && p.workspace_id.as_deref() == Some(workspace)
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
            if let Some(r) = self.read_navigation_record::<DirectoryReservation>(
                &p.operation_id,
                "directory-reservation",
            )? {
                candidates.push((r.path, r.directory_id, r.identity));
            }
            if let Some(t) = &p.transaction {
                for f in t
                    .created_files()
                    .iter()
                    .filter(|f| is_navigation_marker_output(p, t, f))
                {
                    if let Ok(m) = serde_json::from_slice::<DirectoryMarker>(&f.after) {
                        let parent = Path::new(&f.path).parent().unwrap();
                        if let Some(identity) =
                            self.materialized_identity(&p.operation_id, parent)?
                        {
                            candidates.push((display(parent), m.directory_id, identity));
                        }
                    }
                }
            }
            if let Some(a) = &p.sync_application
                && let Some(t) = &p.transaction
            {
                for o in &a.after {
                    if let sync::ObjectKey::RootDirectory(id) = &o.object
                        && let Some((_, identity)) = t.write_parents.iter().find(|(path, _)| {
                            Path::new(path).parent() == Some(Path::new(&o.directory))
                        })
                    {
                        candidates.push((o.directory.clone(), id.clone(), identity.clone()));
                    }
                }
            }
        }
        Ok(candidates)
    }

    fn admitted_tag_ids(
        &self,
        root: &Path,
        workspace: &str,
        identity: &FileIdentity,
    ) -> Result<BTreeSet<String>> {
        let registry = display(&root.join(TAG_REGISTRY));
        let mut ids = BTreeSet::new();
        for plan in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && p.workspace_id.as_deref() == Some(workspace)
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
            let Some(transaction) = &plan.transaction else {
                continue;
            };
            let originals = transaction
                .created_files()
                .iter()
                .chain(tag_update_files(transaction))
                .filter(|f| f.path == registry)
                .collect::<Vec<_>>();
            if originals.is_empty() {
                continue;
            }
            let physical = transaction
                .files
                .iter()
                .any(|f| f.path == registry && &f.stamp.identity == identity)
                || self
                    .materialized_identity(&plan.operation_id, Path::new(&registry))?
                    .as_ref()
                    == Some(identity);
            if !physical {
                continue;
            }
            if matches!(
                transaction.request.intent,
                OperationIntent::CloneSync { .. }
                    | OperationIntent::ApplySync { .. }
                    | OperationIntent::MergeSync { .. }
                    | OperationIntent::RevalidateSync { .. }
            ) {
                // Sync admission is the complete accepted object, not per-edge
                // identity. Decode its original accepted blob once.
                for original in originals {
                    ids.extend(
                        tag_document(Some(&original.after))?
                            .tags
                            .into_iter()
                            .map(|t| t.tag_id),
                    );
                }
            } else if let Some(r) = self
                .read_navigation_record::<TagReservation>(&plan.operation_id, "tag-reservation")?
                && r.workspace_id == workspace
                && r.root == display(root)
                && r.fingerprint == plan.fingerprint
                && r.registry_identity.as_ref().is_none_or(|fi| fi == identity)
            {
                // The committed allocation is the birth fact. Current planners
                // require every reserved ID in the after image; reading every
                // historical full registry again would be quadratic in births.
                ids.extend(std::iter::once(r.tag_id).chain(r.additional_tag_ids));
            }
        }
        Ok(ids)
    }

    fn tag_admitted(
        &self,
        root: &Path,
        workspace: &str,
        id: &str,
        identity: &FileIdentity,
    ) -> Result<bool> {
        Ok(self
            .admitted_tag_ids(root, workspace, identity)?
            .contains(id))
    }

    pub fn capture_navigation_target(
        &self,
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> Result<NavigationTarget> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        if !path.starts_with(&root) {
            return Err(CoreError::new(
                "outsideRoot",
                "Navigation target is outside the selected library",
            ));
        }
        self.check_pending(&path)?;
        let inventory = self.inventory(&root)?;
        let source = inventory
            .entries
            .iter()
            .find(|entry| same_path(&entry.path, &path) && entry.kind == "node")
            .or_else(|| {
                inventory.entries.iter().find(|entry| {
                    Path::new(&entry.path)
                        .parent()
                        .is_some_and(|parent| same_path(&display(parent), &path))
                        && entry.kind == "node"
                })
            });
        if let Some(source) = source {
            if source.identity_status != "admitted" {
                return Ok(target(
                    "node",
                    "identityInvalid",
                    None,
                    Some(display(&path)),
                    source.name.clone(),
                ));
            }
            let reference = entity_ref(
                "node",
                &workspace.workspace_id,
                source.node_id.as_deref().unwrap(),
            );
            return self.resolve_navigation_target(&root, &reference);
        }
        let label = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if path != root
            && !inventory
                .entries
                .iter()
                .any(|entry| same_path(&entry.path, &path) && entry.kind == "directory")
        {
            return Ok(target(
                "directory",
                if inventory.complete {
                    "missing"
                } else {
                    "identityInvalid"
                },
                None,
                None,
                label,
            ));
        }
        match marker_claim(&self.backend, &path) {
            Ok(Some((marker, _))) => self.resolve_navigation_target(
                &root,
                &entity_ref("directory", &workspace.workspace_id, &marker.directory_id),
            ),
            Ok(None) => Ok(target(
                "directory",
                "unmanaged",
                None,
                Some(display(&path)),
                label,
            )),
            Err(error) if error.code == "missing" => {
                Ok(target("directory", "missing", None, None, label))
            }
            Err(error) if error.code == "invalidState" || error.code == "invalidOperationId" => {
                Ok(target(
                    "directory",
                    "identityInvalid",
                    None,
                    Some(display(&path)),
                    label,
                ))
            }
            Err(error) => Err(error),
        }
    }

    pub fn resolve_navigation_target(
        &self,
        root: impl AsRef<Path>,
        reference: &str,
    ) -> Result<NavigationTarget> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let (kind, ref_workspace, id) = parse_ref(reference)?;
        if workspace.workspace_id != ref_workspace {
            return Err(CoreError::new(
                "outsideWorkspace",
                "Navigation reference belongs to a different workspace",
            ));
        }
        if kind == "tag" {
            let registry = root.join(TAG_REGISTRY);
            self.check_pending(&registry)?;
            let guard = ParentGuard::acquire(&self.backend, &registry)?;
            let mut file = match guard.open_file(false, false) {
                Ok(file) => file,
                Err(error) if error.code == "missing" => {
                    return Ok(target(
                        kind,
                        "missing",
                        Some(reference.into()),
                        None,
                        String::new(),
                    ));
                }
                Err(error) => return Err(error),
            };
            let observed_stamp = stamp(&file)?;
            let identity = observed_stamp.identity.clone();
            let bytes = operations::read_blob(&mut file)?;
            let doc = match tag_document(Some(&bytes)) {
                Ok(doc) => doc,
                Err(_) => {
                    return Ok(target(
                        kind,
                        "identityInvalid",
                        Some(reference.into()),
                        None,
                        String::new(),
                    ));
                }
            };
            let captured = operations::CapturedFile {
                path: display(&registry),
                stamp: observed_stamp,
                bytes,
            };
            if self.accepted_revision(&captured)?.is_none()
                || !self.tag_registry_identity_admitted(&root, &identity)?
                || !self.tag_admitted(&root, ref_workspace, id, &identity)?
            {
                return Ok(target(
                    kind,
                    "identityInvalid",
                    Some(reference.into()),
                    None,
                    String::new(),
                ));
            }
            let Some(tag) = doc.tags.iter().find(|tag| tag.tag_id == id) else {
                return Ok(target(
                    kind,
                    "missing",
                    Some(reference.into()),
                    None,
                    String::new(),
                ));
            };
            let mut resolved = target(
                kind,
                "managed",
                Some(reference.into()),
                Some(display(&root)),
                tag.display_name.clone(),
            );
            resolved.display_name = Some(tag.display_name.clone());
            return Ok(resolved);
        }
        let inventory = self.inventory(&root)?;
        if !inventory.complete {
            return Ok(target(
                kind,
                "identityInvalid",
                Some(reference.into()),
                None,
                String::new(),
            ));
        }
        if kind == "node" {
            let matches = inventory
                .entries
                .iter()
                .filter(|entry| entry.node_id.as_deref() == Some(id))
                .collect::<Vec<_>>();
            if matches.is_empty() {
                return Ok(target(
                    kind,
                    "missing",
                    Some(reference.into()),
                    None,
                    String::new(),
                ));
            }
            if matches.len() != 1 || matches[0].identity_status != "admitted" {
                return Ok(target(
                    kind,
                    "identityInvalid",
                    Some(reference.into()),
                    None,
                    String::new(),
                ));
            }
            let entry = matches[0];
            let parent = Path::new(&entry.path).parent().unwrap();
            return Ok(target(
                kind,
                "managed",
                Some(reference.into()),
                Some(display(parent)),
                parent
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        let mut matches = Vec::new();
        let candidates = std::iter::once(root.clone()).chain(
            inventory
                .entries
                .iter()
                .filter(|entry| entry.kind == "directory")
                .map(|entry| PathBuf::from(&entry.path)),
        );
        for path in candidates {
            self.check_pending(&path.join(DIRECTORY_MARKER))?;
            if let Some((marker, identity)) = marker_claim(&self.backend, &path)?
                && marker.directory_id == id
            {
                matches.push((path, identity));
            }
        }
        if matches.is_empty() {
            return Ok(target(
                kind,
                "missing",
                Some(reference.into()),
                None,
                String::new(),
            ));
        }
        if matches.len() != 1 || !self.directory_admitted(ref_workspace, id, &matches[0].1)? {
            return Ok(target(
                kind,
                "identityInvalid",
                Some(reference.into()),
                None,
                String::new(),
            ));
        }
        let path = &matches[0].0;
        Ok(target(
            kind,
            "managed",
            Some(reference.into()),
            Some(display(path)),
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ))
    }

    /// Read the original specialized reservation only after its canonical Plan
    /// matches. Hosted wrappers must not allocate a new identity during status.
    pub(crate) fn original_navigation_target(
        &self,
        operation_id: &str,
        canonical: &serde_json::Value,
    ) -> Result<String> {
        if self
            .feature_operation_status(operation_id, canonical)?
            .is_none()
        {
            return Err(CoreError::new(
                "operationMissing",
                "Original navigation plan is unavailable",
            ));
        }
        let fingerprint = hash(&operations::canonical_json_bytes(canonical)?);
        match canonical["kind"].as_str() {
            Some("admitDirectory") => {
                let reservation: DirectoryReservation = self
                    .read_navigation_record(operation_id, "directory-reservation")?
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidState",
                            "Original directory reservation is unavailable",
                        )
                    })?;
                if reservation.fingerprint != fingerprint
                    || canonical["root"] != reservation.root
                    || canonical["path"] != reservation.path
                {
                    return Err(CoreError::new(
                        "operationConflict",
                        "Original directory reservation differs",
                    ));
                }
                Ok(entity_ref(
                    "directory",
                    &reservation.workspace_id,
                    &reservation.directory_id,
                ))
            }
            Some("createTag") => {
                let reservation: TagReservation = self
                    .read_navigation_record(operation_id, "tag-reservation")?
                    .ok_or_else(|| {
                        CoreError::new("invalidState", "Original tag reservation is unavailable")
                    })?;
                if reservation.fingerprint != fingerprint || canonical["root"] != reservation.root {
                    return Err(CoreError::new(
                        "operationConflict",
                        "Original tag reservation differs",
                    ));
                }
                Ok(entity_ref(
                    "tag",
                    &reservation.workspace_id,
                    &reservation.tag_id,
                ))
            }
            Some("renameTag" | "moveTag") => canonical["tagRef"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| {
                    CoreError::new("invalidState", "Original tag reference is unavailable")
                }),
            _ => Err(CoreError::new(
                "unsupportedIntent",
                "Expected an original navigation operation",
            )),
        }
    }

    pub fn prepare_directory_admission(
        &mut self,
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        valid_operation_id(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(path.as_ref())?;
        let canonical = serde_json::json!({"kind":"admitDirectory","root":display(&root),"path":display(&path)});
        if let Some(preview) = self.feature_operation_status(operation_id, &canonical)? {
            let reservation: DirectoryReservation = self
                .read_navigation_record(operation_id, "directory-reservation")?
                .ok_or_else(|| {
                    CoreError::new("invalidState", "Original directory allocation is missing")
                })?;
            return Ok(NavigationPreparation {
                preview,
                target_ref: entity_ref(
                    "directory",
                    &reservation.workspace_id,
                    &reservation.directory_id,
                ),
            });
        }
        let workspace = self.require_workspace(&root)?;
        if self.capture_navigation_target(&root, &path)?.status != "unmanaged" {
            return Err(CoreError::new(
                "identityConflict",
                "Only an explicitly selected non-Node container can receive a directory identity",
            ));
        }
        let marker_path = path.join(DIRECTORY_MARKER);
        let guard = ParentGuard::acquire(&self.backend, &marker_path)?;
        let identity = guard.directory_identity()?;
        let fingerprint = hash(&operations::canonical_json_bytes(&canonical)?);
        let reservation = if let Some(old) = self
            .read_navigation_record::<DirectoryReservation>(operation_id, "directory-reservation")?
        {
            if old.fingerprint != fingerprint || old.identity != identity {
                return Err(CoreError::new(
                    "operationConflict",
                    "Original directory allocation belongs to another proposal or directory object",
                ));
            }
            old
        } else {
            let new = DirectoryReservation {
                library: self.draft_library_for_root(&root)?,
                operation_id: Some(operation_id.into()),
                root: display(&root),
                path: display(&path),
                workspace_id: workspace.workspace_id,
                directory_id: fresh(),
                identity,
                fingerprint,
            };
            journal::private_create(
                &self.backend,
                &self.journal_path(operation_id, "directory-reservation"),
                &serde_json::to_vec(&new)?,
            )?;
            new
        };
        let snapshot = self
            .read_snapshot(&root, &[display(&marker_path)])?
            .files
            .remove(0);
        let after = serde_json::to_vec_pretty(&DirectoryMarker {
            directory_id: reservation.directory_id.clone(),
        })?;
        let preview = self.prepare_feature_operation(
            OperationRequest {
                operation_id: operation_id.into(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&marker_path),
                        base_id: Some(snapshot.base_id),
                        before: snapshot.bytes,
                        after,
                    }],
                },
            },
            &canonical,
        )?;
        Ok(NavigationPreparation {
            preview,
            target_ref: entity_ref(
                "directory",
                &reservation.workspace_id,
                &reservation.directory_id,
            ),
        })
    }

    pub fn read_tag_registry(&mut self, root: impl AsRef<Path>) -> Result<TagRegistrySnapshot> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let path = root.join(TAG_REGISTRY);
        let snapshot = self
            .read_snapshot(&root, &[display(&path)])?
            .files
            .remove(0);
        let doc = tag_document(snapshot.bytes.as_deref())?;
        let identity = self.snapshots[&snapshot.base_id]
            .captured
            .as_ref()
            .map(|file| file.stamp.identity.clone());
        let mut tags = Vec::new();
        let mut warnings = Vec::new();
        if snapshot.bytes.is_none() && self.tag_registry_was_born(&root)? {
            return Err(CoreError::new(
                "identityConflict",
                "The admitted registry is missing; recover the original object",
            ));
        }
        if snapshot.bytes.is_some()
            && (snapshot.revision.is_none()
                || identity
                    .as_ref()
                    .map(|fi| {
                        self.tag_registry_identity_admitted(&root, fi)
                            .map(|admitted| !admitted)
                    })
                    .transpose()?
                    .unwrap_or(true))
        {
            return Ok(TagRegistrySnapshot {
                tags,
                base_id: snapshot.base_id,
                complete: false,
                warnings: vec![
                    "Registry is external_pending or its physical identity is unadmitted".into(),
                ],
            });
        }
        let admitted = identity
            .as_ref()
            .map(|fi| self.admitted_tag_ids(&root, &workspace.workspace_id, fi))
            .transpose()?
            .unwrap_or_default();
        for tag in doc.tags {
            if admitted.contains(&tag.tag_id) {
                tags.push(TagRecord {
                    tag_ref: entity_ref("tag", &workspace.workspace_id, &tag.tag_id),
                    tag_id: tag.tag_id,
                    display_name: tag.display_name,
                    parent_tag_id: tag.parent_tag_id,
                });
            } else {
                warnings.push(format!(
                    "Tag {} has no committed binding to this registry object",
                    tag.tag_id
                ));
            }
        }
        Ok(TagRegistrySnapshot {
            tags,
            base_id: snapshot.base_id,
            complete: warnings.is_empty(),
            warnings,
        })
    }

    fn tag_write_basis(&self, root: &Path, base_id: &str) -> Result<TagWriteBasis> {
        let basis = self
            .snapshots
            .get(base_id)
            .ok_or_else(|| CoreError::new("staleBase", "Read the tag registry before preparing"))?;
        if basis.path != display(&root.join(TAG_REGISTRY)) {
            return Err(CoreError::new(
                "outsideRoot",
                "Tag registry basis belongs to another root or role",
            ));
        }
        let before = basis.captured.as_ref().map(|file| file.bytes.clone());
        let identity = basis
            .captured
            .as_ref()
            .map(|file| file.stamp.identity.clone());
        if before.is_none() && self.tag_registry_was_born(root)? {
            return Err(CoreError::new(
                "identityConflict",
                "An admitted registry is missing; recover its original object",
            ));
        }
        if let Some(file) = &basis.captured
            && (self.accepted_revision(file)?.is_none()
                || !self.tag_registry_identity_admitted(root, &file.stamp.identity)?)
        {
            return Err(CoreError::new(
                "externalPending",
                "The registry requires its original accepted physical binding",
            ));
        }
        let doc = tag_document(before.as_deref())?;
        let workspace = self.require_workspace(root)?;
        let admitted = identity
            .as_ref()
            .map(|fi| self.admitted_tag_ids(root, &workspace.workspace_id, fi))
            .transpose()?
            .unwrap_or_default();
        for tag in &doc.tags {
            if !admitted.contains(&tag.tag_id) {
                return Err(CoreError::new(
                    "identityConflict",
                    "Registry contains an unadmitted TagId or replaced registry object",
                ));
            }
        }
        Ok(TagWriteBasis {
            document: doc,
            bytes: before,
            identity,
        })
    }

    fn tag_registry_was_born(&self, root: &Path) -> Result<bool> {
        let path = display(&root.join(TAG_REGISTRY));
        Ok(self.committed_plans()?.into_iter().any(|p| {
            p.transaction.as_ref().is_some_and(|t| {
                t.created_files().iter().any(|f| f.path == path)
                    || t.files.iter().any(|f| f.path == path)
                    || tag_update_files(t).iter().any(|f| f.path == path)
            })
        }))
    }

    fn tag_registry_identity_admitted(&self, root: &Path, identity: &FileIdentity) -> Result<bool> {
        let path = display(&root.join(TAG_REGISTRY));
        for plan in self.committed_plans()? {
            let Some(t) = plan.transaction.as_ref() else {
                continue;
            };
            if t.files
                .iter()
                .any(|f| f.path == path && &f.stamp.identity == identity)
            {
                return Ok(true);
            }
            if (t.created_files().iter().any(|f| f.path == path)
                || tag_update_files(t)
                    .iter()
                    .any(|f| f.path == path && f.before.is_none()))
                && self
                    .materialized_identity(&plan.operation_id, Path::new(&path))?
                    .as_ref()
                    == Some(identity)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Qualify only the exact bytes already captured with the caller's other
    /// inputs. This does not open or refresh the registry after the barrier.
    pub fn qualified_tag_document_from_snapshot(
        &self,
        root: impl AsRef<Path>,
        file: &SnapshotFile,
    ) -> Result<TagDocument> {
        let root = absolute_path(root.as_ref())?;
        if Path::new(&file.path) != root.join(TAG_REGISTRY) {
            return Err(CoreError::new("outsideRoot", "Not the selected Tags role"));
        }
        let basis = self.tag_write_basis(&root, &file.base_id)?;
        if basis.bytes != file.bytes {
            return Err(CoreError::new(
                "conflict",
                "Frozen tag bytes differ from the original capture",
            ));
        }
        Ok(basis.document)
    }

    pub(crate) fn validate_tag_registry_absence(&self, root: &Path) -> Result<()> {
        if self.tag_registry_was_born(root)? {
            Err(CoreError::new(
                "identityConflict",
                "The original Tags object is missing",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn qualified_tag_document_from_captured(
        &self,
        root: &Path,
        file: &operations::CapturedFile,
    ) -> Result<TagDocument> {
        if Path::new(&file.path) != root.join(TAG_REGISTRY)
            || !self.tag_registry_identity_admitted(root, &file.stamp.identity)?
            || self.accepted_revision(file)?.is_none()
        {
            return Err(CoreError::new(
                "identityConflict",
                "Captured Tags are not the original accepted registry",
            ));
        }
        let doc = tag_document(Some(&file.bytes))?;
        let ws = self.require_workspace(root)?;
        let admitted = self.admitted_tag_ids(root, &ws.workspace_id, &file.stamp.identity)?;
        for tag in &doc.tags {
            if !admitted.contains(&tag.tag_id) {
                return Err(CoreError::new(
                    "identityConflict",
                    "Captured tag is not admitted",
                ));
            }
        }
        Ok(doc)
    }

    /// Native import owns this one original allocation; complete source refs
    /// map to fresh target identities, with all ancestors in the same Plan.
    pub(crate) fn prepare_import_tag_changes(
        &mut self,
        root: &Path,
        source_workspace: &str,
        source_doc: &TagDocument,
        operation_id: &str,
        fingerprint: &str,
    ) -> Result<(FileChange, std::collections::BTreeMap<String, String>)> {
        source_doc
            .forest()
            .map_err(|e| CoreError::new("tagRegistry", e.to_string()))?;
        valid_operation_id(source_workspace)?;
        valid_operation_id(operation_id)?;
        let registry = self.read_tag_registry(root)?;
        if !registry.complete {
            return Err(CoreError::new(
                "identityConflict",
                "Import needs the complete qualified target Tags object",
            ));
        }
        let TagWriteBasis {
            document: mut doc,
            bytes: before,
            identity,
        } = self.tag_write_basis(root, &registry.base_id)?;
        if doc.tags.len() + source_doc.tags.len() > 5000 {
            return Err(CoreError::new(
                "budget",
                "Imported forest exceeds 5000 tags",
            ));
        }
        let workspace = self.require_workspace(root)?.workspace_id;
        let reservation = if let Some(old) =
            self.read_navigation_record::<TagReservation>(operation_id, "tag-reservation")?
        {
            if old.root != display(root)
                || old.workspace_id != workspace
                || old.registry_identity != identity
                || old.fingerprint != fingerprint
                || old.additional_tag_ids.len() + 1 != source_doc.tags.len().max(1)
            {
                return Err(CoreError::new(
                    "operationConflict",
                    "Original tag import allocation differs",
                ));
            }
            old
        } else {
            let new = TagReservation {
                library: self.draft_library_for_root(root)?,
                operation_id: Some(operation_id.into()),
                root: display(root),
                workspace_id: workspace.clone(),
                tag_id: fresh(),
                additional_tag_ids: (1..source_doc.tags.len()).map(|_| fresh()).collect(),
                registry_identity: identity,
                fingerprint: fingerprint.into(),
            };
            journal::private_create(
                &self.backend,
                &self.journal_path(operation_id, "tag-reservation"),
                &serde_json::to_vec(&new)?,
            )?;
            new
        };
        let map_ids = source_doc
            .tags
            .iter()
            .zip(std::iter::once(&reservation.tag_id).chain(&reservation.additional_tag_ids))
            .map(|(t, id)| (t.tag_id.clone(), id.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        for tag in &source_doc.tags {
            doc.tags.push(TagDefinition {
                tag_id: map_ids[&tag.tag_id].clone(),
                display_name: tag.display_name.clone(),
                parent_tag_id: tag.parent_tag_id.as_ref().map(|p| map_ids[p].clone()),
            });
        }
        doc.forest()
            .map_err(|e| CoreError::new("tagRegistry", e.to_string()))?;
        let mappings = map_ids
            .into_iter()
            .map(|(old, new)| {
                (
                    entity_ref("tag", source_workspace, &old),
                    entity_ref("tag", &workspace, &new),
                )
            })
            .collect();
        Ok((
            FileChange {
                path: display(&root.join(TAG_REGISTRY)),
                base_id: Some(registry.base_id),
                before,
                after: serde_json::to_vec_pretty(&doc)?,
            },
            mappings,
        ))
    }

    /// The common UpdateFiles planner and executor both call this. A generic
    /// writer cannot allocate identities, erase tags, or install a v1 successor.
    pub(crate) fn validate_tag_update(
        &self,
        root: &Path,
        operation: &str,
        change: &FileChange,
    ) -> Result<()> {
        if Path::new(&change.path) != root.join(TAG_REGISTRY) {
            return Ok(());
        }
        validate_sync_tags(&change.after)?;
        let base = change.base_id.as_deref().ok_or_else(|| {
            CoreError::new("basisRequired", "Tag changes require their original basis")
        })?;
        let old = if self.snapshots.contains_key(base) {
            self.tag_write_basis(root, base)?
        } else {
            // A recovered operation uses its persisted original before image
            // and physical stamp, never an invented replacement snapshot.
            let t = self
                .operations
                .get(operation)
                .and_then(|p| p.transaction.as_ref())
                .ok_or_else(|| CoreError::new("staleBase", "Original tag Plan is unavailable"))?;
            let original = t.files.iter().find(|f| f.path == change.path);
            if change.before.is_none() {
                self.validate_tag_registry_absence(root)?;
            }
            if original.map(|f| &f.bytes) != change.before.as_ref() {
                return Err(CoreError::new(
                    "conflict",
                    "Original tag before image differs",
                ));
            }
            let document = tag_document(change.before.as_deref())?;
            let identity = original.map(|f| f.stamp.identity.clone());
            let workspace = self.require_workspace(root)?;
            let admitted = identity
                .as_ref()
                .map(|fi| self.admitted_tag_ids(root, &workspace.workspace_id, fi))
                .transpose()?
                .unwrap_or_default();
            if document
                .tags
                .iter()
                .any(|tag| !admitted.contains(&tag.tag_id))
            {
                return Err(CoreError::new(
                    "identityConflict",
                    "Recovered tags lack their original binding",
                ));
            }
            TagWriteBasis {
                document,
                bytes: change.before.clone(),
                identity,
            }
        };
        if old.bytes != change.before {
            return Err(CoreError::new("conflict", "Tag before bytes differ"));
        }
        let doc = tag_document(Some(&change.after))?;
        let before_ids = old
            .document
            .tags
            .iter()
            .map(|t| &t.tag_id)
            .collect::<BTreeSet<_>>();
        let after_ids = doc.tags.iter().map(|t| &t.tag_id).collect::<BTreeSet<_>>();
        if !before_ids.is_subset(&after_ids) {
            return Err(CoreError::new(
                "unsupportedIntent",
                "Tag deletion is not supported",
            ));
        }
        let added = doc
            .tags
            .iter()
            .filter(|t| !before_ids.contains(&t.tag_id))
            .collect::<Vec<_>>();
        let reservation =
            self.read_navigation_record::<TagReservation>(operation, "tag-reservation")?;
        if !added.is_empty() || reservation.is_some() {
            let reservation = reservation.ok_or_else(|| {
                CoreError::new(
                    "identityRequired",
                    "Tag creation needs its original Core allocation",
                )
            })?;
            let expected = std::iter::once(&reservation.tag_id)
                .chain(&reservation.additional_tag_ids)
                .collect::<BTreeSet<_>>();
            if added.iter().map(|t| &t.tag_id).collect::<BTreeSet<_>>() != expected
                || reservation.root != display(root)
                || reservation.registry_identity != old.identity
            {
                return Err(CoreError::new(
                    "identityConflict",
                    "New tags differ from the original allocation",
                ));
            }
        }
        Ok(())
    }

    /// Every local mutation shares this check, including root-role updates
    /// attached to a native CreateTree. A formerly empty physical slot cannot
    /// become a new registry after another committed registry went missing.
    pub(crate) fn validate_tag_mutation_changes(
        &self,
        root: &Path,
        operation: &str,
        fingerprint: &str,
        mutation: &operations::Mutation,
        synchronized: bool,
    ) -> Result<()> {
        let (created, updates): (&[FileChange], &[FileChange]) = match mutation {
            operations::Mutation::Update { files } => (&[], files),
            operations::Mutation::CreateTree { files, updates, .. } => (files, updates),
            operations::Mutation::Relocate { updates, .. } => (&[], updates),
            operations::Mutation::Compound {
                created_files,
                updates,
                ..
            } => (created_files, updates),
        };
        let mut seen = false;
        for change in created
            .iter()
            .chain(updates)
            .filter(|change| Path::new(&change.path) == root.join(TAG_REGISTRY))
        {
            if seen {
                return Err(CoreError::new(
                    "duplicatePath",
                    "One Plan cannot install the Tags role twice",
                ));
            }
            seen = true;
            // Received complete objects use their original replica evidence,
            // but no old v1 state can become a current successor implicitly.
            if synchronized {
                validate_sync_tags(&change.after)?;
                continue;
            }
            self.validate_tag_update(root, operation, change)?;
            if let Some(reservation) =
                self.read_navigation_record::<TagReservation>(operation, "tag-reservation")?
                && reservation.fingerprint != fingerprint
            {
                return Err(CoreError::new(
                    "operationConflict",
                    "Tag allocation belongs to another original author intent",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_tag_plan_changes(&self, plan: &journal::Plan) -> Result<()> {
        // A retained terminal receipt cannot acquire another filesystem effect.
        // Historical v1 receipts remain readable rather than being rewritten.
        if self.committed(&plan.operation_id)? || self.resolved(&plan.operation_id)? {
            return Ok(());
        }
        let transaction = plan.transaction.as_ref().ok_or_else(|| {
            CoreError::new("invalidState", "The original Tag Plan is unavailable")
        })?;
        self.validate_tag_mutation_changes(
            Path::new(&transaction.request.root),
            &plan.operation_id,
            &plan.fingerprint,
            &transaction.mutation,
            plan.sync_application.is_some() || plan.sync_setup.is_some(),
        )
    }

    pub fn prepare_create_tag(
        &mut self,
        root: impl AsRef<Path>,
        display_name: &str,
        base_id: &str,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        self.prepare_create_tag_under(root, display_name, None, base_id, operation_id)
    }

    pub fn prepare_create_tag_under(
        &mut self,
        root: impl AsRef<Path>,
        display_name: &str,
        parent_ref: Option<&str>,
        base_id: &str,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        self.prepare_create_tag_with_assignment(
            root,
            display_name,
            parent_ref,
            base_id,
            None,
            operation_id,
        )
    }

    /// Local editing supplies a session basis; narrow hosted fields supply an
    /// internal snapshot basis. Both must qualify the original saved bytes,
    /// never silently substitute the latest contents at the remembered path.
    fn tag_assignment_snapshot(
        &mut self,
        root: &Path,
        assignment: &TagAssignment,
    ) -> Result<(String, operations::CapturedFile)> {
        if let Some(file) = self
            .snapshots
            .get(&assignment.base_id)
            .filter(|b| b.path == assignment.path)
            .and_then(|b| b.captured.clone())
        {
            return Ok((assignment.base_id.clone(), file));
        }
        let (session_id, original) = self
            .sessions
            .iter()
            .find(|(_, s)| {
                s.actor == self.actor
                    && s.root == root
                    && display(&s.path) == assignment.path
                    && s.base_id == assignment.base_id
            })
            .map(|(id, s)| (id.clone(), s.clone()))
            .ok_or_else(|| {
                CoreError::new(
                    "staleBase",
                    "First assignment needs the exact saved note basis",
                )
            })?;
        self.check_session_base(&session_id, &assignment.base_id)?;
        let snapshot = self
            .read_snapshot(root, std::slice::from_ref(&assignment.path))?
            .files
            .remove(0);
        let file = self
            .snapshots
            .get(&snapshot.base_id)
            .and_then(|b| b.captured.clone())
            .filter(|f| f.stamp == original.stamp && f.bytes == original.bytes)
            .ok_or_else(|| {
                CoreError::new(
                    "staleBase",
                    "The original saved note changed during assignment preparation",
                )
            })?;
        Ok((snapshot.base_id, file))
    }

    pub fn prepare_create_tag_with_assignment(
        &mut self,
        root: impl AsRef<Path>,
        display_name: &str,
        parent_ref: Option<&str>,
        base_id: &str,
        assignment: Option<TagAssignment>,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        valid_operation_id(operation_id)?;
        validate_tag_name(display_name)?;
        let root = absolute_path(root.as_ref())?;
        let canonical = serde_json::json!({"kind":"createTag","root":display(&root),"displayName":display_name,"parentRef":parent_ref,"baseId":base_id,"assignment":assignment});
        if let Some(preview) = self.feature_operation_status(operation_id, &canonical)? {
            let reservation: TagReservation = self
                .read_navigation_record(operation_id, "tag-reservation")?
                .ok_or_else(|| {
                    CoreError::new("invalidState", "Original tag allocation is missing")
                })?;
            return Ok(NavigationPreparation {
                preview,
                target_ref: entity_ref("tag", &reservation.workspace_id, &reservation.tag_id),
            });
        }
        let workspace = self.require_workspace(&root)?;
        let TagWriteBasis {
            document: mut doc,
            bytes: before,
            identity: registry_identity,
        } = self.tag_write_basis(&root, base_id)?;
        let parent_tag_id = self.tag_parent_id(&workspace.workspace_id, &doc, parent_ref)?;
        if doc.tags.len() >= 5000 {
            return Err(CoreError::new(
                "budget",
                "The tag registry already contains 5000 entries",
            ));
        }
        let fingerprint = hash(&operations::canonical_json_bytes(&canonical)?);
        let reservation = if let Some(old) =
            self.read_navigation_record::<TagReservation>(operation_id, "tag-reservation")?
        {
            if old.fingerprint != fingerprint || old.registry_identity != registry_identity {
                return Err(CoreError::new(
                    "operationConflict",
                    "Tag allocation belongs to another proposal or registry basis",
                ));
            }
            old
        } else {
            let new = TagReservation {
                library: self.draft_library_for_root(&root)?,
                operation_id: Some(operation_id.into()),
                root: display(&root),
                workspace_id: workspace.workspace_id,
                tag_id: fresh(),
                additional_tag_ids: vec![],
                registry_identity,
                fingerprint,
            };
            journal::private_create(
                &self.backend,
                &self.journal_path(operation_id, "tag-reservation"),
                &serde_json::to_vec(&new)?,
            )?;
            new
        };
        doc.tags.push(TagDefinition {
            tag_id: reservation.tag_id.clone(),
            display_name: display_name.into(),
            parent_tag_id,
        });
        let mut changes = vec![FileChange {
            path: display(&root.join(TAG_REGISTRY)),
            base_id: Some(base_id.into()),
            before,
            after: serde_json::to_vec_pretty(&doc)?,
        }];
        let mut dependencies = vec![];
        if let Some(assignment) = &assignment {
            let (source_base, file) = self.tag_assignment_snapshot(&root, assignment)?;
            let path = Path::new(&assignment.path);
            let current = self.managed_node(
                &root,
                path.parent()
                    .ok_or_else(|| CoreError::new("invalidPath", "Assignment needs a note"))?,
                false,
            )?;
            if !same_path(&current.path, Path::new(&file.path))
                || current.stamp.identity != file.stamp.identity
            {
                return Err(CoreError::new(
                    "identityConflict",
                    "Assignment must target the original admitted note",
                ));
            }
            let config = self.read_config(&root, ConfigRole::Schema)?;
            let schema = weftext_data::Schema::from_author_json(
                config
                    .bytes
                    .as_ref()
                    .map(|b| std::str::from_utf8(b))
                    .transpose()
                    .map_err(|_| CoreError::new("invalidSchema", "Schema is not UTF-8"))?
                    .unwrap_or(include_str!(
                        "../../../docs/design/examples/schema.authoring.json"
                    )),
            )
            .map_err(|e| CoreError::new("invalidSchema", e.to_string()))?;
            let source = decode(&file.bytes)?;
            let projection = weftext_data::project(&source, &schema);
            let field = projection
                .fields
                .iter()
                .find(|f| f.field_id == assignment.field_id && f.key == "tags" && f.writable)
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidField",
                        "First assignment requires the installed writable tags field",
                    )
                })?;
            let mut values = match &field.cell {
                weftext_data::Cell::Absent | weftext_data::Cell::Null => vec![],
                weftext_data::Cell::Value {
                    value: weftext_data::NativeValue::Array(values),
                } => values.clone(),
                _ => {
                    return Err(CoreError::new(
                        "invalidField",
                        "Existing tag input is invalid or unavailable; preserve it before assigning",
                    ));
                }
            };
            values.push(weftext_data::NativeValue::Record(vec![
                weftext_data::NativeMember {
                    key: "ref".into(),
                    value: weftext_data::NativeValue::Text(entity_ref(
                        "tag",
                        &reservation.workspace_id,
                        &reservation.tag_id,
                    )),
                },
            ]));
            let patch = weftext_data::patch_field(
                &source,
                &schema,
                &assignment.field_id,
                &[],
                &weftext_data::NativeValue::Array(values),
            )
            .map_err(|e| CoreError::new("invalidField", e.to_string()))?;
            changes.push(FileChange {
                path: file.path,
                base_id: Some(source_base),
                before: Some(file.bytes),
                after: patch.source.into_bytes(),
            });
            // Pin the exact schema in the same Plan, including its absence.
            dependencies.push(config.base_id);
        }
        let preview = self.prepare_dependent_feature_operation(
            OperationRequest {
                operation_id: operation_id.into(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles { changes },
            },
            &canonical,
            &dependencies,
        )?;
        Ok(NavigationPreparation {
            preview,
            target_ref: entity_ref("tag", &reservation.workspace_id, &reservation.tag_id),
        })
    }

    pub fn prepare_rename_tag(
        &mut self,
        root: impl AsRef<Path>,
        reference: &str,
        display_name: &str,
        base_id: &str,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        valid_operation_id(operation_id)?;
        validate_tag_name(display_name)?;
        let root = absolute_path(root.as_ref())?;
        let canonical = serde_json::json!({"kind":"renameTag","root":display(&root),"tagRef":reference,"displayName":display_name,"baseId":base_id});
        if let Some(preview) = self.feature_operation_status(operation_id, &canonical)? {
            return Ok(NavigationPreparation {
                preview,
                target_ref: reference.into(),
            });
        }
        let workspace = self.require_workspace(&root)?;
        let (kind, ref_workspace, id) = parse_ref(reference)?;
        if kind != "tag" || ref_workspace != workspace.workspace_id {
            return Err(CoreError::new(
                "invalidReference",
                "Rename requires a TagRef in the selected workspace",
            ));
        }
        let TagWriteBasis {
            document: mut doc,
            bytes: before,
            ..
        } = self.tag_write_basis(&root, base_id)?;
        let tag = doc
            .tags
            .iter_mut()
            .find(|tag| tag.tag_id == id)
            .ok_or_else(|| {
                CoreError::new(
                    "missingTag",
                    "The selected TagId is absent; matching display names are not replacements",
                )
            })?;
        tag.display_name = display_name.into();
        let preview = self.prepare_feature_operation(
            OperationRequest {
                operation_id: operation_id.into(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&root.join(TAG_REGISTRY)),
                        base_id: Some(base_id.into()),
                        before,
                        after: serde_json::to_vec_pretty(&doc)?,
                    }],
                },
            },
            &canonical,
        )?;
        Ok(NavigationPreparation {
            preview,
            target_ref: reference.into(),
        })
    }

    fn tag_parent_id(
        &self,
        workspace: &str,
        doc: &TagDocument,
        parent: Option<&str>,
    ) -> Result<Option<String>> {
        parent
            .map(|reference| {
                let (kind, ws, id) = parse_ref(reference)?;
                if kind != "tag" || ws != workspace || !doc.tags.iter().any(|t| t.tag_id == id) {
                    return Err(CoreError::new(
                        "invalidReference",
                        "Parent requires an admitted TagRef in this exact registry",
                    ));
                }
                Ok(id.to_owned())
            })
            .transpose()
    }

    pub fn prepare_move_tag(
        &mut self,
        root: impl AsRef<Path>,
        reference: &str,
        parent_ref: Option<&str>,
        base_id: &str,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        valid_operation_id(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        let canonical = serde_json::json!({"kind":"moveTag","root":display(&root),"tagRef":reference,"parentRef":parent_ref,"baseId":base_id});
        if let Some(preview) = self.feature_operation_status(operation_id, &canonical)? {
            return Ok(NavigationPreparation {
                preview,
                target_ref: reference.into(),
            });
        }
        let workspace = self.require_workspace(&root)?;
        let (kind, ws, id) = parse_ref(reference)?;
        if kind != "tag" || ws != workspace.workspace_id {
            return Err(CoreError::new(
                "invalidReference",
                "Move requires a TagRef in this workspace",
            ));
        }
        let TagWriteBasis {
            document: mut doc,
            bytes: before,
            ..
        } = self.tag_write_basis(&root, base_id)?;
        let parent = self.tag_parent_id(ws, &doc, parent_ref)?;
        doc.tags
            .iter_mut()
            .find(|t| t.tag_id == id)
            .ok_or_else(|| CoreError::new("missingTag", "The exact tag is unavailable"))?
            .parent_tag_id = parent;
        doc.forest()
            .map_err(|e| CoreError::new("tagRegistry", e.to_string()))?;
        let preview = self.prepare_feature_operation(
            OperationRequest {
                operation_id: operation_id.into(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&root.join(TAG_REGISTRY)),
                        base_id: Some(base_id.into()),
                        before,
                        after: serde_json::to_vec_pretty(&doc)?,
                    }],
                },
            },
            &canonical,
        )?;
        Ok(NavigationPreparation {
            preview,
            target_ref: reference.into(),
        })
    }

    pub fn cancel_navigation_preparation(
        &mut self,
        root: impl AsRef<Path>,
        operation_id: &str,
        plan_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        let preview = self.operation_details(operation_id)?;
        let original = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
            .ok_or_else(|| {
                CoreError::new("invalidState", "Original navigation Plan is unavailable")
            })?;
        if preview.plan_id != plan_id
            || original.request.root != display(&root)
            || !matches!(&original.request.intent, OperationIntent::UpdateFiles { changes } if changes.iter().any(|c| Path::new(&c.path) == root.join(TAG_REGISTRY) || Path::new(&c.path).file_name().is_some_and(|n| n == DIRECTORY_MARKER)))
        {
            return Err(CoreError::new(
                "operationConflict",
                "Cancellation requires the exact navigation Plan",
            ));
        }
        if matches!(preview.status.as_str(), "committed" | "resolved") {
            return Ok(preview);
        }
        if self
            .backend
            .entry_exists(self.journal_path(operation_id, "install"))?
        {
            return Err(CoreError::new(
                "recoveryPending",
                "Inspect the original operation; installation may have effects",
            ));
        }
        self.resolve_record(operation_id, "cancelPrepared")?;
        self.operation_details(operation_id)
    }

    /// A lost or rejected preparation need not be recreated merely to cancel
    /// it. The existing resolution journal fences the original id durably.
    pub fn cancel_navigation_attempt(
        &mut self,
        root: impl AsRef<Path>,
        operation_id: &str,
        plan_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        valid_operation_id(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        self.require_workspace(&root)?;
        if self.operations.contains_key(operation_id) {
            let actual = self.operation_details(operation_id)?;
            let expected = plan_id.unwrap_or(&actual.plan_id);
            let preview = self.cancel_navigation_preparation(&root, operation_id, expected)?;
            return Ok(
                serde_json::json!({"operationId":operation_id,"planId":preview.plan_id,"status":preview.status}),
            );
        }
        if plan_id.is_some()
            || self
                .backend
                .entry_exists(self.journal_path(operation_id, "install"))?
        {
            return Err(CoreError::new(
                "recoveryPending",
                "The original navigation Plan cannot be qualified",
            ));
        }
        self.resolve_record(operation_id, "cancelPrepared")?;
        Ok(serde_json::json!({"operationId":operation_id,"planId":null,"status":"resolved"}))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Core, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        (temp, core, root, data)
    }

    fn commit(core: &mut Core, prepared: &NavigationPreparation) {
        core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
            .unwrap();
    }

    #[test]
    fn directory_admission_is_explicit_and_survives_rename_and_restart() {
        let (_temp, mut core, root, data) = fixture();
        let path = root.join("Container");
        fs::create_dir(&path).unwrap();
        assert_eq!(
            core.capture_navigation_target(&root, &path).unwrap().status,
            "unmanaged"
        );
        let operation_id = fresh();
        let prepared = core
            .prepare_directory_admission(&root, &path, &operation_id)
            .unwrap();
        assert!(!path.join(DIRECTORY_MARKER).exists());
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .status,
            "missing"
        );
        assert_eq!(
            core.prepare_directory_admission(&root, &path, &operation_id)
                .unwrap()
                .target_ref,
            prepared.target_ref
        );
        commit(&mut core, &prepared);
        assert!(!path.join("Container.typ").exists());
        let marker: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join(DIRECTORY_MARKER)).unwrap()).unwrap();
        assert_eq!(marker.as_object().unwrap().len(), 1);
        let renamed = root.join("Moved");
        fs::rename(&path, &renamed).unwrap();
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .path,
            Some(display(&renamed))
        );
        drop(core);
        let core = Core::new(&data).unwrap();
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .path,
            Some(display(&renamed))
        );
    }

    #[test]
    fn library_root_can_be_explicitly_bookmarked_without_creating_a_node() {
        let (_temp, mut core, root, _) = fixture();
        assert_eq!(
            core.capture_navigation_target(&root, &root).unwrap().status,
            "unmanaged"
        );
        let prepared = core
            .prepare_directory_admission(&root, &root, &fresh())
            .unwrap();
        commit(&mut core, &prepared);
        let resolved = core
            .resolve_navigation_target(&root, &prepared.target_ref)
            .unwrap();
        assert_eq!(resolved.status, "managed");
        assert_eq!(resolved.path, Some(display(&root)));
        assert!(!root.join("library.typ").exists());
    }

    #[test]
    fn same_named_replacement_and_copied_marker_cannot_inherit_directory_identity() {
        let (temp, mut core, root, _) = fixture();
        let path = root.join("Container");
        fs::create_dir(&path).unwrap();
        let prepared = core
            .prepare_directory_admission(&root, &path, &fresh())
            .unwrap();
        commit(&mut core, &prepared);
        let bytes = fs::read(path.join(DIRECTORY_MARKER)).unwrap();
        let copied = root.join("Copy");
        fs::create_dir(&copied).unwrap();
        fs::write(copied.join(DIRECTORY_MARKER), &bytes).unwrap();
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .status,
            "identityInvalid"
        );
        fs::rename(&copied, temp.path().join("copy-outside-library")).unwrap();
        fs::rename(&path, temp.path().join("original-outside-library")).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .status,
            "missing"
        );
        fs::write(path.join(DIRECTORY_MARKER), &bytes).unwrap();
        let replacement = core
            .resolve_navigation_target(&root, &prepared.target_ref)
            .unwrap();
        assert_eq!(replacement.status, "identityInvalid");
        assert!(replacement.path.is_none());
    }

    #[test]
    fn replaced_directory_between_prepare_and_commit_is_never_admitted() {
        let (temp, mut core, root, _) = fixture();
        let path = root.join("Container");
        fs::create_dir(&path).unwrap();
        let prepared = core
            .prepare_directory_admission(&root, &path, &fresh())
            .unwrap();
        fs::rename(&path, temp.path().join("original")).unwrap();
        fs::create_dir(&path).unwrap();
        let error = core
            .execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
            .unwrap_err();
        assert!(matches!(
            error.code.as_str(),
            "scopeChanged" | "identityConflict"
        ));
        assert!(!path.join(DIRECTORY_MARKER).exists());
    }

    #[test]
    fn directory_navigation_registration_preserves_explicit_unmanaged_boundary() {
        let (_temp, mut core, root, _) = fixture();
        let path = root.join("PrivateContainer");
        fs::create_dir(&path).unwrap();
        fs::write(
            root.join(".weftext-rules"),
            br#"{"version":1,"ignored":[],"unmanaged":["PrivateContainer"]}"#,
        )
        .unwrap();
        let prepared = core
            .prepare_directory_admission(&root, &path, &fresh())
            .unwrap();
        commit(&mut core, &prepared);
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .status,
            "managed"
        );
        assert_eq!(
            core.check_discovery_boundary(&root, &path)
                .unwrap_err()
                .code,
            "excludedBoundary"
        );
        assert!(!path.join("PrivateContainer.typ").exists());
    }

    #[test]
    fn node_directory_capture_uses_existing_node_ref() {
        let (_temp, mut core, root, _) = fixture();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let captured = core
            .capture_navigation_target(&root, root.join("Note"))
            .unwrap();
        assert_eq!(
            captured.target_ref,
            Some(entity_ref(
                "node",
                note.workspace_id.as_deref().unwrap(),
                note.node_id.as_deref().unwrap()
            ))
        );
        assert_eq!(captured.kind, "node");
        assert!(!root.join("Note").join(DIRECTORY_MARKER).exists());
    }

    #[test]
    fn copied_node_containers_receive_fresh_directory_births() {
        let (_temp, mut core, root, _) = fixture();
        core.create_note_fixture(&root, &root, "Original").unwrap();
        let child = root.join("Original/Container");
        fs::create_dir(&child).unwrap();
        let admitted = core
            .prepare_directory_admission(&root, &child, &fresh())
            .unwrap();
        commit(&mut core, &admitted);
        let copy = core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent: OperationIntent::CopyNode {
                    source: display(&root.join("Original")),
                    parent: display(&root),
                    name: "Copy".into(),
                    allow_losses: false,
                },
            })
            .unwrap();
        core.execute_operation(&copy.operation_id, &copy.plan_id)
            .unwrap();
        let copied = core
            .capture_navigation_target(&root, root.join("Copy/Container"))
            .unwrap();
        assert_eq!(copied.status, "managed");
        assert_ne!(copied.target_ref.as_ref(), Some(&admitted.target_ref));
        assert!(same_path(
            core.resolve_navigation_target(&root, &admitted.target_ref)
                .unwrap()
                .path
                .as_deref()
                .unwrap(),
            &child
        ));
    }

    #[test]
    fn arbitrary_marker_uuid_cannot_be_born_through_generic_file_updates() {
        let (_temp, mut core, root, _) = fixture();
        let path = root.join("Container");
        fs::create_dir(&path).unwrap();
        let marker = path.join(DIRECTORY_MARKER);
        let basis = core
            .read_snapshot(&root, &[display(&marker)])
            .unwrap()
            .files
            .remove(0);
        let error = core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&root),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: display(&marker),
                        base_id: Some(basis.base_id),
                        before: None,
                        after: serde_json::to_vec(&DirectoryMarker {
                            directory_id: fresh(),
                        })
                        .unwrap(),
                    }],
                },
            })
            .unwrap_err();
        assert_eq!(error.code, "identityRequired");
        assert!(!marker.exists());
    }

    #[test]
    fn tag_rename_and_equal_names_preserve_distinct_refs_and_sources() {
        let (_temp, mut core, root, _) = fixture();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let basis = core.read_tag_registry(&root).unwrap();
        let first = core
            .prepare_create_tag(&root, "Research", &basis.base_id, &fresh())
            .unwrap();
        commit(&mut core, &first);
        let tagged_source = format!(
            "#metadata((id: \"{}\", types: (), tags: ((ref: \"{}\"), (text: \"Research\")))) <weftext>\n\nResearch note",
            note.node_id.as_deref().unwrap(),
            first.target_ref
        );
        core.save(SaveRequest {
            session_id: note.session_id,
            base_id: note.base_id,
            source: tagged_source,
            operation_id: fresh(),
        })
        .unwrap();
        let before_source = fs::read(&note.path).unwrap();
        assert_eq!(core.read_tag_registry(&root).unwrap().tags.len(), 1);
        let basis = core.read_tag_registry(&root).unwrap();
        let second = core
            .prepare_create_tag(&root, "Research", &basis.base_id, &fresh())
            .unwrap();
        commit(&mut core, &second);
        assert_ne!(first.target_ref, second.target_ref);
        assert_eq!(core.read_tag_registry(&root).unwrap().tags.len(), 2);
        let basis = core.read_tag_registry(&root).unwrap();
        let rename = core
            .prepare_rename_tag(
                &root,
                &first.target_ref,
                "Archived Research",
                &basis.base_id,
                &fresh(),
            )
            .unwrap();
        commit(&mut core, &rename);
        assert_eq!(rename.target_ref, first.target_ref);
        assert_eq!(
            core.resolve_navigation_target(&root, &first.target_ref)
                .unwrap()
                .label,
            "Archived Research"
        );
        assert_eq!(
            core.resolve_navigation_target(&root, &second.target_ref)
                .unwrap()
                .label,
            "Research"
        );
        assert_eq!(fs::read(&note.path).unwrap(), before_source);
    }

    #[test]
    fn tag_retry_reuses_original_allocation_and_rejects_changed_author_input() {
        let (_temp, mut core, root, data) = fixture();
        let basis = core.read_tag_registry(&root).unwrap();
        let operation_id = fresh();
        let first = core
            .prepare_create_tag(&root, "Research", &basis.base_id, &operation_id)
            .unwrap();
        let retry = core
            .prepare_create_tag(&root, "Research", &basis.base_id, &operation_id)
            .unwrap();
        assert_eq!(first.preview.plan_id, retry.preview.plan_id);
        assert_eq!(first.target_ref, retry.target_ref);
        assert_eq!(
            core.prepare_create_tag(&root, "Changed", &basis.base_id, &operation_id)
                .unwrap_err()
                .code,
            "operationConflict"
        );
        drop(core);
        let mut core = Core::new(&data).unwrap();
        let retry = core
            .prepare_create_tag(&root, "Research", &basis.base_id, &operation_id)
            .unwrap();
        assert_eq!(first.target_ref, retry.target_ref);
        commit(&mut core, &retry);
        assert_eq!(core.read_tag_registry(&root).unwrap().tags.len(), 1);
    }

    #[test]
    fn same_registry_bytes_on_replaced_object_and_unknown_tag_claims_are_unadmitted() {
        let (temp, mut core, root, _) = fixture();
        let basis = core.read_tag_registry(&root).unwrap();
        let prepared = core
            .prepare_create_tag(&root, "Research", &basis.base_id, &fresh())
            .unwrap();
        commit(&mut core, &prepared);
        let registry = root.join(TAG_REGISTRY);
        let bytes = fs::read(&registry).unwrap();
        fs::rename(&registry, temp.path().join("original-registry")).unwrap();
        fs::write(&registry, &bytes).unwrap();
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .status,
            "identityInvalid"
        );
        let registry_snapshot = core.read_tag_registry(&root).unwrap();
        assert!(!registry_snapshot.complete);
        assert!(registry_snapshot.tags.is_empty());
        let workspace = core.require_workspace(&root).unwrap();
        let fabricated = fresh();
        fs::write(
            &registry,
            serde_json::to_vec(&TagDocument {
                version: 2,
                tags: vec![TagDefinition {
                    tag_id: fabricated.clone(),
                    parent_tag_id: None,
                    display_name: "Research".into(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            core.resolve_navigation_target(
                &root,
                &entity_ref("tag", &workspace.workspace_id, &fabricated)
            )
            .unwrap()
            .status,
            "identityInvalid"
        );
        assert_eq!(
            core.resolve_navigation_target(&root, &prepared.target_ref)
                .unwrap()
                .status,
            "identityInvalid"
        );
    }

    fn tag(core: &mut Core, root: &Path, label: &str, parent: Option<&str>) -> String {
        let basis = core.read_tag_registry(root).unwrap();
        let prepared = core
            .prepare_create_tag_under(root, label, parent, &basis.base_id, &fresh())
            .unwrap();
        commit(core, &prepared);
        prepared.target_ref
    }

    #[test]
    fn hierarchy_move_preserves_direct_references_and_refuses_cycles_and_foreign_parents() {
        let (_temp, mut core, root, state) = fixture();
        let a = tag(&mut core, &root, "同名/name", None);
        let b = tag(&mut core, &root, "同名/name", Some(&a));
        let c = tag(&mut core, &root, "leaf", Some(&b));
        let before = fs::read(root.join(TAG_REGISTRY)).unwrap();
        let basis = core.read_tag_registry(&root).unwrap();
        assert!(
            core.prepare_move_tag(&root, &a, Some(&c), &basis.base_id, &fresh())
                .is_err()
        );
        assert!(
            core.prepare_move_tag(&root, &a, Some(&a), &basis.base_id, &fresh())
                .is_err()
        );
        let foreign = entity_ref("tag", &fresh(), &fresh());
        assert!(
            core.prepare_create_tag_under(
                &root,
                "foreign",
                Some(&foreign),
                &basis.base_id,
                &fresh()
            )
            .is_err()
        );
        assert_eq!(fs::read(root.join(TAG_REGISTRY)).unwrap(), before);
        let moved = core
            .prepare_move_tag(&root, &b, None, &basis.base_id, &fresh())
            .unwrap();
        commit(&mut core, &moved);
        drop(core);
        let mut core = Core::new(state).unwrap();
        let actual = core.read_tag_registry(&root).unwrap();
        assert!(actual.complete);
        assert!(
            actual
                .tags
                .iter()
                .find(|t| t.tag_ref == b)
                .unwrap()
                .parent_tag_id
                .is_none()
        );
        assert_eq!(
            actual
                .tags
                .iter()
                .find(|t| t.tag_ref == c)
                .unwrap()
                .parent_tag_id
                .as_deref(),
            Some(parse_ref(&b).unwrap().2)
        );
        assert_eq!(
            actual
                .tags
                .iter()
                .find(|t| t.tag_ref == a)
                .unwrap()
                .display_name,
            "同名/name"
        );
    }

    #[test]
    fn competing_reparents_use_whole_object_cas_and_revalidate_the_forest() {
        let (_temp, mut core, root, _) = fixture();
        let a = tag(&mut core, &root, "A", None);
        let b = tag(&mut core, &root, "B", None);
        let basis = core.read_tag_registry(&root).unwrap();
        let first = core
            .prepare_move_tag(&root, &a, Some(&b), &basis.base_id, &fresh())
            .unwrap();
        let second = core
            .prepare_move_tag(&root, &b, Some(&a), &basis.base_id, &fresh())
            .unwrap();
        commit(&mut core, &first);
        assert!(
            core.execute_operation(&second.preview.operation_id, &second.preview.plan_id)
                .is_err()
        );
        let basis = core.read_tag_registry(&root).unwrap();
        assert!(
            core.prepare_move_tag(&root, &b, Some(&a), &basis.base_id, &fresh())
                .is_err()
        );
        assert_eq!(
            core.operation_details(&second.preview.operation_id)
                .unwrap()
                .plan_id,
            second.preview.plan_id
        );
    }

    #[test]
    fn generic_tag_writes_cannot_allocate_or_downgrade_and_missing_registry_cannot_rebirth() {
        let (temp, mut core, root, _) = fixture();
        let a = tag(&mut core, &root, "A", None);
        let basis = core.read_tag_registry(&root).unwrap();
        let before = fs::read(root.join(TAG_REGISTRY)).unwrap();
        let old = serde_json::json!({"version":1,"tags":[{"tagId":parse_ref(&a).unwrap().2,"displayName":"A"}]});
        let forged = serde_json::json!({"version":2,"tags":[{"tagId":parse_ref(&a).unwrap().2,"displayName":"A","parentTagId":null},{"tagId":fresh(),"displayName":"forged","parentTagId":null}]});
        for after in [old, forged] {
            assert!(
                core.prepare_operation(OperationRequest {
                    operation_id: fresh(),
                    root: display(&root),
                    intent: OperationIntent::UpdateFiles {
                        changes: vec![FileChange {
                            path: display(&root.join(TAG_REGISTRY)),
                            base_id: Some(basis.base_id.clone()),
                            before: Some(before.clone()),
                            after: serde_json::to_vec(&after).unwrap()
                        }]
                    }
                })
                .is_err()
            );
        }
        fs::rename(
            root.join(TAG_REGISTRY),
            temp.path().join("retained-original-tags"),
        )
        .unwrap();
        assert!(core.read_tag_registry(&root).is_err());
        let basis = core
            .read_snapshot(&root, &[display(&root.join(TAG_REGISTRY))])
            .unwrap()
            .files
            .remove(0);
        assert!(
            core.prepare_create_tag(&root, "replacement", &basis.base_id, &fresh())
                .is_err()
        );
        assert!(!root.join(TAG_REGISTRY).exists());
    }

    #[test]
    fn cancelled_tag_plan_stays_cancelled_across_restart_and_late_commit() {
        let (_temp, mut core, root, state) = fixture();
        let basis = core.read_tag_registry(&root).unwrap();
        let prepared = core
            .prepare_create_tag(&root, "cancelled", &basis.base_id, &fresh())
            .unwrap();
        core.cancel_navigation_preparation(
            &root,
            &prepared.preview.operation_id,
            &prepared.preview.plan_id,
        )
        .unwrap();
        drop(core);
        let mut core = Core::new(state).unwrap();
        assert!(
            core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
                .is_err()
        );
        assert!(core.read_tag_registry(&root).unwrap().tags.is_empty());
        assert!(!root.join(TAG_REGISTRY).exists());
    }

    #[test]
    fn rejected_or_lost_navigation_preparation_can_be_fenced_without_repreparing() {
        let (_temp, mut core, root, state) = fixture();
        let op = fresh();
        let tags = core.read_tag_registry(&root).unwrap();
        let invalid = core.prepare_create_tag_with_assignment(
            &root,
            "retained input",
            None,
            &tags.base_id,
            Some(TagAssignment {
                path: display(&root.join("missing.typ")),
                base_id: fresh(),
                field_id: "core/tags".into(),
            }),
            &op,
        );
        assert_eq!(invalid.unwrap_err().code, "staleBase");
        let cancelled = core.cancel_navigation_attempt(&root, &op, None).unwrap();
        assert_eq!(cancelled["status"], "resolved");
        assert!(cancelled["planId"].is_null());
        drop(core);
        let mut core = Core::new(state).unwrap();
        let tags = core.read_tag_registry(&root).unwrap();
        assert_eq!(
            core.prepare_create_tag(&root, "late", &tags.base_id, &op)
                .unwrap_err()
                .code,
            "operationCancelled"
        );
        assert!(!root.join(TAG_REGISTRY).exists());
        assert_eq!(
            core.cancel_navigation_attempt(&root, &op, None).unwrap(),
            cancelled
        );
        let note = core
            .create_note_fixture(&root, &root, "CannotReuseCancelledId")
            .unwrap();
        assert_eq!(
            core.save(SaveRequest {
                session_id: note.session_id,
                base_id: note.base_id,
                source: format!("{}\nlate different intent\n", note.source),
                operation_id: op
            })
            .unwrap_err()
            .code,
            "operationCancelled"
        );
        assert_eq!(fs::read_to_string(&note.path).unwrap(), note.source);
        let prepared = core
            .prepare_create_tag(&root, "lost response", &tags.base_id, &fresh())
            .unwrap();
        let reply = core
            .cancel_navigation_attempt(&root, &prepared.preview.operation_id, None)
            .unwrap();
        assert_eq!(reply["planId"], prepared.preview.plan_id);
        assert_eq!(reply["status"], "resolved");
        assert!(
            core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
                .is_err()
        );
    }

    #[test]
    fn tag_creation_and_first_assignment_share_original_plan_and_recover_after_restart() {
        let (_temp, mut core, root, state) = fixture();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        core.save(SaveRequest {
            session_id: note.session_id.clone(),
            base_id: note.base_id.clone(),
            source: note
                .source
                .replace("types: ()", "types: (\"task\",), status: \"open\""),
            operation_id: fresh(),
        })
        .unwrap();
        let source = core
            .read_snapshot(&root, std::slice::from_ref(&note.path))
            .unwrap()
            .files
            .remove(0);
        let schema = weftext_data::Schema::bundled().unwrap();
        let field = schema
            .fields
            .iter()
            .find(|f| f.key == "tags")
            .unwrap()
            .id
            .clone();
        let basis = core.read_tag_registry(&root).unwrap();
        let op = fresh();
        let prepared = core
            .prepare_create_tag_with_assignment(
                &root,
                "created and assigned",
                None,
                &basis.base_id,
                Some(TagAssignment {
                    path: note.path.clone(),
                    base_id: source.base_id,
                    field_id: field,
                }),
                &op,
            )
            .unwrap();
        assert_eq!(fs::read(&note.path).unwrap(), source.bytes.unwrap());
        assert!(!root.join(TAG_REGISTRY).exists());
        drop(core);
        let mut core = Core::new(state).unwrap();
        commit(&mut core, &prepared);
        assert!(
            fs::read_to_string(&note.path)
                .unwrap()
                .contains(&prepared.target_ref)
        );
        assert_eq!(
            core.read_tag_registry(&root).unwrap().tags[0].tag_ref,
            prepared.target_ref
        );
        assert!(!root.join(".weftext-schema.json").exists());
        assert_eq!(core.operation_details(&op).unwrap().status, "committed");
    }

    #[test]
    fn first_assignment_qualifies_the_original_local_session_and_rejects_changed_bytes() {
        let (_temp, mut core, root, _state) = fixture();
        let note = core
            .create_note_fixture(&root, &root, "SessionNote")
            .unwrap();
        let source = note
            .source
            .replace("types: ()", "types: (\"task\",), status: \"open\"");
        let saved = core
            .save(SaveRequest {
                session_id: note.session_id.clone(),
                base_id: note.base_id,
                source: source.clone(),
                operation_id: fresh(),
            })
            .unwrap();
        let assignment = TagAssignment {
            path: note.path.clone(),
            base_id: saved.receipt.base_id,
            field_id: "core/tags".into(),
        };
        let tags = core.read_tag_registry(&root).unwrap();
        // A different actor cannot borrow the local session's original basis.
        core.with_actor_for_root("another-actor", &root, |core| {
            assert_eq!(
                core.prepare_create_tag_with_assignment(
                    &root,
                    "no",
                    None,
                    &tags.base_id,
                    Some(assignment.clone()),
                    &fresh()
                )
                .unwrap_err()
                .code,
                "staleBase"
            );
            Ok(())
        })
        .unwrap();
        let prepared = core
            .prepare_create_tag_with_assignment(
                &root,
                "from session",
                None,
                &tags.base_id,
                Some(assignment),
                &fresh(),
            )
            .unwrap();
        commit(&mut core, &prepared);
        assert!(
            fs::read_to_string(&note.path)
                .unwrap()
                .contains(&prepared.target_ref)
        );
        let other = core
            .create_note_fixture(&root, &root, "ChangedNote")
            .unwrap();
        let tags = core.read_tag_registry(&root).unwrap();
        let registry = fs::read(root.join(TAG_REGISTRY)).unwrap();
        fs::write(&other.path, format!("{}\nexternal change\n", other.source)).unwrap();
        assert_eq!(
            core.prepare_create_tag_with_assignment(
                &root,
                "no",
                None,
                &tags.base_id,
                Some(TagAssignment {
                    path: other.path,
                    base_id: other.base_id,
                    field_id: "core/tags".into(),
                }),
                &fresh()
            )
            .unwrap_err()
            .code,
            "staleBase"
        );
        assert_eq!(fs::read(root.join(TAG_REGISTRY)).unwrap(), registry);
    }

    #[test]
    fn historical_v1_decode_preserves_roots_but_current_successor_requires_explicit_parents() {
        let raw = serde_json::to_vec(
            &serde_json::json!({"version":1,"tags":[{"tagId":fresh(),"displayName":"old"}]}),
        )
        .unwrap();
        let doc = decode_historical_tag_document(&raw).unwrap();
        assert_eq!(doc.version, 2);
        assert!(doc.tags[0].parent_tag_id.is_none());
        assert!(validate_sync_tags(&raw).is_err());
        let missing = serde_json::to_vec(
            &serde_json::json!({"version":2,"tags":[{"tagId":fresh(),"displayName":"bad"}]}),
        )
        .unwrap();
        assert!(decode_historical_tag_document(&missing).is_err());
    }
}
