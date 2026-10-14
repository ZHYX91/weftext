use std::collections::BTreeMap;
use weftext_knowledge::*;

const WORKSPACE: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
const A: &str = "12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
const B: &str = "8aa10c08-3e7d-4d10-9488-92bc7f393050";
const C: &str = "82b289ba-01ae-4c7b-8e51-779442316658";
const D: &str = "202173ac-0e45-45cb-8998-7289b1688b0c";
fn owner(id: &str) -> NodeRef {
    NodeRef {
        workspace_id: WORKSPACE.into(),
        node_id: id.into(),
    }
}
fn node(id: &str, path: &str, body: &str, title: &str) -> NodeSnapshot {
    NodeSnapshot {
        owner: owner(id),
        path: url::Url::parse(&format!("file:///D:/notes/{path}"))
            .unwrap()
            .to_string(),
        library_path: path.into(),
        revision_id: format!("rev-{id}"),
        source: format!("#metadata((id: \"{id}\", types: ())) <weftext>\n\n{body}"),
        title: Some(title.into()),
        aliases: vec![],
        labels: vec!["route".into()],
        labels_complete: true,
    }
}
fn inventory(nodes: Vec<NodeSnapshot>) -> InventorySnapshot {
    InventorySnapshot {
        generation: "cut-1".into(),
        coverage: Coverage::Complete,
        nodes,
        resources: vec![],
    }
}
fn id_link(id: &str) -> String {
    format!("weftext-node://{WORKSPACE}/{id}")
}
fn query_declaration(key: &str) -> String {
    format!(
        "#metadata((kind: \"weftext.query\", key: \"{key}\", binding: (schema: \"weftext-config://{WORKSPACE}/schema\", revision: \"{D}\", language: \"weftext.query.cel.v1\"), where: \"true\")) <weftext-query-demo>"
    )
}
fn fragment(text: &str) -> LogicalFragment {
    LogicalFragment {
        id: "html-n1".into(),
        node_key: "html-n1".into(),
        text: text.into(),
        source_range: Some(ExactRange::new(text, 0, text.len()).unwrap()),
    }
}
fn record(target: AnnotationTarget) -> AnnotationRecord {
    AnnotationRecord {
        annotation_id: "annotation-1".into(),
        reply_to: None,
        attribution: "author".into(),
        created_at: "2026-10-13T00:00:00Z".into(),
        purpose: "comment".into(),
        body: "Keep this discussion".into(),
        labels: vec![],
        appearance: serde_json::Value::Null,
        review_state: "open".into(),
        target,
        suggestion: None,
        target_history: vec![],
        unknown: BTreeMap::new(),
    }
}
fn document(records: Vec<AnnotationRecord>) -> AnnotationDocument {
    decode_annotations(
        &serde_json::to_vec(
            &serde_json::json!({"version":1,"annotations":records,"future":{"keep":true}}),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn native_binding_and_destination_are_official_ast_slots() {
    let text = format!("#link(\"{}\")[*display*]", id_link(B));
    let links = native_links(&text);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, "native");
    assert!(
        links[0]
            .destination_slot
            .as_ref()
            .unwrap()
            .raw
            .starts_with('"')
    );
    assert!(native_links(&format!("#let link(x) = x\n{text}")).is_empty());
    assert!(native_links(&format!("#import \"other.typ\": *\n{text}")).is_empty());
    assert!(native_links("#link(destination)[display]").is_empty());
    assert!(
        native_links(&format!(
            "#link(\"{}\", underline: false)[invalid named argument]",
            id_link(B)
        ))
        .is_empty()
    );
    let local = native_links("#link(<plan>)[jump] @plan");
    assert_eq!(local.len(), 2);
    assert_eq!(local[0].kind, "nativeLabel");
    assert_eq!(local[1].kind, "nativeRef");
}
#[test]
fn conversion_preserves_fragment_parameters_display_and_outside_bytes() {
    let source = node(
        A,
        "写作/写作.typ",
        &format!("prefix #link(\"{}#route\")[甲*乙*丙] suffix", id_link(B)),
        "source",
    );
    let target = node(B, "资料/白云山/白云山.typ", "", "白云山");
    let snapshot = inventory(vec![source.clone(), target]);
    let link = native_links(&source.source).remove(0);
    for form in [
        LinkForm::Id,
        LinkForm::Relative,
        LinkForm::Workspace,
        LinkForm::File,
    ] {
        let patch = convert_link(&source, &snapshot, link.from, form).unwrap();
        assert_eq!(
            &patch.after[..patch.range.from],
            &source.source[..patch.range.from]
        );
        assert!(patch.after.ends_with(")[甲*乙*丙] suffix"));
        let mut updated = source.clone();
        updated.source = patch.after;
        let changed = native_links(&updated.source).remove(0);
        let resolved = resolve_link(&changed.target, &updated, &snapshot);
        assert_eq!(resolved.target, Some(owner(B)));
        assert_eq!(resolved.fragment.as_deref(), Some("route"));
    }
}
#[test]
fn relative_links_use_actual_source_directory_not_unique_suffix() {
    let source = node(A, "甲/乙/原文.typ", "", "source");
    let target = node(B, "甲/丙/目标.typ", "", "target");
    let address = target_address(&source, &target, Some("route"), LinkForm::Relative).unwrap();
    assert_eq!(address, "../%E4%B8%99/%E7%9B%AE%E6%A0%87.typ#route");
    let same = target_address(&source, &source, None, LinkForm::Relative).unwrap();
    assert_eq!(same, "%E5%8E%9F%E6%96%87.typ");
}

#[test]
fn bodyless_native_conversion_freezes_original_display_as_one_call_patch() {
    let source = node(
        A,
        "A/A.typ",
        &format!("before #link(\"{}\") after", id_link(B)),
        "A",
    );
    let target = node(B, "B/B.typ", "", "B");
    let snapshot = inventory(vec![source.clone(), target]);
    let link = native_links(&source.source).remove(0);
    assert!(!link.body_explicit);
    let patch = convert_link(&source, &snapshot, link.from, LinkForm::Relative).unwrap();
    assert_eq!(patch.range.from, link.from);
    assert_eq!(patch.range.to, link.to);
    assert!(patch.after.contains("#link(\"../B/B.typ\")[weftext"));
    assert!(native_links(&patch.after)[0].body_explicit);
    assert_eq!(
        weftext_engine::inspect_editor(&patch.after)
            .literal_regions
            .iter()
            .find(|r| r.text == id_link(B))
            .unwrap()
            .text,
        id_link(B)
    );
}
#[test]
fn malformed_encoded_escape_separators_and_boundary_escape_are_rejected() {
    let source = node(A, "A/A.typ", "", "A");
    let snapshot = inventory(vec![source.clone()]);
    for address in [
        "../%2Fsecret.typ",
        "../%5csecret.typ",
        "../../escape.typ",
        "x%ZZ.typ",
        "x%00.typ",
        "file:///D:/notes/A/%2e%2e/A.typ",
        "file://server/share/A.typ",
        "weftext-node://evil/other",
    ] {
        assert_eq!(
            resolve_link(address, &source, &snapshot).status,
            "invalid",
            "{address}"
        );
    }
}
#[test]
fn uri_decodes_once_plus_is_literal_and_no_unicode_normalization() {
    let source = node(A, "A/A.typ", "", "A");
    let literal = node(B, "A/%2F+é.typ", "", "literal");
    let snapshot = inventory(vec![source.clone(), literal]);
    assert_eq!(
        resolve_link("%252F%2Be%CC%81.typ", &source, &snapshot).target,
        Some(owner(B))
    );
    assert_eq!(
        resolve_link("%252F%2B%C3%A9.typ", &source, &snapshot).status,
        "missing"
    );
}
#[test]
fn id_survives_move_path_index_does_not_retarget_namesakes() {
    let source = node(
        A,
        "A/A.typ",
        &format!("#link(\"{}\")[id] #link(\"../B/B.typ\")[path]", id_link(B)),
        "A",
    );
    let target = node(B, "B/B.typ", "", "B");
    let mut snapshot = inventory(vec![source, target]);
    let index = link_index(&snapshot, &owner(B));
    assert_eq!(index.incoming_id.len(), 1);
    assert_eq!(index.incoming_path.len(), 1);
    snapshot.nodes[1].library_path = "Moved/B.typ".into();
    snapshot.nodes[1].path = "file:///D:/notes/Moved/B.typ".into();
    let index = link_index(&snapshot, &owner(B));
    assert_eq!(index.incoming_id.len(), 1);
    assert!(index.incoming_path.is_empty());
    snapshot.nodes.remove(1);
    snapshot.coverage = Coverage::Partial;
    let index = link_index(&snapshot, &owner(A));
    assert!(
        index
            .outgoing
            .iter()
            .all(|edge| edge.resolution.status == "unavailable")
    );
    assert_eq!(index.coverage, Coverage::Partial);
}
#[test]
fn duplicate_and_unavailable_fragments_deny_conversion() {
    let source = node(
        A,
        "A/A.typ",
        &format!("#link(\"{}#route\")[B]", id_link(B)),
        "A",
    );
    let mut target = node(B, "B/B.typ", "", "B");
    target.labels.push("route".into());
    let snapshot = inventory(vec![source.clone(), target.clone()]);
    assert_eq!(
        resolve_link(&format!("{}#route", id_link(B)), &source, &snapshot).status,
        "ambiguous"
    );
    target.labels_complete = false;
    let snapshot = inventory(vec![source.clone(), target]);
    assert_eq!(
        convert_link(
            &source,
            &snapshot,
            native_links(&source.source)[0].from,
            LinkForm::Relative
        )
        .unwrap_err()
        .code,
        "unavailable"
    );
}
#[test]
fn all_same_span_nested_crossing_mentions_have_accessible_lanes() {
    let snapshot = inventory(vec![
        node(A, "A/A.typ", "", "白云山"),
        node(B, "B/B.typ", "", "白云山"),
        node(C, "C/C.typ", "", "山"),
        node(D, "D/D.typ", "", "山南路"),
    ]);
    let current = owner("ffffffff-ffff-4fff-8fff-ffffffffffff");
    let scan = scan_mentions(&snapshot, &current, &[fragment("白云山南路")], 100).unwrap();
    let hits = &scan.fragments[0].hits;
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].targets, vec![owner(A), owner(B)]);
    assert_eq!(
        (hits[0].start_scalar, hits[0].end_scalar, hits[0].lane),
        (0, 3, Some(0))
    );
    assert_eq!(
        (hits[1].start_scalar, hits[1].end_scalar, hits[1].lane),
        (2, 5, Some(1))
    );
    assert_eq!(
        (hits[2].start_scalar, hits[2].end_scalar, hits[2].lane),
        (2, 3, None)
    );
    assert!(hits.iter().all(|hit| hit.component == 0));
}
#[test]
fn touching_ranges_reuse_lane_and_partial_scan_never_claims_complete() {
    let snapshot = inventory(vec![node(B, "B/B.typ", "", "山")]);
    let scan = scan_mentions(
        &snapshot,
        &owner(A),
        &[fragment("山山山"), fragment("山")],
        2,
    )
    .unwrap();
    assert_eq!(scan.coverage, Coverage::Partial);
    assert_eq!(scan.fragments[0].hits.len(), 2);
    assert!(scan.fragments[0].hits.iter().all(|hit| hit.lane == Some(0)));
    assert_eq!(scan.fragments[1].coverage, Coverage::Partial);
}
#[test]
fn partial_graphemes_are_discovery_only_and_utf16_is_explicit() {
    let snapshot = inventory(vec![
        node(B, "B/B.typ", "", "e"),
        node(C, "C/C.typ", "", "👩"),
    ]);
    let scan = scan_mentions(&snapshot, &owner(A), &[fragment("é 👩‍👩‍👧")], 100).unwrap();
    assert_eq!(scan.fragments[0].hits.len(), 3);
    assert!(scan.fragments[0].hits.iter().all(|hit| !hit.writable));
    let first = &scan.fragments[0].hits[1];
    assert_eq!(first.end_utf16 - first.start_utf16, 2);
    assert!(
        source_anchor(
            &node(A, "A/A.typ", "é", ""),
            node(A, "A/A.typ", "é", "").source.len() - 3,
            node(A, "A/A.typ", "é", "").source.len() - 2,
            "e".into()
        )
        .is_err()
    );
}
#[test]
fn potential_conversion_requires_explicit_current_target() {
    let source = node(A, "A/A.typ", "白云山", "source");
    let target = node(B, "B/B.typ", "", "白云山");
    let snapshot = inventory(vec![source.clone(), target]);
    let start = source.source.find("白云山").unwrap();
    let fragment = LogicalFragment {
        id: "x".into(),
        node_key: "x".into(),
        text: "白云山".into(),
        source_range: Some(ExactRange::new(&source.source, start, start + 9).unwrap()),
    };
    let scan = scan_mentions(&snapshot, &source.owner, &[fragment], 100).unwrap();
    let hit = &scan.fragments[0].hits[0];
    assert!(convert_mention(&source, &snapshot, hit, &owner(C)).is_err());
    let patch = convert_mention(&source, &snapshot, hit, &owner(B)).unwrap();
    assert!(patch.after.ends_with("[白云山]"));
    let mut revoked = snapshot.clone();
    revoked.nodes.remove(1);
    assert_eq!(
        convert_mention(&source, &revoked, hit, &owner(B))
            .unwrap_err()
            .code,
        "unavailable"
    );
}
#[test]
fn damaged_duplicate_json_and_unknown_target_are_never_initialized() {
    for bytes in [
        b"{".as_slice(),
        br#"{"version":1,"version":1,"annotations":[]}"#,
        br#"{"version":9,"annotations":[]}"#,
        br#"{"version":1,"annotations":{}}"#,
    ] {
        assert!(decode_annotations(bytes).is_err());
    }
}
#[test]
fn annotation_projection_never_exposes_private_raw_source() {
    let raw = "#link(\"https://private.example/?token=secret\")[公开文字]";
    let source = node(A, "A/A.typ", raw, "A");
    let start = source.source.find(raw).unwrap();
    let anchor = source_anchor(&source, start, source.source.len(), "公开文字".into()).unwrap();
    let doc = document(vec![record(AnnotationTarget::SourceRange { anchor })]);
    let view = serde_json::to_string(&annotation_views(&doc, None, &[])).unwrap();
    assert!(view.contains("公开文字"));
    assert!(!view.contains("token"));
    assert!(!view.contains("private.example"));
    assert!(!view.contains("sourceRange"));
    let after = doc.resolve("annotation-1", true).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(value["future"]["keep"], true);
    assert!(String::from_utf8(after).unwrap().contains("token=secret"));
}
#[test]
fn equal_text_other_revision_stays_stale_exact_edits_relocate() {
    let source = node(A, "A/A.typ", "原文", "A");
    let start = source.source.find("原文").unwrap();
    let anchor = source_anchor(&source, start, start + 6, "原文".into()).unwrap();
    let mut current = source.clone();
    current.revision_id = "rev-after".into();
    assert_eq!(resolve_anchor(&anchor, Some(&current), &[]).status, "stale");
    let patch = SourcePatch::new(
        &source.source,
        ExactRange::new(&source.source, start, start).unwrap(),
        "前缀 ".into(),
    )
    .unwrap();
    current.source = patch.after.clone();
    let resolution = resolve_anchor(
        &anchor,
        Some(&current),
        &[ExactEdit {
            before: source.clone(),
            after: current.clone(),
            patch,
        }],
    );
    assert_eq!(resolution.status, "mapped");
    assert_eq!(resolution.current_range.unwrap().from, start + 7);
    let patch =
        SourcePatch::new(&source.source, anchor.source_range.clone(), "改文".into()).unwrap();
    current.source = patch.after.clone();
    assert_eq!(
        resolve_anchor(
            &anchor,
            Some(&current),
            &[ExactEdit {
                before: source,
                after: current.clone(),
                patch
            }]
        )
        .status,
        "stale"
    );
}
#[test]
fn suggestions_plan_source_and_acceptance_together_reject_needs_no_source() {
    let source = node(A, "A/A.typ", "原文", "A");
    let start = source.source.find("原文").unwrap();
    let anchor = source_anchor(&source, start, start + 6, "原文".into()).unwrap();
    let mut r = record(AnnotationTarget::SourceRange { anchor });
    r.purpose = "suggestion".into();
    r.suggestion = Some(Suggestion {
        replacement: "#metadata[无执行]".into(),
        state: "pending".into(),
        confirmed_revision: source.revision_id.clone(),
        requires_reconfirmation: false,
    });
    let doc = document(vec![r]);
    let plan = suggestion_plan(&doc, "sidecar", "annotation-1", &source).unwrap();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(
        plan.files[0].before.as_deref(),
        Some(source.source.as_bytes())
    );
    assert_eq!(
        plan.files[1].before.as_deref(),
        Some(doc.original.as_slice())
    );
    assert!(plan.source_patch.after.contains("\\#metadata"));
    let after = decode_annotations(&plan.files[1].after).unwrap();
    assert_eq!(
        after.records[0].suggestion.as_ref().unwrap().state,
        "accepted"
    );
    let rejected = decode_annotations(&doc.reject("annotation-1").unwrap()).unwrap();
    assert_eq!(
        rejected.records[0].suggestion.as_ref().unwrap().state,
        "rejected"
    );
    let mut changed = source.clone();
    changed.revision_id = "later".into();
    assert_eq!(
        suggestion_plan(&doc, "sidecar", "annotation-1", &changed)
            .unwrap_err()
            .code,
        "staleSuggestion"
    );
}
#[test]
fn source_string_or_partial_grapheme_cannot_become_suggestion_writer() {
    let source = node(A, "A/A.typ", "#link(\"https://secret.test\")[public]", "A");
    let start = source.source.find("secret").unwrap();
    let anchor = source_anchor(&source, start, start + 6, "secret".into()).unwrap();
    let mut r = record(AnnotationTarget::SourceRange { anchor });
    r.purpose = "suggestion".into();
    r.suggestion = Some(Suggestion {
        replacement: "x".into(),
        state: "pending".into(),
        confirmed_revision: source.revision_id.clone(),
        requires_reconfirmation: false,
    });
    assert_eq!(
        suggestion_plan(&document(vec![r]), "sidecar", "annotation-1", &source)
            .unwrap_err()
            .code,
        "sourceEntry"
    );
}
#[test]
fn exact_media_shapes_validate_ranges_without_whole_attachment_downgrade() {
    let mut r = record(AnnotationTarget::MediaInterval {
        resource: "resource".into(),
        revision_id: "r1".into(),
        timebase: Timebase {
            numerator: 1,
            denominator: 48_000,
        },
        start_tick: 48_000,
        end_tick: 96_000,
        duration_tick: 144_000,
        region: Some(VideoRegion {
            pixel_width: 1920,
            pixel_height: 1080,
            rect: PixelRect {
                x: 100,
                y: 100,
                width: 200,
                height: 300,
            },
        }),
    });
    assert!(decode_annotations(&document(vec![r.clone()]).original).is_ok());
    r.target = AnnotationTarget::ImageRegion {
        resource: "resource".into(),
        revision_id: "r1".into(),
        pixel_width: 100,
        pixel_height: 100,
        rect: PixelRect {
            x: u64::MAX,
            y: 0,
            width: 2,
            height: 2,
        },
    };
    let bytes = serde_json::to_vec(&serde_json::json!({"version":1,"annotations":[r]})).unwrap();
    let invalid = decode_annotations(&bytes).unwrap();
    assert_eq!(
        annotation_views(&invalid, None, &[])[0].target_state,
        "invalid"
    );
    assert!(invalid.resolve("annotation-1", true).is_ok());
}
#[test]
fn discussion_replies_and_resolution_survive_unlocated_target() {
    let r = record(AnnotationTarget::WholeDocument { node: owner(A) });
    let doc = document(vec![r]);
    let mut reply = record(AnnotationTarget::WholeDocument { node: owner(A) });
    reply.annotation_id = "reply-2".into();
    reply.reply_to = Some("annotation-1".into());
    reply.body = "Still useful".into();
    let doc = decode_annotations(&doc.reply(reply).unwrap()).unwrap();
    assert_eq!(doc.records.len(), 2);
    assert_eq!(annotation_views(&doc, None, &[])[1].body, "Still useful");
    assert!(doc.resolve("annotation-1", true).is_ok());
}
fn copy_input(files: Vec<CopyFile>, nodes: Vec<NodeSnapshot>) -> CopyInput {
    let node_map = BTreeMap::from([(owner(A), owner(C)), (owner(B), owner(D))])
        .into_iter()
        .filter(|(owner, _)| {
            files.iter().any(|file| {
                file.owner.as_ref() == Some(owner) && matches!(file.kind, CopyFileKind::Source)
            })
        })
        .collect();
    CopyInput {
        files,
        node_map,
        resource_map: BTreeMap::new(),
        annotation_map: BTreeMap::new(),
        definition_map: BTreeMap::new(),
        revision_map: BTreeMap::new(),
        resource_revision_map: BTreeMap::new(),
        inventory: inventory(nodes),
        allow_losses: false,
    }
}
fn copy_file(node: &NodeSnapshot, target: &str) -> CopyFile {
    CopyFile {
        source_path: node.path.clone(),
        target_path: target.into(),
        before: node.source.as_bytes().to_vec(),
        kind: CopyFileKind::Source,
        owner: Some(node.owner.clone()),
        typed_refs: vec![],
        definition_references: vec![],
        typed_refs_complete: true,
    }
}
#[test]
fn copy_rewrites_all_mutual_native_links_and_ids_without_touching_phone_text() {
    let a = node(
        A,
        "A/A.typ",
        &format!("#link(\"../B/B.typ#route\")[乙]\n电话 {B}"),
        "A",
    );
    let b = node(B, "B/B.typ", &format!("#link(\"{}\")[甲]", id_link(A)), "B");
    let files = vec![
        copy_file(&b, "file:///D:/notes/copyB/B.typ"),
        copy_file(&a, "file:///D:/notes/copyA/A.typ"),
    ];
    let plan = plan_copy(copy_input(files, vec![a, b])).unwrap();
    assert_eq!(plan.files.len(), 2);
    let first = String::from_utf8(plan.files[0].after.clone()).unwrap();
    assert!(first.contains(&id_link(D)));
    assert!(first.contains(&format!("电话 {B}")));
    assert!(first.contains(&format!("id: \"{C}\"")));
    assert!(
        String::from_utf8(plan.files[1].after.clone())
            .unwrap()
            .contains(&id_link(C))
    );
    assert!(plan.losses.is_empty());
}
#[test]
fn unknown_copy_refs_reject_or_require_original_preserving_loss_plan() {
    let a = node(A, "A/A.typ", "unbound", "A");
    let mut f = copy_file(&a, "file:///D:/notes/copyA/A.typ");
    f.typed_refs_complete = false;
    let mut input = copy_input(vec![f], vec![a]);
    assert_eq!(plan_copy(input.clone()).unwrap_err().code, "copyLosses");
    input.allow_losses = true;
    let plan = plan_copy(input).unwrap();
    assert_eq!(plan.losses.len(), 1);
    assert_eq!(plan.preserved_originals.len(), 1);
}
#[test]
fn copy_annotation_mapping_is_order_independent_and_retains_replies() {
    let a = node(A, "A/A.typ", "原文", "A");
    let start = a.source.find("原文").unwrap();
    let r = record(AnnotationTarget::SourceRange {
        anchor: source_anchor(&a, start, start + 6, "原文".into()).unwrap(),
    });
    let doc = document(vec![r]);
    let annotation_file = CopyFile {
        source_path: "old.annotations.json".into(),
        target_path: "new.annotations.json".into(),
        before: doc.original,
        kind: CopyFileKind::Annotations,
        owner: Some(a.owner.clone()),
        typed_refs: vec![],
        definition_references: vec![],
        typed_refs_complete: true,
    };
    let mut input = copy_input(
        vec![
            annotation_file,
            copy_file(&a, "file:///D:/notes/copyA/A.typ"),
        ],
        vec![a],
    );
    input
        .annotation_map
        .insert("annotation-1".into(), "fresh-annotation".into());
    input.revision_map.insert(owner(A), "new-candidate".into());
    let plan = plan_copy(input).unwrap();
    let file = plan
        .files
        .iter()
        .find(|file| file.path == "new.annotations.json")
        .unwrap();
    let doc = decode_annotations(&file.after).unwrap();
    let anchor = doc.records[0].target.anchor().unwrap();
    assert_eq!(anchor.node, owner(C));
    assert_eq!(anchor.revision_id, "new-candidate");
    assert_eq!(anchor.source_range.raw, "原文");
    assert!(plan.losses.is_empty());
}
#[test]
fn template_missing_defaults_null_and_injection_are_distinct() {
    let source = "#metadata((title: \"placeholder\")) <weftext>\n正文";
    let start = source.find("\"placeholder\"").unwrap();
    let slot = TemplateSlot {
        name: "title".into(),
        range: ExactRange::new(source, start, start + 13).unwrap(),
        kind: TemplateSlotKind::String,
        default: Some(TemplateValue::Text("默认".into())),
        nullable: true,
    };
    assert!(
        instantiate_template(source, std::slice::from_ref(&slot), &BTreeMap::new())
            .unwrap()
            .contains("\"默认\"")
    );
    assert!(
        instantiate_template(
            source,
            std::slice::from_ref(&slot),
            &BTreeMap::from([("title".into(), TemplateValue::Null)])
        )
        .unwrap()
        .contains("title: none")
    );
    let after = instantiate_template(
        source,
        &[slot],
        &BTreeMap::from([(
            "title".into(),
            TemplateValue::Text("\"); #metadata((id: \"forged\"))".into()),
        )]),
    )
    .unwrap();
    assert_eq!(after.matches("#metadata").count(), 2); // input remains inside one escaped native string
    assert!(weftext_engine::inspect_metadata(&after).metadata.is_some());
}

