use std::fmt::Write as _;

use weftext_core::{
    DocumentBlock, DocumentBlockSemantic, DocumentInlineKind, DocumentInlineSemantic,
    DocumentListItem, DocumentListKind, DocumentListModel, DocumentModel, DocumentTableCell,
    DocumentTableModel, analyze_document_header_properties,
};

use crate::contract::invalid_source;
use crate::{
    ExportDiagnostic, ExportDiagnosticSeverity, ExportError, MarkdownCompatibilityReport,
    MarkdownMetadataPolicy,
};

pub(crate) struct RenderedMarkdown {
    pub artifact: String,
    pub diagnostics: Vec<ExportDiagnostic>,
    pub report: MarkdownCompatibilityReport,
}

pub(crate) fn render(
    source: &str,
    model: &DocumentModel,
    metadata_policy: MarkdownMetadataPolicy,
) -> Result<RenderedMarkdown, ExportError> {
    let mut renderer = Renderer {
        source,
        model,
        output: String::new(),
        diagnostics: Vec::new(),
        report: MarkdownCompatibilityReport::default(),
    };
    renderer.render_frontmatter(metadata_policy)?;
    let header_end = renderer.render_header_remainder()?;
    for block in &model.blocks {
        if matches!(block.semantic, DocumentBlockSemantic::Frontmatter) {
            continue;
        }
        if block.end <= header_end
            && !matches!(
                block.semantic,
                DocumentBlockSemantic::DocumentTitle | DocumentBlockSemantic::DocumentSubtitle
            )
        {
            continue;
        }
        renderer.render_block(block)?;
    }
    while renderer.output.ends_with("\n\n\n") {
        renderer.output.pop();
    }
    if !renderer.output.ends_with('\n') {
        renderer.output.push('\n');
    }
    Ok(RenderedMarkdown {
        artifact: renderer.output,
        diagnostics: renderer.diagnostics,
        report: renderer.report,
    })
}

struct Renderer<'a> {
    source: &'a str,
    model: &'a DocumentModel,
    output: String,
    diagnostics: Vec<ExportDiagnostic>,
    report: MarkdownCompatibilityReport,
}

