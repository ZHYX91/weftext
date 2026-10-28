//! Structured data, exact native source proposals, and bounded local Query.
//! Core owns every filesystem read/write and the admitted note identity.

use super::*;
#[cfg(test)]
use crate::CoreTestFixture;
use crate::readonly_evidence::capture_readonly;
use serde::de::{MapAccess, Visitor};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
};
use weftext_core::{ConfigRole, OperationIntent, OperationRequest as CoreOperationRequest};
use weftext_data::{NativeValue, Schema, ValuePath};
use weftext_messages::UiMessage;

const BUNDLED_SCHEMA_SOURCE: &str =
    include_str!("../../../docs/design/examples/schema.authoring.json");
#[path = "reminder_api.rs"]
mod reminder_api;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .merge(crate::capture_api::router())
        .merge(reminder_api::router())
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
        .route("/api/query/definition/draft", post(draft_definition))
        .route("/api/query/definition/pending", post(pending_definition))
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
    input_revision: Option<u64>,
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

#[derive(Serialize, Deserialize)]
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

pub(crate) use weftext_core::CaptureCreationRequest as CaptureRequest;

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

#[derive(Clone)]
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
                .map_err(|_| {
                    CoreError::new(
                        "invalidSchema",
                        UiMessage::error_api_data_schema_configuration_encoding(),
                    )
                })?
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
        .map_err(|error| CoreError::from_data("invalidSchema", error))?;
    Ok((config, schema))
}

pub(crate) async fn knowledge_schema(
    state: &AppState,
    root: PathBuf,
) -> Result<(Schema, String), ApiError> {
    let (config, _evidence) = capture_readonly(state, move |capture| {
        capture.read_config(&root, ConfigRole::Schema)
    })
    .await?;
    knowledge_schema_from_bytes(config.bytes.as_deref(), config.revision.as_deref())
        .map_err(ApiError::from)
}

pub(crate) fn knowledge_schema_from_snapshot(
    snapshot: &weftext_core::SnapshotFile,
) -> Result<(Schema, String), CoreError> {
    knowledge_schema_from_bytes(snapshot.bytes.as_deref(), snapshot.revision.as_deref())
}

fn knowledge_schema_from_bytes(
    bytes: Option<&[u8]>,
    revision: Option<&str>,
) -> Result<(Schema, String), CoreError> {
    let source = bytes
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|_| {
            CoreError::new(
                "invalid_request",
                UiMessage::error_api_export_conversion_frozen_schema_encoding(),
            )
        })?
        .unwrap_or(BUNDLED_SCHEMA_SOURCE);
    let schema = Schema::from_author_json(source)
        .map_err(|error| CoreError::from_data("invalid_data", error))?;
    let generation = format!(
        "{}:{}",
        revision.unwrap_or("observed"),
        weftext_engine::source_hash(source)
    );
    Ok((schema, generation))
}

fn data_error(error: weftext_data::DataError) -> ApiError {
    ApiError::from_data(StatusCode::BAD_REQUEST, "invalid_data", error)
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
                UiMessage::error_api_data_schema_changed_during_evaluation(),
            ));
        }
        Ok(())
    })
    .await
}

fn schema_parse_error(config: &SchemaSource, failure: weftext_data::DataError) -> ApiError {
    let cause = data_error(failure);
    let mut error = cause.clone();
    error.code = "invalid_schema".into();
    error.details = Some(Box::new(ApiErrorDetails::SchemaSource {
        source: config.source.clone(),
        role_ref: config.role_ref.clone(),
        revision: config.revision.clone(),
        cause: Box::new(cause),
    }));
    error
}

fn original_operation_error(mut error: ApiError, fact: Value) -> ApiError {
    let cause = Box::new(error.clone());
    error.details = Some(Box::new(ApiErrorDetails::OriginalOperation { fact, cause }));
    error
}

async fn schema(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RootRequest>,
) -> ApiResult {
    let config = core_call(&state, move |core| {
        schema_source(core, Path::new(&req.root))
    })
    .await?;
    let resolved = Schema::from_author_json(&config.source)
        .map_err(|error| schema_parse_error(&config, error))?;
    Ok(Json(
        json!({"semanticDigest":schema_semantic_digest(&resolved)?,"schema":resolved,"source":config.source,"roleRef":config.role_ref,
        "revision":config.revision,"builtinDefault":config.before.is_none(),"canInstall":config.can_install}),
    ))
}
fn schema_semantic_digest(schema: &Schema) -> Result<String, ApiError> {
    Ok(weftext_engine::source_hash(
        &serde_json::to_string(schema).map_err(|_| {
            ApiError::internal(
                weftext_messages::UiMessage::error_api_data_api_schema_serialization_failed(),
            )
        })?,
    ))
}

async fn prepare_schema(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<SchemaPrepare>,
) -> ApiResult {
    if req.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_data_schema_configuration_budget(),
        ));
    }
    Schema::from_author_json(&req.source).map_err(data_error)?;
    let preview = core_call(&state, move |core| {
        let current = schema_source(core, Path::new(&req.root))?;
        if !current.can_install {
            return Err(CoreError::new(
                "unadmittedLibrary",
                UiMessage::error_api_data_managed_library_before_schema(),
            ));
        }
        if current.source != req.before_source || current.revision != req.before_revision {
            return Err(CoreError::new(
                "configConflict",
                UiMessage::error_api_data_exact_schema_base_changed(),
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
    RequestJson(req): RequestJson<SchemaInstall>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            let preview = core.operation_details(&req.operation_id)?;
            if preview.changes.len() != 1
                || !preview.changes[0].path.ends_with(".weftext-schema.json")
            {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_api_data_schema_installation_plan_required(),
                ));
            }
            core.execute_operation(&req.operation_id, &req.plan_id)
        })
        .await?,
    )?))
}

async fn project(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<ProjectionRequest>,
) -> ApiResult {
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
        crate::work::spawn_blocking(move || weftext_data::project(&source, &resolved))
            .await
            .map_err(|_| ApiError::internal(UiMessage::error_api_data_projection_incomplete()))?;
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

async fn edit(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<FieldEdit>,
) -> ApiResult {
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
        .ok_or_else(|| ApiError::bad(UiMessage::error_api_data_selected_field_not_installed()))?;
    let patch = crate::work::spawn_blocking(move || {
        weftext_data::patch_field(&source, &resolved, &req.key, &req.path, &req.value)
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_data_data_edit_incomplete()))?
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

async fn types(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<TypeEdit>,
) -> ApiResult {
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
    let patch = crate::work::spawn_blocking(move || {
        weftext_data::patch_types(&source, &resolved, &req.types)
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_data_type_edit_incomplete()))?
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
    let prefix = before.get(..patch.from).ok_or_else(|| {
        ApiError::internal(UiMessage::error_api_data_source_patch_boundary_invalid())
    })?;
    let end = before.get(..patch.to).ok_or_else(|| {
        ApiError::internal(UiMessage::error_api_data_source_patch_boundary_invalid())
    })?;
    let expected = before.get(patch.from..patch.to).ok_or_else(|| {
        ApiError::internal(UiMessage::error_api_data_source_patch_range_invalid())
    })?;
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
            UiMessage::error_api_data_trash_read_only(),
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
        let cuts = frozen
            .as_ref()
            .map(frozen_query_cut_ids)
            .unwrap_or_default();
        let result = (|| {
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
                        proposed.id.as_ref().is_some_and(|id| {
                            note.entity_key.rsplit('/').next() == Some(id.as_str())
                        })
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
                _ => weftext_data::validate_relation_change(
                    &before, &after, &schema, &targets, complete,
                ),
            };
            if !check.is_valid() {
                return Err(CoreError::from_product(
                    "invalidRelation",
                    None,
                    UiMessage::error_api_data_proposal_validation_failed().into(),
                    check
                        .diagnostics
                        .into_iter()
                        .chain(check.unavailable)
                        .collect(),
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
                Some(field_id) => weftext_data::validate_field_change(
                    &before, &after, &schema, &field_id, &context,
                ),
                None => weftext_data::validate_note(&after, &schema, &context),
            };
            if !validation.is_valid() {
                return Err(CoreError::from_product(
                    "invalidData",
                    None,
                    UiMessage::error_api_data_proposal_validation_failed().into(),
                    validation
                        .diagnostics
                        .into_iter()
                        .chain(validation.unavailable)
                        .collect(),
                ));
            }
            Ok(())
        })();
        release_query_cuts(core, &cuts);
        result
    })
    .await
}

fn capture_template(
    req: &CaptureRequest,
    resolved: &Schema,
) -> Result<String, weftext_data::DataError> {
    for key in req.fields.keys() {
        if resolved.fields.iter().any(|field| field.id == *key) {
            continue;
        }
        if let Some(field) = resolved.fields.iter().find(|field| field.key == *key) {
            return Err(weftext_data::DataError::new(
                UiMessage::error_capture_field_identity_required(
                    weftext_messages::params::ErrorCaptureFieldIdentityRequired {
                        field: field.id.clone(),
                    },
                ),
            ));
        }
        return Err(weftext_data::DataError::new(
            UiMessage::error_api_data_capture_field_unknown(
                weftext_messages::params::ErrorApiDataCaptureFieldUnknown { field: key.clone() },
            ),
        ));
    }
    let body = capture_body(req)?;
    weftext_data::capture_template(&req.types, &req.fields, &body, resolved)
}

fn capture_body(req: &CaptureRequest) -> Result<String, weftext_data::DataError> {
    let body = match &req.body_mode {
        weftext_core::CaptureBodyMode::Text => {
            weftext_engine::encode_literal(&req.body).map_err(weftext_data::DataError::from)?
        }
        weftext_core::CaptureBodyMode::Markup => req.body.clone(),
    };
    Ok(match &req.title {
        Some(title) => format!(
            "\n#set document(title: [{}])\n#title()\n\n{}",
            weftext_engine::encode_literal(title).map_err(weftext_data::DataError::from)?,
            body
        ),
        None => body,
    })
}

type CreationCommit = (
    weftext_core::OperationPreview,
    weftext_core::OperationReceipt,
);
async fn capture(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<CaptureRequest>,
) -> ApiResult {
    let (preview, receipt) = core_call(&state, move |core| capture_document(core, &req)).await?;
    crate::knowledge_api::operation_output(&state, preview, receipt, None)
        .await
        .map(Json)
}
fn capture_intent(req: &CaptureRequest) -> Result<Value, CoreError> {
    Ok(json!({"kind":"capture", "request":serde_json::to_value(req)?}))
}
fn original_creation(
    core: &mut Core,
    id: &str,
    intent: &Value,
) -> Result<Option<CreationCommit>, CoreError> {
    let Some(preview) = core.feature_operation_status(id, intent)? else {
        return Ok(None);
    };
    // Never reinterpret the original author request using today's schema.
    let receipt = core.execute_operation(id, &preview.plan_id)?;
    Ok(Some((preview, receipt)))
}
pub(crate) fn capture_document(
    core: &mut Core,
    req: &CaptureRequest,
) -> Result<CreationCommit, CoreError> {
    let intent = capture_intent(req)?;
    if let Some(original) = original_creation(core, &req.operation_id, &intent)? {
        return Ok(original);
    }
    let frozen = query_snapshot(core, &req.root, weftext_data::QueryPlan::default())?;
    with_temporary_frozen_query(core, frozen, |core, frozen| {
        if let Some(basis) = &req.form_basis {
            let actual_ref = frozen.versions[0]["roleRef"].as_str();
            let actual_digest =
                weftext_engine::source_hash(&serde_json::to_string(&frozen.schema)?);
            if actual_ref != Some(basis.schema_ref.as_str())
                || actual_digest != basis.semantic_digest
            {
                return Err(CoreError::new(
                    "captureSchemaChanged",
                    weftext_messages::UiMessage::error_api_data_api_the_original_capture_schema_identity_or_shape_changed_raw_input_is(),
                ));
            }
        }
        capture_from_frozen_inner(core, req, frozen, &intent)
    })
}
pub(crate) fn capture_frozen_wire(
    core: &mut Core,
    wire: &str,
) -> Result<CreationCommit, CoreError> {
    let request: CaptureRequest = serde_json::from_str(wire).map_err(|error| {
        CoreError::new(
            "captureRequestInvalid",
            UiMessage::error_api_data_frozen_request_invalid(),
        )
        .caused_by(CoreError::from(error))
    })?;
    if (!request.types.is_empty() || !request.fields.is_empty()) && request.form_basis.is_none() {
        return Err(CoreError::new(
            "captureSchemaRequired",
            weftext_messages::UiMessage::error_api_data_api_typed_capture_requires_its_original_schema_identity_and_semantic_digest(),
        ));
    }
    capture_document(core, &request)
}

fn capture_from_frozen_inner(
    core: &mut Core,
    req: &CaptureRequest,
    frozen: &FrozenQuery,
    intent: &Value,
) -> Result<CreationCommit, CoreError> {
    let source = capture_template(req, &frozen.schema)
        .map_err(|error| CoreError::from_data("invalidData", error))?;
    let request = CoreOperationRequest {
        operation_id: req.operation_id.clone(),
        root: frozen.root.clone(),
        intent: OperationIntent::NewNode {
            parent: req.parent.clone(),
            name: if req.name.is_empty() {
                format!("碎片-{}", req.operation_id)
            } else {
                req.name.clone()
            },
            source: Some(source),
        },
    };
    let preview = {
        let proposed = weftext_data::capture_source(
            "00000000-0000-4000-8000-000000000000",
            &req.types,
            &req.fields,
            &capture_body(req).map_err(|error| CoreError::from_data("invalidData", error))?,
            &frozen.schema,
        )
        .map_err(|error| CoreError::from_data("invalidData", error))?;
        let targets = relation_targets(&frozen.input.notes, &frozen.schema);
        let workspace = frozen.versions[0]["roleRef"]
            .as_str()
            .and_then(|reference| reference.strip_prefix("weftext-config://"))
            .and_then(|reference| reference.strip_suffix("/schema"))
            .ok_or_else(|| {
                CoreError::new(
                    "unadmittedLibrary",
                    UiMessage::error_api_data_relation_owner_workspace_missing(),
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
            return Err(CoreError::from_product(
                "invalidRelation",
                None,
                UiMessage::error_api_data_proposal_validation_failed().into(),
                check
                    .diagnostics
                    .into_iter()
                    .chain(check.unavailable)
                    .collect(),
            ));
        }
        let context = weftext_data::ValidationContext {
            owner: None,
            resources: Vec::new(),
            resources_complete: true,
        };
        let note_validation = weftext_data::validate_note(&proposed, &frozen.schema, &context);
        if !note_validation.is_valid() {
            return Err(CoreError::from_product(
                "invalidData",
                None,
                UiMessage::error_api_data_proposal_validation_failed().into(),
                note_validation
                    .diagnostics
                    .into_iter()
                    .chain(note_validation.unavailable)
                    .collect(),
            ));
        }
        let projection = weftext_data::project(&proposed, &frozen.schema);
        if let Some(period) = projection
            .fields
            .iter()
            .find(|field| field.field_id == "calendar/period")
            .and_then(|field| field.cell.native())
        {
            let (plan, policy) = period_plan(frozen, period)?;
            let matched = weftext_data::execute_query(&frozen.input, &plan, &frozen.schema)
                .map_err(|error| CoreError::from_data("invalidPeriod", error))?;
            if let Some(directory) = &plan.scope.directory
                && !Path::new(&req.parent).starts_with(directory)
            {
                return Err(CoreError::new(
                    "periodScope",
                    UiMessage::error_api_data_period_creation_outside_scope(),
                ));
            }
            if policy.multiplicity == "unique" {
                if matched.coverage != "complete" {
                    return Err(CoreError::new(
                        "coverageIncomplete",
                        UiMessage::error_api_data_period_complete_scope_required(),
                    ));
                }
                if !matched.rows.is_empty() {
                    return Err(CoreError::new(
                        "periodConflict",
                        UiMessage::error_api_data_period_already_exists(),
                    ));
                }
            }
        }
        core.prepare_admitted_feature_operation(request, &frozen.input.cut_id, intent)?
    };
    let receipt = core.execute_operation(&req.operation_id, &preview.plan_id)?;
    Ok((preview, receipt))
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
                UiMessage::error_api_data_period_policy_missing(),
            )
        })?;
    let calendar = schema
        .providers
        .iter()
        .find(|provider| provider.id == policy.calendar)
        .ok_or_else(|| {
            CoreError::new(
                "periodProviderUnavailable",
                UiMessage::error_api_data_calendar_provider_unavailable(),
            )
        })?;
    let zone = schema
        .providers
        .iter()
        .find(|provider| provider.id == policy.zone)
        .ok_or_else(|| {
            CoreError::new(
                "periodProviderUnavailable",
                UiMessage::error_api_data_zone_provider_unavailable(),
            )
        })?;
    let provider_text = |data: &Value, field: &str| {
        data[field].as_str().map(str::to_owned).ok_or_else(|| {
            CoreError::new(
                "periodProviderUnavailable",
                UiMessage::error_api_data_period_provider_identity_missing(),
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
        .map_err(|error| CoreError::from_data("invalidPeriod", error))?;
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
            CoreError::new(
                "invalidPeriod",
                UiMessage::error_api_data_period_series_key_not_static(),
            )
        })?;
    let policy = frozen
        .schema
        .series_policies
        .iter()
        .find(|policy| policy.key == series_key)
        .ok_or_else(|| {
            CoreError::new(
                "periodPolicyUnavailable",
                UiMessage::error_api_data_series_policy_unavailable(),
            )
        })?
        .clone();
    if !matches!(policy.multiplicity.as_str(), "unique" | "many") {
        return Err(CoreError::new(
            "invalidPeriodPolicy",
            UiMessage::error_api_data_period_multiplicity_unknown(),
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
                    UiMessage::error_api_data_period_subtree_unavailable(),
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
                        UiMessage::error_api_data_period_subtree_outside_cut(),
                    )
                })?;
            scope.directory = Some(
                Path::new(&node.path)
                    .parent()
                    .ok_or_else(|| {
                        CoreError::new(
                            "periodScopeUnavailable",
                            UiMessage::error_api_data_subtree_parent_missing(),
                        )
                    })?
                    .to_string_lossy()
                    .to_string(),
            );
        }
        _ => {
            return Err(CoreError::new(
                "periodScopeUnavailable",
                UiMessage::error_api_data_period_policy_scope_unsupported(),
            ));
        }
    }
    let field = frozen.schema.field("calendar/period").ok_or_else(|| {
        CoreError::new(
            "periodUnavailable",
            UiMessage::error_api_data_period_field_missing(),
        )
    })?;
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
                        UiMessage::error_api_data_static_period_identity_required(),
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
    RequestJson(req): RequestJson<PeriodRequest>,
) -> ApiResult {
    let output = core_call(&state, move |core| {
        let frozen = query_snapshot(core,&req.root,weftext_data::QueryPlan::default())?;
        with_temporary_frozen_query(core, frozen, |_core, frozen| {
        let value = period_value(&frozen.schema,&req.series_key,&req.kind,&req.key)?;
        let period = weftext_data::period_step(&value,req.steps,&frozen.schema)
            .map_err(|error| CoreError::from_data("invalidPeriod", error))?;
        let bounds = weftext_data::period_bounds(&period,&frozen.schema)
            .map_err(|error| CoreError::from_data("invalidPeriod", error))?;
        let (plan,policy) = period_plan(frozen,&period)?;
        let result = weftext_data::execute_query(&frozen.input,&plan,&frozen.schema)
            .map_err(|error| CoreError::from_data("invalidQuery", error))?;
        let diagnostics = result.diagnostics.into_iter().map(weftext_core::CoreDescription::from).chain(std::mem::take(&mut frozen.diagnostics)).collect::<Vec<_>>();
        let can_create = result.coverage == "complete" && (policy.multiplicity == "many" || result.rows.is_empty());
        Ok(json!({"period":period,"bounds":bounds,"policy":policy,"matches":result.rows,"coverage":result.coverage,
            "canCreate":can_create,"cutId":result.cut_id,"diagnostics":diagnostics,"snapshotMode":"admittedFrozen",
            "snapshotLabel":UiMessage::error_api_data_admitted_snapshot_label(),"rawScope":{"complete":frozen.raw_complete,"pendingCount":frozen.raw_pending}}))
        })
    }).await?;
    Ok(Json(output))
}

