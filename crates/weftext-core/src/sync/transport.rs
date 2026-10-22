//! Same-owner directory transport. This directory carries immutable portable
//! records only. It is neither a working tree nor a copied Core private state.
use super::*;
use crate::{Core, Path, absolute_path, backend::ParentGuard, display};
use weftext_messages::UiMessage;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTransfer {
    pub directory: String,
    pub identity: SyncIdentity,
    pub manifests: usize,
    pub blobs: usize,
    pub bytes: usize,
}

impl Core {
    /// Export a selected complete dependency/transaction closure. Retrying the
    /// same records is create-only and verifies existing exact bytes. Another
    /// replica's records in the same paired directory are retained.
    pub fn write_sync_directory(
        &self,
        root: impl AsRef<Path>,
        directory: impl AsRef<Path>,
        objects: &BTreeSet<ObjectKey>,
    ) -> Result<SyncTransfer> {
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let identity = self.sync_setup_for(&root)?.identity.clone();
        let directory = self.sync_transport_path(directory.as_ref())?;
        self.publish_sync(&root)?;
        let bundle = self.export_sync_objects(&identity, objects)?;
        let _guards = transport_guards(&directory, &identity, true)?;
        // Before any payload write, validate the whole combined bounded store.
        let mut graph = store::load_graph(&directory, &identity)?;
        for (id, bytes) in &bundle.blobs {
            graph.offer_blob(id, bytes.clone())?;
        }
        for (id, manifest) in &bundle.manifests {
            graph.offer_manifest(id, manifest.clone())?;
        }
        let bytes = bundle.blobs.values().map(Vec::len).sum::<usize>()
            + bundle
                .manifests
                .values()
                .map(|m| serde_json::to_vec(m).map(|v| v.len()))
                .collect::<std::result::Result<Vec<_>, _>>()?
                .iter()
                .sum::<usize>();
        for (id, bytes) in &bundle.blobs {
            store::immutable(&directory.join("blobs").join(format!("{id}.blob")), bytes)?;
        }
        for (id, manifest) in &bundle.manifests {
            store::immutable(
                &directory.join("manifests").join(format!("{id}.json")),
                &serde_json::to_vec(manifest)?,
            )?;
        }
        Ok(SyncTransfer {
            directory: display(&directory),
            identity,
            manifests: bundle.manifests.len(),
            blobs: bundle.blobs.len(),
            bytes,
        })
    }

    /// Receiving only stages content. It cannot overwrite current files,
    /// publish another machine's private drafts, or grant hosted permissions.
    pub fn read_sync_directory(
        &self,
        identity: &SyncIdentity,
        directory: impl AsRef<Path>,
    ) -> Result<SyncStatus> {
        self.sync_local_owner()?;
        // Validate explicit pairing even for an empty or malformed directory.
        ReplicaGraph::new(&identity.workspace_id, &identity.genesis)?;
        let directory = self.sync_transport_path(directory.as_ref())?;
        let _guards = transport_guards(&directory, identity, false)?;
        let graph = store::load_graph(&directory, identity)?;
        self.receive_sync(
            identity,
            SyncBundle {
                manifests: graph.manifests,
                blobs: graph.blobs,
            },
        )
    }

    fn sync_transport_path(&self, directory: &Path) -> Result<crate::PathBuf> {
        let directory = absolute_path(directory)?;
        let overlaps = |other: &Path| directory.starts_with(other) || other.starts_with(&directory);
        if directory
            .ancestors()
            .any(|p| p.join(".weftext-workspace.json").exists() || p.join("instance.lock").exists())
            || overlaps(&self.data_dir)
            || self.operations.values().any(|p| {
                self.committed(&p.operation_id)
                    && matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                    && overlaps(Path::new(&p.path))
            })
        {
            return Err(CoreError::new(
                "syncTargetOverlap",
                UiMessage::error_core_sync_transport_must_outside_core_s_private_state_and_working_libraries(),
            ));
        }
        Ok(directory)
    }
}

