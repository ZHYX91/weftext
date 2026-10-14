//! Hosted native Query carriers share the exact source session and Undo buffer.
//! Discovery and edits are proposals only; the original source save persists.
use super::*;
use weftext_data::{NativeValue, QueryPlan};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Snapshot {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Run {
    #[serde(default, deserialize_with = "query_providers::unique_fingerprints")]
    providers: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "query_providers::unique_fingerprints")]
    series_policies: BTreeMap<String, String>,
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    occurrence_id: String,
    #[serde(default, deserialize_with = "data_api::unique_fields")]
    parameters: BTreeMap<String, NativeValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Edit {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    occurrence_id: String,
    plan: Option<QueryPlan>,
    replacement: Option<String>,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/query/embedded/discover", post(discover))
        .route("/api/hosted/query/embedded/run", post(run))
        .route("/api/hosted/query/embedded/edit", post(edit))
}

fn qualify(
    inner: &mut HostedInner,
    auth: &Auth,
    req: &Snapshot,
    observe: bool,
    write: bool,
) -> Result<(String, HostedSession, Rights), ApiError> {
    let access = inner.session(auth, &req.session_id)?;
    if write && !access.2.write {
        return Err(unavailable());
    }
    writing_api::validate_source(&req.source, req.source_hash.as_deref())?;
    if observe {
        writing_api::observe_version(
            &mut inner.editor_versions,
            &req.session_id,
            &req.source,
            req.serial,
        )?;
    } else {
        writing_api::exact_version(
            &inner.editor_versions,
            &req.session_id,
            &req.source,
            req.serial,
        )?;
    }
    Ok(access)
}

fn query_error(_: weftext_data::DataError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "embeddedQueryUnavailable",
        "当前原生查询或准确来源不可用，保留原文；请重新检查该查询",
    )
}

async fn check(
    state: &HostedState,
    auth: &Auth,
    req: &Snapshot,
    observe: bool,
    write: bool,
) -> Result<(String, HostedSession, Rights), ApiError> {
    let auth = auth.clone();
    let req = req.clone();
    locked(state, move |inner| {
        qualify(inner, &auth, &req, observe, write)
    })
    .await
}

async fn discover(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Snapshot>,
) -> ApiResult {
    check(&state, &auth, &req, true, false).await?;
    let source = req.source.clone();
    let (syntax_valid, mut occurrences) = tokio::task::spawn_blocking(move || {
        (
            weftext_engine::inspect(&source).syntax_valid,
            weftext_data::embedded_queries(&source),
        )
    })
    .await
    .map_err(|_| ApiError::internal("查询来源检查未完成"))?;
    let (_, _, rights) = check(&state, &auth, &req, false, false).await?;
    for occurrence in &mut occurrences {
        if !rights.write {
            occurrence.writable = false;
            occurrence.builder_supported = false;
            occurrence.source.range.writable = false;
        }
        if !rights.query.definitions {
            occurrence.runnable = false;
            occurrence.control_reason = Some("运行此定义需要独立的查询定义读取权限".into());
        }
    }
    Ok(Json(json!({"serial":req.serial,"source":req.source,
        "sourceHash":weftext_engine::source_hash(&req.source),
        "syntaxValid":syntax_valid,"occurrences":occurrences})))
}

async fn run(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Run>,
) -> ApiResult {
    let basis = Snapshot {
        session_id: req.session_id,
        source: req.source,
        serial: req.serial,
        source_hash: req.source_hash,
    };
    let (_, session, rights) = check(&state, &auth, &basis, true, false).await?;
    if !rights.query.definitions {
        return Err(unavailable());
    }
    let occurrence =
        weftext_data::embedded_query(&basis.source, &req.occurrence_id).map_err(query_error)?;
    if !occurrence.runnable {
        return Err(query_error(weftext_data::DataError(String::new())));
    }
    let output = query::run_embedded(
        &state,
        &auth,
        session.resource,
        basis.source.clone(),
        occurrence.plan.ok_or_else(unavailable)?,
        occurrence.binding.ok_or_else(unavailable)?,
        req.parameters,
        query_providers::Selection {
            providers: req.providers,
            series_policies: req.series_policies,
        },
        basis.session_id.clone(),
        basis.serial,
    )
    .await?;
    check(&state, &auth, &basis, false, false).await?;
    let mut result = output["result"].clone();
    result["queryId"] = output["queryId"].clone();
    result["columns"] = output["columns"].clone();
    Ok(Json(json!({"serial":basis.serial,"source":basis.source,
        "sourceHash":weftext_engine::source_hash(&basis.source),
        "occurrenceId":req.occurrence_id,"draftPreview":true,"result":result})))
}

async fn edit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Edit>,
) -> ApiResult {
    let basis = Snapshot {
        session_id: req.session_id,
        source: req.source,
        serial: req.serial,
        source_hash: req.source_hash,
    };
    // Edit must consume the registered exact draft; it cannot register itself.
    let (_, session, _) = check(&state, &auth, &basis, false, true).await?;
    let patch = match (req.plan, req.replacement) {
        (Some(plan), None) => {
            let occurrence = weftext_data::embedded_query(&basis.source, &req.occurrence_id)
                .map_err(query_error)?;
            if !occurrence.builder_supported {
                return Err(query_error(weftext_data::DataError(String::new())));
            }
            let plan = query::validate_embedded(
                &state,
                &auth,
                session.resource,
                basis.source.clone(),
                plan.clone(),
                occurrence.binding.ok_or_else(unavailable)?,
            )
            .await?;
            weftext_data::edit_embedded_query_plan(&basis.source, &req.occurrence_id, &plan)
                .map_err(query_error)?
        }
        (None, Some(replacement)) => weftext_data::replace_embedded_query_source(
            &basis.source,
            &req.occurrence_id,
            &replacement,
        )
        .map_err(query_error)?,
        _ => return Err(ApiError::bad("请选择完整构建器提案或准确源码替换之一")),
    };
    check(&state, &auth, &basis, false, true).await?;
    data_api::patch_response(patch, basis.serial, &basis.source)
}
