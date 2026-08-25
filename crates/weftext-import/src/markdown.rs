use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    AdapterDescriptor, AdapterRoute, AgentEnhancementPolicy, ComponentVersion, Confidence,
    DiagnosticSeverity, EgressDisclosure, EncryptionState, FORMAT_PROBE_CONTRACT_VERSION,
    FormatProbe, FormatWorker, ImportAdapter, ImportDiagnostic, ImportDocument, ImportError,
    ImportErrorCode, ImportLimits, ImportNode, ImportNodeKind, ImportPlan, ImportSourceLocation,
    LocalOcrPolicy, PlanRequest, PortablePath, ProbeReader, ProvenanceKind, ProvenanceRecord,
    ResourcePolicy, SourceArtifact, SourceFormat, SplitPolicy, WORKER_REQUEST_CONTRACT_VERSION,
    WORKER_RESPONSE_CONTRACT_VERSION, WorkerContext, WorkerNetworkPolicy, WorkerRequest,
    WorkerResponse, sha256_bytes,
};

const ADAPTER_ID: &str = "weftext.markdown-compatibility";
const ADAPTER_VERSION: &str = "1";
const WORKER_ID: &str = "weftext.markdown-compatibility-worker";
const WORKER_PROTOCOL_VERSION: &str = "weftext.markdown-compatibility-worker.v1";
const PARSER_COMPONENT_VERSION: &str = "weftext.markdown-compatibility-parser.v1";

/// Explicit, one-way Markdown compatibility importer.
///
/// This adapter never identifies Markdown as a managed workspace profile. It accepts one bounded
/// UTF-8 artifact and produces Weftext Import IR for the ordinary immutable preview/commit path.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownCompatibilityAdapter;