async fn create_period(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<PeriodCreateRequest>,
) -> ApiResult {
    let (preview, receipt) = core_call(&state, move |core| {
        let intent = json!({"kind":"period", "request":serde_json::to_value(&req)?});
        if let Some(original) = original_creation(core, &req.operation_id, &intent)? {
            return Ok(original);
        }
        let frozen = query_snapshot(core, &req.root, weftext_data::QueryPlan::default())?;
        with_temporary_frozen_query(core, frozen, |core, frozen| {
            let period = period_value(&frozen.schema, &req.series_key, &req.kind, &req.key)?;
            let capture = CaptureRequest {
                root: req.root,
                parent: req.parent,
                name: req.name,
                types: vec!["period-note".into()],
                title: req.title,
                fields: BTreeMap::from([("calendar/period".into(), period)]),
                body: req.body,
                body_mode: weftext_core::CaptureBodyMode::Markup,
                form_basis: None,
                operation_id: req.operation_id,
            };
            capture_from_frozen_inner(core, &capture, frozen, &intent)
        })
    })
    .await?;
    crate::knowledge_api::operation_output(&state, preview, receipt, None)
        .await
        .map(Json)
}

pub(crate) fn resolve_entity(
    inventory: &weftext_core::Inventory,
    entity_key: &str,
) -> Result<weftext_core::InventoryEntry, CoreError> {
    if !inventory.complete {
        return Err(CoreError::new(
            "coverageIncomplete",
            UiMessage::error_api_data_unique_live_identity_unproved(),
        ));
    }
    let workspace = inventory.workspace_id.as_ref().ok_or_else(|| {
        CoreError::new(
            "unadmittedLibrary",
            UiMessage::error_api_data_stable_node_library_required(),
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
            UiMessage::error_api_data_stable_node_unavailable(),
        )
    })?;
    if matches.next().is_some() {
        return Err(CoreError::new(
            "identityConflict",
            UiMessage::error_api_data_stable_node_ambiguous(),
        ));
    }
    Ok(entry.clone())
}

async fn resolve(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<ResolveRequest>,
) -> ApiResult {
    if req
        .lifecycle
        .as_deref()
        .is_some_and(|lifecycle| lifecycle != "live")
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "trashSnapshotRequired",
            UiMessage::error_api_data_trash_read_only_route_required(),
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

pub(crate) fn open_live_entity(
    core: &mut Core,
    root: &str,
    entity_key: &str,
) -> Result<weftext_core::OpenedDocument, CoreError> {
    let inventory = core.inventory(root)?;
    let entry = resolve_entity(&inventory, entity_key)?;
    let document = core.open(&entry.path, Some(PathBuf::from(&inventory.root)))?;
    if document.kind != "managed"
        || document.node_id != entry.node_id
        || document.workspace_id != inventory.workspace_id
        || weftext_engine::inspect_metadata(&document.source).id != entry.node_id
    {
        return Err(CoreError::new(
            "identityConflict",
            UiMessage::error_api_data_resolved_source_identity_changed(),
        ));
    }
    Ok(document)
}

async fn open_entity(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<ResolveRequest>,
) -> ApiResult {
    let document = core_call(&state, move |core| {
        if req.lifecycle.as_deref() == Some("trash") {
            let revision = req
                .revision
                .as_deref()
                .filter(|revision| !revision.is_empty())
                .ok_or_else(|| {
                    CoreError::new(
                        "trashSnapshotRequired",
                        UiMessage::error_api_data_exact_trash_row_required(),
                    )
                })?;
            let captured = core.trash_admitted_snapshot(&req.root)?;
            let temporary_cut = captured.cut_id.clone();
            let result = (|| {
                if !captured.complete {
                    return Err(CoreError::new(
                        "coverageIncomplete",
                        UiMessage::error_api_data_unique_trash_identity_unproved(),
                    ));
                }
                let workspace = captured.inventory.workspace_id.as_deref().ok_or_else(|| {
                    CoreError::new(
                        "unadmittedLibrary",
                        UiMessage::error_api_data_trash_original_library_required(),
                    )
                })?;
                let mut files = captured.files.iter().filter(|file| {
                    file.node_id.as_ref().is_some_and(|id| {
                        req.entity_key == format!("weftext-node://{workspace}/{id}")
                    })
                });
                let file = files.next().ok_or_else(|| {
                    CoreError::new(
                        "nodeUnavailable",
                        UiMessage::error_api_data_trash_node_absent(),
                    )
                })?;
                if files.next().is_some() {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_api_data_trash_node_ambiguous(),
                    ));
                }
                if let Some(base) = revision.strip_prefix("observed-source:") {
                    core.validate_snapshot(&req.root, &file.path, base)?;
                } else if file.revision.as_deref() != Some(revision) {
                    return Err(CoreError::new(
                        "staleBase",
                        UiMessage::error_api_data_trash_row_changed(),
                    ));
                }
                let expected_source = file.bytes.as_deref().ok_or_else(|| {
                    CoreError::new(
                        "sourceUnavailable",
                        UiMessage::error_api_data_trash_source_unavailable(),
                    )
                })?;
                let document =
                    core.open(&file.path, Some(PathBuf::from(&captured.inventory.root)))?;
                if document.kind != "managedTrash"
                    || document.node_id != file.node_id
                    || document.workspace_id != captured.inventory.workspace_id
                    || document.source.as_bytes() != expected_source
                    || weftext_engine::inspect_metadata(&document.source).id != file.node_id
                {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_api_data_exact_trash_source_changed(),
                    ));
                }
                if let Some(base) = revision.strip_prefix("observed-source:") {
                    core.validate_snapshot(&req.root, &file.path, base)?;
                }
                Ok(document)
            })();
            core.release_admitted_cut(&temporary_cut);
            return result;
        }
        if req
            .lifecycle
            .as_deref()
            .is_some_and(|lifecycle| lifecycle != "live")
        {
            return Err(CoreError::new(
                "invalidLifecycle",
                UiMessage::error_api_data_lifecycle_unsupported(),
            ));
        }
        open_live_entity(core, &req.root, &req.entity_key)
    })
    .await?;
    let mut document = value(document)?;
    if document["kind"] == "managedTrash" {
        document["readOnly"] = json!(true);
    }
    remember(&state, &document)?;
    Ok(Json(document))
}

