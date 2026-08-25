use weftext_asciidoc::{
    AnalysisStatus, EnvelopeAdjacentHeadingBody, EnvelopeChildSort, EnvelopeFieldKind,
    EnvelopeFieldValue, EnvelopeIssueCode, EnvelopeIssueSeverity, EnvelopeProbeState, LinkKind,
    ManagedEnvelopePatch, ManagedEnvelopePatchError, analyze, analyze_managed_envelope,
    new_managed_document_envelope, patch_managed_envelope, probe_managed_envelope,
};

const CANONICAL: &str = include_str!("fixtures/canonical-envelope-v1.adoc");
const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn exact<'a>(source: &'a str, range: &std::ops::Range<u64>) -> &'a str {
    let start = usize::try_from(range.start).expect("range start");
    let end = usize::try_from(range.end).expect("range end");
    assert!(source.is_char_boundary(start));
    assert!(source.is_char_boundary(end));
    &source[start..end]
}

#[test]
fn canonical_shallow_envelope_has_typed_fields_and_exact_ranges() {
    let analysis = analyze(CANONICAL);
    let envelope = analysis.envelope.as_ref().expect("closed envelope");

    assert!(envelope.valid, "unexpected issues: {:#?}", envelope.issues);
    assert!(envelope.issues.is_empty());
    let envelope_end = usize::try_from(envelope.range.end).expect("envelope end");
    assert_eq!(
        exact(CANONICAL, &envelope.range),
        &CANONICAL[..envelope_end]
    );
    assert_eq!(
        exact(
            CANONICAL,
            envelope
                .weftext_key_range
                .as_ref()
                .expect("weftext key range")
        ),
        "weftext"
    );
    assert_eq!(envelope.fields.len(), 7);

    for field in &envelope.fields {
        assert_eq!(exact(CANONICAL, &field.key_range), field.name);
        let _ = exact(CANONICAL, &field.range);
        let _ = exact(CANONICAL, &field.value_range);
    }

    let id = envelope
        .fields
        .iter()
        .find(|field| field.kind == EnvelopeFieldKind::Id)
        .expect("id field");
    assert!(matches!(
        &id.value,
        EnvelopeFieldValue::Scalar { value } if value == UUID
    ));
    assert_eq!(exact(CANONICAL, &id.value_range), format!("\"{UUID}\""));

    let aliases = envelope
        .fields
        .iter()
        .find(|field| field.kind == EnvelopeFieldKind::Aliases)
        .expect("aliases field");
    let EnvelopeFieldValue::StringList { items } = &aliases.value else {
        panic!("aliases must be a typed list");
    };
    assert_eq!(
        items
            .iter()
            .map(|item| item.value.as_str())
            .collect::<Vec<_>>(),
        ["文缕", "Weftext Notes"]
    );
    assert_eq!(exact(CANONICAL, &items[0].value_range), "文缕");
    assert_eq!(exact(CANONICAL, &items[1].value_range), "\"Weftext Notes\"");
}

#[test]
fn retired_and_ambiguous_yaml_shapes_are_rejected_without_legacy_fallback() {
    let cases = [
        (
            format!("---\n_weftext:\n  id: \"{UUID}\"\n---\n"),
            EnvelopeIssueCode::LegacyTopLevelKey,
        ),
        (
            format!("---\nweftext:\n  id: \"{UUID}\"\nreference:\n  key: retired\n---\n"),
            EnvelopeIssueCode::UnknownTopLevelKey,
        ),
        (
            format!("---\nweftext:\n  id: \"{UUID}\"\n  icon: 😀\n  icon: 😺\n---\n"),
            EnvelopeIssueCode::DuplicateField,
        ),
        (
            format!("---\nweftext:\n  id: \"{UUID}\"\n  icon: [😀, 😺]\n---\n"),
            EnvelopeIssueCode::InvalidValue,
        ),
        (
            format!("---\nweftext:\n  id: \"{UUID}\"\n  icon:\n    foreground: 😀\n---\n"),
            EnvelopeIssueCode::InvalidStructure,
        ),
        (
            format!("---\nweftext:\n  id: &identity \"{UUID}\"\n---\n"),
            EnvelopeIssueCode::UnsafeYamlFeature,
        ),
    ];

    for (source, expected) in cases {
        let analysis = analyze(&source);
        let envelope = analysis.envelope.as_ref().expect("closed envelope");
        assert!(!envelope.valid, "{source:?} was accepted");
        let issue = envelope
            .issues
            .iter()
            .find(|issue| issue.code == expected)
            .unwrap_or_else(|| {
                panic!(
                    "missing {expected:?} for {source:?}: {:#?}",
                    envelope.issues
                )
            });
        assert_eq!(issue.severity, EnvelopeIssueSeverity::Error);
        let _ = exact(&source, &issue.range);
        assert_ne!(analysis.status, AnalysisStatus::Complete);
    }
}

