//! Publication-only citation capabilities. Work source evidence remains private;
//! only the explicitly granted publication slice enters the generated native Bib.
use super::*;
use std::path::Path;
use weftext_core::{ConfigRole, QualifiedNode, SnapshotWitness};
use weftext_export::bibliography::{
    BibliographyFormat, BibliographyProjection, CitationWork, FrozenWork, project_bibliography,
};

const PUBLICATION: &str = "library/publication";
const MAX_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) r#use: bool,
    #[serde(default)]
    pub(super) export: bool,
    #[serde(default)]
    pub(super) download: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Authority {
    pub(super) works: BTreeSet<String>,
    #[serde(default)]
    binding: String,
}
struct WorkEvidence {
    resource: String,
    node: QualifiedNode,
    path: PathBuf,
    source: SnapshotWitness,
}
struct Evidence {
    schema_path: String,
    schema: SnapshotWitness,
    rules_path: String,
    rules: SnapshotWitness,
    works: Vec<WorkEvidence>,
}
#[derive(Clone)]
pub(super) struct Capture {
    pub(super) authority: Authority,
    pub(super) works: Vec<CitationWork>,
    pub(super) virtual_path: Option<String>,
    virtual_witness: Option<(String, SnapshotWitness)>,
    pub(super) bibliography: BibliographyProjection,
    evidence: Arc<Evidence>,
    bytes: usize,
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "citationBasisChanged",
        UiMessage::error_api_hosted_citations_citation_basis_changed(),
    )
}
fn changed_after_worker(original: ApiError) -> ApiError {
    changed().inherit_worker_failure_from(&original)
}
fn reference_number_unavailable(original: Option<&ApiError>) -> ApiError {
    let safe = ApiError::bad(UiMessage::error_api_hosted_citations_reference_number_unavailable());
    match original {
        Some(original) => safe.inherit_worker_failure_from(original),
        None => safe,
    }
}
fn access(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
    export: bool,
    download: bool,
) -> Result<(), ApiError> {
    let r = inner.grant(actor, resource)?;
    if !r.citations.r#use
        || !r.fields.get(PUBLICATION).is_some_and(|f| f.read)
        || (export && !r.citations.export)
        || (download && !r.citations.download)
        || !inner
            .file(resource)
            .ok()
            .is_some_and(|f| f.lifecycle == "live")
    {
        return Err(unavailable());
    }
    Ok(())
}
fn binding(inner: &HostedInner, works: &BTreeSet<String>) -> Result<String, ApiError> {
    inputs::binding_hash(inner, works)
}
pub(super) fn authorize(
    inner: &HostedInner,
    actor: &str,
    authority: &Authority,
    export: bool,
    download: bool,
) -> Result<(), ApiError> {
    for id in &authority.works {
        access(inner, actor, id, export, download)?;
    }
    if !authority.binding.is_empty() && binding(inner, &authority.works)? != authority.binding {
        return Err(changed());
    }
    Ok(())
}
pub(super) fn merge_authority(
    inner: &HostedInner,
    to: &mut Authority,
    from: &Authority,
) -> Result<(), ApiError> {
    if !from.binding.is_empty() && binding(inner, &from.works)? != from.binding {
        return Err(changed());
    }
    to.works.extend(from.works.iter().cloned());
    to.binding = binding(inner, &to.works)?;
    Ok(())
}
impl Capture {
    pub(super) fn retained_bytes(&self) -> usize {
        self.bytes
    }
    pub(super) fn validate(&self, inner: &HostedInner, actor: &str) -> Result<(), ApiError> {
        authorize(inner, actor, &self.authority, false, false)?;
        if let Some((path, w)) = &self.virtual_witness {
            inner
                .core
                .validate_snapshot_witness(&inner.config.root, path, w)
                .map_err(|_| changed())?;
        }
        inner
            .core
            .validate_snapshot_witness(
                &inner.config.root,
                &self.evidence.schema_path,
                &self.evidence.schema,
            )
            .map_err(|_| changed())?;
        inner
            .core
            .validate_snapshot_witness(
                &inner.config.root,
                &self.evidence.rules_path,
                &self.evidence.rules,
            )
            .map_err(|_| changed())?;
        for w in &self.evidence.works {
            let current = inner
                .core
                .qualified_node_at(&inner.config.root, &w.path)
                .map_err(|_| changed())?;
            if current.owner != w.node.owner || current.revision != w.node.revision {
                return Err(changed());
            }
            inner
                .core
                .validate_snapshot_witness(&inner.config.root, &w.path, &w.node.witness)
                .map_err(|_| changed())?;
            inner
                .core
                .validate_snapshot_witness(&inner.config.root, &w.path, &w.source)
                .map_err(|_| changed())?;
            author_file(inner, &w.resource)?;
        }
        Ok(())
    }
    pub(super) fn plan_witnesses(&self) -> Result<Vec<SnapshotWitness>, ApiError> {
        let mut pool = vec![];
        Core::merge_feature_witnesses(
            &mut pool,
            vec![self.evidence.schema.clone(), self.evidence.rules.clone()],
        )
        .map_err(|_| changed())?;
        if let Some((_, w)) = &self.virtual_witness {
            Core::merge_feature_witnesses(&mut pool, vec![w.clone()]).map_err(|_| changed())?;
        }
        for w in &self.evidence.works {
            Core::merge_feature_witnesses(
                &mut pool,
                vec![w.node.witness.clone(), w.source.clone()],
            )
            .map_err(|_| changed())?;
        }
        Ok(pool)
    }
    pub(super) fn inject(&self, worker: &mut WorkerRequest) -> Result<(), ApiError> {
        if let Some(path) = &self.virtual_path {
            let files = &mut worker.frozen_files;
            if files
                .keys()
                .any(|p| p.replace('\\', "/").eq_ignore_ascii_case(path))
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_citations_generated_input_name_conflict(),
                ));
            }
            files.insert(
                path.clone(),
                base64::engine::general_purpose::STANDARD.encode(&self.bibliography.bytes),
            );
        }
        Ok(())
    }
}
fn uri(node: &weftext_knowledge::NodeRef) -> String {
    format!("weftext-node://{}/{}", node.workspace_id, node.node_id)
}
fn capture_works(
    inner: &mut HostedInner,
    actor: &str,
    selections: &[(String, String)],
    owner: Option<(&str, &str)>,
) -> Result<Capture, ApiError> {
    if selections.len() > 64 {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_citation_count_limit(),
        ));
    }
    // Authorize every explicit selector before reading Schema or source.
    for (_, id) in selections {
        access(inner, actor, id, false, false)?;
    }
    let root = inner.config.root.clone();
    let doc = inner
        .core
        .read_config(&root, ConfigRole::Schema)
        .map_err(|_| changed())?;
    let schema_witness = inner
        .core
        .take_snapshot_witness(&doc.base_id)
        .map_err(|_| changed())?;
    if doc.bytes.is_some() && doc.revision.is_none() {
        return Err(changed());
    }
    let schema = doc
        .bytes
        .as_deref()
        .map(|b| {
            std::str::from_utf8(b)
                .map_err(|_| changed())
                .and_then(|s| weftext_data::Schema::from_author_json(s).map_err(|_| changed()))
        })
        .transpose()?
        .unwrap_or(weftext_data::Schema::bundled().map_err(|_| changed())?);
    let rules = inner
        .core
        .read_config(&root, ConfigRole::Rules)
        .map_err(|_| changed())?;
    let rules_witness = inner
        .core
        .take_snapshot_witness(&rules.base_id)
        .map_err(|_| changed())?;
    let mut bytes = schema_witness
        .retained_bytes()
        .saturating_add(rules_witness.retained_bytes());
    let mut works = vec![];
    let mut evidence = vec![];
    for (key, id) in selections {
        let file = author_file(inner, id)?;
        let path = root.join(&file.path);
        let qualified = inner
            .core
            .qualified_node_at(&root, &path)
            .map_err(|_| unavailable())?;
        if file
            .expected_node
            .as_ref()
            .is_some_and(|expected| expected != &qualified.owner)
        {
            return Err(unavailable());
        }
        if qualified.revision.is_none() {
            return Err(changed());
        }
        let snapshot = inner
            .core
            .read_snapshot(&root, &[path.to_string_lossy().into_owned()])
            .map_err(|_| changed())?
            .files
            .into_iter()
            .next()
            .ok_or_else(changed)?;
        let source_witness = inner
            .core
            .take_snapshot_witness(&snapshot.base_id)
            .map_err(|_| changed())?;
        bytes = bytes
            .saturating_add(qualified.witness.retained_bytes())
            .saturating_add(source_witness.retained_bytes());
        if bytes > MAX_BYTES {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_citations_citation_input_budget(),
            ));
        }
        let saved = std::str::from_utf8(snapshot.bytes.as_deref().ok_or_else(changed)?)
            .map_err(|_| changed())?;
        let source = owner.filter(|(r, _)| *r == id).map_or(saved, |(_, s)| s);
        let (node_id, _, resolved) =
            weftext_data::narrow_query_metadata(source, &schema).map_err(|_| unavailable())?;
        if node_id != qualified.owner.node_id || !resolved.iter().any(|t| t == "library/work") {
            return Err(unavailable());
        }
        let profile = weftext_data::narrow_profile(
            source,
            &schema,
            &BTreeSet::from([PUBLICATION.to_owned()]),
            None,
        )
        .map_err(|_| unavailable())?;
        let field = profile
            .fields
            .iter()
            .find(|f| f.field_id == PUBLICATION)
            .ok_or_else(unavailable)?;
        let publication = field.cell.native().cloned().ok_or_else(unavailable)?;
        // This public basis describes only the disclosed publication and its
        // expanded shape. A Work body hash/revision never enters the artifact.
        let source_basis=format!("publication:{}",digest(&serde_json::to_vec(&json!({"publication":publication,"shape":field.shape,"type":"library/work","owner":qualified.owner})).map_err(|_|changed())?));
        works.push(CitationWork {
            key: key.clone(),
            work: FrozenWork {
                entity_ref: uri(&qualified.owner),
                source_basis,
                publication,
            },
        });
        evidence.push(WorkEvidence {
            resource: id.clone(),
            node: qualified,
            path,
            source: source_witness,
        });
    }
    let bibliography = project_bibliography(&works, BibliographyFormat::Bibtex).map_err(|_| {
        ApiError::bad(UiMessage::error_api_hosted_citations_publication_fields_unavailable())
    })?;
    bytes = bytes
        .saturating_add(bibliography.bytes.len())
        .saturating_add(serde_json::to_vec(&works).map_err(|_| changed())?.len());
    if bytes > MAX_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_citation_input_budget(),
        ));
    }
    let ids = selections.iter().map(|(_, r)| r.clone()).collect();
    let result = Capture {
        authority: Authority {
            binding: binding(inner, &ids)?,
            works: ids,
        },
        works,
        virtual_path: None,
        virtual_witness: None,
        bibliography,
        evidence: Arc::new(Evidence {
            schema_path: doc.path,
            schema: schema_witness,
            rules_path: rules.path,
            rules: rules_witness,
            works: evidence,
        }),
        bytes,
    };
    result.validate(inner, actor)?;
    Ok(result)
}
pub(super) fn selected(
    inner: &mut HostedInner,
    actor: &str,
    ids: &[String],
) -> Result<Capture, ApiError> {
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_duplicate_citation_selection(),
        ));
    }
    capture_works(
        inner,
        actor,
        &ids.iter()
            .enumerate()
            .map(|(n, id)| (format!("work{}", n + 1), id.clone()))
            .collect::<Vec<_>>(),
        None,
    )
}
/// Derive offline inputs exclusively from the already captured native package.
/// The whole-library cut does not grant citation execution/export for Works.
pub(super) fn native_inputs(
    inner: &HostedInner,
    actor: &str,
    package: &mut weftext_export::exchange::NativePackage,
) -> Result<Authority, ApiError> {
    use weftext_export::exchange::NativeFileRole;
    let schema = package
        .files
        .get(&package.manifest.schema_path)
        .map(|b| {
            std::str::from_utf8(b)
                .map_err(|_| changed())
                .and_then(|s| weftext_data::Schema::from_author_json(s).map_err(|_| changed()))
        })
        .transpose()?
        .unwrap_or(weftext_data::Schema::bundled().map_err(|_| changed())?);
    let source_entries = package
        .manifest
        .entries
        .iter()
        .filter(|(_, e)| e.role == NativeFileRole::Source)
        .map(|(p, e)| (p.clone(), e.node_id.clone().unwrap_or_default()))
        .collect::<Vec<_>>();
    let mut authority = Authority::default();
    for (path, owner_id) in &source_entries {
        let source = std::str::from_utf8(package.files.get(path).ok_or_else(changed)?)
            .map_err(|_| changed())?;
        let Some(declaration) =
            weftext_engine::inspect_citation_declaration(source).map_err(|_| changed())?
        else {
            continue;
        };
        let mut works = vec![];
        for entry in declaration.entries {
            let prefix = format!("weftext-node://{}/", package.manifest.workspace_id);
            let node = entry.work.strip_prefix(&prefix).ok_or_else(unavailable)?;
            let matches = source_entries
                .iter()
                .filter(|(_, id)| id == node)
                .collect::<Vec<_>>();
            let [(work_path, _)] = matches.as_slice() else {
                return Err(unavailable());
            };
            let configured_files = inner.files_view(actor)?;
            let candidates = configured_files
                .iter()
                .filter(|(id, f)| {
                    f.path.replace('\\', "/").eq_ignore_ascii_case(work_path)
                        && access(inner, actor, id, true, false).is_ok()
                })
                .collect::<Vec<_>>();
            let [(resource, file)] = candidates.as_slice() else {
                return Err(unavailable());
            };
            let q = inner
                .core
                .qualified_node_at(&inner.config.root, &inner.config.root.join(&file.path))
                .map_err(|_| unavailable())?;
            if q.owner.workspace_id != package.manifest.workspace_id || q.owner.node_id != node {
                return Err(unavailable());
            }
            let bytes = package.files.get(work_path).ok_or_else(changed)?;
            if q.source_hash
                != weftext_engine::source_hash(std::str::from_utf8(bytes).map_err(|_| changed())?)
            {
                return Err(changed());
            }
            let profile = weftext_data::narrow_profile(
                std::str::from_utf8(bytes).map_err(|_| changed())?,
                &schema,
                &BTreeSet::from([PUBLICATION.into()]),
                None,
            )
            .map_err(|_| unavailable())?;
            let (_, _, types) = weftext_data::narrow_query_metadata(
                std::str::from_utf8(bytes).map_err(|_| changed())?,
                &schema,
            )
            .map_err(|_| unavailable())?;
            if profile.node_id != node || !types.iter().any(|t| t == "library/work") {
                return Err(unavailable());
            }
            let field = profile
                .fields
                .iter()
                .find(|f| f.field_id == PUBLICATION)
                .ok_or_else(unavailable)?;
            let publication = field.cell.native().cloned().ok_or_else(unavailable)?;
            let basis=format!("publication:{}",digest(&serde_json::to_vec(&json!({"publication":publication,"shape":field.shape,"type":"library/work","owner":q.owner})).map_err(|_|changed())?));
            works.push(CitationWork {
                key: entry.key,
                work: FrozenWork {
                    entity_ref: entry.work,
                    source_basis: basis,
                    publication,
                },
            });
            authority.works.insert((*resource).clone());
        }
        let bib =
            project_bibliography(&works, BibliographyFormat::Bibtex).map_err(|_| changed())?;
        weftext_export::exchange::append_generated_bibliography(package, owner_id, bib.bytes)
            .map_err(|_| {
                ApiError::bad(
                    UiMessage::error_api_hosted_citations_offline_input_resource_conflict(),
                )
            })?;
    }
    authority.binding = binding(inner, &authority.works)?;
    Ok(authority)
}
pub(super) fn freeze(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source: &str,
) -> Result<Option<Capture>, ApiError> {
    let Some(declaration) = weftext_engine::inspect_citation_declaration(source).map_err(|_| {
        ApiError::bad(UiMessage::error_api_hosted_citations_static_work_declaration_required())
    })?
    else {
        return Ok(None);
    };
    let mut selections = vec![];
    // Only candidates with their independent publication/use grants are ever
    // qualified. No inventory or target title search takes place here.
    let candidates = inner
        .files_view(actor)?
        .keys()
        .filter(|id| access(inner, actor, id, false, false).is_ok())
        .cloned()
        .collect::<Vec<_>>();
    let wanted = declaration
        .entries
        .iter()
        .map(|e| e.work.clone())
        .collect::<BTreeSet<_>>();
    let mut refs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in candidates {
        let file = &inner.file(&id)?;
        if file
            .expected_node
            .as_ref()
            .is_some_and(|n| !wanted.contains(&uri(n)))
        {
            continue;
        }
        if let Ok(q) = inner
            .core
            .qualified_node_at(&inner.config.root, &inner.config.root.join(&file.path))
        {
            let r = uri(&q.owner);
            if wanted.contains(&r) {
                refs.entry(r).or_default().push(id);
            }
        }
    }
    for entry in &declaration.entries {
        let ids = refs.get(&entry.work).ok_or_else(unavailable)?;
        let [id] = ids.as_slice() else {
            return Err(unavailable());
        };
        selections.push((entry.key.clone(), id.clone()));
    }
    let mut capture = capture_works(inner, actor, &selections, Some((resource, source)))?;
    let file = inner.file(resource).ok().ok_or_else(unavailable)?;
    let path = Path::new(&file.path)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("weftext.citations.bib")
        .to_string_lossy()
        .replace('\\', "/");
    // Exact configured inputs are checked again by inject. The filesystem slot
    // must also be absent; no unconfigured real author resource is overwritten.
    if inner.config.root.join(&path).exists() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_generated_bibliography_occupied(),
        ));
    }
    let absolute = inner.config.root.join(&path).to_string_lossy().into_owned();
    let missing = inner
        .core
        .read_snapshot(&inner.config.root, std::slice::from_ref(&absolute))
        .map_err(|_| changed())?
        .files
        .into_iter()
        .next()
        .ok_or_else(changed)?;
    let witness = inner
        .core
        .take_snapshot_witness(&missing.base_id)
        .map_err(|_| changed())?;
    if missing.bytes.is_some() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_generated_bibliography_occupied(),
        ));
    }
    capture.bytes = capture.bytes.saturating_add(witness.retained_bytes());
    if capture.bytes > MAX_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_citation_input_budget(),
        ));
    }
    capture.virtual_witness = Some((absolute, witness));
    capture.virtual_path = Some(path);
    capture.validate(inner, actor)?;
    Ok(Some(capture))
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/citations", get(catalog))
        .route("/api/hosted/citations/propose", post(propose))
        .route("/api/hosted/citations/editor", post(read_editor))
        .route("/api/hosted/citations/edit", post(edit_native))
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeBasis {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeEdit {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: String,
    generation: String,
    intent: Value,
}
fn native_basis(
    inner: &mut HostedInner,
    auth: &Auth,
    req: &NativeBasis,
    observe: bool,
    write: bool,
) -> Result<(String, HostedSession), ApiError> {
    let (actor, session, rights) = inner.session(auth, &req.session_id)?;
    let file = author_file(inner, &session.resource)?;
    let path = inner.config.root.join(&file.path);
    let owner = inner
        .core
        .qualified_node_at(&inner.config.root, &path)
        .map_err(|_| unavailable())?;
    if file
        .expected_node
        .as_ref()
        .is_some_and(|expected| expected != &owner.owner)
        || weftext_engine::inspect_metadata(&req.source).id.as_deref()
            != Some(owner.owner.node_id.as_str())
    {
        return Err(unavailable());
    }
    inner
        .core
        .validate_snapshot_witness(&inner.config.root, &path, &owner.witness)
        .map_err(|_| changed())?;
    if write && (!rights.write || !inner.file(&session.resource)?.editable) {
        return Err(unavailable());
    }
    writing_api::validate_source(&req.source, Some(&req.source_hash))?;
    if observe {
        writing_api::observe_version(
            &mut inner.editor_versions,
            &req.session_id,
            &req.source,
            req.serial,
        )?;
    } else {
        writing_api::exact_version(
            &inner.editor_versions,
            &req.session_id,
            &req.source,
            req.serial,
        )?;
    }
    Ok((actor, session))
}
fn native_profile(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source: &str,
) -> Result<Value, ApiError> {
    let mut choices = BTreeMap::<String, Value>::new();
    let mut sources = Vec::<(String, String, String)>::new();
    let mut source_bytes = 0usize;
    let mut partial = false;
    let ids = inner
        .files_view(actor)?
        .keys()
        .filter(|id| {
            inner.rights(actor, id).is_ok() || access(inner, actor, id, false, false).is_ok()
        })
        .cloned()
        .collect::<Vec<_>>();
    partial |= ids.len() > 512;
    for id in ids.into_iter().take(512) {
        let file = inner.file(&id)?.clone();
        if inner.rights(actor, &id).is_ok() {
            let readable = (|| {
                let file = author_file(inner, &id)?;
                let path = inner.config.root.join(&file.path);
                let node = inner
                    .core
                    .qualified_node_at(&inner.config.root, &path)
                    .map_err(|_| unavailable())?;
                if file
                    .expected_node
                    .as_ref()
                    .is_some_and(|expected| expected != &node.owner)
                {
                    return Err(unavailable());
                }
                let files = inner.files(actor, std::slice::from_ref(&id))?;
                let saved = files
                    .get(&file.path)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .ok_or_else(unavailable)?;
                if weftext_engine::source_hash(saved) != node.source_hash {
                    return Err(changed());
                }
                inner
                    .core
                    .validate_snapshot_witness(&inner.config.root, &path, &node.witness)
                    .map_err(|_| changed())?;
                let selected = if id == resource { source } else { saved };
                if weftext_engine::inspect_metadata(selected).id.as_deref()
                    != Some(node.owner.node_id.as_str())
                {
                    return Err(unavailable());
                }
                let title = weftext_engine::inspect_editor(selected)
                    .title
                    .text
                    .unwrap_or_else(|| file.title.clone());
                Ok((uri(&node.owner), title, selected.to_owned()))
            })();
            if let Ok((reference, title, source)) = readable {
                source_bytes = source_bytes.saturating_add(source.len());
                if source_bytes <= 16 * 1024 * 1024 {
                    sources.push((reference.clone(), title.clone(), source));
                } else {
                    partial = true;
                }
                choices.insert(reference.clone(),json!({"resource":id,"reference":reference,"title":title,"citationReady":false,"reason":ErrorDescription::from(UiMessage::error_api_hosted_citations_publication_fields_required())}));
            } else {
                partial = true;
            }
        }
        if access(inner, actor, &id, false, false).is_ok() {
            match capture_works(
                inner,
                actor,
                &[("validation".into(), id.clone())],
                Some((resource, source)),
            ) {
                Ok(capture) => {
                    let work = &capture.works[0];
                    let publication = &work.work.publication;
                    let reference = work.work.entity_ref.clone();
                    let title = publication
                        .member("title")
                        .and_then(weftext_data::NativeValue::text)
                        .unwrap_or(&file.title);
                    let suggestion = publication
                        .member("citation_key")
                        .and_then(weftext_data::NativeValue::text)
                        .unwrap_or("work");
                    choices.insert(reference.clone(),json!({"resource":id,"reference":reference,"title":title,"citationReady":true,"suggestedKey":suggestion,"reason":null}));
                }
                Err(_) => {
                    partial = true;
                }
            }
        }
    }
    let mut editor = weftext_engine::inspect_native_citation_editor(source);
    let style_safe = library::environment(inner, actor)
        .and_then(|environment| {
            let mut worker = WorkerRequest {
                source: source.into(),
                path: inner.file(resource)?.path.clone().into(),
                root: inner.config.root.clone(),
                paged: false,
                style: None,
                frozen_files: BTreeMap::new(),
            };
            environment.apply(&mut worker)?;
            Ok(worker
                .style
                .as_ref()
                .is_none_or(weftext_engine::style_allows_rich_editing))
        })
        .unwrap_or(false);
    if !style_safe {
        editor.rename_available = false;
        editor.rename_reason =
            Some(UiMessage::error_api_hosted_citations_rename_coverage_unproven().into());
    }
    let bindings=editor.bindings.iter().map(|entry|{let slots=editor.citations.iter().filter(|slot|slot.key==entry.key && slot.binding_proven).collect::<Vec<_>>();json!({"key":entry.key,"work":entry.work,"state":if slots.is_empty(){"boundOnly"}else{"authored"},"authoredSlots":slots})}).collect::<Vec<_>>();
    let selected_ref = inner
        .core
        .qualified_node_at(
            &inner.config.root,
            &inner.config.root.join(&inner.file(resource)?.path),
        )
        .map(|node| uri(&node.owner))
        .map_err(|_| unavailable())?;
    let mut incoming = Vec::new();
    for (reference, title, source) in &sources {
        if !crate::citation_api::owns_single_virtual_bibliography(source) {
            let inspected = weftext_engine::inspect_native_citation_editor(source);
            if !weftext_engine::native_citation_owner_complete(source)
                || !inspected.bindings.is_empty()
                || !inspected.citations.is_empty()
            {
                partial = true;
            }
            continue;
        }
        let inspected = weftext_engine::inspect_native_citation_editor(source);
        let keys = inspected
            .bindings
            .iter()
            .filter(|binding| binding.work == selected_ref)
            .map(|binding| binding.key.as_str())
            .collect::<BTreeSet<_>>();
        let slots = inspected
            .citations
            .iter()
            .filter(|slot| slot.binding_proven && keys.contains(slot.key.as_str()))
            .collect::<Vec<_>>();
        if !slots.is_empty() {
            incoming.push(json!({"title":title,"reference":reference,"slots":slots,"visibility":"unconfirmed"}));
        }
    }
    if !style_safe {
        incoming.clear();
        partial = true;
    }
    let generation=digest(&serde_json::to_vec(&json!({"source":weftext_engine::source_hash(source),"choices":choices,"incoming":incoming,"actor":actor,"epoch":inner.actor_generation(actor),"deployment":inner.config.deployment})).map_err(|_|changed())?);
    let links=weftext_knowledge::native_links(source).into_iter().filter(|link|link.kind=="native").map(|link|{let reference=link.target.split('#').next().unwrap_or("");let title=choices.get(reference).and_then(|choice|choice["title"].as_str());json!({"range":{"from":link.from,"to":link.to,"utf16From":link.utf16_from,"utf16To":link.utf16_to,"raw":source.get(link.from..link.to).unwrap_or(""),"writable":true},"target":link.target,"title":title,"editable":title.is_some()&&link.body_explicit,"reason":if title.is_some()&&link.body_explicit {None} else {Some(ErrorDescription::from(UiMessage::error_api_hosted_citations_native_link_edit_scope()))}})}).collect::<Vec<_>>();
    Ok(
        json!({"ownerReference":selected_ref,"generation":generation,"coverage":if partial{"partial"}else{"complete"},"choices":choices.into_values().collect::<Vec<_>>(),"editor":editor,"bindings":bindings,"links":links,"incoming":incoming,"bibliographyOwnership":if style_safe && crate::citation_api::owns_single_virtual_bibliography(source){"singleVirtual"}else{"unavailable"},"displayStatus":"unavailable","displayExplanation":UiMessage::error_api_hosted_citations_display_evidence_scope()}),
    )
}
async fn read_editor(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<NativeBasis>,
) -> ApiResult {
    let request = req.clone();
    let check = auth.clone();
    let (profile, frozen) = locked(&state, move |inner| {
        let (actor, session) = native_basis(inner, &check, &request, true, false)?;
        let profile = native_profile(inner, &actor, &session.resource, &request.source)?;
        let frozen = editor::freeze(inner, &actor, &session.resource, &request.source).ok();
        Ok((profile, frozen))
    })
    .await?;
    let compiled = if let Some((input, dependencies, environment)) = frozen {
        let _permit = state.render_gate.try_acquire().map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "busy",
                UiMessage::error_api_hosted_citations_citation_preview_busy(),
            )
        })?;
        Some((
            worker_failure::observe_hosted_result(
                worker_failure::HostedContext::CitationRead,
                render_worker_with(input, &state.workers).await,
            ),
            dependencies,
            environment,
        ))
    } else {
        None
    };
    locked(&state, move |inner| {
        deliver_native_read(inner, &auth, &req, profile, compiled)
    })
    .await
}
fn deliver_native_read(
    inner: &mut HostedInner,
    auth: &Auth,
    req: &NativeBasis,
    mut profile: Value,
    compiled: Option<ReferenceCompile>,
) -> ApiResult {
    let (actor, session) = native_basis(inner, auth, req, false, false)?;
    let fresh = native_profile(inner, &actor, &session.resource, &req.source)?;
    if fresh["generation"] != profile["generation"] {
        return Err(changed());
    }
    if let Some((_, dependencies, environment)) = &compiled {
        editor::validate_inputs(inner, &actor, &session.resource, dependencies)?;
        environment.recheck(inner, &actor)?;
    }
    // With no frozen compile, deliver the newly qualified capability
    // profile itself. A partial generation is not an environment witness.
    profile = fresh;
    if let Some((Ok(output), _, _)) = compiled {
        if output["ok"] == true {
            if let Ok(tree) =
                serde_json::from_value::<weftext_engine::RenderNode>(output["tree"].clone())
            {
                profile["displayed"] = json!(crate::citation_api::displayed_citations(&tree));
                profile["unlabelledTargets"] = json!(
                    crate::citation_api::unlabelled_reference_targets(&req.source, &tree)
                );
                crate::citation_api::classify_reference_controls(
                    &mut profile["editor"],
                    Some(&tree),
                );
                profile["displayStatus"] = json!("nativeCompileAvailable");
            }
            let labels = serde_json::from_value::<Vec<weftext_engine::CompiledLabel>>(
                output["labels"].clone(),
            )
            .unwrap_or_default();
            if let Some(targets) = profile["editor"]["targets"].as_array_mut() {
                targets.retain(|target| {
                    matches!(
                        target["kind"].as_str(),
                        Some("heading" | "figure" | "table" | "equation" | "footnote")
                    ) && labels.iter().any(|label| {
                        label.label == target["label"].as_str().unwrap_or("")
                            && label.count == 1
                            && label.origins.iter().any(|origin| {
                                origin.is_main
                                    && origin.from
                                        == target["range"]["from"].as_u64().unwrap_or(u64::MAX)
                                            as usize
                            })
                    })
                });
            }
        } else {
            profile["editor"]["targets"] = json!([]);
        }
    } else {
        profile["editor"]["targets"] = json!([]);
    }
    if profile["displayStatus"] != "nativeCompileAvailable" {
        profile["coverage"] = json!("partial");
    }
    if serde_json::to_vec(&profile).map_err(|_| changed())?.len() > 8 * 1024 * 1024 {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_citation_catalog_budget(),
        ));
    }
    Ok(Json(
        json!({"resource":session.resource,"serial":req.serial,"sourceHash":req.source_hash,"profile":profile}),
    ))
}
async fn edit_native(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<NativeEdit>,
) -> ApiResult {
    let basis = NativeBasis {
        session_id: req.session_id.clone(),
        source: req.source.clone(),
        serial: req.serial,
        source_hash: req.source_hash.clone(),
    };
    // Cross-reference targets require a new official compile, with the same
    // independent input grants rechecked after worker latency.
    let compiled = if matches!(
        req.intent["kind"].as_str(),
        Some("nativeReferenceInsert" | "nativeReferenceInsertWithLabel")
    ) {
        let check = auth.clone();
        let request = basis.clone();
        let frozen = locked(&state, move |inner| {
            let (actor, session) = native_basis(inner, &check, &request, false, true)?;
            editor::freeze(inner, &actor, &session.resource, &request.source)
        })
        .await?;
        let _permit = state.render_gate.try_acquire().map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "busy",
                UiMessage::error_api_hosted_citations_citation_compile_busy(),
            )
        })?;
        Some((
            worker_failure::observe_hosted_result(
                worker_failure::HostedContext::CitationEdit,
                render_worker_with(frozen.0, &state.workers).await,
            ),
            frozen.1,
            frozen.2,
        ))
    } else {
        None
    };
    locked(&state, move |inner| {
        deliver_native_edit(inner, &auth, &req, &basis, compiled)
    })
    .await
}
type ReferenceCompile = (
    Result<Value, ApiError>,
    BTreeMap<String, String>,
    library::Environment,
);
fn deliver_native_edit(
    inner: &mut HostedInner,
    auth: &Auth,
    req: &NativeEdit,
    basis: &NativeBasis,
    compiled: Option<ReferenceCompile>,
) -> ApiResult {
    let (actor, session) = native_basis(inner, auth, basis, false, true)?;
    let profile = native_profile(inner, &actor, &session.resource, &basis.source)?;
    if profile["generation"].as_str() != Some(req.generation.as_str()) {
        return Err(changed());
    }
    if let Some((_, dependencies, environment)) = &compiled {
        editor::validate_inputs(inner, &actor, &session.resource, dependencies)?;
        environment.recheck(inner, &actor)?;
    }
    if matches!(
        req.intent["kind"].as_str(),
        Some("nativeCitationRename" | "nativeReferenceRename")
    ) && profile["editor"]["renameAvailable"] != true
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_rename_qualification_unavailable(),
        ));
    }
    let s = |key: &str| {
        req.intent[key].as_str().ok_or_else(|| {
            ApiError::bad(UiMessage::error_api_hosted_required_field_missing(
                weftext_messages::params::ErrorApiHostedRequiredFieldMissing { field: key.into() },
            ))
        })
    };
    let at = || {
        req.intent["at"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                ApiError::bad(UiMessage::error_api_hosted_citations_insertion_position_required())
            })
    };
    let fail = |error: weftext_engine::EngineError| {
        ApiError::from_engine(StatusCode::CONFLICT, "unsupportedCitationEdit", error)
    };
    let range = || {
        serde_json::from_value::<weftext_engine::SourceRange>(req.intent["range"].clone()).map_err(
            |_| ApiError::bad(UiMessage::error_api_hosted_citations_source_range_required()),
        )
    };
    let form = || {
        serde_json::from_value::<weftext_engine::CitationForm>(req.intent["form"].clone()).map_err(
            |_| ApiError::bad(UiMessage::error_api_hosted_citations_unknown_citation_form()),
        )
    };
    if req.intent["form"] == "normal"
        && matches!(
            s("kind")?,
            "nativeReferenceInsert" | "nativeReferenceInsertWithLabel"
        )
    {
        let from = if s("kind")? == "nativeReferenceInsertWithLabel" {
            req.intent["target"]["from"].as_u64().unwrap_or(u64::MAX) as usize
        } else {
            weftext_engine::inspect_native_citation_editor(&basis.source)
                .targets
                .into_iter()
                .find(|target| Some(target.label.as_str()) == req.intent["label"].as_str())
                .map_or(usize::MAX, |target| target.range.from)
        };
        let numbered = compiled
            .as_ref()
            .and_then(|(output, _, _)| output.as_ref().ok())
            .and_then(|output| {
                serde_json::from_value::<weftext_engine::RenderNode>(output["tree"].clone()).ok()
            })
            .and_then(|tree| crate::citation_api::reference_numbered_at(&tree, from))
            .unwrap_or(false);
        if !numbered {
            return Err(reference_number_unavailable(
                compiled
                    .as_ref()
                    .and_then(|(output, _, _)| output.as_ref().err()),
            ));
        }
    }
    if s("kind")? == "nativeReferenceInsert"
        && !weftext_engine::inspect_native_citation_editor(&basis.source)
            .targets
            .iter()
            .any(|target| {
                Some(target.label.as_str()) == req.intent["label"].as_str()
                    && matches!(
                        target.kind.as_str(),
                        "heading" | "figure" | "table" | "equation" | "footnote"
                    )
            })
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_citations_native_reference_target_unavailable(),
        ));
    }
    let edited = match s("kind")? {
        "nativeCitationInsert" => {
            let work = s("work")?;
            let choice = profile["choices"]
                .as_array()
                .and_then(|choices| {
                    choices.iter().find(|choice| {
                        choice["reference"] == work && choice["citationReady"] == true
                    })
                })
                .ok_or_else(unavailable)?;
            weftext_engine::insert_native_citation(
                &basis.source,
                &basis.source_hash,
                at()?,
                work,
                choice["suggestedKey"].as_str().unwrap_or("work"),
                req.intent["key"].as_str().filter(|key| !key.is_empty()),
                form()?,
                req.intent["supplement"].as_str().unwrap_or(""),
            )
            .map_err(fail)?
        }
        "nativeCitationUpdate" => weftext_engine::update_native_citation_options(
            &basis.source,
            &basis.source_hash,
            &range()?,
            if req.intent["form"].is_null() {
                None
            } else {
                Some(form()?)
            },
            req.intent["supplement"].as_str(),
        )
        .map_err(fail)?,
        "nativeCitationStyle" => weftext_engine::update_native_citation_style(
            &basis.source,
            &basis.source_hash,
            &range()?,
            req.intent["style"]
                .as_str()
                .filter(|style| !style.is_empty()),
        )
        .map_err(fail)?,
        "nativeCitationRename" => weftext_engine::rename_native_citation_key(
            &basis.source,
            &basis.source_hash,
            s("old")?,
            s("new")?,
        )
        .map_err(fail)?,
        "nativeCitationLink" => {
            let target = s("target")?;
            let choice = profile["choices"]
                .as_array()
                .and_then(|choices| choices.iter().find(|choice| choice["reference"] == target))
                .ok_or_else(unavailable)?;
            weftext_engine::insert_native_link(
                &basis.source,
                &basis.source_hash,
                at()?,
                target,
                req.intent["text"]
                    .as_str()
                    .filter(|text| !text.is_empty())
                    .unwrap_or(choice["title"].as_str().unwrap_or("")),
            )
            .map_err(fail)?
        }
        "nativeCitationLinkText" => {
            let range = range()?;
            let link = weftext_knowledge::native_links(&basis.source)
                .into_iter()
                .find(|link| {
                    link.from == range.from && link.to == range.to && link.kind == "native"
                })
                .ok_or_else(changed)?;
            let reference = link.target.split('#').next().unwrap_or("");
            let title = profile["choices"]
                .as_array()
                .and_then(|choices| {
                    choices
                        .iter()
                        .find(|choice| choice["reference"] == reference)
                })
                .and_then(|choice| choice["title"].as_str())
                .ok_or_else(unavailable)?;
            weftext_engine::update_native_link_text(
                &basis.source,
                &basis.source_hash,
                &range,
                title,
            )
            .map_err(fail)?
        }
        "nativeReferenceInsert" => {
            let label = s("label")?;
            let target = weftext_engine::inspect_native_citation_editor(&basis.source)
                .targets
                .into_iter()
                .find(|target| target.label == label && target.unique)
                .ok_or_else(unavailable)?;
            let Some((output, dependencies, environment)) = compiled else {
                return Err(changed());
            };
            editor::validate_inputs(inner, &actor, &session.resource, &dependencies)?;
            environment.recheck(inner, &actor)?;
            let output = output.map_err(changed_after_worker)?;
            let labels = serde_json::from_value::<Vec<weftext_engine::CompiledLabel>>(
                output["labels"].clone(),
            )
            .unwrap_or_default();
            if output["ok"] != true
                || !labels.iter().any(|entry| {
                    entry.label == label
                        && entry.count == 1
                        && entry
                            .origins
                            .iter()
                            .any(|origin| origin.is_main && origin.from == target.range.from)
                })
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_citations_unique_execution_origin_required(),
                ));
            }
            if s("form")? == "text" {
                weftext_engine::insert_native_cross_link(
                    &basis.source,
                    &basis.source_hash,
                    at()?,
                    label,
                    s("text")?,
                )
                .map_err(fail)?
            } else {
                if !matches!(s("form")?, "normal" | "page") {
                    return Err(ApiError::bad(
                        UiMessage::error_api_hosted_citations_unknown_cross_reference_form(),
                    ));
                }
                weftext_engine::insert_native_cross_reference(
                    &basis.source,
                    &basis.source_hash,
                    at()?,
                    label,
                    s("form")? == "page",
                )
                .map_err(fail)?
            }
        }
        "nativeReferenceInsertWithLabel" => {
            let target =
                serde_json::from_value::<weftext_engine::SourceRange>(req.intent["target"].clone())
                    .map_err(|_| {
                        ApiError::bad(UiMessage::error_api_hosted_citations_object_range_required())
                    })?;
            let Some((output, dependencies, environment)) = compiled else {
                return Err(changed());
            };
            editor::validate_inputs(inner, &actor, &session.resource, &dependencies)?;
            environment.recheck(inner, &actor)?;
            let output = output.map_err(changed_after_worker)?;
            let tree = serde_json::from_value::<weftext_engine::RenderNode>(output["tree"].clone())
                .map_err(|_| changed())?;
            if output["ok"] != true
                || !crate::citation_api::unlabelled_reference_targets(&basis.source, &tree)
                    .iter()
                    .any(|candidate| candidate["range"] == req.intent["target"])
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_citations_unique_native_object_required(),
                ));
            }
            let form = s("form")?;
            if !matches!(form, "normal" | "page") {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_citations_labelled_reference_form_unsupported(),
                ));
            }
            weftext_engine::insert_native_cross_reference_with_label(
                &basis.source,
                &basis.source_hash,
                at()?,
                &target,
                s("label")?,
                form == "page",
            )
            .map_err(fail)?
        }
        "nativeReferenceRename" => weftext_engine::rename_native_label(
            &basis.source,
            &basis.source_hash,
            s("old")?,
            s("new")?,
        )
        .map_err(fail)?,
        _ => {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_citations_unknown_citation_operation(),
            ));
        }
    };
    writing_api::validate_source(&edited.source, None)?;
    let patches=edited.patches.iter().map(|patch|json!({"from":patch.utf16_from,"to":patch.utf16_to,"text":patch.insert,"expected":patch.expected,"byteFrom":patch.from,"byteTo":patch.to})).collect::<Vec<_>>();
    Ok(Json(
        json!({"resource":session.resource,"serial":basis.serial,"sourceHash":basis.source_hash,"patchUnit":"utf16","source":edited.source,"patches":patches}),
    ))
}