impl ImportAdapter for MarkdownCompatibilityAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            adapter_id: ADAPTER_ID.to_owned(),
            adapter_version: ADAPTER_VERSION.to_owned(),
            supported_format: SourceFormat::Markdown,
        }
    }

    fn probe(
        &self,
        source: &SourceArtifact,
        evidence_reader: &mut ProbeReader<'_>,
        limits: &ImportLimits,
    ) -> Result<FormatProbe, ImportError> {
        let evidence_bytes =
            evidence_reader.read_head(source.byte_length.min(limits.max_probe_bytes))?;
        let bounded_evidence = evidence_bytes.as_slice();
        let utf8 = std::str::from_utf8(bounded_evidence).ok();
        let extension_matches = matches!(
            source.extension_hint.as_deref(),
            Some("md" | "markdown" | "mdown" | "mkd")
        );
        let syntax_evidence = utf8.is_some_and(markdown_syntax_evidence);
        let active_content = utf8.is_some_and(contains_active_markdown_content);
        let contains_nul = bounded_evidence.contains(&0);
        let safe = utf8.is_some() && !contains_nul && !active_content;
        let mut mismatch_evidence = Vec::new();
        if !extension_matches {
            mismatch_evidence.push(format!(
                "bounded Markdown text has non-Markdown extension hint {:?}",
                source.extension_hint
            ));
        }
        let mut diagnostics = Vec::new();
        if utf8.is_none() {
            diagnostics.push(blocking_probe(
                "markdown_not_utf8",
                "explicit Markdown compatibility import requires exact UTF-8 source bytes",
            ));
        }
        if contains_nul {
            diagnostics.push(blocking_probe(
                "markdown_nul_byte",
                "Markdown source contains a forbidden NUL byte",
            ));
        }
        if active_content {
            diagnostics.push(blocking_probe(
                "markdown_active_content",
                "active HTML or javascript content requires a separately reviewed import route",
            ));
        }
        let signature_confidence = if extension_matches && syntax_evidence {
            9_500
        } else if extension_matches || syntax_evidence {
            8_000
        } else if utf8.is_some() {
            5_000
        } else {
            0
        };
        Ok(FormatProbe {
            contract_version: FORMAT_PROBE_CONTRACT_VERSION.to_owned(),
            adapter: self.descriptor(),
            source_digest: source.sha256.clone(),
            evidence: evidence_reader.evidence(),
            detected_format: if utf8.is_some() {
                SourceFormat::Markdown
            } else {
                SourceFormat::Unknown
            },
            signature_confidence: Confidence::from_basis_points(signature_confidence)?,
            parser_confidence: Confidence::from_basis_points(if safe { 9_000 } else { 0 })?,
            encryption: EncryptionState::NotEncrypted,
            signature_evidence: if syntax_evidence {
                vec!["bounded UTF-8 evidence contains recognized Markdown block syntax".to_owned()]
            } else if utf8.is_some() {
                vec!["bounded evidence is valid plain UTF-8 Markdown text".to_owned()]
            } else {
                Vec::new()
            },
            mismatch_evidence,
            active_content_detected: active_content,
            page_count: None,
            container_entry_count: None,
            safe_to_plan: safe,
            diagnostics,
        })
    }

    fn plan(
        &self,
        source: &SourceArtifact,
        probe: &FormatProbe,
        request: PlanRequest,
        limits: ImportLimits,
    ) -> Result<ImportPlan, ImportError> {
        if probe.detected_format != SourceFormat::Markdown || !probe.safe_to_plan {
            return Err(ImportError::new(
                ImportErrorCode::UnsupportedFormat,
                "Markdown compatibility import requires a safe bounded UTF-8 probe",
            ));
        }
        if !matches!(request.split_policy, SplitPolicy::SingleNode)
            || !matches!(
                request.resource_policy,
                ResourcePolicy::SkipAll | ResourcePolicy::ExtractAndRetainOriginal
            )
            || request.local_ocr_policy != LocalOcrPolicy::Never
            || !matches!(request.agent_enhancement, AgentEnhancementPolicy::Disabled)
            || !matches!(request.egress, EgressDisclosure::None)
        {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "Markdown v1 supports one node, no referenced-resource reads, no OCR, and no agent egress",
            ));
        }
        ImportPlan::create(
            source,
            probe,
            AdapterRoute {
                adapter: self.descriptor(),
                worker_id: WORKER_ID.to_owned(),
                worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
            },
            request,
            limits,
        )
    }

    fn worker_request(
        &self,
        source: &SourceArtifact,
        plan: &ImportPlan,
        source_locator: PortablePath,
    ) -> Result<WorkerRequest, ImportError> {
        Ok(WorkerRequest {
            contract_version: WORKER_REQUEST_CONTRACT_VERSION.to_owned(),
            request_id: format!("request-{}", &plan.plan_id[5..]),
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
            source: source.clone(),
            source_locator,
            plan: plan.clone(),
            network: WorkerNetworkPolicy::Denied,
            memory_limit_bytes: plan.limits.worker_memory_bytes,
            page_limit: plan.limits.max_pages,
            entry_limit: plan.limits.max_container_entries,
            output_byte_limit: plan.limits.max_total_output_bytes,
            format_options: markdown_format_options(),
        })
    }

    fn map_worker_response(
        &self,
        source: &SourceArtifact,
        _plan: &ImportPlan,
        response: WorkerResponse,
    ) -> Result<ImportDocument, ImportError> {
        if !response.resources.is_empty() {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "Markdown v1 worker cannot return resources from ambient filesystem paths",
            ));
        }
        let payload: MarkdownPayload =
            serde_json::from_value(response.payload).map_err(|error| {
                ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    format!("Markdown worker payload is invalid: {error}"),
                )
            })?;
        let provenance = extraction_provenance(source, Vec::new());
        ImportDocument::create(
            format!("document-{}", &source.sha256.as_str()[..24]),
            source.sha256.clone(),
            payload.title,
            payload.nodes,
            Vec::new(),
            response.diagnostics,
            vec![provenance],
        )
    }
}

/// Bounded in-process parser for inert UTF-8 Markdown text.
///
/// Unlike layout/container workers, this parser receives no network capability, resource path, or
/// workspace handle. It reads only the pipeline-owned exact source file and emits closed JSON IR.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownCompatibilityWorker;

impl FormatWorker for MarkdownCompatibilityWorker {
    fn worker_id(&self) -> &str {
        WORKER_ID
    }

    fn protocol_version(&self) -> &str {
        WORKER_PROTOCOL_VERSION
    }

