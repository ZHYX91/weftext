//! A local whole-document source proposal. This route never saves author bytes
//! and is deliberately absent from Hosted's field-scoped authorization surface.
use super::*;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FieldTransfer {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: String,
    // A missing schema binding is not the same as the bundled schema's null revision.
    #[serde(deserialize_with = "required_revision")]
    schema_revision: Option<String>,
    semantic_digest: String,
    from_field: String,
    to_field: String,
    start: usize,
    end: usize,
    to: usize,
}

fn required_revision<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

pub(super) async fn transfer(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<FieldTransfer>,
) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        Some(&req.source_hash),
    )?;
    let loc = location(&state, &req.session_id)?;
    deny_trash_writer(&loc.path)?;
    let root = loc.root.clone();
    let (config, resolved) =
        core_call(&state, move |core| resolved_schema(core, &loc.root)).await?;
    if config.revision != req.schema_revision
        || schema_semantic_digest(&resolved)? != req.semantic_digest
    {
        return Err(CoreError::new(
            "configConflict",
            UiMessage::error_api_data_schema_changed_during_evaluation(),
        )
        .into());
    }
    // Permanent IDs bind both current schema slots. A same-named key or an
    // absent field does not become the destination of this proposal.
    let selected_fields = [&req.from_field, &req.to_field]
        .into_iter()
        .map(|id| {
            resolved
                .fields
                .iter()
                .find(|field| &field.id == id)
                .map(|field| field.id.clone())
                .ok_or_else(|| {
                    ApiError::bad(UiMessage::error_api_data_selected_field_not_installed())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let source = req.source.clone();
    let from = req.from_field.clone();
    let into = req.to_field.clone();
    let relation_schema = resolved.clone();
    let patch = crate::work::spawn_blocking(move || {
        weftext_data::patch_list_transfer(
            &source, &resolved, &from, &into, req.start, req.end, req.to,
        )
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_data_data_edit_incomplete()))?
    .map_err(data_error)?;
    // One frozen relation graph and validation context cover the joint final
    // state. Neither of the two intermediate single-field edits is published.
    check_relation_proposal(
        &state,
        root.clone(),
        relation_schema,
        req.source.clone(),
        patch.source.clone(),
        Some(selected_fields),
    )
    .await?;
    schema_still_current(&state, root, &config).await?;
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        Some(&req.source_hash),
    )?;
    let Json(mut response) = patch_response(patch, req.serial, &req.source)?;
    response["schemaRevision"] = json!(req.schema_revision);
    response["semanticDigest"] = json!(req.semantic_digest);
    response["transfer"] = json!({"fromField":req.from_field,"toField":req.to_field,
        "start":req.start,"end":req.end,"to":req.to});
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_requires_closed_exact_source_schema_and_ordinal_bindings() {
        let value = json!({"sessionId":"s","source":"source","serial":7,"sourceHash":"hash",
            "schemaRevision":null,"semanticDigest":"digest","fromField":"x/a","toField":"x/b",
            "start":1,"end":2,"to":0});
        assert!(serde_json::from_value::<FieldTransfer>(value.clone()).is_ok());
        for key in [
            "sourceHash",
            "schemaRevision",
            "semanticDigest",
            "fromField",
            "start",
            "end",
            "to",
        ] {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<FieldTransfer>(missing).is_err(),
                "{key}"
            );
        }
        for (key, invalid) in [
            ("start", json!(-1)),
            ("end", json!(1.5)),
            ("to", json!("0")),
            ("schemaRevision", json!(1)),
        ] {
            let mut changed = value.clone();
            changed[key] = invalid;
            assert!(
                serde_json::from_value::<FieldTransfer>(changed).is_err(),
                "{key}"
            );
        }
        let mut widened = value;
        widened["destinationSource"] = json!("other document");
        assert!(serde_json::from_value::<FieldTransfer>(widened).is_err());
    }

    #[cfg(windows)]
    struct Fixture {
        _directory: tempfile::TempDir,
        state: AppState,
        document: weftext_core::OpenedDocument,
        schema: Schema,
        revision: Option<String>,
    }

    #[cfg(windows)]
    fn fixture(min: usize, max: Option<usize>, resource_constraint: bool) -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let root_path = directory.path().join("library");
        let root = root_path.to_string_lossy().to_string();
        let mut core = Core::new(directory.path().join("state")).unwrap();
        core.create_library(&root_path).unwrap();
        let mut schema_json: Value = serde_json::from_str(BUNDLED_SCHEMA_SOURCE).unwrap();
        let semantic = if resource_constraint {
            "resource_ref"
        } else {
            "text"
        };
        for (id, key, minimum, maximum) in [
            ("transfer/from", "from_values", min, None),
            ("transfer/into", "into_values", 0, max),
        ] {
            schema_json["fields"].as_array_mut().unwrap().push(json!({"id":id,"key":key,
                "shape":{"kind":"list","items":{"kind":"scalar","semantic":semantic},"min":minimum,"max":maximum},
                "nullable":false,"default":{"kind":"absent"}}));
        }
        let constraints = if resource_constraint {
            vec![json!({"validator":"core/owned-resources",
            "inputs":{"owner":{"kind":"context","name":"owner"},"values":{"kind":"field","id":"transfer/into","path":[{"kind":"each"}]},"metadata":{"kind":"context","name":"resource_metadata"}},
            "parameters":{"owner_required":true}})]
        } else {
            vec![]
        };
        schema_json["types"].as_array_mut().unwrap().push(json!({"id":"transfer/test",
            "fields":[{"id":"transfer/from","required":false},{"id":"transfer/into","required":false}],
            "requires":[],"conflicts":[],"constraints":constraints}));
        let schema_source = serde_json::to_string(&schema_json).unwrap();
        let schema = Schema::from_author_json(&schema_source).unwrap();
        let preview = core
            .prepare_config_save(
                &root,
                ConfigRole::Schema,
                None,
                schema_source.clone().into_bytes(),
                &uuid::Uuid::new_v4().to_string(),
            )
            .unwrap();
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        let revision = core
            .read_config(&root, ConfigRole::Schema)
            .unwrap()
            .revision;
        let document = core.create_note_fixture(&root, &root, "Move").unwrap();
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("test"),
            port: 12345,
            locations: Arc::new(Mutex::new(HashMap::from([(
                document.session_id.clone(),
                Location {
                    path: document.path.clone().into(),
                    root: root_path,
                },
            )]))),
            renders: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
            exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        Fixture {
            _directory: directory,
            state,
            document,
            schema,
            revision,
        }
    }

    #[cfg(windows)]
    fn request(fixture: &Fixture, fields: &str, serial: u64) -> FieldTransfer {
        let source = format!(
            "// 中文😀\r\n#metadata((id: \"{}\", types: (\"transfer/test\",), {fields})) <weftext>\r\n正文不改\r\n",
            fixture.document.node_id.as_ref().unwrap()
        );
        FieldTransfer {
            session_id: fixture.document.session_id.clone(),
            source_hash: weftext_engine::source_hash(&source),
            source,
            serial,
            schema_revision: fixture.revision.clone(),
            semantic_digest: schema_semantic_digest(&fixture.schema).unwrap(),
            from_field: "transfer/from".into(),
            to_field: "transfer/into".into(),
            start: 1,
            end: 2,
            to: 1,
        }
    }

    #[cfg(windows)]
    fn original_unchanged(fixture: Fixture) {
        let Fixture {
            _directory,
            state,
            document,
            ..
        } = fixture;
        // Do not bypass the active Core author's Windows sharing boundary.
        // Once the request and owner have drained, compare the actual bytes.
        drop(state);
        assert_eq!(
            std::fs::read_to_string(&document.path).unwrap(),
            document.source
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn transfer_returns_one_utf16_patch_and_keeps_disk_original() {
        let fixture = fixture(0, None, false);
        let req = request(
            &fixture,
            "from_values: (\"相同\", \"相同\", \"尾😀\"), unknown: (keep: true), into_values: (\"前\", \"后\"),",
            1,
        );
        let Json(result) = transfer(State(fixture.state.clone()), RequestJson(req.clone()))
            .await
            .unwrap();
        assert_eq!(result["patches"].as_array().unwrap().len(), 1);
        assert_eq!(result["semanticDigest"], req.semantic_digest);
        assert_eq!(result["schemaRevision"], json!(req.schema_revision));
        assert_eq!(
            result["transfer"],
            json!({"fromField":"transfer/from","toField":"transfer/into","start":1,"end":2,"to":1})
        );
        let patch = &result["patches"][0];
        let from = patch["byteFrom"].as_u64().unwrap() as usize;
        let to = patch["byteTo"].as_u64().unwrap() as usize;
        assert_eq!(patch["from"], req.source[..from].encode_utf16().count());
        assert_eq!(patch["to"], req.source[..to].encode_utf16().count());
        assert!(patch["from"].as_u64().unwrap() < from as u64);
        let expected = format!(
            "{}{}{}",
            &req.source[..from],
            patch["text"].as_str().unwrap(),
            &req.source[to..]
        );
        assert_eq!(result["source"], expected);
        let projected = weftext_data::project(&expected, &fixture.schema);
        let values = |id: &str| {
            projected
                .fields
                .iter()
                .find(|field| field.field_id == id)
                .unwrap()
                .cell
                .native()
                .unwrap()
                .array()
                .unwrap()
                .iter()
                .map(|value| value.text().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(values("transfer/from"), ["相同"]);
        assert_eq!(values("transfer/into"), ["前", "相同", "尾😀", "后"]);
        assert!(expected.contains("unknown: (keep: true)"));
        original_unchanged(fixture);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn transfer_refuses_schema_source_snapshot_and_field_rebinding() {
        let fixture = fixture(0, None, false);
        let req = request(
            &fixture,
            "from_values: (\"a\", \"b\", \"c\"), into_values: (\"x\",)",
            1,
        );
        let mut changed = req.clone();
        changed.schema_revision = None;
        assert_eq!(
            transfer(State(fixture.state.clone()), RequestJson(changed))
                .await
                .unwrap_err()
                .code,
            "configConflict"
        );
        let mut changed = req.clone();
        changed.semantic_digest = "0".repeat(64);
        assert_eq!(
            transfer(State(fixture.state.clone()), RequestJson(changed))
                .await
                .unwrap_err()
                .code,
            "configConflict"
        );
        let mut changed = req.clone();
        changed.source_hash = "0".repeat(64);
        assert_eq!(
            transfer(State(fixture.state.clone()), RequestJson(changed))
                .await
                .unwrap_err()
                .code,
            "source_hash_mismatch"
        );
        let mut changed = req.clone();
        changed.from_field = "from_values".into();
        assert_eq!(
            transfer(State(fixture.state.clone()), RequestJson(changed))
                .await
                .unwrap_err()
                .code,
            "invalid_request"
        );
        writing_api::observe_snapshot(
            &fixture.state,
            &req.session_id,
            &req.source,
            2,
            Some(&req.source_hash),
        )
        .unwrap();
        assert_eq!(
            transfer(State(fixture.state.clone()), RequestJson(req))
                .await
                .unwrap_err()
                .code,
            "stale_draft"
        );
        original_unchanged(fixture);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn transfer_rechecks_both_cardinalities_and_current_type_constraints() {
        for (min, max) in [(2, None), (0, Some(2))] {
            let fixture = fixture(min, max, false);
            let req = request(
                &fixture,
                "from_values: (\"a\", \"b\", \"c\"), into_values: (\"x\",)",
                1,
            );
            assert_eq!(
                transfer(State(fixture.state.clone()), RequestJson(req))
                    .await
                    .unwrap_err()
                    .code,
                "invalid_data"
            );
            original_unchanged(fixture);
        }
        let fixture = fixture(0, None, true);
        let resource = format!(
            "weftext-resource://{}/{}",
            fixture.document.workspace_id.as_ref().unwrap(),
            uuid::Uuid::new_v4()
        );
        let mut req = request(
            &fixture,
            &format!("from_values: (\"{resource}\",), into_values: ()"),
            1,
        );
        req.start = 0;
        req.end = 0;
        req.to = 0;
        assert_eq!(
            transfer(State(fixture.state.clone()), RequestJson(req))
                .await
                .unwrap_err()
                .code,
            "invalidData"
        );
        original_unchanged(fixture);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn transfer_never_turns_missing_null_computed_or_invalid_fields_into_lists() {
        let fixture = fixture(0, None, false);
        for (index, fields) in [
            "from_values: (\"a\", \"b\", \"c\")",
            "from_values: (\"a\", \"b\", \"c\"), into_values: none",
            "from_values: (\"a\", \"b\", \"c\"), into_values: \"not a list\"",
            "from_values: (\"a\", \"b\", \"c\"), into_values: (1,)",
            "from_values: (\"a\", \"b\", \"c\"), into_values: (\"x\",) + (\"y\",)",
            "from_values: none, into_values: (\"x\",)",
        ]
        .into_iter()
        .enumerate()
        {
            let req = request(&fixture, fields, index as u64 + 1);
            assert_eq!(
                transfer(State(fixture.state.clone()), RequestJson(req))
                    .await
                    .unwrap_err()
                    .code,
                "invalid_data"
            );
        }
        original_unchanged(fixture);
    }
}
