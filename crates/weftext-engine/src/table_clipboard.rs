//! Bounded plain-text table interchange. These helpers never interpret Typst or HTML.

use crate::{EngineError, UiMessage};

const MAX_INPUT_BYTES: usize = 1_048_576;
const MAX_CELLS: usize = 100_000;

#[derive(Clone, Copy, PartialEq)]
enum FieldState {
    Start,
    Plain,
    Quoted,
    Closed,
}

/// Read a rectangular TSV clipboard payload, including Excel-style quoted fields.
///
/// Delimiters and CR/LF inside quotes are literal cell content. Outside quotes,
/// CRLF, LF and CR terminate records; a final record terminator does not add an
/// extra row. Empty input represents one empty cell. Invalid quoting and ragged
/// rows are rejected before the caller changes the author's source.
pub(super) fn parse_tsv(text: &str) -> Result<Vec<Vec<String>>, EngineError> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_table_clipboard_table_clipboard_byte_budget(),
        ));
    }
    if text.contains('\0') {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_table_clipboard_table_clipboard_nul_unsupported(),
        ));
    }

    let mut chars = text.chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut state = FieldState::Start;
    let mut cells = 0;
    let mut ended_record = false;

    while let Some(ch) = chars.next() {
        if state == FieldState::Quoted {
            if ch == '"' {
                state = FieldState::Closed;
            } else {
                field.push(ch);
            }
            ended_record = false;
            continue;
        }
        match ch {
            '\t' => {
                push_field(&mut row, &mut field, &mut cells)?;
                state = FieldState::Start;
                ended_record = false;
            }
            '\r' | '\n' => {
                if ch == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                push_field(&mut row, &mut field, &mut cells)?;
                push_row(&mut rows, &mut row)?;
                state = FieldState::Start;
                ended_record = true;
            }
            '"' if state == FieldState::Start => {
                state = FieldState::Quoted;
                ended_record = false;
            }
            '"' if state == FieldState::Closed => {
                field.push('"');
                state = FieldState::Quoted;
                ended_record = false;
            }
            _ if state == FieldState::Closed || ch == '"' => {
                return Err(EngineError::new(weftext_messages::UiMessage::error_engine_table_clipboard_table_clipboard_quotes_invalid()));
            }
            _ => {
                field.push(ch);
                state = FieldState::Plain;
                ended_record = false;
            }
        }
    }
    if state == FieldState::Quoted {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_table_clipboard_table_clipboard_quote_unterminated(),
        ));
    }
    if !ended_record {
        push_field(&mut row, &mut field, &mut cells)?;
        push_row(&mut rows, &mut row)?;
    }
    Ok(rows)
}

fn push_field(
    row: &mut Vec<String>,
    field: &mut String,
    cells: &mut usize,
) -> Result<(), EngineError> {
    if *cells >= MAX_CELLS {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_table_clipboard_table_clipboard_cell_budget(),
        ));
    }
    row.push(std::mem::take(field));
    *cells += 1;
    Ok(())
}

fn push_row(rows: &mut Vec<Vec<String>>, row: &mut Vec<String>) -> Result<(), EngineError> {
    if let Some(first) = rows.first()
        && first.len() != row.len()
    {
        return Err(EngineError::new(
            UiMessage::error_engine_table_clipboard_row_not_rectangular(
                weftext_messages::params::ErrorEngineTableClipboardRowNotRectangular {
                    row: weftext_messages::UiInteger::new((rows.len() + 1) as i64)
                        .expect("clipboard row count is bounded by the cell budget"),
                },
            ),
        ));
    }
    rows.push(std::mem::take(row));
    Ok(())
}