#[test]
fn ref_free_query_definitions_get_new_keys_and_unknown_definitions_reject() {
    let a = node(A, "A/A.typ", &query_declaration(B), "A");
    let slots = definition_slots(&a.source, &a.owner).unwrap();
    assert_eq!(slots.len(), 1);
    let mut input = copy_input(vec![copy_file(&a, "file:///D:/notes/copyA/A.typ")], vec![a]);
    input
        .definition_map
        .insert(slots[0].identity.clone(), D.into());
    let plan = plan_copy(input).unwrap();
    let after = String::from_utf8(plan.files[0].after.clone()).unwrap();
    assert!(after.contains(&format!("key: \"{D}\"")));
    assert!(!after.contains(&format!("key: \"{B}\"")));
    assert!(
        definition_slots(
            "#metadata((kind: \"weftext.future\", key: \"x\")) <weftext-query-demo>",
            &owner(A)
        )
        .is_err()
    );
}
#[test]
fn current_definition_refs_follow_map_pinned_triple_is_preserved_as_loss() {
    let query = node(B, "B/B.typ", &query_declaration(C), "B");
    let mut source = node(
        A,
        "A/A.typ",
        &format!(
            "#metadata((owner: \"{}\", key: \"{C}\", revision: \"historical\")) <reference>",
            id_link(B)
        ),
        "A",
    );
    let owner_text = format!("\"{}\"", id_link(B));
    let owner_start = source.source.find(&owner_text).unwrap();
    let key_text = format!("\"{C}\"");
    let key_start = source.source.find(&key_text).unwrap();
    let revision_start = source.source.find("\"historical\"").unwrap();
    let reference = DefinitionReferenceSlots {
        owner: owner(B),
        key: C.into(),
        revision: Some("historical".into()),
        owner_slot: ExactRange::new(&source.source, owner_start, owner_start + owner_text.len())
            .unwrap(),
        key_slot: ExactRange::new(&source.source, key_start, key_start + key_text.len()).unwrap(),
        revision_slot: Some(
            ExactRange::new(&source.source, revision_start, revision_start + 12).unwrap(),
        ),
    };
    let mut file = copy_file(&source, "file:///D:/notes/copyA/A.typ");
    file.definition_references.push(reference.clone());
    file.typed_refs.push(TypedRefSlot {
        from: reference.owner_slot.from,
        to: reference.owner_slot.to,
        target: id_link(B),
        kind: "node_ref".into(),
    });
    let mut input = copy_input(
        vec![file, copy_file(&query, "file:///D:/notes/copyB/B.typ")],
        vec![source.clone(), query.clone()],
    );
    input
        .definition_map
        .insert(definition_identity(&owner(B), C), A.into());
    input.allow_losses = true;
    let plan = plan_copy(input).unwrap();
    let after = String::from_utf8(plan.files[0].after.clone()).unwrap();
    assert!(after.contains(&id_link(B)));
    assert!(after.contains(&format!("key: \"{C}\"")));
    assert!(after.contains("historical"));
    assert_eq!(plan.losses[0].code, "pinnedHistoricalDefinition");
    source.source = source.source.replace(", revision: \"historical\"", "");
    let mut file = copy_file(&source, "file:///D:/notes/copyA/A.typ");
    let mut current = reference;
    current.revision = None;
    current.revision_slot = None;
    file.definition_references.push(current);
    let mut input = copy_input(
        vec![file, copy_file(&query, "file:///D:/notes/copyB/B.typ")],
        vec![source, query],
    );
    input
        .definition_map
        .insert(definition_identity(&owner(B), C), A.into());
    let plan = plan_copy(input).unwrap();
    let after = String::from_utf8(plan.files[0].after.clone()).unwrap();
    assert!(after.contains(&id_link(D)));
    assert!(after.contains(&format!("key: \"{A}\"")));
}
#[test]
fn forged_continuity_map_cannot_reanchor_equal_text() {
    let source = node(A, "A/A.typ", "原文", "A");
    let start = source.source.find("原文").unwrap();
    let anchor = source_anchor(&source, start, start + 6, "原文".into()).unwrap();
    let mut patch = SourcePatch::new(
        &source.source,
        ExactRange::new(&source.source, start, start).unwrap(),
        "前缀".into(),
    )
    .unwrap();
    patch.after = patch.after.replace("types: ()", "types: (\"changed\",)");
    let mut current = source.clone();
    current.source = patch.after.clone();
    current.revision_id = "rev-new".into();
    assert_eq!(
        resolve_anchor(
            &anchor,
            Some(&current),
            &[ExactEdit {
                before: source,
                after: current.clone(),
                patch
            }]
        )
        .status,
        "stale"
    );
}
#[test]
fn resource_facts_require_exact_units_revision_and_real_page_dimensions() {
    let target = AnnotationTarget::PdfRegion {
        resource: "r".into(),
        revision_id: "v1".into(),
        page_index: 0,
        page_width: 600.0,
        page_height: 800.0,
        rotation: 0,
        rect: PointRect {
            x: 10.0,
            y: 20.0,
            width: 50.0,
            height: 60.0,
        },
    };
    let facts = ResourceFacts::Pdf {
        resource: "r".into(),
        revision_id: "v1".into(),
        pages: vec![PdfPageFacts {
            width: 600.0,
            height: 800.0,
            rotation: 0,
            extraction_id: None,
            extracted_scalars: None,
        }],
    };
    assert!(validate_resource_target(&target, &facts).is_ok());
    let mismatch = ResourceFacts::Pdf {
        resource: "r".into(),
        revision_id: "v2".into(),
        pages: vec![PdfPageFacts {
            width: 600.0,
            height: 800.0,
            rotation: 0,
            extraction_id: None,
            extracted_scalars: None,
        }],
    };
    assert!(validate_resource_target(&target, &mismatch).is_err());
}

