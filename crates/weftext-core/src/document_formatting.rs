use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    AdjacentHeadingBody, DocumentBlock, DocumentBlockKind, DocumentProfileId,
    analyze_document_for_profile,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocumentFormatCommand {
    Bold,
    Emphasis,
    InlineCode,
    Link,
    Image { target: String, alt: String },
    Heading { level: u8 },
    Paragraph,
    List,
    QuoteIncrease,
    QuoteDecrease,
    CodeBlock,
    TableInsert,
    TableAddRow,
    TableAddColumn,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFormatPlan {
    pub profile: DocumentProfileId,
    pub source: String,
    pub selection_start: u64,
    pub selection_end: u64,
    pub changed: bool,
}

/// Applies one deterministic canonical-AsciiDoc formatting command to exact UTF-8 source.
///
/// The selection uses UTF-8 byte offsets. Protected regions and non-character boundaries
/// fail closed; callers never need their own `AsciiDoc` formatting rules.
///
/// # Errors
///
/// Returns an error for invalid selections, protected metadata, missing semantic blocks,
/// or a table command outside a table block.
#[allow(clippy::too_many_lines)]
pub fn plan_document_format(
    profile: DocumentProfileId,
    source: &str,
    selection_start: u64,
    selection_end: u64,
    command: DocumentFormatCommand,
) -> Result<DocumentFormatPlan, DocumentFormatError> {
    let start = usize::try_from(selection_start).map_err(|_| DocumentFormatError::InvalidRange)?;
    let end = usize::try_from(selection_end).map_err(|_| DocumentFormatError::InvalidRange)?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(DocumentFormatError::InvalidRange);
    }
    let analysis = analyze_document_for_profile(profile, source, AdjacentHeadingBody::Separate);
    if !selection_is_eligible(
        selection_start,
        selection_end,
        &analysis.occurrences.eligible_text_ranges,
    ) {
        return Err(DocumentFormatError::ProtectedRegion);
    }

    let (next_source, next_start, next_end) = match command {
        inline_command @ (DocumentFormatCommand::Bold
        | DocumentFormatCommand::Emphasis
        | DocumentFormatCommand::InlineCode
        | DocumentFormatCommand::Link) => {
            let (prefix, suffix, placeholder) = inline_format(profile, &inline_command);
            let content = if start == end {
                placeholder
            } else {
                &source[start..end]
            };
            let replacement = format!("{prefix}{content}{suffix}");
            (
                replace_range(source, start, end, &replacement),
                start + prefix.len(),
                start + prefix.len() + content.len(),
            )
        }
        DocumentFormatCommand::TableInsert => {
            let ending = preferred_line_ending(source, start);
            let replacement = table_template(profile, ending);
            let next_end = start + replacement.len();
            (
                replace_range(source, start, end, &replacement),
                start,
                next_end,
            )
        }
        DocumentFormatCommand::Image { target, alt } => {
            if target.is_empty()
                || target.contains('\r')
                || target.contains('\n')
                || alt.contains('\r')
                || alt.contains('\n')
            {
                return Err(DocumentFormatError::InvalidResourceReference);
            }
            let target = encode_relative_target(&target);
            let alt = alt.replace(']', "\\]");
            let replacement = format!("image::{target}[{alt}]");
            let next_end = start + replacement.len();
            (
                replace_range(source, start, end, &replacement),
                start,
                next_end,
            )
        }
        table_command @ (DocumentFormatCommand::TableAddRow
        | DocumentFormatCommand::TableAddColumn) => {
            let block = semantic_block(
                &analysis.model.blocks,
                selection_start,
                Some(DocumentBlockKind::Table),
            )?;
            let block_start =
                usize::try_from(block.start).map_err(|_| DocumentFormatError::InvalidRange)?;
            let block_end =
                usize::try_from(block.end).map_err(|_| DocumentFormatError::InvalidRange)?;
            let original = &source[block_start..block_end];
            let ending = preferred_line_ending(source, block_start);
            let replacement = format_table(
                profile,
                original,
                ending,
                table_command == DocumentFormatCommand::TableAddColumn,
            )?;
            let next_end = block_start + replacement.len();
            (
                replace_range(source, block_start, block_end, &replacement),
                block_start,
                next_end,
            )
        }
        block_command => {
            let block = semantic_block(&analysis.model.blocks, selection_start, None)?;
            let block_start =
                usize::try_from(block.start).map_err(|_| DocumentFormatError::InvalidRange)?;
            let block_end =
                usize::try_from(block.end).map_err(|_| DocumentFormatError::InvalidRange)?;
            let original = &source[block_start..block_end];
            let ending = preferred_line_ending(source, block_start);
            let replacement = format_block(profile, original, ending, &block_command)?;
            let next_end = block_start + replacement.len();
            (
                replace_range(source, block_start, block_end, &replacement),
                block_start,
                next_end,
            )
        }
    };

    Ok(DocumentFormatPlan {
        profile,
        changed: next_source != source,
        source: next_source,
        selection_start: next_start as u64,
        selection_end: next_end as u64,
    })
}

