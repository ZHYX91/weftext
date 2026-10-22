//! Semantic decisions use the same original public author and replica writers.
use super::*;
use crate::ConfigRole;
use crate::backend::ParentGuard;

fn config(core: &mut Core, root: &Path, schema: &weftext_data::Schema) {
    let before = core.read_config(root, ConfigRole::Schema).unwrap();
    let preview = core
        .prepare_config_save(
            root,
            ConfigRole::Schema,
            before.bytes,
            serde_json::to_vec(schema).unwrap(),
            &fresh(),
        )
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
}

fn dependent_save(core: &mut Core, root: &Path, suffix: &str) -> String {
    let source_path = root.join("Note/Note.typ");
    let cut = core
        .read_snapshot(
            root,
            &[
                display(&source_path),
                display(&root.join(".weftext-schema.json")),
            ],
        )
        .unwrap();
    let mut source = cut.files[0].bytes.clone().unwrap();
    source.extend_from_slice(suffix.as_bytes());
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(root),
        intent: OperationIntent::UpdateFiles {
            changes: vec![FileChange {
                path: display(&source_path),
                base_id: Some(cut.files[0].base_id.clone()),
                before: cut.files[0].bytes.clone(),
                after: source.clone(),
            }],
        },
    };
    let preview = core
        .prepare_dependent_feature_operation(
            request,
            &serde_json::json!({"feature":"original-dependent-source"}),
            &[cut.files[1].base_id.clone()],
        )
        .unwrap();
    core.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    String::from_utf8(source).unwrap()
}

fn changed_schema_pair(conflict: bool) -> Pair {
    let mut p = pair(&["Note"]);
    let schema = weftext_data::Schema::bundled().unwrap();
    config(&mut p.a, &p.ar, &schema);
    transfer(
        &p.a,
        &p.ar,
        &p.b,
        &p.identity,
        &BTreeSet::from([ObjectKey::Schema]),
    );
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: BTreeSet::from([ObjectKey::Schema]),
        },
    );
    dependent_save(&mut p.a, &p.ar, "\nauthor semantic dependency");
    if conflict {
        dependent_save(&mut p.b, &p.br, "\nreceiver branch");
    }
    let mut changed = schema;
    changed
        .aliases
        .insert("additional-task".into(), "tasks/task".into());
    config(&mut p.a, &p.ar, &changed);
    let mut selected = p.keys.clone();
    selected.insert(ObjectKey::Schema);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    p.b.publish_sync(&p.br).unwrap();
    p
}

fn resolution(p: &Pair) -> SyncRevalidation {
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let view = &graph.objects().unwrap()[p.keys.first().unwrap()];
    let chosen = view
        .versions
        .iter()
        .find(|version| {
            graph
                .blob(&version.state.files["Note.typ"])
                .unwrap()
                .windows(b"author semantic dependency".len())
                .any(|bytes| bytes == b"author semantic dependency")
        })
        .unwrap_or(&view.versions[0]);
    SyncRevalidation {
        object: p.keys.first().unwrap().clone(),
        revisions: revisions(view),
        decision: SyncRevalidationDecision::Preserve {
            revision: chosen.revision.clone(),
        },
        state: chosen.state.clone(),
        reviewed_dependencies: vec![],
    }
}

fn typed(
    p: &mut Pair,
    resolution: SyncRevalidation,
    blobs: BTreeMap<String, Vec<u8>>,
) -> OperationRequest {
    let review =
        p.b.review_sync_revalidation(&p.br, &p.identity, &[resolution], &blobs)
            .unwrap();
    OperationRequest {
        operation_id: fresh(),
        root: display(&p.br),
        intent: OperationIntent::RevalidateSync {
            identity: p.identity.clone(),
            expected_installed: review.expected_installed,
            reviewed_versions: review.reviewed_versions,
            resolutions: review.resolutions,
            blobs,
        },
    }
}