#[test]
fn computed_links_imports_and_native_aliases_cannot_claim_complete_copy() {
    for body in [
        format!(
            "#let destination=\"{}\"\n#link(destination)[display]",
            id_link(B)
        ),
        "#include \"A.typ\"".into(),
        format!("#let go=link\n#go(\"{}\")[display]", id_link(B)),
    ] {
        let source = node(A, "A/A.typ", &body, "A");
        let mut input = copy_input(
            vec![copy_file(&source, "file:///D:/notes/copyA/A.typ")],
            vec![source],
        );
        assert!(plan_copy(input.clone()).is_err());
        input.allow_losses = true;
        let plan = plan_copy(input).unwrap();
        assert!(!plan.losses.is_empty());
        assert_eq!(plan.preserved_originals.len(), 1);
    }
}
#[test]
fn copied_native_resource_paths_resolve_old_context_and_preserve_literal_percent() {
    let source = node(A, "A/A.typ", "#image(\"asset/pic%2F+.png\")", "A");
    let resource_path = "file:///D:/notes/A/asset/pic%252F+.png";
    let old = format!("weftext-resource://{WORKSPACE}/{B}");
    let new = format!("weftext-resource://{WORKSPACE}/{D}");
    let resource = CopyFile {
        source_path: resource_path.into(),
        target_path: "file:///D:/notes/Other/Assets/pic%252F+.png".into(),
        before: vec![1, 2, 3],
        kind: CopyFileKind::Resource,
        owner: None,
        typed_refs: vec![],
        definition_references: vec![],
        typed_refs_complete: true,
    };
    let mut input = copy_input(
        vec![
            copy_file(&source, "file:///D:/notes/copyA/copyA.typ"),
            resource,
        ],
        vec![source],
    );
    input.inventory.resources.push(ResourceSnapshot {
        key: old.clone(),
        path: resource_path.into(),
        revision_id: "r1".into(),
    });
    input.resource_map.insert(old, new);
    let plan = plan_copy(input).unwrap();
    let after = String::from_utf8(
        plan.files
            .iter()
            .find(|f| f.path.ends_with("copyA.typ"))
            .unwrap()
            .after
            .clone(),
    )
    .unwrap();
    assert!(after.contains("#image(\"../Other/Assets/pic%2F+.png\")"));
    assert!(plan.losses.is_empty());
}
#[test]
fn manual_suggestion_reconfirmation_preserves_history_and_restores_exact_basis() {
    let old = node(A, "A/A.typ", "原文", "A");
    let start = old.source.find("原文").unwrap();
    let mut r = record(AnnotationTarget::SourceRange {
        anchor: source_anchor(&old, start, start + 6, "原文".into()).unwrap(),
    });
    r.purpose = "suggestion".into();
    r.suggestion = Some(Suggestion {
        replacement: "replacement".into(),
        state: "pending".into(),
        confirmed_revision: old.revision_id.clone(),
        requires_reconfirmation: false,
    });
    let doc = document(vec![r]);
    let mut current = old.clone();
    current.source = current.source.replace("原文", "新文");
    current.revision_id = "new-current".into();
    assert!(suggestion_plan(&doc, "annotations", "annotation-1", &current).is_err());
    let target = AnnotationTarget::SourceRange {
        anchor: source_anchor(&current, start, start + 6, "新文".into()).unwrap(),
    };
    assert!(doc.reattach("annotation-1", target.clone()).is_err());
    let after = doc
        .reconfirm("annotation-1", target, "确认后的替换".into())
        .unwrap();
    let confirmed = decode_annotations(&after).unwrap();
    assert_eq!(confirmed.records[0].target_history.len(), 1);
    assert!(suggestion_plan(&confirmed, "annotations", "annotation-1", &current).is_ok());
}

