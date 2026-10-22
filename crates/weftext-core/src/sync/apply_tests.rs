use super::*;
use crate::{PathBuf, SaveRequest};
use std::fs;
#[path = "semantic_application_tests.rs"]
mod semantic_application_tests;

struct Pair {
    _temp: tempfile::TempDir,
    a: Core,
    b: Core,
    ar: PathBuf,
    br: PathBuf,
    bd: PathBuf,
    identity: SyncIdentity,
    keys: BTreeSet<ObjectKey>,
}
fn pair(names: &[&str]) -> Pair {
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("a-library");
    let br = temp.path().join("b-library");
    let bd = temp.path().join("b-private");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&ar).unwrap();
    let keys: BTreeSet<_> = names
        .iter()
        .map(|n| ObjectKey::Node(a.create_note_fixture(&ar, &ar, n).unwrap().node_id.unwrap()))
        .collect();
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
    Pair {
        _temp: temp,
        a,
        b,
        ar,
        br,
        bd,
        identity,
        keys,
    }
}
fn run(core: &mut Core, root: &Path, intent: OperationIntent) -> OperationPreview {
    let preview = core
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(root),
            intent,
        })
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    preview
}
fn save(core: &mut Core, root: &Path, name: &str, suffix: &str) -> String {
    let doc = core
        .open(
            root.join(name).join(format!("{name}.typ")),
            Some(root.into()),
        )
        .unwrap();
    let source = format!("{}\n{suffix}", doc.source);
    core.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        operation_id: fresh(),
        source: source.clone(),
    })
    .unwrap();
    source
}
fn transfer(a: &Core, ar: &Path, b: &Core, identity: &SyncIdentity, keys: &BTreeSet<ObjectKey>) {
    a.publish_sync(ar).unwrap();
    b.receive_sync(identity, a.export_sync_objects(identity, keys).unwrap())
        .unwrap();
}
fn apply(pair: &Pair) -> OperationRequest {
    OperationRequest {
        operation_id: fresh(),
        root: display(&pair.br),
        intent: OperationIntent::ApplySync {
            identity: pair.identity.clone(),
            objects: pair.keys.clone(),
        },
    }
}
fn key(pair: &Pair, name: &str) -> ObjectKey {
    let graph = pair.a.read_sync_history(&pair.identity).unwrap();
    graph
        .objects()
        .unwrap()
        .into_iter()
        .find(|(_, v)| v.versions[0].state.name == name)
        .unwrap()
        .0
}

#[test]
fn cancellation_of_unstarted_sync_is_durable_but_install_marker_requires_recovery() {
    let mut p = pair(&["Note"]);
    save(&mut p.a, &p.ar, "Note", "remote edit");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let request = apply(&p);
    let preview = p.b.prepare_operation(request.clone()).unwrap();
    let path = p.br.join("Note/Note.typ");
    let original = fs::read(&path).unwrap();
    fs::write(&path, b"external third state").unwrap();
    assert!(
        p.b.cancel_prepared_sync(&preview.operation_id, &fresh())
            .is_err()
    );
    assert_eq!(
        p.b.cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .status,
        "resolved"
    );
    assert_eq!(fs::read(&path).unwrap(), b"external third state");
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(p.b.prepare_operation(request).unwrap().status, "resolved");
    assert!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    fs::write(&path, original).unwrap();
    let request = apply(&p);
    let preview = p.b.prepare_operation(request).unwrap();
    // The marker alone fences cancellation; bytes cannot prove no prior effect.
    fs::write(
        p.b.journal_path(&preview.operation_id, "install"),
        b"unknown",
    )
    .unwrap();
    assert_eq!(
        p.b.cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(!p.b.resolved(&preview.operation_id).unwrap());
}

#[test]
fn incoming_save_keeps_draft_does_not_echo_and_later_offline_save_uses_received_predecessor() {
    let mut p = pair(&["Note"]);
    let dirty =
        p.b.open(p.br.join("Note/Note.typ"), Some(p.br.clone()))
            .unwrap();
    let draft = format!("{}\nunsaved receiver proposal", dirty.source);
    p.b.persist_draft(&dirty.session_id, &draft).unwrap();
    let incoming = save(&mut p.a, &p.ar, "Note", "remote saved");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let req = apply(&p);
    let preview = p.b.prepare_operation(req.clone()).unwrap();
    assert_eq!(
        fs::read_to_string(p.br.join("Note/Note.typ")).unwrap(),
        dirty.source
    );
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        fs::read_to_string(p.br.join("Note/Note.typ")).unwrap(),
        incoming
    );
    assert_eq!(
        p.b.read_draft(&p.br.join("Note/Note.typ"))
            .unwrap()
            .unwrap()
            .source,
        draft
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
    assert_eq!(p.b.prepare_operation(req).unwrap().plan_id, preview.plan_id);
    assert_eq!(
        p.b.with_actor("reader", |c| c
            .execute_operation(&preview.operation_id, &preview.plan_id))
            .unwrap_err()
            .code,
        "syncOwnerRequired"
    );
    assert!(
        p.b.save(SaveRequest {
            session_id: dirty.session_id,
            base_id: dirty.base_id,
            operation_id: fresh(),
            source: draft
        })
        .is_err()
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    let opened =
        p.b.open(p.br.join("Note/Note.typ"), Some(p.br.clone()))
            .unwrap();
    p.b.resolve_draft(&opened.session_id, "discardRecovered", None)
        .unwrap();
    let final_source = save(&mut p.b, &p.br, "Note", "receiver next offline save");
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    let view = graph.objects().unwrap();
    assert_eq!(view[p.keys.first().unwrap()].versions.len(), 1);
    assert_eq!(
        graph
            .blob(&view[p.keys.first().unwrap()].versions[0].state.files["Note.typ"])
            .unwrap(),
        final_source.as_bytes()
    );
}

#[test]
fn unrelated_conflict_does_not_block_apply_and_explicit_merge_preserves_both_originals() {
    let mut p = pair(&["Note", "Other"]);
    let note = key(&p, "Note");
    let other = key(&p, "Other");
    let a_source = save(&mut p.a, &p.ar, "Note", "first branch");
    let b_source = save(&mut p.b, &p.br, "Note", "second branch");
    let other_source = save(&mut p.a, &p.ar, "Other", "unrelated fast forward");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: BTreeSet::from([other]),
        },
    );
    assert_eq!(
        fs::read_to_string(p.br.join("Other/Other.typ")).unwrap(),
        other_source
    );
    assert_eq!(
        p.b.prepare_operation(apply(&p)).unwrap_err().code,
        "syncConflict"
    );
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let view = graph.objects().unwrap();
    let sides = &view[&note];
    assert_eq!(sides.versions.len(), 2);
    let mut state = sides.versions[0].state.clone();
    let merged = format!("{a_source}\nexplicitly retained: second branch");
    state
        .files
        .insert("Note.typ".into(), hash(merged.as_bytes()));
    let req = OperationRequest {
        operation_id: fresh(),
        root: display(&p.br),
        intent: OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![SyncResolution {
                object: note.clone(),
                revisions: revisions(sides),
                state,
            }],
            blobs: BTreeMap::from([(hash(merged.as_bytes()), merged.as_bytes().to_vec())]),
        },
    };
    let preview = p.b.prepare_operation(req.clone()).unwrap();
    assert_eq!(
        fs::read_to_string(p.br.join("Note/Note.typ")).unwrap(),
        b_source
    );
    assert_eq!(p.b.prepare_operation(req).unwrap().plan_id, preview.plan_id);
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: BTreeSet::from([note.clone()]),
        },
    );
    assert_eq!(
        fs::read_to_string(p.ar.join("Note/Note.typ")).unwrap(),
        merged
    );
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    assert_eq!(graph.objects().unwrap()[&note].versions.len(), 1);
    assert!(graph.blob(&hash(a_source.as_bytes())).is_some());
    assert!(graph.blob(&hash(b_source.as_bytes())).is_some());
    assert!(
        graph
            .manifests
            .values()
            .any(|m| m.operation_id == preview.operation_id && m.changes[0].before.len() == 2)
    );
}