#[test]
fn typed_preserve_changed_schema_uses_original_request_receipt_and_portable_revision_after_restart()
{
    let mut p = changed_schema_pair(false);
    assert_eq!(
        p.b.prepare_operation(apply(&p)).unwrap_err().code,
        "syncDependencyConflict"
    );
    let chosen = resolution(&p);
    let source_revision = chosen.state.file_revisions["Note.typ"].clone();
    let old = fs::read(p.br.join("Note/Note.typ")).unwrap();
    let old_stamp = crate::operations::capture_optional(&p.br.join("Note/Note.typ"))
        .unwrap()
        .unwrap()
        .stamp;
    let request = typed(&mut p, chosen, BTreeMap::new());
    let preview = p.b.prepare_operation(request.clone()).unwrap();
    assert_eq!(preview.kind, "revalidateSync");
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), old);
    assert!(!p.b.journal_path(&preview.operation_id, "install").exists());
    assert_eq!(
        p.b.prepare_operation(request.clone()).unwrap().plan_id,
        preview.plan_id
    );
    let mut changed_request = request.clone();
    changed_request.root.push('x');
    assert_eq!(
        p.b.prepare_operation(changed_request).unwrap_err().code,
        "operationConflict"
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        serde_json::to_value(p.b.sync_operation_request(&preview.operation_id).unwrap()).unwrap(),
        serde_json::to_value(&request).unwrap()
    );
    assert_eq!(
        p.b.prepare_operation(request.clone()).unwrap().plan_id,
        preview.plan_id
    );
    let receipt =
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
    assert_eq!(receipt.status, "committed");
    let current = crate::operations::capture_optional(&p.br.join("Note/Note.typ"))
        .unwrap()
        .unwrap();
    assert_eq!(current.stamp.identity, old_stamp.identity);
    assert_ne!(current.bytes, old);
    let observed =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Note/Note.typ"))])
            .unwrap();
    assert_eq!(
        observed.files[0].revision.as_deref(),
        Some(source_revision.as_str())
    );
    p.b.publish_sync(&p.br).unwrap();
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    assert!(graph.blob(&hash(&old)).is_some());
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(p.b.prepare_operation(request).unwrap().status, "committed");
    assert_eq!(
        serde_json::to_value(
            p.b.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
}

#[test]
fn typed_repair_uses_explicit_new_file_revision_and_cancelled_id_never_revives() {
    let mut p = changed_schema_pair(false);
    let chosen = resolution(&p);
    let cancelled = typed(&mut p, chosen.clone(), BTreeMap::new());
    let preview = p.b.prepare_operation(cancelled.clone()).unwrap();
    p.b.cancel_prepared_sync(&preview.operation_id, &preview.plan_id)
        .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(p.b.prepare_operation(cancelled).unwrap().status, "resolved");
    assert!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .is_err()
    );
    let mut repair = chosen;
    let base = match &repair.decision {
        SyncRevalidationDecision::Preserve { revision } => revision.clone(),
        _ => unreachable!(),
    };
    repair.decision = SyncRevalidationDecision::Repair {
        base_revision: base,
    };
    let graph = p.b.read_sync_history(&p.identity).unwrap();
    let mut source = graph
        .blob(&repair.state.files["Note.typ"])
        .unwrap()
        .to_vec();
    source.extend_from_slice(b"\nexplicit reviewed repair");
    let file_revision = fresh();
    repair.state.files.insert("Note.typ".into(), hash(&source));
    repair
        .state
        .file_revisions
        .insert("Note.typ".into(), file_revision.clone());
    let request = typed(
        &mut p,
        repair,
        BTreeMap::from([(hash(&source), source.clone())]),
    );
    let preview = p.b.prepare_operation(request).unwrap();
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), source);
    let observed =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Note/Note.typ"))])
            .unwrap();
    assert_eq!(
        observed.files[0].revision.as_deref(),
        Some(file_revision.as_str())
    );
}

#[test]
fn ordinary_merge_cannot_bypass_changed_schema_but_typed_complete_heads_can_preserve() {
    let mut p = changed_schema_pair(true);
    let chosen = resolution(&p);
    assert_eq!(chosen.revisions.len(), 2);
    let request = OperationRequest {
        operation_id: fresh(),
        root: display(&p.br),
        intent: OperationIntent::MergeSync {
            identity: p.identity.clone(),
            resolutions: vec![SyncResolution {
                object: chosen.object.clone(),
                revisions: chosen.revisions.clone(),
                state: chosen.state.clone(),
            }],
            blobs: BTreeMap::new(),
        },
    };
    let before = fs::read(p.br.join("Note/Note.typ")).unwrap();
    assert_eq!(
        p.b.prepare_operation(request.clone()).unwrap_err().code,
        "syncSemanticDecisionRequired"
    );
    assert!(!p.b.journal_path(&request.operation_id, "plan").exists());
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), before);
    let typed = typed(&mut p, chosen, BTreeMap::new());
    let preview = p.b.prepare_operation(typed).unwrap();
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    p.b.publish_sync(&p.br).unwrap();
    assert_eq!(
        p.b.read_sync_history(&p.identity)
            .unwrap()
            .objects()
            .unwrap()[p.keys.first().unwrap()]
        .versions
        .len(),
        1
    );
}

