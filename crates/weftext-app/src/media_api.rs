//! Resource reads use the Core's owned manifest and exact current snapshot.
//! Container metadata is derived from the same bytes that the viewer receives.
use super::{ApiError, ApiResult, AppState, core_call, location, value};
#[cfg(test)]
use crate::CoreTestFixture;
use crate::RequestJson;
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use base64::Engine;
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, io::Cursor};
use weftext_core::{Core, CoreError, ManagedResource, SnapshotFile};
use weftext_knowledge::{PdfPageFacts, ResourceFacts, Timebase};
use weftext_messages::{DiagnosticOrigin, ErrorDescription, UiMessage};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/resources/list", post(list))
        .route("/api/resources/read", post(read))
        .route("/api/resources/prepare-import", post(prepare_import))
        .route("/api/query/preview", post(query_preview))
        .route("/api/query/preview-resource", post(query_preview_resource))
        .route("/api/navigation/preview", post(navigation_preview))
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ResourceMetadata {
    Image {
        width: u64,
        height: u64,
    },
    Pdf {
        pages: Vec<PdfPageFacts>,
    },
    Audio {
        timebase: Timebase,
        #[serde(rename = "durationTick")]
        duration_tick: u64,
    },
    Video {
        timebase: Timebase,
        #[serde(rename = "durationTick")]
        duration_tick: u64,
        width: u64,
        height: u64,
    },
    Unsupported {
        #[serde(flatten)]
        description: ErrorDescription,
    },
}
pub(crate) struct QualifiedResource {
    pub resource_id: String,
    pub resource_ref: String,
    pub node_id: String,
    pub file: SnapshotFile,
    pub manifest: SnapshotFile,
    pub metadata: ResourceMetadata,
    pub facts: Option<ResourceFacts>,
    pub content_type: String,
}
pub(crate) struct ResourceSet {
    pub resources: Vec<QualifiedResource>,
    pub warnings: Vec<ErrorDescription>,
}

