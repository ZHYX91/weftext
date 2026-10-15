use super::*;
use crate::backend::ParentGuard;
use crate::{
    Core, FileChange, OperationIntent, OperationRequest, Path, PathBuf, SaveRequest, display, fresh,
};
use std::fs;

struct Pair {
    temp: tempfile::TempDir,
    a: Core,
    b: Core,
    ar: PathBuf,
    br: PathBuf,
    bd: PathBuf,
    identity: SyncIdentity,
    keys: BTreeSet<ObjectKey>,
}
fn run(core: &mut Core, root: &Path, intent: OperationIntent) {
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(root),
            intent,
        })
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}
fn save(core: &mut Core, root: &Path, relative: &str, suffix: &str) {
    let opened = core.open(root.join(relative), Some(root.into())).unwrap();
    core.save(SaveRequest {
        session_id: opened.session_id,
        base_id: opened.base_id,
        operation_id: fresh(),
        source: format!("{}\n{suffix}", opened.source),
    })
    .unwrap();
}
fn fixture() -> Pair {
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("a");
    let br = temp.path().join("b");
    let bd = temp.path().join("b-private");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&ar).unwrap();
    let mut keys = BTreeSet::new();
    for name in ["Left", "Right", "Container", "Still"] {
        keys.insert(ObjectKey::Node(
            a.create_note(&ar, &ar, name).unwrap().node_id.unwrap(),
        ));
    }
    keys.insert(ObjectKey::Node(
        a.create_note(&ar, ar.join("Left"), "Child")
            .unwrap()
            .node_id
            .unwrap(),
    ));
    for name in ["Left", "Right"] {
        let preview = a
            .prepare_import_resource(
                &ar,
                ar.join(name).join(format!("{name}.typ")),
                "baseline.bin",
                vec![0, 8, 255],
                &fresh(),
            )
            .unwrap();
        a.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    }
    let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
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
    fs::write(br.join("Left/raw.txt"), b"receiver-only left").unwrap();
    fs::write(br.join("Right/raw.txt"), b"receiver-only right").unwrap();
    run(
        &mut a,
        &ar,
        OperationIntent::RenameNode {
            source: display(&ar.join("Left")),
            name: "新名".into(),
        },
    );
    run(
        &mut a,
        &ar,
        OperationIntent::MoveNode {
            source: display(&ar.join("Right")),
            parent: display(&ar.join("Container")),
        },
    );
    save(&mut a, &ar, "新名/新名.typ", "incoming left");
    save(&mut a, &ar, "Container/Right/Right.typ", "incoming right");
    save(&mut a, &ar, "Still/Still.typ", "incoming outside");
    for relative in ["新名/新名.typ", "Container/Right/Right.typ"] {
        let preview = a
            .prepare_import_resource(&ar, ar.join(relative), "added.bin", vec![7, 3, 1], &fresh())
            .unwrap();
        a.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    }
    let path = ar.join("新名/weftext.annotations.json");
    let file = a
        .read_snapshot(&ar, &[display(&path)])
        .unwrap()
        .files
        .remove(0);
    run(
        &mut a,
        &ar,
        OperationIntent::UpdateFiles {
            changes: vec![FileChange {
                path: display(&path),
                base_id: Some(file.base_id),
                before: file.bytes,
                after: br#"{"version":1,"annotations":[],"review":"compound"}"#.to_vec(),
            }],
        },
    );
    a.publish_sync(&ar).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    Pair {
        temp,
        a,
        b,
        ar,
        br,
        bd,
        identity,
        keys,
    }
}
fn request(pair: &Pair) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(&pair.br),
        intent: OperationIntent::ApplySync {
            identity: pair.identity.clone(),
            objects: pair.keys.clone(),
        },
    }
}
fn placements(core: &Core, id: &str) -> Vec<(String, String)> {
    let t = core.operations[id].transaction.as_ref().unwrap();
    assert!(matches!(
        t.mutation,
        crate::operations::Mutation::Compound { .. }
    ));
    t.relocations()
        .iter()
        .map(|step| (step.source.into(), step.target.into()))
        .collect()
}
fn assert_installed(pair: &mut Pair, operation: &str) {
    assert_eq!(
        pair.a
            .sync_binding_info(&pair.ar)
            .unwrap()
            .pending_publications,
        0
    );
    for path in [
        "新名/新名.typ",
        "新名/baseline.bin",
        "新名/added.bin",
        "新名/weftext.resources.json",
        "新名/weftext.annotations.json",
        "Container/Right/Right.typ",
        "Container/Right/baseline.bin",
        "Container/Right/added.bin",
        "Container/Right/weftext.resources.json",
        "Still/Still.typ",
    ] {
        assert_eq!(
            fs::read(pair.br.join(path)).unwrap(),
            fs::read(pair.ar.join(path)).unwrap(),
            "{path}"
        );
    }
    assert!(!pair.br.join("Left").exists());
    assert!(!pair.br.join("Right").exists());
    assert_eq!(
        fs::read(pair.br.join("新名/raw.txt")).unwrap(),
        b"receiver-only left"
    );
    assert_eq!(
        fs::read(pair.br.join("Container/Right/raw.txt")).unwrap(),
        b"receiver-only right"
    );
    assert!(pair.b.committed(operation));
    assert_eq!(
        pair.b
            .sync_binding_info(&pair.br)
            .unwrap()
            .pending_publications,
        0
    );
    let opened = pair
        .b
        .open(pair.br.join("新名/新名.typ"), Some(pair.br.clone()))
        .unwrap();
    let history = pair.b.list_history(&opened.session_id).unwrap();
    let version = history
        .versions
        .iter()
        .find(|version| version.operation_id == operation)
        .unwrap();
    assert_eq!(
        pair.b
            .read_history(&opened.session_id, &version.version_id)
            .unwrap()
            .source,
        opened.source
    );
    assert!(
        pair.b
            .managed_resource_snapshot(&pair.br, pair.br.join("新名/新名.typ"))
            .unwrap()
            .iter()
            .all(|resource| resource.identity_status == "admitted")
    );
}

