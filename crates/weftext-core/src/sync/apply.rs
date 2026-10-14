//! Incoming content uses the existing guarded Update transaction. The original
//! plan also freezes installed portable versions; receipt recovery cannot echo
//! received content as a new local edit or lose its causal predecessor.
use super::replica::{LocalObject, SyncPublication};
use super::*;
use crate::{
    Core, OperationIntent, OperationPreview, OperationReceipt, OperationRequest, Path,
    absolute_path,
    backend::ParentGuard,
    display, fresh,
    journal::Plan,
    operations::{FileChange, FrozenOperation, Mutation, capture_optional, change},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncResolution {
    pub object: ObjectKey,
    /// Exact complete displayed conflict set. A newly received side invalidates
    /// this proposal; the caller must review it before making another proposal.
    pub revisions: BTreeSet<String>,
    pub state: ObjectState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncApplication {
    pub root: String,
    pub identity: SyncIdentity,
    pub before: Vec<LocalObject>,
    pub after: Vec<LocalObject>,
    pub expected_versions: Vec<Dependency>,
    pub blobs: BTreeMap<String, Vec<u8>>,
}

impl Core {
    pub(crate) fn prepare_sync_application(
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
        let root = absolute_path(Path::new(&request.root))?;
        let setup = self.sync_setup_for(&root)?;
        let (identity, requested, resolutions, supplied) = match &request.intent {
            OperationIntent::ApplySync { identity, objects } => {
                (identity, objects.clone(), vec![], BTreeMap::new())
            }
            OperationIntent::MergeSync {
                identity,
                resolutions,
                blobs,
            } => (
                identity,
                resolutions.iter().map(|r| r.object.clone()).collect(),
                resolutions.clone(),
                blobs.clone(),
            ),
            _ => return Err(invalid("Expected replica application intent")),
        };
        if identity != &setup.identity
            || self.require_workspace(&root)?.workspace_id != identity.workspace_id
        {
            return Err(invalid(
                "Replica identity does not match this local binding",
            ));
        }
        if requested.is_empty()
            || requested.len() > MAX_CHANGES
            || (!resolutions.is_empty() && requested.len() != resolutions.len())
        {
            return Err(invalid(
                "Select a bounded nonempty set without duplicate resolutions",
            ));
        }
        // Make every original committed local branch visible before inspecting
        // conflicts. Publication uses immutable outbox bytes, never current files.
        self.publish_sync(&root)?;
        let (local, _) = self.sync_local_state(setup)?;
        let mut graph = self.read_sync_history(identity)?;
        let current = graph.objects()?;
        let installed = graph.closure(&local.values().map(|v| v.commit.clone()).collect());
        let mut needed = requested.clone();
        loop {
            let previous = needed.len();
            let cut = needed
                .iter()
                .map(|key| {
                    current
                        .get(key)
                        .ok_or_else(|| invalid("Selected object is unavailable"))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flat_map(|v| v.versions.iter().map(|v| v.commit.clone()))
                .collect();
            // Any not-yet-installed compound transaction remains indivisible.
            for id in graph.closure(&cut).difference(&installed) {
                let manifest = graph.manifest(id).unwrap();
                needed.extend(manifest.changes.iter().map(|c| c.object.clone()));
                needed.extend(manifest.dependencies.iter().map(|d| d.object.clone()));
            }
            for key in needed.clone() {
                for v in &current[&key].versions {
                    if let Some(parent) = &v.state.parent {
                        needed.insert(parent.clone());
                    }
                    needed.extend(
                        graph
                            .manifest(&v.commit)
                            .unwrap()
                            .dependencies
                            .iter()
                            .map(|d| d.object.clone()),
                    );
                }
            }
            if needed.len() == previous {
                break;
            }
        }
        if graph.manifests.iter().any(|(id, m)| {
            m.changes.iter().any(|c| needed.contains(&c.object))
                && matches!(graph.status(id), Ok(Admission::Staged { .. }))
        }) {
            return Err(CoreError::new(
                "syncIncomplete",
                "Hydrate the selected incomplete transaction before applying content",
            ));
        }
        let expected_versions: Vec<_> = needed
            .iter()
            .map(|k| Dependency {
                object: k.clone(),
                revisions: revisions(&current[k]),
            })
            .collect();
        let mut states = BTreeMap::new();
        for key in &needed {
            let view = &current[key];
            if let Some(resolution) = resolutions.iter().find(|r| &r.object == key) {
                if resolution.revisions != revisions(view) || view.versions.len() < 2 {
                    return Err(CoreError::new(
                        "syncConflictChanged",
                        "Review the exact current conflict branches before resolving",
                    ));
                }
            } else if view.versions.len() != 1 {
                return Err(CoreError::new(
                    "syncConflict",
                    "This selected transaction has unresolved concurrent versions",
                ));
            } else {
                let version = &view.versions[0];
                if !graph.unsettled_dependencies(&version.commit)?.is_empty() {
                    return Err(CoreError::new(
                        "syncDependencyConflict",
                        "The incoming version requires revalidation against changed semantic dependencies",
                    ));
                }
                states.insert(key.clone(), version.clone());
            }
        }
        let publication = if resolutions.is_empty() {
            if !supplied.is_empty() {
                return Err(invalid("Unexpected merge blobs"));
            }
            None
        } else {
            if supplied.len() > MAX_FILES
                || supplied.values().map(Vec::len).sum::<usize>() > MAX_BLOB_BYTES
            {
                return Err(CoreError::new(
                    "budget",
                    "Merge content exceeds the bounded operation budget",
                ));
            }
            for (id, bytes) in &supplied {
                graph.offer_blob(id, bytes.clone())?;
            }
            let parents = needed
                .iter()
                .flat_map(|k| current[k].versions.iter().map(|v| v.commit.clone()))
                .collect();
            let manifest = Manifest {
                version: 1,
                workspace_id: identity.workspace_id.clone(),
                replica_id: setup.replica_id.clone(),
                operation_id: request.operation_id.clone(),
                parents,
                changes: resolutions
                    .iter()
                    .map(|r| Change {
                        object: r.object.clone(),
                        before: r.revisions.clone(),
                        revision: fresh(),
                        after: r.state.clone(),
                    })
                    .collect(),
                dependencies: needed
                    .difference(&requested)
                    .map(|k| Dependency {
                        object: k.clone(),
                        revisions: revisions(&current[k]),
                    })
                    .collect(),
            };
            let id = manifest.id()?;
            graph.offer_manifest(&id, manifest.clone())?;
            if graph.status(&id)? != Admission::Accepted {
                return Err(invalid(
                    "Merged state has missing content or invalid causal predecessors",
                ));
            }
            for c in &manifest.changes {
                states.insert(
                    c.object.clone(),
                    Version {
                        commit: id.clone(),
                        revision: c.revision.clone(),
                        state: c.after.clone(),
                    },
                );
            }
            let used: BTreeSet<_> = manifest.blobs().into_iter().cloned().collect();
            if supplied.keys().any(|k| !used.contains(k)) {
                return Err(invalid("Merge contains unrelated blobs"));
            }
            Some(SyncPublication {
                root: display(&root),
                identity: identity.clone(),
                manifest,
                blobs: supplied,
                locations: vec![],
            })
        };
        let kind = if publication.is_some() {
            "mergeSync"
        } else {
            "applySync"
        };
        let plan_id = fresh();
        let mut preview = OperationPreview {
            operation_id: request.operation_id.clone(),
            plan_id: plan_id.clone(),
            kind: kind.into(),
            status: "prepared".into(),
            changes: vec![],
            node_map: BTreeMap::new(),
            new_node_id: None,
            revisions: BTreeMap::new(),
            resource_map: BTreeMap::new(),
            losses: vec![],
        };
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut blobs = BTreeMap::new();
        let mut files = Vec::new();
        let mut changes = Vec::new();
        let mut write_parents = Vec::new();
        let mut paths = BTreeSet::new();
        let mut resource_ids = BTreeSet::new();
        for (key, version) in states {
            let old = local.get(&key).ok_or_else(|| {
                CoreError::new(
                    "syncHydrationRequired",
                    "This application requires an object not yet installed in the partial replica",
                )
            })?;
            let state = &version.state;
            if state.parent != old.state.parent
                || state.name != old.state.name
                || state.retired != old.state.retired
                || state.retired
                || old.state.files.keys().any(|p| !state.files.contains_key(p))
            {
                return Err(CoreError::new(
                    "syncStructureRequired",
                    "Placement, retirement and file removal require a guarded structure application",
                ));
            }
            let directory = Path::new(&old.directory);
            let resources = validate_state(&graph, &key, state)?;
            if let ObjectKey::Node(id) = &key {
                let canonical = self.managed_node(&root, directory, false)?;
                if self
                    .admitted_binding(&canonical.stamp.identity)
                    .is_none_or(|b| b.node_id != *id)
                {
                    return Err(invalid(
                        "Local node is no longer the originally admitted object",
                    ));
                }
                preview.node_map.insert(id.clone(), id.clone());
            }
            if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = &key {
                let guard = ParentGuard::acquire(&directory.join("weftext.directory.json"))?;
                if !self.directory_admitted(
                    &identity.workspace_id,
                    id,
                    &guard.directory_identity()?,
                )? {
                    return Err(invalid(
                        "Local directory no longer has its original admission",
                    ));
                }
            }
            for (relative, digest) in &state.files {
                let path = directory.join(relative);
                if !paths.insert(display(&path).to_lowercase()) {
                    return Err(invalid("Incoming files overlap"));
                }
                self.check_pending(&path)?;
                let captured = capture_optional(&path)?;
                if captured.as_ref().map(|f| hash(&f.bytes)).as_ref()
                    != old.state.files.get(relative)
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        "Actual local content differs from the installed replica base; preserve and explicitly adopt it first",
                    ));
                }
                if matches!(key, ObjectKey::Tags)
                    && captured
                        .as_ref()
                        .is_some_and(|f| self.accepted_revision(f).is_none())
                {
                    return Err(invalid("Tag registry is not the original observed file"));
                }
                let bytes = graph
                    .blob(digest)
                    .ok_or_else(|| invalid("Incoming content is missing"))?
                    .to_vec();
                if let Some(id) = resources.get(relative) {
                    if !resource_ids.insert(id.clone()) {
                        return Err(invalid("Resource identity is duplicated across objects"));
                    }
                    let ObjectKey::Node(node_id) = &key else {
                        return Err(invalid("Resource lacks Node owner"));
                    };
                    if let Some(file) = &captured {
                        if !self.admitted_resource(
                            id,
                            &identity.workspace_id,
                            node_id,
                            &file.stamp.identity,
                        ) {
                            return Err(invalid(
                                "Incoming resource identity differs from local admission",
                            ));
                        }
                    } else {
                        preview.resource_map.insert(
                            display(&path),
                            format!("weftext-resource://{}/{id}", identity.workspace_id),
                        );
                    }
                } else if captured.is_none()
                    && relative != "weftext.annotations.json"
                    && relative != "weftext.resources.json"
                {
                    return Err(invalid(
                        "A newly received file requires an admitted content role",
                    ));
                }
                let revision = captured
                    .as_ref()
                    .filter(|f| f.bytes == bytes)
                    .and_then(|f| self.accepted_revision(f))
                    .unwrap_or_else(fresh);
                let prior = captured.as_ref().map(|f| f.bytes.clone());
                if let Some(file) = captured {
                    files.push(file);
                }
                let action = if prior.as_ref() == Some(&bytes) {
                    "readDependency"
                } else {
                    "receive"
                };
                preview.changes.push(change(&path, action, bytes.len()));
                preview.revisions.insert(display(&path), revision);
                write_parents.push((
                    display(&path),
                    ParentGuard::acquire(&path)?.directory_identity()?,
                ));
                changes.push(FileChange {
                    path: display(&path),
                    base_id: None,
                    before: prior,
                    after: bytes.clone(),
                });
                blobs.insert(digest.clone(), bytes);
            }
            before.push(old.clone());
            after.push(LocalObject {
                object: key,
                directory: old.directory.clone(),
                commit: version.commit,
                revision: version.revision,
                state: version.state,
            });
        }
        if changes.is_empty()
            || changes.len() > MAX_FILES
            || changes
                .iter()
                .map(|c| c.after.len() + c.before.as_ref().map_or(0, Vec::len))
                .sum::<usize>()
                > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Incoming application exceeds the 64 MiB / 5000 file transaction budget",
            ));
        }
        if before == after {
            return Err(CoreError::new(
                "syncCurrent",
                "Selected objects already have these installed versions",
            ));
        }
        let receipt = OperationReceipt { operation_id: request.operation_id.clone(), plan_id, status: "committed".into(), path: display(&root), node_ids: preview.node_map.keys().cloned().collect(),
            message: "Selected content committed through the original guarded transaction; existing session drafts are retained".into() };
        let mut plan = Plan::file(
            request.operation_id.clone(),
            fingerprint,
            &root,
            None,
            vec![],
            None,
            kind,
        );
        plan.workspace_id = Some(identity.workspace_id.clone());
        plan.sync_application = Some(SyncApplication {
            root: display(&root),
            identity: identity.clone(),
            before,
            after,
            expected_versions,
            blobs,
        });
        plan.sync_publication = publication;
        plan.transaction = Some(FrozenOperation {
            request,
            preview: preview.clone(),
            files,
            directories: vec![],
            mutation: Mutation::Update { files: changes },
            receipt,
            expected_membership: None,
            absent_dependencies: vec![],
            write_parents,
        });
        self.stage(plan)?;
        Ok(preview)
    }

    pub(crate) fn validate_sync_application(&self, plan: &Plan) -> Result<()> {
        let Some(application) = &plan.sync_application else {
            return Ok(());
        };
        self.sync_local_owner()?;
        let setup = self.sync_setup_for(Path::new(&application.root))?;
        if setup.identity != application.identity {
            return Err(invalid("Replica identity changed"));
        }
        let (local, _) = self.sync_local_state(setup)?;
        if application
            .before
            .iter()
            .any(|v| local.get(&v.object) != Some(v))
        {
            return Err(CoreError::new(
                "syncBasisChanged",
                "A local commit changed the reviewed replica basis",
            ));
        }
        self.publish_sync(&application.root)?;
        let current = self.read_sync_history(&application.identity)?.objects()?;
        if application.expected_versions.iter().any(|expected| {
            current
                .get(&expected.object)
                .is_none_or(|v| revisions(v) != expected.revisions)
        }) {
            return Err(CoreError::new(
                "syncConflictChanged",
                "Received versions changed after review; retain the original proposal and review again",
            ));
        }
        Ok(())
    }
}
fn revisions(view: &ObjectView) -> BTreeSet<String> {
    view.versions.iter().map(|v| v.revision.clone()).collect()
}