#[test]
fn pending_copy_negative_schema_prevents_two_move_compound_configuration_creation() {
    for recovery in [false, true] {
        let mut p = pair(&["Left", "Right", "CopySource"]);
        config(&mut p.a, &p.ar, &weftext_data::Schema::bundled().unwrap());
        for name in ["Left", "Right"] {
            run(
                &mut p.a,
                &p.ar,
                OperationIntent::RenameNode {
                    source: display(&p.ar.join(name)),
                    name: format!("New{name}"),
                },
            );
        }
        let mut selected = p.keys.clone();
        selected.remove(&key(&p, "CopySource"));
        selected.insert(ObjectKey::Schema);
        transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
        let original =
            p.b.prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&p.br),
                intent: OperationIntent::ApplySync {
                    identity: p.identity.clone(),
                    objects: selected,
                },
            })
            .unwrap();
        assert!(matches!(
            p.b.operations[&original.operation_id]
                .transaction
                .as_ref()
                .unwrap()
                .mutation,
            crate::operations::Mutation::Compound { .. }
        ));
        let pending =
            p.b.prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&p.br),
                intent: OperationIntent::CopyNode {
                    source: display(&p.br.join("CopySource")),
                    parent: display(&p.br),
                    name: "RetainedCopy".into(),
                    allow_losses: false,
                },
            })
            .unwrap();
        assert!(
            p.b.operations[&pending.operation_id]
                .transaction
                .as_ref()
                .unwrap()
                .absent_dependencies
                .iter()
                .any(|dependency| Path::new(&dependency.path).file_name()
                    == Some(std::ffi::OsStr::new(".weftext-schema.json")))
        );
        assert!(
            p.b.operations[&original.operation_id]
                .transaction
                .as_ref()
                .unwrap()
                .files
                .iter()
                .all(|file| !Path::new(&file.path).starts_with(p.br.join("CopySource")))
        );
        p.b.install_record(&p.b.operations[&pending.operation_id])
            .unwrap();
        if recovery {
            p.b.install_record(&p.b.operations[&original.operation_id])
                .unwrap();
            drop(p.b);
            p.b = Core::new(&p.bd).unwrap();
            assert!(
                p.b.inspect_recovery(&original.operation_id)
                    .unwrap()
                    .allowed_actions
                    .is_empty()
            );
            for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
                assert!(
                    p.b.resolve_recovery(&original.operation_id, action)
                        .is_err()
                );
            }
        } else {
            assert_eq!(
                p.b.execute_operation(&original.operation_id, &original.plan_id)
                    .unwrap_err()
                    .code,
                "recoveryPending"
            );
            assert!(!p.b.journal_path(&original.operation_id, "install").exists());
        }
        assert!(p.br.join("Left/Left.typ").exists());
        assert!(p.br.join("Right/Right.typ").exists());
        assert!(!p.br.join("NewLeft").exists());
        assert!(!p.br.join("NewRight").exists());
        assert!(!p.br.join("RetainedCopy").exists());
        assert!(!p.br.join(".weftext-schema.json").exists());
        assert!(!p.b.committed(&original.operation_id));
        assert!(!p.b.committed(&pending.operation_id));
    }
}