fn selection_is_eligible(start: u64, end: u64, eligible: &[std::ops::Range<u64>]) -> bool {
    eligible.iter().any(|range| {
        if start == end {
            range.start <= start && start <= range.end
        } else {
            range.start <= start && end <= range.end
        }
    })
}

fn semantic_block(
    blocks: &[DocumentBlock],
    offset: u64,
    expected: Option<DocumentBlockKind>,
) -> Result<&DocumentBlock, DocumentFormatError> {
    blocks
        .iter()
        .find(|block| {
            block.kind != DocumentBlockKind::Frontmatter
                && expected.is_none_or(|kind| block.kind == kind)
                && block.start <= offset
                && offset <= block.end
        })
        .ok_or(if expected == Some(DocumentBlockKind::Table) {
            DocumentFormatError::TableRequired
        } else {
            DocumentFormatError::BlockRequired
        })
}

fn inline_format(
    _profile: DocumentProfileId,
    command: &DocumentFormatCommand,
) -> (&'static str, &'static str, &'static str) {
    match command {
        DocumentFormatCommand::Bold => ("*", "*", "加粗文本"),
        DocumentFormatCommand::Emphasis => ("_", "_", "强调文本"),
        DocumentFormatCommand::InlineCode => ("`", "`", "代码"),
        DocumentFormatCommand::Link => ("https://[", "]", "链接文本"),
        _ => unreachable!("only inline commands reach inline_format"),
    }
}

fn format_block(
    profile: DocumentProfileId,
    source: &str,
    line_ending: &str,
    command: &DocumentFormatCommand,
) -> Result<String, DocumentFormatError> {
    if let DocumentFormatCommand::Heading { level } = command {
        if !(1..=9).contains(level) {
            return Err(DocumentFormatError::InvalidHeadingLevel);
        }
        let first = exact_lines(source).into_iter().next().unwrap_or_default();
        let indent_len = first.text.len() - first.text.trim_start_matches(' ').len();
        let indent = &first.text[..indent_len.min(3)];
        let body = strip_semantic_prefix(profile, &first.text[indent.len()..]);
        let marker = "=".repeat(*level as usize + 1);
        let mut replacement = format!("{indent}{marker} {}", body.trim_start());
        replacement.push_str(first.ending);
        for line in exact_lines(source).into_iter().skip(1) {
            replacement.push_str(line.text);
            replacement.push_str(line.ending);
        }
        return Ok(replacement);
    }

    let lines = exact_lines(source);
    let map_lines = |transform: fn(DocumentProfileId, &str) -> String| {
        transform_lines(profile, &lines, transform)
    };
    match command {
        DocumentFormatCommand::Paragraph => Ok(map_lines(paragraph_line)),
        DocumentFormatCommand::List => Ok(map_lines(list_line)),
        DocumentFormatCommand::QuoteIncrease => Ok(map_lines(quote_increase_line)),
        DocumentFormatCommand::QuoteDecrease => Ok(map_lines(quote_decrease_line)),
        DocumentFormatCommand::CodeBlock => Ok(toggle_code_block(profile, source, line_ending)),
        _ => Err(DocumentFormatError::UnsupportedCommand),
    }
}