    fn execute(
        &self,
        request: WorkerRequest,
        context: WorkerContext,
    ) -> Result<WorkerResponse, ImportError> {
        if request.format_options != markdown_format_options() {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "Markdown worker options differ from the reviewed compatibility profile",
            ));
        }
        let bytes = context.read_bounded(&request.source_locator, request.source.byte_length)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != request.source.byte_length
            || sha256_bytes(&bytes) != request.source.sha256
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "Markdown worker input differs from the frozen source artifact",
            ));
        }
        let source = std::str::from_utf8(&bytes).map_err(|_| {
            ImportError::new(
                ImportErrorCode::InvalidSource,
                "Markdown compatibility source must be UTF-8",
            )
        })?;
        if source.contains('\0') || contains_active_markdown_content(source) {
            return Err(ImportError::new(
                ImportErrorCode::InvalidSource,
                "Markdown source contains active or forbidden content",
            ));
        }
        let (payload, diagnostics) = parse_markdown(source, &request.source)?;
        let payload = serde_json::to_value(payload).map_err(|error| {
            ImportError::new(
                ImportErrorCode::WorkerProtocol,
                format!("cannot serialize Markdown worker payload: {error}"),
            )
        })?;
        Ok(WorkerResponse {
            contract_version: WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id: request.request_id,
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
            source_digest: request.source.sha256,
            payload,
            resources: Vec::new(),
            diagnostics,
            components: vec![ComponentVersion {
                component_id: WORKER_ID.to_owned(),
                version: PARSER_COMPONENT_VERSION.to_owned(),
                artifact_digest: None,
            }],
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MarkdownPayload {
    title: String,
    nodes: Vec<ImportNode>,
}

#[derive(Clone, Copy)]
struct SourceLine<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

struct MarkdownParser<'a> {
    source: &'a str,
    artifact: &'a SourceArtifact,
    lines: Vec<SourceLine<'a>>,
    index: usize,
    next_node: usize,
    title_line: Option<usize>,
    diagnostics: Vec<ImportDiagnostic>,
}

fn parse_markdown(
    source: &str,
    artifact: &SourceArtifact,
) -> Result<(MarkdownPayload, Vec<ImportDiagnostic>), ImportError> {
    let lines = source_lines(source);
    let title_line = lines
        .iter()
        .position(|line| atx_heading(line.text).is_some_and(|(level, _)| level == 1));
    let title = title_line
        .and_then(|index| atx_heading(lines[index].text).map(|(_, title)| title.to_owned()))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| source_title(&artifact.display_name));
    let mut parser = MarkdownParser {
        source,
        artifact,
        lines,
        index: 0,
        next_node: 1,
        title_line,
        diagnostics: Vec::new(),
    };
    let nodes = parser.parse_nodes()?;
    Ok((MarkdownPayload { title, nodes }, parser.diagnostics))
}

