//! Knowledge adapters consume guarded Core snapshots. Draft patches remain
//! proposals; all persistent changes enter the existing Core operation journal.
use super::{ApiError, ApiResult, AppState, core_call, location, remember, value};
#[cfg(test)]
use crate::CoreTestFixture;
use crate::writing_api::{check_snapshot, observe_snapshot};
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use weftext_core::{FileChange, OperationIntent, OperationRequest, SnapshotFile};
use weftext_knowledge as knowledge;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/knowledge/read", post(read))
        .route("/api/knowledge/proofread", post(proofread))
        .route("/api/knowledge/prepare", post(prepare))
        .route("/api/knowledge/template-preview", post(template_preview))
        .route("/api/knowledge/link-copy", post(link_copy))
        .route("/api/knowledge/trash", post(read_trash))
        .route("/api/operations/prepare", post(prepare_operation))
        .route("/api/operations/commit", post(commit))
        .route("/api/operations/status", post(status))
        .route("/api/operations/settle", post(settle))
}

/// Navigation returns stable admitted identities. The host opens those through
/// the strong resolver again, rather than reopening a disclosed stale path.
pub(crate) async fn resolve_address(state: &AppState, session_id: &str, target: &str) -> ApiResult {
    let normalized = target.split_once(':').and_then(|(scheme, rest)| {
        ["weftext-node", "weftext-workspace", "weftext-resource"]
            .into_iter()
            .find(|known| scheme.eq_ignore_ascii_case(known))
            .map(|scheme| format!("{scheme}:{rest}"))
    });
    let target = normalized.as_deref().unwrap_or(target);
    let loc = location(state, session_id)?;
    let application = target.starts_with("weftext-node://")
        || target.starts_with("weftext-workspace://")
        || target.starts_with("weftext-resource://");
    let root_check = loc.root.clone();
    if !core_call(state, move |core| core.has_library(root_check)).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            if application {
                "unavailable"
            } else {
                "ordinaryFile"
            },
            "当前会话是普通原生文件",
        ));
    }
    let root = loc.root.clone();
    let (cut, resources, complete) = core_call(state, move |core| {
        let cut = core.admitted_snapshot(&root)?;
        let mut resources = Vec::new();
        let mut complete = cut.complete;
        let mut total = 0usize;
        for file in &cut.files {
            match core.managed_resource_snapshot(&root, &file.path) {
                Ok(items) => {
                    let bytes = items.iter().map(|item| item.bytes.len()).sum::<usize>();
                    if total.saturating_add(bytes) > 64 * 1024 * 1024
                        || resources.len() + items.len() > 5000
                    {
                        complete = false;
                        break;
                    }
                    total += bytes;
                    resources.extend(items);
                }
                Err(_) => complete = false,
            }
        }
        Ok((cut, resources, complete))
    })
    .await?;
    let workspace = cut
        .inventory
        .workspace_id
        .ok_or_else(|| ApiError::bad("当前项目不是已接纳笔记库"))?;
    let mut nodes = Vec::new();
    for entry in cut
        .inventory
        .entries
        .iter()
        .filter(|entry| entry.identity_status == "admitted")
    {
        let Some(file) = cut.files.iter().find(|file| file.path == entry.path) else {
            continue;
        };
        let Some(id) = &file.node_id else {
            continue;
        };
        let Some(bytes) = &file.bytes else {
            continue;
        };
        let source =
            String::from_utf8(bytes.clone()).map_err(|_| ApiError::bad("目标原文不是 UTF-8"))?;
        let title = weftext_engine::inspect_editor(&source).title.text;
        nodes.push(knowledge::NodeSnapshot {
            owner: knowledge::NodeRef {
                workspace_id: workspace.clone(),
                node_id: id.clone(),
            },
            path: file_url(&file.path)?,
            library_path: entry.relative_path.clone(),
            revision_id: file.revision.clone().unwrap_or_default(),
            source,
            title,
            aliases: Vec::new(),
            labels: Vec::new(),
            labels_complete: false,
        });
    }
    let source = nodes
        .iter()
        .find(|node| file_path(&node.path).is_ok_and(|path| path == loc.path.to_string_lossy()))
        .or_else(|| application.then(|| nodes.first()).flatten())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                if application {
                    "unavailable"
                } else {
                    "ordinaryFile"
                },
                "当前会话尚未纳管为笔记节点",
            )
        })?
        .clone();
    let mut resource_index = Vec::new();
    for resource in resources
        .iter()
        .filter(|resource| resource.identity_status == "admitted")
    {
        resource_index.push(knowledge::ResourceSnapshot {
            key: resource.resource_ref.clone(),
            path: file_url(&resource.path)?,
            revision_id: resource.revision.clone().unwrap_or_default(),
        });
    }
    let inventory = knowledge::InventorySnapshot {
        generation: cut.cut_id,
        coverage: if complete {
            knowledge::Coverage::Complete
        } else {
            knowledge::Coverage::Partial
        },
        nodes,
        resources: resource_index,
    };
    let resolution = knowledge::resolve_link(target, &source, &inventory);
    if let Some(node) = inventory
        .nodes
        .iter()
        .find(|node| Some(&node.owner) == resolution.target.as_ref())
    {
        return Ok(Json(
            json!({"entityKey":format!("weftext-node://{}/{}",node.owner.workspace_id,node.owner.node_id),"path":file_path(&node.path)?,"root":loc.root,"fragment":resolution.fragment,"resolution":resolution.status}),
        ));
    }
    if let Some(resource) = resources.iter().find(|resource| {
        Some(&resource.resource_ref) == resolution.resource.as_ref()
            && resource.identity_status == "admitted"
    }) {
        let revision = resource.revision.as_ref().ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "resourceRevision",
                "资源当前没有可验证的准确版本，请重新读取并明确处理",
            )
        })?;
        let fragment = resource_fragment(&resource.bytes, resolution.fragment.as_deref());
        return Ok(Json(
            json!({"resourceRef":resource.resource_ref,"ownerEntityKey":format!("weftext-node://{workspace}/{}",resource.node_id),"revision":revision,"path":resource.path,"root":loc.root,"fragment":resolution.fragment,"resourceFragment":fragment,"resolution":if resolution.fragment.is_some(){fragment["status"].as_str().unwrap_or("unavailable")}else{"resolved"}}),
        ));
    }
    Err(ApiError::new(
        StatusCode::CONFLICT,
        if !application && resolution.status == "missing" {
            "ordinaryFile"
        } else {
            &resolution.status
        },
        "目标地址无效、身份不可用或不在当前获权笔记库内",
    ))
}
/// RFC 3778 / Adobe PDF open parameters use one-based page=N. Only this exact
/// profile is interpreted, and only after the actual PDF reader proves bounds.
fn resource_fragment(bytes: &[u8], fragment: Option<&str>) -> Value {
    resource_fragment_metadata(&crate::media_api::metadata_from_bytes(bytes), fragment)
}
pub(crate) fn resource_fragment_metadata(
    metadata: &crate::media_api::ResourceMetadata,
    fragment: Option<&str>,
) -> Value {
    let Some(fragment) = fragment else {
        return json!({"status":"none"});
    };
    let Some(page) = fragment
        .strip_prefix("page=")
        .filter(|page| !page.is_empty() && page.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|page| page.parse::<u64>().ok())
    else {
        return json!({"status":"unavailable"});
    };
    match metadata {
        crate::media_api::ResourceMetadata::Pdf { pages }
            if page > 0 && page <= pages.len() as u64 =>
        {
            json!({"status":"found","pageIndex":page-1})
        }
        crate::media_api::ResourceMetadata::Pdf { .. } => json!({"status":"missing"}),
        _ => json!({"status":"unavailable"}),
    }
}
fn resource_qualifications(bundle: &Bundle) -> Vec<knowledge::ResourceFragmentQualification> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut metadata = BTreeMap::new();
    for node in std::iter::once(&bundle.selected) {
        for link in knowledge::native_links_bounded(&node.source, 20_000, 4 * 1024 * 1024).links {
            let resolution = knowledge::resolve_link(&link.target, node, &bundle.inventory);
            let (Some(resource), Some(fragment)) = (resolution.resource, resolution.fragment)
            else {
                continue;
            };
            if !seen.insert((resource.clone(), fragment.clone())) {
                continue;
            }
            let Some(snapshot) = bundle
                .inventory
                .resources
                .iter()
                .find(|snapshot| snapshot.key == resource)
            else {
                continue;
            };
            let Some(bytes) = file_path(&snapshot.path)
                .ok()
                .and_then(|path| bundle.files.iter().find(|file| file.path == path))
                .and_then(|file| file.bytes.as_ref())
            else {
                continue;
            };
            let actual = metadata
                .entry(resource.clone())
                .or_insert_with(|| crate::media_api::metadata_from_bytes(bytes));
            let qualification = resource_fragment_metadata(actual, Some(&fragment));
            let status = match qualification["status"].as_str() {
                Some("found") => knowledge::ResourceFragmentStatus::Found,
                Some("missing") => knowledge::ResourceFragmentStatus::Missing,
                _ => knowledge::ResourceFragmentStatus::Unavailable,
            };
            out.push(knowledge::ResourceFragmentQualification {
                resource,
                revision_id: snapshot.revision_id.clone(),
                fragment,
                status,
            });
        }
    }
    out
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Context {
    session_id: String,
    source: String,
    serial: u64,
    base_id: String,
    #[serde(default)]
    dirty: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadRequest {
    context: Context,
}
#[derive(Deserialize)]
struct ProofreadRequest {
    context: Context,
    options: knowledge::ProofreadOptions,
}
fn proofreading_scan(
    state: &AppState,
    context: &Context,
    bundle: &Bundle,
    options: &knowledge::ProofreadOptions,
) -> Result<knowledge::ProofreadResult, ApiError> {
    let tree = render_tree(state, context, &bundle.style_generation)?.ok_or_else(|| {
        ApiError::new(
            StatusCode::CONFLICT,
            "preview_required",
            "请等待当前正文预览成功后再校对；不会使用旧稿结果",
        )
    })?;
    knowledge::proofread(&tree, &context.source, options).map_err(knowledge_error)
}
fn existing_finding(
    document: &knowledge::AnnotationDocument,
    finding: &knowledge::ProofreadFinding,
    current: &knowledge::NodeSnapshot,
) -> bool {
    document.records.iter().any(|record| {
        let origin = record.unknown.get("proofreading");
        origin.is_some_and(|v| {
            v["ruleId"] == finding.rule_id && v["ruleVersion"] == finding.rule_version
        }) && record
            .target
            .anchor()
            .and_then(|anchor| knowledge::resolve_anchor(anchor, Some(current), &[]).current_range)
            .zip(finding.source_range.as_ref())
            .is_some_and(|(a, b)| a.from == b.from && a.to == b.to && a.raw == b.raw)
    })
}
async fn proofread(
    State(state): State<AppState>,
    Json(request): Json<ProofreadRequest>,
) -> ApiResult {
    observe(&state, &request.context)?;
    validate_processing_origin(&state, &request.context, &request.options).await?;
    let bundle = bundle(&state, &request.context).await?;
    let scan = proofreading_scan(&state, &request.context, &bundle, &request.options)?;
    let existing = annotation_document(&bundle.annotations_file)
        .ok()
        .map(|document| {
            scan.findings
                .iter()
                .filter(|f| existing_finding(&document, f, &bundle.committed))
                .map(|f| f.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    exact(&state, &request.context)?;
    validate_processing_origin(&state, &request.context, &request.options).await?;
    let mut output = value(scan)?;
    output["existing"] = json!(existing);
    Ok(Json(output))
}

/// Shared suggestions become a dependency of the explicit adopted operation,
/// while the operation's language and rule inputs remain exactly its own.
async fn validate_processing_origin(
    state: &AppState,
    context: &Context,
    options: &knowledge::ProofreadOptions,
) -> Result<Option<weftext_core::ConfigDocument>, ApiError> {
    let Some(origin) = &options.processing_origin else {
        return Ok(None);
    };
    if origin.source != "shared-library" || origin.base_id.is_empty() {
        return Err(ApiError::bad("共享处理建议缺少准确来源依据，原输入保留"));
    }
    let root = location(state, &context.session_id)?.root;
    let read_root = root.clone();
    let original_base = origin.base_id.clone();
    let original_path = root.join(".weftext-settings.json");
    let mut document = core_call(state, move |core| {
        core.validate_snapshot(&read_root, original_path, &original_base)?;
        core.read_config(read_root, weftext_core::ConfigRole::WorkspaceSettings)
    })
    .await?;
    if origin
        .revision
        .as_ref()
        .is_some_and(|revision| document.revision.as_ref() != Some(revision))
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "processingOriginChanged",
            "共享处理建议的原版本已变化，请保留本次输入并重新核对共享来源",
        ));
    }
    let settings: Value = serde_json::from_slice(
        document
            .bytes
            .as_deref()
            .ok_or_else(|| ApiError::bad("原共享处理设置当前不可用"))?,
    )
    .map_err(|_| ApiError::bad("共享库设置当前不能解析，原输入保留"))?;
    let processing: knowledge::ProcessingDocument =
        serde_json::from_value(settings["processing"].clone())
            .map_err(|_| ApiError::bad("原共享处理建议当前不可用，原输入保留"))?;
    processing.validate().map_err(knowledge_error)?;
    let path = document.path.clone();
    let base = origin.base_id.clone();
    core_call(state, move |core| core.validate_snapshot(root, path, &base)).await?;
    document.base_id = origin.base_id.clone();
    Ok(Some(document))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrepareRequest {
    operation_id: String,
    context: Context,
    intent: Value,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitRequest {
    operation_id: String,
    plan_id: String,
    #[serde(default)]
    context: Option<Context>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusRequest {
    operation_id: String,
    #[serde(default)]
    context: Option<Context>,
}

fn knowledge_error(error: knowledge::KnowledgeError) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, &error.code, error.message)
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    value[key]
        .as_str()
        .ok_or_else(|| ApiError::bad(format!("缺少 {key}")))
}
fn integer(value: &Value, key: &str) -> Result<usize, ApiError> {
    value[key]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| ApiError::bad(format!("无效的 {key}")))
}
fn node_ref(value: &Value) -> Result<knowledge::NodeRef, ApiError> {
    Ok(knowledge::NodeRef {
        workspace_id: text(value, "workspaceId")?.into(),
        node_id: text(value, "nodeId")?.into(),
    })
}
fn observe(state: &AppState, context: &Context) -> Result<(), ApiError> {
    observe_snapshot(
        state,
        &context.session_id,
        &context.source,
        context.serial,
        None,
    )
}
fn exact(state: &AppState, context: &Context) -> Result<(), ApiError> {
    check_snapshot(
        state,
        &context.session_id,
        &context.source,
        context.serial,
        None,
    )
}
fn file_url(path: &str) -> Result<String, ApiError> {
    url::Url::from_file_path(path)
        .map(|url| url.into())
        .map_err(|_| ApiError::bad("文件路径不能生成本机地址"))
}
fn file_path(url: &str) -> Result<String, ApiError> {
    url::Url::parse(url)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
        .ok_or_else(|| ApiError::bad("无效的本机文件地址"))
}
fn annotations_path(path: &Path) -> Result<String, ApiError> {
    Ok(path
        .parent()
        .ok_or_else(|| ApiError::bad("笔记缺少所属目录"))?
        .join("weftext.annotations.json")
        .to_string_lossy()
        .into_owned())
}

struct Bundle {
    mention_candidates: Vec<knowledge::MentionCandidate>,
    inventory: knowledge::InventorySnapshot,
    selected: knowledge::NodeSnapshot,
    committed: knowledge::NodeSnapshot,
    source_file: SnapshotFile,
    annotations_file: SnapshotFile,
    warnings: Vec<String>,
    trash: Value,
    schema: weftext_data::Schema,
    schema_generation: String,
    files: Vec<SnapshotFile>,
    style_generation: String,
    citation_rename_available: bool,
    settings: SnapshotFile,
    tag_tree: Option<Vec<knowledge::GraphTag>>,
}

async fn bundle(state: &AppState, context: &Context) -> Result<Bundle, ApiError> {
    let loc = location(state, &context.session_id)?;
    let source_path = loc.path.to_string_lossy().into_owned();
    let annotation_path = annotations_path(&loc.path)?;
    let root = loc.root.clone();
    let annotation_for_cut = annotation_path.clone();
    let (cut, annotations_file, settings, resources, resource_warnings, trash, tag_document) =
        core_call(state, move |core| {
            let cut = core.admitted_snapshot(&root)?;
            let annotations = core
                .read_snapshot(&root, &[annotation_for_cut])?
                .files
                .remove(0);
            let settings = core
                .read_snapshot(
                    &root,
                    &[root
                        .join(".weftext-settings.json")
                        .to_string_lossy()
                        .into_owned()],
                )?
                .files
                .remove(0);
            let mut resources = Vec::new();
            let mut warnings = Vec::new();
            let mut bytes = 0usize;
            for file in &cut.files {
                match core.managed_resource_snapshot(&root, &file.path) {
                    Ok(items) => {
                        let size = items.iter().map(|item| item.bytes.len()).sum::<usize>();
                        if bytes.saturating_add(size) > 64 * 1024 * 1024
                            || resources.len() + items.len() > 5000
                        {
                            warnings.push(
                                "资源索引达到本次 64 MiB/5000 项读取上限；其余资源保持不可用。"
                                    .into(),
                            );
                            break;
                        }
                        bytes += size;
                        resources.extend(items);
                    }
                    Err(error) => warnings.push(format!("{}: {}", file.path, error.message)),
                }
            }
            let trash = core.trash_entries(&root)?;
            let tag_document = cut.tags.as_ref().and_then(|snapshot| {
                core.qualified_tag_document_from_snapshot(&root, snapshot)
                    .ok()
            });
            Ok((
                cut,
                annotations,
                settings,
                resources,
                warnings,
                trash,
                tag_document,
            ))
        })
        .await?;
    let inventory = cut.inventory;
    let workspace_id = inventory
        .workspace_id
        .clone()
        .ok_or_else(|| ApiError::bad("此功能需要打开已接纳身份的笔记库"))?;
    let source_file = cut
        .files
        .iter()
        .find(|file| file.path == source_path)
        .cloned()
        .ok_or_else(|| ApiError::internal("缺少正文快照"))?;
    let mut nodes = Vec::new();
    let mut warnings = cut.warnings;
    let resources_complete = resource_warnings.is_empty();
    warnings.extend(resource_warnings);
    let (schema, schema_generation) = crate::data_api::knowledge_schema_from_snapshot(
        cut.schema
            .as_ref()
            .ok_or_else(|| ApiError::internal("知识快照缺少字段定义事实"))?,
    )?;
    let mut dictionary_complete = true;
    let admitted = inventory
        .entries
        .iter()
        .filter(|entry| entry.identity_status == "admitted")
        .count();
    for entry in inventory
        .entries
        .iter()
        .filter(|entry| entry.identity_status == "admitted")
    {
        let Some(snapshot) = cut.files.iter().find(|file| file.path == entry.path) else {
            continue;
        };
        let Some(bytes) = &snapshot.bytes else {
            continue;
        };
        let Some(node_id) = &snapshot.node_id else {
            continue;
        };
        // Discovery and exact cut must still name the same admitted object.
        if entry.node_id.as_ref() != Some(node_id) {
            continue;
        }
        let source =
            String::from_utf8(bytes.clone()).map_err(|_| ApiError::bad("笔记原文不是 UTF-8"))?;
        let info = weftext_engine::inspect_metadata(&source);
        let title = weftext_engine::inspect_editor(&source).title;
        if title.status != "writable" && info.writer_status != "unique-literal-profile" {
            dictionary_complete = false;
            warnings.push(format!(
                "{} 的原生标题需要编译资格，本次未把未知标题视为缺失。",
                entry.relative_path
            ));
        }
        let terms = weftext_data::search_terms(&source, &schema);
        if !terms.complete {
            dictionary_complete = false;
            warnings.push(format!(
                "{} 名称字典部分可用：{}",
                entry.relative_path,
                terms.reasons.join("；")
            ));
        }
        nodes.push(knowledge::NodeSnapshot {
            owner: knowledge::NodeRef {
                workspace_id: workspace_id.clone(),
                node_id: node_id.clone(),
            },
            path: file_url(&entry.path)?,
            library_path: entry.relative_path.clone(),
            revision_id: snapshot.revision.clone().unwrap_or_default(),
            source,
            title: title.text,
            aliases: terms.terms,
            labels: Vec::new(),
            labels_complete: false,
        });
    }
    let coverage =
        if cut.complete && admitted == nodes.len() && dictionary_complete && resources_complete {
            knowledge::Coverage::Complete
        } else {
            warnings.push("当前图索引中有已接纳笔记不可用；显示本次准确读取的部分范围。".into());
            knowledge::Coverage::Partial
        };
    let committed = nodes
        .iter()
        .find(|node| file_path(&node.path).is_ok_and(|path| path == source_path))
        .cloned()
        .ok_or_else(|| ApiError::bad("当前文件尚未接纳为有效笔记节点，请检查身份与目录"))?;
    let mut selected = committed.clone();
    selected.source = context.source.clone();
    if selected.source != committed.source {
        selected.revision_id.clear();
    }
    selected.title = weftext_engine::inspect_editor(&selected.source).title.text;
    // Projection generations describe the actual consumed facts, not write
    // qualification. Core base IDs remain private guarded proofs.
    let style = crate::writing_api::style_environment(state, &context.session_id).await?;
    let citation_rename_available = style
        .style
        .as_ref()
        .is_none_or(weftext_engine::style_allows_rich_editing);
    let mut files = cut.files;
    files.push(settings.clone());
    if let Some(schema) = cut.schema {
        files.push(schema);
    }
    if let Some(tags) = cut.tags {
        files.push(tags);
    }
    let mut resource_index = Vec::new();
    for resource in resources
        .into_iter()
        .filter(|resource| resource.identity_status == "admitted")
    {
        resource_index.push(knowledge::ResourceSnapshot {
            key: resource.resource_ref,
            path: file_url(&resource.path)?,
            revision_id: resource.revision.clone().unwrap_or_default(),
        });
        files.push(SnapshotFile {
            path: resource.path,
            bytes: Some(resource.bytes),
            base_id: resource.base_id,
            node_id: Some(resource.node_id),
            kind: "resource".into(),
            currentness: "observed".into(),
            revision: resource.revision,
        });
        if !files.iter().any(|file| file.path == resource.manifest_path) {
            files.push(SnapshotFile {
                path: resource.manifest_path,
                bytes: Some(resource.manifest_bytes),
                base_id: resource.manifest_base_id,
                node_id: None,
                kind: "resourceManifest".into(),
                currentness: "observed".into(),
                revision: None,
            });
        }
    }
    let resources = resource_index;
    let facts = serde_json::to_string(&json!({"nodes":nodes,"resources":resources,"selectedDraft":selected.source,"style":style.key,"schema":schema_generation,"tags":tag_document})).map_err(|_| ApiError::internal("无法编码索引事实"))?;
    let tag_tree = tag_document
        .as_ref()
        .map(|document| knowledge::graph_tag_tree(&workspace_id, document));
    let inventory = knowledge::InventorySnapshot {
        generation: weftext_engine::source_hash(&facts),
        coverage,
        nodes,
        resources,
    };
    exact(state, context)?;
    Ok(Bundle {
        mention_candidates: vec![],
        inventory,
        selected,
        committed,
        source_file,
        annotations_file,
        warnings,
        trash: value(trash)?,
        schema,
        schema_generation,
        files,
        style_generation: style.key,
        citation_rename_available,
        settings,
        tag_tree,
    })
}

/// Label shortcuts are qualified only for a compile whose author-source inputs
/// were supplied from the cut. Unknown include/read inputs remain unavailable.
async fn populate_labels(
    state: &AppState,
    context: &Context,
    bundle: &mut Bundle,
    target: &knowledge::NodeRef,
) -> Result<(), ApiError> {
    let Some(mut node) = bundle
        .inventory
        .nodes
        .iter()
        .find(|node| &node.owner == target)
        .cloned()
    else {
        return Ok(());
    };
    if node.owner == bundle.selected.owner {
        node = bundle.selected.clone();
    }
    if node.labels_complete {
        return Ok(());
    }
    let path = file_path(&node.path)?;
    let Some(file) = bundle.files.iter().find(|file| file.path == path).cloned() else {
        return Ok(());
    };
    let loc = location(state, &context.session_id)?;
    let environment = crate::writing_api::style_environment(state, &context.session_id).await?;
    let files = draft_compile_files(
        &loc.root,
        Path::new(&path),
        &file,
        &frozen_map(&loc.root, &bundle.files, None)?,
        &node.source,
    )?;
    let output = match super::render_frozen_worker(
        state,
        weftext_engine::FrozenRenderRequest {
            source: node.source,
            path: path.clone().into(),
            root: loc.root.clone(),
            style: environment.style.clone(),
            files,
        },
    )
    .await
    {
        Ok(output) => output,
        Err(error) => {
            bundle
                .warnings
                .push(format!("标签编译不可用：{}", error.message));
            return Ok(());
        }
    };
    crate::writing_api::check_style_environment(state, &context.session_id, &environment).await?;
    let root = loc.root;
    let base_id = file.base_id;
    core_call(state, move |core| {
        core.validate_snapshot(root, path, &base_id)
    })
    .await?;
    exact(state, context)?;
    if output["ok"] != true {
        return Ok(());
    }
    let root = location(state, &context.session_id)?.root;
    validate_render_inputs(state, &root, &bundle.files, &output, None).await?;
    let mut allowed: BTreeSet<_> = bundle
        .files
        .iter()
        .filter_map(|file| {
            Path::new(&file.path)
                .strip_prefix(&root)
                .ok()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    if let Some(style) = &environment.style {
        allowed.insert(style.path.replace('\\', "/"));
    }
    if !output["dependencies"]
        .as_array()
        .is_some_and(|dependencies| {
            dependencies.iter().all(|dependency| {
                dependency["status"] == "loaded"
                    && (derived_render_input(dependency)
                        || dependency["path"]
                            .as_str()
                            .is_some_and(|path| allowed.contains(path)))
            })
        })
    {
        return Ok(());
    }
    let Some(labels) = output["labels"].as_array() else {
        return Ok(());
    };
    let mut actual = Vec::new();
    for label in labels {
        let Some(name) = label["label"].as_str() else {
            return Ok(());
        };
        let Some(count) = label["count"].as_u64().filter(|count| *count <= 10000) else {
            return Ok(());
        };
        actual.extend(std::iter::repeat_n(name.to_owned(), count as usize));
    }
    if let Some(node) = bundle
        .inventory
        .nodes
        .iter_mut()
        .find(|node| &node.owner == target)
    {
        node.labels = actual.clone();
        node.labels_complete = true;
        let headings: Vec<weftext_engine::HeadingInfo> =
            serde_json::from_value(output["sourceInfo"]["headings"].clone()).unwrap_or_default();
        let labels: Vec<weftext_engine::CompiledLabel> =
            serde_json::from_value(output["labels"].clone()).unwrap_or_default();
        bundle
            .mention_candidates
            .extend(knowledge::compiled_mention_candidates(
                node, &headings, &labels,
            ));
    }
    if bundle.selected.owner == *target {
        bundle.selected.labels = actual;
        bundle.selected.labels_complete = true;
    }
    Ok(())
}
async fn qualify_link_fragment(
    state: &AppState,
    context: &Context,
    bundle: &mut Bundle,
    occurrence_from: usize,
) -> Result<(), ApiError> {
    let link = knowledge::native_links(&bundle.selected.source)
        .into_iter()
        .find(|link| link.from == occurrence_from)
        .ok_or_else(|| ApiError::bad("链接位置已经变化"))?;
    if let Some((address, _)) = link.target.split_once('#') {
        let target = knowledge::resolve_link(address, &bundle.selected, &bundle.inventory).target;
        if let Some(target) = target {
            populate_labels(state, context, bundle, &target).await?;
        }
    }
    Ok(())
}
async fn populate_diagnostic_labels(
    state: &AppState,
    context: &Context,
    bundle: &mut Bundle,
) -> Result<(), ApiError> {
    // One bounded compile set, shared by read and conversion requalification.
    let targets: Vec<_> = std::iter::once(bundle.selected.owner.clone())
        .chain(
            bundle
                .inventory
                .nodes
                .iter()
                .filter(|n| n.owner != bundle.selected.owner)
                .map(|n| n.owner.clone()),
        )
        .take(8)
        .collect();
    for target in targets {
        populate_labels(state, context, bundle, &target).await?;
    }
    Ok(())
}

fn render_tree(
    state: &AppState,
    context: &Context,
    environment_key: &str,
) -> Result<Option<weftext_engine::RenderNode>, ApiError> {
    let renders = state
        .renders
        .lock()
        .map_err(|_| ApiError::internal("预览会话不可用"))?;
    let Some(render) = renders.get(&context.session_id).filter(|render| {
        render.source == context.source
            && render.serial == context.serial
            && render.output["ok"] == true
            && render.output["environmentKey"].as_str() == Some(environment_key)
    }) else {
        return Ok(None);
    };
    if render.output["tree"].is_null() {
        return Ok(None);
    }
    serde_json::from_value(render.output["tree"].clone())
        .map(Some)
        .map_err(|_| ApiError::internal("预览正文映射无效"))
}
fn mentions(
    state: &AppState,
    context: &Context,
    bundle: &Bundle,
) -> Result<knowledge::MentionScan, ApiError> {
    let tree = render_tree(state, context, &bundle.style_generation)?;
    let fragments = tree
        .as_ref()
        .map(knowledge::visible_fragments)
        .unwrap_or_default();
    let mut scan = knowledge::scan_mentions_with_candidates(
        &bundle.inventory,
        &bundle.selected.owner,
        &fragments,
        &bundle.mention_candidates,
        2_000_000,
        20_000,
    )
    .map_err(knowledge_error)?;
    if tree.is_none() {
        scan.coverage = knowledge::Coverage::Partial;
    }
    if bundle.inventory.nodes.iter().any(|n| !n.labels_complete) {
        scan.coverage = knowledge::Coverage::Partial;
    }
    Ok(scan)
}
fn annotation_document(file: &SnapshotFile) -> Result<knowledge::AnnotationDocument, ApiError> {
    knowledge::decode_annotations(
        file.bytes
            .as_deref()
            .unwrap_or(b"{\"version\":1,\"annotations\":[]}"),
    )
    .map_err(knowledge_error)
}
fn annotation_projection(
    document: &knowledge::AnnotationDocument,
    current: &knowledge::NodeSnapshot,
    clean: bool,
    resources: &[crate::media_api::QualifiedResource],
) -> Value {
    let views = knowledge::annotation_views(document, Some(current), &[]);
    let items: Vec<_> = views.iter().filter(|view| view.reply_to.is_none()).map(|view| {
        let record = document.records.iter().find(|record| record.annotation_id == view.annotation_id);
        let resource=record.and_then(|record|resources.iter().find(|resource|resource.facts.as_ref().is_some_and(|facts|knowledge::validate_resource_target(&record.target,facts).is_ok())));
        let state = if resource.is_some() || record.is_some_and(|record|matches!(&record.target,knowledge::AnnotationTarget::WholeDocument { node } if node==&current.owner)) { "exact" } else if clean { view.target_state.as_str() } else if record.is_some_and(|record| record.target.anchor().is_some()) { "stale" } else { view.target_state.as_str() };
        let replies: Vec<_> = views.iter().filter(|reply| reply.reply_to.as_deref() == Some(view.annotation_id.as_str())).map(|reply| json!({"id":reply.annotation_id,"body":reply.body,"author":reply.attribution,"createdAt":reply.created_at})).collect();
        let source_location=record.and_then(|record|record.target.anchor()).filter(|_|clean).and_then(|anchor|knowledge::resolve_anchor(anchor,Some(current),&[]).current_range).map(|range|json!({"from":range.utf16_from,"to":range.utf16_to}));
        json!({"id":view.annotation_id,"annotationId":view.annotation_id,"body":view.body,"purpose":view.purpose,"reviewState":view.review_state,"suggestionState":view.suggestion_state,"visibleExcerpt":view.visible_excerpt,"targetResolution":state,"author":view.attribution,"createdAt":view.created_at,"verified":clean && matches!(state,"exact"|"mapped"),"replacement":record.and_then(|record|record.suggestion.as_ref()).map(|suggestion|&suggestion.replacement),"replies":replies,"sourceLocation":source_location,"resourceLocation":resource.map(|resource|json!({"resource":resource.resource_ref,"revision":resource.file.revision,"target":record.map(|record|&record.target)}))})
    }).collect();
    json!(items)
}
async fn read(State(state): State<AppState>, Json(request): Json<ReadRequest>) -> ApiResult {
    observe(&state, &request.context)?;
    let mut bundle = bundle(&state, &request.context).await?;
    populate_diagnostic_labels(&state, &request.context, &mut bundle).await?;
    let selected = bundle.committed.owner.clone();
    let fragment_incoming = bundle
        .inventory
        .nodes
        .iter()
        .flat_map(|node| {
            let source = if node.owner == bundle.selected.owner {
                &bundle.selected.source
            } else {
                &node.source
            };
            knowledge::native_links_bounded(source, 20_000, 4 * 1024 * 1024).links
        })
        .any(|link| link.target.contains('#'));
    if fragment_incoming {
        populate_labels(&state, &request.context, &mut bundle, &selected).await?;
    }
    let mut index = knowledge::link_index(&bundle.inventory, &bundle.selected.owner);
    let resource_fragments = resource_qualifications(&bundle);
    let draft_links =
        knowledge::native_links_bounded(&bundle.selected.source, 20_000, 4 * 1024 * 1024);
    if !draft_links.complete
        || !knowledge::source_link_projection_complete(&bundle.selected.source, &draft_links.links)
    {
        index.coverage = knowledge::Coverage::Partial;
        bundle.warnings.push(
            "当前草稿出链因范围上限或未验证原生依赖而部分可用；已确认结果保留，列表不代表完整零结果。".into(),
        );
    }
    index.outgoing = draft_links
        .links
        .into_iter()
        .map(|occurrence| knowledge::LinkEdge {
            source: bundle.selected.owner.clone(),
            resolution: knowledge::resolve_link_with_resource_fragments(
                &occurrence.target,
                &bundle.selected,
                &bundle.inventory,
                &resource_fragments,
            ),
            occurrence,
        })
        .collect();
    if !weftext_engine::inspect_editor(&request.context.source).syntax_valid {
        index.coverage = knowledge::Coverage::Partial;
        bundle.warnings.push(
            "当前草稿的原生语法尚未完整解析，出链列表不代表完整零结果；请保留源码并修复语法。"
                .into(),
        );
    }
    let mut links = value(index)?;
    for group in ["outgoing", "incomingId", "incomingPath"] {
        if let Some(edges) = links[group].as_array_mut() {
            for edge in edges {
                enrich_ref(&mut edge["source"], &bundle.inventory);
                enrich_ref(&mut edge["resolution"]["target"], &bundle.inventory);
                if let Some(source) = if group == "outgoing" {
                    Some(&bundle.selected)
                } else {
                    bundle.inventory.nodes.iter().find(|node| {
                        edge["source"]["nodeId"].as_str() == Some(node.owner.node_id.as_str())
                    })
                } && let (Some(from), Some(to)) = (
                    edge["occurrence"]["from"].as_u64(),
                    edge["occurrence"]["to"].as_u64(),
                ) {
                    let from = from as usize;
                    let to = to as usize;
                    let start = if edge["occurrence"]["kind"] == "native"
                        && from > 0
                        && source.source.as_bytes().get(from - 1) == Some(&b'#')
                    {
                        from - 1
                    } else {
                        from
                    };
                    if let Some(raw) = source.source.get(start..to) {
                        edge["occurrence"]["raw"] = json!(raw);
                    }
                }
            }
        }
    }
    let mut mentions = value(mentions(&state, &request.context, &bundle)?)?;
    if let Some(fragments) = mentions["fragments"].as_array_mut() {
        for fragment in fragments {
            if let Some(hits) = fragment["hits"].as_array_mut() {
                for hit in hits {
                    if let Some(targets) = hit["targets"].as_array_mut() {
                        for target in targets {
                            enrich_ref(target, &bundle.inventory);
                        }
                    }
                    if let Some(candidates) = hit["candidates"].as_array_mut() {
                        for candidate in candidates {
                            enrich_ref(&mut candidate["target"], &bundle.inventory);
                        }
                    }
                }
            }
        }
    }
    let resources = crate::media_api::resource_set(&state, &request.context.session_id).await?;
    let (annotations, annotation_status, diagnostics) = match annotation_document(
        &bundle.annotations_file,
    ) {
        Ok(document) => (
            annotation_projection(
                &document,
                &bundle.committed,
                !request.context.dirty && request.context.source == bundle.committed.source,
                &resources.resources,
            ),
            "available",
            Vec::<String>::new(),
        ),
        Err(error) => {
            let recovered = knowledge::recover_discussion(
                bundle.annotations_file.bytes.as_deref().unwrap_or_default(),
            );
            let discussion:Vec<_>=recovered.into_iter().map(|view|json!({"id":view.annotation_id,"annotationId":view.annotation_id,"body":view.body,"purpose":"recovered","reviewState":view.review_state,"author":view.attribution,"createdAt":view.created_at,"targetResolution":"recoveredUnverified","verified":false})).collect();
            (json!(discussion), "unavailable", vec![error.message])
        }
    };
    let mut property_inputs = Vec::new();
    let mut property_complete = bundle.inventory.coverage == knowledge::Coverage::Complete;
    let mut property_diagnostics = Vec::new();
    for node in &bundle.inventory.nodes {
        let source = if node.owner == bundle.selected.owner {
            &bundle.selected
        } else {
            node
        };
        match weftext_data::typed_ref_closure(&source.source, &bundle.schema) {
            Ok(closure) => {
                property_complete &= closure.complete;
                property_diagnostics.extend(closure.reasons);
                for slot in closure.slots {
                    if matches!(slot.kind.as_str(), "node_ref" | "resource_ref" | "tag_ref") {
                        property_inputs.push(knowledge::PropertyReferenceInput {
                            source: source.owner.clone(),
                            field_id: slot.field_id,
                            item_path: slot
                                .path
                                .iter()
                                .map(|part| serde_json::to_string(part).unwrap_or_default())
                                .collect(),
                            target: slot.target,
                        });
                    }
                }
            }
            Err(error) => {
                property_complete = false;
                property_diagnostics.push(error.to_string());
            }
        }
    }
    let mut properties = value(knowledge::property_references(
        &bundle.inventory,
        &property_inputs,
    ))?;
    if let Some(items) = properties.as_array_mut() {
        for item in items {
            enrich_ref(&mut item["source"], &bundle.inventory);
            enrich_ref(&mut item["resolution"]["target"], &bundle.inventory);
        }
    }
    let property_references = json!({"coverage":if property_complete {"complete"} else {"partial"},"items":properties,"diagnostics":property_diagnostics});
    let mut diagnostic_inventory = bundle.inventory.clone();
    if let Some(node) = diagnostic_inventory
        .nodes
        .iter_mut()
        .find(|n| n.owner == bundle.selected.owner)
    {
        *node = bundle.selected.clone();
    }
    let graph_relations = diagnostic_inventory
        .nodes
        .iter()
        .flat_map(|node| knowledge::graph_relations(node, &bundle.schema, None, None))
        .collect::<Vec<_>>();
    let mut graph = knowledge::relationship_graph(
        &diagnostic_inventory,
        &property_inputs,
        &graph_relations,
        property_complete,
        5_000,
        20_000,
    );
    graph.root = Some(knowledge::graph_key(&bundle.selected.owner));
    graph.tag_tree = bundle.tag_tree.clone();
    for node in &mut graph.nodes {
        node.tags_known = diagnostic_inventory
            .nodes
            .iter()
            .find(|source| knowledge::graph_key(&source.owner) == node.key)
            .is_some_and(|source| {
                knowledge::graph_tag_membership_known(&source.source, &bundle.schema)
            });
    }
    let mut link_diagnostics = value(knowledge::link_diagnostics(
        &diagnostic_inventory,
        &property_inputs,
        property_complete,
        20_000,
    ))?;
    if let Some(issues) = link_diagnostics["issues"].as_array_mut() {
        for edge in issues {
            enrich_ref(&mut edge["source"], &bundle.inventory);
            enrich_ref(&mut edge["resolution"]["target"], &bundle.inventory);
            if let Some(node) = diagnostic_inventory
                .nodes
                .iter()
                .find(|n| edge["source"]["nodeId"] == n.owner.node_id)
            {
                edge["sourceHash"] = json!(weftext_engine::source_hash(&node.source));
            }
        }
    }
    if let Some(nodes) = link_diagnostics["nodes"].as_array_mut() {
        for node in nodes {
            enrich_ref(&mut node["node"], &bundle.inventory);
        }
    }
    if let Some(items) = link_diagnostics["propertyIssues"].as_array_mut() {
        for item in items {
            enrich_ref(&mut item["source"], &bundle.inventory);
            enrich_ref(&mut item["resolution"]["target"], &bundle.inventory);
        }
    }
    let tree = render_tree(&state, &request.context, &bundle.style_generation)?;
    let rendered_links = tree
        .as_ref()
        .map(|tree| {
            knowledge::rendered_links(
                tree,
                &format!(
                    "html:{}:{}",
                    request.context.session_id, request.context.serial
                ),
                "html",
                &bundle.inventory.generation,
            )
        })
        .unwrap_or_default();
    let (citation_tree, citation_labels) =
        citation_products(&state, &request.context, &bundle).await?;
    let heading_structure = knowledge::heading_structure(
        tree.as_ref().or(citation_tree.as_ref()),
        &request.context.source,
        5_000,
    );
    let mut citations = citation_catalog(&bundle, citation_tree.as_ref());
    crate::citation_api::classify_reference_controls(
        &mut citations["editor"],
        citation_tree.as_ref(),
    );
    citations["unlabelledTargets"] = json!(
        citation_tree
            .as_ref()
            .map(|tree| crate::citation_api::unlabelled_reference_targets(
                &request.context.source,
                tree
            ))
            .unwrap_or_default()
    );
    if let Some(targets) = citations["editor"]["targets"].as_array_mut() {
        targets.retain(|target| {
            if !matches!(
                target["kind"].as_str(),
                Some("heading" | "figure" | "table" | "equation" | "footnote")
            ) {
                return false;
            }
            citation_labels.iter().any(|label| {
                label.label == target["label"].as_str().unwrap_or("")
                    && label.count == 1
                    && label.origins.iter().any(|origin| {
                        origin.is_main
                            && origin.from
                                == target["range"]["from"].as_u64().unwrap_or(u64::MAX) as usize
                    })
            })
        });
    }
    let loc = location(&state, &request.context.session_id)?;
    if crate::data_api::knowledge_schema(&state, loc.root.clone())
        .await?
        .1
        != bundle.schema_generation
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "stale_schema",
            "字段定义已变化，请重新读取；当前草稿保留",
        ));
    }
    // The graph and its tag forest are one read. A concurrent reparent must
    // invalidate this receipt rather than install old membership with new names.
    let bases = bundle
        .files
        .iter()
        .map(|file| (file.path.clone(), file.base_id.clone()))
        .collect::<Vec<_>>();
    core_call(&state, move |core| {
        for (path, base) in bases {
            core.validate_snapshot(&loc.root, path, &base)?;
        }
        Ok(())
    })
    .await?;
    exact(&state, &request.context)?;
    let templates = match template_catalog(&bundle) {
        Ok(catalog) => catalog,
        Err(error) => {
            bundle.warnings.push(error.message);
            builtin_template_catalog()
        }
    };
    Ok(Json(
        json!({"graph":graph,"headingStructure":heading_structure,"links":links,"diagnostics":link_diagnostics,"renderedLinks":rendered_links,"propertyReferences":property_references,"citations":citations,"mentions":mentions,"annotations":annotations,"annotationStatus":annotation_status,"annotationDiagnostics":diagnostics,"templates":templates,"trash":bundle.trash,"warnings":bundle.warnings}),
    ))
}
fn unsigned(value: &Value, key: &str) -> Result<u64, ApiError> {
    value[key]
        .as_u64()
        .ok_or_else(|| ApiError::bad(format!("无效的 {key}")))
}
fn number(value: &Value, key: &str) -> Result<f64, ApiError> {
    value[key]
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| ApiError::bad(format!("无效的 {key}")))
}
fn supplied_number(value: &Value, key: &str, actual: f64) -> Result<(), ApiError> {
    if !value[key].is_null() && number(value, key)? != actual {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "resource_geometry",
            "输入的页面、像素或时基与准确资源版本不符；标注草稿保留",
        ));
    }
    Ok(())
}
fn target_from_resource(
    input: &Value,
    resource: &crate::media_api::QualifiedResource,
) -> Result<knowledge::AnnotationTarget, ApiError> {
    let revision_id = resource
        .file
        .revision
        .clone()
        .ok_or_else(|| ApiError::bad("资源尚无已接纳的准确版本"))?;
    target_from_resource_metadata(
        input,
        &resource.resource_ref,
        &revision_id,
        &resource.metadata,
    )
}
pub(crate) fn target_from_resource_metadata(
    input: &Value,
    address: &str,
    revision_id: &str,
    metadata: &crate::media_api::ResourceMetadata,
) -> Result<knowledge::AnnotationTarget, ApiError> {
    if matches!(
        input["kind"].as_str(),
        Some("whole_resource" | "wholeResource")
    ) {
        let target = knowledge::AnnotationTarget::WholeResource {
            resource: address.to_owned(),
            revision_id: revision_id.to_owned(),
        };
        if address.is_empty() || revision_id.is_empty() {
            return Err(ApiError::bad("整资源评论需要准确资源引用和已接纳版本"));
        }
        return Ok(target);
    }
    let facts = crate::media_api::metadata_facts(metadata, address, revision_id)
        .ok_or_else(|| ApiError::bad("此资源缺少已验证的区域证据"))?;
    let address = address.to_owned();
    let revision_id = revision_id.to_owned();
    let target = match (text(input, "kind")?, metadata) {
        ("whole_resource" | "wholeResource", _) => knowledge::AnnotationTarget::WholeResource {
            resource: address,
            revision_id,
        },
        (
            "image_region" | "imageRegion",
            crate::media_api::ResourceMetadata::Image { width, height },
        ) => {
            supplied_number(input, "width", *width as f64)?;
            supplied_number(input, "height", *height as f64)?;
            knowledge::AnnotationTarget::ImageRegion {
                resource: address,
                revision_id,
                pixel_width: *width,
                pixel_height: *height,
                rect: knowledge::PixelRect {
                    x: unsigned(input, "x")?,
                    y: unsigned(input, "y")?,
                    width: unsigned(input, "rectWidth")?,
                    height: unsigned(input, "rectHeight")?,
                },
            }
        }
        ("pdf_region" | "pdfRegion", crate::media_api::ResourceMetadata::Pdf { pages }) => {
            let page_index = u32::try_from(unsigned(input, "pageIndex")?)
                .map_err(|_| ApiError::bad("无效 PDF 页号"))?;
            let page = pages
                .get(page_index as usize)
                .ok_or_else(|| ApiError::bad("PDF 页号超出准确资源"))?;
            supplied_number(input, "width", page.width)?;
            supplied_number(input, "height", page.height)?;
            supplied_number(input, "rotation", f64::from(page.rotation))?;
            knowledge::AnnotationTarget::PdfRegion {
                resource: address,
                revision_id,
                page_index,
                page_width: page.width,
                page_height: page.height,
                rotation: page.rotation,
                rect: knowledge::PointRect {
                    x: number(input, "x")?,
                    y: number(input, "y")?,
                    width: number(input, "rectWidth")?,
                    height: number(input, "rectHeight")?,
                },
            }
        }
        (
            "media_interval" | "mediaInterval",
            crate::media_api::ResourceMetadata::Audio {
                timebase,
                duration_tick,
            },
        ) => {
            supplied_number(input, "timebase", timebase.numerator as f64)?;
            supplied_number(input, "timebaseDenominator", timebase.denominator as f64)?;
            supplied_number(input, "durationTick", *duration_tick as f64)?;
            knowledge::AnnotationTarget::MediaInterval {
                resource: address,
                revision_id,
                timebase: timebase.clone(),
                start_tick: unsigned(input, "startTick")?,
                end_tick: unsigned(input, "endTick")?,
                duration_tick: *duration_tick,
                region: None,
            }
        }
        (
            "media_interval" | "mediaInterval",
            crate::media_api::ResourceMetadata::Video {
                timebase,
                duration_tick,
                width,
                height,
            },
        ) => {
            supplied_number(input, "timebase", timebase.numerator as f64)?;
            supplied_number(input, "timebaseDenominator", timebase.denominator as f64)?;
            supplied_number(input, "durationTick", *duration_tick as f64)?;
            let region = if input["region"].is_object() {
                let region = &input["region"];
                supplied_number(region, "width", *width as f64)?;
                supplied_number(region, "height", *height as f64)?;
                Some(knowledge::VideoRegion {
                    pixel_width: *width,
                    pixel_height: *height,
                    rect: knowledge::PixelRect {
                        x: unsigned(region, "x")?,
                        y: unsigned(region, "y")?,
                        width: unsigned(region, "rectWidth")?,
                        height: unsigned(region, "rectHeight")?,
                    },
                })
            } else {
                None
            };
            knowledge::AnnotationTarget::MediaInterval {
                resource: address,
                revision_id,
                timebase: timebase.clone(),
                start_tick: unsigned(input, "startTick")?,
                end_tick: unsigned(input, "endTick")?,
                duration_tick: *duration_tick,
                region,
            }
        }
        _ => {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "unsupported_resource_target",
                "此标注形式与资源的真实媒体类型不符，或资源格式还没有准确读取器；输入已保留",
            ));
        }
    };
    knowledge::validate_resource_target(&target, &facts).map_err(knowledge_error)?;
    Ok(target)
}
fn enrich_ref(reference: &mut Value, inventory: &knowledge::InventorySnapshot) {
    let Some(node) = inventory.nodes.iter().find(|node| {
        reference["workspaceId"].as_str() == Some(node.owner.workspace_id.as_str())
            && reference["nodeId"].as_str() == Some(node.owner.node_id.as_str())
    }) else {
        return;
    };
    reference["title"] = json!(node.title);
    if let Ok(path) = file_path(&node.path) {
        reference["path"] = json!(path);
    }
}

