//! Stable Node references resolve only through explicitly configured, readable
//! canonical owners. A source link never grants access to its target.
use super::*;
use weftext_knowledge::NodeRef;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Session {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Reference {
    Uri(String),
    Node(NodeRef),
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Resolve {
    reference: Reference,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fragment {
    session_id: String,
    source: String,
    source_hash: String,
    serial: u64,
    reference: String,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/navigation/ref", post(reference))
        .route("/api/hosted/navigation/resolve", post(resolve))
        .route("/api/hosted/navigation/fragment", post(fragment))
}
fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .is_ok_and(|id| id.to_string() == value && id.get_version() == Some(uuid::Version::Random))
}
struct ParsedReference {
    node: NodeRef,
    label: Option<String>,
    reference: String,
}
fn decode_fragment(raw: &str) -> Result<String, ApiError> {
    let invalid = || ApiError::bad("稳定链接片段须为非空的合法 UTF-8 标签；请检查百分号编码");
    if raw.is_empty()
        || raw.len() > 3072
        || raw.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(invalid());
    }
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let pair = bytes.get(at + 1..at + 3).ok_or_else(invalid)?;
            let hi = (pair[0] as char).to_digit(16).ok_or_else(invalid)?;
            let lo = (pair[1] as char).to_digit(16).ok_or_else(invalid)?;
            decoded.push((hi * 16 + lo) as u8);
            at += 3;
        } else {
            decoded.push(bytes[at]);
            at += 1;
        }
    }
    let label = String::from_utf8(decoded).map_err(|_| invalid())?;
    if label.is_empty() || label.len() > 1024 || label.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(label)
}
fn parse_reference(reference: Reference) -> Result<ParsedReference, ApiError> {
    let (node, label, reference) = match reference {
        Reference::Node(node) => {
            let reference = uri(&node);
            (node, None, reference)
        }
        Reference::Uri(uri) => {
            if uri.len() > 4096 {
                return Err(ApiError::bad("稳定链接超出长度预算"));
            }
            let (identity, raw) = uri
                .split_once('#')
                .map_or((uri.as_str(), None), |(identity, fragment)| {
                    (identity, Some(fragment))
                });
            if raw.is_some_and(|raw| raw.contains('#')) {
                return Err(ApiError::bad("稳定链接只允许一个片段分隔符"));
            }
            let label = raw.map(decode_fragment).transpose()?;
            let (workspace, node) = identity
                .strip_prefix("weftext-node://")
                .and_then(|tail| tail.split_once('/'))
                .ok_or_else(|| ApiError::bad("请使用完整的稳定笔记引用"))?;
            (
                NodeRef {
                    workspace_id: workspace.into(),
                    node_id: node.into(),
                },
                label,
                uri,
            )
        }
    };
    if !valid_uuid(&node.workspace_id) || !valid_uuid(&node.node_id) {
        return Err(ApiError::bad("请使用完整的稳定笔记引用"));
    }
    Ok(ParsedReference {
        node,
        label,
        reference,
    })
}
fn uri(node: &NodeRef) -> String {
    format!("weftext-node://{}/{}", node.workspace_id, node.node_id)
}
fn final_owner(
    inner: &mut HostedInner,
    auth: &Auth,
    resource: &str,
    original: &weftext_core::QualifiedNode,
    session_id: Option<&str>,
) -> Result<(), ApiError> {
    let actor = inner.actor(auth)?;
    inner.rights(&actor, resource)?;
    let file = author_file(inner, resource)?;
    let root = inner.config.root.clone();
    let path = root.join(&file.path);
    let current = if let Some(session_id) = session_id {
        let (_, session, _) = inner.session(auth, session_id)?;
        if session.resource != resource {
            return Err(unavailable());
        }
        let scope = inner.actor_scope(&actor);
        inner.core.with_actor(&scope, |core| {
            core.qualified_node_for_session_at(&root, &path, session_id)
        })
    } else {
        inner.core.qualified_node_at(&root, &path)
    }
    .map_err(|_| unavailable())?;
    if current.owner != original.owner {
        return Err(unavailable());
    }
    inner
        .core
        .validate_snapshot_witness(&root, &path, &original.witness)
        .map_err(|_| unavailable())
}
async fn reference(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Session>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let (actor, session, _) = inner.session(&auth, &req.session_id)?;
        let file = author_file(inner, &session.resource)?;
        let root = inner.config.root.clone();
        let path = root.join(&file.path);
        let scope = inner.actor_scope(&actor);
        let qualified = inner.core.with_actor(&scope, |core| core.qualified_node_for_session_at(&root, &path, &req.session_id)).map_err(|_| unavailable())?;
        // The same original owner and source permission are rechecked before
        // publishing; no Core session/path or body is included in this DTO.
        final_owner(inner, &auth, &session.resource, &qualified, Some(&req.session_id))?;
        Ok(json!({"resource":session.resource,"reference":uri(&qualified.owner),"nodeRef":qualified.owner,"readOnly":true}))
    }).await?))
}
async fn resolve(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Resolve>,
) -> ApiResult {
    let target = parse_reference(req.reference)?;
    let check = auth.clone();
    let (resolved,compile)=locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let candidates = inner.config.files.iter().filter(|(r, _)| inner.rights(&actor, r).is_ok()).map(|(r, f)| (r.clone(), f.clone())).collect::<Vec<_>>();
        let mut partial = false;
        let mut found = None;
        for (resource, file) in candidates {
            if !file.editable && std::path::Path::new(&file.path).extension().is_none_or(|e| e != "typ") {
                continue;
            }
            let file = match author_file(inner, &resource) {
                Ok(file) => file,
                Err(_) => { partial = true; continue; }
            };
            let path = inner.config.root.join(&file.path);
            match inner.core.qualified_node_at(&inner.config.root, &path) {
                Ok(qualified) if qualified.owner == target.node => {
                    // Ambiguous configured aliases never select a namesake.
                    if found.is_some() { return Err(unavailable()); }
                    found = Some((resource, file, qualified));
                }
                Ok(_) => {}
                Err(_) => partial = true,
            }
        }
        let (resource, file, qualified) = found.ok_or_else(unavailable)?;
        final_owner(inner, &auth, &resource, &qualified, None)?;
        let mut public=json!({"resource":resource,"title":file.title,"reference":target.reference,"nodeRef":target.node,"canOpen":true,
            "scope":"configuredAuthorizedSources","coverage":if partial { "partial" } else { "complete" },"readOnly":true});
        let compile=if let Some(label)=target.label {
            let source=inner.core.hosted_files(&inner.config.root,std::slice::from_ref(&file.path)).map_err(|_|unavailable())?
                .remove(&file.path).and_then(|bytes|String::from_utf8(bytes).ok()).ok_or_else(unavailable)?;
            if digest(source.as_bytes())!=qualified.source_hash {return Err(unavailable());}
            let compile=prepare_compile(inner,&actor,&resource,source,label,qualified,None)?;
            public["canOpen"]=false.into();Some(compile)
        }else{None};
        Ok((public,compile))
    }).await?;
    let Some(mut compile) = compile else {
        return Ok(Json(resolved));
    };
    let result = compile.worker.take();
    let output = match result {
        Some(worker) => editor::compile_frozen(&state, worker).await.ok(),
        None => None,
    };
    locked(&state, move |inner| {
        recheck_compile(inner, &check, &compile)?;
        let mut resolved = resolved;
        let location = compiled_location(
            &compile.source,
            &compile.label,
            &compile.resource,
            "saved",
            output.as_ref(),
        );
        resolved["canOpen"] = (location["status"] == "found").into();
        resolved["fragment"] = location;
        Ok(Json(resolved))
    })
    .await
}

