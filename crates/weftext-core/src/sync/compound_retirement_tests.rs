//! Existing-replica retirement and restoration through original public writers.
//! Missing-replica Clone and metadata-only member qualification are separate.
use super::*;
use crate::backend::ParentGuard;
use crate::operations::{CapturedFile, Mutation, RelocationStep, capture_optional};
use crate::{
    Core, FileIdentity, OperationIntent, OperationPreview, OperationReceipt, OperationRequest,
    SaveRequest, display, fresh,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Actual {
    Directory(FileIdentity),
    File(FileIdentity, Vec<u8>),
}
fn directory_identity(path: &Path) -> FileIdentity {
    ParentGuard::acquire(
        &crate::BackendContext::local().unwrap(),
        &path.join(".retirement-probe"),
    )
    .unwrap()
    .directory_identity()
    .unwrap()
}
fn capture(path: &Path) -> CapturedFile {
    capture_optional(&crate::BackendContext::local().unwrap(), path)
        .unwrap()
        .unwrap()
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Actual> {
    fn visit(root: &Path, path: &Path, found: &mut BTreeMap<PathBuf, Actual>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        if metadata.is_dir() {
            found.insert(relative, Actual::Directory(directory_identity(path)));
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), found);
            }
        } else {
            let file = capture(path);
            found.insert(relative, Actual::File(file.stamp.identity, file.bytes));
        }
    }
    let mut found = BTreeMap::new();
    visit(root, root, &mut found);
    found
}
fn request(root: &Path, intent: OperationIntent) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent,
    }
}
fn run(core: &mut Core, root: &Path, intent: OperationIntent) -> OperationReceipt {
    let preview = core.prepare_operation(request(root, intent)).unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap()
}
fn source(root: &Path, relative: &Path) -> PathBuf {
    root.join(relative).join(format!(
        "{}.typ",
        relative.file_name().unwrap().to_str().unwrap()
    ))
}
fn save(core: &mut Core, root: &Path, relative: &Path, text: &str) {
    let doc = core
        .open(source(root, relative), Some(root.to_path_buf()))
        .unwrap();
    core.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        operation_id: fresh(),
        source: format!("{}\n{text}\n", doc.source),
    })
    .unwrap();
}

struct Evidence {
    id: String,
    directory: FileIdentity,
    source: FileIdentity,
    resource: CapturedFile,
    resource_ref: String,
    manifest: CapturedFile,
    raw: CapturedFile,
    drafts: BTreeMap<String, String>,
    history: BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug)]
