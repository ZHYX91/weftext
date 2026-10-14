//! Structured data, exact native source proposals, and bounded local Query.
//! Core owns every filesystem read/write and the admitted note identity.

use super::*;
use serde::de::{MapAccess, Visitor};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
};
use weftext_core::{ConfigRole, OperationIntent, OperationRequest as CoreOperationRequest};
use weftext_data::{NativeValue, Schema, ValuePath};

const BUNDLED_SCHEMA_SOURCE: &str =
    include_str!("../../../docs/design/examples/schema.authoring.json");

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/data/schema", post(schema))
        .route("/api/data/schema/prepare", post(prepare_schema))
        .route("/api/data/schema/install", post(install_schema))
        .route("/api/data/project", post(project))
        .route("/api/data/edit", post(edit))
        .route("/api/data/types", post(types))
        .route("/api/data/capture", post(capture))
        .route("/api/data/resolve", post(resolve))
        .route("/api/data/open", post(open_entity))
        .route("/api/data/references", post(references))
        .route("/api/data/period/resolve", post(resolve_period))
        .route("/api/data/period/create", post(create_period))
        .route("/api/query/run", post(query))
        .route("/api/query/definitions", post(definitions))
        .route("/api/query/definition/save", post(save_definition))
        .route("/api/query/definition/run", post(run_definition))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RootRequest {
    root: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResolveRequest {
    root: String,
    entity_key: String,
    lifecycle: Option<String>,
    revision: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QueryRequest {
    root: String,
    plan: weftext_data::QueryPlan,
    #[serde(default, deserialize_with = "unique_fields")]
    parameters: BTreeMap<String, NativeValue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionsRequest {
    root: String,
    owner: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionSaveRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    base_id: String,
    key: Option<String>,
    label: String,
    plan: weftext_data::QueryPlan,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionRunRequest {
    root: String,
    owner: String,
    selection: weftext_data::DefinitionSelection,
    #[serde(default, deserialize_with = "unique_fields")]
    parameters: BTreeMap<String, NativeValue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PeriodRequest {
    root: String,
    series_key: String,
    kind: String,
    key: String,
    #[serde(default)]
    steps: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PeriodCreateRequest {
    root: String,
    parent: String,
    name: String,
    series_key: String,
    kind: String,
    key: String,
    title: Option<String>,
    #[serde(default)]
    body: String,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SchemaPrepare {
    root: String,
    source: String,
    before_source: String,
    before_revision: Option<String>,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SchemaInstall {
    operation_id: String,
    plan_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectionRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FieldEdit {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    key: String,
    #[serde(default)]
    path: Vec<ValuePath>,
    value: NativeValue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TypeEdit {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    types: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureRequest {
    root: String,
    parent: String,
    name: String,
    types: Vec<String>,
    title: Option<String>,
    #[serde(default, deserialize_with = "unique_fields")]
    fields: BTreeMap<String, NativeValue>,
    #[serde(default)]
    body: String,
    operation_id: String,
}

/// Preserve duplicate-key failure rather than allowing JSON map last-wins.
pub(crate) fn unique_fields<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, NativeValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Fields;
    impl<'de> Visitor<'de> for Fields {
        type Value = BTreeMap<String, NativeValue>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object of exactly unique field keys")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, NativeValue>()? {
                if out.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate capture field key"));
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(Fields)
}

struct SchemaSource {
    source: String,
    before: Option<Vec<u8>>,
    role_ref: Option<String>,
    revision: Option<String>,
    can_install: bool,
    base_id: Option<String>,
}

/// Core's guarded config role is the sole configuration read path. An ordinary
/// source can use the bundled schema read-only; it cannot install library config.
fn schema_source(core: &mut Core, root: &Path) -> Result<SchemaSource, CoreError> {
    match core.read_config(root, ConfigRole::Schema) {
        Ok(document) => {
            let source = document
                .bytes
                .as_ref()
                .map(|bytes| String::from_utf8(bytes.clone()))
                .transpose()
                .map_err(|_| CoreError::new("invalidSchema", "Schema configuration must be UTF-8"))?
                .unwrap_or_else(|| BUNDLED_SCHEMA_SOURCE.to_owned());
            Ok(SchemaSource {
                source,
                before: document.bytes,
                role_ref: Some(document.role_ref),
                revision: document.revision,
                can_install: true,
                base_id: Some(document.base_id),
            })
        }
        Err(error) if error.code == "unadmittedLibrary" => {
            // Validate the selected root through Core. Do not probe config files
            // independently or manufacture a managed library identity.
            core.inventory(root)?;
            Ok(SchemaSource {
                source: BUNDLED_SCHEMA_SOURCE.to_owned(),
                before: None,
                role_ref: None,
                revision: None,
                can_install: false,
                base_id: None,
            })
        }
        Err(error) => Err(error),
    }
}

fn resolved_schema(core: &mut Core, root: &Path) -> Result<(SchemaSource, Schema), CoreError> {
    let config = schema_source(core, root)?;
    let schema = Schema::from_author_json(&config.source)
        .map_err(|error| CoreError::new("invalidSchema", error.to_string()))?;
    Ok((config, schema))
}

pub(crate) async fn knowledge_schema(
    state: &AppState,
    root: PathBuf,
) -> Result<(Schema, String), ApiError> {
    let (config, schema) = core_call(state, move |core| resolved_schema(core, &root)).await?;
    let generation = format!(
        "{}:{}",
        config.revision.as_deref().unwrap_or("observed"),
        weftext_engine::source_hash(&config.source)
    );
    Ok((schema, generation))
}

pub(crate) fn knowledge_schema_from_snapshot(
    snapshot: &weftext_core::SnapshotFile,
) -> Result<(Schema, String), ApiError> {
    let source = snapshot
        .bytes
        .as_ref()
        .map(|bytes| String::from_utf8(bytes.clone()))
        .transpose()
        .map_err(|_| ApiError::bad("Frozen schema configuration must be UTF-8"))?
        .unwrap_or_else(|| BUNDLED_SCHEMA_SOURCE.to_owned());
    let schema = Schema::from_author_json(&source).map_err(data_error)?;
    let generation = format!(
        "{}:{}",
        snapshot.revision.as_deref().unwrap_or("observed"),
        weftext_engine::source_hash(&source)
    );
    Ok((schema, generation))
}

fn data_error(error: weftext_data::DataError) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "invalid_data", error.to_string())
}

async fn schema_still_current(
    state: &AppState,
    root: PathBuf,
    expected: &SchemaSource,
) -> Result<(), ApiError> {
    let expected_source = expected.source.clone();
    let expected_revision = expected.revision.clone();
    let expected_role = expected.role_ref.clone();
    core_call(state, move |core| {
        let actual = schema_source(core, &root)?;
        if actual.source != expected_source
            || actual.revision != expected_revision
            || actual.role_ref != expected_role
        {
            return Err(CoreError::new(
                "configConflict",
                "The schema changed while evaluating this proposal; refresh the form",
            ));
        }
        Ok(())
    })
    .await
}

async fn schema(State(state): State<AppState>, Json(req): Json<RootRequest>) -> ApiResult {
    let config = core_call(&state, move |core| {
        schema_source(core, Path::new(&req.root))
    })
    .await?;
    let resolved = Schema::from_author_json(&config.source).map_err(|error| {
        let mut error = data_error(error);
        error.code = "invalid_schema".into();
        error.details = Some(Box::new(
            json!({"source":config.source,"roleRef":config.role_ref,"revision":config.revision}),
        ));
        error
    })?;
    Ok(Json(
        json!({"schema":resolved,"source":config.source,"roleRef":config.role_ref,
        "revision":config.revision,"builtinDefault":config.before.is_none(),"canInstall":config.can_install}),
    ))
}

async fn prepare_schema(
    State(state): State<AppState>,
    Json(req): Json<SchemaPrepare>,
) -> ApiResult {
    if req.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(ApiError::bad("Schema configuration exceeds 2 MiB"));
    }
    Schema::from_author_json(&req.source).map_err(data_error)?;
    let preview = core_call(&state, move |core| {
        let current = schema_source(core, Path::new(&req.root))?;
        if !current.can_install {
            return Err(CoreError::new(
                "unadmittedLibrary",
                "Create or open the original managed library before installing schema",
            ));
        }
        if current.source != req.before_source || current.revision != req.before_revision {
            return Err(CoreError::new(
                "configConflict",
                "The exact schema base changed; refresh before preparing another operation",
            ));
        }
        core.prepare_config_save(
            &req.root,
            ConfigRole::Schema,
            current.before,
            req.source.into_bytes(),
            &req.operation_id,
        )
    })
    .await?;
    Ok(Json(value(preview)?))
}

async fn install_schema(
    State(state): State<AppState>,
    Json(req): Json<SchemaInstall>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            let preview = core.operation_details(&req.operation_id)?;
            if preview.changes.len() != 1
                || !preview.changes[0].path.ends_with(".weftext-schema.json")
            {
                return Err(CoreError::new(
                    "operationConflict",
                    "This is not a schema installation plan",
                ));
            }
            core.execute_operation(&req.operation_id, &req.plan_id)
        })
        .await?,
    )?))
}

async fn project(State(state): State<AppState>, Json(req): Json<ProjectionRequest>) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    let read_only = loc
        .path
        .components()
        .any(|component| component.as_os_str() == ".weftext-trash");
    let root = loc.root.clone();
    let (config, resolved) =
        core_call(&state, move |core| resolved_schema(core, &loc.root)).await?;
    let source = req.source.clone();
    let mut projection =
        tokio::task::spawn_blocking(move || weftext_data::project(&source, &resolved))
            .await
            .map_err(|_| ApiError::internal("Data projection did not complete"))?;
    if read_only {
        for field in &mut projection.fields {
            field.writable = false;
        }
    }
    schema_still_current(&state, root, &config).await?;
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    Ok(Json(json!({"projection":projection,"serial":req.serial,
        "sourceHash":weftext_engine::source_hash(&req.source),"schemaRevision":config.revision,
        "schemaHash":weftext_engine::source_hash(&config.source),"readOnly":read_only})))
}

async fn edit(State(state): State<AppState>, Json(req): Json<FieldEdit>) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    deny_trash_writer(&loc.path)?;
    let root = loc.root.clone();
    let (config, resolved) =
        core_call(&state, move |core| resolved_schema(core, &loc.root)).await?;
    let source = req.source.clone();
    let relation_schema = resolved.clone();
    let field_id = resolved
        .field(&req.key)
        .map(|field| field.id.clone())
        .ok_or_else(|| ApiError::bad("The selected field is not installed"))?;
    let patch = tokio::task::spawn_blocking(move || {
        weftext_data::patch_field(&source, &resolved, &req.key, &req.path, &req.value)
    })
    .await
    .map_err(|_| ApiError::internal("Data edit did not complete"))?
    .map_err(data_error)?;
    check_relation_proposal(
        &state,
        root.clone(),
        relation_schema,
        req.source.clone(),
        patch.source.clone(),
        Some(field_id),
    )
    .await?;
    schema_still_current(&state, root, &config).await?;
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    patch_response(patch, req.serial, &req.source)
}

async fn types(State(state): State<AppState>, Json(req): Json<TypeEdit>) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    deny_trash_writer(&loc.path)?;
    let root = loc.root.clone();
    let (config, resolved) =
        core_call(&state, move |core| resolved_schema(core, &loc.root)).await?;
    let source = req.source.clone();
    let relation_schema = resolved.clone();
    let patch = tokio::task::spawn_blocking(move || {
        weftext_data::patch_types(&source, &resolved, &req.types)
    })
    .await
    .map_err(|_| ApiError::internal("Type edit did not complete"))?
    .map_err(data_error)?;
    check_relation_proposal(
        &state,
        root.clone(),
        relation_schema,
        req.source.clone(),
        patch.source.clone(),
        None,
    )
    .await?;
    schema_still_current(&state, root, &config).await?;
    writing_api::check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    patch_response(patch, req.serial, &req.source)
}

pub(crate) fn patch_response(
    patch: weftext_data::FieldPatch,
    serial: u64,
    before: &str,
) -> ApiResult {
    let prefix = before
        .get(..patch.from)
        .ok_or_else(|| ApiError::internal("Invalid source patch boundary"))?;
    let end = before
        .get(..patch.to)
        .ok_or_else(|| ApiError::internal("Invalid source patch boundary"))?;
    let expected = before
        .get(patch.from..patch.to)
        .ok_or_else(|| ApiError::internal("Invalid source patch range"))?;
    let from = prefix.encode_utf16().count();
    let to = end.encode_utf16().count();
    Ok(Json(
        json!({"source":patch.source,"patches":[{"from":from,"to":to,"text":patch.text,
        "byteFrom":patch.from,"byteTo":patch.to,"expected":expected}],"patchUnit":"utf16",
        "serial":serial,"sourceHash":weftext_engine::source_hash(before)}),
    ))
}

pub(crate) fn deny_trash_writer(path: &Path) -> Result<(), ApiError> {
    if path
        .components()
        .any(|component| component.as_os_str() == ".weftext-trash")
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "trashReadOnly",
            "回收站中的笔记只读；请先通过 Core 恢复笔记",
        ));
    }
    Ok(())
}

fn relation_targets(
    notes: &[weftext_data::FrozenNote],
    schema: &Schema,
) -> Vec<weftext_data::RelationTarget> {
    notes
        .iter()
        .map(|note| weftext_data::RelationTarget {
            entity_key: note.entity_key.clone(),
            types: weftext_data::project(&note.source, schema).types,
            lifecycle: note.lifecycle.clone(),
        })
        .collect()
}

fn relation_sources(notes: &[weftext_data::FrozenNote]) -> Vec<weftext_data::RelationSource> {
    notes
        .iter()
        .map(|note| weftext_data::RelationSource {
            entity_key: note.entity_key.clone(),
            source: note.source.clone(),
            lifecycle: note.lifecycle.clone(),
        })
        .collect()
}

