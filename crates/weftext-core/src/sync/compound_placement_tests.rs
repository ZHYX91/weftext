//! Complex-placement acceptance through real public producers.
//! Every received head is produced by real Core Create/Save/Rename/Move and
//! publish/export/receive operations. No manifest, admission or journal edits.
//! Namespace failpoints interrupt the original writer at its real checkpoints.
use super::*;
use crate::backend::ParentGuard;
use crate::{
    Core, FileIdentity, OperationIntent, OperationRequest, Path, PathBuf, SaveRequest, display,
    fresh,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Clone, Copy, Debug)]
enum Layout {
    Exchange,
    Rotate,
    EmptyChain,
    ExtractChild,
    ExtractDirectoryChild,
    ExtractDirectoryWithNewChild,
    NestedChild,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Actual {
    Directory(FileIdentity),
    File(FileIdentity, Vec<u8>),
}

fn directory_identity(path: &Path) -> FileIdentity {
    ParentGuard::acquire(&path.join(".identity-probe"))
        .unwrap()
        .directory_identity()
        .unwrap()
}

fn file_identity(path: &Path) -> FileIdentity {
    crate::operations::capture_optional(path)
        .unwrap()
        .unwrap()
        .stamp
        .identity
}

fn actual_tree(root: &Path) -> BTreeMap<PathBuf, Actual> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Actual>) {
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        let metadata = fs::symlink_metadata(path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        if metadata.is_dir() {
            result.insert(relative, Actual::Directory(directory_identity(path)));
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), result);
            }
        } else {
            result.insert(
                relative,
                Actual::File(file_identity(path), fs::read(path).unwrap()),
            );
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn original_request(root: &Path, intent: OperationIntent) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent,
    }
}

fn run(core: &mut Core, root: &Path, intent: OperationIntent) {
    let preview = core
        .prepare_operation(original_request(root, intent))
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

fn source_path(root: &Path, directory: &Path) -> PathBuf {
    root.join(directory).join(format!(
        "{}.typ",
        directory.file_name().unwrap().to_str().unwrap()
    ))
}

fn save(core: &mut Core, root: &Path, directory: &Path, suffix: &str) {
    let doc = core
        .open(source_path(root, directory), Some(root.into()))
        .unwrap();
    core.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        operation_id: fresh(),
        source: format!("{}\n{suffix}\n", doc.source),
    })
    .unwrap();
}

fn rename(core: &mut Core, root: &Path, source: &str, name: &str) {
    run(
        core,
        root,
        OperationIntent::RenameNode {
            source: display(&root.join(source)),
            name: name.into(),
        },
    );
}

fn move_node(core: &mut Core, root: &Path, source: &str, parent: &str) {
    run(
        core,
        root,
        OperationIntent::MoveNode {
            source: display(&root.join(source)),
            parent: display(&root.join(parent)),
        },
    );
}

struct Node {
    id: String,
    original: PathBuf,
    final_path: PathBuf,
    directory: FileIdentity,
    source: FileIdentity,
    resource: FileIdentity,
    manifest: FileIdentity,
    raw: FileIdentity,
    opaque_directory: FileIdentity,
    opaque_leaf: FileIdentity,
    resource_ref: String,
    resource_revision: String,
    raw_bytes: Vec<u8>,
    drafts: BTreeMap<String, String>,
    original_sessions: BTreeMap<String, (String, String)>,
    history: BTreeMap<String, String>,
    expected: Vec<u8>,
    received_revision: String,
    received_file_revisions: BTreeMap<String, String>,
}

struct Pair {
    temp: tempfile::TempDir,
    a: Core,
    b: Core,
    ar: PathBuf,
    br: PathBuf,
    bd: PathBuf,
    identity: SyncIdentity,
    selected: BTreeSet<ObjectKey>,
    nodes: Vec<Node>,
    source_operations: usize,
}

impl Pair {
    fn new(layout: Layout) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let ar = temp.path().join("a");
        let br = temp.path().join("b");
        let bd = temp.path().join("b-private");
        let mut a = Core::new(temp.path().join("a-private")).unwrap();
        a.create_library(&ar).unwrap();
        if matches!(
            layout,
            Layout::ExtractDirectoryChild | Layout::ExtractDirectoryWithNewChild
        ) {
            fs::create_dir(ar.join("Parent")).unwrap();
            let admission = a
                .prepare_directory_admission(&ar, ar.join("Parent"), &fresh())
                .unwrap();
            a.execute_operation(&admission.preview.operation_id, &admission.preview.plan_id)
                .unwrap();
        }
        let initial: &[&str] = match layout {
            Layout::Exchange | Layout::EmptyChain => &["Left", "Right", "Still"],
            Layout::Rotate => &["Left", "Middle", "Right", "Still"],
            Layout::ExtractChild => &["Parent", "Parent/Child", "Still"],
            Layout::ExtractDirectoryChild | Layout::ExtractDirectoryWithNewChild => {
                &["Parent/Child", "Still"]
            }
            Layout::NestedChild => &["Parent", "Parent/Child", "Container", "Still"],
        };
        let mut created = Vec::new();
        for relative in initial {
            let directory = PathBuf::from(relative);
            let name = directory.file_name().unwrap().to_str().unwrap();
            let parent = ar.join(directory.parent().unwrap());
            let doc = a.create_note(&ar, &parent, name).unwrap();
            let id = doc.node_id.unwrap();
            save(
                &mut a,
                &ar,
                &directory,
                &format!("A original owner {id} 中文😀"),
            );
            let preview = a
                .prepare_import_resource(
                    &ar,
                    source_path(&ar, &directory),
                    "owned.bin",
                    id.as_bytes().to_vec(),
                    &fresh(),
                )
                .unwrap();
            a.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            created.push((id, directory));
        }
        let binding = a.enable_sync(&ar, &fresh()).unwrap();
        let identity = binding.identity;
        let selected = a.sync_binding_info(&ar).unwrap().installed_objects;
        assert!(selected.contains(&ObjectKey::Rules));
        let mut b = Core::new(&bd).unwrap();
        b.receive_sync(
            &identity,
            a.export_sync_objects(&identity, &selected).unwrap(),
        )
        .unwrap();
        run(
            &mut b,
            &br,
            OperationIntent::CloneSync {
                identity: identity.clone(),
                objects: selected.clone(),
            },
        );

        // Real B Saves establish retained history and a shared causal base.
        for (id, directory) in &created {
            save(
                &mut b,
                &br,
                directory,
                &format!("B retained original history {id}"),
            );
        }
        b.publish_sync(&br).unwrap();
        a.receive_sync(
            &identity,
            b.export_sync_objects(&identity, &selected).unwrap(),
        )
        .unwrap();
        run(
            &mut a,
            &ar,
            OperationIntent::ApplySync {
                identity: identity.clone(),
                objects: selected.clone(),
            },
        );
        for (_, directory) in &created {
            let doc = a
                .open(source_path(&ar, directory), Some(ar.clone()))
                .unwrap();
            if doc.recovered_draft_conflict {
                assert_eq!(doc.recovered_draft, doc.recovered_draft_base_source);
                a.resolve_draft(&doc.session_id, "mergeRecovered", Some(&doc.source))
                    .unwrap();
            }
        }

