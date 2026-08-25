use std::ops::Range;

use weftext_asciidoc::{BlockKind, DiagnosticCode, LinkKind};
use weftext_core::{
    AdjacentHeadingBody, DOCUMENT_CONTRACT_VERSION, DocumentBlock, DocumentBlockKind,
    DocumentCapabilities, DocumentDiagnostic, DocumentDiagnosticCode, DocumentFormatAdapter,
    DocumentLinkKind, DocumentLinkOccurrence, DocumentModel, DocumentProfileDescriptor,
    DocumentProfileId, DocumentSourceOccurrences, active_document_profile, analyze_document,
    analyze_document_with_adapter,
};

struct ExperimentalAsciiDocAdapter;

impl DocumentFormatAdapter for ExperimentalAsciiDocAdapter {
    fn descriptor(&self) -> DocumentProfileDescriptor {
        DocumentProfileDescriptor {
            contract_version: DOCUMENT_CONTRACT_VERSION,
            profile: DocumentProfileId::AsciiDocV1,
            media_type: "text/asciidoc",
            canonical_extension: "adoc",
            capabilities: DocumentCapabilities {
                exact_source: true,
                utf8_source_edits: true,
                yaml_envelope: true,
                max_heading_level: 9,
                actual_quote_depth: true,
                block_ids: true,
                managed_links: true,
                protected_regions: true,
                typed_blocks: true,
                typed_inlines: true,
                nested_lists: true,
                typed_tables: true,
                safe_render_input: true,
                degradation_reports: true,
                adjacent_heading_body_resolution: true,
                typed_effect_evidence: true,
            },
        }
    }

    fn parse(&self, source: &str, _setting: AdjacentHeadingBody) -> DocumentModel {
        let analysis = weftext_asciidoc::analyze(source);
        DocumentModel {
            semantic_model_version: analysis.semantic_model_version,
            status: analysis.status,
            blocks: analysis
                .blocks
                .into_iter()
                .map(|block| DocumentBlock {
                    kind: map_block_kind(block.kind),
                    start: block.range.start,
                    end: block.range.end,
                    text_start: block.text_range.start,
                    text_end: block.text_range.end,
                    text: block.text,
                    heading_level: block.heading_level,
                    quote_depth: block.quote_depth,
                    block_id: block.block_id,
                    roles: block.roles,
                    title: block.title,
                    semantic: block.semantic,
                })
                .collect(),
            inlines: analysis.inlines,
            run_in_groups: Vec::new(),
            adjacent_heading_bodies: analysis.adjacent_heading_bodies,
            effects: analysis.effects,
            diagnostics: analysis
                .diagnostics
                .into_iter()
                .map(|diagnostic| DocumentDiagnostic {
                    code: map_diagnostic_code(diagnostic.code),
                    start: diagnostic.range.start,
                    end: diagnostic.range.end,
                    message: diagnostic.message,
                })
                .collect(),
            degradations: analysis.degradations,
            safe_html: analysis.safe_html,
        }
    }

    fn extract_occurrences(
        &self,
        source: &str,
        _model: &DocumentModel,
    ) -> DocumentSourceOccurrences {
        let analysis = weftext_asciidoc::analyze(source);
        let links = analysis
            .links
            .into_iter()
            .filter_map(|link| match link.kind {
                LinkKind::Node | LinkKind::NodeEmbed => Some(DocumentLinkOccurrence {
                    kind: if link.kind == LinkKind::NodeEmbed {
                        DocumentLinkKind::Embed
                    } else {
                        DocumentLinkKind::Link
                    },
                    start: link.range.start,
                    end: link.range.end,
                    locator_start: link.target_range.start,
                    locator_end: link.target_range.end,
                    authored_locator: link.target,
                    fragment: link.fragment,
                    display_text: link.display,
                }),
                _ => None,
            })
            .collect();
        let eligible_text_ranges = complement_ranges(source.len(), &analysis.protected_ranges);
        DocumentSourceOccurrences {
            links,
            eligible_text_ranges,
            protected_ranges: analysis.protected_ranges,
        }
    }

    fn searchable_text(&self, source: &str, _model: &DocumentModel) -> String {
        weftext_asciidoc::analyze(source).searchable_text
    }
}