async fn check_relation_proposal(
    state: &AppState,
    root: PathBuf,
    schema: Schema,
    before: String,
    after: String,
    selected_field: Option<String>,
) -> Result<(), ApiError> {
    core_call(state, move |core| {
        let frozen = match query_snapshot(
            core,
            &root.to_string_lossy(),
            weftext_data::QueryPlan::default(),
        ) {
            Ok(frozen) => Some(frozen),
            Err(error) if error.code == "unadmittedLibrary" => None,
            Err(error) => return Err(error),
        };
        let targets = frozen
            .as_ref()
            .map(|frozen| relation_targets(&frozen.input.notes, &schema))
            .unwrap_or_default();
        let complete = frozen
            .as_ref()
            .is_some_and(|frozen| frozen.input.coverage == "complete");
        let proposed = weftext_data::project(&after, &schema);
        let owner = frozen
            .as_ref()
            .and_then(|frozen| {
                frozen.input.notes.iter().find(|note| {
                    proposed
                        .id
                        .as_ref()
                        .is_some_and(|id| note.entity_key.rsplit('/').next() == Some(id.as_str()))
                })
            })
            .map(|note| note.entity_key.clone());
        let check = match (&owner, &frozen) {
            (Some(owner), Some(frozen)) => weftext_data::validate_relation_change_with_graph(
                owner,
                &before,
                &after,
                &schema,
                &targets,
                &relation_sources(&frozen.input.notes),
                complete,
            ),
            _ => {
                weftext_data::validate_relation_change(&before, &after, &schema, &targets, complete)
            }
        };
        if !check.is_valid() {
            return Err(CoreError::new(
                "invalidRelation",
                format!(
                    "{}{}",
                    check.diagnostics.join("; "),
                    check.unavailable.join("; ")
                ),
            ));
        }
        let mut context = weftext_data::ValidationContext::default();
        if let Some(frozen) = frozen
            && let Some(note) = frozen.input.notes.iter().find(|note| {
                proposed
                    .id
                    .as_ref()
                    .is_some_and(|id| note.entity_key.rsplit('/').next() == Some(id.as_str()))
            })
        {
            let resources = core.managed_resource_snapshot(&frozen.root, &note.path)?;
            context.owner = Some(note.entity_key.clone());
            context.resources_complete = resources
                .iter()
                .all(|resource| resource.identity_status == "admitted");
            context.resources = resources
                .into_iter()
                .filter(|resource| resource.identity_status == "admitted")
                .map(|resource| weftext_data::ResourceMetadata {
                    resource_ref: resource.resource_ref,
                    owner: note.entity_key.clone(),
                    lifecycle: "live".into(),
                })
                .collect();
        }
        let validation = match selected_field {
            Some(field_id) => {
                weftext_data::validate_field_change(&before, &after, &schema, &field_id, &context)
            }
            None => weftext_data::validate_note(&after, &schema, &context),
        };
        if !validation.is_valid() {
            return Err(CoreError::new(
                "invalidData",
                format!(
                    "{}{}",
                    validation.diagnostics.join("; "),
                    validation.unavailable.join("; ")
                ),
            ));
        }
        Ok(())
    })
    .await
}

fn capture_template(
    req: &CaptureRequest,
    resolved: &Schema,
) -> Result<String, weftext_data::DataError> {
    let mut seen = BTreeSet::new();
    for key in req.fields.keys() {
        let field = resolved
            .field(key)
            .ok_or_else(|| weftext_data::DataError(format!("Unknown capture field {key}")))?;
        if !seen.insert(&field.id) {
            return Err(weftext_data::DataError(format!(
                "One capture supplies {} twice using id and key",
                field.id
            )));
        }
    }
    let body = capture_body(req)?;
    weftext_data::capture_template(&req.types, &req.fields, &body, resolved)
}

fn capture_body(req: &CaptureRequest) -> Result<String, weftext_data::DataError> {
    Ok(match &req.title {
        Some(title) => format!(
            "\n#set document(title: [{}])\n#title()\n\n{}",
            weftext_engine::encode_literal(title)
                .map_err(|error| weftext_data::DataError(error.to_string()))?,
            req.body
        ),
        None => req.body.clone(),
    })
}

async fn capture(State(state): State<AppState>, Json(req): Json<CaptureRequest>) -> ApiResult {
    let document = core_call(&state, move |core| capture_document(core, &req)).await?;
    let document = value(document)?;
    remember(&state, &document)?;
    Ok(Json(document))
}

fn capture_document(
    core: &mut Core,
    req: &CaptureRequest,
) -> Result<weftext_core::OpenedDocument, CoreError> {
    let frozen = query_snapshot(core, &req.root, weftext_data::QueryPlan::default())?;
    capture_from_frozen(core, req, frozen)
}

fn capture_from_frozen(
    core: &mut Core,
    req: &CaptureRequest,
    frozen: FrozenQuery,
) -> Result<weftext_core::OpenedDocument, CoreError> {
    let source = capture_template(req, &frozen.schema)
        .map_err(|error| CoreError::new("invalidData", error.to_string()))?;
    let request = CoreOperationRequest {
        operation_id: req.operation_id.clone(),
        root: frozen.root.clone(),
        intent: OperationIntent::NewNode {
            parent: req.parent.clone(),
            name: req.name.clone(),
            source: Some(source),
        },
    };
    // A retry verifies the exact original Core request and executes only its
    // frozen plan; an already committed period cannot be created again.
    let preview = if core.operation_details(&req.operation_id).is_ok() {
        core.prepare_operation(request)?
    } else {
        let proposed = weftext_data::capture_source(
            "00000000-0000-4000-8000-000000000000",
            &req.types,
            &req.fields,
            &capture_body(req).map_err(|error| CoreError::new("invalidData", error.to_string()))?,
            &frozen.schema,
        )
        .map_err(|error| CoreError::new("invalidData", error.to_string()))?;
        let targets = relation_targets(&frozen.input.notes, &frozen.schema);
        let workspace = frozen.versions[0]["roleRef"]
            .as_str()
            .and_then(|reference| reference.strip_prefix("weftext-config://"))
            .and_then(|reference| reference.strip_suffix("/schema"))
            .ok_or_else(|| {
                CoreError::new(
                    "unadmittedLibrary",
                    "The proposed relation owner has no admitted workspace",
                )
            })?;
        let new_owner = format!("weftext-node://{workspace}/00000000-0000-4000-8000-000000000000");
        let check = weftext_data::validate_relation_change_with_graph(
            &new_owner,
            "",
            &proposed,
            &frozen.schema,
            &targets,
            &relation_sources(&frozen.input.notes),
            frozen.input.coverage == "complete",
        );
        if !check.is_valid() {
            return Err(CoreError::new(
                "invalidRelation",
                format!(
                    "{}{}",
                    check.diagnostics.join("; "),
                    check.unavailable.join("; ")
                ),
            ));
        }
        let context = weftext_data::ValidationContext {
            owner: None,
            resources: Vec::new(),
            resources_complete: true,
        };
        let note_validation = weftext_data::validate_note(&proposed, &frozen.schema, &context);
        if !note_validation.is_valid() {
            return Err(CoreError::new(
                "invalidData",
                format!(
                    "{}{}",
                    note_validation.diagnostics.join("; "),
                    note_validation.unavailable.join("; ")
                ),
            ));
        }
        let projection = weftext_data::project(&proposed, &frozen.schema);
        if let Some(period) = projection
            .fields
            .iter()
            .find(|field| field.field_id == "calendar/period")
            .and_then(|field| field.cell.native())
        {
            let (plan, policy) = period_plan(&frozen, period)?;
            let matched = weftext_data::execute_query(&frozen.input, &plan, &frozen.schema)
                .map_err(|error| CoreError::new("invalidPeriod", error.to_string()))?;
            if let Some(directory) = &plan.scope.directory
                && !Path::new(&req.parent).starts_with(directory)
            {
                return Err(CoreError::new(
                    "periodScope",
                    "The creation parent is outside the configured period scope",
                ));
            }
            if policy.multiplicity == "unique" {
                if matched.coverage != "complete" {
                    return Err(CoreError::new(
                        "coverageIncomplete",
                        "Unique period creation needs the complete admitted policy scope",
                    ));
                }
                if !matched.rows.is_empty() {
                    return Err(CoreError::new(
                        "periodConflict",
                        "A period note already exists for this unique series/key/scope",
                    ));
                }
            }
        }
        core.prepare_admitted_operation(request, &frozen.input.cut_id)?
    };
    core.execute_operation(&req.operation_id, &preview.plan_id)?;
    let source_path = preview
        .changes
        .iter()
        .find(|change| {
            Path::new(&change.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("typ"))
        })
        .ok_or_else(|| {
            CoreError::new("invalidState", "The creation plan has no canonical source")
        })?;
    core.open(&source_path.path, Some(PathBuf::from(&frozen.root)))
}

fn period_value(
    schema: &Schema,
    series_key: &str,
    kind: &str,
    key: &str,
) -> Result<NativeValue, CoreError> {
    let policy = schema
        .series_policies
        .iter()
        .find(|policy| policy.key == series_key)
        .ok_or_else(|| {
            CoreError::new(
                "periodPolicyUnavailable",
                "The requested period series has no installed policy",
            )
        })?;
    let calendar = schema
        .providers
        .iter()
        .find(|provider| provider.id == policy.calendar)
        .ok_or_else(|| {
            CoreError::new(
                "periodProviderUnavailable",
                "The exact calendar provider is unavailable",
            )
        })?;
    let zone = schema
        .providers
        .iter()
        .find(|provider| provider.id == policy.zone)
        .ok_or_else(|| {
            CoreError::new(
                "periodProviderUnavailable",
                "The exact zone provider is unavailable",
            )
        })?;
    let provider_text = |data: &Value, field: &str| {
        data[field].as_str().map(str::to_owned).ok_or_else(|| {
            CoreError::new(
                "periodProviderUnavailable",
                "Period provider has no exact key/version",
            )
        })
    };
    let entries = [
        ("series_key", series_key.to_string()),
        ("kind", kind.to_string()),
        ("key", key.to_string()),
        ("calendar", provider_text(&calendar.data, "key")?),
        (
            "calendar_version",
            provider_text(&calendar.data, "version")?,
        ),
        ("rule", policy.rule.clone()),
        ("zone", provider_text(&zone.data, "key")?),
        ("zone_version", provider_text(&zone.data, "version")?),
    ];
    let period = NativeValue::Record(
        entries
            .into_iter()
            .map(|(key, value)| weftext_data::NativeMember {
                key: key.into(),
                value: NativeValue::Text(value),
            })
            .collect(),
    );
    weftext_data::period_bounds(&period, schema)
        .map_err(|error| CoreError::new("invalidPeriod", error.to_string()))?;
    Ok(period)
}

fn period_plan(
    frozen: &FrozenQuery,
    period: &NativeValue,
) -> Result<(weftext_data::QueryPlan, weftext_data::SeriesPolicy), CoreError> {
    let series_key = period
        .member("series_key")
        .and_then(NativeValue::text)
        .ok_or_else(|| {
            CoreError::new("invalidPeriod", "Period series key is not a static string")
        })?;
    let policy = frozen
        .schema
        .series_policies
        .iter()
        .find(|policy| policy.key == series_key)
        .ok_or_else(|| {
            CoreError::new(
                "periodPolicyUnavailable",
                "The exact series policy is unavailable",
            )
        })?
        .clone();
    if !matches!(policy.multiplicity.as_str(), "unique" | "many") {
        return Err(CoreError::new(
            "invalidPeriodPolicy",
            "Unknown period multiplicity",
        ));
    }
    let mut scope = weftext_data::QueryScope {
        directory: None,
        descendants: true,
        lifecycle: "live".into(),
    };
    match policy.scope["kind"].as_str() {
        Some("library") => {}
        Some("node_subtree") => {
            let target = policy.scope["root"].as_str().ok_or_else(|| {
                CoreError::new(
                    "periodScopeUnavailable",
                    "Period subtree root is unavailable",
                )
            })?;
            let node = frozen
                .input
                .notes
                .iter()
                .find(|note| note.entity_key == target)
                .ok_or_else(|| {
                    CoreError::new(
                        "periodScopeUnavailable",
                        "Period subtree root is not in this admitted snapshot",
                    )
                })?;
            scope.directory = Some(
                Path::new(&node.path)
                    .parent()
                    .ok_or_else(|| {
                        CoreError::new("periodScopeUnavailable", "Subtree source has no parent")
                    })?
                    .to_string_lossy()
                    .to_string(),
            );
        }
        _ => {
            return Err(CoreError::new(
                "periodScopeUnavailable",
                "The central period policy scope is unsupported",
            ));
        }
    }
    let field = frozen
        .schema
        .field("calendar/period")
        .ok_or_else(|| CoreError::new("periodUnavailable", "The period Field is not installed"))?;
    let expression = format!("field({})", serde_json::to_string(&field.id)?);
    let identity = [
        "series_key",
        "kind",
        "calendar",
        "calendar_version",
        "rule",
        "key",
        "zone",
        "zone_version",
    ];
    let predicate = identity
        .into_iter()
        .map(|key| {
            let value = period
                .member(key)
                .and_then(NativeValue::text)
                .ok_or_else(|| {
                    CoreError::new(
                        "invalidPeriod",
                        "Complete static period identity is required",
                    )
                })?;
            Ok(format!(
                "{expression}.{key} == {}",
                serde_json::to_string(value)?
            ))
        })
        .collect::<Result<Vec<_>, CoreError>>()?
        .join(" && ");
    Ok((
        weftext_data::QueryPlan {
            scope,
            types: vec!["calendar/period-note".into()],
            r#where: Some(predicate),
            select: vec![weftext_data::QueryColumn {
                name: "period".into(),
                expr: expression,
            }],
            ..Default::default()
        },
        policy,
    ))
}