pub(crate) async fn resource_set(
    state: &AppState,
    session_id: &str,
) -> Result<ResourceSet, ApiError> {
    let loc = location(state, session_id)?;
    let managed = core_call(state, move |core| {
        core.managed_resource_snapshot(loc.root, loc.path)
    })
    .await?;
    analyze_resources(managed).await
}
async fn analyze_resources(managed: Vec<ManagedResource>) -> Result<ResourceSet, ApiError> {
    crate::work::spawn_blocking(move || {
        let mut resources = Vec::new();
        let mut warnings = Vec::new();
        for resource in managed {
            let revision = resource.revision.clone();
            let (metadata, content_type) = inspect_media(&resource.bytes);
            let facts = revision
                .as_ref()
                .and_then(|revision| metadata_facts(&metadata, &resource.resource_ref, revision));
            if resource.identity_status != "admitted" {
                warnings.push(ErrorDescription::new(
                    UiMessage::error_api_media_resource_admission_pending(
                        weftext_messages::params::ErrorApiMediaResourceAdmissionPending {
                            path: resource.path.clone(),
                        },
                    ),
                ));
            }
            resources.push(QualifiedResource {
                resource_id: resource.resource_id,
                resource_ref: resource.resource_ref,
                node_id: resource.node_id,
                file: SnapshotFile {
                    path: resource.path,
                    bytes: Some(resource.bytes),
                    base_id: resource.base_id,
                    node_id: None,
                    kind: "resource".into(),
                    currentness: if revision.is_some() {
                        "accepted"
                    } else {
                        "observed"
                    }
                    .into(),
                    revision,
                },
                manifest: SnapshotFile {
                    path: resource.manifest_path,
                    bytes: Some(resource.manifest_bytes),
                    base_id: resource.manifest_base_id,
                    node_id: None,
                    kind: "manifest".into(),
                    currentness: "observed".into(),
                    revision: None,
                },
                metadata,
                facts,
                content_type,
            });
        }
        ResourceSet {
            resources,
            warnings,
        }
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_media_metadata_read_incomplete()))
}
pub(crate) async fn qualified_resource(
    state: &AppState,
    session_id: &str,
    resource: &str,
    revision: &str,
) -> Result<QualifiedResource, ApiError> {
    let mut set = resource_set(state, session_id).await?;
    let position = set
        .resources
        .iter()
        .position(|item| item.resource_ref == resource || item.resource_id == resource)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "resourceUnavailable",
                UiMessage::error_api_media_resource_not_registered(),
            )
        })?;
    let selected = set.resources.remove(position);
    if revision.is_empty() || selected.file.revision.as_deref() != Some(revision) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "resourceVersion",
            UiMessage::error_api_media_resource_version_changed(),
        ));
    }
    Ok(selected)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    session_id: String,
}
async fn list(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<SessionRequest>,
) -> ApiResult {
    let set = resource_set(&state, &request.session_id).await?;
    let resources: Vec<_> = set.resources.iter().map(descriptor).collect();
    Ok(Json(json!({"resources":resources,"warnings":set.warnings})))
}
fn descriptor(item: &QualifiedResource) -> Value {
    json!({
        "resourceId":item.resource_id,"resource":item.resource_ref,"nodeId":item.node_id,
        "name":std::path::Path::new(&item.file.path).file_name().unwrap_or_default().to_string_lossy(),
        "revision":item.file.revision,"readBasis":item.file.base_id,"bytes":item.file.bytes.as_ref().map(Vec::len),
        "contentType":item.content_type,"metadata":item.metadata,"annotationReady":item.facts.is_some(),
        "currentness":item.file.currentness,
    })
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QueryPreviewRequest {
    root: String,
    entity_key: String,
    revision: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NavigationPreviewRequest {
    root: String,
    entity_key: String,
    source_hash: String,
}
/// Browsing never opens an editor or replaces a row's source with latest data.
/// The admitted NodeRef and original displayed source hash qualify a read basis;
/// subsequent pixels use the same exact-source/resource preview path as Query.
async fn navigation_preview(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<NavigationPreviewRequest>,
) -> ApiResult {
    let proof = core_call(&state, move |core| {
        let inventory = core.inventory(&request.root)?;
        let entity = crate::data_api::resolve_entity(&inventory, &request.entity_key)?;
        let cut = core.read_snapshot(&request.root, std::slice::from_ref(&entity.path))?;
        let file = &cut.files[0];
        let source = file
            .bytes
            .as_ref()
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        if request.source_hash.len() != 64
            || file.node_id != entity.node_id
            || source
                .is_none_or(|source| weftext_engine::source_hash(source) != request.source_hash)
        {
            return Err(CoreError::new(
                "stalePreview",
                UiMessage::error_api_media_note_preview_basis_changed(),
            ));
        }
        Ok(QueryPreviewRequest {
            root: request.root,
            entity_key: request.entity_key,
            revision: format!("observed-source:{}", file.base_id),
        })
    })
    .await?;
    query_preview(State(state), crate::RequestJson(proof)).await
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QueryPreviewResourceRequest {
    root: String,
    entity_key: String,
    revision: String,
    resource: String,
    resource_revision: String,
}
fn query_resource_cut(
    core: &mut Core,
    request: &QueryPreviewRequest,
) -> Result<Vec<ManagedResource>, CoreError> {
    let inventory = core.inventory(&request.root)?;
    let entity = crate::data_api::resolve_entity(&inventory, &request.entity_key)?;
    let observed = request.revision.strip_prefix("observed-source:");
    if let Some(base) = observed {
        core.validate_snapshot(&request.root, &entity.path, base)?;
    }
    let mut managed = core.managed_resource_snapshot(&request.root, &entity.path)?;
    let manifest = std::path::Path::new(&entity.path)
        .parent()
        .ok_or_else(|| {
            CoreError::new(
                "invalidPath",
                UiMessage::error_api_media_preview_owner_missing(),
            )
        })?
        .join("weftext.resources.json")
        .to_string_lossy()
        .into_owned();
    let mut paths = vec![entity.path.clone(), manifest];
    paths.extend(managed.iter().map(|resource| resource.path.clone()));
    let cut = core.read_snapshot(&request.root, &paths)?;
    if request.revision.is_empty()
        || (observed.is_none() && cut.files[0].revision.as_deref() != Some(&request.revision))
        || cut.files[0].node_id != entity.node_id
    {
        return Err(CoreError::new(
            "stalePreview",
            UiMessage::error_api_media_query_preview_basis_changed(),
        ));
    }
    if let Some(base) = observed {
        core.validate_snapshot(&request.root, &entity.path, base)?;
    }
    let manifest = &cut.files[1];
    if managed.is_empty()
        && manifest.bytes.as_ref().is_some_and(|bytes| {
            serde_json::from_slice::<Value>(bytes)
                .ok()
                .is_none_or(|value| !value["resources"].as_array().is_some_and(Vec::is_empty))
        })
    {
        return Err(CoreError::new(
            "scopeChanged",
            UiMessage::error_api_media_preview_manifest_changed(),
        ));
    }
    for (resource, file) in managed.iter_mut().zip(&cut.files[2..]) {
        if file.bytes.as_ref() != Some(&resource.bytes)
            || file.revision != resource.revision
            || manifest.bytes.as_ref() != Some(&resource.manifest_bytes)
        {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_api_media_preview_object_changed(),
            ));
        }
        resource.base_id = file.base_id.clone();
        resource.manifest_base_id = manifest.base_id.clone();
    }
    Ok(managed)
}
async fn query_preview(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<QueryPreviewRequest>,
) -> ApiResult {
    let source_base = request
        .revision
        .strip_prefix("observed-source:")
        .map(str::to_owned);
    let source_revision = if source_base.is_some() {
        None
    } else {
        Some(request.revision.clone())
    };
    let source_basis_kind = if source_base.is_some() {
        "observed"
    } else {
        "committed"
    };
    let cut = core_call(&state, move |core| query_resource_cut(core, &request)).await;
    let managed = match cut {
        Ok(managed) => managed,
        Err(error) => {
            return Ok(Json(
                json!({"state":"unavailable","sourceRevision":source_revision,"sourceBaseId":source_base,"sourceBasisKind":source_basis_kind,"explanation":error.explanation,"diagnostics":error.diagnostics,"causes":error.causes}),
            ));
        }
    };
    let set = analyze_resources(managed).await?;
    let thumbnail = set.resources.iter().find(|resource| {
        resource.file.revision.is_some()
            && matches!(
                resource.metadata,
                ResourceMetadata::Image { .. } | ResourceMetadata::Pdf { .. }
            )
    });
    if let Some(resource) = thumbnail {
        return Ok(Json(
            json!({"state":"ready","descriptor":descriptor(resource),"previewBasis":"currentResourceAtRowSource","sourceRevision":source_revision,"sourceBaseId":source_base,"sourceBasisKind":source_basis_kind}),
        ));
    }
    Ok(Json(
        json!({"state":if set.resources.is_empty(){"absent"}else{"unsupported"},"sourceRevision":source_revision,"sourceBaseId":source_base,"sourceBasisKind":source_basis_kind,
            "explanation":UiMessage::error_api_media_thumbnail_unavailable(),"diagnostics":[],"causes":[]}),
    ))
}
async fn query_preview_resource(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<QueryPreviewResourceRequest>,
) -> Result<Response, ApiError> {
    let proof = QueryPreviewRequest {
        root: request.root,
        entity_key: request.entity_key,
        revision: request.revision,
    };
    let managed = core_call(&state, move |core| query_resource_cut(core, &proof)).await?;
    let mut set = analyze_resources(managed).await?;
    let index = set
        .resources
        .iter()
        .position(|resource| {
            resource.resource_ref == request.resource
                && resource.file.revision.as_deref() == Some(&request.resource_revision)
        })
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "stalePreview",
                UiMessage::error_api_media_thumbnail_version_changed(),
            )
        })?;
    let resource = set.resources.remove(index);
    let bytes = resource
        .file
        .bytes
        .ok_or_else(|| ApiError::bad(UiMessage::error_api_media_thumbnail_bytes_unavailable()))?;
    Ok((
        [
            (header::CONTENT_TYPE, resource.content_type),
            (header::CONTENT_DISPOSITION, "inline".into()),
        ],
        Body::from(bytes),
    )
        .into_response())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadRequest {
    session_id: String,
    resource: String,
    revision: String,
}
async fn read(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<ReadRequest>,
) -> Result<Response, ApiError> {
    let item = qualified_resource(
        &state,
        &request.session_id,
        &request.resource,
        &request.revision,
    )
    .await?;
    let bytes = item
        .file
        .bytes
        .ok_or_else(|| ApiError::bad(UiMessage::error_api_media_resource_bytes_unavailable()))?;
    Ok((
        [
            (header::CONTENT_TYPE, item.content_type),
            (header::CONTENT_DISPOSITION, "inline".into()),
        ],
        Body::from(bytes),
    )
        .into_response())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportRequest {
    session_id: String,
    name: String,
    bytes: String,
    operation_id: String,
}
async fn prepare_import(
    State(state): State<AppState>,
    RequestJson(request): RequestJson<ImportRequest>,
) -> ApiResult {
    // The authenticated browser explicitly chooses/uploads this input. No client
    // path is read and no API code writes a filesystem object.
    if request.bytes.len() > 86 * 1024 * 1024 {
        return Err(ApiError::bad(UiMessage::error_api_media_import_budget()));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.bytes)
        .map_err(|_| ApiError::bad(UiMessage::error_api_media_upload_base64_invalid()))?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(ApiError::bad(UiMessage::error_api_media_import_budget()));
    }
    let loc = location(&state, &request.session_id)?;
    Ok(Json(value(
        core_call(&state, move |core| {
            core.prepare_import_resource(
                loc.root,
                loc.path,
                &request.name,
                bytes,
                &request.operation_id,
            )
        })
        .await?,
    )?))
}

