//! Explicit installation of one fixed product module through Core's existing
//! owned-resource import. This adapter never writes a note or invents a resource
//! identity from a filename; actor records retain the original operation.
use super::{ApiResult, AppState, Location, core_call, location};
#[cfg(test)]
use crate::CoreTestFixture;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use weftext_core::{
    ActorFeatureStatus, Core, CoreError, OperationIntent, OperationRequest, QualifiedNode,
    QualifiedResource,
};

pub(crate) const MODULE_NAME: &str = "weftext-endnotes.typ";
const ACTOR: &str = "local:builtin-endnotes-module";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModuleIdentity {
    pub(crate) scope: String,
    pub(crate) owner: String,
    pub(crate) binding: String,
    pub(crate) module_path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModuleDistribution {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) sha256: String,
    pub(crate) version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModuleResource {
    pub(crate) resource_ref: String,
    pub(crate) owner: String,
    pub(crate) module_path: String,
}
pub(crate) fn distribution() -> ModuleDistribution {
    ModuleDistribution {
        name: MODULE_NAME.into(),
        source: super::writing_api::ENDNOTES_MODULE.into(),
        sha256: weftext_engine::source_hash(super::writing_api::ENDNOTES_MODULE),
        version: "1".into(),
    }
}
pub(crate) fn validate_distribution(module: &ModuleDistribution) -> Result<(), CoreError> {
    let expected = distribution();
    if module.name != expected.name
        || module.source != expected.source
        || module.sha256 != expected.sha256
        || module.version != expected.version
    {
        return Err(CoreError::new(
            "moduleDistribution",
            "The original request does not contain this exact bundled module; no resource was overwritten",
        ));
    }
    Ok(())
}
pub(crate) fn normalized(status: Option<ActorFeatureStatus>, operation_id: &str) -> Value {
    match status {
        Some(status) => {
            let losses = status
                .preview
                .get("losses")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            item.as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| item.to_string())
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({"operationId":status.operation_id,"status":status.status,"planId":status.plan_id,
                "commitAllowed":status.status == "prepared" && status.plan_id.is_some() && losses.is_empty(),"resourceRefs":status.resource_refs,"losses":losses})
        }
        None => {
            json!({"operationId":operation_id,"status":"notRegistered","planId":null,"commitAllowed":false,"resourceRefs":[],"losses":[]})
        }
    }
}
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/editor/endnotes-module/catalog", post(catalog))
        .route("/api/editor/endnotes-module/read", post(read))
        .route("/api/editor/endnotes-module/authorized", post(authorized))
        .route("/api/editor/endnotes-module/prepare", post(prepare))
        .route("/api/editor/endnotes-module/commit", post(commit))
        .route("/api/editor/endnotes-module/status", post(status))
        .route("/api/editor/endnotes-module/settle", post(settle))
        .layer(DefaultBodyLimit::max(512 * 1024))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogRequest {
    session_id: String,
    identity: Option<ModuleIdentity>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceRequest {
    session_id: String,
    identity: ModuleIdentity,
    resource_ref: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrepareRequest {
    session_id: String,
    operation_id: String,
    identity: ModuleIdentity,
    module: ModuleDistribution,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OperationRequestInput {
    session_id: String,
    operation_id: String,
    identity: ModuleIdentity,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommitRequest {
    session_id: String,
    operation_id: String,
    identity: ModuleIdentity,
    plan_id: String,
}

fn path_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key.to_ascii_lowercase()
    } else {
        key
    }
}
fn qualify(
    core: &Core,
    loc: &Location,
    session: &str,
    expected: Option<&ModuleIdentity>,
) -> Result<(ModuleIdentity, QualifiedNode, PathBuf), CoreError> {
    let node = core.qualified_node_for_session_at(&loc.root, &loc.path, session)?;
    let owner = format!(
        "weftext-node://{}/{}",
        node.owner.workspace_id, node.owner.node_id
    );
    let binding = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(
            &json!({"root":path_key(&loc.root),"path":path_key(&loc.path),"owner":owner})
        )?)
    );
    let identity = ModuleIdentity {
        scope: format!("local:{}", node.owner.workspace_id),
        owner,
        binding,
        module_path: MODULE_NAME.into(),
    };
    if expected.is_some_and(|expected| expected != &identity) {
        return Err(CoreError::new(
            "moduleOwnerChanged",
            "The selected session, physical owner or module placement changed; the original operation was not rebound",
        ));
    }
    let target = loc
        .path
        .parent()
        .ok_or_else(|| CoreError::new("invalidPath", "Missing owned note directory"))?
        .join(MODULE_NAME);
    Ok((identity, node, target))
}
fn feature_resource(identity: &ModuleIdentity) -> String {
    format!("endnotes-module:{}", identity.owner)
}
fn fingerprint(
    identity: &ModuleIdentity,
    module: &ModuleDistribution,
) -> Result<String, CoreError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(
            &json!({"identity":identity,"module":module})
        )?)
    ))
}
fn same_module_path(path: &str) -> bool {
    if cfg!(windows) {
        path.eq_ignore_ascii_case(MODULE_NAME)
    } else {
        path == MODULE_NAME
    }
}
fn selected_resource(
    core: &Core,
    loc: &Location,
    session: &str,
    expected: &ModuleIdentity,
    selected: &ModuleResource,
) -> Result<QualifiedResource, CoreError> {
    let (identity, node, target) = qualify(core, loc, session, Some(expected))?;
    if selected.owner != identity.owner || selected.module_path != MODULE_NAME {
        return Err(CoreError::new(
            "moduleResource",
            "The selected resource is not this note's fixed module",
        ));
    }
    let collection =
        core.qualified_resource_collection_at(&loc.root, &loc.path, &node.owner, "live")?;
    if collection
        .entries
        .iter()
        .filter(|entry| same_module_path(&entry.relative_path))
        .count()
        != 1
        || !collection.entries.iter().any(|entry| {
            same_module_path(&entry.relative_path) && entry.resource_ref == selected.resource_ref
        })
    {
        return Err(CoreError::new(
            "moduleResource",
            "The exact module ResourceRef is not uniquely registered to this note",
        ));
    }
    let resource = core.qualified_resource_at(
        &loc.root,
        &loc.path,
        &node.owner,
        &selected.resource_ref,
        &target,
        "live",
        None,
    )?;
    if resource.revision.is_none() {
        return Err(CoreError::new(
            "resourceVersion",
            "The module has no currently accepted resource version; no path-only reuse was granted",
        ));
    }
    if resource.bytes() != super::writing_api::ENDNOTES_MODULE.as_bytes() {
        return Err(CoreError::new(
            "moduleConflict",
            "The registered module contains different original bytes; it was preserved and no arbitrary resource body was delivered by this fixed-module endpoint",
        ));
    }
    core.validate_resource_collection_witness(&loc.root, &collection.witness)?;
    core.validate_resource_witness(&loc.root, &resource.witness)?;
    Ok(resource)
}
async fn catalog(State(state): State<AppState>, Json(request): Json<CatalogRequest>) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    Ok(Json(core_call(&state, move |core| {
        let (identity, node, target) = qualify(core, &loc, &request.session_id, request.identity.as_ref())?;
        let collection = core.qualified_resource_collection_at(&loc.root, &loc.path, &node.owner, "live")?;
        let resources: Vec<_> = collection.entries.iter().filter(|entry| same_module_path(&entry.relative_path)).map(|entry|
            ModuleResource { resource_ref: entry.resource_ref.clone(), owner: identity.owner.clone(), module_path: MODULE_NAME.into() }).collect();
        if resources.is_empty() {
            let cut = core.read_snapshot(&loc.root, &[target.to_string_lossy().into_owned()])?;
            if cut.files[0].bytes.is_some() {
                return Err(CoreError::new("moduleOccupied", "A file already occupies the module location without an exact owned ResourceRef; it was not overwritten or adopted by name"));
            }
        }
        core.validate_resource_collection_witness(&loc.root, &collection.witness)?;
        Ok(json!({"owner":identity.owner,"identity":identity,"coverage":if collection.unrecognized_entries == 0 {"complete"} else {"partial"},"resources":resources}))
    }).await?))
}
async fn read(State(state): State<AppState>, Json(request): Json<ResourceRequest>) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    Ok(Json(core_call(&state, move |core| {
        let selected = ModuleResource { resource_ref: request.resource_ref, owner: request.identity.owner.clone(), module_path: MODULE_NAME.into() };
        let resource = selected_resource(core, &loc, &request.session_id, &request.identity, &selected)?;
        if resource.bytes().len() > 262_144 { return Err(CoreError::new("moduleBudget", "The existing module exceeds the bounded source-read budget; it was preserved")); }
        let source = String::from_utf8(resource.bytes().to_vec()).map_err(|_| CoreError::new("moduleEncoding", "The existing module is not UTF-8; it was preserved"))?;
        Ok(json!({"resourceRef":resource.resource_ref,"sha256":weftext_engine::source_hash(&source),"source":source}))
    }).await?))
}
async fn authorized(
    State(state): State<AppState>,
    Json(request): Json<ResourceRequest>,
) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    Ok(Json(
        core_call(&state, move |core| {
            let resource = selected_resource(
                core,
                &loc,
                &request.session_id,
                &request.identity,
                &ModuleResource {
                    resource_ref: request.resource_ref,
                    owner: request.identity.owner.clone(),
                    module_path: MODULE_NAME.into(),
                },
            )?;
            let target = loc.path.parent().unwrap().join(MODULE_NAME);
            let relative = target
                .strip_prefix(&loc.root)
                .map_err(|_| CoreError::new("outsideRoot", "Module is outside this library"))?
                .to_string_lossy()
                .replace('\\', "/");
            let compiled = core.admitted_compile_snapshot(&loc.root, "live")?;
            let allowed = resource.bytes() == super::writing_api::ENDNOTES_MODULE.as_bytes()
                && compiled
                    .files
                    .get(&relative)
                    .is_some_and(|bytes| bytes.as_slice() == resource.bytes());
            core.validate_resource_witness(&loc.root, &resource.witness)?;
            Ok(json!({"authorized":allowed}))
        })
        .await?,
    ))
}
fn prepare_original(
    core: &mut Core,
    loc: &Location,
    request: &PrepareRequest,
) -> Result<ActorFeatureStatus, CoreError> {
    validate_distribution(&request.module)?;
    let (identity, node, target) =
        qualify(core, loc, &request.session_id, Some(&request.identity))?;
    let resource = feature_resource(&identity);
    let fingerprint = fingerprint(&identity, &request.module)?;
    core.with_actor_for_root(ACTOR, &loc.root, |core| {
        if let Some(original) = core.actor_feature_retry(&resource, &request.operation_id, &fingerprint)? { return Ok(original); }
        let collection = core.qualified_resource_collection_at(&loc.root, &loc.path, &node.owner, "live")?;
        if collection.unrecognized_entries != 0 || collection.entries.iter().any(|entry| same_module_path(&entry.relative_path)) {
            return Err(CoreError::new("moduleMembership", "The module's absence is not proven by complete owned membership; no duplicate import was prepared"));
        }
        let manifest = loc.path.parent().unwrap().join("weftext.resources.json");
        let cut = core.read_snapshot(&loc.root, &[manifest.to_string_lossy().into_owned(), target.to_string_lossy().into_owned()])?;
        if cut.files[1].bytes.is_some() { return Err(CoreError::new("moduleOccupied", "The module location is occupied; create-only import did not overwrite it")); }
        core.validate_resource_collection_witness(&loc.root, &collection.witness)?;
        let operation = OperationRequest { operation_id: request.operation_id.clone(), root: loc.root.to_string_lossy().into_owned(),
            intent: OperationIntent::ImportResource { node: loc.path.to_string_lossy().into_owned(), name: MODULE_NAME.into(), bytes: request.module.source.as_bytes().to_vec(),
                manifest_base_id: cut.files[0].base_id.clone(), resource_base_id: cut.files[1].base_id.clone() } };
        core.actor_feature_prepare_with_witnesses(&resource, &request.operation_id, &fingerprint, operation,
            json!({"kind":"builtinEndnotesModule","identity":identity,"sha256":request.module.sha256,"version":request.module.version,"losses":[]}), &[], &[&node.witness])
    })
}
fn validate_original_identity(
    status: &ActorFeatureStatus,
    identity: &ModuleIdentity,
) -> Result<(), CoreError> {
    if status
        .preview
        .get("identity")
        .is_some_and(|original| original != &json!(identity))
    {
        return Err(CoreError::new(
            "operationConflict",
            "The original module operation belongs to another owner or placement",
        ));
    }
    Ok(())
}
async fn prepare(State(state): State<AppState>, Json(request): Json<PrepareRequest>) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    let id = request.operation_id.clone();
    Ok(Json(normalized(
        Some(core_call(&state, move |core| prepare_original(core, &loc, &request)).await?),
        &id,
    )))
}
async fn status(
    State(state): State<AppState>,
    Json(request): Json<OperationRequestInput>,
) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    let id = request.operation_id.clone();
    let original = core_call(&state, move |core| {
        qualify(core, &loc, &request.session_id, Some(&request.identity))?;
        core.with_actor_for_root(ACTOR, &loc.root, |core| {
            let status = core.actor_feature_status(
                &feature_resource(&request.identity),
                &request.operation_id,
            )?;
            if let Some(status) = &status {
                validate_original_identity(status, &request.identity)?;
            }
            Ok(status)
        })
    })
    .await?;
    Ok(Json(normalized(original, &id)))
}
async fn commit(State(state): State<AppState>, Json(request): Json<CommitRequest>) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    let id = request.operation_id.clone();
    let committed = core_call(&state, move |core| {
        qualify(core, &loc, &request.session_id, Some(&request.identity))?;
        core.with_actor_for_root(ACTOR, &loc.root, |core| {
            let resource = feature_resource(&request.identity);
            let original = core
                .actor_feature_status(&resource, &request.operation_id)?
                .ok_or_else(|| {
                    CoreError::new(
                        "operationMissing",
                        "Query the original module installation before commit",
                    )
                })?;
            validate_original_identity(&original, &request.identity)?;
            if original.plan_id.as_deref() != Some(&request.plan_id) {
                return Err(CoreError::new(
                    "operationConflict",
                    "The plan is not the original module import",
                ));
            }
            core.actor_feature_commit(&resource, &request.operation_id, &request.plan_id)
        })
    })
    .await?;
    Ok(Json(normalized(Some(committed), &id)))
}
async fn settle(
    State(state): State<AppState>,
    Json(request): Json<OperationRequestInput>,
) -> ApiResult {
    let loc = location(&state, &request.session_id)?;
    let id = request.operation_id.clone();
    let ended = core_call(&state, move |core| {
        qualify(core, &loc, &request.session_id, Some(&request.identity))?;
        core.with_actor_for_root(ACTOR, &loc.root, |core| {
            let resource = feature_resource(&request.identity);
            if let Some(original) = core.actor_feature_status(&resource, &request.operation_id)? {
                validate_original_identity(&original, &request.identity)?;
            }
            core.actor_feature_settle(&resource, &request.operation_id)
        })
    })
    .await?;
    Ok(Json(normalized(Some(ended), &id)))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Core, Location, PrepareRequest) {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let root = temp.path().join("library");
        core.create_library(&root).unwrap();
        let note = core
            .create_note_fixture(&root, &root, "module owner")
            .unwrap();
        let loc = Location {
            root,
            path: note.path.into(),
        };
        let (identity, _, _) = qualify(&core, &loc, &note.session_id, None).unwrap();
        let request = PrepareRequest {
            session_id: note.session_id,
            operation_id: uuid::Uuid::new_v4().to_string(),
            identity,
            module: distribution(),
        };
        (temp, core, loc, request)
    }
    #[test]
    fn bundled_module_original_plan_restart_identity_and_draft_are_preserved() {
        let (temp, mut core, loc, mut request) = fixture();
        let source = std::fs::read(&loc.path).unwrap();
        core.persist_draft(&request.session_id, "PRIVATE_UNSAVED_DRAFT")
            .unwrap();
        let prepared = prepare_original(&mut core, &loc, &request).unwrap();
        assert_eq!(prepared.status, "prepared");
        assert_eq!(prepared.resource_refs.len(), 1);
        assert!(!loc.path.parent().unwrap().join(MODULE_NAME).exists());
        let mut changed = request.clone();
        changed.module.source.push_str("\n#panic(\"changed\")");
        assert!(prepare_original(&mut core, &loc, &changed).is_err());
        let other = core
            .create_note_fixture(&loc.root, &loc.root, "other owner")
            .unwrap();
        let other_loc = Location {
            root: loc.root.clone(),
            path: other.path.into(),
        };
        let (other_identity, _, _) = qualify(&core, &other_loc, &other.session_id, None).unwrap();
        let other_request = PrepareRequest {
            session_id: other.session_id,
            identity: other_identity,
            ..request.clone()
        };
        assert!(prepare_original(&mut core, &other_loc, &other_request).is_err());
        drop(core);
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let reopened = core.open(&loc.path, Some(loc.root.clone())).unwrap();
        assert_eq!(
            reopened.recovered_draft.as_deref(),
            Some("PRIVATE_UNSAVED_DRAFT")
        );
        request.session_id = reopened.session_id;
        let retry = prepare_original(&mut core, &loc, &request).unwrap();
        assert_eq!(retry.plan_id, prepared.plan_id);
        assert_eq!(retry.resource_refs, prepared.resource_refs);
        let committed = core
            .with_actor_for_root(ACTOR, &loc.root, |core| {
                core.actor_feature_commit(
                    &feature_resource(&request.identity),
                    &request.operation_id,
                    prepared.plan_id.as_deref().unwrap(),
                )
            })
            .unwrap();
        assert_eq!(committed.status, "committed");
        assert_eq!(std::fs::read(&loc.path).unwrap(), source);
        let selected = ModuleResource {
            resource_ref: committed.resource_refs[0].clone(),
            owner: request.identity.owner.clone(),
            module_path: MODULE_NAME.into(),
        };
        let resource = selected_resource(
            &core,
            &loc,
            &request.session_id,
            &request.identity,
            &selected,
        )
        .unwrap();
        assert_eq!(
            resource.bytes(),
            super::super::writing_api::ENDNOTES_MODULE.as_bytes()
        );
        let relative = loc
            .path
            .parent()
            .unwrap()
            .join(MODULE_NAME)
            .strip_prefix(&loc.root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(
            core.admitted_compile_snapshot(&loc.root, "live")
                .unwrap()
                .files[&relative],
            super::super::writing_api::ENDNOTES_MODULE.as_bytes()
        );
    }
    #[test]
    fn unknown_settlement_survives_restart_and_fences_late_module_prepare() {
        let (temp, mut core, loc, mut request) = fixture();
        let ended = core
            .with_actor_for_root(ACTOR, &loc.root, |core| {
                core.actor_feature_settle(
                    &feature_resource(&request.identity),
                    &request.operation_id,
                )
            })
            .unwrap();
        assert_eq!(ended.status, "cancelled");
        drop(core);
        let mut core = Core::new(temp.path().join("private")).unwrap();
        request.session_id = core
            .open(&loc.path, Some(loc.root.clone()))
            .unwrap()
            .session_id;
        let late = prepare_original(&mut core, &loc, &request).unwrap();
        assert_eq!(late.status, "cancelled");
        assert!(!loc.path.parent().unwrap().join(MODULE_NAME).exists());
    }
    #[test]
    fn same_name_ordinary_files_and_other_owner_identity_never_become_resources() {
        let (_temp, mut core, loc, request) = fixture();
        std::fs::write(
            loc.path.parent().unwrap().join(MODULE_NAME),
            b"different original",
        )
        .unwrap();
        assert!(prepare_original(&mut core, &loc, &request).is_err());
        assert_eq!(
            std::fs::read(loc.path.parent().unwrap().join(MODULE_NAME)).unwrap(),
            b"different original"
        );
        let other = core
            .create_note_fixture(&loc.root, &loc.root, "other")
            .unwrap();
        let other = Location {
            root: loc.root.clone(),
            path: other.path.into(),
        };
        assert!(qualify(&core, &other, &request.session_id, Some(&request.identity)).is_err());
        let (_other_temp, mut other_core, other_loc, other_request) = fixture();
        let import_id = uuid::Uuid::new_v4().to_string();
        let plan = other_core
            .prepare_import_resource(
                &other_loc.root,
                &other_loc.path,
                MODULE_NAME,
                b"registered different original".to_vec(),
                &import_id,
            )
            .unwrap();
        other_core
            .execute_operation(&import_id, &plan.plan_id)
            .unwrap();
        let reference = other_core
            .managed_resource_snapshot(&other_loc.root, &other_loc.path)
            .unwrap()
            .remove(0)
            .resource_ref;
        let selected = ModuleResource {
            resource_ref: reference,
            owner: other_request.identity.owner.clone(),
            module_path: MODULE_NAME.into(),
        };
        assert!(
            selected_resource(
                &other_core,
                &other_loc,
                &other_request.session_id,
                &other_request.identity,
                &selected
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(other_loc.path.parent().unwrap().join(MODULE_NAME)).unwrap(),
            b"registered different original"
        );
    }

    #[test]
    fn same_bytes_replacement_never_borrows_the_original_resource_birth() {
        let (_temp, mut core, loc, request) = fixture();
        let prepared = prepare_original(&mut core, &loc, &request).unwrap();
        let committed = core
            .with_actor_for_root(ACTOR, &loc.root, |core| {
                core.actor_feature_commit(
                    &feature_resource(&request.identity),
                    &request.operation_id,
                    prepared.plan_id.as_deref().unwrap(),
                )
            })
            .unwrap();
        let target = loc.path.parent().unwrap().join(MODULE_NAME);
        let original = loc
            .path
            .parent()
            .unwrap()
            .join("retained-original-module.typ");
        std::fs::rename(&target, &original).unwrap();
        std::fs::write(&target, super::super::writing_api::ENDNOTES_MODULE).unwrap();
        let selected = ModuleResource {
            resource_ref: committed.resource_refs[0].clone(),
            owner: request.identity.owner.clone(),
            module_path: MODULE_NAME.into(),
        };
        assert!(
            selected_resource(
                &core,
                &loc,
                &request.session_id,
                &request.identity,
                &selected
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(&original).unwrap(),
            super::super::writing_api::ENDNOTES_MODULE.as_bytes()
        );
    }
}