fn citation_catalog(bundle: &Bundle, tree: Option<&weftext_engine::RenderNode>) -> Value {
    let selected = &bundle.selected;
    let mut citation_complete = bundle.inventory.coverage == knowledge::Coverage::Complete
        && tree.is_some()
        && bundle.citation_rename_available;
    let mut choices = Vec::new();
    for node in &bundle.inventory.nodes {
        let node = if node.owner == selected.owner {
            selected
        } else {
            node
        };
        let projection = weftext_data::project(&node.source, &bundle.schema);
        let reference = format!(
            "weftext-node://{}/{}",
            node.owner.workspace_id, node.owner.node_id
        );
        let publication = projection
            .fields
            .iter()
            .find(|field| field.field_id == "library/publication")
            .and_then(|field| field.cell.native())
            .cloned();
        let typed = projection.qualification == "qualified"
            && projection
                .resolved_types
                .iter()
                .any(|kind| kind == "library/work");
        let suggestion = publication
            .as_ref()
            .and_then(|p| p.member("citation_key"))
            .and_then(weftext_data::NativeValue::text)
            .unwrap_or("");
        let ready = typed
            && publication.clone().is_some_and(|publication| {
                weftext_export::bibliography::project_bibliography(
                    &[weftext_export::bibliography::CitationWork {
                        key: "validation".into(),
                        work: weftext_export::bibliography::FrozenWork {
                            entity_ref: reference.clone(),
                            source_basis: "current-authorized-catalog".into(),
                            publication,
                        },
                    }],
                    weftext_export::bibliography::BibliographyFormat::Bibtex,
                )
                .is_ok()
            });
        choices.push(json!({"reference":reference,"title":node.title.as_deref().unwrap_or("未设置可确认标题"),"node":node.owner,"citationReady":ready,"suggestedKey":suggestion,"reason":if ready {None} else {Some("正式文献引用需要已获权且符合 library/work 的完整出版数据；仍可插入普通链接。")}}));
    }
    let mut editor = weftext_engine::inspect_native_citation_editor(&selected.source);
    if !bundle.citation_rename_available {
        editor.rename_available = false;
        editor.rename_reason =
            Some("当前库样式可能生成外部引用，不能证明完整改名范围；保留源码。".into());
    }
    // Source controls are deliberately kept separate from target-specific
    // display evidence. An unexecuted cite is never counted as visible.
    let mut groups = Vec::new();
    for binding in &editor.bindings {
        let source_slots = editor
            .citations
            .iter()
            .filter(|slot| slot.key == binding.key && slot.binding_proven)
            .collect::<Vec<_>>();
        groups.push(json!({"key":binding.key,"work":binding.work,"authoredSlots":source_slots,"state":if source_slots.is_empty(){"boundOnly"}else{"authored"}}));
    }
    let mut incoming = Vec::new();
    let reference = format!(
        "weftext-node://{}/{}",
        selected.owner.workspace_id, selected.owner.node_id
    );
    for node in &bundle.inventory.nodes {
        let node = if node.owner == selected.owner {
            selected
        } else {
            node
        };
        let inspected = weftext_engine::inspect_native_citation_editor(&node.source);
        if !weftext_engine::native_citation_owner_complete(&node.source)
            || (!inspected.bindings.is_empty() || !inspected.citations.is_empty())
                && !crate::citation_api::owns_single_virtual_bibliography(&node.source)
        {
            citation_complete = false;
        }
        let keys = inspected
            .bindings
            .iter()
            .filter(|binding| binding.work == reference)
            .map(|binding| binding.key.as_str())
            .collect::<BTreeSet<_>>();
        let slots = inspected
            .citations
            .iter()
            .filter(|slot| {
                keys.contains(slot.key.as_str())
                    && slot.binding_proven
                    && crate::citation_api::owns_single_virtual_bibliography(&node.source)
            })
            .collect::<Vec<_>>();
        if !slots.is_empty() {
            incoming.push(json!({"source":node.owner,"title":node.title,"reference":format!("weftext-node://{}/{}",node.owner.workspace_id,node.owner.node_id),"slots":slots,"visibility":"unconfirmed"}));
        }
    }
    if !bundle.citation_rename_available {
        incoming.clear();
    }
    let displayed = tree
        .map(crate::citation_api::displayed_citations)
        .unwrap_or_default();
    let links=knowledge::native_links(&selected.source).into_iter().filter(|link|link.kind=="native").map(|link|{let resolution=knowledge::resolve_link(&link.target,selected,&bundle.inventory);let title=resolution.target.and_then(|target|bundle.inventory.nodes.iter().find(|node|node.owner==target)).and_then(|node|node.title.clone());json!({"range":{"from":link.from,"to":link.to,"utf16From":link.utf16_from,"utf16To":link.utf16_to,"raw":selected.source.get(link.from..link.to).unwrap_or(""),"writable":true},"target":link.target,"editable":title.is_some()&&link.body_explicit,"title":title,"reason":"无可确认的获权标题或唯一可写正文；请保留源码。"})}).collect::<Vec<_>>();
    json!({"generation":bundle.inventory.generation,"coverage":if citation_complete{"complete"}else{"partial"},"choices":choices,"editor":editor,"bindings":groups,"incoming":incoming,"displayed":displayed,"links":links,"bibliographyOwnership":if bundle.citation_rename_available && crate::citation_api::owns_single_virtual_bibliography(&selected.source){"singleVirtual"}else{"unavailable"},"displayStatus":if tree.is_some(){"nativeCompileAvailable"}else{"unavailable"},"displayExplanation":"源码引用槽与本次 HTML 编译的显示项分别记录；仅绑定、未执行的条件和 bibliography(full: true) 不计可见正文引用。多个文献表或未知归属的 key 不猜接 Work。"})
}