async fn references(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RootRequest>,
) -> ApiResult {
    let output = core_call(&state,move|core| {
        let frozen = query_snapshot(core,&req.root,weftext_data::QueryPlan::default())?;
        with_temporary_frozen_query(core, frozen, |_core, frozen| {
        let nodes = frozen.input.notes.iter().map(|note| {
            let projection = weftext_data::project(&note.source,&frozen.schema);
            json!({"ref":note.entity_key,"path":note.path,"displayLabel":Path::new(&note.path).file_stem().unwrap_or_default().to_string_lossy(),
                "labelBasis":"filenameFallback","types":projection.types,"resolvedTypes":projection.resolved_types,"revision":note.revision})
        }).collect::<Vec<_>>();
        Ok(json!({"nodes":nodes,"complete":frozen.input.coverage == "complete","diagnostics":frozen.diagnostics,
            "snapshotLabel":UiMessage::error_api_data_admitted_snapshot_label(),"tagsEndpoint":"/api/navigation/tags"}))
        })
    }).await?;
    Ok(Json(output))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryEngineDiagnostics {
    entity_key: String,
    diagnostics: Vec<weftext_engine::Diagnostic>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryProductError {
    entity_key: String,
    error: ApiError,
}

struct FrozenQuery {
    tags: BTreeMap<String, weftext_core::SnapshotFile>,
    input: weftext_data::FrozenQueryInput,
    schema: Schema,
    plan: weftext_data::QueryPlan,
    versions: Vec<Value>,
    diagnostics: Vec<weftext_core::CoreDescription>,
    engine_diagnostics: Vec<QueryEngineDiagnostics>,
    product_errors: Vec<QueryProductError>,
    raw_complete: bool,
    raw_pending: usize,
    membership_ids: Vec<String>,
    root: String,
    scope_cuts: BTreeMap<String, String>,
    schema_source: SchemaSource,
    scope_directories: BTreeSet<String>,
    scope_bindings: BTreeMap<String, String>,
    render_files: Option<BTreeMap<String, Vec<u8>>>,
    render_style: Option<weftext_engine::StyleModule>,
}

fn frozen_query_cut_ids(frozen: &FrozenQuery) -> Vec<String> {
    frozen.scope_cuts.values().cloned().collect()
}

fn release_query_cuts(core: &mut Core, cuts: &[String]) {
    for cut in cuts {
        core.release_admitted_cut(cut);
    }
}

/// Only callers that publish no reusable read basis use this wrapper. A
/// successful Query result retains its original cuts for observed-source rows;
/// strong creation keeps its cut until the operation owns the frozen plan.
fn with_temporary_frozen_query<T>(
    core: &mut Core,
    mut frozen: FrozenQuery,
    operation: impl FnOnce(&mut Core, &mut FrozenQuery) -> Result<T, CoreError>,
) -> Result<T, CoreError> {
    let result = operation(core, &mut frozen);
    release_query_cuts(core, &frozen_query_cut_ids(&frozen));
    result
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
            .map_err(|error| CoreError::from_data("invalidQuery", error))?
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
                    UiMessage::error_api_data_stable_scope_unavailable(),
                ));
            }
            let path = target.path.ok_or_else(|| {
                CoreError::new(
                    "queryScopeUnavailable",
                    UiMessage::error_api_data_stable_scope_path_missing(),
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
    let unpublished_cut = captured.cut_id.clone();
    let result = (|| {
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
                    UiMessage::error_api_data_stable_scope_changed(),
                ));
            }
            plan.scope.directory = Some(expected_path.clone());
        }
        let workspace = inventory.workspace_id.as_ref().ok_or_else(|| {
            CoreError::new(
                "unadmittedLibrary",
                UiMessage::error_api_data_structured_query_library_required(),
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
                    UiMessage::error_api_data_query_directory_traversal(),
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
                        (entry.kind == "directory"
                            && Path::new(&entry.path) == Path::new(&selected))
                            || (entry.kind == "node"
                                && entry.identity_status == "admitted"
                                && Path::new(&entry.path).parent() == Some(Path::new(&selected)))
                    })
                    .map(|_| selected.clone())
                    .ok_or_else(|| {
                        CoreError::new(
                            "queryScopeUnavailable",
                            UiMessage::error_api_data_query_directory_unavailable(),
                        )
                    })?
            };
            plan.scope.directory = Some(target);
        }
        let (config, schema) = match baseline {
            Some(baseline) => baseline,
            None => {
                let schema_file = captured.schema.ok_or_else(|| {
                    CoreError::new(
                        "schemaUnavailable",
                        UiMessage::error_api_data_frozen_schema_role_missing(),
                    )
                })?;
                let (schema, _) =
                    knowledge_schema_from_snapshot(&schema_file).map_err(|mut error| {
                        error.code = "invalidSchema".into();
                        error
                    })?;
                let source = schema_file
                    .bytes
                    .as_ref()
                    .map(|bytes| String::from_utf8(bytes.clone()))
                    .transpose()
                    .map_err(|_| {
                        CoreError::new(
                            "invalidSchema",
                            UiMessage::error_api_data_frozen_schema_encoding(),
                        )
                    })?
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
        diagnostics.push(UiMessage::error_api_data_frozen_admitted_scope().into());
        if inventory
            .entries
            .iter()
            .any(|entry| entry.kind == "node" && entry.identity_status != "admitted")
        {
            diagnostics.push(UiMessage::error_api_data_query_identity_unavailable().into());
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
                    UiMessage::error_api_data_unexpected_snapshot_source(),
                ));
            };
            if file.node_id != entry.node_id {
                complete = false;
                diagnostics.push(UiMessage::error_api_data_snapshot_identity_changed().into());
                continue;
            }
            let Some(bytes) = file.bytes else {
                complete = false;
                diagnostics.push(UiMessage::error_api_data_snapshot_source_unavailable().into());
                continue;
            };
            let source = match String::from_utf8(bytes) {
                Ok(source) => source,
                Err(_) => {
                    complete = false;
                    diagnostics.push(UiMessage::error_api_data_snapshot_source_encoding().into());
                    continue;
                }
            };
            let expected = entry.node_id.as_deref().unwrap();
            let projection = weftext_data::project(&source, &schema);
            if projection.id.as_deref() != Some(expected) {
                complete = false;
                diagnostics.push(UiMessage::error_api_data_snapshot_identity_unqualified().into());
                continue;
            }
            let entity_key = format!("weftext-node://{workspace}/{expected}");
            if entry.source.as_ref() != Some(&source) {
                diagnostics.push(
                    UiMessage::error_api_data_observed_source_changed(
                        weftext_messages::params::ErrorApiDataObservedSourceChanged {
                            entity_key: entity_key.clone(),
                        },
                    )
                    .into(),
                );
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
                authorized_fields: None,
                products: None,
            });
        }
        Ok(FrozenQuery {
            tags: captured
                .tags
                .into_iter()
                .map(|tags| (cut_id.clone(), tags))
                .collect(),
            input: weftext_data::FrozenQueryInput {
                cut_id: cut_id.clone(),
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
            engine_diagnostics: Vec::new(),
            product_errors: Vec::new(),
            raw_complete,
            raw_pending,
            membership_ids,
            root: inventory.root,
            scope_cuts: BTreeMap::from([(lifecycle.into(), cut_id)]),
            schema_source: config,
            scope_directories,
            scope_bindings: stable_scope.into_iter().collect(),
            render_files,
            render_style,
        })
    })();
    if result.is_err() {
        core.release_admitted_cut(&unpublished_cut);
    }
    result
}

async fn query(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<QueryRequest>,
) -> ApiResult {
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
    let result = freeze_query_definitions_with_draft_inner(core, frozen, parameters, draft);
    if result.is_err() {
        release_query_cuts(core, &frozen_query_cut_ids(frozen));
    }
    result
}

fn freeze_query_definitions_with_draft_inner(
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
                .map_err(|error| CoreError::from_data("invalidQuery", error))?;
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
            if !frozen
                .input
                .notes
                .iter()
                .any(|note| note.entity_key == dependency.owner)
            {
                let target = core.resolve_navigation_target(&frozen.root, &dependency.owner)?;
                if target.status == "managed"
                    && target.kind == "node"
                    && !frozen.scope_cuts.contains_key("live")
                {
                    extend_query_lifecycle(core, frozen, "live".into())?;
                }
            }
            let note = frozen
                .input
                .notes
                .iter()
                .find(|note| note.entity_key == dependency.owner)
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        UiMessage::error_api_data_called_owner_unavailable(),
                    )
                })?;
            let owner_id = weftext_data::project(&note.source, &frozen.schema)
                .id
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        UiMessage::error_api_data_called_owner_identity_missing(),
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
                    UiMessage::error_api_data_called_definition_unadmitted(),
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
                        UiMessage::error_api_data_pinned_definition_encoding(),
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
            .map_err(|error| CoreError::from_data("definitionUnavailable", error))?;
            let role = frozen
                .versions
                .first()
                .and_then(|version| version["roleRef"].as_str());
            if role != Some(definition.binding.schema.as_str()) {
                return Err(CoreError::new(
                    "schemaUnavailable",
                    UiMessage::error_api_data_called_schema_authority_mismatch(),
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
                    CoreError::new(
                        "schemaUnavailable",
                        UiMessage::error_api_data_called_schema_encoding(),
                    )
                })?;
                let projection = Schema::from_author_json(&source)
                    .map_err(|error| CoreError::from_data("invalidSchema", error))?;
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
                        UiMessage::error_api_data_selected_definition_unavailable(),
                    )
                })?;
            frozen.versions.push(json!({"role":"definition","owner":dependency.owner,"key":definition.key,"selection":dependency.selection,
                "definitionRevision":selected_source.revision,"sourceHash":weftext_engine::source_hash(&selected_source.source),
                "draftPreview":selected_source.revision.starts_with("draft-source:")}));
        }
        if !progress {
            qualify_query_context(core, frozen, &mut context)?;
            let demands =
                weftext_data::query_read_requirements(&frozen.plan, &frozen.schema, &context)
                    .map_err(|error| CoreError::from_data("invalidQuery", error))?;
            if demands.tag_structure {
                if !frozen
                    .scope_cuts
                    .values()
                    .all(|cut| frozen.tags.contains_key(cut))
                {
                    return Err(CoreError::new(
                        "tagsUnavailable",
                        UiMessage::error_api_data_lifecycle_tags_capture_required(),
                    ));
                }
                let tags = frozen.tags.get(&frozen.input.cut_id).ok_or_else(|| {
                    CoreError::new(
                        "tagsUnavailable",
                        UiMessage::error_api_data_frozen_tags_missing(),
                    )
                })?;
                for (cut, selected) in &frozen.tags {
                    if selected.bytes != tags.bytes || selected.revision != tags.revision {
                        return Err(CoreError::new(
                            "tagsUnavailable",
                            UiMessage::error_api_data_lifecycle_tags_mismatch(),
                        ));
                    }
                    core.bind_admitted_tag_structure(&frozen.root, cut, selected)?;
                }
                let document = core.qualified_tag_document_from_snapshot(&frozen.root, tags)?;
                let workspace = frozen
                    .schema_source
                    .role_ref
                    .as_deref()
                    .and_then(|role| role.strip_prefix("weftext-config://"))
                    .and_then(|role| role.strip_suffix("/schema"))
                    .ok_or_else(|| {
                        CoreError::new(
                            "tagsUnavailable",
                            UiMessage::error_api_data_frozen_tag_workspace_missing(),
                        )
                    })?
                    .to_owned();
                context.tag_structure = Some(
                    weftext_data::FrozenTagStructure::new(workspace, document)
                        .map_err(|error| CoreError::from_data("tagsUnavailable", error))?,
                );
                frozen.versions.push(json!({"role":"tags","path":tags.path,"revision":tags.revision,"baseId":tags.base_id}));
            }
            frozen.input.advanced = Some(context);
            return Ok(());
        }
    }
    Err(CoreError::new(
        "queryBudget",
        UiMessage::error_api_data_definition_closure_depth_budget(),
    ))
}