impl MarkdownParser<'_> {
    fn parse_nodes(&mut self) -> Result<Vec<ImportNode>, ImportError> {
        let mut nodes = Vec::new();
        while self.index < self.lines.len() {
            if self.lines[self.index].text.trim().is_empty() {
                self.index += 1;
                continue;
            }
            if self.index == 0 && self.lines[self.index].text.trim() == "---" {
                nodes.push(self.parse_frontmatter_literal()?);
                continue;
            }
            if Some(self.index) == self.title_line {
                self.index += 1;
                continue;
            }
            if fence_open(self.lines[self.index].text).is_some() {
                nodes.push(self.parse_fence()?);
                continue;
            }
            if let Some((level, title)) = atx_heading(self.lines[self.index].text) {
                let start = self.lines[self.index].start;
                let end = self.lines[self.index].end;
                let location = self.location(start, end);
                let title = lower_inline_markdown(title, &mut self.diagnostics, location);
                self.index += 1;
                nodes.push(self.node(
                    ImportNodeKind::Section {
                        level: level.saturating_sub(1).clamp(1, 8),
                        title,
                        children: Vec::new(),
                    },
                    start,
                    end,
                    9_800,
                )?);
                continue;
            }
            if let Some(level) = self.setext_level() {
                let first = self.lines[self.index];
                let second = self.lines[self.index + 1];
                let location = self.location(first.start, second.end);
                let title =
                    lower_inline_markdown(first.text.trim(), &mut self.diagnostics, location);
                self.index += 2;
                nodes.push(self.node(
                    ImportNodeKind::Section {
                        level,
                        title,
                        children: Vec::new(),
                    },
                    first.start,
                    second.end,
                    9_500,
                )?);
                continue;
            }
            if thematic_break(self.lines[self.index].text) {
                let line = self.lines[self.index];
                self.index += 1;
                nodes.push(self.node(
                    ImportNodeKind::ThematicBreak,
                    line.start,
                    line.end,
                    10_000,
                )?);
                continue;
            }
            if quote_line(self.lines[self.index].text).is_some() {
                nodes.extend(self.parse_quotes()?);
                continue;
            }
            if list_item(self.lines[self.index].text).is_some() {
                nodes.push(self.parse_list()?);
                continue;
            }
            if self.is_table_start() {
                nodes.push(self.parse_table()?);
                continue;
            }
            nodes.push(self.parse_paragraph()?);
        }
        Ok(nodes)
    }

    fn parse_frontmatter_literal(&mut self) -> Result<ImportNode, ImportError> {
        let start = self.lines[self.index].start;
        let mut closing = None;
        for candidate in self.index + 1..self.lines.len() {
            if self.lines[candidate].text.trim() == "---" {
                closing = Some(candidate);
                break;
            }
        }
        let end_index = closing.unwrap_or(self.lines.len().saturating_sub(1));
        let end = self.lines[end_index].end;
        self.diagnostic(
            "markdown_frontmatter_preserved_literal",
            DiagnosticSeverity::Warning,
            "Markdown frontmatter is preserved as literal evidence and is not imported as Weftext system metadata",
            start,
            end,
            None,
        );
        self.index = end_index.saturating_add(1);
        self.node(
            ImportNodeKind::Listing {
                language: Some("yaml".to_owned()),
                source: self.source[start..end]
                    .trim_end_matches(['\r', '\n'])
                    .to_owned(),
            },
            start,
            end,
            if closing.is_some() { 9_000 } else { 6_000 },
        )
    }

    fn parse_fence(&mut self) -> Result<ImportNode, ImportError> {
        let opening = self.lines[self.index];
        let (marker, count, language) = fence_open(opening.text).expect("checked fence");
        let mut closing = None;
        for candidate in self.index + 1..self.lines.len() {
            if fence_close(self.lines[candidate].text, marker, count) {
                closing = Some(candidate);
                break;
            }
        }
        let content_start = opening.end;
        let content_end = closing.map_or(self.source.len(), |index| self.lines[index].start);
        let end = closing.map_or(self.source.len(), |index| self.lines[index].end);
        if closing.is_none() {
            self.diagnostic(
                "markdown_unclosed_fence",
                DiagnosticSeverity::Warning,
                "unclosed Markdown fence was preserved through the end of the source",
                opening.start,
                end,
                None,
            );
        }
        self.index = closing.map_or(self.lines.len(), |index| index + 1);
        self.node(
            ImportNodeKind::Listing {
                language,
                source: self.source[content_start..content_end]
                    .trim_end_matches(['\r', '\n'])
                    .to_owned(),
            },
            opening.start,
            end,
            if closing.is_some() { 9_800 } else { 7_000 },
        )
    }

    fn parse_quotes(&mut self) -> Result<Vec<ImportNode>, ImportError> {
        let mut nodes = Vec::new();
        while self.index < self.lines.len() {
            let Some((depth, first_text)) = quote_line(self.lines[self.index].text) else {
                break;
            };
            let start = self.lines[self.index].start;
            let mut end = self.lines[self.index].end;
            let mut text = vec![first_text.to_owned()];
            self.index += 1;
            while self.index < self.lines.len() {
                let Some((next_depth, next_text)) = quote_line(self.lines[self.index].text) else {
                    break;
                };
                if next_depth != depth {
                    break;
                }
                end = self.lines[self.index].end;
                text.push(next_text.to_owned());
                self.index += 1;
            }
            let mut joined = text.join("\n");
            let location = self.location(start, end);
            joined = lower_inline_markdown(&joined, &mut self.diagnostics, location);
            nodes.push(self.node(
                ImportNodeKind::Quote {
                    depth: depth.min(9),
                    text: joined,
                },
                start,
                end,
                9_500,
            )?);
            if depth > 9 {
                self.diagnostic(
                    "markdown_quote_depth_lowered",
                    DiagnosticSeverity::Warning,
                    "Markdown quote depth greater than nine was lowered to the supported depth-nine profile",
                    start,
                    end,
                    nodes.last().map(|node| node.id.clone()),
                );
            }
        }
        Ok(nodes)
    }

    fn parse_list(&mut self) -> Result<ImportNode, ImportError> {
        let first = self.lines[self.index];
        let (ordered, _, first_text, first_indent) = list_item(first.text).expect("checked list");
        let start = first.start;
        let mut end = first.end;
        let mut items = vec![first_text.to_owned()];
        let mut nested = first_indent > 3;
        self.index += 1;
        while self.index < self.lines.len() {
            let Some((next_ordered, _, text, indent)) = list_item(self.lines[self.index].text)
            else {
                break;
            };
            if next_ordered != ordered {
                break;
            }
            nested |= indent > 3;
            end = self.lines[self.index].end;
            items.push(text.to_owned());
            self.index += 1;
        }
        let location = self.location(start, end);
        for item in &mut items {
            *item = lower_inline_markdown(item, &mut self.diagnostics, location.clone());
        }
        let node = self.node(
            ImportNodeKind::List { ordered, items },
            start,
            end,
            if nested { 8_000 } else { 9_500 },
        )?;
        if nested {
            self.diagnostic(
                "markdown_nested_list_flattened",
                DiagnosticSeverity::Warning,
                "nested Markdown list indentation was flattened by compatibility import v1",
                start,
                end,
                Some(node.id.clone()),
            );
        }
        Ok(node)
    }

    fn parse_table(&mut self) -> Result<ImportNode, ImportError> {
        let start = self.lines[self.index].start;
        let mut rows = vec![pipe_cells(self.lines[self.index].text)];
        self.index += 2;
        let mut end = self.lines[self.index.saturating_sub(1)].end;
        while self.index < self.lines.len() {
            let line = self.lines[self.index];
            if !line.text.contains('|') || line.text.trim().is_empty() {
                break;
            }
            let cells = pipe_cells(line.text);
            if cells.len() != rows[0].len() {
                self.diagnostic(
                    "markdown_table_width_mismatch",
                    DiagnosticSeverity::Warning,
                    "a mismatched Markdown table row ended the imported table",
                    line.start,
                    line.end,
                    None,
                );
                break;
            }
            rows.push(cells);
            end = line.end;
            self.index += 1;
        }
        let location = self.location(start, end);
        for cell in rows.iter_mut().flatten() {
            *cell = lower_inline_markdown(cell, &mut self.diagnostics, location.clone());
        }
        self.node(
            ImportNodeKind::Table {
                header_rows: 1,
                rows,
            },
            start,
            end,
            9_200,
        )
    }

    fn parse_paragraph(&mut self) -> Result<ImportNode, ImportError> {
        let first = self.lines[self.index];
        let start = first.start;
        let mut end = first.end;
        let mut lines = Vec::new();
        while self.index < self.lines.len() {
            let line = self.lines[self.index];
            if line.text.trim().is_empty() || (!lines.is_empty() && self.starts_block(self.index)) {
                break;
            }
            lines.push(line.text.trim_end().to_owned());
            end = line.end;
            self.index += 1;
        }
        let raw = lines.join("\n");
        if raw.trim_start().starts_with('<') {
            self.diagnostic(
                "markdown_html_preserved_text",
                DiagnosticSeverity::Warning,
                "inert raw HTML was preserved as text rather than rendered as active markup",
                start,
                end,
                None,
            );
        }
        let location = self.location(start, end);
        let text = lower_inline_markdown(&raw, &mut self.diagnostics, location);
        self.node(ImportNodeKind::Paragraph { text }, start, end, 8_800)
    }

    fn starts_block(&self, index: usize) -> bool {
        let text = self.lines[index].text;
        atx_heading(text).is_some()
            || fence_open(text).is_some()
            || thematic_break(text)
            || quote_line(text).is_some()
            || list_item(text).is_some()
            || self.is_table_start_at(index)
            || self.setext_level_at(index).is_some()
    }

    fn setext_level(&self) -> Option<u8> {
        self.setext_level_at(self.index)
    }

    fn setext_level_at(&self, index: usize) -> Option<u8> {
        let next = self.lines.get(index + 1)?.text.trim();
        if self.lines[index].text.trim().is_empty() || next.len() < 3 {
            return None;
        }
        if next.bytes().all(|byte| byte == b'=') || next.bytes().all(|byte| byte == b'-') {
            Some(1)
        } else {
            None
        }
    }

    fn is_table_start(&self) -> bool {
        self.is_table_start_at(self.index)
    }

    fn is_table_start_at(&self, index: usize) -> bool {
        let Some(next) = self.lines.get(index + 1) else {
            return false;
        };
        let header = pipe_cells(self.lines[index].text);
        let separator = pipe_cells(next.text);
        header.len() >= 2
            && header.len() == separator.len()
            && separator.iter().all(|cell| {
                let cell = cell.trim().trim_matches(':');
                cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
            })
    }

    fn node(
        &mut self,
        kind: ImportNodeKind,
        start: usize,
        end: usize,
        confidence: u16,
    ) -> Result<ImportNode, ImportError> {
        let id = format!("markdown-node-{}", self.next_node);
        self.next_node = self.next_node.saturating_add(1);
        let location = self.location(start, end);
        Ok(ImportNode {
            id,
            kind,
            confidence: Confidence::from_basis_points(confidence)?,
            source_locations: vec![location.clone()],
            provenance: vec![extraction_provenance(self.artifact, vec![location])],
        })
    }

    fn location(&self, start: usize, end: usize) -> ImportSourceLocation {
        ImportSourceLocation {
            source_digest: self.artifact.sha256.clone(),
            page: None,
            region: None,
            byte_start: Some(u64::try_from(start).unwrap_or(u64::MAX)),
            byte_end: Some(u64::try_from(end).unwrap_or(u64::MAX)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn diagnostic(
        &mut self,
        code: &str,
        severity: DiagnosticSeverity,
        message: &str,
        start: usize,
        end: usize,
        ir_node_id: Option<String>,
    ) {
        self.diagnostics.push(ImportDiagnostic {
            code: code.to_owned(),
            severity,
            message: message.to_owned(),
            source_location: Some(self.location(start, end)),
            ir_node_id,
        });
    }
}

fn source_lines(source: &str) -> Vec<SourceLine<'_>> {
    if source.is_empty() {
        return Vec::new();
    }
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0_usize;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let content_end = if index > start && bytes[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            lines.push(SourceLine {
                start,
                end: index + 1,
                text: &source[start..content_end],
            });
            start = index + 1;
        }
    }
    if start < source.len() {
        lines.push(SourceLine {
            start,
            end: source.len(),
            text: &source[start..],
        });
    }
    lines
}

