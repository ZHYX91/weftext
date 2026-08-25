use weftext_asciidoc::{
    AnalysisStatus, BlockKind, BlockSemantic, InlineKind, MathNotation, SEMANTIC_MODEL_VERSION,
    analyze,
};

const PROFILE: &str = include_str!("fixtures/profile-v1-semantics.adoc");

#[test]
#[allow(clippy::too_many_lines)]
fn profile_fixture_has_lossless_typed_semantics_and_safe_render_input() {
    let analysis = analyze(PROFILE);

    assert_eq!(analysis.semantic_model_version, SEMANTIC_MODEL_VERSION);
    assert_eq!(analysis.status, AnalysisStatus::Degraded);
    assert!(
        !analysis
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Other),
        "fixture unexpectedly fell through to Other: {:#?}",
        analysis.blocks
    );

    let heading = analysis
        .blocks
        .iter()
        .find(|block| block.block_id.as_deref() == Some("intro"))
        .expect("anchored heading");
    assert_eq!(heading.roles, ["run-in"]);
    assert!(matches!(
        heading.semantic,
        BlockSemantic::Heading { level: 1 }
    ));

    let list = analysis
        .blocks
        .iter()
        .find_map(|block| match &block.semantic {
            BlockSemantic::List { model } if model.depth == 1 => Some(model),
            _ => None,
        })
        .expect("typed list");
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].checked, Some(false));
    assert_eq!(list.items[0].children.len(), 1);
    assert_eq!(list.items[0].children[0].checked, Some(true));

    let table = analysis
        .blocks
        .iter()
        .find_map(|block| match &block.semantic {
            BlockSemantic::Table { model } => Some(model),
            _ => None,
        })
        .expect("typed table");
    assert_eq!(table.column_count, 2);
    assert_eq!(
        table.header.as_ref().expect("table header").cells[0].text,
        "Name"
    );
    assert_eq!(table.body.len(), 1);
    assert_eq!(table.body[0].cells[0].text, "Alpha");

    let image = analysis
        .blocks
        .iter()
        .find(|block| matches!(block.semantic, BlockSemantic::Image { .. }))
        .expect("typed image");
    assert_eq!(image.title.as_deref(), Some("Architecture"));
    assert!(matches!(
        &image.semantic,
        BlockSemantic::Image { target, alt }
            if target == "architecture.svg" && alt.as_deref() == Some("Architecture diagram")
    ));

    let listing = analysis
        .blocks
        .iter()
        .find(|block| matches!(block.semantic, BlockSemantic::Listing { .. }))
        .expect("typed listing");
    assert_eq!(listing.title.as_deref(), Some("Example"));
    assert!(matches!(
        &listing.semantic,
        BlockSemantic::Listing { language } if language.as_deref() == Some("rust")
    ));

    assert!(analysis.blocks.iter().any(|block| {
        block.title.as_deref() == Some("Energy")
            && matches!(
                block.semantic,
                BlockSemantic::Math {
                    notation: MathNotation::LatexMath
                }
            )
    }));
    assert!(analysis.blocks.iter().any(|block| {
        block.title.as_deref() == Some("Process")
            && matches!(block.semantic, BlockSemantic::Mermaid)
    }));

    for kind in [
        InlineKind::Anchor,
        InlineKind::Xref,
        InlineKind::NativeLink,
        InlineKind::Footnote,
        InlineKind::LatexMath,
        InlineKind::Bold,
        InlineKind::Italic,
        InlineKind::Monospace,
        InlineKind::RoleSpan,
        InlineKind::Subscript,
        InlineKind::Superscript,
    ] {
        assert!(
            analysis.inlines.iter().any(|inline| inline.kind == kind),
            "missing {kind:?}: {:#?}",
            analysis.inlines
        );
    }

    assert!(analysis.safe_html.contains("<table"));
    assert!(analysis.safe_html.contains("<ul"));
    assert!(analysis.safe_html.contains("data-weftext-image-target"));
    assert!(!analysis.safe_html.contains("<img"));
    assert!(!analysis.safe_html.contains(" href="));
    assert!(!analysis.safe_html.contains("<safe>"));
    assert!(analysis.safe_html.contains("&lt;safe&gt;"));
}

#[test]
fn malformed_inline_and_delimited_constructs_are_reported_without_panicking() {
    let source = "= Broken\n\nxref:missing[close\n\n[mermaid]\n....\nflowchart LR\n";
    let result = std::panic::catch_unwind(|| analyze(source));
    let analysis = result.expect("public analysis boundary must contain parser panics");

    assert_ne!(analysis.status, AnalysisStatus::Complete);
    assert!(!analysis.diagnostics.is_empty());
    assert!(!analysis.degradations.is_empty());
    assert!(analysis.safe_html.contains("data-analysis-status"));
}

#[test]
fn prohibited_effects_have_typed_degradation_reports_and_escaped_fallbacks() {
    let source =
        "include::https://example.invalid/private.adoc[]\n\n++++\n<script>x</script>\n++++\n";
    let analysis = analyze(source);

    assert_eq!(analysis.status, AnalysisStatus::Degraded);
    assert!(
        analysis
            .degradations
            .iter()
            .any(|item| { item.support_state == weftext_asciidoc::SupportState::ProhibitedEffect })
    );
    assert!(!analysis.safe_html.contains("<script>"));
    assert!(analysis.safe_html.contains("&lt;script&gt;"));
}

#[test]
fn unmodeled_native_macros_are_reported_instead_of_silently_flattened() {
    let source = "Press kbd:[Ctrl+C] to stop.\n";
    let analysis = analyze(source);

    let unsupported = analysis
        .inlines
        .iter()
        .find(|inline| inline.kind == InlineKind::Unsupported)
        .expect("keyboard macro must remain a typed unsupported inline");
    assert_eq!(
        &source[usize::try_from(unsupported.range.start).expect("range start")
            ..usize::try_from(unsupported.range.end).expect("range end")],
        "kbd:[Ctrl+C]"
    );
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == weftext_asciidoc::DiagnosticCode::UnsupportedProfileSyntax
            && diagnostic.range == unsupported.range
    }));
    assert!(analysis.degradations.iter().any(|degradation| {
        degradation.kind == weftext_asciidoc::DegradationKind::UnsupportedInline
            && degradation.range == unsupported.range
    }));
}