fn transform_lines(
    profile: DocumentProfileId,
    lines: &[ExactLine<'_>],
    transform: fn(DocumentProfileId, &str) -> String,
) -> String {
    let mut output = String::new();
    for line in lines {
        output.push_str(&transform(profile, line.text));
        output.push_str(line.ending);
    }
    output
}

fn quote_increase_line(_profile: DocumentProfileId, line: &str) -> String {
    let indent_len = (line.len() - line.trim_start_matches(' ').len()).min(3);
    format!("{}> {}", &line[..indent_len], &line[indent_len..])
}

fn quote_decrease_line(_profile: DocumentProfileId, line: &str) -> String {
    let indent_len = (line.len() - line.trim_start_matches(' ').len()).min(3);
    let rest = &line[indent_len..];
    let rest = rest
        .strip_prefix('>')
        .map_or(rest, |value| value.strip_prefix(' ').unwrap_or(value));
    format!("{}{rest}", &line[..indent_len])
}

fn paragraph_line(profile: DocumentProfileId, line: &str) -> String {
    let indent_len = (line.len() - line.trim_start_matches(' ').len()).min(3);
    format!(
        "{}{}",
        &line[..indent_len],
        strip_semantic_prefix(profile, &line[indent_len..])
    )
}

fn list_line(profile: DocumentProfileId, line: &str) -> String {
    let indent_len = line.len() - line.trim_start_matches(' ').len();
    let indent_len = indent_len.min(3);
    let marker = "* ";
    format!(
        "{}{marker}{}",
        &line[..indent_len],
        strip_semantic_prefix(profile, &line[indent_len..]).trim_start()
    )
}

fn strip_semantic_prefix(profile: DocumentProfileId, line: &str) -> &str {
    let rest = match profile {
        DocumentProfileId::AsciiDocV1 => strip_heading(line, '=', 2),
    };
    let rest = strip_quotes(rest);
    strip_list(rest)
}

fn strip_heading(value: &str, marker: char, minimum: usize) -> &str {
    let count = value
        .chars()
        .take_while(|character| *character == marker)
        .count();
    if (minimum..=10).contains(&count) {
        let rest = &value[count..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
            return rest.trim_start_matches([' ', '\t']);
        }
    }
    value
}

fn strip_quotes(mut value: &str) -> &str {
    while let Some(rest) = value.strip_prefix('>') {
        value = rest.strip_prefix(' ').unwrap_or(rest);
    }
    value
}

fn strip_list(value: &str) -> &str {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = value.strip_prefix(marker) {
            return rest;
        }
    }
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        let rest = &value[digits..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return rest;
        }
    }
    value
}

fn toggle_code_block(profile: DocumentProfileId, source: &str, line_ending: &str) -> String {
    let lines = exact_lines(source);
    match profile {
        DocumentProfileId::AsciiDocV1 => {
            if lines.len() >= 2 && lines.last().is_some_and(|line| line.text.trim() == "----") {
                let content_start = if lines[0].text.trim().starts_with('[')
                    && lines.get(1).is_some_and(|line| line.text.trim() == "----")
                {
                    2
                } else {
                    usize::from(lines[0].text.trim() == "----")
                };
                if content_start > 0 {
                    return concatenate_lines(&lines[content_start..lines.len() - 1]);
                }
            }
            format!(
                "[source]{line_ending}----{line_ending}{source}{}----{line_ending}",
                if source.ends_with('\n') || source.ends_with('\r') {
                    ""
                } else {
                    line_ending
                }
            )
        }
    }
}

fn concatenate_lines(lines: &[ExactLine<'_>]) -> String {
    let mut output = String::new();
    for line in lines {
        output.push_str(line.text);
        output.push_str(line.ending);
    }
    output
}

fn table_template(profile: DocumentProfileId, ending: &str) -> String {
    match profile {
        DocumentProfileId::AsciiDocV1 => {
            format!("|==={ending}|列 1 |列 2{ending}{ending}|  |  {ending}|==={ending}")
        }
    }
}

fn encode_relative_target(target: &str) -> String {
    let mut encoded = String::new();
    for byte in target.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn format_table(
    profile: DocumentProfileId,
    source: &str,
    ending: &str,
    add_column: bool,
) -> Result<String, DocumentFormatError> {
    let lines = exact_lines(source);
    if lines.len() < 2 {
        return Err(DocumentFormatError::MalformedTable);
    }
    match profile {
        DocumentProfileId::AsciiDocV1 => {
            let closing = lines
                .iter()
                .rposition(|line| line.text.trim() == "|===")
                .ok_or(DocumentFormatError::MalformedTable)?;
            if !add_column {
                let columns = lines
                    .iter()
                    .skip(1)
                    .take(closing.saturating_sub(1))
                    .find(|line| line.text.contains('|'))
                    .map_or(2, |line| line.text.matches('|').count().max(1));
                let row = (0..columns).map(|_| "|  ").collect::<String>();
                let mut output = String::new();
                for (index, line) in lines.iter().enumerate() {
                    if index == closing {
                        output.push_str(&row);
                        output.push_str(ending);
                    }
                    output.push_str(line.text);
                    output.push_str(line.ending);
                }
                return Ok(output);
            }
            let mut output = String::new();
            for (index, line) in lines.iter().enumerate() {
                output.push_str(line.text);
                if index > 0 && index < closing && !line.text.trim().is_empty() {
                    output.push_str(" |  ");
                }
                output.push_str(line.ending);
            }
            Ok(output)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ExactLine<'a> {
    text: &'a str,
    ending: &'a str,
}

fn exact_lines(source: &str) -> Vec<ExactLine<'_>> {
    if source.is_empty() {
        return vec![ExactLine::default()];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            let text_end = if index > start && source.as_bytes()[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            lines.push(ExactLine {
                text: &source[start..text_end],
                ending: &source[text_end..=index],
            });
            start = index + 1;
        }
    }
    if start < source.len() {
        let text_end = if source.as_bytes().last() == Some(&b'\r') {
            source.len() - 1
        } else {
            source.len()
        };
        lines.push(ExactLine {
            text: &source[start..text_end],
            ending: &source[text_end..],
        });
    }
    lines
}

fn preferred_line_ending(source: &str, offset: usize) -> &str {
    let before = &source[..offset.min(source.len())];
    if before.rfind("\r\n").is_some() {
        "\r\n"
    } else if before.rfind('\n').is_some() {
        "\n"
    } else if before.rfind('\r').is_some() {
        "\r"
    } else if source.contains("\r\n") {
        "\r\n"
    } else if source.contains('\n') {
        "\n"
    } else if source.contains('\r') {
        "\r"
    } else {
        "\n"
    }
}

fn replace_range(source: &str, start: usize, end: usize, replacement: &str) -> String {
    let mut result = String::with_capacity(source.len() + replacement.len());
    result.push_str(&source[..start]);
    result.push_str(replacement);
    result.push_str(&source[end..]);
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentFormatError {
    InvalidRange,
    ProtectedRegion,
    BlockRequired,
    TableRequired,
    MalformedTable,
    InvalidHeadingLevel,
    UnsupportedCommand,
    InvalidResourceReference,
}

impl fmt::Display for DocumentFormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRange => "format selection is not a valid UTF-8 byte range",
            Self::ProtectedRegion => "format selection intersects protected document source",
            Self::BlockRequired => "format command requires a Core semantic block",
            Self::TableRequired => "table command requires a Core table block",
            Self::MalformedTable => "Core table block does not match the selected profile",
            Self::InvalidHeadingLevel => "heading level must be between 1 and 9",
            Self::UnsupportedCommand => "format command is not supported in this context",
            Self::InvalidResourceReference => "resource reference contains invalid line breaks",
        })
    }
}

