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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProposalPreview {
    session_id: String,
    source: String,
    serial: u64,
    proposal_source: String,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/fonts", get(writing_api::fonts))
        .route("/api/hosted/editor/map", post(map))
        .route("/api/hosted/editor/edit", post(edit))
        .route("/api/hosted/editor/render", post(render))
        .route("/api/hosted/editor/preview", post(preview_proposal))
        .route(
            "/api/hosted/editor/endnotes-module",
            get(writing_api::endnotes_module),
        )
}

async fn preview_proposal(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<ProposalPreview>,
) -> ApiResult {
    writing_api::validate_source(&req.proposal_source, None)?;
    let original = Snapshot {
        session_id: req.session_id,
        source: req.source,
        serial: req.serial,
        source_hash: None,
    };
    let captured = original.clone();
    let check = auth.clone();
    let proposal = req.proposal_source;
    let compiled_source = proposal.clone();
    let (input, dependencies, environment) = locked(&state, move |inner| {
        let (actor, session, _) = qualify(inner, &check, &captured, true, false)?;
        freeze(inner, &actor, &session.resource, &proposal)
    })
    .await?;
    let output = compile_frozen(&state, input).await;
    locked(&state, move |inner| {
        let (actor, session, _) = qualify(inner, &auth, &original, false, false)?;
        validate_inputs(inner, &actor, &session.resource, &dependencies)?;
        environment.recheck(inner, &actor)?;
        let mut output = output?;
        scrub_tree(&mut output["tree"], &session.resource, false);
        let failure = (output["ok"] != true).then(|| {
            ErrorDescription::new(UiMessage::error_api_hosted_editor_proposal_preview_failed())
        });
        Ok(Json(
            json!({"ok":output["ok"],"tree":output["tree"],"source":compiled_source,
            "serial":original.serial,
            "failure":failure}),
        ))
    })
    .await
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
                if matches!(
                    key.as_str(),
                    "writable" | "gridWritable" | "topologyWritable" | "levelWritable" | "editable"
                ) {
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
    RequestJson(req): RequestJson<Snapshot>,
) -> ApiResult {
    let captured = req.clone();
    let check = auth.clone();
    locked(&state, move |inner| {
        qualify(inner, &check, &captured, true, false).map(|_| ())
    })
    .await?;
    let source = req.source.clone();
    let result = crate::work::spawn_blocking(move || weftext_engine::inspect_editor(&source))
        .await
        .map_err(|_| {
            ApiError::internal(UiMessage::error_api_hosted_editor_editor_mapping_incomplete())
        })?;
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
    RequestJson(req): RequestJson<Edit>,
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
    let needs_module = match &command.intent {
        weftext_engine::EditIntent::NativeNote { action } => action.requires_frozen_module(&source),
        weftext_engine::EditIntent::HeadingNumbering { action } => {
            action.requires_frozen_module(&source)
        }
        _ => false,
    };
    if needs_module {
        let captured = basis.clone();
        let check = auth.clone();
        let (input, dependencies, environment) = locked(&state, move |inner| {
            let (actor, session, _) = qualify(inner, &check, &captured, false, true)?;
            freeze(inner, &actor, &session.resource, &captured.source)
        })
        .await?;
        let _permit = state.render_gate.try_acquire().map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "busy",
                UiMessage::error_api_hosted_editor_preview_busy(),
            )
        })?;
        // Worker errors may include private typed diagnostics. Qualify at
        // delivery time before publishing success OR a worker failure.
        let output = worker_failure::observe_hosted_result(
            worker_failure::HostedContext::NativeNotes,
            super::super::notes_edit_from_worker_with(input, command, &state.workers).await,
        );
        return locked(&state, move |inner| {
            Ok(Json(
                deliver_native_notes(inner, &auth, &basis, &dependencies, &environment, output)?
                    .edit,
            ))
        })
        .await;
    }
    let result = crate::work::spawn_blocking(move || weftext_engine::edit(&source, &command))
        .await
        .map_err(|_| {
            ApiError::internal(UiMessage::error_api_hosted_editor_edit_proposal_incomplete())
        })?;
    locked(&state, move |inner| {
        qualify(inner, &auth, &basis, false, true)?;
        let result = result.map_err(|_| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupported_edit",
                UiMessage::error_api_hosted_editor_native_edit_unsupported(),
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
    RequestJson(req): RequestJson<Snapshot>,
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
        let mut info = json!({"syntaxTokens":output["sourceInfo"]["syntaxTokens"],"syntaxValid":output["sourceInfo"]["syntaxValid"],"bodyInsertion":output["sourceInfo"]["bodyInsertion"],"notes":output["sourceInfo"]["notes"]});
        if !rights.write { readonly(&mut info); }
        let failure = (output["ok"] != true).then(|| ErrorDescription::new(UiMessage::error_api_hosted_editor_authorized_compilation_failed()));
        Ok(Json(json!({"ok":output["ok"],"tree":output["tree"],"sourceInfo":info,
            "compilerVersion":output["compilerVersion"],"source":req.source,"serial":req.serial,
            "failure":failure})))
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
        frozen_files: files
            .into_iter()
            .map(|(p, b)| (p, base64::engine::general_purpose::STANDARD.encode(b)))
            .collect(),
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
            UiMessage::error_api_hosted_editor_authorized_dependencies_changed(),
        ));
    }
    Ok(())
}
// One Notes-specific terminal handoff. Old frozen worker outputs and typed
// worker failures cannot leave this endpoint before permission revalidation.
fn deliver_native_notes<T>(
    inner: &mut HostedInner,
    auth: &Auth,
    basis: &Snapshot,
    dependencies: &BTreeMap<String, String>,
    environment: &library::Environment,
    work: Result<T, ApiError>,
) -> Result<T, ApiError> {
    let (actor, session, _) = qualify(inner, auth, basis, false, true)?;
    validate_inputs(inner, &actor, &session.resource, dependencies)?;
    environment.recheck(inner, &actor)?;
    work
}

