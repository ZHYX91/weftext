//! Resource reads use the Core's owned manifest and exact current snapshot.
//! Container metadata is derived from the same bytes that the viewer receives.
use super::{ApiError, ApiResult, AppState, core_call, location, value};
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
        message: String,
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
    pub warnings: Vec<String>,
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
    tokio::task::spawn_blocking(move || {
        let mut resources = Vec::new();
        let mut warnings = Vec::new();
        for resource in managed {
            let revision = resource.revision.clone();
            let (metadata, content_type) = inspect_media(&resource.bytes);
            let facts = revision
                .as_ref()
                .and_then(|revision| metadata_facts(&metadata, &resource.resource_ref, revision));
            if resource.identity_status != "admitted" {
                warnings.push(format!(
                    "{}：资源身份或版本尚未接纳，只能重新读取后明确处理。",
                    resource.path
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
    .map_err(|_| ApiError::internal("资源元数据读取未完成；原件保留"))
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
                "此资源未在当前笔记的获权清单中登记；标注草稿已保留",
            )
        })?;
    let selected = set.resources.remove(position);
    if revision.is_empty() || selected.file.revision.as_deref() != Some(revision) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "resourceVersion",
            "资源对象或准确版本已变化，请重新打开资源；标注草稿已保留",
        ));
    }
    Ok(selected)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    session_id: String,
}
async fn list(State(state): State<AppState>, Json(request): Json<SessionRequest>) -> ApiResult {
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
    Json(request): Json<NavigationPreviewRequest>,
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
                "笔记内容或身份已变化，请刷新当前列表；未采用新版缩略图",
            ));
        }
        Ok(QueryPreviewRequest {
            root: request.root,
            entity_key: request.entity_key,
            revision: format!("observed-source:{}", file.base_id),
        })
    })
    .await?;
    query_preview(State(state), Json(proof)).await
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
        .ok_or_else(|| CoreError::new("invalidPath", "Missing preview resource owner"))?
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
            "The actual source object or revision differs from the frozen query row",
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
            "The resource manifest changed while reading the row preview",
        ));
    }
    for (resource, file) in managed.iter_mut().zip(&cut.files[2..]) {
        if file.bytes.as_ref() != Some(&resource.bytes)
            || file.revision != resource.revision
            || manifest.bytes.as_ref() != Some(&resource.manifest_bytes)
        {
            return Err(CoreError::new(
                "scopeChanged",
                "The resource object or manifest changed while reading the row preview",
            ));
        }
        resource.base_id = file.base_id.clone();
        resource.manifest_base_id = manifest.base_id.clone();
    }
    Ok(managed)
}
async fn query_preview(
    State(state): State<AppState>,
    Json(request): Json<QueryPreviewRequest>,
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
                json!({"state":"unavailable","sourceRevision":source_revision,"sourceBaseId":source_base,"sourceBasisKind":source_basis_kind,"message":error.message}),
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
        json!({"state":if set.resources.is_empty(){"absent"}else{"unsupported"},"sourceRevision":source_revision,"sourceBaseId":source_base,"sourceBasisKind":source_basis_kind,"message":"当前资源清单没有可验证的图片或 PDF 缩略图。"}),
    ))
}
async fn query_preview_resource(
    State(state): State<AppState>,
    Json(request): Json<QueryPreviewResourceRequest>,
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
                "缩略图资源版本已变化，请刷新当前列表",
            )
        })?;
    let resource = set.resources.remove(index);
    let bytes = resource
        .file
        .bytes
        .ok_or_else(|| ApiError::bad("缩略图资源字节不可用"))?;
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
    Json(request): Json<ReadRequest>,
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
        .ok_or_else(|| ApiError::bad("资源字节不可用"))?;
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
    Json(request): Json<ImportRequest>,
) -> ApiResult {
    // The authenticated browser explicitly chooses/uploads this input. No client
    // path is read and no API code writes a filesystem object.
    if request.bytes.len() > 86 * 1024 * 1024 {
        return Err(ApiError::bad("导入资源超过 64 MiB 上限"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.bytes)
        .map_err(|_| ApiError::bad("资源上传不是有效 Base64"))?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(ApiError::bad("导入资源超过 64 MiB 上限"));
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
fn unsupported(message: impl Into<String>, mime: &str) -> (ResourceMetadata, String) {
    (
        ResourceMetadata::Unsupported {
            message: message.into(),
        },
        mime.into(),
    )
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
            _ => return unsupported("当前图片格式尚无准确区域读取器", "application/octet-stream"),
        };
        let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let result = (|| {
            let mut decoder = reader
                .into_decoder()
                .map_err(|_| "图片无法完整解码或超出解码上限")?;
            if decoder.orientation().map_err(|_| "图片方向元数据无效")?
                != image::metadata::Orientation::NoTransforms
            {
                return Err("带方向变换的图片暂不能创建原始像素区域；原件保留");
            }
            let image =
                image::DynamicImage::from_decoder(decoder).map_err(|_| "图片字节无法完整解码")?;
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
        "此资源可保留；当前没有验证该格式的准确页面、像素或时基读取器",
        "application/octet-stream",
    )
}
pub(crate) fn metadata_from_bytes(bytes: &[u8]) -> ResourceMetadata {
    inspect_media(bytes).0
}

fn pdf_metadata(bytes: &[u8]) -> Result<ResourceMetadata, String> {
    // This library's structural stream decompression is not output-bounded.
    // Refuse it before loading instead of claiming an unqualified parser sandbox.
    if bytes.len() > 16 * 1024 * 1024 || pdf_structural_stream(bytes) {
        return Err("当前 PDF 页面读取器仅支持 16 MiB 内的普通对象与 xref 表；压缩对象/xref 流原件保留，区域暂不可写".into());
    }
    let document =
        lopdf::Document::load_mem(bytes).map_err(|_| "PDF 无法读取；原件保留".to_owned())?;
    if document.is_encrypted() {
        return Err("加密 PDF 未解锁，不能建立准确页面区域".into());
    }
    let pages = document.get_pages();
    if pages.is_empty() || pages.len() > 2000 {
        return Err("PDF 页面为空或超过 2000 页上限".into());
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
                return Err("PDF 页面继承链循环或超出上限".into());
            }
            let dictionary = document
                .get_dictionary(cursor)
                .map_err(|_| "PDF 页面字典无效")?;
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
        let resolve = |object: lopdf::Object| -> Result<lopdf::Object, String> {
            match object {
                lopdf::Object::Reference(reference) => document
                    .get_object(reference)
                    .cloned()
                    .map_err(|_| "PDF 页面属性引用无效".into()),
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
        let media = resolve(media.ok_or("PDF 页面缺少 MediaBox")?)?;
        let array = media.as_array().map_err(|_| "PDF MediaBox 不是准确矩形")?;
        if array.len() != 4 {
            return Err("PDF 页面矩形无效".into());
        }
        let values: Option<Vec<_>> = array.iter().map(numeric).collect();
        let values = values.ok_or("PDF 页面矩形不是数值")?;
        if values.iter().any(|n| !n.is_finite()) || values[2] <= values[0] || values[3] <= values[1]
        {
            return Err("PDF 页面尺寸无效".into());
        }
        if let Some(crop) = crop
            && resolve(crop)? != media
        {
            return Err("带裁切页面的 PDF 暂不能创建未旋转页面区域；原件保留".into());
        }
        if let Some(unit) = unit
            && numeric(&resolve(unit)?) != Some(1.0)
        {
            return Err("PDF 非标准 UserUnit 暂不支持准确点坐标".into());
        }
        let rotation = rotation
            .map(resolve)
            .transpose()?
            .as_ref()
            .and_then(numeric)
            .unwrap_or(0.0);
        if rotation.fract() != 0.0 {
            return Err("PDF 页面旋转无效".into());
        }
        let rotation = (rotation as i32).rem_euclid(360);
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err("PDF 页面旋转无效".into());
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
fn wav_metadata(bytes: &[u8]) -> Result<ResourceMetadata, String> {
    let declared = u32::from_le_bytes(
        bytes
            .get(4..8)
            .ok_or("WAV 头不完整")?
            .try_into()
            .map_err(|_| "WAV 头不完整")?,
    ) as usize;
    if declared.checked_add(8) != Some(bytes.len()) {
        return Err("WAV 容器长度与实际字节不一致".into());
    }
    let mut position = 12usize;
    let mut format = None;
    let mut data = None;
    while position < bytes.len() {
        let header = bytes.get(position..position + 8).ok_or("WAV 块头不完整")?;
        let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        let start = position.checked_add(8).ok_or("WAV 块长度溢出")?;
        let end = start.checked_add(length).ok_or("WAV 块长度溢出")?;
        let block = bytes.get(start..end).ok_or("WAV 数据块超出真实字节")?;
        match &header[..4] {
            b"fmt " => {
                if format.replace(block).is_some() {
                    return Err("WAV 存在多个格式块".into());
                }
            }
            b"data" if data.replace(block).is_some() => return Err("WAV 存在多个音频数据块".into()),
            _ => {}
        }
        position = end.checked_add(length % 2).ok_or("WAV 块长度溢出")?;
    }
    let format = format.ok_or("WAV 缺少格式块")?;
    let data = data.ok_or("WAV 缺少音频数据")?;
    if format.len() < 16 {
        return Err("WAV 格式块不完整".into());
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
        || byte_rate != rate.checked_mul(u32::from(align)).ok_or("WAV 时基溢出")?
        || data.len() % usize::from(align) != 0
    {
        return Err("WAV 不是受支持的完整 PCM/浮点采样，不能建立准确采样时基".into());
    }
    let duration_tick = (data.len() / usize::from(align)) as u64;
    if duration_tick == 0 {
        return Err("WAV 音频为空".into());
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
fn boxes(mut bytes: &[u8]) -> Result<Vec<Mp4Box<'_>>, String> {
    let mut output = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 8 || output.len() > 10000 {
            return Err("MP4 块头无效或超出上限".into());
        }
        let small = u32::from_be_bytes(bytes[..4].try_into().unwrap());
        let kind = bytes[4..8].try_into().unwrap();
        let (size, header) = match small {
            0 => (bytes.len(), 8),
            1 => {
                if bytes.len() < 16 {
                    return Err("MP4 长块头不完整".into());
                }
                (
                    usize::try_from(u64::from_be_bytes(bytes[8..16].try_into().unwrap()))
                        .map_err(|_| "MP4 长度超出范围")?,
                    16,
                )
            }
            n => (n as usize, 8),
        };
        if size < header || size > bytes.len() {
            return Err("MP4 块超出实际资源字节".into());
        }
        output.push(Mp4Box {
            kind,
            body: &bytes[header..size],
        });
        bytes = &bytes[size..];
    }
    Ok(output)
}
fn one<'a>(input: &[Mp4Box<'a>], kind: &[u8; 4]) -> Result<Mp4Box<'a>, String> {
    let found: Vec<_> = input
        .iter()
        .filter(|item| &item.kind == kind)
        .copied()
        .collect();
    if let [item] = found.as_slice() {
        Ok(*item)
    } else {
        Err(format!(
            "MP4 必须恰有一个 {} 块",
            String::from_utf8_lossy(kind)
        ))
    }
}
fn be32(input: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_be_bytes(
        input
            .get(offset..offset + 4)
            .ok_or("MP4 字段不完整")?
            .try_into()
            .unwrap(),
    ))
}
fn be64(input: &[u8], offset: usize) -> Result<u64, String> {
    Ok(u64::from_be_bytes(
        input
            .get(offset..offset + 8)
            .ok_or("MP4 字段不完整")?
            .try_into()
            .unwrap(),
    ))
}
fn verify_mp4_samples(
    bytes: &[u8],
    root: &[Mp4Box<'_>],
    stbl: &[Mp4Box<'_>],
    samples: u64,
) -> Result<(), String> {
    if samples > 1_000_000 {
        return Err("MP4 采样数超过当前验证上限".into());
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
            return Err("MP4 合成时间偏移暂不能映射到准确播放器区间".into());
        }
    }
    let size = one(stbl, b"stsz")?;
    let fixed = be32(size.body, 4)?;
    let count = be32(size.body, 8)? as usize;
    if (fixed == 0 && size.body.len() != 12 + count * 4) || (fixed != 0 && size.body.len() != 12) {
        return Err("MP4 采样大小表长度无效".into());
    }
    let offsets: Vec<u64> = if let Ok(stco) = one(stbl, b"stco") {
        if stbl.iter().any(|item| item.kind == *b"co64") {
            return Err("MP4 块偏移表不唯一".into());
        }
        let count = be32(stco.body, 4)? as usize;
        if count > 1_000_000 || stco.body.len() != 8 + count * 4 {
            return Err("MP4 块偏移表无效".into());
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
            return Err("MP4 长块偏移表无效".into());
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
        return Err("MP4 采样块映射无效".into());
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
            return Err("MP4 采样块映射不连续或采样描述不唯一".into());
        }
        mappings.push((first as usize, per_chunk));
        prior = first;
    }
    if mappings[0].0 != 1 {
        return Err("MP4 采样块映射缺少第一块".into());
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
                return Err("MP4 采样块超出实际采样数".into());
            }
            let length = if fixed == 0 {
                be32(size.body, 12 + sample * 4)?
            } else {
                fixed
            };
            if length == 0 {
                return Err("MP4 媒体采样为空".into());
            }
            total = total
                .checked_add(u64::from(length))
                .ok_or("MP4 采样字节范围溢出")?;
            sample += 1;
        }
        let end = offset.checked_add(total).ok_or("MP4 采样字节范围溢出")?;
        if !media
            .iter()
            .any(|(start, limit)| offset >= start && end <= *limit)
        {
            return Err("MP4 采样指向实际媒体数据范围之外".into());
        }
    }
    if sample != count || sample as u64 != samples {
        return Err("MP4 采样块未覆盖完整采样时序".into());
    }
    Ok(())
}
fn mp4_metadata(bytes: &[u8]) -> Result<ResourceMetadata, String> {
    let root = boxes(bytes)?;
    one(&root, b"ftyp")?;
    let moov = boxes(one(&root, b"moov")?.body)?;
    if root.iter().any(|item| item.kind == *b"moof")
        || moov.iter().any(|item| item.kind == *b"mvex")
    {
        return Err("分片 MP4 暂不支持准确区间标注".into());
    }
    if !root
        .iter()
        .any(|item| item.kind == *b"mdat" && !item.body.is_empty())
    {
        return Err("MP4 缺少实际媒体数据".into());
    }
    let mut audio = None;
    let mut video = None;
    for trak in moov.iter().filter(|item| item.kind == *b"trak") {
        let track = boxes(trak.body)?;
        // Edit lists translate the media timeline to the browser's presentation
        // timeline. They are rejected until that mapping is represented exactly.
        if track.iter().any(|item| item.kind == *b"edts") {
            return Err("带编辑时间线的 MP4 暂不能建立准确播放器区间；原件保留".into());
        }
        let mdia = boxes(one(&track, b"mdia")?.body)?;
        let handler = one(&mdia, b"hdlr")?;
        let kind = handler.body.get(8..12).ok_or("MP4 轨道类型不完整")?;
        if !matches!(kind, b"vide" | b"soun") {
            continue;
        }
        let mdhd = one(&mdia, b"mdhd")?;
        let (rate, duration) = match mdhd.body.first() {
            Some(0) => (be32(mdhd.body, 12)?, u64::from(be32(mdhd.body, 16)?)),
            Some(1) => (be32(mdhd.body, 20)?, be64(mdhd.body, 24)?),
            _ => return Err("MP4 时间头版本无效".into()),
        };
        if rate == 0 || duration == 0 || duration > 9_007_199_254_740_991 {
            return Err("MP4 时间范围无效或超出准确整数上限".into());
        }
        let minf = boxes(one(&mdia, b"minf")?.body)?;
        let dinf = boxes(one(&minf, b"dinf")?.body)?;
        let dref = one(&dinf, b"dref")?;
        if be32(dref.body, 4)? != 1 {
            return Err("MP4 外部或多源数据引用暂不支持准确区域".into());
        }
        let references = boxes(dref.body.get(8..).ok_or("MP4 数据引用无效")?)?;
        if references.len() != 1
            || references[0].kind != *b"url "
            || references[0].body != [0, 0, 0, 1]
        {
            return Err("MP4 媒体数据必须完整包含在准确资源字节中".into());
        }
        let stbl = boxes(one(&minf, b"stbl")?.body)?;
        let stts = one(&stbl, b"stts")?;
        let entries = be32(stts.body, 4)? as usize;
        if entries > 100000 || stts.body.len() != 8 + entries * 8 {
            return Err("MP4 采样时序表无效".into());
        }
        let mut sample_count = 0u64;
        let mut sample_duration = 0u64;
        for entry in stts.body[8..].as_chunks::<8>().0 {
            let count = u64::from(be32(entry, 0)?);
            let delta = u64::from(be32(entry, 4)?);
            if count == 0 || delta == 0 {
                return Err("MP4 采样时间无效".into());
            }
            sample_count = sample_count.checked_add(count).ok_or("MP4 采样数量溢出")?;
            sample_duration = sample_duration
                .checked_add(count.checked_mul(delta).ok_or("MP4 采样时间溢出")?)
                .ok_or("MP4 采样时间溢出")?;
        }
        if sample_duration != duration || sample_count == 0 {
            return Err("MP4 时间头与实际采样时序不一致".into());
        }
        let stsz = one(&stbl, b"stsz")?;
        if u64::from(be32(stsz.body, 8)?) != sample_count {
            return Err("MP4 采样大小和时序数量不一致".into());
        }
        verify_mp4_samples(bytes, &root, &stbl, sample_count)?;
        let stsd = one(&stbl, b"stsd")?;
        if be32(stsd.body, 4)? != 1 {
            return Err("多格式 MP4 轨道暂不支持准确区域".into());
        }
        let descriptions = boxes(stsd.body.get(8..).ok_or("MP4 采样描述无效")?)?;
        if descriptions.len() != 1 {
            return Err("MP4 采样描述不唯一".into());
        }
        let description = descriptions[0];
        if description.body.get(6..8) != Some(&[0, 1]) {
            return Err("MP4 采样引用未绑定唯一内部媒体数据".into());
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
                return Err("此 MP4 视频采样格式暂不支持准确像素".into());
            }
            let width = u64::from(u16::from_be_bytes(
                description.body[24..26].try_into().unwrap(),
            ));
            let height = u64::from(u16::from_be_bytes(
                description.body[26..28].try_into().unwrap(),
            ));
            if width == 0 || height == 0 {
                return Err("MP4 视频像素尺寸为空".into());
            }
            let tkhd = one(&track, b"tkhd")?;
            let matrix = match tkhd.body.first() {
                Some(0) => 40,
                Some(1) => 52,
                _ => return Err("MP4 轨道头版本无效".into()),
            };
            let identity = [65536u32, 0, 0, 0, 65536, 0, 0, 0, 1073741824];
            for (index, expected) in identity.iter().enumerate() {
                if be32(tkhd.body, matrix + index * 4)? != *expected {
                    return Err("带旋转/变换的视频暂不支持原始像素区域".into());
                }
            }
            let child = boxes(&description.body[78..])?;
            if child.iter().any(|item| item.kind == *b"clap") {
                return Err("带裁切的视频暂不支持原始像素区域".into());
            }
            for aspect in child.iter().filter(|item| item.kind == *b"pasp") {
                if be32(aspect.body, 0)? != be32(aspect.body, 4)? {
                    return Err("非方形像素视频暂不支持准确区域".into());
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
                return Err("多视频轨道暂不支持准确播放器目标".into());
            }
        } else {
            if description.kind != *b"mp4a" {
                return Err("此 MP4 音频采样格式暂不支持准确时基".into());
            }
            if audio
                .replace(ResourceMetadata::Audio {
                    timebase,
                    duration_tick: duration,
                })
                .is_some()
            {
                return Err("多音频轨道暂不支持准确播放器目标".into());
            }
        }
    }
    video.or(audio).ok_or("MP4 缺少受支持的媒体轨道".into())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let opened = core.create_note(&library, &library, "媒体").unwrap();
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
            .map_err(|error| error.message)
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
            Json(ReadRequest {
                session_id: opened.session_id.clone(),
                resource: selected.resource_ref.clone(),
                revision: revision.clone(),
            }),
        )
        .await
        .map_err(|error| error.message)
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
        let opened = core.create_note(&library, &library, "缩略图").unwrap();
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
        let ready = query_preview(State(state.clone()), Json(request.clone()))
            .await
            .map_err(|error| error.message)
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
        let response = query_preview_resource(State(state.clone()), Json(raw))
            .await
            .map_err(|error| error.message)
            .unwrap();
        let delivered = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(delivered.as_ref(), bytes.as_slice());
        let stale = query_preview(
            State(state.clone()),
            Json(QueryPreviewRequest {
                revision: uuid::Uuid::new_v4().to_string(),
                ..request.clone()
            }),
        )
        .await
        .map_err(|error| error.message)
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
        let ready = query_preview(State(state.clone()), Json(observed_request.clone()))
            .await
            .map_err(|error| error.message)
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
        let stale = query_preview(State(state.clone()), Json(observed_request))
            .await
            .map_err(|error| error.message)
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