impl std::error::Error for DocumentFormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    const ENVELOPE: &str = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n";

    #[test]
    fn inline_commands_are_utf8_exact() {
        let source = format!("{ENVELOPE}== 标题\n正文 😀\n");
        let start = source.find('😀').expect("emoji");
        let end = start + '😀'.len_utf8();
        let plan = plan_document_format(
            DocumentProfileId::AsciiDocV1,
            &source,
            start as u64,
            end as u64,
            DocumentFormatCommand::Bold,
        )
        .expect("format");
        assert!(plan.source.contains("正文 *😀*"));
        let selection_start = usize::try_from(plan.selection_start).expect("selection start");
        let selection_end = usize::try_from(plan.selection_end).expect("selection end");
        assert_eq!(&plan.source[selection_start..selection_end], "😀");
    }

    #[test]
    fn headings_code_and_tables_use_canonical_asciidoc() {
        let asciidoc = format!("{ENVELOPE}plain\n");
        let asciidoc_heading = plan_document_format(
            DocumentProfileId::AsciiDocV1,
            &asciidoc,
            ENVELOPE.len() as u64,
            ENVELOPE.len() as u64,
            DocumentFormatCommand::Heading { level: 3 },
        )
        .expect("AsciiDoc heading");
        assert!(asciidoc_heading.source.ends_with("==== plain\n"));

        let table = plan_document_format(
            DocumentProfileId::AsciiDocV1,
            &asciidoc,
            ENVELOPE.len() as u64,
            ENVELOPE.len() as u64,
            DocumentFormatCommand::TableInsert,
        )
        .expect("AsciiDoc table");
        assert!(table.source.contains("|===\n|列 1 |列 2"));

        let image = plan_document_format(
            DocumentProfileId::AsciiDocV1,
            &asciidoc,
            ENVELOPE.len() as u64,
            ENVELOPE.len() as u64,
            DocumentFormatCommand::Image {
                target: "图 示.png".to_owned(),
                alt: "图示".to_owned(),
            },
        )
        .expect("AsciiDoc image");
        assert!(
            image
                .source
                .contains("image::%E5%9B%BE%20%E7%A4%BA.png[图示]")
        );
    }

    #[test]
    fn protected_metadata_and_non_utf8_boundaries_fail_closed() {
        let source = format!("{ENVELOPE}正文 😀\n");
        assert_eq!(
            plan_document_format(
                DocumentProfileId::AsciiDocV1,
                &source,
                4,
                4,
                DocumentFormatCommand::Bold,
            ),
            Err(DocumentFormatError::ProtectedRegion)
        );
        let emoji = source.find('😀').expect("emoji");
        assert_eq!(
            plan_document_format(
                DocumentProfileId::AsciiDocV1,
                &source,
                (emoji + 1) as u64,
                (emoji + 1) as u64,
                DocumentFormatCommand::Bold,
            ),
            Err(DocumentFormatError::InvalidRange)
        );
    }
}