#[test]
fn source_annotation_transaction_rechecks_every_preimage_before_any_write() {
    let mut p = pair(&["Note"]);
    let source = p.ar.join("Note/Note.typ");
    let annotations = p.ar.join("Note/weftext.annotations.json");
    let cut =
        p.a.read_snapshot(&p.ar, &[display(&source), display(&annotations)])
            .unwrap();
    let mut bytes = cut.files[0].bytes.clone().unwrap();
    bytes.extend_from_slice(b"\nremote annotated source");
    let annotation_bytes = br#"{"version":1,"annotations":[],"review":"kept"}"#.to_vec();
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::UpdateFiles {
            changes: vec![
                FileChange {
                    path: display(&source),
                    base_id: Some(cut.files[0].base_id.clone()),
                    before: cut.files[0].bytes.clone(),
                    after: bytes.clone(),
                },
                FileChange {
                    path: display(&annotations),
                    base_id: Some(cut.files[1].base_id.clone()),
                    before: None,
                    after: annotation_bytes.clone(),
                },
            ],
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    let destination = p.br.join("Note/weftext.annotations.json");
    fs::write(&destination, b"external draft").unwrap();
    assert!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert_ne!(fs::read(p.br.join("Note/Note.typ")).unwrap(), bytes);
    assert_eq!(fs::read(&destination).unwrap(), b"external draft");
}

#[test]
fn incoming_multinode_transaction_cannot_be_applied_as_one_selected_file() {
    let mut p = pair(&["Note", "Other"]);
    let note = key(&p, "Note");
    let paths: Vec<_> = ["Note", "Other"]
        .iter()
        .map(|n| display(&p.ar.join(n).join(format!("{n}.typ"))))
        .collect();
    let cut = p.a.read_snapshot(&p.ar, &paths).unwrap();
    let changes = cut
        .files
        .iter()
        .map(|f| {
            let mut bytes = f.bytes.clone().unwrap();
            bytes.extend_from_slice(b"\ncompound edit");
            FileChange {
                path: f.path.clone(),
                base_id: Some(f.base_id.clone()),
                before: f.bytes.clone(),
                after: bytes,
            }
        })
        .collect();
    run(&mut p.a, &p.ar, OperationIntent::UpdateFiles { changes });
    transfer(
        &p.a,
        &p.ar,
        &p.b,
        &p.identity,
        &BTreeSet::from([note.clone()]),
    );
    let preview = run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: BTreeSet::from([note]),
        },
    );
    assert_eq!(preview.node_map.len(), 2);
    for name in ["Note", "Other"] {
        assert!(
            fs::read_to_string(p.br.join(name).join(format!("{name}.typ")))
                .unwrap()
                .ends_with("compound edit")
        );
    }
}

#[test]
fn received_newer_version_and_local_commit_each_invalidate_reviewed_plan() {
    let mut p = pair(&["Note"]);
    save(&mut p.a, &p.ar, "Note", "first remote");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    save(&mut p.a, &p.ar, "Note", "later remote");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "syncConflictChanged"
    );
    let next = p.b.prepare_operation(apply(&p)).unwrap();
    let local = save(&mut p.b, &p.br, "Note", "local after preview");
    assert_eq!(
        p.b.execute_operation(&next.operation_id, &next.plan_id)
            .unwrap_err()
            .code,
        "syncBasisChanged"
    );
    assert_eq!(
        fs::read_to_string(p.br.join("Note/Note.typ")).unwrap(),
        local
    );
}

#[test]
fn external_before_prepare_is_preserved_and_uncommitted_application_does_not_advance_basis() {
    let mut p = pair(&["Note"]);
    save(&mut p.a, &p.ar, "Note", "remote");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let before =
        p.b.sync_local_state(p.b.sync_setup_for(&p.br).unwrap())
            .unwrap()
            .0;
    p.b.prepare_operation(apply(&p)).unwrap();
    assert_eq!(
        p.b.sync_local_state(p.b.sync_setup_for(&p.br).unwrap())
            .unwrap()
            .0,
        before
    );
    fs::write(p.br.join("Note/Note.typ"), b"unadmitted external draft").unwrap();
    assert!(p.b.prepare_operation(apply(&p)).is_err());
    assert_eq!(
        fs::read(p.br.join("Note/Note.typ")).unwrap(),
        b"unadmitted external draft"
    );
}

#[test]
fn incoming_new_resources_keep_each_actual_node_owner_and_recover_original_afterimages() {
    let mut p = pair(&["Note", "Other"]);
    for (i, name) in ["Note", "Other"].iter().enumerate() {
        let preview =
            p.a.prepare_import_resource(
                &p.ar,
                p.ar.join(name).join(format!("{name}.typ")),
                "sample.bin",
                vec![255, i as u8, 4],
                &fresh(),
            )
            .unwrap();
        p.a.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    }
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let snapshot =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Note/Note.typ"))])
            .unwrap();
    let source_path = snapshot.files[0].path.clone();
    let original_revision = snapshot.files[0].revision.clone().unwrap();
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    assert_eq!(
        preview
            .revisions
            .iter()
            .find(|(path, _)| Path::new(path) == Path::new(&source_path))
            .unwrap()
            .1,
        &original_revision
    );
    assert_eq!(preview.resource_map.len(), 2);
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    for (i, name) in ["Note", "Other"].iter().enumerate() {
        let resources =
            p.b.managed_resource_snapshot(&p.br, p.br.join(name).join(format!("{name}.typ")))
                .unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].bytes, vec![255, i as u8, 4]);
        assert_eq!(resources[0].identity_status, "admitted");
    }
    // Controlled missing-decision fixture, not a hardware power-loss claim.
    fs::rename(
        p.b.journal_path(&preview.operation_id, "commit"),
        p._temp.path().join("retained-commit.json"),
    )
    .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        p.b.with_actor("reader", |c| c
            .resolve_recovery(&preview.operation_id, "adoptAfterImages"))
            .unwrap_err()
            .code,
        "syncOwnerRequired"
    );
    assert_eq!(
        p.b.resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
    for name in ["Note", "Other"] {
        assert_eq!(
            p.b.managed_resource_snapshot(&p.br, p.br.join(name).join(format!("{name}.typ")))
                .unwrap()[0]
                .identity_status,
            "admitted"
        );
    }
    save(&mut p.b, &p.br, "Note", "after original recovery");
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    for v in
        p.a.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()
            .values()
    {
        assert_eq!(v.versions.len(), 1);
    }
}

#[test]
fn merge_requires_exact_conflict_cut_and_is_invalidated_by_another_received_side() {
    let mut p = pair(&["Note"]);
    save(&mut p.a, &p.ar, "Note", "a");
    save(&mut p.b, &p.br, "Note", "b");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    p.b.publish_sync(&p.br).unwrap();
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let view = graph.objects().unwrap();
    let key = p.keys.first().unwrap().clone();
    let sides = &view[&key];
    let mut resolution = SyncResolution {
        object: key,
        revisions: revisions(sides),
        state: sides.versions[0].state.clone(),
    };
    resolution.revisions.pop_first();
    let mut req = OperationRequest {
        operation_id: fresh(),
        root: display(&p.br),
        intent: OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![resolution],
            blobs: BTreeMap::new(),
        },
    };
    assert_eq!(
        p.b.prepare_operation(req.clone()).unwrap_err().code,
        "syncConflictChanged"
    );
    if let OperationIntent::MergeSync { resolutions, .. } = &mut req.intent {
        resolutions[0].revisions = revisions(sides);
    }
    let preview = p.b.prepare_operation(req).unwrap();
    let original = fs::read(p.br.join("Note/Note.typ")).unwrap();
    save(&mut p.a, &p.ar, "Note", "a second offline commit");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "syncConflictChanged"
    );
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), original);
    assert_eq!(
        p.b.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()[p.keys.first().unwrap()]
        .versions
        .len(),
        2
    );
}

#[test]
fn received_registry_update_admits_new_tags_only_on_original_guarded_file() {
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("a-library");
    let br = temp.path().join("b-library");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&ar).unwrap();
    let initial = a.read_tag_registry(&ar).unwrap();
    let tag = a
        .prepare_create_tag(&ar, "original", &initial.base_id, &fresh())
        .unwrap();
    a.execute_operation(&tag.preview.operation_id, &tag.preview.plan_id)
        .unwrap();
    let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
    let keys = BTreeSet::from([ObjectKey::Tags]);
    let mut b = Core::new(temp.path().join("b-private")).unwrap();
    transfer(&a, &ar, &b, &identity, &keys);
    run(
        &mut b,
        &br,
        OperationIntent::CloneSync {
            identity: identity.clone(),
            objects: keys.clone(),
        },
    );
    let initial = a.read_tag_registry(&ar).unwrap();
    let added = a
        .prepare_create_tag(&ar, "incoming", &initial.base_id, &fresh())
        .unwrap();
    a.execute_operation(&added.preview.operation_id, &added.preview.plan_id)
        .unwrap();
    transfer(&a, &ar, &b, &identity, &keys);
    let preview = run(
        &mut b,
        &br,
        OperationIntent::ApplySync {
            identity: identity.clone(),
            objects: keys.clone(),
        },
    );
    assert_eq!(
        b.with_actor("reader", |c| c.operation_details(&preview.operation_id))
            .unwrap_err()
            .code,
        "syncOwnerRequired"
    );
    assert_eq!(
        b.with_actor("reader", |c| c
            .prepared_operation_files(&preview.operation_id))
            .unwrap_err()
            .code,
        "syncOwnerRequired"
    );
    assert_eq!(
        b.with_actor("reader", |c| c.operation_status(&preview.operation_id))
            .unwrap_err()
            .code,
        "syncOwnerRequired"
    );
    let registry = b.read_tag_registry(&br).unwrap();
    assert!(registry.complete);
    assert_eq!(registry.tags.len(), 2);
    assert!(registry.tags.iter().any(|t| t.tag_ref == added.target_ref));
    let path = br.join(".weftext-tags.json");
    let bytes = fs::read(&path).unwrap();
    fs::rename(&path, temp.path().join("retained-original-registry.json")).unwrap();
    fs::write(&path, bytes).unwrap();
    assert!(!b.read_tag_registry(&br).unwrap().complete);
    let current = a.read_tag_registry(&ar).unwrap();
    let next = a
        .prepare_create_tag(
            &ar,
            "after receiver replacement",
            &current.base_id,
            &fresh(),
        )
        .unwrap();
    a.execute_operation(&next.preview.operation_id, &next.preview.plan_id)
        .unwrap();
    transfer(&a, &ar, &b, &identity, &keys);
    assert!(
        b.prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&br),
            intent: OperationIntent::ApplySync {
                identity,
                objects: keys
            }
        })
        .is_err()
    );
}