enum Mode {
    Multiple,
    Mixed,
    RestoreMixed,
}
struct Fixture {
    temp: tempfile::TempDir,
    a: Core,
    b: Core,
    ar: PathBuf,
    br: PathBuf,
    bd: PathBuf,
    identity: SyncIdentity,
    keys: BTreeSet<ObjectKey>,
    nodes: Vec<Evidence>,
    sender_operations: usize,
    earlier_retirement: Option<String>,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let ar = temp.path().join("sender");
        let br = temp.path().join("receiver");
        let bd = temp.path().join("receiver-private");
        let mut a = Core::new(temp.path().join("sender-private")).unwrap();
        a.create_library(&ar).unwrap();
        let mut ids = Vec::new();
        for relative in ["Parent", "Parent/Child", "Other", "Live"] {
            let relative = PathBuf::from(relative);
            let note = a
                .create_note_fixture(
                    &ar,
                    ar.join(relative.parent().unwrap()),
                    relative.file_name().unwrap().to_str().unwrap(),
                )
                .unwrap();
            let imported = a
                .prepare_import_resource(
                    &ar,
                    &note.path,
                    "owned.bin",
                    format!("owned by {}", note.node_id.as_ref().unwrap()).into_bytes(),
                    &fresh(),
                )
                .unwrap();
            a.execute_operation(&imported.operation_id, &imported.plan_id)
                .unwrap();
            ids.push((note.node_id.unwrap(), relative));
        }
        let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
        let keys = a.sync_binding_info(&ar).unwrap().installed_objects;
        let mut b = Core::new(&bd).unwrap();
        b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
            .unwrap();
        run(
            &mut b,
            &br,
            OperationIntent::CloneSync {
                identity: identity.clone(),
                objects: keys.clone(),
            },
        );
        // Both replicas share the receiver's root-note history. The inherited
        // child retains its clean initial checkpoint: saving it here would
        // explicitly depend on the old Parent head and need a separate typed
        // review after Parent's lifecycle changes. Its actual receiver Clone
        // is still a retained source-history record, checked below.
        for (_, relative) in &ids {
            if relative.components().count() == 1 {
                save(&mut b, &br, relative, "receiver retained original history");
            }
        }
        b.publish_sync(&br).unwrap();
        a.receive_sync(&identity, b.export_sync_objects(&identity, &keys).unwrap())
            .unwrap();
        run(
            &mut a,
            &ar,
            OperationIntent::ApplySync {
                identity: identity.clone(),
                objects: keys.clone(),
            },
        );
        for (_, relative) in &ids {
            let doc = a.open(source(&ar, relative), Some(ar.clone())).unwrap();
            if doc.recovered_draft_conflict {
                a.resolve_draft(&doc.session_id, "mergeRecovered", Some(&doc.source))
                    .unwrap();
            }
        }
        let mut nodes = Vec::new();
        for (id, relative) in ids {
            let path = source(&br, &relative);
            let doc = b.open(&path, Some(br.clone())).unwrap();
            let history: BTreeMap<_, _> = b
                .list_history(&doc.session_id)
                .unwrap()
                .versions
                .into_iter()
                .filter(|version| version.status == "available")
                .map(|version| {
                    let text = b
                        .read_history(&doc.session_id, &version.version_id)
                        .unwrap()
                        .source;
                    (version.version_id, text)
                })
                .collect();
            assert!(
                !history.is_empty(),
                "{id}: the actual receiver Clone/Save must retain source history"
            );
            let raw_path = br.join(&relative).join("personal.raw");
            fs::write(&raw_path, format!("receiver private original {id}")).unwrap();
            let mut drafts = BTreeMap::new();
            for actor in ["alice", "bob"] {
                let proposal = format!(
                    "{}\n{actor} private retirement proposal for {id}",
                    doc.source
                );
                b.with_actor_for_root(actor, &br, |core| {
                    let opened = core.open(&path, Some(br.clone()))?;
                    core.persist_draft(&opened.session_id, &proposal)
                })
                .unwrap();
                drafts.insert(actor.into(), proposal);
            }
            let resource = b.managed_resource_snapshot(&br, &path).unwrap().remove(0);
            nodes.push(Evidence {
                id,
                directory: directory_identity(&br.join(&relative)),
                source: capture(&path).stamp.identity,
                resource: capture(&br.join(&relative).join("owned.bin")),
                resource_ref: resource.resource_ref,
                manifest: capture(&br.join(&relative).join("weftext.resources.json")),
                raw: capture(&raw_path),
                drafts,
                history,
            });
        }
        Self {
            temp,
            a,
            b,
            ar,
            br,
            bd,
            identity,
            keys,
            nodes,
            sender_operations: 0,
            earlier_retirement: None,
        }
    }
    fn exchange(&mut self) {
        self.a.publish_sync(&self.ar).unwrap();
        let before = tree(&self.br);
        self.b
            .receive_sync(
                &self.identity,
                self.a
                    .export_sync_objects(&self.identity, &self.keys)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            tree(&self.br),
            before,
            "Receive may not install author effects"
        );
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let objects = graph.objects().unwrap();
        for (object, view) in &objects {
            assert_eq!(view.versions.len(), 1, "{object:?}");
            let chosen = &view.versions[0];
            let unsettled = graph.unsettled_dependencies(&chosen.commit).unwrap();
            let dependency_heads = graph
                .manifest(&chosen.commit)
                .unwrap()
                .dependencies
                .iter()
                .map(|dependency| {
                    (
                        dependency.object.clone(),
                        dependency.revisions.clone(),
                        objects.get(&dependency.object).map(|view| {
                            view.versions
                                .iter()
                                .map(|version| version.revision.clone())
                                .collect::<Vec<_>>()
                        }),
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                unsettled.is_empty(),
                "{object:?}: name={}, parent={:?}, retired={}, commit={}, unsettled={unsettled:?}, dependencies=(object, original revisions, current heads)={dependency_heads:?}",
                chosen.state.name,
                chosen.state.parent,
                chosen.state.retired,
                chosen.commit,
            );
        }
        self.sender_operations = self.a.operations.len();
    }
    fn produce(&mut self, mode: Mode) {
        run(
            &mut self.a,
            &self.ar,
            OperationIntent::TrashNode {
                source: display(&self.ar.join("Parent")),
            },
        );
        if matches!(mode, Mode::Multiple | Mode::RestoreMixed) {
            run(
                &mut self.a,
                &self.ar,
                OperationIntent::TrashNode {
                    source: display(&self.ar.join("Other")),
                },
            );
        } else {
            run(
                &mut self.a,
                &self.ar,
                OperationIntent::RenameNode {
                    source: display(&self.ar.join("Other")),
                    name: "MovedOther".into(),
                },
            );
            save(
                &mut self.a,
                &self.ar,
                Path::new("Live"),
                "sender live content joins the original retirement Plan",
            );
        }
        self.exchange();
        if matches!(mode, Mode::RestoreMixed) {
            let (req, preview) = self.prepare();
            self.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            self.assert_installed(&req, &preview);
            self.earlier_retirement = Some(preview.operation_id);
            let parent = self.nodes[0].id.clone();
            let entry = self
                .a
                .trash_entries(&self.ar)
                .unwrap()
                .into_iter()
                .find(|entry| entry.node_id == parent)
                .unwrap();
            run(
                &mut self.a,
                &self.ar,
                OperationIntent::RestoreNode {
                    trash_operation_id: entry.operation_id,
                    node_id: Some(parent),
                    parent: display(&self.ar),
                    name: "RestoredParent".into(),
                },
            );
            run(
                &mut self.a,
                &self.ar,
                OperationIntent::RenameNode {
                    source: display(&self.ar.join("Live")),
                    name: "MovedLive".into(),
                },
            );
            self.exchange();
        }
    }
    fn prepare(&mut self) -> (OperationRequest, OperationPreview) {
        let req = request(
            &self.br,
            OperationIntent::ApplySync {
                identity: self.identity.clone(),
                objects: self.keys.clone(),
            },
        );
        let before = tree(&self.br);
        let preview = self.b.prepare_operation(req.clone()).unwrap();
        assert_eq!(
            self.b.prepare_operation(req.clone()).unwrap().plan_id,
            preview.plan_id
        );
        assert_eq!(tree(&self.br), before);
        let transaction = self.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        assert!(
            matches!(transaction.mutation, Mutation::Compound { .. }),
            "Existing roots must share one original Compound decision"
        );
        (req, preview)
    }
    fn steps(&self, preview: &OperationPreview) -> Vec<RelocationStep> {
        let Mutation::Compound { relocations, .. } = &self.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap()
            .mutation
        else {
            unreachable!()
        };
        relocations.clone()
    }
    fn restart(&mut self) {
        let replacement = Core::new(self.temp.path().join(format!("unused-{}", fresh()))).unwrap();
        drop(std::mem::replace(&mut self.b, replacement));
        self.b = Core::new(&self.bd).unwrap();
    }
    fn assert_installed(&mut self, req: &OperationRequest, preview: &OperationPreview) {
        assert!(self.b.committed(&preview.operation_id).unwrap());
        assert_eq!(
            serde_json::to_value(
                self.b
                    .sync_operation_request(&preview.operation_id)
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(req).unwrap()
        );
        let local = self
            .b
            .sync_local_state(self.b.sync_setup_for(&self.br).unwrap())
            .unwrap()
            .0;
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let heads = graph.objects().unwrap();
        for key in &self.keys {
            assert_eq!(local[key].revision, heads[key].versions[0].revision);
            assert_eq!(local[key].state, heads[key].versions[0].state);
        }
        let inherited_retired = |key: &ObjectKey| {
            let mut current = Some(key.clone());
            let mut seen = BTreeSet::new();
            while let Some(key) = current {
                assert!(seen.insert(key.clone()));
                let state = &local[&key].state;
                if state.retired {
                    return true;
                }
                current = state.parent.clone();
            }
            false
        };
        let mut live = BTreeSet::new();
        let mut trash = BTreeSet::new();
        for node in &self.nodes {
            let key = ObjectKey::Node(node.id.clone());
            let installed = &local[&key];
            let directory = Path::new(&installed.directory);
            let canonical = format!("{}.typ", installed.state.name);
            let path = directory.join(&canonical);
            let captured = capture(&path);
            assert_eq!(directory_identity(directory), node.directory);
            assert_eq!(captured.stamp.identity, node.source);
            assert_eq!(
                captured.bytes,
                graph.blob(&installed.state.files[&canonical]).unwrap()
            );
            assert_eq!(
                self.b.accepted_revision(&captured).unwrap().as_ref(),
                installed.state.file_revisions.get(&canonical)
            );
            let resource = capture(&directory.join("owned.bin"));
            assert_eq!(resource.stamp.identity, node.resource.stamp.identity);
            assert_eq!(resource.bytes, node.resource.bytes);
            let manifest = capture(&directory.join("weftext.resources.json"));
            assert_eq!(manifest.stamp.identity, node.manifest.stamp.identity);
            assert_eq!(manifest.bytes, node.manifest.bytes);
            let raw = capture(&directory.join("personal.raw"));
            assert_eq!(raw.stamp.identity, node.raw.stamp.identity);
            assert_eq!(raw.bytes, node.raw.bytes);
            let retired = inherited_retired(&key);
            if retired {
                trash.insert(node.id.clone());
            } else {
                live.insert(node.id.clone());
            }
            let lifecycle = if retired { "trash" } else { "live" };
            let owner = weftext_knowledge::NodeRef {
                workspace_id: self.identity.workspace_id.clone(),
                node_id: node.id.clone(),
            };
            self.b
                .qualified_query_node_at(&self.br, &path, &owner, lifecycle)
                .unwrap();
            let qualified = self
                .b
                .qualified_resource_at(
                    &self.br,
                    &path,
                    &owner,
                    &node.resource_ref,
                    &directory.join("owned.bin"),
                    lifecycle,
                    None,
                )
                .unwrap();
            assert_eq!(qualified.bytes(), node.resource.bytes);
            let opened = self.b.open(&path, Some(self.br.clone())).unwrap();
            assert_eq!(opened.node_id.as_deref(), Some(node.id.as_str()));
            for (version, text) in &node.history {
                assert_eq!(
                    self.b
                        .read_history(&opened.session_id, version)
                        .unwrap()
                        .source,
                    *text
                );
            }
            for (actor, proposal) in &node.drafts {
                self.b
                    .with_actor_for_root(actor, &self.br, |core| {
                        let opened = core.open(&path, Some(self.br.clone()))?;
                        assert_eq!(opened.recovered_draft.as_deref(), Some(proposal.as_str()));
                        assert_eq!(core.read_draft(&path)?.unwrap().source, *proposal);
                        Ok(())
                    })
                    .unwrap();
            }
        }
        let live_snapshot = self.b.admitted_snapshot(&self.br).unwrap();
        assert!(live_snapshot.complete);
        assert_eq!(
            live_snapshot
                .membership_ids
                .into_iter()
                .collect::<BTreeSet<_>>(),
            live
        );
        let trash_snapshot = self.b.trash_admitted_snapshot(&self.br).unwrap();
        assert!(trash_snapshot.complete);
        assert_eq!(
            trash_snapshot
                .membership_ids
                .into_iter()
                .collect::<BTreeSet<_>>(),
            trash
        );
        let roots = local
            .iter()
            .filter_map(|(key, object)| match key {
                ObjectKey::Node(id) if object.state.retired => Some(id.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let entries = self.b.trash_entries(&self.br).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.node_id.clone())
                .collect::<BTreeSet<_>>(),
            roots
        );
        for entry in entries {
            let expected = if let Some(earlier) = &self.earlier_retirement
                && entry.node_id == self.nodes[2].id
            {
                earlier
            } else {
                &preview.operation_id
            };
            assert_eq!(&entry.operation_id, expected);
        }
        assert_eq!(
            self.b
                .sync_binding_info(&self.br)
                .unwrap()
                .pending_publications,
            0
        );
        assert_eq!(self.a.operations.len(), self.sender_operations);
    }
}

fn positive(mode: Mode) {
    let mut f = Fixture::new();
    f.produce(mode);
    let (req, preview) = f.prepare();
    let receipt =
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    f.assert_installed(&req, &preview);
    let installed = tree(&f.br);
    f.restart();
    assert_eq!(tree(&f.br), installed);
    f.assert_installed(&req, &preview);
    assert_eq!(
        serde_json::to_value(
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    assert_eq!(tree(&f.br), installed);
}
#[test]
fn compound_retirement_multiple_existing_roots_preserve_original_objects_and_inherited_lifecycle() {
    positive(Mode::Multiple);
}
#[test]
fn compound_retirement_mixed_existing_root_move_and_content_share_original_decision() {
    positive(Mode::Mixed);
}
#[test]
fn compound_retirement_restore_and_live_move_preserve_original_receiver_trash_ownership() {
    positive(Mode::RestoreMixed);
}

#[test]
fn compound_retirement_original_install_namespace_content_and_facts_checkpoints_recover_same_plan()
{
    for stage in ["installed", "firstMove", "secondMove", "content0", "facts"] {
        let mut f = Fixture::new();
        f.produce(Mode::Mixed);
        let (req, preview) = f.prepare();
        let steps = f.steps(&preview);
        assert_eq!(steps.len(), 2);
        let point = match stage {
            "firstMove" => "move0",
            "secondMove" => "move1",
            _ => stage,
        };
        let original = serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap();
        let before = tree(&f.br);
        crate::operations::compound::set_compound_failpoint(point);
        let error =
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap_err();
        assert_eq!(error.code, "recoveryPending", "{point}: {error:?}");
        assert!(
            error.message.contains("underlying testInterrupted:"),
            "{point}: {error:?}"
        );
        let install = f.b.journal_path(&preview.operation_id, "install");
        let marker = fs::read(&install).unwrap();
        let interrupted = tree(&f.br);
        f.restart();
        assert_eq!(tree(&f.br), interrupted);
        assert_eq!(fs::read(&install).unwrap(), marker);
        assert_eq!(
            serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap(),
            original
        );
        let info = f.b.inspect_recovery(&preview.operation_id).unwrap();
        assert!(
            !info
                .allowed_actions
                .iter()
                .any(|action| action == "keepCurrent")
                || (interrupted == before && info.coherence == "knownBefore"),
            "{point}: {}",
            serde_json::to_string(&info).unwrap()
        );
        let action = if info.coherence == "knownAfter" {
            "adoptAfterImages"
        } else {
            "resumeOriginalPlan"
        };
        assert!(
            info.allowed_actions.iter().any(|allowed| allowed == action),
            "{point}: {}",
            serde_json::to_string(&info).unwrap()
        );
        f.b.resolve_recovery(&preview.operation_id, action).unwrap();
        f.assert_installed(&req, &preview);
        let installed = tree(&f.br);
        f.restart();
        f.assert_installed(&req, &preview);
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(tree(&f.br), installed);
    }
}

#[test]
fn compound_retirement_late_exact_or_case_alias_foreign_target_refuses_without_any_first_effect() {
    for alias in [false, true] {
        let mut f = Fixture::new();
        f.produce(Mode::Mixed);
        let (_, preview) = f.prepare();
        let retired = f
            .steps(&preview)
            .into_iter()
            .find(|step| step.source_identity == f.nodes[0].directory)
            .unwrap();
        let target = PathBuf::from(retired.target);
        let occupied = if alias {
            target.with_file_name(
                target
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_ascii_uppercase(),
            )
        } else {
            target.clone()
        };
        fs::create_dir_all(&occupied).unwrap();
        fs::write(
            occupied.join("foreign.raw"),
            b"original foreign target owner",
        )
        .unwrap();
        let current = tree(&f.br);
        let original = serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap();
        assert!(
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err()
        );
        assert!(!f.b.journal_path(&preview.operation_id, "install").exists());
        assert_eq!(tree(&f.br), current);
        f.restart();
        let info = f.b.inspect_recovery(&preview.operation_id).unwrap();
        assert!(
            info.allowed_actions.is_empty(),
            "{}",
            serde_json::to_string(&info).unwrap()
        );
        for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
            assert!(f.b.resolve_recovery(&preview.operation_id, action).is_err());
        }
        assert_eq!(tree(&f.br), current);
        assert_eq!(
            serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap(),
            original
        );
        assert!(!f.b.committed(&preview.operation_id).unwrap());
    }
}