fn qualify_query_context(
    core: &mut Core,
    frozen: &mut FrozenQuery,
    context: &mut weftext_data::FrozenAdvancedInput,
) -> Result<(), CoreError> {
    let lifecycles = weftext_data::required_query_lifecycles(&frozen.plan, &context.definitions)
        .map_err(|error| CoreError::from_data("invalidQuery", error))?;
    for lifecycle in lifecycles {
        if !frozen.scope_cuts.contains_key(&lifecycle) {
            extend_query_lifecycle(core, frozen, lifecycle)?;
        }
    }
    let scopes = weftext_data::required_query_scopes(&frozen.plan, &context.definitions)
        .map_err(|error| CoreError::from_data("invalidQuery", error))?;
    for scope in scopes {
        let selected = if scope.starts_with("weftext-directory://")
            || scope.starts_with("weftext-node://")
        {
            let target = core.resolve_navigation_target(&frozen.root, &scope)?;
            if target.status != "managed" || !matches!(target.kind.as_str(), "directory" | "node") {
                return Err(CoreError::new(
                    "queryScopeUnavailable",
                    UiMessage::error_api_data_nested_scope_unavailable(),
                ));
            }
            target.path.ok_or_else(|| {
                CoreError::new(
                    "queryScopeUnavailable",
                    UiMessage::error_api_data_nested_scope_path_missing(),
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
                    UiMessage::error_api_data_query_scope_traversal(),
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
                    UiMessage::error_api_data_nested_scope_outside_inventory(),
                )
            })?;
        context.scope_bindings.insert(scope, selected.clone());
    }
    Ok(())
}

fn extend_query_lifecycle(
    core: &mut Core,
    frozen: &mut FrozenQuery,
    lifecycle: String,
) -> Result<(), CoreError> {
    // Keep each original Core read cut. The primary cut is not replaced by a
    // fabricated mixed writable basis; Query/save-definition only consume the
    // exact readonly rows and original per-lifecycle evidence below.
    let plan = weftext_data::QueryPlan {
        scope: weftext_data::QueryScope {
            lifecycle: lifecycle.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    let extra = query_snapshot_impl(
        core,
        &frozen.root,
        plan,
        Some((frozen.schema_source.clone(), frozen.schema.clone())),
        false,
    )?;
    let unpublished_cuts = frozen_query_cut_ids(&extra);
    let result = (|| {
        if extra.root != frozen.root || extra.input.schema_revision != frozen.input.schema_revision
        {
            return Err(CoreError::new(
                "queryScopeUnavailable",
                UiMessage::error_api_data_lifecycle_baseline_mismatch(),
            ));
        }
        let count = frozen
            .input
            .notes
            .len()
            .saturating_add(extra.input.notes.len());
        let bytes = frozen
            .input
            .notes
            .iter()
            .chain(&extra.input.notes)
            .map(|note| note.source.len())
            .sum::<usize>();
        if count > 5_000 || bytes > 64 * 1024 * 1024 {
            return Err(CoreError::new(
                "queryBudget",
                UiMessage::error_api_data_combined_lifecycle_budget(),
            ));
        }
        let owners = frozen
            .input
            .notes
            .iter()
            .map(|note| note.entity_key.as_str())
            .collect::<BTreeSet<_>>();
        let paths = frozen
            .input
            .notes
            .iter()
            .map(|note| note.path.as_str())
            .collect::<BTreeSet<_>>();
        for note in &extra.input.notes {
            if owners.contains(note.entity_key.as_str()) || paths.contains(note.path.as_str()) {
                return Err(CoreError::new(
                    "queryScopeUnavailable",
                    UiMessage::error_api_data_competing_lifecycle_node(),
                ));
            }
            if let Some(files) = &frozen.render_files {
                let relative = Path::new(&note.path)
                    .strip_prefix(&frozen.root)
                    .map_err(|_| {
                        CoreError::new(
                            "outsideRoot",
                            UiMessage::error_api_data_lifecycle_source_outside_library(),
                        )
                    })?
                    .to_string_lossy()
                    .replace('\\', "/");
                if files.get(&relative).map(Vec::as_slice) != Some(note.source.as_bytes()) {
                    return Err(CoreError::new(
                        "scopeChanged",
                        UiMessage::error_api_data_lifecycle_source_world_mismatch(),
                    ));
                }
            }
        }
        if extra.input.coverage != "complete" {
            frozen.input.coverage = "partial".into();
        }
        frozen.tags.extend(extra.tags);
        frozen.scope_cuts.extend(extra.scope_cuts);
        frozen.input.notes.extend(extra.input.notes);
        frozen.versions.extend(extra.versions.into_iter().skip(1));
        frozen.diagnostics.extend(extra.diagnostics);
        frozen.engine_diagnostics.extend(extra.engine_diagnostics);
        frozen.product_errors.extend(extra.product_errors);
        frozen.scope_directories.extend(extra.scope_directories);
        frozen.raw_complete &= extra.raw_complete;
        frozen.raw_pending = frozen.raw_pending.saturating_add(extra.raw_pending);
        frozen.membership_ids.extend(extra.membership_ids);
        frozen.membership_ids.sort();
        frozen.membership_ids.dedup();
        Ok(())
    })();
    if result.is_err() {
        release_query_cuts(core, &unpublished_cuts);
    }
    result
}

fn unavailable_products(
    identity: String,
    kind: weftext_data::DataErrorKind,
    reason: UiMessage,
) -> weftext_data::NoteProducts {
    let unavailable = || weftext_data::Cell::Unavailable {
        kind,
        reason: reason.clone().into(),
        diagnostics: Vec::new(),
    };
    weftext_data::NoteProducts {
        compile_identity: identity,
        target: "html".into(),
        title: unavailable(),
        body: unavailable(),
        headings: unavailable(),
    }
}

pub(crate) fn native_query_products(
    identity: String,
    products: weftext_engine::RenderProducts,
) -> weftext_data::NoteProducts {
    weftext_data::native_query_products(identity, products)
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
                weftext_data::DataErrorKind::NativeBudget,
                UiMessage::error_api_data_native_product_budget(),
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
                            weftext_data::DataErrorKind::Unavailable,
                            UiMessage::error_api_data_native_products_unqualified(),
                        ),
                    }
                }
                Ok(Ok(output)) => {
                    let diagnostics = serde_json::from_value::<Vec<weftext_engine::Diagnostic>>(
                        output["diagnostics"].clone(),
                    )
                    .map_err(|error| {
                        ApiError::internal(UiMessage::error_api_data_native_diagnostics_invalid())
                            .diagnostic(weftext_messages::DiagnosticOrigin::Json, error.to_string())
                    })?;
                    frozen.engine_diagnostics.push(QueryEngineDiagnostics {
                        entity_key: note.entity_key.clone(),
                        diagnostics,
                    });
                    unavailable_products(
                        identity,
                        weftext_data::DataErrorKind::Unavailable,
                        UiMessage::error_api_data_native_compile_failed(),
                    )
                }
                Ok(Err(error)) => {
                    frozen.product_errors.push(QueryProductError {
                        entity_key: note.entity_key.clone(),
                        error,
                    });
                    unavailable_products(
                        identity,
                        weftext_data::DataErrorKind::Unavailable,
                        UiMessage::error_api_data_native_worker_unavailable(),
                    )
                }
                Err(_) => unavailable_products(
                    identity,
                    weftext_data::DataErrorKind::NativeBudget,
                    UiMessage::error_api_data_native_product_deadline(),
                ),
            }
        } else {
            unavailable_products(
                identity,
                weftext_data::DataErrorKind::Unavailable,
                UiMessage::error_api_data_native_environment_unavailable(),
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
            frozen.diagnostics.push(
                UiMessage::error_api_data_native_product_coverage_partial(
                    weftext_messages::params::ErrorApiDataNativeProductCoveragePartial {
                        entity_key: note.entity_key.clone(),
                    },
                )
                .into(),
            );
        }
        note.products = Some(products);
    }
    Ok(demands)
}

fn publish_query_diagnostics(
    output: &mut Value,
    diagnostics: Vec<weftext_core::CoreDescription>,
    engine_diagnostics: Vec<QueryEngineDiagnostics>,
    product_errors: Vec<QueryProductError>,
) -> Result<(), ApiError> {
    output["diagnostics"] = value(diagnostics)?;
    output["engineDiagnostics"] = value(engine_diagnostics)?;
    output["productErrors"] = value(product_errors)?;
    Ok(())
}

async fn evaluate_frozen(state: &AppState, frozen: FrozenQuery) -> Result<Value, ApiError> {
    let cuts = frozen_query_cut_ids(&frozen);
    let result = evaluate_frozen_inner(state, frozen).await;
    if result.is_err() {
        core_call(state, move |core| {
            release_query_cuts(core, &cuts);
            Ok(())
        })
        .await?;
    }
    result
}

async fn evaluate_frozen_inner(
    state: &AppState,
    mut frozen: FrozenQuery,
) -> Result<Value, ApiError> {
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
    let engine_diagnostics = frozen.engine_diagnostics;
    let product_errors = frozen.product_errors;
    let raw_complete = frozen.raw_complete;
    let raw_pending = frozen.raw_pending;
    let membership_ids = frozen.membership_ids;
    let scope_cuts = frozen.scope_cuts;
    let mut result = crate::work::spawn_blocking(move || {
        weftext_data::execute_query(&frozen.input, &frozen.plan, &frozen.schema)
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_data_query_evaluation_incomplete()))?
    .map_err(data_error)?;
    let diagnostics: Vec<weftext_core::CoreDescription> = result
        .diagnostics
        .drain(..)
        .map(weftext_core::CoreDescription::from)
        .chain(diagnostics)
        .collect();
    export_columns.retain(|name| result.terminal_schema.contains_key(name));
    for name in result.terminal_schema.keys() {
        if !export_columns.contains(name) {
            export_columns.push(name.clone());
        }
    }
    let mut output = value(result)?;
    publish_query_diagnostics(&mut output, diagnostics, engine_diagnostics, product_errors)?;
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
    output["snapshotLabel"] = json!(UiMessage::error_api_data_admitted_snapshot_label());
    output["externalPending"] = json!(!raw_complete || raw_pending > 0);
    output["rawScope"] = json!({"complete":raw_complete,"pendingCount":raw_pending});
    output["membershipIds"] = json!(membership_ids);
    output["scopeCuts"] = json!(scope_cuts);
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
            UiMessage::error_api_data_definition_owner_unavailable(),
        )
    })?;
    if file.node_id != entry.node_id {
        return Err(CoreError::new(
            "identityConflict",
            UiMessage::error_api_data_definition_owner_identity_changed(),
        ));
    }
    let source = file.bytes.as_ref().ok_or_else(|| {
        CoreError::new(
            "sourceUnavailable",
            UiMessage::error_api_data_definition_owner_unavailable(),
        )
    })?;
    let source = String::from_utf8(source.clone()).map_err(|_| {
        CoreError::new(
            "sourceUnavailable",
            UiMessage::error_api_data_definition_owner_encoding(),
        )
    })?;
    if weftext_engine::inspect_metadata(&source).id != entry.node_id {
        return Err(CoreError::new(
            "identityConflict",
            UiMessage::error_api_data_definition_owner_unadmitted(),
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
    RequestJson(req): RequestJson<DefinitionsRequest>,
) -> ApiResult {
    let output = core_call(&state, move |core| {
        let current = owner_source(core, &req.root, &req.owner)?;
        let candidates = weftext_data::saved_queries(&current.source)
            .map_err(|error| CoreError::from_data("invalidQueryDefinition", error))?;
        let mut admitted = Vec::new(); let mut unavailable = Vec::new();
        for candidate in candidates {
            if core.is_definition_key_admitted(&req.root, current.entry.node_id.as_deref().unwrap(), &candidate.key)? {
                admitted.push(candidate);
            } else { unavailable.push(json!({"key":candidate.key,"label":candidate.label,"reason":weftext_messages::ErrorDescription::new(UiMessage::error_api_data_definition_key_unadmitted_reason())})); }
        }
        Ok(json!({"definitions":admitted,"ownerRevision":current.file.revision,"owner":req.owner,
            "ownerBaseId":current.file.base_id,"unavailable":unavailable,"snapshotMode":"perFileObserved"}))
    }).await?;
    Ok(Json(output))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionPendingRequest {
    session_id: String,
    source: String,
    serial: u64,
}

async fn pending_definition(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<DefinitionPendingRequest>,
) -> ApiResult {
    writing_api::observe_snapshot(&state, &req.session_id, &req.source, req.serial, None)?;
    let loc = location(&state, &req.session_id)?;
    let result = core_call(&state, move |core| {
        // Query controls may be unavailable while the author still saves the
        // original native source. This lookup must not add a new Save gate.
        let Ok(definitions) = weftext_data::saved_queries(&req.source) else {
            return Ok(json!({"operationId":null,"queryState":"unparsed"}));
        };
        let owner = core.inventory(&loc.root)?.entries.into_iter().find(|entry| entry.path == loc.path.to_string_lossy() && entry.identity_status == "admitted").and_then(|entry| entry.node_id);
        let Some(owner) = owner else { return Ok(json!({"operationId":null,"queryState":"unadmitted"})); };
        let mut pending = Vec::new();
        let mut unadmitted = false;
        for definition in definitions {
            if !core.is_definition_key_admitted(&loc.root, &owner, &definition.key)? {
                if let Some(operation) = core.pending_definition_operation(&loc.root, &loc.path, &definition.key)? { pending.push(operation); }
                else { unadmitted = true; }
            }
        }
        if pending.len() > 1 {
            return Ok(json!({"operationId":null,"queryState":"multiplePending"}));
        }
        Ok(json!({"operationId":pending.first(),"queryState":if unadmitted {"unadmitted"} else if pending.is_empty() {"admitted"} else {"reserved"}}))
    })
    .await?;
    Ok(Json(result))
}

async fn draft_definition(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<DefinitionSaveRequest>,
) -> ApiResult {
    writing_api::observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let loc = location(&state, &req.session_id)?;
    deny_trash_writer(&loc.path)?;
    let before = req.source.clone();
    let serial = req.serial;
    let check = state.clone();
    let (patch, key) = core_call(&state, move |core| {
        if req.key.is_some() {
            return Err(CoreError::new(
                "invalidQueryDefinition",
                UiMessage::error_api_data_new_draft_definition_required(),
            ));
        }
        let inventory = core.inventory(&loc.root)?;
        let entry = inventory
            .entries
            .iter()
            .find(|entry| {
                entry.path == loc.path.to_string_lossy() && entry.identity_status == "admitted"
            })
            .ok_or_else(|| {
                CoreError::new(
                    "identityConflict",
                    UiMessage::error_api_data_admitted_owner_required(),
                )
            })?;
        if weftext_engine::inspect_metadata(&req.source).id != entry.node_id {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_api_data_draft_owner_changed(),
            ));
        }
        for definition in weftext_data::saved_queries(&req.source)
            .map_err(|e| CoreError::from_data("invalidQueryDefinition", e))?
        {
            if !core.is_definition_key_admitted(
                &loc.root,
                entry.node_id.as_deref().unwrap(),
                &definition.key,
            )? {
                return Err(CoreError::new(
                    "definitionUnavailable",
                    UiMessage::error_api_data_existing_definition_must_be_saved(),
                ));
            }
        }
        let (config, schema) = resolved_schema(core, &loc.root)?;
        let binding = weftext_data::SavedQueryBinding {
            schema: config.role_ref.clone().unwrap(),
            revision: config.revision.clone().ok_or_else(|| {
                CoreError::new(
                    "schemaRevisionRequired",
                    UiMessage::error_api_data_explicit_schema_before_query(),
                )
            })?,
            language: "weftext.query.cel.v1".into(),
        };
        let current = core
            .read_snapshot(&loc.root, &[loc.path.to_string_lossy().to_string()])?
            .files
            .remove(0);
        let key =
            core.reserve_definition_key(&loc.root, &loc.path, &current.base_id, &req.operation_id)?;
        let context = query_snapshot(core, &loc.root.to_string_lossy(), req.plan.clone())?;
        let context = with_temporary_frozen_query(core, context, |core, context| {
            freeze_query_definitions(core, context, BTreeMap::new())?;
            Ok(context.input.advanced.take().unwrap_or_default())
        })?;
        let patch = weftext_data::upsert_saved_query_with_context(
            &req.source,
            None,
            &key,
            &req.label,
            &binding,
            &req.plan,
            &schema,
            &context,
        )
        .map_err(|e| CoreError::from_data("invalidQueryDefinition", e))?;
        writing_api::check_snapshot(
            &check,
            &req.session_id,
            &req.source,
            req.serial,
            req.source_hash.as_deref(),
        )
        .map_err(|e| e.into_core(e.code()))?;
        Ok((patch, key))
    })
    .await?;
    let Json(mut output) = patch_response(patch, serial, &before)?;
    output["draftOnly"] = json!(true);
    output["key"] = json!(key);
    Ok(Json(output))
}

async fn save_definition(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<DefinitionSaveRequest>,
) -> ApiResult {
    let replay_request = req.clone();
    let replay = core_call(&state, move |core| {
        replay_saved_definition(core, &replay_request)
    })
    .await;
    match replay {
        Ok(Some(replayed)) => {
            return definition_write_response(&state, replayed, req.serial, &req.source);
        }
        Ok(None) => {}
        Err(error) => {
            let id = req.operation_id.clone();
            let fact = core_call(&state, move |core| core.local_operation_status(&id)).await?;
            if fact["status"] == "committed" {
                if error.code == "operationConflict" {
                    return Err(original_operation_error(error, fact));
                }
                let mut output = fact;
                output["documentError"] = value(error)?;
                return Ok(Json(output));
            }
            return Err(error);
        }
    }
    let loc = location(&state, &req.session_id)?;
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
    let write = core_call(&state, move |core| {
        let inventory = core.inventory(&loc.root)?;
        let entry = inventory
            .entries
            .iter()
            .find(|entry| {
                entry.path == loc.path.to_string_lossy() && entry.identity_status == "admitted"
            })
            .ok_or_else(|| {
                CoreError::new(
                    "identityConflict",
                    UiMessage::error_api_data_current_definition_owner_required(),
                )
            })?;
        let node_id = entry.node_id.as_deref().unwrap().to_string();
        if weftext_engine::inspect_metadata(&req.source).id.as_deref() != Some(&node_id) {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_api_data_draft_owner_identity_required(),
            ));
        }
        let (config, resolved) = resolved_schema(core, &loc.root)?;
        let schema_revision = config.revision.clone().ok_or_else(|| {
            CoreError::new(
                "schemaRevisionRequired",
                UiMessage::error_api_data_explicit_schema_baseline_required(),
            )
        })?;
        let binding = weftext_data::SavedQueryBinding {
            schema: config.role_ref.clone().unwrap(),
            revision: schema_revision,
            language: "weftext.query.cel.v1".into(),
        };
        let mut current =
            core.read_snapshot(&loc.root, &[loc.path.to_string_lossy().to_string()])?;
        let current = current.files.pop().ok_or_else(|| {
            CoreError::new(
                "sourceUnavailable",
                UiMessage::error_api_data_owner_snapshot_unavailable(),
            )
        })?;
        let key = match &req.key {
            Some(key) if core.is_definition_key_admitted(&loc.root, &node_id, key)? => key.clone(),
            Some(_) => {
                return Err(CoreError::new(
                    "definitionUnavailable",
                    UiMessage::error_api_data_definition_key_unadmitted(),
                ));
            }
            None => core.reserve_definition_key(
                &loc.root,
                &loc.path,
                &current.base_id,
                &req.operation_id,
            )?,
        };
        let definition_context =
            query_snapshot(core, &loc.root.to_string_lossy(), req.plan.clone())?;
        let definition_context =
            with_temporary_frozen_query(core, definition_context, |core, context| {
                freeze_query_definitions(core, context, BTreeMap::new())?;
                Ok(context.input.advanced.take().unwrap_or_default())
            })?;
        let patch = weftext_data::upsert_saved_query_with_context(
            &req.source,
            req.key.as_deref(),
            &key,
            &req.label,
            &binding,
            &req.plan,
            &resolved,
            &definition_context,
        )
        .map_err(|error| CoreError::from_data("invalidQueryDefinition", error))?;
        let next_config = schema_source(core, &loc.root)?;
        if next_config.source != config.source || next_config.revision != config.revision {
            return Err(CoreError::new(
                "configConflict",
                UiMessage::error_api_data_definition_schema_changed(),
            ));
        }
        writing_api::check_snapshot(
            &final_check,
            &req.session_id,
            &req.source,
            req.serial,
            req.source_hash.as_deref(),
        )
        .map_err(|error| error.into_core(error.code()))?;
        // Projection is validated before commit. The live open is delivery.
        let definition = weftext_data::saved_queries(&patch.source)
            .map_err(|error| CoreError::from_data("invalidQueryDefinition", error))?
            .into_iter()
            .find(|definition| definition.key == key)
            .ok_or_else(|| {
                CoreError::new(
                    "definitionUnavailable",
                    UiMessage::error_api_data_definition_projection_unavailable(),
                )
            })?;
        let operation_id = req.operation_id.clone();
        let session_id = req.session_id.clone();
        let revision = req.input_revision;
        let intent = SaveRequest {
            session_id: req.session_id,
            base_id: req.base_id,
            source: patch.source.clone(),
            operation_id: req.operation_id,
        };
        let outcome = match revision {
            Some(revision) => core.save_ordered_derived(intent, req.source.clone(), revision)?,
            None => core.save(intent)?,
        };
        let document = core
            .refresh_local_saved_session(&operation_id, &session_id)
            .map(|(_, document)| document);
        Ok((document, definition, outcome, patch))
    })
    .await?;
    definition_write_response(&state, write, serial, &before_source)
}