fn transport_guards(
    path: &Path,
    identity: &SyncIdentity,
    create: bool,
) -> Result<Vec<ParentGuard>> {
    let mut guards = Vec::new();
    // A new transport has one explicit family binding. Never annex an existing
    // unpaired user directory, even if it happens to be empty.
    if create && !path.exists() {
        let parent = ParentGuard::acquire(path)?;
        let handle = parent.create_directory()?;
        guards.push(ParentGuard::acquire(&path.join(".transport-root"))?);
        store::immutable(&path.join("identity.json"), &serde_json::to_vec(identity)?)?;
        drop(handle);
        guards.push(parent);
    } else {
        guards.push(ParentGuard::acquire(&path.join(".transport-root"))?);
        let stored: SyncIdentity =
            serde_json::from_slice(&store::read_record(&path.join("identity.json"), 1024)?)?;
        if &stored != identity {
            return Err(invalid(
                UiMessage::error_core_sync_transport_paired_family_mismatch(),
            ));
        }
    }
    let (entries, complete) = guards[0].directory_entries(32)?;
    if !complete
        || entries.iter().any(|(name, _)| {
            name.to_str().is_none_or(|n| {
                !matches!(n, "identity.json" | "blobs" | "manifests") && !n.ends_with(".pending")
            })
        })
    {
        return Err(invalid(
            UiMessage::error_core_sync_transport_unrelated_entries(),
        ));
    }
    for name in ["blobs", "manifests"] {
        let directory = path.join(name);
        if create && !directory.exists() {
            let parent = ParentGuard::acquire(&directory)?;
            let handle = parent.create_directory()?;
            guards.push(ParentGuard::acquire(&directory.join(".transport-records"))?);
            drop(handle);
            guards.push(parent);
        } else {
            guards.push(ParentGuard::acquire(&directory.join(".transport-records"))?);
        }
    }
    Ok(guards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperationIntent, OperationRequest, SaveRequest, fresh};
    use std::fs;

    #[test]
    fn real_directory_exchange_is_separate_staged_and_partial_then_applies_via_original_plan() {
        let temp = tempfile::tempdir().unwrap();
        let ar = temp.path().join("a-library");
        let br = temp.path().join("b-library");
        let transport = temp.path().join("transport");
        let mut a = Core::new(temp.path().join("a-private")).unwrap();
        a.create_library(&ar).unwrap();
        let note = a.create_note_fixture(&ar, &ar, "Note").unwrap();
        a.create_note_fixture(&ar, &ar, "NotSelected").unwrap();
        a.persist_draft(&note.session_id, "PRIVATE UNSAVED DRAFT")
            .unwrap();
        let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
        let keys = BTreeSet::from([ObjectKey::Node(note.node_id.unwrap())]);
        let first = a.write_sync_directory(&ar, &transport, &keys).unwrap();
        assert_eq!(
            first.manifests,
            a.write_sync_directory(&ar, &transport, &keys)
                .unwrap()
                .manifests
        );
        let mut b = Core::new(temp.path().join("b-private")).unwrap();
        let status = b.read_sync_directory(&identity, &transport).unwrap();
        assert_eq!(status.objects.len(), 1);
        assert!(!br.exists());
        let preview = b
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&br),
                intent: OperationIntent::CloneSync {
                    identity: identity.clone(),
                    objects: keys.clone(),
                },
            })
            .unwrap();
        b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        let opened = b.open(br.join("Note/Note.typ"), Some(br.clone())).unwrap();
        assert!(b.read_draft(&br.join("Note/Note.typ")).unwrap().is_none());
        let edited = format!("{}\nreceiver offline save", opened.source);
        b.save(SaveRequest {
            session_id: opened.session_id,
            base_id: opened.base_id,
            operation_id: fresh(),
            source: edited.clone(),
        })
        .unwrap();
        b.write_sync_directory(&br, &transport, &keys).unwrap();
        a.read_sync_directory(&identity, &transport).unwrap();
        assert_eq!(
            fs::read_to_string(ar.join("Note/Note.typ")).unwrap(),
            note.source
        );
        let preview = a
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&ar),
                intent: OperationIntent::ApplySync {
                    identity: identity.clone(),
                    objects: keys.clone(),
                },
            })
            .unwrap();
        a.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(
            fs::read_to_string(ar.join("Note/Note.typ")).unwrap(),
            edited
        );
        assert_eq!(
            a.read_draft(&ar.join("Note/Note.typ"))
                .unwrap()
                .unwrap()
                .source,
            "PRIVATE UNSAVED DRAFT"
        );
        assert!(!br.join("NotSelected").exists());
        assert_eq!(
            a.with_actor("reader", |c| c.write_sync_directory(&ar, &transport, &keys))
                .unwrap_err()
                .code,
            "syncOwnerRequired"
        );
        assert_eq!(
            b.with_actor("reader", |c| c.read_sync_directory(&identity, &transport))
                .unwrap_err()
                .code,
            "syncOwnerRequired"
        );
    }

    #[test]
    fn corrupt_wrong_family_unpaired_and_author_directories_are_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let data = temp.path().join("private");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
        let keys = BTreeSet::from([ObjectKey::Node(note.node_id.unwrap())]);
        for path in [&root, &root.join("transport"), &data, temp.path()] {
            assert_eq!(
                core.write_sync_directory(&root, path, &keys)
                    .unwrap_err()
                    .code,
                "syncTargetOverlap"
            );
        }
        let unpaired = temp.path().join("unpaired");
        fs::create_dir(&unpaired).unwrap();
        fs::write(unpaired.join("sentinel.txt"), "owned elsewhere").unwrap();
        assert!(core.write_sync_directory(&root, &unpaired, &keys).is_err());
        assert_eq!(
            fs::read_to_string(unpaired.join("sentinel.txt")).unwrap(),
            "owned elsewhere"
        );
        assert!(!unpaired.join("identity.json").exists());
        let transport = temp.path().join("transport");
        core.write_sync_directory(&root, &transport, &keys).unwrap();
        let wrong = SyncIdentity {
            workspace_id: fresh(),
            genesis: identity.genesis.clone(),
        };
        assert!(core.read_sync_directory(&wrong, &transport).is_err());
        let file = fs::read_dir(transport.join("blobs"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(&file, "corrupted").unwrap();
        assert!(core.read_sync_directory(&identity, &transport).is_err());
        assert!(core.write_sync_directory(&root, &transport, &keys).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "corrupted");
        assert_eq!(
            fs::read_to_string(root.join("Note/Note.typ")).unwrap(),
            note.source
        );
    }
}