#[test]
fn candidate_budget_keeps_whole_same_span_target_sets_and_reports_actual_stop() {
    let snapshot = inventory(vec![
        node(B, "B/B.typ", "", "山"),
        node(C, "C/C.typ", "", "山"),
    ]);
    let scan = scan_mentions_bounded(
        &snapshot,
        &owner(A),
        &[fragment("山山山"), fragment("山")],
        100,
        3,
    )
    .unwrap();
    assert_eq!(scan.coverage, Coverage::Partial);
    assert_eq!(scan.fragments[0].hits.len(), 1);
    assert_eq!(scan.fragments[0].hits[0].targets.len(), 2);
    assert_eq!(scan.fragments[0].scanned_scalars, 2);
    assert_eq!(
        scan.fragments[0].partial_reason.as_deref(),
        Some("candidateBudget")
    );
    assert_eq!(scan.fragments[1].scanned_scalars, 0);
    assert!(scan.fragments[1].hits.is_empty());
    let tiny = scan_mentions_bounded(&snapshot, &owner(A), &[fragment("山")], 100, 1).unwrap();
    assert!(tiny.fragments[0].hits.is_empty());
    assert_eq!(tiny.fragments[0].coverage, Coverage::Partial);
}
#[test]
fn file_uri_percent_hex_spelling_resolves_but_unqualified_case_alias_stays_partial() {
    let source = node(A, "A/A.typ", "#link(\"../b/B.typ\")[case alias]", "A");
    let target = node(B, "B/B.typ", "", "B");
    let snapshot = inventory(vec![source.clone(), target]);
    let index = link_index(&snapshot, &owner(B));
    assert_eq!(index.coverage, Coverage::Partial);
    assert!(index.incoming_path.is_empty());
    let chinese = node(C, "资料/资料.typ", "", "资料");
    let address = chinese.path.replace("%E8", "%e8").replace("%E6", "%e6");
    let snapshot = inventory(vec![source.clone(), chinese]);
    assert_eq!(
        resolve_link(&address, &source, &snapshot).target,
        Some(owner(C))
    );
}

