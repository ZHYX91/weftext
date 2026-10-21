//! Citation generation consumes only a Core-admitted immutable compile cut.
//! It does not look up matching DOI/title/suggested keys or reread live sources.
use serde::Serialize;
use std::path::{Component, Path};
use weftext_core::{AdmittedCompileSnapshot, CoreError};
use weftext_export::bibliography::{
    BibliographyFormat, BibliographyProjection, CitationWork, FrozenWork, project_bibliography,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrozenCitations {
    pub cut_id: String,
    pub owner_source_hash: String,
    pub schema_generation: String,
    pub virtual_path: String,
    pub declaration: weftext_engine::CitationDeclaration,
    pub works: Vec<CitationWork>,
    pub bibliography: BibliographyProjection,
    pub authored_cites: Vec<weftext_engine::NativeCitationOccurrence>,
}

/// Resolve every authored stable Work independently from the same cut, qualify
/// its installed Work type and publication FieldProjection, then add one frozen
/// read-only native input slot. Failure leaves the captured input map unchanged.
pub(crate) fn freeze_citations(
    captured: &mut AdmittedCompileSnapshot,
    owner_source: &str,
    owner_path: &Path,
) -> Result<Option<FrozenCitations>, CoreError> {
    let Some(declaration) = weftext_engine::inspect_citation_declaration(owner_source)
        .map_err(|e| CoreError::new("citationDeclarationUnavailable", e.to_string()))?
    else {
        return Ok(None);
    };
    let inventory = &captured.snapshot.inventory;
    let root = Path::new(&inventory.root);
    if owner_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(CoreError::new(
            "citationOwnerUnavailable",
            "Citation owner path contains parent traversal",
        ));
    }
    let owner_path = if owner_path.is_absolute() {
        owner_path.to_owned()
    } else {
        root.join(owner_path)
    };
    let relative = owner_path.strip_prefix(root).map_err(|_| {
        CoreError::new(
            "citationOwnerUnavailable",
            "Citation owner is outside the frozen library",
        )
    })?;
    let virtual_path = relative
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("weftext.citations.bib")
        .to_string_lossy()
        .replace('\\', "/");
    let collision = |value: &str| value.replace('\\', "/").eq_ignore_ascii_case(&virtual_path);
    if captured.files.keys().any(|key| collision(key))
        || inventory
            .entries
            .iter()
            .any(|entry| collision(&entry.relative_path))
    {
        return Err(CoreError::new(
            "citationResourceCollision",
            "An authored file or directory occupies weftext.citations.bib; generated data cannot replace it",
        ));
    }
    let workspace = inventory.workspace_id.as_deref().ok_or_else(|| {
        CoreError::new(
            "citationWorkUnavailable",
            "A managed library identity is required to resolve citation Works",
        )
    })?;
    let owner = captured
        .snapshot
        .files
        .iter()
        .find(|f| Path::new(&f.path) == owner_path)
        .ok_or_else(|| {
            CoreError::new(
                "citationOwnerUnavailable",
                "Citation owner is not in the admitted source cut",
            )
        })?;
    if owner
        .node_id
        .as_deref()
        .is_none_or(|id| weftext_engine::inspect(owner_source).id.as_deref() != Some(id))
    {
        return Err(CoreError::new(
            "citationOwnerUnavailable",
            "Draft source no longer has its admitted owner identity",
        ));
    }
    let schema_file = captured.snapshot.schema.as_ref().ok_or_else(|| {
        CoreError::new(
            "citationSchemaUnavailable",
            "Frozen schema role is unavailable",
        )
    })?;
    let (schema, schema_generation) = super::data_api::knowledge_schema_from_snapshot(schema_file)
        .map_err(|e| CoreError::new("citationSchemaUnavailable", e.message))?;
    let mut works = vec![];
    for entry in &declaration.entries {
        let prefix = format!("weftext-node://{workspace}/");
        let node = entry.work.strip_prefix(&prefix).ok_or_else(|| {
            CoreError::new(
                "citationWorkUnavailable",
                format!(
                    "{}: Work belongs to a library outside this authorized cut",
                    entry.key
                ),
            )
        })?;
        let targets = inventory
            .entries
            .iter()
            .filter(|target| target.node_id.as_deref() == Some(node))
            .collect::<Vec<_>>();
        let [target] = targets.as_slice() else {
            return Err(CoreError::new(
                "citationWorkUnavailable",
                format!(
                    "{}: missing, inaccessible or ambiguous stable Work",
                    entry.key
                ),
            ));
        };
        if target.kind != "node" || target.identity_status != "admitted" {
            return Err(CoreError::new(
                "citationWorkUnavailable",
                format!("{}: target is not an admitted live Work", entry.key),
            ));
        }
        let files = captured
            .snapshot
            .files
            .iter()
            .filter(|file| file.path == target.path && file.node_id.as_deref() == Some(node))
            .collect::<Vec<_>>();
        let [file] = files.as_slice() else {
            return Err(CoreError::new(
                "citationWorkUnavailable",
                format!("{}: exact Work bytes are not in this cut", entry.key),
            ));
        };
        let source = if Path::new(&file.path) == owner_path {
            owner_source
        } else {
            let bytes = file.bytes.as_deref().ok_or_else(|| {
                CoreError::new(
                    "citationWorkUnavailable",
                    format!("{}: Work bytes were unavailable", entry.key),
                )
            })?;
            std::str::from_utf8(bytes).map_err(|_| {
                CoreError::new(
                    "citationWorkUnavailable",
                    format!("{}: Work source is not UTF-8", entry.key),
                )
            })?
        };
        let projection = weftext_data::project(source, &schema);
        if projection.qualification != "qualified"
            || projection.id.as_deref() != Some(node)
            || !projection
                .resolved_types
                .iter()
                .any(|t| t == "library/work")
        {
            return Err(CoreError::new(
                "citationWorkType",
                format!(
                    "{}: target does not qualify as installed library/work in this cut",
                    entry.key
                ),
            ));
        }
        let publication = projection
            .fields
            .iter()
            .find(|field| field.field_id == "library/publication")
            .and_then(|field| field.cell.native())
            .cloned()
            .ok_or_else(|| {
                CoreError::new(
                    "citationPublicationUnavailable",
                    format!(
                        "{}: typed publication field is absent, invalid or unavailable",
                        entry.key
                    ),
                )
            })?;
        works.push(CitationWork {
            key: entry.key.clone(),
            work: FrozenWork {
                entity_ref: entry.work.clone(),
                source_basis: format!(
                    "cut:{}:base:{}:hash:{}",
                    captured.snapshot.cut_id,
                    file.base_id,
                    weftext_engine::source_hash(source)
                ),
                publication,
            },
        });
    }
    let bibliography = project_bibliography(&works, BibliographyFormat::Bibtex)
        .map_err(|e| CoreError::new("citationBibliographyUnavailable", e.to_string()))?;
    let frozen = FrozenCitations {
        cut_id: captured.snapshot.cut_id.clone(),
        owner_source_hash: weftext_engine::source_hash(owner_source),
        schema_generation,
        virtual_path: virtual_path.clone(),
        declaration,
        works,
        bibliography,
        authored_cites: weftext_engine::proven_static_cites(owner_source),
    };
    captured
        .files
        .insert(virtual_path, frozen.bibliography.bytes.clone());
    captured.files.insert(
        relative.to_string_lossy().replace('\\', "/"),
        owner_source.as_bytes().to_vec(),
    );
    if let Some(owner) = captured
        .snapshot
        .files
        .iter_mut()
        .find(|file| Path::new(&file.path) == owner_path)
    {
        owner.bytes = Some(owner_source.as_bytes().to_vec());
    }
    Ok(Some(frozen))
}