fn synchronized_tags() -> (Pair, String, String) {
    let mut p = pair(&["Note"]);
    let mut references = Vec::new();
    for name in ["same/name", "same/name"] {
        let basis = p.a.read_tag_registry(&p.ar).unwrap();
        let prepared =
            p.a.prepare_create_tag(&p.ar, name, &basis.base_id, &fresh())
                .unwrap();
        p.a.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
            .unwrap();
        references.push(prepared.target_ref);
    }
    p.keys = BTreeSet::from([ObjectKey::Tags]);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    (p, references.remove(0), references.remove(0))
}

#[test]
fn concurrent_opposite_tag_moves_keep_exact_sides_and_base_and_refuse_cyclic_merge() {
    let (mut p, a, b) = synchronized_tags();
    let base =
        p.a.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()[&ObjectKey::Tags]
            .versions[0]
            .clone();
    let basis = p.a.read_tag_registry(&p.ar).unwrap();
    let left =
        p.a.prepare_move_tag(&p.ar, &a, Some(&b), &basis.base_id, &fresh())
            .unwrap();
    p.a.execute_operation(&left.preview.operation_id, &left.preview.plan_id)
        .unwrap();
    let basis = p.b.read_tag_registry(&p.br).unwrap();
    let right =
        p.b.prepare_move_tag(&p.br, &b, Some(&a), &basis.base_id, &fresh())
            .unwrap();
    p.b.execute_operation(&right.preview.operation_id, &right.preview.plan_id)
        .unwrap();
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let request = apply(&p);
    assert_eq!(
        p.b.prepare_operation(request).unwrap_err().code,
        "syncConflict"
    );
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let view = &graph.objects().unwrap()[&ObjectKey::Tags];
    assert_eq!(view.versions.len(), 2);
    assert_eq!(view.common_bases[0].revision, base.revision);
    let ai = a.rsplit('/').next().unwrap();
    let bi = b.rsplit('/').next().unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"version":2,"tags":[
        {"tagId":ai,"displayName":"same/name","parentTagId":bi},
        {"tagId":bi,"displayName":"same/name","parentTagId":ai}
    ]}))
    .unwrap();
    let mut invalid_state = view.versions[0].state.clone();
    invalid_state
        .files
        .insert(".weftext-tags.json".into(), hash(&bytes));
    invalid_state
        .file_revisions
        .insert(".weftext-tags.json".into(), fresh());
    let before = fs::read(p.br.join(".weftext-tags.json")).unwrap();
    let invalid = OperationRequest {
        operation_id: fresh(),
        root: display(&p.br),
        intent: OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![SyncResolution {
                object: ObjectKey::Tags,
                revisions: revisions(view),
                state: invalid_state,
            }],
            blobs: BTreeMap::from([(hash(&bytes), bytes)]),
        },
    };
    assert!(p.b.prepare_operation(invalid).is_err());
    assert_eq!(fs::read(p.br.join(".weftext-tags.json")).unwrap(), before);
    let chosen = view
        .versions
        .iter()
        .find(|v| {
            let data: serde_json::Value =
                serde_json::from_slice(graph.blob(&v.state.files[".weftext-tags.json"]).unwrap())
                    .unwrap();
            data["tags"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["tagId"] == ai && t["parentTagId"] == bi)
        })
        .unwrap();
    run(
        &mut p.b,
        &p.br,
        OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![SyncResolution {
                object: ObjectKey::Tags,
                revisions: revisions(view),
                state: chosen.state.clone(),
            }],
            blobs: BTreeMap::new(),
        },
    );
    let registry = p.b.read_tag_registry(&p.br).unwrap();
    assert!(registry.complete);
    assert_eq!(
        registry
            .tags
            .iter()
            .find(|t| t.tag_ref == a)
            .unwrap()
            .parent_tag_id
            .as_deref(),
        Some(bi)
    );
    for version in view.versions.iter().chain(&view.common_bases) {
        let history =
            p.b.read_received_sync_file(
                &p.identity,
                &ObjectKey::Tags,
                &version.revision,
                ".weftext-tags.json",
            )
            .unwrap();
        assert_eq!(
            history.bytes,
            graph
                .blob(&version.state.files[".weftext-tags.json"])
                .unwrap()
        );
    }
}

#[test]
fn retained_legacy_tag_branch_is_readable_but_cannot_install_over_current_hierarchy() {
    let (mut p, a, b) = synchronized_tags();
    let basis = p.a.read_tag_registry(&p.ar).unwrap();
    let moved =
        p.a.prepare_move_tag(&p.ar, &a, Some(&b), &basis.base_id, &fresh())
            .unwrap();
    p.a.execute_operation(&moved.preview.operation_id, &moved.preview.plan_id)
        .unwrap();
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let current = graph.objects().unwrap()[&ObjectKey::Tags].versions[0].clone();
    let bytes = serde_json::to_vec(&serde_json::json!({"version":1,"tags":[
        {"tagId":a.rsplit('/').next().unwrap(),"displayName":"legacy offline name"},
        {"tagId":b.rsplit('/').next().unwrap(),"displayName":"same/name"}
    ]}))
    .unwrap();
    let revision = fresh();
    let mut state = current.state.clone();
    state
        .files
        .insert(".weftext-tags.json".into(), hash(&bytes));
    state
        .file_revisions
        .insert(".weftext-tags.json".into(), fresh());
    let manifest = Manifest {
        version: 1,
        workspace_id: p.identity.workspace_id.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: graph.heads(),
        changes: vec![Change {
            object: ObjectKey::Tags,
            before: BTreeSet::from([current.revision]),
            revision: revision.clone(),
            after: state,
        }],
        dependencies: vec![],
    };
    p.b.receive_sync(
        &p.identity,
        SyncBundle {
            manifests: BTreeMap::from([(manifest.id().unwrap(), manifest.clone())]),
            blobs: BTreeMap::new(),
        },
    )
    .unwrap();
    let before = fs::read(p.br.join(".weftext-tags.json")).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&before).unwrap()["version"],
        2
    );
    assert!(p.b.read_tag_registry(&p.br).unwrap().complete);
    assert_eq!(
        p.b.read_received_sync_file(
            &p.identity,
            &ObjectKey::Tags,
            &revision,
            ".weftext-tags.json",
        )
        .unwrap_err()
        .code,
        "syncIncomplete",
        "A readable current v2 tree cannot replace the missing exact v1 blob"
    );
    assert_eq!(fs::read(p.br.join(".weftext-tags.json")).unwrap(), before);
    p.b.receive_sync(
        &p.identity,
        SyncBundle {
            manifests: BTreeMap::from([(manifest.id().unwrap(), manifest)]),
            blobs: BTreeMap::from([(hash(&bytes), bytes.clone())]),
        },
    )
    .unwrap();
    let retained =
        p.b.read_received_sync_file(
            &p.identity,
            &ObjectKey::Tags,
            &revision,
            ".weftext-tags.json",
        )
        .unwrap();
    assert_eq!(retained.bytes, bytes);
    let historical =
        crate::directory_identity::decode_historical_tag_document(&retained.bytes).unwrap();
    assert!(
        historical
            .tags
            .iter()
            .all(|tag| tag.parent_tag_id.is_none())
    );
    let request = apply(&p);
    assert!(p.b.prepare_operation(request).is_err());
    assert_eq!(fs::read(p.br.join(".weftext-tags.json")).unwrap(), before);
    let current = p.b.read_tag_registry(&p.br).unwrap();
    assert!(current.complete);
    assert_eq!(
        current
            .tags
            .iter()
            .find(|t| t.tag_ref == a)
            .unwrap()
            .parent_tag_id
            .as_deref(),
        b.rsplit('/').next()
    );
}