#[test]
fn included_leaf_cannot_manufacture_a_root_writer_even_if_marked_writable() {
    let tree = weftext_engine::RenderNode {
        key: "included".into(),
        kind: "text".into(),
        tag: None,
        text: Some("山".into()),
        attrs: BTreeMap::new(),
        children: vec![],
        origin: Some(weftext_engine::Origin {
            path: "included.typ".into(),
            from: 0,
            to: 3,
            utf16_from: 0,
            utf16_to: 1,
            raw: "山".into(),
            syntax_kind: "Text".into(),
            is_main: false,
        }),
        writable: true,
    };
    let fragments = visible_fragments(&tree);
    assert_eq!(fragments.len(), 1);
    assert!(fragments[0].source_range.is_none());
}
#[test]
fn typed_resource_reference_has_its_own_resolved_projection() {
    let source = node(A, "A/A.typ", "正文", "A");
    let key = format!("weftext-resource://{WORKSPACE}/{B}");
    let mut snapshot = inventory(vec![source]);
    snapshot.resources.push(ResourceSnapshot {
        key: key.clone(),
        path: "file:///D:/notes/A/image.png".into(),
        revision_id: "resource-rev".into(),
    });
    let references = property_references(
        &snapshot,
        &[PropertyReferenceInput {
            source: owner(A),
            field_id: "avatar".into(),
            item_path: vec![],
            target: key,
        }],
    );
    assert_eq!(references[0].resolution.status, "resolved");
    assert!(references[0].resolution.resource.is_some());
    assert!(link_index(&snapshot, &owner(A)).outgoing.is_empty());
}