fn atx_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len().saturating_sub(trimmed.len()) > 3 {
        return None;
    }
    let count = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=9).contains(&count)
        || trimmed
            .as_bytes()
            .get(count)
            .is_some_and(|byte| !byte.is_ascii_whitespace())
    {
        return None;
    }
    let title = trimmed[count..].trim().trim_end_matches('#').trim_end();
    Some((u8::try_from(count).unwrap_or(9), title))
}

fn fence_open(line: &str) -> Option<(u8, usize, Option<String>)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len().saturating_sub(trimmed.len()) > 3 {
        return None;
    }
    let marker = *trimmed.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let count = trimmed.bytes().take_while(|byte| *byte == marker).count();
    if count < 3 {
        return None;
    }
    let language = trimmed[count..]
        .trim()
        .split_ascii_whitespace()
        .next()
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-'))
        })
        .map(str::to_owned);
    Some((marker, count, language))
}

fn fence_close(line: &str, marker: u8, minimum: usize) -> bool {
    let trimmed = line.trim();
    let count = trimmed.bytes().take_while(|byte| *byte == marker).count();
    count >= minimum && trimmed[count..].trim().is_empty()
}

fn thematic_break(line: &str) -> bool {
    let compact = line
        .trim()
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    compact.len() >= 3
        && compact
            .first()
            .is_some_and(|first| matches!(first, b'-' | b'*' | b'_'))
        && compact.iter().all(|byte| byte == &compact[0])
}

