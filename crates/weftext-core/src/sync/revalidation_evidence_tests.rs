use super::*;
use crate::{PathBuf, display, fresh};

struct Fixture {
    _temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    source: PathBuf,
    owner: NodeRef,
    graph: ReplicaGraph,
    local: BTreeMap<ObjectKey, LocalObject>,
    chosen: BTreeMap<ObjectKey, Version>,
    physical: Vec<SyncPhysicalBinding>,
    snapshots: Vec<SnapshotWitness>,
}
impl Fixture {
    fn new() -> Self {
        Self::build(false)
    }
    fn build(with_resource: bool) -> Self {
        Self::build_variant(with_resource, false)
    }
    fn build_variant(with_resource: bool, missing_target: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let source = PathBuf::from(note.path);
        let owner = NodeRef {
            workspace_id: core.require_workspace(&root).unwrap().workspace_id,
            node_id: note.node_id.unwrap(),
        };
        if with_resource || missing_target {
            let target = if missing_target {
                serde_json::json!({"kind":"wholeDocument","node":{"workspaceId":owner.workspace_id,"nodeId":fresh()}})
            } else {
                let preview = core
                    .prepare_import_resource(
                        &root,
                        &source,
                        "owned.bin",
                        b"owned original".to_vec(),
                        &fresh(),
                    )
                    .unwrap();
                core.execute_operation(&preview.operation_id, &preview.plan_id)
                    .unwrap();
                let reference = preview.resource_map.values().next().unwrap();
                let qualified = core
                    .qualified_resource_at(
                        &root,
                        &source,
                        &owner,
                        reference,
                        &root.join("Note/owned.bin"),
                        "live",
                        None,
                    )
                    .unwrap();
                serde_json::json!({"kind": "wholeResource", "resource": reference, "revisionId": qualified.revision.as_ref().unwrap()})
            };
            let annotations = serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "annotations": [{
                    "annotationId": fresh(), "replyTo": null, "attribution": "author", "createdAt": "2026-10-15T00:00:00Z",
                    "purpose": "comment", "body": "owned exact target", "labels": [], "appearance": {}, "reviewState": "open", "suggestion": null,
                    "target": target
                }]
            })).unwrap();
            assert_eq!(
                weftext_knowledge::decode_annotations(&annotations)
                    .unwrap()
                    .records
                    .len(),
                1
            );
            let annotation_path = root.join("Note/weftext.annotations.json");
            let basis = core
                .read_snapshot(&root, &[display(&annotation_path)])
                .unwrap()
                .files
                .remove(0);
            assert!(basis.bytes.is_none());
            let preview = core
                .prepare_operation(crate::OperationRequest {
                    operation_id: fresh(),
                    root: display(&root),
                    intent: crate::OperationIntent::UpdateFiles {
                        changes: vec![crate::operations::FileChange {
                            path: display(&annotation_path),
                            base_id: Some(basis.base_id.clone()),
                            before: basis.bytes,
                            after: annotations,
                        }],
                    },
                })
                .unwrap();
            drop(core.take_snapshot_witness(&basis.base_id).unwrap());
            core.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
        }
        let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
        let graph = core.read_sync_history(&identity).unwrap();
        let setup = core.sync_setup_for(&root).unwrap().clone();
        let local = core.sync_local_state(&setup).unwrap().0;
        let chosen = graph
            .objects()
            .unwrap()
            .into_iter()
            .map(|(object, view)| {
                assert_eq!(view.versions.len(), 1);
                (object, view.versions[0].clone())
            })
            .collect();
        let mut paths = local
            .values()
            .flat_map(|object| {
                object
                    .state
                    .files
                    .keys()
                    .map(|relative| display(&Path::new(&object.directory).join(relative)))
            })
            .collect::<Vec<_>>();
        paths.push(display(&root.join(".weftext-schema.json")));
        let snapshot = core.read_snapshot(&root, &paths).unwrap();
        let snapshots = snapshot
            .files
            .into_iter()
            .map(|file| core.take_snapshot_witness(&file.base_id).unwrap())
            .collect();
        Self {
            _temp: temp,
            core,
            root,
            source,
            owner,
            graph,
            local,
            chosen,
            physical: setup.physical_bindings,
            snapshots,
        }
    }
    fn checked(&self) -> CheckedRevalidation {
        let object = ObjectKey::Node(self.owner.node_id.clone());
        let current = self.graph.objects().unwrap();
        let dependencies = [ObjectKey::Schema, ObjectKey::Rules]
            .into_iter()
            .filter_map(|key| {
                current.get(&key).map(|view| Dependency {
                    object: key,
                    revisions: view
                        .versions
                        .iter()
                        .map(|version| version.revision.clone())
                        .collect(),
                })
            })
            .collect();
        super::super::revalidation::validate_revalidation(
            &self.graph,
            &self.chosen,
            &self.local,
            &[super::super::revalidation::SyncRevalidation {
                object: object.clone(),
                revisions: current[&object]
                    .versions
                    .iter()
                    .map(|version| version.revision.clone())
                    .collect(),
                decision: super::super::revalidation::SyncRevalidationDecision::Preserve {
                    revision: self.chosen[&object].revision.clone(),
                },
                state: self.chosen[&object].state.clone(),
                reviewed_dependencies: dependencies,
            }],
        )
        .unwrap()
    }
    fn qualify(&self) -> QualifiedQueryNode {
        self.core
            .qualified_query_node_at(&self.root, &self.source, &self.owner, "live")
            .unwrap()
    }
}