#[test]
fn resource_link_four_forms_use_existing_identity_and_real_root() {
    let key = format!("weftext-resource://{WORKSPACE}/{B}");
    let source = node(
        A,
        "A/A.typ",
        &format!("#link(\"{key}\")[*PDF display*]"),
        "A",
    );
    let resource = ResourceSnapshot {
        key: key.clone(),
        path: "file:///D:/notes/Assets/report%252F%2B.pdf".into(),
        revision_id: "pdf-r1".into(),
    };
    let mut snapshot = inventory(vec![source.clone()]);
    snapshot.resources.push(resource);
    let occurrence = native_links(&source.source)[0].from;
    for (form, address) in [
        (LinkForm::Id, key.as_str()),
        (LinkForm::Relative, "../Assets/report%252F%2B.pdf"),
        (
            LinkForm::Workspace,
            &format!("weftext-workspace://{WORKSPACE}/Assets/report%252F%2B.pdf"),
        ),
        (LinkForm::File, "file:///D:/notes/Assets/report%252F%2B.pdf"),
    ] {
        let patch = convert_link(&source, &snapshot, occurrence, form).unwrap();
        let link = &native_links(&patch.after)[0];
        assert_eq!(link.target, address);
        assert!(patch.after.ends_with("[*PDF display*]"));
        let result = resolve_link(address, &source, &snapshot);
        assert_eq!(result.status, "resolved");
        assert_eq!(result.resource, Some(key.clone()));
        assert!(result.target.is_none());
    }
}