async fn resolve_period(
    State(state): State<AppState>,
    Json(req): Json<PeriodRequest>,
) -> ApiResult {
    let output = core_call(&state, move |core| {
        let frozen = query_snapshot(core,&req.root,weftext_data::QueryPlan::default())?;
        let value = period_value(&frozen.schema,&req.series_key,&req.kind,&req.key)?;
        let period = weftext_data::period_step(&value,req.steps,&frozen.schema)
            .map_err(|error| CoreError::new("invalidPeriod",error.to_string()))?;
        let bounds = weftext_data::period_bounds(&period,&frozen.schema)
            .map_err(|error| CoreError::new("invalidPeriod",error.to_string()))?;
        let (plan,policy) = period_plan(&frozen,&period)?;
        let mut result = weftext_data::execute_query(&frozen.input,&plan,&frozen.schema)
            .map_err(|error| CoreError::new("invalidQuery",error.to_string()))?;
        result.diagnostics.extend(frozen.diagnostics);
        let can_create = result.coverage == "complete" && (policy.multiplicity == "many" || result.rows.is_empty());
        Ok(json!({"period":period,"bounds":bounds,"policy":policy,"matches":result.rows,"coverage":result.coverage,
            "canCreate":can_create,"cutId":result.cut_id,"diagnostics":result.diagnostics,"snapshotMode":"admittedFrozen",
            "snapshotLabel":"已纳管笔记快照","rawScope":{"complete":frozen.raw_complete,"pendingCount":frozen.raw_pending}}))
    }).await?;
    Ok(Json(output))
}

async fn create_period(
    State(state): State<AppState>,
    Json(req): Json<PeriodCreateRequest>,
) -> ApiResult {
    let document = core_call(&state, move |core| {
        let frozen = query_snapshot(core, &req.root, weftext_data::QueryPlan::default())?;
        let period = period_value(&frozen.schema, &req.series_key, &req.kind, &req.key)?;
        let capture = CaptureRequest {
            root: req.root,
            parent: req.parent,
            name: req.name,
            types: vec!["period-note".into()],
            title: req.title,
            fields: BTreeMap::from([("calendar/period".into(), period)]),
            body: req.body,
            operation_id: req.operation_id,
        };
        capture_from_frozen(core, &capture, frozen)
    })
    .await?;
    let document = value(document)?;
    remember(&state, &document)?;
    Ok(Json(document))
}

pub(crate) fn resolve_entity(
    inventory: &weftext_core::Inventory,
    entity_key: &str,
) -> Result<weftext_core::InventoryEntry, CoreError> {
    if !inventory.complete {
        return Err(CoreError::new(
            "coverageIncomplete",
            "Current inventory cannot prove a unique live identity",
        ));
    }
    let workspace = inventory.workspace_id.as_ref().ok_or_else(|| {
        CoreError::new(
            "unadmittedLibrary",
            "Stable node addresses need the original managed library",
        )
    })?;
    let mut matches = inventory.entries.iter().filter(|entry| {
        entry.identity_status == "admitted"
            && entry
                .node_id
                .as_ref()
                .is_some_and(|id| entity_key == format!("weftext-node://{workspace}/{id}"))
    });
    let entry = matches.next().ok_or_else(|| {
        CoreError::new(
            "nodeUnavailable",
            "The stable node is unavailable in this live library",
        )
    })?;
    if matches.next().is_some() {
        return Err(CoreError::new(
            "identityConflict",
            "The stable node identity is not unique",
        ));
    }
    Ok(entry.clone())
}

async fn resolve(State(state): State<AppState>, Json(req): Json<ResolveRequest>) -> ApiResult {
    if req
        .lifecycle
        .as_deref()
        .is_some_and(|lifecycle| lifecycle != "live")
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "trashSnapshotRequired",
            "Use the read-only open route with the exact trash row revision",
        ));
    }
    let (inventory, entry) = core_call(&state, move |core| {
        let inventory = core.inventory(&req.root)?;
        let entry = resolve_entity(&inventory, &req.entity_key)?;
        Ok((inventory, entry))
    })
    .await?;
    Ok(Json(
        json!({"path":entry.path,"root":inventory.root,"nodeId":entry.node_id}),
    ))
}

async fn open_entity(State(state): State<AppState>, Json(req): Json<ResolveRequest>) -> ApiResult {
    let document = core_call(&state,move|core| {
        if req.lifecycle.as_deref() == Some("trash") {
            let revision = req.revision.as_deref().filter(|revision|!revision.is_empty()).ok_or_else(||CoreError::new("trashSnapshotRequired","Read-only trash opening needs the exact frozen row revision"))?;
            let captured = core.trash_admitted_snapshot(&req.root)?;
            if !captured.complete {return Err(CoreError::new("coverageIncomplete","The current owned trash set cannot prove a unique identity"));}
            let workspace = captured.inventory.workspace_id.as_deref().ok_or_else(||CoreError::new("unadmittedLibrary","Trash opening needs the original library"))?;
            let mut files = captured.files.iter().filter(|file|file.node_id.as_ref().is_some_and(|id|req.entity_key == format!("weftext-node://{workspace}/{id}")));
            let file = files.next().ok_or_else(||CoreError::new("nodeUnavailable","The exact node is absent from the owned trash set"))?;
            if files.next().is_some() {return Err(CoreError::new("identityConflict","The owned trash node identity is ambiguous"));}
            if let Some(base) = revision.strip_prefix("observed-source:") {core.validate_snapshot(&req.root,&file.path,base)?;}
            else if file.revision.as_deref() != Some(revision) {return Err(CoreError::new("staleBase","The trash row source changed since the query"));}
            let expected_source = file.bytes.as_deref().ok_or_else(||CoreError::new("sourceUnavailable","The trash source is unavailable"))?;
            let document = core.open(&file.path,Some(PathBuf::from(&captured.inventory.root)))?;
            if document.kind != "managedTrash" || document.node_id != file.node_id || document.workspace_id != captured.inventory.workspace_id
                || document.source.as_bytes() != expected_source || weftext_engine::inspect_metadata(&document.source).id != file.node_id {
                return Err(CoreError::new("identityConflict","The exact trash source changed before opening"));
            }
            if let Some(base) = revision.strip_prefix("observed-source:") {core.validate_snapshot(&req.root,&file.path,base)?;}
            return Ok(document);
        }
        if req.lifecycle.as_deref().is_some_and(|lifecycle|lifecycle != "live") {return Err(CoreError::new("invalidLifecycle","Unsupported node lifecycle"));}
        let inventory = core.inventory(&req.root)?;
        let entry = resolve_entity(&inventory,&req.entity_key)?;
        let document = core.open(&entry.path,Some(PathBuf::from(&inventory.root)))?;
        if document.kind != "managed" || document.node_id != entry.node_id
            || document.workspace_id != inventory.workspace_id
            || weftext_engine::inspect_metadata(&document.source).id != entry.node_id {
            return Err(CoreError::new("identityConflict","The resolved source changed identity before opening; no other document was selected"));
        }
        Ok(document)
    }).await?;
    let mut document = value(document)?;
    if document["kind"] == "managedTrash" {
        document["readOnly"] = json!(true);
    }
    remember(&state, &document)?;
    Ok(Json(document))
}

async fn references(State(state): State<AppState>, Json(req): Json<RootRequest>) -> ApiResult {
    let output = core_call(&state,move|core| {
        let frozen = query_snapshot(core,&req.root,weftext_data::QueryPlan::default())?;
        let nodes = frozen.input.notes.iter().map(|note| {
            let projection = weftext_data::project(&note.source,&frozen.schema);
            json!({"ref":note.entity_key,"path":note.path,"displayLabel":Path::new(&note.path).file_stem().unwrap_or_default().to_string_lossy(),
                "labelBasis":"filenameFallback","types":projection.types,"resolvedTypes":projection.resolved_types,"revision":note.revision})
        }).collect::<Vec<_>>();
        Ok(json!({"nodes":nodes,"complete":frozen.input.coverage == "complete","diagnostics":frozen.diagnostics,
            "snapshotLabel":"已纳管笔记快照","tagsEndpoint":"/api/navigation/tags"}))
    }).await?;
    Ok(Json(output))
}

struct FrozenQuery {
    input: weftext_data::FrozenQueryInput,
    schema: Schema,
    plan: weftext_data::QueryPlan,
    versions: Vec<Value>,
    diagnostics: Vec<String>,
    raw_complete: bool,
    raw_pending: usize,
    membership_ids: Vec<String>,
    root: String,
    lifecycle: String,
    scope_directories: BTreeSet<String>,
    scope_bindings: BTreeMap<String, String>,
    render_files: Option<BTreeMap<String, Vec<u8>>>,
    render_style: Option<weftext_engine::StyleModule>,
}

fn query_snapshot(
    core: &mut Core,
    root: &str,
    plan: weftext_data::QueryPlan,
) -> Result<FrozenQuery, CoreError> {
    query_snapshot_with_schema(core, root, plan, None)
}

fn query_snapshot_with_schema(
    core: &mut Core,
    root: &str,
    plan: weftext_data::QueryPlan,
    baseline: Option<(SchemaSource, Schema)>,
) -> Result<FrozenQuery, CoreError> {
    query_snapshot_impl(core, root, plan, baseline, false)
}

fn query_render_snapshot(
    core: &mut Core,
    root: &str,
    plan: weftext_data::QueryPlan,
    baseline: Option<(SchemaSource, Schema)>,
) -> Result<FrozenQuery, CoreError> {
    let render = plan.advanced.is_some()
        || !weftext_data::query_products(&plan, &[])
            .map_err(|error| CoreError::new("invalidQuery", error.to_string()))?
            .is_empty();
    query_snapshot_impl(core, root, plan, baseline, render)
}

fn query_snapshot_impl(
    core: &mut Core,
    root: &str,
    mut plan: weftext_data::QueryPlan,
    baseline: Option<(SchemaSource, Schema)>,
    render: bool,
) -> Result<FrozenQuery, CoreError> {
    let stable_scope = plan
        .scope
        .directory
        .as_ref()
        .filter(|directory| {
            directory.starts_with("weftext-directory://")
                || directory.starts_with("weftext-node://")
        })
        .map(|reference| {
            let target = core.resolve_navigation_target(root, reference)?;
            if target.status != "managed" || !matches!(target.kind.as_str(), "directory" | "node") {
                return Err(CoreError::new(
                    "queryScopeUnavailable",
                    "The stable scope is unavailable in the original library",
                ));
            }
            let path = target.path.ok_or_else(|| {
                CoreError::new(
                    "queryScopeUnavailable",
                    "The stable scope has no current path",
                )
            })?;
            Ok((reference.clone(), path))
        })
        .transpose()?;
    let lifecycle = if plan.scope.lifecycle == "trash" {
        "trash"
    } else {
        "live"
    };
    let (captured, render_files, render_style) = if render {
        let captured = core.admitted_compile_snapshot(root, lifecycle)?;
        (captured.snapshot, Some(captured.files), captured.style)
    } else if lifecycle == "trash" {
        (core.trash_admitted_snapshot(root)?, None, None)
    } else {
        (core.admitted_snapshot(root)?, None, None)
    };
    let mut complete = captured.complete;
    let cut_id = captured.cut_id;
    let raw_complete = captured.raw_complete;
    let raw_pending = captured.raw_pending;
    let membership_ids = captured.membership_ids;
    let inventory = captured.inventory;
    if let Some((reference, expected_path)) = &stable_scope {
        let target = core.resolve_navigation_target(root, reference)?;
        if target.status != "managed" || target.path.as_ref() != Some(expected_path) {
            return Err(CoreError::new(
                "queryScopeUnavailable",
                "The stable scope changed during capture; refresh its identity before querying",
            ));
        }
        plan.scope.directory = Some(expected_path.clone());
    }
    let workspace = inventory.workspace_id.as_ref().ok_or_else(|| {
        CoreError::new(
            "unadmittedLibrary",
            "Structured Query needs the original managed library",
        )
    })?;
    let mut scope_directories = BTreeSet::from([inventory.root.clone()]);
    for entry in &inventory.entries {
        if entry.kind == "directory" {
            scope_directories.insert(entry.path.clone());
        }
        if entry.kind == "node" && entry.identity_status == "admitted" {
            scope_directories.insert(entry.parent_path.clone());
        }
    }
    // Bind directory scope to the current Core inventory, never a title or an
    // arbitrary prefix that happens to make an empty result look successful.
    if let Some(directory) = &plan.scope.directory {
        let supplied = PathBuf::from(directory);
        if supplied
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(CoreError::new(
                "outsideRoot",
                "Query directory cannot contain parent traversal",
            ));
        }
        let selected = if supplied.is_absolute() {
            supplied
        } else {
            PathBuf::from(&inventory.root).join(supplied)
        };
        let selected = selected.to_string_lossy().to_string();
        let target = if Path::new(&selected) == Path::new(&inventory.root) {
            inventory.root.clone()
        } else {
            inventory
                .entries
                .iter()
                .find(|entry| {
                    (entry.kind == "directory" && Path::new(&entry.path) == Path::new(&selected))
                        || (entry.kind == "node"
                            && entry.identity_status == "admitted"
                            && Path::new(&entry.path).parent() == Some(Path::new(&selected)))
                })
                .map(|_| selected.clone())
                .ok_or_else(|| {
                    CoreError::new(
                        "queryScopeUnavailable",
                        "The requested directory is not available in this inventory",
                    )
                })?
        };
        plan.scope.directory = Some(target);
    }
    let (config, schema) = match baseline {
        Some(baseline) => baseline,
        None => {
            let schema_file = captured.schema.ok_or_else(|| {
                CoreError::new("schemaUnavailable", "Core did not freeze the schema role")
            })?;
            let (schema, _) = knowledge_schema_from_snapshot(&schema_file)
                .map_err(|error| CoreError::new("invalidSchema", error.message))?;
            let source = schema_file
                .bytes
                .as_ref()
                .map(|bytes| String::from_utf8(bytes.clone()))
                .transpose()
                .map_err(|_| CoreError::new("invalidSchema", "Frozen schema must be UTF-8"))?
                .unwrap_or_else(|| BUNDLED_SCHEMA_SOURCE.to_owned());
            (
                SchemaSource {
                    source,
                    before: schema_file.bytes,
                    role_ref: Some(format!("weftext-config://{workspace}/schema")),
                    revision: schema_file.revision,
                    can_install: true,
                    base_id: Some(schema_file.base_id),
                },
                schema,
            )
        }
    };
    let schema_revision = config.revision.clone().unwrap_or_else(|| {
        if config.before.is_none() {
            "bundled-schema-v0.3".into()
        } else {
            format!(
                "observed-config:{}",
                config.base_id.as_deref().unwrap_or("unavailable")
            )
        }
    });
    let mut versions = vec![
        json!({"role":"schema","roleRef":config.role_ref,"revision":config.revision,
        "baseId":config.base_id,"sourceHash":weftext_engine::source_hash(&config.source),
        "builtinDefault":config.before.is_none()}),
    ];
    let mut diagnostics = captured.warnings;
    diagnostics.push("已纳管笔记快照以 Core 私有准入集合为范围，同时冻结源码与模式配置；完整计数属于该冻结范围。未经准入的外部文件单独显示，结果未取得持续订阅的当前性。".into());
    if inventory
        .entries
        .iter()
        .any(|entry| entry.kind == "node" && entry.identity_status != "admitted")
    {
        diagnostics.push("未准入或身份有冲突的源码不提供结构化查询身份。".into());
    }
    let mut notes = Vec::new();
    for file in captured.files {
        let Some(entry) = inventory
            .entries
            .iter()
            .find(|entry| entry.path == file.path)
        else {
            return Err(CoreError::new(
                "invalidState",
                "Snapshot returned an unexpected source path",
            ));
        };
        if file.node_id != entry.node_id {
            complete = false;
            diagnostics.push("读取期间文件身份发生变化；相关行未发布。".into());
            continue;
        }
        let Some(bytes) = file.bytes else {
            complete = false;
            diagnostics.push("读取期间源码已不可用；相关行未发布。".into());
            continue;
        };
        let source = match String::from_utf8(bytes) {
            Ok(source) => source,
            Err(_) => {
                complete = false;
                diagnostics.push("非 UTF-8 源码无法投影。".into());
                continue;
            }
        };
        let expected = entry.node_id.as_deref().unwrap();
        let projection = weftext_data::project(&source, &schema);
        if projection.id.as_deref() != Some(expected) {
            complete = false;
            diagnostics.push("观察到的源码不再符合准入身份；相关行未发布。".into());
            continue;
        }
        let entity_key = format!("weftext-node://{workspace}/{expected}");
        if entry.source.as_ref() != Some(&source) {
            diagnostics.push(format!(
                "{entity_key}：目录观察与逐文件快照之间源码已变化。"
            ));
        }
        versions.push(json!({"entityKey":entity_key,"path":file.path,"baseId":file.base_id,
                "revision":file.revision,"currentness":file.currentness,"sourceHash":weftext_engine::source_hash(&source)}));
        notes.push(weftext_data::FrozenNote {
            entity_key,
            path: file.path,
            source,
            revision: file
                .revision
                .unwrap_or_else(|| format!("observed-source:{}", file.base_id)),
            lifecycle: lifecycle.into(),
            title: None,
            body: None,
            products: None,
        });
    }
    Ok(FrozenQuery {
        input: weftext_data::FrozenQueryInput {
            cut_id,
            schema_revision,
            coverage: if complete { "complete" } else { "partial" }.into(),
            freshness: "stale".into(),
            notes,
            advanced: None,
        },
        schema,
        plan,
        versions,
        diagnostics,
        raw_complete,
        raw_pending,
        membership_ids,
        root: inventory.root,
        lifecycle: lifecycle.into(),
        scope_directories,
        scope_bindings: stable_scope.into_iter().collect(),
        render_files,
        render_style,
    })
}

