//! Durable immutable staging owned by Core. A transport can copy these exact
//! records in any order; it cannot turn admission into author-tree installation.
use super::*;
use crate::{Core, backend::ParentGuard, journal::private_create};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use weftext_messages::UiMessage;

const MAX_RECORDS: usize = 20_000;

/// Explicit pairing binds both the workspace and its exact creation history.
/// This is selected by the local owner, never inferred from received metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncIdentity {
    pub workspace_id: String,
    pub genesis: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncBundle {
    pub manifests: BTreeMap<String, Manifest>,
    pub blobs: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub identity: SyncIdentity,
    pub heads: BTreeSet<String>,
    pub commits: Vec<CommitStatus>,
    pub objects: Vec<ObjectStatus>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitStatus {
    pub id: String,
    pub admission: Admission,
    pub unsettled_dependencies: Vec<ObjectKey>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStatus {
    pub object: ObjectKey,
    pub view: ObjectView,
}

/// Exact retained portable content. This is received history, not an installed
/// file, a current author identity, or authority to execute the returned bytes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedSyncFile {
    pub object: ObjectKey,
    pub object_revision: String,
    pub file_revision: String,
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedSyncVersion {
    pub commit: String,
    pub parents: BTreeSet<String>,
    pub before: BTreeSet<String>,
    pub revision: String,
    pub state: ObjectState,
    pub admission: Admission,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedSyncVersions {
    pub versions: Vec<ReceivedSyncVersion>,
    pub next_commit: Option<String>,
    /// Completeness of this object's received records; pagination is separate.
    pub coverage: String,
}

impl Core {
    /// Full owner transport includes staged records too. Admission and local
    /// installation remain separate; a relay is not another current-file owner.
    pub fn export_sync_history(&self, identity: &SyncIdentity) -> Result<SyncBundle> {
        let graph = self.read_sync_history(identity)?;
        Ok(SyncBundle {
            manifests: graph.manifests,
            blobs: graph.blobs,
        })
    }

    /// Inspect one immutable portable record without filesystem paths from a
    /// client. A completed record can be reused after a transfer restart.
    pub fn sync_record(
        &self,
        identity: &SyncIdentity,
        manifest: bool,
        id: &str,
    ) -> Result<Option<Vec<u8>>> {
        self.check_sync_owner(identity)?;
        digest(id)?;
        let (path, _guards) = self.sync_store(identity, false)?;
        let path = path
            .join(if manifest { "manifests" } else { "blobs" })
            .join(format!("{id}.{}", if manifest { "json" } else { "blob" }));
        match read_record(
            &path,
            if manifest {
                MAX_MANIFEST_BYTES
            } else {
                MAX_BLOB_BYTES
            },
        ) {
            Ok(bytes) => {
                if hash(&bytes) != id {
                    return Err(invalid(
                        UiMessage::error_core_sync_store_stored_record_digest_mismatch(),
                    ));
                }
                if manifest {
                    let value: Manifest = serde_json::from_slice(&bytes)?;
                    if value.id()? != id {
                        return Err(invalid(
                            UiMessage::error_core_sync_store_stored_manifest_not_canonical(),
                        ));
                    }
                }
                Ok(Some(bytes))
            }
            Err(error) => match std::fs::symlink_metadata(&path) {
                Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(error),
            },
        }
    }

    pub fn receive_sync_record(
        &self,
        identity: &SyncIdentity,
        manifest: bool,
        id: &str,
        bytes: Vec<u8>,
    ) -> Result<()> {
        self.check_sync_owner(identity)?;
        digest(id)?;
        if hash(&bytes) != id {
            return Err(invalid(
                UiMessage::error_core_sync_store_record_digest_mismatch(),
            ));
        }
        let mut bundle = SyncBundle::default();
        if manifest {
            if bytes.len() > MAX_MANIFEST_BYTES {
                return Err(invalid(
                    UiMessage::error_core_sync_store_manifest_record_budget(),
                ));
            }
            let value: Manifest = serde_json::from_slice(&bytes)?;
            if value.id()? != id {
                return Err(invalid(
                    UiMessage::error_core_sync_store_manifest_not_canonical(),
                ));
            }
            bundle.manifests.insert(id.into(), value);
        } else {
            if bytes.len() > MAX_BLOB_BYTES {
                return Err(invalid(
                    UiMessage::error_core_sync_store_blob_record_budget(),
                ));
            }
            bundle.blobs.insert(id.into(), bytes);
        }
        self.receive_sync(identity, bundle)?;
        Ok(())
    }

    /// A read does not drain the outbox or install a received version.
    pub fn sync_status(&self, identity: &SyncIdentity) -> Result<SyncStatus> {
        status(&self.read_sync_history(identity)?, identity)
    }

    /// Receive into private staging only. Every byte is validated before any
    /// payload is written; a crash can leave a complete subset, never a partial
    /// visible record. Retry the same records to finish that subset.
    pub fn receive_sync(&self, identity: &SyncIdentity, bundle: SyncBundle) -> Result<SyncStatus> {
        self.check_sync_owner(identity)?;
        if bundle.manifests.len() > MAX_COMMITS || bundle.blobs.len() > MAX_RECORDS {
            return Err(invalid(
                UiMessage::error_core_sync_store_transfer_record_budget(),
            ));
        }
        let (path, _guards) = self.sync_store(identity, true)?;
        let mut graph = load_graph(&path, identity)?;
        for (id, blob) in &bundle.blobs {
            graph.offer_blob(id, blob.clone())?;
        }
        for (id, manifest) in &bundle.manifests {
            graph.offer_manifest(id, manifest.clone())?;
        }
        let status = status(&graph, identity)?;
        // The identity is durable before payloads. No default policy or copied
        // author-tree UUID can silently re-pair an existing local store.
        immutable(&path.join("identity.json"), &serde_json::to_vec(identity)?)?;
        for (id, blob) in bundle.blobs {
            immutable(&path.join("blobs").join(format!("{id}.blob")), &blob)?;
        }
        for (id, manifest) in bundle.manifests {
            immutable(
                &path.join("manifests").join(format!("{id}.json")),
                &serde_json::to_vec(&manifest)?,
            )?;
        }
        Ok(status)
    }

    pub fn read_sync_history(&self, identity: &SyncIdentity) -> Result<ReplicaGraph> {
        self.check_sync_owner(identity)?;
        let (path, _guards) = self.sync_store(identity, false)?;
        load_graph(&path, identity)
    }

    /// Read a specified admitted object revision, including retained ancestors
    /// outside the current heads/common bases. Never substitutes newer content.
    pub fn read_received_sync_file(
        &self,
        identity: &SyncIdentity,
        object: &ObjectKey,
        object_revision: &str,
        relative_path: &str,
    ) -> Result<ReceivedSyncFile> {
        key(object)?;
        uuid(object_revision)?;
        relative(relative_path)?;
        let graph = self.read_sync_history(identity)?;
        let mut selected = None;
        for (id, manifest) in &graph.manifests {
            for change in &manifest.changes {
                if &change.object != object || change.revision != object_revision {
                    continue;
                }
                if graph.status(id)? != Admission::Accepted {
                    return Err(CoreError::new(
                        "syncIncomplete",
                        UiMessage::error_core_sync_specified_received_object_revision_not_admitted(
                        ),
                    ));
                }
                if selected.replace(&change.after).is_some() {
                    return Err(invalid(
                        UiMessage::error_core_sync_store_received_revision_ambiguous(),
                    ));
                }
            }
        }
        let state = selected.ok_or_else(|| {
            CoreError::new(
                "syncHistoryMissing",
                UiMessage::error_core_sync_specified_object_revision_unavailable(),
            )
        })?;
        let digest = state.files.get(relative_path).ok_or_else(|| {
            CoreError::new(
                "syncHistoryMissing",
                UiMessage::error_core_sync_specified_revision_has_no_such_file(),
            )
        })?;
        let bytes = graph.blob(digest).ok_or_else(|| {
            CoreError::new(
                "syncIncomplete",
                UiMessage::error_core_sync_specified_exact_history_blob_unavailable(),
            )
        })?;
        Ok(ReceivedSyncFile {
            object: object.clone(),
            object_revision: object_revision.into(),
            file_revision: state.file_revisions[relative_path].clone(),
            relative_path: relative_path.into(),
            bytes: bytes.to_vec(),
        })
    }

    /// Commit IDs provide a lexical cursor, not chronological or causal order.
    /// Concurrent receive can change the cut; refresh instead of treating pages
    /// as a durable snapshot. No blobs are copied to enumerate these records.
    pub fn received_sync_versions(
        &self,
        identity: &SyncIdentity,
        object: &ObjectKey,
        after_commit: Option<&str>,
        limit: usize,
    ) -> Result<ReceivedSyncVersions> {
        key(object)?;
        if !(1..=100).contains(&limit) {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_history_page_size_must_between1_and100(),
            ));
        }
        if let Some(cursor) = after_commit {
            digest(cursor)?;
        }
        let graph = self.read_sync_history(identity)?;
        let records: Vec<_> = graph
            .manifests
            .iter()
            .filter_map(|(id, manifest)| {
                manifest
                    .changes
                    .iter()
                    .find(|c| &c.object == object)
                    .map(|c| (id, manifest, c))
            })
            .collect();
        if after_commit
            .is_some_and(|cursor| !records.iter().any(|(id, _, _)| id.as_str() == cursor))
        {
            return Err(CoreError::new(
                "syncHistoryMissing",
                UiMessage::error_core_sync_cursor_does_not_belong_to_specified_object_s_received_records(),
            ));
        }
        let complete = records
            .iter()
            .all(|(id, _, _)| graph.admitted.contains(*id));
        let mut page = ReceivedSyncVersions {
            versions: Vec::new(),
            next_commit: None,
            coverage: if complete { "complete" } else { "partial" }.into(),
        };
        let mut records = records
            .into_iter()
            .filter(|(id, _, _)| after_commit.is_none_or(|cursor| id.as_str() > cursor))
            .peekable();
        while let Some((id, manifest, change)) = records.next() {
            let version = ReceivedSyncVersion {
                commit: id.clone(),
                parents: manifest.parents.clone(),
                before: change.before.clone(),
                revision: change.revision.clone(),
                state: change.after.clone(),
                admission: graph.status(id)?,
            };
            page.versions.push(version);
            page.next_commit = if records.peek().is_some() {
                Some(id.clone())
            } else {
                None
            };
            if serde_json::to_vec(&page)?.len() > MAX_MANIFEST_BYTES {
                page.versions.pop();
                if page.versions.is_empty() {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_sync_received_history_record_exceeds_page_budget(),
                    ));
                }
                page.next_commit = page.versions.last().map(|v| v.commit.clone());
                break;
            }
            if page.versions.len() == limit {
                break;
            }
        }
        Ok(page)
    }

    pub fn export_sync_objects(
        &self,
        identity: &SyncIdentity,
        objects: &BTreeSet<ObjectKey>,
    ) -> Result<SyncBundle> {
        self.read_sync_history(identity)?.export_objects(objects)
    }

    fn check_sync_owner(&self, identity: &SyncIdentity) -> Result<()> {
        // A field-only or hosted actor must not acquire the full portable
        // history. A future hosted owner transport needs a separate capability.
        if self.actor.is_some() {
            return Err(CoreError::new(
                "syncOwnerRequired",
                UiMessage::error_core_sync_managed_sync_local_owner_operation(),
            ));
        }
        uuid(&identity.workspace_id)?;
        digest(&identity.genesis)
    }

    fn sync_store(
        &self,
        identity: &SyncIdentity,
        create: bool,
    ) -> Result<(PathBuf, Vec<ParentGuard>)> {
        let base = self.data_dir.join("sync");
        let path = base.join(&identity.workspace_id);
        let mut guards = Vec::new();
        for directory in [&base, &path, &path.join("blobs"), &path.join("manifests")] {
            if create && !directory.exists() {
                // Qualify each immediate ancestor before creating the next;
                // create_dir_all could follow an untrusted intermediate link.
                let parent = ParentGuard::acquire(directory)?;
                let handle = parent.create_directory()?;
                drop(handle);
            }
            guards.push(ParentGuard::acquire(&directory.join(".sync-store"))?);
        }
        let binding = path.join("identity.json");
        match read_record(&binding, 1024) {
            Ok(bytes) => {
                let stored: SyncIdentity = serde_json::from_slice(&bytes)?;
                if &stored != identity {
                    return Err(invalid(
                        UiMessage::error_core_sync_store_replica_family_mismatch(),
                    ));
                }
            }
            Err(e) if e.code == "missing" && create => {}
            Err(e) => return Err(e),
        }
        Ok((path, guards))
    }
}

