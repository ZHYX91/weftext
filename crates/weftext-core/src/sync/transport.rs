//! Same-owner directory transport. This directory carries immutable portable
//! records only. It is neither a working tree nor a copied Core private state.
use super::*;
use crate::{Core, Path, absolute_path, backend::ParentGuard, display};

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
        let _guards = transport_guards(&self.backend, &directory, &identity, true)?;
        // Before any payload write, validate the whole combined bounded store.
        let mut graph = store::load_graph(&self.backend, &directory, &identity)?;
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
            store::immutable(
                &self.backend,
                &directory.join("blobs").join(format!("{id}.blob")),
                bytes,
            )?;
        }
        for (id, manifest) in &bundle.manifests {
            store::immutable(
                &self.backend,
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
        let _guards = transport_guards(&self.backend, &directory, identity, false)?;
        let graph = store::load_graph(&self.backend, &directory, identity)?;
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
        if self.backend.ancestors(&directory)?.into_iter().try_fold(
            false,
            |found, p| -> Result<bool> {
                if found {
                    Ok(true)
                } else {
                    Ok({
                        self.backend
                            .entry_exists(p.join(".weftext-workspace.json"))?
                            || self.backend.entry_exists(p.join("instance.lock"))?
                    })
                }
            },
        )? || overlaps(&self.data_dir)
            || self
                .operations
                .values()
                .try_fold(false, |found, p| -> Result<bool> {
                    if found {
                        Ok(true)
                    } else {
                        Ok({
                            self.committed(&p.operation_id)?
                                && matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                                && overlaps(Path::new(&p.path))
                        })
                    }
                })?
        {
            return Err(CoreError::new(
                "syncTargetOverlap",
                "Transport must be outside this Core's private state and working libraries",
            ));
        }
        Ok(directory)
    }
}

fn transport_guards(
    backend: &crate::BackendContext,
    path: &Path,
    identity: &SyncIdentity,
    create: bool,
) -> Result<Vec<ParentGuard>> {
    let mut guards = Vec::new();
    // A new transport has one explicit family binding. Never annex an existing
    // unpaired user directory, even if it happens to be empty.
    if create && !backend.entry_exists(path)? {
        let parent = ParentGuard::acquire(backend, path)?;
        let handle = parent.create_directory()?;
        guards.push(ParentGuard::acquire(
            backend,
            &path.join(".transport-root"),
        )?);
        store::immutable(
            backend,
            &path.join("identity.json"),
            &serde_json::to_vec(identity)?,
        )?;
        drop(handle);
        guards.push(parent);
    } else {
        guards.push(ParentGuard::acquire(
            backend,
            &path.join(".transport-root"),
        )?);
        let stored: SyncIdentity = serde_json::from_slice(&store::read_record(
            backend,
            &path.join("identity.json"),
            1024,
        )?)?;
        if &stored != identity {
            return Err(invalid(
                "Transport directory belongs to another paired family",
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
        return Err(invalid("Transport directory contains unrelated entries"));
    }
    for name in ["blobs", "manifests"] {
        let directory = path.join(name);
        if create && !backend.entry_exists(&directory)? {
            let parent = ParentGuard::acquire(backend, &directory)?;
            let handle = parent.create_directory()?;
            guards.push(ParentGuard::acquire(
                backend,
                &directory.join(".transport-records"),
            )?);
            drop(handle);
            guards.push(parent);
        } else {
            guards.push(ParentGuard::acquire(
                backend,
                &directory.join(".transport-records"),
            )?);
        }
    }
    Ok(guards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FixtureRoots, copy_transport, open_core};
    use crate::{OperationIntent, OperationRequest, SaveRequest, fresh};
    use std::fs;

    #[test]
    #[cfg_attr(
        target_os = "linux",
        ignore = "requires three explicit administrator peer enrollments; run serially"
    )]
    fn real_directory_exchange_is_separate_staged_and_partial_then_applies_via_original_plan() {
        let temp = FixtureRoots::new();
        let ar = temp.a_path().join("a-library");
        let br = temp.b_path().join("b-library");
        let transport = temp.a_path().join("transport");
        let receiver_transport = temp.b_path().join("transport");
        let mut a = open_core(temp.a_path().join("a-private")).unwrap();
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
        let mut b = open_core(temp.b_path().join("b-private")).unwrap();
        #[cfg(target_os = "linux")]
        assert_eq!(
            b.read_sync_directory(&identity, &transport)
                .unwrap_err()
                .code,
            "outsideRoot"
        );
        copy_transport(&transport, &receiver_transport);
        let status = b
            .read_sync_directory(&identity, &receiver_transport)
            .unwrap();
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
        b.write_sync_directory(&br, &receiver_transport, &keys)
            .unwrap();
        copy_transport(&receiver_transport, &transport);
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
            b.with_actor("reader", |c| c
                .read_sync_directory(&identity, &receiver_transport))
                .unwrap_err()
                .code,
            "syncOwnerRequired"
        );
    }

    #[test]
    #[cfg_attr(
        target_os = "linux",
        ignore = "requires three explicit administrator peer enrollments; run serially"
    )]
    fn corrupt_wrong_family_unpaired_and_author_directories_are_never_overwritten() {
        let temp = FixtureRoots::new();
        let root = temp.a_path().join("library");
        let data = temp.a_path().join("private");
        let mut core = open_core(&data).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
        let keys = BTreeSet::from([ObjectKey::Node(note.node_id.unwrap())]);
        for path in [&root, &root.join("transport"), &data, temp.a_path()] {
            assert_eq!(
                core.write_sync_directory(&root, path, &keys)
                    .unwrap_err()
                    .code,
                "syncTargetOverlap"
            );
        }
        let unpaired = temp.a_path().join("unpaired");
        fs::create_dir(&unpaired).unwrap();
        fs::write(unpaired.join("sentinel.txt"), "owned elsewhere").unwrap();
        assert!(core.write_sync_directory(&root, &unpaired, &keys).is_err());
        assert_eq!(
            fs::read_to_string(unpaired.join("sentinel.txt")).unwrap(),
            "owned elsewhere"
        );
        assert!(!unpaired.join("identity.json").exists());
        let transport = temp.a_path().join("transport");
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