impl Renderer<'_> {
    fn render_frontmatter(&mut self, policy: MarkdownMetadataPolicy) -> Result<(), ExportError> {
        let frontmatter = self
            .model
            .blocks
            .iter()
            .find(|block| matches!(block.semantic, DocumentBlockSemantic::Frontmatter));
        match (policy, frontmatter) {
            (MarkdownMetadataPolicy::PreserveWeftext, Some(block)) => {
                let exact = self.block_source(block)?;
                self.output.push_str(exact.trim_end_matches(['\r', '\n']));
                self.output.push_str("\n\n");
                self.report.exact_blocks = self.report.exact_blocks.saturating_add(1);
            }
            (MarkdownMetadataPolicy::RemoveWeftext, Some(block)) => {
                self.diagnostic(
                    "weftext_metadata_removed",
                    ExportDiagnosticSeverity::Omission,
                    "the explicit plain-Markdown option removed the Weftext operational envelope",
                    Some(block),
                );
                self.report.omitted_blocks = self.report.omitted_blocks.saturating_add(1);
            }
            (
                MarkdownMetadataPolicy::PreserveWeftext | MarkdownMetadataPolicy::RemoveWeftext,
                None,
            ) => {
                return Err(invalid_source(
                    "canonical managed source has no modeled Weftext envelope",
                ));
            }
        }
        Ok(())
    }

    fn render_header_remainder(&mut self) -> Result<u64, ExportError> {
        let analysis = analyze_document_header_properties(self.source);
        for diagnostic in analysis.diagnostics {
            self.diagnostics.push(ExportDiagnostic {
                code: format!("asciidoc_header_{:?}", diagnostic.code).to_ascii_lowercase(),
                severity: ExportDiagnosticSeverity::Warning,
                message: diagnostic.message,
                source_start: Some(diagnostic.range.start),
                source_end: Some(diagnostic.range.end),
            });
        }
        let title_end = self
            .model
            .blocks
            .iter()
            .filter(|block| {
                matches!(
                    block.semantic,
                    DocumentBlockSemantic::DocumentTitle | DocumentBlockSemantic::DocumentSubtitle
                )
            })
            .map(|block| block.end)
            .max()
            .unwrap_or(analysis.header_range.start);
        let start = title_end.max(analysis.header_range.start);
        if start < analysis.header_range.end {
            let fragment = source_slice(self.source, start, analysis.header_range.end)?
                .trim_matches(['\r', '\n']);
            if !fragment.is_empty() {
                render_fence(&mut self.output, Some("asciidoc"), fragment);
                self.diagnostics.push(ExportDiagnostic {
                    code: "asciidoc_header_preserved_literal".to_owned(),
                    severity: ExportDiagnosticSeverity::Warning,
                    message: "AsciiDoc author/revision/attribute header lines were preserved as a visible literal block instead of being guessed into Markdown YAML".to_owned(),
                    source_start: Some(start),
                    source_end: Some(analysis.header_range.end),
                });
                self.report.preserved_literal_blocks =
                    self.report.preserved_literal_blocks.saturating_add(1);
            }
        }
        Ok(analysis.header_range.end)
    }

    #[allow(clippy::too_many_lines)]
    fn render_block(&mut self, block: &DocumentBlock) -> Result<(), ExportError> {
        match &block.semantic {
            DocumentBlockSemantic::Frontmatter => {}
            DocumentBlockSemantic::DocumentTitle => {
                let text = self.inline_text(block)?;
                writeln!(self.output, "# {text}\n").expect("String writes cannot fail");
                self.lowered();
            }
            DocumentBlockSemantic::DocumentSubtitle => {
                let text = self.inline_text(block)?;
                writeln!(self.output, "*{text}*\n").expect("String writes cannot fail");
                self.lowered();
            }
            DocumentBlockSemantic::Heading { level } => {
                let text = self.inline_text(block)?;
                writeln!(self.output, "{} {text}\n", "#".repeat(usize::from(*level)))
                    .expect("String writes cannot fail");
                if *level > 6 {
                    self.diagnostic(
                        "markdown_extended_heading",
                        ExportDiagnosticSeverity::Warning,
                        "H7-H9 was emitted with its exact Weftext Markdown marker depth; generic CommonMark readers may treat it as text",
                        Some(block),
                    );
                }
                self.lowered();
            }
            DocumentBlockSemantic::Paragraph => {
                let text = self.inline_text(block)?;
                self.output.push_str(&text.replace('\n', "  \n"));
                self.output.push_str("\n\n");
                self.lowered();
            }
            DocumentBlockSemantic::Quote { depth, .. } => {
                let depth = depth.map(u64::from).or(block.quote_depth).unwrap_or(1);
                let marker = "> ".repeat(usize::try_from(depth).unwrap_or(9));
                let text = self.inline_text(block)?;
                for line in text.lines() {
                    writeln!(self.output, "{marker}{line}").expect("String writes cannot fail");
                }
                self.output.push('\n');
                self.lowered();
            }
            DocumentBlockSemantic::Listing { language } => {
                render_fence(&mut self.output, language.as_deref(), &block.text);
                self.lowered();
            }
            DocumentBlockSemantic::Literal => {
                render_fence(&mut self.output, None, &block.text);
                self.lowered();
            }
            DocumentBlockSemantic::List { model } => {
                self.render_list(model, block);
                self.lowered();
            }
            DocumentBlockSemantic::Table { model } => {
                self.render_table(model, block);
                self.lowered();
            }
            DocumentBlockSemantic::Image { target, alt } => {
                let alt = escape_markdown_inline(alt.as_deref().unwrap_or(target));
                writeln!(self.output, "![{alt}]({})\n", markdown_destination(target))
                    .expect("String writes cannot fail");
                self.diagnostic(
                    "external_image_reference_not_copied",
                    ExportDiagnosticSeverity::Warning,
                    "the standalone Markdown artifact keeps the authored image locator but does not copy node-owned resources",
                    Some(block),
                );
                self.lowered();
            }
            DocumentBlockSemantic::BlockTitle => {
                writeln!(self.output, "**{}**\n", escape_markdown_inline(&block.text))
                    .expect("String writes cannot fail");
                self.lowered();
            }
            DocumentBlockSemantic::Math { .. } => {
                render_fence(&mut self.output, Some("math"), &block.text);
                self.diagnostic(
                    "math_preserved_fenced",
                    ExportDiagnosticSeverity::Warning,
                    "math source was preserved in a fenced Markdown block without claiming renderer equivalence",
                    Some(block),
                );
                self.literal();
            }
            DocumentBlockSemantic::Mermaid => {
                render_fence(&mut self.output, Some("mermaid"), &block.text);
                self.diagnostic(
                    "mermaid_preserved_fenced",
                    ExportDiagnosticSeverity::Warning,
                    "Mermaid source was preserved in a fenced block; rendering depends on the destination reader",
                    Some(block),
                );
                self.literal();
            }
            DocumentBlockSemantic::Passthrough => {
                let exact = self.block_source(block)?.to_owned();
                render_fence(&mut self.output, Some("asciidoc"), &exact);
                self.diagnostic(
                    "passthrough_effect_disabled_literal",
                    ExportDiagnosticSeverity::Warning,
                    "disabled passthrough source was preserved inertly as an AsciiDoc literal",
                    Some(block),
                );
                self.literal();
            }
            DocumentBlockSemantic::Comment => {
                self.diagnostic(
                    "asciidoc_comment_omitted",
                    ExportDiagnosticSeverity::Omission,
                    "AsciiDoc comment source was omitted from the compatibility artifact",
                    Some(block),
                );
                self.report.omitted_blocks = self.report.omitted_blocks.saturating_add(1);
            }
            DocumentBlockSemantic::ThematicBreak => {
                self.output.push_str("---\n\n");
                self.lowered();
            }
            DocumentBlockSemantic::Unsupported { context } => {
                let exact = self.block_source(block)?.to_owned();
                render_fence(&mut self.output, Some("asciidoc"), &exact);
                self.diagnostic(
                    "unsupported_asciidoc_preserved_literal",
                    ExportDiagnosticSeverity::Warning,
                    &format!(
                        "unsupported AsciiDoc context `{context}` was preserved inertly as exact source"
                    ),
                    Some(block),
                );
                self.literal();
            }
        }
        Ok(())
    }

    fn inline_text(&mut self, block: &DocumentBlock) -> Result<String, ExportError> {
        let start = block.text_start;
        let end = block.text_end;
        let mut inlines = self
            .model
            .inlines
            .iter()
            .filter(|inline| inline.range.start >= start && inline.range.end <= end)
            .collect::<Vec<_>>();
        inlines.sort_by_key(|inline| (inline.range.start, u64::MAX - inline.range.end));
        let mut output = String::new();
        let mut cursor = start;
        for inline in inlines {
            if inline.range.start < cursor || inline.range.end <= inline.range.start {
                continue;
            }
            output.push_str(&escape_markdown_inline(source_slice(
                self.source,
                cursor,
                inline.range.start,
            )?));
            output.push_str(&self.lower_inline(inline));
            cursor = inline.range.end;
        }
        output.push_str(&escape_markdown_inline(source_slice(
            self.source,
            cursor,
            end,
        )?));
        if output.is_empty() && !block.text.is_empty() {
            return Ok(escape_markdown_inline(&block.text));
        }
        Ok(output)
    }

    #[allow(clippy::too_many_lines)]
    fn lower_inline(&mut self, inline: &DocumentInlineSemantic) -> String {
        let visible = escape_markdown_inline(inline.text.as_deref().unwrap_or_default());
        match inline.kind {
            DocumentInlineKind::Bold => format!("**{visible}**"),
            DocumentInlineKind::Italic | DocumentInlineKind::Quoted => format!("*{visible}*"),
            DocumentInlineKind::Monospace => markdown_code(inline.text.as_deref().unwrap_or("")),
            DocumentInlineKind::Highlight => {
                self.inline_diagnostic(
                    "highlight_lowered_to_emphasis",
                    "highlight was lowered to strong emphasis because plain Markdown has no portable highlight syntax",
                    inline,
                );
                format!("**{visible}**")
            }
            DocumentInlineKind::Superscript | DocumentInlineKind::Subscript => {
                self.inline_diagnostic(
                    "script_lowered_visible_text",
                    "superscript/subscript presentation was lowered to visible text",
                    inline,
                );
                visible
            }
            DocumentInlineKind::RoleSpan => {
                self.inline_diagnostic(
                    "role_span_lowered_visible_text",
                    "AsciiDoc roles were lowered to visible text",
                    inline,
                );
                visible
            }
            DocumentInlineKind::NativeLink | DocumentInlineKind::Xref => {
                let target = inline.target.as_deref().unwrap_or_default();
                let label = if visible.is_empty() {
                    escape_markdown_inline(target)
                } else {
                    visible
                };
                if inline.kind == DocumentInlineKind::Xref {
                    self.inline_diagnostic(
                        "xref_preserved_external_locator",
                        "AsciiDoc xref was emitted as an external Markdown link without copying its target",
                        inline,
                    );
                }
                format!("[{label}]({})", markdown_destination(target))
            }
            DocumentInlineKind::Image => {
                let target = inline.target.as_deref().unwrap_or_default();
                self.inline_diagnostic(
                    "external_image_reference_not_copied",
                    "inline image locator was retained but the standalone export did not copy its resource",
                    inline,
                );
                format!("![{visible}]({})", markdown_destination(target))
            }
            DocumentInlineKind::Node | DocumentInlineKind::NodeEmbed => {
                let target = inline.target.as_deref().unwrap_or_default();
                let mut destination = format!("weftext://node/{target}");
                if let Some(fragment) = &inline.fragment {
                    destination.push('#');
                    destination.push_str(fragment);
                }
                self.inline_diagnostic(
                    "node_link_lowered_weftext_uri",
                    "managed node semantics were lowered to an explicit weftext:// locator",
                    inline,
                );
                let label = if visible.is_empty() {
                    escape_markdown_inline(target)
                } else {
                    visible
                };
                format!("[{label}]({})", markdown_destination(&destination))
            }
            DocumentInlineKind::Footnote | DocumentInlineKind::Endnote => {
                self.inline_diagnostic(
                    "note_lowered_inline_text",
                    "footnote/endnote semantics were lowered to explicit inline note text",
                    inline,
                );
                format!("(Note: {visible})")
            }
            DocumentInlineKind::Stem | DocumentInlineKind::LatexMath => {
                self.inline_diagnostic(
                    "inline_math_lowered_dollar",
                    "inline math was lowered to dollar-delimited Markdown math syntax",
                    inline,
                );
                format!("${}$", inline.text.as_deref().unwrap_or_default())
            }
            DocumentInlineKind::Anchor => {
                let target = inline.target.as_deref().unwrap_or_default();
                self.inline_diagnostic(
                    "anchor_lowered_visible_marker",
                    "AsciiDoc anchor was lowered to a visible non-HTML marker",
                    inline,
                );
                format!("[anchor: {}]", escape_markdown_inline(target))
            }
            DocumentInlineKind::Passthrough | DocumentInlineKind::Unsupported => {
                self.inline_diagnostic(
                    "unsupported_inline_preserved_code",
                    "unsupported or disabled inline source was preserved as inert code text",
                    inline,
                );
                markdown_code(inline.text.as_deref().unwrap_or_default())
            }
        }
    }

    fn render_list(&mut self, model: &DocumentListModel, block: &DocumentBlock) {
        render_list_items(&mut self.output, model.kind, &model.items, 0);
        self.output.push('\n');
        if model.kind == DocumentListKind::Description || model.kind == DocumentListKind::Callout {
            self.diagnostic(
                "special_list_lowered_unordered",
                ExportDiagnosticSeverity::Warning,
                "description/callout list semantics were lowered to visible unordered items",
                Some(block),
            );
        }
        if model.items.iter().any(item_has_unmodeled_continuations) {
            self.diagnostic(
                "list_continuation_omitted",
                ExportDiagnosticSeverity::Omission,
                "one or more unmodeled AsciiDoc list continuations were omitted",
                Some(block),
            );
        }
    }

    fn render_table(&mut self, model: &DocumentTableModel, block: &DocumentBlock) {
        let width = usize::try_from(model.column_count).unwrap_or(0).max(1);
        let header = model.header.as_ref().map_or_else(
            || vec![String::new(); width],
            |row| table_row(&row.cells, width),
        );
        write_table_row(&mut self.output, &header);
        write_table_row(&mut self.output, &vec!["---".to_owned(); width]);
        for row in &model.body {
            write_table_row(&mut self.output, &table_row(&row.cells, width));
        }
        if let Some(footer) = &model.footer {
            write_table_row(&mut self.output, &table_row(&footer.cells, width));
        }
        self.output.push('\n');
        if model
            .header
            .iter()
            .chain(model.body.iter())
            .chain(model.footer.iter())
            .flat_map(|row| &row.cells)
            .any(|cell| cell.column_span != 1 || cell.row_span != 1 || cell.nested_asciidoc)
        {
            self.diagnostic(
                "table_span_or_nested_content_flattened",
                ExportDiagnosticSeverity::Warning,
                "row/column spans or nested AsciiDoc cells were flattened into visible GFM table cells",
                Some(block),
            );
        }
    }

    fn block_source<'a>(&self, block: &DocumentBlock) -> Result<&'a str, ExportError>
    where
        Self: 'a,
    {
        source_slice(self.source, block.start, block.end)
    }

    fn diagnostic(
        &mut self,
        code: &str,
        severity: ExportDiagnosticSeverity,
        message: &str,
        block: Option<&DocumentBlock>,
    ) {
        self.diagnostics.push(ExportDiagnostic {
            code: code.to_owned(),
            severity,
            message: message.to_owned(),
            source_start: block.map(|block| block.start),
            source_end: block.map(|block| block.end),
        });
    }

    fn inline_diagnostic(&mut self, code: &str, message: &str, inline: &DocumentInlineSemantic) {
        self.diagnostics.push(ExportDiagnostic {
            code: code.to_owned(),
            severity: ExportDiagnosticSeverity::Warning,
            message: message.to_owned(),
            source_start: Some(inline.range.start),
            source_end: Some(inline.range.end),
        });
    }

    fn lowered(&mut self) {
        self.report.lowered_blocks = self.report.lowered_blocks.saturating_add(1);
    }

    fn literal(&mut self) {
        self.report.preserved_literal_blocks =
            self.report.preserved_literal_blocks.saturating_add(1);
    }
}

