use std::fmt::Write;
use std::fs;

use tempfile::tempdir;
use uuid::Uuid;
use weftext_core::{
    AdjacentHeadingBody, Anchor, Annotation, AnnotationBody, AnnotationBodyFormat, AnnotationKind,
    AnnotationState, AnnotationStore, DocumentBlockKind, DocumentDiagnosticCode, DocumentEdit,
    DocumentEnvelopeState, DocumentError, DocumentLinkKind, DocumentProfileId, DocumentRevision,
    ThreadMessage, WorkspaceDocumentGeneration, active_document_profile, analyze_document,
    build_workspace_link_index, canonical_document_file_name, canonical_document_locator,
    canonical_document_path, commit_document_edit, commit_workspace_transaction, create_child_node,
    create_workspace, parse_document, patch_document_header_property, patch_node_icon_property,
    plan_create_child_node, plan_document_edit, probe_document_envelope, read_node_document,
    scan_workspace, workspace_document_format,
};

fn source(id: weftext_core::NodeId, body: &str, newline: &str) -> String {
    format!("---{newline}weftext:{newline}  id: \"{id}\"{newline}---{newline}{body}")
}

#[test]
fn canonical_workspace_requires_the_exact_asciidoc_marker_and_layout() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("知识库");
    let created = create_workspace(&root).unwrap();
    let format = workspace_document_format(&root);
    assert_eq!(format.generation, WorkspaceDocumentGeneration::AsciiDocV1);
    assert_eq!(format.canonical_extension, "adoc");
    assert_eq!(
        fs::read(root.join(".weftext-format")).unwrap(),
        b"weftext.asciidoc.v1\n"
    );
    assert_eq!(canonical_document_file_name("知识库"), "知识库.adoc");
    assert_eq!(
        canonical_document_path(&root, "知识库"),
        created.document_path
    );
    assert_eq!(canonical_document_locator("父/子", "子"), "父/子/子.adoc");
    assert!(!root.join("知识库.md").exists());

    assert!(scan_workspace(&root).is_valid());
}

#[test]
fn exact_source_snapshot_preserves_line_endings_and_multiscript_text() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("Root");
    let created = create_workspace(&root).unwrap();
    let variants = [
        source(created.id, "= 标题\nemoji 🧠 שלום", "\n"),
        source(created.id, "= 标题\r\nemoji 🧠 שלום", "\r\n"),
        format!(
            "---\r\nweftext:\n  id: \"{}\"\r\n---\n= 标题\r\nemoji 🧠 שלום\n",
            created.id
        ),
    ];
    for exact in variants {
        fs::write(&created.document_path, exact.as_bytes()).unwrap();
        let snapshot = read_node_document(&root).unwrap();
        assert_eq!(snapshot.profile, DocumentProfileId::AsciiDocV1);
        assert_eq!(snapshot.source.as_bytes(), exact.as_bytes());
        assert_eq!(snapshot.revision, DocumentRevision::from_source(&exact));
    }
}

#[test]
fn envelope_probe_and_malformed_source_fail_closed_without_normalization() {
    let mixed = "\u{feff}---\r\nkey: 值\n---\r\n正文";
    let envelope = probe_document_envelope(mixed);
    assert_eq!(envelope.state, DocumentEnvelopeState::Absent);
    assert!(envelope.range.is_none());

    let crlf = "---\r\nkey: 值\n---\r\n正文";
    let envelope = probe_document_envelope(crlf);
    assert_eq!(envelope.state, DocumentEnvelopeState::Closed);
    let range = envelope.range.unwrap();
    assert_eq!(
        &crlf[usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()],
        "---\r\nkey: 值\n---\r\n"
    );

    let malformed = "---\r\nkey: value\n正文";
    let envelope = probe_document_envelope(malformed);
    assert_eq!(envelope.state, DocumentEnvelopeState::Unclosed);
    let analysis = analyze_document(malformed, AdjacentHeadingBody::Separate);
    assert_eq!(analysis.model.blocks.len(), 0);
    assert_eq!(
        analysis.model.diagnostics[0].code,
        DocumentDiagnosticCode::UnclosedFrontmatter
    );

    let doubled_bom =
        "\u{feff}\u{feff}---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n";
    assert_eq!(
        probe_document_envelope(doubled_bom).state,
        DocumentEnvelopeState::Absent
    );

    let temp = tempdir().unwrap();
    let root = temp.path().join("Root");
    let created = create_workspace(&root).unwrap();
    fs::write(&created.document_path, [0xff, 0xfe, 0xfd]).unwrap();
    assert!(matches!(
        read_node_document(&root),
        Err(DocumentError::InvalidUtf8(_))
    ));
}

