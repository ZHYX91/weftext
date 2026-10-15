use super::*;
use crate::{ConfigRole, fresh};

struct Fixture {
    _temp: tempfile::TempDir,
    core: Core,
    root: PathBuf,
    source: PathBuf,
    object: ObjectKey,
    identity: SyncIdentity,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note(&root, &root, "Original").unwrap();
        let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
        Self {
            _temp: temp,
            core,
            root,
            source: PathBuf::from(note.path),
            object: ObjectKey::Node(note.node_id.unwrap()),
            identity,
        }
    }
    fn resolution(&self) -> SyncRevalidation {
        let graph = self.core.read_sync_history(&self.identity).unwrap();
        let view = &graph.objects().unwrap()[&self.object];
        assert_eq!(view.versions.len(), 1);
        SyncRevalidation {
            object: self.object.clone(),
            revisions: BTreeSet::from([view.versions[0].revision.clone()]),
            decision: revalidation::SyncRevalidationDecision::Preserve {
                revision: view.versions[0].revision.clone(),
            },
            state: view.versions[0].state.clone(),
            reviewed_dependencies: vec![],
        }
    }
    fn save_schema(&mut self) {
        let preview = self
            .core
            .prepare_config_save(
                &self.root,
                ConfigRole::Schema,
                None,
                serde_json::to_vec(&weftext_data::Schema::bundled().unwrap()).unwrap(),
                &fresh(),
            )
            .unwrap();
        self.core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        self.core.publish_sync(&self.root).unwrap();
    }
}

#[test]
fn original_core_review_returns_exact_dependency_cuts_without_staging_or_author_writes() {
    let mut f = Fixture::new();
    let source_before = std::fs::read(&f.source).unwrap();
    let operations_before = f.core.operations.len();
    let cuts_before = f.core.admitted_cuts.len();
    let request = f.resolution();
    let cut = f
        .core
        .sync_revalidation_cut(&f.root, &BTreeSet::from([f.object.clone()]))
        .unwrap();
    assert!(
        cut.objects
            .iter()
            .any(|candidate| candidate.object == f.object)
    );
    let review = f
        .core
        .review_sync_revalidation(&f.root, &f.identity, &[request], &BTreeMap::new())
        .unwrap();
    assert_eq!(std::fs::read(&f.source).unwrap(), source_before);
    assert_eq!(f.core.operations.len(), operations_before);
    assert_eq!(f.core.admitted_cuts.len(), cuts_before);
    assert!(
        review
            .expected_installed
            .iter()
            .any(|entry| entry.object == f.object)
    );
    assert!(
        review
            .reviewed_versions
            .iter()
            .any(|entry| entry.object == ObjectKey::Rules)
    );
    assert!(
        review.resolutions[0]
            .reviewed_dependencies
            .iter()
            .any(|entry| entry.object == ObjectKey::Rules)
    );
    assert!(serde_json::to_value(review).is_ok());
}

#[test]
fn review_follows_the_actual_new_schema_instead_of_older_manifest_dependency_list() {
    let mut f = Fixture::new();
    let request = f.resolution();
    f.save_schema();
    let operations_before = f.core.operations.len();
    let review = f
        .core
        .review_sync_revalidation(&f.root, &f.identity, &[request], &BTreeMap::new())
        .unwrap();
    assert!(
        review.resolutions[0]
            .reviewed_dependencies
            .iter()
            .any(|entry| entry.object == ObjectKey::Schema)
    );
    assert!(
        review
            .reviewed_versions
            .iter()
            .any(|entry| entry.object == ObjectKey::Schema)
    );
    assert!(
        review
            .expected_installed
            .iter()
            .any(|entry| entry.object == ObjectKey::Schema)
    );
    assert_eq!(f.core.operations.len(), operations_before);
}

#[test]
fn late_received_configuration_invalidates_the_original_negative_semantic_cut() {
    let mut f = Fixture::new();
    let setup = f.core.sync_setup_for(&f.root).unwrap();
    let local = f.core.sync_local_state(setup).unwrap().0;
    let application = SyncApplication {
        root: display(&f.root),
        identity: f.identity.clone(),
        before: local.values().cloned().collect(),
        after: local.values().cloned().collect(),
        expected_versions: vec![],
        selection: BTreeSet::from([f.object.clone()]),
        blobs: BTreeMap::new(),
        semantic_evidence: None,
    };
    let evidence = FrozenSemanticEvidence {
        native_products: vec![],
        absent_objects: BTreeSet::from([ObjectKey::Schema]),
        historical_inputs: vec![],
        provider_requirements: vec![],
        provider_dependencies: vec![],
        discharged: vec![],
        deferred_queries: vec![],
        annotation_targets: vec![],
    };
    f.core
        .validate_sync_semantic_evidence(&fresh(), &application, &evidence)
        .unwrap();
    f.save_schema();
    assert_eq!(
        f.core
            .validate_sync_semantic_evidence(&fresh(), &application, &evidence)
            .unwrap_err()
            .code,
        "syncConflictChanged"
    );
}