async fn citation_products(
    state: &AppState,
    context: &Context,
    bundle: &Bundle,
) -> Result<
    (
        Option<weftext_engine::RenderNode>,
        Vec<weftext_engine::CompiledLabel>,
    ),
    ApiError,
> {
    let loc = location(state, &context.session_id)?;
    let environment = crate::writing_api::style_environment(state, &context.session_id).await?;
    if environment.key != bundle.style_generation {
        return Err(ApiError::bad(
            "原生样式环境已变化，请重新读取引用；输入保留",
        ));
    }
    let expected = bundle
        .files
        .iter()
        .map(|file| {
            (
                file.path.clone(),
                (file.bytes.clone(), file.node_id.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let original_bases = bundle
        .files
        .iter()
        .map(|file| (file.path.clone(), file.base_id.clone()))
        .collect::<Vec<_>>();
    let root = loc.root.clone();
    let path = loc.path.clone();
    let source = context.source.clone();
    let captured = core_call(state, move |core| {
        for (path, base) in original_bases {
            core.validate_snapshot(&root, path, &base)?;
        }
        let cut = core.admitted_compile_snapshot(&root, "live")?;
        for file in cut.snapshot.files.iter().chain(cut.snapshot.schema.iter()) {
            if expected.get(&file.path) != Some(&(file.bytes.clone(), file.node_id.clone())) {
                return Err(weftext_core::CoreError::new(
                    "citationCutChanged",
                    "引用目录与冻结编译不属于同次原生源依据",
                ));
            }
        }
        // Citation generation may use a self-Work's Draft, but the physical cut
        // and its original snapshot bytes remain the final recheck evidence.
        let mut citation_view = cut.clone();
        let citation = crate::citation_api::freeze_citations(&mut citation_view, &source, &path);
        Ok((cut, citation))
    })
    .await?;
    let (captured, citation) = captured;
    let citation = match citation {
        Ok(value) => value,
        Err(_) => return Ok((None, vec![])),
    };
    let mut files = draft_compile_files(
        &loc.root,
        &loc.path,
        &bundle.source_file,
        &captured.files,
        &context.source,
    )?;
    if let Some(citation) = &citation {
        files.insert(
            citation.virtual_path.clone(),
            citation.bibliography.bytes.clone(),
        );
    }
    let output = super::render_frozen_worker(
        state,
        weftext_engine::FrozenRenderRequest {
            source: context.source.clone(),
            path: loc.path.clone(),
            root: loc.root.clone(),
            style: captured.style.clone().or_else(|| environment.style.clone()),
            files,
        },
    )
    .await?;
    exact(state, context)?;
    crate::writing_api::check_style_environment(state, &context.session_id, &environment).await?;
    if output["ok"] != true {
        return Ok((None, vec![]));
    }
    let mut checks = Vec::new();
    let virtual_path = citation
        .as_ref()
        .map(|citation| citation.virtual_path.as_str());
    for dependency in output["dependencies"].as_array().into_iter().flatten() {
        let path = dependency["path"]
            .as_str()
            .ok_or_else(|| ApiError::bad("原生引用依赖没有准确地址"))?;
        if derived_render_input(dependency) {
            continue;
        }
        if dependency["status"] != "loaded"
            || Path::new(path).components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(ApiError::bad("原生引用依赖无法完整证明，原稿保留"));
        }
        if loc
            .root
            .join(path)
            .to_string_lossy()
            .eq_ignore_ascii_case(&loc.path.to_string_lossy())
        {
            continue;
        }
        let expected = if virtual_path == Some(path) {
            None
        } else {
            Some(
                captured
                    .files
                    .get(path)
                    .cloned()
                    .ok_or_else(|| ApiError::bad("原生引用依赖未进入冻结源范围"))?,
            )
        };
        checks.push((loc.root.join(path).to_string_lossy().into_owned(), expected));
    }
    let root = loc.root;
    let bases = captured
        .snapshot
        .files
        .iter()
        .chain(captured.snapshot.schema.iter())
        .map(|file| (file.path.clone(), file.base_id.clone()))
        .collect::<Vec<_>>();
    core_call(state, move |core| {
        for (path, base) in bases {
            core.validate_snapshot(&root, path, &base)?;
        }
        let checks = checks
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect::<Vec<_>>();
        for chunk in checks.chunks(64) {
            let paths = chunk
                .iter()
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>();
            let current = core.read_snapshot(&root, &paths)?;
            for (path, expected) in chunk {
                if current
                    .files
                    .iter()
                    .find(|file| &file.path == path)
                    .and_then(|file| file.bytes.as_ref())
                    != expected.as_ref()
                {
                    return Err(weftext_core::CoreError::new(
                        "citationDependencyChanged",
                        "引用编译依赖已变化，未交付旧位置或编号",
                    ));
                }
            }
        }
        Ok(())
    })
    .await?;
    exact(state, context)?;
    let tree = serde_json::from_value(output["tree"].clone()).ok();
    let labels = serde_json::from_value(output["labels"].clone()).unwrap_or_default();
    Ok((tree, labels))
}

fn native_citation_intent(
    source: &str,
    intent: &Value,
    bundle: &Bundle,
) -> Result<weftext_engine::EditOutput, ApiError> {
    let hash = weftext_engine::source_hash(source);
    if matches!(
        intent["kind"].as_str(),
        Some("nativeCitationRename" | "nativeReferenceRename")
    ) && !bundle.citation_rename_available
    {
        return Err(ApiError::bad(
            "当前库样式可能生成引用，无法证明完整改名范围；原稿保留",
        ));
    }
    if intent["generation"].as_str() != Some(bundle.inventory.generation.as_str()) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "stale_index",
            "引用目录或当前草稿已变化，请刷新后重试；输入保留",
        ));
    }
    let fail = |error: weftext_engine::EngineError| {
        ApiError::new(
            StatusCode::CONFLICT,
            "unsupported_citation_edit",
            error.to_string(),
        )
    };
    let range = || {
        serde_json::from_value::<weftext_engine::SourceRange>(intent["range"].clone())
            .map_err(|_| ApiError::bad("缺少准确原生引用范围"))
    };
    let form = || {
        serde_json::from_value::<weftext_engine::CitationForm>(intent["form"].clone())
            .map_err(|_| ApiError::bad("请选择准确引用形式"))
    };
    match text(intent, "kind")? {
        "nativeCitationInsert" => {
            let work = text(intent, "work")?;
            let node = bundle
                .inventory
                .nodes
                .iter()
                .find(|node| {
                    format!(
                        "weftext-node://{}/{}",
                        node.owner.workspace_id, node.owner.node_id
                    ) == work
                })
                .ok_or_else(|| ApiError::bad("所选 Work 当前不可用；请刷新目录"))?;
            let node = if node.owner == bundle.selected.owner {
                &bundle.selected
            } else {
                node
            };
            let projection = weftext_data::project(&node.source, &bundle.schema);
            if projection.qualification != "qualified"
                || !projection
                    .resolved_types
                    .iter()
                    .any(|kind| kind == "library/work")
            {
                return Err(ApiError::bad(
                    "目标不符合正式文献类型，请选择普通链接或完善出版数据",
                ));
            }
            let publication = projection
                .fields
                .iter()
                .find(|field| field.field_id == "library/publication")
                .and_then(|field| field.cell.native())
                .cloned()
                .ok_or_else(|| ApiError::bad("目标出版数据当前不可用"))?;
            weftext_export::bibliography::project_bibliography(
                &[weftext_export::bibliography::CitationWork {
                    key: "validation".into(),
                    work: weftext_export::bibliography::FrozenWork {
                        entity_ref: work.into(),
                        source_basis: "current-authorized-proposal".into(),
                        publication: publication.clone(),
                    },
                }],
                weftext_export::bibliography::BibliographyFormat::Bibtex,
            )
            .map_err(|error| ApiError::bad(error.to_string()))?;
            let suggestion = publication
                .member("citation_key")
                .and_then(weftext_data::NativeValue::text)
                .unwrap_or("work");
            weftext_engine::insert_native_citation(
                source,
                &hash,
                integer(intent, "at")?,
                work,
                suggestion,
                intent["key"].as_str().filter(|key| !key.is_empty()),
                form()?,
                intent["supplement"].as_str().unwrap_or(""),
            )
            .map_err(fail)
        }
        "nativeCitationUpdate" => weftext_engine::update_native_citation_options(
            source,
            &hash,
            &range()?,
            if intent["form"].is_null() {
                None
            } else {
                Some(form()?)
            },
            intent["supplement"].as_str(),
        )
        .map_err(fail),
        "nativeCitationStyle" => weftext_engine::update_native_citation_style(
            source,
            &hash,
            &range()?,
            intent["style"].as_str().filter(|value| !value.is_empty()),
        )
        .map_err(fail),
        "nativeCitationRename" => weftext_engine::rename_native_citation_key(
            source,
            &hash,
            text(intent, "old")?,
            text(intent, "new")?,
        )
        .map_err(fail),
        "nativeCitationLink" => {
            let reference = text(intent, "target")?;
            let target = bundle
                .inventory
                .nodes
                .iter()
                .find(|node| {
                    format!(
                        "weftext-node://{}/{}",
                        node.owner.workspace_id, node.owner.node_id
                    ) == reference
                })
                .ok_or_else(|| ApiError::bad("链接目标当前不可用，请刷新后重新选择"))?;
            let display = intent["text"]
                .as_str()
                .filter(|text| !text.is_empty())
                .or(target.title.as_deref())
                .ok_or_else(|| ApiError::bad("目标没有可确认标题，请明确输入显示文字"))?;
            weftext_engine::insert_native_link(
                source,
                &hash,
                integer(intent, "at")?,
                reference,
                display,
            )
            .map_err(fail)
        }
        "nativeCitationLinkText" => {
            let from = intent["occurrence"]["from"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| ApiError::bad("缺少准确链接范围"))?;
            let to = intent["occurrence"]["to"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| ApiError::bad("缺少准确链接范围"))?;
            let link = knowledge::native_links(source)
                .into_iter()
                .find(|link| {
                    link.from == from
                        && link.to == to
                        && intent["occurrence"]["target"] == link.target
                })
                .ok_or_else(|| ApiError::bad("当前准确链接出现项已变化"))?;
            let resolution =
                knowledge::resolve_link(&link.target, &bundle.selected, &bundle.inventory);
            let target = resolution
                .target
                .and_then(|target| {
                    bundle
                        .inventory
                        .nodes
                        .iter()
                        .find(|node| node.owner == target)
                })
                .ok_or_else(|| ApiError::bad("目标当前不可读，未交付旧标题或修改显示文字"))?;
            let range = weftext_engine::SourceRange {
                from,
                to,
                utf16_from: source[..from].encode_utf16().count(),
                utf16_to: source[..to].encode_utf16().count(),
                raw: source[from..to].into(),
                writable: true,
            };
            let title = target
                .title
                .as_deref()
                .ok_or_else(|| ApiError::bad("目标没有可确认的当前标题，未改变原显示文字"))?;
            weftext_engine::update_native_link_text(source, &hash, &range, title).map_err(fail)
        }
        "nativeReferenceInsert" => {
            let label = text(intent, "label")?;
            let form = text(intent, "form")?;
            if form == "text" {
                let editor = weftext_engine::inspect_native_citation_editor(source);
                if !editor
                    .targets
                    .iter()
                    .any(|target| target.label == label && target.unique)
                {
                    return Err(ApiError::bad("标签缺失或不唯一，请刷新后重新选择"));
                }
                weftext_engine::insert_native_cross_link(
                    source,
                    &hash,
                    integer(intent, "at")?,
                    label,
                    text(intent, "text")?,
                )
                .map_err(fail)
            } else {
                if !matches!(form, "normal" | "page") {
                    return Err(ApiError::bad("未知交叉引用形式"));
                }
                weftext_engine::insert_native_cross_reference(
                    source,
                    &hash,
                    integer(intent, "at")?,
                    label,
                    form == "page",
                )
                .map_err(fail)
            }
        }
        "nativeReferenceRename" => weftext_engine::rename_native_label(
            source,
            &hash,
            text(intent, "old")?,
            text(intent, "new")?,
        )
        .map_err(fail),
        "nativeReferenceInsertWithLabel" => {
            let target =
                serde_json::from_value::<weftext_engine::SourceRange>(intent["target"].clone())
                    .map_err(|_| ApiError::bad("缺少准确原生对象范围"))?;
            let form = text(intent, "form")?;
            if !matches!(form, "normal" | "page") {
                return Err(ApiError::bad(
                    "补标签同稿目前支持编号或页码引用，请在已有标签上插入自定义文字链接",
                ));
            }
            weftext_engine::insert_native_cross_reference_with_label(
                source,
                &hash,
                integer(intent, "at")?,
                &target,
                text(intent, "label")?,
                form == "page",
            )
            .map_err(fail)
        }
        _ => Err(ApiError::bad("未知原生引用操作")),
    }
}