pub(super) fn load_graph(path: &Path, identity: &SyncIdentity) -> Result<ReplicaGraph> {
    let mut graph = ReplicaGraph::new(&identity.workspace_id, &identity.genesis)?;
    for (folder, extension, limit) in [
        ("blobs", ".blob", MAX_BLOB_BYTES),
        ("manifests", ".json", MAX_MANIFEST_BYTES),
    ] {
        let directory = path.join(folder);
        let guard = ParentGuard::acquire(&directory.join(".enumeration"))?;
        let (entries, complete) = guard.directory_entries(MAX_RECORDS)?;
        if !complete {
            return Err(invalid(
                UiMessage::error_core_sync_store_record_inventory_incomplete(),
            ));
        }
        for (name, _) in entries {
            let Some(name) = name.to_str() else {
                return Err(invalid(
                    UiMessage::error_core_sync_store_record_name_not_utf8(),
                ));
            };
            if name.ends_with(".pending") {
                continue;
            }
            let Some(id) = name.strip_suffix(extension) else {
                return Err(invalid(UiMessage::error_core_sync_store_record_unexpected()));
            };
            digest(id)?;
            let bytes = read_record(&directory.join(name), limit)?;
            if folder == "blobs" {
                graph.offer_blob(id, bytes)?;
            } else {
                graph.offer_manifest(id, serde_json::from_slice(&bytes)?)?;
            }
        }
    }
    Ok(graph)
}