type DefinitionWrite = (
    Result<weftext_core::OpenedDocument, CoreError>,
    weftext_data::SavedQuery,
    weftext_core::SaveOutcome,
    weftext_data::FieldPatch,
);

fn replay_saved_definition(
    core: &mut Core,
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
            UiMessage::error_api_data_original_source_basis_mismatch(),
        ));
    }
    // Original author identity is checked before any optional schema/result
    // delivery. The journal already retains the exact submitted save bytes.
    let outcome = core
        .saved_outcome_for_request_revision(
            &SaveRequest {
                session_id: req.session_id.clone(),
                base_id: req.base_id.clone(),
                source: original.source.clone(),
                operation_id: req.operation_id.clone(),
            },
            req.input_revision,
            Some(&req.source),
        )?
        .ok_or_else(|| {
            CoreError::new(
                "recoveryPending",
                UiMessage::error_api_data_original_definition_not_committed(),
            )
        })?;
    let key = req
        .key
        .as_ref()
        .or(original.definition_key.as_ref())
        .ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                UiMessage::error_api_data_original_definition_key_missing(),
            )
        })?;
    let definition = weftext_data::saved_queries(&original.source)
        .map_err(|error| CoreError::from_data("operationConflict", error))?
        .into_iter()
        .find(|definition| &definition.key == key)
        .ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                UiMessage::error_api_data_original_definition_missing(),
            )
        })?;
    let bytes = core.read_config_revision(
        original.root.as_ref().ok_or_else(|| {
            CoreError::new(
                "committedResultUnavailable",
                UiMessage::error_api_data_committed_library_unavailable(),
            )
        })?,
        ConfigRole::Schema,
        &definition.binding.revision,
    )?;
    let source = String::from_utf8(bytes).map_err(|_| {
        CoreError::new(
            "schemaUnavailable",
            UiMessage::error_api_data_original_schema_encoding(),
        )
    })?;
    let schema = Schema::from_author_json(&source)
        .map_err(|error| CoreError::from_data("schemaUnavailable", error))?;
    let patch = weftext_data::upsert_saved_query_replay(
        &req.source,
        req.key.as_deref(),
        key,
        &req.label,
        &definition.binding,
        &req.plan,
        &schema,
    )
    .map_err(|error| CoreError::from_data("operationConflict", error))?;
    if patch.source != original.source {
        return Err(CoreError::new(
            "operationConflict",
            UiMessage::error_api_data_original_committed_source_mismatch(),
        ));
    }
    let document = match core.refresh_local_saved_session(&req.operation_id, &req.session_id) {
        Err(error) if error.code == "sessionMissing" => core.reopen_local_save(&req.operation_id),
        result => result,
    }
    .map(|(_, document)| document);
    Ok(Some((document, definition, outcome, patch)))
}

fn definition_write_response(
    state: &AppState,
    write: DefinitionWrite,
    serial: u64,
    before_source: &str,
) -> ApiResult {
    let (document, definition, outcome, patch) = write;
    let mut output = value(&outcome)?;
    output["status"] = json!("committed");
    output["definition"] = value(definition)?;
    output["definitionRevision"] = json!(
        outcome
            .receipt
            .revision_id
            .as_ref()
            .unwrap_or(&outcome.receipt.base_id)
    );
    output["committedSource"] = json!(patch.source);
    let delivered = (|| -> Result<(), ApiError> {
        let document = value(document.map_err(ApiError::from)?)?;
        remember(state, &document)?;
        output["documentSourceMatchesCommit"] =
            json!(document_matches_committed_source(&document, &patch.source));
        output["document"] = document;
        let Json(patch_output) = patch_response(patch, serial, before_source)?;
        output["patches"] = patch_output["patches"].clone();
        output["serial"] = json!(serial);
        output["sourceHash"] = patch_output["sourceHash"].clone();
        Ok(())
    })();
    if let Err(error) = delivered {
        output["documentError"] = value(error)?;
    }
    Ok(Json(output))
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
        CoreError::new(
            "queryUnavailable",
            UiMessage::error_api_data_ordinary_query_permission_missing(),
        )
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
                UiMessage::error_api_data_query_owner_unadmitted(),
            )
        })?;
    let node = owner.node_id.as_deref().ok_or_else(|| {
        CoreError::new(
            "queryUnavailable",
            UiMessage::error_api_data_query_owner_identity_unavailable(),
        )
    })?;
    let owner_ref = format!("weftext-node://{workspace}/{node}");
    let current_config = core.read_config(root, ConfigRole::Schema)?;
    if current_config.role_ref != binding.schema {
        return Err(CoreError::new(
            "schemaUnavailable",
            UiMessage::error_api_data_query_schema_authority_mismatch(),
        ));
    }
    let bytes = core.read_config_revision(root, ConfigRole::Schema, &binding.revision)?;
    let baseline_source = String::from_utf8(bytes.clone()).map_err(|_| {
        CoreError::new(
            "schemaUnavailable",
            UiMessage::error_api_data_retained_query_schema_encoding(),
        )
    })?;
    let baseline = Schema::from_author_json(&baseline_source)
        .map_err(|e| CoreError::from_data("invalidSchema", e))?;
    let (_, current_schema) = resolved_schema(core, Path::new(root))?;
    let projection = weftext_data::project(source, &baseline);
    if projection.qualification != "qualified" || projection.id.as_deref() != Some(node) {
        return Err(CoreError::new(
            "queryUnavailable",
            UiMessage::error_api_data_original_draft_identity_required(),
        ));
    }
    if !weftext_data::baseline_contract_compatible(&baseline, &current_schema)
        .map_err(|e| CoreError::from_data("schemaUnavailable", e))?
    {
        return Err(CoreError::new(
            "schemaMigrationUnavailable",
            UiMessage::error_api_data_query_baseline_migration_required(),
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
        with_temporary_frozen_query(core, frozen, |_core, frozen| {
            weftext_data::validate_query_plan_with_context(
                &frozen.plan,
                &frozen.schema,
                frozen.input.advanced.as_ref().unwrap(),
            )
            .map_err(|e| CoreError::from_data("invalidQuery", e))
        })
    })
    .await
}