fn unknown_marker_payload_does_not_grant_directory_role(route: &str) {
    let temp = tempfile::tempdir().unwrap();
    let ar = temp.path().join("a-library");
    let br = temp.path().join("b-library");
    let bd = temp.path().join("b-private");
    let mut a = Core::new(temp.path().join("a-private")).unwrap();
    a.create_library(&ar).unwrap();
    let mut keys = BTreeSet::new();
    let mut initial = BTreeSet::new();
    for name in ["Note", "Left", "Right"] {
        let object = ObjectKey::Node(
            a.create_note_fixture(&ar, &ar, name)
                .unwrap()
                .node_id
                .unwrap(),
        );
        keys.insert(object.clone());
        if name != "Note" {
            initial.insert(object);
        }
    }
    let id = fresh();
    let bytes = serde_json::to_vec(&serde_json::json!({"directoryId":id})).unwrap();
    // Unknown authored bytes precede sync setup. A Node container does not
    // acquire another navigation role merely because this filename is present.
    fs::write(ar.join("Note/weftext.directory.json"), &bytes).unwrap();
    let identity = a.enable_sync(&ar, &fresh()).unwrap().identity;
    let graph = a.read_sync_history(&identity).unwrap();
    let objects = graph.objects().unwrap();
    assert!(objects.values().any(|view| {
        view.versions[0].state.name == "Note"
            && view.versions[0]
                .state
                .files
                .contains_key("weftext.directory.json")
    }));
    assert!(!objects.contains_key(&ObjectKey::Directory(id.clone())));
    let source_fi = ParentGuard::acquire(&ar.join("Note/.probe"))
        .unwrap()
        .directory_identity()
        .unwrap();
    assert!(
        !a.directory_admitted(&identity.workspace_id, &id, &source_fi)
            .unwrap()
    );
    if route == "source" {
        return;
    }
    let mut b = Core::new(&bd).unwrap();
    b.receive_sync(&identity, a.export_sync_objects(&identity, &keys).unwrap())
        .unwrap();
    let cloned = run(
        &mut b,
        &br,
        OperationIntent::CloneSync {
            identity: identity.clone(),
            objects: if route.starts_with("clone") {
                keys.clone()
            } else {
                initial
            },
        },
    );
    if route == "cloneRestart" || route == "cloneRecovery" {
        // The original Clone Plan already records the exact Node role and
        // materialization. The fix changes only its admission consumers, so no
        // simulated writer or altered role metadata is needed for an old Plan.
        let request = b.sync_operation_request(&cloned.operation_id).unwrap();
        let count = b.operations.len();
        if route == "cloneRecovery" {
            fs::rename(
                b.journal_path(&cloned.operation_id, "commit"),
                temp.path().join("original-clone-decision.json"),
            )
            .unwrap();
        }
        drop(b);
        b = Core::new(&bd).unwrap();
        if route == "cloneRecovery" {
            assert_eq!(
                b.inspect_recovery(&cloned.operation_id).unwrap().coherence,
                "knownAfter"
            );
            b.resolve_recovery(&cloned.operation_id, "adoptAfterImages")
                .unwrap();
        }
        assert_eq!(b.operations.len(), count);
        assert_eq!(
            b.operation_details(&cloned.operation_id).unwrap().status,
            "committed"
        );
        assert_eq!(
            serde_json::to_value(b.sync_operation_request(&cloned.operation_id).unwrap()).unwrap(),
            serde_json::to_value(request).unwrap()
        );
    }
    if !route.starts_with("clone") {
        assert!(!br.join("Note").exists());
        if route == "compound" {
            for name in ["Left", "Right"] {
                run(
                    &mut a,
                    &ar,
                    OperationIntent::RenameNode {
                        source: display(&ar.join(name)),
                        name: format!("Moved{name}"),
                    },
                );
            }
        }
        transfer(&a, &ar, &b, &identity, &keys);
        let applied = run(
            &mut b,
            &br,
            OperationIntent::ApplySync {
                identity: identity.clone(),
                objects: keys.clone(),
            },
        );
        if route == "compound" {
            assert!(matches!(
                b.operations[&applied.operation_id]
                    .transaction
                    .as_ref()
                    .unwrap()
                    .mutation,
                crate::operations::Mutation::Compound { .. }
            ));
        }
    }
    let directory = br.join("Note");
    assert_eq!(
        fs::read(directory.join("weftext.directory.json")).unwrap(),
        bytes
    );
    let physical = ParentGuard::acquire(&directory.join(".probe"))
        .unwrap()
        .directory_identity()
        .unwrap();
    assert!(
        !b.directory_admitted(&identity.workspace_id, &id, &physical)
            .unwrap(),
        "{route}"
    );
    assert!(
        b.directory_admission_candidates(&identity.workspace_id)
            .unwrap()
            .iter()
            .all(|(_, candidate, _)| candidate != &id),
        "{route}"
    );
    let reference = format!("weftext-directory://{}/{id}", identity.workspace_id);
    assert_ne!(
        b.resolve_navigation_target(&br, &reference).unwrap().status,
        "managed",
        "{route}"
    );
}

