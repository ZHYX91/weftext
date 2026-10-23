// Included within data_api::tests so the actual App fixtures/producers stay shared.

async fn retire_query_task(state: &AppState, root: &str) -> PathBuf {
    let Json(document) = ok(capture(
        State(state.clone()),
        crate::RequestJson(task_capture(
            root,
            "Retired",
            &uuid::Uuid::new_v4().to_string(),
        )),
    )
    .await);
    assert_eq!(document["status"], "committed");
    let document = document["document"].clone();
    let mut core = state.core.lock().unwrap();
    let retired = core
        .prepare_operation(CoreOperationRequest {
            root: root.into(),
            operation_id: uuid::Uuid::new_v4().to_string(),
            intent: OperationIntent::TrashNode {
                source: Path::new(document["path"].as_str().unwrap())
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            },
        })
        .unwrap();
    core.execute_operation(&retired.operation_id, &retired.plan_id)
        .unwrap();
    PathBuf::from(
        core.trash_admitted_snapshot(root)
            .unwrap()
            .files
            .remove(0)
            .path,
    )
}

fn mixed_status_plan() -> weftext_data::QueryPlan {
    let live = status_plan("open");
    let mut trash = status_plan("open");
    trash.scope.lifecycle = "trash".into();
    weftext_data::QueryPlan {
        advanced: Some(weftext_data::QueryAdvanced {
            union_all: Some(vec![live, trash]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn assert_mixed_rows(result: &Value) {
    assert_eq!(result["coverage"], "complete");
    let rows = result["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .map(|row| row["lifecycle"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["live", "trash"])
    );
    for row in rows {
        let status: weftext_data::Cell =
            serde_json::from_value(row["cells"]["status"].clone()).unwrap();
        assert_eq!(status.native(), Some(&NativeValue::Text("open".into())));
        assert_eq!(row["readOnly"], row["lifecycle"] == "trash");
    }
    let cuts = result["scopeCuts"].as_object().unwrap();
    assert_eq!(cuts.len(), 2);
    assert_ne!(cuts["live"], cuts["trash"]);
    assert_eq!(result["membershipIds"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn mixed_lifecycle_definition_save_run_and_callee_use_both_original_read_cuts() {
    let (_directory, state, root) = fixture();
    install_bundled_schema(&state, &root).await;
    let Json(live) = ok(capture(
        State(state.clone()),
        crate::RequestJson(task_capture(
            &root,
            "Live",
            &uuid::Uuid::new_v4().to_string(),
        )),
    )
    .await);
    assert_eq!(live["status"], "committed");
    let live = live["document"].clone();
    let trash_path = retire_query_task(&state, &root).await;
    let trash_bytes = std::fs::read(&trash_path).unwrap();
    let mut request =
        definition_request(&live, live["source"].as_str().unwrap().into(), None, "open");
    request.plan = mixed_status_plan();
    let original_request = request.clone();
    let Json(saved) = ok(save_definition(State(state.clone()), crate::RequestJson(request)).await);
    let Json(replayed) =
        ok(save_definition(State(state.clone()), crate::RequestJson(original_request)).await);
    assert_eq!(replayed["definition"]["key"], saved["definition"]["key"]);
    let current = &saved["document"];
    let owner = format!(
        "weftext-node://{}/{}",
        current["workspaceId"].as_str().unwrap(),
        current["nodeId"].as_str().unwrap()
    );
    let selection = weftext_data::DefinitionSelection::Current {
        key: saved["definition"]["key"].as_str().unwrap().into(),
    };
    let Json(ran) = ok(run_definition(
        State(state.clone()),
        crate::RequestJson(DefinitionRunRequest {
            root: root.clone(),
            owner: owner.clone(),
            selection: selection.clone(),
            parameters: BTreeMap::new(),
        }),
    )
    .await);
    assert_mixed_rows(&ran["result"]);
    let Json(called) = ok(query(
        State(state.clone()),
        crate::RequestJson(QueryRequest {
            root: root.clone(),
            plan: weftext_data::QueryPlan {
                advanced: Some(weftext_data::QueryAdvanced {
                    call: Some(weftext_data::QueryCall {
                        owner,
                        selection,
                        arguments: BTreeMap::new(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            parameters: BTreeMap::new(),
        }),
    )
    .await);
    assert_mixed_rows(&called);
    assert_eq!(std::fs::read(&trash_path).unwrap(), trash_bytes);
    assert_eq!(
        std::fs::read_to_string(current["path"].as_str().unwrap()).unwrap(),
        saved["committedSource"].as_str().unwrap()
    );
}

#[tokio::test]
async fn mixed_lifecycle_addition_cannot_replace_a_source_in_the_original_native_world() {
    let (_directory, state, root) = fixture();
    let _ = ok(capture(
        State(state.clone()),
        crate::RequestJson(task_capture(
            &root,
            "Live",
            &uuid::Uuid::new_v4().to_string(),
        )),
    )
    .await);
    let trash_path = retire_query_task(&state, &root).await;
    let mut core = state.core.lock().unwrap();
    let mut frozen = query_render_snapshot(&mut core, &root, mixed_status_plan(), None).unwrap();
    assert_eq!(frozen.scope_cuts.len(), 1);
    let changed = format!(
        "{}\nExternal later draft",
        std::fs::read_to_string(&trash_path).unwrap()
    );
    std::fs::write(&trash_path, &changed).unwrap();
    assert_eq!(
        freeze_query_definitions(&mut core, &mut frozen, BTreeMap::new())
            .unwrap_err()
            .code,
        "scopeChanged"
    );
    assert_eq!(frozen.scope_cuts.len(), 1);
    assert!(
        frozen
            .input
            .notes
            .iter()
            .all(|note| note.lifecycle == "live")
    );
    assert_eq!(std::fs::read_to_string(trash_path).unwrap(), changed);
}

#[tokio::test]
async fn mixed_lifecycle_trash_leaf_loads_a_qualified_live_callee_before_scope_closure() {
    let (_directory, state, root) = fixture();
    install_bundled_schema(&state, &root).await;
    let Json(live) = ok(capture(
        State(state.clone()),
        crate::RequestJson(task_capture(
            &root,
            "Live",
            &uuid::Uuid::new_v4().to_string(),
        )),
    )
    .await);
    assert_eq!(live["status"], "committed");
    let live = live["document"].clone();
    let trash_path = retire_query_task(&state, &root).await;
    let trash_bytes = std::fs::read(&trash_path).unwrap();
    let Json(saved) = ok(save_definition(
        State(state.clone()),
        crate::RequestJson(definition_request(
            &live,
            live["source"].as_str().unwrap().into(),
            None,
            "open",
        )),
    )
    .await);
    let owner = format!(
        "weftext-node://{}/{}",
        live["workspaceId"].as_str().unwrap(),
        live["nodeId"].as_str().unwrap()
    );
    let call = weftext_data::QueryPlan {
        advanced: Some(weftext_data::QueryAdvanced {
            call: Some(weftext_data::QueryCall {
                owner: owner.clone(),
                selection: weftext_data::DefinitionSelection::Current {
                    key: saved["definition"]["key"].as_str().unwrap().into(),
                },
                arguments: BTreeMap::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut plan = status_plan("open");
    plan.scope.lifecycle = "trash".into();
    plan.advanced = Some(weftext_data::QueryAdvanced {
        related: Some(weftext_data::QueryRelated {
            relation: "people/parent".into(),
            from: Box::new(call.clone()),
            direction: "forward".into(),
        }),
        ..Default::default()
    });
    let Json(result) = ok(query(
        State(state.clone()),
        crate::RequestJson(QueryRequest {
            root: root.clone(),
            plan,
            parameters: BTreeMap::new(),
        }),
    )
    .await);
    // Tasks have no people/parent edges: zero follows actual qualified scans,
    // rather than a missing owner or an omitted lifecycle masquerading as zero.
    assert_eq!(result["coverage"], "complete");
    assert!(result["rows"].as_array().unwrap().is_empty());
    assert_eq!(result["scopeCuts"].as_object().unwrap().len(), 2);
    assert!(
        result["inputVersions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|version| {
                version["role"] == "definition"
                    && version["owner"] == owner
                    && version["definitionRevision"] == saved["definitionRevision"]
            })
    );
    assert_eq!(std::fs::read(&trash_path).unwrap(), trash_bytes);
    let mut invalid_terminal = call;
    invalid_terminal.scope.lifecycle = "trash".into();
    let rejected = query(
        State(state),
        crate::RequestJson(QueryRequest {
            root,
            plan: invalid_terminal,
            parameters: BTreeMap::new(),
        }),
    )
    .await;
    let rejected = rejected.unwrap_err();
    assert_eq!(rejected.code, "invalid_data");
    assert_eq!(
        rejected.explanation,
        UiMessage::error_data_query_advanced_terminal_scope_must_be_in_scan_branches()
    );
}