fn quote_line(line: &str) -> Option<(u8, &str)> {
    let mut rest = line.trim_start_matches(' ');
    let mut depth = 0_u8;
    while let Some(after) = rest.strip_prefix('>') {
        depth = depth.saturating_add(1);
        rest = after.strip_prefix(' ').unwrap_or(after);
    }
    (depth > 0).then_some((depth, rest))
}

fn list_item(line: &str) -> Option<(bool, usize, &str, usize)> {
    let trimmed = line.trim_start_matches(' ');
    let indent = line.len().saturating_sub(trimmed.len());
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && matches!(bytes[0], b'-' | b'+' | b'*') && bytes[1].is_ascii_whitespace()
    {
        return Some((false, 1, trimmed[1..].trim_start(), indent));
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits > 0
        && digits <= 9
        && bytes
            .get(digits)
            .is_some_and(|byte| matches!(byte, b'.' | b')'))
        && bytes.get(digits + 1).is_some_and(u8::is_ascii_whitespace)
    {
        return Some((true, digits + 1, trimmed[digits + 1..].trim_start(), indent));
    }
    None
}

fn pipe_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim().trim_matches('|');
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in trimmed.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '|' {
            cells.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(character);
        }
    }
    if escaped {
        current.push('\\');
    }
    cells.push(current.trim().to_owned());
    cells
}

fn lower_inline_markdown(
    source: &str,
    diagnostics: &mut Vec<ImportDiagnostic>,
    location: ImportSourceLocation,
) -> String {
    let (without_images, images) = lower_inline_construct(source, true);
    let (without_links, links) = lower_inline_construct(&without_images, false);
    if images > 0 {
        diagnostics.push(ImportDiagnostic {
            code: "markdown_image_reference_omitted".to_owned(),
            severity: DiagnosticSeverity::Warning,
            message:
                "external Markdown image bytes were not read; alt text and target remain visible"
                    .to_owned(),
            source_location: Some(location.clone()),
            ir_node_id: None,
        });
    }
    if links > 0 {
        diagnostics.push(ImportDiagnostic {
            code: "markdown_link_lowered_text".to_owned(),
            severity: DiagnosticSeverity::Warning,
            message: "Markdown links were lowered to visible label and target text".to_owned(),
            source_location: Some(location),
            ir_node_id: None,
        });
    }
    without_links.replace("**", "*").replace("__", "_")
}