/// Encode TSV (`'\t'`) or CSV (`','`) without losing embedded delimiters or EOLs.
/// The caller validates the source table and output budget before clipboard I/O.
#[cfg(test)]
pub(super) fn encode_delimited(rows: &[Vec<String>], separator: char) -> String {
    let mut out = String::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row_index > 0 {
            out.push('\n');
        }
        for (column, field) in row.iter().enumerate() {
            if column > 0 {
                out.push(separator);
            }
            // An empty final single-cell record must be quoted: otherwise its
            // preceding LF would be indistinguishable from a record terminator.
            let empty_final_record =
                field.is_empty() && row.len() == 1 && row_index + 1 == rows.len();
            if empty_final_record
                || field
                    .chars()
                    .any(|ch| ch == separator || matches!(ch, '"' | '\r' | '\n'))
            {
                out.push('"');
                for ch in field.chars() {
                    if ch == '"' {
                        out.push('"');
                    }
                    out.push(ch);
                }
                out.push('"');
            } else {
                out.push_str(field);
            }
        }
    }
    out
}

/// Escape author text for an HTML text node or quoted attribute value.
#[cfg(test)]
pub(super) fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect()
    }

    #[test]
    fn quoted_tabs_quotes_newlines_and_unicode_are_literal() {
        let input = "\"中\t文\"\t\"a\"\"b\"\r\n\"一\r\n二\"\t\"\"\r\n";
        assert_eq!(
            parse_tsv(input).unwrap(),
            matrix(&[&["中\t文", "a\"b"], &["一\r\n二", ""]])
        );
    }

    #[test]
    fn empty_cells_and_final_terminator_have_distinct_meanings() {
        assert_eq!(parse_tsv("").unwrap(), matrix(&[&[""]]));
        assert_eq!(parse_tsv("\t\r\n").unwrap(), matrix(&[&["", ""]]));
        assert_eq!(parse_tsv("\n\n").unwrap(), matrix(&[&[""], &[""]]));
        assert_eq!(
            parse_tsv("a\tb\n\t").unwrap(),
            matrix(&[&["a", "b"], &["", ""]])
        );
        assert_eq!(parse_tsv("a\rb\n").unwrap(), matrix(&[&["a"], &["b"]]));
    }

    #[test]
    fn malformed_quotes_ragged_rows_and_nul_are_rejected() {
        for input in [
            "\"unfinished",
            "a\"b",
            "\"a\"trailer",
            "\"a\" \tvalue",
            "a\tb\nc",
            "a\tb\n\n",
            "a\0b",
        ] {
            assert!(parse_tsv(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn byte_and_cell_budgets_are_checked_before_acceptance() {
        assert!(parse_tsv(&"x".repeat(MAX_INPUT_BYTES)).is_ok());
        assert!(parse_tsv(&"x".repeat(MAX_INPUT_BYTES + 1)).is_err());
        assert!(parse_tsv(&"中".repeat(MAX_INPUT_BYTES / 3 + 1)).is_err());
        assert!(parse_tsv(&"\t".repeat(MAX_CELLS - 1)).is_ok());
        assert!(parse_tsv(&"\t".repeat(MAX_CELLS)).is_err());
        assert!(parse_tsv(&"\n".repeat(MAX_CELLS)).is_ok());
        assert!(parse_tsv(&"\n".repeat(MAX_CELLS + 1)).is_err());
    }

    #[test]
    fn tsv_roundtrip_preserves_empty_rows_and_special_text() {
        for rows in [
            matrix(&[&[""]]),
            matrix(&[&["a"], &[""], &[""]]),
            matrix(&[&["a\tb", "\"quoted\""], &["中\n文", "\r\n"]]),
            matrix(&[&["", ""], &["", ""]]),
        ] {
            assert_eq!(parse_tsv(&encode_delimited(&rows, '\t')).unwrap(), rows);
        }
    }

    #[test]
    fn csv_encoding_quotes_only_required_fields() {
        assert_eq!(
            encode_delimited(&matrix(&[&["a,b", "a\"b", "line\nnext", "", "中"]]), ','),
            "\"a,b\",\"a\"\"b\",\"line\nnext\",,中"
        );
    }

    #[test]
    fn html_text_cannot_create_markup_or_entities() {
        assert_eq!(
            escape_html("<&>\"' 中文\n&copy;"),
            "&lt;&amp;&gt;&quot;&#39; 中文\n&amp;copy;"
        );
    }
}
