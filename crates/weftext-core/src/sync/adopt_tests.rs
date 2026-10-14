use super::*;
use crate::{PathBuf, SaveRequest};
use std::fs;

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    core: Core,
    identity: SyncIdentity,
    objects: BTreeSet<ObjectKey>,
    node: ObjectKey,
    path: PathBuf,
    source: String,
}
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let data = temp.path().join("private");
    let mut core = Core::new(&data).unwrap();
    core.create_library(&root).unwrap();
    let note = core.create_note(&root, &root, "Note").unwrap();
    let node = ObjectKey::Node(note.node_id.unwrap());
    let resource = core
        .prepare_import_resource(
            &root,
            &note.path,
            "asset.bin",
            b"original resource".to_vec(),
            &fresh(),
        )
        .unwrap();
    core.execute_operation(&resource.operation_id, &resource.plan_id)
        .unwrap();
    let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
    Fixture {
        temp,
        root,
        data,
        core,
        identity,
        objects: BTreeSet::from([node.clone(), ObjectKey::Rules]),
        node,
        path: note.path.into(),
        source: note.source,
    }
}
fn request(f: &mut Fixture) -> OperationRequest {
    let cut = f.core.read_sync_adoption_cut(&f.root, &f.objects).unwrap();
    OperationRequest {
        operation_id: fresh(),
        root: display(&f.root),
        intent: OperationIntent::AdoptSyncExternal {
            identity: cut.identity,
            expected_installed: cut.expected_installed,
            changes: cut.changes,
        },
    }
}
fn raw(f: &Fixture, text: &str) -> String {
    let source = format!("{}\n{text}", f.source);
    fs::write(&f.path, &source).unwrap();
    source
}

#[test]
fn sync_adoption_observed_content_new_versions_restart_retry_and_all_drafts_retained() {
    let mut f = fixture();
    let before = f
        .core
        .sync_binding_info(&f.root)
        .unwrap()
        .installed_versions;
    let doc = f.core.open(&f.path, Some(f.root.clone())).unwrap();
    f.core
        .persist_draft(&doc.session_id, "local original dirty buffer")
        .unwrap();
    f.core
        .with_actor("other", |c| {
            let doc = c.open(&f.path, Some(f.root.clone()))?;
            c.persist_draft(&doc.session_id, "other actor private draft")
        })
        .unwrap();
    let external = raw(&f, "external observed content");
    assert_eq!(
        f.core
            .with_actor("ordinary", |c| {
                let fresh_doc = c.open(&f.path, Some(f.root.clone()))?;
                c.save(SaveRequest {
                    session_id: fresh_doc.session_id,
                    base_id: fresh_doc.base_id,
                    source: format!("{external}\nordinary edit"),
                    operation_id: fresh(),
                })
            })
            .unwrap_err()
            .code,
        "syncExternalChange"
    );
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input.clone()).unwrap();
    assert_eq!(fs::read_to_string(&f.path).unwrap(), external);
    assert_eq!(
        f.core.prepare_operation(input.clone()).unwrap().plan_id,
        preview.plan_id
    );
    let plan = f.core.operations[&preview.operation_id].clone();
    assert_eq!(
        plan.sync_publication
            .as_ref()
            .unwrap()
            .manifest
            .changes
            .len(),
        1
    );
    assert_eq!(
        plan.sync_publication.as_ref().unwrap().manifest.changes[0].before,
        BTreeSet::from([before
            .iter()
            .find(|(key, _)| key == &f.node)
            .unwrap()
            .1
            .clone()])
    );
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core
            .sync_operation_request(&preview.operation_id)
            .unwrap()
            .operation_id,
        input.operation_id
    );
    assert_eq!(
        f.core.prepare_operation(input).unwrap().plan_id,
        preview.plan_id
    );
    let receipt = f
        .core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .operation_id,
        receipt.operation_id
    );
    let after = f
        .core
        .sync_binding_info(&f.root)
        .unwrap()
        .installed_versions;
    assert_ne!(before, after);
    assert_eq!(
        f.core.read_draft(&f.path).unwrap().unwrap().source,
        "local original dirty buffer"
    );
    assert_eq!(
        f.core
            .with_actor("ordinary", |c| c.read_draft(&f.path))
            .unwrap()
            .unwrap()
            .source,
        format!("{external}\nordinary edit")
    );
    assert_eq!(
        f.core
            .with_actor("other", |c| c.read_draft(&f.path))
            .unwrap()
            .unwrap()
            .source,
        "other actor private draft"
    );
    f.core.publish_sync(&f.root).unwrap();
    let graph = f.core.read_sync_history(&f.identity).unwrap();
    assert!(graph.blob(&hash(f.source.as_bytes())).is_some());
    assert!(graph.blob(&hash(external.as_bytes())).is_some());
    assert_eq!(
        f.core
            .accepted_revision(&capture_optional(&f.path).unwrap().unwrap()),
        Some(preview.revisions[&display(&f.path)].clone())
    );
    f.core
        .with_actor("fresh-after-adoption", |c| {
            let opened = c.open(&f.path, Some(f.root.clone()))?;
            c.save(SaveRequest {
                session_id: opened.session_id,
                base_id: opened.base_id,
                source: format!("{external}\nafter explicit adoption"),
                operation_id: fresh(),
            })
        })
        .unwrap();
}