        let mut nodes = Vec::new();
        for (id, directory) in created {
            let root = br.join(&directory);
            let doc = b
                .open(source_path(&br, &directory), Some(br.clone()))
                .unwrap();
            let timeline = b.list_history(&doc.session_id).unwrap();
            let history = timeline
                .versions
                .iter()
                .filter(|version| version.status == "available")
                .map(|version| {
                    (
                        version.version_id.clone(),
                        b.read_history(&doc.session_id, &version.version_id)
                            .unwrap()
                            .source,
                    )
                })
                .collect::<BTreeMap<_, _>>();
            assert!(!history.is_empty());
            let raw_bytes = format!("B private raw owner {id}").into_bytes();
            fs::write(root.join("personal.raw"), &raw_bytes).unwrap();
            fs::create_dir(root.join("opaque")).unwrap();
            fs::write(root.join("opaque/keep.raw"), &raw_bytes).unwrap();
            let resource = b
                .managed_resource_snapshot(&br, source_path(&br, &directory))
                .unwrap()
                .remove(0);
            assert_eq!(resource.identity_status, "admitted");
            let mut drafts = BTreeMap::new();
            let mut original_sessions = BTreeMap::new();
            for actor in ["alice", "bob"] {
                let source = format!(
                    "{}\n{actor} private placement proposal for {id}",
                    doc.source
                );
                let session = b
                    .with_actor_for_root(actor, &br, |core| {
                        let opened = core.open(source_path(&br, &directory), Some(br.clone()))?;
                        core.persist_draft(&opened.session_id, &source)?;
                        Ok((opened.session_id, opened.base_id))
                    })
                    .unwrap();
                drafts.insert(actor.into(), source);
                original_sessions.insert(actor.into(), session);
            }
            nodes.push(Node {
                id,
                original: directory.clone(),
                final_path: directory.clone(),
                directory: directory_identity(&root),
                source: file_identity(&source_path(&br, &directory)),
                resource: file_identity(&root.join("owned.bin")),
                manifest: file_identity(&root.join("weftext.resources.json")),
                raw: file_identity(&root.join("personal.raw")),
                opaque_directory: directory_identity(&root.join("opaque")),
                opaque_leaf: file_identity(&root.join("opaque/keep.raw")),
                resource_ref: resource.resource_ref,
                resource_revision: resource.revision.unwrap(),
                raw_bytes,
                drafts,
                original_sessions,
                history,
                expected: vec![],
                received_revision: String::new(),
                received_file_revisions: BTreeMap::new(),
            });
        }
        let mut pair = Self {
            temp,
            a,
            b,
            ar,
            br,
            bd,
            identity,
            selected,
            nodes,
            source_operations: 0,
        };
        pair.produce(layout);
        pair
    }

    fn final_path(&mut self, original: &str, final_path: &str) {
        self.nodes
            .iter_mut()
            .find(|node| node.original == Path::new(original))
            .unwrap()
            .final_path = final_path.into();
    }

    fn produce(&mut self, layout: Layout) {
        match layout {
            Layout::Exchange => {
                rename(&mut self.a, &self.ar, "Left", "Holding");
                rename(&mut self.a, &self.ar, "Right", "Left");
                rename(&mut self.a, &self.ar, "Holding", "Right");
                self.final_path("Left", "Right");
                self.final_path("Right", "Left");
            }
            Layout::Rotate => {
                rename(&mut self.a, &self.ar, "Left", "Holding");
                rename(&mut self.a, &self.ar, "Middle", "Left");
                rename(&mut self.a, &self.ar, "Right", "Middle");
                rename(&mut self.a, &self.ar, "Holding", "Right");
                self.final_path("Left", "Right");
                self.final_path("Middle", "Left");
                self.final_path("Right", "Middle");
            }
            Layout::EmptyChain => {
                rename(&mut self.a, &self.ar, "Right", "Empty");
                rename(&mut self.a, &self.ar, "Left", "Right");
                self.final_path("Left", "Right");
                self.final_path("Right", "Empty");
            }
            Layout::ExtractChild => {
                move_node(&mut self.a, &self.ar, "Parent/Child", "");
                rename(&mut self.a, &self.ar, "Parent", "RenamedParent");
                self.final_path("Parent", "RenamedParent");
                self.final_path("Parent/Child", "Child");
            }
            Layout::ExtractDirectoryChild | Layout::ExtractDirectoryWithNewChild => {
                if matches!(layout, Layout::ExtractDirectoryWithNewChild) {
                    let created = self
                        .a
                        .create_note(&self.ar, self.ar.join("Parent"), "NewChild")
                        .unwrap();
                    self.selected
                        .insert(ObjectKey::Node(created.node_id.unwrap()));
                }
                move_node(&mut self.a, &self.ar, "Parent/Child", "");
                run(
                    &mut self.a,
                    &self.ar,
                    OperationIntent::RelocateDirectory {
                        source: display(&self.ar.join("Parent")),
                        parent: display(&self.ar),
                        name: "RenamedParent".into(),
                    },
                );
                self.final_path("Parent/Child", "Child");
                if matches!(layout, Layout::ExtractDirectoryWithNewChild) {
                    save(
                        &mut self.a,
                        &self.ar,
                        Path::new("RenamedParent/NewChild"),
                        "final child reads the actual relocated Directory head",
                    );
                }
            }
            Layout::NestedChild => {
                // Parent first, then child: the latter's real publication reads
                // the final Parent head, without a stale semantic dependency.
                move_node(&mut self.a, &self.ar, "Parent", "Container");
                rename(
                    &mut self.a,
                    &self.ar,
                    "Container/Parent/Child",
                    "RenamedChild",
                );
                self.final_path("Parent", "Container/Parent");
                self.final_path("Parent/Child", "Container/Parent/RenamedChild");
            }
        }
        assert!(!self.ar.join("Holding").exists());
        // A descendant's real Save reads its current parent version. Save the
        // final ancestors first so a later Container Save does not invalidate
        // the already-published Parent/Child dependency cut in this fixture.
        let mut final_sources = self
            .nodes
            .iter()
            .map(|node| (node.final_path.clone(), node.id.clone()))
            .collect::<Vec<_>>();
        final_sources.sort_by_key(|(directory, _)| directory.components().count());
        for (directory, id) in final_sources {
            save(
                &mut self.a,
                &self.ar,
                &directory,
                &format!("A final placement body for {id}"),
            );
        }
        self.a.publish_sync(&self.ar).unwrap();
        let before = actual_tree(&self.br);
        self.b
            .receive_sync(
                &self.identity,
                self.a
                    .export_sync_objects(&self.identity, &self.selected)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            actual_tree(&self.br),
            before,
            "Receive cannot perform any author placement"
        );
        let graph = self.b.read_sync_history(&self.identity).unwrap();
        let current = graph.objects().unwrap();
        for node in &mut self.nodes {
            let view = &current[&ObjectKey::Node(node.id.clone())];
            assert_eq!(
                view.versions.len(),
                1,
                "Real producer must reach a single complete final head"
            );
            let head = &view.versions[0];
            let unsettled = graph.unsettled_dependencies(&head.commit).unwrap();
            let manifest = graph.manifest(&head.commit).unwrap();
            let dependency_heads = manifest
                .dependencies
                .iter()
                .map(|dependency| {
                    (
                        dependency.object.clone(),
                        dependency.revisions.clone(),
                        current.get(&dependency.object).map(|view| {
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
                "Placement fixture must reach the namespace planner: layout={layout:?}, node={}, original={}, final={}, commit={}, unsettled={unsettled:?}, dependencies=(object, original revisions, current revisions)={dependency_heads:?}",
                node.id,
                node.original.display(),
                node.final_path.display(),
                head.commit,
            );
            node.received_revision = view.versions[0].revision.clone();
            node.received_file_revisions = view.versions[0].state.file_revisions.clone();
            node.expected = fs::read(source_path(&self.ar, &node.final_path)).unwrap();
        }
        self.source_operations = self.a.operations.len();
    }

    fn request(&self) -> OperationRequest {
        original_request(
            &self.br,
            OperationIntent::ApplySync {
                identity: self.identity.clone(),
                objects: self.selected.clone(),
            },
        )
    }

    fn prepare(&mut self) -> (OperationRequest, crate::OperationPreview) {
        let request = self.request();
        let before = actual_tree(&self.br);
        let count = self.b.operations.len();
        // This is a required positive. syncStructureRequired or occupied-slot
        // refusal is an unimplemented feature, never a successful negative.
        let preview = self.b.prepare_operation(request.clone()).unwrap();
        assert_eq!(preview.kind, "applySync");
        assert_eq!(self.b.operations.len(), count + 1);
        assert_eq!(
            self.b.prepare_operation(request.clone()).unwrap().plan_id,
            preview.plan_id
        );
        assert_eq!(self.b.operations.len(), count + 1);
        assert_eq!(
            actual_tree(&self.br),
            before,
            "Preparation must preserve all original FIs and bytes"
        );
        assert_eq!(
            self.a.operations.len(),
            self.source_operations,
            "Receiver cannot run new sender operations"
        );
        (request, preview)
    }

    fn restart(&mut self) {
        let replacement = Core::new(self.temp.path().join(format!("unused-{}", fresh()))).unwrap();
        drop(std::mem::replace(&mut self.b, replacement));
        self.b = Core::new(&self.bd).unwrap();
    }

    fn assert_installed(&mut self, request: &OperationRequest, preview: &crate::OperationPreview) {
        assert!(self.b.committed(&preview.operation_id));
        let retained = self
            .b
            .sync_operation_request(&preview.operation_id)
            .unwrap();
        assert_eq!(
            serde_json::to_value(retained).unwrap(),
            serde_json::to_value(request).unwrap()
        );
        assert_eq!(
            self.b
                .operation_details(&preview.operation_id)
                .unwrap()
                .revisions,
            preview.revisions
        );
        let binding = self.b.sync_binding_info(&self.br).unwrap();
        assert_eq!(binding.pending_publications, 0);
        for node in &self.nodes {
            let directory = self.br.join(&node.final_path);
            let source = source_path(&self.br, &node.final_path);
            assert_eq!(directory_identity(&directory), node.directory);
            assert_eq!(file_identity(&source), node.source);
            assert_eq!(file_identity(&directory.join("owned.bin")), node.resource);
            assert_eq!(
                file_identity(&directory.join("weftext.resources.json")),
                node.manifest
            );
            assert_eq!(file_identity(&directory.join("personal.raw")), node.raw);
            assert_eq!(
                directory_identity(&directory.join("opaque")),
                node.opaque_directory
            );
            assert_eq!(
                file_identity(&directory.join("opaque/keep.raw")),
                node.opaque_leaf
            );
            assert_eq!(fs::read(&source).unwrap(), node.expected);
            assert_eq!(
                fs::read(directory.join("personal.raw")).unwrap(),
                node.raw_bytes
            );
            assert_eq!(
                fs::read(directory.join("opaque/keep.raw")).unwrap(),
                node.raw_bytes
            );
            let doc = self.b.open(&source, Some(self.br.clone())).unwrap();
            assert_eq!(doc.node_id.as_deref(), Some(node.id.as_str()));
            assert_eq!(
                doc.workspace_id.as_deref(),
                Some(self.identity.workspace_id.as_str())
            );
            for (version, before) in &node.history {
                assert_eq!(
                    self.b
                        .read_history(&doc.session_id, version)
                        .unwrap()
                        .source,
                    *before
                );
            }
            let resource = self
                .b
                .managed_resource_snapshot(&self.br, &source)
                .unwrap()
                .remove(0);
            assert_eq!(resource.resource_ref, node.resource_ref);
            assert_eq!(
                resource.revision.as_deref(),
                Some(node.resource_revision.as_str())
            );
            assert_eq!(resource.identity_status, "admitted");
            for (actor, draft) in &node.drafts {
                self.b
                    .with_actor_for_root(actor, &self.br, |core| {
                        let opened = core.open(&source, Some(self.br.clone()))?;
                        assert_eq!(opened.recovered_draft.as_deref(), Some(draft.as_str()));
                        assert_eq!(core.read_draft(&source)?.unwrap().source, *draft);
                        Ok(())
                    })
                    .unwrap();
            }
            let graph = self.b.read_sync_history(&self.identity).unwrap();
            let view = &graph.objects().unwrap()[&ObjectKey::Node(node.id.clone())];
            assert_eq!(view.versions.len(), 1);
            assert_eq!(view.versions[0].revision, node.received_revision);
            assert_eq!(
                view.versions[0].state.file_revisions,
                node.received_file_revisions
            );
        }
        assert_eq!(self.a.operations.len(), self.source_operations);
        assert!(
            !self.br.join("Holding").exists(),
            "Sender history's temporary author name is not a receiver-created writer"
        );
    }

    fn reject_every_effect(&mut self, preview: &crate::OperationPreview) {
        let current = actual_tree(&self.br);
        assert!(
            self.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err()
        );
        assert!(
            !self
                .b
                .journal_path(&preview.operation_id, "install")
                .exists()
        );
        assert_eq!(actual_tree(&self.br), current);
        let info = self.b.inspect_recovery(&preview.operation_id).unwrap();
        assert!(
            info.allowed_actions.is_empty(),
            "{}",
            serde_json::to_string(&info).unwrap()
        );
        for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
            assert!(
                self.b
                    .resolve_recovery(&preview.operation_id, action)
                    .is_err()
            );
        }
        assert_eq!(actual_tree(&self.br), current);
    }
}

fn positive(layout: Layout) {
    let mut pair = Pair::new(layout);
    let (request, preview) = pair.prepare();
    let count = pair.b.operations.len();
    let receipt = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    pair.assert_installed(&request, &preview);
    let installed = actual_tree(&pair.br);
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .plan_id,
        receipt.plan_id
    );
    assert_eq!(pair.b.operations.len(), count);
    assert_eq!(actual_tree(&pair.br), installed);
}

#[test]
fn compound_real_two_root_exchange_keeps_original_objects_and_private_facts() {
    positive(Layout::Exchange);
}

#[test]
fn compound_real_three_root_cycle_keeps_original_objects_and_private_facts() {
    positive(Layout::Rotate);
}

#[test]
fn compound_real_occupied_participant_chain_ends_in_original_empty_slot() {
    positive(Layout::EmptyChain);
}

#[test]
fn compound_real_child_extraction_and_parent_rename_deduplicate_overlapping_old_tree() {
    positive(Layout::ExtractChild);
}

#[test]
fn compound_real_parent_move_and_child_rename_keep_nested_final_parent_identity() {
    positive(Layout::NestedChild);
}

#[test]
fn compound_exchange_original_prepared_and_committed_requests_survive_restart() {
    let mut pair = Pair::new(Layout::Exchange);
    let (request, preview) = pair.prepare();
    let before = actual_tree(&pair.br);
    pair.restart();
    assert_eq!(actual_tree(&pair.br), before);
    assert_eq!(
        pair.b.prepare_operation(request.clone()).unwrap().plan_id,
        preview.plan_id
    );
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let installed = actual_tree(&pair.br);
    pair.restart();
    assert_eq!(actual_tree(&pair.br), installed);
    pair.assert_installed(&request, &preview);
}

fn original_checkpoint(
    pair: &mut Pair,
    request: &OperationRequest,
    preview: &crate::OperationPreview,
    point: &str,
) {
    let frozen = serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap();
    let receipt = serde_json::to_value(
        &pair.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap()
            .receipt,
    )
    .unwrap();
    let before = actual_tree(&pair.br);
    interrupt_placement(pair, preview, point);
    let interrupted = actual_tree(&pair.br);
    pair.restart();
    assert_eq!(actual_tree(&pair.br), interrupted, "{point}/restart");
    assert_eq!(
        serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
        frozen
    );
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    let keeps_current = info
        .allowed_actions
        .iter()
        .any(|action| action == "keepCurrent");
    // An install marker, or a canonical-leaf checkpoint whose old/new names
    // are equal, need not change the author tree. The public knownBefore
    // contract may keep that exact tree; actual namespace/content effects may
    // not be accepted as an unchanged original before state.
    assert!(
        !keeps_current || (interrupted == before && info.coherence == "knownBefore"),
        "{point}: author_tree_changed={}: {}",
        interrupted != before,
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
    pair.b
        .resolve_recovery(&preview.operation_id, action)
        .unwrap();
    pair.assert_installed(request, preview);
    let installed = actual_tree(&pair.br);
    let drafts = actor_draft_views(pair, false);
    pair.restart();
    assert_eq!(actual_tree(&pair.br), installed);
    let replay = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(serde_json::to_value(replay).unwrap(), receipt);
    assert_eq!(actual_tree(&pair.br), installed);
    assert_eq!(actor_draft_views(pair, true), drafts);
}

fn original_install_content_and_facts_checkpoints(layout: Layout) {
    for point in ["installed", "content0", "facts"] {
        let mut pair = Pair::new(layout);
        let (request, preview) = pair.prepare();
        original_checkpoint(&mut pair, &request, &preview, point);
    }
}

#[test]
fn compound_complex_original_install_content_and_facts_checkpoints_resume_same_plan() {
    for layout in [Layout::Exchange, Layout::Rotate, Layout::NestedChild] {
        original_install_content_and_facts_checkpoints(layout);
    }
}

#[test]
fn compound_nested_child_resumes_when_parent_is_final_and_child_is_still_parked() {
    let mut pair = Pair::new(Layout::NestedChild);
    let (request, preview) = pair.prepare();
    let transaction = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let crate::operations::Mutation::Compound { relocations, .. } = &transaction.mutation else {
        panic!("Nested placement must use its real Compound Plan");
    };
    assert_eq!(relocations.len(), 2);
    assert!(relocations.iter().all(|step| step.parking.is_some()));
    let parent = (0..relocations.len())
        .min_by_key(|index| Path::new(&relocations[*index].target).components().count())
        .unwrap();
    original_checkpoint(&mut pair, &request, &preview, &format!("move{parent}"));
}

#[test]
fn compound_extract_child_original_install_content_and_facts_checkpoints_resume_same_plan() {
    original_install_content_and_facts_checkpoints(Layout::ExtractChild);
}

#[test]
fn compound_extract_child_original_leaf_park_and_move_checkpoints_resume_same_plan() {
    for stage in [
        "childLeaf",
        "childPark",
        "parentLeaf",
        "parentPark",
        "parked",
        "parentMove",
        "childMove",
    ] {
        let mut pair = Pair::new(Layout::ExtractChild);
        let (request, preview) = pair.prepare();
        let transaction = pair.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        let crate::operations::Mutation::Compound { relocations, .. } = &transaction.mutation
        else {
            panic!("Extraction must use its real Compound Plan");
        };
        assert_eq!(relocations.len(), 2);
        assert!(relocations.iter().all(|step| step.parking.is_some()));
        let child = (0..relocations.len())
            .max_by_key(|index| Path::new(&relocations[*index].source).components().count())
            .unwrap();
        let parent = (0..relocations.len())
            .min_by_key(|index| Path::new(&relocations[*index].source).components().count())
            .unwrap();
        assert_ne!(child, parent);
        assert!(
            crate::history::history_relative_path(
                Path::new(&relocations[child].source),
                Path::new(&relocations[parent].source),
            )
            .is_some_and(|relative| !relative.as_os_str().is_empty())
        );
        let point = match stage {
            "childLeaf" => format!("leaf{child}"),
            "childPark" => format!("park{child}"),
            "parentLeaf" => format!("leaf{parent}"),
            "parentPark" => format!("park{parent}"),
            "parentMove" => format!("move{parent}"),
            "childMove" => format!("move{child}"),
            _ => "parked".into(),
        };
        original_checkpoint(&mut pair, &request, &preview, &point);
    }
}

#[test]
fn compound_exchange_late_foreign_participant_slot_preserves_current_and_displaced_original() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let retained = pair.temp.path().join("retained-original-left");
    fs::rename(pair.br.join("Left"), &retained).unwrap();
    let original = actual_tree(&retained);
    fs::create_dir(pair.br.join("Left")).unwrap();
    fs::write(
        pair.br.join("Left/foreign.raw"),
        b"late foreign final-slot owner",
    )
    .unwrap();
    pair.reject_every_effect(&preview);
    assert_eq!(actual_tree(&retained), original);
}

#[test]
fn compound_extract_child_late_free_target_occupancy_prevents_parent_first_effect() {
    let mut pair = Pair::new(Layout::ExtractChild);
    let (_, preview) = pair.prepare();
    fs::create_dir(pair.br.join("Child")).unwrap();
    fs::write(
        pair.br.join("Child/foreign.raw"),
        b"late external target owner",
    )
    .unwrap();
    pair.reject_every_effect(&preview);
    assert!(!pair.br.join("RenamedParent").exists());
}

#[test]
fn compound_exchange_same_bytes_resource_replacement_preserves_original_and_refuses_every_effect() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let resource = pair.br.join("Right/owned.bin");
    let retained = pair.temp.path().join("retained-original-resource.bin");
    let identity = file_identity(&resource);
    fs::rename(&resource, &retained).unwrap();
    fs::copy(&retained, &resource).unwrap();
    assert_ne!(file_identity(&resource), identity);
    pair.reject_every_effect(&preview);
    assert_eq!(file_identity(&retained), identity);
    assert_eq!(fs::read(&resource).unwrap(), fs::read(&retained).unwrap());
}