#[test]
fn registered_resource_and_whole_annotation_use_original_owner_file_revision() {
    let f = Fixture::build(true);
    let node = f.qualify();
    let rules = f.root.join(".weftext-rules");
    let objects = BTreeMap::from([
        (
            ObjectKey::Node(f.owner.node_id.clone()),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&node),
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let state = &f.chosen[&ObjectKey::Node(f.owner.node_id.clone())].state;
    let manifest: serde_json::Value = serde_json::from_slice(
        f.graph
            .blob(&state.files["weftext.resources.json"])
            .unwrap(),
    )
    .unwrap();
    let reference = format!(
        "weftext-resource://{}/{}",
        f.owner.workspace_id,
        manifest["resources"][0]["resourceId"].as_str().unwrap()
    );
    let qualified = f
        .core
        .qualified_resource_at(
            &f.root,
            &f.source,
            &f.owner,
            &reference,
            &f.root.join("Note/owned.bin"),
            "live",
            None,
        )
        .unwrap();
    let resources = [&qualified];
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &resources,
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    let checked = f.checked();
    assert!(
        checked
            .required_evidence
            .iter()
            .any(|item| matches!(item, RequiredSemanticEvidence::Resource { .. }))
    );
    assert!(
        checked
            .required_evidence
            .iter()
            .any(|item| matches!(item, RequiredSemanticEvidence::AnnotationTargets { .. }))
    );
    let reads = capture_evidence(&f.core, &cut, &checked)
        .unwrap()
        .into_read_set()
        .unwrap();
    assert!(reads.witnesses.iter().any(|witness| {
        witness
            .binding
            .captured
            .as_ref()
            .is_some_and(|file| file.bytes == b"owned original")
    }));
}

#[test]
fn original_node_and_negative_schema_return_real_read_bindings() {
    let f = Fixture::new();
    // Keep this case on the original setup fallback: LC01 adds a separate
    // accepted Rules observation, which older producers did not record.
    // Preserve setup physical bindings and the exact portable genesis.
    let creation = f
        .core
        .operations
        .values()
        .find(|p| p.kind == "createLibrary")
        .unwrap();
    let observations = f.core.journal_path(&creation.operation_id, "observations");
    let mut facts: Vec<crate::operations::RevisionFact> =
        serde_json::from_slice(&std::fs::read(&observations).unwrap()).unwrap();
    let original_count = facts.len();
    facts.retain(|fact| !same_locator(Path::new(&fact.path), &f.root.join(".weftext-rules")));
    assert_eq!(original_count - facts.len(), 1);
    std::fs::write(&observations, serde_json::to_vec(&facts).unwrap()).unwrap();
    let original_rules = f
        .snapshots
        .iter()
        .find(|witness| {
            same_locator(
                Path::new(&witness.binding.path),
                &f.root.join(".weftext-rules"),
            )
        })
        .unwrap()
        .binding
        .captured
        .as_ref()
        .unwrap();
    // This producer has no separate accepted Rules observation. Its portable
    // revision is an actual immutable enable_sync genesis binding, not None.
    assert!(f.core.accepted_revision(original_rules).unwrap().is_none());
    assert!(
        f.local[&ObjectKey::Rules]
            .state
            .file_revisions
            .contains_key(".weftext-rules")
    );
    let node = f.qualify();
    let rules = f.root.join(".weftext-rules");
    let objects = BTreeMap::from([
        (
            ObjectKey::Node(f.owner.node_id.clone()),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&node),
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    let checked = f.checked();
    let captured = capture_evidence(&f.core, &cut, &checked).unwrap();
    assert!(captured.unfulfilled().is_empty());
    let reads = captured.into_read_set().unwrap();
    assert_eq!(reads.discharged.len(), checked.required_evidence.len());
    assert!(
        reads
            .read_objects
            .contains(&ObjectKey::Node(f.owner.node_id.clone()))
    );
    assert!(reads.witnesses.iter().any(|witness| {
        same_locator(
            Path::new(&witness.binding.path),
            &f.root.join(".weftext-schema.json"),
        ) && witness.binding.captured.is_none()
    }));
    let mut forged = f.local.clone();
    forged
        .get_mut(&ObjectKey::Rules)
        .unwrap()
        .state
        .file_revisions
        .insert(".weftext-rules".into(), fresh());
    let forged_cut = SuppliedEvidenceCut {
        installed: &forged,
        ..cut
    };
    assert_eq!(
        capture_evidence(&f.core, &forged_cut, &checked)
            .err()
            .unwrap()
            .code,
        "syncEvidenceChanged"
    );
}

#[test]
fn missing_annotation_target_preserves_discussion_and_reports_unavailable_without_target_read() {
    let f = Fixture::build_variant(false, true);
    let node = f.qualify();
    let rules = f.root.join(".weftext-rules");
    let object = ObjectKey::Node(f.owner.node_id.clone());
    let objects = BTreeMap::from([
        (
            object.clone(),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&node),
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let checked = f.checked();
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    let original = std::fs::read(f.root.join("Note/weftext.annotations.json")).unwrap();
    let reads = capture_evidence(&f.core, &cut, &checked)
        .unwrap()
        .into_read_set()
        .unwrap();
    assert_eq!(reads.annotation_targets.len(), 1);
    assert_eq!(reads.annotation_targets[0].state, "unavailable");
    assert_eq!(
        reads.annotation_targets[0].reason,
        "preserved-target-not-in-authorized-read-cut"
    );
    assert_eq!(
        reads.read_objects,
        BTreeSet::from([object, ObjectKey::Rules])
    );
    assert_eq!(
        std::fs::read(f.root.join("Note/weftext.annotations.json")).unwrap(),
        original
    );
}

#[test]
fn public_node_descriptor_cannot_rebind_another_opaque_qualification() {
    let mut f = Fixture::new();
    let other = f
        .core
        .create_note_fixture(&f.root, &f.root, "Other")
        .unwrap();
    let other_owner = NodeRef {
        workspace_id: f.owner.workspace_id.clone(),
        node_id: other.node_id.unwrap(),
    };
    let mut wrong = f
        .core
        .qualified_query_node_at(&f.root, Path::new(&other.path), &other_owner, "live")
        .unwrap();
    // Public descriptor fields do not change its original private witness.
    wrong.owner = f.owner.clone();
    wrong.source_hash = f.qualify().source_hash;
    let rules = f.root.join(".weftext-rules");
    let objects = BTreeMap::from([
        (
            ObjectKey::Node(f.owner.node_id.clone()),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&wrong),
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    assert_eq!(
        capture_evidence(&f.core, &cut, &f.checked())
            .err()
            .unwrap()
            .code,
        "syncEvidenceChanged"
    );
}

#[test]
fn original_negative_schema_is_not_recaptured_after_appearance() {
    let f = Fixture::new();
    let checked = f.checked();
    let node = f.qualify();
    let rules = f.root.join(".weftext-rules");
    let objects = BTreeMap::from([
        (
            ObjectKey::Node(f.owner.node_id.clone()),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&node),
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    std::fs::write(f.root.join(".weftext-schema.json"), b"{}").unwrap();
    assert!(capture_evidence(&f.core, &cut, &checked).is_err());
}

#[test]
fn missing_invocation_values_and_key_admission_cannot_finish_a_read_set() {
    let f = Fixture::new();
    let node = f.qualify();
    let rules = f.root.join(".weftext-rules");
    let objects = BTreeMap::from([
        (
            ObjectKey::Node(f.owner.node_id.clone()),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&node),
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    let mut checked = f.checked();
    let source_revision = f.chosen[&ObjectKey::Node(f.owner.node_id.clone())]
        .state
        .file_revisions["Note.typ"]
        .clone();
    let definition_key = fresh();
    let invocation_key = fresh();
    checked.required_evidence.extend([
        RequiredSemanticEvidence::Definition {
            owner: f.owner.clone(),
            key: definition_key.clone(),
            source_revision: source_revision.clone(),
        },
        RequiredSemanticEvidence::QueryInvocation {
            owner: f.owner.clone(),
            key: invocation_key.clone(),
            source_revision,
        },
    ]);
    let captured = capture_evidence(&f.core, &cut, &checked).unwrap();
    assert!(captured.unfulfilled().iter().any(|item| matches!(
        item.gap,
        EvidenceGap::DefinitionAdmission
    ) && matches!(&item.obligation, RequiredSemanticEvidence::Definition { owner, key, .. }
                    if owner == &f.owner && key == &definition_key)));
    assert!(captured.unfulfilled().iter().any(|item| matches!(
        item.gap,
        EvidenceGap::ExactInvocationInputs
    ) && matches!(&item.obligation, RequiredSemanticEvidence::QueryInvocation { owner, key, .. }
                    if owner == &f.owner && key == &invocation_key)));
    assert_eq!(
        captured.into_read_set().err().unwrap().code,
        "syncEvidenceRequired"
    );
}

#[test]
fn registered_directory_requires_its_original_marker_scope_and_physical_cut() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Library");
    let directory = root.join("Container");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    core.create_library(&root).unwrap();
    std::fs::create_dir(&directory).unwrap();
    let admitted = core
        .prepare_directory_admission(&root, &directory, &fresh())
        .unwrap();
    core.execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
        .unwrap();
    let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
    let graph = core.read_sync_history(&identity).unwrap();
    let setup = core.sync_setup_for(&root).unwrap().clone();
    let local = core.sync_local_state(&setup).unwrap().0;
    let chosen = graph
        .objects()
        .unwrap()
        .into_iter()
        .map(|(key, view)| {
            assert_eq!(view.versions.len(), 1);
            (key, view.versions[0].clone())
        })
        .collect::<BTreeMap<_, _>>();
    let (_, _, id) = crate::directory_identity::parse_ref(&admitted.target_ref).unwrap();
    let object = ObjectKey::Directory(id.into());
    let version = &chosen[&object];
    let checked = super::super::revalidation::validate_revalidation(
        &graph,
        &chosen,
        &local,
        &[super::super::revalidation::SyncRevalidation {
            object: object.clone(),
            revisions: BTreeSet::from([version.revision.clone()]),
            decision: super::super::revalidation::SyncRevalidationDecision::Preserve {
                revision: version.revision.clone(),
            },
            state: version.state.clone(),
            reviewed_dependencies: vec![],
        }],
    )
    .unwrap();
    let scope = core
        .qualified_query_scope_at(&root, &directory, &admitted.target_ref, "live")
        .unwrap();
    let marker = directory.join("weftext.directory.json");
    let objects = BTreeMap::from([(
        object.clone(),
        SuppliedObject {
            path: &marker,
            qualification: ObjectQualification::Directory(&scope),
        },
    )]);
    let paths = local
        .values()
        .flat_map(|entry| {
            entry
                .state
                .files
                .keys()
                .map(|relative| display(&Path::new(&entry.directory).join(relative)))
        })
        .collect::<Vec<_>>();
    let snapshots = core
        .read_snapshot(&root, &paths)
        .unwrap()
        .files
        .into_iter()
        .map(|file| core.take_snapshot_witness(&file.base_id).unwrap())
        .collect::<Vec<_>>();
    let cut = SuppliedEvidenceCut {
        root: &root,
        graph: &graph,
        installed: &local,
        chosen: &chosen,
        physical: &setup.physical_bindings,
        snapshots: &snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[],
        after_locations: &BTreeMap::new(),
        planned: None,
    };
    let reads = capture_evidence(&core, &cut, &checked)
        .unwrap()
        .into_read_set()
        .unwrap();
    assert!(reads.read_objects.contains(&object));
    let mut forged = setup.physical_bindings.clone();
    let identity = &mut forged
        .iter_mut()
        .find(|entry| entry.object == object && entry.relative_path == "weftext.directory.json")
        .unwrap()
        .stamp
        .identity;
    match identity {
        crate::FileIdentity::Windows { index, .. } => *index ^= 1,
        crate::FileIdentity::Linux { handle, .. } => handle[0] ^= 1,
    }
    let bad_cut = SuppliedEvidenceCut {
        physical: &forged,
        ..cut
    };
    assert_eq!(
        capture_evidence(&core, &bad_cut, &checked)
            .err()
            .unwrap()
            .code,
        "syncEvidenceChanged"
    );
}

#[test]
fn actual_row_provider_uses_original_admitted_cut_and_records_runtime_dependencies() {
    let mut f = Fixture::new();
    let schema = weftext_data::Schema::bundled().unwrap();
    let preview = f
        .core
        .prepare_config_save(
            &f.root,
            crate::ConfigRole::Schema,
            None,
            serde_json::to_vec(&schema).unwrap(),
            &fresh(),
        )
        .unwrap();
    f.core
        .execute_operation(&preview.operation_id, &preview.plan_id)
        .unwrap();
    let config = f
        .core
        .read_config(&f.root, crate::ConfigRole::Schema)
        .unwrap();
    let definition_operation = fresh();
    let basis = f
        .core
        .read_snapshot(&f.root, &[display(&f.source)])
        .unwrap()
        .files
        .remove(0);
    let key = f
        .core
        .reserve_definition_key(&f.root, &f.source, &basis.base_id, &definition_operation)
        .unwrap();
    drop(f.core.take_snapshot_witness(&basis.base_id).unwrap());
    let binding = weftext_data::SavedQueryBinding {
        schema: config.role_ref,
        revision: config.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let plan = weftext_data::QueryPlan {
        select: vec![
            weftext_data::QueryColumn {
                name: "next".into(),
                expr: "nextCivilDay(status == 'done' ? date('9999-12-31') : date('2026-01-01'))"
                    .into(),
            },
            weftext_data::QueryColumn {
                name: "unknown".into(),
                expr: "spare".into(),
            },
        ],
        advanced: Some(
            serde_json::from_value(serde_json::json!({
                "parameters":[{"name":"spare","shape":{"kind":"scalar","semantic":"int"}}]
            }))
            .unwrap(),
        ),
        ..Default::default()
    };
    let source = format!(
        "#metadata((id: \"{}\", types: (\"task\",), status: \"open\")) <weftext>\n",
        f.owner.node_id
    );
    let after = weftext_data::upsert_saved_query(
        &source,
        None,
        &key,
        "weftext-query-runtime",
        &binding,
        &plan,
        &schema,
    )
    .unwrap()
    .source;
    let doc = f.core.open(&f.source, Some(f.root.clone())).unwrap();
    f.core
        .save(crate::SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            operation_id: definition_operation,
            source: after,
        })
        .unwrap();
    assert!(
        f.core
            .is_definition_key_admitted(&f.root, &f.owner.node_id, &key)
            .unwrap()
    );
    f.core.publish_sync(&f.root).unwrap();
    let setup = f.core.sync_setup_for(&f.root).unwrap().clone();
    f.graph = f.core.read_sync_history(&setup.identity).unwrap();
    f.local = f.core.sync_local_state(&setup).unwrap().0;
    f.chosen = f
        .graph
        .objects()
        .unwrap()
        .into_iter()
        .map(|(key, view)| {
            assert_eq!(view.versions.len(), 1);
            (key, view.versions[0].clone())
        })
        .collect();
    let installed_stamps = f
        .core
        .sync_installed_stamps(&f.local, &f.local.keys().cloned().collect(), None)
        .unwrap();
    f.physical = f
        .local
        .iter()
        .flat_map(|(object, local)| {
            local
                .state
                .file_revisions
                .iter()
                .map(|(relative, revision)| SyncPhysicalBinding {
                    object: object.clone(),
                    relative_path: relative.clone(),
                    stamp: installed_stamps[revision].clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let query = f.core.admitted_snapshot(&f.root).unwrap();
    let query_witnesses = query
        .files
        .iter()
        .chain(query.schema.iter())
        .map(|file| f.core.take_snapshot_witness(&file.base_id).unwrap())
        .collect::<Vec<_>>();
    let paths = f
        .local
        .values()
        .flat_map(|local| {
            local
                .state
                .files
                .keys()
                .map(|relative| display(&Path::new(&local.directory).join(relative)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    f.snapshots = f
        .core
        .read_snapshot(&f.root, &paths)
        .unwrap()
        .files
        .into_iter()
        .map(|file| f.core.take_snapshot_witness(&file.base_id).unwrap())
        .collect();
    Core::merge_feature_witnesses(&mut f.snapshots, query_witnesses).unwrap();
    let node = f.qualify();
    let object = ObjectKey::Node(f.owner.node_id.clone());
    let schema_path = f.root.join(".weftext-schema.json");
    let rules_path = f.root.join(".weftext-rules");
    let objects = BTreeMap::from([
        (
            object.clone(),
            SuppliedObject {
                path: &f.source,
                qualification: ObjectQualification::Node(&node),
            },
        ),
        (
            ObjectKey::Schema,
            SuppliedObject {
                path: &schema_path,
                qualification: ObjectQualification::Configuration,
            },
        ),
        (
            ObjectKey::Rules,
            SuppliedObject {
                path: &rules_path,
                qualification: ObjectQualification::Configuration,
            },
        ),
    ]);
    let after_locations = f
        .local
        .iter()
        .map(|(key, local)| (key.clone(), PathBuf::from(&local.directory)))
        .collect();
    let current = f.graph.objects().unwrap();
    let resolution = super::super::revalidation::SyncRevalidation {
        object: object.clone(),
        revisions: BTreeSet::from([f.chosen[&object].revision.clone()]),
        decision: super::super::revalidation::SyncRevalidationDecision::Preserve {
            revision: f.chosen[&object].revision.clone(),
        },
        state: f.chosen[&object].state.clone(),
        reviewed_dependencies: vec![],
    };
    let checked = super::super::revalidation::inspect_revalidation(
        &f.graph,
        &f.chosen,
        &f.local,
        &[resolution],
        &BTreeSet::from([object.clone()]),
    )
    .unwrap();
    assert_eq!(checked.invocations.len(), 1);
    assert_eq!(
        checked.deferred_queries[0].required_parameters,
        vec!["spare"]
    );
    assert!(checked.invocations[0].context.parameters.is_empty());
    assert_eq!(checked.invocations[0].plan.select.len(), 1);
    assert_eq!(
        checked.invocations[0].schema_baseline.revision,
        binding.revision
    );
    // Definition preflight may retain static Provider descriptors. Only the
    // original actual row input below discharges the invocation obligation.
    assert!(checked.dependency_reads[&object].iter().any(|d| {
        d.object == ObjectKey::Schema
            && d.revisions
                == current[&ObjectKey::Schema]
                    .versions
                    .iter()
                    .map(|v| v.revision.clone())
                    .collect()
    }));
    let cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &f.chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[&query],
        after_locations: &after_locations,
        planned: None,
    };
    let reads = capture_evidence(&f.core, &cut, &checked)
        .unwrap()
        .into_read_set()
        .unwrap();
    assert_eq!(
        reads.query_membership,
        Some(BTreeSet::from([f.owner.node_id.clone()]))
    );
    assert!(
        reads
            .provider_dependencies
            .iter()
            .flat_map(|d| &d.baselines)
            .any(|d| !d.providers.is_empty())
    );
    // A real read witness without the original complete admitted input does
    // not authorize an empty-result substitute for the actual invocation.
    let without_input = SuppliedEvidenceCut {
        query_snapshots: &[],
        ..cut
    };
    let evidence = capture_evidence(&f.core, &without_input, &checked).unwrap();
    assert!(
        evidence
            .unfulfilled()
            .iter()
            .any(|e| matches!(e.gap, EvidenceGap::ExactInvocationInputs))
    );
    assert_eq!(
        evidence.into_read_set().err().unwrap().code,
        "syncEvidenceRequired"
    );
    // Evaluate the selected repaired after bytes. The installed original
    // remains open, while the explicit after now reaches an invalid endpoint.
    let old_source = String::from_utf8(
        f.graph
            .blob(&f.chosen[&object].state.files["Note.typ"])
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let bad_source = old_source.replace("status: \"open\"", "status: \"done\"");
    assert_ne!(old_source, bad_source);
    let bad_digest = hash(bad_source.as_bytes());
    f.graph
        .offer_blob(&bad_digest, bad_source.into_bytes())
        .unwrap();
    let mut bad_chosen = f.chosen.clone();
    let bad_version = bad_chosen.get_mut(&object).unwrap();
    bad_version
        .state
        .files
        .insert("Note.typ".into(), bad_digest);
    bad_version
        .state
        .file_revisions
        .insert("Note.typ".into(), fresh());
    let resolution = super::super::revalidation::SyncRevalidation {
        object: object.clone(),
        revisions: BTreeSet::from([bad_version.revision.clone()]),
        decision: super::super::revalidation::SyncRevalidationDecision::Repair {
            base_revision: bad_version.revision.clone(),
        },
        state: bad_version.state.clone(),
        reviewed_dependencies: vec![],
    };
    let bad_checked = super::super::revalidation::inspect_revalidation(
        &f.graph,
        &bad_chosen,
        &f.local,
        &[resolution],
        &BTreeSet::from([object]),
    )
    .unwrap();
    let bad_cut = SuppliedEvidenceCut {
        root: &f.root,
        graph: &f.graph,
        installed: &f.local,
        chosen: &bad_chosen,
        physical: &f.physical,
        snapshots: &f.snapshots,
        objects: &objects,
        resources: &[],
        query_snapshots: &[&query],
        after_locations: &after_locations,
        planned: None,
    };
    assert_eq!(
        capture_evidence(&f.core, &bad_cut, &bad_checked)
            .err()
            .unwrap()
            .code,
        "queryInvocationUnavailable"
    );
}