#[test]
fn original_install_only_configuration_fence_blocks_negative_reads_across_restart() {
    let mut f = Fixture::new();
    let setup = f.core.sync_setup_for(&f.root).unwrap();
    let local = f.core.sync_local_state(setup).unwrap().0;
    let application = SyncApplication {
        root: display(&f.root),
        identity: f.identity.clone(),
        before: local.values().cloned().collect(),
        after: local.values().cloned().collect(),
        expected_versions: vec![],
        selection: BTreeSet::from([f.object.clone()]),
        blobs: BTreeMap::new(),
        semantic_evidence: None,
    };
    let evidence = FrozenSemanticEvidence {
        native_products: vec![],
        absent_objects: BTreeSet::from([ObjectKey::Schema]),
        historical_inputs: vec![],
        provider_requirements: vec![],
        provider_dependencies: vec![],
        discharged: vec![],
        deferred_queries: vec![],
        annotation_targets: vec![],
    };
    let sync_operation = fresh();
    f.core
        .validate_sync_semantic_evidence(&sync_operation, &application, &evidence)
        .unwrap();
    let preview = f
        .core
        .prepare_config_save(
            &f.root,
            ConfigRole::Schema,
            None,
            serde_json::to_vec(&weftext_data::Schema::bundled().unwrap()).unwrap(),
            &fresh(),
        )
        .unwrap();
    let config_plan = f.core.operations[&preview.operation_id].clone();
    f.core.install_record(&config_plan).unwrap();
    let original_source = std::fs::read(&f.source).unwrap();
    assert!(!f.root.join(".weftext-schema.json").exists());
    assert_eq!(
        f.core
            .validate_sync_semantic_evidence(&sync_operation, &application, &evidence)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    let private = f._temp.path().join("private");
    drop(f.core);
    f.core = Core::new(private).unwrap();
    assert_eq!(
        f.core
            .validate_sync_semantic_evidence(&sync_operation, &application, &evidence)
            .unwrap_err()
            .code,
        "recoveryPending"
    );
    assert_eq!(std::fs::read(&f.source).unwrap(), original_source);
    assert!(!f.root.join(".weftext-schema.json").exists());
}

#[test]
fn original_create_plan_supplies_new_owner_admission_and_actual_after_query_rows() {
    let mut f = Fixture::new();
    f.save_schema();
    let replica_root = f._temp.path().join("Replica");
    let mut receiver = Core::new(f._temp.path().join("receiver-private")).unwrap();
    let controls = BTreeSet::from([ObjectKey::Rules]);
    receiver
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &controls).unwrap(),
        )
        .unwrap();
    let clone = receiver
        .prepare_operation(crate::OperationRequest {
            operation_id: fresh(),
            root: display(&replica_root),
            intent: crate::OperationIntent::CloneSync {
                identity: f.identity.clone(),
                objects: controls,
            },
        })
        .unwrap();
    receiver
        .execute_operation(&clone.operation_id, &clone.plan_id)
        .unwrap();
    let created = f.core.create_note(&f.root, &f.root, "Fresh").unwrap();
    let id = created.node_id.unwrap();
    let path = PathBuf::from(created.path);
    let object = ObjectKey::Node(id.clone());
    let config = f.core.read_config(&f.root, ConfigRole::Schema).unwrap();
    let binding = weftext_data::SavedQueryBinding {
        schema: config.role_ref,
        revision: config.revision.unwrap(),
        language: "weftext.query.cel.v1".into(),
    };
    let operation = fresh();
    let basis = f
        .core
        .read_snapshot(&f.root, &[display(&path)])
        .unwrap()
        .files
        .remove(0);
    let key = f
        .core
        .reserve_definition_key(&f.root, &path, &basis.base_id, &operation)
        .unwrap();
    drop(f.core.take_snapshot_witness(&basis.base_id).unwrap());
    let plan = weftext_data::QueryPlan {
        types: vec!["task".into()],
        select: vec![
            weftext_data::QueryColumn {
                name: "actual".into(),
                expr: "nextCivilDay(status == 'done' ? date('9999-12-31') : date('2026-01-01'))"
                    .into(),
            },
            weftext_data::QueryColumn {
                name: "state".into(),
                expr: "status".into(),
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
    let source =
        format!("#metadata((id: \"{id}\", types: (\"task\",), status: \"open\")) <weftext>\n");
    let source = weftext_data::upsert_saved_query(
        &source,
        None,
        &key,
        "weftext-query-fresh",
        &binding,
        &plan,
        &weftext_data::Schema::bundled().unwrap(),
    )
    .unwrap()
    .source;
    let doc = f.core.open(&path, Some(f.root.clone())).unwrap();
    f.core
        .save(crate::SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            source,
            operation_id: operation,
        })
        .unwrap();
    f.core.publish_sync(&f.root).unwrap();
    let graph = f.core.read_sync_history(&f.identity).unwrap();
    let all = graph.objects().unwrap().into_keys().collect();
    receiver
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &all).unwrap(),
        )
        .unwrap();
    let current = receiver
        .read_sync_history(&f.identity)
        .unwrap()
        .objects()
        .unwrap();
    let received = &current[&object].versions[0];
    let resolution = SyncRevalidation {
        object: object.clone(),
        revisions: BTreeSet::from([received.revision.clone()]),
        decision: revalidation::SyncRevalidationDecision::Preserve {
            revision: received.revision.clone(),
        },
        state: received.state.clone(),
        reviewed_dependencies: vec![],
    };
    let review = receiver
        .review_sync_revalidation(&replica_root, &f.identity, &[resolution], &BTreeMap::new())
        .unwrap();
    let input = crate::OperationRequest {
        operation_id: fresh(),
        root: display(&replica_root),
        intent: crate::OperationIntent::ApplySync {
            identity: f.identity.clone(),
            objects: review
                .reviewed_versions
                .iter()
                .map(|entry| entry.object.clone())
                .collect(),
        },
    };
    let preview = receiver.prepare_operation(input).unwrap();
    // Reuse a real original prepared writer plan. The helper only freezes a
    // checked copy here; it never installs or stages a second author operation.
    let mut original_plan = receiver.operations[&preview.operation_id].clone();
    let evidence = receiver
        .freeze_sync_revalidation_plan(&mut original_plan, &review.resolutions)
        .unwrap();
    assert!(!replica_root.join("Fresh/Fresh.typ").exists());
    assert!(
        original_plan
            .transaction
            .as_ref()
            .unwrap()
            .births()
            .iter()
            .any(|birth| birth.node_id == id)
    );
    assert!(
        evidence
            .discharged
            .iter()
            .any(|record| record["kind"] == "definition" && record["key"] == key)
    );
    assert!(evidence.provider_dependencies.iter().any(|record| {
        record["baselines"].as_array().is_some_and(|baselines| {
            baselines.iter().any(|baseline| {
                baseline["providers"]
                    .as_object()
                    .is_some_and(|providers| !providers.is_empty())
            })
        })
    }));
    assert!(
        original_plan
            .transaction
            .as_ref()
            .unwrap()
            .expected_membership
            .is_some()
    );
    // Removing the original birth is not repaired from received metadata.
    let transaction = original_plan.transaction.as_mut().unwrap();
    match &mut transaction.mutation {
        crate::operations::Mutation::CreateTree { births, .. }
        | crate::operations::Mutation::Compound { births, .. } => {
            births.retain(|birth| birth.node_id != id)
        }
        _ => panic!("Original new owner must use the original create writer"),
    }
    assert_eq!(
        receiver
            .freeze_sync_revalidation_plan(&mut original_plan, &review.resolutions)
            .err()
            .unwrap()
            .code,
        "syncEvidenceRequired"
    );
    // A genuinely row-dependent failure differentiates after rows from an
    // empty before snapshot whose Provider preflight alone would still pass.
    let doc = f.core.open(&path, Some(f.root.clone())).unwrap();
    let after = doc.source.replace("status: \"open\"", "status: \"done\"");
    assert_ne!(after, doc.source);
    f.core
        .save(crate::SaveRequest {
            session_id: doc.session_id,
            base_id: doc.base_id,
            source: after,
            operation_id: fresh(),
        })
        .unwrap();
    f.core.publish_sync(&f.root).unwrap();
    receiver
        .receive_sync(
            &f.identity,
            f.core.export_sync_objects(&f.identity, &all).unwrap(),
        )
        .unwrap();
    let current = receiver
        .read_sync_history(&f.identity)
        .unwrap()
        .objects()
        .unwrap();
    let received = &current[&object].versions[0];
    let resolution = SyncRevalidation {
        object: object.clone(),
        revisions: BTreeSet::from([received.revision.clone()]),
        decision: revalidation::SyncRevalidationDecision::Preserve {
            revision: received.revision.clone(),
        },
        state: received.state.clone(),
        reviewed_dependencies: vec![],
    };
    let review = receiver
        .review_sync_revalidation(&replica_root, &f.identity, &[resolution], &BTreeMap::new())
        .unwrap();
    let preview = receiver
        .prepare_operation(crate::OperationRequest {
            operation_id: fresh(),
            root: display(&replica_root),
            intent: crate::OperationIntent::ApplySync {
                identity: f.identity.clone(),
                objects: review
                    .reviewed_versions
                    .iter()
                    .map(|entry| entry.object.clone())
                    .collect(),
            },
        })
        .unwrap();
    let mut after_plan = receiver.operations[&preview.operation_id].clone();
    assert_eq!(
        receiver
            .freeze_sync_revalidation_plan(&mut after_plan, &review.resolutions)
            .err()
            .unwrap()
            .code,
        "queryInvocationUnavailable"
    );
    assert!(!replica_root.join("Fresh/Fresh.typ").exists());
}