#[test]
fn compound_exchange_late_unmanaged_members_follow_their_original_tree() {
    let mut pair = Pair::new(Layout::Exchange);
    let (request, preview) = pair.prepare();
    fs::write(
        pair.br.join("Left/late.raw"),
        b"late raw belongs to original Left",
    )
    .unwrap();
    fs::create_dir(pair.br.join("Left/late-directory")).unwrap();
    fs::write(
        pair.br.join("Left/late-directory/keep.raw"),
        b"late raw child",
    )
    .unwrap();
    let leaf = file_identity(&pair.br.join("Left/late.raw"));
    let directory = directory_identity(&pair.br.join("Left/late-directory"));
    let child = file_identity(&pair.br.join("Left/late-directory/keep.raw"));
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    pair.assert_installed(&request, &preview);
    assert_eq!(file_identity(&pair.br.join("Right/late.raw")), leaf);
    assert_eq!(
        directory_identity(&pair.br.join("Right/late-directory")),
        directory
    );
    assert_eq!(
        file_identity(&pair.br.join("Right/late-directory/keep.raw")),
        child
    );
    assert_eq!(
        fs::read(pair.br.join("Right/late.raw")).unwrap(),
        b"late raw belongs to original Left"
    );
}

#[test]
fn compound_exchange_late_new_node_is_not_an_unmanaged_companion() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let original_plan = pair.b.operations[&preview.operation_id].clone();
    let before = actual_tree(&pair.br);
    let before_local = pair
        .b
        .sync_local_state(pair.b.sync_setup_for(&pair.br).unwrap())
        .unwrap()
        .0;
    let creation_id = fresh();
    let child = pair
        .b
        .create_note_with_id(
            &pair.br,
            pair.br.join("Left"),
            "LateManagedChild",
            &creation_id,
        )
        .unwrap();
    assert!(pair.b.committed(&creation_id));
    let child_key = ObjectKey::Node(child.node_id.unwrap());
    let after_local = pair
        .b
        .sync_local_state(pair.b.sync_setup_for(&pair.br).unwrap())
        .unwrap()
        .0;
    assert!(!before_local.contains_key(&child_key));
    assert!(after_local.contains_key(&child_key));
    assert_eq!(after_local.len(), before_local.len() + 1);
    for (object, original) in &before_local {
        assert_eq!(after_local.get(object), Some(original));
    }
    for original in &original_plan.sync_application.as_ref().unwrap().before {
        assert_eq!(after_local.get(&original.object), Some(original));
    }
    let created = actual_tree(&pair.br);
    for (path, original) in &before {
        assert_eq!(created.get(path), Some(original));
    }
    let additions = created
        .keys()
        .filter(|path| !before.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        additions,
        BTreeSet::from([
            PathBuf::from("Left/LateManagedChild"),
            PathBuf::from("Left/LateManagedChild/LateManagedChild.typ"),
        ])
    );
    // A changed original before-cut would only test the earlier basis gate.
    // This exact rejection must come from the committed-FI admission check on
    // the newly added member, whose whole tree cannot travel as private raw.
    let error = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap_err();
    assert_eq!(error.code, "scopeChanged", "{error:?}");
    assert_eq!(
        error.message,
        "A newly admitted member changed the original moving tree"
    );
    assert_eq!(actual_tree(&pair.br), created);
    pair.reject_every_effect(&preview);
    pair.restart();
    pair.reject_every_effect(&preview);
    assert_eq!(actual_tree(&pair.br), created);
    assert!(pair.b.committed(&creation_id));
    assert!(!pair.b.committed(&preview.operation_id));
    assert_eq!(
        serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
        serde_json::to_value(original_plan).unwrap()
    );
}