#[test]
fn unknown_inner_field_is_preserved_as_opaque_forward_compatibility_evidence() {
    let source = format!(
        "---\nweftext:\n  id: \"{UUID}\"\n  future_profile:\n    nested: [opaque, bytes]\n---\n= Title\n"
    );
    let analysis = analyze(&source);
    let envelope = analysis.envelope.as_ref().expect("closed envelope");
    let future = envelope
        .fields
        .iter()
        .find(|field| field.kind == EnvelopeFieldKind::Unknown)
        .expect("unknown field evidence");

    assert!(envelope.valid, "unexpected issues: {:#?}", envelope.issues);
    assert!(matches!(future.value, EnvelopeFieldValue::Opaque));
    assert_eq!(exact(&source, &future.key_range), "future_profile");
    assert!(exact(&source, &future.range).contains("nested: [opaque, bytes]"));
    assert!(envelope.issues.iter().any(|issue| {
        issue.code == EnvelopeIssueCode::UnknownWeftextField
            && issue.severity == EnvelopeIssueSeverity::Warning
            && issue.range == future.key_range
    }));
}

#[test]
fn cjk_crlf_ranges_are_utf8_exact_and_envelope_text_stays_protected() {
    let source = format!(
        "---\r\nweftext:\r\n  id: \"{UUID}\"\r\n  icon: '😀'\r\n  aliases:\r\n    - 文缕\r\n    - 'node:{UUID}[受保护]'\r\n    - 'include::https://example.invalid/private.adoc[]'\r\n---\r\n= 标题 😀\r\n\r\n正文 node:{UUID}[可见]。\r\n"
    );
    let analysis = analyze(&source);
    let envelope = analysis.envelope.as_ref().expect("closed envelope");

    assert!(envelope.valid, "unexpected issues: {:#?}", envelope.issues);
    for field in &envelope.fields {
        let _ = exact(&source, &field.range);
        let _ = exact(&source, &field.key_range);
        let _ = exact(&source, &field.value_range);
        if let EnvelopeFieldValue::StringList { items } = &field.value {
            for item in items {
                let _ = exact(&source, &item.range);
                let _ = exact(&source, &item.value_range);
            }
        }
    }
    assert!(
        analysis.protected_ranges.iter().any(|range| {
            range.start == envelope.range.start && range.end == envelope.range.end
        })
    );
    assert_eq!(
        analysis
            .links
            .iter()
            .filter(|link| link.kind == LinkKind::Node)
            .count(),
        1,
        "node macro text inside frontmatter must not become a link"
    );
    assert!(!analysis.diagnostics.iter().any(|diagnostic| {
        (diagnostic.message.contains("include expansion")
            || diagnostic.message.contains("remote URI loading"))
            && diagnostic.range.start < envelope.range.end
    }));
}

#[test]
fn delimiter_probe_is_exact_and_rejects_bom_or_nonleading_envelopes() {
    let closed = format!("---\r\nweftext:\r\n  id: \"{UUID}\"\r\n---\r\n= 标题\r\n");
    let probe = probe_managed_envelope(&closed);
    assert_eq!(probe.state, EnvelopeProbeState::Closed);
    let expected_body_start = closed.find("= 标题").unwrap();
    assert_eq!(
        exact(&closed, probe.range.as_ref().unwrap()),
        &closed[..expected_body_start]
    );
    assert_eq!(probe.body_start, expected_body_start as u64);

    let unclosed = format!("---\nweftext:\n  id: \"{UUID}\"");
    let probe = probe_managed_envelope(&unclosed);
    assert_eq!(probe.state, EnvelopeProbeState::Unclosed);
    assert_eq!(probe.range, Some(0..unclosed.len() as u64));
    assert_eq!(probe.body_start, unclosed.len() as u64);

    for source in [
        format!("\u{feff}---\nweftext:\n  id: \"{UUID}\"\n---\n"),
        format!(" \n---\nweftext:\n  id: \"{UUID}\"\n---\n"),
    ] {
        assert_eq!(
            probe_managed_envelope(&source).state,
            EnvelopeProbeState::Absent
        );
        assert!(analyze_managed_envelope(&source).semantic.is_none());
    }
}

