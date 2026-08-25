use std::ops::Range;

use weftext_asciidoc::{
    AdjacentHeadingBodyDefault, AdjacentHeadingBodyPresentation, BlockKind, DiagnosticCode,
    LinkKind,
};

use crate::{
    AdjacentHeadingBody, DOCUMENT_CONTRACT_VERSION, DocumentBlock, DocumentBlockKind,
    DocumentCapabilities, DocumentDiagnostic, DocumentDiagnosticCode, DocumentFormatAdapter,
    DocumentLinkKind, DocumentLinkOccurrence, DocumentModel, DocumentProfileDescriptor,
    DocumentProfileId, DocumentSourceOccurrences, RunInGroup,
};

pub(crate) static ASCIIDOC_DOCUMENT_ADAPTER: AsciiDocDocumentAdapter = AsciiDocDocumentAdapter;

pub(crate) struct AsciiDocDocumentAdapter;

impl DocumentFormatAdapter for AsciiDocDocumentAdapter {
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

    fn parse(&self, source: &str, setting: AdjacentHeadingBody) -> DocumentModel {
        let default = match setting {
            AdjacentHeadingBody::RunIn => AdjacentHeadingBodyDefault::RunIn,
            AdjacentHeadingBody::Separate => AdjacentHeadingBodyDefault::Separate,
        };
        let analysis = weftext_asciidoc::analyze_with_presentation(source, default);
        let blocks = analysis
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
            .collect::<Vec<_>>();
        let adjacent_heading_bodies = analysis.adjacent_heading_bodies;
        let run_in_groups = adjacent_heading_bodies
            .iter()
            .filter_map(|resolution| {
                (resolution.presentation == AdjacentHeadingBodyPresentation::RunIn)
                    .then_some(resolution.body_block)
                    .flatten()
                    .map(|body_block| RunInGroup {
                        heading_block: resolution.heading_block,
                        body_block,
                    })
            })
            .collect();
        DocumentModel {
            semantic_model_version: analysis.semantic_model_version,
            status: analysis.status,
            blocks,
            inlines: analysis.inlines,
            run_in_groups,
            adjacent_heading_bodies,
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
        DocumentSourceOccurrences {
            links,
            eligible_text_ranges: complement_ranges(source.len(), &analysis.protected_ranges),
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