#[test]
fn compound_exchange_late_raw_metadata_budget_is_shared_across_original_roots() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let original_plan = serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap();
    for root in ["Left", "Right"] {
        let directory = pair.br.join(root).join("late-budget");
        fs::create_dir(&directory).unwrap();
        for index in 0..2500 {
            fs::write(directory.join(format!("{index:04}.raw")), b"private raw").unwrap();
        }
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2500);
    }
    // Each new subtree has 2501 members and independently fits the bound. Only
    // a budget shared across the original roots rejects their 5002 additions,
    // together with the original Plan's already captured metadata members.
    let current = actual_tree(&pair.br);
    let error = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap_err();
    assert_eq!(error.code, "coverageIncomplete", "{error:?}");
    assert_eq!(error.message, "Compound metadata members exceed 5000");
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert_eq!(actual_tree(&pair.br), current);
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert!(
        info.allowed_actions.is_empty(),
        "{}",
        serde_json::to_string(&info).unwrap()
    );
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            pair.b
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert_eq!(actual_tree(&pair.br), current);
    pair.restart();
    assert_eq!(actual_tree(&pair.br), current);
    assert!(
        pair.b
            .inspect_recovery(&preview.operation_id)
            .unwrap()
            .allowed_actions
            .is_empty()
    );
    assert!(!pair.b.committed(&preview.operation_id));
    assert_eq!(
        serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
        original_plan
    );
}