fn lower_inline_construct(source: &str, image: bool) -> (String, usize) {
    let prefix = if image { "![" } else { "[" };
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    let mut count = 0_usize;
    while let Some(start) = rest.find(prefix) {
        output.push_str(&rest[..start]);
        let candidate = &rest[start + prefix.len()..];
        let Some(label_end) = candidate.find("](") else {
            output.push_str(&rest[start..]);
            return (output, count);
        };
        let after_label = &candidate[label_end + 2..];
        let Some(target_end) = after_label.find(')') else {
            output.push_str(&rest[start..]);
            return (output, count);
        };
        let label = &candidate[..label_end];
        let target = &after_label[..target_end];
        if image {
            output.push_str("[Image omitted: ");
            output.push_str(if label.trim().is_empty() {
                "image"
            } else {
                label
            });
            output.push_str(" — ");
            output.push_str(target);
            output.push(']');
        } else {
            output.push_str(label);
            output.push_str(" (");
            output.push_str(target);
            output.push(')');
        }
        count = count.saturating_add(1);
        rest = &after_label[target_end + 1..];
    }
    output.push_str(rest);
    (output, count)
}

fn extraction_provenance(
    source: &SourceArtifact,
    source_locations: Vec<ImportSourceLocation>,
) -> ProvenanceRecord {
    ProvenanceRecord {
        kind: ProvenanceKind::LocalExtraction,
        component_id: WORKER_ID.to_owned(),
        component_version: PARSER_COMPONENT_VERSION.to_owned(),
        input_digests: vec![source.sha256.clone()],
        output_digest: None,
        source_locations,
    }
}

fn markdown_format_options() -> serde_json::Value {
    json!({
        "dialect": "weftext.markdown-compatibility.v1",
        "rawHtml": "inert-text-or-reject-active",
        "externalResources": "omit",
        "frontmatter": "preserve-literal",
    })
}

fn markdown_syntax_evidence(source: &str) -> bool {
    source.lines().any(|line| {
        atx_heading(line).is_some()
            || fence_open(line).is_some()
            || quote_line(line).is_some()
            || list_item(line).is_some()
            || thematic_break(line)
    })
}

