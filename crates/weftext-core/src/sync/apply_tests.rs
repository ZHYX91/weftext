use super::*;
use crate::{PathBuf, SaveRequest};
use std::fs;

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
        .map(|n| ObjectKey::Node(a.create_note(&ar, &ar, n).unwrap().node_id.unwrap()))
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