#[test]
fn compound_exchange_late_busy_raw_uses_placement_metadata_and_moves_original_file() {
    use std::os::windows::fs::OpenOptionsExt;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum MetadataEntry {
        Directory(FileIdentity),
        File(crate::backend::FileStamp),
    }
    fn metadata_tree(root: &Path) -> BTreeMap<PathBuf, MetadataEntry> {
        fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, MetadataEntry>) {
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            let metadata = fs::symlink_metadata(path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                result.insert(relative, MetadataEntry::Directory(directory_identity(path)));
                for entry in fs::read_dir(path).unwrap() {
                    visit(root, &entry.unwrap().path(), result);
                }
            } else {
                let guard = ParentGuard::acquire(path).unwrap();
                let file = guard.open_placement_file().unwrap();
                result.insert(
                    relative,
                    MetadataEntry::File(crate::backend::stamp(&file).unwrap()),
                );
            }
        }
        let mut result = BTreeMap::new();
        visit(root, root, &mut result);
        result
    }

    let mut pair = Pair::new(Layout::Exchange);
    let (request, preview) = pair.prepare();
    let steps = exchange_parking_steps(&pair, &preview);
    let busy_index = steps
        .iter()
        .position(|step| Path::new(&step.source).file_name() == Some(std::ffi::OsStr::new("Left")))
        .unwrap();
    let original_plan = pair.b.operations[&preview.operation_id].clone();
    let frozen_plan = serde_json::to_value(&original_plan).unwrap();
    let frozen_receipt =
        serde_json::to_value(&original_plan.transaction.as_ref().unwrap().receipt).unwrap();
    let original_files = serde_json::to_value(
        pair.b
            .prepared_operation_files(&preview.operation_id)
            .unwrap(),
    )
    .unwrap();
    // Snapshot the existing FI-bound primary records and any consumed markers
    // before namespace effects. This is QA preservation evidence only: Pending
    // coordinates have no committed lineage and must not qualify a draft read.
    let mut original_draft_records = Vec::new();
    for actor in ["alice", "bob"] {
        let records = pair
            .b
            .with_actor_for_root(actor, &pair.br, |core| {
                for node in &pair.nodes {
                    let path = core.draft_path(&source_path(&pair.br, &node.original));
                    let record: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
                    assert_eq!(record["identity"], serde_json::to_value(&node.source)?);
                    assert_eq!(record["source"].as_str(), Some(node.drafts[actor].as_str()));
                }
                let directory = core.draft_directory("drafts");
                Ok((directory.clone(), actual_tree(&directory)))
            })
            .unwrap();
        original_draft_records.push(records);
    }
    let original = pair.br.join("Left/late-busy.raw");
    let bytes = b"private raw body must not become a read precondition";
    fs::write(&original, bytes).unwrap();
    let identity = file_identity(&original);
    let before = metadata_tree(&pair.br);
    let held = fs::OpenOptions::new()
        .write(true)
        .share_mode(2 | 4)
        .open(&original)
        .unwrap();
    assert_eq!(crate::backend::stamp(&held).unwrap().identity, identity);
    assert_eq!(fs::read(&original).unwrap_err().raw_os_error(), Some(32));
    pair.b
        .validate_compound_captured_membership(&original_plan)
        .unwrap();
    assert_eq!(metadata_tree(&pair.br), before);
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    // READ_DATA is unavailable but the same real metadata preflight succeeds.
    // NTFS still refuses the directory rename while this child writer is open,
    // including with SHARE_DELETE. Preserve the exact intermediate Plan and
    // finish it only after the external owner explicitly releases its handle.
    let error = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap_err();
    assert_eq!(error.code, "recoveryPending", "{error:?}");
    assert!(
        error.message.contains("underlying permissionDenied:"),
        "{error:?}"
    );
    let install = pair.b.journal_path(&preview.operation_id, "install");
    let marker = fs::read(&install).unwrap();
    assert!(!pair.b.committed(&preview.operation_id));
    let mut expected = BTreeMap::new();
    let coordinates = steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            PathBuf::from(if index < busy_index {
                step.parking.as_ref().unwrap()
            } else {
                &step.source
            })
        })
        .collect::<Vec<_>>();
    for (path, actual) in &before {
        let mut placed = path.clone();
        for (index, step) in steps.iter().enumerate() {
            let source = Path::new(&step.source).file_name().unwrap();
            let Ok(relative) = path.strip_prefix(Path::new(source)) else {
                continue;
            };
            let leaf = if index <= busy_index && relative == Path::new(&step.old_leaf) {
                Path::new(&step.new_leaf)
            } else {
                relative
            };
            placed = PathBuf::from(coordinates[index].file_name().unwrap()).join(leaf);
            break;
        }
        assert!(expected.insert(placed, actual.clone()).is_none());
    }
    assert_eq!(metadata_tree(&pair.br), expected);
    for (directory, records) in &original_draft_records {
        assert_eq!(actual_tree(directory), *records);
    }
    pair.restart();
    assert_eq!(metadata_tree(&pair.br), expected);
    pair.b
        .validate_compound_captured_membership(&original_plan)
        .unwrap();
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert!(
        info.allowed_actions
            .iter()
            .any(|action| action == "resumeOriginalPlan"),
        "{}",
        serde_json::to_string(&info).unwrap()
    );
    let error = pair
        .b
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap_err();
    assert_eq!(error.code, "recoveryPending", "{error:?}");
    assert!(
        error.message.contains("Underlying permissionDenied:"),
        "{error:?}"
    );
    assert_eq!(metadata_tree(&pair.br), expected);
    assert_eq!(fs::read(&install).unwrap(), marker);
    assert_eq!(
        serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
        frozen_plan
    );
    assert_eq!(
        serde_json::to_value(
            pair.b
                .prepared_operation_files(&preview.operation_id)
                .unwrap()
        )
        .unwrap(),
        original_files
    );
    for (directory, records) in &original_draft_records {
        assert_eq!(actual_tree(directory), *records);
    }
    assert_eq!(crate::backend::stamp(&held).unwrap().identity, identity);
    assert!(!pair.b.committed(&preview.operation_id));
    drop(held);
    pair.b
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    pair.assert_installed(&request, &preview);
    let target = pair.br.join("Right/late-busy.raw");
    assert!(!original.exists());
    assert_eq!(file_identity(&target), identity);
    assert_eq!(fs::read(&target).unwrap(), bytes);
    let installed = actual_tree(&pair.br);
    let drafts = actor_draft_views(&mut pair, false);
    pair.restart();
    assert_eq!(actual_tree(&pair.br), installed);
    let replay = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(serde_json::to_value(replay).unwrap(), frozen_receipt);
    assert_eq!(actor_draft_views(&mut pair, true), drafts);
    assert_eq!(actual_tree(&pair.br), installed);
    assert_eq!(file_identity(&target), identity);
    assert_eq!(fs::read(&target).unwrap(), bytes);
    assert!(!original.exists());
}