fn native_citation_draft_plan(
    operation_id: &str,
    context: &Context,
    edited: weftext_engine::EditOutput,
) -> Value {
    let from = edited.patches.first().map_or(0, |patch| patch.utf16_from);
    let to = edited.patches.last().map_or(from, |patch| patch.utf16_to);
    let before_utf16 = context.source.encode_utf16().count();
    let after_utf16 = edited.source.encode_utf16().count();
    let inserted_end = after_utf16 - (before_utf16 - to);
    let replacement =
        String::from_utf16(&edited.source.encode_utf16().collect::<Vec<_>>()[from..inserted_end])
            .unwrap_or_default();
    json!({"operationId":operation_id,"planId":format!("draft:{}:{}",context.serial,weftext_engine::source_hash(&context.source)),"kind":"draftPatch","status":"prepared","draftOnly":true,"source":edited.source,"edit":{"from":from,"to":to,"text":replacement},"context":context,"changes":[{"action":"draftEdit"}],"preview":{"source":{"before":context.source,"after":edited.source},"nativePatches":edited.patches}})
}

fn draft_plan(operation_id: &str, context: &Context, patch: knowledge::SourcePatch) -> Value {
    json!({"operationId":operation_id,"planId":format!("draft:{}:{}",context.serial,weftext_engine::source_hash(&context.source)),"kind":"draftPatch","status":"prepared","draftOnly":true,"source":patch.after,"edit":{"from":patch.range.utf16_from,"to":patch.range.utf16_to,"text":patch.replacement},"context":context,"changes":[{"action":"draftEdit","before":patch.range.raw,"after":patch.replacement}]})
}
fn clean_source(context: &Context, bundle: &Bundle) -> Result<(), ApiError> {
    if context.dirty || context.source != bundle.committed.source {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "save_required",
            "这次批注操作需要已保存的准确原文；请先保存当前草稿，输入的讨论和建议会保留",
        ));
    }
    Ok(())
}

async fn prepare(State(state): State<AppState>, Json(request): Json<PrepareRequest>) -> ApiResult {
    observe(&state, &request.context)?;
    let kind = text(&request.intent, "kind")?;
    let loc = location(&state, &request.context.session_id)?;
    let canonical =
        json!({"root":loc.root,"path":loc.path,"context":request.context,"intent":request.intent});
    if !matches!(kind, "linkConvert" | "mentionConvert")
        && !kind.starts_with("nativeCitation")
        && !kind.starts_with("nativeReference")
    {
        let operation_id = request.operation_id.clone();
        let original = canonical.clone();
        if let Some(preview) = core_call(&state, move |core| {
            core.feature_operation_status(&operation_id, &original)
        })
        .await?
        {
            return frozen_feature_preview(&state, preview, &request.context)
                .await
                .map(Json);
        }
    }
    if !kind.starts_with("annotation")
        && !matches!(
            kind,
            "linkConvert" | "mentionConvert" | "templateCreate" | "templateRegister"
        )
        && !kind.starts_with("nativeCitation")
        && !kind.starts_with("nativeReference")
    {
        let loc = location(&state, &request.context.session_id)?;
        let intent: OperationIntent = serde_json::from_value(request.intent)
            .map_err(|error| ApiError::bad(error.to_string()))?;
        let op = OperationRequest {
            operation_id: request.operation_id,
            root: loc.root.to_string_lossy().into_owned(),
            intent,
        };
        let preview = core_call(&state, move |core| {
            core.prepare_feature_operation(op, &canonical)
        })
        .await?;
        exact(&state, &request.context)?;
        return frozen_feature_preview(&state, preview, &request.context)
            .await
            .map(Json);
    }
    if kind == "templateCreate" {
        return prepare_template(&state, request, canonical).await;
    }
    if kind == "templateRegister" {
        return register_template(&state, request, canonical).await;
    }
    let mut bundle = bundle(&state, &request.context).await?;
    if kind.starts_with("nativeCitation") || kind.starts_with("nativeReference") {
        if kind == "nativeReferenceInsertWithLabel" {
            let (tree, _) = citation_products(&state, &request.context, &bundle).await?;
            let tree = tree.ok_or_else(|| {
                ApiError::bad("请先等待当前原生预览完成，再选择准确对象；输入保留")
            })?;
            let target = &request.intent["target"];
            if request.intent["form"] == "normal"
                && !crate::citation_api::reference_numbered_at(
                    &tree,
                    target["from"].as_u64().unwrap_or(u64::MAX) as usize,
                )
                .unwrap_or(false)
            {
                return Err(ApiError::bad(
                    "目标当前没有获证的可引用编号；可改为页码引用，或先设置对象编号",
                ));
            }
            if !crate::citation_api::unlabelled_reference_targets(&request.context.source, &tree)
                .iter()
                .any(|candidate| candidate["range"] == *target)
            {
                return Err(ApiError::bad(
                    "目标没有本次唯一原生对象及准确来源证明，原稿保留",
                ));
            }
        }
        if kind == "nativeReferenceInsert" {
            let (tree, labels) = citation_products(&state, &request.context, &bundle).await?;
            let label = text(&request.intent, "label")?;
            let editor = weftext_engine::inspect_native_citation_editor(&request.context.source);
            let target = editor
                .targets
                .iter()
                .find(|target| {
                    target.label == label
                        && target.unique
                        && matches!(
                            target.kind.as_str(),
                            "heading" | "figure" | "table" | "equation" | "footnote"
                        )
                })
                .ok_or_else(|| ApiError::bad("目标标签缺失或有歧义"))?;
            if request.intent["form"] == "normal"
                && !tree
                    .as_ref()
                    .and_then(|tree| {
                        crate::citation_api::reference_numbered_at(tree, target.range.from)
                    })
                    .unwrap_or(false)
            {
                return Err(ApiError::bad(
                    "目标当前没有获证的可引用编号；请用自定义文字链接或适用的页码引用",
                ));
            }
            if !labels.iter().any(|compiled| {
                compiled.label == label
                    && compiled.count == 1
                    && compiled
                        .origins
                        .iter()
                        .any(|origin| origin.is_main && origin.from == target.range.from)
            }) {
                return Err(ApiError::bad(
                    "目标没有本次准确编译与唯一作者来源证明，请刷新预览再选择；原稿保留",
                ));
            }
        }
        let edited = native_citation_intent(&request.context.source, &request.intent, &bundle)?;
        crate::writing_api::validate_source(&edited.source, None)?;
        exact(&state, &request.context)?;
        return Ok(Json(native_citation_draft_plan(
            &request.operation_id,
            &request.context,
            edited,
        )));
    }
    if kind == "linkConvert" || kind == "mentionConvert" {
        if request.intent["generation"].as_str() != Some(bundle.inventory.generation.as_str()) {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "stale_index",
                "链接字典或当前草稿已变化，请重新读取；草稿保留",
            ));
        }
        let patch = if kind == "linkConvert" {
            let from = request.intent["occurrence"]["from"]
                .as_u64()
                .and_then(|from| usize::try_from(from).ok())
                .ok_or_else(|| ApiError::bad("缺少准确链接位置"))?;
            qualify_link_fragment(&state, &request.context, &mut bundle, from).await?;
            let form: knowledge::LinkForm = serde_json::from_value(request.intent["form"].clone())
                .map_err(|_| ApiError::bad("未知链接形式"))?;
            knowledge::convert_link_with_resource_fragments(
                &bundle.selected,
                &bundle.inventory,
                from,
                form,
                &resource_qualifications(&bundle),
            )
            .map_err(knowledge_error)?
        } else {
            populate_diagnostic_labels(&state, &request.context, &mut bundle).await?;
            let scan = mentions(&state, &request.context, &bundle)?;
            let fragment = text(&request.intent, "fragmentId")?;
            let from = integer(&request.intent, "startScalar")?;
            let to = integer(&request.intent, "endScalar")?;
            let hit = scan
                .fragments
                .iter()
                .find(|item| item.id == fragment)
                .and_then(|fragment| {
                    fragment
                        .hits
                        .iter()
                        .find(|hit| hit.start_scalar == from && hit.end_scalar == to)
                })
                .ok_or_else(|| ApiError::bad("当前预览没有这一准确候选范围"))?;
            let target = node_ref(&request.intent["target"])?;
            knowledge::convert_mention_fragment(
                &bundle.selected,
                &bundle.inventory,
                hit,
                &target,
                request.intent["fragment"].as_str(),
            )
            .map_err(knowledge_error)?
        };
        exact(&state, &request.context)?;
        return Ok(Json(draft_plan(
            &request.operation_id,
            &request.context,
            patch,
        )));
    }
    prepare_annotation(&state, request, bundle, canonical).await
}
async fn frozen_feature_preview(
    state: &AppState,
    preview: weftext_core::OperationPreview,
    context: &Context,
) -> Result<Value, ApiError> {
    let operation_id = preview.operation_id.clone();
    let files = core_call(state, move |core| {
        core.prepared_operation_files(&operation_id)
    })
    .await?;
    let loc = location(state, &context.session_id)?;
    let mut output = value(preview)?;
    output["context"] = json!(context);
    let summaries:Vec<_>=files.iter().map(|file| {
        let textual=Path::new(&file.path).extension().is_some_and(|extension|extension.eq_ignore_ascii_case("typ")||extension.eq_ignore_ascii_case("json"));
        json!({"path":file.path,"action":if file.before.as_ref()==Some(&file.after){"readDependency"}else if file.before.is_none(){"create"}else{"update"},"before":if textual{file.before.as_ref().and_then(|bytes|std::str::from_utf8(bytes).ok())}else{None},"after":if textual{std::str::from_utf8(&file.after).ok()}else{None},"bytes":file.after.len(),"binary":!textual})
    }).collect();
    output["preview"] = json!({"files":summaries});
    if let Some(file) = files.iter().find(|file| {
        file.path == loc.path.to_string_lossy() && file.before.as_ref() != Some(&file.after)
    }) {
        let after = std::str::from_utf8(&file.after)
            .map_err(|_| ApiError::internal("准备结果正文不是 UTF-8"))?;
        output["source"] = json!(after);
        output["edit"] = json!({"from":0,"to":context.source.encode_utf16().count(),"text":after});
    }
    exact(state, context)?;
    Ok(output)
}