fn validate_state(
    graph: &ReplicaGraph,
    key: &ObjectKey,
    state: &ObjectState,
) -> Result<BTreeMap<String, String>> {
    let mut resources = BTreeMap::new();
    if let ObjectKey::Node(id) = key {
        let source = state
            .files
            .get(&format!("{}.typ", state.name))
            .and_then(|b| graph.blob(b))
            .ok_or_else(|| invalid("Node has no canonical source"))?;
        crate::check_size(source)?;
        if weftext_engine::inspect(&crate::decode(source)?).id.as_ref() != Some(id) {
            return Err(invalid(
                "Incoming canonical source changes the Node identity",
            ));
        }
        if let Some(blob) = state.files.get("weftext.resources.json") {
            let manifest: serde_json::Value = serde_json::from_slice(graph.blob(blob).unwrap())?;
            let entries = manifest["resources"]
                .as_array()
                .filter(|_| manifest["version"] == 1)
                .ok_or_else(|| invalid("Invalid owned resource manifest"))?;
            let mut ids = BTreeSet::new();
            let mut paths = BTreeSet::new();
            for entry in entries {
                let id = entry["resourceId"]
                    .as_str()
                    .ok_or_else(|| invalid("Resource identity missing"))?;
                let path = entry["path"]
                    .as_str()
                    .ok_or_else(|| invalid("Resource path missing"))?;
                uuid(id)?;
                super::relative(path)?;
                if !ids.insert(id)
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
                    return Err(invalid("Duplicate or invalid owned resource binding"));
                }
                resources.insert(path.into(), id.into());
            }
        }
    }
    for (path, digest) in &state.files {
        let bytes = graph
            .blob(digest)
            .ok_or_else(|| invalid("Missing incoming file blob"))?;
        if matches!(key, ObjectKey::Tags) {
            crate::directory_identity::validate_sync_tags(bytes)?;
        }
        if matches!(
            path.as_str(),
            "weftext.directory.json" | "weftext.annotations.json"
        ) || !matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_))
        {
            let _: serde_json::Value = serde_json::from_slice(bytes)?;
        }
        if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = key
            && path == "weftext.directory.json"
        {
            crate::directory_identity::validate_directory_marker(bytes)?;
            if serde_json::from_slice::<serde_json::Value>(bytes)?["directoryId"] != *id {
                return Err(invalid("Directory identity changed"));
            }
        }
    }
    Ok(resources)
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod tests;