async fn query(State(state): State<AppState>, Json(req): Json<QueryRequest>) -> ApiResult {
    let frozen = core_call(&state, move |core| {
        let mut frozen = query_render_snapshot(core, &req.root, req.plan, None)?;
        freeze_query_definitions(core, &mut frozen, req.parameters)?;
        Ok(frozen)
    })
    .await?;
    Ok(Json(evaluate_frozen(&state, frozen).await?))
}

fn freeze_query_definitions(
    core: &mut Core,
    frozen: &mut FrozenQuery,
    parameters: BTreeMap<String, NativeValue>,
) -> Result<(), CoreError> {
    freeze_query_definitions_with_draft(core, frozen, parameters, None)
}

fn freeze_query_definitions_with_draft(
    core: &mut Core,
    frozen: &mut FrozenQuery,
    parameters: BTreeMap<String, NativeValue>,
    draft: Option<(&str, &str)>,
) -> Result<(), CoreError> {
    let mut context = weftext_data::FrozenAdvancedInput {
        parameters,
        scope_bindings: frozen.scope_bindings.clone(),
        ..Default::default()
    };
    let mut loaded = BTreeSet::new();
    for _ in 0..64 {
        let dependencies =
            weftext_data::definition_dependencies(&frozen.plan, &context.definitions)
                .map_err(|error| CoreError::new("invalidQuery", error.to_string()))?;
        let mut progress = false;
        for dependency in dependencies {
            let token = format!(
                "{}:{}",
                dependency.owner,
                serde_json::to_string(&dependency.selection)?
            );
            if !loaded.insert(token) {
                continue;
            }
            progress = true;
            let note = frozen
                .input
                .notes
                .iter()
                .find(|note| note.entity_key == dependency.owner)
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        "A called owner is unavailable in this exact admitted snapshot",
                    )
                })?;
            let owner_id = weftext_data::project(&note.source, &frozen.schema)
                .id
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        "Called owner has no qualified source identity",
                    )
                })?;
            let key = match &dependency.selection {
                weftext_data::DefinitionSelection::Current { key }
                | weftext_data::DefinitionSelection::Pinned { key, .. } => key,
            };
            let local_draft = draft.filter(|(owner, _)| *owner == note.entity_key);
            if (local_draft.is_none()
                || matches!(
                    dependency.selection,
                    weftext_data::DefinitionSelection::Pinned { .. }
                ))
                && !core.is_definition_key_admitted(&frozen.root, &owner_id, key)?
            {
                return Err(CoreError::new(
                    "definitionUnavailable",
                    "A called definition key has no original Core admission",
                ));
            }
            let index = match context
                .definitions
                .iter()
                .position(|owner| owner.owner == note.entity_key)
            {
                Some(index) => index,
                None => {
                    context
                        .definitions
                        .push(weftext_data::FrozenDefinitionOwner {
                            owner: note.entity_key.clone(),
                            history: vec![weftext_data::DefinitionRevision {
                                revision: local_draft.map_or_else(
                                    || note.revision.clone(),
                                    |(_, source)| {
                                        format!(
                                            "draft-source:{}",
                                            weftext_engine::source_hash(source)
                                        )
                                    },
                                ),
                                source: local_draft.map_or_else(
                                    || note.source.clone(),
                                    |(_, source)| source.to_owned(),
                                ),
                                current: true,
                            }],
                        });
                    context.definitions.len() - 1
                }
            };
            if let weftext_data::DefinitionSelection::Pinned { revision, .. } =
                &dependency.selection
                && !context.definitions[index]
                    .history
                    .iter()
                    .any(|history| &history.revision == revision)
            {
                let bytes = core.read_source_revision(&frozen.root, &owner_id, revision)?;
                let source = String::from_utf8(bytes).map_err(|_| {
                    CoreError::new(
                        "definitionUnavailable",
                        "Pinned definition source is not UTF-8",
                    )
                })?;
                context.definitions[index]
                    .history
                    .push(weftext_data::DefinitionRevision {
                        revision: revision.clone(),
                        source,
                        current: false,
                    });
            }
            let definition = weftext_data::select_definition(
                &dependency.selection,
                &context.definitions[index].history,
            )
            .map_err(|error| CoreError::new("definitionUnavailable", error.to_string()))?;
            let role = frozen
                .versions
                .first()
                .and_then(|version| version["roleRef"].as_str());
            if role != Some(definition.binding.schema.as_str()) {
                return Err(CoreError::new(
                    "schemaUnavailable",
                    "Called definition schema belongs to another authority",
                ));
            }
            if !context.schemas.iter().any(|schema| {
                schema.schema == definition.binding.schema
                    && schema.revision == definition.binding.revision
            }) {
                let bytes = core.read_config_revision(
                    &frozen.root,
                    ConfigRole::Schema,
                    &definition.binding.revision,
                )?;
                let source = String::from_utf8(bytes).map_err(|_| {
                    CoreError::new("schemaUnavailable", "Called definition schema is not UTF-8")
                })?;
                let projection = Schema::from_author_json(&source)
                    .map_err(|error| CoreError::new("invalidSchema", error.to_string()))?;
                context.schemas.push(weftext_data::FrozenDefinitionSchema {
                    schema: definition.binding.schema.clone(),
                    revision: definition.binding.revision.clone(),
                    projection,
                });
                frozen.versions.push(json!({"role":"definitionSchema","roleRef":definition.binding.schema,"revision":definition.binding.revision,"sourceHash":weftext_engine::source_hash(&source)}));
            }
            let selected_source = context.definitions[index]
                .history
                .iter()
                .find(|history| match &dependency.selection {
                    weftext_data::DefinitionSelection::Current { .. } => history.current,
                    weftext_data::DefinitionSelection::Pinned { revision, .. } => {
                        &history.revision == revision
                    }
                })
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        "Selected definition revision is unavailable",
                    )
                })?;
            frozen.versions.push(json!({"role":"definition","owner":dependency.owner,"key":definition.key,"selection":dependency.selection,
                "definitionRevision":selected_source.revision,"sourceHash":weftext_engine::source_hash(&selected_source.source),
                "draftPreview":selected_source.revision.starts_with("draft-source:")}));
        }
        if !progress {
            qualify_query_context(core, frozen, &mut context)?;
            frozen.input.advanced = Some(context);
            return Ok(());
        }
    }
    Err(CoreError::new(
        "queryBudget",
        "Definition call closure exceeded the bounded expansion depth",
    ))
}

fn qualify_query_context(
    core: &mut Core,
    frozen: &FrozenQuery,
    context: &mut weftext_data::FrozenAdvancedInput,
) -> Result<(), CoreError> {
    let lifecycles = weftext_data::required_query_lifecycles(&frozen.plan, &context.definitions)
        .map_err(|error| CoreError::new("invalidQuery", error.to_string()))?;
    if lifecycles
        .iter()
        .any(|lifecycle| lifecycle != &frozen.lifecycle)
    {
        return Err(CoreError::new(
            "queryScopeUnavailable",
            "This frozen query cut does not cover all requested lifecycles",
        ));
    }
    let scopes = weftext_data::required_query_scopes(&frozen.plan, &context.definitions)
        .map_err(|error| CoreError::new("invalidQuery", error.to_string()))?;
    for scope in scopes {
        let selected = if scope.starts_with("weftext-directory://")
            || scope.starts_with("weftext-node://")
        {
            let target = core.resolve_navigation_target(&frozen.root, &scope)?;
            if target.status != "managed" || !matches!(target.kind.as_str(), "directory" | "node") {
                return Err(CoreError::new(
                    "queryScopeUnavailable",
                    "A nested stable scope is unavailable",
                ));
            }
            target.path.ok_or_else(|| {
                CoreError::new(
                    "queryScopeUnavailable",
                    "A nested scope has no current path",
                )
            })?
        } else {
            let supplied = PathBuf::from(&scope);
            if supplied
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(CoreError::new(
                    "outsideRoot",
                    "Query scope cannot contain parent traversal",
                ));
            }
            if supplied.is_absolute() {
                supplied
            } else {
                Path::new(&frozen.root).join(supplied)
            }
            .to_string_lossy()
            .to_string()
        };
        let selected = frozen
            .scope_directories
            .iter()
            .find(|path| Path::new(path) == Path::new(&selected))
            .ok_or_else(|| {
                CoreError::new(
                    "queryScopeUnavailable",
                    "A nested scope is absent from this exact inventory",
                )
            })?;
        context.scope_bindings.insert(scope, selected.clone());
    }
    Ok(())
}

fn unavailable_products(identity: String, reason: &str) -> weftext_data::NoteProducts {
    let unavailable = || weftext_data::Cell::Unavailable {
        reason: reason.into(),
    };
    weftext_data::NoteProducts {
        compile_identity: identity,
        target: "html".into(),
        title: unavailable(),
        body: unavailable(),
        headings: unavailable(),
    }
}

fn native_query_products(
    identity: String,
    products: weftext_engine::RenderProducts,
) -> weftext_data::NoteProducts {
    let title = match (products.title_status.as_str(), products.title) {
        ("value", Some(title)) => weftext_data::Cell::value(NativeValue::Text(title)),
        ("absent", None) => weftext_data::Cell::Absent,
        ("ambiguous", _) => weftext_data::Cell::Invalid {
            diagnostics: vec!["The final native document title producers disagree".into()],
            value: None,
        },
        _ => weftext_data::Cell::Unavailable {
            reason: "The final native document did not qualify a title producer".into(),
        },
    };
    let body = match (products.body_status.as_str(), products.body) {
        ("complete", Some(body)) => weftext_data::Cell::value(NativeValue::Text(body)),
        _ => weftext_data::Cell::Unavailable {
            reason: "The final native HTML contains an unsupported body island".into(),
        },
    };
    let headings = NativeValue::Array(
        products
            .headings
            .into_iter()
            .map(|heading| {
                let display = if heading.visible {
                    "visible"
                } else if heading.is_main {
                    "mainSource"
                } else if heading.path.is_some() {
                    "includedSource"
                } else {
                    "unassociated"
                };
                NativeValue::Record(vec![
                    weftext_data::NativeMember {
                        key: "level".into(),
                        value: NativeValue::Int(heading.level.to_string()),
                    },
                    weftext_data::NativeMember {
                        key: "text".into(),
                        value: NativeValue::Text(heading.text),
                    },
                    weftext_data::NativeMember {
                        key: "generated".into(),
                        value: NativeValue::Bool(heading.generated),
                    },
                    weftext_data::NativeMember {
                        key: "displayAssociation".into(),
                        value: NativeValue::Text(display.into()),
                    },
                ])
            })
            .collect(),
    );
    weftext_data::NoteProducts {
        compile_identity: identity,
        target: products.target,
        title,
        body,
        headings: weftext_data::Cell::value(headings),
    }
}