#[test]
fn utf8_source_patch_is_narrow_for_cjk_emoji_rtl_and_mixed_endings() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("Root");
    let created = create_workspace(&root).unwrap();
    let exact = source(created.id, "# 标题\r\n旧🧠 שלום\n尾行\r\n", "\r\n");
    fs::write(&created.document_path, exact.as_bytes()).unwrap();
    let snapshot = read_node_document(&root).unwrap();
    let start = exact.find("旧🧠 שלום").unwrap();
    let end = start + "旧🧠 שלום".len();
    let plan = plan_document_edit(
        &root,
        &snapshot.revision,
        [DocumentEdit {
            start: u64::try_from(start).unwrap(),
            end: u64::try_from(end).unwrap(),
            replacement: "新✨ مرحبا".to_owned(),
        }],
    )
    .unwrap();
    let expected = exact.replacen("旧🧠 שלום", "新✨ مرحبا", 1);
    assert_eq!(plan.next_source().as_bytes(), expected.as_bytes());
    commit_document_edit(&plan).unwrap();
    assert_eq!(
        fs::read(&created.document_path).unwrap(),
        expected.as_bytes()
    );

    let current = read_node_document(&root).unwrap();
    let emoji = current.source.find('✨').unwrap();
    let invalid = plan_document_edit(
        &root,
        &current.revision,
        [DocumentEdit {
            start: u64::try_from(emoji + 1).unwrap(),
            end: u64::try_from(emoji + 1).unwrap(),
            replacement: "x".to_owned(),
        }],
    );
    assert!(matches!(
        invalid,
        Err(DocumentError::NonCharacterBoundary { .. })
    ));
}

#[test]
fn document_property_and_icon_patches_preserve_unrelated_exact_bytes() {
    let exact = "---\r\nweftext:\r\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n  icon: 'weftext:book'\r\n---\r\n= 标题\r\n:status: 旧\r\n\r\n正文 🧠\n";
    let title = patch_document_header_property(exact, "status", Some("新值")).unwrap();
    assert_eq!(title, exact.replacen(":status: 旧", ":status: 新值", 1));
    let icon = patch_node_icon_property(&title, Some("😀")).unwrap();
    assert_eq!(
        icon,
        title.replacen("icon: 'weftext:book'", "icon: \"😀\"", 1)
    );
    assert!(
        patch_node_icon_property(
            "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n  icon: 😀\n  icon: 😺\n---\n",
            Some("weftext:book")
        )
        .is_err()
    );
}