#[test]
fn partial_replica_hydrates_new_tree_and_compound_existing_update_in_one_decision() {
    let mut p = pair(&["Note"]);
    let new = p.a.create_note_fixture(&p.ar, &p.ar, "New").unwrap();
    let child =
        p.a.create_note_fixture(&p.ar, p.ar.join("New"), "Child")
            .unwrap();
    let resource =
        p.a.prepare_import_resource(&p.ar, &child.path, "new.bin", vec![1, 0, 255], &fresh())
            .unwrap();
    p.a.execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    let child_key = ObjectKey::Node(child.node_id.clone().unwrap());
    let new_key = ObjectKey::Node(new.node_id.clone().unwrap());
    let paths = [display(&p.ar.join("Note/Note.typ")), child.path.clone()];
    let cut = p.a.read_snapshot(&p.ar, &paths).unwrap();
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::UpdateFiles {
            changes: cut
                .files
                .iter()
                .map(|f| {
                    let mut after = f.bytes.clone().unwrap();
                    after.extend_from_slice(b"\ncompound with hydrated child");
                    FileChange {
                        path: f.path.clone(),
                        base_id: Some(f.base_id.clone()),
                        before: f.bytes.clone(),
                        after,
                    }
                })
                .collect(),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let req = apply(&p);
    let preview = p.b.prepare_operation(req.clone()).unwrap();
    assert_eq!(p.b.prepare_operation(req).unwrap().plan_id, preview.plan_id);
    assert!(!p.br.join("New").exists());
    assert_eq!(
        p.b.sync_binding_info(&p.br)
            .unwrap()
            .installed_objects
            .len(),
        1
    );
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let opened =
        p.b.open(p.br.join("New/Child/Child.typ"), Some(p.br.clone()))
            .unwrap();
    assert_eq!(opened.node_id, child.node_id);
    assert!(opened.source.ends_with("compound with hydrated child"));
    assert!(
        fs::read_to_string(p.br.join("Note/Note.typ"))
            .unwrap()
            .ends_with("compound with hydrated child")
    );
    let resources = p.b.managed_resource_snapshot(&p.br, &opened.path).unwrap();
    assert_eq!(resources[0].bytes, vec![1, 0, 255]);
    assert_eq!(resources[0].identity_status, "admitted");
    let info = p.b.sync_binding_info(&p.br).unwrap();
    assert!(info.installed_objects.contains(&child_key));
    assert!(info.installed_objects.contains(&new_key));
    assert_eq!(info.pending_publications, 0);
    let source = format!("{}\nlocal hydrated edit", opened.source);
    p.b.save(SaveRequest {
        session_id: opened.session_id,
        base_id: opened.base_id,
        operation_id: fresh(),
        source: source.clone(),
    })
    .unwrap();
    transfer(
        &p.b,
        &p.br,
        &p.a,
        &p.identity,
        &BTreeSet::from([child_key.clone()]),
    );
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    assert_eq!(graph.objects().unwrap()[&child_key].versions.len(), 1);
    assert!(graph.blob(&hash(source.as_bytes())).is_some());
}

#[test]
fn hydration_rechecks_absent_directory_and_does_not_advance_existing_source_on_collision() {
    let mut p = pair(&["Note"]);
    let new = p.a.create_note_fixture(&p.ar, &p.ar, "New").unwrap();
    p.keys.insert(ObjectKey::Node(new.node_id.unwrap()));
    save(&mut p.a, &p.ar, "Note", "incoming existing update");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let before = fs::read(p.br.join("Note/Note.typ")).unwrap();
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    fs::create_dir(p.br.join("New")).unwrap();
    fs::write(p.br.join("New/user.txt"), "external owner").unwrap();
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "exists"
    );
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), before);
    assert_eq!(
        fs::read_to_string(p.br.join("New/user.txt")).unwrap(),
        "external owner"
    );
    assert!(!p.br.join("New/New.typ").exists());
    assert!(!p.b.journal_path(&preview.operation_id, "install").exists());
}

#[test]
fn hydrated_node_afterimage_recovery_uses_original_objects_and_keeps_unrelated_session() {
    let mut p = pair(&["Note"]);
    let unrelated =
        p.b.open(p.br.join("Note/Note.typ"), Some(p.br.clone()))
            .unwrap();
    let new = p.a.create_note_fixture(&p.ar, &p.ar, "New").unwrap();
    let selected = BTreeSet::from([ObjectKey::Node(new.node_id.clone().unwrap())]);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    let preview = run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: selected.clone(),
        },
    );
    assert_eq!(
        p.b.sessions[&unrelated.session_id].base_id,
        unrelated.base_id
    );
    fs::rename(
        p.b.journal_path(&preview.operation_id, "commit"),
        p._temp.path().join("retained-hydration-commit.json"),
    )
    .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        p.b.resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        p.b.open(p.br.join("New/New.typ"), Some(p.br.clone()))
            .unwrap()
            .node_id,
        new.node_id
    );
    assert!(
        p.b.sync_binding_info(&p.br)
            .unwrap()
            .selected_objects
            .unwrap()
            .is_superset(&selected)
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
}

#[test]
fn hydration_carries_new_directory_bookmark_root_bookmark_tags_and_owned_child() {
    let mut p = pair(&["Note"]);
    let folder = p.ar.join("Folder");
    fs::create_dir(&folder).unwrap();
    let bookmark =
        p.a.prepare_directory_admission(&p.ar, &folder, &fresh())
            .unwrap();
    p.a.execute_operation(&bookmark.preview.operation_id, &bookmark.preview.plan_id)
        .unwrap();
    let root_bookmark =
        p.a.prepare_directory_admission(&p.ar, &p.ar, &fresh())
            .unwrap();
    p.a.execute_operation(
        &root_bookmark.preview.operation_id,
        &root_bookmark.preview.plan_id,
    )
    .unwrap();
    let child = p.a.create_note_fixture(&p.ar, &folder, "Child").unwrap();
    let registry = p.a.read_tag_registry(&p.ar).unwrap();
    let tag =
        p.a.prepare_create_tag(&p.ar, "new synchronized tag", &registry.base_id, &fresh())
            .unwrap();
    p.a.execute_operation(&tag.preview.operation_id, &tag.preview.plan_id)
        .unwrap();
    p.a.publish_sync(&p.ar).unwrap();
    let all: BTreeSet<_> =
        p.a.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()
            .keys()
            .cloned()
            .collect();
    assert_eq!(all.len(), 6);
    assert!(all.contains(&ObjectKey::Rules));
    transfer(&p.a, &p.ar, &p.b, &p.identity, &all);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: all,
        },
    );
    assert_eq!(
        p.b.open(p.br.join("Folder/Child/Child.typ"), Some(p.br.clone()))
            .unwrap()
            .node_id,
        child.node_id
    );
    assert_eq!(
        p.b.resolve_navigation_target(&p.br, &bookmark.target_ref)
            .unwrap()
            .status,
        "managed"
    );
    assert_eq!(
        p.b.resolve_navigation_target(&p.br, &root_bookmark.target_ref)
            .unwrap()
            .status,
        "managed"
    );
    let receiver_tags = p.b.read_tag_registry(&p.br).unwrap();
    assert!(receiver_tags.complete);
    assert_eq!(receiver_tags.tags[0].tag_ref, tag.target_ref);
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        p.b.resolve_navigation_target(&p.br, &root_bookmark.target_ref)
            .unwrap()
            .status,
        "managed"
    );
    let newborn =
        p.b.create_note_fixture(&p.br, p.br.join("Folder"), "LocalChild")
            .unwrap();
    p.b.publish_sync(&p.br).unwrap();
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    assert!(
        graph
            .objects()
            .unwrap()
            .contains_key(&ObjectKey::Node(newborn.node_id.unwrap()))
    );
}

fn node_snapshot(core: &mut Core, root: &Path, name: &str) -> weftext_knowledge::NodeSnapshot {
    let file = core
        .read_snapshot(
            root,
            &[display(&root.join(name).join(format!("{name}.typ")))],
        )
        .unwrap()
        .files
        .remove(0);
    weftext_knowledge::NodeSnapshot {
        owner: weftext_knowledge::NodeRef {
            workspace_id: core.require_workspace(root).unwrap().workspace_id,
            node_id: file.node_id.unwrap(),
        },
        path: file.path,
        library_path: format!("{name}/{name}.typ"),
        revision_id: file.revision.unwrap(),
        source: String::from_utf8(file.bytes.unwrap()).unwrap(),
        title: None,
        aliases: vec![],
        labels: vec![],
        labels_complete: false,
    }
}

