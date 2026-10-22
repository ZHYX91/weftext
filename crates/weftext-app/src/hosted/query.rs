//! Query over explicit hosted resources, independently granted native products
//! and exact saved-definition revisions. No local inventory routes are reused.
use super::*;
use std::path::Path;
use weftext_core::{ConfigRole, SnapshotFile, SnapshotWitness};
use weftext_data::{DefinitionSelection, NativeValue, QueryPlan, SavedQueryBinding, Schema};

const PRODUCTS: [&str; 5] = ["title", "body", "headings", "types", "self"];
const QUERY_BYTES: usize = 32 * 1024 * 1024;

fn budget_exceeded() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "queryBudget",
        "查询的累计冻结输入或原生产物超过 32 MiB 预算，请缩小范围后明确重新执行。",
    )
}
pub(super) fn charge_bytes(used: &mut usize, bytes: usize) -> Result<(), ApiError> {
    let next = used.checked_add(bytes).ok_or_else(budget_exceeded)?;
    if next > QUERY_BYTES {
        return Err(budget_exceeded());
    }
    *used = next;
    Ok(())
}
fn snapshot_capture_bytes(witnesses: &[(String, SnapshotWitness)]) -> Result<usize, ApiError> {
    let mut bytes = 0;
    for (_, witness) in witnesses {
        // The visible cut bytes and the opaque original evidence coexist
        // during capture. Both allocations participate in the same cap.
        charge_bytes(&mut bytes, witness.retained_bytes())?;
        charge_bytes(&mut bytes, witness.retained_bytes())?;
    }
    Ok(bytes)
}
struct BudgetWriter<'a>(&'a mut usize);
impl std::io::Write for BudgetWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        charge_bytes(self.0, bytes.len())
            .map_err(|_| std::io::Error::other("query byte budget exceeded"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn charge_serialized(used: &mut usize, value: &impl Serialize) -> Result<(), ApiError> {
    // Count into a bounded sink rather than allocate a second large JSON copy.
    serde_json::to_writer(BudgetWriter(used), value).map_err(|_| budget_exceeded())
}
pub(super) fn serialized_query_bytes(value: &impl Serialize) -> Result<usize, ApiError> {
    let mut bytes = 0;
    charge_serialized(&mut bytes, value)?;
    Ok(bytes)
}
fn base64_bytes(bytes: usize) -> Result<usize, ApiError> {
    bytes
        .checked_add(2)
        .and_then(|n| (n / 3).checked_mul(4))
        .ok_or_else(budget_exceeded)
}
fn worker_capture_bytes(
    source_bytes: usize,
    path_bytes: usize,
    root_bytes: usize,
    inputs: impl IntoIterator<Item = (usize, usize)>,
) -> Result<usize, ApiError> {
    let mut used = 0;
    // The raw source and its frozen base64 copy coexist. Allow for the request,
    // owned paths and map entries, and charge shared inputs once per worker.
    for bytes in [
        4096,
        source_bytes,
        base64_bytes(source_bytes)?,
        path_bytes,
        path_bytes,
        root_bytes,
    ] {
        charge_bytes(&mut used, bytes)?;
    }
    for (path_bytes, bytes) in inputs {
        for bytes in [256, path_bytes, base64_bytes(bytes)?] {
            charge_bytes(&mut used, bytes)?;
        }
    }
    Ok(used)
}
fn frozen_input_bytes(
    input: &weftext_data::FrozenQueryInput,
    metadata: &BTreeMap<String, weftext_data::AuthorizedQueryMetadata>,
    schema: &Schema,
    plan: &QueryPlan,
) -> Result<usize, ApiError> {
    let mut used = 0;
    charge_serialized(&mut used, &(input, schema, plan))?;
    if let Some(tags) = input
        .advanced
        .as_ref()
        .and_then(|a| a.tag_structure.as_ref())
    {
        charge_bytes(&mut used, tags.retained_bytes())?;
    }
    for note in &input.notes {
        // The trusted field capability is deliberately skipped by wire serde.
        // It still owns memory and must participate in the execution budget.
        charge_serialized(&mut used, &note.authorized_fields)?;
    }
    for (key, m) in metadata {
        charge_serialized(&mut used, &(key, &m.identity, &m.types, &m.resolved_types))?;
    }
    Ok(used)
}
fn take_witnesses(
    inner: &mut HostedInner,
    files: &[SnapshotFile],
) -> Result<Vec<(String, SnapshotWitness)>, ApiError> {
    let mut witnesses = vec![];
    let mut failed = false;
    for file in files {
        match inner.core.take_snapshot_witness(&file.base_id) {
            Ok(witness) => witnesses.push((file.path.clone(), witness)),
            Err(_) => failed = true,
        }
    }
    if failed {
        return Err(changed());
    }
    Ok(witnesses)
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct QueryRights {
    #[serde(default)]
    pub(super) definitions: bool,
    #[serde(default)]
    pub(super) definition_history: bool,
    #[serde(default)]
    pub(super) products: BTreeMap<String, bool>,
    #[serde(default)]
    pub(super) relations: BTreeSet<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Authority {
    #[serde(default)]
    pub(super) tag_structure: bool,
    pub(super) execution: library::ExecutionAuthority,
    pub(super) scopes: query_scopes::Authority,
    pub(super) providers: query_providers::Selection,
    pub(super) provider_dependencies: weftext_data::QueryProviderDependencies,
    pub(super) fields: BTreeMap<String, BTreeSet<String>>,
    pub(super) products: BTreeMap<String, BTreeSet<String>>,
    pub(super) definitions: BTreeMap<String, bool>,
    pub(super) relations: BTreeMap<String, BTreeSet<String>>,
    pub(super) inputs: BTreeSet<String>,
}
pub(super) fn authorize_authority(
    inner: &HostedInner,
    actor: &str,
    authority: &Authority,
) -> Result<(), ApiError> {
    if authority.tag_structure {
        navigation_registry::authorize_tag_read(inner, actor)?;
    }
    library::authorize_execution(inner, actor, &authority.execution)?;
    query_providers::authorize(inner, actor, &authority.providers)?;
    query_scopes::authorize(inner, actor, &authority.scopes)?;
    let access = |resource: &str| {
        if inner.file(resource).is_err() {
            return Err(unavailable());
        }
        inner.grant(actor, resource)
    };
    for (resource, fields) in &authority.fields {
        let rights = access(resource)?;
        if fields
            .iter()
            .any(|f| !rights.fields.get(f).is_some_and(|r| r.read))
        {
            return Err(unavailable());
        }
    }
    for (resource, products) in &authority.products {
        let rights = access(resource)?;
        if products
            .iter()
            .any(|p| !rights.query.products.get(p).copied().unwrap_or(false))
        {
            return Err(unavailable());
        }
    }
    for (resource, history) in &authority.definitions {
        let rights = access(resource)?;
        if !rights.query.definitions || *history && !rights.query.definition_history {
            return Err(unavailable());
        }
    }
    for (resource, relations) in &authority.relations {
        let rights = access(resource)?;
        if !relations.is_subset(&rights.query.relations) {
            return Err(unavailable());
        }
    }
    for resource in &authority.inputs {
        inputs::access(inner, actor, resource)?;
    }
    Ok(())
}
pub(super) fn valid_rights(rights: &QueryRights) -> bool {
    (!rights.definition_history || rights.definitions)
        && rights
            .products
            .keys()
            .all(|p| PRODUCTS.contains(&p.as_str()))
        && rights.relations.len() <= 64
        && rights
            .relations
            .iter()
            .all(|r| !r.is_empty() && r.len() <= 256)
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct QueryRequest {
    #[serde(default, deserialize_with = "query_providers::unique_fingerprints")]
    pub(super) providers: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "query_providers::unique_fingerprints")]
    pub(super) series_policies: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) fields: BTreeSet<String>,
    pub(super) resources: Option<BTreeSet<String>>,
    #[serde(default)]
    pub(super) products: BTreeSet<String>,
    #[serde(default)]
    pub(super) relations: BTreeSet<String>,
    #[serde(default, deserialize_with = "data_api::unique_fields")]
    pub(super) parameters: BTreeMap<String, NativeValue>,
    pub(super) plan: QueryPlan,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Definitions {
    resource: String,
    selection: Option<DefinitionSelection>,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/query/catalog", get(catalog))
        .route("/api/hosted/query/definitions", post(definitions))
        .route("/api/hosted/query", post(run))
}
pub(super) fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "queryUnavailable",
        "查询的准确定义、版本或获权依据已变化，请保留输入并明确重新执行。",
    )
}
pub(super) fn invalid() -> ApiError {
    ApiError::bad("查询不可用：请检查获权字段、参数、表达式及预算")
}
fn source(file: &SnapshotFile) -> Result<String, ApiError> {
    if file.kind != "managed"
        || file.currentness != "accepted"
        || file.node_id.is_none()
        || file.revision.is_none()
    {
        return Err(changed());
    }
    String::from_utf8(file.bytes.clone().ok_or_else(changed)?).map_err(|_| changed())
}
fn definitions_access(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
    history: bool,
) -> Result<(), ApiError> {
    let authority = Authority {
        definitions: BTreeMap::from([(resource.into(), history)]),
        ..Default::default()
    };
    authorize_authority(inner, actor, &authority)?;
    if !inner.file(resource).ok().is_some_and(|f| f.editable) {
        return Err(unavailable());
    }
    structure::guard_slot(inner, resource)?;
    Ok(())
}
fn read_definitions(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    selection: Option<&DefinitionSelection>,
) -> Result<Value, ApiError> {
    let pinned = matches!(selection, Some(DefinitionSelection::Pinned { .. }));
    definitions_access(inner, actor, resource, pinned)?;
    let path = inner.config.root.join(&inner.file(resource)?.path);
    let root = inner.config.root.clone();
    let snapshot = inner
        .core
        .read_snapshot(&root, &[path.to_string_lossy().into_owned()])
        .map_err(|_| changed())?
        .files
        .remove(0);
    let witnesses = take_witnesses(inner, std::slice::from_ref(&snapshot))?;
    let current = source(&snapshot)?;
    let owner = snapshot.node_id.as_ref().unwrap();
    let content = match selection {
        Some(DefinitionSelection::Pinned { revision, .. }) => String::from_utf8(
            inner
                .core
                .read_source_revision_at(&root, &path, owner, revision)
                .map_err(|_| changed())?,
        )
        .map_err(|_| changed())?,
        _ => current,
    };
    let candidates = weftext_data::saved_queries(&content)
        .map_err(|_| changed())?
        .into_iter()
        .filter(|d| {
            selection.is_none_or(|s| match s {
                DefinitionSelection::Current { key } | DefinitionSelection::Pinned { key, .. } => {
                    &d.key == key
                }
            })
        })
        .collect::<Vec<_>>();
    if selection.is_some() && candidates.len() != 1 {
        return Err(changed());
    }
    let mut definitions = vec![];
    let mut unavailable = vec![];
    for definition in candidates {
        if inner
            .core
            .is_definition_key_admitted(&root, owner, &definition.key)
            .map_err(|_| changed())?
        {
            definitions.push(definition);
        } else if selection.is_some() {
            return Err(changed());
        } else {
            unavailable.push(
                json!({"key":definition.key,"label":definition.label,"status":"unavailable"}),
            );
        }
    }
    inner
        .core
        .validate_snapshot_witness(&root, &snapshot.path, &witnesses[0].1)
        .map_err(|_| changed())?;
    let revision = match selection {
        Some(DefinitionSelection::Pinned { revision, .. }) => Some(revision.clone()),
        _ => snapshot.revision,
    };
    Ok(
        json!({"resource":resource,"ownerRevision":revision,"definitions":definitions,"selection":selection,
            "coverage":if unavailable.is_empty(){"complete"}else{"partial"},"unavailable":unavailable}),
    )
}
async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let grants = inner.grants_view(&actor)?;
        let mut resources = vec![];
        let mut definitions = vec![];
        let mut unavailable_definitions = vec![];
        let mut form_fields = BTreeSet::new();
        for (resource, rights) in grants {
            let Some(file) = inner.file(&resource).ok().filter(|f| f.editable || f.lifecycle=="trash" && inner.policy.users[&actor].query_scopes.trash) else { continue; };
            let fields = rights.fields.iter().filter(|(_, r)| r.read).map(|(id, _)| id.clone()).collect::<Vec<_>>();
            form_fields.extend(fields.iter().cloned());
            if fields.is_empty() && !rights.query.definitions && !rights.query.products.values().any(|p| *p) && rights.query.relations.is_empty() { continue; }
            resources.push(json!({"resource":resource,"title":file.title,"lifecycle":file.lifecycle,"readOnly":file.lifecycle=="trash","fields":fields,
                "products":PRODUCTS.iter().map(|p|(*p,rights.query.products.get(*p).copied().unwrap_or(false))).collect::<BTreeMap<_,_>>(),
                "relations":rights.query.relations,"definitions":rights.query.definitions,"definitionHistory":rights.query.definition_history}));
            if rights.query.definitions && file.lifecycle=="live" {
                match read_definitions(inner, &actor, &resource, None) {
                    Ok(result) => {
                        for definition in result["definitions"].as_array().ok_or_else(changed)? {
                            let mut definition = definition.clone();
                            definition["resource"] = json!(resource);
                            definition["revision"] = result["ownerRevision"].clone();
                            definitions.push(definition);
                        }
                        if result["coverage"] == "partial" { unavailable_definitions.push(json!({"resource":resource,"status":"unavailable"})); }
                    }
                    Err(_) => unavailable_definitions.push(json!({"resource":resource,"status":"unavailable"})),
                }
            }
        }
        let provider_catalog = provider_catalog(inner, &actor);
        let scope_catalog = query_scopes::catalog(inner,&actor);
        let mut result = json!({"resources":resources,"definitions":definitions,"coverage":if unavailable_definitions.is_empty()&&provider_catalog["providerCoverage"]=="complete"&&scope_catalog["scopeCoverage"]=="complete"{"complete"}else{"partial"},"unavailableDefinitions":unavailable_definitions});
        // Field-read grants already disclose these exact shapes. Do not expose
        // unrelated fields, type domains, relations or Provider descriptions.
        if !form_fields.is_empty() {
            let root = inner.config.root.clone();
            let projected = (|| {
                let config = inner.core.read_config(&root, ConfigRole::Schema).map_err(|_| changed())?;
                let witness = inner.core.take_snapshot_witness(&config.base_id).map_err(|_| changed())?;
                let schema = match config.bytes { Some(bytes) if config.revision.is_some() => Schema::from_author_json(std::str::from_utf8(&bytes).map_err(|_| changed())?).map_err(|_| changed())?, Some(_) => return Err(changed()), None => Schema::bundled().map_err(|_| changed())? };
                let narrowed = restricted_schema(&schema, &form_fields, &BTreeSet::new(), &BTreeSet::new())?;
                inner.core.validate_snapshot_witness(&root, &config.path, &witness).map_err(|_| changed())?;
                serde_json::to_value(narrowed).map_err(|_| changed())
            })();
            result["formSchema"] = projected.unwrap_or(Value::Null);
        }
        result.as_object_mut().unwrap().extend(provider_catalog.as_object().unwrap().clone());
        result.as_object_mut().unwrap().extend(scope_catalog.as_object().unwrap().clone());
        Ok(Json(result))
    }).await
}
fn provider_catalog(inner: &mut HostedInner, actor: &str) -> Value {
    let rights = inner.policy.users[actor].query_providers.clone();
    if rights.providers.is_empty() && rights.series_policies.is_empty() {
        return query_providers::unavailable_catalog(&rights);
    }
    let root = inner.config.root.clone();
    let result = (|| {
        let config = inner
            .core
            .read_config(&root, ConfigRole::Schema)
            .map_err(|_| changed())?;
        let witness = inner
            .core
            .take_snapshot_witness(&config.base_id)
            .map_err(|_| changed())?;
        let schema = match config.bytes {
            Some(bytes) if config.revision.is_some() => {
                Schema::from_author_json(std::str::from_utf8(&bytes).map_err(|_| changed())?)
                    .map_err(|_| changed())?
            }
            Some(_) => return Err(changed()),
            None => Schema::bundled().map_err(|_| changed())?,
        };
        let result = query_providers::catalog(&schema, &rights)?;
        inner
            .core
            .validate_snapshot_witness(&root, &config.path, &witness)
            .map_err(|_| changed())?;
        Ok(result)
    })();
    result.unwrap_or_else(|_: ApiError| query_providers::unavailable_catalog(&rights))
}
async fn definitions(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Definitions>,
) -> ApiResult {
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        Ok(Json(read_definitions(
            inner,
            &actor,
            &req.resource,
            req.selection.as_ref(),
        )?))
    })
    .await
}
async fn run(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<QueryRequest>,
) -> ApiResult {
    Ok(Json(execute(&state, &auth, req).await?))
}
#[derive(Clone)]
struct Draft {
    resource: String,
    source: String,
    binding: SavedQueryBinding,
    session_id: Option<String>,
    serial: Option<u64>,
}
struct Capture {
    actor: String,
    epoch: u64,
    authority: Authority,
    config_hash: String,
    witnesses: Vec<(String, SnapshotWitness)>,
    inputs: inputs::Captured,
    scopes: query_scopes::Capture,
    input: weftext_data::FrozenQueryInput,
    schema: Schema,
    plan: QueryPlan,
    metadata: BTreeMap<String, weftext_data::AuthorizedQueryMetadata>,
    columns: Vec<String>,
    workers: Vec<(String, WorkerRequest)>,
    environment: Option<library::Environment>,
    products: BTreeSet<String>,
    resource_map: BTreeMap<String, String>,
    draft: Option<Draft>,
}
fn all_resources(authority: &Authority) -> BTreeSet<String> {
    authority
        .fields
        .keys()
        .chain(authority.products.keys())
        .chain(authority.definitions.keys())
        .chain(authority.relations.keys())
        .chain(&authority.inputs)
        .cloned()
        .collect()
}
fn row_access(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
    req: &QueryRequest,
) -> Result<(), ApiError> {
    let authority = Authority {
        fields: BTreeMap::from([(resource.into(), req.fields.clone())]),
        products: BTreeMap::from([(resource.into(), req.products.clone())]),
        relations: BTreeMap::from([(resource.into(), req.relations.clone())]),
        ..Default::default()
    };
    authorize_authority(inner, actor, &authority)?;
    let rights = inner.grant(actor, resource)?;
    if !inner
        .file(resource)
        .ok()
        .is_some_and(|f| f.editable && f.lifecycle == "live" || f.lifecycle == "trash")
        || (!rights.fields.values().any(|f| f.read)
            && !rights.query.products.values().any(|p| *p)
            && !rights.query.definitions
            && rights.query.relations.is_empty())
    {
        return Err(unavailable());
    }
    Ok(())
}
fn request_budget(req: &QueryRequest) -> Result<(), ApiError> {
    query_providers::budget(&query_providers::Selection {
        providers: req.providers.clone(),
        series_policies: req.series_policies.clone(),
    })?;
    if req.fields.len() > 64
        || req.products.iter().any(|p| !PRODUCTS.contains(&p.as_str()))
        || req.relations.len() > 64
        || req.resources.as_ref().is_some_and(|r| r.len() > 64)
        || req.parameters.len() > 64
        || serde_json::to_vec(&req.plan).map_err(|_| invalid())?.len() > 256 * 1024
    {
        return Err(invalid());
    }
    fn visit(plan: &QueryPlan, nodes: &mut usize, depth: usize) -> Result<(), ApiError> {
        *nodes += 1;
        if *nodes > 1024
            || depth > 64
            || !matches!(plan.scope.lifecycle.as_str(), "" | "live" | "trash")
            || plan.take.is_some_and(|n| n > 500)
        {
            return Err(invalid());
        }
        if let Some(a) = &plan.advanced {
            for p in &a.union_all {
                visit(p, nodes, depth + 1)?;
            }
            if let Some(r) = &a.related {
                visit(&r.from, nodes, depth + 1)?;
            }
        }
        Ok(())
    }
    visit(&req.plan, &mut 0, 0)
}
fn selected_key(selection: &DefinitionSelection) -> &str {
    match selection {
        DefinitionSelection::Current { key } | DefinitionSelection::Pinned { key, .. } => key,
    }
}
fn schema_from(file: &SnapshotFile) -> Result<Schema, ApiError> {
    match file.bytes.as_ref() {
        Some(bytes) if file.revision.is_some() => {
            Schema::from_author_json(std::str::from_utf8(bytes).map_err(|_| changed())?)
                .map_err(|_| changed())
        }
        Some(_) => Err(changed()),
        None => Schema::bundled().map_err(|_| changed()),
    }
}
fn restricted_schema(
    schema: &Schema,
    fields: &BTreeSet<String>,
    types: &BTreeSet<String>,
    relations: &BTreeSet<String>,
) -> Result<Schema, ApiError> {
    let mut result = if fields.is_empty() {
        let mut result = schema.clone();
        result.fields.clear();
        result.definitions.clear();
        result.types.clear();
        result.aliases.clear();
        result.relations.clear();
        result.providers.clear();
        result.validators.clear();
        result.series_policies.clear();
        result
    } else {
        weftext_data::narrow_query_schema(schema, fields).map_err(|_| invalid())?
    };
    result.types = schema
        .types
        .iter()
        .filter(|t| types.contains(&t.id))
        .cloned()
        .map(|mut t| {
            t.fields.retain(|f| fields.contains(&f.id));
            t.requires.retain(|id| types.contains(id));
            t.conflicts.retain(|id| types.contains(id));
            t.constraints.clear();
            t
        })
        .collect();
    result.aliases = schema
        .aliases
        .iter()
        .filter(|(_, id)| types.contains(*id))
        .map(|(alias, id)| (alias.clone(), id.clone()))
        .collect();
    result.relations = schema
        .relations
        .iter()
        .filter(|r| relations.contains(&r.id))
        .cloned()
        .collect();
    Ok(result)
}
struct ProviderProjection<'a> {
    fields: &'a BTreeSet<String>,
    types: &'a BTreeSet<String>,
    relations: &'a BTreeSet<String>,
    explicit: Option<&'a query_providers::Selection>,
}
fn provider_requirements(
    inner: &HostedInner,
    actor: &str,
    plan: &QueryPlan,
    schema: &Schema,
    context: &weftext_data::FrozenAdvancedInput,
    fields: &BTreeSet<String>,
    explicit: Option<&query_providers::Selection>,
) -> Result<weftext_data::QueryProviderRequirements, ApiError> {
    let inaccessible = || {
        if explicit.is_some() {
            invalid()
        } else {
            unavailable()
        }
    };
    query_providers::check_shape_names(schema, fields, plan).map_err(|_| inaccessible())?;
    for dependency in weftext_data::definition_dependencies(plan, &context.definitions)
        .map_err(|_| inaccessible())?
    {
        let owner = context
            .definitions
            .iter()
            .find(|owner| owner.owner == dependency.owner)
            .ok_or_else(inaccessible)?;
        let definition = weftext_data::select_definition(&dependency.selection, &owner.history)
            .map_err(|_| inaccessible())?;
        let baseline = context
            .schemas
            .iter()
            .find(|baseline| {
                baseline.schema == definition.binding.schema
                    && baseline.revision == definition.binding.revision
            })
            .ok_or_else(inaccessible)?;
        query_providers::check_shape_names(&baseline.projection, fields, &definition.plan)
            .map_err(|_| inaccessible())?;
    }
    let mut selected_context = context.clone();
    let selected_schema = if let Some(selected) = explicit {
        for baseline in &mut selected_context.schemas {
            baseline.projection =
                query_providers::selected_schema(inner, actor, &baseline.projection, selected)?;
        }
        query_providers::selected_schema(inner, actor, schema, selected)?
    } else {
        schema.clone()
    };
    let requirements = if explicit.is_some() {
        weftext_data::query_provider_runtime_requirements(plan, &selected_schema, &selected_context)
    } else {
        weftext_data::query_provider_requirements(plan, &selected_schema, &selected_context)
    }
    .map_err(|_| {
        if explicit.is_some() {
            invalid()
        } else {
            unavailable()
        }
    })?;
    let mut joined = weftext_data::QueryProviderRequirements::default();
    let mut fingerprints = query_providers::Selection::default();
    for requirement in requirements {
        query_providers::merge(
            &mut fingerprints,
            &query_providers::Selection {
                providers: requirement.providers,
                series_policies: requirement.series_policies,
            },
        )?;
        joined.provider_calls.extend(requirement.provider_calls);
        joined.validators.extend(requirement.validators);
        joined.definitions.extend(requirement.definitions);
        joined
            .dynamic_selectors
            .extend(requirement.dynamic_selectors);
    }
    joined.providers = fingerprints.providers;
    joined.series_policies = fingerprints.series_policies;
    Ok(joined)
}
fn restricted_provider_projection(
    inner: &HostedInner,
    actor: &str,
    original: &Schema,
    demand: &weftext_data::QueryProviderRequirements,
    projection: &ProviderProjection<'_>,
) -> Result<(Schema, query_providers::Selection), ApiError> {
    let mut schema = restricted_schema(
        original,
        projection.fields,
        projection.types,
        projection.relations,
    )?;
    let authority = query_providers::project(
        inner,
        actor,
        original,
        &mut schema,
        demand,
        projection.explicit,
    )?;
    Ok((schema, authority))
}
fn owner_resource(
    inner: &mut HostedInner,
    actor: &str,
    owner: &str,
    workspace: &str,
) -> Result<String, ApiError> {
    if inner.file(owner).is_ok() {
        definitions_access(inner, actor, owner, false)?;
        return Ok(owner.into());
    }
    if !owner.starts_with(&format!("weftext-node://{workspace}/")) {
        return Err(unavailable());
    }
    // Native definition calls retain NodeRefs. Search only explicitly permitted
    // configured definition owners, never a library inventory or ambient path.
    let available_grants = inner.grants_view(actor)?;
    let candidates = available_grants
        .iter()
        .filter(|(r, g)| g.query.definitions && inner.file(r).ok().is_some_and(|f| f.editable))
        .map(|(r, _)| r.clone())
        .collect::<Vec<_>>();
    let mut found = vec![];
    for batch in candidates.chunks(64) {
        if batch.is_empty() {
            continue;
        }
        let paths = batch
            .iter()
            .map(|r| {
                Ok(inner
                    .config
                    .root
                    .join(&inner.file(r)?.path)
                    .to_string_lossy()
                    .into_owned())
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        let files = inner
            .core
            .read_snapshot(&inner.config.root, &paths)
            .map_err(|_| changed())?
            .files;
        let _witnesses = take_witnesses(inner, &files)?;
        for (resource, file) in batch.iter().zip(files) {
            if file.kind == "managed"
                && file.currentness == "accepted"
                && file
                    .node_id
                    .as_ref()
                    .is_some_and(|id| owner == format!("weftext-node://{workspace}/{id}"))
            {
                found.push(resource.clone());
            }
        }
    }
    if found.len() != 1 {
        return Err(unavailable());
    }
    Ok(found.remove(0))
}
struct DefinitionContext<'a> {
    workspace: &'a str,
    role: &'a str,
    draft: Option<&'a Draft>,
    draft_keys: Option<&'a BTreeSet<String>>,
}
fn freeze_definitions(
    inner: &mut HostedInner,
    actor: &str,
    plan: &QueryPlan,
    snapshots: &BTreeMap<String, SnapshotFile>,
    selected: DefinitionContext<'_>,
    owners: &mut BTreeMap<String, String>,
    authority: &mut Authority,
) -> Result<(weftext_data::FrozenAdvancedInput, bool), ApiError> {
    let DefinitionContext {
        workspace,
        role,
        draft,
        draft_keys,
    } = selected;
    let mut context = weftext_data::FrozenAdvancedInput::default();
    let mut baselines = BTreeMap::new();
    for _ in 0..64 {
        let dependencies = weftext_data::definition_dependencies(plan, &context.definitions)
            .map_err(|_| invalid())?;
        let mut progressed = false;
        for dependency in dependencies {
            let resource = match owners.get(&dependency.owner) {
                Some(resource) => resource.clone(),
                None => {
                    let resource = owner_resource(inner, actor, &dependency.owner, workspace)?;
                    owners.insert(dependency.owner.clone(), resource);
                    return Ok((context, true));
                }
            };
            let pinned = matches!(dependency.selection, DefinitionSelection::Pinned { .. });
            definitions_access(inner, actor, &resource, pinned)?;
            authority
                .definitions
                .entry(resource.clone())
                .and_modify(|h| *h |= pinned)
                .or_insert(pinned);
            let current = snapshots.get(&resource).ok_or_else(changed)?;
            let content = source(current)?;
            let owner_id = current.node_id.as_ref().unwrap();
            let key = selected_key(&dependency.selection);
            let local = draft.filter(|d| d.resource == resource && !pinned);
            if (local.is_none() || draft_keys.is_some_and(|keys| !keys.contains(key)))
                && !inner
                    .core
                    .is_definition_key_admitted(&inner.config.root, owner_id, key)
                    .map_err(|_| changed())?
            {
                return Err(changed());
            }
            let index = match context
                .definitions
                .iter()
                .position(|o| o.owner == dependency.owner)
            {
                Some(i) => i,
                None => {
                    context
                        .definitions
                        .push(weftext_data::FrozenDefinitionOwner {
                            owner: dependency.owner.clone(),
                            history: vec![weftext_data::DefinitionRevision {
                                revision: local.map_or_else(
                                    || current.revision.clone().unwrap(),
                                    |d| format!("draft-source:{}", digest(d.source.as_bytes())),
                                ),
                                source: local.map_or(content, |d| d.source.clone()),
                                current: true,
                            }],
                        });
                    progressed = true;
                    context.definitions.len() - 1
                }
            };
            if let DefinitionSelection::Pinned { revision, .. } = &dependency.selection
                && !context.definitions[index]
                    .history
                    .iter()
                    .any(|h| &h.revision == revision)
            {
                let path = inner.config.root.join(&inner.file(&resource)?.path);
                let bytes = inner
                    .core
                    .read_source_revision_at(&inner.config.root, &path, owner_id, revision)
                    .map_err(|_| changed())?;
                context.definitions[index]
                    .history
                    .push(weftext_data::DefinitionRevision {
                        revision: revision.clone(),
                        source: String::from_utf8(bytes).map_err(|_| changed())?,
                        current: false,
                    });
                progressed = true;
            }
            let definition = weftext_data::select_definition(
                &dependency.selection,
                &context.definitions[index].history,
            )
            .map_err(|_| changed())?;
            request_budget(&QueryRequest {
                providers: BTreeMap::new(),
                series_policies: BTreeMap::new(),
                fields: BTreeSet::new(),
                resources: None,
                products: BTreeSet::new(),
                relations: BTreeSet::new(),
                parameters: BTreeMap::new(),
                plan: definition.plan.clone(),
            })?;
            if definition.binding.schema != role
                || definition.binding.language != "weftext.query.cel.v1"
            {
                return Err(changed());
            }
            let identity = (
                definition.binding.schema.clone(),
                definition.binding.revision.clone(),
            );
            if let std::collections::btree_map::Entry::Vacant(entry) = baselines.entry(identity) {
                let bytes = inner
                    .core
                    .read_config_revision(
                        &inner.config.root,
                        ConfigRole::Schema,
                        &definition.binding.revision,
                    )
                    .map_err(|_| changed())?;
                entry.insert(
                    Schema::from_author_json(std::str::from_utf8(&bytes).map_err(|_| changed())?)
                        .map_err(|_| changed())?,
                );
            }
        }
        if !progressed {
            context.schemas = baselines
                .into_iter()
                .map(
                    |((schema, revision), projection)| weftext_data::FrozenDefinitionSchema {
                        schema,
                        revision,
                        projection,
                    },
                )
                .collect();
            return Ok((context, false));
        }
    }
    Err(invalid())
}
pub(super) struct DefinitionValidationRequest<'a> {
    pub(super) resource: &'a str,
    pub(super) source: &'a str,
    pub(super) plan: &'a QueryPlan,
    pub(super) binding: Option<&'a SavedQueryBinding>,
    pub(super) draft_keys: &'a BTreeSet<String>,
}
pub(super) struct DefinitionValidation {
    pub(super) binding: SavedQueryBinding,
    pub(super) schema: Schema,
    pub(super) context: weftext_data::FrozenAdvancedInput,
    pub(super) plan: QueryPlan,
    pub(super) guard: DefinitionGuard,
}
fn canonical_definition_owners(
    plan: &mut QueryPlan,
    owners: &BTreeMap<String, String>,
) -> Result<(), ApiError> {
    if let Some(advanced) = &mut plan.advanced {
        if let Some(call) = &mut advanced.call {
            call.owner = owners.get(&call.owner).ok_or_else(changed)?.clone();
        }
        for child in &mut advanced.union_all {
            canonical_definition_owners(child, owners)?;
        }
        if let Some(related) = &mut advanced.related {
            canonical_definition_owners(&mut related.from, owners)?;
        }
    }
    Ok(())
}
pub(super) struct DefinitionGuard {
    actor: String,
    epoch: u64,
    authority: Authority,
    config_hash: String,
    pub(super) witnesses: Vec<(String, SnapshotWitness)>,
    scopes: query_scopes::Capture,
    scope_snapshots: Vec<SnapshotWitness>,
    pub(super) owner_path: PathBuf,
}
impl DefinitionGuard {
    pub(super) fn recheck(&self, inner: &HostedInner, auth: &Auth) -> Result<(), ApiError> {
        let actor = inner.actor(auth)?;
        if actor != self.actor || inner.actor_generation(&actor) != self.epoch {
            return Err(expired());
        }
        authorize_authority(inner, &actor, &self.authority)?;
        self.scopes.recheck(inner, &actor)?;
        if office::binding_hash(inner, &all_resources(&self.authority))? != self.config_hash {
            return Err(changed());
        }
        for (path, witness) in &self.witnesses {
            inner
                .core
                .validate_snapshot_witness(&inner.config.root, path, witness)
                .map_err(|_| changed())?;
        }
        Ok(())
    }
    pub(super) fn snapshot_witnesses(&self) -> Vec<&SnapshotWitness> {
        self.witnesses
            .iter()
            .map(|(_, w)| w)
            .chain(&self.scope_snapshots)
            .collect()
    }
    pub(super) fn merge_witnesses(
        &mut self,
        pool: &mut Vec<SnapshotWitness>,
    ) -> Result<(), CoreError> {
        Core::merge_snapshot_witnesses(
            pool,
            std::mem::take(&mut self.witnesses)
                .into_iter()
                .map(|(_, w)| w)
                .chain(std::mem::take(&mut self.scope_snapshots))
                .collect(),
        )
    }
}
fn static_authority(
    grants: &BTreeMap<String, Rights>,
    configured: &BTreeSet<String>,
    demands: &weftext_data::QueryReadRequirements,
) -> Result<Authority, ApiError> {
    let mut authority = Authority::default();
    for field in &demands.fields {
        let (resource, _) = grants
            .iter()
            .find(|(r, g)| configured.contains(*r) && g.fields.get(field).is_some_and(|f| f.read))
            .ok_or_else(unavailable)?;
        authority
            .fields
            .entry(resource.clone())
            .or_default()
            .insert(field.clone());
    }
    for product in &demands.products {
        let (resource, _) = grants
            .iter()
            .find(|(r, g)| {
                configured.contains(*r) && g.query.products.get(product).copied().unwrap_or(false)
            })
            .ok_or_else(unavailable)?;
        authority
            .products
            .entry(resource.clone())
            .or_default()
            .insert(product.clone());
    }
    for relation in &demands.relations {
        let (resource, _) = grants
            .iter()
            .find(|(r, g)| configured.contains(*r) && g.query.relations.contains(relation))
            .ok_or_else(unavailable)?;
        authority
            .relations
            .entry(resource.clone())
            .or_default()
            .insert(relation.clone());
    }
    if demands.type_filter && !demands.products.contains("types") {
        return Err(unavailable());
    }
    Ok(authority)
}
/// Type-check a saved carrier without fetching query rows, evaluating parameters
/// or compiling body products. Only the configured owner/schema and actual
/// definition calls enter the immutable read closure.
pub(super) fn validate_definition(
    inner: &mut HostedInner,
    auth: &Auth,
    request: DefinitionValidationRequest<'_>,
) -> Result<DefinitionValidation, ApiError> {
    let DefinitionValidationRequest {
        resource,
        source: draft_source,
        plan,
        binding,
        draft_keys,
    } = request;
    let actor = inner.actor(auth)?;
    definitions_access(inner, &actor, resource, false)?;
    if !inner.rights(&actor, resource)?.write {
        return Err(unavailable());
    }
    writing_api::validate_source(draft_source, None)?;
    request_budget(&QueryRequest {
        providers: BTreeMap::new(),
        series_policies: BTreeMap::new(),
        fields: BTreeSet::new(),
        resources: None,
        products: BTreeSet::new(),
        relations: BTreeSet::new(),
        parameters: BTreeMap::new(),
        plan: plan.clone(),
    })?;
    query_scopes::preauthorize(inner, &actor, plan)?;
    let config = inner
        .core
        .read_config(&inner.config.root, ConfigRole::Schema)
        .map_err(|_| changed())?;
    drop(
        inner
            .core
            .take_snapshot_witness(&config.base_id)
            .map_err(|_| changed())?,
    );
    let workspace = config
        .role_ref
        .strip_prefix("weftext-config://")
        .and_then(|s| s.strip_suffix("/schema"))
        .ok_or_else(changed)?
        .to_owned();
    let owner_path = inner.config.root.join(&inner.file(resource)?.path);
    let mut owners = BTreeMap::new();
    let mut tag_needed = false;
    for _ in 0..64 {
        let resources = owners
            .values()
            .cloned()
            .chain([resource.to_owned()])
            .collect::<BTreeSet<String>>();
        if resources.len() >= 64 {
            return Err(invalid());
        }
        let mut paths = vec![config.path.clone()];
        if tag_needed {
            navigation_registry::authorize_tag_read(inner, &actor)?;
            paths.push(
                inner
                    .config
                    .root
                    .join(".weftext-tags.json")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        for r in &resources {
            paths.push(
                inner
                    .config
                    .root
                    .join(&inner.file(r)?.path)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        let mut files = inner
            .core
            .read_snapshot(&inner.config.root, &paths)
            .map_err(|_| changed())?
            .files;
        let tags = if tag_needed {
            let document = inner
                .core
                .qualified_tag_document_from_snapshot(&inner.config.root, &files[1])
                .map_err(|_| changed())?;
            Some(
                weftext_data::FrozenTagStructure::new(workspace.clone(), document)
                    .map_err(|_| changed())?,
            )
        } else {
            None
        };
        let witnesses = take_witnesses(inner, &files)?;
        if tag_needed {
            files.remove(1);
        }
        let schema_file = files.remove(0);
        // Persisted definitions always have an admitted, exact config revision;
        // the bundled default is only an execution/read fallback.
        let current_revision = schema_file.revision.as_ref().ok_or_else(changed)?;
        let current_schema = schema_from(&schema_file)?;
        let binding = binding.cloned().unwrap_or(SavedQueryBinding {
            schema: config.role_ref.clone(),
            revision: current_revision.clone(),
            language: "weftext.query.cel.v1".into(),
        });
        if binding.schema != config.role_ref || binding.language != "weftext.query.cel.v1" {
            return Err(changed());
        }
        let baseline = if binding.revision == *current_revision {
            current_schema.clone()
        } else {
            let bytes = inner
                .core
                .read_config_revision(&inner.config.root, ConfigRole::Schema, &binding.revision)
                .map_err(|_| changed())?;
            Schema::from_author_json(std::str::from_utf8(&bytes).map_err(|_| changed())?)
                .map_err(|_| changed())?
        };
        let files = resources.into_iter().zip(files).collect::<BTreeMap<_, _>>();
        let owner = files.get(resource).ok_or_else(changed)?;
        // The accepted Core identity remains authoritative even when an
        // ordinary save retained unfinished body syntax. Repairing that body
        // must not require its previous bytes to qualify as a Query carrier.
        source(owner)?;
        let draft_id = weftext_data::narrow_query_metadata(draft_source, &current_schema)
            .map_err(|_| changed())?
            .0;
        if owner.node_id.as_ref() != Some(&draft_id) {
            return Err(changed());
        }
        let draft = Draft {
            resource: resource.into(),
            source: draft_source.into(),
            binding: binding.clone(),
            session_id: None,
            serial: None,
        };
        let mut authority = Authority {
            definitions: BTreeMap::from([(resource.into(), false)]),
            inputs: BTreeSet::from([resource.into()]),
            ..Default::default()
        };
        let (mut context, expanded) = freeze_definitions(
            inner,
            &actor,
            plan,
            &files,
            DefinitionContext {
                workspace: &workspace,
                role: &config.role_ref,
                draft: Some(&draft),
                draft_keys: Some(draft_keys),
            },
            &mut owners,
            &mut authority,
        )?;
        if expanded {
            continue;
        }
        let mut scopes = query_scopes::capture(
            inner,
            &actor,
            plan,
            &mut context,
            snapshot_capture_bytes(&witnesses)?,
        )?;
        if scopes.needs_rules_guard() {
            for (r, file) in &files {
                scopes.qualify_node(inner, r, file, &workspace)?;
            }
        }
        authority.scopes = scopes.authority.clone();
        let demands = weftext_data::query_read_requirements(plan, &baseline, &context)
            .map_err(|_| unavailable())?;
        if demands.tag_structure != tag_needed {
            tag_needed = demands.tag_structure;
            continue;
        }
        context.tag_structure = tags;
        authority.tag_structure = demands.tag_structure;
        let configured = inner
            .files_view(&actor)?
            .iter()
            .filter(|(_, f)| {
                f.editable
                    || f.lifecycle == "trash" && inner.policy.users[&actor].query_scopes.trash
            })
            .map(|(id, _)| id.clone())
            .collect();
        let effects = static_authority(&inner.grants_view(&actor)?, &configured, &demands)?;
        authority.fields = effects.fields;
        authority.products = effects.products;
        authority.relations = effects.relations;
        authorize_authority(inner, &actor, &authority)?;
        let providers = provider_requirements(
            inner,
            &actor,
            plan,
            &baseline,
            &context,
            &demands.fields,
            None,
        )?;
        let projection = ProviderProjection {
            fields: &demands.fields,
            types: &demands.types,
            relations: &demands.relations,
            explicit: None,
        };
        let (schema, provider_authority) =
            restricted_provider_projection(inner, &actor, &baseline, &providers, &projection)?;
        query_providers::merge(&mut authority.providers, &provider_authority)?;
        let (current, _) = restricted_provider_projection(
            inner,
            &actor,
            &current_schema,
            &providers,
            &projection,
        )?;
        if !weftext_data::baseline_contract_compatible(&schema, &current).map_err(|_| changed())? {
            return Err(changed());
        }
        for selected in &mut context.schemas {
            let (schema, provider_authority) = restricted_provider_projection(
                inner,
                &actor,
                &selected.projection,
                &providers,
                &projection,
            )?;
            query_providers::merge(&mut authority.providers, &provider_authority)?;
            selected.projection = schema;
        }
        let canonical_owners = owners
            .iter()
            .map(|(selector, resource)| {
                let id = files[resource].node_id.as_ref().ok_or_else(changed)?;
                Ok((selector.clone(), format!("weftext-node://{workspace}/{id}")))
            })
            .collect::<Result<BTreeMap<_, _>, ApiError>>()?;
        let mut plan = plan.clone();
        canonical_definition_owners(&mut plan, &canonical_owners)?;
        query_scopes::canonicalize(inner, &actor, &mut plan)?;
        let mut canonical_context = Vec::<weftext_data::FrozenDefinitionOwner>::new();
        for mut owner in std::mem::take(&mut context.definitions) {
            owner.owner = canonical_owners
                .get(&owner.owner)
                .ok_or_else(changed)?
                .clone();
            if let Some(existing) = canonical_context
                .iter_mut()
                .find(|o| o.owner == owner.owner)
            {
                for revision in owner.history {
                    if let Some(old) = existing
                        .history
                        .iter()
                        .find(|r| r.revision == revision.revision)
                    {
                        if old.source != revision.source || old.current != revision.current {
                            return Err(changed());
                        }
                    } else {
                        existing.history.push(revision);
                    }
                }
            } else {
                canonical_context.push(owner);
            }
        }
        context.definitions = canonical_context;
        weftext_data::validate_query_plan_with_context(&plan, &schema, &context)
            .map_err(|_| unavailable())?;
        // Definitions, historical baselines and typed parameters share the
        // execution input cap even though this path never compiles any rows.
        let config_hash = office::binding_hash(inner, &all_resources(&authority))?;
        let epoch = inner.actor_generation(&actor);
        let mut scope_snapshots = scopes.guard_witnesses()?;
        if scopes.needs_rules_guard() {
            let rules = inner
                .core
                .read_snapshot(
                    &inner.config.root,
                    &[inner
                        .config
                        .root
                        .join(".weftext-rules")
                        .to_string_lossy()
                        .into_owned()],
                )
                .map_err(|_| changed())?
                .files
                .remove(0);
            let witness = inner
                .core
                .take_snapshot_witness(&rules.base_id)
                .map_err(|_| changed())?;
            if rules.bytes.is_none() {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "basisRequired",
                    "目录或 Trash 定义的保存保护需要现存的库规则文件；请由库所有者显式安装后重试。",
                ));
            }
            Core::merge_snapshot_witnesses(&mut scope_snapshots, vec![witness])
                .map_err(|_| changed())?;
        }
        let mut validation_bytes = scopes.retained_bytes();
        for witness in &scope_snapshots {
            charge_bytes(&mut validation_bytes, witness.retained_bytes())?;
        }
        charge_serialized(&mut validation_bytes, &(&context, &schema, &plan))?;
        if let Some(tags) = &context.tag_structure {
            charge_bytes(&mut validation_bytes, tags.retained_bytes())?;
        }
        return Ok(DefinitionValidation {
            binding,
            schema,
            context,
            plan,
            guard: DefinitionGuard {
                actor,
                epoch,
                authority,
                config_hash,
                witnesses,
                scopes,
                scope_snapshots,
                owner_path,
            },
        });
    }
    Err(invalid())
}
fn capture(
    inner: &mut HostedInner,
    auth: &Auth,
    mut req: QueryRequest,
    draft: Option<Draft>,
    validate_only: bool,
) -> Result<Capture, ApiError> {
    let actor = inner.actor(auth)?;
    request_budget(&req)?;
    query_scopes::preauthorize(inner, &actor, &req.plan)?;
    let selected_providers = query_providers::Selection {
        providers: req.providers.clone(),
        series_policies: req.series_policies.clone(),
    };
    // Perform this gate even for resources=[] and before any Schema discovery.
    query_providers::authorize(inner, &actor, &selected_providers)?;
    req.plan.take.get_or_insert(500);
    let grants = inner.grants_view(&actor)?;
    let all_file_entries = grants
        .keys()
        .map(|r| Ok((r.clone(), inner.file(r)?)))
        .collect::<Result<BTreeMap<_, _>, ApiError>>()?;
    for field in &req.fields {
        if !grants
            .iter()
            .any(|(r, g)| inner.file(r).is_ok() && g.fields.get(field).is_some_and(|f| f.read))
        {
            return Err(unavailable());
        }
    }
    for product in &req.products {
        if !grants.iter().any(|(r, g)| {
            inner.file(r).is_ok() && g.query.products.get(product).copied().unwrap_or(false)
        }) {
            return Err(unavailable());
        }
    }
    for relation in &req.relations {
        if !grants
            .iter()
            .any(|(r, g)| inner.file(r).is_ok() && g.query.relations.contains(relation))
        {
            return Err(unavailable());
        }
    }
    if let Some(d) = &draft {
        definitions_access(inner, &actor, &d.resource, false)?;
        inner.rights(&actor, &d.resource)?;
        writing_api::validate_source(&d.source, None)?;
    }
    let preliminary_lifecycles = weftext_data::required_query_lifecycles(&req.plan, &[]).ok();
    for resource in req.resources.iter().flatten() {
        row_access(inner, &actor, resource, &req)?;
    }
    let mut resources = match (&req.resources, &preliminary_lifecycles) {
        (_, None) => BTreeSet::new(),
        (Some(resources), Some(lifecycles)) => resources
            .iter()
            .filter(|r| lifecycles.contains(&all_file_entries[*r].lifecycle))
            .cloned()
            .collect(),
        (None, Some(lifecycles)) => grants
            .keys()
            .filter(|r| row_access(inner, &actor, r, &req).is_ok())
            .filter(|r| lifecycles.contains(&all_file_entries[*r].lifecycle))
            .cloned()
            .collect(),
    };
    for resource in &resources {
        row_access(inner, &actor, resource, &req)?;
    }
    let config = inner
        .core
        .read_config(&inner.config.root, ConfigRole::Schema)
        .map_err(|_| changed())?;
    // This preliminary role lookup is re-read as part of the final cut.
    drop(
        inner
            .core
            .take_snapshot_witness(&config.base_id)
            .map_err(|_| changed())?,
    );
    let workspace = config
        .role_ref
        .strip_prefix("weftext-config://")
        .and_then(|s| s.strip_suffix("/schema"))
        .ok_or_else(changed)?
        .to_owned();
    let mut owners = BTreeMap::new();
    let mut inputs = BTreeSet::new();
    let mut tag_needed = false;
    for _ in 0..64 {
        let mut authority = Authority::default();
        for r in &resources {
            authority.fields.insert(r.clone(), req.fields.clone());
            authority.products.insert(r.clone(), req.products.clone());
            authority.relations.insert(r.clone(), req.relations.clone());
        }
        if let Some(d) = &draft {
            authority.definitions.insert(d.resource.clone(), false);
            authority.inputs.insert(d.resource.clone());
        }
        let captured_inputs =
            inputs::capture_query(inner, &actor, &inputs.iter().cloned().collect::<Vec<_>>())?;
        let mut file_resources = resources
            .iter()
            .chain(owners.values())
            .chain(captured_inputs.ids())
            .cloned()
            .collect::<BTreeSet<_>>();
        if let Some(d) = &draft {
            file_resources.insert(d.resource.clone());
        }
        let mut paths = vec![config.path.clone()];
        if tag_needed {
            navigation_registry::authorize_tag_read(inner, &actor)?;
            paths.push(
                inner
                    .config
                    .root
                    .join(".weftext-tags.json")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        paths.extend(
            file_resources
                .iter()
                .map(|r| {
                    Ok(inner
                        .config
                        .root
                        .join(inputs::path(inner, &actor, r)?)
                        .to_string_lossy()
                        .into_owned())
                })
                .collect::<Result<Vec<_>, ApiError>>()?,
        );
        if paths.len() > 64 {
            return Err(invalid());
        }
        let mut cut = inner
            .core
            .read_snapshot(&inner.config.root, &paths)
            .map_err(|_| changed())?
            .files;
        let tags = if tag_needed {
            let document = inner
                .core
                .qualified_tag_document_from_snapshot(&inner.config.root, &cut[1])
                .map_err(|_| changed())?;
            Some(
                weftext_data::FrozenTagStructure::new(workspace.clone(), document)
                    .map_err(|_| changed())?,
            )
        } else {
            None
        };
        let witnesses = take_witnesses(inner, &cut)?;
        if tag_needed {
            cut.remove(1);
        }
        let schema_file = cut.remove(0);
        let full_schema = schema_from(&schema_file)?;
        let snapshots = file_resources
            .iter()
            .cloned()
            .zip(cut)
            .collect::<BTreeMap<_, _>>();
        let (mut context, expanded) = freeze_definitions(
            inner,
            &actor,
            &req.plan,
            &snapshots,
            DefinitionContext {
                workspace: &workspace,
                role: &config.role_ref,
                draft: draft.as_ref(),
                draft_keys: None,
            },
            &mut owners,
            &mut authority,
        )?;
        if expanded {
            continue;
        }
        let mut scopes = query_scopes::capture(
            inner,
            &actor,
            &req.plan,
            &mut context,
            snapshot_capture_bytes(&witnesses)?,
        )?;
        for r in owners.values().chain(draft.iter().map(|d| &d.resource)) {
            scopes.qualify_node(inner, r, snapshots.get(r).ok_or_else(changed)?, &workspace)?;
        }
        authority.scopes = scopes.authority.clone();
        let selected = match &req.resources {
            Some(selected) => selected
                .iter()
                .filter(|r| scopes.includes(&all_file_entries[*r].lifecycle))
                .cloned()
                .collect(),
            None => grants
                .keys()
                .filter(|r| {
                    row_access(inner, &actor, r, &req).is_ok()
                        && scopes.includes(&all_file_entries[*r].lifecycle)
                })
                .cloned()
                .collect(),
        };
        if resources != selected {
            resources = selected;
            continue;
        }
        authority.fields.retain(|r, _| resources.contains(r));
        authority.products.retain(|r, _| resources.contains(r));
        authority.relations.retain(|r, _| resources.contains(r));
        let mut root_schema = full_schema.clone();
        if let Some(d) = &draft {
            if d.binding.schema != config.role_ref || d.binding.language != "weftext.query.cel.v1" {
                return Err(changed());
            }
            let bytes = inner
                .core
                .read_config_revision(&inner.config.root, ConfigRole::Schema, &d.binding.revision)
                .map_err(|_| changed())?;
            root_schema =
                Schema::from_author_json(std::str::from_utf8(&bytes).map_err(|_| changed())?)
                    .map_err(|_| changed())?;
            let saved = source(snapshots.get(&d.resource).ok_or_else(changed)?)?;
            let current_id = weftext_data::narrow_query_metadata(&saved, &full_schema)
                .map_err(|_| changed())?
                .0;
            let draft_id = weftext_data::narrow_query_metadata(&d.source, &full_schema)
                .map_err(|_| changed())?
                .0;
            if current_id != draft_id || snapshots[&d.resource].node_id.as_ref() != Some(&draft_id)
            {
                return Err(changed());
            }
        }
        // Preserve legacy default field projection only at a direct root scan.
        // Union/call terminal columns belong to their actual producer.
        if req.plan.select.is_empty()
            && req.plan.group.is_none()
            && req
                .plan
                .advanced
                .as_ref()
                .is_none_or(|a| a.union_all.is_empty() && a.call.is_none())
            && !req.fields.is_empty()
        {
            req.plan.select = req
                .fields
                .iter()
                .map(|id| weftext_data::QueryColumn {
                    name: root_schema
                        .fields
                        .iter()
                        .find(|f| &f.id == id)
                        .map(|f| f.key.clone())
                        .unwrap_or_else(|| id.clone()),
                    expr: format!("field({})", serde_json::to_string(id).unwrap()),
                })
                .collect();
        }
        let demands = weftext_data::query_read_requirements(&req.plan, &root_schema, &context)
            .map_err(|_| {
                if draft.is_some() {
                    unavailable()
                } else {
                    invalid()
                }
            })?;
        if demands.tag_structure != tag_needed {
            tag_needed = demands.tag_structure;
            continue;
        }
        context.tag_structure = tags;
        authority.tag_structure = demands.tag_structure;
        if draft.is_some() {
            req.fields = demands.fields.clone();
            req.products = demands.products.clone();
            req.relations = demands.relations.clone();
            resources = grants
                .keys()
                .filter(|r| row_access(inner, &actor, r, &req).is_ok())
                .filter(|r| scopes.includes(&all_file_entries[*r].lifecycle))
                .cloned()
                .collect();
            if resources
                != file_resources
                    .iter()
                    .filter(|r| !owners.values().any(|o| o == *r) && !inputs.contains(*r))
                    .cloned()
                    .collect()
            {
                // Re-capture after the true draft dependency selection is known.
                for resource in &resources {
                    row_access(inner, &actor, resource, &req)?;
                }
                if !resources.iter().all(|r| snapshots.contains_key(r)) {
                    continue;
                }
            }
            authority.fields.clear();
            authority.products.clear();
            authority.relations.clear();
            for r in &resources {
                authority.fields.insert(r.clone(), req.fields.clone());
                authority.products.insert(r.clone(), req.products.clone());
                authority.relations.insert(r.clone(), req.relations.clone());
            }
        }
        if !demands.fields.is_subset(&req.fields) || !demands.relations.is_subset(&req.relations) {
            return Err(invalid());
        }
        if demands.type_filter && !req.products.contains("types") {
            return Err(unavailable());
        }
        for r in &resources {
            row_access(inner, &actor, r, &req)?;
        }
        // Even resources=[] uses a real explicit authorized object as evidence
        // for every requested field/product/relation, never schema discovery.
        if resources.is_empty() {
            for field in &req.fields {
                let r = grants
                    .iter()
                    .find(|(r, g)| {
                        inner.file(r).is_ok() && g.fields.get(field).is_some_and(|f| f.read)
                    })
                    .ok_or_else(unavailable)?
                    .0;
                authority
                    .fields
                    .entry(r.clone())
                    .or_default()
                    .insert(field.clone());
            }
            for p in &req.products {
                let r = grants
                    .iter()
                    .find(|(r, g)| {
                        inner.file(r).is_ok() && g.query.products.get(p).copied().unwrap_or(false)
                    })
                    .ok_or_else(unavailable)?
                    .0;
                authority
                    .products
                    .entry(r.clone())
                    .or_default()
                    .insert(p.clone());
            }
            for relation in &req.relations {
                let r = grants
                    .iter()
                    .find(|(r, g)| inner.file(r).is_ok() && g.query.relations.contains(relation))
                    .ok_or_else(unavailable)?
                    .0;
                authority
                    .relations
                    .entry(r.clone())
                    .or_default()
                    .insert(relation.clone());
            }
        }
        let mut needed_inputs = BTreeSet::new();
        if req
            .products
            .iter()
            .any(|p| matches!(p.as_str(), "title" | "body" | "headings"))
        {
            for r in &resources {
                needed_inputs.extend(inner.file(r)?.inputs.iter().cloned());
            }
        }
        for r in &needed_inputs {
            inputs::access(inner, &actor, r)?;
        }
        authority.inputs.extend(needed_inputs.clone());
        if needed_inputs != inputs {
            inputs = needed_inputs;
            continue;
        }
        authority.inputs.extend(captured_inputs.authority_ids());
        authorize_authority(inner, &actor, &authority)?;
        context.parameters = req.parameters.clone();
        let explicit = (!validate_only).then_some(&selected_providers);
        let providers = provider_requirements(
            inner,
            &actor,
            &req.plan,
            &root_schema,
            &context,
            &req.fields,
            explicit,
        )
        .map_err(|error| {
            if draft.is_some() && error.status == StatusCode::BAD_REQUEST {
                unavailable()
            } else {
                error
            }
        })?;
        let projection = ProviderProjection {
            fields: &req.fields,
            types: &demands.types,
            relations: &req.relations,
            explicit,
        };
        let (schema, provider_authority) =
            restricted_provider_projection(inner, &actor, &root_schema, &providers, &projection)?;
        query_providers::merge(&mut authority.providers, &provider_authority)?;
        let (current_restricted, _) =
            restricted_provider_projection(inner, &actor, &full_schema, &providers, &projection)?;
        if !weftext_data::baseline_contract_compatible(&schema, &current_restricted)
            .map_err(|_| changed())?
        {
            return Err(changed());
        }
        for baseline in &mut context.schemas {
            let (schema, provider_authority) = restricted_provider_projection(
                inner,
                &actor,
                &baseline.projection,
                &providers,
                &projection,
            )?;
            query_providers::merge(&mut authority.providers, &provider_authority)?;
            baseline.projection = schema;
        }
        context.parameters = req.parameters.clone();
        weftext_data::validate_query_plan_with_context(&req.plan, &schema, &context).map_err(
            |_| {
                if draft.is_some() {
                    unavailable()
                } else {
                    invalid()
                }
            },
        )?;
        if !validate_only {
            weftext_data::prepare_tag_structure(&req.plan, &schema, &context)
                .map_err(|_| invalid())?;
        }
        let cut_id = uuid::Uuid::new_v4().to_string();
        let mut notes = vec![];
        let mut metadata = BTreeMap::new();
        let mut workers = vec![];
        let mut worker_bytes = scopes.retained_bytes();
        charge_serialized(&mut worker_bytes, &(&context, &schema, &req.plan))?;
        let mut resource_map = BTreeMap::new();
        let mut complete = true;
        if !validate_only {
            for r in &resources {
                let Some(file) = snapshots.get(r) else {
                    return Err(changed());
                };
                let Ok(saved) = source(file) else {
                    complete = false;
                    continue;
                };
                let before = scopes.retained_bytes();
                let coordinate = scopes.qualify_node(inner, r, file, &workspace)?;
                charge_bytes(&mut worker_bytes, scopes.retained_bytes() - before)?;
                authority.scopes = scopes.authority.clone();
                let source = draft
                    .as_ref()
                    .filter(|d| &d.resource == r)
                    .map_or(saved.as_str(), |d| d.source.as_str());
                let Ok((id, types, resolved)) =
                    weftext_data::narrow_query_metadata(source, &full_schema)
                else {
                    complete = false;
                    continue;
                };
                if file.node_id.as_deref() != Some(&id) {
                    return Err(changed());
                }
                let entity = format!("weftext-node://{workspace}/{id}");
                resource_map.insert(entity.clone(), r.clone());
                let fields = if req.fields.is_empty() {
                    BTreeMap::new()
                } else {
                    let Ok(profile) =
                        weftext_data::narrow_profile(source, &full_schema, &req.fields, None)
                    else {
                        complete = false;
                        continue;
                    };
                    if profile.node_id != id {
                        return Err(changed());
                    }
                    profile
                        .fields
                        .into_iter()
                        .map(|f| (f.field_id, f.cell))
                        .collect()
                };
                metadata.insert(
                    entity.clone(),
                    weftext_data::AuthorizedQueryMetadata {
                        identity: req.products.contains("self").then(|| entity.clone()),
                        types: req.products.contains("types").then_some(types),
                        resolved_types: resolved,
                    },
                );
                notes.push(weftext_data::FrozenNote {
                    entity_key: entity.clone(),
                    path: coordinate.to_string_lossy().into_owned(),
                    source: String::new(),
                    revision: cut_id.clone(),
                    lifecycle: inner.file(r)?.lifecycle.clone(),
                    title: None,
                    body: None,
                    products: None,
                    authorized_fields: Some(fields),
                });
                if req
                    .products
                    .iter()
                    .any(|p| matches!(p.as_str(), "title" | "body" | "headings"))
                {
                    let config_file = &inner.file(r)?;
                    let row_inputs = captured_inputs.members_for(&config_file.inputs)?;
                    let missing = row_inputs
                        .iter()
                        .map(|input| captured_inputs.entry(input).map(|entry| entry.is_none()))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .any(|missing| missing);
                    if missing {
                        complete = false;
                        continue;
                    }
                    let input_sizes = row_inputs
                        .iter()
                        .map(|input| {
                            let (path, bytes, accepted) =
                                captured_inputs.entry(input)?.ok_or_else(changed)?;
                            if !accepted {
                                return Err(changed());
                            }
                            Ok((path.len(), bytes.len()))
                        })
                        .collect::<Result<Vec<_>, ApiError>>()?;
                    charge_bytes(
                        &mut worker_bytes,
                        worker_capture_bytes(
                            source.len(),
                            config_file.path.len(),
                            inner.config.root.as_os_str().len(),
                            input_sizes,
                        )?,
                    )?;
                    let mut frozen_files = row_inputs
                        .iter()
                        .map(|input| {
                            let (path, bytes, accepted) =
                                captured_inputs.entry(input)?.ok_or_else(changed)?;
                            if !accepted {
                                return Err(changed());
                            }
                            Ok((
                                path.to_owned(),
                                base64::engine::general_purpose::STANDARD.encode(bytes),
                            ))
                        })
                        .collect::<Result<BTreeMap<_, _>, ApiError>>()?;
                    frozen_files.insert(
                        config_file.path.clone(),
                        base64::engine::general_purpose::STANDARD.encode(source.as_bytes()),
                    );
                    workers.push((
                        entity,
                        WorkerRequest {
                            source: source.into(),
                            path: config_file.path.clone().into(),
                            root: inner.config.root.clone(),
                            paged: false,
                            style: None,
                            frozen_files,
                        },
                    ));
                }
            }
        }
        // Static field projections never read or execute the library style.
        // All semantic workers in this cut share one captured environment.
        let environment = if workers.is_empty() {
            None
        } else {
            let mut environment = library::environment(inner, &actor)?;
            // Frozen semantic results carry their actual compilation input
            // authority into downstream Office delivery as well as Query read.
            environment.attach_inputs(inner, &actor, captured_inputs.clone())?;
            for (entity, worker) in &mut workers {
                let resource = resource_map.get(entity).ok_or_else(changed)?;
                if let Some(citations) = citations::freeze(inner, &actor, resource, &worker.source)?
                {
                    citations.inject(worker)?;
                    environment.attach_citations(inner, &actor, citations)?;
                }
            }
            charge_bytes(&mut worker_bytes, environment.retained_bytes())?;
            for (_, worker) in &mut workers {
                let before = serialized_query_bytes(worker)?;
                environment.apply(worker)?;
                let after = serialized_query_bytes(worker)?;
                charge_bytes(&mut worker_bytes, after.saturating_sub(before))?;
            }
            authority.execution = environment.authority.clone();
            Some(environment)
        };
        let columns = weftext_data::query_output_columns(&req.plan, &context.definitions)
            .map_err(|_| invalid())?;
        let config_hash = office::binding_hash(inner, &all_resources(&authority))?;
        charge_bytes(&mut worker_bytes, captured_inputs.retained_bytes())?;
        return Ok(Capture {
            actor: actor.clone(),
            epoch: inner.actor_generation(&actor),
            authority,
            config_hash,
            witnesses,
            inputs: captured_inputs,
            scopes,
            input: weftext_data::FrozenQueryInput {
                cut_id: cut_id.clone(),
                schema_revision: cut_id,
                coverage: if complete { "complete" } else { "partial" }.into(),
                freshness: "current".into(),
                notes,
                advanced: Some(context),
            },
            schema,
            plan: req.plan,
            metadata,
            columns,
            workers,
            environment,
            products: req.products,
            resource_map,
            draft,
        });
    }
    Err(invalid())
}
async fn execute_captured(
    state: &HostedState,
    auth: &Auth,
    mut captured: Capture,
) -> Result<Value, ApiError> {
    let mut execution_bytes = frozen_input_bytes(
        &captured.input,
        &captured.metadata,
        &captured.schema,
        &captured.plan,
    )?;
    charge_bytes(&mut execution_bytes, captured.scopes.retained_bytes())?;
    charge_bytes(&mut execution_bytes, captured.inputs.retained_bytes())?;
    if let Some(environment) = &captured.environment {
        charge_bytes(&mut execution_bytes, environment.retained_bytes())?;
    }
    for (entity, worker) in std::mem::take(&mut captured.workers) {
        let output = editor::compile_frozen(state, worker).await?;
        let products = if output["ok"] == true {
            serde_json::from_value::<weftext_engine::RenderProducts>(output["products"].clone())
                .ok()
                .map(|p| {
                    data_api::native_query_products(
                        format!("{}:{entity}", captured.input.cut_id),
                        p,
                    )
                })
        } else {
            None
        };
        if let Some(note) = captured
            .input
            .notes
            .iter_mut()
            .find(|n| n.entity_key == entity)
        {
            let mut products = products.unwrap_or_else(|| weftext_data::NoteProducts {
                compile_identity: captured.input.cut_id.clone(),
                target: "html".into(),
                title: weftext_data::Cell::Unavailable {
                    reason: "native-product-unavailable".into(),
                },
                body: weftext_data::Cell::Unavailable {
                    reason: "native-product-unavailable".into(),
                },
                headings: weftext_data::Cell::Unavailable {
                    reason: "native-product-unavailable".into(),
                },
            });
            for (name, cell) in [
                ("title", &mut products.title),
                ("body", &mut products.body),
                ("headings", &mut products.headings),
            ] {
                if !captured.products.contains(name) {
                    *cell = weftext_data::Cell::Unavailable {
                        reason: "product-not-authorized".into(),
                    };
                }
            }
            charge_serialized(&mut execution_bytes, &products)?;
            note.products = Some(products);
        }
    }
    let (mut result, provider_dependencies) =
        weftext_data::execute_authorized_query_with_provider_dependencies(
            &captured.input,
            &captured.plan,
            &captured.schema,
            &weftext_data::QueryBudget {
                max_rows: 5000,
                ..Default::default()
            },
            &captured.metadata,
        )
        .map_err(|_| invalid())?;
    if result.rows.len() > 500 {
        return Err(invalid());
    }
    let auth = auth.clone();
    captured.authority.provider_dependencies = provider_dependencies;
    for consumed in &captured.authority.provider_dependencies.baselines {
        for (actual, selected) in [
            (&consumed.providers, &captured.authority.providers.providers),
            (
                &consumed.series_policies,
                &captured.authority.providers.series_policies,
            ),
        ] {
            if actual.iter().any(|(id, fp)| selected.get(id) != Some(fp)) {
                return Err(unavailable());
            }
        }
    }
    locked(state,move|inner|{
        let actor=inner.actor(&auth)?;if actor!=captured.actor||inner.actor_generation(&actor)!=captured.epoch{return Err(expired());}authorize_authority(inner,&actor,&captured.authority)?;
        if office::binding_hash(inner,&all_resources(&captured.authority))?!=captured.config_hash{return Err(changed());}
        for (path,witness) in &captured.witnesses{inner.core.validate_snapshot_witness(&inner.config.root,path,witness).map_err(|_|changed())?;}
        captured.scopes.recheck(inner,&actor)?;
        captured.inputs.validate(inner,&actor)?;
        if let Some(environment)=&captured.environment {environment.recheck(inner,&actor)?;}
        if let Some(d)=&captured.draft && let (Some(id),Some(serial))=(&d.session_id,d.serial){let (_,session,_)=inner.session(&auth,id)?;if session.resource!=d.resource{return Err(unavailable());}writing_api::exact_version(&inner.editor_versions,id,&d.source,serial)?;}
        for row in &mut result.rows{row.key=digest(format!("{}:{}",result.cut_id,row.key).as_bytes());row.revision=row.revision.as_ref().map(|_|result.cut_id.clone());if let Some(entity)=&row.entity_key{let resource=captured.resource_map.get(entity).ok_or_else(changed)?;row.entity_key=Some(resource.clone());row.path=Some(format!("resource:{resource}"));}}
        result.diagnostics=result.diagnostics.into_iter().map(|_|"获权投影或表达式有未决结果；请核对Cell状态与覆盖范围。".into()).collect::<BTreeSet<_>>().into_iter().collect();
        let mut public=serde_json::to_value(&result).map_err(|_|invalid())?;
        for row in public["rows"].as_array_mut().ok_or_else(invalid)?{if let Some(resource)=row["entityKey"].as_str().map(str::to_owned){row["resource"]=json!(resource);row["canOpen"]=json!(inner.file(&resource)?.lifecycle=="live"&&inner.file(&resource)?.editable&&inner.rights(&actor,&resource).is_ok());}}
        let tag_witness = if captured.authority.tag_structure {Some(captured.witnesses.iter().find(|(path, _)| Path::new(path) == inner.config.root.join(".weftext-tags.json")).ok_or_else(changed)?.clone())} else {None};
        let id=fields::retain_query(inner,&actor,result,captured.columns.clone(),captured.schema,captured.authority,tag_witness)?;
        Ok(json!({"result":public,"queryId":id,"columns":captured.columns,"scope":"authorizedProjection","readOnly":true}))
    }).await
}
pub(super) async fn execute(
    state: &HostedState,
    auth: &Auth,
    req: QueryRequest,
) -> Result<Value, ApiError> {
    let auth_for_capture = auth.clone();
    let captured = locked(state, move |inner| {
        capture(inner, &auth_for_capture, req, None, false)
    })
    .await?;
    execute_captured(state, auth, captured).await
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_embedded(
    state: &HostedState,
    auth: &Auth,
    resource: String,
    source: String,
    plan: QueryPlan,
    binding: SavedQueryBinding,
    parameters: BTreeMap<String, NativeValue>,
    providers: query_providers::Selection,
    session_id: String,
    serial: u64,
) -> Result<Value, ApiError> {
    let check = auth.clone();
    let req = QueryRequest {
        providers: providers.providers,
        series_policies: providers.series_policies,
        fields: BTreeSet::new(),
        resources: None,
        products: BTreeSet::new(),
        relations: BTreeSet::new(),
        parameters,
        plan,
    };
    let draft = Draft {
        resource,
        source,
        binding,
        session_id: Some(session_id),
        serial: Some(serial),
    };
    let captured = locked(state, move |inner| {
        capture(inner, &check, req, Some(draft), false)
    })
    .await?;
    execute_captured(state, auth, captured).await
}
pub(super) async fn validate_embedded(
    state: &HostedState,
    auth: &Auth,
    resource: String,
    source: String,
    plan: QueryPlan,
    binding: SavedQueryBinding,
) -> Result<QueryPlan, ApiError> {
    let check = auth.clone();
    let req = QueryRequest {
        providers: BTreeMap::new(),
        series_policies: BTreeMap::new(),
        fields: BTreeSet::new(),
        resources: None,
        products: BTreeSet::new(),
        relations: BTreeSet::new(),
        parameters: BTreeMap::new(),
        plan,
    };
    let draft = Draft {
        resource,
        source,
        binding,
        session_id: None,
        serial: None,
    };
    locked(state, move |inner| {
        let captured = capture(inner, &check, req, Some(draft), true)?;
        captured.scopes.recheck(inner, &captured.actor)?;
        let mut plan = captured.plan;
        query_scopes::canonicalize(inner, &captured.actor, &mut plan)?;
        Ok(plan)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> weftext_data::FrozenQueryInput {
        weftext_data::FrozenQueryInput {
            cut_id: "cut".into(),
            schema_revision: "schema".into(),
            coverage: "complete".into(),
            freshness: "current".into(),
            notes: vec![],
            advanced: None,
        }
    }

    #[test]
    fn definition_static_authority_requires_each_product_field_and_relation_separately() {
        let mut grants = BTreeMap::from([(
            "Shared".into(),
            Rights {
                read: true,
                write: true,
                query: QueryRights {
                    definitions: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )]);
        let configured = BTreeSet::from(["Shared".into()]);
        let mut demands = weftext_data::QueryReadRequirements::default();
        demands.fields.insert("tasks/status".into());
        assert_eq!(
            static_authority(&grants, &configured, &demands)
                .err()
                .unwrap()
                .status,
            StatusCode::NOT_FOUND
        );
        grants.get_mut("Shared").unwrap().fields.insert(
            "tasks/status".into(),
            FieldRights {
                read: true,
                write: false,
            },
        );
        demands.products.insert("body".into());
        assert_eq!(
            static_authority(&grants, &configured, &demands)
                .err()
                .unwrap()
                .status,
            StatusCode::NOT_FOUND
        );
        grants
            .get_mut("Shared")
            .unwrap()
            .query
            .products
            .insert("body".into(), true);
        demands.relations.insert("task-parent".into());
        assert!(static_authority(&grants, &configured, &demands).is_err());
        grants
            .get_mut("Shared")
            .unwrap()
            .query
            .relations
            .insert("task-parent".into());
        let proof = static_authority(&grants, &configured, &demands).unwrap();
        assert_eq!(proof.fields["Shared"], demands.fields);
        assert_eq!(proof.products["Shared"], demands.products);
        assert_eq!(proof.relations["Shared"], demands.relations);
        assert!(static_authority(&grants, &BTreeSet::new(), &demands).is_err());
    }

    #[test]
    fn native_owner_normalization_recurses_and_preserves_pinned_selection_and_arguments() {
        let mut plan: QueryPlan = serde_json::from_value(json!({"advanced":{"related":{"relation":"parent","direction":"forward","from":{"advanced":{"unionAll":[
            {"advanced":{"call":{"owner":"Shared","selection":{"mode":"pinned","key":"original-key","revision":"original-revision"},"arguments":{"wanted":"\"open\""}}}},
            {"advanced":{"call":{"owner":"Other","selection":{"mode":"current","key":"other-key"}}}}
        ]}}}}})).unwrap();
        canonical_definition_owners(
            &mut plan,
            &BTreeMap::from([
                ("Shared".into(), "weftext-node://workspace/first".into()),
                ("Other".into(), "weftext-node://workspace/second".into()),
            ]),
        )
        .unwrap();
        let value = serde_json::to_value(&plan).unwrap();
        let first =
            &value["advanced"]["related"]["from"]["advanced"]["unionAll"][0]["advanced"]["call"];
        assert_eq!(first["owner"], "weftext-node://workspace/first");
        assert_eq!(
            first["selection"],
            json!({"mode":"pinned","key":"original-key","revision":"original-revision"})
        );
        assert_eq!(first["arguments"]["wanted"], "\"open\"");
        let second =
            &value["advanced"]["related"]["from"]["advanced"]["unionAll"][1]["advanced"]["call"];
        assert_eq!(second["owner"], "weftext-node://workspace/second");
        assert!(canonical_definition_owners(&mut plan, &BTreeMap::new()).is_err());
    }

    #[test]
    fn shared_inputs_are_charged_again_for_each_worker() {
        // Each request fits the native worker's 24 MiB bound. Reusing twenty
        // input files for another document must not multiply capture memory.
        let worker = worker_capture_bytes(1024 * 1024, 40, 80, vec![(40, 700 * 1024); 20]).unwrap();
        assert!(worker < 24 * 1024 * 1024);
        let mut total = 0;
        charge_bytes(&mut total, worker).unwrap();
        let prior = total;
        assert_eq!(
            charge_bytes(&mut total, worker).unwrap_err().code,
            "queryBudget"
        );
        assert_eq!(total, prior);
    }

    #[test]
    fn capture_budget_accepts_exact_limit_and_rejects_overflow() {
        let mut used = QUERY_BYTES - 1;
        charge_bytes(&mut used, 1).unwrap();
        assert_eq!(used, QUERY_BYTES);
        assert!(charge_bytes(&mut used, 1).is_err());
        assert!(base64_bytes(usize::MAX).is_err());
        assert!(worker_capture_bytes(usize::MAX, 0, 0, []).is_err());
    }

    #[test]
    fn execution_charges_trusted_fields_and_products_as_they_arrive() {
        let mut input = input();
        input.notes.push(weftext_data::FrozenNote {
            entity_key: "node".into(),
            path: "resource:one".into(),
            source: String::new(),
            revision: "cut".into(),
            lifecycle: "live".into(),
            title: None,
            body: None,
            products: None,
            authorized_fields: Some(BTreeMap::from([(
                "text".into(),
                weftext_data::Cell::value(NativeValue::Text("x".repeat(1024 * 1024))),
            )])),
        });
        assert!(serde_json::to_vec(&input).unwrap().len() < 1024);
        let mut used = frozen_input_bytes(
            &input,
            &BTreeMap::new(),
            &Schema::bundled().unwrap(),
            &QueryPlan::default(),
        )
        .unwrap();
        assert!(used > 1024 * 1024);
        let products = weftext_data::NoteProducts {
            compile_identity: "cut:node".into(),
            target: "html".into(),
            title: weftext_data::Cell::Absent,
            body: weftext_data::Cell::value(NativeValue::Text("x".repeat(8 * 1024 * 1024))),
            headings: weftext_data::Cell::Absent,
        };
        for _ in 0..3 {
            charge_serialized(&mut used, &products).unwrap();
        }
        assert_eq!(
            charge_serialized(&mut used, &products).unwrap_err().code,
            "queryBudget"
        );
    }

    #[test]
    fn definition_history_is_part_of_initial_execution_budget() {
        let mut input = input();
        input.advanced = Some(weftext_data::FrozenAdvancedInput {
            definitions: (0..17)
                .map(|n| weftext_data::FrozenDefinitionOwner {
                    owner: format!("resource:{n}"),
                    history: vec![weftext_data::DefinitionRevision {
                        revision: "accepted".into(),
                        source: "x".repeat(2 * 1024 * 1024),
                        current: true,
                    }],
                })
                .collect(),
            ..Default::default()
        });
        assert_eq!(
            frozen_input_bytes(
                &input,
                &BTreeMap::new(),
                &Schema::bundled().unwrap(),
                &QueryPlan::default()
            )
            .unwrap_err()
            .code,
            "queryBudget"
        );
    }
}
