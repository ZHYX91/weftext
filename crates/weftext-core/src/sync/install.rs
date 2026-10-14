//! Replica creation reuses the original guarded CreateTree transaction, with
//! fresh receiver file identities and the same portable content identities.
use super::replica::{LocalObject, SyncSetup};
use super::*;
use crate::{
    Core, OperationIntent, OperationPreview, OperationReceipt, OperationRequest, Path, PathBuf,
    STORAGE_PROFILE, WorkspaceConfig, absolute_path,
    backend::ParentGuard,
    display, fresh,
    journal::Plan,
    operations::{FileChange, FrozenOperation, Mutation, NewBirth, change, ensure_absent},
};

impl Core {
    pub(crate) fn prepare_sync_clone(
        &mut self,
        request: OperationRequest,
    ) -> Result<OperationPreview> {
        self.sync_local_owner()?;
        crate::valid_operation_id(&request.operation_id)?;
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        if let Some(old) = self.operations.get(&request.operation_id) {
            if old.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    "Replica operation already names different input",
                ));
            }
            return self.operation_details(&request.operation_id);
        }
        let OperationIntent::CloneSync { identity, objects } = &request.intent else {
            return Err(invalid("Expected replica creation intent"));
        };
        let root = absolute_path(Path::new(&request.root))?;
        if root.starts_with(&self.data_dir)
            || self.data_dir.starts_with(&root)
            || self
                .operations
                .values()
                .filter(|p| {
                    self.committed(&p.operation_id)
                        && matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                })
                .any(|p| root.starts_with(&p.path) || Path::new(&p.path).starts_with(&root))
        {
            return Err(CoreError::new(
                "syncTargetOverlap",
                "Create a replica outside private state and existing libraries",
            ));
        }
        if self
            .sync_setups()
            .iter()
            .any(|s| s.identity.workspace_id == identity.workspace_id)
        {
            return Err(CoreError::new(
                "syncReplicaExists",
                "Each Core state owns one replica of a workspace; use independent private state",
            ));
        }
        self.check_sync_setup_pending(&identity.workspace_id)?;
        ensure_absent(&root)?;
        let parent = ParentGuard::acquire(&root)?.directory_identity()?;
        let source_graph = self.read_sync_history(identity)?;
        let bundle = source_graph.export_objects(objects)?;
        let mut graph = ReplicaGraph::new(&identity.workspace_id, &identity.genesis)?;
        for (id, bytes) in &bundle.blobs {
            graph.offer_blob(id, bytes.clone())?;
        }
        for (id, manifest) in &bundle.manifests {
            graph.offer_manifest(id, manifest.clone())?;
        }
        let view = graph.objects()?;
        for (id, manifest) in &source_graph.manifests {
            if manifest
                .changes
                .iter()
                .any(|c| view.contains_key(&c.object))
                && matches!(source_graph.status(id)?, Admission::Staged { .. })
            {
                return Err(CoreError::new(
                    "syncIncomplete",
                    "Selected content has an incomplete received transaction; hydrate it before creating this replica",
                ));
            }
        }
        let mut states = BTreeMap::new();
        for (key, object) in view {
            if object.versions.len() != 1 {
                return Err(CoreError::new(
                    "syncConflict",
                    "Choose or merge every conflicting object in this selected transaction closure",
                ));
            }
            let version = object.versions.into_iter().next().unwrap();
            if version.state.retired {
                return Err(CoreError::new(
                    "syncRetiredSelection",
                    "Trashed content remains in received history; live replica installation needs an explicit restore plan",
                ));
            }
            states.insert(key, version);
        }
        let mut locations: BTreeMap<ObjectKey, PathBuf> = BTreeMap::new();
        let mut remaining: BTreeSet<_> = states.keys().cloned().collect();
        while !remaining.is_empty() {
            let mut advanced = false;
            for key in remaining.clone() {
                let state = &states[&key].state;
                let target = match &key {
                    ObjectKey::Node(_) | ObjectKey::Directory(_) => {
                        let parent = if let Some(parent) = &state.parent {
                            let Some(path) = locations.get(parent) else {
                                continue;
                            };
                            path.clone()
                        } else {
                            root.clone()
                        };
                        parent.join(&state.name)
                    }
                    _ => root.clone(),
                };
                locations.insert(key.clone(), target);
                remaining.remove(&key);
                advanced = true;
            }
            if !advanced {
                return Err(invalid(
                    "Selected placement graph is cyclic or lacks its parent",
                ));
            }
        }
        let plan_id = fresh();
        let mut preview = OperationPreview {
            operation_id: request.operation_id.clone(),
            plan_id: plan_id.clone(),
            kind: "cloneSync".into(),
            status: "prepared".into(),
            changes: vec![],
            node_map: BTreeMap::new(),
            new_node_id: None,
            revisions: BTreeMap::new(),
            resource_map: BTreeMap::new(),
            losses: vec![],
        };
        let mut output = BTreeMap::new();
        let mut directories = BTreeSet::new();
        let mut births = Vec::new();
        let mut initial = Vec::new();
        let mut resource_ids = BTreeSet::new();
        let mut occupied_directories = BTreeSet::new();
        for (key, version) in &states {
            let directory = &locations[key];
            if matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_))
                && !occupied_directories.insert(display(directory).to_lowercase())
            {
                return Err(invalid(
                    "Two content objects claim the same receiver directory",
                ));
            }
            let state = &version.state;
            if let ObjectKey::Node(id) = key {
                let source = format!("{}.typ", state.name);
                let blob = state
                    .files
                    .get(&source)
                    .ok_or_else(|| invalid("Node has no canonical source"))?;
                let bytes = graph
                    .blob(blob)
                    .ok_or_else(|| invalid("Canonical source blob is unavailable"))?;
                crate::check_size(bytes)?;
                if weftext_engine::inspect(&crate::decode(bytes)?).id.as_ref() != Some(id) {
                    return Err(invalid(
                        "Portable Node identity differs from the actual author declaration",
                    ));
                }
                births.push(NewBirth {
                    path: display(&directory.join(source)),
                    node_id: id.clone(),
                });
                preview.node_map.insert(id.clone(), id.clone());
                if let Some(blob) = state.files.get("weftext.resources.json") {
                    let manifest: serde_json::Value =
                        serde_json::from_slice(graph.blob(blob).unwrap())?;
                    let resources = manifest["resources"]
                        .as_array()
                        .filter(|_| manifest["version"] == 1)
                        .ok_or_else(|| {
                            invalid("Resource manifest is unavailable or unsupported")
                        })?;
                    let mut paths = BTreeSet::new();
                    for resource in resources {
                        let id = resource["resourceId"]
                            .as_str()
                            .ok_or_else(|| invalid("Resource identity is missing"))?;
                        uuid(id)?;
                        let path = resource["path"]
                            .as_str()
                            .ok_or_else(|| invalid("Resource path is missing"))?;
                        super::relative(path)?;
                        if !resource_ids.insert(id.to_owned())
                            || !paths.insert(path.to_lowercase())
                            || !state.files.contains_key(path)
                            || path == format!("{}.typ", state.name)
                            || matches!(
                                path,
                                "weftext.resources.json"
                                    | "weftext.annotations.json"
                                    | "weftext.directory.json"
                            )
                        {
                            return Err(invalid(
                                "Duplicate, missing or invalid owned resource binding",
                            ));
                        }
                        preview.resource_map.insert(
                            display(&directory.join(path)),
                            format!("weftext-resource://{}/{id}", identity.workspace_id),
                        );
                    }
                }
            }
            for (relative, blob) in &state.files {
                let bytes = graph
                    .blob(blob)
                    .ok_or_else(|| invalid("Selected file blob is missing"))?;
                if matches!(key, ObjectKey::Tags) {
                    crate::directory_identity::validate_sync_tags(bytes)?;
                }
                let path = directory.join(relative);
                let name = path.file_name().unwrap().to_string_lossy();
                if matches!(
                    name.as_ref(),
                    "weftext.directory.json" | "weftext.annotations.json"
                ) || !matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_))
                {
                    let _: serde_json::Value = serde_json::from_slice(bytes)?;
                }
                if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = key
                    && relative == "weftext.directory.json"
                {
                    crate::directory_identity::validate_directory_marker(bytes)?;
                    let value: serde_json::Value = serde_json::from_slice(bytes)?;
                    if value["directoryId"] != *id {
                        return Err(invalid(
                            "Directory marker identity differs from its content role",
                        ));
                    }
                }
                let lower = display(&path).to_lowercase();
                if output
                    .insert(lower, (path.clone(), bytes.to_vec()))
                    .is_some()
                {
                    return Err(invalid("Receiver files overlap"));
                }
                let mut parent = path.parent().unwrap();
                while parent != root {
                    directories.insert(display(
                        parent
                            .strip_prefix(&root)
                            .map_err(|_| invalid("Receiver path escaped its root"))?,
                    ));
                    parent = parent
                        .parent()
                        .ok_or_else(|| invalid("Receiver path escaped its root"))?;
                }
            }
            initial.push(LocalObject {
                object: key.clone(),
                directory: display(directory),
                commit: version.commit.clone(),
                revision: version.revision.clone(),
                state: state.clone(),
            });
        }
        let config = WorkspaceConfig {
            version: 1,
            workspace_id: identity.workspace_id.clone(),
            storage_profile: STORAGE_PROFILE.into(),
            schema_support: "notImplemented".into(),
            sync_support: "managed-v1".into(),
        };
        for (name, bytes) in [
            (
                ".weftext-workspace.json",
                serde_json::to_vec_pretty(&config)?,
            ),
            (".weftext-format", b"weftext.typst.v1\n".to_vec()),
            (
                ".weftext-mvp-commit.json",
                serde_json::to_vec(
                    &serde_json::json!({"workspaceId":identity.workspace_id,"operationId":request.operation_id,"storageProfile":STORAGE_PROFILE}),
                )?,
            ),
        ] {
            let path = root.join(name);
            if output
                .insert(display(&path).to_lowercase(), (path, bytes))
                .is_some()
            {
                return Err(invalid("Content cannot replace receiver control files"));
            }
        }
        if output.len() > MAX_FILES
            || output.values().map(|(_, bytes)| bytes.len()).sum::<usize>() > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Replica creation exceeds 64 MiB or 5000 files",
            ));
        }
        let files: Vec<_> = output
            .into_values()
            .map(|(path, after)| {
                preview.changes.push(change(&path, "receive", after.len()));
                preview.revisions.insert(display(&path), fresh());
                FileChange {
                    path: display(&path),
                    base_id: None,
                    before: None,
                    after,
                }
            })
            .collect();
        let receipt = OperationReceipt { operation_id: request.operation_id.clone(), plan_id, status: "committed".into(), path: display(&root), node_ids: preview.node_map.keys().cloned().collect(),
            message: "Selected content and its full received dependency/transaction closure installed as an independent partial replica".into() };
        let mut plan = Plan::file(
            request.operation_id.clone(),
            fingerprint,
            &root,
            None,
            vec![],
            None,
            "cloneSync",
        );
        plan.workspace_id = Some(identity.workspace_id.clone());
        plan.sync_setup = Some(SyncSetup {
            root: display(&root),
            identity: identity.clone(),
            replica_id: fresh(),
            initial,
            bundle,
            selection: Some(objects.clone()),
        });
        plan.transaction = Some(FrozenOperation {
            request,
            preview: preview.clone(),
            files: vec![],
            directories: vec![],
            mutation: Mutation::CreateTree {
                target: display(&root),
                directories: directories.into_iter().collect(),
                files,
                births,
                updates: vec![],
            },
            receipt,
            expected_membership: None,
            absent_dependencies: vec![],
            write_parents: vec![(display(&root), parent)],
        });
        self.stage(plan)?;
        Ok(preview)
    }
}

#[cfg(all(test, windows))]
#[path = "install_tests.rs"]
mod tests;