#[test]
fn compound_two_moves_one_decision_keeps_resources_child_history_actor_drafts_and_raw() {
    let mut pair = fixture();
    let paths = [
        pair.br.join("Left/Left.typ"),
        pair.br.join("Right/Right.typ"),
        pair.br.join("Left/Child/Child.typ"),
    ];
    let before = paths
        .iter()
        .map(|path| {
            crate::operations::capture_optional(path)
                .unwrap()
                .unwrap()
                .stamp
                .identity
        })
        .collect::<Vec<_>>();
    for actor in ["alice", "bob"] {
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                for path in &paths {
                    let doc = core.open(path, Some(pair.br.clone()))?;
                    core.persist_draft(
                        &doc.session_id,
                        &format!("{}\n{actor} proposal", doc.source),
                    )?;
                }
                Ok(())
            })
            .unwrap();
    }
    let req = request(&pair);
    let preview = pair.b.prepare_operation(req.clone()).unwrap();
    assert_eq!(placements(&pair.b, &preview.operation_id).len(), 2);
    assert!(
        pair.b
            .prepared_operation_files(&preview.operation_id)
            .unwrap()
            .iter()
            .any(|change| Path::new(&change.path) == pair.br.join("Still/Still.typ"))
    );
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_eq!(
        pair.b.prepare_operation(req.clone()).unwrap().plan_id,
        preview.plan_id
    );
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_installed(&mut pair, &preview.operation_id);
    let after = [
        pair.br.join("新名/新名.typ"),
        pair.br.join("Container/Right/Right.typ"),
        pair.br.join("新名/Child/Child.typ"),
    ];
    for (index, path) in after.iter().enumerate() {
        assert_eq!(
            crate::operations::capture_optional(path)
                .unwrap()
                .unwrap()
                .stamp
                .identity,
            before[index]
        );
    }
    for actor in ["alice", "bob"] {
        pair.b
            .with_actor_for_root(actor, &pair.br, |core| {
                for path in &after {
                    let doc = core.open(path, Some(pair.br.clone()))?;
                    assert!(
                        core.read_draft(path)?
                            .unwrap()
                            .source
                            .contains(&format!("{actor} proposal"))
                    );
                    assert!(doc.recovered_draft.is_some());
                }
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .plan_id,
        preview.plan_id
    );
    assert_eq!(
        pair.b.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
}

#[test]
fn compound_second_target_occupied_prevents_first_effect() {
    let mut pair = fixture();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let moves = placements(&pair.b, &preview.operation_id);
    fs::create_dir(&moves[1].1).unwrap();
    fs::write(Path::new(&moves[1].1).join("external.txt"), b"untouched").unwrap();
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(Path::new(&moves[0].0).exists());
    assert!(!Path::new(&moves[0].1).exists());
    assert_eq!(
        fs::read(Path::new(&moves[1].1).join("external.txt")).unwrap(),
        b"untouched"
    );
}

#[test]
fn compound_second_root_same_bytes_replacement_prevents_first_effect() {
    let mut pair = fixture();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let moves = placements(&pair.b, &preview.operation_id);
    let source = Path::new(&moves[1].0);
    let t = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let step = &t.relocations()[1];
    let leaf = source.join(step.old_leaf);
    let retained = pair.temp.path().join("retained-original.typ");
    fs::rename(&leaf, &retained).unwrap();
    fs::copy(&retained, &leaf).unwrap();
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(Path::new(&moves[0].0).exists());
    assert!(!Path::new(&moves[0].1).exists());
}

#[test]
fn compound_first_move_restart_resumes_only_remaining_original_steps_and_content() {
    let mut pair = fixture();
    let req = request(&pair);
    let preview = pair.b.prepare_operation(req.clone()).unwrap();
    let moves = placements(&pair.b, &preview.operation_id);
    crate::operations::compound::set_compound_failpoint("move0");
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(Path::new(&moves[0].1).exists());
    assert!(Path::new(&moves[1].0).exists());
    let first_identity = ParentGuard::acquire(&Path::new(&moves[0].1).join(".probe"))
        .unwrap()
        .directory_identity()
        .unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(
        info.coherence,
        "mixedKnown",
        "{}",
        serde_json::to_string(&info).unwrap()
    );
    assert!(
        info.allowed_actions
            .iter()
            .any(|action| action == "resumeOriginalPlan")
    );
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    pair.b
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    assert_installed(&mut pair, &preview.operation_id);
    assert_eq!(
        ParentGuard::acquire(&Path::new(&moves[0].1).join(".probe"))
            .unwrap()
            .directory_identity()
            .unwrap(),
        first_identity
    );
    assert_eq!(
        pair.b
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        pair.b.prepare_operation(req).unwrap().plan_id,
        preview.plan_id
    );
}

#[test]
fn compound_original_leaf_only_intermediate_can_resume_without_namesake_guess() {
    let mut pair = fixture();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let t = pair.b.operations[&preview.operation_id]
        .transaction
        .as_ref()
        .unwrap();
    let leaf_step = t
        .relocations()
        .iter()
        .position(|step| step.old_leaf != step.new_leaf)
        .unwrap();
    crate::operations::compound::set_compound_failpoint(&format!("leaf{leaf_step}"));
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert!(
        info.members
            .iter()
            .any(|member| member.state == "knownIntermediate")
    );
    assert!(
        info.allowed_actions
            .iter()
            .any(|action| action == "resumeOriginalPlan")
    );
    pair.b
        .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
        .unwrap();
    assert_installed(&mut pair, &preview.operation_id);
}

#[test]
fn compound_partial_content_and_facts_restart_keep_original_versions_and_refs() {
    for point in ["content0", "facts"] {
        let mut pair = fixture();
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        let revisions = preview.revisions.clone();
        let resources = preview.resource_map.clone();
        crate::operations::compound::set_compound_failpoint(point);
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err()
        );
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        let action = if info.coherence == "knownAfter" {
            "adoptAfterImages"
        } else {
            "resumeOriginalPlan"
        };
        assert!(
            info.allowed_actions.iter().any(|allowed| allowed == action),
            "{}",
            serde_json::to_string(&info).unwrap()
        );
        pair.b
            .resolve_recovery(&preview.operation_id, action)
            .unwrap();
        assert_installed(&mut pair, &preview.operation_id);
        let t = pair.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        assert_eq!(t.preview.revisions, revisions);
        assert_eq!(t.preview.resource_map, resources);
    }
}

#[test]
fn compound_recovery_third_state_never_moves_second_tree_or_overwrites_outside() {
    for replacement in [false, true] {
        let mut pair = fixture();
        let preview = pair.b.prepare_operation(request(&pair)).unwrap();
        let moves = placements(&pair.b, &preview.operation_id);
        crate::operations::compound::set_compound_failpoint("move0");
        assert!(
            pair.b
                .execute_operation(&preview.operation_id, &preview.plan_id)
                .is_err()
        );
        let still = pair.br.join("Still/Still.typ");
        if replacement {
            let old = pair.temp.path().join("retained-still.typ");
            fs::rename(&still, &old).unwrap();
            fs::copy(&old, &still).unwrap();
        } else {
            fs::write(&still, b"external third state").unwrap();
        }
        let bytes = fs::read(&still).unwrap();
        drop(pair.b);
        pair.b = Core::new(&pair.bd).unwrap();
        let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
        assert_eq!(info.coherence, "thirdState");
        assert!(info.allowed_actions.is_empty());
        for action in ["keepCurrent", "adoptAfterImages", "resumeOriginalPlan"] {
            assert!(
                pair.b
                    .resolve_recovery(&preview.operation_id, action)
                    .is_err()
            );
        }
        assert!(Path::new(&moves[1].0).exists());
        assert!(!Path::new(&moves[1].1).exists());
        assert_eq!(fs::read(&still).unwrap(), bytes);
        assert!(!pair.b.committed(&preview.operation_id));
    }
}

#[test]
fn compound_same_bytes_external_stamp_requires_adoption_before_prepare() {
    let mut pair = fixture();
    let path = pair.br.join("Right/Right.typ");
    let bytes = fs::read(&path).unwrap();
    let old = crate::operations::capture_optional(&path)
        .unwrap()
        .unwrap()
        .stamp;
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_234_567),
    ))
    .unwrap();
    drop(file);
    assert_ne!(
        crate::operations::capture_optional(&path)
            .unwrap()
            .unwrap()
            .stamp,
        old
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        pair.b.prepare_operation(request(&pair)).unwrap_err().code,
        "syncExternalChange"
    );
    assert!(pair.br.join("Left").exists());
    assert!(pair.br.join("Right").exists());
}