fn utf16_byte(source: &str, offset: usize) -> Result<usize, ApiError> {
    let mut units = 0;
    for (byte, ch) in source.char_indices() {
        if units == offset {
            return Ok(byte);
        }
        units += ch.len_utf16();
        if units > offset {
            return Err(ApiError::bad("选区分割了 UTF-16 字符"));
        }
    }
    if units == offset {
        Ok(source.len())
    } else {
        Err(ApiError::bad("选区超出当前正文"))
    }
}
fn source_target(
    context: &Context,
    bundle: &Bundle,
    target: &Value,
) -> Result<knowledge::AnnotationTarget, ApiError> {
    match text(target, "kind")? {
        "whole_document" | "wholeDocument" => Ok(knowledge::AnnotationTarget::WholeDocument {
            node: bundle.committed.owner.clone(),
        }),
        "source_range" | "sourceRange" | "source_element" | "sourceElement" | "source_point" => {
            clean_source(context, bundle)?;
            if bundle.committed.revision_id.is_empty() {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "revision_required",
                    "此原文尚无 Core 验证的持久版本，请先保存原文再建立范围批注；讨论输入已保留",
                ));
            }
            let from = utf16_byte(&context.source, integer(target, "from")?)?;
            let to = utf16_byte(&context.source, integer(target, "to")?)?;
            let point = target["kind"] == "source_point";
            if !point && from >= to || point && from != to {
                return Err(ApiError::bad("范围批注需要非空选区"));
            }
            if point
                && !knowledge::writable_suggestion_range(
                    &context.source,
                    &knowledge::ExactRange::new(&context.source, from, to)
                        .map_err(knowledge_error)?,
                )
            {
                return Err(ApiError::bad("插入点须位于准确可写的普通正文中"));
            }
            let anchor = knowledge::source_anchor(
                &bundle.committed,
                from,
                to,
                target["visibleExcerpt"].as_str().unwrap_or_default().into(),
            )
            .map_err(knowledge_error)?;
            if matches!(
                target["kind"].as_str(),
                Some("source_element" | "sourceElement")
            ) {
                let role = text(target, "elementRole")?;
                let map = weftext_engine::inspect_editor(&context.source);
                let text_element = role == "text"
                    && map
                        .literal_regions
                        .iter()
                        .any(|region| region.range.from == from && region.range.to == to);
                let structure = map.structures.iter().any(|structure| {
                    structure.kind == role
                        && (structure.range.from == from && structure.range.to == to
                            || structure
                                .body
                                .as_ref()
                                .is_some_and(|body| body.from == from && body.to == to))
                });
                if !text_element && !structure {
                    return Err(ApiError::new(
                        StatusCode::CONFLICT,
                        "element_evidence",
                        "当前选区没有完整源元素的语法证明；请改用准确源范围，批注输入保留",
                    ));
                }
                Ok(knowledge::AnnotationTarget::SourceElement {
                    anchor,
                    element_role: role.into(),
                })
            } else {
                Ok(knowledge::AnnotationTarget::SourceRange { anchor })
            }
        }
        _ => Err(ApiError::new(
            StatusCode::CONFLICT,
            "resource_evidence_required",
            "媒体区域需要已读取资源版本及真实页面、像素或时基证据；当前输入保留，请先打开准确资源",
        )),
    }
}
fn new_record(
    target: knowledge::AnnotationTarget,
    body: String,
    purpose: String,
    replacement: Option<String>,
) -> knowledge::AnnotationRecord {
    let confirmed_revision = target
        .anchor()
        .map(|anchor| anchor.revision_id.clone())
        .unwrap_or_default();
    knowledge::AnnotationRecord {
        annotation_id: uuid::Uuid::new_v4().to_string(),
        reply_to: None,
        attribution: "本机作者".into(),
        created_at: format!(
            "unix:{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ),
        purpose,
        body,
        labels: Vec::new(),
        appearance: json!({}),
        review_state: "open".into(),
        target,
        suggestion: replacement.map(|replacement| knowledge::Suggestion {
            replacement,
            state: "pending".into(),
            confirmed_revision,
            requires_reconfirmation: false,
        }),
        target_history: Vec::new(),
        unknown: BTreeMap::new(),
    }
}
async fn prepare_annotation(
    state: &AppState,
    request: PrepareRequest,
    mut bundle: Bundle,
    canonical: Value,
) -> ApiResult {
    let document = annotation_document(&bundle.annotations_file)?;
    let kind = text(&request.intent, "kind")?;
    let mut source_patch = None;
    let mut changes = Vec::new();
    let mut media_target = None;
    if matches!(kind, "annotationCreate" | "annotationReattach")
        && matches!(
            request.intent["target"]["kind"].as_str(),
            Some(
                "whole_resource"
                    | "wholeResource"
                    | "image_region"
                    | "imageRegion"
                    | "pdf_region"
                    | "pdfRegion"
                    | "media_interval"
                    | "mediaInterval"
            )
        )
    {
        let input = &request.intent["target"];
        let resource = crate::media_api::qualified_resource(
            state,
            &request.context.session_id,
            text(input, "resource")?,
            text(input, "revision")?,
        )
        .await?;
        media_target = Some(target_from_resource(input, &resource)?);
        for file in [&resource.file, &resource.manifest] {
            let bytes = file
                .bytes
                .clone()
                .ok_or_else(|| ApiError::bad("已接纳资源缺少准确字节"))?;
            changes.push(FileChange {
                path: file.path.clone(),
                base_id: Some(file.base_id.clone()),
                before: Some(bytes.clone()),
                after: bytes,
            });
        }
    }
    if kind == "annotationProofread"
        || matches!(
            kind,
            "annotationCreate" | "annotationReattach" | "annotationReconfirm"
        ) && matches!(
            request.intent["target"]["kind"].as_str(),
            Some(
                "source_range"
                    | "sourceRange"
                    | "source_element"
                    | "sourceElement"
                    | "source_point"
            )
        )
    {
        clean_source(&request.context, &bundle)?;
        let loc = location(state, &request.context.session_id)?;
        let path = bundle.source_file.path.clone();
        let base_id = bundle.source_file.base_id.clone();
        let operation_id = request.operation_id.clone();
        let session_id = request.context.session_id.clone();
        let session_base = request.context.base_id.clone();
        bundle.committed.revision_id = core_call(state, move |core| {
            core.check_session_base(&session_id, &session_base)?;
            core.reserve_source_revision(loc.root, path, &base_id, &operation_id)
        })
        .await?;
        // Admission of an exact legacy observation and its range annotation
        // become visible at the same durable decision. No source bytes change.
        changes.push(FileChange {
            path: bundle.source_file.path.clone(),
            base_id: Some(bundle.source_file.base_id.clone()),
            before: bundle.source_file.bytes.clone(),
            after: bundle.committed.source.as_bytes().to_vec(),
        });
    }
    let after = match kind {
        "annotationProofread" => {
            clean_source(&request.context, &bundle)?;
            let options: knowledge::ProofreadOptions =
                serde_json::from_value(request.intent["options"].clone())
                    .map_err(|e| ApiError::bad(e.to_string()))?;
            let processing_dependency =
                validate_processing_origin(state, &request.context, &options).await?;
            let ids: BTreeSet<String> = serde_json::from_value(request.intent["findings"].clone())
                .map_err(|_| ApiError::bad("请选择当前校对结果"))?;
            if ids.is_empty() || ids.len() > 200 {
                return Err(ApiError::bad("每批请选择 1 至 200 条校对结果"));
            }
            let scan = proofreading_scan(state, &request.context, &bundle, &options)?;
            validate_processing_origin(state, &request.context, &options).await?;
            if let Some(document) = processing_dependency {
                let bytes = document
                    .bytes
                    .ok_or_else(|| ApiError::bad("原共享设置快照不可用"))?;
                changes.push(FileChange {
                    path: document.path,
                    base_id: Some(document.base_id),
                    before: Some(bytes.clone()),
                    after: bytes,
                });
            }
            if ids
                .iter()
                .any(|id| !scan.findings.iter().any(|f| &f.id == id))
            {
                return Err(ApiError::bad("所选校对结果已变化，请重新检查"));
            }
            let editor = weftext_engine::inspect_editor(&bundle.committed.source);
            let mut records = vec![];
            for finding in scan.findings.iter().filter(|f| ids.contains(&f.id)) {
                if existing_finding(&document, finding, &bundle.committed) {
                    continue;
                }
                let range = finding
                    .source_range
                    .as_ref()
                    .ok_or_else(|| ApiError::bad("所选结果没有准确作者源位置，请保留只读提示"))?;
                let target = source_target(
                    &request.context,
                    &bundle,
                    &json!({"kind":"source_range","from":range.utf16_from,"to":range.utf16_to,"visibleExcerpt":finding.excerpt}),
                )?;
                let executable = finding.executable
                    && editor.literal_regions.iter().any(|r| {
                        r.range.writable && r.range.from <= range.from && r.range.to >= range.to
                    });
                let mut record = new_record(
                    target,
                    format!(
                        "{}：{}{}",
                        finding.excerpt,
                        finding.message,
                        finding
                            .replacement
                            .as_ref()
                            .map(|r| format!("；建议：{} → {r}", finding.excerpt))
                            .unwrap_or_default()
                    ),
                    if executable { "suggestion" } else { "comment" }.into(),
                    if executable {
                        finding.replacement.clone()
                    } else {
                        None
                    },
                );
                record.attribution = "Weftext 校对".into();
                record.labels = vec!["proofreading".into(), finding.category.clone()];
                if request.intent["ignore"] == true {
                    record.review_state = "resolved".into();
                    if let Some(s) = &mut record.suggestion {
                        s.state = "rejected".into();
                    }
                }
                record.unknown.insert("proofreading".into(),json!({"ruleId":finding.rule_id,"ruleVersion":finding.rule_version,"category":finding.category,"ignored":request.intent["ignore"]==true}));
                records.push(record);
            }
            document.create_many(records).map_err(knowledge_error)?
        }
        "annotationCreate" => {
            let target = match media_target {
                Some(target) => target,
                None => source_target(&request.context, &bundle, &request.intent["target"])?,
            };
            let purpose = request.intent["purpose"]
                .as_str()
                .unwrap_or("comment")
                .to_owned();
            if !matches!(purpose.as_str(), "comment" | "mark" | "suggestion") {
                return Err(ApiError::bad("未知批注目的"));
            }
            let replacement = if purpose == "suggestion" {
                if target.anchor().is_none() {
                    return Err(ApiError::bad("文字建议需要已保存的准确原文选区"));
                }
                Some(text(&request.intent, "replacement")?.into())
            } else {
                None
            };
            document
                .create(new_record(
                    target,
                    text(&request.intent, "body")?.into(),
                    purpose,
                    replacement,
                ))
                .map_err(knowledge_error)?
        }
        "annotationReply" => {
            let id = text(&request.intent, "annotationId")?;
            let parent = document
                .records
                .iter()
                .find(|record| record.annotation_id == id)
                .ok_or_else(|| ApiError::bad("原讨论已变化"))?;
            let mut record = new_record(
                parent.target.clone(),
                text(&request.intent, "body")?.into(),
                "comment".into(),
                None,
            );
            record.reply_to = Some(id.into());
            document.reply(record).map_err(knowledge_error)?
        }
        "annotationResolve" => document
            .resolve(
                text(&request.intent, "annotationId")?,
                request.intent["resolved"]
                    .as_bool()
                    .ok_or_else(|| ApiError::bad("缺少讨论状态"))?,
            )
            .map_err(knowledge_error)?,
        "annotationReject" => document
            .reject(text(&request.intent, "annotationId")?)
            .map_err(knowledge_error)?,
        "annotationReattach" => document
            .reattach(
                text(&request.intent, "annotationId")?,
                match media_target {
                    Some(target) => target,
                    None => source_target(&request.context, &bundle, &request.intent["target"])?,
                },
            )
            .map_err(knowledge_error)?,
        "annotationReconfirm" => document
            .reconfirm(
                text(&request.intent, "annotationId")?,
                source_target(&request.context, &bundle, &request.intent["target"])?,
                text(&request.intent, "replacement")?.into(),
            )
            .map_err(knowledge_error)?,
        "annotationAccept" => {
            clean_source(&request.context, &bundle)?;
            let session_id = request.context.session_id.clone();
            let session_base = request.context.base_id.clone();
            core_call(state, move |core| {
                core.check_session_base(&session_id, &session_base)
            })
            .await?;
            let id = text(&request.intent, "annotationId")?;
            let anchor = document
                .records
                .iter()
                .find(|record| record.annotation_id == id)
                .and_then(|record| record.target.anchor())
                .ok_or_else(|| ApiError::bad("此建议缺少准确原文目标"))?;
            if !knowledge::writable_suggestion_range(&bundle.committed.source, &anchor.source_range)
            {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "source_entry",
                    "当前建议跨越代码或结构，缺少编辑器的安全文字写入证明；建议和草稿均保留",
                ));
            }
            let plan = knowledge::suggestion_plan(
                &document,
                &bundle.annotations_file.path,
                text(&request.intent, "annotationId")?,
                &bundle.committed,
            )
            .map_err(knowledge_error)?;
            let source = plan
                .files
                .iter()
                .find(|file| file.path == bundle.committed.path)
                .ok_or_else(|| ApiError::internal("建议缺少正文变动"))?;
            changes.push(FileChange {
                path: bundle.source_file.path.clone(),
                base_id: Some(bundle.source_file.base_id.clone()),
                before: bundle.source_file.bytes.clone(),
                after: source.after.clone(),
            });
            source_patch = Some(plan.source_patch);
            plan.files
                .into_iter()
                .find(|file| file.path == bundle.annotations_file.path)
                .ok_or_else(|| ApiError::internal("建议缺少独立批注变动"))?
                .after
        }
        _ => return Err(ApiError::bad("未知批注操作")),
    };
    changes.push(FileChange {
        path: bundle.annotations_file.path,
        base_id: Some(bundle.annotations_file.base_id),
        before: bundle.annotations_file.bytes,
        after,
    });
    let loc = location(state, &request.context.session_id)?;
    let operation = OperationRequest {
        operation_id: request.operation_id,
        root: loc.root.to_string_lossy().into_owned(),
        intent: OperationIntent::UpdateFiles { changes },
    };
    exact(state, &request.context)?;
    let plan = core_call(state, move |core| {
        core.prepare_feature_operation(operation, &canonical)
    })
    .await?;
    let mut preview = frozen_feature_preview(state, plan, &request.context).await?;
    exact(state, &request.context)?;
    preview["context"] = json!(request.context);
    if kind == "annotationReattach" || kind == "annotationReconfirm" {
        preview["preview"]["annotationId"] = request.intent["annotationId"].clone();
        preview["preview"]["targetAfter"] = request.intent["target"].clone();
        preview["preview"]["replacementAfter"] = request.intent["replacement"].clone();
        preview["preview"]["reconfirmation"] = json!(kind == "annotationReconfirm");
    }
    if let Some(patch) = source_patch {
        preview["preview"]["source"] = json!({"before":request.context.source,"after":patch.after});
        preview["preview"]["suggestion"] = json!({"annotationId":request.intent["annotationId"],"before":"pending","after":"accepted"});
        preview["source"] = json!(patch.after);
        preview["edit"] = json!({"from":patch.range.utf16_from,"to":patch.range.utf16_to,"text":patch.replacement});
    }
    Ok(Json(preview))
}

async fn prepare_operation(
    State(state): State<AppState>,
    Json(request): Json<OperationRequest>,
) -> ApiResult {
    let prepared = core_call(&state, move |core| core.capture_sync_operation(request)).await?;
    let preview = match prepared {
        weftext_core::sync::SyncOperationPreparation::Prepared(preview) => *preview,
        weftext_core::sync::SyncOperationPreparation::Native(captured) => {
            let outputs =
                super::sync_api::native_validation_outputs(&state, captured.render_inputs())
                    .await?;
            core_call(&state, move |core| {
                core.finish_sync_native_preparation(captured, outputs)
            })
            .await?
        }
    };
    Ok(Json(value(preview)?))
}