#[test]
fn resource_fragment_requires_exact_reader_revision_and_preserves_display() {
    let key = format!("weftext-resource://{WORKSPACE}/{B}");
    let source = node(
        A,
        "A/A.typ",
        &format!("#link(\"{key}#page=2\")[page two]"),
        "A",
    );
    let mut snapshot = inventory(vec![source.clone()]);
    snapshot.resources.push(ResourceSnapshot {
        key: key.clone(),
        path: "file:///D:/notes/A/report.pdf".into(),
        revision_id: "actual-pdf-r1".into(),
    });
    let occurrence = native_links(&source.source)[0].from;
    assert!(convert_link(&source, &snapshot, occurrence, LinkForm::Relative).is_err());
    let mut qualification = ResourceFragmentQualification {
        resource: key.clone(),
        revision_id: "old-pdf".into(),
        fragment: "page=2".into(),
        status: ResourceFragmentStatus::Found,
    };
    assert!(
        convert_link_with_resource_fragments(
            &source,
            &snapshot,
            occurrence,
            LinkForm::Relative,
            std::slice::from_ref(&qualification)
        )
        .is_err()
    );
    qualification.revision_id = "actual-pdf-r1".into();
    for form in [
        LinkForm::Id,
        LinkForm::Relative,
        LinkForm::Workspace,
        LinkForm::File,
    ] {
        let patch = convert_link_with_resource_fragments(
            &source,
            &snapshot,
            occurrence,
            form,
            std::slice::from_ref(&qualification),
        )
        .unwrap();
        let target = &native_links(&patch.after)[0].target;
        let resolved = resolve_link_with_resource_fragments(
            target,
            &source,
            &snapshot,
            std::slice::from_ref(&qualification),
        );
        assert_eq!(resolved.resource, Some(key.clone()));
        assert_eq!(resolved.fragment.as_deref(), Some("page=2"));
        assert_eq!(resolved.status, "resolved");
        assert!(patch.after.ends_with("[page two]"));
    }
    qualification.status = ResourceFragmentStatus::Missing;
    assert!(
        convert_link_with_resource_fragments(
            &source,
            &snapshot,
            occurrence,
            LinkForm::Id,
            &[qualification]
        )
        .is_err()
    );
}

#[test]
fn resource_paths_outside_root_and_identity_collisions_do_not_gain_writers() {
    let key = format!("weftext-resource://{WORKSPACE}/{B}");
    let source = node(A, "A/A.typ", &format!("#link(\"{key}\")[resource]"), "A");
    let mut snapshot = inventory(vec![source.clone()]);
    snapshot.resources.push(ResourceSnapshot {
        key: key.clone(),
        path: "file:///E:/outside/report.pdf".into(),
        revision_id: "r1".into(),
    });
    let occurrence = native_links(&source.source)[0].from;
    assert!(convert_link(&source, &snapshot, occurrence, LinkForm::Relative).is_err());
    assert!(convert_link(&source, &snapshot, occurrence, LinkForm::Workspace).is_err());
    assert!(convert_link(&source, &snapshot, occurrence, LinkForm::File).is_ok());
    snapshot.resources[0].path = source.path.clone();
    assert_eq!(
        resolve_link(&source.path, &source, &snapshot).status,
        "ambiguous"
    );
    let mut unproven = source.clone();
    unproven.path = "file:///D:/other/Mismatch.typ".into();
    assert!(
        resource_address(
            &unproven,
            &snapshot.resources[0],
            None,
            LinkForm::Relative,
            &snapshot
        )
        .is_err()
    );
}

#[test]
fn custom_template_registration_rejects_forged_literal_slots_and_stale_revision() {
    let source = node(A, "A/A.typ", "#let amount=123\nText placeholder", "A");
    let number = source.source.find("123").unwrap();
    let numeric = TemplateSlot {
        name: "amount".into(),
        range: ExactRange::new(&source.source, number, number + 3).unwrap(),
        kind: TemplateSlotKind::Int,
        default: None,
        nullable: false,
    };
    assert!(validate_template_slots(&source.source, std::slice::from_ref(&numeric)).is_ok());
    let mut forged = numeric.clone();
    forged.kind = TemplateSlotKind::Markup;
    assert!(validate_template_slots(&source.source, &[forged]).is_err());
    let identity = source.source.find(&format!("\"{A}\"")).unwrap();
    let protected = TemplateSlot {
        name: "id".into(),
        range: ExactRange::new(&source.source, identity, identity + A.len() + 2).unwrap(),
        kind: TemplateSlotKind::String,
        default: None,
        nullable: false,
    };
    assert!(validate_template_slots(&source.source, &[protected]).is_err());
    let mut definition = TemplateDefinition {
        id: "custom-template".into(),
        title: "Reusable".into(),
        description: "".into(),
        source_node: source.owner.clone(),
        source_revision: source.revision_id.clone(),
        slots: vec![numeric],
    };
    assert!(validate_template_definition(&definition, &source).is_ok());
    definition.source_revision = "old-revision".into();
    assert!(validate_template_definition(&definition, &source).is_err());
}

#[test]
fn saved_query_refs_bind_actual_nested_controls_without_rewriting_cel_or_text() {
    let current = format!("weftext-node://{WORKSPACE}/{B}");
    let pinned = format!("weftext-node://{WORKSPACE}/{C}");
    let current_plan = format!(
        "(scope: (directory: \"{current}\"), advanced: (call: (owner: \"{current}\", selection: (mode: \"current\", key: \"{C}\")),))"
    );
    let pinned_plan = format!(
        "(advanced: (call: (owner: \"{pinned}\", selection: (mode: \"pinned\", key: \"{D}\", revision: \"historical-r1\")),))"
    );
    let related_plan = format!("(advanced: (related: (relation: \"test\", from: {pinned_plan}),))");
    let advanced = format!("advanced: (unionAll: ({current_plan}, {related_plan},))");
    let declaration = query_declaration(B).replace("where: \"true\"", &advanced);
    let source = node(
        A,
        "A/A.typ",
        &format!("{declaration}\n\nThis literal {current} stays text."),
        "A",
    );
    let slots = saved_query_reference_slots(
        &source.source,
        &weftext_data::Schema::bundled().unwrap(),
        &source.owner,
    )
    .unwrap();
    assert_eq!(slots.typed_refs.len(), 3);
    assert_eq!(slots.definition_references.len(), 2);
    assert!(slots.definition_references[0].revision.is_none());
    assert_eq!(
        slots.definition_references[1].revision.as_deref(),
        Some("historical-r1")
    );
    for reference in &slots.definition_references {
        assert!(reference.owner_slot.verify(&source.source));
        assert!(reference.key_slot.verify(&source.source));
    }
    assert!(
        slots
            .typed_refs
            .iter()
            .all(|s| s.to < source.source.find("This literal").unwrap())
    );
    assert!(
        saved_query_reference_slots(
            &source.source,
            &weftext_data::Schema::bundled().unwrap(),
            &owner(B)
        )
        .is_err()
    );
}