async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{let actor=inner.actor(&auth)?;let ids=inner.files_view(&actor)?.keys().filter(|id|access(inner,&actor,id,false,false).is_ok()).cloned().collect::<Vec<_>>();let mut partial=false;let mut works=vec![];
        for id in ids {match selected(inner,&actor,std::slice::from_ref(&id)) {Ok(c)=> {let w=&c.works[0];let title=w.work.publication.member("title").and_then(weftext_data::NativeValue::text).map(str::to_owned).unwrap_or(inner.file(&id)?.title.clone());let grant=&inner.grant(&actor,&id)?;works.push(json!({"resource":id,"reference":w.work.entity_ref,"title":title,"status":"ready","use":true,"export":grant.citations.export && grant.outputs.bibliography.export,"download":grant.citations.export && grant.citations.download && grant.outputs.bibliography.export && grant.outputs.bibliography.download}));},Err(_)=> {partial=true;works.push(json!({"resource":id,"title":inner.file(&id)?.title,"status":"unavailable","use":true,"export":false,"download":false}));}}
            if serde_json::to_vec(&works).map_err(|_|changed())?.len()>8*1024*1024 {return Err(ApiError::bad(UiMessage::error_api_hosted_citations_bibliography_catalog_budget()));}
        }
        Ok(json!({"works":works,"coverage":if partial{"partial"}else{"complete"}}))}).await?))
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Propose {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: String,
    key: String,
    work: String,
}
async fn propose(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Propose>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{
        let (actor,session,rights)=inner.session(&auth,&req.session_id)?;
        if !rights.write || !inner.file(&session.resource)?.editable {return Err(unavailable());}
        writing_api::validate_source(&req.source,Some(&req.source_hash))?;
        writing_api::exact_version(&inner.editor_versions,&req.session_id,&req.source,req.serial)?;
        if req.key.len()>256 || !weftext_engine::valid_citation_key(&req.key) {return Err(ApiError::bad(UiMessage::error_api_hosted_citations_bounded_ascii_citation_key_required()));}
        let work=selected(inner,&actor,std::slice::from_ref(&req.work))?;
        let reference=work.works[0].work.entity_ref.clone();
        let declaration=weftext_engine::inspect_citation_declaration(&req.source).map_err(|_|ApiError::bad(UiMessage::error_api_hosted_citations_bibliography_declaration_uneditable()))?;
        let mut entries=declaration.as_ref().map(|d|d.entries.clone()).unwrap_or_default();
        if entries.iter().any(|e|e.key==req.key) {return Err(ApiError::bad(UiMessage::error_api_hosted_citations_citation_key_exists()));}
        entries.push(weftext_engine::CitationEntry{key:req.key.clone(),work:reference});
        let text=format!("#metadata((kind: \"weftext.citations\", entries: ({}))) <weftext-citations>\n",entries.iter().map(|e|format!("(key: {}, work: {}),",serde_json::to_string(&e.key).unwrap(),serde_json::to_string(&e.work).unwrap())).collect::<String>());
        let before_info=weftext_engine::inspect(&req.source);
        let (from,to)=if let Some(d)=&declaration {
            let from=d.range.from;
            if from==0 || req.source.get(from-1..from)!=Some("#") {return Err(changed());}
            (from-1,d.range.to)
        }else{
            let at=before_info.metadata.as_ref().map_or(0,|m|m.to);
            (at,at)
        };
        if req.source.get(from..to).is_none() {return Err(changed());}
        let suffix=format!("\n@{}\n{}",req.key,if declaration.is_none(){"#bibliography(\"weftext.citations.bib\")\n"}else{""});
        let text=format!("\n{text}");
        let mut after=req.source.clone();after.push_str(&suffix);after.replace_range(from..to,&text);
        weftext_engine::inspect_citation_declaration(&after).map_err(|_|ApiError::bad(UiMessage::error_api_hosted_citations_static_declaration_proposal_unavailable()))?.ok_or_else(changed)?;
        let after_info=weftext_engine::inspect(&after);
        if !after_info.syntax_valid || after_info.id!=before_info.id {return Err(changed());}
        work.validate(inner,&actor)?;
        let utf16=|n|req.source[..n].encode_utf16().count();
        let patches=if from==to && to==req.source.len() {json!([{"from":utf16(from),"to":utf16(to),"text":format!("{text}{suffix}"),"byteFrom":from,"byteTo":to,"expected":""}])}else{json!([{"from":utf16(from),"to":utf16(to),"text":text,"byteFrom":from,"byteTo":to,"expected":req.source[from..to]},{"from":utf16(req.source.len()),"to":utf16(req.source.len()),"text":suffix,"byteFrom":req.source.len(),"byteTo":req.source.len(),"expected":""}])};
        Ok(json!({"resource":session.resource,"serial":req.serial,"sourceHash":req.source_hash,"patchUnit":"utf16","source":after,"patches":patches}))
    }).await?))
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    const PRIVATE: &str = "PRIVATE WORK BODY 绝不交付";
    #[test]
    fn actual_citation_read_rechecks_frozen_environment_on_success_and_degraded_failure() {
        for failure in [false, true] {
            let (_temp, state, auth, doc, _) = fixture();
            let mut inner = state.inner.lock().unwrap();
            let source = doc["source"].as_str().unwrap().to_owned();
            let basis = NativeBasis {
                session_id: doc["sessionId"].as_str().unwrap().into(),
                source: source.clone(),
                serial: 1,
                source_hash: weftext_engine::source_hash(&source),
            };
            let result = || {
                if failure {
                    Err(crate::worker_failure::diagnostics_tests::original_failure(
                        crate::worker_failure::Kind::OutputRejected,
                    ))
                } else {
                    Ok(json!({"ok":true,"tree":null}))
                }
            };
            let profile = native_profile(&mut inner, "alice", "Shared", &source).unwrap();
            let (_, dependencies, environment) =
                editor::freeze(&mut inner, "alice", "Shared", &source).unwrap();
            assert!(
                deliver_native_read(
                    &mut inner,
                    &auth,
                    &basis,
                    profile,
                    Some((result(), dependencies, environment))
                )
                .is_ok()
            );
            let profile = native_profile(&mut inner, "alice", "Shared", &source).unwrap();
            let (_, dependencies, environment) =
                editor::freeze(&mut inner, "alice", "Shared", &source).unwrap();
            let root = inner.config.root.clone();
            let settings = inner
                .core
                .read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)
                .unwrap();
            std::fs::write(
                settings.path,
                br#"{"version":1,"defaultStyle":{"enabled":false}}"#,
            )
            .unwrap();
            let error = deliver_native_read(
                &mut inner,
                &auth,
                &basis,
                profile,
                Some((result(), dependencies, environment)),
            )
            .err()
            .unwrap();
            assert!(error.private_worker_failure.is_none());
        }
    }
    #[test]
    fn normal_reference_success_and_worker_failure_recheck_changed_actual_environment_before_interpretation()
     {
        for failure in [false, true] {
            let (_temp, state, auth, doc, _) = fixture();
            let mut inner = state.inner.lock().unwrap();
            let source = doc["source"].as_str().unwrap().to_owned();
            let basis = NativeBasis {
                session_id: doc["sessionId"].as_str().unwrap().into(),
                source: source.clone(),
                serial: 1,
                source_hash: weftext_engine::source_hash(&source),
            };
            let profile = native_profile(&mut inner, "alice", "Shared", &source).unwrap();
            let req = NativeEdit {
                session_id: basis.session_id.clone(),
                source,
                serial: basis.serial,
                source_hash: basis.source_hash.clone(),
                generation: profile["generation"].as_str().unwrap().into(),
                intent: json!({"kind":"nativeReferenceInsert", "label":"not-numbered", "form":"normal", "at":0}),
            };
            let output = || {
                if failure {
                    Err(crate::worker_failure::diagnostics_tests::original_failure(
                        crate::worker_failure::Kind::OutputRejected,
                    ))
                } else {
                    Ok(json!({"ok":true,"tree":null}))
                }
            };
            let (_, dependencies, environment) =
                editor::freeze(&mut inner, "alice", "Shared", &basis.source).unwrap();
            let first = deliver_native_edit(
                &mut inner,
                &auth,
                &req,
                &basis,
                Some((output(), dependencies, environment)),
            )
            .err()
            .unwrap();
            assert_eq!(
                first.explanation,
                UiMessage::error_api_hosted_citations_reference_number_unavailable()
            );
            let (_, dependencies, environment) =
                editor::freeze(&mut inner, "alice", "Shared", &basis.source).unwrap();
            let root = inner.config.root.clone();
            let settings = inner
                .core
                .read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)
                .unwrap();
            std::fs::write(
                settings.path,
                br#"{"version":1,"defaultStyle":{"enabled":false}}"#,
            )
            .unwrap();
            let error = deliver_native_edit(
                &mut inner,
                &auth,
                &req,
                &basis,
                Some((output(), dependencies, environment)),
            )
            .err()
            .unwrap();
            assert_ne!(
                error.explanation,
                UiMessage::error_api_hosted_citations_reference_number_unavailable()
            );
            assert!(error.private_worker_failure.is_none());
        }
    }

    #[test]
    fn native_catalog_never_discloses_a_configured_path_with_another_expected_owner() {
        let (_temp, state, _auth, doc, _reference) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let expected = inner
            .core
            .qualified_node_at(&root, &root.join("Shared/Shared.typ"))
            .unwrap()
            .owner;
        inner.config.files.get_mut("Target").unwrap().expected_node = Some(expected);
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Target")
            .unwrap()
            .read = true;
        let profile = native_profile(
            &mut inner,
            "alice",
            "Shared",
            doc["source"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(profile["coverage"], "partial");
        let encoded = profile.to_string();
        assert!(!encoded.contains("Granted Publication"));
        assert!(!encoded.contains(PRIVATE));
        assert!(
            !profile["choices"]
                .as_array()
                .unwrap()
                .iter()
                .any(|choice| choice["resource"] == "Target")
        );
    }
    #[tokio::test]
    async fn native_controls_reuse_work_in_one_draft_and_reject_revoked_or_stale_selection() {
        let (_temp, state, auth, doc, reference) = fixture();
        let source = doc["source"].as_str().unwrap().to_owned();
        let session = doc["sessionId"].as_str().unwrap().to_owned();
        let path = state
            .inner
            .lock()
            .unwrap()
            .config
            .root
            .join("Shared/Shared.typ");
        let original = std::fs::read(&path).unwrap();
        let profile = {
            let mut inner = state.inner.lock().unwrap();
            native_profile(&mut inner, "alice", "Shared", &source).unwrap()
        };
        assert!(!profile.to_string().contains(PRIVATE));
        let mut req = NativeEdit {
            session_id: session.clone(),
            source: source.clone(),
            serial: 1,
            source_hash: weftext_engine::source_hash(&source),
            generation: profile["generation"].as_str().unwrap().into(),
            intent: json!({"kind":"nativeCitationInsert","at":source.find("Saved B").unwrap(),"work":reference,"key":"reader2026","form":"prose","supplement":"第 12 页 ] #panic(\"never execute\")"}),
        };
        let result = edit_native(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(req.clone()),
        )
        .await
        .unwrap()
        .0;
        let after = result["source"].as_str().unwrap().to_owned();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(
            weftext_engine::inspect_citation_declaration(&after)
                .unwrap()
                .unwrap()
                .entries
                .len(),
            1
        );
        assert_eq!(
            weftext_engine::inspect_native_citation_editor(&after).citations[0].form,
            Some(weftext_engine::CitationForm::Prose)
        );
        {
            let mut inner = state.inner.lock().unwrap();
            writing_api::observe_version(&mut inner.editor_versions, &session, &after, 2).unwrap();
        }
        assert!(
            edit_native(
                State(state.clone()),
                Extension(auth.clone()),
                crate::RequestJson(req.clone())
            )
            .await
            .is_err()
        );
        let profile = {
            let mut inner = state.inner.lock().unwrap();
            native_profile(&mut inner, "alice", "Shared", &after).unwrap()
        };
        req.source = after.clone();
        req.serial = 2;
        req.source_hash = weftext_engine::source_hash(&after);
        req.generation = profile["generation"].as_str().unwrap().into();
        req.intent = json!({"kind":"nativeCitationInsert","at":after.find("Saved B").unwrap(),"work":reference,"form":"none","supplement":""});
        let second = edit_native(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(req.clone()),
        )
        .await
        .unwrap()
        .0;
        let second = second["source"].as_str().unwrap();
        assert_eq!(
            weftext_engine::inspect_citation_declaration(second)
                .unwrap()
                .unwrap()
                .entries
                .len(),
            1
        );
        assert_eq!(
            weftext_engine::inspect_native_citation_editor(second)
                .citations
                .len(),
            2
        );
        state
            .inner
            .lock()
            .unwrap()
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Target")
            .unwrap()
            .fields
            .clear();
        assert!(
            edit_native(State(state), Extension(auth), crate::RequestJson(req))
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
    fn fixture() -> (tempfile::TempDir, HostedState, Auth, Value, String) {
        let (temp, state, auth, doc, _, _) = history_api::test_fixture();
        let reference = {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let opened = inner
                .core
                .open(root.join("Target/Target.typ"), Some(root.clone()))
                .unwrap();
            let id = opened.node_id.as_ref().unwrap();
            let source = format!(
                "#metadata((id: \"{id}\", types: (\"paper\",), publication: (kind: \"article\", title: \"Granted Publication\", contributors: ((agent: (text: \"Author A\"), roles: (\"author\",)),)))) <weftext>\n{PRIVATE}\n"
            );
            inner
                .core
                .save(weftext_core::SaveRequest {
                    session_id: opened.session_id.clone(),
                    base_id: opened.base_id,
                    source,
                    operation_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap();
            inner.core.close_session(&opened.session_id).unwrap();
            inner.policy.users.get_mut("alice").unwrap().grants.insert(
                "Target".into(),
                super::super::Rights {
                    fields: BTreeMap::from([(
                        PUBLICATION.into(),
                        FieldRights {
                            read: true,
                            write: false,
                        },
                    )]),
                    citations: Rights {
                        r#use: true,
                        export: true,
                        download: true,
                    },
                    outputs: office::OutputRights {
                        bibliography: office::FormatRights {
                            export: true,
                            download: true,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            let q = inner
                .core
                .qualified_node_at(&root, &root.join("Target/Target.typ"))
                .unwrap();
            uri(&q.owner)
        };
        (temp, state, auth, doc, reference)
    }
    #[test]
    fn publication_slice_grants_no_source_and_basis_excludes_private_body_hash() {
        let (_temp, state, _auth, _doc, _ref) = fixture();
        let mut inner = state.inner.lock().unwrap();
        assert!(inner.rights("alice", "Target").is_err());
        let capture = selected(&mut inner, "alice", &["Target".into()]).unwrap();
        let delivered = serde_json::to_string(&capture.works).unwrap();
        assert!(delivered.contains("Granted Publication"));
        assert!(!delivered.contains(PRIVATE));
        assert!(!delivered.contains(&capture.evidence.works[0].node.source_hash));
        assert!(
            capture.works[0]
                .work
                .source_basis
                .starts_with("publication:")
        );
        for format in [
            BibliographyFormat::Bibtex,
            BibliographyFormat::CslJson,
            BibliographyFormat::Ris,
        ] {
            let bytes = project_bibliography(&capture.works, format).unwrap().bytes;
            let text = String::from_utf8(bytes).unwrap();
            assert!(!text.contains(PRIVATE));
            assert!(!text.contains(&capture.evidence.works[0].node.source_hash));
        }
    }
    #[test]
    fn field_and_use_are_both_required_and_hidden_selectors_refuse_before_capture() {
        let (_temp, state, _auth, _doc, _ref) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let hidden = selected(&mut inner, "alice", &["Shared".into()])
            .err()
            .unwrap();
        let absent = selected(&mut inner, "alice", &["NeverConfigured".into()])
            .err()
            .unwrap();
        assert_eq!(hidden.status, absent.status);
        assert_eq!(hidden.code, absent.code);
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Target")
            .unwrap()
            .fields
            .clear();
        assert!(selected(&mut inner, "alice", &["Target".into()]).is_err());
        let r = inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Target")
            .unwrap();
        r.fields.insert(
            PUBLICATION.into(),
            FieldRights {
                read: true,
                write: false,
            },
        );
        r.citations.r#use = false;
        assert!(selected(&mut inner, "alice", &["Target".into()]).is_err());
    }
    #[test]
    fn generated_input_contains_only_bib_and_original_missing_slot_is_a_feature_dependency() {
        let (_temp, state, _auth, doc, reference) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let source = format!(
            "{}\n#metadata((kind: \"weftext.citations\", entries: ((key: \"exact\", work: \"{reference}\"),))) <weftext-citations>\n@exact #bibliography(\"weftext.citations.bib\")",
            doc["source"].as_str().unwrap()
        );
        let capture = freeze(&mut inner, "alice", "Shared", &source)
            .unwrap()
            .unwrap();
        let mut worker = WorkerRequest {
            source,
            path: "Shared/Shared.typ".into(),
            root: inner.config.root.clone(),
            paged: false,
            style: None,
            frozen_files: BTreeMap::new(),
        };
        capture.inject(&mut worker).unwrap();
        let files = worker.frozen_files;
        assert_eq!(files.len(), 1);
        assert!(files.contains_key("Shared/weftext.citations.bib"));
        assert!(
            !String::from_utf8(
                base64::engine::general_purpose::STANDARD
                    .decode(files.values().next().unwrap())
                    .unwrap()
            )
            .unwrap()
            .contains(PRIVATE)
        );
        let pool = capture.plan_witnesses().unwrap();
        assert!(pool.len() >= 4);
        std::fs::write(
            inner.config.root.join("Shared/weftext.citations.bib"),
            b"PRIVATE_REAL_BIB",
        )
        .unwrap();
        assert!(capture.validate(&inner, "alice").is_err());
    }
    #[tokio::test]
    async fn exact_proposal_is_draft_only_and_preserves_identity_then_rejects_serial_aba() {
        let (_temp, state, auth, doc, reference) = fixture();
        let before = std::fs::read(
            state
                .inner
                .lock()
                .unwrap()
                .config
                .root
                .join("Shared/Shared.typ"),
        )
        .unwrap();
        let req = Propose {
            session_id: doc["sessionId"].as_str().unwrap().into(),
            source: doc["source"].as_str().unwrap().into(),
            serial: 1,
            source_hash: weftext_engine::source_hash(doc["source"].as_str().unwrap()),
            key: "actualKey".into(),
            work: "Target".into(),
        };
        let result = propose(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(req.clone()),
        )
        .await
        .unwrap()
        .0;
        let after = result["source"].as_str().unwrap();
        let role = weftext_engine::inspect_citation_declaration(after)
            .unwrap()
            .unwrap();
        assert_eq!(role.entries[0].work, reference);
        assert_eq!(
            weftext_engine::inspect(after).id,
            weftext_engine::inspect(&req.source).id
        );
        assert_eq!(
            std::fs::read(
                state
                    .inner
                    .lock()
                    .unwrap()
                    .config
                    .root
                    .join("Shared/Shared.typ")
            )
            .unwrap(),
            before
        );
        // Header-only documents have one shared EOF insertion. Apply the
        // delivered byte patches independently and verify the exact proposal.
        let header = weftext_engine::inspect(&req.source).metadata.unwrap().to;
        let minimal = req.source[..header].to_owned();
        {
            let mut inner = state.inner.lock().unwrap();
            writing_api::observe_version(&mut inner.editor_versions, &req.session_id, &minimal, 2)
                .unwrap();
        }
        let minimal_req = Propose {
            source: minimal.clone(),
            serial: 2,
            source_hash: weftext_engine::source_hash(&minimal),
            ..req.clone()
        };
        let minimal_result = propose(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(minimal_req),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(minimal_result["patches"].as_array().unwrap().len(), 1);
        let patch = &minimal_result["patches"][0];
        let mut applied = minimal;
        applied.replace_range(
            patch["byteFrom"].as_u64().unwrap() as usize
                ..patch["byteTo"].as_u64().unwrap() as usize,
            patch["text"].as_str().unwrap(),
        );
        assert_eq!(applied, minimal_result["source"].as_str().unwrap());
        assert!(weftext_engine::inspect(&applied).id.is_some());
        // Updating the original direct role includes its Hash token once.
        {
            let mut inner = state.inner.lock().unwrap();
            writing_api::observe_version(&mut inner.editor_versions, &req.session_id, after, 3)
                .unwrap();
        }
        let second = Propose {
            source: after.into(),
            serial: 3,
            source_hash: weftext_engine::source_hash(after),
            key: "secondKey".into(),
            ..req.clone()
        };
        let second_result = propose(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(second),
        )
        .await
        .unwrap()
        .0;
        let second_source = second_result["source"].as_str().unwrap();
        assert_eq!(
            weftext_engine::inspect_citation_declaration(second_source)
                .unwrap()
                .unwrap()
                .entries
                .len(),
            2
        );
        assert_eq!(
            weftext_engine::inspect(second_source).id,
            weftext_engine::inspect(&req.source).id
        );
        {
            let mut inner = state.inner.lock().unwrap();
            writing_api::observe_version(
                &mut inner.editor_versions,
                &req.session_id,
                &(req.source.clone() + "Changed"),
                4,
            )
            .unwrap();
            writing_api::observe_version(
                &mut inner.editor_versions,
                &req.session_id,
                &req.source,
                5,
            )
            .unwrap();
        }
        assert!(
            propose(State(state), Extension(auth), crate::RequestJson(req))
                .await
                .is_err()
        );
    }
    #[test]
    fn frozen_publication_bytes_survive_body_change_but_new_delivery_still_requires_original_grants()
     {
        let (_temp, state, _auth, _doc, _reference) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let capture = selected(&mut inner, "alice", &["Target".into()]).unwrap();
        let bytes = capture.bibliography.bytes.clone();
        let root = inner.config.root.clone();
        let opened = inner
            .core
            .open(root.join("Target/Target.typ"), Some(root))
            .unwrap();
        inner
            .core
            .save(weftext_core::SaveRequest {
                session_id: opened.session_id.clone(),
                base_id: opened.base_id,
                source: opened.source + "\nUnrelated private body edit",
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        inner.core.close_session(&opened.session_id).unwrap();
        assert!(capture.validate(&inner, "alice").is_err());
        let refreshed = selected(&mut inner, "alice", &["Target".into()]).unwrap();
        assert_eq!(
            capture.works[0].work.source_basis,
            refreshed.works[0].work.source_basis
        );
        authorize(&inner, "alice", &capture.authority, true, true).unwrap();
        assert_eq!(capture.bibliography.bytes, bytes);
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Target")
            .unwrap()
            .citations
            .download = false;
        assert!(authorize(&inner, "alice", &capture.authority, true, true).is_err());
        authorize(&inner, "alice", &capture.authority, true, false).unwrap();
    }
    #[test]
    fn original_feature_plan_keeps_missing_bib_and_schema_after_restart() {
        for name in [".weftext-schema.json", "Shared/weftext.citations.bib"] {
            let (temp, state, _auth, doc, reference) = fixture();
            let mut inner = state.inner.lock().unwrap();
            let source = format!(
                "{}\n#metadata((kind: \"weftext.citations\", entries: ((key: \"exact\", work: \"{reference}\"),))) <weftext-citations>\n@exact #bibliography(\"weftext.citations.bib\")",
                doc["source"].as_str().unwrap()
            );
            let capture = freeze(&mut inner, "alice", "Shared", &source)
                .unwrap()
                .unwrap();
            let pool = capture.plan_witnesses().unwrap();
            let root = inner.config.root.clone();
            let path = root.join("Shared/Shared.typ");
            let before = std::fs::read(&path).unwrap();
            let base = inner
                .core
                .read_snapshot(&root, &[path.to_string_lossy().into_owned()])
                .unwrap()
                .files
                .into_iter()
                .next()
                .unwrap()
                .base_id;
            let after = [before.as_slice(), b"\nOriginal proposed change"].concat();
            let op = uuid::Uuid::new_v4().to_string();
            let scope = inner.actor_scope("alice") + ":citation-tests";
            let prepared = inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_feature_prepare_with_witnesses(
                        "Shared",
                        &op,
                        &digest(b"original-citation-intent"),
                        weftext_core::OperationRequest {
                            root: root.to_string_lossy().into_owned(),
                            operation_id: op.clone(),
                            intent: weftext_core::OperationIntent::UpdateFiles {
                                changes: vec![weftext_core::FileChange {
                                    path: path.to_string_lossy().into_owned(),
                                    base_id: Some(base),
                                    before: Some(before.clone()),
                                    after,
                                }],
                            },
                        },
                        json!({"summary":"original"}),
                        &[],
                        &pool.iter().collect::<Vec<_>>(),
                    )
                })
                .unwrap();
            let plan = prepared.plan_id.unwrap();
            drop(inner);
            drop(state);
            std::fs::write(root.join(name), b"new entity occupies original absence").unwrap();
            let mut core = Core::new(temp.path().join("state")).unwrap();
            assert!(
                core.with_actor(&scope, |core| core
                    .actor_feature_commit("Shared", &op, &plan))
                    .is_err()
            );
            assert_eq!(std::fs::read(path).unwrap(), before);
        }
    }
}

#[cfg(test)]
mod worker_diagnostic_contract_tests {
    use super::*;
    use crate::worker_failure::{
        Kind,
        diagnostics_tests::{assert_public_response, assert_safe_wrapper, original_failure},
    };

    #[tokio::test]
    async fn normal_reference_early_bad_keeps_original_worker_and_its_exact_public_contract() {
        for kind in [
            Kind::Deadline,
            Kind::NonzeroExit,
            Kind::OutputRejected,
            Kind::SupervisionLost,
        ] {
            let original = original_failure(kind);
            let retained = original.private_worker_failure.as_ref().unwrap().clone();
            assert_safe_wrapper(
                reference_number_unavailable(Some(&original)),
                &retained,
                StatusCode::BAD_REQUEST,
                "invalid_request",
                UiMessage::error_api_hosted_citations_reference_number_unavailable(),
            )
            .await;
        }
    }
    #[tokio::test]
    async fn both_reference_insertions_share_the_safe_changed_wrapper_without_losing_facts() {
        for kind in [
            Kind::Deadline,
            Kind::NonzeroExit,
            Kind::OutputRejected,
            Kind::SupervisionLost,
        ] {
            let original = original_failure(kind);
            let retained = original.private_worker_failure.as_ref().unwrap().clone();
            assert_safe_wrapper(
                changed_after_worker(original),
                &retained,
                StatusCode::CONFLICT,
                "citationBasisChanged",
                UiMessage::error_api_hosted_citations_citation_basis_changed(),
            )
            .await;
        }
    }
    #[tokio::test]
    async fn missing_number_without_a_private_failure_never_invents_worker_evidence() {
        let ordinary = ApiError::bad(UiMessage::error_compiler_output_invalid());
        for error in [
            reference_number_unavailable(None),
            reference_number_unavailable(Some(&ordinary)),
        ] {
            assert!(error.private_worker_failure.is_none());
            assert!(std::error::Error::source(&error).is_none());
            assert_public_response(
                error,
                StatusCode::BAD_REQUEST,
                "invalid_request",
                UiMessage::error_api_hosted_citations_reference_number_unavailable(),
            )
            .await;
        }
    }
}