#[test]
fn one_runtime_adapter_exposes_generic_model_capabilities_and_view() {
    let source = "= 标题\n\n[#body]\n正文\n";
    let analysis = analyze_document(source, AdjacentHeadingBody::RunIn);
    assert_eq!(analysis.descriptor, active_document_profile());
    assert_eq!(analysis.descriptor.profile, DocumentProfileId::AsciiDocV1);
    assert!(analysis.descriptor.capabilities.exact_source);
    assert!(analysis.descriptor.capabilities.utf8_source_edits);
    assert_eq!(analysis.descriptor.capabilities.max_heading_level, 9);
    assert!(analysis.descriptor.capabilities.typed_blocks);
    assert!(analysis.descriptor.capabilities.typed_inlines);
    assert!(analysis.descriptor.capabilities.safe_render_input);
    assert!(analysis.descriptor.capabilities.degradation_reports);
    assert!(
        analysis
            .descriptor
            .capabilities
            .adjacent_heading_body_resolution
    );
    assert!(analysis.descriptor.capabilities.typed_effect_evidence);
    assert_eq!(analysis.descriptor.contract_version, 2);
    assert_eq!(analysis.model.semantic_model_version, 3);
    assert_eq!(analysis.view.blocks, analysis.model.blocks);
    assert_eq!(analysis.view.inlines, analysis.model.inlines);
    assert_eq!(analysis.view.run_in_groups, analysis.model.run_in_groups);
    assert_eq!(
        analysis.view.adjacent_heading_bodies,
        analysis.model.adjacent_heading_bodies
    );
    assert_eq!(analysis.view.effects, analysis.model.effects);
    assert_eq!(analysis.view.degradations, analysis.model.degradations);
    assert_eq!(analysis.view.safe_html, analysis.model.safe_html);
    assert!(analysis.searchable_text.contains("正文"));
    assert_eq!(
        parse_document(source, AdjacentHeadingBody::RunIn),
        analysis.model
    );
}

#[test]
fn generic_model_locks_h1_h9_actual_quotes_run_in_and_block_ids() {
    let headings = (1..=9).fold(String::new(), |mut result, level| {
        writeln!(result, "[#h{level}]\n{} H{level}\n", "=".repeat(level + 1)).unwrap();
        result
    });
    let source = format!(
        "= Document\n\n{headings}> > > 引用 🧠\n\n[.run-in]\n[#heading]\n== Run in\n[#body]\n正文 שלום\n"
    );
    let model = parse_document(&source, AdjacentHeadingBody::RunIn);
    let levels = model
        .blocks
        .iter()
        .filter_map(|block| block.heading_level)
        .collect::<Vec<_>>();
    assert_eq!(levels[0], 0);
    assert_eq!(&levels[1..10], (1..=9).collect::<Vec<_>>().as_slice());
    assert_eq!(levels[10], 1);
    let quote = model
        .blocks
        .iter()
        .find(|block| block.kind == DocumentBlockKind::Quote)
        .unwrap();
    assert_eq!(quote.quote_depth, Some(3));
    assert_eq!(quote.text, "引用 🧠");
    assert_eq!(model.run_in_groups.len(), 1);
    assert!(model.blocks.iter().any(|block| {
        block.kind == DocumentBlockKind::Paragraph && block.block_id.as_deref() == Some("body")
    }));
}

#[test]
fn syntax_occurrences_are_extracted_before_uuid_resolution_and_respect_protected_regions() {
    let target = "550e8400-e29b-41d4-a716-446655440000";
    let source = format!(
        "---\nweftext:\n  id: \"11111111-1111-4111-8111-111111111111\"\n  aliases:\n    - 'node:{target}[]'\n---\n= Document\n\nnode:{target}#section[标题]\n\n+node:{target}[Code]+\n\n[source]\n----\nnode:{target}[Fence]\n----\n\nnode::{target}#block[]\n"
    );
    let analysis = analyze_document(&source, AdjacentHeadingBody::Separate);
    assert_eq!(analysis.occurrences.links.len(), 2);
    assert_eq!(
        analysis
            .occurrences
            .links
            .iter()
            .map(|link| (link.kind, link.authored_locator.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (DocumentLinkKind::Link, target),
            (DocumentLinkKind::Embed, target)
        ]
    );
    let serialized = serde_json::to_value(&analysis.occurrences.links[0]).unwrap();
    assert!(serialized.get("targetNodeIds").is_none());
    assert!(
        analysis
            .occurrences
            .protected_ranges
            .iter()
            .any(|range| { range.start == 0 && range.end >= analysis.model.blocks[0].end })
    );
}

#[test]
fn semantic_link_resolution_consumes_occurrences_without_changing_source_order() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("Root");
    let root_node = create_workspace(&root).unwrap();
    let beta = create_child_node(&root, "Beta").unwrap();
    let exact = source(
        root_node.id,
        &format!("node:{}[Beta] then node:{}[again]\n", beta.id, beta.id),
        "\n",
    );
    fs::write(&root_node.document_path, exact.as_bytes()).unwrap();
    let index = build_workspace_link_index(&root).unwrap();
    assert_eq!(index.outgoing.len(), 2);
    assert!(index.outgoing[0].start < index.outgoing[1].start);
    assert_eq!(index.outgoing[0].target_node_ids, vec![beta.id]);
    assert_eq!(index.outgoing[1].target_node_ids, vec![beta.id]);
    assert_eq!(
        fs::read(&root_node.document_path).unwrap(),
        exact.as_bytes()
    );
}