fn status(graph: &ReplicaGraph, identity: &SyncIdentity) -> Result<SyncStatus> {
    let objects = graph.objects()?;
    Ok(SyncStatus {
        identity: identity.clone(),
        heads: graph.heads(),
        commits: graph
            .manifests
            .keys()
            .map(|id| {
                Ok(CommitStatus {
                    id: id.clone(),
                    admission: graph.status(id)?,
                    unsettled_dependencies: if graph.admitted.contains(id) {
                        graph.unsettled_dependencies_at(id, &objects)?
                    } else {
                        Vec::new()
                    },
                })
            })
            .collect::<Result<_>>()?,
        objects: objects
            .into_iter()
            .map(|(object, view)| ObjectStatus { object, view })
            .collect(),
    })
}

pub(super) fn read_record(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let guard = ParentGuard::acquire(path)?;
    let file = guard.open_file(false, false)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid(
            UiMessage::error_core_sync_store_record_byte_budget(),
        ));
    }
    Ok(bytes)
}

pub(super) fn immutable(path: &Path, bytes: &[u8]) -> Result<()> {
    match read_record(path, bytes.len()) {
        Ok(existing) if existing == bytes => Ok(()),
        Ok(_) => Err(invalid(
            UiMessage::error_core_sync_store_immutable_record_differs(),
        )),
        Err(e) if e.code == "missing" => private_create(path, bytes),
        Err(e) => Err(e),
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::fresh;
    use std::fs;

    fn fixture() -> (SyncIdentity, SyncBundle) {
        let workspace = fresh();
        let bytes = b"#metadata((id: \"example\"))\nBody".to_vec();
        let blob = hash(&bytes);
        let manifest = Manifest {
            version: 1,
            workspace_id: workspace.clone(),
            replica_id: fresh(),
            operation_id: fresh(),
            parents: BTreeSet::new(),
            dependencies: Vec::new(),
            changes: vec![Change {
                object: ObjectKey::Node(fresh()),
                before: BTreeSet::new(),
                revision: fresh(),
                after: ObjectState {
                    file_revisions: BTreeMap::from([("Note.typ".into(), fresh())]),
                    parent: None,
                    name: "Note".into(),
                    retired: false,
                    files: BTreeMap::from([("Note.typ".into(), blob.clone())]),
                },
            }],
        };
        let genesis = manifest.id().unwrap();
        (
            SyncIdentity {
                workspace_id: workspace,
                genesis: genesis.clone(),
            },
            SyncBundle {
                manifests: BTreeMap::from([(genesis, manifest)]),
                blobs: BTreeMap::from([(blob, bytes)]),
            },
        )
    }

    #[test]
    fn missing_content_stays_staged_across_restart_then_exact_retry_finishes() {
        let temp = tempfile::tempdir().unwrap();
        let (identity, bundle) = fixture();
        let data = temp.path().join("private");
        let author = temp.path().join("author.typ");
        fs::write(&author, "never touched").unwrap();
        {
            let core = Core::new(&data).unwrap();
            let result = core
                .receive_sync(
                    &identity,
                    SyncBundle {
                        manifests: bundle.manifests.clone(),
                        blobs: BTreeMap::new(),
                    },
                )
                .unwrap();
            assert!(result.heads.is_empty());
            assert!(matches!(
                result.commits[0].admission,
                Admission::Staged { .. }
            ));
        }
        {
            let core = Core::new(&data).unwrap();
            assert!(
                core.read_sync_history(&identity)
                    .unwrap()
                    .heads()
                    .is_empty()
            );
            let result = core.receive_sync(&identity, bundle.clone()).unwrap();
            assert_eq!(result.heads, BTreeSet::from([identity.genesis.clone()]));
            assert_eq!(
                core.receive_sync(&identity, bundle).unwrap().heads,
                result.heads
            );
        }
        let core = Core::new(&data).unwrap();
        assert_eq!(
            core.read_sync_history(&identity).unwrap().heads(),
            BTreeSet::from([identity.genesis])
        );
        assert_eq!(fs::read_to_string(author).unwrap(), "never touched");
    }

    #[test]
    fn local_identity_cannot_be_repaired_by_arriving_bundle_or_hosted_actor() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path()).unwrap();
        let (identity, bundle) = fixture();
        core.receive_sync(&identity, bundle.clone()).unwrap();
        let mut wrong = identity.clone();
        wrong.genesis = hash(b"another genesis");
        assert!(core.receive_sync(&wrong, SyncBundle::default()).is_err());
        assert_eq!(
            core.with_actor("reader", |c| c.receive_sync(&identity, bundle))
                .unwrap_err()
                .code,
            "syncOwnerRequired"
        );
        assert_eq!(
            core.with_actor("reader", |c| c.read_sync_history(&identity).map(|_| ()))
                .unwrap_err()
                .code,
            "syncOwnerRequired"
        );
    }

    #[test]
    fn corrupt_private_content_is_not_silently_replaced_or_admitted() {
        let temp = tempfile::tempdir().unwrap();
        let core = Core::new(temp.path()).unwrap();
        let (identity, bundle) = fixture();
        core.receive_sync(&identity, bundle.clone()).unwrap();
        let blob = bundle.blobs.keys().next().unwrap();
        let path = temp
            .path()
            .join("sync")
            .join(&identity.workspace_id)
            .join("blobs")
            .join(format!("{blob}.blob"));
        fs::write(&path, b"damaged externally").unwrap();
        assert!(core.read_sync_history(&identity).is_err());
        assert!(core.receive_sync(&identity, bundle).is_err());
        assert_eq!(fs::read(path).unwrap(), b"damaged externally");
    }

    #[test]
    fn invalid_bundle_has_no_payload_side_effects() {
        let temp = tempfile::tempdir().unwrap();
        let core = Core::new(temp.path()).unwrap();
        let (identity, mut bundle) = fixture();
        let blob = bundle.blobs.values_mut().next().unwrap();
        blob.push(0);
        assert!(core.receive_sync(&identity, bundle).is_err());
        let path = temp.path().join("sync").join(identity.workspace_id);
        assert_eq!(fs::read_dir(path.join("blobs")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(path.join("manifests")).unwrap().count(), 0);
    }
}
