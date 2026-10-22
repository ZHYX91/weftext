//! Explicit authority for an owner's current and future registered resources.
//! The native manifest remains the only membership and locator authority.
use super::*;
use axum::extract::DefaultBodyLimit;
use weftext_core::{
    ActorFeatureStatus, OperationIntent, OperationRequest, QualifiedResourceCollection,
};
use weftext_knowledge::NodeRef;

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_UPLOAD: usize = 16 * 1024 * 1024;
const TTL_MS: u64 = 900_000;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Collection {
    pub(super) owner: String,
    #[serde(default)]
    pub(super) expected_owner: Option<NodeRef>,
    #[serde(default)]
    pub(super) from_import_slot: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) from_structure_slot: bool,
    #[serde(default = "query_scopes::live")]
    pub(super) lifecycle: String,
    #[serde(default)]
    pub(super) title: String,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) read: bool,
    #[serde(default)]
    pub(super) download: bool,
    #[serde(default)]
    pub(super) r#import: bool,
}
fn canonical_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn is_false(value: &bool) -> bool {
    !*value
}
pub(super) fn split_member<'a>(config: &Config, id: &'a str) -> Option<(&'a str, &'a str)> {
    if let Some(member) = content_import::split_asset(config, id) {
        return Some(member);
    }
    let (collection, member) = id.split_once('/')?;
    (config.resource_collections.contains_key(collection) && canonical_uuid(member))
        .then_some((collection, member))
}
pub(super) fn selection<'a>(config: &Config, id: &'a str) -> Option<&'a str> {
    if let Some(owner) = id
        .strip_suffix("/*")
        .filter(|owner| content_import::split_member(config, owner).is_some())
    {
        return Some(owner);
    }
    let collection = id.strip_suffix("/*")?;
    config
        .resource_collections
        .contains_key(collection)
        .then_some(collection)
}
pub(super) fn valid_config(config: &Config) -> bool {
    config.resource_collections.len() <= 64
        && config.resource_collections.iter().all(|(id, c)| {
            valid_id(id)
                && !config.files.contains_key(id)
                && !config.resources.contains_key(id)
                && usize::from(c.expected_owner.is_some())
                    + usize::from(c.from_import_slot)
                    + usize::from(c.from_structure_slot)
                    == 1
                && c.expected_owner.as_ref().is_none_or(|owner| {
                    canonical_uuid(&owner.workspace_id) && canonical_uuid(&owner.node_id)
                })
                && c.title.len() <= 500
                && matches!(c.lifecycle.as_str(), "live" | "trash")
                && config.files.get(&c.owner).is_some_and(|f| {
                    f.lifecycle == c.lifecycle
                        && if let Some(expected) = &c.expected_owner {
                            f.expected_node.as_ref().is_none_or(|n| n == expected)
                        } else if c.from_import_slot {
                            f.importable && f.lifecycle == "live" && f.expected_node.is_none()
                        } else {
                            c.from_structure_slot
                                && config.structure_slots.contains_key(&c.owner)
                                && f.lifecycle == "live"
                        }
                })
        })
}
pub(super) fn valid_rights(config: &Config, rights: &BTreeMap<String, Rights>) -> bool {
    rights.len() <= 64
        && rights.iter().all(|(id, r)| {
            config
                .resource_collections
                .get(id)
                .is_some_and(|c| (!r.download || r.read) && (!r.r#import || c.lifecycle == "live"))
        })
}
pub(super) fn configured(inner: &HostedInner, collection: &str) -> Result<Collection, ApiError> {
    if let Some(c) = inner.config.resource_collections.get(collection) {
        return Ok(c.clone());
    }
    let file = content_import::member_file(inner, collection)?.ok_or_else(unavailable)?;
    Ok(Collection {
        owner: collection.into(),
        expected_owner: file.expected_node,
        from_import_slot: false,
        from_structure_slot: false,
        lifecycle: "live".into(),
        title: file.title,
    })
}
fn access(
    inner: &HostedInner,
    actor: &str,
    collection: &str,
) -> Result<(Collection, Rights), ApiError> {
    let c = configured(inner, collection)?;
    let r = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.resource_collections.get(collection).cloned())
        .or_else(|| content_import::resource_rights(inner, actor, collection))
        .ok_or_else(unavailable)?;
    Ok((c, r))
}
/// Policy only: frozen outputs must not expand or read a newer manifest.
pub(super) fn input_access(
    inner: &HostedInner,
    actor: &str,
    collection: &str,
    download: bool,
) -> Result<(), ApiError> {
    if content_import::split_member(&inner.config, collection).is_some() {
        let r =
            content_import::resource_rights(inner, actor, collection).ok_or_else(unavailable)?;
        return if r.read && (!download || r.download) {
            Ok(())
        } else {
            Err(unavailable())
        };
    }
    let (_, r) = access(inner, actor, collection)?;
    if !r.read || (download && !r.download) {
        return Err(unavailable());
    }
    Ok(())
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "resourceCollectionChanged",
        UiMessage::error_api_hosted_resource_collections_collection_basis_changed(),
    )
}
fn resource_diagnostic_line(
    setting: Option<&std::ffi::OsStr>,
    phase: &str,
    id: &str,
    code: &str,
    diagnostics: &[RawDiagnostic],
) -> Option<String> {
    if setting != Some(std::ffi::OsStr::new("1")) {
        return None;
    }
    fn safe_code(value: &str) -> &str {
        if !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            value
        } else {
            "redacted"
        }
    }
    // Keep only bounded typed provenance. Diagnostic wording is private raw
    // evidence, never an OS-code parser or an error-state discriminator.
    let origins = diagnostics
        .iter()
        .take(32)
        .map(|diagnostic| diagnostic.origin)
        .collect::<Vec<_>>();
    let safe_id = digest(id.as_bytes()).chars().take(16).collect::<String>();
    let record = json!({
        "phase":safe_code(phase),
        "id":safe_id,
        "code":safe_code(code),
        "diagnosticOrigins":origins,
    });
    serde_json::to_string(&record)
        .ok()
        .map(|record| format!("weftext-resource-diagnostic {record}"))
}
fn resource_diagnostic(phase: &str, id: &str, code: &str, diagnostics: &[RawDiagnostic]) {
    if let Some(line) = resource_diagnostic_line(
        std::env::var_os("WEFTEXT_TEST_RESOURCE_DIAGNOSTICS").as_deref(),
        phase,
        id,
        code,
        diagnostics,
    ) {
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr().lock(), "{line}");
    }
}
fn scoped_core_error(phase: &str, collection: &str, error: CoreError) -> ApiError {
    resource_diagnostic(phase, collection, &error.code, &error.diagnostics);
    changed()
}
fn core_error(error: CoreError) -> ApiError {
    resource_diagnostic(
        "generic_core",
        "unscoped_core",
        &error.code,
        &error.diagnostics,
    );
    changed()
}
/// The special empty Office slot is tied to its original committed producer,
/// never to whichever author declaration happens to occupy that path now.
pub(super) fn effective_owner(inner: &HostedInner, collection: &str) -> Result<NodeRef, ApiError> {
    let c = configured(inner, collection)?;
    if let Some(expected) = &c.expected_owner {
        return Ok(expected.clone());
    }
    if c.from_structure_slot {
        let file = inner
            .file(&c.owner)
            .ok()
            .filter(|f| f.lifecycle == "live")
            .ok_or_else(unavailable)?;
        let node = inner
            .core
            .qualified_structure_slot_at(
                &inner.config.root,
                &inner.config.root.join(&file.path),
                &c.owner,
            )
            .map_err(|error| scoped_core_error("structure_owner", collection, error))?;
        if file
            .expected_node
            .as_ref()
            .is_some_and(|owner| owner != &node.owner)
        {
            return Err(changed());
        }
        return Ok(node.owner);
    }
    if !c.from_import_slot {
        return Err(unavailable());
    }
    let owner = inner
        .file(&c.owner)
        .ok()
        .filter(|f| f.importable && f.lifecycle == "live")
        .ok_or_else(unavailable)?;
    inner
        .core
        .qualified_imported_query_node_at(&inner.config.root, &inner.config.root.join(&owner.path))
        .map(|node| node.owner)
        .map_err(|error| scoped_core_error("import_owner", collection, error))
}
pub(super) fn qualify(
    inner: &HostedInner,
    collection: &str,
) -> Result<QualifiedResourceCollection, ApiError> {
    content_import::qualify_member(inner, collection).inspect_err(|error| {
        resource_diagnostic("qualify_member", collection, &error.code, &[]);
    })?;
    let c = configured(inner, collection)?;
    let owner = inner.file(&c.owner).ok().ok_or_else(unavailable)?;
    let expected = effective_owner(inner, collection)?;
    let captured = inner
        .core
        .qualified_resource_collection_at(
            &inner.config.root,
            &inner.config.root.join(&owner.path),
            &expected,
            &c.lifecycle,
        )
        .map_err(|error| scoped_core_error("qualify_collection", collection, error))?;
    if captured.retained_bytes() > MAX_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_resource_collections_collection_read_budget(),
        ));
    }
    Ok(captured)
}
pub(super) fn capture(
    inner: &HostedInner,
    actor: &str,
    collection: &str,
) -> Result<QualifiedResourceCollection, ApiError> {
    input_access(inner, actor, collection, false)?;
    qualify(inner, collection)
}
type ExpandedInputs = (Vec<String>, Vec<(String, Arc<QualifiedResourceCollection>)>);
pub(super) fn expand_inputs(
    inner: &HostedInner,
    actor: &str,
    ids: &[String],
) -> Result<ExpandedInputs, ApiError> {
    let mut expanded = vec![];
    for id in ids {
        if let Some(slot) = id
            .strip_suffix("/*")
            .filter(|s| inner.config.content_import_slots.contains_key(*s))
        {
            let r = inner
                .policy
                .users
                .get(actor)
                .filter(|u| u.enabled)
                .and_then(|u| u.content_imports.get(slot))
                .ok_or_else(unavailable)?;
            if !r.members.read || !r.resources.read {
                return Err(unavailable());
            }
            let members = content_import::slot_member_ids(inner, slot)?;
            if members.len() > 64 {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_resource_collections_expanded_input_count_limit(),
                ));
            }
            for member in members {
                expanded.push(member.clone());
                expanded.push(format!("{member}/*"));
            }
        } else {
            expanded.push(id.clone());
        }
    }
    let mut actual = BTreeSet::new();
    let mut captures = BTreeMap::new();
    let mut bytes = 0usize;
    for id in &expanded {
        if let Some(collection) =
            selection(&inner.config, id).or_else(|| split_member(&inner.config, id).map(|(c, _)| c))
        {
            if !captures.contains_key(collection) {
                let captured = capture(inner, actor, collection)?;
                // Unknown/ambiguous records cannot silently disappear from an
                // explicitly requested complete compilation environment.
                if selection(&inner.config, id).is_some() && captured.unrecognized_entries > 0 {
                    resource_diagnostic(
                        "expand_new_unrecognized",
                        collection,
                        "unrecognized_entries",
                        &[],
                    );
                    return Err(changed());
                }
                bytes = bytes.saturating_add(captured.retained_bytes());
                if bytes > MAX_BYTES {
                    return Err(ApiError::bad(
                        UiMessage::error_api_hosted_resource_collections_collection_input_budget(),
                    ));
                }
                captures.insert(collection.to_owned(), Arc::new(captured));
            }
            let captured = &captures[collection];
            if selection(&inner.config, id).is_some() {
                if captured.unrecognized_entries > 0 {
                    resource_diagnostic(
                        "expand_cached_unrecognized",
                        collection,
                        "unrecognized_entries",
                        &[],
                    );
                    return Err(changed());
                }
                for entry in &captured.entries {
                    let member = entry.resource_ref.rsplit('/').next().ok_or_else(changed)?;
                    actual.insert(format!("{collection}/{member}"));
                }
            } else {
                let (_, member) = split_member(&inner.config, id).ok_or_else(unavailable)?;
                if !captured
                    .entries
                    .iter()
                    .any(|e| e.resource_ref.rsplit('/').next() == Some(member))
                {
                    return Err(unavailable());
                }
                actual.insert(id.clone());
            }
        } else {
            actual.insert(id.clone());
        }
        if actual.len() > 64 {
            return Err(ApiError::bad(UiMessage::error_api_hosted_resource_collections_explicit_expanded_input_count_limit()));
        }
    }
    Ok((actual.into_iter().collect(), captures.into_iter().collect()))
}
pub(super) fn title(inner: &HostedInner, collection: &str) -> Result<String, ApiError> {
    let c = configured(inner, collection)?;
    Ok(if c.title.is_empty() {
        inner.file(&c.owner)?.title
    } else {
        c.title
    })
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    collection: String,
    name: String,
    bytes_base64: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    collection: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    collection: String,
    operation_id: String,
    plan_id: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    epoch: u64,
    expires: u64,
    binding: String,
    owner: NodeRef,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/media/import/prepare", post(prepare))
        .route("/api/hosted/media/import/commit", post(commit))
        .route("/api/hosted/media/import/operation", post(operation))
        .route("/api/hosted/media/import/settle", post(settle))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}