fn source_slice(source: &str, start: u64, end: u64) -> Result<&str, ExportError> {
    let start = usize::try_from(start)
        .map_err(|_| invalid_source("Core source range exceeds this platform"))?;
    let end = usize::try_from(end)
        .map_err(|_| invalid_source("Core source range exceeds this platform"))?;
    source.get(start..end).ok_or_else(|| {
        invalid_source(format!(
            "Core source range {start}..{end} is outside {} bytes or not a UTF-8 boundary",
            source.len()
        ))
    })
}

fn render_fence(output: &mut String, language: Option<&str>, source: &str) {
    let longest = source
        .lines()
        .filter(|line| line.bytes().all(|byte| byte == b'`'))
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.saturating_add(1).max(3));
    let language = language.filter(|language| {
        !language.is_empty()
            && language.len() <= 64
            && language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-'))
    });
    writeln!(output, "{fence}{}", language.unwrap_or_default()).expect("String writes cannot fail");
    output.push_str(source);
    if !source.ends_with('\n') {
        output.push('\n');
    }
    writeln!(output, "{fence}\n").expect("String writes cannot fail");
}

fn render_list_items(
    output: &mut String,
    kind: DocumentListKind,
    items: &[DocumentListItem],
    inherited_depth: usize,
) {
    for (index, item) in items.iter().enumerate() {
        let depth = usize::from(item.depth)
            .saturating_sub(1)
            .max(inherited_depth);
        output.push_str(&"  ".repeat(depth));
        match kind {
            DocumentListKind::Ordered => {
                let _ = write!(output, "{}. ", index.saturating_add(1));
            }
            DocumentListKind::Unordered
            | DocumentListKind::Description
            | DocumentListKind::Callout => output.push_str("- "),
        }
        if let Some(checked) = item.checked {
            output.push_str(if checked { "[x] " } else { "[ ] " });
        }
        output.push_str(&escape_markdown_inline(&item.text));
        output.push('\n');
        render_list_items(output, kind, &item.children, depth.saturating_add(1));
    }
}

fn item_has_unmodeled_continuations(item: &DocumentListItem) -> bool {
    !item.unmodeled_continuations.is_empty()
        || item.children.iter().any(item_has_unmodeled_continuations)
}

fn table_row(cells: &[DocumentTableCell], width: usize) -> Vec<String> {
    let mut values = cells
        .iter()
        .map(|cell| {
            escape_markdown_inline(&cell.text)
                .replace('|', "\\|")
                .replace(['\r', '\n'], " ")
        })
        .collect::<Vec<_>>();
    values.resize(width, String::new());
    values.truncate(width);
    values
}

fn write_table_row(output: &mut String, cells: &[String]) {
    output.push('|');
    for cell in cells {
        let _ = write!(output, " {cell} |");
    }
    output.push('\n');
}

fn markdown_code(source: &str) -> String {
    let longest = source
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let marker = "`".repeat(longest.saturating_add(1).max(1));
    format!("{marker}{source}{marker}")
}

fn markdown_destination(target: &str) -> String {
    let mut encoded = String::new();
    for byte in target.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':' | b'#') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn escape_markdown_inline(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '[' | ']' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}