fn map_block_kind(kind: BlockKind) -> DocumentBlockKind {
    match kind {
        BlockKind::Frontmatter => DocumentBlockKind::Frontmatter,
        BlockKind::DocumentTitle => DocumentBlockKind::DocumentTitle,
        BlockKind::DocumentSubtitle => DocumentBlockKind::DocumentSubtitle,
        BlockKind::Heading => DocumentBlockKind::Heading,
        BlockKind::Paragraph => DocumentBlockKind::Paragraph,
        BlockKind::Listing => DocumentBlockKind::Listing,
        BlockKind::Literal => DocumentBlockKind::Literal,
        BlockKind::Quote => DocumentBlockKind::Quote,
        BlockKind::List => DocumentBlockKind::List,
        BlockKind::Table => DocumentBlockKind::Table,
        BlockKind::Image => DocumentBlockKind::Image,
        BlockKind::BlockTitle => DocumentBlockKind::BlockTitle,
        BlockKind::Math => DocumentBlockKind::Math,
        BlockKind::Mermaid => DocumentBlockKind::Mermaid,
        BlockKind::Passthrough => DocumentBlockKind::Passthrough,
        BlockKind::Comment => DocumentBlockKind::Comment,
        BlockKind::ThematicBreak => DocumentBlockKind::ThematicBreak,
        BlockKind::Other => DocumentBlockKind::Unsupported,
    }
}

fn map_diagnostic_code(code: DiagnosticCode) -> DocumentDiagnosticCode {
    match code {
        DiagnosticCode::UnclosedFrontmatter => DocumentDiagnosticCode::UnclosedFrontmatter,
        DiagnosticCode::UnsafeInclude
        | DiagnosticCode::ConditionalDisabled
        | DiagnosticCode::ProcessorEffectDisabled
        | DiagnosticCode::RemoteUri
        | DiagnosticCode::PassthroughDisabled => DocumentDiagnosticCode::UnsafeActiveContent,
        DiagnosticCode::ParserError => DocumentDiagnosticCode::ParserError,
        DiagnosticCode::AdditionalDocumentTitle => DocumentDiagnosticCode::InvalidDocumentStructure,
        DiagnosticCode::UnsupportedProfileSyntax | DiagnosticCode::InvalidNodeLink => {
            DocumentDiagnosticCode::UnsupportedProfileSyntax
        }
        DiagnosticCode::ParserWarning | DiagnosticCode::QuoteSyntaxUnresolved => {
            DocumentDiagnosticCode::ProfileWarning
        }
    }
}

fn complement_ranges(length: usize, protected: &[Range<u64>]) -> Vec<Range<u64>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for protected in protected {
        if start < protected.start {
            ranges.push(start..protected.start);
        }
        start = start.max(protected.end);
    }
    if start < length as u64 {
        ranges.push(start..length as u64);
    }
    ranges
}

#[test]
fn canonical_profile_runs_behind_the_core_contract_and_is_active() {
    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    let source = format!(
        "---\nweftext:\n  id: \"{uuid}\"\n---\n= Document\n\n== Section\nnode:{uuid}[Target]\n"
    );
    let analysis = analyze_document_with_adapter(
        &ExperimentalAsciiDocAdapter,
        &source,
        AdjacentHeadingBody::Separate,
    );
    assert_eq!(analysis.descriptor.profile, DocumentProfileId::AsciiDocV1);
    assert_eq!(analysis.occurrences.links.len(), 1);
    assert_eq!(analysis.occurrences.links[0].authored_locator, uuid);
    assert!(analysis.model.blocks.iter().any(|block| {
        block.kind == DocumentBlockKind::Heading && block.heading_level == Some(1)
    }));
    assert_eq!(
        active_document_profile().profile,
        DocumentProfileId::AsciiDocV1
    );
}

#[test]
fn unsafe_constructs_are_diagnostics_not_capabilities() {
    let source = "include::https://example.invalid/secret.adoc[]\npass:[<script>x</script>]\n";
    let analysis = analyze_document_with_adapter(
        &ExperimentalAsciiDocAdapter,
        source,
        AdjacentHeadingBody::Separate,
    );
    assert!(
        analysis
            .model
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == DocumentDiagnosticCode::UnsafeActiveContent })
    );
    assert_eq!(
        active_document_profile().profile,
        DocumentProfileId::AsciiDocV1
    );
}

