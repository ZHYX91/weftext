//! Local replica binding and publication outbox. The original Core plan stores
//! the exact portable result before author writes; its existing commit marker
//! decides whether that result may be published. Transport failures cannot undo
//! a saved author operation, and retries cannot allocate another causal branch.
use super::*;
use crate::{
    Core, Path, absolute_path, display, fresh,
    journal::Plan,
    operations::{FileChange, Mutation},
    valid_operation_id,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalObject {
    pub object: ObjectKey,
    pub directory: String,
    pub commit: String,
    pub revision: String,
    pub state: ObjectState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncSetup {
    pub root: String,
    pub replica_id: String,
    pub identity: SyncIdentity,
    pub initial: Vec<LocalObject>,
    pub bundle: SyncBundle,
    #[serde(default)]
    pub selection: Option<BTreeSet<ObjectKey>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncPublication {
    pub root: String,
    pub identity: SyncIdentity,
    pub manifest: Manifest,
    pub blobs: BTreeMap<String, Vec<u8>>,
    /// Private physical locators, never part of the portable manifest.
    pub locations: Vec<(ObjectKey, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncBindingInfo {
    pub root: String,
    pub identity: SyncIdentity,
    pub replica_id: String,
    pub pending_publications: usize,
    pub partial: bool,
    pub selected_objects: Option<BTreeSet<ObjectKey>>,
}

impl Core {
    pub fn enable_sync(
        &mut self,
        root: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<SyncBindingInfo> {
        self.sync_local_owner()?;
        valid_operation_id(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let fingerprint = hash(&serde_json::to_vec(
            &serde_json::json!({"kind":"enableSync","root":display(&root),"workspace":workspace.workspace_id}),
        )?);
        if let Some(plan) = self.operations.get(operation_id).cloned() {
            if plan.fingerprint != fingerprint
                || plan.sync_setup.is_none()
                || self.resolved(operation_id)
            {
                return Err(CoreError::new(
                    "operationConflict",
                    "This operation does not name the same sync setup",
                ));
            }
            let setup = plan.sync_setup.as_ref().unwrap();
            // Setup has no author-tree writes; retry its exact retained bytes.
            self.receive_sync(&setup.identity, setup.bundle.clone())?;
            if !self.committed(operation_id) {
                self.commit(&plan)?;
            }
            return self.sync_binding_info(&root);
        }
        if self
            .sync_setups()
            .iter()
            .any(|s| s.root == display(&root) || s.identity.workspace_id == workspace.workspace_id)
        {
            return Err(CoreError::new(
                "syncAlreadyEnabled",
                "Use the existing replica binding and original operation",
            ));
        }
        self.check_sync_setup_pending(&workspace.workspace_id)?;
        let captured = self.admitted_compile_snapshot(&root, "live")?;
        let trash = self.trash_admitted_snapshot(&root)?;
        if !captured.snapshot.complete || !trash.complete {
            return Err(CoreError::new(
                "coverageIncomplete",
                "Sync setup requires a qualified content inventory",
            ));
        }
        let replica_id = fresh();
        let genesis = Manifest {
            version: 1,
            workspace_id: workspace.workspace_id.clone(),
            replica_id: replica_id.clone(),
            operation_id: operation_id.into(),
            parents: BTreeSet::new(),
            changes: vec![],
            dependencies: vec![],
        };
        let identity = SyncIdentity {
            workspace_id: workspace.workspace_id.clone(),
            genesis: genesis.id()?,
        };
        let mut bundle = SyncBundle {
            manifests: BTreeMap::from([(identity.genesis.clone(), genesis)]),
            blobs: BTreeMap::new(),
        };
        let mut objects = BTreeMap::new();
        for source in captured
            .snapshot
            .files
            .iter()
            .chain(trash.files.iter())
            .filter(|f| f.node_id.is_some())
        {
            let path = Path::new(&source.path);
            let directory = path.parent().unwrap();
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let object = ObjectKey::Node(source.node_id.clone().unwrap());
            if objects.contains_key(&object) {
                return Err(invalid("Duplicate Node in replica setup"));
            }
            objects.insert(
                object.clone(),
                LocalObject {
                    object,
                    directory: display(directory),
                    commit: String::new(),
                    revision: fresh(),
                    state: ObjectState {
                        parent: None,
                        name,
                        retired: directory.starts_with(root.join(".weftext-trash")),
                        files: BTreeMap::new(),
                    },
                },
            );
        }
        // Directory identity must already be admitted; a copied marker is not
        // made authoritative merely because it is present in this capture.
        for (relative, bytes) in &captured.files {
            let path = root.join(relative);
            if path
                .file_name()
                .is_some_and(|n| n == "weftext.directory.json")
                && !objects
                    .values()
                    .any(|o| Path::new(&o.directory) == path.parent().unwrap())
            {
                let value: serde_json::Value = serde_json::from_slice(bytes)?;
                let id = value["directoryId"]
                    .as_str()
                    .ok_or_else(|| invalid("Directory marker lacks an identity"))?;
                let reference = format!("weftext-directory://{}/{id}", workspace.workspace_id);
                let resolved = self.resolve_navigation_target(&root, &reference)?;
                if resolved.status != "managed"
                    || resolved.path.as_deref() != Some(&display(path.parent().unwrap()))
                {
                    return Err(invalid("A directory marker is not currently admitted"));
                }
                let object = if path.parent() == Some(&root) {
                    ObjectKey::RootDirectory(id.into())
                } else {
                    ObjectKey::Directory(id.into())
                };
                objects.insert(
                    object.clone(),
                    LocalObject {
                        object,
                        directory: display(path.parent().unwrap()),
                        commit: String::new(),
                        revision: fresh(),
                        state: ObjectState {
                            parent: None,
                            name: path
                                .parent()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .into(),
                            retired: false,
                            files: BTreeMap::new(),
                        },
                    },
                );
            }
        }
        for (relative, bytes) in &captured.files {
            let path = root.join(relative);
            if let Some(object) = config_key(&root, &path) {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                objects.insert(
                    object.clone(),
                    LocalObject {
                        object,
                        directory: display(&root),
                        commit: String::new(),
                        revision: fresh(),
                        state: ObjectState {
                            parent: None,
                            name,
                            retired: false,
                            files: BTreeMap::new(),
                        },
                    },
                );
            }
            if let Some(object) = owner(&objects, &root, &path) {
                let tracked = objects.get_mut(&object).unwrap();
                let name = relative_to(&path, Path::new(&tracked.directory))?;
                super::relative(&name)?;
                let blob = hash(bytes);
                tracked.state.files.insert(name, blob.clone());
                bundle.blobs.insert(blob, bytes.clone());
            }
        }
        let parent_lookup = objects.clone();
        for tracked in objects.values_mut() {
            if matches!(tracked.object, ObjectKey::Node(_) | ObjectKey::Directory(_))
                && !tracked.state.retired
            {
                tracked.state.parent = parent_id(
                    &parent_lookup,
                    &root,
                    Path::new(&tracked.directory).parent().unwrap(),
                )?;
            }
            if let ObjectKey::Node(id) = &tracked.object {
                let canonical = format!("{}.typ", tracked.state.name);
                let blob =
                    tracked.state.files.get(&canonical).ok_or_else(|| {
                        invalid("Canonical source is absent from the captured node")
                    })?;
                if weftext_engine::inspect(&crate::decode(&bundle.blobs[blob])?)
                    .id
                    .as_ref()
                    != Some(id)
                {
                    return Err(invalid("Captured source changed identity"));
                }
            }
        }
        // Initial checkpoint records use only real parent/dependency edges, so
        // choosing one note does not require downloading unrelated note bodies.
        let mut initial: Vec<_> = objects.into_values().collect();
        initial.sort_by_key(|o| Path::new(&o.directory).components().count());
        let mut birth_commits = BTreeMap::new();
        for tracked in &mut initial {
            let mut parents = BTreeSet::from([identity.genesis.clone()]);
            if let Some(parent) = &tracked.state.parent {
                parents.insert(
                    birth_commits
                        .get(parent)
                        .cloned()
                        .ok_or_else(|| invalid("Missing initial parent checkpoint"))?,
                );
            }
            let manifest = Manifest {
                version: 1,
                workspace_id: workspace.workspace_id.clone(),
                replica_id: replica_id.clone(),
                operation_id: fresh(),
                parents,
                changes: vec![Change {
                    object: tracked.object.clone(),
                    before: BTreeSet::new(),
                    revision: tracked.revision.clone(),
                    after: tracked.state.clone(),
                }],
                dependencies: vec![],
            };
            tracked.commit = manifest.id()?;
            birth_commits.insert(tracked.object.clone(), tracked.commit.clone());
            bundle.manifests.insert(tracked.commit.clone(), manifest);
        }
        let setup = SyncSetup {
            root: display(&root),
            replica_id,
            identity,
            initial,
            bundle,
            selection: None,
        };
        let mut plan = Plan::file(
            operation_id.into(),
            fingerprint,
            &root,
            None,
            vec![],
            None,
            "enableSync",
        );
        plan.workspace_id = Some(workspace.workspace_id);
        plan.sync_setup = Some(setup.clone());
        self.stage(plan.clone())?;
        self.receive_sync(&setup.identity, setup.bundle)?;
        self.commit(&plan)?;
        self.sync_binding_info(&root)
    }

    pub fn sync_binding_info(&self, root: impl AsRef<Path>) -> Result<SyncBindingInfo> {
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let setup = self.sync_setup_for(&root)?;
        let graph = self.read_sync_history(&setup.identity)?;
        let mut pending = 0;
        for publication in self.sync_publications(&setup.root)? {
            if !matches!(
                graph.status(&publication.manifest.id()?),
                Ok(Admission::Accepted)
            ) {
                pending += 1;
            }
        }
        Ok(SyncBindingInfo {
            root: setup.root.clone(),
            identity: setup.identity.clone(),
            replica_id: setup.replica_id.clone(),
            pending_publications: pending,
            partial: setup.selection.is_some(),
            selected_objects: setup.selection.clone(),
        })
    }

    /// Publication drains original committed plans, never the latest filesystem
    /// bytes. Later edits cannot alter a pending earlier operation's transport.
    pub fn publish_sync(&self, root: impl AsRef<Path>) -> Result<SyncStatus> {
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let setup = self.sync_setup_for(&root)?;
        let mut bundle = SyncBundle::default();
        for publication in self.sync_publications(&setup.root)? {
            bundle
                .manifests
                .insert(publication.manifest.id()?, publication.manifest.clone());
            bundle.blobs.extend(
                publication
                    .blobs
                    .iter()
                    .map(|(id, bytes)| (id.clone(), bytes.clone())),
            );
        }
        self.receive_sync(&setup.identity, bundle)
    }

    pub(crate) fn sync_local_owner(&self) -> Result<()> {
        if self.actor.is_some() {
            Err(CoreError::new(
                "syncOwnerRequired",
                "Replica management requires the local owner",
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) fn check_sync_setup_pending(&self, workspace: &str) -> Result<()> {
        if self.operations.values().any(|p| {
            p.sync_setup
                .as_ref()
                .is_some_and(|s| s.identity.workspace_id == workspace)
                && !self.committed(&p.operation_id)
                && !self.resolved(&p.operation_id)
        }) {
            return Err(CoreError::new(
                "syncSetupPending",
                "Resume or resolve the original replica setup before allocating another",
            ));
        }
        Ok(())
    }
    pub(crate) fn sync_setups(&self) -> Vec<&SyncSetup> {
        self.operations
            .values()
            .filter_map(|p| {
                p.sync_setup
                    .as_ref()
                    .filter(|_| self.committed(&p.operation_id))
            })
            .collect()
    }
    fn sync_setup_for(&self, root: &Path) -> Result<&SyncSetup> {
        let matches: Vec<_> = self
            .sync_setups()
            .into_iter()
            .filter(|s| Path::new(&s.root) == root)
            .collect();
        if matches.len() != 1 {
            return Err(CoreError::new(
                "syncNotEnabled",
                "This root does not have one committed replica binding",
            ));
        }
        Ok(matches.into_iter().next().unwrap())
    }
    fn sync_publications(&self, root: &str) -> Result<Vec<&SyncPublication>> {
        let mut records = Vec::new();
        for plan in self.operations.values() {
            if let Some(publication) = &plan.sync_publication
                && publication.root == root
                && self.committed(&plan.operation_id)
            {
                let fact: serde_json::Value = serde_json::from_slice(&std::fs::read(
                    self.journal_path(&plan.operation_id, "commit"),
                )?)?;
                let sequence = fact["commitSequence"]
                    .as_u64()
                    .ok_or_else(|| invalid("Publication lacks its durable commit order"))?;
                records.push((sequence, publication));
            }
        }
        records.sort_by_key(|r| r.0);
        Ok(records.into_iter().map(|r| r.1).collect())
    }

    pub(crate) fn freeze_sync_publication(&self, plan: &mut Plan) -> Result<()> {
        if plan.sync_setup.is_some() || plan.sync_publication.is_some() {
            return Ok(());
        }
        if plan.transaction.as_ref().is_some_and(|t| {
            matches!(
                t.request.intent,
                crate::OperationIntent::BackupLibrary { .. }
            )
        }) {
            return Ok(());
        }
        let matches: Vec<_> = self
            .sync_setups()
            .into_iter()
            .filter(|s| {
                plan.transaction.as_ref().map_or_else(
                    || Path::new(&plan.path).starts_with(&s.root),
                    |t| t.request.root == s.root,
                )
            })
            .collect();
        if matches.is_empty() {
            return Ok(());
        }
        if matches.len() != 1 {
            return Err(invalid("Author operation crosses replica bindings"));
        }
        let setup = &matches[0];
        let root = Path::new(&setup.root);
        // Hosted save publication is internal writer responsibility; it does
        // not expose any history or transport bytes to the authenticated actor.
        let mut objects: BTreeMap<_, _> = setup
            .initial
            .iter()
            .cloned()
            .map(|o| (o.object.clone(), o))
            .collect();
        let mut known_blobs: BTreeMap<_, _> = setup
            .bundle
            .blobs
            .iter()
            .map(|(id, bytes)| (id.clone(), bytes))
            .collect();
        for prior in self.sync_publications(&setup.root)? {
            let commit = prior.manifest.id()?;
            for c in &prior.manifest.changes {
                let location = prior
                    .locations
                    .iter()
                    .find(|(key, _)| key == &c.object)
                    .map(|(_, p)| p.clone())
                    .or_else(|| objects.get(&c.object).map(|o| o.directory.clone()))
                    .ok_or_else(|| invalid("Publication has no local object locator"))?;
                objects.insert(
                    c.object.clone(),
                    LocalObject {
                        object: c.object.clone(),
                        directory: location,
                        commit: commit.clone(),
                        revision: c.revision.clone(),
                        state: c.after.clone(),
                    },
                );
            }
            for (key, path) in &prior.locations {
                if let Some(o) = objects.get_mut(key) {
                    o.directory = path.clone();
                }
            }
            known_blobs.extend(prior.blobs.iter().map(|(id, bytes)| (id.clone(), bytes)));
        }
        let before = objects.clone();
        let mut blobs = BTreeMap::new();
        let mut dependencies = BTreeSet::new();
        if let Some(transaction) = &plan.transaction {
            for file in &transaction.files {
                if let Some(key) = owner(&before, root, Path::new(&file.path)) {
                    let object = &before[&key];
                    let relative =
                        relative_to(Path::new(&file.path), Path::new(&object.directory))?;
                    if object
                        .state
                        .files
                        .get(&relative)
                        .is_some_and(|blob| *blob != hash(&file.bytes))
                    {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            "An observed operation input differs from its replica base; explicitly adopt external content first",
                        ));
                    }
                    dependencies.insert(key);
                }
            }
            match &transaction.mutation {
                Mutation::Update { files } => {
                    for file in files {
                        patch(&mut objects, root, file, &known_blobs, &mut blobs)?;
                    }
                }
                Mutation::CreateTree {
                    births,
                    files,
                    updates,
                    ..
                } => {
                    for birth in births {
                        register_node(&mut objects, root, Path::new(&birth.path), &birth.node_id)?;
                    }
                    for file in files.iter().chain(updates) {
                        patch(&mut objects, root, file, &known_blobs, &mut blobs)?;
                    }
                }
                Mutation::Relocate {
                    source,
                    target,
                    old_leaf,
                    new_leaf,
                    ..
                } => {
                    let key = before
                        .iter()
                        .find(|(_, o)| {
                            Path::new(&o.directory) == Path::new(source)
                                && matches!(o.object, ObjectKey::Node(_))
                        })
                        .map(|(k, _)| k.clone())
                        .ok_or_else(|| invalid("Relocated object has no replica basis"))?;
                    let parent = if Path::new(target).starts_with(root.join(".weftext-trash")) {
                        before[&key].state.parent.clone()
                    } else {
                        parent_id(&objects, root, Path::new(target).parent().unwrap())?
                    };
                    let node = objects.get_mut(&key).unwrap();
                    let source_hash = node.state.files.remove(old_leaf).ok_or_else(|| {
                        invalid("Relocated canonical file is missing from replica basis")
                    })?;
                    node.state.files.insert(new_leaf.clone(), source_hash);
                    node.state.name = Path::new(new_leaf)
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .into();
                    node.state.parent = parent;
                    node.state.retired = Path::new(target).starts_with(root.join(".weftext-trash"));
                    for object in objects.values_mut() {
                        if let Ok(relative) = Path::new(&object.directory).strip_prefix(source) {
                            object.directory = if relative.as_os_str().is_empty() {
                                target.clone()
                            } else {
                                display(&Path::new(target).join(relative))
                            };
                        }
                    }
                }
            }
        } else if plan.kind == "createNote" {
            register_node(
                &mut objects,
                root,
                Path::new(&plan.path),
                plan.node_id
                    .as_ref()
                    .ok_or_else(|| invalid("Node creation lacks an identity"))?,
            )?;
            patch(
                &mut objects,
                root,
                &FileChange {
                    path: plan.path.clone(),
                    base_id: None,
                    before: None,
                    after: plan.after.clone(),
                },
                &known_blobs,
                &mut blobs,
            )?;
        } else if matches!(plan.kind.as_str(), "save" | "saveAs") {
            patch(
                &mut objects,
                root,
                &FileChange {
                    path: plan.path.clone(),
                    base_id: None,
                    before: plan.before.clone(),
                    after: plan.after.clone(),
                },
                &known_blobs,
                &mut blobs,
            )?;
        } else {
            return Ok(());
        }
        let mut changes = Vec::new();
        let mut parents = BTreeSet::new();
        let mut locations = Vec::new();
        for (key, after) in &objects {
            let original = before.get(key);
            if original.is_none_or(|o| o.directory != after.directory) {
                locations.push((key.clone(), after.directory.clone()));
            }
            if original.is_some_and(|o| o.state == after.state) {
                continue;
            }
            if let Some(o) = original {
                parents.insert(o.commit.clone());
            }
            if let Some(parent) = &after.state.parent {
                let p = before.get(parent);
                if let Some(p) = p {
                    dependencies.insert(p.object.clone());
                }
            }
            changes.push(Change {
                object: key.clone(),
                before: original
                    .map(|o| BTreeSet::from([o.revision.clone()]))
                    .unwrap_or_default(),
                revision: fresh(),
                after: after.state.clone(),
            });
        }
        if changes.is_empty() {
            return Ok(());
        }
        let dependencies: Vec<_> = dependencies
            .into_iter()
            .filter_map(|key| {
                before.get(&key).map(|o| {
                    parents.insert(o.commit.clone());
                    Dependency {
                        object: key,
                        revisions: BTreeSet::from([o.revision.clone()]),
                    }
                })
            })
            .collect();
        if parents.is_empty() {
            parents.insert(setup.identity.genesis.clone());
        }
        let manifest = Manifest {
            version: 1,
            workspace_id: setup.identity.workspace_id.clone(),
            replica_id: setup.replica_id.clone(),
            operation_id: plan.operation_id.clone(),
            parents,
            changes,
            dependencies,
        };
        manifest.id()?;
        plan.workspace_id = Some(setup.identity.workspace_id.clone());
        plan.sync_publication = Some(SyncPublication {
            root: setup.root.clone(),
            identity: setup.identity.clone(),
            manifest,
            blobs,
            locations,
        });
        Ok(())
    }
}

fn config_key(root: &Path, path: &Path) -> Option<ObjectKey> {
    if path.parent() != Some(root) {
        return None;
    }
    match path.file_name()?.to_str()? {
        ".weftext-schema.json" => Some(ObjectKey::Schema),
        ".weftext-settings.json" => Some(ObjectKey::Settings),
        ".weftext-tags.json" => Some(ObjectKey::Tags),
        _ => None,
    }
}
fn owner(
    objects: &BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    path: &Path,
) -> Option<ObjectKey> {
    if let Some(key) = config_key(root, path) {
        return Some(key);
    }
    if path == root.join("weftext.directory.json") {
        return objects
            .keys()
            .find(|k| matches!(k, ObjectKey::RootDirectory(_)))
            .cloned();
    }
    objects
        .values()
        .filter(|o| {
            matches!(o.object, ObjectKey::Node(_) | ObjectKey::Directory(_))
                && path.starts_with(&o.directory)
        })
        .max_by_key(|o| Path::new(&o.directory).components().count())
        .map(|o| o.object.clone())
}
fn parent_id(
    objects: &BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    parent: &Path,
) -> Result<Option<ObjectKey>> {
    if parent == root {
        return Ok(None);
    }
    objects
        .values()
        .find_map(|o| {
            if Path::new(&o.directory) == parent {
                match &o.object {
                    ObjectKey::Node(_) | ObjectKey::Directory(_) => Some(o.object.clone()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .map(Some)
        .ok_or_else(|| {
            CoreError::new(
                "syncUnmanagedParent",
                "Admit the containing directory before synchronizing its children",
            )
        })
}
fn relative_to(path: &Path, directory: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(directory)
        .map_err(|_| invalid("Object file is outside its owner"))?
        .to_string_lossy()
        .replace('\\', "/"))
}
fn register_node(
    objects: &mut BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    path: &Path,
    id: &str,
) -> Result<()> {
    let key = ObjectKey::Node(id.into());
    if objects.contains_key(&key) {
        return Err(invalid("Node identity was already present in this replica"));
    }
    let directory = path.parent().unwrap();
    let parent = parent_id(objects, root, directory.parent().unwrap())?;
    objects.insert(
        key.clone(),
        LocalObject {
            object: key,
            directory: display(directory),
            commit: String::new(),
            revision: String::new(),
            state: ObjectState {
                parent,
                name: path.file_stem().unwrap().to_string_lossy().into(),
                retired: false,
                files: BTreeMap::new(),
            },
        },
    );
    Ok(())
}
fn patch(
    objects: &mut BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    file: &FileChange,
    known: &BTreeMap<String, &Vec<u8>>,
    blobs: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let path = Path::new(&file.path);
    let Some(key) = owner(objects, root, path) else {
        return Ok(());
    };
    if !objects.contains_key(&key) {
        objects.insert(
            key.clone(),
            LocalObject {
                object: key.clone(),
                directory: display(root),
                commit: String::new(),
                revision: String::new(),
                state: ObjectState {
                    parent: None,
                    name: path.file_name().unwrap().to_string_lossy().into(),
                    retired: false,
                    files: BTreeMap::new(),
                },
            },
        );
    }
    let object = objects.get_mut(&key).unwrap();
    let relative = relative_to(path, Path::new(&object.directory))?;
    super::relative(&relative)?;
    let previous = object
        .state
        .files
        .get(&relative)
        .map(|id| {
            known
                .get(id)
                .copied()
                .or_else(|| blobs.get(id))
                .ok_or_else(|| invalid("Original replica blob is missing"))
        })
        .transpose()?;
    if previous.map(Vec::as_slice) != file.before.as_deref() {
        return Err(CoreError::new(
            "syncExternalChange",
            "Current bytes differ from the replica's observed base; preserve and explicitly adopt external content before publishing",
        ));
    }
    let blob = hash(&file.after);
    object.state.files.insert(relative, blob.clone());
    blobs.insert(blob, file.after.clone());
    Ok(())
}

#[cfg(all(test, windows))]
#[path = "replica_tests.rs"]
mod tests;