#[test]
fn actual_source_and_resource_revisions_keep_annotation_anchor_exact_after_clone_and_update() {
    use weftext_knowledge as knowledge;
    let mut p = pair(&["Note"]);
    let baseline = node_snapshot(&mut p.a, &p.ar, "Note");
    let copied = node_snapshot(&mut p.b, &p.br, "Note");
    assert_eq!(baseline.revision_id, copied.revision_id);
    let old_anchor =
        knowledge::source_anchor(&baseline, 0, baseline.source.len(), baseline.source.clone())
            .unwrap();
    assert_eq!(
        knowledge::resolve_anchor(&old_anchor, Some(&copied), &[]).status,
        "exact"
    );
    save(&mut p.a, &p.ar, "Note", "待批注文字");
    let current = node_snapshot(&mut p.a, &p.ar, "Note");
    let start = current.source.find("待批注文字").unwrap();
    let anchor =
        knowledge::source_anchor(&current, start, current.source.len(), "待批注文字".into())
            .unwrap();
    let record = knowledge::AnnotationRecord {
        annotation_id: fresh(),
        reply_to: None,
        attribution: "reviewer".into(),
        created_at: "2026-10-14T00:00:00Z".into(),
        purpose: "comment".into(),
        body: "检查这个位置".into(),
        labels: vec![],
        appearance: serde_json::json!({}),
        review_state: "open".into(),
        target: knowledge::AnnotationTarget::SourceRange { anchor },
        suggestion: None,
        target_history: vec![],
        unknown: BTreeMap::new(),
    };
    let annotations = knowledge::decode_annotations(br#"{"version":1,"annotations":[]}"#)
        .unwrap()
        .create(record)
        .unwrap();
    let annotation_path = p.ar.join("Note/weftext.annotations.json");
    let cut =
        p.a.read_snapshot(&p.ar, &[current.path.clone(), display(&annotation_path)])
            .unwrap();
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::UpdateFiles {
            changes: vec![
                FileChange {
                    path: current.path,
                    base_id: Some(cut.files[0].base_id.clone()),
                    before: cut.files[0].bytes.clone(),
                    after: cut.files[0].bytes.clone().unwrap(),
                },
                FileChange {
                    path: display(&annotation_path),
                    base_id: Some(cut.files[1].base_id.clone()),
                    before: None,
                    after: annotations.clone(),
                },
            ],
        },
    );
    let resource =
        p.a.prepare_import_resource(
            &p.ar,
            p.ar.join("Note/Note.typ"),
            "image.bin",
            vec![1, 2, 255],
            &fresh(),
        )
        .unwrap();
    p.a.execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    let original_resource =
        p.a.managed_resource_snapshot(&p.ar, p.ar.join("Note/Note.typ"))
            .unwrap()
            .remove(0);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let intent = apply(&p).intent;
    run(&mut p.b, &p.br, intent);
    let receiver = node_snapshot(&mut p.b, &p.br, "Note");
    assert_eq!(receiver.revision_id, current.revision_id);
    let document = knowledge::decode_annotations(
        &fs::read(p.br.join("Note/weftext.annotations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        knowledge::annotation_views(&document, Some(&receiver), &[])[0].target_state,
        "exact"
    );
    assert_eq!(
        fs::read(p.br.join("Note/weftext.annotations.json")).unwrap(),
        annotations
    );
    assert_eq!(
        p.b.managed_resource_snapshot(&p.br, p.br.join("Note/Note.typ"))
            .unwrap()[0]
            .revision,
        original_resource.revision
    );
    assert_eq!(
        p.b.read_source_revision(&p.br, &receiver.owner.node_id, &receiver.revision_id)
            .unwrap(),
        receiver.source.as_bytes()
    );
    let reopened_session =
        p.b.open(p.br.join("Note/Note.typ"), Some(p.br.clone()))
            .unwrap();
    let no_op =
        p.b.save(SaveRequest {
            session_id: reopened_session.session_id,
            base_id: reopened_session.base_id,
            operation_id: fresh(),
            source: reopened_session.source,
        })
        .unwrap();
    assert_eq!(
        no_op.receipt.revision_id.as_deref(),
        Some(current.revision_id.as_str())
    );
    assert_eq!(
        node_snapshot(&mut p.b, &p.br, "Note").revision_id,
        current.revision_id
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    let reopened = node_snapshot(&mut p.b, &p.br, "Note");
    assert_eq!(
        knowledge::annotation_views(&document, Some(&reopened), &[])[0].target_state,
        "exact"
    );
    let third_root = p._temp.path().join("third-library");
    let mut third = Core::new(p._temp.path().join("third-private")).unwrap();
    transfer(&p.a, &p.ar, &third, &p.identity, &p.keys);
    run(
        &mut third,
        &third_root,
        OperationIntent::CloneSync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    let copied = node_snapshot(&mut third, &third_root, "Note");
    let copied_annotations = knowledge::decode_annotations(
        &fs::read(third_root.join("Note/weftext.annotations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        knowledge::annotation_views(&copied_annotations, Some(&copied), &[])[0].target_state,
        "exact"
    );
}

#[test]
fn equal_bytes_explicit_branch_choice_keeps_the_chosen_revision_through_restart() {
    let mut p = pair(&["Note"]);
    save(&mut p.a, &p.ar, "Note", "same independent text");
    save(&mut p.b, &p.br, "Note", "same independent text");
    let a_snapshot = node_snapshot(&mut p.a, &p.ar, "Note");
    let b_snapshot = node_snapshot(&mut p.b, &p.br, "Note");
    assert_eq!(a_snapshot.source, b_snapshot.source);
    assert_ne!(a_snapshot.revision_id, b_snapshot.revision_id);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    p.b.publish_sync(&p.br).unwrap();
    let objects =
        p.b.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap();
    let key = p.keys.first().unwrap().clone();
    let sides = &objects[&key];
    let selected = sides
        .versions
        .iter()
        .find(|v| v.state.file_revisions["Note.typ"] == a_snapshot.revision_id)
        .unwrap()
        .state
        .clone();
    run(
        &mut p.b,
        &p.br,
        OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![SyncResolution {
                object: key,
                revisions: revisions(sides),
                state: selected,
            }],
            blobs: BTreeMap::new(),
        },
    );
    assert_eq!(
        node_snapshot(&mut p.b, &p.br, "Note").revision_id,
        a_snapshot.revision_id
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        node_snapshot(&mut p.b, &p.br, "Note").revision_id,
        a_snapshot.revision_id
    );
}

#[test]
fn existing_node_can_receive_a_new_resource_subdirectory_with_original_recovery() {
    let mut p = pair(&["Note"]);
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    let view = graph.objects().unwrap();
    let key = p.keys.first().unwrap().clone();
    let base = &view[&key].versions[0];
    let mut state = base.state.clone();
    let resource_id = fresh();
    let manifest = serde_json::to_vec(&serde_json::json!({"version":1,"resources":[{"resourceId":resource_id,"path":"assets/file.bin"}]})).unwrap();
    let binary = vec![0, 255, 1, 3];
    state
        .files
        .insert("weftext.resources.json".into(), hash(&manifest));
    state
        .file_revisions
        .insert("weftext.resources.json".into(), fresh());
    state.files.insert("assets/file.bin".into(), hash(&binary));
    state
        .file_revisions
        .insert("assets/file.bin".into(), fresh());
    let received = Manifest {
        version: 1,
        workspace_id: p.identity.workspace_id.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::from([base.commit.clone()]),
        dependencies: vec![],
        changes: vec![Change {
            object: key,
            before: BTreeSet::from([base.revision.clone()]),
            revision: fresh(),
            after: state,
        }],
    };
    p.b.receive_sync(
        &p.identity,
        SyncBundle {
            manifests: BTreeMap::from([(received.id().unwrap(), received)]),
            blobs: BTreeMap::from([(hash(&manifest), manifest), (hash(&binary), binary.clone())]),
        },
    )
    .unwrap();
    let intent = apply(&p).intent;
    let preview = run(&mut p.b, &p.br, intent);
    let resources =
        p.b.managed_resource_snapshot(&p.br, p.br.join("Note/Note.typ"))
            .unwrap();
    assert_eq!(resources[0].resource_id, resource_id);
    assert_eq!(resources[0].bytes, binary);
    assert_eq!(resources[0].identity_status, "admitted");
    fs::rename(
        p.b.journal_path(&preview.operation_id, "commit"),
        p._temp.path().join("retained-resource-commit.json"),
    )
    .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        p.b.resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        p.b.managed_resource_snapshot(&p.br, p.br.join("Note/Note.typ"))
            .unwrap()[0]
            .identity_status,
        "admitted"
    );
}

#[test]
fn received_rename_retains_resources_history_draft_and_physical_identity() {
    let mut p = pair(&["Note", "Other"]);
    let note = key(&p, "Note");
    save(&mut p.a, &p.ar, "Note", "saved before rename");
    let resource =
        p.a.prepare_import_resource(
            &p.ar,
            p.ar.join("Note/Note.typ"),
            "raw.bin",
            vec![0, 1, 255],
            &fresh(),
        )
        .unwrap();
    p.a.execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    let before =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Note/Note.typ"))])
            .unwrap()
            .files
            .remove(0);
    let physical = capture_optional(&p.br.join("Note/Note.typ"))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    let dirty =
        p.b.open(p.br.join("Note/Note.typ"), Some(p.br.clone()))
            .unwrap();
    p.b.persist_draft(&dirty.session_id, "unsaved local proposal")
        .unwrap();
    let other =
        p.b.open(p.br.join("Other/Other.typ"), Some(p.br.clone()))
            .unwrap();
    let history = p.b.list_history(&dirty.session_id).unwrap();
    fs::write(p.br.join("Note/local-companion.txt"), "only here").unwrap();
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "Renamed".into(),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let request = apply(&p);
    let preview = p.b.prepare_operation(request.clone()).unwrap();
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert!(!p.br.join("Note").exists());
    assert_eq!(
        fs::read(p.br.join("Renamed/raw.bin")).unwrap(),
        vec![0, 1, 255]
    );
    assert_eq!(
        fs::read_to_string(p.br.join("Renamed/local-companion.txt")).unwrap(),
        "only here"
    );
    assert_eq!(
        capture_optional(&p.br.join("Renamed/Renamed.typ"))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        physical
    );
    assert_eq!(p.b.sessions[&other.session_id].base_id, other.base_id);
    assert_eq!(
        p.b.read_draft(&p.br.join("Renamed/Renamed.typ"))
            .unwrap()
            .unwrap()
            .source,
        "unsaved local proposal"
    );
    let after =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Renamed/Renamed.typ"))])
            .unwrap()
            .files
            .remove(0);
    assert_eq!(after.revision, before.revision);
    let reopened =
        p.b.open(p.br.join("Renamed/Renamed.typ"), Some(p.br.clone()))
            .unwrap();
    assert_eq!(
        p.b.list_history(&reopened.session_id)
            .unwrap()
            .versions
            .len(),
        history.versions.len()
    );
    assert_eq!(
        p.b.prepare_operation(request).unwrap().plan_id,
        preview.plan_id
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
    p.b.resolve_draft(&reopened.session_id, "discardRecovered", None)
        .unwrap();
    save(&mut p.b, &p.br, "Renamed", "after received rename");
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    assert_eq!(
        p.a.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()[&note]
            .versions
            .len(),
        1
    );
}

#[test]
fn received_subtree_move_updates_child_locations_and_retirement_uses_real_trash_consumers() {
    let mut p = pair(&["Note", "Destination"]);
    let child =
        p.a.create_note_fixture(&p.ar, p.ar.join("Note"), "Child")
            .unwrap();
    let child_key = ObjectKey::Node(child.node_id.clone().unwrap());
    p.keys.insert(child_key.clone());
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    let parent_key = key(&p, "Note");
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::MoveNode {
            source: display(&p.ar.join("Note")),
            parent: display(&p.ar.join("Destination")),
        },
    );
    let selected = BTreeSet::from([parent_key.clone()]);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: selected.clone(),
        },
    );
    assert!(p.br.join("Destination/Note/Child/Child.typ").exists());
    let moved_child =
        p.b.open(
            p.br.join("Destination/Note/Child/Child.typ"),
            Some(p.br.clone()),
        )
        .unwrap();
    assert!(
        !p.b.list_history(&moved_child.session_id)
            .unwrap()
            .versions
            .is_empty()
    );
    let child_owner = weftext_knowledge::NodeRef {
        workspace_id: p.identity.workspace_id.clone(),
        node_id: child.node_id.clone().unwrap(),
    };
    p.b.qualified_query_node_at(
        &p.br,
        &p.br.join("Destination/Note/Child/Child.typ"),
        &child_owner,
        "live",
    )
    .unwrap();
    let setup = p.b.sync_setup_for(&p.br).unwrap();
    let (local, _) = p.b.sync_local_state(setup).unwrap();
    assert_eq!(
        Path::new(&local[&child_key].directory),
        p.br.join("Destination/Note/Child")
    );
    let trashed = run(
        &mut p.a,
        &p.ar,
        OperationIntent::TrashNode {
            source: display(&p.ar.join("Destination/Note")),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    let receiver_trash = run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: selected.clone(),
        },
    );
    let entries = p.b.trash_entries(&p.br).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].operation_id, receiver_trash.operation_id);
    let live = p.b.admitted_snapshot(&p.br).unwrap();
    assert!(live.complete);
    assert_eq!(live.membership_ids.len(), 1);
    let trash = p.b.trash_admitted_snapshot(&p.br).unwrap();
    assert!(trash.complete);
    assert_eq!(trash.membership_ids.len(), 2);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RestoreNode {
            trash_operation_id: trashed.operation_id,
            node_id: None,
            parent: display(&p.ar),
            name: "Restored".into(),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: selected,
        },
    );
    assert!(p.br.join("Restored/Restored.typ").exists());
    assert!(p.br.join("Restored/Child/Child.typ").exists());
    p.b.qualified_query_node_at(
        &p.br,
        &p.br.join("Restored/Child/Child.typ"),
        &child_owner,
        "live",
    )
    .unwrap();
    assert!(p.b.trash_entries(&p.br).unwrap().is_empty());
    assert_eq!(
        p.b.admitted_snapshot(&p.br).unwrap().membership_ids.len(),
        3
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    let restored_child =
        p.b.open(p.br.join("Restored/Child/Child.typ"), Some(p.br.clone()))
            .unwrap();
    assert!(
        !p.b.list_history(&restored_child.session_id)
            .unwrap()
            .versions
            .is_empty()
    );
    assert_eq!(
        p.b.open(p.br.join("Restored/Child/Child.typ"), Some(p.br.clone()))
            .unwrap()
            .node_id,
        child.node_id
    );
}

