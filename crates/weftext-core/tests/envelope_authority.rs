use weftext_asciidoc::{
    EnvelopeFieldKind, EnvelopeProbeState, ManagedEnvelopePatch, analyze_managed_envelope,
    patch_managed_envelope, probe_managed_envelope,
};
use weftext_core::{
    DocumentEnvelopeState, FrontmatterError, NodeMetadataScope, parse_node_metadata,
    parse_node_metadata_with_diagnostics, patch_node_icon_property, probe_document_envelope,
    project_node_metadata,
};

const ID: &str = "550e8400-e29b-41d4-a716-446655440000";

#[test]
fn core_delimiter_probe_is_exactly_the_profile_probe() {
    for source in [
        String::new(),
        format!("---\nweftext:\n  id: \"{ID}\"\n---\n= Title\n"),
        format!("---\r\nweftext:\r\n  id: \"{ID}\"\r\n---\r\n= 标题\r\n"),
        format!("---\nweftext:\n  id: \"{ID}\""),
        format!("\u{feff}---\nweftext:\n  id: \"{ID}\"\n---\n"),
        format!(" \n---\nweftext:\n  id: \"{ID}\"\n---\n"),
    ] {
        let profile = probe_managed_envelope(&source);
        let core = probe_document_envelope(&source);
        assert_eq!(core.state, profile.state);
        assert_eq!(core.range, profile.range);
        assert_eq!(core.content_range, profile.content_range);
        assert_eq!(core.body_start, profile.body_start);
    }
}

#[test]
fn runtime_acceptance_is_the_profile_acceptance_without_legacy_fallback() {
    let cases = [
        format!("---\nweftext:\n  id: \"{ID}\"\n---\n"),
        format!("---\nweftext:\n  id: \"{ID}\"\n  future:\n    exact: [kept, bytes]\n---\n"),
        "---\nweftext:\n  icon: 😀\n---\n".to_owned(),
        format!("---\n_weftext:\n  id: \"{ID}\"\n---\n"),
        format!("---\nweftext:\n  id: \"{ID}\"\nreference: {{}}\n---\n"),
        format!("---\nweftext:\n  id: \"{ID}\"\n  icon: [😀, 😺]\n---\n"),
        format!("---\nweftext:\n  id: &identity \"{ID}\"\n---\n"),
        format!("\u{feff}---\nweftext:\n  id: \"{ID}\"\n---\n"),
    ];

    for source in cases {
        let profile = analyze_managed_envelope(&source);
        let profile_accepts = profile
            .semantic
            .as_ref()
            .is_some_and(|envelope| envelope.valid);
        assert_eq!(
            parse_node_metadata(&source).is_ok(),
            profile_accepts,
            "cross-layer drift for {source:?}"
        );
    }

    assert_eq!(
        parse_node_metadata("---\nweftext:\n  icon: 😀\n---\n"),
        Err(FrontmatterError::MissingIdentity)
    );
    assert_eq!(
        parse_node_metadata("---\nweftext: []\n---\n"),
        Err(FrontmatterError::UnsupportedReservedYaml)
    );
    assert_eq!(
        probe_document_envelope(&format!("\u{feff}---\nweftext:\n  id: \"{ID}\"\n---\n")).state,
        DocumentEnvelopeState::Absent
    );
    assert_eq!(
        probe_managed_envelope("---\nweftext:\n").state,
        EnvelopeProbeState::Unclosed
    );
}

#[test]
fn unknown_field_diagnostic_range_is_profile_owned_and_root_rule_stays_in_core() {
    let source = format!(
        "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  future_字段:\r\n    exact: [保留, bytes]\r\n  adjacent_heading_body: run_in\r\n---\r\n= 标题\r\n"
    );
    // Non-ASCII YAML keys are deliberately not canonical mapping names.
    assert!(parse_node_metadata_with_diagnostics(&source).is_err());

    let source = format!(
        "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  future_field:\r\n    exact: [保留, bytes]\r\n  adjacent_heading_body: run_in\r\n---\r\n= 标题\r\n"
    );
    let profile = analyze_managed_envelope(&source)
        .semantic
        .expect("closed envelope");
    let profile_unknown = profile
        .fields
        .iter()
        .find(|field| field.kind == EnvelopeFieldKind::Unknown)
        .unwrap();
    let (_, diagnostics) = parse_node_metadata_with_diagnostics(&source).unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].range, profile_unknown.key_range);
    assert_eq!(diagnostics[0].field, profile_unknown.name);

    assert!(project_node_metadata(&source, NodeMetadataScope::WorkspaceRoot).is_ok());
    assert_eq!(
        project_node_metadata(&source, NodeMetadataScope::Node),
        Err(FrontmatterError::WorkspaceSettingOutsideRoot)
    );
}

#[test]
fn core_icon_action_is_the_profile_typed_patch_and_preserves_exact_bytes() {
    let source = format!(
        "---\r\nweftext:\r\n  id: \"{ID}\"\r\n  future:\r\n    exact: [保留, bytes]\r\n  icon: '😀'\r\n---\r\n= 标题\r\n\r\n正文\r\n"
    );
    let profile = patch_managed_envelope(
        &source,
        ManagedEnvelopePatch::Icon(Some("weftext:book".to_owned())),
    )
    .unwrap();
    let core = patch_node_icon_property(&source, Some("weftext:book")).unwrap();
    assert_eq!(core, profile);
    assert_eq!(
        core,
        source.replacen("  icon: '😀'", "  icon: \"weftext:book\"", 1)
    );
}
