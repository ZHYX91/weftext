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
    let note = core.create_note_fixture(&root, &root, "Note").unwrap();
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
            removed_members: cut.removed_members,
        },
    }
}
fn raw(f: &Fixture, text: &str) -> String {
    let source = format!("{}\n{text}", f.source);
    fs::write(&f.path, &source).unwrap();
    source
}

#[test]
fn sync_adoption_static_opaque_author_members_keep_exact_source_without_compile() {
    let mut f = fixture();
    let ObjectKey::Node(id) = &f.node else {
        unreachable!()
    };
    let external = format!(
        "#metadata((id: \"{id}\", types: (\"task\",), status: \"open\", opaque: (possible_ref: \"weftext-node://{}/{}\"), opaque_code: preserved_expression())) <weftext>\n#unavailable_display_only()",
        fresh(),
        fresh(),
    );
    fs::write(&f.path, &external).unwrap();
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    assert!(
        f.core.operations[&preview.operation_id]
            .sync_application
            .as_ref()
            .unwrap()
            .semantic_evidence
            .is_none()
    );
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
}

fn saved_definition_fixture() -> (Fixture, String) {
    let mut f = fixture();
    let schema = weftext_data::Schema::bundled().unwrap();
    let current = f
        .core
        .read_config(&f.root, crate::ConfigRole::Schema)
        .unwrap();
    let preview = f
        .core
        .prepare_config_save(
            &f.root,
            crate::ConfigRole::Schema,
            current.bytes,
            serde_json::to_vec(&schema).unwrap(),
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    f.objects.insert(ObjectKey::Schema);
    let document = f.core.open(&f.path, Some(f.root.clone())).unwrap();
    let basis = f
        .core
        .read_snapshot(&f.root, &[display(&f.path)])
        .unwrap()
        .files
        .remove(0);
    let operation = fresh();
    let key = f
        .core
        .reserve_definition_key(&f.root, &f.path, &basis.base_id, &operation)
        .unwrap();
    let current = f
        .core
        .read_config(&f.root, crate::ConfigRole::Schema)
        .unwrap();
    let binding = weftext_data::SavedQueryBinding {
        schema: current.role_ref,
        revision: current.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let plan = weftext_data::QueryPlan {
        select: vec![weftext_data::QueryColumn {
            name: "value".into(),
            expr: "1".into(),
        }],
        ..Default::default()
    };
    let source = weftext_data::upsert_saved_query_with_context(
        &document.source,
        None,
        &key,
        "weftext-query-adoption",
        &binding,
        &plan,
        &schema,
        &weftext_data::FrozenAdvancedInput::default(),
    )
    .unwrap()
    .source;
    f.core
        .save(SaveRequest {
            session_id: document.session_id,
            base_id: document.base_id,
            source: source.clone(),
            operation_id: operation,
        })
        .unwrap();
    f.source = source;
    f.core.publish_sync(&f.root).unwrap();
    (f, key)
}

#[test]
fn sync_adoption_existing_definition_external_body_publishes_and_transfers_exact_source() {
    let (mut f, key) = saved_definition_fixture();
    let original = capture_optional(&f.path).unwrap().unwrap();
    let original_revision = f.core.accepted_revision(&original).unwrap();
    // Ordinary body compilation is deliberately unavailable. This is not a
    // reason to reject the unchanged, already-admitted static Query control.
    let external = raw(&f, "#unavailable_display_only()\nexternal body edit");
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input.clone()).unwrap();
    assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
    assert!(
        !f.core
            .journal_path(&preview.operation_id, "install")
            .exists()
    );
    assert!(
        f.core.operations[&preview.operation_id]
            .sync_application
            .as_ref()
            .unwrap()
            .semantic_evidence
            .is_some()
    );
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core.prepare_operation(input.clone()).unwrap().plan_id,
        preview.plan_id
    );
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let adopted = capture_optional(&f.path).unwrap().unwrap();
    assert_eq!(adopted.stamp.identity, original.stamp.identity);
    assert_eq!(adopted.bytes, external.as_bytes());
    let revision = f.core.accepted_revision(&adopted).unwrap();
    assert_ne!(revision, original_revision);
    let ObjectKey::Node(owner) = &f.node else {
        unreachable!()
    };
    assert!(
        f.core
            .is_definition_key_admitted(&f.root, owner, &key)
            .unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            f.core
                .sync_operation_request(&preview.operation_id)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(input).unwrap()
    );
    f.core.publish_sync(&f.root).unwrap();
    let mut receiver = Core::new(f.temp.path().join("receiver-private")).unwrap();
    receiver
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &f.objects).unwrap(),
        )
        .unwrap();
    let replica = f.temp.path().join("receiver-library");
    let clone = receiver
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&replica),
            intent: OperationIntent::CloneSync {
                identity: f.identity.clone(),
                objects: f.objects,
            },
        })
        .unwrap();
    receiver
        .execute_operation(&clone.operation_id, &clone.plan_id)
        .unwrap();
    let received = replica.join("Note/Note.typ");
    assert_eq!(fs::read(&received).unwrap(), external.as_bytes());
    let snapshot = receiver
        .read_snapshot(&replica, &[display(&received)])
        .unwrap();
    assert_eq!(
        snapshot.files[0].revision.as_deref(),
        Some(revision.as_str())
    );
}