#[derive(Deserialize)]
struct TrashRequest {
    root: String,
}
async fn read_trash(State(state): State<AppState>, Json(request): Json<TrashRequest>) -> ApiResult {
    let entries = core_call(&state, move |core| core.trash_entries(request.root)).await?;
    Ok(Json(json!({"trash":entries})))
}
async fn commit(State(state): State<AppState>, Json(request): Json<CommitRequest>) -> ApiResult {
    if let Some(context) = &request.context {
        exact(&state, context)?;
    }
    let context = request.context;
    let (preview, receipt) = core_call(&state, move |core| {
        let preview = core.operation_details(&request.operation_id)?;
        let receipt = match core.execute_operation(&request.operation_id, &request.plan_id) {
            Ok(receipt) => receipt,
            Err(error)
                if core
                    .operation_details(&request.operation_id)
                    .is_ok_and(|current| current.status == "recoveryPending") =>
            {
                return Err(weftext_core::CoreError::new(
                    "unknown",
                    format!(
                        "原操作 {} 的部分步骤可能已有作用，请检查原操作结果；草稿保留。{}",
                        request.operation_id, error.message
                    ),
                ));
            }
            Err(error) => return Err(error),
        };
        Ok((preview, receipt))
    })
    .await?;
    operation_output(&state, preview, receipt, context.as_ref())
        .await
        .map(Json)
}
fn canonical_leaf(directory: &Path) -> Result<PathBuf, ApiError> {
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ApiError::bad("操作结果缺少准确节点名称"))?;
    Ok(directory.join(format!("{name}.typ")))
}
pub(crate) async fn operation_output(
    state: &AppState,
    preview: weftext_core::OperationPreview,
    receipt: weftext_core::OperationReceipt,
    context: Option<&Context>,
) -> Result<Value, ApiError> {
    let mut output = value(&receipt)?;
    if let Err(error) = operation_delivery(state, &preview, &receipt, context, &mut output).await {
        output["documentError"] = json!({"code":error.code,"message":error.message});
    }
    Ok(output)
}
async fn operation_delivery(
    state: &AppState,
    preview: &weftext_core::OperationPreview,
    receipt: &weftext_core::OperationReceipt,
    context: Option<&Context>,
    output: &mut Value,
) -> Result<(), ApiError> {
    output["kind"] = json!(preview.kind);
    if let Some(source) = preview
        .changes
        .iter()
        .find(|change| change.action == "relocateFrom")
    {
        output["affectedSource"] = json!(source.path);
    }
    let loc = context.and_then(|context| location(state, &context.session_id).ok());
    let result = PathBuf::from(&receipt.path);
    let expected_source = if preview.kind == "updateFiles" {
        let operation_id = receipt.operation_id.clone();
        let files = core_call(state, move |core| {
            core.prepared_operation_files(&operation_id)
        })
        .await?;
        loc.as_ref()
            .and_then(|loc| {
                files
                    .iter()
                    .find(|file| file.path == loc.path.to_string_lossy())
            })
            .and_then(|file| String::from_utf8(file.after.clone()).ok())
    } else {
        None
    };
    if let Some(expected) = &expected_source {
        output["expectedSource"] = json!(expected);
        output["committedSource"] = json!(expected);
    }
    let destination = match preview.kind.as_str() {
        "newNode" | "copyNode" | "templateNode" | "restoreNode" => {
            Some((canonical_leaf(&result)?, "created"))
        }
        "renameNode" | "moveNode" | "relocateDirectory" => {
            let source = preview
                .changes
                .iter()
                .find(|change| change.action == "relocateFrom")
                .map(|change| PathBuf::from(&change.path));
            if let (Some(loc), Some(source)) = (&loc, source) {
                if let Ok(relative) = loc.path.strip_prefix(&source) {
                    let path = if preview.kind != "relocateDirectory"
                        && loc.path == canonical_leaf(&source)?
                    {
                        canonical_leaf(&result)?
                    } else {
                        result.join(relative)
                    };
                    Some((path, "rebound"))
                } else {
                    None
                }
            } else if preview.kind != "relocateDirectory" {
                Some((canonical_leaf(&result)?, "rebound"))
            } else {
                None
            }
        }
        "trashNode" => {
            output["documentDisposition"] = json!("trashed");
            None
        }
        "updateFiles" => loc
            .filter(|loc| {
                preview
                    .changes
                    .iter()
                    .any(|change| change.path == loc.path.to_string_lossy())
            })
            .map(|loc| (loc.path, "rebound")),
        _ => None,
    };
    if let Some((path, disposition)) = destination {
        output["documentDisposition"] = json!(disposition);
        let id = receipt.operation_id.clone();
        match core_call(state, move |core| {
            core.reopen_local_operation_node(&id, &path.to_string_lossy())
        })
        .await
        {
            Ok(document) => {
                let document = value(document)?;
                remember(state, &document)?;
                if let Some(expected) = &expected_source {
                    let matches = document["source"].as_str() == Some(expected.as_str());
                    output["documentSourceMatchesCommit"] = json!(matches);
                    if !matches {
                        output["externalChanged"] = json!(true);
                        output["documentError"] = json!({"code":"externalChanged","message":"原操作已提交，但正文随后已变化；请保留当前草稿并比较准确磁盘版本"});
                    }
                }
                output["document"] = document;
            }
            Err(error) => {
                output["documentError"] = json!({"code":error.code,"message":error.message});
            }
        }
    }
    if matches!(
        preview.kind.as_str(),
        "renameNode" | "moveNode" | "trashNode" | "relocateDirectory"
    ) && let Some(source) = preview
        .changes
        .iter()
        .find(|change| change.action == "relocateFrom")
        .map(|change| PathBuf::from(&change.path))
    {
        let locations: Vec<_> = state
            .locations
            .lock()
            .map_err(|_| ApiError::internal("会话暂不可用"))?
            .iter()
            .filter(|(id, loc)| {
                loc.path.starts_with(&source)
                    && context.is_none_or(|context| context.session_id != **id)
            })
            .map(|(id, loc)| (id.clone(), loc.clone()))
            .collect();
        let mut affected = Vec::new();
        for (id, loc) in locations {
            if preview.kind == "trashNode" {
                affected.push(json!({"priorSessionId":id,"documentDisposition":"trashed","priorPath":loc.path}));
                continue;
            }
            let relative = loc
                .path
                .strip_prefix(&source)
                .map_err(|_| ApiError::internal("移动会话范围不一致"))?;
            let path =
                if preview.kind != "relocateDirectory" && loc.path == canonical_leaf(&source)? {
                    canonical_leaf(&result)?
                } else {
                    result.join(relative)
                };
            let operation_id = receipt.operation_id.clone();
            match core_call(state,move|core|core.reopen_local_operation_node(&operation_id,&path.to_string_lossy())).await {
                Ok(document)=>{let document=value(document)?;remember(state,&document)?;affected.push(json!({"priorSessionId":id,"documentDisposition":"rebound","document":document}));},
                Err(error)=>affected.push(json!({"priorSessionId":id,"documentDisposition":"rebound","documentError":{"code":error.code,"message":error.message}})),
            }
        }
        output["affectedDocuments"] = json!(affected);
    }
    Ok(())
}
#[cfg(all(test, windows))]
#[path = "knowledge_directory_tests.rs"]
mod directory_result_tests;
#[derive(Deserialize)]
struct LinkCopyRequest {
    context: Context,
    occurrence: Value,
    form: knowledge::LinkForm,
}
async fn link_copy(
    State(state): State<AppState>,
    Json(request): Json<LinkCopyRequest>,
) -> ApiResult {
    observe(&state, &request.context)?;
    let mut bundle = bundle(&state, &request.context).await?;
    let from = integer(&request.occurrence, "from")?;
    qualify_link_fragment(&state, &request.context, &mut bundle, from).await?;
    let links: Vec<_> = knowledge::native_links(&bundle.selected.source)
        .into_iter()
        .filter(|link| link.from == from)
        .collect();
    let [link] = links.as_slice() else {
        return Err(ApiError::bad("请选择一条准确的原生链接"));
    };
    let start = if link.kind == "native"
        && link.from > 0
        && request.context.source.as_bytes().get(link.from - 1) == Some(&b'#')
    {
        link.from - 1
    } else {
        link.from
    };
    let literal = request
        .context
        .source
        .get(start..link.to)
        .ok_or_else(|| ApiError::bad("链接范围已经变化"))?;
    let fragments = resource_qualifications(&bundle);
    let resolution = knowledge::resolve_link_with_resource_fragments(
        &link.target,
        &bundle.selected,
        &bundle.inventory,
        &fragments,
    );
    if resolution.status != "resolved" {
        exact(&state, &request.context)?;
        return Ok(Json(
            json!({"address":link.target,"fullLink":literal,"literal":true,"status":resolution.status}),
        ));
    }
    let patch = knowledge::convert_link_with_resource_fragments(
        &bundle.selected,
        &bundle.inventory,
        from,
        request.form,
        &fragments,
    )
    .map_err(knowledge_error)?;
    let end = link
        .to
        .checked_sub(patch.range.to - patch.range.from)
        .and_then(|end| end.checked_add(patch.replacement.len()))
        .ok_or_else(|| ApiError::bad("复制链接范围溢出"))?;
    let full_link = patch
        .after
        .get(start..end)
        .ok_or_else(|| ApiError::bad("复制链接结果不在准确范围"))?;
    let converted = knowledge::native_links(&patch.after)
        .into_iter()
        .find(|item| item.from == link.from || item.from == start + 1)
        .ok_or_else(|| ApiError::bad("复制链接结果缺少准确原生地址"))?;
    let address = converted.target;
    let proof = knowledge::resolve_link_with_resource_fragments(
        &address,
        &bundle.selected,
        &bundle.inventory,
        &fragments,
    );
    if proof.status != "resolved"
        || proof.target != resolution.target
        || proof.resource != resolution.resource
        || proof.fragment != resolution.fragment
    {
        return Err(ApiError::bad("复制地址未返回同一准确目标"));
    }
    exact(&state, &request.context)?;
    Ok(Json(
        json!({"address":address,"fullLink":full_link,"literal":false,"status":"resolved"}),
    ))
}
async fn status(State(state): State<AppState>, Json(request): Json<StatusRequest>) -> ApiResult {
    let id = request.operation_id.clone();
    let (fact, transaction) = core_call(&state, move |core| {
        let fact = core.local_operation_status(&id)?;
        let transaction = if fact["status"] == "committed" {
            match core.operation_details(&id) {
                Ok(preview) => Some((
                    preview,
                    core.committed_operation_receipt(&id)?.ok_or_else(|| {
                        weftext_core::CoreError::new(
                            "invalidState",
                            "Committed original has no receipt",
                        )
                    })?,
                )),
                Err(error) if error.code == "operationMissing" => None,
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        Ok((fact, transaction))
    })
    .await?;
    if transaction
        .as_ref()
        .is_some_and(|(preview, _)| preview.kind == "createLibrary")
    {
        let id = request.operation_id.clone();
        let receipt = core_call(&state, move |core| core.local_library_creation_info(&id)).await?;
        return super::library_creation_output(&state, receipt)
            .await
            .map(Json);
    }
    match transaction {
        Some((preview, receipt)) => {
            operation_output(&state, preview, receipt, request.context.as_ref())
                .await
                .map(Json)
        }
        None => {
            if fact["status"] == "committed" && fact["baseId"].is_string() {
                Ok(Json(
                    file_save_delivery(&state, request.operation_id, fact).await,
                ))
            } else {
                Ok(Json(fact))
            }
        }
    }
}
pub(crate) async fn file_save_delivery(state: &AppState, id: String, mut output: Value) -> Value {
    let delivered = async {
        let (source, document) = core_call(state, move |core| core.reopen_local_save(&id)).await?;
        output["committedSource"] = json!(source);
        let document = value(document)?;
        output["documentSourceMatchesCommit"] =
            json!(document["source"] == output["committedSource"]);
        remember(state, &document)?;
        output["document"] = document;
        Ok::<(), ApiError>(())
    }
    .await;
    if let Err(error) = delivered {
        output["documentError"] = json!({"code":error.code,"message":error.message});
    }
    output
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettleRequest {
    operation_id: String,
    request_kind: String,
    wire_digest: String,
}
async fn settle(State(state): State<AppState>, Json(request): Json<SettleRequest>) -> ApiResult {
    if request.wire_digest.len() != 64
        || !request.wire_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(ApiError::bad("原请求字节摘要必须是 64 位十六进制 SHA-256"));
    }
    let fact = core_call(&state, move |core| {
        core.settle_local_attempt(
            &request.operation_id,
            &request.request_kind,
            &request.wire_digest,
        )
    })
    .await?;
    Ok(Json(fact))
}

const TEMPLATE_SOURCE: &str = "#metadata((id: \"00000000-0000-4000-8000-000000000000\", types: ())) <weftext>\n\n#set document(title: [标题])\n#title()\n\n正文\n";

/// Overlay one already admitted main file in a disposable compiler view. The
/// original SnapshotFile, Core bases and every other frozen input stay intact.
fn draft_compile_files(
    root: &Path,
    path: &Path,
    admitted: &SnapshotFile,
    files: &BTreeMap<String, Vec<u8>>,
    source: &str,
) -> Result<BTreeMap<String, Vec<u8>>, ApiError> {
    let fail = || ApiError::bad("当前草稿没有唯一的已接纳主源编译位置；原稿与输入保留");
    fn key(text: &str) -> Option<String> {
        let key = text.replace('\\', "/");
        if key.is_empty()
            || key.chars().any(|c| c.is_control() || c == ':')
            || key
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
            || Path::new(&key)
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return None;
        }
        Some(key)
    }
    if !root.is_absolute()
        || !path.is_absolute()
        || Path::new(&admitted.path) != path
        || admitted.node_id.is_none()
    {
        return Err(fail());
    }
    let relative = path.strip_prefix(root).map_err(|_| fail())?;
    let main = key(relative.to_str().ok_or_else(fail)?).ok_or_else(fail)?;
    let original = admitted.bytes.as_ref().ok_or_else(fail)?;
    let mut seen = BTreeSet::new();
    let mut main_key = None;
    for (raw, bytes) in files {
        let normalized = key(raw).ok_or_else(fail)?;
        if !seen.insert(normalized.to_ascii_lowercase()) {
            return Err(fail());
        }
        if normalized == main {
            if bytes != original {
                return Err(fail());
            }
            main_key = Some(raw.clone());
        }
    }
    let main_key = main_key.ok_or_else(fail)?;
    let mut compiled = files.clone();
    compiled.insert(main_key, source.as_bytes().to_vec());
    Ok(compiled)
}

fn frozen_map(
    root: &Path,
    files: &[SnapshotFile],
    replaced: Option<&Path>,
) -> Result<BTreeMap<String, Vec<u8>>, ApiError> {
    let mut out = BTreeMap::new();
    for file in files {
        if replaced.is_some_and(|path| path == Path::new(&file.path)) {
            continue;
        }
        let Some(bytes) = &file.bytes else {
            continue;
        };
        let relative = Path::new(&file.path)
            .strip_prefix(root)
            .map_err(|_| ApiError::bad("冻结编译输入不在资料库内"))?
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(previous) = out.insert(relative, bytes.clone())
            && previous != *bytes
        {
            return Err(ApiError::bad("冻结输入同一路径含有不同事实"));
        }
    }
    Ok(out)
}
// The private worker marks only its actual producer-owned FileId. An authored
// file with a similar name must still participate in ordinary snapshot checks.
fn derived_render_input(dependency: &Value) -> bool {
    dependency["status"] == "loaded"
        && matches!(
            dependency["kind"].as_str(),
            Some("derived-source" | "derived-file")
        )
}
async fn validate_render_inputs(
    state: &AppState,
    root: &Path,
    files: &[SnapshotFile],
    output: &Value,
    replaced: Option<&Path>,
) -> Result<Vec<String>, ApiError> {
    let mut bases = BTreeSet::new();
    let mut qualified = Vec::new();
    for dependency in output["dependencies"].as_array().into_iter().flatten() {
        let Some(path) = dependency["path"].as_str() else {
            return Err(ApiError::bad("编译依赖缺少准确位置"));
        };
        if derived_render_input(dependency) {
            continue;
        }
        let full = root.join(path);
        if replaced.is_some_and(|path| path == full) {
            continue;
        }
        let file = files
            .iter()
            .find(|file| Path::new(&file.path) == full)
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::CONFLICT,
                    "template_dependency",
                    "模板含有未被准确读取的原生依赖；重新准备后再创建",
                )
            })?;
        if dependency["status"] != "loaded" {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "template_dependency",
                "模板依赖在冻结范围内不可用；原文与参数保留",
            ));
        }
        if bases.insert(file.base_id.clone()) {
            qualified.push((file.path.clone(), file.base_id.clone()));
        }
    }
    let ids = bases.into_iter().collect();
    let root = root.to_path_buf();
    core_call(state, move |core| {
        for (path, base) in qualified {
            core.validate_snapshot(&root, path, &base)?;
        }
        Ok(())
    })
    .await?;
    Ok(ids)
}
fn builtin_template_catalog() -> Value {
    json!([{"id":"native-note","title":"原生笔记","description":"本机原生 Typst 模板；标题和正文通过准确文字槽填入。","source":TEMPLATE_SOURCE,"parameters":[{"name":"title","title":"标题","type":"text","required":true},{"name":"body","title":"正文","type":"text","default":""}]}])
}
/// Reject duplicate JSON members at every depth before preserving unknown
/// configuration fields. A last-wins parse cannot safely qualify a write.
struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate object members")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(json!(v)))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueJson(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(UniqueJson(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!("duplicate member {key}")));
                    }
                    let UniqueJson(v) = map.next_value()?;
                    values.insert(key, v);
                }
                Ok(UniqueJson(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
pub(crate) fn template_registry(
    file: &SnapshotFile,
) -> Result<(Value, Vec<knowledge::TemplateDefinition>), ApiError> {
    let settings = file
        .bytes
        .as_ref()
        .map(|bytes| serde_json::from_slice::<UniqueJson>(bytes).map(|v| v.0))
        .transpose()
        .map_err(|error| {
            ApiError::new(
                StatusCode::CONFLICT,
                "template_settings",
                format!("模板设置 JSON 不可验证：{error}；原始字节保留"),
            )
        })?
        .unwrap_or_else(|| json!({"version":1}));
    if !settings.is_object() {
        return Err(ApiError::bad("资料库设置必须是对象；原始字节保留"));
    }
    let definitions: Vec<knowledge::TemplateDefinition> = match settings.get("templates") {
        None => vec![],
        Some(registry) if registry["version"] == 1 => {
            serde_json::from_value(registry["entries"].clone()).map_err(|error| {
                ApiError::bad(format!("模板登记格式不可验证：{error}；原始字节保留"))
            })?
        }
        Some(_) => return Err(ApiError::bad("模板登记版本不可验证；原始字节保留")),
    };
    let mut ids = BTreeSet::new();
    if definitions.len() > 256
        || definitions.iter().any(|definition| {
            definition.id == "native-note"
                || definition.id.is_empty()
                || !ids.insert(&definition.id)
                || definition.slots.len() > 32
        })
    {
        return Err(ApiError::bad("模板登记身份重复或超出 256 模板/32 参数上限"));
    }
    Ok((settings, definitions))
}
pub(crate) fn template_value(value: &knowledge::TemplateValue) -> Value {
    match value {
        knowledge::TemplateValue::Null => Value::Null,
        knowledge::TemplateValue::Text(v) => json!(v),
        knowledge::TemplateValue::Bool(v) => json!(v),
        knowledge::TemplateValue::Int(v) => json!(v.to_string()),
    }
}
fn template_dto(
    definition: &knowledge::TemplateDefinition,
    inventory: &knowledge::InventorySnapshot,
) -> Value {
    let mut source_node = json!(definition.source_node);
    enrich_ref(&mut source_node, inventory);
    let available = inventory
        .nodes
        .iter()
        .find(|node| node.owner == definition.source_node)
        .is_some_and(|node| knowledge::validate_template_definition(definition, node).is_ok());
    let parameters:Vec<_>=definition.slots.iter().map(|slot|{let mut parameter=json!({"name":slot.name,"kind":slot.kind,"nullable":slot.nullable,"required":slot.default.is_none(),"from":slot.range.utf16_from,"to":slot.range.utf16_to});if let Some(default)=&slot.default {parameter["default"]=template_value(default);}parameter}).collect();
    json!({"id":definition.id,"title":definition.title,"description":definition.description,"sourceNode":source_node,"sourceRevision":definition.source_revision,"available":available,"parameters":parameters})
}
fn template_catalog(bundle: &Bundle) -> Result<Value, ApiError> {
    let (_, definitions) = template_registry(&bundle.settings)?;
    let mut catalog = builtin_template_catalog()
        .as_array()
        .cloned()
        .unwrap_or_default();
    catalog.extend(
        definitions
            .iter()
            .map(|definition| template_dto(definition, &bundle.inventory)),
    );
    Ok(json!(catalog))
}
pub(crate) fn parse_template_value(
    input: &Value,
    kind: knowledge::TemplateSlotKind,
) -> Result<knowledge::TemplateValue, ApiError> {
    if input.is_null() {
        return Ok(knowledge::TemplateValue::Null);
    }
    Ok(match kind {
        knowledge::TemplateSlotKind::Markup | knowledge::TemplateSlotKind::String => {
            knowledge::TemplateValue::Text(
                input
                    .as_str()
                    .ok_or_else(|| ApiError::bad("模板文字参数必须是字符串"))?
                    .into(),
            )
        }
        knowledge::TemplateSlotKind::Bool => knowledge::TemplateValue::Bool(
            input
                .as_bool()
                .ok_or_else(|| ApiError::bad("模板布尔参数必须是布尔值"))?,
        ),
        knowledge::TemplateSlotKind::Int => knowledge::TemplateValue::Int(
            input
                .as_str()
                .ok_or_else(|| ApiError::bad("模板整数必须是十进制字符串，以保留准确 i64"))?
                .parse::<i64>()
                .map_err(|_| ApiError::bad("模板整数超出准确 i64 范围"))?,
        ),
    })
}
async fn register_template(
    state: &AppState,
    request: PrepareRequest,
    canonical: Value,
) -> ApiResult {
    let mut bundle = bundle(state, &request.context).await?;
    clean_source(&request.context, &bundle)?;
    let loc = location(state, &request.context.session_id)?;
    let session = request.context.session_id.clone();
    let base = request.context.base_id.clone();
    let root = loc.root.clone();
    let path = bundle.source_file.path.clone();
    let source_base = bundle.source_file.base_id.clone();
    let operation_id = request.operation_id.clone();
    let revision = core_call(state, move |core| {
        core.check_session_base(&session, &base)?;
        core.reserve_source_revision(root, path, &source_base, &operation_id)
    })
    .await?;
    bundle.committed.revision_id = revision;
    let (mut settings, mut definitions) = template_registry(&bundle.settings)?;
    let title = text(&request.intent, "title")?.trim().to_string();
    let description = request.intent["description"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if title.is_empty() || title.len() > 1024 || description.len() > 8192 {
        return Err(ApiError::bad("模板名称为空或文字超出上限"));
    }
    let inputs = request.intent["parameters"]
        .as_array()
        .ok_or_else(|| ApiError::bad("缺少准确模板参数槽"))?;
    if inputs.len() > 32 {
        return Err(ApiError::bad("每个模板最多 32 个准确参数槽"));
    }
    let mut slots = Vec::new();
    for input in inputs {
        let kind: knowledge::TemplateSlotKind = serde_json::from_value(input["kind"].clone())
            .map_err(|_| ApiError::bad("未知模板参数类型"))?;
        let from = utf16_byte(&request.context.source, integer(input, "from")?)?;
        let to = utf16_byte(&request.context.source, integer(input, "to")?)?;
        slots.push(knowledge::TemplateSlot {
            name: text(input, "name")?.into(),
            range: knowledge::ExactRange::new(&request.context.source, from, to)
                .map_err(knowledge_error)?,
            kind,
            default: input
                .get("default")
                .map(|value| parse_template_value(value, kind))
                .transpose()?,
            nullable: input["nullable"].as_bool().unwrap_or(false),
        });
    }
    let id = request.intent["templateId"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let existing = definitions
        .iter()
        .position(|definition| definition.id == id);
    if request.intent.get("templateId").is_some() && existing.is_none() {
        return Err(ApiError::bad("待更新模板不存在"));
    }
    if existing.is_some_and(|index| definitions[index].source_node != bundle.committed.owner) {
        return Err(ApiError::bad("只能从原登记源节点更新同一模板"));
    }
    let definition = knowledge::TemplateDefinition {
        id,
        title,
        description,
        source_node: bundle.committed.owner.clone(),
        source_revision: bundle.committed.revision_id.clone(),
        slots,
    };
    knowledge::validate_template_definition(&definition, &bundle.committed)
        .map_err(knowledge_error)?;
    let before = existing.map(|index| template_dto(&definitions[index], &bundle.inventory));
    let mut preview_inventory = bundle.inventory.clone();
    if let Some(node) = preview_inventory
        .nodes
        .iter_mut()
        .find(|node| node.owner == bundle.committed.owner)
    {
        *node = bundle.committed.clone();
    }
    let after = template_dto(&definition, &preview_inventory);
    if let Some(index) = existing {
        definitions[index] = definition;
    } else {
        definitions.push(definition);
    }
    if definitions.len() > 256 {
        return Err(ApiError::bad("登记模板达到 256 项上限"));
    }
    settings["templates"] = json!({"version":1,"entries":definitions});
    let operation = OperationRequest {
        operation_id: request.operation_id,
        root: loc.root.to_string_lossy().into_owned(),
        intent: OperationIntent::UpdateFiles {
            changes: vec![
                FileChange {
                    path: bundle.source_file.path,
                    base_id: Some(bundle.source_file.base_id),
                    before: bundle.source_file.bytes.clone(),
                    after: bundle.source_file.bytes.unwrap_or_default(),
                },
                FileChange {
                    path: bundle.settings.path,
                    base_id: Some(bundle.settings.base_id),
                    before: bundle.settings.bytes,
                    after: serde_json::to_vec_pretty(&settings)
                        .map_err(|_| ApiError::internal("无法编码模板设置"))?,
                },
            ],
        },
    };
    exact(state, &request.context)?;
    let plan = core_call(state, move |core| {
        core.prepare_feature_operation(operation, &canonical)
    })
    .await?;
    let mut preview = frozen_feature_preview(state, plan, &request.context).await?;
    preview["preview"]["templateBefore"] = json!(before);
    preview["preview"]["templateAfter"] = after;
    Ok(Json(preview))
}
pub(crate) fn template_source(intent: &Value) -> Result<String, ApiError> {
    if text(intent, "templateId")? != "native-note" {
        return Err(ApiError::bad("未知原生模板"));
    }
    let slots: Vec<_> = [
        ("title", "标题", None),
        (
            "body",
            "正文",
            Some(knowledge::TemplateValue::Text(String::new())),
        ),
    ]
    .into_iter()
    .map(|(name, raw, default)| {
        let from = TEMPLATE_SOURCE.find(raw).expect("built-in native slot");
        knowledge::TemplateSlot {
            name: name.into(),
            range: knowledge::ExactRange::new(TEMPLATE_SOURCE, from, from + raw.len())
                .expect("built-in exact range"),
            kind: knowledge::TemplateSlotKind::Markup,
            default,
            nullable: false,
        }
    })
    .collect();
    let mut parameters = BTreeMap::new();
    let inputs = intent["parameters"]
        .as_object()
        .ok_or_else(|| ApiError::bad("缺少模板参数"))?;
    for (name, input) in inputs {
        parameters.insert(
            name.clone(),
            if input.is_null() {
                knowledge::TemplateValue::Null
            } else {
                knowledge::TemplateValue::Text(
                    input
                        .as_str()
                        .ok_or_else(|| ApiError::bad("模板文字参数类型错误"))?
                        .into(),
                )
            },
        );
    }
    knowledge::instantiate_template(TEMPLATE_SOURCE, &slots, &parameters).map_err(knowledge_error)
}
fn configured_template_source(
    intent: &Value,
    bundle: &Bundle,
) -> Result<(String, Option<knowledge::NodeSnapshot>), ApiError> {
    let id = text(intent, "templateId")?;
    if id == "native-note" {
        return Ok((template_source(intent)?, None));
    }
    let (_, definitions) = template_registry(&bundle.settings)?;
    let definition = definitions
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| ApiError::bad("模板登记不存在"))?;
    let node = bundle
        .inventory
        .nodes
        .iter()
        .find(|node| node.owner == definition.source_node)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "staleTemplate",
                "模板源节点不可用；参数保留",
            )
        })?;
    knowledge::validate_template_definition(definition, node).map_err(knowledge_error)?;
    let inputs = intent["parameters"]
        .as_object()
        .ok_or_else(|| ApiError::bad("缺少模板参数"))?;
    let mut parameters = BTreeMap::new();
    for (name, input) in inputs {
        let slot = definition
            .slots
            .iter()
            .find(|slot| slot.name == *name)
            .ok_or_else(|| ApiError::bad("输入不对应登记的准确参数槽"))?;
        parameters.insert(name.clone(), parse_template_value(input, slot.kind)?);
    }
    let source = knowledge::instantiate_template(&node.source, &definition.slots, &parameters)
        .map_err(knowledge_error)?;
    Ok((source, Some(node.clone())))
}
async fn prepare_template(
    state: &AppState,
    request: PrepareRequest,
    canonical: Value,
) -> ApiResult {
    let bundle = bundle(state, &request.context).await?;
    let (source, original) = configured_template_source(&request.intent, &bundle)?;
    let loc = location(state, &request.context.session_id)?;
    let environment =
        crate::writing_api::style_environment(state, &request.context.session_id).await?;
    if environment.key != bundle.style_generation {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "stale_environment",
            "资料库样式已变化，请重新准备模板；参数保留",
        ));
    }
    let parent = text(&request.intent, "parent")?.to_string();
    let name = text(&request.intent, "name")?.to_string();
    let intent = if let Some(original) = original {
        let source_path = file_path(&original.path)?;
        let base = bundle
            .files
            .iter()
            .find(|file| file.path == source_path)
            .ok_or_else(|| ApiError::bad("模板源缺少准确 Core 基础"))?
            .base_id
            .clone();
        OperationIntent::TemplateNode {
            source: source_path,
            parent: parent.clone(),
            name: name.clone(),
            instantiated_source: source,
            source_base_id: base,
        }
    } else {
        OperationIntent::NewNode {
            parent: parent.clone(),
            name: name.clone(),
            source: Some(source),
        }
    };
    let op = OperationRequest {
        operation_id: request.operation_id,
        root: loc.root.to_string_lossy().into_owned(),
        intent,
    };
    let dependencies: Vec<_> = bundle
        .files
        .iter()
        .map(|file| file.base_id.clone())
        .collect();
    let plan = core_call(state, move |core| {
        core.prepare_dependent_feature_operation(op, &canonical, &dependencies)
    })
    .await?;
    let operation_id = plan.operation_id.clone();
    let files = core_call(state, move |core| {
        core.prepared_operation_files(&operation_id)
    })
    .await?;
    let target = Path::new(&parent).join(&name).join(format!("{name}.typ"));
    let actual = files
        .iter()
        .find(|file| Path::new(&file.path) == target)
        .ok_or_else(|| ApiError::internal("模板计划缺少新节点准确原文"))?;
    let actual_source = String::from_utf8(actual.after.clone())
        .map_err(|_| ApiError::internal("模板计划原文不是 UTF-8"))?;
    let mut inputs = frozen_map(&loc.root, &bundle.files, None)?;
    for file in &files {
        let relative = Path::new(&file.path)
            .strip_prefix(&loc.root)
            .map_err(|_| ApiError::bad("模板创建计划不在资料库内"))?
            .to_string_lossy()
            .replace('\\', "/");
        inputs.insert(relative, file.after.clone());
    }
    let rendered = super::render_frozen_worker(
        state,
        weftext_engine::FrozenRenderRequest {
            source: actual_source.clone(),
            path: target,
            root: loc.root.clone(),
            files: inputs,
            style: environment.style.clone(),
        },
    )
    .await?;
    crate::writing_api::check_style_environment(state, &request.context.session_id, &environment)
        .await?;
    let mut preview = frozen_feature_preview(state, plan, &request.context).await?;
    preview["preview"]["template"] = json!({"source":actual_source,"tree":rendered["tree"],"diagnostics":rendered["diagnostics"],"ok":rendered["ok"],"parameters":request.intent["parameters"]});
    exact(state, &request.context)?;
    preview["context"] = json!(request.context);
    Ok(Json(preview))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplatePreviewRequest {
    context: Context,
    template_id: String,
    parameters: Value,
}
async fn template_preview(
    State(state): State<AppState>,
    Json(request): Json<TemplatePreviewRequest>,
) -> ApiResult {
    observe(&state, &request.context)?;
    let bundle = bundle(&state, &request.context).await?;
    let (source, original) = configured_template_source(
        &json!({"templateId":request.template_id,"parameters":request.parameters}),
        &bundle,
    )?;
    let loc = location(&state, &request.context.session_id)?;
    let environment =
        crate::writing_api::style_environment(&state, &request.context.session_id).await?;
    let path = original
        .as_ref()
        .map(|node| file_path(&node.path).map(PathBuf::from))
        .transpose()?
        .unwrap_or(loc.path);
    let output = super::render_frozen_worker(
        &state,
        weftext_engine::FrozenRenderRequest {
            source: source.clone(),
            path: path.clone(),
            root: loc.root.clone(),
            style: environment.style.clone(),
            files: frozen_map(&loc.root, &bundle.files, Some(&path))?,
        },
    )
    .await?;
    crate::writing_api::check_style_environment(&state, &request.context.session_id, &environment)
        .await?;
    exact(&state, &request.context)?;
    Ok(Json(
        json!({"source":source,"tree":output["tree"],"diagnostics":output["diagnostics"],"ok":output["ok"],"previewBasis":"registeredSource","sourceNode":original.as_ref().map(|node|&node.owner)}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn draft_compile_view_keeps_the_original_core_cut_and_rejects_aliases() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = weftext_core::Core::new(temp.path().join("state")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let captured = core.admitted_compile_snapshot(&root, "live").unwrap();
        let admitted = captured
            .snapshot
            .files
            .iter()
            .find(|file| file.path == note.path)
            .unwrap();
        let before = serde_json::to_value(&captured).unwrap();
        let draft = format!(
            "{}\n#set heading(numbering:\"1\")\n= Draft 😀 <draft-only>\n",
            note.source
        );
        let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
        for native_keys in [false, true] {
            let original = captured
                .files
                .iter()
                .map(|(path, bytes)| {
                    (
                        if native_keys {
                            path.replace('/', "\\")
                        } else {
                            path.clone()
                        },
                        bytes.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let compiled =
                draft_compile_files(&root, Path::new(&note.path), admitted, &original, &draft)
                    .unwrap();
            let main_key = if native_keys {
                "Note\\Note.typ"
            } else {
                "Note/Note.typ"
            };
            assert_eq!(compiled[main_key], draft.as_bytes());
            for (path, bytes) in &original {
                if path != main_key {
                    assert_eq!(compiled[path], *bytes);
                }
            }
            let output = engine
                .render_frozen(&weftext_engine::FrozenRenderRequest {
                    source: draft.clone(),
                    path: note.path.clone().into(),
                    root: root.clone(),
                    files: compiled,
                    style: captured.style.clone(),
                })
                .unwrap();
            assert!(output.ok, "{:?}", output.diagnostics);
            assert!(
                output
                    .labels
                    .iter()
                    .any(|label| label.label == "draft-only" && label.count == 1)
            );
        }
        assert_eq!(serde_json::to_value(&captured).unwrap(), before);
        core.validate_snapshot(&root, &admitted.path, &admitted.base_id)
            .unwrap();
        assert_eq!(std::fs::read(&note.path).unwrap(), note.source.as_bytes());
        for alias in ["Note\\Note.typ", "note/Note.typ"] {
            let mut files = captured.files.clone();
            files.insert(alias.into(), note.source.as_bytes().to_vec());
            assert!(
                draft_compile_files(&root, Path::new(&note.path), admitted, &files, &draft)
                    .is_err(),
                "{alias}"
            );
        }
        for invalid in [
            "../outside.typ",
            "/outside.typ",
            "Note/./alias.typ",
            "Note//alias.typ",
            "Note/file:ads",
        ] {
            let mut files = captured.files.clone();
            files.insert(invalid.into(), b"invalid".to_vec());
            assert!(
                draft_compile_files(&root, Path::new(&note.path), admitted, &files, &draft)
                    .is_err(),
                "{invalid}"
            );
        }
        let mut missing = captured.files.clone();
        missing.remove("Note/Note.typ");
        assert!(
            draft_compile_files(&root, Path::new(&note.path), admitted, &missing, &draft).is_err()
        );
        let mut different = captured.files.clone();
        different.insert("Note/Note.typ".into(), b"different original".to_vec());
        assert!(
            draft_compile_files(&root, Path::new(&note.path), admitted, &different, &draft)
                .is_err()
        );
        let mut unqualified = admitted.clone();
        unqualified.node_id = None;
        assert!(
            draft_compile_files(
                &root,
                Path::new(&note.path),
                &unqualified,
                &captured.files,
                &draft
            )
            .is_err()
        );
        assert!(
            draft_compile_files(
                &root,
                &temp.path().join("outside.typ"),
                admitted,
                &captured.files,
                &draft
            )
            .is_err()
        );
    }

    #[test]
    fn utf16_offsets_require_exact_code_points() {
        assert_eq!(utf16_byte("A🙂中", 3).ok().unwrap(), 5);
        assert!(utf16_byte("A🙂中", 2).is_err());
        assert!(utf16_byte("A🙂中", 5).is_err());
    }
    #[test]
    fn template_parameters_are_native_literals() {
        let source=template_source(&json!({"templateId":"native-note","parameters":{"title":"#panic(\"injection\")","body":"你好🙂"}})).ok().unwrap();
        assert!(source.contains("\\#panic"));
        assert!(source.contains("#set document(title: ["));
        assert!(source.contains("#title()"));
        assert!(!source.contains("\n= "));
        let plain=template_source(&json!({"templateId":"native-note","parameters":{"title":"真实文章标题","body":"正文🙂"}})).unwrap();
        let editor = weftext_engine::inspect_editor(&plain);
        assert_eq!(editor.title.status, "writable");
        assert_eq!(editor.title.text.as_deref(), Some("真实文章标题"));
        assert!(
            editor
                .structures
                .iter()
                .all(|structure| structure.kind != "heading")
        );
        assert_eq!(
            weftext_engine::inspect_metadata(&plain).id.as_deref(),
            Some("00000000-0000-4000-8000-000000000000")
        );
        assert!(
            template_source(&json!({"templateId":"native-note","parameters":{"title":null}}))
                .is_err()
        );
        assert!(
            template_source(
                &json!({"templateId":"native-note","parameters":{"title":"A","unknown":"B"}})
            )
            .is_err()
        );
    }
    #[test]
    fn enriched_disclosure_is_not_node_identity_input() {
        let target=node_ref(&json!({"workspaceId":"w","nodeId":"n","title":"disclosed title","path":"C:/notes/n.typ"})).ok().unwrap();
        assert_eq!(
            target,
            knowledge::NodeRef {
                workspace_id: "w".into(),
                node_id: "n".into()
            }
        );
    }
    #[test]
    fn settings_and_integer_parameters_keep_unambiguous_author_facts() {
        assert!(
            serde_json::from_slice::<UniqueJson>(br#"{"templates":{"entries":[],"entries":[]}}"#)
                .is_err()
        );
        let value = parse_template_value(
            &json!("9223372036854775807"),
            knowledge::TemplateSlotKind::Int,
        )
        .unwrap();
        assert_eq!(template_value(&value), json!("9223372036854775807"));
        assert!(
            parse_template_value(
                &json!(9223372036854775807u64),
                knowledge::TemplateSlotKind::Int
            )
            .is_err()
        );
        assert!(
            parse_template_value(
                &json!("9223372036854775808"),
                knowledge::TemplateSlotKind::Int
            )
            .is_err()
        );
    }
    #[cfg(windows)]
    fn local_state() -> (AppState, Context) {
        use std::sync::{Arc, Mutex};
        let root =
            std::env::temp_dir().join(format!("weftext-knowledge-api-{}", uuid::Uuid::new_v4()));
        let mut core = weftext_core::Core::new(root.join("private")).unwrap();
        let library = root.join("library");
        core.create_library(&library).unwrap();
        // Exercise original v0.1 admissions; no migration replaces these files.
        let opened = core
            .create_note_fixture(&library, &library, "原始笔记")
            .unwrap();
        let source = format!("{}原文\n", opened.source);
        let receipt = core
            .save(weftext_core::SaveRequest {
                session_id: opened.session_id.clone(),
                base_id: opened.base_id,
                source: source.clone(),
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
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
        let document = json!({"sessionId":opened.session_id,"path":opened.path,"root":opened.root});
        remember(&state, &document).ok().unwrap();
        let context = Context {
            session_id: document["sessionId"].as_str().unwrap().into(),
            source,
            serial: 0,
            base_id: receipt.receipt.base_id,
            dirty: false,
        };
        (state, context)
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn adopted_shared_processing_origin_revalidates_exact_version_without_overwriting_options()
     {
        let (state, context) = local_state();
        let root = location(&state, &context.session_id).unwrap().root;
        let installed = core_call(&state, {
            let root = root.clone();
            move |core| {
                let before =
                    core.read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)?;
                let processing = knowledge::ProcessingDocument {
                    schema_version: 1,
                    defaults: Default::default(),
                    rules: vec![],
                    scoped_allowlist: vec![],
                };
                let mut settings: Value = before
                    .bytes
                    .as_deref()
                    .map(serde_json::from_slice)
                    .transpose()?
                    .unwrap_or(json!({"version":1}));
                settings["processing"] = serde_json::to_value(processing)?;
                let op = uuid::Uuid::new_v4().to_string();
                let plan = core.prepare_config_save(
                    &root,
                    weftext_core::ConfigRole::WorkspaceSettings,
                    before.bytes,
                    serde_json::to_vec(&settings)?,
                    &op,
                )?;
                core.execute_operation(&op, &plan.plan_id)?;
                core.read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)
            }
        })
        .await
        .unwrap();
        let mut options = knowledge::ProofreadOptions {
            processing_origin: Some(knowledge::SharedProcessingOrigin {
                source: "shared-library".into(),
                base_id: installed.base_id.clone(),
                revision: installed.revision.clone(),
            }),
            ..Default::default()
        };
        options.allowlist.push("本次个人词条".into());
        assert!(
            validate_processing_origin(&state, &context, &options)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(options.allowlist, ["本次个人词条"]);
        core_call(&state, move |core| {
            let before = core.read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)?;
            let mut settings: Value = serde_json::from_slice(before.bytes.as_deref().unwrap())?;
            settings["unrelatedExplicitUpdate"] = json!(true);
            let op = uuid::Uuid::new_v4().to_string();
            let plan = core.prepare_config_save(
                &root,
                weftext_core::ConfigRole::WorkspaceSettings,
                before.bytes,
                serde_json::to_vec(&settings)?,
                &op,
            )?;
            core.execute_operation(&op, &plan.plan_id).map(|_| ())
        })
        .await
        .unwrap();
        assert!(
            validate_processing_origin(&state, &context, &options)
                .await
                .is_err()
        );
        assert_eq!(options.allowlist, ["本次个人词条"]);
        options.processing_origin.as_mut().unwrap().source = "invented-source".into();
        assert!(
            validate_processing_origin(&state, &context, &options)
                .await
                .is_err()
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn schema_aliases_and_native_titles_produce_all_overlapping_adapter_targets() {
        let (state, context) = local_state();
        let root = location(&state, &context.session_id).unwrap().root;
        let identities=core_call(&state,move|core| {
            let mut ids=vec![];
            for (name,title,alias) in [("甲","白云山",None),("乙","白云山2026",Some("白云山")),("丙","山",None)] {
                let opened=core.create_note_fixture(&root,&root,name)?;
                let id=weftext_engine::inspect_metadata(&opened.source).id.unwrap();
                let alias=alias.map(|alias|format!(", aliases: (\"{alias}\",)")).unwrap_or_default();
                let source=format!("#metadata((id: \"{id}\", types: (\"person\",){alias})) <weftext>\n#title[{title}]\n");
                core.save(weftext_core::SaveRequest {session_id:opened.session_id,base_id:opened.base_id,source,operation_id:uuid::Uuid::new_v4().to_string()})?;
                ids.push(id);
            }Ok(ids)
        }).await.unwrap();
        observe(&state, &context).unwrap();
        let facts = bundle(&state, &context).await.unwrap();
        let named = facts
            .inventory
            .nodes
            .iter()
            .find(|node| node.owner.node_id == identities[1])
            .unwrap();
        assert_eq!(named.title.as_deref(), Some("白云山2026"));
        assert_eq!(named.aliases, ["白云山"]);
        let scan = knowledge::scan_mentions(
            &facts.inventory,
            &facts.selected.owner,
            &[knowledge::LogicalFragment {
                id: "visible".into(),
                node_key: "visible".into(),
                text: "白云山2026".into(),
                source_range: None,
            }],
            100,
        )
        .unwrap();
        let hits = &scan.fragments[0].hits;
        let same = hits
            .iter()
            .find(|hit| hit.start_scalar == 0 && hit.end_scalar == 3)
            .unwrap();
        assert_eq!(same.targets.len(), 2);
        assert!(
            same.targets
                .iter()
                .any(|target| target.node_id == identities[0])
        );
        assert!(
            same.targets
                .iter()
                .any(|target| target.node_id == identities[1])
        );
        assert!(hits.iter().any(|hit| hit.start_scalar == 0
            && hit.end_scalar == 7
            && hit.targets[0].node_id == identities[1]));
        assert!(hits.iter().any(|hit| hit.start_scalar == 2
            && hit.end_scalar == 3
            && hit.targets[0].node_id == identities[2]));
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn custom_template_registration_persists_exact_source_version_and_stales_after_source_edit()
     {
        let (state, mut context) = local_state();
        let loc = location(&state, &context.session_id).unwrap();
        let byte = context.source.find("原文").unwrap();
        let from = context.source[..byte].encode_utf16().count();
        let id = uuid::Uuid::new_v4().to_string();
        let intent = json!({"kind":"templateRegister","title":"复用当前笔记","description":"原生源","parameters":[{"name":"body","kind":"markup","from":from,"to":from+2,"default":"默认"}]});
        let plan = prepare(
            State(state.clone()),
            Json(PrepareRequest {
                operation_id: id.clone(),
                context: context.clone(),
                intent: intent.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(
            plan["preview"]["templateAfter"]["parameters"][0]["default"],
            "默认"
        );
        let repeat = prepare(
            State(state.clone()),
            Json(PrepareRequest {
                operation_id: id.clone(),
                context: context.clone(),
                intent,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(plan["planId"], repeat["planId"]);
        assert_eq!(plan["preview"]["files"], repeat["preview"]["files"]);
        let receipt = commit(
            State(state.clone()),
            Json(CommitRequest {
                operation_id: id,
                plan_id: plan["planId"].as_str().unwrap().into(),
                context: Some(context.clone()),
            }),
        )
        .await
        .unwrap()
        .0;
        context.session_id = receipt["document"]["sessionId"].as_str().unwrap().into();
        context.base_id = receipt["document"]["baseId"].as_str().unwrap().into();
        observe(&state, &context).unwrap();
        let facts = bundle(&state, &context).await.unwrap();
        let template_id = plan["preview"]["templateAfter"]["id"].as_str().unwrap();
        let (source, original) = configured_template_source(
            &json!({"templateId":template_id,"parameters":{"body":"#panic(\"user\")"}}),
            &facts,
        )
        .unwrap();
        assert!(source.contains("\\#panic"));
        assert!(original.is_some());
        assert!(
            configured_template_source(
                &json!({"templateId":template_id,"parameters":{"body":null}}),
                &facts
            )
            .is_err()
        );
        let settings = template_registry(&facts.settings).unwrap();
        assert_eq!(settings.1.len(), 1);
        assert!(!settings.1[0].source_revision.is_empty());
        let input = OperationRequest {
            operation_id: uuid::Uuid::new_v4().to_string(),
            root: loc.root.to_string_lossy().into_owned(),
            intent: OperationIntent::TemplateNode {
                source: facts.source_file.path.clone(),
                parent: loc.root.to_string_lossy().into_owned(),
                name: "模板产物".into(),
                instantiated_source: source,
                source_base_id: facts.source_file.base_id.clone(),
            },
        };
        let created = core_call(&state, move |core| {
            let preview = core.prepare_operation(input)?;
            let files = core.prepared_operation_files(&preview.operation_id)?;
            let receipt = core.execute_operation(&preview.operation_id, &preview.plan_id)?;
            Ok((preview, files, receipt))
        })
        .await
        .unwrap();
        assert_ne!(
            created.0.new_node_id.as_deref(),
            Some(facts.selected.owner.node_id.as_str())
        );
        assert!(
            created
                .1
                .iter()
                .any(|file| file.path.ends_with("模板产物.typ")
                    && String::from_utf8_lossy(&file.after).contains("\\#panic"))
        );
        let root = loc.root;
        let file = facts.source_file;
        core_call(&state, move |core| {
            let op = uuid::Uuid::new_v4().to_string();
            let after = String::from_utf8(file.bytes.clone().unwrap())
                .unwrap()
                .replace("原文", "新版原文")
                .into_bytes();
            let plan = core.prepare_operation(OperationRequest {
                operation_id: op.clone(),
                root: root.to_string_lossy().into_owned(),
                intent: OperationIntent::UpdateFiles {
                    changes: vec![FileChange {
                        path: file.path,
                        base_id: Some(file.base_id),
                        before: file.bytes,
                        after,
                    }],
                },
            })?;
            core.execute_operation(&op, &plan.plan_id).map(|_| ())
        })
        .await
        .unwrap();
        let changed = bundle(&state, &context).await.unwrap();
        assert!(
            configured_template_source(
                &json!({"templateId":template_id,"parameters":{}}),
                &changed
            )
            .is_err()
        );
        let catalog = template_catalog(&changed).unwrap();
        assert_eq!(catalog[1]["available"], false);
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn original_note_suggestion_and_state_share_one_decision() {
        let (state, mut context) = local_state();
        let byte = context.source.find("原文").unwrap();
        let from = context.source[..byte].encode_utf16().count();
        let op = uuid::Uuid::new_v4().to_string();
        let plan=prepare(State(state.clone()),Json(PrepareRequest {operation_id:op.clone(),context:context.clone(),intent:json!({"kind":"annotationCreate","target":{"kind":"source_range","from":from,"to":from+2,"visibleExcerpt":"原文"},"body":"修订意见","purpose":"suggestion","replacement":"修订"})})).await.map_err(|error|error.message).unwrap().0;
        let loc = location(&state, &context.session_id).ok().unwrap();
        let sidecar = annotations_path(&loc.path).ok().unwrap();
        let before = core_call(&state, {
            let root = loc.root.clone();
            let sidecar = sidecar.clone();
            move |core| core.read_snapshot(root, &[sidecar])
        })
        .await
        .map_err(|error| error.message)
        .unwrap();
        assert!(
            before.files[0].bytes.is_none(),
            "preparing annotations must not persist them"
        );
        let receipt = commit(
            State(state.clone()),
            Json(CommitRequest {
                operation_id: op.clone(),
                plan_id: plan["planId"].as_str().unwrap().into(),
                context: Some(context.clone()),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        let document = &receipt["document"];
        context.session_id = document["sessionId"].as_str().unwrap().into();
        context.base_id = document["baseId"].as_str().unwrap().into();
        let snapshot = read(
            State(state.clone()),
            Json(ReadRequest {
                context: context.clone(),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        let annotation = &snapshot["annotations"][0];
        assert_eq!(annotation["targetResolution"], "exact");
        assert_eq!(annotation["suggestionState"], "pending");
        let id = annotation["annotationId"].as_str().unwrap();
        let accept_op = uuid::Uuid::new_v4().to_string();
        let mut dirty = context.clone();
        dirty.serial = 1;
        dirty.dirty = true;
        dirty.source.push_str("未保存");
        let error = prepare(
            State(state.clone()),
            Json(PrepareRequest {
                operation_id: uuid::Uuid::new_v4().to_string(),
                context: dirty,
                intent: json!({"kind":"annotationAccept","annotationId":id}),
            }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.code, "save_required");
        context.serial = 2;
        let accept = prepare(
            State(state.clone()),
            Json(PrepareRequest {
                operation_id: accept_op.clone(),
                context: context.clone(),
                intent: json!({"kind":"annotationAccept","annotationId":id}),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert!(accept["source"].as_str().unwrap().contains("修订"));
        assert_eq!(accept["changes"].as_array().unwrap().len(), 2);
        let _ = commit(
            State(state.clone()),
            Json(CommitRequest {
                operation_id: accept_op.clone(),
                plan_id: accept["planId"].as_str().unwrap().into(),
                context: Some(context.clone()),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap();
        let pair = core_call(&state, move |core| {
            core.read_snapshot(
                loc.root,
                &[loc.path.to_string_lossy().into_owned(), sidecar],
            )
        })
        .await
        .map_err(|error| error.message)
        .unwrap();
        let source = String::from_utf8(pair.files[0].bytes.clone().unwrap()).unwrap();
        assert!(source.contains("修订"));
        assert!(!source.contains("原文"));
        let annotations =
            knowledge::decode_annotations(pair.files[1].bytes.as_ref().unwrap()).unwrap();
        assert_eq!(
            annotations.records[0].suggestion.as_ref().unwrap().state,
            "accepted"
        );
        let status = status(
            State(state),
            Json(StatusRequest {
                operation_id: accept_op,
                context: Some(context.clone()),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert_eq!(status["status"], "committed");
        assert!(
            status["document"]["source"]
                .as_str()
                .unwrap()
                .contains("修订")
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn rename_and_unknown_result_status_rebind_the_actual_same_note() {
        let (state, context) = local_state();
        let loc = location(&state, &context.session_id).ok().unwrap();
        let plan=prepare(State(state.clone()),Json(PrepareRequest {operation_id:uuid::Uuid::new_v4().to_string(),context:context.clone(),intent:json!({"kind":"renameNode","source":loc.path.parent().unwrap().to_string_lossy(),"name":"重命名结果"})})).await.map_err(|error|error.message).unwrap().0;
        let receipt = commit(
            State(state.clone()),
            Json(CommitRequest {
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
                context: Some(context.clone()),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert_eq!(receipt["documentDisposition"], "rebound");
        assert!(
            receipt["document"]["path"]
                .as_str()
                .unwrap()
                .ends_with("重命名结果\\重命名结果.typ")
        );
        assert_eq!(receipt["document"]["source"], context.source);
        let outcome = status(
            State(state.clone()),
            Json(StatusRequest {
                operation_id: plan["operationId"].as_str().unwrap().into(),
                context: Some(context.clone()),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert_eq!(outcome["status"], "committed");
        assert_eq!(outcome["document"]["path"], receipt["document"]["path"]);
        assert_eq!(outcome["document"]["nodeId"], receipt["document"]["nodeId"]);
        let unknown = status(
            State(state),
            Json(StatusRequest {
                operation_id: uuid::Uuid::new_v4().to_string(),
                context: Some(context),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert_eq!(unknown["status"], "unknown");
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn preparing_same_annotation_intent_reuses_original_frozen_record() {
        let (state, context) = local_state();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let intent = json!({"kind":"annotationCreate","target":{"kind":"whole_document"},"purpose":"comment","body":"同一意见"});
        let first = prepare(
            State(state.clone()),
            Json(PrepareRequest {
                operation_id: operation_id.clone(),
                context: context.clone(),
                intent: intent.clone(),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        let again = prepare(
            State(state.clone()),
            Json(PrepareRequest {
                operation_id: operation_id.clone(),
                context: context.clone(),
                intent,
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert_eq!(first["planId"], again["planId"]);
        assert_eq!(first["preview"]["files"], again["preview"]["files"]);
        let changed=prepare(State(state),Json(PrepareRequest {operation_id,context,intent:json!({"kind":"annotationCreate","target":{"kind":"whole_document"},"purpose":"comment","body":"不同意见"})})).await.err().unwrap();
        assert_eq!(changed.code, "operationConflict");
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn moving_parent_returns_child_session_and_recovers_private_dirty_draft() {
        let (state, context) = local_state();
        let loc = location(&state, &context.session_id).ok().unwrap();
        let child = core_call(&state, {
            let root = loc.root.clone();
            let parent = loc.path.parent().unwrap().to_path_buf();
            move |core| core.create_note_fixture(root, parent, "子笔记")
        })
        .await
        .map_err(|error| error.message)
        .unwrap();
        let child_value = value(&child).ok().unwrap();
        remember(&state, &child_value).ok().unwrap();
        let dirty_source = format!("{}未保存的子草稿", child.source);
        let child_id = child.session_id.clone();
        let draft = dirty_source.clone();
        core_call(&state, move |core| core.persist_draft(&child_id, &draft))
            .await
            .map_err(|error| error.message)
            .unwrap();
        let plan=prepare(State(state.clone()),Json(PrepareRequest {operation_id:uuid::Uuid::new_v4().to_string(),context:context.clone(),intent:json!({"kind":"renameNode","source":loc.path.parent().unwrap().to_string_lossy(),"name":"新父目录"})})).await.map_err(|error|error.message).unwrap().0;
        let result = commit(
            State(state.clone()),
            Json(CommitRequest {
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
                context: Some(context),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        let affected = result["affectedDocuments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|document| document["priorSessionId"] == child.session_id)
            .unwrap();
        assert!(
            affected["document"]["path"]
                .as_str()
                .unwrap()
                .ends_with("新父目录\\子笔记\\子笔记.typ")
        );
        assert_eq!(affected["document"]["nodeId"], child_value["nodeId"]);
        assert_eq!(affected["document"]["recoveredDraft"], dirty_source);
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn a_later_source_decision_is_not_the_reviewed_committed_patch() {
        let (state, context) = local_state();
        let loc = location(&state, &context.session_id).ok().unwrap();
        let expected = context.source.replace("原文", "已提交");
        let operation_id = uuid::Uuid::new_v4().to_string();
        let (preview, receipt) = core_call(&state, {
            let root = loc.root.clone();
            let path = loc.path.to_string_lossy().into_owned();
            let expected = expected.clone();
            move |core| {
                let file = core
                    .read_snapshot(&root, std::slice::from_ref(&path))?
                    .files
                    .remove(0);
                let preview = core.prepare_operation(OperationRequest {
                    operation_id: operation_id.clone(),
                    root: root.to_string_lossy().into_owned(),
                    intent: OperationIntent::UpdateFiles {
                        changes: vec![FileChange {
                            path: path.clone(),
                            base_id: Some(file.base_id),
                            before: file.bytes,
                            after: expected.as_bytes().to_vec(),
                        }],
                    },
                })?;
                let receipt = core.execute_operation(&operation_id, &preview.plan_id)?;
                let file = core
                    .read_snapshot(&root, std::slice::from_ref(&path))?
                    .files
                    .remove(0);
                let later = uuid::Uuid::new_v4().to_string();
                let after = String::from_utf8(file.bytes.clone().unwrap())
                    .unwrap()
                    .replace("已提交", "后来修改")
                    .into_bytes();
                let changed = core.prepare_operation(OperationRequest {
                    operation_id: later.clone(),
                    root: root.to_string_lossy().into_owned(),
                    intent: OperationIntent::UpdateFiles {
                        changes: vec![FileChange {
                            path,
                            base_id: Some(file.base_id),
                            before: file.bytes,
                            after,
                        }],
                    },
                })?;
                core.execute_operation(&later, &changed.plan_id)?;
                Ok((preview, receipt))
            }
        })
        .await
        .map_err(|error| error.message)
        .unwrap();
        let result = operation_output(&state, preview, receipt, Some(&context))
            .await
            .map_err(|error| error.message)
            .unwrap();
        assert_eq!(result["status"], "committed");
        assert_eq!(result["expectedSource"], expected);
        assert_eq!(result["documentSourceMatchesCommit"], false);
        assert_eq!(result["documentError"]["code"], "externalChanged");
        assert!(
            result["document"]["source"]
                .as_str()
                .unwrap()
                .contains("后来修改")
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn resource_region_uses_actual_bytes_and_one_guarded_sidecar_plan() {
        let (state, context) = local_state();
        let loc = location(&state, &context.session_id).ok().unwrap();
        let mut image_bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(3, 2)
            .write_to(&mut image_bytes, image::ImageFormat::Png)
            .unwrap();
        let op = uuid::Uuid::new_v4().to_string();
        let core_plan = core_call(&state, {
            let root = loc.root.clone();
            let path = loc.path.clone();
            let op = op.clone();
            move |core| {
                core.prepare_import_resource(
                    root,
                    path,
                    "实际图片.png",
                    image_bytes.into_inner(),
                    &op,
                )
            }
        })
        .await
        .map_err(|error| error.message)
        .unwrap();
        let _ = core_call(&state, move |core| {
            core.execute_operation(&op, &core_plan.plan_id)
        })
        .await
        .map_err(|error| error.message)
        .unwrap();
        let set = crate::media_api::resource_set(&state, &context.session_id)
            .await
            .map_err(|error| error.message)
            .unwrap();
        let resource = &set.resources[0];
        let revision = resource.file.revision.as_ref().unwrap();
        let target = json!({"kind":"image_region","resource":resource.resource_ref,"revision":revision,"width":3,"height":2,"x":0,"y":0,"rectWidth":2,"rectHeight":1});
        let mut forged = target.clone();
        forged["width"] = json!(300);
        let error=prepare(State(state.clone()),Json(PrepareRequest {operation_id:uuid::Uuid::new_v4().to_string(),context:context.clone(),intent:json!({"kind":"annotationCreate","target":forged,"body":"区域批注","purpose":"comment"})})).await.err().unwrap();
        assert_eq!(error.code, "resource_geometry");
        let plan=prepare(State(state.clone()),Json(PrepareRequest {operation_id:uuid::Uuid::new_v4().to_string(),context:context.clone(),intent:json!({"kind":"annotationCreate","target":target,"body":"区域批注","purpose":"comment"})})).await.map_err(|error|error.message).unwrap().0;
        assert_eq!(plan["changes"].as_array().unwrap().len(), 3);
        let _ = commit(
            State(state.clone()),
            Json(CommitRequest {
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
                context: Some(context.clone()),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap();
        let result = read(
            State(state.clone()),
            Json(ReadRequest {
                context: context.clone(),
            }),
        )
        .await
        .map_err(|error| error.message)
        .unwrap()
        .0;
        assert_eq!(result["annotations"][0]["targetResolution"], "exact");
        assert_eq!(
            result["annotations"][0]["resourceLocation"]["target"]["pixelWidth"],
            3
        );
        let mut draft = context.clone();
        draft.serial += 1;
        draft.dirty = true;
        draft
            .source
            .push_str(&format!("#link(\"{}\")[准确资源]\n", resource.resource_ref));
        let occurrence = knowledge::native_links(&draft.source)
            .last()
            .unwrap()
            .clone();
        for form in [
            knowledge::LinkForm::Id,
            knowledge::LinkForm::Relative,
            knowledge::LinkForm::Workspace,
            knowledge::LinkForm::File,
        ] {
            let copied = link_copy(
                State(state.clone()),
                Json(LinkCopyRequest {
                    context: draft.clone(),
                    occurrence: json!(occurrence),
                    form,
                }),
            )
            .await
            .unwrap()
            .0;
            assert_eq!(copied["status"], "resolved");
            assert!(copied["fullLink"].as_str().unwrap().starts_with("#link("));
            let resolved = resolve_address(
                &state,
                &draft.session_id,
                copied["address"].as_str().unwrap(),
            )
            .await
            .unwrap()
            .0;
            assert_eq!(resolved["resourceRef"], resource.resource_ref);
            assert_eq!(resolved["revision"], *revision);
            assert!(
                resolved["ownerEntityKey"]
                    .as_str()
                    .unwrap()
                    .ends_with(&resource.node_id)
            );
            if form == knowledge::LinkForm::Id {
                let uppercase = copied["address"].as_str().unwrap().replacen(
                    "weftext-resource:",
                    "WEFTEXT-RESOURCE:",
                    1,
                );
                let resolved = resolve_address(&state, &draft.session_id, &uppercase)
                    .await
                    .unwrap()
                    .0;
                assert_eq!(resolved["resourceRef"], resource.resource_ref);
            }
        }
    }
}
