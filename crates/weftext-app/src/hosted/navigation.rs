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
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/navigation/ref", post(reference))
        .route("/api/hosted/navigation/resolve", post(resolve))
}
fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .is_ok_and(|id| id.to_string() == value && id.get_version() == Some(uuid::Version::Random))
}
fn parse_reference(reference: Reference) -> Result<NodeRef, ApiError> {
    let node = match reference {
        Reference::Node(node) => node,
        Reference::Uri(uri) => {
            let (workspace, node) = uri
                .strip_prefix("weftext-node://")
                .and_then(|tail| tail.split_once('/'))
                .ok_or_else(|| ApiError::bad("请使用完整的稳定笔记引用"))?;
            NodeRef {
                workspace_id: workspace.into(),
                node_id: node.into(),
            }
        }
    };
    if !valid_uuid(&node.workspace_id) || !valid_uuid(&node.node_id) {
        return Err(ApiError::bad("请使用完整的稳定笔记引用"));
    }
    Ok(node)
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
    let file = inner
        .config
        .files
        .get(resource)
        .filter(|f| f.editable)
        .ok_or_else(unavailable)?;
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
        let file = inner.config.files.get(&session.resource).filter(|f| f.editable).ok_or_else(unavailable)?;
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
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let candidates = inner.config.files.iter().filter(|(r, f)| f.editable && inner.rights(&actor, r).is_ok()).map(|(r, f)| (r.clone(), f.clone())).collect::<Vec<_>>();
        let mut partial = false;
        let mut found = None;
        for (resource, file) in candidates {
            let path = inner.config.root.join(&file.path);
            match inner.core.qualified_node_at(&inner.config.root, &path) {
                Ok(qualified) if qualified.owner == target => {
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
        Ok(json!({"resource":resource,"title":file.title,"reference":uri(&target),"nodeRef":target,"canOpen":true,
            "scope":"configuredAuthorizedSources","coverage":if partial { "partial" } else { "complete" },"readOnly":true}))
    }).await?))
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
            parse_reference(Reference::Uri(uri(&target))).unwrap(),
            target
        );
        assert_eq!(
            parse_reference(Reference::Node(target.clone())).unwrap(),
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