#[test]
fn precise_unknown_json_numbers_are_never_rounded_by_annotation_edits() {
    for unknown in ["18446744073709551617", "0.10000000000000001"] {
        let bytes = format!("{{\"version\":1,\"annotations\":[],\"future\":{unknown}}}");
        let error = decode_annotations(bytes.as_bytes()).unwrap_err();
        assert_eq!(error.code, "unsupportedAnnotations");
    }
    for unknown in [
        "18446744073709551615",
        "100000000000000000000",
        "0.1",
        "1.00e2",
    ] {
        let bytes = format!(
            "{{\"version\":1,\"annotations\":[],\"future\":{unknown},\"text\":\"18446744073709551617\"}}"
        );
        assert!(decode_annotations(bytes.as_bytes()).is_ok());
    }
}

#[test]
fn unqualified_generated_links_never_produce_a_complete_absence_claim() {
    for body in [
        "#link(destination)[computed]",
        "#include \"other.typ\"",
        "#let link(x)=x\n#link(\"https://example.test\")",
        "#let produce()=none\n#produce()",
    ] {
        let snapshot = inventory(vec![node(A, "A/A.typ", body, "A")]);
        let result = link_index(&snapshot, &owner(B));
        assert_eq!(result.coverage, Coverage::Partial);
        assert!(result.incoming_id.is_empty());
        assert!(result.incoming_path.is_empty());
    }
}

#[test]
fn link_index_budgets_keep_confirmed_edges_and_prioritize_selected_source() {
    let other = node(
        B,
        "B/B.typ",
        &format!(
            "#link(\"{}\")[one] #link(\"{}\")[two]",
            id_link(A),
            id_link(A)
        ),
        "B",
    );
    let selected = node(
        A,
        "A/A.typ",
        &format!("#link(\"{}\")[outgoing]", id_link(B)),
        "A",
    );
    let snapshot = inventory(vec![other, selected]);
    let result = link_index_bounded(&snapshot, &owner(A), 2, 1_000_000);
    assert_eq!(result.coverage, Coverage::Partial);
    assert_eq!(result.outgoing.len(), 1);
    assert_eq!(result.incoming_id.len(), 1);
    let tiny = link_index_bounded(&snapshot, &owner(A), 100, 1);
    assert_eq!(tiny.coverage, Coverage::Partial);
    assert!(tiny.outgoing.is_empty());
    assert!(tiny.incoming_id.is_empty());
    let complete = link_index_bounded(&snapshot, &owner(A), 3, 1_000_000);
    assert_eq!(complete.coverage, Coverage::Complete);
    assert_eq!(complete.incoming_id.len(), 2);
}

#[test]
fn uri_schemes_are_ascii_case_insensitive_without_normalizing_identity_or_path() {
    let source = node(
        A,
        "A/A.typ",
        &format!("#link(\"WEFTEXT-NODE://{WORKSPACE}/{B}\")[display]"),
        "A",
    );
    let snapshot = inventory(vec![source.clone(), node(B, "B/B.typ", "", "B")]);
    assert_eq!(
        resolve_link(
            &format!("WEFTEXT-NODE://{WORKSPACE}/{B}"),
            &source,
            &snapshot
        )
        .target,
        Some(owner(B))
    );
    assert_eq!(
        resolve_link(
            &format!("WeFtExT-WoRkSpAcE://{WORKSPACE}/B/B.typ"),
            &source,
            &snapshot
        )
        .target,
        Some(owner(B))
    );
    assert_eq!(
        resolve_link("FILE:///D:/notes/B/B.typ", &source, &snapshot).target,
        Some(owner(B))
    );
    assert_eq!(
        resolve_link("HTTPS://example.test/path#exact", &source, &snapshot).status,
        "external"
    );
    let occurrence = native_links(&source.source)[0].from;
    assert!(
        convert_link(&source, &snapshot, occurrence, LinkForm::Relative)
            .unwrap()
            .after
            .ends_with("[display]")
    );
    assert_ne!(
        resolve_link(
            &format!("WEFTEXT-NODE://{WORKSPACE}/{}", B.to_ascii_uppercase()),
            &source,
            &snapshot
        )
        .status,
        "resolved"
    );
    assert_ne!(
        resolve_link(
            &format!("WEFTEXT-WORKSPACE://{WORKSPACE}/b/B.typ"),
            &source,
            &snapshot
        )
        .status,
        "resolved"
    );
}

#[test]
fn native_document_title_template_uses_the_proven_markup_role() {
    let source = "#set document(title: [标题])\n#title()\n\n正文";
    let start = source.find("标题").unwrap();
    let slot = TemplateSlot {
        name: "title".into(),
        range: ExactRange::new(source, start, start + "标题".len()).unwrap(),
        kind: TemplateSlotKind::Markup,
        default: None,
        nullable: false,
    };
    let title = "A #title() [literal]";
    let after = instantiate_template(
        source,
        std::slice::from_ref(&slot),
        &BTreeMap::from([("title".into(), TemplateValue::Text(title.into()))]),
    )
    .unwrap();
    let role = weftext_engine::inspect_editor(&after).title;
    assert_eq!(role.status, "writable");
    assert_eq!(role.text.as_deref(), Some(title));
    assert!(after.ends_with("\n#title()\n\n正文"));
    let ambiguous = format!("{source}\n#set document(title: [another])");
    assert!(validate_template_slots(&ambiguous, std::slice::from_ref(&slot)).is_err());
    let computed = "#set document(title: [#value])\n#title()";
    let start = computed.find("value").unwrap();
    let mut unknown = slot;
    unknown.range = ExactRange::new(computed, start, start + 5).unwrap();
    assert!(validate_template_slots(computed, &[unknown]).is_err());
}

#[test]
fn default_literal_document_title_certifies_complete_link_absence() {
    let source = node(
        A,
        "A/A.typ",
        "#set document(title: [标题])\n#title()\n\n正文",
        "标题",
    );
    let links = native_links(&source.source);
    assert!(source_link_projection_complete(&source.source, &links));
    let index = link_index(&inventory(vec![source]), &owner(A));
    assert_eq!(index.coverage, Coverage::Complete);
    assert!(index.outgoing.is_empty());
    for body in [
        "#set document(title: [#value])\n#title()",
        "#let title() = none\n#set document(title: [标题])\n#title()",
        "#set document(title: [标题])\n#show title: it => it\n#title()",
        "#import \"other.typ\": *\n#set document(title: [标题])\n#title()",
        "#set document(title: [标题])\n#title(..options)",
        "#set document(title: [标题])\n#set title(body: [alternate])\n#title()",
        "#set document(title: [https://example.test])\n#title()",
    ] {
        let source = node(A, "A/A.typ", body, "标题");
        assert!(
            !source_link_projection_complete(&source.source, &native_links(&source.source)),
            "{body}"
        );
        assert_eq!(
            link_index(&inventory(vec![source]), &owner(A)).coverage,
            Coverage::Partial
        );
    }
}