pub(crate) fn metadata_facts(
    metadata: &ResourceMetadata,
    resource: &str,
    revision: &str,
) -> Option<ResourceFacts> {
    Some(match metadata {
        ResourceMetadata::Image { width, height } => ResourceFacts::Image {
            resource: resource.into(),
            revision_id: revision.into(),
            width: *width,
            height: *height,
        },
        ResourceMetadata::Pdf { pages } => ResourceFacts::Pdf {
            resource: resource.into(),
            revision_id: revision.into(),
            pages: pages.clone(),
        },
        ResourceMetadata::Audio {
            timebase,
            duration_tick,
        } => ResourceFacts::Audio {
            resource: resource.into(),
            revision_id: revision.into(),
            timebase: timebase.clone(),
            duration_tick: *duration_tick,
        },
        ResourceMetadata::Video {
            timebase,
            duration_tick,
            width,
            height,
        } => ResourceFacts::Video {
            resource: resource.into(),
            revision_id: revision.into(),
            timebase: timebase.clone(),
            duration_tick: *duration_tick,
            width: *width,
            height: *height,
        },
        ResourceMetadata::Unsupported { .. } => return None,
    })
}
fn unsupported(description: ErrorDescription, mime: &str) -> (ResourceMetadata, String) {
    (ResourceMetadata::Unsupported { description }, mime.into())
}
pub(crate) fn inspect_media(bytes: &[u8]) -> (ResourceMetadata, String) {
    if bytes.starts_with(b"%PDF-") {
        return match pdf_metadata(bytes) {
            Ok(metadata) => (metadata, "application/pdf".into()),
            Err(message) => unsupported(message, "application/pdf"),
        };
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
        return match wav_metadata(bytes) {
            Ok(metadata) => (metadata, "audio/wav".into()),
            Err(message) => unsupported(message, "audio/wav"),
        };
    }
    if bytes.get(4..8) == Some(b"ftyp") {
        return match mp4_metadata(bytes) {
            Ok(metadata) => (metadata, "video/mp4".into()),
            Err(message) => unsupported(message, "video/mp4"),
        };
    }
    if let Ok(format) = image::guess_format(bytes) {
        let mime = match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::Gif => "image/gif",
            image::ImageFormat::WebP => "image/webp",
            _ => {
                return unsupported(
                    ErrorDescription::new(UiMessage::error_api_media_image_format_unsupported()),
                    "application/octet-stream",
                );
            }
        };
        let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let result = (|| -> Result<ResourceMetadata, ErrorDescription> {
            let mut decoder = reader.into_decoder().map_err(|error| {
                ErrorDescription::new(UiMessage::error_api_media_image_decode_unavailable())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })?;
            if decoder.orientation().map_err(|error| {
                ErrorDescription::new(UiMessage::error_api_media_image_orientation_invalid())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })? != image::metadata::Orientation::NoTransforms
            {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_image_transform_unsupported(),
                ));
            }
            let image = image::DynamicImage::from_decoder(decoder).map_err(|error| {
                ErrorDescription::new(UiMessage::error_api_media_image_bytes_undecodable())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })?;
            Ok(ResourceMetadata::Image {
                width: u64::from(image.width()),
                height: u64::from(image.height()),
            })
        })();
        return match result {
            Ok(metadata) => (metadata, mime.into()),
            Err(message) => unsupported(message, mime),
        };
    }
    unsupported(
        ErrorDescription::new(UiMessage::error_api_media_format_reader_unavailable()),
        "application/octet-stream",
    )
}
pub(crate) fn metadata_from_bytes(bytes: &[u8]) -> ResourceMetadata {
    inspect_media(bytes).0
}