#[test]
fn node_unknown_marker_payload_remains_node_content_at_original_sync_setup() {
    unknown_marker_payload_does_not_grant_directory_role("source");
}
#[test]
fn node_unknown_marker_payload_is_not_directory_after_received_apply() {
    unknown_marker_payload_does_not_grant_directory_role("apply");
}
#[test]
fn node_unknown_marker_payload_is_not_directory_after_received_clone() {
    unknown_marker_payload_does_not_grant_directory_role("clone");
}
#[test]
fn node_unknown_marker_payload_is_not_directory_after_received_compound() {
    unknown_marker_payload_does_not_grant_directory_role("compound");
}

#[test]
fn legacy_committed_clone_node_marker_does_not_gain_directory_role_after_restart() {
    unknown_marker_payload_does_not_grant_directory_role("cloneRestart");
}
#[test]
fn legacy_known_after_clone_node_marker_does_not_gain_directory_role_after_original_recovery() {
    unknown_marker_payload_does_not_grant_directory_role("cloneRecovery");
}

#[test]
fn typed_negative_schema_read_refuses_original_install_only_writer_before_effect_and_recovery() {
    let mut p = pair(&["Note"]);
    let chosen = resolution(&p);
    let request = typed(&mut p, chosen, BTreeMap::new());
    let original = p.b.prepare_operation(request).unwrap();
    let source = fs::read(p.br.join("Note/Note.typ")).unwrap();
    let pending =
        p.b.prepare_config_save(
            &p.br,
            ConfigRole::Schema,
            None,
            serde_json::to_vec(&weftext_data::Schema::bundled().unwrap()).unwrap(),
            &fresh(),
        )
        .unwrap();
    p.b.install_record(&p.b.operations[&pending.operation_id])
        .unwrap();
    assert_eq!(
        p.b.execute_operation(&original.operation_id, &original.plan_id)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert!(!p.b.journal_path(&original.operation_id, "install").exists());
    p.b.install_record(&p.b.operations[&original.operation_id])
        .unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert!(
        p.b.inspect_recovery(&original.operation_id)
            .unwrap()
            .allowed_actions
            .is_empty()
    );
    for action in ["keepCurrent", "resumeOriginalPlan", "adoptAfterImages"] {
        assert!(
            p.b.resolve_recovery(&original.operation_id, action)
                .is_err()
        );
    }
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), source);
    assert!(!p.br.join(".weftext-schema.json").exists());
    assert!(!p.b.committed(&original.operation_id));
    assert!(!p.b.committed(&pending.operation_id));
}