#[test]
fn sync_adoption_cut_no_change_cleans_bases_same_bytes_external_write_is_new_fact() {
    let mut f = fixture();
    let original_count = f.core.snapshots.len();
    for _ in 0..3 {
        assert_eq!(
            f.core
                .read_sync_adoption_cut(&f.root, &f.objects)
                .unwrap_err()
                .code,
            "noChange"
        );
    }
    assert_eq!(f.core.snapshots.len(), original_count);
    let before = f
        .core
        .accepted_revision(&capture_optional(&f.path).unwrap().unwrap())
        .unwrap();
    fs::write(&f.path, &f.source).unwrap();
    let input = request(&mut f);
    let changes = match &input.intent {
        OperationIntent::AdoptSyncExternal { changes, .. } => changes,
        _ => unreachable!(),
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(f.core.snapshots.len(), original_count + 1); // only changed cut basis
    let preview = f.core.prepare_operation(input).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_ne!(before, preview.revisions[&display(&f.path)]);
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.source);
}

#[test]
fn sync_samebytes_external_new_base_save_requires_adoption_then_core_noop_remains_valid() {
    let mut f = fixture();
    let original = capture_optional(&f.path).unwrap().unwrap().stamp;
    fs::write(&f.path, &f.source).unwrap();
    assert_ne!(capture_optional(&f.path).unwrap().unwrap().stamp, original);
    let forbidden = format!("{}\nnew editor proposal", f.source);
    assert_eq!(
        f.core
            .with_actor("fresh-external-base", |core| {
                let opened = core.open(&f.path, Some(f.root.clone()))?;
                core.save(SaveRequest {
                    session_id: opened.session_id,
                    base_id: opened.base_id,
                    source: forbidden.clone(),
                    operation_id: fresh(),
                })
            })
            .unwrap_err()
            .code,
        "syncExternalChange"
    );
    assert_eq!(fs::read_to_string(&f.path).unwrap(), f.source);
    assert_eq!(
        f.core
            .with_actor("fresh-external-base", |core| core.read_draft(&f.path))
            .unwrap()
            .unwrap()
            .source,
        forbidden
    );
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let adopted = preview.revisions[&display(&f.path)].clone();
    // A genuine no-op Core save can update its physical observation while the
    // portable content revision remains unchanged. The next write uses that
    // latest durable observation, not a stale same-revision timestamp.
    f.core
        .with_actor("after-adoption", |core| {
            let opened = core.open(&f.path, Some(f.root.clone()))?;
            core.save(SaveRequest {
                session_id: opened.session_id,
                base_id: opened.base_id,
                source: opened.source,
                operation_id: fresh(),
            })
        })
        .unwrap();
    assert_eq!(
        f.core
            .accepted_revision(&capture_optional(&f.path).unwrap().unwrap()),
        Some(adopted)
    );
    f.core
        .with_actor("after-real-noop", |core| {
            let opened = core.open(&f.path, Some(f.root.clone()))?;
            core.save(SaveRequest {
                session_id: opened.session_id,
                base_id: opened.base_id,
                source: format!("{}\nafter real Core observation", opened.source),
                operation_id: fresh(),
            })
        })
        .unwrap();
}

