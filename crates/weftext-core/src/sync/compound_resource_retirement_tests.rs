//! A real sender removes registered Resource leaves; an existing receiver
//! preserves their original physical files in the same frozen Compound Plan.
use super::*;
use crate::backend::ParentGuard;
use crate::operations::{CapturedFile, Mutation, RetiredMember, capture_optional};
use crate::{
    Core, FileIdentity, OperationIntent, OperationPreview, OperationRequest, SaveRequest, display,
    fresh,
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
    ParentGuard::acquire(&path.join(".resource-retirement-probe"))
        .unwrap()
        .directory_identity()
        .unwrap()
}
fn capture(path: &Path) -> CapturedFile {
    capture_optional(path).unwrap().unwrap()
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Actual> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Actual>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        if metadata.is_dir() {
            result.insert(relative, Actual::Directory(directory_identity(path)));
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), result);
            }
        } else {
            let observed = capture(path);
            result.insert(
                relative,
                Actual::File(observed.stamp.identity, observed.bytes),
            );
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
fn request(root: &Path, intent: OperationIntent) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent,
    }
}
fn run(core: &mut Core, root: &Path, intent: OperationIntent) {
    let preview = core.prepare_operation(request(root, intent)).unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}
fn save(core: &mut Core, root: &Path, directory: &str, suffix: &str) {
    let doc = core
        .open(
            root.join(directory).join(format!("{directory}.typ")),
            Some(root.to_path_buf()),
        )
        .unwrap();
    core.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        operation_id: fresh(),
        source: format!("{}\n{suffix}\n", doc.source),
    })
    .unwrap();
}
struct Asset {
    name: String,
    id: String,
    reference: String,
    revision: String,
    original: CapturedFile,
}
struct Owner {
    id: String,
    name: String,
    directory: FileIdentity,
    source: FileIdentity,
    manifest: FileIdentity,
    raw: CapturedFile,
    removed: Vec<Asset>,
    keep: Asset,
    history: BTreeMap<String, String>,
    drafts: BTreeMap<String, String>,
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
    owners: Vec<Owner>,
    sender_originals: Vec<CapturedFile>,
    sender_operations: usize,
}
impl Fixture {
    fn new(mixed: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let ar = temp.path().join("sender");
        let br = temp.path().join("receiver");
        let bd = temp.path().join("receiver-private");
        let mut a = Core::new(temp.path().join("sender-private")).unwrap();
        a.create_library(&ar).unwrap();
        let mut ids = Vec::new();
        for (name, assets) in [
            ("Left", vec!["asset-a.bin", "asset-b.bin", "keep.bin"]),
            ("Right", vec!["asset-c.bin", "keep.bin"]),
        ] {
            let note = a.create_note_fixture(&ar, &ar, name).unwrap();
            for asset in assets {
                let imported = a
                    .prepare_import_resource(
                        &ar,
                        &note.path,
                        asset,
                        format!("original {name}/{asset} 中文").into_bytes(),
                        &fresh(),
                    )
                    .unwrap();
                a.execute_operation(&imported.operation_id, &imported.plan_id)
                    .unwrap();
            }
            ids.push((note.node_id.unwrap(), name.to_owned()));
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
        for (_, name) in &ids {
            save(&mut b, &br, name, "receiver original saved input");
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
        for (_, name) in &ids {
            let doc = a
                .open(ar.join(name).join(format!("{name}.typ")), Some(ar.clone()))
                .unwrap();
            if doc.recovered_draft_conflict {
                a.resolve_draft(&doc.session_id, "mergeRecovered", Some(&doc.source))
                    .unwrap();
            }
        }
        let mut owners = Vec::new();
        for (id, name) in ids {
            let path = br.join(&name).join(format!("{name}.typ"));
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
            assert!(!history.is_empty());
            let raw = br.join(&name).join("personal.raw");
            fs::write(&raw, format!("private receiver input {id}")).unwrap();
            let mut drafts = BTreeMap::new();
            for actor in ["alice", "bob"] {
                let proposal = format!(
                    "{}\n{actor} original Resource-retirement proposal for {id}",
                    doc.source
                );
                b.with_actor_for_root(actor, &br, |core| {
                    let opened = core.open(&path, Some(br.clone()))?;
                    core.persist_draft(&opened.session_id, &proposal)
                })
                .unwrap();
                drafts.insert(actor.into(), proposal);
            }
            let mut assets = b
                .managed_resource_snapshot(&br, &path)
                .unwrap()
                .into_iter()
                .map(|asset| Asset {
                    name: Path::new(&asset.path)
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .into(),
                    id: asset.resource_id,
                    reference: asset.resource_ref,
                    revision: asset.revision.unwrap(),
                    original: capture(Path::new(&asset.path)),
                })
                .collect::<Vec<_>>();
            let keep = assets.remove(
                assets
                    .iter()
                    .position(|asset| asset.name == "keep.bin")
                    .unwrap(),
            );
            owners.push(Owner {
                id,
                directory: directory_identity(&br.join(&name)),
                source: capture(&path).stamp.identity,
                manifest: capture(&br.join(&name).join("weftext.resources.json"))
                    .stamp
                    .identity,
                raw: capture(&raw),
                name,
                removed: assets,
                keep,
                history,
                drafts,
            });
        }
        let mut fixture = Self {
            temp,
            a,
            b,
            ar,
            br,
            bd,
            identity,
            keys,
            owners,
            sender_originals: vec![],
            sender_operations: 0,
        };
        fixture.remove_adopt_and_receive(mixed);
        fixture
    }
    fn remove_adopt_and_receive(&mut self, mixed: bool) {
        for owner in &self.owners {
            for asset in &owner.removed {
                let source = self.ar.join(&owner.name).join(&asset.name);
                let original = capture(&source);
                let retained = self
                    .temp
                    .path()
                    .join(format!("sender-original-{}-{}", owner.name, asset.name));
                fs::rename(&source, &retained).unwrap();
                let retained = capture(&retained);
                assert_eq!(retained.stamp.identity, original.stamp.identity);
                assert_eq!(retained.bytes, original.bytes);
                self.sender_originals.push(retained);
            }
            let manifest = self.ar.join(&owner.name).join("weftext.resources.json");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
            value["resources"].as_array_mut().unwrap().retain(|entry| {
                !owner
                    .removed
                    .iter()
                    .any(|asset| entry["path"].as_str() == Some(asset.name.as_str()))
            });
            fs::write(manifest, serde_json::to_vec(&value).unwrap()).unwrap();
        }
        let cut = self.a.read_sync_adoption_cut(&self.ar, &self.keys).unwrap();
        assert_eq!(cut.removed_members.len(), 3);
        let adopt = self
            .a
            .prepare_operation(request(
                &self.ar,
                OperationIntent::AdoptSyncExternal {
                    identity: cut.identity,
                    expected_installed: cut.expected_installed,
                    changes: cut.changes,
                    removed_members: cut.removed_members,
                },
            ))
            .unwrap();
        self.a
            .execute_operation(&adopt.operation_id, &adopt.plan_id)
            .unwrap();
        if mixed {
            run(
                &mut self.a,
                &self.ar,
                OperationIntent::RenameNode {
                    source: display(&self.ar.join("Left")),
                    name: "MovedLeft".into(),
                },
            );
            save(
                &mut self.a,
                &self.ar,
                "Right",
                "sender changed body shares the Resource retirement decision",
            );
        }
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
        assert_eq!(tree(&self.br), before);
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let objects = graph.objects().unwrap();
        for (object, view) in &objects {
            assert_eq!(view.versions.len(), 1, "{object:?}");
            let chosen = &view.versions[0];
            assert!(
                graph
                    .unsettled_dependencies(&chosen.commit)
                    .unwrap()
                    .is_empty(),
                "{object:?}, name={}, manifest={:?}",
                chosen.state.name,
                graph.manifest(&chosen.commit).unwrap().dependencies
            );
        }
        self.sender_operations = self.a.operations.len();
    }
    fn prepare(&mut self) -> (OperationRequest, OperationPreview, Vec<RetiredMember>) {
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
        assert!(matches!(transaction.mutation, Mutation::Compound { .. }));
        let retired = transaction.retired_members().to_vec();
        assert_eq!(retired.len(), 3);
        let mut identities = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for member in &retired {
            let (owner, asset) = self.asset(member);
            assert_eq!(member.object, ObjectKey::Node(owner.id.clone()));
            assert_eq!(member.relative_path, asset.name);
            assert_eq!(member.identity, asset.original.stamp.identity);
            assert_eq!(member.parent, owner.directory);
            assert_eq!(member.digest, hash(&asset.original.bytes));
            assert_eq!(member.file_revision, asset.revision);
            assert_eq!(
                capture(Path::new(&member.source)).stamp,
                asset.original.stamp
            );
            assert!(!Path::new(&member.target).exists());
            assert!(identities.insert(serde_json::to_string(&member.identity).unwrap()));
            assert!(targets.insert(member.target.clone()));
        }
        (req, preview, retired)
    }
    fn asset(&self, member: &RetiredMember) -> (&Owner, &Asset) {
        self.owners
            .iter()
            .find_map(|owner| {
                owner
                    .removed
                    .iter()
                    .find(|asset| asset.id == member.resource_id)
                    .map(|asset| (owner, asset))
            })
            .unwrap()
    }
    fn restart(&mut self) {
        let replacement = Core::new(self.temp.path().join(format!("unused-{}", fresh()))).unwrap();
        drop(std::mem::replace(&mut self.b, replacement));
        self.b = Core::new(&self.bd).unwrap();
    }
    fn draft_records(&mut self) -> BTreeMap<PathBuf, BTreeMap<PathBuf, Actual>> {
        let mut result = BTreeMap::new();
        for actor in ["alice", "bob"] {
            let directory = self
                .b
                .with_actor_for_root(actor, &self.br, |core| Ok(core.draft_directory("drafts")))
                .unwrap();
            result.insert(directory.clone(), tree(&directory));
        }
        result
    }
    fn assert_sender_originals(&self) {
        for original in &self.sender_originals {
            let observed = capture(Path::new(&original.path));
            assert_eq!(observed.stamp.identity, original.stamp.identity);
            assert_eq!(observed.bytes, original.bytes);
        }
        assert_eq!(self.a.operations.len(), self.sender_operations);
    }
    fn assert_installed(
        &mut self,
        req: &OperationRequest,
        preview: &OperationPreview,
        retired: &[RetiredMember],
    ) {
        assert!(self.b.committed(&preview.operation_id));
        assert_eq!(
            serde_json::to_value(
                self.b
                    .sync_operation_request(&preview.operation_id)
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(req).unwrap()
        );
        let transaction = self.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap()
            .clone();
        let local = self
            .b
            .sync_local_state(self.b.sync_setup_for(&self.br).unwrap())
            .unwrap()
            .0;
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let objects = graph.objects().unwrap();
        for object in &self.keys {
            assert_eq!(local[object].revision, objects[object].versions[0].revision);
            assert_eq!(local[object].state, objects[object].versions[0].state);
        }
        for member in retired {
            let (_, asset) = self.asset(member);
            let preserved = capture(Path::new(&member.target));
            assert_eq!(preserved.stamp, asset.original.stamp);
            assert_eq!(preserved.bytes, asset.original.bytes);
            assert_eq!(
                self.b.accepted_revision(&preserved).as_ref(),
                Some(&member.file_revision)
            );
            assert!(!Path::new(&transaction.relocated_path(&member.source)).exists());
        }
        for owner in &self.owners {
            let installed = &local[&ObjectKey::Node(owner.id.clone())];
            assert!(!installed.state.retired);
            let directory = Path::new(&installed.directory);
            assert_eq!(directory_identity(directory), owner.directory);
            let canonical = format!("{}.typ", installed.state.name);
            let path = directory.join(&canonical);
            let current = capture(&path);
            assert_eq!(current.stamp.identity, owner.source);
            assert_eq!(
                current.bytes,
                graph.blob(&installed.state.files[&canonical]).unwrap()
            );
            assert_eq!(
                self.b.accepted_revision(&current).as_ref(),
                installed.state.file_revisions.get(&canonical)
            );
            assert_eq!(
                capture(&directory.join("weftext.resources.json"))
                    .stamp
                    .identity,
                owner.manifest
            );
            let raw = capture(&directory.join("personal.raw"));
            assert_eq!(raw.stamp.identity, owner.raw.stamp.identity);
            assert_eq!(raw.bytes, owner.raw.bytes);
            let resources = self.b.managed_resource_snapshot(&self.br, &path).unwrap();
            assert_eq!(resources.len(), 1);
            assert_eq!(resources[0].resource_ref, owner.keep.reference);
            assert_eq!(
                resources[0].revision.as_deref(),
                Some(owner.keep.revision.as_str())
            );
            let keep = capture(Path::new(&resources[0].path));
            assert_eq!(keep.stamp, owner.keep.original.stamp);
            assert_eq!(keep.bytes, owner.keep.original.bytes);
            let node = weftext_knowledge::NodeRef {
                workspace_id: self.identity.workspace_id.clone(),
                node_id: owner.id.clone(),
            };
            self.b
                .qualified_query_node_at(&self.br, &path, &node, "live")
                .unwrap();
            for asset in &owner.removed {
                assert!(!installed.state.files.contains_key(&asset.name));
                assert!(!directory.join(&asset.name).exists());
                let member = retired
                    .iter()
                    .find(|member| member.resource_id == asset.id)
                    .unwrap();
                assert!(
                    self.b
                        .qualified_resource_at(
                            &self.br,
                            &path,
                            &node,
                            &asset.reference,
                            Path::new(&member.target),
                            "live",
                            None
                        )
                        .is_err(),
                    "Preservation is not current owner Resource admission"
                );
            }
            let opened = self.b.open(&path, Some(self.br.clone())).unwrap();
            assert_eq!(opened.node_id.as_deref(), Some(owner.id.as_str()));
            for (version, text) in &owner.history {
                assert_eq!(
                    self.b
                        .read_history(&opened.session_id, version)
                        .unwrap()
                        .source,
                    *text
                );
            }
            for (actor, proposal) in &owner.drafts {
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
        let live = self.b.admitted_snapshot(&self.br).unwrap();
        assert!(live.complete);
        assert_eq!(
            live.membership_ids.into_iter().collect::<BTreeSet<_>>(),
            self.owners.iter().map(|owner| owner.id.clone()).collect()
        );
        let trash = self.b.trash_admitted_snapshot(&self.br).unwrap();
        assert!(trash.complete);
        assert!(
            trash.membership_ids.is_empty(),
            "Resource retention must not invent retired Nodes"
        );
        assert!(self.b.trash_entries(&self.br).unwrap().is_empty());
        assert_eq!(
            self.b
                .sync_binding_info(&self.br)
                .unwrap()
                .pending_publications,
            0
        );
        self.assert_sender_originals();
    }
}
fn positive(mixed: bool) {
    let mut f = Fixture::new(mixed);
    let (req, preview, retired) = f.prepare();
    let receipt =
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    f.assert_installed(&req, &preview, &retired);
    let installed = tree(&f.br);
    let drafts = f.draft_records();
    f.restart();
    assert_eq!(tree(&f.br), installed);
    f.assert_installed(&req, &preview, &retired);
    assert_eq!(f.draft_records(), drafts);
    let replay =
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    assert_eq!(tree(&f.br), installed);
    assert_eq!(f.draft_records(), drafts);
}
#[test]
fn compound_resource_retirement_real_multi_owner_members_preserve_original_files_without_current_role()
 {
    positive(false);
}
#[test]
fn compound_resource_retirement_mixed_move_content_and_three_members_share_original_decision() {
    positive(true);
}

#[test]
fn compound_resource_retirement_original_moved_file_checkpoints_restart_and_finish_same_plan() {
    for stage in ["installed", "retire0", "retire2", "content", "facts"] {
        let mut f = Fixture::new(true);
        let (req, preview, retired) = f.prepare();
        let point = if stage == "content" {
            let transaction = f.b.operations[&preview.operation_id]
                .transaction
                .as_ref()
                .unwrap();
            let (index, original) = transaction
                .files
                .iter()
                .enumerate()
                .find(|(_, file)| {
                    !transaction.is_retired_member(&file.path)
                        && Path::new(&file.path).file_name()
                            == Some(std::ffi::OsStr::new("Right.typ"))
                })
                .expect("The real sender Save changes the surviving Right source");
            let changes = transaction.relocated_files();
            let path = transaction.relocated_path(&original.path);
            let after = changes
                .iter()
                .find(|change| {
                    crate::backend::namespace_path_compare(
                        Path::new(&change.path),
                        Path::new(&path),
                    )
                    .unwrap()
                        == std::cmp::Ordering::Equal
                })
                .unwrap();
            assert_eq!(after.before.as_deref(), Some(original.bytes.as_slice()));
            assert_ne!(after.after, original.bytes);
            format!("content{index}")
        } else {
            stage.to_owned()
        };
        let original = serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap();
        let before = tree(&f.br);
        let drafts = f.draft_records();
        crate::operations::compound::set_compound_failpoint(&point);
        let error =
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .expect_err(&format!("The original {stage}/{point} hook must interrupt"));
        assert_eq!(error.code, "recoveryPending", "{point}: {error:?}");
        assert!(
            error
                .causes
                .as_slice()
                .iter()
                .any(|cause| cause.code == "testInterrupted"),
            "{point}: {error:?}"
        );
        let install = f.b.journal_path(&preview.operation_id, "install");
        let marker = fs::read(&install).unwrap();
        let interrupted = tree(&f.br);
        if point.starts_with("retire") {
            let last = point
                .strip_prefix("retire")
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let transaction = f.b.operations[&preview.operation_id]
                .transaction
                .as_ref()
                .unwrap();
            for member in retired.iter().take(last + 1) {
                let (_, asset) = f.asset(member);
                assert_eq!(
                    capture(Path::new(&member.target)).stamp,
                    asset.original.stamp
                );
                assert!(!Path::new(&transaction.relocated_path(&member.source)).exists());
            }
            assert_eq!(f.draft_records(), drafts);
        }
        f.restart();
        assert_eq!(tree(&f.br), interrupted);
        assert_eq!(fs::read(&install).unwrap(), marker);
        assert_eq!(
            serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap(),
            original
        );
        let info = f.b.inspect_recovery(&preview.operation_id).unwrap();
        let members = info
            .members
            .iter()
            .filter(|member| member.role == "memberRetirement")
            .collect::<Vec<_>>();
        assert_eq!(
            members.len(),
            retired.len(),
            "{point}: {}",
            serde_json::to_string(&info).unwrap()
        );
        let transaction = f.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        for retired in &retired {
            let member = members
                .iter()
                .find(|member| member.path == transaction.relocated_path(&retired.source))
                .unwrap();
            assert_eq!(member.identity_binding, "original");
            assert_eq!(
                member.before.sha256.as_deref(),
                Some(retired.digest.as_str())
            );
            assert_eq!(
                member.after.sha256.as_deref(),
                Some(retired.digest.as_str())
            );
            assert_eq!(
                member.current_identity.as_ref().unwrap().index,
                retired.identity.index.to_string()
            );
            if point == "retire0" || point == "retire2" {
                let after = Path::new(&retired.target).exists();
                assert_eq!(member.matches_after, after);
                assert_eq!(member.matches_before, !after);
                if after {
                    assert_eq!(member.target_path.as_deref(), Some(retired.target.as_str()));
                }
            }
        }
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
        f.assert_installed(&req, &preview, &retired);
        let installed = tree(&f.br);
        f.restart();
        f.assert_installed(&req, &preview, &retired);
        f.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(tree(&f.br), installed);
    }
}

fn reject_before_install(f: &mut Fixture, preview: &OperationPreview) {
    let current = tree(&f.br);
    let original = serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap();
    let drafts = f.draft_records();
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
    assert_eq!(f.draft_records(), drafts);
    assert_eq!(
        serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap(),
        original
    );
    assert!(!f.b.committed(&preview.operation_id));
    f.assert_sender_originals();
}
#[test]
fn compound_resource_retirement_late_target_alias_or_same_bytes_source_replacement_has_no_first_effect()
 {
    for fault in ["target", "targetAlias", "sourceReplacement"] {
        let mut f = Fixture::new(true);
        let (_, preview, retired) = f.prepare();
        let member = &retired[0];
        let retained = if fault == "sourceReplacement" {
            let source = Path::new(&member.source);
            let retained = f.temp.path().join("receiver-original-resource.bin");
            fs::rename(source, &retained).unwrap();
            fs::copy(&retained, source).unwrap();
            assert_ne!(capture(source).stamp.identity, member.identity);
            Some((retained.clone(), capture(&retained)))
        } else {
            let target = PathBuf::from(&member.target);
            let occupied = if fault == "targetAlias" {
                target.with_file_name(
                    target
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_ascii_uppercase(),
                )
            } else {
                target
            };
            fs::create_dir_all(occupied.parent().unwrap()).unwrap();
            fs::write(&occupied, b"foreign retention-slot owner").unwrap();
            None
        };
        reject_before_install(&mut f, &preview);
        if let Some((path, original)) = retained {
            assert_eq!(capture(&path).stamp.identity, original.stamp.identity);
            assert_eq!(capture(&path).bytes, original.bytes);
        }
    }
}

#[test]
fn compound_resource_retirement_after_first_original_move_rejects_foreign_target_or_source_alias() {
    for source_alias in [false, true] {
        let mut f = Fixture::new(true);
        let (_, preview, retired) = f.prepare();
        let original = serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap();
        let original_files =
            serde_json::to_value(f.b.prepared_operation_files(&preview.operation_id).unwrap())
                .unwrap();
        let drafts = f.draft_records();
        crate::operations::compound::set_compound_failpoint("retire0");
        let error =
            f.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap_err();
        assert_eq!(error.code, "recoveryPending");
        assert!(
            error
                .causes
                .as_slice()
                .iter()
                .any(|cause| cause.code == "testInterrupted")
        );
        let first = capture(Path::new(&retired[0].target));
        assert_eq!(first.stamp.identity, retired[0].identity);
        let transaction = f.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        if source_alias {
            let source = PathBuf::from(transaction.relocated_path(&retired[0].source));
            let alias = source.with_file_name(
                source
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            assert!(!alias.exists());
            fs::copy(&retired[0].target, &alias).unwrap();
            assert_ne!(capture(&alias).stamp.identity, retired[0].identity);
            assert_eq!(capture(&alias).bytes, first.bytes);
        } else {
            assert!(!Path::new(&retired[1].target).exists());
            fs::write(&retired[1].target, b"foreign second preservation owner").unwrap();
        }
        let current = tree(&f.br);
        let install = f.b.journal_path(&preview.operation_id, "install");
        let marker = fs::read(&install).unwrap();
        for _ in 0..2 {
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
            assert_eq!(f.draft_records(), drafts);
            assert_eq!(fs::read(&install).unwrap(), marker);
            assert_eq!(
                serde_json::to_value(&f.b.operations[&preview.operation_id]).unwrap(),
                original
            );
            assert_eq!(
                serde_json::to_value(f.b.prepared_operation_files(&preview.operation_id).unwrap())
                    .unwrap(),
                original_files
            );
            assert_eq!(capture(Path::new(&retired[0].target)).stamp, first.stamp);
            assert_eq!(capture(Path::new(&retired[0].target)).bytes, first.bytes);
            assert!(!f.b.committed(&preview.operation_id));
        }
        f.assert_sender_originals();
    }
}