fn save_definition(
    core: &mut Core,
    root: &Path,
    name: &str,
    plan: &weftext_data::QueryPlan,
    context: &weftext_data::FrozenAdvancedInput,
) -> (String, String, String) {
    let path = root.join(name).join(format!("{name}.typ"));
    let document = core.open(&path, Some(root.into())).unwrap();
    let basis = core
        .read_snapshot(root, &[display(&path)])
        .unwrap()
        .files
        .remove(0);
    let operation = fresh();
    let key = core
        .reserve_definition_key(root, &path, &basis.base_id, &operation)
        .unwrap();
    let schema = core.read_config(root, ConfigRole::Schema).unwrap();
    let binding = weftext_data::SavedQueryBinding {
        schema: schema.role_ref,
        revision: schema.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let source = weftext_data::upsert_saved_query_with_context(
        &document.source,
        None,
        &key,
        "weftext-query-original",
        &binding,
        plan,
        &weftext_data::Schema::bundled().unwrap(),
        context,
    )
    .unwrap()
    .source;
    core.save(SaveRequest {
        session_id: document.session_id,
        base_id: document.base_id,
        operation_id: operation,
        source: source.clone(),
    })
    .unwrap();
    let revision = core.read_snapshot(root, &[display(&path)]).unwrap().files[0]
        .revision
        .clone()
        .unwrap();
    (key, revision, source)
}

fn chosen_definition(core: &Core, identity: &SyncIdentity, object: &ObjectKey) -> SyncRevalidation {
    let graph = core.read_sync_history(identity).unwrap();
    let objects = graph.objects().unwrap();
    let view = &objects[object];
    assert_eq!(view.versions.len(), 1);
    SyncRevalidation {
        object: object.clone(),
        revisions: revisions(view),
        decision: SyncRevalidationDecision::Preserve {
            revision: view.versions[0].revision.clone(),
        },
        state: view.versions[0].state.clone(),
        reviewed_dependencies: vec![],
    }
}

#[test]
fn typed_committed_current_definition_keeps_exact_native_key_admission_after_restart() {
    let mut p = pair(&["Note"]);
    config(&mut p.a, &p.ar, &weftext_data::Schema::bundled().unwrap());
    let plan = weftext_data::QueryPlan {
        select: vec![weftext_data::QueryColumn {
            name: "value".into(),
            expr: "1".into(),
        }],
        ..Default::default()
    };
    let (definition, revision, source) = save_definition(
        &mut p.a,
        &p.ar,
        "Note",
        &plan,
        &weftext_data::FrozenAdvancedInput::default(),
    );
    let mut selected = p.keys.clone();
    selected.insert(ObjectKey::Schema);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    let node = p.keys.first().unwrap().clone();
    let ObjectKey::Node(owner) = &node else {
        unreachable!()
    };
    assert!(
        !p.b.is_definition_key_admitted(&p.br, owner, &definition)
            .unwrap()
    );
    let chosen = chosen_definition(&p.b, &p.identity, &node);
    let request = typed(&mut p, chosen, BTreeMap::new());
    let preview = p.b.prepare_operation(request).unwrap();
    assert!(
        !p.b.is_definition_key_admitted(&p.br, owner, &definition)
            .unwrap()
    );
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        fs::read_to_string(p.br.join("Note/Note.typ")).unwrap(),
        source
    );
    assert!(
        p.b.is_definition_key_admitted(&p.br, owner, &definition)
            .unwrap()
    );
    assert!(
        !p.b.is_definition_key_admitted(&p.br, owner, &fresh())
            .unwrap()
    );
    let observed =
        p.b.read_snapshot(&p.br, &[display(&p.br.join("Note/Note.typ"))])
            .unwrap();
    assert_eq!(
        observed.files[0].revision.as_deref(),
        Some(revision.as_str())
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert!(
        p.b.is_definition_key_admitted(&p.br, owner, &definition)
            .unwrap()
    );
}

fn lifecycle_query_pair(mixed: bool, bad: bool) -> (Pair, OperationRequest, ObjectKey, ObjectKey) {
    let mut p = pair(&["Note", "Live", "Archived"]);
    config(&mut p.a, &p.ar, &weftext_data::Schema::bundled().unwrap());
    transfer(
        &p.a,
        &p.ar,
        &p.b,
        &p.identity,
        &BTreeSet::from([ObjectKey::Schema]),
    );
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: BTreeSet::from([ObjectKey::Schema]),
        },
    );
    for name in ["Live", "Archived"] {
        let path = p.ar.join(name).join(format!("{name}.typ"));
        let document = p.a.open(&path, Some(p.ar.clone())).unwrap();
        let id = weftext_engine::inspect_metadata(&document.source)
            .id
            .unwrap();
        let status = if name == "Archived" && bad {
            "done"
        } else {
            "open"
        };
        p.a.save(SaveRequest {
            session_id: document.session_id,
            base_id: document.base_id,
            operation_id: fresh(),
            source: format!(
                "#metadata((id: \"{id}\", types: (\"task\",), status: \"{status}\")) <weftext>\n"
            ),
        })
        .unwrap();
    }
    transfer(&p.a, &p.ar, &p.b, &p.identity, &p.keys);
    let apply_request = apply(&p);
    let applied = p.b.prepare_operation(apply_request).unwrap();
    p.b.execute_operation(&applied.operation_id, &applied.plan_id)
        .unwrap();
    let archived = key(&p, "Archived");
    run(
        &mut p.a,
        &p.ar,
        OperationIntent::TrashNode {
            source: display(&p.ar.join("Archived")),
        },
    );
    transfer(
        &p.a,
        &p.ar,
        &p.b,
        &p.identity,
        &BTreeSet::from([archived.clone()]),
    );
    run(
        &mut p.b,
        &p.br,
        OperationIntent::ApplySync {
            identity: p.identity.clone(),
            objects: BTreeSet::from([archived.clone()]),
        },
    );
    let leaf = |lifecycle: &str| weftext_data::QueryPlan {
        scope: weftext_data::QueryScope {
            lifecycle: lifecycle.into(),
            ..Default::default()
        },
        types: vec!["task".into()],
        select: vec![weftext_data::QueryColumn {
            name: "actual".into(),
            expr: "nextCivilDay(status == 'done' ? date('9999-12-31') : date('2026-01-01'))".into(),
        }],
        ..Default::default()
    };
    let plan = if mixed {
        weftext_data::QueryPlan {
            advanced: Some(weftext_data::QueryAdvanced {
                union_all: Some(vec![leaf("live"), leaf("trash")]),
                ..Default::default()
            }),
            ..Default::default()
        }
    } else {
        leaf("trash")
    };
    save_definition(
        &mut p.a,
        &p.ar,
        "Note",
        &plan,
        &weftext_data::FrozenAdvancedInput::default(),
    );
    let note = key(&p, "Note");
    transfer(
        &p.a,
        &p.ar,
        &p.b,
        &p.identity,
        &BTreeSet::from([note.clone()]),
    );
    let chosen = chosen_definition(&p.b, &p.identity, &note);
    let request = typed(&mut p, chosen, BTreeMap::new());
    (p, request, note, archived)
}