async fn qualify_query_products(
    state: &AppState,
    frozen: &mut FrozenQuery,
) -> Result<Vec<String>, ApiError> {
    let owners = frozen
        .input
        .advanced
        .as_ref()
        .map(|context| context.definitions.as_slice())
        .unwrap_or(&[]);
    let demands = weftext_data::query_products(&frozen.plan, owners).map_err(data_error)?;
    if demands.is_empty() {
        return Ok(demands);
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    if let Some(files) = &frozen.render_files {
        use base64::Engine as _;
        let inputs = files.iter().map(|(path,bytes)|json!({"path":path,"byteLength":bytes.len(),
            "contentHash":weftext_engine::source_hash(&base64::engine::general_purpose::STANDARD.encode(bytes))})).collect::<Vec<_>>();
        frozen.versions.push(json!({"role":"nativeCompile","cutId":frozen.input.cut_id,"target":"html","files":inputs,
            "style":frozen.render_style.as_ref().map(|style|json!({"path":style.path,"sourceHash":weftext_engine::source_hash(&style.source),"enabled":style.enabled,"entryFunction":style.entry_function}))}));
    }
    for (index, note) in frozen.input.notes.iter_mut().enumerate() {
        let identity = format!(
            "native-html:{}:{}:{}",
            frozen.input.cut_id,
            note.entity_key,
            weftext_engine::source_hash(&note.source)
        );
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let products = if index >= 32 || remaining.is_zero() {
            unavailable_products(
                identity,
                "The bounded native product compilation budget was exhausted",
            )
        } else if let Some(files) = &frozen.render_files {
            let request = weftext_engine::FrozenRenderRequest {
                source: note.source.clone(),
                path: PathBuf::from(&note.path),
                root: PathBuf::from(&frozen.root),
                files: files.clone(),
                style: frozen.render_style.clone(),
            };
            match tokio::time::timeout(remaining, render_frozen_worker(state, request)).await {
                Ok(Ok(output)) if output["ok"] == true => {
                    match serde_json::from_value::<weftext_engine::RenderProducts>(
                        output["products"].clone(),
                    ) {
                        Ok(products) if products.target == "html" => {
                            frozen.versions.push(json!({"role":"nativeProduct","entityKey":note.entity_key,"compileIdentity":identity,
                            "compilerVersion":output["compilerVersion"],"target":"html","dependencies":output["dependencies"]}));
                            native_query_products(identity, products)
                        }
                        _ => unavailable_products(
                            identity,
                            "The successful worker did not publish qualified native HTML products",
                        ),
                    }
                }
                Ok(Ok(output)) => {
                    frozen.diagnostics.push(format!(
                        "{}：原生 HTML 编译失败，相关产品不可用（{}）。",
                        note.entity_key, output["diagnostics"]
                    ));
                    unavailable_products(identity, "The frozen native HTML compile failed")
                }
                Ok(Err(error)) => {
                    frozen
                        .diagnostics
                        .push(format!("{}：{}", note.entity_key, error.message));
                    unavailable_products(
                        identity,
                        "The isolated frozen native product worker was unavailable",
                    )
                }
                Err(_) => unavailable_products(
                    identity,
                    "The bounded native product compilation deadline was exhausted",
                ),
            }
        } else {
            unavailable_products(
                identity,
                "The exact same-cut native environment is unavailable",
            )
        };
        let partial = demands.iter().any(|demand| {
            let cell = match demand.as_str() {
                "title" => &products.title,
                "body" => &products.body,
                "headings" => &products.headings,
                _ => return true,
            };
            matches!(
                cell,
                weftext_data::Cell::Invalid { .. } | weftext_data::Cell::Unavailable { .. }
            )
        });
        if partial {
            frozen.input.coverage = "partial".into();
            frozen.diagnostics.push(format!(
                "{}：所需的最终原生产品未完全取得，查询覆盖标记为部分。",
                note.entity_key
            ));
        }
        note.products = Some(products);
    }
    Ok(demands)
}

async fn evaluate_frozen(state: &AppState, mut frozen: FrozenQuery) -> Result<Value, ApiError> {
    let products = qualify_query_products(state, &mut frozen).await?;
    let export_root = frozen.root.clone();
    let export_schema = value(&frozen.schema)?;
    let mut export_columns = frozen
        .plan
        .group
        .as_ref()
        .map(|g| {
            g.keys
                .iter()
                .map(|k| k.name.clone())
                .chain(g.aggregates.iter().map(|a| a.name.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| frozen.plan.select.iter().map(|c| c.name.clone()).collect());
    let row_lifecycles = frozen
        .input
        .notes
        .iter()
        .map(|note| {
            (
                (note.entity_key.clone(), note.revision.clone()),
                note.lifecycle.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let versions = frozen.versions;
    let diagnostics = frozen.diagnostics;
    let raw_complete = frozen.raw_complete;
    let raw_pending = frozen.raw_pending;
    let membership_ids = frozen.membership_ids;
    let mut result = tokio::task::spawn_blocking(move || {
        weftext_data::execute_query(&frozen.input, &frozen.plan, &frozen.schema)
    })
    .await
    .map_err(|_| ApiError::internal("Query evaluation did not complete"))?
    .map_err(data_error)?;
    result.diagnostics.extend(diagnostics);
    export_columns.retain(|name| result.terminal_schema.contains_key(name));
    for name in result.terminal_schema.keys() {
        if !export_columns.contains(name) {
            export_columns.push(name.clone());
        }
    }
    let mut output = value(result)?;
    output["columns"] = json!(export_columns);
    if let Some(rows) = output["rows"].as_array_mut() {
        for row in rows {
            let basis = row["entityKey"]
                .as_str()
                .zip(row["revision"].as_str())
                .map(|(entity, revision)| (entity.to_owned(), revision.to_owned()));
            if let Some(lifecycle) = basis.and_then(|basis| row_lifecycles.get(&basis)) {
                row["lifecycle"] = json!(lifecycle);
            }
        }
    }
    output["snapshotMode"] = json!("admittedFrozen");
    output["snapshotLabel"] = json!("已纳管笔记快照");
    output["externalPending"] = json!(!raw_complete || raw_pending > 0);
    output["rawScope"] = json!({"complete":raw_complete,"pendingCount":raw_pending});
    output["membershipIds"] = json!(membership_ids);
    output["inputVersions"] = json!(versions);
    output["access"] = json!({"audience":"authenticatedLocalSession","scope":"admittedNodesAtSelectedLifecycle",
        "bodyProduct":products.iter().any(|product|product == "body"),"titleProduct":products.iter().any(|product|product == "title"),
        "headingsProduct":products.iter().any(|product|product == "headings"),"resultGrantsWriteAccess":false});
    export_api::retain_query(
        state,
        export_root,
        &mut output,
        export_columns,
        export_schema,
    )?;
    Ok(output)
}

struct OwnerSource {
    entry: weftext_core::InventoryEntry,
    file: weftext_core::SnapshotFile,
    source: String,
}

fn owner_source(core: &mut Core, root: &str, owner: &str) -> Result<OwnerSource, CoreError> {
    let inventory = core.inventory(root)?;
    let entry = resolve_entity(&inventory, owner)?;
    let mut snapshot = core.read_snapshot(&inventory.root, std::slice::from_ref(&entry.path))?;
    let file = snapshot.files.pop().ok_or_else(|| {
        CoreError::new(
            "sourceUnavailable",
            "Definition owner source is unavailable",
        )
    })?;
    if file.node_id != entry.node_id {
        return Err(CoreError::new(
            "identityConflict",
            "Definition owner changed identity during reading",
        ));
    }
    let source = file.bytes.as_ref().ok_or_else(|| {
        CoreError::new(
            "sourceUnavailable",
            "Definition owner source is unavailable",
        )
    })?;
    let source = String::from_utf8(source.clone())
        .map_err(|_| CoreError::new("sourceUnavailable", "Definition owner is not UTF-8"))?;
    if weftext_engine::inspect_metadata(&source).id != entry.node_id {
        return Err(CoreError::new(
            "identityConflict",
            "Definition owner source no longer has its admitted identity",
        ));
    }
    Ok(OwnerSource {
        entry,
        file,
        source,
    })
}

async fn definitions(
    State(state): State<AppState>,
    Json(req): Json<DefinitionsRequest>,
) -> ApiResult {
    let output = core_call(&state, move |core| {
        let current = owner_source(core, &req.root, &req.owner)?;
        let candidates = weftext_data::saved_queries(&current.source)
            .map_err(|error| CoreError::new("invalidQueryDefinition", error.to_string()))?;
        let mut admitted = Vec::new(); let mut unavailable = Vec::new();
        for candidate in candidates {
            if core.is_definition_key_admitted(&req.root, current.entry.node_id.as_deref().unwrap(), &candidate.key)? {
                admitted.push(candidate);
            } else { unavailable.push(json!({"key":candidate.key,"label":candidate.label,"reason":"definition key is not admitted by Core"})); }
        }
        Ok(json!({"definitions":admitted,"ownerRevision":current.file.revision,"owner":req.owner,
            "ownerBaseId":current.file.base_id,"unavailable":unavailable,"snapshotMode":"perFileObserved"}))
    }).await?;
    Ok(Json(output))
}

async fn save_definition(
    State(state): State<AppState>,
    Json(req): Json<DefinitionSaveRequest>,
) -> ApiResult {
    let loc = location(&state, &req.session_id)?;
    let replay_location = loc.clone();
    let replay_request = req.clone();
    if let Some(replayed) = core_call(&state, move |core| {
        replay_saved_definition(core, &replay_location, &replay_request)
    })
    .await?
    {
        return definition_write_response(&state, replayed, req.serial, &req.source);
    }
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let final_check = state.clone();
    let before_source = req.source.clone();
    let serial = req.serial;
    let (document, definition, revision, patch) = core_call(&state, move |core| {
        let inventory = core.inventory(&loc.root)?;
        let entry = inventory.entries.iter().find(|entry| entry.path == loc.path.to_string_lossy() && entry.identity_status == "admitted")
            .ok_or_else(|| CoreError::new("identityConflict", "Save definitions only in the currently admitted owner note"))?;
        let node_id = entry.node_id.as_deref().unwrap().to_string();
        if weftext_engine::inspect_metadata(&req.source).id.as_deref() != Some(&node_id) {
            return Err(CoreError::new("identityConflict", "The draft must retain the current owner identity"));
        }
        let (config, resolved) = resolved_schema(core, &loc.root)?;
        let schema_revision = config.revision.clone().ok_or_else(|| CoreError::new("schemaRevisionRequired", "Explicitly install a schema before saving a Query baseline; observed or bundled schema has no admitted revision"))?;
        let binding = weftext_data::SavedQueryBinding { schema: config.role_ref.clone().unwrap(),
            revision: schema_revision, language: "weftext.query.cel.v1".into() };
        let mut current = core.read_snapshot(&loc.root, &[loc.path.to_string_lossy().to_string()])?;
        let current = current.files.pop().ok_or_else(|| CoreError::new("sourceUnavailable", "Owner snapshot is unavailable"))?;
        let key = match &req.key {
            Some(key) if core.is_definition_key_admitted(&loc.root, &node_id, key)? => key.clone(),
            Some(_) => return Err(CoreError::new("definitionUnavailable", "This owner has no admitted definition key")),
            None => core.reserve_definition_key(&loc.root, &loc.path, &current.base_id, &req.operation_id)?,
        };
        let mut definition_context = query_snapshot(core, &loc.root.to_string_lossy(), req.plan.clone())?;
        freeze_query_definitions(core, &mut definition_context, BTreeMap::new())?;
        let definition_context = definition_context.input.advanced.unwrap_or_default();
        let patch = weftext_data::upsert_saved_query_with_context(&req.source, req.key.as_deref(), &key, &req.label, &binding, &req.plan, &resolved, &definition_context)
            .map_err(|error| CoreError::new("invalidQueryDefinition", error.to_string()))?;
        let next_config = schema_source(core, &loc.root)?;
        if next_config.source != config.source || next_config.revision != config.revision {
            return Err(CoreError::new("configConflict", "Schema changed while preparing the definition"));
        }
        writing_api::check_snapshot(&final_check, &req.session_id, &req.source, req.serial, req.source_hash.as_deref())
            .map_err(|error| CoreError::new(&error.code, error.message))?;
        let receipt = core.save(SaveRequest { session_id: req.session_id, base_id: req.base_id,
            source: patch.source.clone(), operation_id: req.operation_id })?;
        let document = core.open(&loc.path, Some(loc.root.clone()))?;
        let definition = weftext_data::saved_queries(&patch.source)
            .map_err(|error| CoreError::new("invalidQueryDefinition", error.to_string()))?.into_iter()
            .find(|definition| definition.key == key).ok_or_else(|| CoreError::new("definitionUnavailable", "Committed definition could not be projected"))?;
        Ok((document, definition, Some(receipt.base_id), patch))
    }).await?;
    definition_write_response(
        &state,
        (document, definition, revision, patch),
        serial,
        &before_source,
    )
}

type DefinitionWrite = (
    weftext_core::OpenedDocument,
    weftext_data::SavedQuery,
    Option<String>,
    weftext_data::FieldPatch,
);

fn replay_saved_definition(
    core: &mut Core,
    location: &Location,
    req: &DefinitionSaveRequest,
) -> Result<Option<DefinitionWrite>, CoreError> {
    let Some(original) = core.saved_operation_source(&req.operation_id)? else {
        return Ok(None);
    };
    if req.source.len() > weftext_core::MAX_SOURCE_BYTES
        || req
            .source_hash
            .as_ref()
            .is_some_and(|hash| hash != &weftext_engine::source_hash(&req.source))
    {
        return Err(CoreError::new(
            "operationConflict",
            "The original source basis does not match this retry",
        ));
    }
    let key = req
        .key
        .as_ref()
        .or(original.definition_key.as_ref())
        .ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                "This save did not allocate or name a Query definition",
            )
        })?;
    let definition = weftext_data::saved_queries(&original.source)
        .map_err(|error| CoreError::new("operationConflict", error.to_string()))?
        .into_iter()
        .find(|definition| &definition.key == key)
        .ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                "The original save has no matching definition",
            )
        })?;
    let bytes = core.read_config_revision(
        &location.root,
        ConfigRole::Schema,
        &definition.binding.revision,
    )?;
    let source = String::from_utf8(bytes).map_err(|_| {
        CoreError::new(
            "schemaUnavailable",
            "The original saved schema is not UTF-8",
        )
    })?;
    let schema = Schema::from_author_json(&source)
        .map_err(|error| CoreError::new("schemaUnavailable", error.to_string()))?;
    let patch = weftext_data::upsert_saved_query_replay(
        &req.source,
        req.key.as_deref(),
        key,
        &req.label,
        &definition.binding,
        &req.plan,
        &schema,
    )
    .map_err(|error| CoreError::new("operationConflict", error.to_string()))?;
    if patch.source != original.source {
        return Err(CoreError::new(
            "operationConflict",
            "This retry does not reconstruct the exact original committed source",
        ));
    }
    let receipt = core.save(SaveRequest {
        session_id: req.session_id.clone(),
        base_id: req.base_id.clone(),
        source: patch.source.clone(),
        operation_id: req.operation_id.clone(),
    })?;
    let inventory = core.inventory(&location.root)?;
    let owner_id = weftext_engine::inspect_metadata(&req.source)
        .id
        .ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                "The original draft has no owner identity",
            )
        })?;
    let workspace = inventory.workspace_id.as_ref().ok_or_else(|| {
        CoreError::new(
            "committedResultUnavailable",
            "The original save committed, but the library is unavailable",
        )
    })?;
    let entity = format!("weftext-node://{workspace}/{owner_id}");
    let entry = resolve_entity(&inventory, &entity).map_err(|_| {
        CoreError::new(
            "committedResultUnavailable",
            "The original definition was saved; its live owner is now unavailable",
        )
    })?;
    let document = core.open(entry.path, Some(location.root.clone()))?;
    if document.node_id.as_deref() != Some(&owner_id) || document.kind != "managed" {
        return Err(CoreError::new(
            "committedResultUnavailable",
            "The original effect is retained, but its current source changed identity",
        ));
    }
    Ok(Some((document, definition, Some(receipt.base_id), patch)))
}