#[test]
fn structural_create_uses_only_the_canonical_asciidoc_layout() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("Root");
    let root_node = create_workspace(&root).unwrap();
    let plan = plan_create_child_node(&root, root_node.id, "Child").unwrap();
    commit_workspace_transaction(&plan).unwrap();
    assert!(root.join("Child/Child.adoc").is_file());
    assert!(!root.join("Child/Child.md").exists());
    assert!(scan_workspace(&root).is_valid());
}

#[test]
fn annotation_sidecar_v3_tags_exact_asciidoc_body_and_actor() {
    let id = weftext_core::NodeId::new_v4();
    let now = "2026-08-23T00:00:00Z".to_owned();
    let store = AnnotationStore {
        version: weftext_core::ANNOTATION_STORE_VERSION,
        document_id: id,
        annotations: vec![Annotation {
            id: Uuid::new_v4(),
            kind: AnnotationKind::Comment,
            target: Anchor::Document,
            appearance: None,
            suggested_source: None,
            labels: Vec::new(),
            thread: vec![ThreadMessage {
                id: Uuid::new_v4(),
                author_id: Uuid::new_v4(),
                author_name: "审阅者".to_owned(),
                body: AnnotationBody {
                    format: AnnotationBodyFormat::AsciiDocInlineV1,
                    source: "保留 *AsciiDoc*".to_owned(),
                },
                created_at: now.clone(),
                updated_at: now.clone(),
            }],
            state: AnnotationState::Open,
            resolution: None,
            created_at: now.clone(),
            updated_at: now,
        }],
    };
    let json = store.to_pretty_json().unwrap();
    assert!(json.contains("\"format\": \"weftext.asciidoc.inline.v1\""));
    assert!(json.contains("\"source\": \"保留 *AsciiDoc*\""));
    assert!(json.contains("\"author_name\": \"审阅者\""));
    assert_eq!(AnnotationStore::from_json(&json).unwrap(), store);
}

#[test]
fn annotation_runtime_rejects_legacy_sidecar_versions() {
    let document_id = weftext_core::NodeId::new_v4();
    let annotation_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let legacy = serde_json::json!({
        "version": 1,
        "document_id": document_id,
        "annotations": [{
            "id": annotation_id,
            "anchor": {"kind": "document"},
            "appearance": {"mark": "none", "color": "yellow"},
            "labels": [],
            "thread": [{
                "id": message_id,
                "body_markdown": "保留 **Markdown** 与换行\r\n",
                "created_at": "2026-08-23T00:00:00Z",
                "updated_at": "2026-08-23T00:00:00Z"
            }],
            "state": "open",
            "created_at": "2026-08-23T00:00:00Z",
            "updated_at": "2026-08-23T00:00:00Z"
        }]
    });
    assert!(matches!(
        AnnotationStore::from_json(&legacy.to_string()),
        Err(weftext_core::AnnotationValidationError::UnsupportedVersion(
            1
        ))
    ));
}