/// Producer-only facts emitted after the pinned official CiteGroup realizes.
/// Author HTML attributes never create these protected semantic attributes.
pub(crate) fn displayed_citations(tree: &weftext_engine::RenderNode) -> Vec<serde_json::Value> {
    fn visit(node: &weftext_engine::RenderNode, out: &mut Vec<serde_json::Value>) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "citation")
        {
            let keys = node
                .attrs
                .get("data-weftext-citation-keys")
                .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok());
            let forms = node
                .attrs
                .get("data-weftext-citation-forms")
                .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok());
            let origins = node
                .attrs
                .get("data-weftext-citation-origins")
                .and_then(|raw| {
                    serde_json::from_str::<Vec<Option<weftext_engine::Origin>>>(raw).ok()
                });
            if let (Some(keys), Some(forms), Some(origins)) = (keys, forms, origins)
                && keys.len() == forms.len()
                && keys.len() == origins.len()
            {
                for ((key, form), origin) in keys.into_iter().zip(forms).zip(origins) {
                    let ordinal = out.len();
                    let range=origin.as_ref().filter(|origin|origin.is_main).map(|origin|serde_json::json!({"from":origin.from,"to":origin.to,"utf16From":origin.utf16_from,"utf16To":origin.utf16_to,"raw":origin.raw,"writable":false}));
                    out.push(serde_json::json!({"key":key,"form":form,"ordinal":ordinal,"range":range,"originKind":if origin.as_ref().is_some_and(|origin|origin.is_main){"mainSource"}else{"generatedOrExternal"},"visibility":if form=="none"{"bibliographyOnly"}else{"displayedNativeCitationGroup"}}));
                }
            }
        }
        for child in &node.children {
            visit(child, out);
        }
    }
    let mut out = Vec::new();
    visit(tree, &mut out);
    out
}