fn pdf_metadata(bytes: &[u8]) -> Result<ResourceMetadata, ErrorDescription> {
    // This library's structural stream decompression is not output-bounded.
    // Refuse it before loading instead of claiming an unqualified parser sandbox.
    if bytes.len() > 16 * 1024 * 1024 || pdf_structural_stream(bytes) {
        return Err(ErrorDescription::new(
            UiMessage::error_api_media_pdf_structural_profile_unsupported(),
        )
        .into());
    }
    let document = lopdf::Document::load_mem(bytes).map_err(|error| {
        ErrorDescription::new(UiMessage::error_api_media_pdf_unreadable())
            .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
    })?;
    if document.is_encrypted() {
        return Err(ErrorDescription::new(UiMessage::error_api_media_pdf_encrypted()).into());
    }
    let pages = document.get_pages();
    if pages.is_empty() || pages.len() > 2000 {
        return Err(ErrorDescription::new(UiMessage::error_api_media_pdf_page_budget()).into());
    }
    let mut facts = Vec::with_capacity(pages.len());
    for id in pages.values() {
        let mut cursor = *id;
        let mut visited = BTreeSet::new();
        let mut media = None;
        let mut crop = None;
        let mut rotation = None;
        let mut unit = None;
        loop {
            if !visited.insert(cursor) || visited.len() > 64 {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_pdf_inheritance_cycle(),
                )
                .into());
            }
            let dictionary = document.get_dictionary(cursor).map_err(|error| {
                ErrorDescription::new(UiMessage::error_api_media_pdf_page_dictionary_invalid())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })?;
            if media.is_none() {
                media = dictionary.get(b"MediaBox").ok().cloned();
            }
            if crop.is_none() {
                crop = dictionary.get(b"CropBox").ok().cloned();
            }
            if rotation.is_none() {
                rotation = dictionary.get(b"Rotate").ok().cloned();
            }
            if unit.is_none() {
                unit = dictionary.get(b"UserUnit").ok().cloned();
            }
            match dictionary
                .get(b"Parent")
                .ok()
                .and_then(|object| object.as_reference().ok())
            {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
        let resolve = |object: lopdf::Object| -> Result<lopdf::Object, ErrorDescription> {
            match object {
                lopdf::Object::Reference(reference) => {
                    document.get_object(reference).cloned().map_err(|error| {
                        ErrorDescription::new(
                            UiMessage::error_api_media_pdf_page_reference_invalid(),
                        )
                        .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
                    })
                }
                object => Ok(object),
            }
        };
        let numeric = |object: &lopdf::Object| -> Option<f64> {
            match object {
                lopdf::Object::Integer(n) => Some(*n as f64),
                lopdf::Object::Real(n) => Some(f64::from(*n)),
                _ => None,
            }
        };
        let media = resolve(media.ok_or(ErrorDescription::new(
            UiMessage::error_api_media_pdf_media_box_missing(),
        ))?)?;
        let array = media.as_array().map_err(|error| {
            ErrorDescription::new(UiMessage::error_api_media_pdf_media_box_not_rectangle())
                .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
        })?;
        if array.len() != 4 {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_pdf_rectangle_invalid()).into(),
            );
        }
        let values: Option<Vec<_>> = array.iter().map(numeric).collect();
        let values = values.ok_or(ErrorDescription::new(
            UiMessage::error_api_media_pdf_rectangle_not_numeric(),
        ))?;
        if values.iter().any(|n| !n.is_finite()) || values[2] <= values[0] || values[3] <= values[1]
        {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_pdf_page_size_invalid()).into(),
            );
        }
        if let Some(crop) = crop
            && resolve(crop)? != media
        {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_pdf_crop_unsupported()).into(),
            );
        }
        if let Some(unit) = unit
            && numeric(&resolve(unit)?) != Some(1.0)
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_pdf_user_unit_unsupported(),
            )
            .into());
        }
        let rotation = rotation
            .map(resolve)
            .transpose()?
            .as_ref()
            .and_then(numeric)
            .unwrap_or(0.0);
        if rotation.fract() != 0.0 {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_pdf_rotation_invalid()).into(),
            );
        }
        let rotation = (rotation as i32).rem_euclid(360);
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_pdf_rotation_invalid()).into(),
            );
        }
        facts.push(PdfPageFacts {
            width: values[2] - values[0],
            height: values[3] - values[1],
            rotation: rotation as u16,
            extraction_id: None,
            extracted_scalars: None,
        });
    }
    Ok(ResourceMetadata::Pdf { pages: facts })
}
fn pdf_structural_stream(bytes: &[u8]) -> bool {
    let hex = |byte: u8| -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    };
    let delimiter = |byte: u8| byte.is_ascii_whitespace() || b"()<>[]{}/%".contains(&byte);
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] != b'/' {
            position += 1;
            continue;
        }
        position += 1;
        let mut name = Vec::new();
        while position < bytes.len() && !delimiter(bytes[position]) {
            if bytes[position] == b'#'
                && position + 2 < bytes.len()
                && let (Some(high), Some(low)) =
                    (hex(bytes[position + 1]), hex(bytes[position + 2]))
            {
                name.push(high * 16 + low);
                position += 3;
            } else {
                name.push(bytes[position]);
                position += 1;
            }
            if name.len() > 32 {
                while position < bytes.len() && !delimiter(bytes[position]) {
                    position += 1;
                }
                break;
            }
        }
        if matches!(name.as_slice(), b"ObjStm" | b"XRef" | b"XRefStm") {
            return true;
        }
    }
    false
}
fn wav_metadata(bytes: &[u8]) -> Result<ResourceMetadata, ErrorDescription> {
    let declared = u32::from_le_bytes(
        bytes
            .get(4..8)
            .ok_or(ErrorDescription::new(
                UiMessage::error_api_media_wav_header_incomplete(),
            ))?
            .try_into()
            .map_err(|error| {
                ErrorDescription::new(UiMessage::error_api_media_wav_header_incomplete())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })?,
    ) as usize;
    if declared.checked_add(8) != Some(bytes.len()) {
        return Err(ErrorDescription::new(UiMessage::error_api_media_wav_length_mismatch()).into());
    }
    let mut position = 12usize;
    let mut format = None;
    let mut data = None;
    while position < bytes.len() {
        let header = bytes
            .get(position..position + 8)
            .ok_or(ErrorDescription::new(
                UiMessage::error_api_media_wav_chunk_header_incomplete(),
            ))?;
        let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        let start = position.checked_add(8).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_wav_chunk_length_overflow(),
        ))?;
        let end = start.checked_add(length).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_wav_chunk_length_overflow(),
        ))?;
        let block = bytes.get(start..end).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_wav_chunk_outside_bytes(),
        ))?;
        match &header[..4] {
            b"fmt " => {
                if format.replace(block).is_some() {
                    return Err(ErrorDescription::new(
                        UiMessage::error_api_media_wav_format_ambiguous(),
                    )
                    .into());
                }
            }
            b"data" if data.replace(block).is_some() => {
                return Err(
                    ErrorDescription::new(UiMessage::error_api_media_wav_data_ambiguous()).into(),
                );
            }
            _ => {}
        }
        position = end.checked_add(length % 2).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_wav_chunk_length_overflow(),
        ))?;
    }
    let format = format.ok_or(ErrorDescription::new(
        UiMessage::error_api_media_wav_format_missing(),
    ))?;
    let data = data.ok_or(ErrorDescription::new(
        UiMessage::error_api_media_wav_data_missing(),
    ))?;
    if format.len() < 16 {
        return Err(
            ErrorDescription::new(UiMessage::error_api_media_wav_format_incomplete()).into(),
        );
    }
    let u16_at = |offset| u16::from_le_bytes(format[offset..offset + 2].try_into().unwrap());
    let u32_at = |offset| u32::from_le_bytes(format[offset..offset + 4].try_into().unwrap());
    let codec = u16_at(0);
    let channels = u16_at(2);
    let rate = u32_at(4);
    let byte_rate = u32_at(8);
    let align = u16_at(12);
    let bits = u16_at(14);
    if !matches!(codec, 1 | 3)
        || channels == 0
        || rate == 0
        || bits == 0
        || (codec == 3 && !matches!(bits, 32 | 64))
        || bits % 8 != 0
        || align == 0
        || u32::from(align) != u32::from(channels) * u32::from(bits / 8)
        || byte_rate
            != rate
                .checked_mul(u32::from(align))
                .ok_or(ErrorDescription::new(
                    UiMessage::error_api_media_wav_timebase_overflow(),
                ))?
        || data.len() % usize::from(align) != 0
    {
        return Err(
            ErrorDescription::new(UiMessage::error_api_media_wav_sampling_unsupported()).into(),
        );
    }
    let duration_tick = (data.len() / usize::from(align)) as u64;
    if duration_tick == 0 {
        return Err(ErrorDescription::new(UiMessage::error_api_media_wav_audio_empty()).into());
    }
    Ok(ResourceMetadata::Audio {
        timebase: Timebase {
            numerator: u64::from(rate),
            denominator: 1,
        },
        duration_tick,
    })
}