#[test]
fn identity_is_required_and_unsafe_unknown_yaml_fails_closed() {
    let missing = "---\nweftext:\n  icon: 😀\n---\n";
    let envelope = analyze_managed_envelope(missing)
        .semantic
        .expect("closed envelope");
    assert!(!envelope.valid);
    assert!(envelope.issues.iter().any(|issue| {
        issue.code == EnvelopeIssueCode::MissingRequiredField
            && issue.message.contains("weftext.id is required")
    }));

    let unsafe_unknown =
        format!("---\nweftext:\n  id: \"{UUID}\"\n  future:\n    nested: [opaque, *shared]\n---\n");
    let envelope = analyze_managed_envelope(&unsafe_unknown)
        .semantic
        .expect("closed envelope");
    assert!(!envelope.valid);
    assert!(
        envelope
            .issues
            .iter()
            .any(|issue| issue.code == EnvelopeIssueCode::UnsafeYamlFeature)
    );
}

#[test]
fn typed_patches_are_narrow_reparse_and_preserve_unknown_cjk_crlf_bytes() {
    let source = format!(
        "---\r\nweftext:\r\n  id: \"{UUID}\"\r\n  aliases: [old, \"旧名\"]\r\n  future:\r\n    nested: [opaque, bytes]\r\n  icon: '😀'\r\n# keep exact 注释\r\n---\r\n= 标题\r\n\r\n正文 🧠\r\n"
    );
    let icon = patch_managed_envelope(
        &source,
        ManagedEnvelopePatch::Icon(Some("weftext:book".to_owned())),
    )
    .expect("icon patch");
    assert_eq!(
        icon,
        source.replacen("  icon: '😀'", "  icon: \"weftext:book\"", 1)
    );

    let aliases = patch_managed_envelope(
        &icon,
        ManagedEnvelopePatch::Aliases(vec!["文缕".to_owned(), "Weftext Notes".to_owned()]),
    )
    .expect("aliases patch");
    assert!(
        aliases.contains("  aliases:\r\n    - \"文缕\"\r\n    - \"Weftext Notes\"\r\n  future:")
    );
    assert!(aliases.contains("  future:\r\n    nested: [opaque, bytes]\r\n"));
    assert!(aliases.contains("# keep exact 注释\r\n---\r\n= 标题\r\n\r\n正文 🧠\r\n"));
    assert!(
        analyze_managed_envelope(&aliases)
            .semantic
            .is_some_and(|envelope| envelope.valid)
    );

    let sorted = patch_managed_envelope(
        &aliases,
        ManagedEnvelopePatch::ChildSort(Some(EnvelopeChildSort::Manual)),
    )
    .expect("sort patch");
    let presented = patch_managed_envelope(
        &sorted,
        ManagedEnvelopePatch::AdjacentHeadingBody(Some(EnvelopeAdjacentHeadingBody::RunIn)),
    )
    .expect("presentation patch");
    assert!(presented.contains("  child_sort: manual\r\n"));
    assert!(presented.contains("  adjacent_heading_body: run_in\r\n"));

    let cleared =
        patch_managed_envelope(&presented, ManagedEnvelopePatch::Icon(None)).expect("remove icon");
    assert!(!cleared.contains("  icon:"));
    assert!(cleared.contains("# keep exact 注释\r\n"));
}

#[test]
fn typed_patch_refuses_invalid_values_and_invalid_source() {
    let source = format!("---\nweftext:\n  id: \"{UUID}\"\n---\n");
    assert_eq!(
        patch_managed_envelope(
            &source,
            ManagedEnvelopePatch::Icon(Some("vendor:custom".to_owned()))
        ),
        Err(ManagedEnvelopePatchError::InvalidValue)
    );
    assert_eq!(
        patch_managed_envelope(&source, ManagedEnvelopePatch::Icon(Some("😀😺".to_owned()))),
        Err(ManagedEnvelopePatchError::InvalidValue)
    );
    assert_eq!(
        patch_managed_envelope(
            &source,
            ManagedEnvelopePatch::Aliases(vec!["same".to_owned(), "same".to_owned()])
        ),
        Err(ManagedEnvelopePatchError::InvalidValue)
    );
    let duplicate =
        format!("---\nweftext:\n  id: \"{UUID}\"\n  sibling_rank: 1\n  sibling_rank: 2\n---\n");
    assert_eq!(
        patch_managed_envelope(&duplicate, ManagedEnvelopePatch::SiblingRank(Some(3))),
        Err(ManagedEnvelopePatchError::InvalidEnvelope)
    );
}

#[test]
fn minimal_envelope_creation_uses_the_same_profile_authority() {
    let id = uuid::Uuid::parse_str(UUID).unwrap();
    let source = new_managed_document_envelope(id).expect("UUIDv4 envelope");
    assert_eq!(source, format!("---\nweftext:\n  id: \"{UUID}\"\n---\n"));
    assert!(
        analyze_managed_envelope(&source)
            .semantic
            .is_some_and(|envelope| envelope.valid)
    );
}