/// A matching key is not enough to associate an external or multiple Bib.
pub(crate) fn owns_single_virtual_bibliography(source: &str) -> bool {
    use typst_syntax::{
        LinkedNode, Source,
        ast::{self, AstNode},
    };
    if !weftext_engine::standard_binding_available(source, "bibliography")
        || !weftext_engine::native_citation_owner_complete(source)
    {
        return false;
    }
    fn visit(node: &LinkedNode<'_>, found: &mut Vec<bool>) {
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|id| id.get() == "bibliography")
        {
            found.push(matches!(call.args().items().next(),Some(ast::Arg::Pos(expr)) if expr.to_untyped().cast::<ast::Str>().is_some_and(|path|path.get()=="weftext.citations.bib")));
        }
        for child in node.children() {
            visit(&child, found);
        }
    }
    let parsed = Source::detached(source);
    let mut found = vec![];
    visit(&LinkedNode::new(parsed.root()), &mut found);
    found == [true]
}

/// Source candidates become selectable only when the same successful native
/// product emitted one real object at that exact main-source span.
pub(crate) fn unlabelled_reference_targets(
    source: &str,
    tree: &weftext_engine::RenderNode,
) -> Vec<serde_json::Value> {
    use typst_syntax::{
        LinkedNode, Source, SyntaxKind,
        ast::{self, AstNode},
    };
    fn count(node: &weftext_engine::RenderNode, from: usize, kind: &str) -> usize {
        usize::from(
            node.origin
                .as_ref()
                .is_some_and(|origin| origin.is_main && origin.from == from)
                && node
                    .attrs
                    .get("data-weftext-role")
                    .is_some_and(|role| role == kind),
        ) + node
            .children
            .iter()
            .map(|child| count(child, from, kind))
            .sum::<usize>()
    }
    let parsed = Source::detached(source);
    let root = LinkedNode::new(parsed.root());
    let labelled = weftext_engine::inspect_native_citation_editor(source).targets;
    let mut out = Vec::new();
    for node in root.children() {
        let kind = match node.kind() {
            SyntaxKind::Heading => "heading",
            SyntaxKind::Equation => "equation",
            _ => {
                let Some(call) = node.get().cast::<ast::FuncCall>() else {
                    continue;
                };
                let Some(name) = call.callee().to_untyped().cast::<ast::Ident>() else {
                    continue;
                };
                if !matches!(name.get().as_str(), "figure" | "table")
                    || !weftext_engine::standard_binding_available(source, name.get())
                {
                    continue;
                }
                if name.get() == "figure" {
                    "figure"
                } else {
                    "table"
                }
            }
        };
        let range = node.range();
        if labelled
            .iter()
            .any(|target| target.range.from == range.start)
            || count(tree, range.start, kind) != 1
        {
            continue;
        }
        out.push(serde_json::json!({"kind":kind,"range":{"from":range.start,"to":range.end,"utf16From":source[..range.start].encode_utf16().count(),"utf16To":source[..range.end].encode_utf16().count(),"raw":source[range.clone()],"writable":true},"title":source[range].chars().take(80).collect::<String>()}));
    }
    out
}