#[test]
fn sync_adoption_rechecks_observed_bytes_physical_object_and_all_unselected_members() {
    let mut f = fixture();
    raw(&f, "reviewed external source");
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input.clone()).unwrap();
    raw(&f, "later source");
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "conflict"
    );
    f.core
        .cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    let asset = f.root.join("Note/asset.bin");
    fs::write(&asset, b"unreviewed resource third state").unwrap();
    assert!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    assert_eq!(
        fs::read(&asset).unwrap(),
        b"unreviewed resource third state"
    );
    f.core
        .cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
        .unwrap();
    fs::write(&asset, b"original resource").unwrap();
    let bytes = fs::read(&f.path).unwrap();
    fs::rename(&f.path, f.temp.path().join("original-object.typ")).unwrap();
    fs::write(&f.path, bytes).unwrap();
    let count = f.core.snapshots.len();
    assert_eq!(
        f.core
            .read_sync_adoption_cut(&f.root, &f.objects)
            .unwrap_err()
            .code,
        "identityConflict"
    );
    assert_eq!(f.core.snapshots.len(), count);
}

#[test]
fn sync_adoption_installed_cut_changed_or_cancelled_original_never_reexecutes() {
    let mut f = fixture();
    raw(&f, "external proposal");
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input.clone()).unwrap();
    let rules = f
        .core
        .read_config(&f.root, crate::ConfigRole::Rules)
        .unwrap();
    let changed = br#"{"version":1,"ignored":["raw.txt"],"unmanaged":[]}"#.to_vec();
    let p = f
        .core
        .prepare_config_save(
            &f.root,
            crate::ConfigRole::Rules,
            rules.bytes,
            changed,
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&p.operation_id, &p.plan_id)
        .unwrap();
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "syncBasisChanged"
    );
    assert_eq!(
        f.core
            .cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .status,
        "resolved"
    );
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(f.core.prepare_operation(input).unwrap().status, "resolved");
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(!f.core.committed(&preview.operation_id));
}

#[test]
fn sync_adoption_original_afterimage_recovery_preserves_unknown_effect_fence() {
    let mut f = fixture();
    raw(&f, "external original outcome");
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    fs::rename(
        f.core.journal_path(&preview.operation_id, "commit"),
        f.temp.path().join("retained-original-commit.json"),
    )
    .unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core
            .cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert_eq!(
        f.core
            .resolve_recovery(&preview.operation_id, "adoptAfterImages")
            .unwrap()
            .status,
        "committed"
    );
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap()
            .status,
        "committed"
    );
    assert!(
        f.core
            .publish_sync(&f.root)
            .unwrap()
            .commits
            .iter()
            .all(|c| c.admission == Admission::Accepted)
    );
}

#[test]
fn sync_adoption_invalid_typed_rules_and_uninstalled_or_late_schema_fail_closed() {
    let mut f = fixture();
    let id = match &f.node {
        ObjectKey::Node(id) => id,
        _ => unreachable!(),
    };
    let invalid = format!(
        "#metadata((id: \"{id}\", types: (\"task\",), dependencies: (\"not-a-ref\",))) <weftext>\n= Invalid\n"
    );
    fs::write(&f.path, invalid).unwrap();
    let input = request(&mut f);
    assert_eq!(
        f.core.prepare_operation(input).unwrap_err().code,
        "invalidTypedSource"
    );
    raw(&f, "external valid source");
    fs::write(
        f.root.join(".weftext-rules"),
        br#"{"version":1,"ignored":[],"unmanaged":[],"extra":true}"#,
    )
    .unwrap();
    let input = request(&mut f);
    assert_eq!(
        f.core.prepare_operation(input).unwrap_err().code,
        "unsupportedRules"
    );
    fs::write(
        f.root.join(".weftext-rules"),
        br#"{"version":1,"ignored":[],"unmanaged":[]}"#,
    )
    .unwrap();
    let schema_path = f.root.join(".weftext-schema.json");
    fs::write(&schema_path, b"{}").unwrap();
    let input = request(&mut f);
    assert_eq!(
        f.core.prepare_operation(input).unwrap_err().code,
        "syncDependencyRequired"
    );
    fs::rename(
        &schema_path,
        f.temp.path().join("retained-uninstalled-schema.json"),
    )
    .unwrap();
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    fs::write(&schema_path, b"{}").unwrap();
    assert_eq!(
        f.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "scopeChanged"
    );
    assert!(!f.core.committed(&preview.operation_id));
}