fn definition_write_response(
    state: &AppState,
    write: DefinitionWrite,
    serial: u64,
    before_source: &str,
) -> ApiResult {
    let (document, definition, revision, patch) = write;
    let document = value(document)?;
    remember(state, &document)?;
    let committed_source = patch.source.clone();
    let source_matches_commit = document_matches_committed_source(&document, &committed_source);
    let Json(patch_output) = patch_response(patch, serial, before_source)?;
    Ok(Json(
        json!({"document":document,"definition":definition,"definitionRevision":revision,
        "committedSource":committed_source,"documentSourceMatchesCommit":source_matches_commit,
        "patches":patch_output["patches"],"serial":serial,"sourceHash":patch_output["sourceHash"]}),
    ))
}

fn document_matches_committed_source(document: &Value, committed_source: &str) -> bool {
    document["source"].as_str() == Some(committed_source)
}

fn freeze_embedded_query(
    core: &mut Core,
    root: &str,
    path: &Path,
    source: &str,
    plan: weftext_data::QueryPlan,
    binding: weftext_data::SavedQueryBinding,
    parameters: BTreeMap<String, NativeValue>,
) -> Result<FrozenQuery, CoreError> {
    let inventory = core.inventory(root)?;
    let workspace = inventory.workspace_id.as_ref().ok_or_else(|| {
        CoreError::new("queryUnavailable", "普通 Typst 文件未取得资料库 Query 权限")
    })?;
    let owner = inventory
        .entries
        .iter()
        .find(|entry| {
            Path::new(&entry.path) == path
                && entry.kind == "node"
                && entry.identity_status == "admitted"
        })
        .ok_or_else(|| {
            CoreError::new(
                "queryUnavailable",
                "此文件不是原资料库已纳管的笔记；Query 不可用",
            )
        })?;
    let node = owner
        .node_id
        .as_deref()
        .ok_or_else(|| CoreError::new("queryUnavailable", "Query owner identity is unavailable"))?;
    let owner_ref = format!("weftext-node://{workspace}/{node}");
    let current_config = core.read_config(root, ConfigRole::Schema)?;
    if current_config.role_ref != binding.schema {
        return Err(CoreError::new(
            "schemaUnavailable",
            "Query schema belongs to a different configuration authority",
        ));
    }
    let bytes = core.read_config_revision(root, ConfigRole::Schema, &binding.revision)?;
    let baseline_source = String::from_utf8(bytes.clone())
        .map_err(|_| CoreError::new("schemaUnavailable", "Retained Query schema is not UTF-8"))?;
    let baseline = Schema::from_author_json(&baseline_source)
        .map_err(|e| CoreError::new("invalidSchema", e.to_string()))?;
    let (_, current_schema) = resolved_schema(core, Path::new(root))?;
    let projection = weftext_data::project(source, &baseline);
    if projection.qualification != "qualified" || projection.id.as_deref() != Some(node) {
        return Err(CoreError::new(
            "queryUnavailable",
            "Draft does not retain its original admitted owner identity",
        ));
    }
    if !weftext_data::baseline_contract_compatible(&baseline, &current_schema)
        .map_err(|e| CoreError::new("schemaUnavailable", e.to_string()))?
    {
        return Err(CoreError::new(
            "schemaMigrationUnavailable",
            "Saved Query baseline requires an explicit migration",
        ));
    }
    let config = SchemaSource {
        source: baseline_source,
        before: Some(bytes),
        role_ref: Some(binding.schema),
        revision: Some(binding.revision),
        can_install: false,
        base_id: None,
    };
    let mut frozen = query_render_snapshot(core, root, plan, Some((config, baseline)))?;
    freeze_query_definitions_with_draft(core, &mut frozen, parameters, Some((&owner_ref, source)))?;
    frozen.versions.push(json!({"role":"draftDefinition","owner":owner_ref,"sourceHash":weftext_engine::source_hash(source)}));
    Ok(frozen)
}

/// Embedded presentation freezes the author Draft's existing plan/baseline,
/// then consumes the same admitted cut, callee qualification and executor.
pub(crate) async fn run_embedded_query(
    state: &AppState,
    root: String,
    path: PathBuf,
    source: String,
    plan: weftext_data::QueryPlan,
    binding: weftext_data::SavedQueryBinding,
    parameters: BTreeMap<String, NativeValue>,
) -> Result<Value, ApiError> {
    let frozen = core_call(state, move |core| {
        freeze_embedded_query(core, &root, &path, &source, plan, binding, parameters)
    })
    .await?;
    evaluate_frozen(state, frozen).await
}

pub(crate) async fn validate_embedded_query_plan(
    state: &AppState,
    root: String,
    path: PathBuf,
    source: String,
    plan: weftext_data::QueryPlan,
    binding: weftext_data::SavedQueryBinding,
) -> Result<(), ApiError> {
    core_call(state, move |core| {
        let frozen =
            freeze_embedded_query(core, &root, &path, &source, plan, binding, BTreeMap::new())?;
        weftext_data::validate_query_plan_with_context(
            &frozen.plan,
            &frozen.schema,
            frozen.input.advanced.as_ref().unwrap(),
        )
        .map_err(|e| CoreError::new("invalidQuery", e.to_string()))
    })
    .await
}

