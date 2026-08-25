use weftext_asciidoc::{
    AdjacentHeadingBodyDefault, AdjacentHeadingBodyEligibility, AdjacentHeadingBodyPresentation,
    AdjacentHeadingBodyRule, AnalysisStatus, BlockKind, BlockSemantic, DiagnosticCode,
    EffectCapability, EffectDecision, EffectOrigin, analyze, analyze_with_presentation,
};

#[test]
fn adjacent_heading_body_resolution_is_typed_exact_and_shared_with_rendering() {
    let source = concat!(
        "= 文缕\r\n",
        "\r\n",
        "[.run-in]\r\n",
        "== 显式 😀\r\n",
        "\r\n",
        "[#first-body]\r\n",
        "正文 שלום\r\n",
        "\n",
        "[.separate]\n",
        "== 分开\n",
        "直接段落\n",
    );
    let analysis = analyze_with_presentation(source, AdjacentHeadingBodyDefault::RunIn);
    let explicit = analysis
        .adjacent_heading_bodies
        .iter()
        .find(|resolution| resolution.rule == AdjacentHeadingBodyRule::ExplicitRunInRole)
        .expect("explicit run-in decision");

    assert_eq!(
        explicit.presentation,
        AdjacentHeadingBodyPresentation::RunIn
    );
    assert_eq!(
        explicit.eligibility,
        AdjacentHeadingBodyEligibility::Eligible
    );
    let heading = &analysis.blocks[usize::try_from(explicit.heading_block).unwrap()];
    let body = &analysis.blocks[usize::try_from(explicit.body_block.unwrap()).unwrap()];
    assert_eq!(heading.kind, BlockKind::Heading);
    assert_eq!(body.kind, BlockKind::Paragraph);
    assert_eq!(body.block_id.as_deref(), Some("first-body"));
    assert!(heading.range.end <= body.range.start);
    assert!(source.is_char_boundary(usize::try_from(heading.text_range.start).unwrap()));
    assert!(source.is_char_boundary(usize::try_from(body.text_range.start).unwrap()));
    assert!(
        analysis
            .safe_html
            .contains("data-adjacent-heading-body=\"run_in\"")
    );
    assert!(analysis.safe_html.contains("data-run-in-heading-block="));

    let separate = analysis
        .adjacent_heading_bodies
        .iter()
        .find(|resolution| resolution.rule == AdjacentHeadingBodyRule::ExplicitSeparateRole)
        .expect("explicit separate decision");
    assert_eq!(
        separate.presentation,
        AdjacentHeadingBodyPresentation::Separate
    );
}

#[test]
fn workspace_default_requires_physical_adjacency_and_never_merges_non_paragraphs() {
    let immediate = analyze_with_presentation(
        "== Immediate\r\n正文\r\n",
        AdjacentHeadingBodyDefault::RunIn,
    );
    assert_eq!(
        immediate.adjacent_heading_bodies[0].presentation,
        AdjacentHeadingBodyPresentation::RunIn
    );

    let spaced = analyze_with_presentation(
        "== Spaced\r\n\r\n正文\r\n",
        AdjacentHeadingBodyDefault::RunIn,
    );
    assert_eq!(
        spaced.adjacent_heading_bodies[0].eligibility,
        AdjacentHeadingBodyEligibility::NotOnImmediatelyFollowingPhysicalLine
    );
    assert_eq!(
        spaced.adjacent_heading_bodies[0].presentation,
        AdjacentHeadingBodyPresentation::Separate
    );

    let list = analyze_with_presentation(
        "[.run-in]\n== Not a paragraph\n* item\n",
        AdjacentHeadingBodyDefault::RunIn,
    );
    assert_eq!(
        list.adjacent_heading_bodies[0].eligibility,
        AdjacentHeadingBodyEligibility::FollowingBlockIsNotParagraph
    );
    assert_eq!(list.adjacent_heading_bodies[0].body_block, None);
    assert!(list.adjacent_heading_bodies.iter().all(|resolution| {
        resolution.presentation == AdjacentHeadingBodyPresentation::Separate
    }));
}