#[test]
fn sync_adoption_semantic_citation_gate_preserves_unresolved_ordinary_links() {
    let mut f = fixture();
    let reference = format!("weftext-node://{}/{}", f.identity.workspace_id, fresh());
    raw(
        &f,
        &format!(
            "#metadata((kind: \"weftext.citations\", entries: ((key: \"work\", work: \"{reference}\"),))) <weftext-citations>"
        ),
    );
    let input = request(&mut f);
    assert_eq!(
        f.core.prepare_operation(input).unwrap_err().code,
        "syncSemanticDecisionRequired"
    );
    raw(
        &f,
        &format!("#link(\"{reference}\")[unresolved ordinary link]"),
    );
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

#[test]
fn sync_adoption_legacy_without_original_physical_fact_never_fabricates_binding() {
    let mut f = fixture();
    raw(&f, "external current");
    let enable = f
        .core
        .operations
        .values()
        .find(|p| p.kind == "enableSync")
        .unwrap()
        .operation_id
        .clone();
    let mut plan = f.core.operations[&enable].clone();
    plan.sync_setup.as_mut().unwrap().physical_bindings.clear();
    // Controlled old-plan fixture: omit this new optional private field. The
    // immutable portable manifests and commit fingerprint are not rewritten.
    let mut value = serde_json::to_value(&plan).unwrap();
    value["syncSetup"]
        .as_object_mut()
        .unwrap()
        .remove("physicalBindings");
    fs::write(
        f.core.journal_path(&enable, "plan"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core
            .read_sync_adoption_cut(&f.root, &f.objects)
            .unwrap_err()
            .code,
        "identityUnavailable"
    );
    assert!(
        fs::read_to_string(&f.path)
            .unwrap()
            .contains("external current")
    );
}

#[test]
fn sync_received_history_page_exact_wrapper_budget_and_owner_bound_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::new(temp.path().join("private")).unwrap();
    let object = ObjectKey::Node(fresh());
    let workspace = fresh();
    let part = "x".repeat(201);
    let mut state = ObjectState {
        parent: None,
        name: "Note".into(),
        retired: false,
        files: BTreeMap::new(),
        file_revisions: BTreeMap::new(),
    };
    let blob = hash(b"exact");
    for i in 0..1000 {
        let path = format!("{part}/{part}/{part}/{part}/{part}/{i}.bin");
        state.files.insert(path.clone(), blob.clone());
        state.file_revisions.insert(path, fresh());
    }
    let first = Manifest {
        version: 1,
        workspace_id: workspace.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::new(),
        dependencies: vec![],
        changes: vec![Change {
            object: object.clone(),
            before: BTreeSet::new(),
            revision: fresh(),
            after: state.clone(),
        }],
    };
    let identity = SyncIdentity {
        workspace_id: workspace,
        genesis: first.id().unwrap(),
    };
    let second = Manifest {
        version: 1,
        workspace_id: identity.workspace_id.clone(),
        replica_id: fresh(),
        operation_id: fresh(),
        parents: BTreeSet::from([identity.genesis.clone()]),
        dependencies: vec![],
        changes: vec![Change {
            object: object.clone(),
            before: BTreeSet::from([first.changes[0].revision.clone()]),
            revision: fresh(),
            after: state,
        }],
    };
    core.receive_sync(
        &identity,
        SyncBundle {
            manifests: BTreeMap::from([
                (identity.genesis.clone(), first),
                (second.id().unwrap(), second),
            ]),
            blobs: BTreeMap::from([(blob, b"exact".to_vec())]),
        },
    )
    .unwrap();
    let page = core
        .received_sync_versions(&identity, &object, None, 100)
        .unwrap();
    assert_eq!(page.versions.len(), 1);
    assert_eq!(page.coverage, "complete");
    assert!(serde_json::to_vec(&page).unwrap().len() <= MAX_MANIFEST_BYTES);
    let next = core
        .received_sync_versions(&identity, &object, page.next_commit.as_deref(), 100)
        .unwrap();
    assert_eq!(next.versions.len(), 1);
    assert!(next.next_commit.is_none());
    assert_ne!(page.versions[0].revision, next.versions[0].revision);
    assert_eq!(
        core.received_sync_versions(
            &identity,
            &ObjectKey::Node(fresh()),
            page.next_commit.as_deref(),
            1
        )
        .unwrap_err()
        .code,
        "syncHistoryMissing"
    );
    assert!(
        core.received_sync_versions(&identity, &object, Some("invalid"), 1)
            .is_err()
    );
}