#[derive(Clone, Copy)]
struct Mp4Box<'a> {
    kind: [u8; 4],
    body: &'a [u8],
}
fn boxes(mut bytes: &[u8]) -> Result<Vec<Mp4Box<'_>>, ErrorDescription> {
    let mut output = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 8 || output.len() > 10000 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_chunk_header_invalid(),
            )
            .into());
        }
        let small = u32::from_be_bytes(bytes[..4].try_into().unwrap());
        let kind = bytes[4..8].try_into().unwrap();
        let (size, header) = match small {
            0 => (bytes.len(), 8),
            1 => {
                if bytes.len() < 16 {
                    return Err(ErrorDescription::new(
                        UiMessage::error_api_media_mp4_extended_header_incomplete(),
                    )
                    .into());
                }
                (
                    usize::try_from(u64::from_be_bytes(bytes[8..16].try_into().unwrap())).map_err(
                        |error| {
                            ErrorDescription::new(
                                UiMessage::error_api_media_mp4_length_out_of_range(),
                            )
                            .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
                        },
                    )?,
                    16,
                )
            }
            n => (n as usize, 8),
        };
        if size < header || size > bytes.len() {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_mp4_chunk_outside_bytes()).into(),
            );
        }
        output.push(Mp4Box {
            kind,
            body: &bytes[header..size],
        });
        bytes = &bytes[size..];
    }
    Ok(output)
}
fn one<'a>(input: &[Mp4Box<'a>], kind: &[u8; 4]) -> Result<Mp4Box<'a>, ErrorDescription> {
    let found: Vec<_> = input
        .iter()
        .filter(|item| &item.kind == kind)
        .copied()
        .collect();
    if let [item] = found.as_slice() {
        Ok(*item)
    } else {
        Err(ErrorDescription::new(
            UiMessage::error_api_media_mp4_box_not_unique(
                weftext_messages::params::ErrorApiMediaMp4BoxNotUnique {
                    box_kind: String::from_utf8_lossy(kind).into_owned(),
                },
            ),
        ))
    }
}
fn be32(input: &[u8], offset: usize) -> Result<u32, ErrorDescription> {
    Ok(u32::from_be_bytes(
        input
            .get(offset..offset + 4)
            .ok_or(ErrorDescription::new(
                UiMessage::error_api_media_mp4_field_incomplete(),
            ))?
            .try_into()
            .unwrap(),
    ))
}
fn be64(input: &[u8], offset: usize) -> Result<u64, ErrorDescription> {
    Ok(u64::from_be_bytes(
        input
            .get(offset..offset + 8)
            .ok_or(ErrorDescription::new(
                UiMessage::error_api_media_mp4_field_incomplete(),
            ))?
            .try_into()
            .unwrap(),
    ))
}
fn verify_mp4_samples(
    bytes: &[u8],
    root: &[Mp4Box<'_>],
    stbl: &[Mp4Box<'_>],
    samples: u64,
) -> Result<(), ErrorDescription> {
    if samples > 1_000_000 {
        return Err(ErrorDescription::new(UiMessage::error_api_media_mp4_sample_budget()).into());
    }
    if stbl.iter().any(|item| item.kind == *b"ctts") {
        let ctts = one(stbl, b"ctts")?;
        let count = be32(ctts.body, 4)? as usize;
        if ctts.body.len() != 8 + count * 8
            || ctts.body[8..]
                .as_chunks::<8>()
                .0
                .iter()
                .any(|entry| entry[4..] != [0, 0, 0, 0])
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_composition_offset_unsupported(),
            )
            .into());
        }
    }
    let size = one(stbl, b"stsz")?;
    let fixed = be32(size.body, 4)?;
    let count = be32(size.body, 8)? as usize;
    if (fixed == 0 && size.body.len() != 12 + count * 4) || (fixed != 0 && size.body.len() != 12) {
        return Err(ErrorDescription::new(
            UiMessage::error_api_media_mp4_sample_size_table_invalid(),
        )
        .into());
    }
    let offsets: Vec<u64> = if let Ok(stco) = one(stbl, b"stco") {
        if stbl.iter().any(|item| item.kind == *b"co64") {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_chunk_offsets_ambiguous(),
            )
            .into());
        }
        let count = be32(stco.body, 4)? as usize;
        if count > 1_000_000 || stco.body.len() != 8 + count * 4 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_chunk_offsets_invalid(),
            )
            .into());
        }
        stco.body[8..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|entry| u64::from(u32::from_be_bytes(*entry)))
            .collect()
    } else {
        let co64 = one(stbl, b"co64")?;
        let count = be32(co64.body, 4)? as usize;
        if count > 1_000_000 || co64.body.len() != 8 + count * 8 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_extended_offsets_invalid(),
            )
            .into());
        }
        co64.body[8..]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|entry| u64::from_be_bytes(*entry))
            .collect()
    };
    let map = one(stbl, b"stsc")?;
    let entries = be32(map.body, 4)? as usize;
    if entries == 0 || entries > offsets.len() || map.body.len() != 8 + entries * 12 {
        return Err(
            ErrorDescription::new(UiMessage::error_api_media_mp4_sample_chunks_invalid()).into(),
        );
    }
    let mut mappings = Vec::new();
    let mut prior = 0u32;
    for entry in map.body[8..].as_chunks::<12>().0 {
        let first = be32(entry, 0)?;
        let per_chunk = be32(entry, 4)?;
        if first <= prior
            || first as usize > offsets.len()
            || per_chunk == 0
            || be32(entry, 8)? != 1
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_sample_chunks_discontinuous(),
            )
            .into());
        }
        mappings.push((first as usize, per_chunk));
        prior = first;
    }
    if mappings[0].0 != 1 {
        return Err(
            ErrorDescription::new(UiMessage::error_api_media_mp4_first_chunk_missing()).into(),
        );
    }
    let media: Vec<_> = root
        .iter()
        .filter(|item| item.kind == *b"mdat")
        .map(|item| {
            let start = (item.body.as_ptr() as usize - bytes.as_ptr() as usize) as u64;
            (start, start + item.body.len() as u64)
        })
        .collect();
    let mut sample = 0usize;
    let mut mapping = 0usize;
    for (index, offset) in offsets.iter().enumerate() {
        while mappings
            .get(mapping + 1)
            .is_some_and(|next| next.0 <= index + 1)
        {
            mapping += 1;
        }
        let mut total = 0u64;
        for _ in 0..mappings[mapping].1 {
            if sample >= count {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_chunks_exceed_samples(),
                )
                .into());
            }
            let length = if fixed == 0 {
                be32(size.body, 12 + sample * 4)?
            } else {
                fixed
            };
            if length == 0 {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_media_sample_empty(),
                )
                .into());
            }
            total = total
                .checked_add(u64::from(length))
                .ok_or(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_sample_range_overflow(),
                ))?;
            sample += 1;
        }
        let end = offset.checked_add(total).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_mp4_sample_range_overflow(),
        ))?;
        if !media
            .iter()
            .any(|(start, limit)| offset >= start && end <= *limit)
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_samples_outside_media(),
            )
            .into());
        }
    }
    if sample != count || sample as u64 != samples {
        return Err(ErrorDescription::new(
            UiMessage::error_api_media_mp4_sample_timeline_incomplete(),
        )
        .into());
    }
    Ok(())
}
fn mp4_metadata(bytes: &[u8]) -> Result<ResourceMetadata, ErrorDescription> {
    let root = boxes(bytes)?;
    one(&root, b"ftyp")?;
    let moov = boxes(one(&root, b"moov")?.body)?;
    if root.iter().any(|item| item.kind == *b"moof")
        || moov.iter().any(|item| item.kind == *b"mvex")
    {
        return Err(
            ErrorDescription::new(UiMessage::error_api_media_mp4_fragmented_unsupported()).into(),
        );
    }
    if !root
        .iter()
        .any(|item| item.kind == *b"mdat" && !item.body.is_empty())
    {
        return Err(
            ErrorDescription::new(UiMessage::error_api_media_mp4_media_data_missing()).into(),
        );
    }
    let mut audio = None;
    let mut video = None;
    for trak in moov.iter().filter(|item| item.kind == *b"trak") {
        let track = boxes(trak.body)?;
        // Edit lists translate the media timeline to the browser's presentation
        // timeline. They are rejected until that mapping is represented exactly.
        if track.iter().any(|item| item.kind == *b"edts") {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_edited_timeline_unsupported(),
            )
            .into());
        }
        let mdia = boxes(one(&track, b"mdia")?.body)?;
        let handler = one(&mdia, b"hdlr")?;
        let kind = handler.body.get(8..12).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_mp4_track_type_incomplete(),
        ))?;
        if !matches!(kind, b"vide" | b"soun") {
            continue;
        }
        let mdhd = one(&mdia, b"mdhd")?;
        let (rate, duration) = match mdhd.body.first() {
            Some(0) => (be32(mdhd.body, 12)?, u64::from(be32(mdhd.body, 16)?)),
            Some(1) => (be32(mdhd.body, 20)?, be64(mdhd.body, 24)?),
            _ => {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_time_header_version_invalid(),
                )
                .into());
            }
        };
        if rate == 0 || duration == 0 || duration > 9_007_199_254_740_991 {
            return Err(
                ErrorDescription::new(UiMessage::error_api_media_mp4_time_range_invalid()).into(),
            );
        }
        let minf = boxes(one(&mdia, b"minf")?.body)?;
        let dinf = boxes(one(&minf, b"dinf")?.body)?;
        let dref = one(&dinf, b"dref")?;
        if be32(dref.body, 4)? != 1 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_external_sources_unsupported(),
            )
            .into());
        }
        let references = boxes(dref.body.get(8..).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_mp4_data_reference_invalid(),
        ))?)?;
        if references.len() != 1
            || references[0].kind != *b"url "
            || references[0].body != [0, 0, 0, 1]
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_media_must_be_internal(),
            )
            .into());
        }
        let stbl = boxes(one(&minf, b"stbl")?.body)?;
        let stts = one(&stbl, b"stts")?;
        let entries = be32(stts.body, 4)? as usize;
        if entries > 100000 || stts.body.len() != 8 + entries * 8 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_sample_timeline_invalid(),
            )
            .into());
        }
        let mut sample_count = 0u64;
        let mut sample_duration = 0u64;
        for entry in stts.body[8..].as_chunks::<8>().0 {
            let count = u64::from(be32(entry, 0)?);
            let delta = u64::from(be32(entry, 4)?);
            if count == 0 || delta == 0 {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_sample_time_invalid(),
                )
                .into());
            }
            sample_count = sample_count
                .checked_add(count)
                .ok_or(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_sample_count_overflow(),
                ))?;
            sample_duration = sample_duration
                .checked_add(count.checked_mul(delta).ok_or(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_sample_time_overflow(),
                ))?)
                .ok_or(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_sample_time_overflow(),
                ))?;
        }
        if sample_duration != duration || sample_count == 0 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_time_header_mismatch(),
            )
            .into());
        }
        let stsz = one(&stbl, b"stsz")?;
        if u64::from(be32(stsz.body, 8)?) != sample_count {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_sample_counts_mismatch(),
            )
            .into());
        }
        verify_mp4_samples(bytes, &root, &stbl, sample_count)?;
        let stsd = one(&stbl, b"stsd")?;
        if be32(stsd.body, 4)? != 1 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_multiple_formats_unsupported(),
            )
            .into());
        }
        let descriptions = boxes(stsd.body.get(8..).ok_or(ErrorDescription::new(
            UiMessage::error_api_media_mp4_sample_description_invalid(),
        ))?)?;
        if descriptions.len() != 1 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_sample_description_ambiguous(),
            )
            .into());
        }
        let description = descriptions[0];
        if description.body.get(6..8) != Some(&[0, 1]) {
            return Err(ErrorDescription::new(
                UiMessage::error_api_media_mp4_sample_reference_ambiguous(),
            )
            .into());
        }
        let timebase = Timebase {
            numerator: u64::from(rate),
            denominator: 1,
        };
        if kind == b"vide" {
            if !matches!(
                &description.kind,
                b"avc1" | b"avc3" | b"hvc1" | b"hev1" | b"vp09" | b"av01"
            ) || description.body.len() < 78
            {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_video_format_unsupported(),
                )
                .into());
            }
            let width = u64::from(u16::from_be_bytes(
                description.body[24..26].try_into().unwrap(),
            ));
            let height = u64::from(u16::from_be_bytes(
                description.body[26..28].try_into().unwrap(),
            ));
            if width == 0 || height == 0 {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_video_dimensions_empty(),
                )
                .into());
            }
            let tkhd = one(&track, b"tkhd")?;
            let matrix = match tkhd.body.first() {
                Some(0) => 40,
                Some(1) => 52,
                _ => {
                    return Err(ErrorDescription::new(
                        UiMessage::error_api_media_mp4_track_header_version_invalid(),
                    )
                    .into());
                }
            };
            let identity = [65536u32, 0, 0, 0, 65536, 0, 0, 0, 1073741824];
            for (index, expected) in identity.iter().enumerate() {
                if be32(tkhd.body, matrix + index * 4)? != *expected {
                    return Err(ErrorDescription::new(
                        UiMessage::error_api_media_mp4_video_transform_unsupported(),
                    )
                    .into());
                }
            }
            let child = boxes(&description.body[78..])?;
            if child.iter().any(|item| item.kind == *b"clap") {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_video_crop_unsupported(),
                )
                .into());
            }
            for aspect in child.iter().filter(|item| item.kind == *b"pasp") {
                if be32(aspect.body, 0)? != be32(aspect.body, 4)? {
                    return Err(ErrorDescription::new(
                        UiMessage::error_api_media_mp4_pixel_aspect_unsupported(),
                    )
                    .into());
                }
            }
            if video
                .replace(ResourceMetadata::Video {
                    timebase,
                    duration_tick: duration,
                    width,
                    height,
                })
                .is_some()
            {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_multiple_video_tracks_unsupported(),
                )
                .into());
            }
        } else {
            if description.kind != *b"mp4a" {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_audio_format_unsupported(),
                )
                .into());
            }
            if audio
                .replace(ResourceMetadata::Audio {
                    timebase,
                    duration_tick: duration,
                })
                .is_some()
            {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_media_mp4_multiple_audio_tracks_unsupported(),
                )
                .into());
            }
        }
    }
    video.or(audio).ok_or(
        ErrorDescription::new(UiMessage::error_api_media_mp4_supported_track_missing()).into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_media_keeps_product_validation_and_real_parser_diagnostics_separate() {
        let (metadata, content_type) = inspect_media(b"%PDF-invalid-owned-test");
        assert_eq!(content_type, "application/pdf");
        let ResourceMetadata::Unsupported { description } = metadata else {
            panic!("invalid PDF was admitted")
        };
        assert_eq!(
            description.explanation.key(),
            "error.api.media.pdfUnreadable"
        );
        assert_eq!(description.diagnostics.len(), 1);
        assert_eq!(
            description.diagnostics[0].origin,
            DiagnosticOrigin::Dependency
        );
        assert!(!description.diagnostics[0].text.is_empty());
        let wire = serde_json::to_value(ResourceMetadata::Unsupported { description }).unwrap();
        assert_eq!(wire["kind"], "unsupported");
        assert!(wire.get("message").is_none());
        assert!(wire.get("explanation").is_some());
        assert!(wire["diagnostics"].is_array());
        assert_eq!(wire["causes"], json!([]));

        let (metadata, _) = inspect_media(b"RIFF\0\0\0\0WAVE");
        let ResourceMetadata::Unsupported { description } = metadata else {
            panic!("invalid WAV was admitted")
        };
        assert_eq!(
            description.explanation.key(),
            "error.api.media.wavLengthMismatch"
        );
        assert!(description.diagnostics.is_empty());
    }
    fn wave() -> Vec<u8> {
        let data = vec![0u8; 960];
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36u32 + data.len() as u32).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(48000u32.to_le_bytes());
        bytes.extend(96000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((data.len() as u32).to_le_bytes());
        bytes.extend(data);
        bytes
    }
    #[test]
    fn wave_facts_come_from_actual_sample_bytes() {
        let bytes = wave();
        let metadata = wav_metadata(&bytes).unwrap();
        let ResourceMetadata::Audio {
            timebase,
            duration_tick,
        } = metadata
        else {
            panic!()
        };
        assert_eq!(timebase.numerator, 48000);
        assert_eq!(duration_tick, 480);
        let mut changed = bytes.clone();
        changed[28..32].copy_from_slice(&123u32.to_le_bytes());
        assert!(wav_metadata(&changed).is_err());
        assert!(wav_metadata(&bytes[..bytes.len() - 1]).is_err());
    }
    #[test]
    fn malformed_boxes_never_create_timing_facts() {
        assert!(boxes(&[0, 0, 0, 1, b'm', b'o', b'o', b'v']).is_err());
        assert!(boxes(&[0, 0, 0, 7, b'm', b'o', b'o', b'v']).is_err());
        assert!(mp4_metadata(b"not media").is_err());
    }
    fn mp4_box(kind: &[u8; 4], body: Vec<u8>) -> Vec<u8> {
        let mut output = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        output.extend(kind);
        output.extend(body);
        output
    }
    fn video_container() -> Vec<u8> {
        let mut mdhd = vec![0u8; 24];
        mdhd[12..16].copy_from_slice(&30000u32.to_be_bytes());
        mdhd[16..20].copy_from_slice(&90000u32.to_be_bytes());
        let mut hdlr = vec![0u8; 12];
        hdlr[8..12].copy_from_slice(b"vide");
        let mut tkhd = vec![0u8; 84];
        for (index, value) in [65536u32, 0, 0, 0, 65536, 0, 0, 0, 1073741824]
            .iter()
            .enumerate()
        {
            tkhd[40 + index * 4..44 + index * 4].copy_from_slice(&value.to_be_bytes());
        }
        let mut stts = vec![0u8; 4];
        stts.extend(1u32.to_be_bytes());
        stts.extend(90u32.to_be_bytes());
        stts.extend(1000u32.to_be_bytes());
        let mut stsz = vec![0u8; 4];
        stsz.extend(1u32.to_be_bytes());
        stsz.extend(90u32.to_be_bytes());
        let mut sample = vec![0u8; 78];
        sample[6..8].copy_from_slice(&1u16.to_be_bytes());
        sample[24..26].copy_from_slice(&320u16.to_be_bytes());
        sample[26..28].copy_from_slice(&180u16.to_be_bytes());
        let mut stsd = vec![0u8; 4];
        stsd.extend(1u32.to_be_bytes());
        stsd.extend(mp4_box(b"avc1", sample));
        let mut stsc = vec![0u8; 4];
        stsc.extend(1u32.to_be_bytes());
        stsc.extend(1u32.to_be_bytes());
        stsc.extend(90u32.to_be_bytes());
        stsc.extend(1u32.to_be_bytes());
        let mut stco = vec![0u8; 4];
        stco.extend(1u32.to_be_bytes());
        stco.extend(0u32.to_be_bytes());
        let stbl = [
            mp4_box(b"stts", stts),
            mp4_box(b"stsz", stsz),
            mp4_box(b"stsd", stsd),
            mp4_box(b"stsc", stsc),
            mp4_box(b"stco", stco),
        ]
        .concat();
        let mut dref = vec![0u8; 4];
        dref.extend(1u32.to_be_bytes());
        dref.extend(mp4_box(b"url ", vec![0, 0, 0, 1]));
        let minf = [
            mp4_box(b"dinf", mp4_box(b"dref", dref)),
            mp4_box(b"stbl", stbl),
        ]
        .concat();
        let mdia = [
            mp4_box(b"mdhd", mdhd),
            mp4_box(b"hdlr", hdlr),
            mp4_box(b"minf", minf),
        ]
        .concat();
        let trak = [mp4_box(b"tkhd", tkhd), mp4_box(b"mdia", mdia)].concat();
        let mut bytes = [
            mp4_box(b"ftyp", b"isom0000".to_vec()),
            mp4_box(b"moov", mp4_box(b"trak", trak)),
            mp4_box(b"mdat", vec![1u8; 90]),
        ]
        .concat();
        let offset = (bytes.windows(4).position(|slice| slice == b"mdat").unwrap() + 4) as u32;
        let stco = bytes.windows(4).position(|slice| slice == b"stco").unwrap();
        bytes[stco + 12..stco + 16].copy_from_slice(&offset.to_be_bytes());
        bytes
    }
    #[test]
    fn mp4_descriptor_uses_matching_track_and_sample_times() {
        // This bounded container fixture verifies metadata only. The viewer
        // additionally requires native decoding before enabling a selection.
        let bytes = video_container();
        let ResourceMetadata::Video {
            timebase,
            duration_tick,
            width,
            height,
        } = mp4_metadata(&bytes).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (timebase.numerator, duration_tick, width, height),
            (30000, 90000, 320, 180)
        );
        let mut changed = bytes.clone();
        let position = changed
            .windows(4)
            .position(|slice| slice == b"stts")
            .unwrap();
        changed[position + 16..position + 20].copy_from_slice(&999u32.to_be_bytes());
        assert!(mp4_metadata(&changed).is_err());
        let mut changed = bytes.clone();
        let stco = changed
            .windows(4)
            .position(|slice| slice == b"stco")
            .unwrap();
        changed[stco + 12..stco + 16].copy_from_slice(&1u32.to_be_bytes());
        assert!(
            mp4_metadata(&changed).is_err(),
            "metadata cannot certify samples outside actual mdat bytes"
        );
    }
    #[test]
    fn real_h264_video_fixture_has_exact_native_track_metadata() {
        let bytes = include_bytes!("../tests/fixtures/media-blue.mp4");
        let ResourceMetadata::Video {
            timebase,
            duration_tick,
            width,
            height,
        } = mp4_metadata(bytes).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (
                timebase.numerator,
                timebase.denominator,
                duration_tick,
                width,
                height
            ),
            (15360, 1, 15360, 320, 180)
        );
    }
    #[test]
    fn image_requires_complete_decoding() {
        let mut bytes = Cursor::new(Vec::new());
        let image = image::DynamicImage::new_rgb8(3, 2);
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let bytes = bytes.into_inner();
        let (metadata, mime) = inspect_media(&bytes);
        assert!(matches!(
            metadata,
            ResourceMetadata::Image {
                width: 3,
                height: 2
            }
        ));
        assert_eq!(mime, "image/png");
        assert!(matches!(
            inspect_media(&bytes[..30]).0,
            ResourceMetadata::Unsupported { .. }
        ));
    }
    #[test]
    fn pdf_page_facts_are_real_and_unrotated() {
        use lopdf::{Object, dictionary};
        let mut document = lopdf::Document::with_version("1.5");
        document.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
        let pages = document.new_object_id();
        let page=document.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![Object::Integer(10),Object::Integer(20),Object::Integer(310),Object::Integer(420)],"Rotate"=>90});
        document.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! {"Type"=>"Pages","Kids"=>vec![Object::Reference(page)],"Count"=>1},
            ),
        );
        let catalog = document.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
        document.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        let ResourceMetadata::Pdf { pages } = pdf_metadata(&bytes).unwrap() else {
            panic!()
        };
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].width, 300.0);
        assert_eq!(pages[0].height, 400.0);
        assert_eq!(pages[0].rotation, 90);
    }
    #[test]
    fn compressed_pdf_structure_is_refused_before_unbounded_decoder() {
        assert!(pdf_structural_stream(b"/Type /ObjStm"));
        assert!(pdf_structural_stream(b"/Type /Obj#53tm"));
        assert!(pdf_structural_stream(b"/Type /XRef"));
        assert!(!pdf_structural_stream(b"/Filter /FlateDecode /Type /Page"));
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn owned_resource_delivery_requires_real_admission_and_exact_version() {
        use std::sync::{Arc, Mutex};
        let root = std::env::temp_dir().join(format!("weftext-media-api-{}", uuid::Uuid::new_v4()));
        let mut core = weftext_core::Core::new(root.join("private")).unwrap();
        let library = root.join("library");
        core.create_library(&library).unwrap();
        let opened = core
            .create_note_fixture(&library, &library, "媒体")
            .unwrap();
        let bytes = wave();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let preview = core
            .prepare_import_resource(
                &library,
                &opened.path,
                "sound.wav",
                bytes.clone(),
                &operation_id,
            )
            .unwrap();
        assert!(
            core.managed_resource_snapshot(&library, &opened.path)
                .unwrap()
                .is_empty()
        );
        core.execute_operation(&operation_id, &preview.plan_id)
            .unwrap();
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("test-token"),
            port: 0,
            locations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            renders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            editor_versions: Arc::new(Mutex::new(std::collections::HashMap::new())),
            exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        super::super::remember(
            &state,
            &json!({"sessionId":opened.session_id,"path":opened.path,"root":opened.root}),
        )
        .ok()
        .unwrap();
        let mut set = resource_set(&state, &opened.session_id)
            .await
            .map_err(ApiError::into_description)
            .unwrap();
        assert_eq!(set.resources.len(), 1);
        let selected = set.resources.remove(0);
        let revision = selected.file.revision.clone().unwrap();
        assert!(uuid::Uuid::parse_str(&revision).is_ok());
        assert!(selected.facts.is_some());
        assert_eq!(selected.file.bytes.as_ref(), Some(&bytes));
        assert!(
            qualified_resource(&state, &opened.session_id, "D:/unowned.wav", &revision)
                .await
                .is_err()
        );
        let response = read(
            State(state.clone()),
            crate::RequestJson(ReadRequest {
                session_id: opened.session_id.clone(),
                resource: selected.resource_ref.clone(),
                revision: revision.clone(),
            }),
        )
        .await
        .map_err(ApiError::into_description)
        .unwrap();
        let delivered = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(delivered.as_ref(), bytes.as_slice());
        std::fs::write(
            &selected.file.path,
            wave().into_iter().chain([1]).collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(
            qualified_resource(
                &state,
                &opened.session_id,
                &selected.resource_ref,
                &revision
            )
            .await
            .is_err(),
            "same-path external bytes cannot inherit an accepted version"
        );
        println!("Retained disposable media API fixture: {}", root.display());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn query_thumbnail_does_not_open_a_note_and_rejects_stale_row_source() {
        use std::sync::{Arc, Mutex};
        use weftext_core::{FileChange, OperationIntent, OperationRequest};
        let root =
            std::env::temp_dir().join(format!("weftext-query-preview-{}", uuid::Uuid::new_v4()));
        let mut core = weftext_core::Core::new(root.join("private")).unwrap();
        let library = root.join("library");
        core.create_library(&library).unwrap();
        let opened = core
            .create_note_fixture(&library, &library, "缩略图")
            .unwrap();
        let mut image = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(3, 2)
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        let bytes = image.into_inner();
        let operation = uuid::Uuid::new_v4().to_string();
        let plan = core
            .prepare_import_resource(
                &library,
                &opened.path,
                "preview.png",
                bytes.clone(),
                &operation,
            )
            .unwrap();
        core.execute_operation(&operation, &plan.plan_id).unwrap();
        let before = core
            .read_snapshot(&library, std::slice::from_ref(&opened.path))
            .unwrap()
            .files
            .remove(0);
        let mut after = before.bytes.clone().unwrap();
        after.extend(b"\nPreview source\n");
        let operation = uuid::Uuid::new_v4().to_string();
        let plan = core
            .prepare_operation(OperationRequest {
                operation_id: operation.clone(),
                root: library.to_string_lossy().into_owned(),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: opened.path.clone(),
                        base_id: Some(before.base_id),
                        before: before.bytes,
                        after,
                    }],
                },
            })
            .unwrap();
        core.execute_operation(&operation, &plan.plan_id).unwrap();
        let revision = core
            .read_snapshot(&library, std::slice::from_ref(&opened.path))
            .unwrap()
            .files
            .remove(0)
            .revision
            .unwrap();
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("test-token"),
            port: 0,
            locations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            renders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            editor_versions: Arc::new(Mutex::new(std::collections::HashMap::new())),
            exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        let request = QueryPreviewRequest {
            root: library.to_string_lossy().into_owned(),
            entity_key: format!(
                "weftext-node://{}/{}",
                opened.workspace_id.unwrap(),
                opened.node_id.unwrap()
            ),
            revision,
        };
        let ready = query_preview(State(state.clone()), crate::RequestJson(request.clone()))
            .await
            .map_err(ApiError::into_description)
            .unwrap()
            .0;
        assert_eq!(ready["state"], "ready");
        assert_eq!(ready["previewBasis"], "currentResourceAtRowSource");
        assert!(
            state.locations.lock().unwrap().is_empty(),
            "thumbnail does not create or select an editor session"
        );
        let raw = QueryPreviewResourceRequest {
            root: request.root.clone(),
            entity_key: request.entity_key.clone(),
            revision: request.revision.clone(),
            resource: ready["descriptor"]["resource"].as_str().unwrap().into(),
            resource_revision: ready["descriptor"]["revision"].as_str().unwrap().into(),
        };
        let response = query_preview_resource(State(state.clone()), crate::RequestJson(raw))
            .await
            .map_err(ApiError::into_description)
            .unwrap();
        let delivered = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(delivered.as_ref(), bytes.as_slice());
        let stale = query_preview(
            State(state.clone()),
            crate::RequestJson(QueryPreviewRequest {
                revision: uuid::Uuid::new_v4().to_string(),
                ..request.clone()
            }),
        )
        .await
        .map_err(ApiError::into_description)
        .unwrap()
        .0;
        assert_eq!(stale["state"], "unavailable");
        assert!(stale.get("descriptor").is_none());
        std::fs::write(
            &opened.path,
            format!("{}\nExternal observed content\n", opened.source),
        )
        .unwrap();
        let observed = state
            .core
            .lock()
            .unwrap()
            .read_snapshot(&library, std::slice::from_ref(&opened.path))
            .unwrap()
            .files
            .remove(0);
        assert!(observed.revision.is_none());
        let observed_request = QueryPreviewRequest {
            revision: format!("observed-source:{}", observed.base_id),
            ..request
        };
        let ready = query_preview(
            State(state.clone()),
            crate::RequestJson(observed_request.clone()),
        )
        .await
        .map_err(ApiError::into_description)
        .unwrap()
        .0;
        assert_eq!(ready["state"], "ready");
        assert_eq!(ready["sourceBasisKind"], "observed");
        assert!(ready["sourceRevision"].is_null());
        assert_eq!(ready["sourceBaseId"], observed.base_id);
        std::fs::write(
            &opened.path,
            format!("{}\nAnother external draft\n", opened.source),
        )
        .unwrap();
        let stale = query_preview(State(state.clone()), crate::RequestJson(observed_request))
            .await
            .map_err(ApiError::into_description)
            .unwrap()
            .0;
        assert_eq!(stale["state"], "unavailable");
        assert!(stale.get("descriptor").is_none());
        println!(
            "Retained disposable query preview fixture: {}",
            root.display()
        );
    }
}