#[test]
fn a_second_level_zero_title_has_a_dedicated_failing_structure_diagnostic() {
    let source = "= First\r\n\r\n== Section\nBody\n\n= Second 标题 😀\r\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::AdditionalDocumentTitle)
        .expect("dedicated additional-title diagnostic");
    assert_eq!(analysis.status, AnalysisStatus::Failed);
    let exact = &source[usize::try_from(diagnostic.range.start).unwrap()
        ..usize::try_from(diagnostic.range.end).unwrap()];
    assert_eq!(exact, "= Second 标题 😀\r\n");
    assert!(
        analysis
            .safe_html
            .contains("data-analysis-status=\"failed\"")
    );
    assert!(analysis.safe_html.contains("= Second 标题 😀"));
}

#[test]
fn active_effects_are_typed_but_ordinary_external_links_do_not_request_loading() {
    let source = concat!(
        "= Effects\n",
        ":source-highlighter: rouge\n",
        "\n",
        "https://example.invalid[ordinary link]\n",
        "image::https://cdn.example.invalid/image.png[remote]\n",
        "include::https://example.invalid/private.adoc[]\n",
        "ifdef::feature[]\n",
        "conditional text\n",
        "endif::[]\n",
        "pass:[<script>x</script>]\n",
        "\n",
        "[source]\n",
        "----\n",
        "include::https://protected.invalid/no.adoc[]\n",
        "pass:[protected]\n",
        "----\n",
    );
    let analysis = analyze(source);

    assert!(analysis.effects.iter().any(|effect| {
        effect.origin == EffectOrigin::DocumentHeaderAttribute
            && effect.required_capability == EffectCapability::ProcessorExecution
            && effect.decision == EffectDecision::Denied
            && effect.target.as_deref() == Some("source-highlighter")
    }));
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == DiagnosticCode::ProcessorEffectDisabled })
    );
    assert!(analysis.effects.iter().any(|effect| {
        effect.origin == EffectOrigin::IncludeDirective
            && effect.required_capability == EffectCapability::IncludeExpansion
            && effect.decision == EffectDecision::Denied
    }));
    assert!(analysis.effects.iter().any(|effect| {
        effect.origin == EffectOrigin::IncludeDirective
            && effect.required_capability == EffectCapability::NetworkRead
    }));
    assert!(analysis.effects.iter().any(|effect| {
        effect.origin == EffectOrigin::ConditionalDirective
            && effect.required_capability == EffectCapability::ConditionalEvaluation
    }));
    assert!(analysis.effects.iter().any(|effect| {
        effect.origin == EffectOrigin::ImageResource
            && effect.required_capability == EffectCapability::NetworkRead
            && effect.target.as_deref() == Some("https://cdn.example.invalid/image.png")
    }));
    assert!(analysis.effects.iter().any(|effect| {
        effect.origin == EffectOrigin::InlinePassthrough
            && effect.required_capability == EffectCapability::PassthroughRendering
    }));
    assert!(
        !analysis
            .effects
            .iter()
            .any(|effect| { effect.target.as_deref() == Some("https://example.invalid") })
    );
    assert!(
        !analysis.effects.iter().any(|effect| {
            effect.target.as_deref() == Some("https://protected.invalid/no.adoc")
        })
    );

    let ordinary_link_only = analyze("https://example.invalid[ordinary link]\n");
    assert!(ordinary_link_only.effects.is_empty());
    assert!(
        !ordinary_link_only
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::RemoteUri)
    );
}

#[test]
fn marker_quote_keeps_true_depth_and_unfrozen_forms_fail_visible() {
    let deep = format!("{}深层 CJK 😀\n", "> ".repeat(300));
    let source = format!(
        "{deep}>\n> continued +\n> attributed\n-- Unknown Author\n\n[source]\n----\n> protected\n----\n"
    );
    let analysis = analyze(&source);
    let quote = analysis
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::Quote && block.quote_depth == Some(300))
        .expect("depth must not saturate or flatten");
    assert_eq!(quote.text, "深层 CJK 😀");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::QuoteSyntaxUnresolved)
    );
    let attribution_start = source.find("-- Unknown").unwrap() as u64;
    assert!(
        analysis
            .blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Quote && block.range.start < attribution_start)
            .all(|block| {
                matches!(
                    &block.semantic,
                    BlockSemantic::Quote {
                        attribution: None,
                        citation: None,
                        ..
                    }
                )
            })
    );
    assert!(
        !analysis
            .blocks
            .iter()
            .any(|block| { block.kind == BlockKind::Quote && block.text.contains("protected") })
    );
}
