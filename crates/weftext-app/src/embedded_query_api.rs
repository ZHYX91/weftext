//! Source-scoped embedded Query presentation and Draft proposals. The session
//! buffer/Undo owns edits; this adapter never saves or installs definitions.
use super::*;
use std::collections::BTreeMap;

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
}
fn query_error(error: weftext_data::DataError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "embeddedQueryUnavailable",
        error.to_string(),
    )
}

async fn discover(State(state): State<AppState>, Json(req): Json<DiscoverRequest>) -> ApiResult {
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
    let (syntax_valid, mut occurrences) = tokio::task::spawn_blocking(move || {
        (
            weftext_engine::inspect(&source).syntax_valid,
            weftext_data::embedded_queries(&source),
        )
    })
    .await
    .map_err(|_| ApiError::internal("Query source discovery did not complete"))?;
    if read_only {
        for occurrence in &mut occurrences {
            occurrence.writable = false;
            occurrence.builder_supported = false;
            occurrence.source.range.writable = false;
            occurrence.source.reason = Some("此会话是只读笔记，Query 源码不能写入。".into());
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

async fn run(State(state): State<AppState>, Json(req): Json<RunRequest>) -> ApiResult {
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
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "embeddedQueryUnavailable",
            occurrence
                .source
                .reason
                .or(occurrence.control_reason)
                .unwrap_or_else(|| "Query source cannot be qualified for this Draft".into()),
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

async fn edit(State(state): State<AppState>, Json(req): Json<EditRequest>) -> ApiResult {
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
    let patch = match (req.plan, req.replacement) {
        (Some(plan), None) => {
            let occurrence = weftext_data::embedded_query(&req.source, &req.occurrence_id)
                .map_err(query_error)?;
            if !occurrence.builder_supported {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "embeddedQueryBuilderUnsupported",
                    occurrence
                        .control_reason
                        .or(occurrence.source.reason)
                        .unwrap_or_else(|| "Builder cannot preserve this native Query".into()),
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
        (None, Some(replacement)) => weftext_data::replace_embedded_query_source(
            &req.source,
            &req.occurrence_id,
            &replacement,
        )
        .map_err(query_error)?,
        _ => {
            return Err(ApiError::bad(
                "Provide exactly one builder plan or source replacement",
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
            Json(DiscoverRequest {
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
            Json(EditRequest {
                session_id: session.clone(),
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
                occurrence_id: id.clone(),
                plan: None,
                replacement: Some("#metadata((invalid: ".into()),
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
            Json(EditRequest {
                session_id: session,
                source: QUERY.into(),
                serial: 1,
                source_hash: None,
                occurrence_id: id,
                plan: None,
                replacement: Some(QUERY.into()),
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
            Json(DiscoverRequest {
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
            Json(RunRequest {
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
