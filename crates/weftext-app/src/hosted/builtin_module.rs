//! Fixed endnote distribution installed through the existing member-upload
//! transaction. No author save, permission grant or arbitrary byte delivery.
use super::*;
use crate::builtin_module_api::{
    ModuleDistribution, ModuleIdentity, ModuleResource, validate_distribution,
};
use axum::extract::DefaultBodyLimit;
use weftext_knowledge::NodeRef;

const NAME: &str = "weftext-endnotes.typ";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    session_id: String,
    identity: Option<ModuleIdentity>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Read {
    session_id: String,
    identity: ModuleIdentity,
    resource_ref: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    session_id: String,
    operation_id: String,
    identity: ModuleIdentity,
    module: ModuleDistribution,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    session_id: String,
    operation_id: String,
    identity: ModuleIdentity,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    session_id: String,
    operation_id: String,
    plan_id: String,
    identity: ModuleIdentity,
}
struct Selected {
    actor: String,
    resource: String,
    collection: String,
    owner: NodeRef,
    identity: ModuleIdentity,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/editor/endnotes-module/catalog", post(catalog))
        .route("/api/hosted/editor/endnotes-module/read", post(read))
        .route(
            "/api/hosted/editor/endnotes-module/authorized",
            post(authorized),
        )
        .route("/api/hosted/editor/endnotes-module/prepare", post(prepare))
        .route("/api/hosted/editor/endnotes-module/commit", post(commit))
        .route("/api/hosted/editor/endnotes-module/status", post(status))
        .route("/api/hosted/editor/endnotes-module/settle", post(settle))
        .layer(DefaultBodyLimit::max(512 * 1024))
}

fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "endnoteModuleChanged",
        "尾注模块的原归属、集合、准确资源或授权依据已变化；原件与原操作保留，未改正文。",
    )
}
fn absent() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "endnoteModuleUnavailable",
        "当前没有本篇获权的尾注模块集合或执行依赖；须由所有者配置资源读取、导入和编译输入。",
    )
}
fn owner_uri(owner: &NodeRef) -> String {
    format!("weftext-node://{}/{}", owner.workspace_id, owner.node_id)
}
fn same_identity(a: &ModuleIdentity, b: &ModuleIdentity) -> bool {
    a.scope == b.scope
        && a.owner == b.owner
        && a.binding == b.binding
        && a.module_path == b.module_path
}
fn identity(
    inner: &HostedInner,
    actor: &str,
    collection: &str,
    owner: &NodeRef,
) -> Result<ModuleIdentity, ApiError> {
    let scope = digest(
        &serde_json::to_vec(&json!({
            "actor":inner.actor_scope(actor),"epoch":inner.actor_generation(actor)
        }))
        .map_err(|_| changed())?,
    );
    let binding = digest(
        &serde_json::to_vec(&json!({
            "collection":collection,"owner":owner,
            "binding":media::binding_hash(inner,&BTreeSet::from([format!("{collection}/*")]))?
        }))
        .map_err(|_| changed())?,
    );
    Ok(ModuleIdentity {
        scope,
        owner: owner_uri(owner),
        binding,
        module_path: NAME.into(),
    })
}
fn expected_path(inner: &HostedInner, selected: &Selected) -> Result<PathBuf, ApiError> {
    let file = inner.file(&selected.resource)?;
    Ok(std::path::Path::new(&file.path)
        .parent()
        .ok_or_else(changed)?
        .join(NAME))
}
fn future_input_selected(inner: &HostedInner, selected: &Selected, inputs: &[String]) -> bool {
    inputs.contains(&format!("{}/*", selected.collection))
        || content_import::split_member(&inner.config, &selected.collection)
            .is_some_and(|(slot, _)| inputs.contains(&format!("{slot}/*")))
}