async fn run_definition(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<DefinitionRunRequest>,
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
                            UiMessage::error_api_data_retained_definition_encoding(),
                        )
                    })?;
                    (key, Some(revision.clone()), "pinned", source)
                }
            };
            if !core.is_definition_key_admitted(&req.root, node_id, key)? {
                return Err(CoreError::new(
                    "definitionUnavailable",
                    UiMessage::error_api_data_definition_key_admission_missing(),
                ));
            }
            let definition = weftext_data::saved_queries(&source)
                .map_err(|error| CoreError::from_data("invalidQueryDefinition", error))?
                .into_iter()
                .find(|definition| &definition.key == key)
                .ok_or_else(|| {
                    CoreError::new(
                        "definitionUnavailable",
                        UiMessage::error_api_data_definition_key_at_revision_missing(),
                    )
                })?;
            let current_config = core.read_config(&req.root, ConfigRole::Schema)?;
            if current_config.role_ref != definition.binding.schema {
                return Err(CoreError::new(
                    "schemaUnavailable",
                    UiMessage::error_api_data_saved_query_schema_authority_mismatch(),
                ));
            }
            let bytes = core.read_config_revision(
                &req.root,
                ConfigRole::Schema,
                &definition.binding.revision,
            )?;
            let baseline_source = String::from_utf8(bytes.clone()).map_err(|_| {
                CoreError::new(
                    "schemaUnavailable",
                    UiMessage::error_api_data_retained_query_schema_encoding(),
                )
            })?;
            let baseline = Schema::from_author_json(&baseline_source)
                .map_err(|error| CoreError::from_data("invalidSchema", error))?;
            let (_, current_schema) = resolved_schema(core, Path::new(&req.root))?;
            if !weftext_data::baseline_contract_compatible(&baseline, &current_schema)
                .map_err(|error| CoreError::from_data("schemaUnavailable", error))?
            {
                return Err(CoreError::new(
                    "schemaMigrationUnavailable",
                    UiMessage::error_api_data_saved_query_contracts_changed(),
                ));
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

    #[test]
    fn capture_exact_id_preflight_ignores_an_earlier_matching_native_key() {
        let mut schema = Schema::bundled().unwrap();
        schema.fields.insert(
            0,
            weftext_data::Field {
                id: "private/config".into(),
                key: "tasks/status".into(),
                shape: weftext_data::Shape::Scalar {
                    semantic: "text".into(),
                },
                nullable: false,
                default: Some(json!({"kind":"absent"})),
            },
        );
        let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
        let mut request: CaptureRequest = serde_json::from_value(json!({"root":"owned","parent":"owned","name":"Task","types":["task"],"title":null,"fields":{"tasks/status":{"kind":"text","value":"open"}},"body":"原文","formBasis":null,"operationId":uuid::Uuid::new_v4().to_string()})).unwrap();
        assert!(
            capture_template(&request, &schema)
                .unwrap()
                .contains("\"status\": \"open\"")
        );
        request.fields = BTreeMap::from([("status".into(), NativeValue::Text("open".into()))]);
        let error = capture_template(&request, &schema).unwrap_err();
        assert_eq!(
            error.description().explanation,
            UiMessage::error_capture_field_identity_required(
                weftext_messages::params::ErrorCaptureFieldIdentityRequired {
                    field: "tasks/status".into()
                }
            )
        );
        request.fields =
            BTreeMap::from([("unknown/field".into(), NativeValue::Text("open".into()))]);
        let error = capture_template(&request, &schema).unwrap_err();
        assert_eq!(
            error.description().explanation,
            UiMessage::error_api_data_capture_field_unknown(
                weftext_messages::params::ErrorApiDataCaptureFieldUnknown {
                    field: "unknown/field".into()
                }
            )
        );
    }

    #[test]
    fn data_bridge_keeps_semantic_kind_and_all_parallel_diagnostics() {
        let reason = weftext_messages::ErrorDescription::new(
            UiMessage::error_api_data_native_product_budget(),
        );
        let parallel = vec![reason.clone(); 48];
        let failure = data_error(
            weftext_data::DataError::native_budget(reason).with_diagnostics(parallel.clone()),
        );
        assert_eq!(failure.status, StatusCode::BAD_REQUEST);
        assert_eq!(failure.code, "invalid_data");
        let Some(ApiErrorDetails::Core { error }) = failure.details.as_deref() else {
            panic!("complete Core failure was lost")
        };
        let Some(weftext_core::CoreErrorDetails::Aggregate { groups }) = &error.details else {
            panic!("parallel diagnostics were lost")
        };
        let Some(weftext_core::ErrorDetailGroup::Data {
            classification,
            diagnostics,
        }) = groups.first()
        else {
            panic!("semantic data classification was lost")
        };
        assert_eq!(*classification, weftext_data::DataErrorKind::NativeBudget);
        assert_eq!(
            serde_json::to_value(diagnostics).unwrap(),
            serde_json::to_value(parallel).unwrap()
        );
        assert!(failure.causes.as_slice().is_empty());
    }

    #[test]
    fn query_diagnostics_keep_entities_original_ranges_and_more_than_cause_budget() {
        let raw = json!({"severity":"error","diagnostics":[{"origin":"typst","text":"unknown variable: 作者 <x>"}],
            "origin":{"path":"作者.typ","from":3,"to":9,"utf16From":3,"utf16To":5,"raw":"作者","syntaxKind":"Diagnostic","isMain":true}});
        let engine: weftext_engine::Diagnostic = serde_json::from_value(raw.clone()).unwrap();
        let description = weftext_messages::ErrorDescription::new(
            UiMessage::error_api_data_native_compile_failed(),
        );
        let mut original = CoreError::from_product(
            "original_failure",
            Some("native_kind".into()),
            description.clone(),
            vec![description.clone(); 48],
        );
        original.current_source = Some("等待期间输入 Y 😀".into());
        let error = ApiError::from(original);
        let expected = value(&error).unwrap();
        let engines = (0..48)
            .map(|index| QueryEngineDiagnostics {
                entity_key: format!("entity-{index}"),
                diagnostics: vec![engine.clone(); 48],
            })
            .collect();
        let products = (0..48)
            .map(|index| QueryProductError {
                entity_key: format!("entity-{index}"),
                error: error.clone(),
            })
            .collect();
        let mut output = json!({"coverage":"partial","limited":true,"aggregateExact":false});
        publish_query_diagnostics(&mut output, vec![description.into(); 48], engines, products)
            .unwrap();
        assert_eq!(output["diagnostics"].as_array().unwrap().len(), 48);
        assert_eq!(output["engineDiagnostics"].as_array().unwrap().len(), 48);
        assert_eq!(output["productErrors"].as_array().unwrap().len(), 48);
        for index in 0..48 {
            assert_eq!(
                output["engineDiagnostics"][index]["entityKey"],
                format!("entity-{index}")
            );
            assert_eq!(
                output["engineDiagnostics"][index]["diagnostics"]
                    .as_array()
                    .unwrap()
                    .len(),
                48
            );
            assert_eq!(output["engineDiagnostics"][index]["diagnostics"][47], raw);
            assert_eq!(output["productErrors"][index]["error"], expected);
        }
        assert_eq!(output["coverage"], "partial");
        assert_eq!(output["limited"], true);
        assert_eq!(output["aggregateExact"], false);
    }

    #[test]
    fn schema_and_original_fact_wrappers_preserve_complete_error_bodies() {
        let description = weftext_messages::ErrorDescription::new(
            UiMessage::error_api_data_proposal_validation_failed(),
        );
        let config = SchemaSource {
            source: "exact malformed author schema".into(),
            before: None,
            role_ref: None,
            revision: Some("original-revision".into()),
            can_install: false,
            base_id: None,
        };
        let source = weftext_data::DataError::native_budget(description.clone())
            .with_diagnostics(vec![description.clone(); 48]);
        let expected = value(data_error(source.clone())).unwrap();
        let wrapped = schema_parse_error(&config, source);
        assert_eq!(wrapped.status, StatusCode::BAD_REQUEST);
        assert_eq!(wrapped.code, "invalid_schema");
        let Some(ApiErrorDetails::SchemaSource {
            source,
            role_ref,
            revision,
            cause,
        }) = wrapped.details.as_deref()
        else {
            panic!("schema facts missing")
        };
        assert_eq!(source, &config.source);
        assert_eq!(role_ref, &None);
        assert_eq!(revision, &config.revision);
        assert_eq!(value(cause).unwrap(), expected);
        let mut original = CoreError::from_data(
            "operationConflict",
            weftext_data::DataError::coverage(description.clone())
                .with_diagnostics(vec![description; 48]),
        );
        original.current_source = Some("unsaved Y 😀".into());
        let original = ApiError::from(original);
        let original_wire = value(&original).unwrap();
        let fact = json!({"operationId":"ad6f2976-8381-4163-8f6c-1ec3711eb2df","status":"committed","path":"original.typ","baseId":"original-base"});
        let wrapped = original_operation_error(original, fact.clone());
        let Some(ApiErrorDetails::OriginalOperation {
            fact: retained,
            cause,
        }) = wrapped.details.as_deref()
        else {
            panic!("original operation fact missing")
        };
        assert_eq!(retained, &fact);
        assert_eq!(value(cause).unwrap(), original_wire);
        assert_eq!(wrapped.code, "operationConflict");
    }

    fn ok<T>(result: Result<T, ApiError>) -> T {
        result.unwrap_or_else(|error| panic!("{error:?}"))
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
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        (directory, state, root.to_string_lossy().to_string())
    }

    #[cfg(windows)]
    fn query_base_ids(frozen: &FrozenQuery) -> Vec<String> {
        frozen
            .versions
            .iter()
            .filter_map(|version| version["baseId"].as_str().map(str::to_owned))
            .chain(frozen.tags.values().map(|file| file.base_id.clone()))
            .collect()
    }

    #[cfg(windows)]
    #[test]
    fn temporary_query_cuts_release_all_lifecycles_on_success_and_error() {
        let (_directory, state, root) = fixture();
        let mut core = state.core.lock().unwrap();
        core.create_note_fixture(&root, &root, "Retained").unwrap();
        let retained = query_snapshot(&mut core, &root, Default::default()).unwrap();
        let retained_bases = query_base_ids(&retained);
        for fail in [false, true] {
            let temporary = query_snapshot(&mut core, &root, Default::default()).unwrap();
            let mut temporary_bases = query_base_ids(&temporary);
            let result = with_temporary_frozen_query(&mut core, temporary, |core, frozen| {
                extend_query_lifecycle(core, frozen, "trash".into())?;
                temporary_bases.extend(query_base_ids(frozen));
                assert_eq!(frozen.scope_cuts.len(), 2);
                for base in &temporary_bases {
                    core.snapshot_witness(base)?;
                }
                if fail {
                    Err(CoreError::new(
                        "invalidQuery",
                        UiMessage::error_api_data_temporary_validation_rejected(),
                    ))
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.is_err(), fail);
            for base in &temporary_bases {
                assert!(core.snapshot_witness(base).is_err());
            }
            for base in &retained_bases {
                assert!(core.snapshot_witness(base).is_ok());
            }
        }
        let mut invalid = query_snapshot(
            &mut core,
            &root,
            weftext_data::QueryPlan {
                r#where: Some("(".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let invalid_bases = query_base_ids(&invalid);
        assert!(freeze_query_definitions(&mut core, &mut invalid, BTreeMap::new()).is_err());
        for base in invalid_bases {
            assert!(core.snapshot_witness(&base).is_err());
        }
        for base in retained_bases {
            assert!(core.snapshot_witness(&base).is_ok());
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn failed_query_evaluation_releases_unpublished_basis_but_success_retains_strong_cut() {
        let (_directory, state, root) = fixture();
        state
            .core
            .lock()
            .unwrap()
            .create_note_fixture(&root, &root, "One")
            .unwrap();
        let (retained, retained_bases, invalid, invalid_bases) = {
            let mut core = state.core.lock().unwrap();
            let retained = query_snapshot(&mut core, &root, Default::default()).unwrap();
            let retained_bases = query_base_ids(&retained);
            let invalid = query_snapshot(
                &mut core,
                &root,
                weftext_data::QueryPlan {
                    r#where: Some("(".into()),
                    ..Default::default()
                },
            )
            .unwrap();
            let invalid_bases = query_base_ids(&invalid);
            (retained, retained_bases, invalid, invalid_bases)
        };
        assert!(evaluate_frozen(&state, invalid).await.is_err());
        let output = evaluate_frozen(&state, retained).await.unwrap();
        let mut core = state.core.lock().unwrap();
        for base in invalid_bases {
            assert!(core.snapshot_witness(&base).is_err());
        }
        for base in retained_bases {
            assert!(core.snapshot_witness(&base).is_ok());
        }
        core.prepare_admitted_operation(
            CoreOperationRequest {
                operation_id: uuid::Uuid::new_v4().to_string(),
                root: root.clone(),
                intent: OperationIntent::NewNode {
                    parent: root,
                    name: "Strong".into(),
                    source: None,
                },
            },
            output["cutId"].as_str().unwrap(),
        )
        .unwrap();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn successful_trash_query_retains_observed_row_basis_for_exact_opening() {
        let (_directory, state, root) = fixture();
        let (path, source) = {
            let mut core = state.core.lock().unwrap();
            let note = core.create_note_fixture(&root, &root, "Observed").unwrap();
            let preview = core
                .prepare_operation(CoreOperationRequest {
                    root: root.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    intent: OperationIntent::TrashNode {
                        source: Path::new(&note.path)
                            .parent()
                            .unwrap()
                            .to_string_lossy()
                            .into(),
                    },
                })
                .unwrap();
            core.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap();
            let temporary = core.trash_admitted_snapshot(&root).unwrap();
            let path = temporary.files[0].path.clone();
            core.release_admitted_cut(&temporary.cut_id);
            (path, format!("{}\n// observed external edit", note.source))
        };
        std::fs::write(&path, &source).unwrap();
        let Json(rows) = ok(query(
            State(state.clone()),
            crate::RequestJson(QueryRequest {
                root: root.clone(),
                parameters: BTreeMap::new(),
                plan: weftext_data::QueryPlan {
                    scope: weftext_data::QueryScope {
                        lifecycle: "trash".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
        )
        .await);
        let row = &rows["rows"][0];
        let revision = row["revision"].as_str().unwrap();
        let base = revision.strip_prefix("observed-source:").unwrap();
        assert!(state.core.lock().unwrap().snapshot_witness(base).is_ok());
        let Json(opened) = ok(open_entity(
            State(state.clone()),
            crate::RequestJson(ResolveRequest {
                root,
                entity_key: row["entityKey"].as_str().unwrap().into(),
                lifecycle: Some("trash".into()),
                revision: Some(revision.into()),
            }),
        )
        .await);
        assert_eq!(opened["source"], source);
        assert_eq!(opened["readOnly"], true);
        assert!(state.core.lock().unwrap().snapshot_witness(base).is_ok());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn definition_save_rejects_short_circuited_missing_tag_root_with_deferred_parameters() {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let initial_root = root.clone();
        let (tag, document) = ok(core_call(&state, move |core| {
            let basis = core.read_tag_registry(&initial_root)?;
            let prepared = core.prepare_create_tag(
                &initial_root,
                "root",
                &basis.base_id,
                &uuid::Uuid::new_v4().to_string(),
            )?;
            core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)?;
            let document = core.create_note_fixture(&initial_root, &initial_root, "Definition")?;
            Ok((prepared.target_ref, document))
        })
        .await);
        let original = document.source.clone();
        let document = serde_json::to_value(document).unwrap();
        remember(&state, &document).unwrap();
        let missing = format!(
            "weftext-tag://{}/{}",
            tag.split('/').nth(2).unwrap(),
            uuid::Uuid::new_v4()
        );
        let parameter = weftext_data::QueryParameter {
            name: "root".into(),
            shape: weftext_data::Shape::Scalar {
                semantic: "tag_ref".into(),
            },
            nullable: false,
            default: None,
        };
        let plan = weftext_data::QueryPlan {
            r#where: Some(format!(
                "false && tagInSubtree(tagRef('{tag}'), tagRef('{missing}'))"
            )),
            select: vec![weftext_data::QueryColumn {
                name: "value".into(),
                expr: "true".into(),
            }],
            advanced: Some(weftext_data::QueryAdvanced {
                parameters: vec![parameter],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut request = definition_request(&document, original.clone(), None, "open");
        request.plan = plan.clone();
        let error = save_definition(State(state.clone()), crate::RequestJson(request))
            .await
            .unwrap_err();
        assert_eq!(
            error.explanation,
            UiMessage::error_data_query_tag_root_unavailable_in_frozen_qualified_tree()
        );
        assert_eq!(
            std::fs::read_to_string(document["path"].as_str().unwrap()).unwrap(),
            original
        );
        let mut request = definition_request(&document, original, None, "open");
        request.plan = plan;
        request.plan.r#where = Some(format!("false && tagInSubtree(tagRef('{tag}'), root)"));
        let Json(saved) = ok(save_definition(State(state), crate::RequestJson(request)).await);
        assert!(saved["definitionRevision"].is_string());
    }

    #[cfg(windows)]
    #[test]
    fn tag_query_keeps_the_original_core_cut_after_move_and_zero_rows_still_validate_root() {
        let (_directory, state, root) = fixture();
        let mut core = state.core.lock().unwrap();
        let create = |core: &mut Core, parent: Option<&str>| {
            let basis = core.read_tag_registry(&root).unwrap();
            let prepared = core
                .prepare_create_tag_under(
                    &root,
                    "same/name",
                    parent,
                    &basis.base_id,
                    &uuid::Uuid::new_v4().to_string(),
                )
                .unwrap();
            core.execute_operation(&prepared.preview.operation_id, &prepared.preview.plan_id)
                .unwrap();
            prepared.target_ref
        };
        let parent = create(&mut core, None);
        let leaf = create(&mut core, Some(&parent));
        let plan = weftext_data::QueryPlan {
            r#where: Some(format!(
                "tagInSubtree(tagRef('{leaf}'), tagRef('{parent}'))"
            )),
            select: vec![weftext_data::QueryColumn {
                name: "value".into(),
                expr: "true".into(),
            }],
            ..Default::default()
        };
        let mut zero = query_snapshot(&mut core, &root, plan.clone()).unwrap();
        freeze_query_definitions(&mut core, &mut zero, BTreeMap::new()).unwrap();
        assert!(zero.versions.iter().any(|v| v["role"] == "tags"));
        let result = weftext_data::execute_query(&zero.input, &zero.plan, &zero.schema).unwrap();
        assert!(result.rows.is_empty());
        assert_eq!(result.coverage, "complete");
        let invalid = weftext_data::QueryPlan {
            r#where: Some(format!(
                "true || tagInSubtree(tagRef('{leaf}'), tagRef('weftext-tag://{}/{}'))",
                parent.split('/').nth(2).unwrap(),
                uuid::Uuid::new_v4()
            )),
            ..plan.clone()
        };
        let mut invalid = query_snapshot(&mut core, &root, invalid).unwrap();
        freeze_query_definitions(&mut core, &mut invalid, BTreeMap::new()).unwrap();
        assert!(
            weftext_data::execute_query(&invalid.input, &invalid.plan, &invalid.schema).is_err()
        );
        core.create_note_fixture(&root, &root, "one").unwrap();
        let mut original = query_snapshot(&mut core, &root, plan.clone()).unwrap();
        let mut missing_tags = query_snapshot(&mut core, &root, plan.clone()).unwrap();
        let basis = core.read_tag_registry(&root).unwrap();
        let moved = core
            .prepare_move_tag(
                &root,
                &leaf,
                None,
                &basis.base_id,
                &uuid::Uuid::new_v4().to_string(),
            )
            .unwrap();
        core.execute_operation(&moved.preview.operation_id, &moved.preview.plan_id)
            .unwrap();
        missing_tags.tags.clear();
        assert_eq!(
            freeze_query_definitions(&mut core, &mut missing_tags, BTreeMap::new())
                .unwrap_err()
                .code,
            "tagsUnavailable",
            "The missing original capture must not be replaced by the current moved tree"
        );
        freeze_query_definitions(&mut core, &mut original, BTreeMap::new()).unwrap();
        assert_eq!(
            weftext_data::execute_query(&original.input, &original.plan, &original.schema)
                .unwrap()
                .rows
                .len(),
            1
        );
        let mut current = query_snapshot(&mut core, &root, plan.clone()).unwrap();
        freeze_query_definitions(&mut core, &mut current, BTreeMap::new()).unwrap();
        assert!(
            weftext_data::execute_query(&current.input, &current.plan, &current.schema)
                .unwrap()
                .rows
                .is_empty()
        );
        use std::os::windows::fs::OpenOptionsExt;
        let busy = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(Path::new(&root).join(".weftext-tags.json"))
            .unwrap();
        let exact = weftext_data::QueryPlan {
            r#where: Some(format!("tagRef('{leaf}') == tagRef('{leaf}')")),
            ..plan.clone()
        };
        let mut unrelated = query_snapshot(&mut core, &root, exact).unwrap();
        assert!(unrelated.tags.is_empty());
        freeze_query_definitions(&mut core, &mut unrelated, BTreeMap::new()).unwrap();
        assert_eq!(
            weftext_data::execute_query(&unrelated.input, &unrelated.plan, &unrelated.schema)
                .unwrap()
                .rows
                .len(),
            1
        );
        let mut structural = query_snapshot(&mut core, &root, plan).unwrap();
        assert!(freeze_query_definitions(&mut core, &mut structural, BTreeMap::new()).is_err());
        drop(busy);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn bootstrap_reopens_current_source_and_conflicting_draft_without_reusing_launch_session()
    {
        for managed in [true, false] {
            let (_directory, mut state, root) = fixture();
            let (initial, draft, changed) = {
                let mut core = state.core.lock().unwrap();
                let initial = if managed {
                    core.create_note_fixture(&root, &root, "Launch").unwrap()
                } else {
                    let path = Path::new(&root).join("ordinary.typ");
                    std::fs::write(&path, "Original ordinary source").unwrap();
                    core.open(path, Some(PathBuf::from(&root))).unwrap()
                };
                let draft = format!("{}\nPersonal retained proposal", initial.source);
                let changed = format!("{}\nExternal current bytes", initial.source);
                core.persist_draft(&initial.session_id, &draft).unwrap();
                std::fs::write(&initial.path, &changed).unwrap();
                (initial, draft, changed)
            };
            state.initial = Some(Arc::new(serde_json::to_value(&initial).unwrap()));
            let routed_state = state.clone();
            assert!(Arc::ptr_eq(
                state.initial.as_ref().unwrap(),
                routed_state.initial.as_ref().unwrap()
            ));
            let mut previous_session = initial.session_id.clone();
            for _ in 0..2 {
                let Json(result) = ok(crate::bootstrap(State(state.clone())).await);
                let fresh = &result["initialDocument"];
                assert!(result["initialDocumentError"].is_null());
                assert_eq!(fresh["source"], changed);
                assert_eq!(fresh["recoveredDraft"], draft);
                assert_eq!(fresh["recoveredDraftBaseSource"], initial.source);
                assert_eq!(fresh["recoveredDraftConflict"], true);
                assert_ne!(fresh["sessionId"], previous_session);
                previous_session = fresh["sessionId"].as_str().unwrap().to_owned();
                assert!(
                    state
                        .locations
                        .lock()
                        .unwrap()
                        .contains_key(&previous_session)
                );
                assert_eq!(std::fs::read_to_string(&initial.path).unwrap(), changed);
            }
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn bootstrap_follows_original_moved_identity_and_refuses_same_byte_replacement() {
        let (_directory, mut state, root) = fixture();
        let initial = state
            .core
            .lock()
            .unwrap()
            .create_note_fixture(&root, &root, "Launch")
            .unwrap();
        state.initial = Some(Arc::new(serde_json::to_value(&initial).unwrap()));
        let draft = format!("{}\nRetain across movement", initial.source);
        {
            let mut core = state.core.lock().unwrap();
            core.persist_draft(&initial.session_id, &draft).unwrap();
            let plan = core
                .prepare_operation(CoreOperationRequest {
                    root: root.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    intent: OperationIntent::RenameNode {
                        source: Path::new(&initial.path)
                            .parent()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        name: "Moved".into(),
                    },
                })
                .unwrap();
            core.execute_operation(&plan.operation_id, &plan.plan_id)
                .unwrap();
            let namesake = core.create_note_fixture(&root, &root, "Launch").unwrap();
            assert_ne!(namesake.node_id, initial.node_id);
        }
        let Json(result) = ok(crate::bootstrap(State(state.clone())).await);
        let moved = &result["initialDocument"];
        assert_eq!(moved["nodeId"].as_str(), initial.node_id.as_deref());
        assert_eq!(moved["recoveredDraft"], draft);
        let path = PathBuf::from(moved["path"].as_str().unwrap());
        assert_ne!(path, PathBuf::from(&initial.path));
        let bytes = std::fs::read(&path).unwrap();
        std::fs::rename(&path, path.with_extension("retained-original")).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let Json(unavailable) = ok(crate::bootstrap(State(state.clone())).await);
        assert!(unavailable["initialDocument"].is_null());
        assert!(unavailable["initialDocumentError"]["code"].is_string());
        assert!(unavailable["recoveries"].is_array());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
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
                (
                    Schema::from_author_json(BUNDLED_SCHEMA_SOURCE)
                        .unwrap()
                        .field("status")
                        .unwrap()
                        .id
                        .clone(),
                    NativeValue::Text("open".into()),
                ),
                (
                    Schema::from_author_json(BUNDLED_SCHEMA_SOURCE)
                        .unwrap()
                        .field("start")
                        .unwrap()
                        .id
                        .clone(),
                    NativeValue::Date("2026-10-13".into()),
                ),
            ]),
            body: "作者正文与注释 // retained\n".into(),
            body_mode: weftext_core::CaptureBodyMode::Markup,
            form_basis: None,
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
            crate::RequestJson(task_capture(&root, "Native", &operation)),
        )
        .await);
        assert_eq!(first["status"], "committed");
        let first = first["document"].clone();
        let Json(retry) = ok(capture(
            State(state.clone()),
            crate::RequestJson(task_capture(&root, "Native", &operation)),
        )
        .await);
        assert_eq!(retry["status"], "committed");
        let retry = retry["document"].clone();
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
            crate::RequestJson(task_capture(
                &root,
                "Edit",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
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
        let Json(proposal) = ok(edit(State(state.clone()), crate::RequestJson(request)).await);
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
            crate::RequestJson(FieldEdit {
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
            crate::RequestJson(RootRequest { root: root.clone() }),
        )
        .await);
        assert_eq!(initial["builtinDefault"], true);
        assert!(!Path::new(&root).join(".weftext-schema.json").exists());
        let mut edited: Value = serde_json::from_str(initial["source"].as_str().unwrap()).unwrap();
        edited["aliases"]["chores"] = json!("tasks/task");
        let new_source = serde_json::to_string(&edited).unwrap();
        let Json(preview) = ok(prepare_schema(
            State(state.clone()),
            crate::RequestJson(SchemaPrepare {
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
            crate::RequestJson(SchemaInstall {
                operation_id: preview["operationId"].as_str().unwrap().into(),
                plan_id: preview["planId"].as_str().unwrap().into(),
            }),
        )
        .await);
        let Json(installed) = ok(schema(
            State(state.clone()),
            crate::RequestJson(RootRequest { root: root.clone() }),
        )
        .await);
        assert_eq!(installed["source"], new_source);
        assert_eq!(installed["schema"]["aliases"]["chores"], "tasks/task");
        let next_source = format!("{new_source}\n");
        let Json(next) = ok(prepare_schema(
            State(state.clone()),
            crate::RequestJson(SchemaPrepare {
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
            crate::RequestJson(SchemaInstall {
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
            crate::RequestJson(task_capture(
                &root,
                "Before",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
        let entity = format!(
            "weftext-node://{}/{}",
            document["workspaceId"].as_str().unwrap(),
            document["nodeId"].as_str().unwrap()
        );
        let plan: weftext_data::QueryPlan = serde_json::from_value(json!({"scope":{"directory":entity,"descendants":true},"types":["task"],"where":"status == 'open'","select":[{"name":"status","expr":"status"}]})).unwrap();
        let Json(rows) = ok(query(
            State(state.clone()),
            crate::RequestJson(QueryRequest {
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
            crate::RequestJson(ResolveRequest {
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
            crate::RequestJson(QueryRequest {
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
            crate::RequestJson(ResolveRequest {
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
            crate::RequestJson(QueryRequest {
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
            crate::RequestJson(RootRequest { root: root.into() }),
        )
        .await);
        let source = initial["source"].as_str().unwrap().to_string();
        let Json(preview) = ok(prepare_schema(
            State(state.clone()),
            crate::RequestJson(SchemaPrepare {
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
            crate::RequestJson(SchemaInstall {
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
            input_revision: None,
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
            crate::RequestJson(task_capture(
                &root,
                "Embedded",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
        let Json(saved) = ok(save_definition(
            State(state.clone()),
            crate::RequestJson(definition_request(
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
            crate::RequestJson(task_capture(
                &root,
                "Saved",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
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
            crate::RequestJson(definition_request(&document, dirty_source, None, "open")),
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
        assert_eq!(first_document["sessionId"], document["sessionId"]);
        let mut second_request = definition_request(
            first_document,
            first_document["source"].as_str().unwrap().into(),
            Some(key.clone()),
            "done",
        );
        // Applying the first committed patch advances this same editor's
        // source generation; a fresh Session must not hide a stale serial.
        second_request.serial = 2;
        let Json(second) =
            ok(save_definition(State(state.clone()), crate::RequestJson(second_request)).await);
        assert_ne!(second["definitionRevision"], old_revision);
        let Json(current) = ok(run_definition(
            State(state.clone()),
            crate::RequestJson(DefinitionRunRequest {
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
            crate::RequestJson(DefinitionRunRequest {
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
            crate::RequestJson(DefinitionRunRequest {
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
            crate::RequestJson(DefinitionRunRequest {
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
            crate::RequestJson(QueryRequest {
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
            crate::RequestJson(QueryRequest {
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
        assert_eq!(
            raw["snapshotLabel"],
            json!(UiMessage::error_api_data_admitted_snapshot_label())
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn periods_use_exact_iso_keys_and_unique_policy_blocks_every_creation_route() {
        let (_directory, state, root) = fixture();
        let Json(initial) = ok(resolve_period(
            State(state.clone()),
            crate::RequestJson(PeriodRequest {
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
        let Json(created) =
            ok(create_period(State(state.clone()), crate::RequestJson(make())).await);
        assert_eq!(created["status"], "committed");
        let created = created["document"].clone();
        let schema_path = Path::new(&root).join(".weftext-schema.json");
        let original_schema = std::fs::read(&schema_path).ok();
        std::fs::write(&schema_path, b"invalid current schema").unwrap();
        let Json(retry) = ok(create_period(State(state.clone()), crate::RequestJson(make())).await);
        match original_schema {
            Some(bytes) => std::fs::write(&schema_path, bytes).unwrap(),
            None => std::fs::remove_file(&schema_path).unwrap(),
        }

        assert_eq!(retry["status"], "committed");
        let retry = retry["document"].clone();
        assert_eq!(created["nodeId"], retry["nodeId"]);
        let Json(found) = ok(resolve_period(
            State(state.clone()),
            crate::RequestJson(PeriodRequest {
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
            crate::RequestJson(CaptureRequest {
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
                body_mode: weftext_core::CaptureBodyMode::Markup,
                form_basis: None,
                operation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
        .await;
        assert_eq!(duplicate.err().unwrap().code, "periodConflict");
        assert!(!Path::new(&root).join("Duplicate").exists());
        let Json(next) = ok(resolve_period(
            State(state.clone()),
            crate::RequestJson(PeriodRequest {
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
            crate::RequestJson(PeriodRequest {
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
            crate::RequestJson(task_capture(
                &root,
                "Trash",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
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
            crate::RequestJson(QueryRequest {
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
            crate::RequestJson(ResolveRequest {
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
            crate::RequestJson(ProjectionRequest {
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
            crate::RequestJson(FieldEdit {
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
            crate::RequestJson(task_capture(
                &root,
                "Race",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
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
        .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(reopened["source"], external);
        assert!(!document_matches_committed_source(&reopened, &committed));
        let owner = document["nodeId"].as_str().unwrap().to_string();
        let retained = ok(core_call(&state, move |core| {
            core.read_source_revision(
                root,
                &owner,
                receipt
                    .receipt
                    .revision_id
                    .as_deref()
                    .unwrap_or(&receipt.receipt.base_id),
            )
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
            crate::RequestJson(task_capture(
                &root,
                "Retry",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
        let request = definition_request(
            &document,
            document["source"].as_str().unwrap().into(),
            None,
            "open",
        );
        let Json(first) =
            ok(save_definition(State(state.clone()), crate::RequestJson(request.clone())).await);
        let later_draft = format!(
            "{}\nlater unsaved input\n",
            first["committedSource"].as_str().unwrap()
        );
        ok(
            writing_api::observe_snapshot(&state, &request.session_id, &later_draft, 2, None)
                .map_err(ApiError::from),
        );
        let Json(current_schema) = ok(schema(
            State(state.clone()),
            crate::RequestJson(RootRequest { root: root.clone() }),
        )
        .await);
        let changed_schema = format!("{}\n ", current_schema["source"].as_str().unwrap());
        let Json(preview) = ok(prepare_schema(
            State(state.clone()),
            crate::RequestJson(SchemaPrepare {
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
            crate::RequestJson(SchemaInstall {
                operation_id: preview["operationId"].as_str().unwrap().into(),
                plan_id: preview["planId"].as_str().unwrap().into(),
            }),
        )
        .await);
        let Json(replayed) =
            ok(save_definition(State(state.clone()), crate::RequestJson(request.clone())).await);
        assert_eq!(replayed["definitionRevision"], first["definitionRevision"]);
        assert_eq!(replayed["definition"]["key"], first["definition"]["key"]);
        assert_eq!(replayed["committedSource"], first["committedSource"]);
        ok(
            writing_api::check_snapshot(&state, &request.session_id, &later_draft, 2, None)
                .map_err(ApiError::from),
        );
        let definitions =
            weftext_data::saved_queries(replayed["committedSource"].as_str().unwrap()).unwrap();
        assert_eq!(definitions.len(), 1);
        let mut changed = request;
        changed.label = "different input".into();
        let rejected = save_definition(State(state), crate::RequestJson(changed)).await;
        assert_eq!(rejected.err().unwrap().code, "operationConflict");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn saved_definition_keeps_typed_scope_and_resolves_it_after_owner_rename() {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(document) = ok(capture(
            State(state.clone()),
            crate::RequestJson(task_capture(
                &root,
                "Scoped",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        assert_eq!(document["status"], "committed");
        let document = document["document"].clone();
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
        let Json(saved) =
            ok(save_definition(State(state.clone()), crate::RequestJson(request)).await);
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
            crate::RequestJson(DefinitionRunRequest {
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

    #[cfg(windows)]
    include!("data_lifecycle_tests.rs");
    #[cfg(windows)]
    #[tokio::test]
    async fn original_capture_replay_precedes_current_schema_interpretation() {
        let (_directory, state, root) = fixture();
        let id = uuid::Uuid::new_v4().to_string();
        let Json(first) = ok(capture(
            State(state.clone()),
            crate::RequestJson(task_capture(&root, "Original", &id)),
        )
        .await);
        assert_eq!(first["status"], "committed");
        std::fs::write(
            Path::new(&root).join(".weftext-schema.json"),
            b"invalid current schema",
        )
        .unwrap();
        let Json(retry) = ok(capture(
            State(state.clone()),
            crate::RequestJson(task_capture(&root, "Original", &id)),
        )
        .await);
        assert_eq!(retry["operationId"], id);
        assert_eq!(retry["status"], "committed");
        assert_eq!(retry["document"]["nodeId"], first["document"]["nodeId"]);
        let changed = task_capture(&root, "Changed", &id);
        assert_eq!(
            capture(State(state), crate::RequestJson(changed))
                .await
                .err()
                .unwrap()
                .code,
            "operationConflict"
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn actual_postcommit_open_failure_keeps_original_receipt_and_does_not_recreate() {
        let (_directory, state, root) = fixture();
        let id = uuid::Uuid::new_v4().to_string();
        let original_root = root.clone();
        let operation = id.clone();
        let (preview, receipt) = core_call(&state, move |core| {
            let receipt =
                core.create_note_with_id(&original_root, &original_root, "Committed", &operation)?;
            Ok((core.operation_details(&operation)?, receipt))
        })
        .await
        .unwrap();
        let original = PathBuf::from(&receipt.path);
        let retained = PathBuf::from(&root).join("retained-original");
        std::fs::rename(&original, &retained).unwrap();
        let output = crate::knowledge_api::operation_output(&state, preview, receipt, None)
            .await
            .unwrap();
        assert_eq!(output["status"], "committed");
        assert_eq!(output["operationId"], id);
        assert!(output["documentError"]["code"].is_string());
        assert!(output["documentError"].get("message").is_none());
        serde_json::from_value::<UiMessage>(output["documentError"]["explanation"].clone())
            .unwrap();
        serde_json::from_value::<Vec<weftext_messages::RawDiagnostic>>(
            output["documentError"]["diagnostics"].clone(),
        )
        .unwrap();
        serde_json::from_value::<weftext_messages::ErrorCauses>(
            output["documentError"]["causes"].clone(),
        )
        .unwrap();
        assert_eq!(output["documentError"]["details"]["kind"], "core");
        assert_eq!(
            output["documentError"]["details"]["code"],
            output["documentError"]["code"]
        );
        assert!(output["document"].is_null());
        let retry = crate::create_note(
            State(state.clone()),
            crate::RequestJson(crate::CreateNote {
                root: root.clone(),
                parent: root,
                name: "Committed".into(),
                operation_id: Some(id.clone()),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(retry["status"], "committed");
        assert_eq!(retry["operationId"], id);
        assert!(!original.exists());
        assert!(retained.join("Committed.typ").is_file());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn file_save_delivery_keeps_typed_failure_and_original_fact_until_physical_owner_returns()
    {
        let (directory, state, _root) = fixture();
        let path = directory.path().join("ordinary.typ");
        let retained = directory.path().join("original-retained.typ");
        let substitute = directory.path().join("substitute-retained.typ");
        std::fs::write(&path, "original B").unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let fact = core_call(&state, {
            let path = path.clone();
            let id = id.clone();
            move |core| {
                let document = core.open(&path, None)?;
                core.save(SaveRequest {
                    session_id: document.session_id.clone(),
                    base_id: document.base_id,
                    source: "committed S 😀".into(),
                    operation_id: id.clone(),
                })?;
                core.close_session(&document.session_id)?;
                core.local_operation_status(&id)
            }
        })
        .await
        .unwrap();
        let plan = directory
            .path()
            .join("state/journal")
            .join(&id[..2])
            .join(format!("{id}.plan.json"));
        let commit = directory
            .path()
            .join("state/journal")
            .join(&id[..2])
            .join(format!("{id}.commit.json"));
        let plan_bytes = std::fs::read(&plan).unwrap();
        let commit_bytes = std::fs::read(&commit).unwrap();
        let source_bytes = std::fs::read(&path).unwrap();
        std::fs::rename(&path, &retained).unwrap();
        std::fs::write(&path, &source_bytes).unwrap();
        let unavailable =
            crate::knowledge_api::file_save_delivery(&state, id.clone(), fact.clone(), None).await;
        assert_eq!(unavailable["status"], "committed");
        assert_eq!(unavailable["operationId"], id);
        assert_eq!(unavailable["baseId"], fact["baseId"]);
        assert!(unavailable["document"].is_null());
        let error = &unavailable["documentError"];
        assert_eq!(error["code"], "committedResultUnavailable");
        assert!(error.get("message").is_none());
        serde_json::from_value::<UiMessage>(error["explanation"].clone()).unwrap();
        assert!(error["diagnostics"].is_array());
        assert!(error["causes"].is_array());
        assert_eq!(error["details"]["kind"], "core");
        assert_eq!(error["details"]["code"], error["code"]);
        std::fs::rename(&path, &substitute).unwrap();
        std::fs::rename(&retained, &path).unwrap();
        let delivered =
            crate::knowledge_api::file_save_delivery(&state, id.clone(), fact.clone(), None).await;
        assert_eq!(delivered["status"], "committed");
        assert_eq!(delivered["operationId"], id);
        assert_eq!(delivered["baseId"], fact["baseId"]);
        assert!(delivered.get("documentError").is_none());
        assert_eq!(delivered["document"]["source"], "committed S 😀");
        assert_eq!(delivered["documentSourceMatchesCommit"], true);
        assert_eq!(std::fs::read(&path).unwrap(), source_bytes);
        assert_eq!(std::fs::read(&substitute).unwrap(), source_bytes);
        assert_eq!(std::fs::read(&plan).unwrap(), plan_bytes);
        assert_eq!(std::fs::read(&commit).unwrap(), commit_bytes);
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn rejected_definition_reservation_can_be_fenced_and_a_new_label_saved() {
        let (_directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(created) = ok(capture(
            State(state.clone()),
            crate::RequestJson(task_capture(
                &root,
                "Definitions",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let document = &created["document"];
        let Json(saved) = ok(save_definition(
            State(state.clone()),
            crate::RequestJson(definition_request(
                document,
                document["source"].as_str().unwrap().into(),
                None,
                "open",
            )),
        )
        .await);
        let document = &saved["document"];
        let mut bad = definition_request(
            document,
            document["source"].as_str().unwrap().into(),
            None,
            "open",
        );
        bad.serial = 2;
        let id = bad.operation_id.clone();
        assert!(
            save_definition(State(state.clone()), crate::RequestJson(bad))
                .await
                .is_err()
        );
        let settled = core_call(&state, {
            let id = id.clone();
            move |core| core.settle_local_attempt(&id, "save", &"a".repeat(64))
        })
        .await
        .unwrap();
        assert_eq!(settled["status"], "resolved");
        let mut next = definition_request(
            document,
            document["source"].as_str().unwrap().into(),
            None,
            "open",
        );
        next.label = "weftext-query-distinct".into();
        next.serial = 2;
        let Json(saved) = ok(save_definition(State(state.clone()), crate::RequestJson(next)).await);
        assert_eq!(saved["status"], "committed");
        let destination = Path::new(&root)
            .parent()
            .unwrap()
            .join("backup-after-rejected-definition");
        core_call(&state, move |core| {
            let p = core.prepare_operation(CoreOperationRequest {
                operation_id: uuid::Uuid::new_v4().to_string(),
                root,
                intent: OperationIntent::BackupLibrary {
                    destination: destination.to_string_lossy().into_owned(),
                },
            })?;
            core.execute_operation(&p.operation_id, &p.plan_id)?;
            Ok(())
        })
        .await
        .unwrap();
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn definition_replay_qualifies_the_complete_original_save_without_old_sessions() {
        let (directory, state, root) = fixture();
        install_bundled_schema(&state, &root).await;
        let Json(created) = ok(capture(
            State(state.clone()),
            crate::RequestJson(task_capture(
                &root,
                "DefinitionReplay",
                &uuid::Uuid::new_v4().to_string(),
            )),
        )
        .await);
        let document = &created["document"];
        let request = definition_request(
            document,
            document["source"].as_str().unwrap().into(),
            None,
            "open",
        );
        let Json(first) =
            ok(save_definition(State(state.clone()), crate::RequestJson(request.clone())).await);
        let plan_path = directory
            .path()
            .join("state/journal")
            .join(&request.operation_id[..2])
            .join(format!("{}.plan.json", request.operation_id));
        let committed_plan = std::fs::read(&plan_path).unwrap();
        let path = PathBuf::from(first["document"]["path"].as_str().unwrap());
        let committed_source = std::fs::read(&path).unwrap();
        let session = request.session_id.clone();
        core_call(&state, move |core| core.close_session(&session))
            .await
            .unwrap();
        state.locations.lock().unwrap().clear();
        let Json(replayed) =
            ok(save_definition(State(state.clone()), crate::RequestJson(request.clone())).await);
        assert_eq!(replayed["baseId"], first["baseId"]);
        assert_eq!(replayed["committedSource"], first["committedSource"]);
        assert_ne!(replayed["document"]["sessionId"], request.session_id);
        for field in ["session", "base", "source"] {
            let mut changed = request.clone();
            match field {
                "session" => changed.session_id = uuid::Uuid::new_v4().to_string(),
                "base" => changed.base_id = uuid::Uuid::new_v4().to_string(),
                _ => changed.source.push('Y'),
            }
            let error = save_definition(State(state.clone()), crate::RequestJson(changed))
                .await
                .err()
                .unwrap();
            assert_eq!(error.code, "operationConflict");
            let error = value(error).unwrap();
            assert!(error.get("message").is_none());
            assert_eq!(error["details"]["kind"], "originalOperation");
            assert_eq!(
                error["details"]["fact"]["operationId"],
                request.operation_id
            );
            assert_eq!(error["details"]["fact"]["baseId"], first["baseId"]);
            assert_eq!(error["details"]["cause"]["code"], "operationConflict");
            assert_eq!(error["details"]["cause"]["details"]["kind"], "core");
        }
        assert_eq!(std::fs::read(&plan_path).unwrap(), committed_plan);
        assert_eq!(std::fs::read(&path).unwrap(), committed_source);
    }
}
