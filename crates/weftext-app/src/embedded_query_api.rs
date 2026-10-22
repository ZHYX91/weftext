//! Source-scoped embedded Query presentation and Draft proposals. The session
//! buffer/Undo owns edits; this adapter never saves or installs definitions.
use super::*;
use std::collections::BTreeMap;
use weftext_messages::UiMessage;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/query/embedded/discover", post(discover))
        .route("/api/query/embedded/run", post(run))
        .route("/api/query/embedded/edit", post(edit))
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DiscoverRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RunRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    occurrence_id: String,
    #[serde(default, deserialize_with = "data_api::unique_fields")]
    parameters: BTreeMap<String, weftext_data::NativeValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EditRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    occurrence_id: String,
    plan: Option<weftext_data::QueryPlan>,
    replacement: Option<String>,
    action: Option<String>,
}
fn query_error(error: weftext_data::DataError) -> ApiError {
    ApiError::from_data(StatusCode::CONFLICT, "embeddedQueryUnavailable", error)
}

// Source-origin and Data-control failures are independent facts. Preserve the
// original chosen headline while retaining classification and every problem.
fn occurrence_error(
    occurrence: &weftext_data::EmbeddedQueryOccurrence,
    code: &str,
    description: ErrorDescription,
) -> ApiError {
    let mut groups = Vec::new();
    if let Some(classification) = occurrence.control_kind {
        let mut diagnostics = occurrence
            .control_reason
            .clone()
            .into_iter()
            .collect::<Vec<_>>();
        diagnostics.extend(occurrence.control_diagnostics.clone());
        groups.push(weftext_core::ErrorDetailGroup::Data {
            classification,
            diagnostics,
        });
    }
    if let Some(source) = occurrence.source.reason.clone() {
        groups.push(weftext_core::ErrorDetailGroup::Product {
            code: None,
            diagnostics: vec![source],
        });
    }
    let details =
        (!groups.is_empty()).then_some(weftext_core::CoreErrorDetails::Aggregate { groups });
    ApiError::from_core_parts(StatusCode::CONFLICT, code, description, details)
}

async fn discover(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<DiscoverRequest>,
) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    let read_only = data_api::deny_trash_writer(&loc.path).is_err();
    let source = req.source.clone();
    let (syntax_valid, mut occurrences) = crate::work::spawn_blocking(move || {
        (
            weftext_engine::inspect(&source).syntax_valid,
            weftext_data::embedded_queries(&source),
        )
    })
    .await
    .map_err(|_| {
        ApiError::internal(UiMessage::error_api_embedded_query_query_discovery_incomplete())
    })?;
    if read_only {
        for occurrence in &mut occurrences {
            occurrence.writable = false;
            occurrence.builder_supported = false;
            occurrence.source.range.writable = false;
            occurrence.source.reason =
                Some(UiMessage::error_api_embedded_query_read_only_source().into());
        }
    }
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    Ok(Json(
        json!({"serial":req.serial,"source":req.source,"sourceHash":weftext_engine::source_hash(&req.source),"syntaxValid":syntax_valid,"occurrences":occurrences}),
    ))
}

async fn run(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RunRequest>,
) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    let occurrence =
        weftext_data::embedded_query(&req.source, &req.occurrence_id).map_err(query_error)?;
    if !occurrence.runnable {
        return Err(occurrence_error(
            &occurrence,
            "embeddedQueryUnavailable",
            occurrence
                .source
                .reason
                .clone()
                .or_else(|| occurrence.control_reason.clone())
                .unwrap_or_else(|| {
                    UiMessage::error_api_embedded_query_draft_source_unqualified().into()
                }),
        ));
    }
    let result = data_api::run_embedded_query(
        &state,
        loc.root.to_string_lossy().into(),
        loc.path,
        req.source.clone(),
        occurrence.plan.unwrap(),
        occurrence.binding.unwrap(),
        req.parameters,
    )
    .await?;
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    if let Some(id) = result["queryId"].as_str() {
        export_api::bind_embedded_query(
            &state,
            id,
            export_api::EmbeddedQueryBasis {
                session_id: req.session_id.clone(),
                source_hash: weftext_engine::source_hash(&req.source),
                serial: req.serial,
                occurrence_id: req.occurrence_id.clone(),
            },
        )?;
    }
    Ok(Json(
        json!({"serial":req.serial,"source":req.source,"sourceHash":weftext_engine::source_hash(&req.source),"occurrenceId":req.occurrence_id,"draftPreview":true,"result":result}),
    ))
}