#[test]
fn compound_exchange_interrupted_original_cut_rejects_late_received_head_without_any_resume() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    crate::operations::compound::set_compound_failpoint("installed");
    let error = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap_err();
    assert_eq!(error.code, "recoveryPending");
    assert!(
        error.message.contains("underlying testInterrupted:"),
        "{error:?}"
    );
    pair.restart();
    let original_files = pair
        .b
        .prepared_operation_files(&preview.operation_id)
        .unwrap();
    let before = actual_tree(&pair.br);
    save(
        &mut pair.a,
        &pair.ar,
        Path::new("Right"),
        "late sender head after original install marker",
    );
    pair.a.publish_sync(&pair.ar).unwrap();
    pair.b
        .receive_sync(
            &pair.identity,
            pair.a
                .export_sync_objects(&pair.identity, &pair.selected)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(actual_tree(&pair.br), before);
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert!(info.allowed_actions.is_empty());
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            pair.b
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert_eq!(actual_tree(&pair.br), before);
    assert_eq!(
        serde_json::to_value(
            pair.b
                .prepared_operation_files(&preview.operation_id)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(original_files).unwrap()
    );
    assert!(!pair.b.committed(&preview.operation_id));
}

fn exchange_parking_steps(
    pair: &Pair,
    preview: &crate::OperationPreview,
) -> Vec<crate::operations::RelocationStep> {
    let transaction = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let crate::operations::Mutation::Compound { relocations, .. } = &transaction.mutation else {
        panic!("The real exchange must use its original Compound Plan");
    };
    assert_eq!(relocations.len(), 2);
    let parent = directory_identity(&pair.br);
    let mut slots = BTreeSet::new();
    for step in relocations {
        let parking = Path::new(step.parking.as_ref().expect("frozen parking slot"));
        assert_eq!(step.source_parent_identity, parent);
        assert_eq!(directory_identity(parking.parent().unwrap()), parent);
        assert!(slots.insert(parking.to_path_buf()));
        assert!(!parking.exists());
    }
    relocations.clone()
}

fn interrupt_placement(pair: &mut Pair, preview: &crate::OperationPreview, point: &str) {
    crate::operations::compound::set_compound_failpoint(point);
    let error = pair
        .b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap_err();
    assert_eq!(error.code, "recoveryPending", "{point}: {error:?}");
    assert!(
        error.message.contains("underlying testInterrupted:"),
        "{point}: {error:?}"
    );
    assert!(
        pair.b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(!pair.b.committed(&preview.operation_id));
}

fn copy_foreign_tree(source: &Path, target: &Path) {
    fs::create_dir(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let metadata = fs::symlink_metadata(entry.path()).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let destination = target.join(entry.file_name());
        if metadata.is_dir() {
            copy_foreign_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn bytes_and_membership(tree: &BTreeMap<PathBuf, Actual>) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    tree.iter()
        .map(|(path, actual)| {
            (
                path.clone(),
                match actual {
                    Actual::Directory(_) => None,
                    Actual::File(_, bytes) => Some(bytes.clone()),
                },
            )
        })
        .collect()
}

fn reject_interrupted_placement_without_effect(pair: &mut Pair, preview: &crate::OperationPreview) {
    let current = actual_tree(&pair.br);
    let install = pair.b.journal_path(&preview.operation_id, "install");
    let original_marker = fs::read(&install).unwrap();
    let original_plan = serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap();
    let original_files = serde_json::to_value(
        pair.b
            .prepared_operation_files(&preview.operation_id)
            .unwrap(),
    )
    .unwrap();
    for _ in 0..2 {
        pair.restart();
        assert_eq!(actual_tree(&pair.br), current);
        let error = pair
            .b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err();
        assert_eq!(error.code, "recoveryPending", "{error:?}");
        assert_eq!(actual_tree(&pair.br), current);
        let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        assert!(
            info.allowed_actions.is_empty(),
            "{}",
            serde_json::to_string(&info).unwrap()
        );
        for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
            assert!(
                pair.b
                    .resolve_recovery(&preview.operation_id, action)
                    .is_err(),
                "A third-state namespace must reject {action}"
            );
            assert_eq!(actual_tree(&pair.br), current);
        }
        assert!(!pair.b.committed(&preview.operation_id));
        assert_eq!(fs::read(&install).unwrap(), original_marker);
        assert_eq!(
            serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
            original_plan
        );
        assert_eq!(
            serde_json::to_value(
                pair.b
                    .prepared_operation_files(&preview.operation_id)
                    .unwrap()
            )
            .unwrap(),
            original_files
        );
    }
}

#[test]
fn compound_exchange_original_leaf_park_and_move_checkpoints_resume_same_frozen_plan() {
    for point in ["leaf0", "park0", "parked", "move0"] {
        let mut pair = Pair::new(Layout::Exchange);
        let (request, preview) = pair.prepare();
        let steps = exchange_parking_steps(&pair, &preview);
        let before = actual_tree(&pair.br);
        let original_plan =
            serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap();
        let expected_receipt = serde_json::to_value(
            &pair.b.operations[&preview.operation_id]
                .transaction
                .as_ref()
                .unwrap()
                .receipt,
        )
        .unwrap();
        let operation_count = pair.b.operations.len();
        interrupt_placement(&mut pair, &preview, point);

        // Check the whole real namespace, including the original untracked
        // companions. The failpoint must stop at this exact placement phase,
        // before either participant's received content is installed.
        let mut expected = BTreeMap::new();
        for (path, actual) in &before {
            let mut placed = path.clone();
            for (index, step) in steps.iter().enumerate() {
                let source = Path::new(&step.source).file_name().unwrap();
                let Ok(relative) = path.strip_prefix(Path::new(source)) else {
                    continue;
                };
                let coordinate = match (point, index) {
                    ("leaf0", _) | ("park0", 1) => &step.source,
                    ("move0", 0) => &step.target,
                    _ => step.parking.as_ref().unwrap(),
                };
                let renamed = index == 0 || matches!(point, "parked" | "move0");
                let leaf = if renamed && relative == Path::new(&step.old_leaf) {
                    Path::new(&step.new_leaf)
                } else {
                    relative
                };
                placed = PathBuf::from(Path::new(coordinate).file_name().unwrap()).join(leaf);
                break;
            }
            assert!(expected.insert(placed, actual.clone()).is_none());
        }
        assert_eq!(actual_tree(&pair.br), expected, "{point}");
        pair.restart();
        assert_eq!(actual_tree(&pair.br), expected, "{point}/restart");
        assert_eq!(
            serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
            original_plan,
            "Recovery must retain the original parking slots and complete cut"
        );
        let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        assert!(
            info.allowed_actions
                .iter()
                .any(|action| action == "resumeOriginalPlan"),
            "{point}: {}",
            serde_json::to_string(&info).unwrap()
        );
        assert!(
            !info
                .allowed_actions
                .iter()
                .any(|action| action == "keepCurrent")
        );
        pair.b
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap();
        pair.assert_installed(&request, &preview);
        for step in &steps {
            assert!(!Path::new(step.parking.as_ref().unwrap()).exists());
        }
        let installed = actual_tree(&pair.br);
        let drafts = actor_draft_views(&mut pair, false);
        pair.restart();
        assert_eq!(actual_tree(&pair.br), installed);
        assert_eq!(actor_draft_views(&mut pair, true), drafts);
        for _ in 0..2 {
            let receipt = pair
                .b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            assert_eq!(serde_json::to_value(receipt).unwrap(), expected_receipt);
            assert_eq!(pair.b.operations.len(), operation_count);
            assert_eq!(actual_tree(&pair.br), installed);
            assert_eq!(actor_draft_views(&mut pair, false), drafts);
        }
    }
}

#[test]
fn compound_exchange_late_foreign_parking_slot_refuses_before_first_effect() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let steps = exchange_parking_steps(&pair, &preview);
    let parking = Path::new(steps[0].parking.as_ref().unwrap());
    fs::create_dir(parking).unwrap();
    fs::write(
        parking.join("foreign.raw"),
        b"foreign original parking owner",
    )
    .unwrap();
    let foreign = actual_tree(parking);
    let original_plan = serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap();
    pair.reject_every_effect(&preview);
    pair.restart();
    pair.reject_every_effect(&preview);
    assert_eq!(actual_tree(parking), foreign);
    assert_eq!(
        serde_json::to_value(&pair.b.operations[&preview.operation_id]).unwrap(),
        original_plan
    );
    assert!(!pair.b.committed(&preview.operation_id));
}

#[test]
fn compound_exchange_parked_same_bytes_foreign_root_preserves_original_and_refuses_recovery() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let steps = exchange_parking_steps(&pair, &preview);
    interrupt_placement(&mut pair, &preview, "park0");
    let parking = Path::new(steps[0].parking.as_ref().unwrap());
    assert_eq!(directory_identity(parking), steps[0].source_identity);
    let retained = pair.temp.path().join("retained-original-parked-root");
    fs::rename(parking, &retained).unwrap();
    let original = actual_tree(&retained);
    copy_foreign_tree(&retained, parking);
    assert_ne!(directory_identity(parking), steps[0].source_identity);
    assert_eq!(
        bytes_and_membership(&actual_tree(parking)),
        bytes_and_membership(&original)
    );
    reject_interrupted_placement_without_effect(&mut pair, &preview);
    assert_eq!(actual_tree(&retained), original);
}

#[test]
fn compound_exchange_replaced_parking_parent_refuses_with_original_roots_at_frozen_coordinates() {
    let mut pair = Pair::new(Layout::Exchange);
    let (_, preview) = pair.prepare();
    let steps = exchange_parking_steps(&pair, &preview);
    interrupt_placement(&mut pair, &preview, "installed");
    let original = actual_tree(&pair.br);
    let parent_identity = directory_identity(&pair.br);
    let retained = pair.temp.path().join("retained-original-parking-parent");
    fs::rename(&pair.br, &retained).unwrap();
    fs::create_dir(&pair.br).unwrap();
    for entry in fs::read_dir(&retained).unwrap() {
        let entry = entry.unwrap();
        fs::rename(entry.path(), pair.br.join(entry.file_name())).unwrap();
    }
    let replacement_identity = directory_identity(&pair.br);
    assert_ne!(replacement_identity, parent_identity);
    assert_eq!(directory_identity(&retained), parent_identity);
    assert_eq!(fs::read_dir(&retained).unwrap().count(), 0);
    let mut expected = original;
    expected.insert(PathBuf::new(), Actual::Directory(replacement_identity));
    assert_eq!(actual_tree(&pair.br), expected);
    for step in &steps {
        assert_eq!(
            directory_identity(Path::new(&step.source)),
            step.source_identity
        );
        assert_eq!(
            file_identity(&Path::new(&step.source).join(&step.old_leaf)),
            step.leaf_identity
        );
        assert!(!Path::new(step.parking.as_ref().unwrap()).exists());
    }
    reject_interrupted_placement_without_effect(&mut pair, &preview);
    assert_eq!(actual_tree(&pair.br), expected);
    assert_eq!(directory_identity(&retained), parent_identity);
    assert_eq!(fs::read_dir(&retained).unwrap().count(), 0);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DraftView {
    primary: Option<String>,
    base: Option<String>,
    conflict: bool,
    alternatives: Vec<(String, String, String, bool)>,
}

impl DraftView {
    fn contains(&self, source: &str) -> bool {
        self.primary.as_deref() == Some(source)
            || self
                .alternatives
                .iter()
                .any(|(_, _, proposal, _)| proposal == source)
    }
}

fn actor_draft_views(pair: &mut Pair, reverse: bool) -> BTreeMap<(String, String), DraftView> {
    let mut nodes = pair
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.final_path.clone()))
        .collect::<Vec<_>>();
    if reverse {
        nodes.reverse();
    }
    let mut views = BTreeMap::new();
    for actor in ["alice", "bob"] {
        for (id, directory) in &nodes {
            let view = pair
                .b
                .with_actor_for_root(actor, &pair.br, |core| {
                    let doc = core.open(source_path(&pair.br, directory), Some(pair.br.clone()))?;
                    assert_eq!(doc.node_id.as_deref(), Some(id.as_str()));
                    let mut alternatives = doc
                        .recovered_draft_variants
                        .into_iter()
                        .map(|variant| {
                            (
                                variant.path,
                                variant.base_source,
                                variant.source,
                                variant.base_known,
                            )
                        })
                        .collect::<Vec<_>>();
                    // Sort only: repeated copies remain visible and fail exact
                    // restart/replay equality rather than being deduplicated here.
                    alternatives.sort();
                    Ok(DraftView {
                        primary: doc.recovered_draft,
                        base: doc.recovered_draft_base_source,
                        conflict: doc.recovered_draft_conflict,
                        alternatives,
                    })
                })
                .unwrap();
            views.insert((actor.into(), id.clone()), view);
        }
    }
    views
}