#[test]
fn compound_interrupted_original_cut_rejects_late_receive_without_replaying_second_move() {
    let mut pair = fixture();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    let moves = placements(&pair.b, &preview.operation_id);
    crate::operations::compound::set_compound_failpoint("move0");
    assert!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert!(
        pair.b
            .inspect_recovery(&preview.operation_id)
            .unwrap()
            .allowed_actions
            .iter()
            .any(|action| action == "resumeOriginalPlan")
    );
    save(
        &mut pair.a,
        &pair.ar,
        "新名/新名.typ",
        "late new received head",
    );
    pair.a.publish_sync(&pair.ar).unwrap();
    pair.b
        .receive_sync(
            &pair.identity,
            pair.a
                .export_sync_objects(&pair.identity, &pair.keys)
                .unwrap(),
        )
        .unwrap();
    let still = fs::read(pair.br.join("Still/Still.typ")).unwrap();
    let info = pair.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(info.coherence, "unavailable");
    assert!(info.allowed_actions.is_empty());
    assert!(
        info.members.iter().any(
            |member| member.kind == "syncCut" && member.message.contains("syncConflictChanged")
        )
    );
    for action in ["keepCurrent", "adoptAfterImages", "resumeOriginalPlan"] {
        assert!(
            pair.b
                .resolve_recovery(&preview.operation_id, action)
                .is_err()
        );
    }
    assert!(Path::new(&moves[0].1).exists());
    assert!(Path::new(&moves[1].0).exists());
    assert!(!Path::new(&moves[1].1).exists());
    assert_eq!(fs::read(pair.br.join("Still/Still.typ")).unwrap(), still);
    assert!(!pair.b.committed(&preview.operation_id));
}