fn preview_worker_failure(original: ApiError) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "previewUnavailable",
        UiMessage::error_api_hosted_editor_authorized_preview_unavailable(),
    )
    .inherit_worker_failure_from(&original)
}
pub(super) async fn compile_frozen(
    state: &HostedState,
    input: WorkerRequest,
) -> Result<Value, ApiError> {
    let _permit = state.render_gate.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            UiMessage::error_api_hosted_editor_preview_busy(),
        )
    })?;
    worker_failure::observe_hosted_result(
        worker_failure::HostedContext::Editor,
        render_worker_with(input, &state.workers).await,
    )
    .map_err(preview_worker_failure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn native_notes_worker_success_and_failure_cannot_bypass_revocation() {
        // Freeze through the real hosted owner/session/environment path, then
        // change the actual policy while the worker result is outstanding.
        for failure in [false, true] {
            let (_temp, state, auth, document, _, _) = history_api::test_fixture();
            let mut inner = state.inner.lock().unwrap();
            let basis = Snapshot {
                session_id: document["sessionId"].as_str().unwrap().into(),
                source: document["source"].as_str().unwrap().into(),
                serial: 1,
                source_hash: None,
            };
            let (actor, session, _) = qualify(&mut inner, &auth, &basis, false, true).unwrap();
            let (_, dependencies, environment) =
                freeze(&mut inner, &actor, &session.resource, &basis.source).unwrap();
            let result = || {
                if failure {
                    Err(crate::worker_failure::diagnostics_tests::original_failure(
                        crate::worker_failure::Kind::OutputRejected,
                    ))
                } else {
                    Ok(json!({"protected": "worker source"}))
                }
            };
            // Both terminal paths are deliverable while the captured authority
            // remains current; neither can cross a later write revocation.
            assert_eq!(
                deliver_native_notes(
                    &mut inner,
                    &auth,
                    &basis,
                    &dependencies,
                    &environment,
                    result()
                )
                .is_err(),
                failure
            );
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap()
                .write = false;
            let error = deliver_native_notes(
                &mut inner,
                &auth,
                &basis,
                &dependencies,
                &environment,
                result(),
            )
            .unwrap_err();
            assert_eq!(error.code, "unavailable");
            assert_eq!(error.status, StatusCode::NOT_FOUND);
            assert!(error.private_worker_failure.is_none());
        }
    }

    #[tokio::test]
    async fn authorized_preview_preserves_only_the_original_private_worker_facts() {
        use crate::worker_failure::{
            Kind,
            diagnostics_tests::{assert_safe_wrapper, original_failure},
        };
        for kind in [
            Kind::Deadline,
            Kind::NonzeroExit,
            Kind::OutputRejected,
            Kind::SupervisionLost,
        ] {
            let original = original_failure(kind);
            let retained = original.private_worker_failure.as_ref().unwrap().clone();
            assert_safe_wrapper(
                preview_worker_failure(original),
                &retained,
                StatusCode::UNPROCESSABLE_ENTITY,
                "previewUnavailable",
                UiMessage::error_api_hosted_editor_authorized_preview_unavailable(),
            )
            .await;
        }
    }

    #[cfg(windows)]
    #[test]
    fn hosted_collection_endnote_edit_uses_the_actual_frozen_native_path() {
        let (_temp, state, _auth, document, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let main = root.join("Shared/Shared.typ");
        let owner = inner.core.qualified_node_at(&root, &main).unwrap().owner;
        let operation = uuid::Uuid::new_v4().to_string();
        let plan = inner
            .core
            .prepare_import_resource(
                &root,
                &main,
                "weftext-endnotes.typ",
                writing_api::ENDNOTES_MODULE.as_bytes().to_vec(),
                &operation,
            )
            .unwrap();
        inner
            .core
            .execute_operation(&operation, &plan.plan_id)
            .unwrap();
        inner.config.resource_collections.insert(
            "Notes".into(),
            resource_collections::Collection {
                owner: "Shared".into(),
                expected_owner: Some(owner),
                from_import_slot: false,
                from_structure_slot: false,
                lifecycle: "live".into(),
                title: "本篇尾注".into(),
            },
        );
        inner.config.files.get_mut("Shared").unwrap().inputs = vec!["Notes/*".into()];
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resource_collections
            .insert(
                "Notes".into(),
                resource_collections::Rights {
                    read: true,
                    download: false,
                    r#import: false,
                },
            );
        let source = format!(
            "{}\n正文#footnote[脚注]",
            document["source"].as_str().unwrap()
        );
        let (worker, dependencies, environment) =
            freeze(&mut inner, "alice", "Shared", &source).unwrap();
        let files = worker.frozen_files;
        let actual_module_key = std::path::Path::new("Shared")
            .join("weftext-endnotes.typ")
            .to_string_lossy()
            .into_owned();
        assert_eq!(actual_module_key, "Shared\\weftext-endnotes.typ");
        assert!(files.contains_key(&actual_module_key));
        assert!(!files.contains_key("Shared/weftext-endnotes.typ"));
        let input = weftext_engine::FrozenRenderRequest {
            source: worker.source,
            path: worker.path,
            root: worker.root,
            style: worker.style,
            files: files
                .into_iter()
                .map(|(path, encoded)| {
                    (
                        path,
                        base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .unwrap(),
                    )
                })
                .collect(),
        };
        let request = weftext_engine::EditRequest {
            source_hash: weftext_engine::source_hash(&source),
            intent: weftext_engine::EditIntent::NativeNote {
                action: weftext_engine::NotesIntent::InsertEndnote {
                    at: weftext_engine::inspect_editor(&source)
                        .body_insertion
                        .unwrap(),
                    body: weftext_engine::NotesBody {
                        mode: weftext_engine::NotesBodyMode::Literal,
                        value: "尾注正文".into(),
                    },
                },
            },
        };
        let output = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .edit_notes_frozen_with_dependencies(&input, &request)
            .unwrap();
        assert!(
            output
                .edit
                .map
                .notes
                .entries
                .iter()
                .any(|entry| entry.kind == "endnote-definition" && entry.writable)
        );
        assert!(output.dependencies.iter().any(|dependency| dependency.path
            == "Shared/weftext-endnotes.typ"
            && dependency.status == "loaded"));
        let mut numbered_input = input.clone();
        numbered_input.source = output.edit.source.clone();
        let main_key = numbered_input
            .files
            .keys()
            .find(|key| key.replace('\\', "/") == "Shared/Shared.typ")
            .unwrap()
            .clone();
        numbered_input
            .files
            .insert(main_key, numbered_input.source.as_bytes().to_vec());
        let numbered = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .edit_notes_frozen_with_dependencies(
                &numbered_input,
                &weftext_engine::EditRequest {
                    source_hash: weftext_engine::source_hash(&numbered_input.source),
                    intent: weftext_engine::EditIntent::HeadingNumbering {
                        action: Default::default(),
                    },
                },
            )
            .unwrap();
        assert!(numbered.edit.map.heading_numbering.editable);
        assert!(numbered.edit.source.ends_with(&numbered_input.source));
        assert!(
            numbered
                .dependencies
                .iter()
                .any(
                    |dependency| dependency.path == "Shared/weftext-endnotes.typ"
                        && dependency.status == "loaded"
                )
        );
        validate_inputs(&inner, "alice", "Shared", &dependencies).unwrap();
        environment.recheck(&inner, "alice").unwrap();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resource_collections
            .get_mut("Notes")
            .unwrap()
            .read = false;
        assert!(environment.recheck(&inner, "alice").is_err());
        assert!(freeze(&mut inner, "alice", "Shared", &source).is_err());
    }

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