fn contains_active_markdown_content(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    [
        "<script",
        "<iframe",
        "<object",
        "<embed",
        "javascript:",
        "vbscript:",
        "data:text/html",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn source_title(display_name: &str) -> String {
    let stem = display_name
        .rsplit_once('.')
        .map_or(display_name, |(stem, _)| stem)
        .trim();
    if stem.is_empty() {
        "Imported Markdown".to_owned()
    } else {
        stem.to_owned()
    }
}

fn blocking_probe(code: &str, message: &str) -> ImportDiagnostic {
    ImportDiagnostic {
        code: code.to_owned(),
        severity: DiagnosticSeverity::Blocking,
        message: message.to_owned(),
        source_location: None,
        ir_node_id: None,
    }
}

/// Returns the exact adapter descriptor used by immutable preview validation.
#[must_use]
pub fn markdown_compatibility_descriptor() -> AdapterDescriptor {
    MarkdownCompatibilityAdapter.descriptor()
}

/// Validates immutable route, policy, and component evidence without re-running conversion.
///
/// # Errors
///
/// Rejects any field outside the reviewed Markdown compatibility v1 profile.
pub fn validate_markdown_compatibility_preview_evidence(
    probe: &FormatProbe,
    plan: &ImportPlan,
    components: &[ComponentVersion],
) -> Result<(), ImportError> {
    let expected_component = [ComponentVersion {
        component_id: WORKER_ID.to_owned(),
        version: PARSER_COMPONENT_VERSION.to_owned(),
        artifact_digest: None,
    }];
    if probe.adapter != MarkdownCompatibilityAdapter.descriptor()
        || probe.detected_format != SourceFormat::Markdown
        || !probe.safe_to_plan
        || probe.active_content_detected
        || plan.route.adapter != probe.adapter
        || plan.route.worker_id != WORKER_ID
        || plan.route.worker_protocol_version != WORKER_PROTOCOL_VERSION
        || !matches!(plan.split_policy, SplitPolicy::SingleNode)
        || !matches!(
            plan.resource_policy,
            ResourcePolicy::SkipAll | ResourcePolicy::ExtractAndRetainOriginal
        )
        || plan.local_ocr_policy != LocalOcrPolicy::Never
        || !matches!(plan.agent_enhancement, AgentEnhancementPolicy::Disabled)
        || !matches!(plan.egress, EgressDisclosure::None)
        || components != expected_component
    {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "Markdown preview evidence differs from the reviewed compatibility v1 route",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{MarkdownCompatibilityAdapter, MarkdownCompatibilityWorker};
    use crate::{
        AsciiDocV1ProposalValidator, CancellationToken, ImportErrorCode, ImportLimits,
        ImportPipeline, ImportTempRoot, IntakeRequest, OriginClass, PlanRequest, PortablePath,
        ResourcePolicy,
    };

    fn request() -> PlanRequest {
        let mut request =
            PlanRequest::single_node(PortablePath::parse("Imported").expect("destination"));
        request.resource_policy = ResourcePolicy::SkipAll;
        request.local_ocr_policy = crate::LocalOcrPolicy::Never;
        request
    }

    #[test]
    fn explicit_markdown_enters_ir_and_renders_only_canonical_asciidoc() {
        let temporary = tempfile::tempdir().expect("temporary");
        let pipeline = ImportPipeline::new(
            ImportTempRoot::initialize(temporary.path().join("intake")).unwrap(),
            Arc::new(AsciiDocV1ProposalValidator),
        );
        let source = concat!(
            "---\r\n",
            "status: draft\r\n",
            "---\r\n",
            "# 文缕导入 😀\r\n",
            "## 第二级\r\n",
            "正文 **加粗** 与 [站点](https://example.test)。\r\n\r\n",
            "> 引用\r\n",
            "- [ ] 待办\r\n",
            "- 完成项\r\n\r\n",
            "| 名称 | 值 |\r\n",
            "| --- | ---: |\r\n",
            "| 中文 | ✅ |\r\n\r\n",
            "```rust\r\n",
            "fn main() {}\r\n",
            "```\r\n",
            "![替代文字](missing.png)\r\n",
        );
        let preview = pipeline
            .preview(
                IntakeRequest {
                    display_name: "输入.MD".to_owned(),
                    origin: OriginClass::TestFixture,
                    bytes: source.as_bytes().to_vec(),
                    plan: request(),
                    limits: ImportLimits::default(),
                    cancellation: CancellationToken::default(),
                },
                &MarkdownCompatibilityAdapter,
                Arc::new(MarkdownCompatibilityWorker),
            )
            .expect("Markdown preview");
        let exact = &preview.proposal.proposal().nodes[0].exact_asciidoc;
        assert!(exact.starts_with("---\nweftext:\n  id: \""));
        assert_eq!(exact.matches("\nweftext:\n").count(), 1);
        assert!(exact.contains("= 文缕导入 😀"));
        assert!(exact.contains("== 第二级"));
        assert!(exact.contains("正文 *加粗* 与 站点 (https://example.test)"));
        assert!(exact.contains("> 引用"));
        assert!(exact.contains("* [ ] 待办"));
        assert!(exact.contains("[source,rust]\n----\nfn main() {}\n----"));
        assert!(exact.contains("[source,yaml]"));
        assert!(!exact.contains("\nstatus: draft\n"));
        assert!(
            preview
                .proposal
                .proposal()
                .warnings
                .iter()
                .any(|warning| warning.contains("markdown_frontmatter_preserved_literal"))
        );
        assert!(
            preview
                .proposal
                .proposal()
                .warnings
                .iter()
                .any(|warning| warning.contains("markdown_image_reference_omitted"))
        );
    }

    #[test]
    fn active_content_and_invalid_utf8_fail_before_a_plan_can_run() {
        let temporary = tempfile::tempdir().expect("temporary");
        let pipeline = ImportPipeline::new(
            ImportTempRoot::initialize(temporary.path().join("intake")).unwrap(),
            Arc::new(AsciiDocV1ProposalValidator),
        );
        for bytes in [
            b"# Bad\n<script>alert(1)</script>\n".to_vec(),
            vec![0xff, 0xfe, 0xfd],
        ] {
            let error = pipeline
                .preview(
                    IntakeRequest {
                        display_name: "unsafe.md".to_owned(),
                        origin: OriginClass::TestFixture,
                        bytes,
                        plan: request(),
                        limits: ImportLimits::default(),
                        cancellation: CancellationToken::default(),
                    },
                    &MarkdownCompatibilityAdapter,
                    Arc::new(MarkdownCompatibilityWorker),
                )
                .expect_err("unsafe Markdown");
            assert!(matches!(
                error.code(),
                ImportErrorCode::ProbeRejected | ImportErrorCode::InvalidSource
            ));
        }
    }
}