#[test]
fn sync_adoption_definition_uses_installed_cut_and_preserves_uninstalled_remote_branch() {
    let (mut f, _) = saved_definition_fixture();
    let installed = f
        .core
        .sync_binding_info(&f.root)
        .unwrap()
        .installed_versions;
    let prior = BTreeSet::from([installed
        .iter()
        .find(|(object, _)| object == &f.node)
        .unwrap()
        .1
        .clone()]);
    let mut remote = Core::new(f.temp.path().join("remote-private")).unwrap();
    remote
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &f.objects).unwrap(),
        )
        .unwrap();
    let root = f.temp.path().join("remote-library");
    let clone = remote
        .prepare_operation(OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::CloneSync {
                identity: f.identity.clone(),
                objects: f.objects.clone(),
            },
        })
        .unwrap();
    remote
        .execute_operation(&clone.operation_id, &clone.plan_id)
        .unwrap();
    let path = root.join("Note/Note.typ");
    let opened = remote.open(&path, Some(root.clone())).unwrap();
    let remote_source = format!("{}\nreceived remote branch", opened.source);
    remote
        .save(SaveRequest {
            session_id: opened.session_id,
            base_id: opened.base_id,
            source: remote_source.clone(),
            operation_id: fresh(),
        })
        .unwrap();
    remote.publish_sync(&root).unwrap();
    f.core
        .receive_sync(
            &f.identity,
            remote.export_sync_objects(&f.identity, &f.objects).unwrap(),
        )
        .unwrap();
    let external = raw(
        &f,
        "local observed body from original installed predecessor",
    );
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    let manifest = &f.core.operations[&preview.operation_id]
        .sync_publication
        .as_ref()
        .unwrap()
        .manifest;
    assert_eq!(
        manifest
            .changes
            .iter()
            .find(|c| c.object == f.node)
            .unwrap()
            .before,
        prior
    );
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    f.core.publish_sync(&f.root).unwrap();
    let graph = f.core.read_sync_history(&f.identity).unwrap();
    let objects = graph.objects().unwrap();
    let versions = &objects[&f.node].versions;
    assert_eq!(versions.len(), 2);
    let sources: BTreeSet<_> = versions
        .iter()
        .map(|v| graph.blob(&v.state.files["Note.typ"]).unwrap().to_vec())
        .collect();
    assert_eq!(
        sources,
        BTreeSet::from([external.into_bytes(), remote_source.into_bytes()])
    );
}

#[test]
fn sync_adoption_definition_rejects_invalid_foreign_and_unadmitted_controls_without_staging() {
    let (mut f, key) = saved_definition_fixture();
    let definition = weftext_data::saved_queries(&f.source).unwrap().remove(0);
    let schema = weftext_data::Schema::bundled().unwrap();
    let original_operations = f.core.operations.len();
    for (foreign, expected) in [(false, "invalidQuery"), (true, "schemaUnavailable")] {
        let mut plan = definition.plan.clone();
        let mut binding = definition.binding.clone();
        if foreign {
            binding.schema = format!("weftext-config://{}/schema", fresh());
        } else {
            plan.select[0].expr = "1 +".into();
        }
        // This is deliberately raw external input. The replay formatter only
        // serializes it; adoption must apply the actual semantic validator.
        let external = weftext_data::upsert_saved_query_replay(
            &f.source,
            Some(&key),
            &key,
            &definition.label,
            &binding,
            &plan,
            &schema,
        )
        .unwrap()
        .source;
        fs::write(&f.path, &external).unwrap();
        let input = request(&mut f);
        assert_eq!(f.core.prepare_operation(input).unwrap_err().code, expected);
        assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
        assert_eq!(f.core.operations.len(), original_operations);
    }
    let forged = fresh();
    let external = f.source.replace(&key, &forged);
    assert_ne!(external, f.source);
    fs::write(&f.path, &external).unwrap();
    let input = request(&mut f);
    assert_eq!(
        f.core.prepare_operation(input).unwrap_err().code,
        "syncEvidenceRequired"
    );
    assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
    assert_eq!(f.core.operations.len(), original_operations);
    let ObjectKey::Node(owner) = &f.node else {
        unreachable!()
    };
    assert!(
        !f.core
            .is_definition_key_admitted(&f.root, owner, &forged)
            .unwrap()
    );
}