pub(crate) fn reference_numbered_at(
    tree: &weftext_engine::RenderNode,
    from: usize,
) -> Option<bool> {
    if tree
        .origin
        .as_ref()
        .is_some_and(|origin| origin.is_main && origin.from == from)
        && tree
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| matches!(role.as_str(), "heading" | "figure" | "equation"))
    {
        if let Some(numbered) = tree.attrs.get("data-weftext-numbered") {
            return Some(numbered == "true");
        }
        if let Some(kind) = tree.attrs.get("data-weftext-numbering-kind") {
            return Some(kind != "none");
        }
    }
    tree.children
        .iter()
        .find_map(|child| reference_numbered_at(child, from))
}

pub(crate) fn classify_reference_controls(
    editor: &mut serde_json::Value,
    tree: Option<&weftext_engine::RenderNode>,
) {
    fn document_reference(node: &weftext_engine::RenderNode, from: usize, label: &str) -> bool {
        node.origin
            .as_ref()
            .is_some_and(|origin| origin.is_main && origin.from == from)
            && node
                .attrs
                .get("data-weftext-role")
                .is_some_and(|role| role == "reference")
            && node
                .attrs
                .get("data-weftext-label")
                .is_some_and(|value| value == label)
            || node
                .children
                .iter()
                .any(|child| document_reference(child, from, label))
    }
    let citations = tree.map(displayed_citations).unwrap_or_default();
    if let Some(references) = editor["crossReferences"].as_array_mut() {
        for reference in references {
            let from = reference["range"]["from"].as_u64().unwrap_or(u64::MAX) as usize;
            let label = reference["label"].as_str().unwrap_or("");
            let kind = if tree.is_some_and(|tree| document_reference(tree, from, label)) {
                "document"
            } else if citations.iter().any(|citation| {
                citation["key"] == label && citation["range"]["from"].as_u64() == Some(from as u64)
            }) {
                "citation"
            } else if reference["kind"] == "link" {
                "documentSourceLink"
            } else {
                "unconfirmed"
            };
            reference["renderedKind"] = serde_json::json!(kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftext_core::{AdmittedSnapshot, Inventory, InventoryEntry, SnapshotFile};
    const LIB: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
    const OWNER: &str = "12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    const WORK: &str = "8aa10c08-3e7d-4d10-9488-92bc7f393050";
    #[test]
    fn compiled_citation_products_keep_actual_members_and_hide_physical_origins() {
        let mut node = weftext_engine::RenderNode {
            key: "producer".into(),
            kind: "element".into(),
            tag: Some("span".into()),
            text: None,
            attrs: std::collections::BTreeMap::new(),
            children: vec![],
            origin: None,
            writable: false,
        };
        node.attrs.insert("role".into(), "doc-biblioref".into());
        assert!(displayed_citations(&node).is_empty());
        node.attrs
            .insert("data-weftext-role".into(), "citation".into());
        node.attrs.insert(
            "data-weftext-citation-keys".into(),
            "[\"one\",\"two\"]".into(),
        );
        node.attrs.insert(
            "data-weftext-citation-forms".into(),
            "[\"normal\",\"none\"]".into(),
        );
        node.attrs.insert(
            "data-weftext-citation-origins".into(),
            serde_json::to_string(&vec![
                Some(weftext_engine::Origin {
                    path: "PRIVATE_SERVER_PATH".into(),
                    from: 0,
                    to: 4,
                    utf16_from: 0,
                    utf16_to: 4,
                    raw: "@one".into(),
                    syntax_kind: "Ref".into(),
                    is_main: true,
                }),
                None,
            ])
            .unwrap(),
        );
        let products = displayed_citations(&node);
        assert_eq!(products.len(), 2);
        assert_eq!(products[1]["visibility"], "bibliographyOnly");
        assert!(
            !serde_json::to_string(&products)
                .unwrap()
                .contains("PRIVATE_SERVER_PATH")
        );
    }
    #[test]
    fn unlabelled_target_requires_one_real_object_and_label_plus_ref_compile_together() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("citation-target.typ");
        let source = "#set heading(numbering: \"1.\")\n= 未标记目标\n\n正文\n";
        let engine = weftext_engine::Engine::new().unwrap();
        let output = engine.render(source, &path, dir.path()).unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        let candidates = unlabelled_reference_targets(source, output.tree.as_ref().unwrap());
        assert_eq!(candidates.len(), 1);
        let target =
            serde_json::from_value::<weftext_engine::SourceRange>(candidates[0]["range"].clone())
                .unwrap();
        let edited = weftext_engine::insert_native_cross_reference_with_label(
            source,
            &weftext_engine::source_hash(source),
            source.find("正文").unwrap(),
            &target,
            "新目标",
            false,
        )
        .unwrap();
        assert_eq!(edited.patches.len(), 2);
        let rendered = engine.render(&edited.source, &path, dir.path()).unwrap();
        assert!(rendered.ok, "{:?}", rendered.diagnostics);
        assert!(
            rendered
                .labels
                .iter()
                .any(|label| label.label == "新目标" && label.count == 1)
        );
        assert!(
            unlabelled_reference_targets(&edited.source, rendered.tree.as_ref().unwrap())
                .is_empty()
        );
    }
    #[test]
    fn official_citation_groups_keep_bibliography_only_members_separate_from_visible_members() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("works.bib"),
            "@article{entry, title={Native Citation}, author={Author, First}, year={2026}}\n",
        )
        .unwrap();
        let engine = weftext_engine::Engine::new().unwrap();
        let source = "#cite(<entry>, form: none)\n\n#cite(<entry>, form: \"prose\")\n\n#bibliography(\"works.bib\")";
        let output = engine
            .render(source, &dir.path().join("citation.typ"), dir.path())
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        let products = displayed_citations(output.tree.as_ref().unwrap());
        assert!(
            products
                .iter()
                .any(|item| item["visibility"] == "bibliographyOnly")
        );
        assert!(products.iter().any(|item| item["form"] == "prose"));
    }
    fn fixture() -> (AdmittedCompileSnapshot, String, std::path::PathBuf) {
        let root = std::env::temp_dir().join("weftext-frozen-citation-fixture");
        let owner_path = root.join("owner.typ");
        let work_path = root.join("work.typ");
        let owner_source = format!(
            "#metadata((id: \"{OWNER}\")) <weftext>\n#metadata((kind: \"weftext.citations\", entries: ((key: \"actualKey\", work: \"weftext-node://{LIB}/{WORK}\"),))) <weftext-citations>\n@actualKey #bibliography(\"weftext.citations.bib\")"
        );
        let work_source = format!(
            "#metadata((id: \"{WORK}\", types: (\"paper\",), publication: (kind: \"article\", title: \"Actual Work\", citation_key: \"otherSuggestion\", contributors: ((agent: (text: \"A\"), roles: (\"author\",)),)))) <weftext>"
        );
        let files = [
            (owner_path.clone(), OWNER, owner_source.clone()),
            (work_path.clone(), WORK, work_source),
        ]
        .into_iter()
        .map(|(path, id, source)| SnapshotFile {
            path: path.to_string_lossy().into(),
            bytes: Some(source.into_bytes()),
            base_id: format!("base-{id}"),
            node_id: Some(id.into()),
            kind: "node".into(),
            currentness: "frozen".into(),
            revision: None,
        })
        .collect::<Vec<_>>();
        let entries = files
            .iter()
            .map(|f| InventoryEntry {
                path: f.path.clone(),
                relative_path: Path::new(&f.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into(),
                name: String::new(),
                parent_path: root.to_string_lossy().into(),
                kind: "node".into(),
                node_id: f.node_id.clone(),
                identity_status: "admitted".into(),
                title: None,
                excerpt: None,
                source: None,
            })
            .collect();
        let map = files
            .iter()
            .map(|f| {
                (
                    Path::new(&f.path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into(),
                    f.bytes.clone().unwrap(),
                )
            })
            .collect();
        (
            AdmittedCompileSnapshot {
                snapshot: AdmittedSnapshot {
                    cut_id: "cut-fixture".into(),
                    membership_ids: vec![],
                    inventory: Inventory {
                        root: root.to_string_lossy().into(),
                        workspace_id: Some(LIB.into()),
                        status: "localMvp".into(),
                        entries,
                        warnings: vec![],
                        complete: true,
                    },
                    files,
                    schema: Some(SnapshotFile {
                        path: root.join("weftext.schema.json").to_string_lossy().into(),
                        bytes: None,
                        base_id: "schema-base".into(),
                        node_id: None,
                        kind: "schema".into(),
                        currentness: "frozen".into(),
                        revision: None,
                    }),
                    tags: None,
                    complete: true,
                    warnings: vec![],
                    raw_complete: true,
                    raw_pending: 0,
                },
                files: map,
                style: None,
            },
            owner_source,
            owner_path,
        )
    }
    #[test]
    fn declarations_resolve_typed_work_and_never_suggestion_identity() {
        let (mut snapshot, mut source, path) = fixture();
        source.push_str("\nDraft text differing from the frozen disk owner");
        let output = freeze_citations(&mut snapshot, &source, &path)
            .unwrap()
            .unwrap();
        assert_eq!(output.works[0].key, "actualKey");
        assert!(
            String::from_utf8(snapshot.files["weftext.citations.bib"].clone())
                .unwrap()
                .starts_with("@article{actualKey,")
        );
        assert!(output.authored_cites.is_empty());
        assert_eq!(snapshot.files["owner.typ"], source.as_bytes());
        assert_eq!(
            snapshot.snapshot.files[0].bytes.as_deref(),
            Some(source.as_bytes())
        );
        assert_eq!(
            output.owner_source_hash,
            weftext_engine::source_hash(&source)
        );
    }
    #[test]
    fn self_work_reads_same_draft_publication_as_main_compile() {
        let (mut snapshot, _, path) = fixture();
        let source = |title: &str| {
            format!(
                "#metadata((id: \"{OWNER}\", types: (\"paper\",), publication: (kind: \"article\", title: \"{title}\", contributors: ((agent: (text: \"Author\"), roles: (\"author\",)),)))) <weftext>\n#metadata((kind: \"weftext.citations\", entries: ((key: \"selfKey\", work: \"weftext-node://{LIB}/{OWNER}\"),))) <weftext-citations>\n@selfKey #bibliography(\"weftext.citations.bib\")"
            )
        };
        snapshot.snapshot.files[0].bytes = Some(source("Disk Title").into_bytes());
        snapshot
            .files
            .insert("owner.typ".into(), source("Disk Title").into_bytes());
        let draft = source("Draft Title");
        let frozen = freeze_citations(&mut snapshot, &draft, &path)
            .unwrap()
            .unwrap();
        assert_eq!(
            frozen.works[0]
                .work
                .publication
                .member("title")
                .unwrap()
                .text(),
            Some("Draft Title")
        );
        assert!(
            String::from_utf8(frozen.bibliography.bytes)
                .unwrap()
                .contains("title = {Draft Title}")
        );
        assert!(
            frozen.works[0]
                .work
                .source_basis
                .contains(&weftext_engine::source_hash(&draft))
        );
        assert_eq!(snapshot.snapshot.files[0].base_id, format!("base-{OWNER}"));
        assert!(snapshot.snapshot.files[0].revision.is_none());
    }
    #[test]
    fn resource_collision_missing_work_and_wrong_type_keep_inputs_unchanged() {
        let (mut snapshot, source, path) = fixture();
        snapshot
            .files
            .insert("weftext.citations.bib".into(), b"author bytes".to_vec());
        assert_eq!(
            freeze_citations(&mut snapshot, &source, &path)
                .unwrap_err()
                .code,
            "citationResourceCollision"
        );
        assert_eq!(snapshot.files["weftext.citations.bib"], b"author bytes");
        let (mut snapshot, source, path) = fixture();
        snapshot.snapshot.files.pop();
        let before = snapshot.files.clone();
        assert_eq!(
            freeze_citations(&mut snapshot, &source, &path)
                .unwrap_err()
                .code,
            "citationWorkUnavailable"
        );
        assert_eq!(snapshot.files, before);
        let (mut snapshot, source, path) = fixture();
        let work = snapshot.snapshot.files.last_mut().unwrap();
        work.bytes = Some(
            format!("#metadata((id: \"{WORK}\", types: (\"person\",))) <weftext>").into_bytes(),
        );
        assert_eq!(
            freeze_citations(&mut snapshot, &source, &path)
                .unwrap_err()
                .code,
            "citationWorkType"
        );
    }
}