#[test]
fn core_mapping_is_differentially_identical_to_the_profile_semantics() {
    let source = include_str!("../../weftext-asciidoc/tests/fixtures/profile-v1-semantics.adoc");
    let profile = weftext_asciidoc::analyze(source);
    let core = analyze_document(source, AdjacentHeadingBody::Separate);

    assert_eq!(
        core.model.semantic_model_version,
        profile.semantic_model_version
    );
    assert_eq!(core.model.status, profile.status);
    assert_eq!(core.model.inlines, profile.inlines);
    assert_eq!(
        core.model.adjacent_heading_bodies,
        profile.adjacent_heading_bodies
    );
    assert_eq!(core.model.effects, profile.effects);
    assert_eq!(core.model.degradations, profile.degradations);
    assert_eq!(core.model.safe_html, profile.safe_html);
    assert_eq!(core.model.blocks.len(), profile.blocks.len());
    for (mapped, original) in core.model.blocks.iter().zip(profile.blocks.iter()) {
        assert_eq!(mapped.start..mapped.end, original.range);
        assert_eq!(mapped.text_start..mapped.text_end, original.text_range);
        assert_eq!(mapped.roles, original.roles);
        assert_eq!(mapped.title, original.title);
        assert_eq!(mapped.semantic, original.semantic);
    }
    assert!(core.model.blocks.iter().any(|block| {
        block.kind == DocumentBlockKind::DocumentTitle && block.heading_level == Some(0)
    }));
    assert!(core.model.blocks.iter().any(|block| {
        block.kind == DocumentBlockKind::Image
            && matches!(
                block.semantic,
                weftext_core::DocumentBlockSemantic::Image { .. }
            )
    }));
    assert!(!core.model.blocks.iter().any(|block| {
        matches!(
            block.kind,
            DocumentBlockKind::Html | DocumentBlockKind::FencedCode
        )
    }));
    assert_eq!(
        core.view.semantic_model_version,
        profile.semantic_model_version
    );
    assert_eq!(core.view.inlines, profile.inlines);
    assert_eq!(
        core.view.adjacent_heading_bodies,
        profile.adjacent_heading_bodies
    );
    assert_eq!(core.view.effects, profile.effects);
    assert_eq!(core.view.safe_html, profile.safe_html);
}

#[test]
fn run_in_resolution_honors_roles_before_the_workspace_default() {
    let explicit = analyze_document(
        "[.run-in]\n== Explicit\n\nBody\n",
        AdjacentHeadingBody::Separate,
    );
    assert_eq!(explicit.model.run_in_groups.len(), 1);
    assert_eq!(
        explicit.model.adjacent_heading_bodies[0].rule,
        weftext_core::DocumentAdjacentHeadingBodyRule::ExplicitRunInRole
    );
    assert!(
        explicit
            .model
            .safe_html
            .contains("data-adjacent-heading-body=\"run_in\"")
    );

    let separate = analyze_document(
        "[.separate]\n== Separate\nBody\n",
        AdjacentHeadingBody::RunIn,
    );
    assert!(separate.model.run_in_groups.is_empty());

    let immediate = analyze_document("== Default\nBody\n", AdjacentHeadingBody::RunIn);
    assert_eq!(immediate.model.run_in_groups.len(), 1);

    let spaced = analyze_document("== Default\n\nBody\n", AdjacentHeadingBody::RunIn);
    assert!(spaced.model.run_in_groups.is_empty());
    assert_eq!(
        spaced.model.adjacent_heading_bodies[0].eligibility,
        weftext_core::DocumentAdjacentHeadingBodyEligibility::NotOnImmediatelyFollowingPhysicalLine
    );
}

#[test]
fn parser_failure_reaches_core_as_failed_typed_model_and_escaped_source() {
    let source = concat!(
        "[source]\n----\n",
        "[.weftext-query,version=1,view=task-list]\n....\n",
        "from tasks as task\nscope workspace\nwhere task.closed = false\n",
        "select task.title\norder by task.title asc\nlimit 100\n....\n",
        "----\n",
    );
    let analysis = analyze_document(source, AdjacentHeadingBody::Separate);

    assert_eq!(
        analysis.model.status,
        weftext_core::DocumentAnalysisStatus::Failed
    );
    assert!(
        analysis
            .model
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == DocumentDiagnosticCode::ParserError })
    );
    assert!(
        analysis
            .model
            .safe_html
            .contains("data-analysis-status=\"failed\"")
    );
    assert!(!analysis.model.safe_html.contains("<script>"));
}