async fn edit(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<EditRequest>,
) -> ApiResult {
    // An edit must match a previously observed opening basis. Registering an
    // old proposal here would allow it to acquire a writer after latency.
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    data_api::deny_trash_writer(&loc.path)?;
    let patch = match (req.plan, req.replacement, req.action) {
        (Some(plan), None, None) => {
            let occurrence = weftext_data::embedded_query(&req.source, &req.occurrence_id)
                .map_err(query_error)?;
            if !occurrence.builder_supported {
                return Err(occurrence_error(
                    &occurrence,
                    "embeddedQueryBuilderUnsupported",
                    occurrence
                        .control_reason
                        .clone()
                        .or_else(|| occurrence.source.reason.clone())
                        .unwrap_or_else(|| {
                            UiMessage::error_api_embedded_query_builder_cannot_preserve().into()
                        }),
                ));
            }
            data_api::validate_embedded_query_plan(
                &state,
                loc.root.to_string_lossy().into(),
                loc.path,
                req.source.clone(),
                plan.clone(),
                occurrence.binding.unwrap(),
            )
            .await?;
            weftext_data::edit_embedded_query_plan(&req.source, &req.occurrence_id, &plan)
                .map_err(query_error)?
        }
        (None, Some(replacement), None) => weftext_data::replace_embedded_query_source(
            &req.source,
            &req.occurrence_id,
            &replacement,
        )
        .map_err(query_error)?,
        (None, None, Some(action)) => {
            weftext_data::arrange_embedded_query(&req.source, &req.occurrence_id, &action)
                .map_err(query_error)?
        }
        _ => {
            return Err(ApiError::bad(
                UiMessage::error_api_embedded_query_single_query_proposal_required(),
            ));
        }
    };
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    data_api::patch_response(patch, req.serial, &req.source)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    const QUERY: &str = "#metadata((kind: \"weftext.query\", key: \"f8597b15-6b7f-4a6a-9b52-c2732730fe41\", binding: (schema: \"weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema\", revision: \"8a81e35f-ad48-4c12-af4f-3d2607102fa9\", language: \"weftext.query.cel.v1\"), where: \"true\")) <weftext-query-one>";

    #[test]
    fn control_failure_keeps_real_classification_and_all_parallel_source_facts() {
        let mut occurrence = weftext_data::embedded_queries(QUERY).remove(0);
        let original_range = serde_json::to_value(&occurrence.source.range).unwrap();
        let parallel = (0..48)
            .map(|index| {
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: format!("problem-{index}"),
                    },
                ))
                .diagnostic(
                    DiagnosticOrigin::Dependency,
                    format!("original diagnostic {index}"),
                )
            })
            .collect::<Vec<_>>();
        occurrence.control_kind = Some(weftext_data::DataErrorKind::NativeBudget);
        occurrence.control_reason = Some(UiMessage::common_save().into());
        occurrence.control_diagnostics = parallel.clone();
        let source = ErrorDescription::new(UiMessage::error_api_embedded_query_read_only_source());
        occurrence.source.reason = Some(source.clone());
        let error = occurrence_error(&occurrence, "embeddedQueryUnavailable", source.clone());
        let wire = serde_json::to_value(&error).unwrap();
        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(wire["code"], "embeddedQueryUnavailable");
        assert_eq!(
            wire["explanation"],
            serde_json::to_value(&source.explanation).unwrap()
        );
        let groups = wire["details"]["details"]["groups"].as_array().unwrap();
        assert_eq!(groups[0]["classification"], "nativeBudget");
        let problems = groups[0]["diagnostics"].as_array().unwrap();
        assert_eq!(problems.len(), 49);
        assert_eq!(
            problems[0],
            serde_json::to_value(occurrence.control_reason.as_ref().unwrap()).unwrap()
        );
        assert_eq!(
            &problems[1..],
            serde_json::to_value(&parallel).unwrap().as_array().unwrap()
        );
        assert_eq!(
            groups[1]["diagnostics"],
            serde_json::to_value(vec![source]).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&occurrence.source.range).unwrap(),
            original_range
        );
        assert_eq!(
            occurrence.control_kind,
            Some(weftext_data::DataErrorKind::NativeBudget)
        );
    }

    fn fixture() -> (tempfile::TempDir, AppState, String) {
        let directory = tempfile::tempdir().unwrap();
        let mut core = Core::new(directory.path().join("state")).unwrap();
        let path = directory.path().join("ordinary.typ");
        std::fs::write(&path, QUERY).unwrap();
        let document = core.open(&path, Some(directory.path().to_owned())).unwrap();
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("fixture"),
            port: 12345,
            locations: Arc::new(Mutex::new(HashMap::new())),
            renders: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
            exports: Arc::new(Mutex::new(export_api::ExportStore::default())),
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        remember(&state, &value(&document).unwrap()).unwrap();
        (directory, state, document.session_id)
    }

    #[tokio::test]
    async fn source_proposals_are_exact_draft_only_and_late_edits_cannot_reacquire_writer() {
        let (directory, state, session) = fixture();
        let Json(found) = discover(
            State(state.clone()),
            crate::RequestJson(DiscoverRequest {
                session_id: session.clone(),
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
            }),
        )
        .await
        .unwrap();
        let id = found["occurrences"][0]["id"].as_str().unwrap().to_string();
        let Json(patch) = edit(
            State(state.clone()),
            crate::RequestJson(EditRequest {
                session_id: session.clone(),
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
                occurrence_id: id.clone(),
                plan: None,
                replacement: Some("#metadata((invalid: ".into()),
                action: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(patch["patches"][0]["expected"], QUERY);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("ordinary.typ")).unwrap(),
            QUERY
        );
        writing_api::observe_snapshot(
            &state,
            &session,
            &format!("{QUERY}\nnew unsaved input"),
            2,
            None,
        )
        .unwrap();
        let error = edit(
            State(state),
            crate::RequestJson(EditRequest {
                session_id: session,
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
                occurrence_id: id,
                plan: None,
                replacement: Some(QUERY.into()),
                action: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "stale_draft");
    }

    #[tokio::test]
    async fn ordinary_source_carriers_do_not_grant_library_query_authority() {
        let (_directory, state, session) = fixture();
        let Json(found) = discover(
            State(state.clone()),
            crate::RequestJson(DiscoverRequest {
                session_id: session.clone(),
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
            }),
        )
        .await
        .unwrap();
        let error = run(
            State(state),
            crate::RequestJson(RunRequest {
                session_id: session,
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
                occurrence_id: found["occurrences"][0]["id"].as_str().unwrap().into(),
                parameters: BTreeMap::new(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "queryUnavailable");
    }
}

#[cfg(test)]
mod typed_error_tests {
    use super::*;
    use weftext_core::{CoreErrorDetails, ErrorDetailGroup};
    use weftext_messages::ErrorDescription;

    #[test]
    fn embedded_query_error_keeps_semantic_kind_and_all_parallel_diagnostics() {
        let explanation = UiMessage::error_api_embedded_query_draft_source_unqualified();
        let diagnostics: Vec<_> = (0..48)
            .map(|_| ErrorDescription::new(UiMessage::error_api_embedded_query_read_only_source()))
            .collect();
        let error = query_error(
            weftext_data::DataError::native_budget(explanation.clone())
                .with_diagnostics(diagnostics.clone()),
        );
        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.code, "embeddedQueryUnavailable");
        assert_eq!(error.explanation, explanation);
        assert!(error.causes.as_slice().is_empty());
        let Some(ApiErrorDetails::Core { error: core }) = error.details.as_deref() else {
            panic!("missing original Core error");
        };
        let Some(CoreErrorDetails::Aggregate { groups }) = &core.details else {
            panic!("missing typed Data error details");
        };
        let [
            ErrorDetailGroup::Data {
                classification,
                diagnostics: retained,
            },
        ] = groups.as_slice()
        else {
            panic!("unexpected Data error detail kind");
        };
        assert_eq!(*classification, weftext_data::DataErrorKind::NativeBudget);
        assert_eq!(retained, &diagnostics);
    }
}