#[test]
fn received_relocation_rechecks_all_preimages_and_recovery_keeps_unknown_effects_guarded() {
    let mut p = pair(&["Note"]);
    fs::write(p.br.join("Note/companion.bin"), [0, 7]).unwrap();
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "Moved".into(),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    fs::write(p.br.join("Note/companion.bin"), [8, 9]).unwrap();
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "conflict"
    );
    assert!(!p.b.journal_path(&preview.operation_id, "install").exists());
    assert!(p.br.join("Note/Note.typ").exists());
    let next = p.b.prepare_operation(apply(&p)).unwrap();
    p.b.execute_operation(&next.operation_id, &next.plan_id)
        .unwrap();
    fs::rename(
        p.b.journal_path(&next.operation_id, "commit"),
        p._temp.path().join("retained-move-decision.json"),
    )
    .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    let recovery = p.b.inspect_recovery(&next.operation_id).unwrap();
    assert_eq!(
        recovery.coherence,
        "knownAfter",
        "{}",
        serde_json::to_string(&recovery).unwrap()
    );
    fs::write(p.br.join("Moved/companion.bin"), [4, 5]).unwrap();
    assert_eq!(
        p.b.inspect_recovery(&next.operation_id).unwrap().coherence,
        "thirdState"
    );
    assert!(
        p.b.resolve_recovery(&next.operation_id, "adoptAfterImages")
            .is_err()
    );
    fs::write(p.br.join("Moved/companion.bin"), [8, 9]).unwrap();
    let expected_revision = p.b.operations[&next.operation_id]
        .transaction
        .as_ref()
        .unwrap()
        .preview
        .revisions
        .iter()
        .find(|(path, _)| Path::new(path) == p.br.join("Moved/Moved.typ"))
        .unwrap()
        .1
        .clone();
    assert_eq!(
        p.b.resolve_recovery(&next.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
    let source =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Moved/Moved.typ"))])
            .unwrap()
            .files
            .remove(0);
    assert_eq!(source.revision.as_ref(), Some(&expected_revision));
}

#[test]
fn received_move_and_content_change_both_refuse_an_occupied_destination() {
    let mut p = pair(&["Note"]);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "New".into(),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let prepared = p.b.prepare_operation(apply(&p)).unwrap();
    fs::create_dir(p.br.join("New")).unwrap();
    fs::write(p.br.join("New/owner.txt"), "user directory").unwrap();
    assert_eq!(
        p.b.execute_operation(&prepared.operation_id, &prepared.plan_id)
            .unwrap_err()
            .code,
        "exists"
    );
    assert!(!p.b.journal_path(&prepared.operation_id, "install").exists());
    assert_eq!(
        fs::read_to_string(p.br.join("New/owner.txt")).unwrap(),
        "user directory"
    );
    save(
        &mut p.a,
        &p.ar,
        "New",
        "change while receiver has old placement",
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    assert_eq!(p.b.prepare_operation(apply(&p)).unwrap_err().code, "exists");
    assert!(p.br.join("Note/Note.typ").exists());
}

#[test]
fn concurrent_renames_require_explicit_choice_and_preserve_both_portable_branches() {
    let mut p = pair(&["Note"]);
    let node = p.keys.first().unwrap().clone();
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "Left".into(),
        },
    );
    run(
        &mut p.b,
        &p.br,
        OperationIntent::RenameNode {
            source: display(&p.br.join("Note")),
            name: "Right".into(),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    assert_eq!(
        p.b.prepare_operation(apply(&p)).unwrap_err().code,
        "syncConflict"
    );
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let objects = graph.objects().unwrap();
    let view = &objects[&node];
    assert_eq!(view.versions.len(), 2);
    let resolution = SyncResolution {
        object: node.clone(),
        revisions: revisions(view),
        state: view
            .versions
            .iter()
            .find(|v| v.state.name == "Left")
            .unwrap()
            .state
            .clone(),
    };
    run(
        &mut p.b,
        &p.br,
        OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![resolution],
            blobs: BTreeMap::new(),
        },
    );
    assert!(p.br.join("Left/Left.typ").exists());
    assert!(!p.br.join("Right").exists());
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    let merged = graph.objects().unwrap()[&node].versions[0].clone();
    assert_eq!(graph.manifest(&merged.commit).unwrap().parents.len(), 2);
    assert_eq!(merged.state.name, "Left");
    assert!(
        view.versions
            .iter()
            .all(|v| graph.manifest(&v.commit).is_some())
    );
}

#[test]
fn locally_restore_received_trash_then_publish_uses_received_retirement_as_predecessor() {
    let mut p = pair(&["Note"]);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::TrashNode {
            source: display(&p.ar.join("Note")),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let preview = run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(p.b.trash_entries(&p.br).unwrap().len(), 1);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::RestoreNode {
            trash_operation_id: preview.operation_id,
            node_id: None,
            parent: display(&p.br),
            name: "Recovered".into(),
        },
    );
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    assert!(p.ar.join("Recovered/Recovered.typ").exists());
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    let objects = graph.objects().unwrap();
    let view = &objects[p.keys.first().unwrap()];
    assert_eq!(view.versions.len(), 1);
    assert!(!view.versions[0].state.retired);
}

#[test]
fn received_move_rejects_same_byte_resource_replacement_and_changed_parent_dependency() {
    let mut p = pair(&["Note", "Parent"]);
    let resource =
        p.a.prepare_import_resource(
            &p.ar,
            p.ar.join("Note/Note.typ"),
            "asset.bin",
            vec![2, 3],
            &fresh(),
        )
        .unwrap();
    p.a.execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::MoveNode {
            source: display(&p.ar.join("Note")),
            parent: display(&p.ar.join("Parent")),
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    let original_parent = fs::read(p.br.join("Parent/Parent.typ")).unwrap();
    fs::write(p.br.join("Parent/Parent.typ"), "external parent source").unwrap();
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "conflict"
    );
    assert!(!p.b.journal_path(&preview.operation_id, "install").exists());
    fs::write(p.br.join("Parent/Parent.typ"), original_parent).unwrap();
    fs::rename(
        p.br.join("Note/asset.bin"),
        p._temp.path().join("original-asset.bin"),
    )
    .unwrap();
    fs::write(p.br.join("Note/asset.bin"), [2, 3]).unwrap();
    assert_eq!(
        p.b.prepare_operation(apply(&p)).unwrap_err().code,
        "syncExternalChange"
    );
    assert!(p.br.join("Note/Note.typ").exists());
}

fn compound_move_pair() -> Pair {
    let mut p = pair(&["Note", "Other"]);
    let selected = BTreeSet::from([key(&p, "Note")]);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "New".into(),
        },
    );
    for name in ["New", "Other"] {
        let resource =
            p.a.prepare_import_resource(
                &p.ar,
                p.ar.join(name).join(format!("{name}.typ")),
                "new.bin",
                vec![0, 255, 9],
                &fresh(),
            )
            .unwrap();
        p.a.execute_operation(&resource.operation_id, &resource.plan_id)
            .unwrap();
    }
    let paths = [
        p.ar.join("New/New.typ"),
        p.ar.join("Other/Other.typ"),
        p.ar.join("New/weftext.annotations.json"),
    ];
    let cut =
        p.a.read_snapshot(&p.ar, &paths.iter().map(|p| display(p)).collect::<Vec<_>>())
            .unwrap();
    let changes = cut
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let after = if i == 2 {
                br#"{"version":1,"annotations":[],"review":"received together"}"#.to_vec()
            } else {
                let mut bytes = file.bytes.clone().unwrap();
                bytes.extend_from_slice(b"\nreceived compound source");
                bytes
            };
            FileChange {
                path: file.path.clone(),
                base_id: Some(file.base_id.clone()),
                before: file.bytes.clone(),
                after,
            }
        })
        .collect();
    run(&mut p.a, &p.ar, OperationIntent::UpdateFiles { changes });
    p.keys = selected;
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    p
}