pub(super) fn import_access(
    inner: &HostedInner,
    actor: &str,
    id: &str,
) -> Result<Collection, ApiError> {
    let (c, r) = access(inner, actor, id)?;
    if c.lifecycle != "live" || !r.r#import {
        return Err(unavailable());
    }
    Ok(c.clone())
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:resource-import", inner.actor_scope(actor))
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn binding(inner: &HostedInner, id: &str) -> Result<String, ApiError> {
    media::binding_hash(inner, &BTreeSet::from([format!("{id}/*")]))
}
fn original_allowed(
    inner: &HostedInner,
    actor: &str,
    id: &str,
    status: &ActorFeatureStatus,
) -> bool {
    serde_json::from_value::<Decision>(status.preview["_resourceImport"].clone()).is_ok_and(|d| {
        d.epoch == inner.actor_generation(actor)
            && binding(inner, id).is_ok_and(|b| b == d.binding)
            && configured(inner, id)
                .ok()
                .is_some_and(|_| effective_owner(inner, id).is_ok_and(|owner| owner == d.owner))
            && configured(inner, id)
                .ok()
                .and_then(|c| inner.file(&c.owner).ok())
                .is_some_and(|f| {
                    inner
                        .core
                        .qualified_node_at(&inner.config.root, &inner.config.root.join(&f.path))
                        .is_ok_and(|n| n.owner == d.owner)
                })
    })
}
fn safe_status(
    inner: &HostedInner,
    actor: &str,
    id: &str,
    status: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let same = original_allowed(inner, actor, id, &status);
    let mut preview = json!({});
    if same {
        for key in ["summary", "name", "bytes", "contentType", "metadata"] {
            if let Some(v) = status.preview.get(key) {
                preview[key] = v.clone();
            }
        }
    }
    let refs = if same { status.resource_refs } else { vec![] };
    let assets = refs
        .iter()
        .filter_map(|r| {
            r.rsplit('/')
                .next()
                .filter(|s| canonical_uuid(s))
                .map(|m| format!("{id}/{m}"))
        })
        .collect::<Vec<_>>();
    let allowed = status.status == "prepared"
        && same
        && serde_json::from_value::<Decision>(status.preview["_resourceImport"].clone())
            .is_ok_and(|d| d.expires > now_ms());
    Ok(
        json!({"collection":id,"owner":configured(inner,id)?.owner,"operationId":status.operation_id,"status":status.status,"planId":status.plan_id.filter(|_|allowed),"preview":preview,"resourceRefs":refs,"assets":assets,"commitAllowed":allowed}),
    )
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && !name.contains(['/', '\\', ':'])
        && !name.ends_with(['.', ' '])
        && !name.chars().any(|c| c.is_control())
        && !matches!(name, "." | "..")
}
fn member_import_fingerprint(
    collection: &str,
    name: &str,
    bytes: &[u8],
) -> Result<String, ApiError> {
    Ok(digest(
        &serde_json::to_vec(&json!({"collection":collection,"name":name,
        "uploadDigest":digest(bytes)}))
        .map_err(|_| changed())?,
    ))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Prepare>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            import_access(inner, &actor, &req.collection)?;
            if req.bytes_base64.len() > MAX_UPLOAD.div_ceil(3) * 4 {
                return Err(ApiError::bad(UiMessage::error_api_hosted_resource_collections_resource_original_size_limit()));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&req.bytes_base64)
                .map_err(|_| ApiError::bad(UiMessage::error_api_hosted_resource_collections_resource_original_encoding_invalid()))?;
            prepare_member_import(
                inner,
                &actor,
                &req.collection,
                &req.name,
                bytes,
                &req.operation_id,
            )
        })
        .await?,
    ))
}
/// Reuse the exact same member operation under the caller's original hosted
/// lock. Builtin adapters may add narrower identity checks without a second
/// authorization gap or a competing resource-import persistence authority.
pub(super) fn prepare_member_import(
    inner: &mut HostedInner,
    actor: &str,
    collection: &str,
    name: &str,
    bytes: Vec<u8>,
    operation_id: &str,
) -> Result<Value, ApiError> {
    let config = import_access(inner, actor, collection)?;
    if !valid_name(name) {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_resource_collections_resource_filename_invalid(),
        ));
    }
    if bytes.len() > MAX_UPLOAD {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_resource_collections_resource_original_size_limit(),
        ));
    }
    let fp = member_import_fingerprint(collection, name, &bytes)?;
    let actor_scope = scope(inner, actor);
    if let Some(old) = inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
            core.actor_feature_retry(collection, operation_id, &fp)
        })
        .map_err(core_error)?
    {
        return safe_status(inner, actor, collection, old);
    }
    let captured = qualify(inner, collection)?;
    let root = inner.config.root.clone();
    let owner_path = root.join(&inner.file(&config.owner)?.path);
    let node = inner
        .core
        .qualified_query_node_at(&root, &owner_path, &captured.owner, "live")
        .map_err(core_error)?;
    let directory = owner_path.parent().ok_or_else(changed)?;
    let cut = inner
        .core
        .read_snapshot(
            &root,
            &[
                directory
                    .join("weftext.resources.json")
                    .to_string_lossy()
                    .into_owned(),
                directory.join(name).to_string_lossy().into_owned(),
            ],
        )
        .map_err(core_error)?;
    let ids = cut
        .files
        .iter()
        .map(|f| f.base_id.clone())
        .collect::<Vec<_>>();
    let result = (|| {
        let mut pool = vec![];
        Core::merge_query_node_witness(&mut pool, &node.witness).map_err(core_error)?;
        if captured.initialized {
            Core::merge_resource_collection_witness(&mut pool, &captured.witness)
                .map_err(core_error)?;
        }
        inner
            .core
            .validate_resource_collection_witness(&root, &captured.witness)
            .map_err(core_error)?;
        if captured
            .retained_bytes()
            .saturating_add(node.witness.retained_bytes())
            .saturating_add(bytes.len())
            > MAX_BYTES
        {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_resource_collections_resource_import_basis_budget(),
            ));
        }
        let (metadata, content_type) = crate::media_api::inspect_media(&bytes);
        let decision = Decision {
            epoch: inner.actor_generation(actor),
            expires: now_ms().saturating_add(TTL_MS),
            binding: binding(inner, collection)?,
            owner: captured.owner,
        };
        let preview = json!({"summary":UiMessage::error_api_hosted_resource_collections_import_plan_summary(),"name":name,"bytes":bytes.len(),"contentType":content_type,"metadata":metadata,"_resourceImport":decision});
        let request = OperationRequest {
            operation_id: operation_id.into(),
            root: root.to_string_lossy().into_owned(),
            intent: OperationIntent::ImportResource {
                node: owner_path.to_string_lossy().into_owned(),
                name: name.into(),
                bytes,
                manifest_base_id: cut.files[0].base_id.clone(),
                resource_base_id: cut.files[1].base_id.clone(),
            },
        };
        let refs = pool.iter().collect::<Vec<_>>();
        let status = inner
            .core
            .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
                core.actor_feature_prepare_with_witnesses(
                    collection,
                    operation_id,
                    &fp,
                    request,
                    preview,
                    &[],
                    &refs,
                )
            })
            .map_err(core_error)?;
        safe_status(inner, actor, collection, status)
    })();
    for id in ids {
        let _ = inner.core.take_snapshot_witness(&id);
    }
    result
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            member_import_status(inner, &actor, &req.collection, &req.operation_id)
        })
        .await?,
    ))
}
pub(super) fn member_import_status(
    inner: &mut HostedInner,
    actor: &str,
    collection: &str,
    operation_id: &str,
) -> Result<Value, ApiError> {
    let config = import_access(inner, actor, collection)?;
    match inner
        .core
        .with_actor_for_root(&scope(inner, actor), &inner.config.root, |core| {
            core.actor_feature_status(collection, operation_id)
        })
        .map_err(core_error)?
    {
        Some(status) => safe_status(inner, actor, collection, status),
        None => Ok(
            json!({"collection":collection,"owner":config.owner,"operationId":operation_id,"status":"notRegistered","planId":null,"preview":{},"resourceRefs":[],"assets":[],"commitAllowed":false}),
        ),
    }
}
/// Status-only exact intent qualification for narrower built-in adapters.
/// Missing operations remain missing; this never prepares a replacement plan.
pub(super) fn member_import_status_matching(
    inner: &mut HostedInner,
    actor: &str,
    collection: &str,
    operation_id: &str,
    name: &str,
    bytes: &[u8],
) -> Result<Value, ApiError> {
    import_access(inner, actor, collection)?;
    let fingerprint = member_import_fingerprint(collection, name, bytes)?;
    let original = inner
        .core
        .with_actor_for_root(&scope(inner, actor), &inner.config.root, |core| {
            core.actor_feature_retry(collection, operation_id, &fingerprint)
        })
        .map_err(core_error)?;
    match original {
        Some(status) => {
            // Generic status may intentionally retain a committed fact while
            // withholding stale resourceRefs. A fixed-module installer must
            // not interpret that empty projection as permission to adopt a
            // different current owner's same-named resource. Tombstones for
            // an unknown cancelled operation have no decision and stay useful
            // as a durable fence against late preparation.
            if status.preview.get("_resourceImport").is_some()
                && !original_allowed(inner, actor, collection, &status)
            {
                return Err(changed());
            }
            safe_status(inner, actor, collection, status)
        }
        None => member_import_status(inner, actor, collection, operation_id),
    }
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Commit>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            commit_member_import(
                inner,
                &actor,
                &req.collection,
                &req.operation_id,
                &req.plan_id,
            )
        })
        .await?,
    ))
}
pub(super) fn commit_member_import(
    inner: &mut HostedInner,
    actor: &str,
    collection: &str,
    operation_id: &str,
    plan_id: &str,
) -> Result<Value, ApiError> {
    import_access(inner, actor, collection)?;
    let actor_scope = scope(inner, actor);
    let old = inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
            core.actor_feature_status(collection, operation_id)
        })
        .map_err(core_error)?
        .ok_or_else(changed)?;
    if old.status == "prepared"
        && (!original_allowed(inner, actor, collection, &old)
            || old.plan_id.as_deref() != Some(plan_id)
            || !serde_json::from_value::<Decision>(old.preview["_resourceImport"].clone())
                .is_ok_and(|d| d.expires > now_ms()))
    {
        return Err(changed());
    }
    let status = inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
            core.actor_feature_commit(collection, operation_id, plan_id)
        })
        .map_err(core_error)?;
    safe_status(inner, actor, collection, status)
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            settle_member_import(inner, &actor, &req.collection, &req.operation_id)
        })
        .await?,
    ))
}
pub(super) fn settle_member_import(
    inner: &mut HostedInner,
    actor: &str,
    collection: &str,
    operation_id: &str,
) -> Result<Value, ApiError> {
    import_access(inner, actor, collection)?;
    let status = inner
        .core
        .with_actor_for_root(&scope(inner, actor), &inner.config.root, |core| {
            core.actor_feature_settle(collection, operation_id)
        })
        .map_err(core_error)?;
    safe_status(inner, actor, collection, status)
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn resource_diagnostics_default_to_silent_and_require_the_exact_gate() {
        for setting in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("01"),
            Some("1 "),
        ] {
            assert!(
                resource_diagnostic_line(
                    setting.map(OsStr::new),
                    "qualify_collection",
                    "private collection",
                    "resourceManifestUnavailable",
                    &[RawDiagnostic {
                        origin: DiagnosticOrigin::Os,
                        text: "private message (os error 5)".into()
                    }],
                )
                .is_none()
            );
        }
        assert!(
            resource_diagnostic_line(Some(OsStr::new("1")), "phase", "id", "code", &[]).is_some()
        );
    }

    #[test]
    fn resource_diagnostic_formatter_emits_bounded_typed_origins_without_parsing_or_disclosing_text()
     {
        let id = "private collection/path?token=secret";
        let message = "private body and path (os error 2), later failure (os error 32)";
        let line = resource_diagnostic_line(
            Some(OsStr::new("1")),
            "qualify_collection",
            id,
            "resourceManifestUnavailable",
            &[RawDiagnostic {
                origin: DiagnosticOrigin::Os,
                text: message.into(),
            }],
        )
        .unwrap();
        let record: Value =
            serde_json::from_str(line.strip_prefix("weftext-resource-diagnostic ").unwrap())
                .unwrap();
        assert_eq!(record.as_object().unwrap().len(), 4);
        assert_eq!(record["phase"], "qualify_collection");
        assert_eq!(record["code"], "resourceManifestUnavailable");
        assert_eq!(record["diagnosticOrigins"], json!(["os"]));
        assert_eq!(record["id"], digest(id.as_bytes())[..16]);
        assert!(
            record["id"]
                .as_str()
                .unwrap()
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
        for private in [id, message, "private", "path", "token", "secret"] {
            assert!(!line.contains(private));
        }
        let redacted = resource_diagnostic_line(
            Some(OsStr::new("1")),
            "private/phase",
            id,
            "private/code\nbody",
            &vec![
                RawDiagnostic {
                    origin: DiagnosticOrigin::Dependency,
                    text: "private (os error 5) later (os error not-a-number)".into()
                };
                40
            ],
        )
        .unwrap();
        let record: Value = serde_json::from_str(
            redacted
                .strip_prefix("weftext-resource-diagnostic ")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(record["phase"], "redacted");
        assert_eq!(record["code"], "redacted");
        assert_eq!(record["diagnosticOrigins"].as_array().unwrap().len(), 32);
        assert!(
            record["diagnosticOrigins"]
                .as_array()
                .unwrap()
                .iter()
                .all(|origin| origin == "dependency")
        );
        assert!(!redacted.contains("private"));
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn fixture(read: bool, import: bool) -> (tempfile::TempDir, HostedState, Auth, String) {
        let (temp, state, auth, _) = media::tests::fixture();
        let mut inner = state.inner.lock().unwrap();
        let existing = inner.config.resources["Diagram"].clone();
        let asset = format!("Owned/{}", existing.reference.rsplit('/').next().unwrap());
        inner.config.resource_collections.insert(
            "Owned".into(),
            Collection {
                owner: "Shared".into(),
                expected_owner: Some(existing.expected_owner),
                from_import_slot: false,
                from_structure_slot: false,
                lifecycle: "live".into(),
                title: "明确批准的集合".into(),
            },
        );
        let alice = inner.policy.users.get_mut("alice").unwrap();
        alice.resources.clear();
        alice.resource_collections.insert(
            "Owned".into(),
            Rights {
                read,
                download: false,
                import,
            },
        );
        drop(inner);
        (temp, state, auth, asset)
    }
    async fn propose(
        state: &HostedState,
        auth: &Auth,
        id: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<Value, ApiError> {
        prepare(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Prepare {
                collection: "Owned".into(),
                name: name.into(),
                bytes_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
                operation_id: id.into(),
            }),
        )
        .await
        .map(|r| r.0)
    }
    #[test]
    fn selection_and_policy_do_not_infer_filesystem_members_or_write_trash() {
        let (_temp, state, _auth, asset) = fixture(true, false);
        let mut inner = state.inner.lock().unwrap();
        assert!(media::is_asset(&inner.config, &asset));
        assert!(media::valid_input(&inner.config, "Owned/*"));
        assert!(!media::is_asset(&inner.config, "Owned/*"));
        for id in [
            "Owned/../private.png",
            "Owned/private.png",
            "Unknown/*",
            "Owned/*/nested",
        ] {
            assert!(!media::valid_input(&inner.config, id));
        }
        let random = format!("Owned/{}", uuid::Uuid::new_v4());
        // A pure grant check is deliberately separate from actual membership.
        media::input_access(&inner, "alice", &random, false).unwrap();
        assert!(media::resolve_asset(&inner, "alice", &random).is_err());
        assert!(media::input_access(&inner, "alice", &asset, true).is_err());
        inner
            .config
            .resource_collections
            .get_mut("Owned")
            .unwrap()
            .lifecycle = "trash".into();
        assert!(!valid_rights(
            &inner.config,
            &BTreeMap::from([(
                "Owned".into(),
                Rights {
                    r#import: true,
                    ..Default::default()
                }
            )])
        ));
    }
    #[test]
    fn import_slot_mode_is_exclusive_and_cannot_borrow_an_ordinary_node_birth() {
        let (_temp, state, _auth, asset) = fixture(true, false);
        let mut inner = state.inner.lock().unwrap();
        inner
            .config
            .resource_collections
            .get_mut("Owned")
            .unwrap()
            .from_import_slot = true;
        assert!(!valid_config(&inner.config));
        inner
            .config
            .resource_collections
            .get_mut("Owned")
            .unwrap()
            .expected_owner = None;
        assert!(!valid_config(&inner.config));
        inner.config.files.get_mut("Shared").unwrap().importable = true;
        // Marking a configured file as a future slot cannot manufacture the
        // original committed ImportDocx/CreateTree physical birth evidence.
        assert!(effective_owner(&inner, "Owned").is_err());
        assert!(media::resolve_asset(&inner, "alice", &asset).is_err());
        inner.config.files.get_mut("Shared").unwrap().expected_node = Some(NodeRef {
            workspace_id: uuid::Uuid::new_v4().to_string(),
            node_id: uuid::Uuid::new_v4().to_string(),
        });
        assert!(!valid_config(&inner.config));
    }
    #[test]
    fn expansion_uses_only_registered_symbols_and_retains_member_and_selector_witness() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_temp, state, _auth, asset) = fixture(true, false);
        let inner = state.inner.lock().unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(inner.config.root.join("Shared/private-locator.png"))
            .unwrap();
        let (members, w) =
            expand_inputs(&inner, "alice", &[asset.clone(), "Owned/*".into()]).unwrap();
        assert_eq!(members, vec![asset.clone()]);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0, "Owned");
        assert!(w[0].1.revision.is_some());
        assert!(inner.rights("alice", "Shared").is_err());
        // Compilation and viewer capture must still acquire the selected body.
        assert!(media::capture_input(&inner, "alice", &asset).is_err());
        let before =
            media::binding_hash(&inner, &BTreeSet::from([asset, "Owned/*".into()])).unwrap();
        assert_eq!(
            before,
            media::binding_hash(
                &inner,
                &BTreeSet::from([members[0].clone(), "Owned/*".into()])
            )
            .unwrap()
        );
        drop(locked);
    }
    #[tokio::test]
    async fn import_only_receipt_preserves_source_and_does_not_grant_resource_read() {
        let (_temp, state, auth, _asset) = fixture(false, true);
        let source = std::fs::read(
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
        let proposal = propose(&state, &auth, &id, "own-upload.bin", b"ONLY_MY_UPLOAD")
            .await
            .unwrap();
        assert_eq!(proposal["status"], "prepared");
        assert_eq!(proposal["preview"]["bytes"], 14);
        assert!(!proposal.to_string().contains("private-locator"));
        assert!(!proposal.to_string().contains("Saved B"));
        assert_eq!(proposal["resourceRefs"].as_array().unwrap().len(), 1);
        let replay = propose(&state, &auth, &id, "own-upload.bin", b"ONLY_MY_UPLOAD")
            .await
            .unwrap();
        assert_eq!(proposal, replay);
        assert_eq!(
            propose(&state, &auth, &id, "another.bin", b"ONLY_MY_UPLOAD")
                .await
                .unwrap_err()
                .status,
            StatusCode::CONFLICT
        );
        let result = commit(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Commit {
                collection: "Owned".into(),
                operation_id: id,
                plan_id: proposal["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(result["status"], "committed");
        assert_eq!(result["resourceRefs"], proposal["resourceRefs"]);
        let inner = state.inner.lock().unwrap();
        let new_asset = result["assets"][0].as_str().unwrap();
        assert_eq!(
            media::resolve_asset(&inner, "alice", new_asset)
                .err()
                .unwrap()
                .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read(inner.config.root.join("Shared/own-upload.bin")).unwrap(),
            b"ONLY_MY_UPLOAD"
        );
        assert_eq!(qualify(&inner, "Owned").unwrap().entries.len(), 2);
    }
    #[tokio::test]
    async fn settled_unknown_and_prepared_imports_never_install_late_requests() {
        let (_temp, state, auth, _asset) = fixture(true, true);
        let missing = uuid::Uuid::new_v4().to_string();
        let settled = settle(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Operation {
                collection: "Owned".into(),
                operation_id: missing.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(settled["status"], "cancelled");
        let late = propose(&state, &auth, &missing, "late.bin", b"late")
            .await
            .unwrap();
        assert_eq!(late["status"], "cancelled");
        assert!(late["assets"].as_array().unwrap().is_empty());
        let known = uuid::Uuid::new_v4().to_string();
        let prepared = propose(&state, &auth, &known, "cancelled.bin", b"late")
            .await
            .unwrap();
        assert_eq!(prepared["status"], "prepared");
        let settled = settle(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Operation {
                collection: "Owned".into(),
                operation_id: known.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(settled["status"], "cancelled");
        assert_eq!(
            propose(&state, &auth, &known, "cancelled.bin", b"late")
                .await
                .unwrap()["status"],
            "cancelled"
        );
        let inner = state.inner.lock().unwrap();
        assert!(!inner.config.root.join("Shared/late.bin").exists());
        assert!(!inner.config.root.join("Shared/cancelled.bin").exists());
    }
    #[tokio::test]
    async fn original_owner_remap_never_exposes_or_commits_old_import_plan() {
        let (_temp, state, auth, _asset) = fixture(true, true);
        let id = uuid::Uuid::new_v4().to_string();
        let proposal = propose(&state, &auth, &id, "old-plan.bin", b"old")
            .await
            .unwrap();
        {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let owner = inner
                .core
                .qualified_node_at(&root, &root.join("Target/Target.typ"))
                .unwrap()
                .owner;
            let c = inner.config.resource_collections.get_mut("Owned").unwrap();
            c.owner = "Target".into();
            c.expected_owner = Some(owner);
        }
        let replay = propose(&state, &auth, &id, "old-plan.bin", b"old")
            .await
            .unwrap();
        assert_eq!(replay["status"], "prepared");
        assert_eq!(replay["preview"], json!({}));
        assert_eq!(replay["commitAllowed"], false);
        assert!(replay["assets"].as_array().unwrap().is_empty());
        assert_eq!(
            commit(
                State(state.clone()),
                Extension(auth.clone()),
                crate::RequestJson(Commit {
                    collection: "Owned".into(),
                    operation_id: id,
                    plan_id: proposal["planId"].as_str().unwrap().into()
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::CONFLICT
        );
        assert!(
            !state
                .inner
                .lock()
                .unwrap()
                .config
                .root
                .join("Shared/old-plan.bin")
                .exists()
        );
    }
}