/// Freeze the actual current compiler selection. Resource read/import do not
/// manufacture an input grant, and a path alone never proves selected identity.
fn compiler_authorized(
    inner: &mut HostedInner,
    selected: &Selected,
    reference: Option<&str>,
) -> Result<bool, ApiError> {
    let inputs = inner.file(&selected.resource)?.inputs.clone();
    if reference.is_none() && !future_input_selected(inner, selected, &inputs) {
        return Ok(false);
    }
    let captured = match inputs::capture(inner, &selected.actor, &inputs) {
        Ok(captured) => captured,
        Err(_) => return Ok(false),
    };
    captured.validate(inner, &selected.actor)?;
    let Some(reference) = reference else {
        return Ok(true);
    };
    let path = expected_path(inner, selected)?;
    for id in captured.ids() {
        if !media::is_asset(&inner.config, id) {
            continue;
        }
        let actual = media::resolve_asset(inner, &selected.actor, id)?;
        if actual.reference == reference
            && actual.expected_owner == selected.owner
            && actual.owner == selected.resource
            && std::path::Path::new(&actual.path) == path
            && captured.entry(id)?.is_some_and(|(_, _, accepted)| accepted)
        {
            captured.validate(inner, &selected.actor)?;
            return Ok(true);
        }
    }
    Ok(false)
}

fn module_entries(
    inner: &HostedInner,
    selected: &Selected,
) -> Result<(bool, Vec<ModuleResource>), ApiError> {
    let captured = resource_collections::capture(inner, &selected.actor, &selected.collection)?;
    if captured.owner != selected.owner {
        return Err(changed());
    }
    let resources = captured
        .entries
        .iter()
        .filter(|entry| entry.relative_path == NAME)
        .map(|entry| ModuleResource {
            resource_ref: entry.resource_ref.clone(),
            owner: owner_uri(&captured.owner),
            module_path: NAME.into(),
        })
        .collect::<Vec<_>>();
    if resources.len() > 1 {
        return Err(changed());
    }
    inner
        .core
        .validate_resource_collection_witness(&inner.config.root, &captured.witness)
        .map_err(|_| changed())?;
    Ok((captured.unrecognized_entries == 0, resources))
}

fn select(
    inner: &mut HostedInner,
    auth: &Auth,
    session_id: &str,
    original: Option<&ModuleIdentity>,
) -> Result<Selected, ApiError> {
    let (actor, session, _) = inner.session(auth, session_id)?;
    if original.is_some_and(|identity| identity.module_path != NAME) {
        return Err(changed());
    }
    let file = inner.file(&session.resource)?.clone();
    let root = inner.config.root.clone();
    let actor_scope = inner.actor_scope(&actor);
    let owner = inner
        .core
        .with_actor_for_root(&actor_scope, &root, |core| {
            core.qualified_node_for_session_at(&root, &root.join(&file.path), session_id)
        })
        .map_err(|_| changed())?
        .owner;
    let mut candidates = inner
        .config
        .resource_collections
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if content_import::split_member(&inner.config, &session.resource).is_some() {
        candidates.insert(session.resource.clone());
    }
    for collection in candidates {
        let Ok(config) = resource_collections::configured(inner, &collection) else {
            continue;
        };
        if config.owner != session.resource
            || config.lifecycle != "live"
            || resource_collections::input_access(inner, &actor, &collection, false).is_err()
        {
            continue;
        }
        if resource_collections::effective_owner(inner, &collection)? != owner {
            return Err(changed());
        }
        let identity = identity(inner, &actor, &collection, &owner)?;
        if original.is_some_and(|original| !same_identity(original, &identity)) {
            continue;
        }
        let selected = Selected {
            actor: actor.clone(),
            resource: session.resource.clone(),
            collection,
            owner: owner.clone(),
            identity,
        };
        if original.is_some() {
            return Ok(selected);
        }
        let (complete, entries) = module_entries(inner, &selected)?;
        if entries
            .first()
            .is_some_and(|entry| !entry.resource_ref.is_empty())
        {
            if compiler_authorized(inner, &selected, Some(&entries[0].resource_ref))? {
                return Ok(selected);
            }
        } else if complete
            && resource_collections::import_access(inner, &actor, &selected.collection).is_ok()
            && compiler_authorized(inner, &selected, None)?
        {
            return Ok(selected);
        }
    }
    Err(if original.is_some() {
        changed()
    } else {
        absent()
    })
}