#[test]
fn typed_trash_and_mixed_queries_execute_original_actual_rows_and_freeze_both_lifecycles() {
    for mixed in [false, true] {
        let (mut p, request, _, archived) = lifecycle_query_pair(mixed, false);
        let before = fs::read(p.br.join("Note/Note.typ")).unwrap();
        let preview = p.b.prepare_operation(request.clone()).unwrap();
        let transaction = p.b.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        let ObjectKey::Node(archived_id) = archived else {
            unreachable!()
        };
        assert_eq!(
            transaction.expected_trash_membership,
            Some(BTreeSet::from([archived_id]))
        );
        assert_eq!(transaction.expected_membership.is_some(), mixed);
        assert!(
            !transaction
                .expected_membership
                .as_ref()
                .is_some_and(|members| members.is_empty())
        );
        assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), before);
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        drop(p.b);
        p.b = Core::new(&p.bd).unwrap();
        assert_eq!(p.b.prepare_operation(request).unwrap().status, "committed");
        let snapshot = p.b.trash_admitted_snapshot(&p.br).unwrap();
        assert!(snapshot.complete);
        assert_eq!(snapshot.files.len(), 1);
        // A real registered read-only Trash cut is still no generic writer.
        let error =
            p.b.prepare_admitted_operation(
                OperationRequest {
                    operation_id: fresh(),
                    root: display(&p.br),
                    intent: OperationIntent::NewNode {
                        parent: display(&p.br),
                        name: "Unauthorized".into(),
                        source: None,
                    },
                },
                &snapshot.cut_id,
            )
            .unwrap_err();
        assert_eq!(error.code, "readOnly");
        assert!(!p.br.join("Unauthorized").exists());
    }
}

#[test]
fn typed_trash_and_mixed_queries_cannot_replace_real_failed_rows_with_empty_input() {
    for mixed in [false, true] {
        let (mut p, request, _, _) = lifecycle_query_pair(mixed, true);
        let before = fs::read(p.br.join("Note/Note.typ")).unwrap();
        let operation = request.operation_id.clone();
        assert_eq!(
            p.b.prepare_operation(request).unwrap_err().code,
            "queryInvocationUnavailable"
        );
        assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), before);
        assert!(!p.b.operations.contains_key(&operation));
    }
}

#[test]
fn typed_trash_membership_is_rechecked_before_effect_and_after_restart() {
    let (mut p, request, _, _) = lifecycle_query_pair(false, false);
    let preview = p.b.prepare_operation(request).unwrap();
    let trash = p.b.trash_entries(&p.br).unwrap().remove(0);
    run(
        &mut p.b,
        &p.br,
        OperationIntent::RestoreNode {
            trash_operation_id: trash.operation_id,
            node_id: None,
            parent: display(&p.br),
            name: "Restored".into(),
        },
    );
    let before = fs::read(p.br.join("Note/Note.typ")).unwrap();
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert_eq!(
        p.b.validate_recovery_scope(
            p.b.operations[&preview.operation_id]
                .transaction
                .as_ref()
                .unwrap()
        )
        .unwrap_err()
        .code,
        "scopeChanged"
    );
    assert_eq!(
        p.b.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap_err()
            .code,
        "syncBasisChanged"
    );
    assert_eq!(fs::read(p.br.join("Note/Note.typ")).unwrap(), before);
    assert!(!p.b.committed(&preview.operation_id));
}

