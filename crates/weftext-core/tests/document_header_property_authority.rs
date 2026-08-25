use weftext_asciidoc::{analyze_document_header, patch_document_header_attribute};
use weftext_core::{
    DocumentPropertyDiagnosticCode, analyze_document_header_properties,
    patch_document_header_property,
};

#[test]
fn core_consumes_the_profile_header_projection_without_a_second_syntax_interpretation() {
    let source = concat!(
        "---\r\n",
        "weftext:\r\n",
        "  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n",
        "---\r\n",
        "= 标题\r\n",
        ":status: 草稿 😀\r\n",
        ":toc: left\r\n",
        ":wrapped: first \\\r\n",
        "第二行\r\n",
        "\r\n",
        "正文\r\n",
        ":status: body-only\r\n"
    );
    let profile = analyze_document_header(source);
    let core = analyze_document_header_properties(source);

    assert_eq!(core.header_range, profile.range);
    let projected = profile
        .attributes
        .iter()
        .filter(|attribute| attribute.projected)
        .collect::<Vec<_>>();
    assert_eq!(core.properties.len(), projected.len());
    for (property, attribute) in core.properties.iter().zip(projected) {
        assert_eq!(property.name, attribute.name);
        assert_eq!(property.value, attribute.literal_value.as_deref().unwrap());
        assert_eq!(property.range, attribute.range);
        assert_eq!(property.name_range, attribute.name_range);
        assert_eq!(property.value_range, attribute.value_range);
    }
    assert!(core.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DocumentPropertyDiagnosticCode::ProcessorControl
            && diagnostic.name.as_deref() == Some("toc")
    }));
    assert!(core.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DocumentPropertyDiagnosticCode::ContinuedValue
            && diagnostic.name.as_deref() == Some("wrapped")
    }));
}

#[test]
fn core_and_profile_use_the_same_narrow_patch_plan() {
    let source = "= Title\n:status: old\n\n正文\n:status: body\n";
    let core = patch_document_header_property(source, "status", Some("新值")).expect("Core patch");
    let profile =
        patch_document_header_attribute(source, "status", Some("新值")).expect("Profile patch");
    assert_eq!(core, profile);
    assert_eq!(core, "= Title\n:status: 新值\n\n正文\n:status: body\n");
}