fn simulate_original_namespace_after(p: &mut Pair, preview: &OperationPreview) {
    // Crash fixture keeps the original physical objects and the reviewed plan.
    // It simulates an interrupted original namespace step, never a retry writer.
    crate::journal::private_create(
        &p.b.journal_path(&preview.operation_id, "install"),
        br#"{"testInterruption":true}"#,
    )
    .unwrap();
    fs::rename(p.br.join("Note/Note.typ"), p.br.join("Note/New.typ")).unwrap();
    fs::rename(p.br.join("Note"), p.br.join("New")).unwrap();
}

#[test]
fn compound_received_move_commits_sources_annotations_resources_and_outside_participant_together() {
    let mut p = compound_move_pair();
    let dirty =
        p.b.open(p.br.join("Note/Note.typ"), Some(p.br.clone()))
            .unwrap();
    p.b.persist_draft(&dirty.session_id, "independent draft retained")
        .unwrap();
    let before_history = p.b.list_history(&dirty.session_id).unwrap().versions.len();
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    assert!(
        preview
            .changes
            .iter()
            .any(|c| Path::new(&c.path) == p.br.join("Other/Other.typ"))
    );
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert!(!p.br.join("Note").exists());
    for relative in [
        "New/New.typ",
        "Other/Other.typ",
        "New/weftext.annotations.json",
        "New/new.bin",
        "New/weftext.resources.json",
    ] {
        assert_eq!(
            fs::read(p.br.join(relative)).unwrap(),
            fs::read(p.ar.join(relative)).unwrap()
        );
    }
    let resources =
        p.b.managed_resource_snapshot(&p.br, p.br.join("New/New.typ"))
            .unwrap();
    assert_eq!(resources[0].identity_status, "admitted");
    assert_eq!(resources[0].bytes, [0, 255, 9]);
    let reopened =
        p.b.open(p.br.join("New/New.typ"), Some(p.br.clone()))
            .unwrap();
    assert_eq!(
        p.b.read_draft(&p.br.join("New/New.typ"))
            .unwrap()
            .unwrap()
            .source,
        "independent draft retained"
    );
    let history = p.b.list_history(&reopened.session_id).unwrap();
    assert_eq!(history.versions.len(), before_history + 1);
    let latest = history
        .versions
        .iter()
        .find(|v| v.operation_id == preview.operation_id)
        .unwrap();
    assert_eq!(
        p.b.read_history(&reopened.session_id, &latest.version_id)
            .unwrap()
            .source,
        reopened.source
    );
    let receiver = node_snapshot(&mut p.b, &p.br, "New");
    let author = node_snapshot(&mut p.a, &p.ar, "New");
    assert_eq!(receiver.revision_id, author.revision_id);
    assert_eq!(
        p.b.read_source_revision(&p.br, &reopened.node_id.unwrap(), &receiver.revision_id)
            .unwrap(),
        receiver.source.as_bytes()
    );
    p.b.resolve_draft(&reopened.session_id, "discardRecovered", None)
        .unwrap();
    save(&mut p.b, &p.br, "New", "next offline commit");
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    assert_eq!(
        p.a.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()[p.keys.first().unwrap()]
        .versions
        .len(),
        1
    );
}

#[test]
fn compound_received_move_resumes_only_content_after_proven_original_namespace_completion() {
    let mut p = compound_move_pair();
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    simulate_original_namespace_after(&mut p, &preview);
    let changes = p.b.prepared_operation_files(&preview.operation_id).unwrap();
    let source = changes
        .iter()
        .find(|c| Path::new(&c.path) == p.br.join("New/New.typ"))
        .unwrap();
    fs::write(&source.path, &source.after).unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    let inspected = p.b.inspect_recovery(&preview.operation_id).unwrap();
    assert_eq!(
        inspected.coherence,
        "mixedKnown",
        "{}",
        serde_json::to_string(&inspected).unwrap()
    );
    assert!(
        inspected
            .allowed_actions
            .iter()
            .any(|a| a == "resumeOriginalPlan"),
        "{}",
        serde_json::to_string(&inspected).unwrap()
    );
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    let result =
        p.b.resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap();
    assert_eq!(result.status, "committed");
    assert_eq!(
        p.b.resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .unwrap()
            .status,
        "committed"
    );
    for change in changes {
        assert_eq!(fs::read(&change.path).unwrap(), change.after);
    }
    assert_eq!(
        p.b.managed_resource_snapshot(&p.br, p.br.join("New/New.typ"))
            .unwrap()[0]
            .identity_status,
        "admitted"
    );
    assert_eq!(
        p.b.sync_binding_info(&p.br).unwrap().pending_publications,
        0
    );
    fs::rename(
        p.b.journal_path(&preview.operation_id, "commit"),
        p._temp.path().join("retained-compound-decision.json"),
    )
    .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        p.b.inspect_recovery(&preview.operation_id)
            .unwrap()
            .coherence,
        "knownAfter"
    );
    assert_eq!(
        p.b.resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    let received = node_snapshot(&mut p.b, &p.br, "New");
    let author = node_snapshot(&mut p.a, &p.ar, "New");
    assert_eq!(received.revision_id, author.revision_id);
    save(&mut p.b, &p.br, "New", "after original compound recovery");
}

#[test]
fn compound_received_move_never_resumes_unknown_namespace_or_external_third_state() {
    let mut p = compound_move_pair();
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    crate::journal::private_create(&p.b.journal_path(&preview.operation_id, "install"), b"{}")
        .unwrap();
    fs::rename(p.br.join("Note/Note.typ"), p.br.join("Note/New.typ")).unwrap();
    let intermediate = p.b.inspect_recovery(&preview.operation_id).unwrap();
    assert!(
        !intermediate
            .allowed_actions
            .iter()
            .any(|a| a == "resumeOriginalPlan")
    );
    assert!(
        p.b.resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .is_err()
    );
    assert!(!p.br.join("New").exists());
    fs::rename(p.br.join("Note"), p.br.join("New")).unwrap();
    fs::write(p.br.join("Other/Other.typ"), "external third manuscript").unwrap();
    let other = fs::read(p.br.join("New/New.typ")).unwrap();
    assert_eq!(
        p.b.inspect_recovery(&preview.operation_id)
            .unwrap()
            .coherence,
        "thirdState"
    );
    assert!(
        p.b.resolve_recovery(&preview.operation_id, "resumeOriginalPlan")
            .is_err()
    );
    assert_eq!(fs::read(p.br.join("New/New.typ")).unwrap(), other);
    assert_eq!(
        fs::read_to_string(p.br.join("Other/Other.typ")).unwrap(),
        "external third manuscript"
    );
    assert!(!p.br.join("New/new.bin").exists());
}