#[test]
fn compound_committed_fact_is_not_cancelled_by_later_received_cut() {
    let mut pair = fixture();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    pair.b
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    save(
        &mut pair.a,
        &pair.ar,
        "新名/新名.typ",
        "later author version",
    );
    pair.a.publish_sync(&pair.ar).unwrap();
    pair.b
        .receive_sync(
            &pair.identity,
            pair.a
                .export_sync_objects(&pair.identity, &pair.keys)
                .unwrap(),
        )
        .unwrap();
    drop(pair.b);
    pair.b = Core::new(&pair.bd).unwrap();
    assert_eq!(
        pair.b
            .inspect_recovery(&preview.operation_id)
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        pair.b
            .resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .plan_id,
        preview.plan_id
    );
}

#[test]
fn compound_prepared_cut_rejects_local_new_save_before_any_first_effect() {
    let mut pair = fixture();
    let preview = pair.b.prepare_operation(request(&pair)).unwrap();
    save(
        &mut pair.b,
        &pair.br,
        "Still/Still.typ",
        "local newer decision",
    );
    let still = fs::read(pair.br.join("Still/Still.typ")).unwrap();
    assert_eq!(
        pair.b
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "syncBasisChanged"
    );
    assert!(
        !pair
            .b
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(pair.br.join("Left/Left.typ").exists());
    assert!(pair.br.join("Right/Right.typ").exists());
    assert_eq!(fs::read(pair.br.join("Still/Still.typ")).unwrap(), still);
}

#[test]
fn compound_recovery_cut_gate_also_protects_original_update_from_late_received_head() {
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("author");
    let br = temp.path().join("receiver");
    let bd = temp.path().join("receiver-private");
    let mut a = Core::new(temp.path().join("author-private")).unwrap();
    a.create_library(&ar).unwrap();
    let node = a.create_note(&ar, &ar, "Note").unwrap();
    let keys = BTreeSet::from([ObjectKey::Node(node.node_id.unwrap())]);
    let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
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
    save(&mut a, &ar, "Note/Note.typ", "first incoming body");
    a.publish_sync(&ar).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    let preview = b
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&br),
            intent: OperationIntent::ApplySync {
                identity: identity.clone(),
                objects: keys.clone(),
            },
        })
        .unwrap();
    assert!(matches!(
        b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap()
            .mutation,
        crate::operations::Mutation::Update { .. }
    ));
    let changes = b.prepared_operation_files(&preview.operation_id).unwrap();
    let note = changes
        .iter()
        .find(|change| Path::new(&change.path) == br.join("Note/Note.typ"))
        .unwrap();
    // Retain original FI and after bytes, as an interrupted original Update's
    // content write. This is the existing Core recovery fixture mechanism.
    crate::journal::private_create(&b.journal_path(&preview.operation_id, "install"), b"{}")
        .unwrap();
    fs::write(&note.path, &note.after).unwrap();
    drop(b);
    let mut b = Core::new(&bd).unwrap();
    assert!(
        b.inspect_recovery(&preview.operation_id)
            .unwrap()
            .allowed_actions
            .iter()
            .any(|action| action == "adoptAfterImages")
    );
    save(&mut a, &ar, "Note/Note.typ", "later received head");
    a.publish_sync(&ar).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    let info = b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(info.coherence, "unavailable");
    assert!(info.allowed_actions.is_empty());
    for action in ["keepCurrent", "adoptAfterImages", "resumeOriginalPlan"] {
        assert!(b.resolve_recovery(&preview.operation_id, action).is_err());
    }
    assert_eq!(fs::read(&note.path).unwrap(), note.after);
    assert!(!b.committed(&preview.operation_id));
}
