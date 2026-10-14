//! Native draft proposals, qualified by the hosted actor and exact draft.
//! No author write takes place here; the original save owns persistence.
use super::*;

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
struct Edit {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: String,
    intent: weftext_engine::EditIntent,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/fonts", get(writing_api::fonts))
        .route("/api/hosted/editor/map", post(map))
        .route("/api/hosted/editor/edit", post(edit))
        .route("/api/hosted/editor/render", post(render))
}
fn qualify(
    inner: &mut HostedInner,
    auth: &Auth,
    req: &Snapshot,
    observe: bool,
    write: bool,
) -> Result<(String, HostedSession, Rights), ApiError> {
    let qualified = inner.session(auth, &req.session_id)?;
    if write && !qualified.2.write {
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
    Ok(qualified)
}
// Maps carry source ranges, not capabilities. Reader maps explicitly disable
// write flags as well as the independent endpoint authorization check.
fn readonly(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "writable" || key == "gridWritable" {
                    *value = json!(false);
                } else {
                    readonly(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                readonly(value);
            }
        }
        _ => {}
    }
}
async fn map(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Snapshot>,
) -> ApiResult {
    let captured = req.clone();
    let check = auth.clone();
    locked(&state, move |inner| {
        qualify(inner, &check, &captured, true, false).map(|_| ())
    })
    .await?;
    let source = req.source.clone();
    let result = tokio::task::spawn_blocking(move || weftext_engine::inspect_editor(&source))
        .await
        .map_err(|_| ApiError::internal("编辑映射未完成"))?;
    locked(&state, move |inner| {
        let (_, _, rights) = qualify(inner, &auth, &req, false, false)?;
        let mut result = value(result)?;
        if !rights.write {
            readonly(&mut result);
        }
        result["serial"] = json!(req.serial);
        Ok(Json(result))
    })
    .await
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
        source_hash: Some(req.source_hash.clone()),
    };
    let captured = basis.clone();
    let check = auth.clone();
    locked(&state, move |inner| {
        qualify(inner, &check, &captured, false, true).map(|_| ())
    })
    .await?;
    let source = basis.source.clone();
    let command = weftext_engine::EditRequest {
        source_hash: req.source_hash,
        intent: req.intent,
    };
    let result = tokio::task::spawn_blocking(move || weftext_engine::edit(&source, &command))
        .await
        .map_err(|_| ApiError::internal("编辑提案未完成"))?;
    locked(&state, move |inner| {
        qualify(inner, &auth, &basis, false, true)?;
        let result = result.map_err(|_| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupported_edit",
                "当前原生构造不能安全执行此编辑；草稿保持原样，可使用源码入口",
            )
        })?;
        Ok(Json(value(result)?))
    })
    .await
}
fn scrub_tree(tree: &mut Value, resource: &str, writable: bool) {
    if tree.is_null() {
        return;
    }
    if tree["origin"]["isMain"] == true {
        tree["origin"]["path"] = json!(format!("resource:{resource}"));
        if !writable {
            tree["writable"] = json!(false);
        }
    } else {
        tree["origin"] = Value::Null;
        tree["writable"] = json!(false);
    }
    // Query carrier identifiers and ordinary/native link destinations are
    // presentation data. Their consumers independently authorize execution or
    // target opening; arbitrary metadata and active attributes never cross.
    if tree["kind"] == "tag" {
        let href = tree["attrs"]["href"]
            .as_str()
            .filter(|href| {
                tree["tag"] == "a"
                    && !href.chars().any(char::is_control)
                    && (href.starts_with("weftext-node://")
                        || href.starts_with("weftext-resource://")
                        || href
                            .get(..7)
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
                        || href
                            .get(..8)
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://")))
            })
            .map(str::to_owned);
        tree["attrs"] = Value::Object(
            ["data-weftext-query-label", "data-weftext-query-occurrence"]
                .into_iter()
                .filter_map(|key| {
                    tree["attrs"][key]
                        .as_str()
                        .map(|value| (key.to_string(), json!(value)))
                })
                .collect(),
        );
        if let Some(href) = href {
            tree["attrs"]["href"] = json!(href);
        }
    }
    if let Some(children) = tree["children"].as_array_mut() {
        for child in children {
            scrub_tree(child, resource, writable);
        }
    }
}
async fn render(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Snapshot>,
) -> ApiResult {
    let captured = req.clone();
    let check = auth.clone();
    let (input, dependencies, environment) = locked(&state, move |inner| {
        let (actor, session, _) = qualify(inner, &check, &captured, true, false)?;
        freeze(inner, &actor, &session.resource, &captured.source)
    })
    .await?;
    let output = compile_frozen(&state, input).await;
    locked(&state, move |inner| {
        let (actor, session, rights) = qualify(inner, &auth, &req, false, false)?;
        validate_inputs(inner, &actor, &session.resource, &dependencies)?;
        environment.recheck(inner, &actor)?;
        let mut output = output?;
        scrub_tree(&mut output["tree"], &session.resource, rights.write);
        let mut info = json!({"syntaxTokens":output["sourceInfo"]["syntaxTokens"],"syntaxValid":output["sourceInfo"]["syntaxValid"],"bodyInsertion":output["sourceInfo"]["bodyInsertion"]});
        if !rights.write { readonly(&mut info); }
        Ok(Json(json!({"ok":output["ok"],"tree":output["tree"],"sourceInfo":info,
            "compilerVersion":output["compilerVersion"],"source":req.source,"serial":req.serial,
            "message":if output["ok"] == true { "" } else { "无法在当前获权依赖内编译；草稿保留，可在源码入口修正" }})))
    }).await
}
/// Internal compilation capability shared by authorized writing and review.
pub(super) fn freeze(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source: &str,
) -> Result<
    (
        WorkerRequest,
        BTreeMap<String, String>,
        library::Environment,
    ),
    ApiError,
> {
    inner.rights(actor, resource)?;
    let selected = inner
        .file(resource)
        .ok()
        .ok_or_else(unavailable)?
        .inputs
        .clone();
    let captured = inputs::capture(inner, actor, &selected)?;
    freeze_with_inputs(inner, actor, resource, source, captured)
}
pub(super) fn freeze_with_inputs(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source: &str,
    captured_inputs: inputs::Captured,
) -> Result<
    (
        WorkerRequest,
        BTreeMap<String, String>,
        library::Environment,
    ),
    ApiError,
> {
    inner.rights(actor, resource)?;
    writing_api::validate_source(source, None)?;
    let file = inner.file(resource).ok().ok_or_else(unavailable)?.clone();
    captured_inputs.validate(inner, actor)?;
    // This entry only reuses an already frozen selection for this exact slot.
    if !captured_inputs.matches_selection(&file.inputs) {
        return Err(unavailable());
    }
    let mut files = captured_inputs.files()?;
    let dependencies = files.iter().map(|(p, b)| (p.clone(), digest(b))).collect();
    files.insert(file.path.clone(), source.as_bytes().to_vec());
    let mut worker = WorkerRequest {
        source: source.into(),
        path: file.path.clone().into(),
        root: inner.config.root.clone(),
        paged: false,
        style: None,
        frozen_files: Some(
            files
                .into_iter()
                .map(|(p, b)| (p, base64::engine::general_purpose::STANDARD.encode(b)))
                .collect(),
        ),
    };
    let mut environment = library::environment(inner, actor)?;
    environment.attach_inputs(inner, actor, captured_inputs)?;
    if let Some(citations) = citations::freeze(inner, actor, resource, source)? {
        citations.inject(&mut worker)?;
        environment.attach_citations(inner, actor, citations)?;
    }
    environment.apply(&mut worker)?;
    Ok((worker, dependencies, environment))
}
pub(super) fn validate_inputs(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
    expected: &BTreeMap<String, String>,
) -> Result<(), ApiError> {
    inner.rights(actor, resource)?;
    let file = inner.file(resource).ok().ok_or_else(unavailable)?;
    let current = inner
        .files(actor, &file.inputs)?
        .iter()
        .map(|(p, b)| (p.clone(), digest(b)))
        .collect::<BTreeMap<_, _>>();
    if &current != expected {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "staleEnvironment",
            "获权依赖已变化，请重新检查。",
        ));
    }
    Ok(())
}
pub(super) async fn compile_frozen(
    state: &HostedState,
    input: WorkerRequest,
) -> Result<Value, ApiError> {
    let _permit = state.render_gate.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            "预览正在运行，请稍后再试",
        )
    })?;
    render_worker(input).await.map_err(|_| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "previewUnavailable",
            "无法在获权依赖内完成预览",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_links_keep_destinations_without_forwarding_active_attributes_or_paths() {
        for href in [
            "https://example.test/a",
            "HTTP://example.test/b",
            "weftext-node://workspace/node",
            "weftext-resource://workspace/resource",
        ] {
            let mut tree = json!({"kind":"tag","tag":"a","origin":{"isMain":true,"path":"C:/private/note.typ"},
                "attrs":{"href":href,"onclick":"unsafe()","style":"private","data-private":"secret"},"children":[]});
            scrub_tree(&mut tree, "Shared", false);
            assert_eq!(tree["attrs"], json!({"href":href}));
            assert_eq!(tree["origin"]["path"], "resource:Shared");
            assert_eq!(tree["writable"], false);
        }
        for href in [
            "javascript:alert(1)",
            "file:///C:/private/note.typ",
            "C:/private/note.typ",
            "data:text/html,unsafe",
            "https://example.test/\nprivate",
        ] {
            let mut tree = json!({"kind":"tag","tag":"a","attrs":{"href":href},"children":[]});
            scrub_tree(&mut tree, "Shared", true);
            assert!(tree["attrs"].as_object().unwrap().is_empty());
        }
        let mut carrier = json!({"kind":"tag","tag":"span","attrs":{"href":"https://example.test/","data-weftext-query-label":"query-one","data-weftext-query-occurrence":"1","data-private":"secret"},"children":[]});
        scrub_tree(&mut carrier, "Shared", true);
        assert_eq!(
            carrier["attrs"],
            json!({"data-weftext-query-label":"query-one","data-weftext-query-occurrence":"1"})
        );
    }
}