#[test]
fn compound_received_move_preflights_new_files_before_any_namespace_or_content_effect() {
    let mut p = compound_move_pair();
    let preview = p.b.prepare_operation(apply(&p)).unwrap();
    let other = fs::read(p.br.join("Other/Other.typ")).unwrap();
    fs::write(p.br.join("Note/new.bin"), "external occupied resource").unwrap();
    assert!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert!(!p.b.journal_path(&preview.operation_id, "install").exists());
    assert!(p.br.join("Note/Note.typ").exists());
    assert!(!p.br.join("New").exists());
    assert_eq!(fs::read(p.br.join("Other/Other.typ")).unwrap(), other);
    assert_eq!(
        fs::read_to_string(p.br.join("Note/new.bin")).unwrap(),
        "external occupied resource"
    );
}

#[test]
fn compound_move_edit_conflict_requires_explicit_both_side_merge() {
    let mut p = pair(&["Note"]);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "New".into(),
        },
    );
    let a_source = save(&mut p.a, &p.ar, "New", "author moved and edited");
    let b_source = save(&mut p.b, &p.br, "Note", "receiver edited independently");
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    assert_eq!(
        p.b.prepare_operation(apply(&p)).unwrap_err().code,
        "syncConflict"
    );
    let node = p.keys.first().unwrap().clone();
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let objects = graph.objects().unwrap();
    let sides = &objects[&node];
    let mut state = sides
        .versions
        .iter()
        .find(|v| v.state.name == "New")
        .unwrap()
        .state
        .clone();
    let merged = format!("{a_source}\nreceiver edited independently");
    state
        .files
        .insert("New.typ".into(), hash(merged.as_bytes()));
    let preview = run(
        &mut p.b,
        &p.br,
        OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![SyncResolution {
                object: node.clone(),
                revisions: revisions(sides),
                state,
            }],
            blobs: BTreeMap::from([(hash(merged.as_bytes()), merged.as_bytes().to_vec())]),
        },
    );
    assert_eq!(
        fs::read_to_string(p.br.join("New/New.typ")).unwrap(),
        merged
    );
    assert!(!p.br.join("Note").exists());
    transfer(&p.b, &p.br, &p.a, &p.identity, &p.keys);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: p.keys.clone(),
        },
    );
    assert_eq!(
        fs::read_to_string(p.ar.join("New/New.typ")).unwrap(),
        merged
    );
    let graph = p.a.read_sync_history(&p.identity).unwrap();
    assert!(graph.blob(&hash(a_source.as_bytes())).is_some());
    assert!(graph.blob(&hash(b_source.as_bytes())).is_some());
    let view = graph.objects().unwrap();
    assert_eq!(view[&node].versions.len(), 1);
    assert_eq!(
        graph
            .manifest(&view[&node].versions[0].commit)
            .unwrap()
            .operation_id,
        preview.operation_id
    );
}

#[test]
fn compound_move_keeps_real_review_anchor_and_existing_resource_revision_after_restart() {
    use weftext_knowledge as knowledge;
    let mut p = pair(&["Note"]);
    let resource =
        p.a.prepare_import_resource(
            &p.ar,
            p.ar.join("Note/Note.typ"),
            "asset.bin",
            vec![1, 2],
            &fresh(),
        )
        .unwrap();
    p.a.execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let intent = apply(&p).intent;
    run(&mut p.b, &p.br, intent);
    let identity = capture_optional(&p.br.join("Note/asset.bin"))
        .unwrap()
        .unwrap()
        .stamp
        .identity;
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::RenameNode {
            source: display(&p.ar.join("Note")),
            name: "New".into(),
        },
    );
    save(&mut p.a, &p.ar, "New", "移动后的批注文字😀");
    let current = node_snapshot(&mut p.a, &p.ar, "New");
    let start = current.source.find("移动后的批注文字").unwrap();
    let anchor = knowledge::source_anchor(
        &current,
        start,
        current.source.len(),
        "移动后的批注文字😀".into(),
    )
    .unwrap();
    let annotations = knowledge::decode_annotations(br#"{"version":1,"annotations":[]}"#)
        .unwrap()
        .create(knowledge::AnnotationRecord {
            annotation_id: fresh(),
            reply_to: None,
            attribution: "reviewer".into(),
            created_at: "2026-10-14T00:00:00Z".into(),
            purpose: "comment".into(),
            body: "Keep the exact moved range".into(),
            labels: vec![],
            appearance: serde_json::json!({}),
            review_state: "open".into(),
            target: knowledge::AnnotationTarget::SourceRange { anchor },
            suggestion: None,
            target_history: vec![],
            unknown: BTreeMap::new(),
        })
        .unwrap();
    let annotation_path = p.ar.join("New/weftext.annotations.json");
    let snapshot =
        p.a.read_snapshot(&p.ar, &[display(&annotation_path)])
            .unwrap()
            .files
            .remove(0);
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::UpdateFiles {
            changes: vec![FileChange {
                path: snapshot.path,
                base_id: Some(snapshot.base_id),
                before: snapshot.bytes,
                after: annotations.clone(),
            }],
        },
    );
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    // Resource replacement is an admitted manifest fixture; local import remains create-only.
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let views = graph.objects().unwrap();
    let node = p.keys.first().unwrap().clone();
    let prior = &views[&node].versions[0];
    let mut updated_state = prior.state.clone();
    let resource_revision = fresh();
    updated_state
        .files
        .insert("asset.bin".into(), hash(&[0, 255, 3]));
    updated_state
        .file_revisions
        .insert("asset.bin".into(), resource_revision.clone());
    let manifest = Manifest {
        version: 1,
        workspace_id: p.identity.workspace_id.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::from([prior.commit.clone()]),
        changes: vec![Change {
            object: node,
            before: BTreeSet::from([prior.revision.clone()]),
            revision: fresh(),
            after: updated_state,
        }],
        dependencies: vec![],
    };
    p.b.receive_sync(
        &p.identity,
        SyncBundle {
            manifests: BTreeMap::from([(manifest.id().unwrap(), manifest)]),
            blobs: BTreeMap::from([(hash(&[0, 255, 3]), vec![0, 255, 3])]),
        },
    )
    .unwrap();
    let intent = apply(&p).intent;
    run(&mut p.b, &p.br, intent);
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    let receiver = node_snapshot(&mut p.b, &p.br, "New");
    assert_eq!(receiver.revision_id, current.revision_id);
    let document = knowledge::decode_annotations(
        &fs::read(p.br.join("New/weftext.annotations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        knowledge::annotation_views(&document, Some(&receiver), &[])[0].target_state,
        "exact"
    );
    assert_eq!(
        capture_optional(&p.br.join("New/asset.bin"))
            .unwrap()
            .unwrap()
            .stamp
            .identity,
        identity
    );
    let received =
        p.b.managed_resource_snapshot(&p.br, p.br.join("New/New.typ"))
            .unwrap()
            .remove(0);
    assert_eq!(received.bytes, [0, 255, 3]);
    assert_eq!(received.revision, Some(resource_revision));
}

#[test]
fn compound_move_rejects_one_resource_identity_claimed_by_distinct_owners() {
    let mut p = compound_move_pair();
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let objects = graph.objects().unwrap();
    let note = objects
        .iter()
        .find(|(_, v)| v.versions[0].state.name == "New")
        .unwrap();
    let other = objects
        .iter()
        .find(|(_, v)| v.versions[0].state.name == "Other")
        .unwrap();
    let resource_id = serde_json::from_slice::<serde_json::Value>(
        graph
            .blob(&note.1.versions[0].state.files["weftext.resources.json"])
            .unwrap(),
    )
    .unwrap()["resources"][0]["resourceId"]
        .clone();
    let mut registry: serde_json::Value = serde_json::from_slice(
        graph
            .blob(&other.1.versions[0].state.files["weftext.resources.json"])
            .unwrap(),
    )
    .unwrap();
    registry["resources"][0]["resourceId"] = resource_id;
    let bytes = serde_json::to_vec(&registry).unwrap();
    let mut other_state = other.1.versions[0].state.clone();
    other_state
        .files
        .insert("weftext.resources.json".into(), hash(&bytes));
    other_state
        .file_revisions
        .insert("weftext.resources.json".into(), fresh());
    let manifest = Manifest {
        version: 1,
        workspace_id: p.identity.workspace_id.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::from([
            note.1.versions[0].commit.clone(),
            other.1.versions[0].commit.clone(),
        ]),
        changes: vec![
            Change {
                object: note.0.clone(),
                before: revisions(note.1),
                revision: fresh(),
                after: note.1.versions[0].state.clone(),
            },
            Change {
                object: other.0.clone(),
                before: revisions(other.1),
                revision: fresh(),
                after: other_state,
            },
        ],
        dependencies: vec![],
    };
    p.b.receive_sync(
        &p.identity,
        SyncBundle {
            manifests: BTreeMap::from([(manifest.id().unwrap(), manifest)]),
            blobs: BTreeMap::from([(hash(&bytes), bytes)]),
        },
    )
    .unwrap();
    let error = p.b.prepare_operation(apply(&p)).unwrap_err();
    assert!(
        (error.explanation)
            .eq(&UiMessage::error_core_sync_compound_resource_owner_identity_duplicate()),
        "{error:?}"
    );
    assert!(p.br.join("Note/Note.typ").exists());
    assert!(!p.br.join("New").exists());
    assert!(!p.br.join("Other/new.bin").exists());
}