async fn run_definition(
    State(state): State<AppState>,
    Json(req): Json<DefinitionRunRequest>,
) -> ApiResult {
    let (frozen, definition, revision, mode, owner_revision, changed) =
        core_call(&state, move |core| {
            let current = owner_source(core, &req.root, &req.owner)?;
            let node_id = current.entry.node_id.as_deref().unwrap();
            let (key, revision, mode, source) = match &req.selection {
                weftext_data::DefinitionSelection::Current { key } => (
                    key,
                    current.file.revision.clone(),
                    "current",
                    current.source.clone(),
                ),
                weftext_data::DefinitionSelection::Pinned { key, revision } => {
                    let bytes = core.read_source_revision(&req.root, node_id, revision)?;
                    let source = String::from_utf8(bytes).map_err(|_| {
                        CoreError::new(
                            "definitionUnavailable",
                            "Retained definition source is not UTF-8",
                        )
                    })?;
                    (key, Some(revision.clone()), "pinned", source)
                }
            };
            if !core.is_definition_key_admitted(&req.root, node_id, key)? {
                return Err(CoreError::new(
                    "definitionUnavailable",
                    "Definition key has no original Core admission",
                ));
            }
            let definition = weftext_data::saved_queries(&source)
                .map_err(|error| CoreError::new("invalidQueryDefinition", error.to_string()))?
                .into_iter()
                .find(|definition| &definition.key == key)
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        "Definition key is absent at the exact selected source revision",
                    )
                })?;
            let current_config = core.read_config(&req.root, ConfigRole::Schema)?;
            if current_config.role_ref != definition.binding.schema {
                return Err(CoreError::new(
                    "schemaUnavailable",
                    "Saved Query schema belongs to a different configuration authority",
                ));
            }
            let bytes = core.read_config_revision(
                &req.root,
                ConfigRole::Schema,
                &definition.binding.revision,
            )?;
            let baseline_source = String::from_utf8(bytes.clone()).map_err(|_| {
                CoreError::new("schemaUnavailable", "Retained Query schema is not UTF-8")
            })?;
            let baseline = Schema::from_author_json(&baseline_source)
                .map_err(|error| CoreError::new("invalidSchema", error.to_string()))?;
            let (_,current_schema) = resolved_schema(core,Path::new(&req.root))?;
            if !weftext_data::baseline_contract_compatible(&baseline,&current_schema)
                .map_err(|error|CoreError::new("schemaUnavailable",error.to_string()))? {
                return Err(CoreError::new("schemaMigrationUnavailable","Saved Query baseline contracts differ from the current schema; an explicit migration is required"));
            }
            let baseline_config = SchemaSource {
                source: baseline_source,
                before: Some(bytes),
                role_ref: Some(definition.binding.schema.clone()),
                revision: Some(definition.binding.revision.clone()),
                can_install: false,
                base_id: None,
            };
            let mut frozen = query_render_snapshot(
                core,
                &req.root,
                definition.plan.clone(),
                Some((baseline_config, baseline)),
            )?;
            freeze_query_definitions(core, &mut frozen, req.parameters)?;
            let latest =
                weftext_data::saved_queries(&current.source)
                    .ok()
                    .and_then(|definitions| {
                        definitions
                            .into_iter()
                            .find(|definition| &definition.key == key)
                    });
            let changed = latest.as_ref().is_none_or(|latest| {
                serde_json::to_value(latest).ok() != serde_json::to_value(&definition).ok()
            });
            Ok((
                frozen,
                definition,
                revision,
                mode,
                current.file.revision,
                changed,
            ))
        })
        .await?;
    Ok(Json(
        json!({"result":evaluate_frozen(&state,frozen).await?,"definition":definition,"definitionRevision":revision,
        "mode":mode,"currentOwnerRevision":owner_revision,"definitionChanged":changed}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok<T>(result: Result<T, ApiError>) -> T {
        result.unwrap_or_else(|error| panic!("{}: {}", error.code, error.message))
    }

    #[test]
    fn duplicate_capture_json_is_rejected_before_any_write() {
        let raw = r#"{"root":"library","parent":"library","name":"Test","types":["task"],"fields":{"status":{"kind":"text","value":"open"},"status":{"kind":"text","value":"done"}},"operationId":"00000000-0000-4000-8000-000000000001"}"#;
        assert!(serde_json::from_str::<CaptureRequest>(raw).is_err());
    }

    #[test]
    fn field_patch_transport_uses_utf16_for_non_ascii_source() {
        let before = "前😀\r\nvalue末";
        let from = before.find("value").unwrap();
        let to = from + "value".len();
        let patch = weftext_data::FieldPatch {
            from,
            to,
            text: "changed".into(),
            source: before.replacen("value", "changed", 1),
        };
        let Json(output) = ok(patch_response(patch, 7, before));
        assert_eq!(output["patches"][0]["from"], 5);
        assert_eq!(output["patches"][0]["to"], 10);
        assert_eq!(output["patches"][0]["byteFrom"], 9);
        assert_eq!(output["patches"][0]["expected"], "value");
        assert_eq!(output["patchUnit"], "utf16");
    }

    #[test]
    fn native_products_keep_title_absence_and_actual_generated_deep_headings() {
        let products = native_query_products(
            "exact-cut".into(),
            weftext_engine::RenderProducts {
                target: "html".into(),
                body_status: "complete".into(),
                body: Some("actual final native text".into()),
                title_status: "absent".into(),
                title: None,
                metadata_title: None,
                native_title_bodies: Vec::new(),
                headings: vec![weftext_engine::HeadingInfo {
                    text: "generated deep heading".into(),
                    path: None,
                    is_main: false,
                    level: 12,
                    from: 0,
                    to: 0,
                    utf16_from: 0,
                    utf16_to: 0,
                    generated: true,
                    visible: false,
                }],
            },
        );
        assert_eq!(products.title, weftext_data::Cell::Absent);
        assert_eq!(
            products.body.native(),
            Some(&NativeValue::Text("actual final native text".into()))
        );
        let heading = &products.headings.native().unwrap().array().unwrap()[0];
        assert_eq!(
            heading.member("level"),
            Some(&NativeValue::Int("12".into()))
        );
        assert_eq!(heading.member("generated"), Some(&NativeValue::Bool(true)));
        assert_eq!(
            heading
                .member("displayAssociation")
                .and_then(NativeValue::text),
            Some("unassociated")
        );
    }

    #[test]
    fn unsupported_native_body_and_ambiguous_title_remain_explicit() {
        let products = native_query_products(
            "exact-cut".into(),
            weftext_engine::RenderProducts {
                target: "html".into(),
                body_status: "unavailable".into(),
                body: None,
                title_status: "ambiguous".into(),
                title: None,
                metadata_title: Some("A".into()),
                native_title_bodies: vec!["B".into()],
                headings: Vec::new(),
            },
        );
        assert!(matches!(products.title, weftext_data::Cell::Invalid { .. }));
        assert!(matches!(
            products.body,
            weftext_data::Cell::Unavailable { .. }
        ));
    }

    #[cfg(windows)]
    fn fixture() -> (tempfile::TempDir, AppState, String) {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("library");
        let mut core = Core::new(directory.path().join("state")).unwrap();
        core.create_library(&root).unwrap();
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("fixture-token"),
            port: 12345,
            locations: Arc::new(Mutex::new(HashMap::new())),
            renders: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
            exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            initial: None,
        };
        (directory, state, root.to_string_lossy().to_string())
    }

    #[cfg(windows)]
    fn task_capture(root: &str, name: &str, operation_id: &str) -> CaptureRequest {
        CaptureRequest {
            root: root.into(),
            parent: root.into(),
            name: name.into(),
            types: vec!["task".into()],
            title: Some("精确记录 😀".into()),
            fields: BTreeMap::from([
                ("status".into(), NativeValue::Text("open".into())),
                ("start".into(), NativeValue::Date("2026-10-13".into())),
            ]),
            body: "作者正文与注释 // retained\n".into(),
            operation_id: operation_id.into(),
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn capture_is_atomic_admitted_and_retry_preserves_native_values() {
        let (_directory, state, root) = fixture();
        let operation = uuid::Uuid::new_v4().to_string();
        let Json(first) = ok(capture(
            State(state.clone()),
            Json(task_capture(&root, "Native", &operation)),
        )
        .await);
        let Json(retry) = ok(capture(
            State(state.clone()),
            Json(task_capture(&root, "Native", &operation)),
        )
        .await);
        assert_eq!(first["nodeId"], retry["nodeId"]);
        assert_eq!(first["source"], retry["source"]);
        let source = first["source"].as_str().unwrap();
        let schema = Schema::bundled().unwrap();
        let projection = weftext_data::project(source, &schema);
        assert_eq!(projection.types, vec!["task"]);
        assert_eq!(projection.id.as_deref(), first["nodeId"].as_str());
        assert_eq!(
            projection
                .fields
                .iter()
                .find(|field| field.key == "start")
                .unwrap()
                .cell
                .native(),
            Some(&NativeValue::Date("2026-10-13".into()))
        );
        assert!(source.contains("作者正文与注释 // retained"));
        let editor = weftext_engine::inspect_editor(source);
        let editor = serde_json::to_value(editor).unwrap();
        assert_eq!(editor["title"]["status"], "writable");
        assert_eq!(editor["title"]["text"], "精确记录 😀");
        let inventory = core_call(&state, move |core| core.inventory(root)).await;
        let inventory = ok(inventory);
        assert_eq!(
            inventory
                .entries
                .iter()
                .filter(|entry| entry.identity_status == "admitted")
                .count(),
            1
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn field_edit_preserves_exact_source_and_cannot_write_invalid_value() {
        let (_directory, state, root) = fixture();
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Edit",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let original = document["source"].as_str().unwrap().to_string();
        let source = format!("// 前😀\r\n{original}");
        let session = document["sessionId"].as_str().unwrap().to_string();
        let request = FieldEdit {
            session_id: session.clone(),
            source: source.clone(),
            serial: 1,
            source_hash: Some(weftext_engine::source_hash(&source)),
            key: "status".into(),
            path: vec![],
            value: NativeValue::Text("done".into()),
        };
        let Json(proposal) = ok(edit(State(state.clone()), Json(request)).await);
        let patch = &proposal["patches"][0];
        let from = patch["byteFrom"].as_u64().unwrap() as usize;
        let to = patch["byteTo"].as_u64().unwrap() as usize;
        let after = proposal["source"].as_str().unwrap();
        assert_eq!(&source[..from], &after[..from]);
        assert_eq!(
            &source[to..],
            &after[from + patch["text"].as_str().unwrap().len()..]
        );
        assert!(patch["from"].as_u64().unwrap() < patch["byteFrom"].as_u64().unwrap());
        let result = edit(
            State(state.clone()),
            Json(FieldEdit {
                session_id: session,
                source,
                serial: 1,
                source_hash: None,
                key: "status".into(),
                path: vec![],
                value: NativeValue::Bool(false),
            }),
        )
        .await;
        assert_eq!(result.err().unwrap().code, "invalid_data");
        assert_eq!(
            std::fs::read_to_string(document["path"].as_str().unwrap()).unwrap(),
            original
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn schema_prepare_install_checks_exact_base_and_external_changes() {
        let (_directory, state, root) = fixture();
        let Json(initial) = ok(schema(
            State(state.clone()),
            Json(RootRequest { root: root.clone() }),
        )
        .await);
        assert_eq!(initial["builtinDefault"], true);
        assert!(!Path::new(&root).join(".weftext-schema.json").exists());
        let mut edited: Value = serde_json::from_str(initial["source"].as_str().unwrap()).unwrap();
        edited["aliases"]["chores"] = json!("tasks/task");
        let new_source = serde_json::to_string(&edited).unwrap();
        let Json(preview) = ok(prepare_schema(
            State(state.clone()),
            Json(SchemaPrepare {
                root: root.clone(),
                source: new_source.clone(),
                before_source: initial["source"].as_str().unwrap().into(),
                before_revision: None,
                operation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
        .await);
        let _ = ok(install_schema(
            State(state.clone()),
            Json(SchemaInstall {
                operation_id: preview["operationId"].as_str().unwrap().into(),
                plan_id: preview["planId"].as_str().unwrap().into(),
            }),
        )
        .await);
        let Json(installed) = ok(schema(
            State(state.clone()),
            Json(RootRequest { root: root.clone() }),
        )
        .await);
        assert_eq!(installed["source"], new_source);
        assert_eq!(installed["schema"]["aliases"]["chores"], "tasks/task");
        let next_source = format!("{new_source}\n");
        let Json(next) = ok(prepare_schema(
            State(state.clone()),
            Json(SchemaPrepare {
                root: root.clone(),
                source: next_source,
                before_source: new_source.clone(),
                before_revision: installed["revision"].as_str().map(str::to_owned),
                operation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
        .await);
        // This explicitly simulates an external editor on an owned test fixture.
        std::fs::write(
            Path::new(&root).join(".weftext-schema.json"),
            format!("{new_source}  "),
        )
        .unwrap();
        let result = install_schema(
            State(state),
            Json(SchemaInstall {
                operation_id: next["operationId"].as_str().unwrap().into(),
                plan_id: next["planId"].as_str().unwrap().into(),
            }),
        )
        .await;
        assert_eq!(result.err().unwrap().code, "conflict");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn query_freezes_complete_admitted_scope_and_stable_resolve_survives_rename() {
        let (_directory, state, root) = fixture();
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Before",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let entity = format!(
            "weftext-node://{}/{}",
            document["workspaceId"].as_str().unwrap(),
            document["nodeId"].as_str().unwrap()
        );
        let plan: weftext_data::QueryPlan = serde_json::from_value(json!({"scope":{"directory":entity,"descendants":true},"types":["task"],"where":"status == 'open'","select":[{"name":"status","expr":"status"}]})).unwrap();
        let Json(rows) = ok(query(
            State(state.clone()),
            Json(QueryRequest {
                parameters: BTreeMap::new(),
                root: root.clone(),
                plan,
            }),
        )
        .await);
        assert_eq!(rows["rows"].as_array().unwrap().len(), 1);
        assert_eq!(rows["rows"][0]["entityKey"], entity);
        assert_eq!(rows["coverage"], "complete");
        assert_eq!(rows["freshness"], "stale");
        assert_eq!(rows["aggregateExact"], true);
        assert_eq!(rows["snapshotMode"], "admittedFrozen");
        let old_source = document["path"].as_str().unwrap().to_string();
        let source_directory = Path::new(&old_source)
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let operation_root = root.clone();
        ok(core_call(&state, move |core| {
            let preview = core.prepare_operation(CoreOperationRequest {
                root: operation_root,
                operation_id: uuid::Uuid::new_v4().to_string(),
                intent: OperationIntent::RenameNode {
                    source: source_directory,
                    name: "After".into(),
                },
            })?;
            core.execute_operation(&preview.operation_id, &preview.plan_id)
        })
        .await);
        let Json(target) = ok(resolve(
            State(state.clone()),
            Json(ResolveRequest {
                root: root.clone(),
                entity_key: entity.clone(),
                ..Default::default()
            }),
        )
        .await);
        assert_ne!(target["path"], old_source);
        assert!(
            target["path"]
                .as_str()
                .unwrap()
                .ends_with("After\\After.typ")
        );
        let mut plan = weftext_data::QueryPlan::default();
        plan.scope.directory = Some(entity.clone());
        plan.select = vec![weftext_data::QueryColumn {
            name: "status".into(),
            expr: "status".into(),
        }];
        let Json(renamed_rows) = ok(query(
            State(state.clone()),
            Json(QueryRequest {
                root: root.clone(),
                plan,
                ..Default::default()
            }),
        )
        .await);
        assert_eq!(renamed_rows["rows"].as_array().unwrap().len(), 1);
        assert_eq!(renamed_rows["rows"][0]["path"], target["path"]);
        let Json(opened) = ok(open_entity(
            State(state.clone()),
            Json(ResolveRequest {
                root: root.clone(),
                entity_key: entity,
                ..Default::default()
            }),
        )
        .await);
        assert_eq!(opened["nodeId"], document["nodeId"]);
        let mut unavailable = weftext_data::QueryPlan::default();
        unavailable.scope.directory = Some(format!(
            "weftext-node://{}/{}",
            document["workspaceId"].as_str().unwrap(),
            uuid::Uuid::new_v4()
        ));
        let result = query(
            State(state),
            Json(QueryRequest {
                root,
                plan: unavailable,
                ..Default::default()
            }),
        )
        .await;
        assert_eq!(result.err().unwrap().code, "queryScopeUnavailable");
    }

    #[cfg(windows)]
    async fn install_bundled_schema(state: &AppState, root: &str) {
        let Json(initial) = ok(schema(
            State(state.clone()),
            Json(RootRequest { root: root.into() }),
        )
        .await);
        let source = initial["source"].as_str().unwrap().to_string();
        let Json(preview) = ok(prepare_schema(
            State(state.clone()),
            Json(SchemaPrepare {
                root: root.into(),
                source: source.clone(),
                before_source: source,
                before_revision: None,
                operation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
        .await);
        let _ = ok(install_schema(
            State(state.clone()),
            Json(SchemaInstall {
                operation_id: preview["operationId"].as_str().unwrap().into(),
                plan_id: preview["planId"].as_str().unwrap().into(),
            }),
        )
        .await);
    }

    #[cfg(windows)]
    fn status_plan(status: &str) -> weftext_data::QueryPlan {
        serde_json::from_value(
            json!({"types":["task"],"where":format!("status == '{status}'"),
            "select":[{"name":"status","expr":"status"}]}),
        )
        .unwrap()
    }

    #[cfg(windows)]
    fn definition_request(
        document: &Value,
        source: String,
        key: Option<String>,
        status: &str,
    ) -> DefinitionSaveRequest {
        DefinitionSaveRequest {
            session_id: document["sessionId"].as_str().unwrap().into(),
            source,
            serial: 1,
            source_hash: None,
            base_id: document["baseId"].as_str().unwrap().into(),
            key,
            label: "weftext-query-status".into(),
            plan: status_plan(status),
            operation_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn embedded_draft_uses_shared_executor_without_saving_and_pinned_calls_ignore_draft_definition()
     {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Embedded",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let Json(saved) = ok(save_definition(
            State(state.clone()),
            Json(definition_request(
                &document,
                document["source"].as_str().unwrap().into(),
                None,
                "open",
            )),
        )
        .await);
        let current = &saved["document"];
        let committed = saved["committedSource"].as_str().unwrap();
        let original = weftext_data::embedded_queries(committed).pop().unwrap();
        let draft = weftext_data::edit_embedded_query_plan(
            committed,
            &original.source.id,
            &status_plan("done"),
        )
        .unwrap()
        .source;
        let direct = weftext_data::embedded_queries(&draft).pop().unwrap();
        let path = PathBuf::from(current["path"].as_str().unwrap());
        let result = ok(run_embedded_query(
            &state,
            root.clone(),
            path.clone(),
            draft.clone(),
            direct.plan.clone().unwrap(),
            direct.binding.clone().unwrap(),
            BTreeMap::new(),
        )
        .await);
        assert!(result["rows"].as_array().unwrap().is_empty());
        assert_eq!(result["snapshotMode"], "admittedFrozen");
        assert_eq!(result["freshness"], "stale");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), committed);
        let owner = format!(
            "weftext-node://{}/{}",
            current["workspaceId"].as_str().unwrap(),
            current["nodeId"].as_str().unwrap()
        );
        for (selection, expected) in [
            (
                weftext_data::DefinitionSelection::Current {
                    key: original.key.clone().unwrap(),
                },
                0,
            ),
            (
                weftext_data::DefinitionSelection::Pinned {
                    key: original.key.clone().unwrap(),
                    revision: saved["definitionRevision"].as_str().unwrap().into(),
                },
                1,
            ),
        ] {
            let plan = weftext_data::QueryPlan {
                advanced: Some(weftext_data::QueryAdvanced {
                    call: Some(weftext_data::QueryCall {
                        owner: owner.clone(),
                        selection,
                        arguments: BTreeMap::new(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let result = ok(run_embedded_query(
                &state,
                root.clone(),
                path.clone(),
                draft.clone(),
                plan,
                direct.binding.clone().unwrap(),
                BTreeMap::new(),
            )
            .await);
            assert_eq!(result["rows"].as_array().unwrap().len(), expected);
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), committed);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn saved_queries_retain_exact_definition_history_and_use_current_data() {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Saved",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let owner = format!(
            "weftext-node://{}/{}",
            document["workspaceId"].as_str().unwrap(),
            document["nodeId"].as_str().unwrap()
        );
        let dirty_source = format!(
            "{}\n用户尚未保存的正文 😀\n",
            document["source"].as_str().unwrap()
        );
        let Json(first) = ok(save_definition(
            State(state.clone()),
            Json(definition_request(&document, dirty_source, None, "open")),
        )
        .await);
        assert!(
            first["document"]["source"]
                .as_str()
                .unwrap()
                .contains("用户尚未保存的正文 😀")
        );
        let key = first["definition"]["key"].as_str().unwrap().to_string();
        let old_revision = first["definitionRevision"].as_str().unwrap().to_string();
        let first_document = &first["document"];
        let Json(second) = ok(save_definition(
            State(state.clone()),
            Json(definition_request(
                first_document,
                first_document["source"].as_str().unwrap().into(),
                Some(key.clone()),
                "done",
            )),
        )
        .await);
        assert_ne!(second["definitionRevision"], old_revision);
        let Json(current) = ok(run_definition(
            State(state.clone()),
            Json(DefinitionRunRequest {
                parameters: BTreeMap::new(),
                root: root.clone(),
                owner: owner.clone(),
                selection: weftext_data::DefinitionSelection::Current { key: key.clone() },
            }),
        )
        .await);
        assert!(current["result"]["rows"].as_array().unwrap().is_empty());
        let Json(pinned) = ok(run_definition(
            State(state.clone()),
            Json(DefinitionRunRequest {
                parameters: BTreeMap::new(),
                root: root.clone(),
                owner: owner.clone(),
                selection: weftext_data::DefinitionSelection::Pinned {
                    key: key.clone(),
                    revision: old_revision.clone(),
                },
            }),
        )
        .await);
        assert_eq!(pinned["result"]["rows"].as_array().unwrap().len(), 1);
        assert_eq!(pinned["definitionRevision"], old_revision);
        assert_eq!(pinned["definitionChanged"], true);
        let saved_document = &second["document"];
        let current_source = saved_document["source"].as_str().unwrap();
        let patch = weftext_data::patch_field(
            current_source,
            &Schema::bundled().unwrap(),
            "status",
            &[],
            &NativeValue::Text("done".into()),
        )
        .unwrap();
        let save = SaveRequest {
            session_id: saved_document["sessionId"].as_str().unwrap().into(),
            base_id: saved_document["baseId"].as_str().unwrap().into(),
            source: patch.source,
            operation_id: uuid::Uuid::new_v4().to_string(),
        };
        ok(core_call(&state, move |core| core.save(save)).await);
        let Json(after_data_change) = ok(run_definition(
            State(state.clone()),
            Json(DefinitionRunRequest {
                parameters: BTreeMap::new(),
                root: root.clone(),
                owner: owner.clone(),
                selection: weftext_data::DefinitionSelection::Pinned {
                    key: key.clone(),
                    revision: old_revision,
                },
            }),
        )
        .await);
        assert!(
            after_data_change["result"]["rows"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let missing = run_definition(
            State(state),
            Json(DefinitionRunRequest {
                parameters: BTreeMap::new(),
                root,
                owner,
                selection: weftext_data::DefinitionSelection::Pinned {
                    key,
                    revision: uuid::Uuid::new_v4().to_string(),
                },
            }),
        )
        .await;
        assert_eq!(missing.err().unwrap().code, "revisionUnavailable");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn admitted_complete_zero_and_external_pending_remain_distinct() {
        let (_directory, state, root) = fixture();
        let Json(empty) = ok(query(
            State(state.clone()),
            Json(QueryRequest {
                parameters: BTreeMap::new(),
                root: root.clone(),
                plan: status_plan("done"),
            }),
        )
        .await);
        assert!(empty["rows"].as_array().unwrap().is_empty());
        assert_eq!(empty["coverage"], "complete");
        assert_eq!(empty["aggregateExact"], true);
        let external = Path::new(&root).join("External");
        std::fs::create_dir(&external).unwrap();
        std::fs::write(external.join("External.typ"),format!("#metadata((id: \"{}\", types: (\"task\",), status: \"done\")) <weftext>\n外部未准入笔记\n",uuid::Uuid::new_v4())).unwrap();
        let Json(raw) = ok(query(
            State(state),
            Json(QueryRequest {
                parameters: BTreeMap::new(),
                root,
                plan: status_plan("done"),
            }),
        )
        .await);
        assert!(raw["rows"].as_array().unwrap().is_empty());
        assert_eq!(raw["coverage"], "complete");
        assert_eq!(raw["externalPending"], true);
        assert_eq!(raw["rawScope"]["pendingCount"], 1);
        assert_eq!(raw["snapshotLabel"], "已纳管笔记快照");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn periods_use_exact_iso_keys_and_unique_policy_blocks_every_creation_route() {
        let (_directory, state, root) = fixture();
        let Json(initial) = ok(resolve_period(
            State(state.clone()),
            Json(PeriodRequest {
                root: root.clone(),
                series_key: "work-log".into(),
                kind: "week".into(),
                key: "2020-W53".into(),
                steps: 0,
            }),
        )
        .await);
        assert_eq!(initial["coverage"], "complete");
        assert_eq!(initial["canCreate"], true);
        let bounds: NativeValue = serde_json::from_value(initial["bounds"].clone()).unwrap();
        assert_eq!(
            bounds.member("start"),
            Some(&NativeValue::Date("2020-12-28".into()))
        );
        assert_eq!(
            bounds.member("end_exclusive"),
            Some(&NativeValue::Date("2021-01-04".into()))
        );
        let operation = uuid::Uuid::new_v4().to_string();
        let make = || PeriodCreateRequest {
            root: root.clone(),
            parent: root.clone(),
            name: "ISOWeek".into(),
            series_key: "work-log".into(),
            kind: "week".into(),
            key: "2020-W53".into(),
            title: None,
            body: "周记正文\n".into(),
            operation_id: operation.clone(),
        };
        let Json(created) = ok(create_period(State(state.clone()), Json(make())).await);
        let Json(retry) = ok(create_period(State(state.clone()), Json(make())).await);
        assert_eq!(created["nodeId"], retry["nodeId"]);
        let Json(found) = ok(resolve_period(
            State(state.clone()),
            Json(PeriodRequest {
                root: root.clone(),
                series_key: "work-log".into(),
                kind: "week".into(),
                key: "2020-W53".into(),
                steps: 0,
            }),
        )
        .await);
        assert_eq!(found["matches"].as_array().unwrap().len(), 1);
        assert_eq!(found["canCreate"], false);
        let duplicate = capture(
            State(state.clone()),
            Json(CaptureRequest {
                root: root.clone(),
                parent: root.clone(),
                name: "Duplicate".into(),
                types: vec!["period-note".into()],
                title: None,
                fields: BTreeMap::from([(
                    "calendar/period".into(),
                    serde_json::from_value(found["period"].clone()).unwrap(),
                )]),
                body: String::new(),
                operation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
        .await;
        assert_eq!(duplicate.err().unwrap().code, "periodConflict");
        assert!(!Path::new(&root).join("Duplicate").exists());
        let Json(next) = ok(resolve_period(
            State(state.clone()),
            Json(PeriodRequest {
                root: root.clone(),
                series_key: "work-log".into(),
                kind: "week".into(),
                key: "2020-W53".into(),
                steps: 1,
            }),
        )
        .await);
        let next_value: NativeValue = serde_json::from_value(next["period"].clone()).unwrap();
        assert_eq!(
            next_value.member("key").and_then(NativeValue::text),
            Some("2021-W01")
        );
        let invalid = resolve_period(
            State(state),
            Json(PeriodRequest {
                root,
                series_key: "work-log".into(),
                kind: "week".into(),
                key: "2021-W53".into(),
                steps: 0,
            }),
        )
        .await;
        assert_eq!(invalid.err().unwrap().code, "invalidPeriod");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn trash_queries_are_actual_read_only_snapshots_and_field_writers_are_blocked() {
        let (_directory, state, root) = fixture();
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Trash",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let source_directory = Path::new(document["path"].as_str().unwrap())
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let operation_root = root.clone();
        ok(core_call(&state, move |core| {
            let preview = core.prepare_operation(CoreOperationRequest {
                root: operation_root,
                operation_id: uuid::Uuid::new_v4().to_string(),
                intent: OperationIntent::TrashNode {
                    source: source_directory,
                },
            })?;
            core.execute_operation(&preview.operation_id, &preview.plan_id)
        })
        .await);
        let mut plan = status_plan("open");
        plan.scope.lifecycle = "trash".into();
        let Json(rows) = ok(query(
            State(state.clone()),
            Json(QueryRequest {
                parameters: BTreeMap::new(),
                root: root.clone(),
                plan,
            }),
        )
        .await);
        assert_eq!(rows["coverage"], "complete");
        assert_eq!(rows["rows"].as_array().unwrap().len(), 1);
        assert_eq!(rows["rows"][0]["readOnly"], true);
        let Json(opened) = ok(open_entity(
            State(state.clone()),
            Json(ResolveRequest {
                root: root.clone(),
                entity_key: rows["rows"][0]["entityKey"].as_str().unwrap().into(),
                lifecycle: Some("trash".into()),
                revision: Some(rows["rows"][0]["revision"].as_str().unwrap().into()),
            }),
        )
        .await);
        assert_eq!(opened["kind"], "managedTrash");
        assert_eq!(opened["readOnly"], true);
        let source = opened["source"].as_str().unwrap().to_string();
        let session = opened["sessionId"].as_str().unwrap().to_string();
        let Json(projection) = ok(project(
            State(state.clone()),
            Json(ProjectionRequest {
                session_id: session.clone(),
                source: source.clone(),
                serial: 1,
                source_hash: None,
            }),
        )
        .await);
        assert_eq!(projection["readOnly"], true);
        assert!(
            projection["projection"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .all(|field| field["writable"] == false)
        );
        let failed = edit(
            State(state),
            Json(FieldEdit {
                session_id: session,
                source,
                serial: 1,
                source_hash: None,
                key: "status".into(),
                path: Vec::new(),
                value: NativeValue::Text("done".into()),
            }),
        )
        .await;
        assert_eq!(failed.err().unwrap().code, "trashReadOnly");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn post_commit_external_source_is_retained_and_never_claimed_as_owned_write() {
        let (_directory, state, root) = fixture();
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Race",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let committed = format!("{}\nowned B\n", document["source"].as_str().unwrap());
        let save = SaveRequest {
            session_id: document["sessionId"].as_str().unwrap().into(),
            base_id: document["baseId"].as_str().unwrap().into(),
            source: committed.clone(),
            operation_id: uuid::Uuid::new_v4().to_string(),
        };
        let receipt = ok(core_call(&state, move |core| core.save(save)).await);
        let external = format!("{committed}\nexternal C\n");
        let path = document["path"].as_str().unwrap().to_string();
        // The external editor is deliberately injected after owned B committed
        // and before the API's fresh reopen, on this test-owned fixture only.
        std::fs::write(&path, &external).unwrap();
        let opened_root = root.clone();
        let reopened = value(ok(core_call(&state, move |core| {
            core.open(path, Some(PathBuf::from(opened_root)))
        })
        .await))
        .unwrap_or_else(|error| panic!("{}", error.message));
        assert_eq!(reopened["source"], external);
        assert!(!document_matches_committed_source(&reopened, &committed));
        let owner = document["nodeId"].as_str().unwrap().to_string();
        let retained = ok(core_call(&state, move |core| {
            core.read_source_revision(root, &owner, &receipt.base_id)
        })
        .await);
        assert_eq!(retained, committed.as_bytes());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn definition_retry_replays_original_effect_after_newer_draft_and_schema_change() {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Retry",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let request = definition_request(
            &document,
            document["source"].as_str().unwrap().into(),
            None,
            "open",
        );
        let Json(first) = ok(save_definition(State(state.clone()), Json(request.clone())).await);
        let later_draft = format!(
            "{}\nlater unsaved input\n",
            first["committedSource"].as_str().unwrap()
        );
        ok(writing_api::observe_snapshot(
            &state,
            &request.session_id,
            &later_draft,
            2,
            None,
        ));
        let Json(current_schema) = ok(schema(
            State(state.clone()),
            Json(RootRequest { root: root.clone() }),
        )
        .await);
        let changed_schema = format!("{}\n ", current_schema["source"].as_str().unwrap());
        let Json(preview) = ok(prepare_schema(
            State(state.clone()),
            Json(SchemaPrepare {
                root: root.clone(),
                source: changed_schema,
                before_source: current_schema["source"].as_str().unwrap().into(),
                before_revision: current_schema["revision"].as_str().map(str::to_owned),
                operation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
        .await);
        let _ = ok(install_schema(
            State(state.clone()),
            Json(SchemaInstall {
                operation_id: preview["operationId"].as_str().unwrap().into(),
                plan_id: preview["planId"].as_str().unwrap().into(),
            }),
        )
        .await);
        let Json(replayed) = ok(save_definition(State(state.clone()), Json(request.clone())).await);
        assert_eq!(replayed["definitionRevision"], first["definitionRevision"]);
        assert_eq!(replayed["definition"]["key"], first["definition"]["key"]);
        assert_eq!(replayed["committedSource"], first["committedSource"]);
        ok(writing_api::check_snapshot(
            &state,
            &request.session_id,
            &later_draft,
            2,
            None,
        ));
        let definitions =
            weftext_data::saved_queries(replayed["committedSource"].as_str().unwrap()).unwrap();
        assert_eq!(definitions.len(), 1);
        let mut changed = request;
        changed.label = "different input".into();
        let rejected = save_definition(State(state), Json(changed)).await;
        assert_eq!(rejected.err().unwrap().code, "operationConflict");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn saved_definition_keeps_typed_scope_and_resolves_it_after_owner_rename() {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(document) = ok(capture(
            State(state.clone()),
            Json(task_capture(
                &root,
                "Scoped",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let owner = format!(
            "weftext-node://{}/{}",
            document["workspaceId"].as_str().unwrap(),
            document["nodeId"].as_str().unwrap()
        );
        let mut request = definition_request(
            &document,
            document["source"].as_str().unwrap().into(),
            None,
            "open",
        );
        request.plan.scope.directory = Some(owner.clone());
        let Json(saved) = ok(save_definition(State(state.clone()), Json(request)).await);
        assert_eq!(saved["definition"]["plan"]["scope"]["directory"], owner);
        let source_directory = Path::new(saved["document"]["path"].as_str().unwrap())
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let rename_root = root.clone();
        ok(core_call(&state, move |core| {
            let preview = core.prepare_operation(CoreOperationRequest {
                root: rename_root,
                operation_id: uuid::Uuid::new_v4().to_string(),
                intent: OperationIntent::RenameNode {
                    source: source_directory,
                    name: "ScopedAfter".into(),
                },
            })?;
            core.execute_operation(&preview.operation_id, &preview.plan_id)
        })
        .await);
        let Json(result) = ok(run_definition(
            State(state),
            Json(DefinitionRunRequest {
                root,
                owner,
                selection: weftext_data::DefinitionSelection::Pinned {
                    key: saved["definition"]["key"].as_str().unwrap().into(),
                    revision: saved["definitionRevision"].as_str().unwrap().into(),
                },
                parameters: BTreeMap::new(),
            }),
        )
        .await);
        assert_eq!(result["result"]["rows"].as_array().unwrap().len(), 1);
        assert!(
            result["result"]["rows"][0]["path"]
                .as_str()
                .unwrap()
                .ends_with("ScopedAfter\\ScopedAfter.typ")
        );
        assert_eq!(result["result"]["rows"][0]["lifecycle"], "live");
    }
}