fn qualified_module(
    inner: &HostedInner,
    selected: &Selected,
    reference: &str,
) -> Result<weftext_core::QualifiedResource, ApiError> {
    let (_, entries) = module_entries(inner, selected)?;
    let entry = entries
        .iter()
        .find(|entry| entry.resource_ref == reference)
        .ok_or_else(absent)?;
    let member = entry.resource_ref.rsplit('/').next().ok_or_else(changed)?;
    let asset = format!("{}/{member}", selected.collection);
    let resource = media::capture_input(inner, &selected.actor, &asset)?;
    if resource.owner != selected.owner
        || resource.resource_ref != reference
        || resource.revision.is_none()
    {
        return Err(changed());
    }
    if resource.bytes() != writing_api::ENDNOTES_MODULE.as_bytes() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "endnoteModuleConflict",
            "本篇同名模块已有不同内容；原资源保留，未覆盖或交付其它资源字节。",
        ));
    }
    media::validate_original(inner, &selected.actor, &asset, &resource)?;
    Ok(resource)
}
fn receipt(value: Value) -> Value {
    json!({"operationId":value["operationId"],"status":value["status"],
        "planId":value["planId"],"commitAllowed":value["commitAllowed"],
        "resourceRefs":value["resourceRefs"],"losses":[]})
}
fn current_receipt(
    inner: &mut HostedInner,
    selected: &Selected,
    value: Value,
) -> Result<Value, ApiError> {
    let mut result = receipt(value);
    if result["status"] == "prepared"
        && result["commitAllowed"] == true
        && !compiler_authorized(inner, selected, None)?
    {
        // Retain the original plan for recovery, but never present an import
        // as executable after its separate compiler-input grant was removed.
        result["commitAllowed"] = json!(false);
    }
    Ok(result)
}
fn original_status(
    inner: &mut HostedInner,
    selected: &Selected,
    operation_id: &str,
) -> Result<Value, ApiError> {
    resource_collections::member_import_status_matching(
        inner,
        &selected.actor,
        &selected.collection,
        operation_id,
        NAME,
        writing_api::ENDNOTES_MODULE.as_bytes(),
    )
}