fn assert_other_drafts_unchanged(
    before: &BTreeMap<(String, String), DraftView>,
    after: &BTreeMap<(String, String), DraftView>,
    changed: &(String, String),
) {
    assert_eq!(before.len(), after.len());
    for (key, view) in before {
        if key != changed {
            assert_eq!(
                &after[key], view,
                "Another actor or original FI lost its proposal: {key:?}"
            );
        }
    }
}

fn assert_archived_author_input(core: &Core, source: &str) {
    // Preservation QA only. These bytes are never fed back as qualification,
    // display input or a writable basis; all user-visible reads use Core.open.
    let directory = core.draft_directory("draft-history");
    let present = fs::read_dir(directory).unwrap().any(|entry| {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("json")
        {
            return false;
        }
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
        value.get("source").and_then(serde_json::Value::as_str) == Some(source)
    });
    assert!(
        present,
        "Explicitly resolved original author input must remain archived"
    );
}

#[test]
fn compound_exchange_original_actor_sessions_keep_continued_input_and_stale_save_on_original_node()
{
    let mut pair = Pair::new(Layout::Exchange);
    let (request, preview) = pair.prepare();
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    pair.assert_installed(&request, &preview);
    // Open both exchanged Nodes before writing through old sessions. This is
    // the real path-slot handoff that previously displaced the other FI draft.
    let mut expected = actor_draft_views(&mut pair, false);
    let original_left = pair
        .nodes
        .iter()
        .find(|node| node.original == Path::new("Left"))
        .unwrap();
    let left_id = original_left.id.clone();
    let sessions = original_left.original_sessions.clone();
    let original_drafts = original_left.drafts.clone();
    let author_tree = actual_tree(&pair.br);
    let operation_count = pair.b.operations.len();
    for actor in ["alice", "bob"] {
        let key = (actor.into(), left_id.clone());
        let (session_id, base_id) = &sessions[actor];
        let continued = format!(
            "{}\n{actor} continued typing through original exchanged session",
            original_drafts[actor]
        );
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                core.persist_draft(session_id, &continued)
            })
            .unwrap();
        assert_eq!(actual_tree(&pair.br), author_tree);
        let after_input = actor_draft_views(&mut pair, true);
        assert!(after_input[&key].contains(&continued));
        assert_other_drafts_unchanged(&expected, &after_input, &key);
        let proposal = format!("{continued}\n{actor} explicit stale-base Save proposal");
        let operation_id = fresh();
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                let error = core
                    .save(SaveRequest {
                        session_id: session_id.clone(),
                        base_id: base_id.clone(),
                        operation_id: operation_id.clone(),
                        source: proposal.clone(),
                    })
                    .unwrap_err();
                assert_eq!(error.code, "staleBase");
                assert!(!core.committed(&operation_id));
                assert!(!core.journal_path(&operation_id, "install").exists());
                Ok(())
            })
            .unwrap();
        assert_eq!(actual_tree(&pair.br), author_tree);
        let after_save = actor_draft_views(&mut pair, false);
        assert!(after_save[&key].contains(&proposal));
        assert_other_drafts_unchanged(&after_input, &after_save, &key);
        expected = after_save;
    }
    assert_eq!(
        pair.b.operations.len(),
        operation_count,
        "Rejected old-session Saves cannot create author operations"
    );
    pair.restart();
    assert_eq!(actor_draft_views(&mut pair, true), expected);
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(actor_draft_views(&mut pair, false), expected);
    assert_eq!(actual_tree(&pair.br), author_tree);
}

#[test]
fn compound_exchange_explicit_actor_draft_resolution_does_not_resurrect_consumed_input_after_restart()
 {
    let mut pair = Pair::new(Layout::Exchange);
    let (request, preview) = pair.prepare();
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    pair.assert_installed(&request, &preview);
    let before = actor_draft_views(&mut pair, false);
    let node = pair
        .nodes
        .iter()
        .find(|node| node.original == Path::new("Left"))
        .unwrap();
    let left_id = node.id.clone();
    let path = source_path(&pair.br, &node.final_path);
    let originals = node.drafts.clone();
    let merged = format!(
        "{}\nBob explicitly reviewed and merged the correct original Node 😀",
        String::from_utf8(node.expected.clone()).unwrap()
    );
    let author_tree = actual_tree(&pair.br);
    for (actor, action) in [("alice", "discardRecovered"), ("bob", "mergeRecovered")] {
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                let opened = core.open(&path, Some(pair.br.clone()))?;
                assert!(opened.recovered_draft_conflict);
                core.resolve_draft(
                    &opened.session_id,
                    action,
                    if actor == "bob" { Some(&merged) } else { None },
                )?;
                assert_archived_author_input(core, &originals[actor]);
                Ok(())
            })
            .unwrap();
    }
    let expected = actor_draft_views(&mut pair, true);
    let alice = &("alice".into(), left_id.clone());
    let bob = &("bob".into(), left_id.clone());
    assert_eq!(expected[alice].primary, None);
    assert!(expected[alice].alternatives.is_empty());
    assert!(!expected[alice].conflict);
    assert_eq!(expected[bob].primary.as_deref(), Some(merged.as_str()));
    assert!(expected[bob].alternatives.is_empty());
    assert!(!expected[bob].conflict);
    for (key, view) in &before {
        if key.1 != left_id {
            assert_eq!(&expected[key], view);
        }
    }
    assert_eq!(actual_tree(&pair.br), author_tree);
    pair.restart();
    assert_eq!(actor_draft_views(&mut pair, false), expected);
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(actor_draft_views(&mut pair, true), expected);
    let merged_revision = pair
        .b
        .with_actor_for_root("bob", &pair.br, |core| {
            let opened = core.open(&path, Some(pair.br.clone()))?;
            assert_eq!(opened.node_id.as_deref(), Some(left_id.as_str()));
            assert_eq!(opened.recovered_draft.as_deref(), Some(merged.as_str()));
            assert!(!opened.recovered_draft_conflict);
            Ok(core
                .save(SaveRequest {
                    session_id: opened.session_id,
                    base_id: opened.base_id,
                    operation_id: fresh(),
                    source: merged.clone(),
                })?
                .revision_id)
        })
        .unwrap();
    assert!(merged_revision.is_some());
    let explicitly_saved = actual_tree(&pair.br);
    assert_eq!(fs::read(&path).unwrap(), merged.as_bytes());
    pair.restart();
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        actual_tree(&pair.br),
        explicitly_saved,
        "Replaying the old receipt cannot overwrite the later explicit Save"
    );
    let after_save = actor_draft_views(&mut pair, false);
    for actor in ["alice", "bob"] {
        let key = (actor.into(), left_id.clone());
        assert_eq!(after_save[&key].primary, None);
        assert!(after_save[&key].alternatives.is_empty());
        assert!(!after_save[&key].conflict);
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                assert_archived_author_input(core, &originals[actor]);
                Ok(())
            })
            .unwrap();
    }
    for (key, view) in &expected {
        if key.1 != left_id {
            assert_eq!(&after_save[key], view);
        }
    }
}