#[test]
fn sync_adoption_definition_stale_observation_and_pending_keep_original_records() {
    let (mut f, _) = saved_definition_fixture();
    raw(&f, "first external body");
    let stale = request(&mut f);
    let external = raw(&f, "later external body");
    assert_eq!(
        f.core.prepare_operation(stale).unwrap_err().code,
        "staleBase"
    );
    let first_request = request(&mut f);
    let first = f.core.prepare_operation(first_request.clone()).unwrap();
    let second_request = request(&mut f);
    let second = f.core.prepare_operation(second_request).unwrap();
    // An original Core install checkpoint owns this exact observed source.
    let pending = f.core.operations[&second.operation_id].clone();
    f.core.install_record(&pending).unwrap();
    let first_record = fs::read(f.core.journal_path(&first.operation_id, "plan")).unwrap();
    let second_record = fs::read(f.core.journal_path(&second.operation_id, "plan")).unwrap();
    assert_eq!(
        f.core
            .execute_operation(&first.operation_id, &first.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(!f.core.journal_path(&first.operation_id, "install").exists());
    assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
    assert_eq!(
        fs::read(f.core.journal_path(&first.operation_id, "plan")).unwrap(),
        first_record
    );
    assert_eq!(
        fs::read(f.core.journal_path(&second.operation_id, "plan")).unwrap(),
        second_record
    );
    drop(f.core);
    f.core = Core::new(&f.data).unwrap();
    assert_eq!(
        f.core.prepare_operation(first_request).unwrap().plan_id,
        first.plan_id
    );
    assert_eq!(
        f.core
            .execute_operation(&first.operation_id, &first.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(!f.core.committed(&first.operation_id));
    assert!(!f.core.committed(&second.operation_id));
    assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
}

#[test]
fn sync_adoption_external_citation_uses_exact_selected_work_publication() {
    let mut f = fixture();
    let work = f
        .core
        .create_note_fixture(&f.root, &f.root, "Work")
        .unwrap();
    let work_id = work.node_id.unwrap();
    let work_path = PathBuf::from(work.path);
    let publication = format!(
        "#metadata((id: \"{work_id}\", types: (\"paper\",), publication: (kind: \"article\", title: \"Reviewed Publication\", contributors: ((agent: (text: \"Author\"), roles: (\"author\",)),)))) <weftext>\n#unavailable_work_body()"
    );
    let opened = f.core.open(&work_path, Some(f.root.clone())).unwrap();
    f.core
        .save(SaveRequest {
            session_id: opened.session_id,
            base_id: opened.base_id,
            source: publication,
            operation_id: fresh(),
        })
        .unwrap();
    f.core.publish_sync(&f.root).unwrap();
    let external = raw(
        &f,
        &format!(
            "#metadata((kind: \"weftext.citations\", entries: ((key: \"work\", work: \"weftext-node://{}/{work_id}\"),))) <weftext-citations>\n#unavailable_document_body()",
            f.identity.workspace_id
        ),
    );
    let missing_dependency = request(&mut f);
    assert_eq!(
        f.core
            .prepare_operation(missing_dependency)
            .unwrap_err()
            .code,
        "syncDependencyRequired"
    );
    f.objects.insert(ObjectKey::Node(work_id));
    let input = request(&mut f);
    let preview = f.core.prepare_operation(input).unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(fs::read(&f.path).unwrap(), external.as_bytes());
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
fn sync_adoption_missing_citation_dependency_preserves_unresolved_ordinary_links() {
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
        "syncDependencyRequired"
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
    // LC01 creation now records Rules as an accepted physical observation.
    // This legacy fixture must omit that fact as well as setup bindings; the
    // unchanged portable revision alone must never recreate physical evidence.
    let creation = f
        .core
        .operations
        .values()
        .find(|p| p.kind == "createLibrary")
        .unwrap();
    let observations = f.core.journal_path(&creation.operation_id, "observations");
    let mut facts: Vec<crate::operations::RevisionFact> =
        serde_json::from_slice(&fs::read(&observations).unwrap()).unwrap();
    let original_count = facts.len();
    facts.retain(|fact| Path::new(&fact.path) != f.root.join(".weftext-rules"));
    assert_eq!(original_count - facts.len(), 1);
    fs::write(&observations, serde_json::to_vec(&facts).unwrap()).unwrap();
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