async fn catalog(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Catalog>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let selected = select(inner, &auth, &req.session_id, req.identity.as_ref())?;
        let (complete, resources) = module_entries(inner, &selected)?;
        Ok(json!({"identity":selected.identity,"coverage":if complete {"complete"} else {"partial"},
            "owner":owner_uri(&selected.owner),"resources":resources}))
    }).await?))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Read>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let selected = select(inner, &auth, &req.session_id, Some(&req.identity))?;
            let resource = qualified_module(inner, &selected, &req.resource_ref)?;
            Ok(
                json!({"resourceRef":resource.resource_ref,"source":writing_api::ENDNOTES_MODULE,
            "sha256":weftext_engine::source_hash(writing_api::ENDNOTES_MODULE)}),
            )
        })
        .await?,
    ))
}
async fn authorized(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Read>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let selected = select(inner, &auth, &req.session_id, Some(&req.identity))?;
            let resource = qualified_module(inner, &selected, &req.resource_ref)?;
            let allowed = compiler_authorized(inner, &selected, Some(&req.resource_ref))?;
            inner
                .core
                .validate_resource_witness(&inner.config.root, &resource.witness)
                .map_err(|_| changed())?;
            Ok(json!({"authorized":allowed}))
        })
        .await?,
    ))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let selected = select(inner, &auth, &req.session_id, Some(&req.identity))?;
            validate_distribution(&req.module).map_err(|_| {
                ApiError::bad("仅可安装当前产品随附的准确尾注模块；未采用替代字节或版本")
            })?;
            let old = original_status(inner, &selected, &req.operation_id)?;
            if old["status"] == "notRegistered" {
                let (complete, existing) = module_entries(inner, &selected)?;
                if !complete || !existing.is_empty() {
                    return Err(changed());
                }
                if !compiler_authorized(inner, &selected, None)? {
                    return Err(absent());
                }
            }
            let original = resource_collections::prepare_member_import(
                inner,
                &selected.actor,
                &selected.collection,
                NAME,
                writing_api::ENDNOTES_MODULE.as_bytes().to_vec(),
                &req.operation_id,
            )?;
            current_receipt(inner, &selected, original)
        })
        .await?,
    ))
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Commit>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let selected = select(inner, &auth, &req.session_id, Some(&req.identity))?;
            let old = original_status(inner, &selected, &req.operation_id)?;
            // The existing upload decision still owns exact bytes, TTL, owner
            // identity, manifest and destination absence at execution time.
            if old["status"] == "prepared" && !compiler_authorized(inner, &selected, None)? {
                return Err(absent());
            }
            Ok(receipt(resource_collections::commit_member_import(
                inner,
                &selected.actor,
                &selected.collection,
                &req.operation_id,
                &req.plan_id,
            )?))
        })
        .await?,
    ))
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let selected = select(inner, &auth, &req.session_id, Some(&req.identity))?;
            let original = original_status(inner, &selected, &req.operation_id)?;
            current_receipt(inner, &selected, original)
        })
        .await?,
    ))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let selected = select(inner, &auth, &req.session_id, Some(&req.identity))?;
            original_status(inner, &selected, &req.operation_id)?;
            Ok(receipt(resource_collections::settle_member_import(
                inner,
                &selected.actor,
                &selected.collection,
                &req.operation_id,
            )?))
        })
        .await?,
    ))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn fixture(compile: bool) -> (tempfile::TempDir, HostedState, Auth, String) {
        let (temp, state, auth, _) = media::tests::fixture();
        let mut inner = state.inner.lock().unwrap();
        let owner = inner.config.resources["Diagram"].expected_owner.clone();
        inner.config.resource_collections.insert(
            "Owned".into(),
            resource_collections::Collection {
                owner: "Shared".into(),
                expected_owner: Some(owner),
                from_import_slot: false,
                from_structure_slot: false,
                lifecycle: "live".into(),
                title: "本篇资源".into(),
            },
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resource_collections
            .insert(
                "Owned".into(),
                resource_collections::Rights {
                    read: true,
                    download: false,
                    r#import: true,
                },
            );
        let rights = inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap();
        rights.read = true;
        rights.write = true;
        inner.config.files.get_mut("Shared").unwrap().inputs = if compile {
            vec!["Owned/*".into()]
        } else {
            vec![]
        };
        let opened = inner.open("alice", "Shared").unwrap();
        let session = opened["sessionId"].as_str().unwrap().to_owned();
        drop(inner);
        (temp, state, auth, session)
    }
    fn module() -> ModuleDistribution {
        serde_json::from_value(json!({"name":NAME,"source":writing_api::ENDNOTES_MODULE,
            "sha256":weftext_engine::source_hash(writing_api::ENDNOTES_MODULE),"version":"1"}))
        .unwrap()
    }
    async fn list(state: &HostedState, auth: &Auth, session: &str) -> Result<Value, ApiError> {
        catalog(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Catalog {
                session_id: session.into(),
                identity: None,
            }),
        )
        .await
        .map(|r| r.0)
    }
    fn identity_from(value: &Value) -> ModuleIdentity {
        serde_json::from_value(value["identity"].clone()).unwrap()
    }
    async fn propose(
        state: &HostedState,
        auth: &Auth,
        session: &str,
        catalog: &Value,
        operation_id: &str,
    ) -> Result<Value, ApiError> {
        prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Prepare {
                session_id: session.into(),
                operation_id: operation_id.into(),
                identity: identity_from(catalog),
                module: module(),
            }),
        )
        .await
        .map(|r| r.0)
    }
    #[tokio::test]
    async fn catalog_needs_separate_resource_import_read_and_actual_compiler_selection() {
        let (_temp, state, auth, session) = fixture(false);
        assert_eq!(
            list(&state, &auth, &session).await.unwrap_err().status,
            StatusCode::NOT_FOUND
        );
        {
            let mut inner = state.inner.lock().unwrap();
            inner.config.files.get_mut("Shared").unwrap().inputs = vec!["Owned/*".into()];
        }
        let catalog = list(&state, &auth, &session).await.unwrap();
        assert!(
            catalog["identity"]["owner"]
                .as_str()
                .unwrap()
                .starts_with("weftext-node://")
        );
        assert_eq!(catalog["coverage"], "complete");
        assert_eq!(catalog["resources"], json!([]));
        assert!(!catalog.to_string().contains("private-locator"));
        {
            let mut inner = state.inner.lock().unwrap();
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .resource_collections
                .get_mut("Owned")
                .unwrap()
                .read = false;
        }
        assert_eq!(
            list(&state, &auth, &session).await.unwrap_err().status,
            StatusCode::NOT_FOUND
        );
    }
    #[tokio::test]
    async fn compiler_revocation_retains_original_plan_but_disables_commit_until_restored() {
        let (_temp, state, auth, session) = fixture(true);
        {
            let mut inner = state.inner.lock().unwrap();
            inner.config.files.get_mut("Shared").unwrap().inputs =
                vec!["Owned/*".into(), "Diagram".into()];
        }
        let catalog = list(&state, &auth, &session).await.unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let original = propose(&state, &auth, &session, &catalog, &id)
            .await
            .unwrap();
        let plan_id = original["planId"].as_str().unwrap().to_owned();
        let author_source = {
            let mut inner = state.inner.lock().unwrap();
            // A separate selected input can lose read authority without
            // changing this module collection's owner or import permission.
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .resources
                .get_mut("Diagram")
                .unwrap()
                .read = false;
            std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap()
        };
        let current = status(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                session_id: session.clone(),
                operation_id: id.clone(),
                identity: identity_from(&catalog),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(current["status"], "prepared");
        assert_eq!(current["planId"], original["planId"]);
        assert_eq!(current["resourceRefs"], original["resourceRefs"]);
        assert_eq!(current["commitAllowed"], false);
        assert_eq!(
            propose(&state, &auth, &session, &catalog, &id)
                .await
                .unwrap(),
            current
        );
        assert_eq!(
            commit(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Commit {
                    session_id: session.clone(),
                    operation_id: id.clone(),
                    identity: identity_from(&catalog),
                    plan_id,
                }),
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
        {
            let mut inner = state.inner.lock().unwrap();
            assert_eq!(
                std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap(),
                author_source
            );
            assert!(!inner.config.root.join("Shared").join(NAME).exists());
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .resources
                .get_mut("Diagram")
                .unwrap()
                .read = true;
        }
        assert_eq!(
            propose(&state, &auth, &session, &catalog, &id)
                .await
                .unwrap(),
            original
        );
    }
    #[tokio::test]
    async fn original_member_transaction_installs_only_fixed_bytes_and_reuses_actual_identity() {
        let (temp, state, auth, session) = fixture(true);
        let catalog = list(&state, &auth, &session).await.unwrap();
        let original = std::fs::read(
            state
                .inner
                .lock()
                .unwrap()
                .config
                .root
                .join("Shared/Shared.typ"),
        )
        .unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let plan = propose(&state, &auth, &session, &catalog, &id)
            .await
            .unwrap();
        assert_eq!(plan["status"], "prepared");
        assert_eq!(plan["commitAllowed"], true);
        assert_eq!(
            propose(&state, &auth, &session, &catalog, &id)
                .await
                .unwrap(),
            plan
        );
        let mut wrong = module();
        wrong.source.push('!');
        assert_eq!(
            prepare(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Prepare {
                    session_id: session.clone(),
                    operation_id: id.clone(),
                    identity: identity_from(&catalog),
                    module: wrong
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::BAD_REQUEST
        );
        // Reload the same Core ledger, retaining the original hosted scope.
        let session = {
            let mut inner = state.inner.lock().unwrap();
            let placeholder = Core::new(temp.path().join("placeholder")).unwrap();
            drop(std::mem::replace(&mut inner.core, placeholder));
            inner.core = Core::new(temp.path().join("state")).unwrap();
            let opened = inner.open("alice", "Shared").unwrap();
            opened["sessionId"].as_str().unwrap().to_owned()
        };
        let saved = status(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                session_id: session.clone(),
                operation_id: id.clone(),
                identity: identity_from(&catalog),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(saved, plan);
        let committed = commit(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Commit {
                session_id: session.clone(),
                operation_id: id.clone(),
                identity: identity_from(&catalog),
                plan_id: plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(committed["status"], "committed");
        let after = list(&state, &auth, &session).await.unwrap();
        assert_eq!(after["identity"], catalog["identity"]);
        let reference = after["resources"][0]["resourceRef"].as_str().unwrap();
        assert_eq!(committed["resourceRefs"][0], reference);
        let output = read(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Read {
                session_id: session.clone(),
                identity: identity_from(&after),
                resource_ref: reference.into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(output["source"], writing_api::ENDNOTES_MODULE);
        assert_eq!(
            authorized(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Read {
                    session_id: session.clone(),
                    identity: identity_from(&after),
                    resource_ref: reference.into()
                })
            )
            .await
            .unwrap()
            .0["authorized"],
            true
        );
        {
            let inner = state.inner.lock().unwrap();
            assert_eq!(
                std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap(),
                original
            );
            assert!(!inner.policy.users["alice"].resource_collections["Owned"].download);
            assert_eq!(inner.config.files["Shared"].inputs, vec!["Owned/*"]);
        }
        // Reusing existing admitted bytes requires read and compiler input,
        // independently of whether this actor may create another resource.
        {
            let mut inner = state.inner.lock().unwrap();
            inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .resource_collections
                .get_mut("Owned")
                .unwrap()
                .r#import = false;
        }
        let readonly = list(&state, &auth, &session).await.unwrap();
        assert_eq!(readonly["resources"][0]["resourceRef"], reference);
        assert_eq!(
            authorized(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Read {
                    session_id: session,
                    identity: identity_from(&readonly),
                    resource_ref: reference.into()
                })
            )
            .await
            .unwrap()
            .0["authorized"],
            true
        );
    }
    #[tokio::test]
    async fn same_name_conflict_and_settled_unknown_never_overwrite_or_install() {
        let (_temp, state, auth, session) = fixture(true);
        let catalog = list(&state, &auth, &session).await.unwrap();
        let settled_id = uuid::Uuid::new_v4().to_string();
        let settled = settle(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                session_id: session.clone(),
                operation_id: settled_id.clone(),
                identity: identity_from(&catalog),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(settled["status"], "cancelled");
        assert_eq!(
            propose(&state, &auth, &session, &catalog, &settled_id)
                .await
                .unwrap()["status"],
            "cancelled"
        );
        {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let owner = root.join("Shared/Shared.typ");
            let op = uuid::Uuid::new_v4().to_string();
            let plan = inner
                .core
                .prepare_import_resource(&root, &owner, NAME, b"USER_ORIGINAL".to_vec(), &op)
                .unwrap();
            inner.core.execute_operation(&op, &plan.plan_id).unwrap();
        }
        let current = list(&state, &auth, &session).await.unwrap();
        let reference = current["resources"][0]["resourceRef"].as_str().unwrap();
        assert_eq!(
            read(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Read {
                    session_id: session.clone(),
                    identity: identity_from(&current),
                    resource_ref: reference.into()
                })
            )
            .await
            .unwrap_err()
            .code,
            "endnoteModuleConflict"
        );
        assert_eq!(
            propose(
                &state,
                &auth,
                &session,
                &current,
                &uuid::Uuid::new_v4().to_string()
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::CONFLICT
        );
        let inner = state.inner.lock().unwrap();
        assert_eq!(
            std::fs::read(inner.config.root.join("Shared/weftext-endnotes.typ")).unwrap(),
            b"USER_ORIGINAL"
        );
    }
    #[tokio::test]
    async fn builtin_status_commit_and_settle_cannot_claim_an_unrelated_member_upload() {
        let (_temp, state, auth, session) = fixture(true);
        let catalog = list(&state, &auth, &session).await.unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let plan = {
            let mut inner = state.inner.lock().unwrap();
            resource_collections::prepare_member_import(
                &mut inner,
                "alice",
                "Owned",
                "ordinary.bin",
                b"OTHER_ORIGINAL".to_vec(),
                &id,
            )
            .unwrap()
        };
        assert_eq!(
            status(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Operation {
                    session_id: session.clone(),
                    operation_id: id.clone(),
                    identity: identity_from(&catalog)
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::CONFLICT
        );
        assert_eq!(
            settle(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Operation {
                    session_id: session.clone(),
                    operation_id: id.clone(),
                    identity: identity_from(&catalog)
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::CONFLICT
        );
        assert_eq!(
            commit(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Commit {
                    session_id: session,
                    operation_id: id.clone(),
                    identity: identity_from(&catalog),
                    plan_id: plan["planId"].as_str().unwrap().into()
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::CONFLICT
        );
        let mut inner = state.inner.lock().unwrap();
        assert_eq!(
            resource_collections::member_import_status(&mut inner, "alice", "Owned", &id).unwrap()
                ["status"],
            "prepared"
        );
        assert!(!inner.config.root.join("Shared/ordinary.bin").exists());
        assert!(
            !inner
                .config
                .root
                .join("Shared/weftext-endnotes.typ")
                .exists()
        );
    }
    #[tokio::test]
    async fn committed_module_cannot_be_rebound_through_redacted_generic_status() {
        for remap_owner in [false, true] {
            let (_temp, state, auth, session) = fixture(true);
            let catalog = list(&state, &auth, &session).await.unwrap();
            let id = uuid::Uuid::new_v4().to_string();
            let plan = propose(&state, &auth, &session, &catalog, &id)
                .await
                .unwrap();
            let plan_id = plan["planId"].as_str().unwrap().to_owned();
            let committed = commit(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Commit {
                    session_id: session,
                    operation_id: id.clone(),
                    plan_id: plan_id.clone(),
                    identity: identity_from(&catalog),
                }),
            )
            .await
            .unwrap()
            .0;
            assert_eq!(committed["status"], "committed");
            let (new_session, originals) = {
                let mut inner = state.inner.lock().unwrap();
                let root = inner.config.root.clone();
                let originals = ["Shared/Shared.typ", "Target/Target.typ"]
                    .map(|path| (path, std::fs::read(root.join(path)).unwrap()));
                let current = if remap_owner {
                    let owner = inner
                        .core
                        .qualified_node_at(&root, &root.join("Target/Target.typ"))
                        .unwrap()
                        .owner;
                    let config = inner.config.resource_collections.get_mut("Owned").unwrap();
                    config.owner = "Target".into();
                    config.expected_owner = Some(owner);
                    inner.config.files.get_mut("Target").unwrap().inputs = vec!["Owned/*".into()];
                    let rights = inner.policy.users["alice"].grants["Shared"].clone();
                    inner
                        .policy
                        .users
                        .get_mut("alice")
                        .unwrap()
                        .grants
                        .insert("Target".into(), rights);
                    let new_id = uuid::Uuid::new_v4().to_string();
                    let new_plan = inner
                        .core
                        .prepare_import_resource(
                            &root,
                            root.join("Target/Target.typ"),
                            NAME,
                            writing_api::ENDNOTES_MODULE.as_bytes().to_vec(),
                            &new_id,
                        )
                        .unwrap();
                    inner
                        .core
                        .execute_operation(&new_id, &new_plan.plan_id)
                        .unwrap();
                    "Target"
                } else {
                    inner.policy.users.get_mut("alice").unwrap().epoch += 1;
                    "Shared"
                };
                let projection =
                    resource_collections::member_import_status(&mut inner, "alice", "Owned", &id)
                        .unwrap();
                assert_eq!(projection["status"], "committed");
                assert_eq!(projection["resourceRefs"], json!([]));
                let opened = inner.open("alice", current).unwrap();
                (opened["sessionId"].as_str().unwrap().to_owned(), originals)
            };
            let current = list(&state, &auth, &new_session).await.unwrap();
            assert_ne!(current["identity"], catalog["identity"]);
            assert_eq!(current["resources"].as_array().unwrap().len(), 1);
            for result in [
                status(
                    State(state.clone()),
                    Extension(auth.clone()),
                    Json(Operation {
                        session_id: new_session.clone(),
                        operation_id: id.clone(),
                        identity: identity_from(&current),
                    }),
                )
                .await,
                commit(
                    State(state.clone()),
                    Extension(auth.clone()),
                    Json(Commit {
                        session_id: new_session,
                        operation_id: id,
                        plan_id,
                        identity: identity_from(&current),
                    }),
                )
                .await,
            ] {
                assert_eq!(result.unwrap_err().status, StatusCode::CONFLICT);
            }
            let inner = state.inner.lock().unwrap();
            for (path, original) in originals {
                assert_eq!(
                    std::fs::read(inner.config.root.join(path)).unwrap(),
                    original
                );
            }
        }
    }
    #[tokio::test]
    async fn stale_session_cannot_follow_a_configured_alias_to_another_physical_node() {
        let (_temp, state, auth, session) = fixture(true);
        let catalog = list(&state, &auth, &session).await.unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let plan = propose(&state, &auth, &session, &catalog, &id)
            .await
            .unwrap();
        let (original_file, original_collection, original_sources, fresh_target) = {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let original_file = inner.config.files["Shared"].clone();
            let original_collection = inner.config.resource_collections["Owned"].clone();
            let original_sources = ["Shared/Shared.typ", "Target/Target.typ"]
                .map(|path| (path, std::fs::read(root.join(path)).unwrap()));
            let target = inner
                .core
                .qualified_node_at(&root, &root.join("Target/Target.typ"))
                .unwrap()
                .owner;
            let file = inner.config.files.get_mut("Shared").unwrap();
            file.path = "Target/Target.typ".into();
            file.expected_node = Some(target.clone());
            inner
                .config
                .resource_collections
                .get_mut("Owned")
                .unwrap()
                .expected_owner = Some(target);
            let opened = inner.open("alice", "Shared").unwrap();
            (
                original_file,
                original_collection,
                original_sources,
                opened["sessionId"].as_str().unwrap().to_owned(),
            )
        };
        assert_eq!(
            list(&state, &auth, &session).await.unwrap_err().status,
            StatusCode::CONFLICT
        );
        let current = list(&state, &auth, &fresh_target).await.unwrap();
        assert_ne!(current["identity"]["owner"], catalog["identity"]["owner"]);
        assert_eq!(
            status(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Operation {
                    session_id: fresh_target,
                    operation_id: id.clone(),
                    identity: identity_from(&current),
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::CONFLICT
        );
        let fresh_original = {
            let mut inner = state.inner.lock().unwrap();
            inner.config.files.insert("Shared".into(), original_file);
            inner
                .config
                .resource_collections
                .insert("Owned".into(), original_collection);
            let opened = inner.open("alice", "Shared").unwrap();
            opened["sessionId"].as_str().unwrap().to_owned()
        };
        let restored = list(&state, &auth, &fresh_original).await.unwrap();
        assert_eq!(restored["identity"], catalog["identity"]);
        let recovered = status(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                session_id: fresh_original,
                operation_id: id,
                identity: identity_from(&restored),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(recovered, plan);
        let inner = state.inner.lock().unwrap();
        for (path, source) in original_sources {
            assert_eq!(std::fs::read(inner.config.root.join(path)).unwrap(), source);
        }
    }
}