struct Compilation {
    actor: String,
    epoch: u64,
    resource: String,
    source: String,
    label: String,
    owner: weftext_core::QualifiedNode,
    worker: Option<WorkerRequest>,
    dependencies: BTreeMap<String, String>,
    environment: Option<library::Environment>,
    draft: Option<Fragment>,
}
fn prepare_compile(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source: String,
    label: String,
    owner: weftext_core::QualifiedNode,
    draft: Option<Fragment>,
) -> Result<Compilation, ApiError> {
    let prepared = editor::freeze(inner, actor, resource, &source).ok();
    let (worker, dependencies, environment) = match prepared {
        Some((worker, dependencies, environment)) => {
            let bytes = owner
                .witness
                .retained_bytes()
                .saturating_add(source.len())
                .saturating_add(environment.retained_bytes())
                .saturating_add(
                    serde_json::to_vec(&worker)
                        .map_err(|_| ApiError::bad("标签编译输入不可用"))?
                        .len(),
                );
            if bytes > 32 * 1024 * 1024 {
                return Err(ApiError::bad("标签编译及原始依据超出 32 MiB 预算"));
            }
            (Some(worker), dependencies, Some(environment))
        }
        None => (None, BTreeMap::new(), None),
    };
    Ok(Compilation {
        actor: actor.into(),
        epoch: inner.actor_generation(actor),
        resource: resource.into(),
        source,
        label,
        owner,
        worker,
        dependencies,
        environment,
        draft,
    })
}
fn recheck_compile(
    inner: &mut HostedInner,
    auth: &Auth,
    compile: &Compilation,
) -> Result<(), ApiError> {
    let actor = inner.actor(auth)?;
    if actor != compile.actor || inner.actor_generation(&actor) != compile.epoch {
        return Err(expired());
    }
    final_owner(
        inner,
        auth,
        &compile.resource,
        &compile.owner,
        compile.draft.as_ref().map(|d| d.session_id.as_str()),
    )?;
    if let Some(d) = &compile.draft {
        writing_api::validate_source(&d.source, Some(&d.source_hash))?;
        writing_api::exact_version(&inner.editor_versions, &d.session_id, &d.source, d.serial)?;
    }
    if let Some(environment) = &compile.environment {
        editor::validate_inputs(inner, &actor, &compile.resource, &compile.dependencies)?;
        environment.recheck(inner, &actor)?;
    }
    Ok(())
}
/// Native introspection proves existence and cardinality; an exact main-source
/// origin proves where to reveal. No DOM id or author AST label is guessed.
fn compiled_location(
    source: &str,
    label: &str,
    resource: &str,
    basis: &str,
    output: Option<&Value>,
) -> Value {
    let mut result = json!({"label":label,"status":"unavailable","basis":basis,"sourceHash":digest(source.as_bytes())});
    let Some(output) = output.filter(|output| output["ok"] == true) else {
        return result;
    };
    let Some(labels) = output["labels"].as_array() else {
        return result;
    };
    let mut matching = labels.iter().filter(|actual| actual["label"] == label);
    let Some(actual) = matching.next() else {
        result["status"] = "missing".into();
        return result;
    };
    if matching.next().is_some() {
        result["status"] = "ambiguous".into();
        return result;
    }
    let Ok(actual) = serde_json::from_value::<weftext_engine::CompiledLabel>(actual.clone()) else {
        return result;
    };
    if actual.count > 1 {
        result["status"] = "ambiguous".into();
        return result;
    }
    if actual.count == 0 {
        result["status"] = "missing".into();
        return result;
    }
    let [origin] = actual.origins.as_slice() else {
        return result;
    };
    if !origin.is_main
        || origin.from >= origin.to
        || source.get(origin.from..origin.to) != Some(origin.raw.as_str())
        || source
            .get(..origin.from)
            .is_none_or(|s| s.encode_utf16().count() != origin.utf16_from)
        || source
            .get(..origin.to)
            .is_none_or(|s| s.encode_utf16().count() != origin.utf16_to)
    {
        return result;
    }
    let mut origin = origin.clone();
    origin.path = format!("resource:{resource}");
    result["status"] = "found".into();
    result["origin"] = serde_json::to_value(origin).unwrap_or(Value::Null);
    result
}
async fn fragment(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Fragment>,
) -> ApiResult {
    let parsed = parse_reference(Reference::Uri(req.reference.clone()))?;
    let label = parsed
        .label
        .ok_or_else(|| ApiError::bad("此定位请求须带一个明确标签片段"))?;
    let check = auth.clone();
    let captured = req.clone();
    let mut compile = locked(&state, move |inner| {
        let (actor, session, _) = inner.session(&check, &captured.session_id)?;
        writing_api::validate_source(&captured.source, Some(&captured.source_hash))?;
        writing_api::exact_version(
            &inner.editor_versions,
            &captured.session_id,
            &captured.source,
            captured.serial,
        )?;
        let file = author_file(inner, &session.resource)?;
        let root = inner.config.root.clone();
        let path = root.join(&file.path);
        let scope = inner.actor_scope(&actor);
        let owner = inner
            .core
            .with_actor(&scope, |core| {
                core.qualified_node_for_session_at(&root, &path, &captured.session_id)
            })
            .map_err(|_| unavailable())?;
        if owner.owner != parsed.node {
            return Err(unavailable());
        }
        prepare_compile(
            inner,
            &actor,
            &session.resource,
            captured.source.clone(),
            label,
            owner,
            Some(captured),
        )
    })
    .await?;
    let worker = compile.worker.take();
    let output = match worker {
        Some(worker) => editor::compile_frozen(&state, worker).await.ok(),
        None => None,
    };
    locked(&state,move|inner| {
        recheck_compile(inner,&auth,&compile)?;
        let mut location=compiled_location(&compile.source,&compile.label,&compile.resource,"draft",output.as_ref());
        if weftext_engine::inspect_metadata(&compile.source).id.as_deref()!=Some(compile.owner.owner.node_id.as_str()) {location["status"]="unavailable".into();location.as_object_mut().unwrap().remove("origin");}
        Ok(Json(json!({"resource":compile.resource,"sourceHash":req.source_hash,"serial":req.serial,"reference":req.reference,"fragment":location})))
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_reference_inputs_are_exact_and_never_paths_or_names() {
        let target = NodeRef {
            workspace_id: uuid::Uuid::new_v4().to_string(),
            node_id: uuid::Uuid::new_v4().to_string(),
        };
        assert_eq!(
            parse_reference(Reference::Uri(uri(&target))).unwrap().node,
            target
        );
        assert_eq!(
            parse_reference(Reference::Node(target.clone()))
                .unwrap()
                .node,
            target
        );
        for value in [
            "Shared",
            "C:/Notes/Shared.typ",
            "https://server/Shared",
            "weftext-node://workspace/node",
            &format!("{}?path=Shared", uri(&target)),
            &format!("{}/other", uri(&target)),
        ] {
            assert!(parse_reference(Reference::Uri(value.into())).is_err());
        }
        let extra = json!({"reference":{"workspaceId":target.workspace_id,"nodeId":target.node_id,"path":"private"}});
        assert!(serde_json::from_value::<Resolve>(extra).is_err());
    }
    #[test]
    fn fragment_decodes_utf8_once_without_normalizing_native_label_identity() {
        let node = NodeRef {
            workspace_id: uuid::Uuid::new_v4().to_string(),
            node_id: uuid::Uuid::new_v4().to_string(),
        };
        for (fragment, label) in [
            ("中文", "中文"),
            ("%E4%B8%AD%E6%96%87", "中文"),
            ("%23", "#"),
            ("%25", "%"),
            ("%2523", "%23"),
            ("a+b", "a+b"),
        ] {
            let reference = format!("{}#{fragment}", uri(&node));
            let parsed = parse_reference(Reference::Uri(reference.clone())).unwrap();
            assert_eq!(parsed.node, node);
            assert_eq!(parsed.label.as_deref(), Some(label));
            assert_eq!(parsed.reference, reference);
        }
        for fragment in [
            "",
            "%",
            "%G0",
            "%1",
            "%FF",
            "%C0%AF",
            "%ED%A0%80",
            "%00",
            "%0A",
            "%7F",
            "first#second",
            "raw space",
        ] {
            assert_eq!(
                parse_reference(Reference::Uri(format!("{}#{fragment}", uri(&node))))
                    .err()
                    .unwrap()
                    .status,
                StatusCode::BAD_REQUEST
            );
        }
    }
    #[test]
    fn native_cardinality_and_exact_main_origin_are_independent_requirements() {
        let source = "😀正文";
        let label = "定位";
        let mut output = json!({"ok":true,"labels":[{"label":label,"count":1,"origins":[{"path":"private/module.typ","from":4,"to":10,"utf16From":2,"utf16To":4,"raw":"正文","syntaxKind":"Text","isMain":true}]}]});
        let located = compiled_location(source, label, "Shared", "draft", Some(&output));
        assert_eq!(located["status"], "found");
        assert_eq!(located["origin"]["path"], "resource:Shared");
        assert!(!located.to_string().contains("private/module"));
        output["labels"][0]["count"] = 2.into();
        assert_eq!(
            compiled_location(source, label, "Shared", "draft", Some(&output))["status"],
            "ambiguous"
        );
        output["labels"][0]["count"] = 1.into();
        output["labels"][0]["origins"][0]["isMain"] = false.into();
        assert_eq!(
            compiled_location(source, label, "Shared", "draft", Some(&output))["status"],
            "unavailable"
        );
        output["labels"][0]["origins"][0]["isMain"] = true.into();
        output["labels"][0]["origins"][0]["utf16From"] = 4.into();
        assert_eq!(
            compiled_location(source, label, "Shared", "draft", Some(&output))["status"],
            "unavailable"
        );
        output["labels"][0]["origins"] = json!([]);
        assert_eq!(
            compiled_location(source, label, "Shared", "draft", Some(&output))["status"],
            "unavailable"
        );
        output["labels"] = json!([]);
        assert_eq!(
            compiled_location("<定位>", label, "Shared", "saved", Some(&output))["status"],
            "missing"
        );
        output["ok"] = false.into();
        assert_eq!(
            compiled_location(source, label, "Shared", "draft", Some(&output))["status"],
            "unavailable"
        );
    }
    #[cfg(windows)]
    fn label_fixture() -> (tempfile::TempDir, HostedState, Auth, String, String) {
        let (temp, state, auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let source_path = root.join("Shared/Shared.typ");
        let owner = inner
            .core
            .qualified_node_at(&root, &source_path)
            .unwrap()
            .owner;
        let op = uuid::Uuid::new_v4().to_string();
        let plan = inner
            .core
            .prepare_import_resource(
                &root,
                &source_path,
                "labels.typ",
                b"= Only input <input-only>".to_vec(),
                &op,
            )
            .unwrap();
        let reference = plan.resource_map.values().next().unwrap().clone();
        inner.core.execute_operation(&op, &plan.plan_id).unwrap();
        inner.config.resources.insert(
            "LabelInput".into(),
            media::Resource {
                owner: "Shared".into(),
                expected_owner: owner,
                path: "Shared/labels.typ".into(),
                title: "批准输入".into(),
                reference,
                lifecycle: "live".into(),
            },
        );
        inner.config.files.get_mut("Shared").unwrap().inputs = vec!["LabelInput".into()];
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resources
            .insert(
                "LabelInput".into(),
                media::Rights {
                    read: true,
                    download: false,
                },
            );
        let source = format!(
            "{}\n#include \"labels.typ\"\n= 中文😀 <main-label>\n#if false [= Not emitted <unevaluated>]\n",
            doc["source"].as_str().unwrap()
        );
        drop(inner);
        (
            temp,
            state,
            auth,
            doc["sessionId"].as_str().unwrap().into(),
            source,
        )
    }
    #[cfg(windows)]
    #[test]
    fn actual_native_labels_do_not_promote_foreign_origins_and_recheck_original_inputs() {
        let (_temp, state, auth, session_id, source) = label_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let owner = inner
            .core
            .qualified_node_at(&root, &root.join("Shared/Shared.typ"))
            .unwrap();
        let req = Fragment {
            session_id: session_id.clone(),
            source: source.clone(),
            source_hash: digest(source.as_bytes()),
            serial: 10,
            reference: format!("{}#main-label", uri(&owner.owner)),
        };
        writing_api::observe_version(&mut inner.editor_versions, &session_id, &source, 10).unwrap();
        let mut capture = prepare_compile(
            &mut inner,
            "alice",
            "Shared",
            source.clone(),
            "main-label".into(),
            owner,
            Some(req),
        )
        .unwrap();
        let worker = capture.worker.take().unwrap();
        let files = worker
            .frozen_files
            .unwrap()
            .into_iter()
            .map(|(p, b)| {
                (
                    p,
                    base64::engine::general_purpose::STANDARD.decode(b).unwrap(),
                )
            })
            .collect();
        let output = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render_frozen(&weftext_engine::FrozenRenderRequest {
                source: worker.source,
                path: worker.path,
                root: worker.root,
                style: worker.style,
                files,
            })
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        let output = serde_json::to_value(output).unwrap();
        assert_eq!(
            compiled_location(&source, "main-label", "Shared", "draft", Some(&output))["status"],
            "found"
        );
        let foreign = compiled_location(&source, "input-only", "Shared", "draft", Some(&output));
        assert_eq!(foreign["status"], "unavailable");
        assert!(!foreign.to_string().contains("labels.typ"));
        assert_eq!(
            compiled_location(&source, "unevaluated", "Shared", "draft", Some(&output))["status"],
            "missing"
        );
        recheck_compile(&mut inner, &auth, &capture).unwrap();
        std::fs::write(
            root.join("Shared/labels.typ"),
            b"= Late change <main-label>",
        )
        .unwrap();
        assert_eq!(
            recheck_compile(&mut inner, &auth, &capture)
                .unwrap_err()
                .status,
            StatusCode::CONFLICT
        );
    }
    #[cfg(windows)]
    #[test]
    fn ungranted_inputs_never_compile_and_late_draft_aba_never_delivers_old_ranges() {
        let (_temp, state, auth, session_id, source) = label_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resources
            .clear();
        let owner = inner
            .core
            .qualified_node_at(&root, &root.join("Shared/Shared.typ"))
            .unwrap();
        let denied = prepare_compile(
            &mut inner,
            "alice",
            "Shared",
            source.clone(),
            "input-only".into(),
            owner,
            None,
        )
        .unwrap();
        assert!(denied.worker.is_none());
        assert_eq!(
            compiled_location(&source, "input-only", "Shared", "saved", None)["status"],
            "unavailable"
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resources
            .insert(
                "LabelInput".into(),
                media::Rights {
                    read: true,
                    download: false,
                },
            );
        let owner = inner
            .core
            .qualified_node_at(&root, &root.join("Shared/Shared.typ"))
            .unwrap();
        let req = Fragment {
            session_id: session_id.clone(),
            source: source.clone(),
            source_hash: digest(source.as_bytes()),
            serial: 4,
            reference: format!("{}#main-label", uri(&owner.owner)),
        };
        writing_api::observe_version(&mut inner.editor_versions, &session_id, &source, 4).unwrap();
        let capture = prepare_compile(
            &mut inner,
            "alice",
            "Shared",
            source.clone(),
            "main-label".into(),
            owner,
            Some(req),
        )
        .unwrap();
        writing_api::observe_version(
            &mut inner.editor_versions,
            &session_id,
            &format!("{source}\nChanged"),
            5,
        )
        .unwrap();
        writing_api::observe_version(&mut inner.editor_versions, &session_id, &source, 6).unwrap();
        assert_eq!(
            recheck_compile(&mut inner, &auth, &capture)
                .unwrap_err()
                .status,
            StatusCode::CONFLICT
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn readonly_ref_and_resolution_need_no_history_and_never_probe_hidden_sources() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_temp, state, auth, doc, _version, _original) =
            super::super::history_api::test_fixture();
        let reader_doc = state
            .inner
            .lock()
            .unwrap()
            .open("reader", "Shared")
            .unwrap();
        let reader = Auth {
            digest: digest(b"reader"),
        };
        let Json(captured) = reference(
            State(state.clone()),
            Extension(reader.clone()),
            Json(Session {
                session_id: reader_doc["sessionId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(captured["resource"], "Shared");
        assert!(captured.get("source").is_none());
        let Json(resolved) = resolve(
            State(state.clone()),
            Extension(reader),
            Json(Resolve {
                reference: Reference::Uri(captured["reference"].as_str().unwrap().into()),
            }),
        )
        .await
        .unwrap();
        assert_eq!(resolved["resource"], "Shared");
        assert_eq!(resolved["scope"], "configuredAuthorizedSources");
        assert_eq!(resolved["coverage"], "complete");
        let (path, hidden) = {
            let inner = state.inner.lock().unwrap();
            let path = inner.config.root.join("Target/Target.typ");
            let owner = inner
                .core
                .qualified_node_at(&inner.config.root, &path)
                .unwrap()
                .owner;
            (path, owner)
        };
        let _locked_hidden = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .unwrap();
        let mut unknown = hidden.clone();
        unknown.node_id = uuid::Uuid::new_v4().to_string();
        let mut refusal = None;
        for node in [hidden, unknown] {
            let error = resolve(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Resolve {
                    reference: Reference::Node(node),
                }),
            )
            .await
            .unwrap_err();
            assert_eq!(error.status, StatusCode::NOT_FOUND);
            let current = (error.status, error.code, error.message);
            assert_eq!(refusal.get_or_insert(current.clone()), &current);
        }
        // Even a locked configured hidden source cannot obstruct capture of the
        // independently authorized original owner or trigger inventory reads.
        let _ = reference(
            State(state),
            Extension(auth),
            Json(Session {
                session_id: doc["sessionId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn navigation_rejects_old_session_remap_and_ambiguous_configured_aliases() {
        let (_temp, state, auth, doc, _version, _original) =
            super::super::history_api::test_fixture();
        let Json(captured) = reference(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Session {
                session_id: doc["sessionId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
        {
            let mut inner = state.inner.lock().unwrap();
            let file = inner.config.files["Shared"].clone();
            let rights = inner.policy.users["alice"].grants["Shared"].clone();
            inner.config.files.insert("Alias".into(), file);
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .grants
                .insert("Alias".into(), rights);
        }
        assert_eq!(
            resolve(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Resolve {
                    reference: Reference::Uri(captured["reference"].as_str().unwrap().into())
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
        state
            .inner
            .lock()
            .unwrap()
            .config
            .files
            .get_mut("Shared")
            .unwrap()
            .path = "Target/Target.typ".into();
        assert_eq!(
            reference(
                State(state),
                Extension(auth),
                Json(Session {
                    session_id: doc["sessionId"].as_str().unwrap().into()
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
    }
    #[cfg(windows)]
    #[test]
    fn final_qualification_rechecks_rules_and_original_source_witness() {
        let (_temp, state, auth, doc, _version, _original) =
            super::super::history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let path = root.join("Shared/Shared.typ");
        let qualified = inner.core.qualified_node_at(&root, &path).unwrap();
        std::fs::write(
            root.join(".weftext-rules"),
            br#"{"version":1,"ignored":["Shared"],"unmanaged":[]}"#,
        )
        .unwrap();
        assert_eq!(
            final_owner(&mut inner, &auth, "Shared", &qualified, None)
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            final_owner(
                &mut inner,
                &auth,
                "Shared",
                &qualified,
                Some(doc["sessionId"].as_str().unwrap())
            )
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
        std::fs::write(
            root.join(".weftext-rules"),
            br#"{"version":1,"ignored":[],"unmanaged":[]}"#,
        )
        .unwrap();
        std::fs::write(
            &path,
            doc["source"].as_str().unwrap().to_string() + "\nExternal new source\n",
        )
        .unwrap();
        assert_eq!(
            final_owner(&mut inner, &auth, "Shared", &qualified, None)
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            final_owner(
                &mut inner,
                &auth,
                "Shared",
                &qualified,
                Some(doc["sessionId"].as_str().unwrap())
            )
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
    }
}
