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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRecord {
    pub tag_ref: String,
    pub tag_id: String,
    pub display_name: String,
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
    root: String,
    workspace_id: String,
    tag_id: String,
    registry_identity: Option<FileIdentity>,
    fingerprint: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TagDocument {
    version: u32,
    tags: Vec<TagDefinition>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TagDefinition {
    tag_id: String,
    display_name: String,
}

struct TagWriteBasis {
    document: TagDocument,
    bytes: Option<Vec<u8>>,
    identity: Option<FileIdentity>,
}

pub(crate) fn validate_directory_marker(bytes: &[u8]) -> Result<()> {
    let marker: DirectoryMarker = serde_json::from_slice(bytes)?;
    valid_operation_id(&marker.directory_id)
}

fn tag_document(bytes: Option<&[u8]>) -> Result<TagDocument> {
    let Some(bytes) = bytes else {
        return Ok(TagDocument {
            version: 1,
            tags: vec![],
        });
    };
    let doc: TagDocument = serde_json::from_slice(bytes)?;
    if doc.version != 1 || doc.tags.len() > 5000 {
        return Err(CoreError::new(
            "tagRegistry",
            "Unsupported or oversized tag registry",
        ));
    }
    let mut ids = BTreeSet::new();
    for tag in &doc.tags {
        valid_operation_id(&tag.tag_id)?;
        validate_tag_name(&tag.display_name)?;
        if !ids.insert(&tag.tag_id) {
            return Err(CoreError::new(
                "duplicateTag",
                "Duplicate TagId claims are ambiguous",
            ));
        }
    }
    Ok(doc)
}

pub(crate) fn validate_sync_tags(bytes: &[u8]) -> Result<()> {
    tag_document(Some(bytes)).map(|_| ())
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

fn marker_claim(path: &Path) -> Result<Option<(DirectoryMarker, FileIdentity)>> {
    let marker_path = path.join(DIRECTORY_MARKER);
    let guard = ParentGuard::acquire(&marker_path)?;
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
        let guard = ParentGuard::acquire(path)?;
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
        match fs::read(self.journal_path(operation_id, suffix)) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
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
        for plan in self
            .operations
            .values()
            .filter(|plan| self.committed(&plan.operation_id))
        {
            if plan.workspace_id.as_deref() == Some(workspace)
                && let Some(transaction) = plan.transaction.as_ref()
                && matches!(
                    transaction.request.intent,
                    OperationIntent::CopyNode { .. }
                        | OperationIntent::TemplateNode { .. }
                        | OperationIntent::CloneSync { .. }
                        | OperationIntent::ApplySync { .. }
                        | OperationIntent::MergeSync { .. }
                )
                && let operations::Mutation::CreateTree { files, .. } = &transaction.mutation
            {
                for file in files.iter().filter(|file| {
                    Path::new(&file.path)
                        .file_name()
                        .is_some_and(|name| name == DIRECTORY_MARKER)
                }) {
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
    pub(crate) fn directory_admission_candidates(
        &self,
        workspace: &str,
    ) -> Result<Vec<(String, String, FileIdentity)>> {
        let mut candidates = Vec::new();
        for p in self.operations.values().filter(|p| {
            self.committed(&p.operation_id) && p.workspace_id.as_deref() == Some(workspace)
        }) {
            if let Some(r) = self.read_navigation_record::<DirectoryReservation>(
                &p.operation_id,
                "directory-reservation",
            )? {
                candidates.push((r.path, r.directory_id, r.identity));
            }
            if let Some(t) = &p.transaction
                && let operations::Mutation::CreateTree { files, .. } = &t.mutation
            {
                for f in files.iter().filter(|f| {
                    Path::new(&f.path)
                        .file_name()
                        .is_some_and(|n| n == DIRECTORY_MARKER)
                }) {
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

    fn tag_admitted(
        &self,
        root: &Path,
        workspace: &str,
        id: &str,
        identity: &FileIdentity,
    ) -> Result<bool> {
        for plan in self
            .operations
            .values()
            .filter(|plan| self.committed(&plan.operation_id))
        {
            if plan.workspace_id.as_deref() == Some(workspace)
                && let Some(transaction) = &plan.transaction
                && matches!(
                    transaction.request.intent,
                    OperationIntent::CloneSync { .. }
                        | OperationIntent::ApplySync { .. }
                        | OperationIntent::MergeSync { .. }
                )
                && let operations::Mutation::CreateTree { files, .. } = &transaction.mutation
            {
                let registry = root.join(TAG_REGISTRY);
                if self
                    .materialized_identity(&plan.operation_id, &registry)?
                    .as_ref()
                    == Some(identity)
                    && files.iter().any(|f| {
                        f.path == display(&registry)
                            && tag_document(Some(&f.after))
                                .is_ok_and(|doc| doc.tags.iter().any(|tag| tag.tag_id == id))
                    })
                {
                    return Ok(true);
                }
            }
            if plan.workspace_id.as_deref() == Some(workspace)
                && plan
                    .sync_application
                    .as_ref()
                    .is_some_and(|a| a.root == display(root))
                && let Some(transaction) = &plan.transaction
            {
                let updates = match &transaction.mutation {
                    operations::Mutation::Update { files } => files,
                    operations::Mutation::CreateTree { updates, .. }
                    | operations::Mutation::Relocate { updates, .. }
                    | operations::Mutation::Compound { updates, .. } => updates,
                };
                let registry = display(&root.join(TAG_REGISTRY));
                if transaction
                    .files
                    .iter()
                    .any(|f| f.path == registry && f.stamp.identity == *identity)
                    && updates.iter().any(|f| {
                        f.path == registry
                            && tag_document(Some(&f.after))
                                .is_ok_and(|doc| doc.tags.iter().any(|tag| tag.tag_id == id))
                    })
                {
                    return Ok(true);
                }
            }
            let Some(reservation) = self
                .read_navigation_record::<TagReservation>(&plan.operation_id, "tag-reservation")?
            else {
                continue;
            };
            if reservation.workspace_id != workspace
                || reservation.tag_id != id
                || reservation.root != display(root)
                || reservation.fingerprint != plan.fingerprint
            {
                continue;
            }
            let registry = root.join(TAG_REGISTRY);
            let born_identity = match reservation.registry_identity {
                Some(identity) => Some(identity),
                None => self.materialized_identity(&plan.operation_id, &registry)?,
            };
            if born_identity.as_ref() != Some(identity) {
                continue;
            }
            let Some(transaction) = plan.transaction.as_ref() else {
                continue;
            };
            if let OperationIntent::UpdateFiles { changes } = &transaction.request.intent
                && changes.iter().any(|change| {
                    change.path == display(&registry)
                        && tag_document(Some(&change.after))
                            .is_ok_and(|doc| doc.tags.iter().any(|tag| tag.tag_id == id))
                })
            {
                return Ok(true);
            }
        }
        Ok(false)
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
        match marker_claim(&path) {
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
            let guard = ParentGuard::acquire(&registry)?;
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
            let identity = stamp(&file)?.identity;
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
            let Some(tag) = doc.tags.iter().find(|tag| tag.tag_id == id) else {
                return Ok(target(
                    kind,
                    "missing",
                    Some(reference.into()),
                    None,
                    String::new(),
                ));
            };
            if !self.tag_admitted(&root, ref_workspace, id, &identity)? {
                return Ok(target(
                    kind,
                    "identityInvalid",
                    Some(reference.into()),
                    None,
                    tag.display_name.clone(),
                ));
            }
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
            if let Some((marker, identity)) = marker_claim(&path)?
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
            Some("renameTag") => canonical["tagRef"]
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
        let guard = ParentGuard::acquire(&marker_path)?;
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
                root: display(&root),
                path: display(&path),
                workspace_id: workspace.workspace_id,
                directory_id: fresh(),
                identity,
                fingerprint,
            };
            journal::private_create(
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
        for tag in doc.tags {
            if let Some(identity) = &identity
                && self.tag_admitted(&root, &workspace.workspace_id, &tag.tag_id, identity)?
            {
                tags.push(TagRecord {
                    tag_ref: entity_ref("tag", &workspace.workspace_id, &tag.tag_id),
                    tag_id: tag.tag_id,
                    display_name: tag.display_name,
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
        let doc = tag_document(before.as_deref())?;
        let workspace = self.require_workspace(root)?;
        for tag in &doc.tags {
            let admitted = match &identity {
                Some(identity) => {
                    self.tag_admitted(root, &workspace.workspace_id, &tag.tag_id, identity)?
                }
                None => false,
            };
            if !admitted {
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

    pub fn prepare_create_tag(
        &mut self,
        root: impl AsRef<Path>,
        display_name: &str,
        base_id: &str,
        operation_id: &str,
    ) -> Result<NavigationPreparation> {
        valid_operation_id(operation_id)?;
        validate_tag_name(display_name)?;
        let root = absolute_path(root.as_ref())?;
        let canonical = serde_json::json!({"kind":"createTag","root":display(&root),"displayName":display_name,"baseId":base_id});
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
                root: display(&root),
                workspace_id: workspace.workspace_id,
                tag_id: fresh(),
                registry_identity,
                fingerprint,
            };
            journal::private_create(
                &self.journal_path(operation_id, "tag-reservation"),
                &serde_json::to_vec(&new)?,
            )?;
            new
        };
        doc.tags.push(TagDefinition {
            tag_id: reservation.tag_id.clone(),
            display_name: display_name.into(),
        });
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
        let note = core.create_note(&root, &root, "Note").unwrap();
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
        core.create_note(&root, &root, "Original").unwrap();
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
        let note = core.create_note(&root, &root, "Note").unwrap();
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
                version: 1,
                tags: vec![TagDefinition {
                    tag_id: fabricated.clone(),
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
            "missing"
        );
    }
}