#[test]
fn typed_committed_pinned_definition_uses_exact_historical_source_without_inventing_current_slot() {
    let mut p = pair(&["Note"]);
    let original = fs::read_to_string(p.ar.join("Note/Note.typ")).unwrap();
    config(&mut p.a, &p.ar, &weftext_data::Schema::bundled().unwrap());
    let callee_plan = weftext_data::QueryPlan {
        select: vec![weftext_data::QueryColumn {
            name: "value".into(),
            expr: "1".into(),
        }],
        ..Default::default()
    };
    let (key, old_revision, old_source) = save_definition(
        &mut p.a,
        &p.ar,
        "Note",
        &callee_plan,
        &weftext_data::FrozenAdvancedInput::default(),
    );
    let doc =
        p.a.open(p.ar.join("Note/Note.typ"), Some(p.ar.clone()))
            .unwrap();
    p.a.save(SaveRequest {
        session_id: doc.session_id,
        base_id: doc.base_id,
        operation_id: fresh(),
        source: original.clone(),
    })
    .unwrap();
    let new_revision =
        p.a.read_snapshot(&p.ar, &[display(&p.ar.join("Note/Note.typ"))])
            .unwrap()
            .files[0]
            .revision
            .clone()
            .unwrap();
    let caller = ObjectKey::Node(
        p.a.create_note_fixture(&p.ar, &p.ar, "Caller")
            .unwrap()
            .node_id
            .unwrap(),
    );
    let callee = p.keys.first().unwrap().clone();
    let ObjectKey::Node(owner) = &callee else {
        unreachable!()
    };
    let owner_ref = format!("weftext-node://{}/{owner}", p.identity.workspace_id);
    let schema = p.a.read_config(&p.ar, ConfigRole::Schema).unwrap();
    let context = weftext_data::FrozenAdvancedInput {
        definitions: vec![weftext_data::FrozenDefinitionOwner {
            owner: owner_ref.clone(),
            history: vec![
                weftext_data::DefinitionRevision {
                    revision: old_revision.clone(),
                    current: false,
                    source: old_source.clone(),
                },
                weftext_data::DefinitionRevision {
                    revision: new_revision,
                    current: true,
                    source: original.clone(),
                },
            ],
        }],
        schemas: vec![weftext_data::FrozenDefinitionSchema {
            schema: schema.role_ref,
            revision: schema.revision.unwrap(),
            projection: weftext_data::Schema::bundled().unwrap(),
        }],
        ..Default::default()
    };
    let caller_plan = weftext_data::QueryPlan {
        select: vec![weftext_data::QueryColumn {
            name: "value".into(),
            expr: "value".into(),
        }],
        advanced: Some(weftext_data::QueryAdvanced {
            call: Some(weftext_data::QueryCall {
                owner: owner_ref,
                selection: weftext_data::DefinitionSelection::Pinned {
                    key: key.clone(),
                    revision: old_revision.clone(),
                },
                arguments: BTreeMap::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let (caller_key, _, _) = save_definition(&mut p.a, &p.ar, "Caller", &caller_plan, &context);
    let selected = BTreeSet::from([callee.clone(), caller.clone(), ObjectKey::Schema]);
    transfer(&p.a, &p.ar, &p.b, &p.identity, &selected);
    let chosen = chosen_definition(&p.b, &p.identity, &caller);
    let request = typed(&mut p, chosen, BTreeMap::new());
    let preview = p.b.prepare_operation(request).unwrap();
    let evidence = p.b.operations[&preview.operation_id]
        .sync_application
        .as_ref()
        .unwrap()
        .semantic_evidence
        .as_ref()
        .unwrap();
    assert!(
        evidence
            .historical_inputs
            .iter()
            .any(|input| input["fileRevision"] == old_revision)
    );
    p.b.execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    assert_eq!(
        fs::read_to_string(p.br.join("Note/Note.typ")).unwrap(),
        original
    );
    assert!(weftext_data::saved_queries(&original).unwrap().is_empty());
    assert!(p.b.is_definition_key_admitted(&p.br, owner, &key).unwrap());
    let ObjectKey::Node(caller_owner) = &caller else {
        unreachable!()
    };
    assert!(
        p.b.is_definition_key_admitted(&p.br, caller_owner, &caller_key)
            .unwrap()
    );
    drop(p.b);
    p.b = Core::new(&p.bd).unwrap();
    assert!(p.b.is_definition_key_admitted(&p.br, owner, &key).unwrap());
}