#[test]
fn compound_extract_directory_parent_with_original_child_members_recovers_same_plan() {
    for stage in [
        "childLeaf",
        "childPark",
        "parentPark",
        "parentMove",
        "childMove",
    ] {
        let mut pair = Pair::new(Layout::ExtractDirectoryChild);
        let directory = pair.br.join("Parent");
        let directory_identity_before = directory_identity(&directory);
        let marker = directory.join("weftext.directory.json");
        let marker_identity = file_identity(&marker);
        let marker_bytes = fs::read(&marker).unwrap();
        let admitted = pair
            .b
            .capture_navigation_target(&pair.br, &directory)
            .unwrap();
        assert_eq!(admitted.status, "managed");
        let reference = admitted.target_ref.unwrap();
        let directory_key = pair
            .selected
            .iter()
            .find(|key| matches!(key, ObjectKey::Directory(_)))
            .unwrap()
            .clone();
        let received_graph = pair.b.read_sync_history(&pair.identity).unwrap();
        let objects = received_graph.objects().unwrap();
        let view = &objects[&directory_key];
        assert_eq!(view.versions.len(), 1);
        let received = view.versions[0].clone();
        assert!(
            received_graph
                .unsettled_dependencies(&received.commit)
                .unwrap()
                .is_empty()
        );
        fs::write(
            directory.join("directory-private.raw"),
            b"original Directory private companion",
        )
        .unwrap();
        let private_identity = file_identity(&directory.join("directory-private.raw"));
        let child_node = pair
            .nodes
            .iter()
            .find(|node| node.original == Path::new("Parent/Child"))
            .unwrap();
        let original_child_identity = child_node.source.clone();
        let original_child_id = child_node.id.clone();
        let (request, preview) = pair.prepare();
        let transaction = pair.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        let crate::operations::Mutation::Compound { relocations, .. } = &transaction.mutation
        else {
            panic!("Directory extraction must use the real Compound Plan");
        };
        assert_eq!(relocations.len(), 2);
        let child = (0..relocations.len())
            .max_by_key(|index| Path::new(&relocations[*index].source).components().count())
            .unwrap();
        let parent = (0..relocations.len())
            .min_by_key(|index| Path::new(&relocations[*index].source).components().count())
            .unwrap();
        assert_ne!(child, parent);
        assert_eq!(
            relocations[parent].source_identity,
            directory_identity_before
        );
        assert!(relocations.iter().all(|step| step.parking.is_some()));
        assert!(
            !relocations[parent].members.is_empty(),
            "A Node parent cannot stand in for a Directory member proof"
        );
        assert!(
            relocations[parent].members.iter().any(|member| {
                member.node_id.as_deref() == Some(original_child_id.as_str())
                    && member.identity == original_child_identity
                    && Path::new(&member.relative_path) == Path::new("Child/Child.typ")
            }),
            "The real Directory relocation must capture this original child FI as a member"
        );
        let point = match stage {
            "childLeaf" => format!("leaf{child}"),
            "childPark" => format!("park{child}"),
            "parentPark" => format!("park{parent}"),
            "parentMove" => format!("move{parent}"),
            _ => format!("move{child}"),
        };
        original_checkpoint(&mut pair, &request, &preview, &point);
        let target = pair.br.join("RenamedParent");
        assert_eq!(directory_identity(&target), directory_identity_before);
        assert_eq!(
            file_identity(&target.join("weftext.directory.json")),
            marker_identity
        );
        assert_eq!(
            fs::read(target.join("weftext.directory.json")).unwrap(),
            marker_bytes
        );
        assert_eq!(
            file_identity(&target.join("directory-private.raw")),
            private_identity
        );
        assert_eq!(
            fs::read(target.join("directory-private.raw")).unwrap(),
            b"original Directory private companion"
        );
        assert!(!target.join("Child").exists());
        assert!(!target.join("RenamedParent.typ").exists());
        let resolved = pair
            .b
            .resolve_navigation_target(&pair.br, &reference)
            .unwrap();
        assert_eq!(resolved.status, "managed");
        assert_eq!(
            directory_identity(Path::new(resolved.path.as_ref().unwrap())),
            directory_identity_before
        );
        let installed = pair
            .b
            .read_sync_history(&pair.identity)
            .unwrap()
            .objects()
            .unwrap();
        assert_eq!(installed[&directory_key].versions.len(), 1);
        assert_eq!(
            installed[&directory_key].versions[0].revision,
            received.revision
        );
        assert_eq!(
            installed[&directory_key].versions[0].state.file_revisions,
            received.state.file_revisions
        );
        let local = pair
            .b
            .sync_local_state(pair.b.sync_setup_for(&pair.br).unwrap())
            .unwrap()
            .0;
        assert_eq!(local[&directory_key].revision, received.revision);
        assert_eq!(local[&directory_key].state, received.state);
        assert_eq!(
            directory_identity(Path::new(&local[&directory_key].directory)),
            directory_identity_before
        );
    }
}

#[test]
fn compound_extract_directory_parked_created_child_and_exact_5000_metadata_members_succeed() {
    let mut pair = Pair::new(Layout::ExtractDirectoryWithNewChild);
    let parent_identity = directory_identity(&pair.br.join("Parent"));
    let marker = capture_optional_for_budget(&pair.br.join("Parent/weftext.directory.json"));
    let graph = pair.b.read_sync_history(&pair.identity).unwrap();
    let objects = graph.objects().unwrap();
    let (new_key, view) = objects
        .iter()
        .find(|(key, view)| {
            matches!(key, ObjectKey::Node(_))
                && view.versions.len() == 1
                && view.versions[0].state.name == "NewChild"
        })
        .unwrap();
    let ObjectKey::Node(new_id) = new_key else {
        unreachable!()
    };
    let new_id = new_id.clone();
    let chosen = view.versions[0].clone();
    assert!(
        graph
            .unsettled_dependencies(&chosen.commit)
            .unwrap()
            .is_empty()
    );
    assert!(
        !pair
            .b
            .sync_binding_info(&pair.br)
            .unwrap()
            .installed_objects
            .contains(new_key),
        "The new child must be absent from the receiver's real initial Clone"
    );
    let (request, preview) = pair.prepare();
    let plan = pair.b.operations[&preview.operation_id].clone();
    let transaction = plan.transaction.as_ref().unwrap();
    let crate::operations::Mutation::Compound {
        relocations,
        updates,
        created_files,
        created_directories,
        ..
    } = &transaction.mutation
    else {
        unreachable!()
    };
    assert_eq!(relocations.len(), 2);
    assert!(relocations.iter().all(|step| step.parking.is_some()));
    let new_directory = pair.br.join("RenamedParent/NewChild");
    assert!(
        created_directories
            .iter()
            .any(|path| Path::new(path) == new_directory)
    );
    let birth = transaction
        .births()
        .iter()
        .find(|birth| birth.node_id == new_id)
        .expect("This exact Plan creates the received NewChild identity");
    let birth_path = PathBuf::from(&birth.path);
    let created_source = updates
        .iter()
        .chain(created_files)
        .find(|file| file.path == birth.path)
        .expect("The original birth must name its own create-only source output");
    assert!(created_source.before.is_none());
    let expected_new_source = created_source.after.clone();

    // Compute the boundary from this actual retained Plan, rather than making
    // a larger synthetic Plan or assigning new authoritative metadata. Captured
    // object identities count once; each absent output reserves one member.
    let mut original_identities = BTreeSet::new();
    for file in &transaction.files {
        original_identities.insert(serde_json::to_string(&file.stamp.identity).unwrap());
    }
    for directory in &transaction.directories {
        original_identities.insert(serde_json::to_string(&directory.identity).unwrap());
    }
    let mut new_outputs = BTreeSet::new();
    let mut created_count = created_directories.len();
    for directory in created_directories {
        assert!(new_outputs.insert(directory.clone()));
    }
    for file in updates
        .iter()
        .chain(created_files)
        .filter(|file| file.before.is_none())
    {
        created_count += 1;
        assert!(new_outputs.insert(file.path.clone()));
    }
    assert_eq!(new_outputs.len(), created_count);
    let used = original_identities.len() + created_count;
    assert!(used < 5000);
    let extra = 5000 - used;
    let mut raw = Vec::new();
    for index in 0..extra {
        let name = format!("budget-{index:04}.raw");
        let path = pair.br.join("Parent/Child").join(&name);
        fs::write(&path, b"exact budget private companion").unwrap();
        raw.push((name, file_identity(&path)));
    }
    assert_eq!(used + raw.len(), 5000);
    let before = actual_tree(&pair.br);
    pair.b.validate_compound_captured_membership(&plan).unwrap();
    assert_eq!(
        actual_tree(&pair.br),
        before,
        "Preflight is read-only at the exact bound"
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    // The original writer creates NewChild inside the moving Parent before
    // parking it. Its already reserved directory must not be counted as new
    // raw again during the Parked membership check.
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    pair.assert_installed(&request, &preview);
    assert_eq!(
        directory_identity(&pair.br.join("RenamedParent")),
        parent_identity
    );
    let installed_marker =
        capture_optional_for_budget(&pair.br.join("RenamedParent/weftext.directory.json"));
    assert_eq!(installed_marker.stamp.identity, marker.stamp.identity);
    assert_eq!(installed_marker.bytes, marker.bytes);
    for (name, identity) in raw {
        let path = pair.br.join("Child").join(name);
        assert_eq!(file_identity(&path), identity);
        assert_eq!(fs::read(path).unwrap(), b"exact budget private companion");
    }
    let new_source = new_directory.join("NewChild.typ");
    let captured = capture_optional_for_budget(&new_source);
    assert_eq!(
        crate::backend::namespace_path_compare(&birth_path, &new_source).unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(captured.bytes, expected_new_source);
    // Materialization records are keyed by the original frozen Plan spelling,
    // rather than by an independently reconstructed equivalent pathname.
    assert_eq!(
        pair.b
            .materialized_identity(&preview.operation_id, &birth_path)
            .unwrap(),
        Some(captured.stamp.identity.clone())
    );
    assert_eq!(
        pair.b.accepted_revision(&captured).as_ref(),
        chosen.state.file_revisions.get("NewChild.typ")
    );
    let owner = weftext_knowledge::NodeRef {
        workspace_id: pair.identity.workspace_id.clone(),
        node_id: new_id.clone(),
    };
    pair.b
        .qualified_query_node_at(&pair.br, &new_source, &owner, "live")
        .unwrap();
    let opened = pair.b.open(&new_source, Some(pair.br.clone())).unwrap();
    assert_eq!(opened.node_id.as_deref(), Some(new_id.as_str()));
    let local = pair
        .b
        .sync_local_state(pair.b.sync_setup_for(&pair.br).unwrap())
        .unwrap()
        .0;
    assert_eq!(local[&ObjectKey::Node(new_id)].revision, chosen.revision);
    assert_eq!(local[new_key].state, chosen.state);
}

fn capture_optional_for_budget(path: &Path) -> crate::operations::CapturedFile {
    crate::operations::capture_optional(path).unwrap().unwrap()
}
