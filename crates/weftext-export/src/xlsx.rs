//! SpreadsheetML export over frozen Query results. Numeric strings never pass
//! through a floating-point conversion. Template edits do not insert rows or
//! rewrite references: existing standard names/tables define bounded slots.
use crate::{ExportArtifact, ExportError, ExportLoss, Result};
use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
};
use weftext_data::{Cell, NativeValue, QueryResult};
use weftext_messages::{DiagnosticOrigin, ErrorDescription, UiMessage};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const MAIN_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PKG_REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_PART: usize = 16 * 1024 * 1024;
const MAX_PARTS: usize = 2048;
const MAX_CELLS: usize = 200_000;
const MAX_ROWS: u32 = 1_048_576;
const MAX_COLS: u32 = 16_384;
const TEXT_LIMIT: usize = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XlsxRequest {
    pub sheets: Vec<XlsxSheetInput>,
    #[serde(default)]
    pub template: Option<XlsxTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XlsxSheetInput {
    pub name: String,
    /// Explicit terminal-column order, including columns in a zero-row result.
    pub columns: Vec<String>,
    pub result: QueryResult,
    /// Exact installed schema from the same frozen Query cut, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XlsxTemplate {
    pub bytes: Vec<u8>,
    #[serde(default)]
    pub fields: Vec<XlsxFieldSlot>,
    #[serde(default)]
    pub queries: Vec<XlsxQuerySlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XlsxFieldSlot {
    /// An existing workbook defined name that refers to exactly one cell.
    pub anchor: String,
    pub cell: Cell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XlsxQuerySlot {
    /// Existing workbook defined range (header + capacity) or table name.
    pub anchor: String,
    pub sheet: usize,
}

fn refuse(message: impl Into<weftext_messages::ErrorDescription>) -> ExportError {
    ExportError::new(message)
}
fn invalid_xml_utf8(error: std::str::Utf8Error) -> ExportError {
    refuse(
        ErrorDescription::new(UiMessage::error_export_xlsx_xml_invalid())
            .diagnostic(DiagnosticOrigin::Dependency, error.to_string()),
    )
}
fn loss(
    losses: &mut Vec<ExportLoss>,
    code: &str,
    message: impl Into<weftext_messages::ErrorDescription>,
    location: &str,
) {
    losses.push(ExportLoss {
        code: code.into(),
        description: message.into(),
        location: Some(location.into()),

        product_diagnostics: Vec::new(),
    });
}

/// Produces a real XLSX. A visible companion worksheet contains exact, ordered
/// JSON chunks of all inputs and conversion reports, so display truncation and
/// Excel's numeric/date limitations cannot destroy the frozen source values.
pub fn export_xlsx(request: &XlsxRequest) -> Result<ExportArtifact> {
    validate_request(request)?;
    let mut losses = vec![];
    let mut parts = if let Some(template) = &request.template {
        read_template(&template.bytes)?
    } else {
        blank_package()
    };
    let mut workbook = Workbook::read(&parts)?;
    if let Some(template) = &request.template {
        fill_template(&mut parts, &workbook, request, template, &mut losses)?;
    } else {
        for sheet in &request.sheets {
            let mut rows = vec![];
            rows.push(
                sheet
                    .columns
                    .iter()
                    .map(|s| Cell::value(NativeValue::Text(s.clone())))
                    .collect(),
            );
            for row in &sheet.result.rows {
                rows.push(
                    sheet
                        .columns
                        .iter()
                        .map(|c| row.cells.get(c).cloned().unwrap_or(Cell::Absent))
                        .collect(),
                );
            }
            let xml = sheet_xml(&rows, &sheet.name, &mut losses)?;
            workbook.add_sheet(&mut parts, &sheet.name, xml)?;
        }
    }
    for sheet in &request.sheets {
        if sheet.result.coverage != "complete"
            || sheet.result.limited
            || !sheet.result.aggregate_exact
        {
            loss(
                &mut losses,
                "query_incomplete",
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_query_total_not_exact_complete(),
                ),
                &sheet.name,
            );
        }
        if sheet.result.freshness != "current" {
            loss(
                &mut losses,
                "query_freshness",
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_query_freshness_retained(),
                ),
                &sheet.name,
            );
        }
    }
    loss(
        &mut losses,
        "xlsx_fonts",
        weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_fonts_not_embedded(),
        ),
        "workbook",
    );
    let structure_name = workbook.unique_name("Weftext structure");
    let mut structure = vec![text_row(&[
        "Item",
        "Part",
        "Parts",
        "Exact JSON (concatenate parts in order)",
    ])];
    append_json(
        &mut structure,
        "format",
        &serde_json::json!({
            "format":"weftext-xlsx-structure", "version":1,
            "encoding":"JSON UTF-8, ordered chunks; numeric NativeValue payloads are strings",
            "display":"Value/absent/null/invalid/unavailable remain distinct. Dates, decimals, binary floats and large integers use exact text. No strings become formulas."
        }),
    )?;
    for (i, sheet) in request.sheets.iter().enumerate() {
        append_json(&mut structure, &format!("sheet:{i}"), sheet)?;
    }
    if let Some(template) = &request.template {
        append_json(&mut structure, "template-fields", &template.fields)?;
        append_json(&mut structure, "template-query-slots", &template.queries)?;
    }
    append_json(&mut structure, "conversion-report", &losses)?;
    let xml = sheet_xml(&structure, &structure_name, &mut vec![])?;
    workbook.add_sheet(&mut parts, &structure_name, xml)?;
    let bytes = write_package(parts)?;
    Ok(ExportArtifact {
        bytes,
        content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".into(),
        extension: "xlsx".into(),
        losses,
        summary: weftext_messages::UiMessage::error_export_xlsx_summary(
            weftext_messages::params::ErrorExportXlsxSummary {
                sheets: weftext_messages::UiInteger::new(
                    i64::try_from(request.sheets.len()).expect("bounded product count"),
                )
                .expect("bounded product count"),
                rows: weftext_messages::UiInteger::new(
                    i64::try_from(
                        request
                            .sheets
                            .iter()
                            .map(|s| s.result.rows.len())
                            .sum::<usize>(),
                    )
                    .expect("bounded product count"),
                )
                .expect("bounded product count"),
                structure: (structure_name).to_string(),
            },
        ),
    })
}

fn validate_request(request: &XlsxRequest) -> Result<()> {
    if request.sheets.len() > 128 || request.sheets.is_empty() && request.template.is_none() {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_sheet_or_field_template_required(),
        )));
    }
    let mut names = BTreeSet::new();
    let mut cells = 0usize;
    for s in &request.sheets {
        check_sheet_name(&s.name)?;
        if !names.insert(s.name.to_lowercase()) {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_sheet_name_duplicate(),
            )));
        }
        if s.columns.is_empty()
            || s.columns.len() > MAX_COLS as usize
            || s.result.rows.len() >= MAX_ROWS as usize
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_sheet_dimensions_exceeded(),
            )));
        }
        let columns: BTreeSet<_> = s.columns.iter().collect();
        if columns.len() != s.columns.len() || columns.iter().any(|c| c.is_empty()) {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_query_columns_nonempty_unique_required(),
            )));
        }
        if s.result
            .terminal_schema
            .keys()
            .any(|c| !columns.contains(c))
            || s.result
                .rows
                .iter()
                .any(|r| r.cells.keys().any(|c| !columns.contains(c)))
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_column_order_incomplete(),
            )));
        }
        cells = cells.saturating_add(s.columns.len().saturating_mul(s.result.rows.len() + 1));
        if cells > MAX_CELLS {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_cell_budget_exceeded(),
            )));
        }
        for row in &s.result.rows {
            for cell in row.cells.values() {
                check_value_budget(cell)?;
            }
        }
    }
    if let Some(template) = &request.template {
        for field in &template.fields {
            check_value_budget(&field.cell)?;
        }
    }
    // Serialization also proves that maliciously nested transport values do not
    // sneak past the bounded output. No decimal or integer parsing occurs here.
    if serde_json::to_vec(request)?.len() > MAX_BYTES {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_input_bytes_exceeded(),
        )));
    }
    Ok(())
}
fn check_value_budget(cell: &Cell) -> Result<()> {
    let value = match cell {
        Cell::Value { value } => Some(value),
        Cell::Invalid { value, .. } => value.as_ref(),
        _ => None,
    };
    let Some(value) = value else {
        return Ok(());
    };
    let mut pending = vec![(value, 0usize)];
    let mut count = 0usize;
    while let Some((value, depth)) = pending.pop() {
        count += 1;
        if depth > 64 || count > MAX_CELLS {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_native_value_structure_exceeded(),
            )));
        }
        match value {
            NativeValue::Array(values) => pending.extend(values.iter().map(|v| (v, depth + 1))),
            NativeValue::Record(values) => {
                pending.extend(values.iter().map(|v| (&v.value, depth + 1)))
            }
            _ => {}
        }
    }
    Ok(())
}
fn check_sheet_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.encode_utf16().count() > 31
        || name.contains(['[', ']', ':', '*', '?', '/', '\\'])
        || name.starts_with('\'')
        || name.ends_with('\'')
        || name.chars().any(|c| c.is_control())
    {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_sheet_name_invalid(),
        )));
    }
    Ok(())
}
fn text_row(values: &[&str]) -> Vec<Cell> {
    values
        .iter()
        .map(|s| Cell::value(NativeValue::Text((*s).into())))
        .collect()
}
fn append_json<T: Serialize>(rows: &mut Vec<Vec<Cell>>, item: &str, value: &T) -> Result<()> {
    let json = serde_json::to_string(value)?;
    let chunks = chunks(&json, TEXT_LIMIT);
    for (i, chunk) in chunks.iter().enumerate() {
        rows.push(vec![
            Cell::value(NativeValue::Text(item.into())),
            Cell::value(NativeValue::Int((i + 1).to_string())),
            Cell::value(NativeValue::Int(chunks.len().to_string())),
            Cell::value(NativeValue::Text(chunk.clone())),
        ]);
    }
    if rows.len() > MAX_CELLS / 4 {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_structure_sheet_exceeded(),
        )));
    }
    Ok(())
}
fn chunks(value: &str, max: usize) -> Vec<String> {
    let mut chunks = vec![];
    let mut start = 0;
    let mut count = 0;
    for (offset, c) in value.char_indices() {
        if count + c.len_utf16() > max {
            chunks.push(value[start..offset].to_owned());
            start = offset;
            count = 0;
        }
        count += c.len_utf16();
    }
    chunks.push(value[start..].to_owned());
    chunks
}
fn xml_text(value: &str) -> String {
    // SpreadsheetML ST_Xstring escapes preserve XML-forbidden controls and
    // escape literal _xHHHH_ spellings so Excel cannot reinterpret author text.
    let mut result = String::new();
    let bytes = value.as_bytes();
    for (i, c) in value.char_indices() {
        if c == '_'
            && bytes.get(i + 1) == Some(&b'x')
            && bytes.get(i + 6) == Some(&b'_')
            && bytes
                .get(i + 2..i + 6)
                .is_some_and(|s| s.iter().all(u8::is_ascii_hexdigit))
        {
            result.push_str("_x005F_");
            continue;
        }
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            '\r' => result.push_str("_x000D_"),
            c if !matches!(c, '\t' | '\n') && (c < ' ' || matches!(c, '\u{fffe}' | '\u{ffff}')) => {
                result.push_str(&format!("_x{:04X}_", c as u32))
            }
            _ => result.push(c),
        }
    }
    result
}
fn readable(value: &NativeValue) -> Result<String> {
    Ok(match value {
        NativeValue::None => "<null>".into(),
        NativeValue::Text(s)
        | NativeValue::Int(s)
        | NativeValue::Float(s)
        | NativeValue::Decimal(s)
        | NativeValue::Date(s)
        | NativeValue::Time(s)
        | NativeValue::LocalDatetime(s)
        | NativeValue::Duration(s) => s.clone(),
        NativeValue::Bool(b) => b.to_string(),
        NativeValue::Array(_) | NativeValue::Record(_) => serde_json::to_string_pretty(value)?,
    })
}
fn cell_xml(
    cell: &Cell,
    address: &str,
    style: Option<&str>,
    location: &str,
    losses: &mut Vec<ExportLoss>,
) -> Result<String> {
    let style = style
        .map(|s| format!(" s=\"{}\"", xml_text(s)))
        .unwrap_or_default();
    if let Cell::Value {
        value: NativeValue::Bool(b),
    } = cell
    {
        return Ok(format!(
            "<c r=\"{address}\"{style} t=\"b\"><v>{}</v></c>",
            u8::from(*b)
        ));
    }
    if let Cell::Value {
        value: NativeValue::Int(s),
    } = cell
    {
        let digits = s.strip_prefix('-').unwrap_or(s);
        let canonical = digits == "0" || !digits.starts_with('0');
        if !digits.is_empty()
            && digits.len() <= 15
            && digits.bytes().all(|c| c.is_ascii_digit())
            && canonical
            && s != "-0"
        {
            return Ok(format!("<c r=\"{address}\"{style}><v>{s}</v></c>"));
        }
        loss(
            losses,
            "integer_as_exact_text",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_integer_as_exact_text(),
            ),
            location,
        );
    }
    let mut text = match cell {
        Cell::Value { value } => {
            match value {
                NativeValue::Float(_) | NativeValue::Decimal(_) => loss(
                    losses,
                    "number_as_exact_text",
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_exact_numeric_spelling_as_text(),
                    ),
                    location,
                ),
                NativeValue::Date(_)
                | NativeValue::Time(_)
                | NativeValue::LocalDatetime(_)
                | NativeValue::Duration(_) => loss(
                    losses,
                    "temporal_as_exact_text",
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_temporal_values_as_exact_text(),
                    ),
                    location,
                ),
                NativeValue::Array(_) | NativeValue::Record(_) => loss(
                    losses,
                    "nested_structure",
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_nested_values_as_json(),
                    ),
                    location,
                ),
                _ => {}
            }
            readable(value)?
        }
        Cell::Absent => "<absent>".into(),
        Cell::Null => "<null>".into(),
        // Product diagnostic messages remain typed data in an exported cell.
        // Rendering a UI locale here would make a language switch rewrite the
        // frozen Query value and lose its independent diagnostic provenance.
        Cell::Invalid { .. } | Cell::Unavailable { .. } => serde_json::to_string(cell)?,
    };
    if text.encode_utf16().count() > TEXT_LIMIT {
        text = chunks(&text, TEXT_LIMIT - 100).remove(0);
        text.push_str("\n[display shortened; complete value in Weftext structure]");
        loss(
            losses,
            "cell_display_shortened",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_readable_text_limit_exceeded(),
            ),
            location,
        );
    }
    Ok(format!(
        "<c r=\"{address}\"{style} t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
        xml_text(&text)
    ))
}
fn column(mut col: u32) -> String {
    let mut out = String::new();
    while col > 0 {
        col -= 1;
        out.insert(0, (b'A' + (col % 26) as u8) as char);
        col /= 26;
    }
    out
}
fn address(row: u32, col: u32) -> String {
    format!("{}{row}", column(col))
}
fn sheet_xml(rows: &[Vec<Cell>], name: &str, losses: &mut Vec<ExportLoss>) -> Result<Vec<u8>> {
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"{MAIN_NS}\"><sheetViews><sheetView workbookViewId=\"0\"/></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><sheetData>"
    );
    for (r, row) in rows.iter().enumerate() {
        out.push_str(&format!("<row r=\"{}\">", r + 1));
        for (c, cell) in row.iter().enumerate() {
            let a = address(r as u32 + 1, c as u32 + 1);
            out.push_str(&cell_xml(cell, &a, None, &format!("{name}!{a}"), losses)?);
        }
        out.push_str("</row>");
        if out.len() > MAX_BYTES {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_sheet_output_exceeded(),
            )));
        }
    }
    out.push_str("</sheetData></worksheet>");
    Ok(out.into_bytes())
}

type Parts = BTreeMap<String, Vec<u8>>;
fn blank_package() -> Parts {
    BTreeMap::from([
        ("[Content_Types].xml".into(),b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>".to_vec()),
        ("_rels/.rels".into(),format!("<Relationships xmlns=\"{PKG_REL_NS}\"><Relationship Id=\"rId1\" Type=\"{REL_NS}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>").into_bytes()),
        ("xl/workbook.xml".into(),format!("<workbook xmlns=\"{MAIN_NS}\" xmlns:r=\"{REL_NS}\"><sheets></sheets></workbook>").into_bytes()),
        ("xl/_rels/workbook.xml.rels".into(),format!("<Relationships xmlns=\"{PKG_REL_NS}\"></Relationships>").into_bytes()),
    ])
}
fn write_package(parts: Parts) -> Result<Vec<u8>> {
    if parts.len() > MAX_PARTS || parts.values().map(Vec::len).sum::<usize>() > MAX_BYTES {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_package_budget_exceeded(),
        )));
    }
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in parts {
        zip.start_file(
            name,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )?;
        zip.write_all(&bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

#[derive(Debug, Clone)]
struct Element {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Element>,
    text: String,
    start: usize,
    open_end: usize,
    close_start: usize,
    end: usize,
    empty: bool,
}
impl Element {
    fn local(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }
    fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.local() == name)
    }
    fn descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Element>) {
        if self.local() == name {
            out.push(self);
        }
        for c in &self.children {
            c.descendants(name, out);
        }
    }
}
fn parse_xml(bytes: &[u8]) -> Result<Element> {
    if bytes.len() > MAX_PART {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_template_xml_bounded_utf_eight_required(
            ),
        )));
    }
    if !bytes.is_ascii()
        && let Err(error) = std::str::from_utf8(bytes)
    {
        return Err(refuse(
            ErrorDescription::new(
                UiMessage::error_export_xlsx_template_xml_bounded_utf_eight_required(),
            )
            .diagnostic(DiagnosticOrigin::Dependency, error.to_string()),
        ));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<Element> = vec![];
    let mut root = None;
    let mut count = 0;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(|e| {
            refuse(
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_invalid_xml(),
                )
                .diagnostic(
                    weftext_messages::DiagnosticOrigin::Dependency,
                    e.to_string(),
                ),
            )
        })?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let empty = bytes.get(end.saturating_sub(2)) == Some(&b'/');
                count += 1;
                if stack.len() > 64 || count > MAX_CELLS * 4 {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_template_xml_structure_exceeded(),
                    )));
                }
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|error| {
                        refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_xml_element_name_invalid(
                            ),
                        ).diagnostic(DiagnosticOrigin::Dependency, error.to_string()))
                    })?
                    .to_string();
                let mut attrs = BTreeMap::new();
                for a in e.attributes() {
                    let a = a.map_err(|e| {
                        refuse(
                            weftext_messages::ErrorDescription::new(
                                weftext_messages::UiMessage::error_export_xml(),
                            )
                            .diagnostic(
                                weftext_messages::DiagnosticOrigin::Dependency,
                                e.to_string(),
                            ),
                        )
                    })?;
                    let key = std::str::from_utf8(a.key.as_ref())
                        .map_err(|error| {
                            refuse(weftext_messages::ErrorDescription::new(
                                weftext_messages::UiMessage::error_export_xlsx_xml_attribute_invalid(),
                            ).diagnostic(DiagnosticOrigin::Dependency, error.to_string()))
                        })?
                        .to_string();
                    let value = a
                        .decode_and_unescape_value(reader.decoder())
                        .map_err(|e| {
                            refuse(
                                weftext_messages::ErrorDescription::new(
                                    weftext_messages::UiMessage::error_export_xml(),
                                )
                                .diagnostic(
                                    weftext_messages::DiagnosticOrigin::Dependency,
                                    e.to_string(),
                                ),
                            )
                        })?
                        .into_owned();
                    if attrs.insert(key, value).is_some() {
                        return Err(refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_xml_attribute_duplicate(
                            ),
                        )));
                    }
                }
                let element = Element {
                    name,
                    attrs,
                    children: vec![],
                    text: String::new(),
                    start,
                    open_end: end,
                    close_start: if empty { start } else { 0 },
                    end: if empty { end } else { 0 },
                    empty,
                };
                if empty {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(element);
                    } else if root.replace(element).is_some() {
                        return Err(refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_xml_multiple_roots(),
                        )));
                    }
                } else {
                    stack.push(element);
                }
            }
            Event::End(_) => {
                let mut e = stack.pop().ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_xml_end_unexpected(),
                    ))
                })?;
                e.close_start = start;
                e.end = end;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(e);
                } else if root.replace(e).is_some() {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_xml_multiple_roots(),
                    )));
                }
            }
            Event::Text(t) => {
                let text = t.xml_content().map_err(|e| {
                    refuse(
                        weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xml(),
                        )
                        .diagnostic(
                            weftext_messages::DiagnosticOrigin::Dependency,
                            e.to_string(),
                        ),
                    )
                })?;
                let text = quick_xml::escape::unescape(&text).map_err(|e| {
                    refuse(
                        weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xml(),
                        )
                        .diagnostic(
                            weftext_messages::DiagnosticOrigin::Dependency,
                            e.to_string(),
                        ),
                    )
                })?;
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&text);
                } else if !text.trim().is_empty() {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_xml_text_outside_root(),
                    )));
                }
            }
            Event::CData(t) => {
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&t.decode().map_err(|e| {
                        refuse(
                            weftext_messages::ErrorDescription::new(
                                weftext_messages::UiMessage::error_export_xml(),
                            )
                            .diagnostic(
                                weftext_messages::DiagnosticOrigin::Dependency,
                                e.to_string(),
                            ),
                        )
                    })?);
                }
            }
            Event::DocType(_) => {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_xml_declarations_forbidden(),
                )));
            }
            Event::GeneralRef(r) => {
                let name = r.decode().map_err(|e| {
                    refuse(
                        weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xml(),
                        )
                        .diagnostic(
                            weftext_messages::DiagnosticOrigin::Dependency,
                            e.to_string(),
                        ),
                    )
                })?;
                let encoded = format!("&{name};");
                let value = quick_xml::escape::unescape(&encoded).map_err(|error| {
                    refuse(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_xml_unresolved_entities_forbidden()).diagnostic(DiagnosticOrigin::Dependency, error.to_string()))
                })?;
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&value);
                } else {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_xml_entity_outside_root(),
                    )));
                }
            }
            Event::Decl(d) => {
                if d.version()
                    .map_err(|e| {
                        refuse(
                            weftext_messages::ErrorDescription::new(
                                weftext_messages::UiMessage::error_export_xml(),
                            )
                            .diagnostic(
                                weftext_messages::DiagnosticOrigin::Dependency,
                                e.to_string(),
                            ),
                        )
                    })?
                    .as_ref()
                    != b"1.0"
                {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_xml_version_unsupported(),
                    )));
                }
                if let Some(encoding) = d.encoding() {
                    let encoding = encoding.map_err(|e| {
                        refuse(
                            weftext_messages::ErrorDescription::new(
                                weftext_messages::UiMessage::error_export_xml(),
                            )
                            .diagnostic(
                                weftext_messages::DiagnosticOrigin::Dependency,
                                e.to_string(),
                            ),
                        )
                    })?;
                    if !encoding.eq_ignore_ascii_case(b"UTF-8") {
                        return Err(refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_xml_encoding_unsupported(
                            ),
                        )));
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_xml_element_unclosed(),
        )));
    }
    root.ok_or_else(|| {
        refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_xml_root_missing(),
        ))
    })
}
fn part<'a>(parts: &'a Parts, name: &str) -> Result<&'a [u8]> {
    parts.get(name).map(Vec::as_slice).ok_or_else(|| {
        refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_workbook_part_missing(
                weftext_messages::params::ErrorExportXlsxWorkbookPartMissing {
                    part: (name).to_string(),
                },
            ),
        ))
    })
}
fn read_template(bytes: &[u8]) -> Result<Parts> {
    if bytes.len() > MAX_BYTES {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_template_upload_exceeded(),
        )));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    if zip.len() > MAX_PARTS {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_template_part_count_exceeded(),
        )));
    }
    let mut parts = Parts::new();
    let mut part_names = BTreeSet::new();
    let mut total = 0usize;
    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        let name = file.name().to_string();
        if file.is_dir() {
            continue;
        }
        if name.starts_with('/')
            || name.contains(['\\', ':', '\0'])
            || name
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == "..")
            || file.enclosed_name().is_none()
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_template_part_path_unsafe(),
            )));
        }
        let lower = name.to_lowercase();
        if lower.contains("vbaproject")
            || lower.contains("macrosheet")
            || lower.contains("externallink")
            || lower.starts_with("_xmlsignatures/")
            || lower.contains("activex")
            || lower.contains("embeddings/")
            || lower == "xl/connections.xml"
            || lower.contains("/querytables/")
            || lower.contains("/webextensions/")
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_active_workbook_unsupported(),
            )));
        }
        if !part_names.insert(lower) {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_template_part_case_ambiguous(),
            )));
        }
        if file.size() > MAX_PART as u64 {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_template_part_size_exceeded(),
            )));
        }
        let mut data = vec![];
        file.take(MAX_PART as u64 + 1).read_to_end(&mut data)?;
        total = total.saturating_add(data.len());
        if data.len() > MAX_PART || total > MAX_BYTES {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_template_decompression_exceeded(),
            )));
        }
        if parts.insert(name, data).is_some() {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_template_part_duplicate(),
            )));
        }
    }
    for (name, data) in &parts {
        let lower = name.to_lowercase();
        if lower.ends_with(".xml") || lower.ends_with(".rels") {
            let root = parse_xml(data)?;
            if lower.ends_with(".rels") {
                if root.local() != "Relationships" || root.attr("xmlns") != Some(PKG_REL_NS) {
                    return Err(refuse(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_relationship_namespace_or_root_invalid())));
                }
                for r in &root.children {
                    if r.local() != "Relationship" {
                        return Err(refuse(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_relationship_element_unsupported())));
                    }
                    if r.attr("TargetMode").is_some_and(|m| m != "Internal") {
                        return Err(refuse(
                            weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_external_relationship_unsupported()),
                        ));
                    }
                    let target = r
                        .attr("Target")
                        .ok_or_else(|| refuse(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_relationship_target_missing())))?;
                    let owner = relationship_owner(name)?;
                    let resolved = resolve_target(&owner, target)?;
                    if !parts.contains_key(&resolved) {
                        return Err(refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_relationship_part_missing(),
                        )));
                    }
                }
            }
            let mut formulas = vec![];
            root.descendants("f", &mut formulas);
            for f in formulas {
                let upper = f.text.to_uppercase().replace([' ', '\t', '\r', '\n'], "");
                if upper.contains(['[', '|'])
                    || [
                        "WEBSERVICE(",
                        "HYPERLINK(",
                        "RTD(",
                        "IMAGE(",
                        "CALL(",
                        "REGISTER(",
                    ]
                    .iter()
                    .any(|s| upper.contains(s))
                {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_active_formula_unsupported(),
                    )));
                }
            }
            if name == "[Content_Types].xml"
                && root.children.iter().any(|c| {
                    c.attr("ContentType")
                        .is_some_and(|t| t.to_lowercase().contains("macroenabled"))
                })
            {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_macro_content_type_unsupported(),
                )));
            }
        }
    }
    let root_rels = parse_xml(part(&parts, "_rels/.rels")?)?;
    let main: Vec<_> = root_rels
        .children
        .iter()
        .filter(|r| r.attr("Type") == Some(&format!("{REL_NS}/officeDocument")))
        .collect();
    if main.len() != 1
        || main[0]
            .attr("Target")
            .map(|t| resolve_target("", t))
            .transpose()?
            .as_deref()
            != Some("xl/workbook.xml")
    {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_standard_package_root_required(),
        )));
    }
    let content = parse_xml(part(&parts, "[Content_Types].xml")?)?;
    if content.local() != "Types"
        || content.attr("xmlns")
            != Some("http://schemas.openxmlformats.org/package/2006/content-types")
    {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_content_types_namespace_or_root_invalid(
            ),
        )));
    }
    let main: Vec<_> = content
        .children
        .iter()
        .filter(|c| c.attr("PartName") == Some("/xl/workbook.xml"))
        .collect();
    if main.len() != 1
        || main[0].attr("ContentType")
            != Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml")
    {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_ordinary_workbook_content_type_required(
            ),
        )));
    }
    Ok(parts)
}
fn relationship_owner(path: &str) -> Result<String> {
    if path == "_rels/.rels" {
        return Ok(String::new());
    }
    let (directory, file) = path.rsplit_once("/_rels/").ok_or_else(|| {
        refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_relationship_location_unsupported(),
        ))
    })?;
    Ok(format!(
        "{directory}/{}",
        file.strip_suffix(".rels")
            .ok_or_else(|| refuse(UiMessage::error_export_xlsx_relationship_part_name_invalid()))?
    ))
}
fn resolve_target(owner: &str, target: &str) -> Result<String> {
    if target.contains(['\\', ':', '?', '#', '\0', '%']) {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_relationship_uri_unsupported(),
        )));
    }
    let mut pieces = if target.starts_with('/') {
        vec![]
    } else {
        owner
            .rsplit_once('/')
            .map(|(d, _)| d.split('/').map(str::to_string).collect())
            .unwrap_or_default()
    };
    for piece in target.trim_start_matches('/').split('/') {
        match piece {
            "" | "." => {}
            ".." => {
                if pieces.pop().is_none() {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_relationship_escapes_package(
                        ),
                    )));
                }
            }
            s => pieces.push(s.to_string()),
        }
    }
    if pieces.is_empty() {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_relationship_target_empty(),
        )));
    }
    Ok(pieces.join("/"))
}

#[derive(Clone)]
struct Worksheet {
    name: String,
    path: String,
    id: u32,
}
struct Workbook {
    sheets: Vec<Worksheet>,
}
impl Workbook {
    fn read(parts: &Parts) -> Result<Self> {
        let rels = parse_xml(part(parts, "xl/_rels/workbook.xml.rels")?)?;
        let mut relationships = BTreeMap::new();
        for r in &rels.children {
            let id = r.attr("Id").ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_workbook_relationship_id_missing(
                    ),
                ))
            })?;
            if relationships.insert(id.to_string(), r).is_some() {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_workbook_relationship_id_duplicate(),
                )));
            }
        }
        let workbook = parse_xml(part(parts, "xl/workbook.xml")?)?;
        if workbook.local() != "workbook" || workbook.attr("xmlns") != Some(MAIN_NS) {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_transitional_workbook_required(),
            )));
        }
        let sheets = workbook.child("sheets").ok_or_else(|| {
            refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_workbook_sheets_missing(),
            ))
        })?;
        let mut out = vec![];
        let mut names = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for s in &sheets.children {
            if s.local() != "sheet" {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_workbook_sheets_content_unsupported(),
                )));
            }
            let name = s
                .attr("name")
                .ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_sheet_name_missing(),
                    ))
                })?
                .to_string();
            check_sheet_name(&name)?;
            let id = s
                .attr("sheetId")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|i| *i > 0)
                .ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_sheet_id_invalid(),
                    ))
                })?;
            let rid = s.attr("r:id").ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_sheet_relationship_missing(),
                ))
            })?;
            let r = relationships.get(rid).ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_sheet_relationship_unavailable(),
                ))
            })?;
            if r.attr("Type") != Some(&format!("{REL_NS}/worksheet")) {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_non_worksheet_sheet_unsupported(
                    ),
                )));
            }
            let path = resolve_target(
                "xl/workbook.xml",
                r.attr("Target").ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_worksheet_target_missing(),
                    ))
                })?,
            )?;
            part(parts, &path)?;
            if !names.insert(name.to_lowercase()) || !ids.insert(id) || !paths.insert(path.clone())
            {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_workbook_sheets_ambiguous(),
                )));
            }
            out.push(Worksheet { name, path, id });
        }
        Ok(Self { sheets: out })
    }
    fn unique_name(&self, base: &str) -> String {
        for i in 0..1000 {
            let n = if i == 0 {
                base.to_string()
            } else {
                format!("{base} {i}")
            };
            if !self.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(&n)) {
                return n;
            }
        }
        unreachable!()
    }
    fn add_sheet(&mut self, parts: &mut Parts, name: &str, xml: Vec<u8>) -> Result<()> {
        check_sheet_name(name)?;
        if self
            .sheets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(name))
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_sheet_name_already_exists(),
            )));
        }
        let mut number = 1;
        while parts.contains_key(&format!("xl/worksheets/weftext{number}.xml")) {
            number += 1;
        }
        let path = format!("xl/worksheets/weftext{number}.xml");
        let id = self
            .sheets
            .iter()
            .map(|s| s.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_sheet_id_overflow(),
                ))
            })?;
        let rel_xml = part(parts, "xl/_rels/workbook.xml.rels")?.to_vec();
        let rel_root = parse_xml(&rel_xml)?;
        let mut rid = format!("weftextSheet{number}");
        while rel_root.children.iter().any(|r| r.attr("Id") == Some(&rid)) {
            rid.push('_');
        }
        let wb_xml = part(parts, "xl/workbook.xml")?.to_vec();
        let wb = parse_xml(&wb_xml)?;
        let sheets = wb.child("sheets").ok_or_else(|| {
            refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_sheets_missing(),
            ))
        })?;
        let entry = format!(
            "<sheet name=\"{}\" sheetId=\"{id}\" r:id=\"{rid}\" xmlns:r=\"{REL_NS}\"/>",
            xml_text(name)
        );
        parts.insert(
            "xl/workbook.xml".into(),
            append_child(&wb_xml, sheets, &entry)?,
        );
        let rel_entry = format!(
            "<Relationship Id=\"{rid}\" Type=\"{REL_NS}/worksheet\" Target=\"worksheets/weftext{number}.xml\"/>"
        );
        parts.insert(
            "xl/_rels/workbook.xml.rels".into(),
            append_child(&rel_xml, &rel_root, &rel_entry)?,
        );
        let content_xml = part(parts, "[Content_Types].xml")?.to_vec();
        let content = parse_xml(&content_xml)?;
        parts.insert("[Content_Types].xml".into(),append_child(&content_xml,&content,&format!("<Override PartName=\"/{path}\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"))?);
        parts.insert(path.clone(), xml);
        self.sheets.push(Worksheet {
            name: name.into(),
            path,
            id,
        });
        Ok(())
    }
}
fn append_child(bytes: &[u8], element: &Element, child: &str) -> Result<Vec<u8>> {
    if element.name.contains(':') {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_prefixed_container_insertion_unsupported(
            ),
        )));
    }
    let replacement = if element.empty {
        let open = std::str::from_utf8(&bytes[element.start..element.end - 2])
            .map_err(invalid_xml_utf8)?;
        format!("{open}>{child}</{}>", element.name)
    } else {
        format!(
            "{}{child}{}",
            std::str::from_utf8(&bytes[element.start..element.close_start])
                .map_err(invalid_xml_utf8)?,
            std::str::from_utf8(&bytes[element.close_start..element.end])
                .map_err(invalid_xml_utf8)?
        )
    };
    let mut out = bytes[..element.start].to_vec();
    out.extend_from_slice(replacement.as_bytes());
    out.extend_from_slice(&bytes[element.end..]);
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XlsxTemplateInspection {
    pub anchors: Vec<XlsxAnchorInfo>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XlsxAnchorInfo {
    pub name: String,
    pub kind: String,
    pub sheet: Option<String>,
    pub rows: u32,
    pub columns: u32,
    pub capacity: u32,
    pub supported: bool,
    pub reason: Option<ExportError>,
}
#[derive(Debug, Clone, Copy)]
struct Rect {
    first_row: u32,
    last_row: u32,
    first_col: u32,
    last_col: u32,
}
impl Rect {
    fn rows(self) -> u32 {
        self.last_row - self.first_row + 1
    }
    fn columns(self) -> u32 {
        self.last_col - self.first_col + 1
    }
    fn contains(self, row: u32, col: u32) -> bool {
        row >= self.first_row
            && row <= self.last_row
            && col >= self.first_col
            && col <= self.last_col
    }
    fn overlaps(self, other: Self) -> bool {
        self.first_row <= other.last_row
            && self.last_row >= other.first_row
            && self.first_col <= other.last_col
            && self.last_col >= other.first_col
    }
}
#[derive(Clone)]
struct Anchor {
    info: XlsxAnchorInfo,
    path: Option<String>,
    rect: Option<Rect>,
    table_columns: Option<Vec<String>>,
    totals: u32,
}

/// Lists the actual standard workbook anchors. Unsupported formulas, relative
/// refs, protected/merged targets and ambiguous names are surfaced explicitly.
pub fn inspect_xlsx_template(bytes: &[u8]) -> Result<XlsxTemplateInspection> {
    let parts = read_template(bytes)?;
    let workbook = Workbook::read(&parts)?;
    Ok(XlsxTemplateInspection {
        anchors: anchors(&parts, &workbook)?
            .into_iter()
            .map(|a| a.info)
            .collect(),
    })
}
fn parse_address(value: &str, absolute: bool) -> Result<(u32, u32)> {
    let mut s = value;
    if absolute {
        s = s.strip_prefix('$').ok_or_else(|| {
            refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_defined_name_absolute_cell_required(
                ),
            ))
        })?;
    } else {
        s = s.strip_prefix('$').unwrap_or(s);
    }
    let letters = s.bytes().take_while(u8::is_ascii_alphabetic).count();
    if letters == 0 || letters > 3 {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_cell_column_invalid(),
        )));
    }
    let (col_text, mut row_text) = s.split_at(letters);
    if !col_text.bytes().all(|c| c.is_ascii_uppercase()) {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_uppercase_column_address_required(),
        )));
    }
    if absolute {
        row_text = row_text.strip_prefix('$').ok_or_else(|| {
            refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_defined_name_absolute_row_required(),
            ))
        })?;
    } else {
        row_text = row_text.strip_prefix('$').unwrap_or(row_text);
    }
    if row_text.is_empty()
        || row_text.starts_with('0')
        || !row_text.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_cell_row_invalid(),
        )));
    }
    let row: u32 = row_text.parse().map_err(|_| {
        refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_cell_row_invalid(),
        ))
    })?;
    let col = col_text
        .bytes()
        .fold(0u32, |n, c| n * 26 + u32::from(c - b'A' + 1));
    if row == 0 || row > MAX_ROWS || col == 0 || col > MAX_COLS {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_cell_address_out_of_bounds(),
        )));
    }
    Ok((row, col))
}
fn parse_rect(value: &str, absolute: bool) -> Result<Rect> {
    let (first, last) = value.split_once(':').unwrap_or((value, value));
    let (fr, fc) = parse_address(first, absolute)?;
    let (lr, lc) = parse_address(last, absolute)?;
    if fr > lr || fc > lc {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_cell_range_reversed(),
        )));
    }
    Ok(Rect {
        first_row: fr,
        last_row: lr,
        first_col: fc,
        last_col: lc,
    })
}
fn defined_reference(value: &str) -> Result<(String, Rect)> {
    let (name, range) = value.trim().rsplit_once('!').ok_or_else(|| {
        refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_defined_name_reference_unsupported(),
        ))
    })?;
    let name = if name.starts_with('\'') && name.ends_with('\'') {
        name[1..name.len() - 1].replace("''", "'")
    } else {
        name.to_string()
    };
    check_sheet_name(&name)?;
    if name.contains(['!', '[', ']']) {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_external_or_multi_sheet_name_unsupported(
            ),
        )));
    }
    Ok((name, parse_rect(range, true)?))
}
fn anchors(parts: &Parts, workbook: &Workbook) -> Result<Vec<Anchor>> {
    let wb = parse_xml(part(parts, "xl/workbook.xml")?)?;
    let mut out = vec![];
    let mut worksheet_roots = BTreeMap::new();
    if let Some(names) = wb.child("definedNames") {
        for n in &names.children {
            if n.local() != "definedName" {
                continue;
            }
            let name = n
                .attr("name")
                .ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_defined_name_name_missing(),
                    ))
                })?
                .to_string();
            let mut a = Anchor {
                info: XlsxAnchorInfo {
                    name,
                    kind: "definedName".into(),
                    sheet: None,
                    rows: 0,
                    columns: 0,
                    capacity: 0,
                    supported: false,
                    reason: None,
                },
                path: None,
                rect: None,
                table_columns: None,
                totals: 0,
            };
            let reference = defined_reference(&n.text).and_then(|(name, rect)| {
                if n.attr("localSheetId").is_some() {
                    return Err(refuse(
                        weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_sheet_scoped_name_unsupported()),
                    ));
                }
                let sheet = workbook
                    .sheets
                    .iter()
                    .find(|s| s.name == name)
                    .ok_or_else(|| refuse(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_defined_name_sheet_unavailable())))?;
                Ok((sheet, rect))
            });
            match reference {
                Ok((s, r)) => {
                    a.info.sheet = Some(s.name.clone());
                    a.info.rows = r.rows();
                    a.info.columns = r.columns();
                    a.info.capacity = r.rows().saturating_sub(1);
                    a.path = Some(s.path.clone());
                    a.rect = Some(r);
                    a.info.supported = true;
                }
                Err(e) => a.info.reason = Some(e),
            }
            out.push(a);
        }
    }
    for sheet in &workbook.sheets {
        let xml = parse_xml(part(parts, &sheet.path)?)?;
        let Some(table_parts) = xml.child("tableParts") else {
            worksheet_roots.insert(sheet.path.clone(), xml);
            continue;
        };
        let (directory, file) = sheet.path.rsplit_once('/').ok_or_else(|| {
            refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_worksheet_path_unsupported(),
            ))
        })?;
        let rel_path = format!("{directory}/_rels/{file}.rels");
        let rels = parse_xml(part(parts, &rel_path)?)?;
        for t in &table_parts.children {
            let rid = t.attr("r:id").ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_table_relationship_missing(),
                ))
            })?;
            let rels: Vec<_> = rels
                .children
                .iter()
                .filter(|r| r.attr("Id") == Some(rid))
                .collect();
            if rels.len() != 1 || rels[0].attr("Type") != Some(&format!("{REL_NS}/table")) {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_table_relationship_invalid(),
                )));
            }
            let path = resolve_target(
                &sheet.path,
                rels[0].attr("Target").ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_table_target_missing(),
                    ))
                })?,
            )?;
            let table = parse_xml(part(parts, &path)?)?;
            let name = table
                .attr("displayName")
                .or_else(|| table.attr("name"))
                .ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_table_name_missing(),
                    ))
                })?
                .to_string();
            let mut a = Anchor {
                info: XlsxAnchorInfo {
                    name,
                    kind: "table".into(),
                    sheet: Some(sheet.name.clone()),
                    rows: 0,
                    columns: 0,
                    capacity: 0,
                    supported: false,
                    reason: None,
                },
                path: Some(sheet.path.clone()),
                rect: None,
                table_columns: None,
                totals: 0,
            };
            let shape = (|| -> Result<(Rect, Vec<String>, u32)> {
                let rect = parse_rect(
                    table.attr("ref").ok_or_else(|| {
                        refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_table_range_missing(),
                        ))
                    })?,
                    false,
                )?;
                if table.attr("headerRowCount").unwrap_or("1") != "1" {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_query_table_header_required(
                        ),
                    )));
                }
                let totals = table
                    .attr("totalsRowCount")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .map_err(|_| {
                        refuse(weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_xlsx_table_totals_rows_invalid(),
                        ))
                    })?;
                if totals > 1 || rect.rows() < 1 + totals {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_table_totals_shape_unsupported(),
                    )));
                }
                let cols = table.child("tableColumns").ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_table_columns_missing(),
                    ))
                })?;
                let mut names = vec![];
                for col in &cols.children {
                    if col.child("calculatedColumnFormula").is_some() {
                        return Err(refuse(
                            weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_calculated_column_overwrite_forbidden()),
                        ));
                    }
                    names.push(
                        col.attr("name")
                            .ok_or_else(|| refuse(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_xlsx_table_column_name_missing())))?
                            .to_string(),
                    );
                }
                if names.len() != rect.columns() as usize {
                    return Err(refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_table_width_mismatch(),
                    )));
                }
                Ok((rect, names, totals))
            })();
            match shape {
                Ok((r, n, t)) => {
                    a.info.rows = r.rows();
                    a.info.columns = r.columns();
                    a.info.capacity = r.rows() - 1 - t;
                    a.rect = Some(r);
                    a.table_columns = Some(n);
                    a.totals = t;
                    a.info.supported = true;
                }
                Err(e) => a.info.reason = Some(e),
            }
            out.push(a);
        }
        worksheet_roots.insert(sheet.path.clone(), xml);
    }
    if out.len() > 1024 {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_standard_anchor_count_exceeded(),
        )));
    }
    let mut counts = BTreeMap::new();
    for a in &out {
        *counts.entry(a.info.name.to_lowercase()).or_insert(0) += 1;
    }
    // Validate each bounded rectangle before presenting it as selectable. This
    // is the same admission logic used by export, not an optimistic UI list.
    for a in &mut out {
        if counts[&a.info.name.to_lowercase()] > 1 {
            a.info.supported = false;
            a.info.reason = Some(ExportError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_standard_name_ambiguous(),
            )));
            continue;
        }
        if a.info.supported {
            let r = a.rect.unwrap();
            let check = if u64::from(r.rows()) * u64::from(r.columns()) > MAX_CELLS as u64 {
                Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_anchor_cells_exceeded(),
                )))
            } else {
                check_target_rectangle(worksheet_roots.get(a.path.as_ref().unwrap()).unwrap(), r)
            };
            if let Err(e) = check {
                a.info.supported = false;
                a.info.reason = Some(e);
            }
        }
    }
    Ok(out)
}
fn check_target_rectangle(root: &Element, rect: Rect) -> Result<()> {
    if root.local() != "worksheet" || root.attr("xmlns") != Some(MAIN_NS) || root.name.contains(':')
    {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_editable_worksheet_kind_required(),
        )));
    }
    if root.child("sheetProtection").is_some() {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_protected_anchor_unsupported(),
        )));
    }
    if let Some(merges) = root.child("mergeCells") {
        for m in &merges.children {
            if parse_rect(
                m.attr("ref").ok_or_else(|| {
                    refuse(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_xlsx_merge_reference_missing(),
                    ))
                })?,
                false,
            )?
            .overlaps(rect)
            {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_anchor_overlaps_merge(),
                )));
            }
        }
    }
    let mut cells = vec![];
    root.descendants("c", &mut cells);
    for c in cells {
        let (row, col) = parse_address(
            c.attr("r").ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_explicit_cell_address_required(),
                ))
            })?,
            false,
        )?;
        if rect.contains(row, col)
            && (c.name.contains(':')
                || c.attrs
                    .keys()
                    .any(|k| !matches!(k.as_str(), "r" | "s" | "t"))
                || c.children.iter().any(|c| !matches!(c.local(), "v" | "is")))
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_anchor_content_unsupported(),
            )));
        }
        if let Some(f) = c.child("f")
            && let Some(r) = f.attr("ref")
            && parse_rect(r, false)?.overlaps(rect)
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_anchor_overlaps_formula_range(),
            )));
        }
    }
    Ok(())
}
fn find_anchor<'a>(anchors: &'a [Anchor], name: &str) -> Result<&'a Anchor> {
    let matches: Vec<_> = anchors
        .iter()
        .filter(|a| a.info.name.eq_ignore_ascii_case(name))
        .collect();
    if matches.len() != 1 {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_template_anchor_ambiguous(
                weftext_messages::params::ErrorExportXlsxTemplateAnchorAmbiguous {
                    name: (name).to_string(),
                },
            ),
        )));
    }
    if !matches[0].info.supported {
        return Err({
            let explanation =
                weftext_messages::UiMessage::error_export_xlsx_template_anchor_unsupported(
                    weftext_messages::params::ErrorExportXlsxTemplateAnchorUnsupported {
                        name: name.to_string(),
                    },
                );
            match matches[0].info.reason.clone() {
                Some(reason) => {
                    let (description, details) = reason.into_parts();
                    ExportError::with_cause(explanation, "templateAnchor", description, details)
                }
                None => ExportError::new(explanation),
            }
        });
    }
    Ok(matches[0])
}
type Writes = BTreeMap<(u32, u32), Option<Cell>>;
fn fill_template(
    parts: &mut Parts,
    workbook: &Workbook,
    request: &XlsxRequest,
    template: &XlsxTemplate,
    losses: &mut Vec<ExportLoss>,
) -> Result<()> {
    if template.fields.is_empty() && template.queries.is_empty() {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_explicit_anchor_mapping_required(),
        )));
    }
    let anchors = anchors(parts, workbook)?;
    let mut writes: BTreeMap<String, Writes> = BTreeMap::new();
    let mut regions: Vec<(String, Rect)> = vec![];
    let mut mapped = BTreeSet::new();
    let mut registered_cells = 0usize;
    let mut register = |anchor: &Anchor, region: Rect| -> Result<()> {
        let path = anchor.path.as_ref().unwrap();
        registered_cells = registered_cells
            .saturating_add((u64::from(region.rows()) * u64::from(region.columns())) as usize);
        if registered_cells > MAX_CELLS {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_combined_mapping_preallocation_exceeded(),
            )));
        }
        if regions.iter().any(|(p, r)| p == path && r.overlaps(region)) {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_mappings_overlap(),
            )));
        }
        for other in &anchors {
            if other.info.kind == "table"
                && other.path.as_ref() == Some(path)
                && other.rect.is_some_and(|r| r.overlaps(region))
                && !(anchor.info.kind == "table" && other.info.name == anchor.info.name)
            {
                return Err(refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_named_range_overlaps_table(),
                )));
            }
        }
        regions.push((path.clone(), region));
        Ok(())
    };
    for field in &template.fields {
        let anchor = find_anchor(&anchors, &field.anchor)?;
        let r = anchor.rect.unwrap();
        if anchor.info.kind != "definedName" || r.rows() != 1 || r.columns() != 1 {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_typed_field_single_cell_name_required(),
            )));
        }
        register(anchor, r)?;
        writes
            .entry(anchor.path.clone().unwrap())
            .or_default()
            .insert((r.first_row, r.first_col), Some(field.cell.clone()));
    }
    for query in &template.queries {
        let sheet = request.sheets.get(query.sheet).ok_or_else(|| {
            refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_query_sheet_unavailable(),
            ))
        })?;
        let anchor = find_anchor(&anchors, &query.anchor)?;
        let r = anchor.rect.unwrap();
        if r.columns() as usize != sheet.columns.len()
            || sheet.result.rows.len() > anchor.info.capacity as usize
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_query_anchor_bounds_exceeded(),
            )));
        }
        if let Some(columns) = &anchor.table_columns
            && columns != &sheet.columns
        {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_query_columns_mismatch(),
            )));
        }
        register(anchor, r)?;
        mapped.insert(query.sheet);
        let target = writes.entry(anchor.path.clone().unwrap()).or_default();
        if anchor.info.kind == "definedName" {
            for (c, name) in sheet.columns.iter().enumerate() {
                target.insert(
                    (r.first_row, r.first_col + c as u32),
                    Some(Cell::value(NativeValue::Text(name.clone()))),
                );
            }
        }
        for row in 0..anchor.info.capacity {
            for (c, name) in sheet.columns.iter().enumerate() {
                let cell = sheet
                    .result
                    .rows
                    .get(row as usize)
                    .map(|row| row.cells.get(name).cloned().unwrap_or(Cell::Absent));
                target.insert((r.first_row + 1 + row, r.first_col + c as u32), cell);
            }
        }
    }
    if mapped.len() != request.sheets.len() {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_query_sheet_mapping_required(),
        )));
    }
    if writes.values().map(BTreeMap::len).sum::<usize>() > MAX_CELLS {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_combined_mapping_cells_exceeded(),
        )));
    }
    for (path, values) in writes {
        let bytes = rewrite_sheet(part(parts, &path)?, &values, &path, losses)?;
        parts.insert(path, bytes);
    }
    // Values feeding existing formulas changed. Remove stale cached results,
    // retain the formulas/references, and ask applications to recalculate.
    let mut formula_count = 0;
    for sheet in &workbook.sheets {
        let bytes = part(parts, &sheet.path)?.to_vec();
        let root = parse_xml(&bytes)?;
        let mut cells = vec![];
        root.descendants("c", &mut cells);
        let mut edits = vec![];
        for c in cells {
            if c.child("f").is_some() {
                formula_count += 1;
                if let Some(v) = c.child("v") {
                    edits.push((v.start, v.end, Vec::new()));
                }
            }
        }
        if !edits.is_empty() {
            parts.insert(sheet.path.clone(), apply_edits(&bytes, edits)?);
        }
    }
    if formula_count > 0 {
        let bytes = part(parts, "xl/workbook.xml")?.to_vec();
        let root = parse_xml(&bytes)?;
        let calc = if let Some(old) = root.child("calcPr") {
            let mut attrs = old.attrs.clone();
            attrs.insert("fullCalcOnLoad".into(), "1".into());
            attrs.insert("forceFullCalc".into(), "1".into());
            let text = empty_element("calcPr", &attrs);
            apply_edits(&bytes, vec![(old.start, old.end, text.into_bytes())])?
        } else {
            let entry = b"<calcPr fullCalcOnLoad=\"1\" forceFullCalc=\"1\"/>";
            // Workbook children have schema order. Extensions/pivot/custom
            // views must stay after calcPr rather than being reordered.
            if let Some(next) = root.children.iter().find(|e| {
                matches!(
                    e.local(),
                    "oleSize"
                        | "customWorkbookViews"
                        | "pivotCaches"
                        | "smartTagPr"
                        | "smartTagTypes"
                        | "webPublishing"
                        | "fileRecoveryPr"
                        | "webPublishObjects"
                        | "extLst"
                )
            }) {
                apply_edits(&bytes, vec![(next.start, next.start, entry.to_vec())])?
            } else {
                append_child(&bytes, &root, std::str::from_utf8(entry).unwrap())?
            }
        };
        parts.insert("xl/workbook.xml".into(), calc);
        loss(
            losses,
            "template_formula_recalculation",
            weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_formula_recalculation_requested(),
            ),
            "workbook",
        );
    }
    loss(
        losses,
        "template_fixed_bounds",
        weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_bounded_slots_preserve_structure(),
        ),
        "workbook",
    );
    Ok(())
}
fn empty_element(name: &str, attrs: &BTreeMap<String, String>) -> String {
    let mut out = format!("<{name}");
    for (k, v) in attrs {
        out.push_str(&format!(" {k}=\"{}\"", xml_text(v)));
    }
    out.push_str("/>");
    out
}
fn apply_edits(bytes: &[u8], mut edits: Vec<(usize, usize, Vec<u8>)>) -> Result<Vec<u8>> {
    edits.sort_by_key(|e| (e.0, e.1));
    let mut out = vec![];
    let mut cursor = 0;
    for (start, end, replacement) in edits {
        if start < cursor || end < start || end > bytes.len() {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_xml_replacements_invalid_or_overlapping(),
            )));
        }
        out.extend_from_slice(&bytes[cursor..start]);
        out.extend_from_slice(&replacement);
        cursor = end;
    }
    out.extend_from_slice(&bytes[cursor..]);
    if out.len() > MAX_PART {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_modified_sheet_bytes_exceeded(),
        )));
    }
    Ok(out)
}
fn target_cell(
    cell: Option<&Cell>,
    row: u32,
    col: u32,
    style: Option<&str>,
    location: &str,
    losses: &mut Vec<ExportLoss>,
) -> Result<String> {
    let a = address(row, col);
    if let Some(cell) = cell {
        cell_xml(cell, &a, style, &format!("{location}!{a}"), losses)
    } else {
        let style = style
            .map(|s| format!(" s=\"{}\"", xml_text(s)))
            .unwrap_or_default();
        Ok(format!("<c r=\"{a}\"{style}/>"))
    }
}
fn rewrite_row(
    bytes: &[u8],
    row: &Element,
    row_number: u32,
    values: &BTreeMap<u32, Option<Cell>>,
    location: &str,
    losses: &mut Vec<ExportLoss>,
) -> Result<Vec<u8>> {
    let mut cells = BTreeMap::new();
    let mut last = 0;
    for c in &row.children {
        if c.local() != "c" || c.name.contains(':') {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_edited_row_content_unsupported(),
            )));
        }
        let (r, col) = parse_address(
            c.attr("r").ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_template_cell_address_missing(),
                ))
            })?,
            false,
        )?;
        if r != row_number || col <= last {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_row_cell_address_order_invalid(),
            )));
        }
        last = col;
        cells.insert(col, c);
    }
    let mut pending = values.clone();
    let mut out = vec![];
    if row.empty {
        let open = &bytes[row.start..row.end - 2];
        out.extend_from_slice(open);
        out.extend_from_slice(b">");
    } else {
        out.extend_from_slice(&bytes[row.start..row.open_end]);
    }
    let mut cursor = row.open_end;
    for (col, c) in cells {
        if !row.empty {
            out.extend_from_slice(&bytes[cursor..c.start]);
        }
        let earlier: Vec<_> = pending.keys().copied().take_while(|k| *k < col).collect();
        for earlier in earlier {
            let value = pending.remove(&earlier).unwrap();
            out.extend_from_slice(
                target_cell(value.as_ref(), row_number, earlier, None, location, losses)?
                    .as_bytes(),
            );
        }
        if let Some(value) = pending.remove(&col) {
            out.extend_from_slice(
                target_cell(
                    value.as_ref(),
                    row_number,
                    col,
                    c.attr("s"),
                    location,
                    losses,
                )?
                .as_bytes(),
            );
        } else {
            out.extend_from_slice(&bytes[c.start..c.end]);
        }
        cursor = c.end;
    }
    if !row.empty {
        out.extend_from_slice(&bytes[cursor..row.close_start]);
    }
    for (col, value) in pending {
        out.extend_from_slice(
            target_cell(value.as_ref(), row_number, col, None, location, losses)?.as_bytes(),
        );
    }
    if row.empty {
        out.extend_from_slice(format!("</{}>", row.name).as_bytes());
    } else {
        out.extend_from_slice(&bytes[row.close_start..row.end]);
    }
    Ok(out)
}
fn new_row(
    row: u32,
    values: &BTreeMap<u32, Option<Cell>>,
    location: &str,
    losses: &mut Vec<ExportLoss>,
) -> Result<Vec<u8>> {
    let mut out = format!("<row r=\"{row}\">");
    for (col, value) in values {
        out.push_str(&target_cell(
            value.as_ref(),
            row,
            *col,
            None,
            location,
            losses,
        )?);
    }
    out.push_str("</row>");
    Ok(out.into_bytes())
}
fn rewrite_sheet(
    bytes: &[u8],
    writes: &Writes,
    location: &str,
    losses: &mut Vec<ExportLoss>,
) -> Result<Vec<u8>> {
    let root = parse_xml(bytes)?;
    let data = root.child("sheetData").ok_or_else(|| {
        refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_sheet_data_missing(),
        ))
    })?;
    if data.name.contains(':') {
        return Err(refuse(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_xlsx_prefixed_sheet_data_insertion_unsupported(),
        )));
    }
    let mut pending: BTreeMap<u32, BTreeMap<u32, Option<Cell>>> = BTreeMap::new();
    for ((r, c), cell) in writes {
        pending.entry(*r).or_default().insert(*c, cell.clone());
    }
    let mut out = vec![];
    if data.empty {
        out.extend_from_slice(&bytes[data.start..data.end - 2]);
        out.extend_from_slice(b">");
    } else {
        out.extend_from_slice(&bytes[data.start..data.open_end]);
    }
    let mut cursor = data.open_end;
    let mut last_row = 0;
    for row in &data.children {
        if row.local() != "row" || row.name.contains(':') {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_sheet_data_content_unsupported(),
            )));
        }
        let r = row
            .attr("r")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|r| *r > 0 && *r <= MAX_ROWS)
            .ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_row_explicit_address_required(),
                ))
            })?;
        if r <= last_row {
            return Err(refuse(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_xlsx_row_order_invalid(),
            )));
        }
        last_row = r;
        if !data.empty {
            out.extend_from_slice(&bytes[cursor..row.start]);
        }
        let earlier: Vec<_> = pending.keys().copied().take_while(|k| *k < r).collect();
        for earlier in earlier {
            out.extend_from_slice(&new_row(
                earlier,
                &pending.remove(&earlier).unwrap(),
                location,
                losses,
            )?);
        }
        if let Some(values) = pending.remove(&r) {
            out.extend_from_slice(&rewrite_row(bytes, row, r, &values, location, losses)?);
        } else {
            out.extend_from_slice(&bytes[row.start..row.end]);
        }
        cursor = row.end;
    }
    if !data.empty {
        out.extend_from_slice(&bytes[cursor..data.close_start]);
    }
    for (r, values) in pending {
        out.extend_from_slice(&new_row(r, &values, location, losses)?);
    }
    if data.empty {
        out.extend_from_slice(b"</sheetData>");
    } else {
        out.extend_from_slice(&bytes[data.close_start..data.end]);
    }
    let mut edits = vec![(data.start, data.end, out)];
    if let Some(dim) = root.child("dimension") {
        let mut rect = parse_rect(
            dim.attr("ref").ok_or_else(|| {
                refuse(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_xlsx_sheet_dimension_missing(),
                ))
            })?,
            false,
        )?;
        for (r, c) in writes.keys() {
            rect.first_row = rect.first_row.min(*r);
            rect.first_col = rect.first_col.min(*c);
            rect.last_row = rect.last_row.max(*r);
            rect.last_col = rect.last_col.max(*c);
        }
        let mut attrs = dim.attrs.clone();
        attrs.insert(
            "ref".into(),
            format!(
                "{}:{}",
                address(rect.first_row, rect.first_col),
                address(rect.last_row, rect.last_col)
            ),
        );
        edits.push((
            dim.start,
            dim.end,
            empty_element(&dim.name, &attrs).into_bytes(),
        ));
    }
    apply_edits(bytes, edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftext_data::{NativeMember, QueryRow, QueryValueType};

    #[test]
    fn original_utf8_parser_diagnostics_survive_admission_and_all_append_paths() {
        for original in [b"<root/>".as_slice(), b"<root>author</root>".as_slice()] {
            let element = parse_xml(original).unwrap();
            let corrupt_at = if element.empty {
                vec![1]
            } else {
                vec![element.open_end, element.close_start + 2]
            };
            for at in corrupt_at {
                let mut bytes = original.to_vec();
                bytes[at] = 0xff;
                let slice = if element.empty {
                    &bytes[element.start..element.end - 2]
                } else if at < element.close_start {
                    &bytes[element.start..element.close_start]
                } else {
                    &bytes[element.close_start..element.end]
                };
                let expected = std::str::from_utf8(slice).unwrap_err().to_string();
                let error = append_child(&bytes, &element, "<new/>").unwrap_err();
                assert_eq!(
                    error.description.explanation,
                    UiMessage::error_export_xlsx_xml_invalid()
                );
                assert_eq!(
                    error.description.diagnostics,
                    vec![weftext_messages::RawDiagnostic {
                        origin: DiagnosticOrigin::Dependency,
                        text: expected,
                    }]
                );
                let admitted = parse_xml(&bytes).unwrap_err();
                assert_eq!(
                    admitted.description.explanation,
                    UiMessage::error_export_xlsx_template_xml_bounded_utf_eight_required()
                );
                assert_eq!(
                    admitted.description.diagnostics[0].text,
                    std::str::from_utf8(&bytes).unwrap_err().to_string()
                );
            }
        }
        let invalid = relationship_owner("xl/_rels/workbook.xml.bad").unwrap_err();
        assert_eq!(
            invalid.description.explanation,
            UiMessage::error_export_xlsx_relationship_part_name_invalid()
        );
        assert!(invalid.description.diagnostics.is_empty());
    }

    fn input(cells: Vec<(&str, Cell)>) -> XlsxSheetInput {
        let columns = cells.iter().map(|(name, _)| name.to_string()).collect();
        let cells: BTreeMap<_, _> = cells
            .into_iter()
            .map(|(name, cell)| (name.to_string(), cell))
            .collect();
        XlsxSheetInput {
            name: "中文资料".into(),
            columns,
            schema: None,
            result: QueryResult {
                cut_id: "frozen-cut".into(),
                schema_revision: "schema-exact".into(),
                rows: vec![QueryRow {
                    key: "row:0".into(),
                    entity_key: Some("entity:0".into()),
                    path: Some("中文.typ".into()),
                    revision: Some("revision:0".into()),
                    cells,
                    read_only: true,
                    lifecycle: Some("live".into()),
                }],
                coverage: "complete".into(),
                freshness: "current".into(),
                diagnostics: vec![],
                total_known: 1,
                limited: false,
                aggregate_exact: true,
                runtime: "frozen".into(),
                terminal_schema: BTreeMap::new(),
            },
        }
    }
    fn v(value: NativeValue) -> Cell {
        Cell::value(value)
    }
    fn text(value: &str) -> Cell {
        v(NativeValue::Text(value.into()))
    }
    fn unpack(bytes: &[u8]) -> Parts {
        read_template(bytes).unwrap()
    }
    fn sheet_cells(parts: &Parts, index: usize) -> BTreeMap<String, (String, String)> {
        let wb = Workbook::read(parts).unwrap();
        let root = parse_xml(part(parts, &wb.sheets[index].path).unwrap()).unwrap();
        let mut cells = vec![];
        root.descendants("c", &mut cells);
        cells
            .into_iter()
            .map(|c| {
                let value = c
                    .child("v")
                    .map(|v| v.text.clone())
                    .or_else(|| {
                        c.child("is")
                            .and_then(|is| is.child("t"))
                            .map(|t| t.text.clone())
                    })
                    .unwrap_or_default();
                (
                    c.attr("r").unwrap().to_string(),
                    (c.attr("t").unwrap_or("n").to_string(), value),
                )
            })
            .collect()
    }
    fn exact_item(parts: &Parts, item: &str) -> String {
        let wb = Workbook::read(parts).unwrap();
        let cells = sheet_cells(parts, wb.sheets.len() - 1);
        let mut chunks = BTreeMap::new();
        for (r, (_, label)) in &cells {
            if r.starts_with('A') && label == item {
                let row = &r[1..];
                let part = cells[&format!("B{row}")].1.parse::<usize>().unwrap();
                chunks.insert(part, cells[&format!("D{row}")].1.clone());
            }
        }
        chunks.into_values().collect()
    }
    #[test]
    fn real_cells_preserve_exact_numbers_chinese_states_duplicates_and_order() {
        let contact = NativeValue::Record(vec![
            NativeMember {
                key: "phone".into(),
                value: NativeValue::Text("0013800000000".into()),
            },
            NativeMember {
                key: "label".into(),
                value: NativeValue::Text("私人".into()),
            },
        ]);
        let mut s = input(vec![
            ("integer", v(NativeValue::Int("123456789012345".into()))),
            ("large", v(NativeValue::Int("9223372036854775807".into()))),
            (
                "decimal",
                v(NativeValue::Decimal("12345678901234567890.0012300".into())),
            ),
            ("phone", text("+86 0013800000000")),
            ("identifier", text("001234567890123456789")),
            ("bool", v(NativeValue::Bool(false))),
            (
                "formula-looking",
                text("=HYPERLINK(\"https://example.test\",\"点击\")"),
            ),
            (
                "nested",
                v(NativeValue::Array(vec![contact.clone(), contact])),
            ),
            ("absent", Cell::Absent),
            ("null", Cell::Null),
            ("empty", text("")),
            (
                "invalid",
                Cell::Invalid {
                    diagnostics: vec![weftext_messages::UiMessage::error_export_json().into()],
                    value: Some(NativeValue::Text("bad".into())),
                },
            ),
            (
                "unavailable",
                Cell::Unavailable {
                    kind: weftext_data::DataErrorKind::Coverage,
                    reason: weftext_messages::UiMessage::error_export_data_qualification().into(),
                    diagnostics: vec![],
                },
            ),
            (
                "month",
                v(NativeValue::Record(vec![
                    NativeMember {
                        key: "precision".into(),
                        value: NativeValue::Text("month".into()),
                    },
                    NativeMember {
                        key: "year".into(),
                        value: NativeValue::Int("2020".into()),
                    },
                    NativeMember {
                        key: "month".into(),
                        value: NativeValue::Int("3".into()),
                    },
                ])),
            ),
        ]);
        s.result
            .terminal_schema
            .insert("nested".into(), QueryValueType::Any);
        let artifact = export_xlsx(&XlsxRequest {
            sheets: vec![s.clone()],
            template: None,
        })
        .unwrap();
        assert_eq!(artifact.extension, "xlsx");
        assert!(artifact.bytes.starts_with(b"PK"));
        let parts = unpack(&artifact.bytes);
        let cells = sheet_cells(&parts, 0);
        assert_eq!(cells["A2"], ("n".into(), "123456789012345".into()));
        assert_eq!(
            cells["B2"],
            ("inlineStr".into(), "9223372036854775807".into())
        );
        assert_eq!(cells["C2"].1, "12345678901234567890.0012300");
        assert_eq!(cells["E2"].1, "001234567890123456789");
        assert_eq!(cells["F2"], ("b".into(), "0".into()));
        assert_eq!(cells["G2"].0, "inlineStr");
        assert_eq!(cells["I2"].1, "<absent>");
        assert_eq!(cells["J2"].1, "<null>");
        assert_eq!(cells["K2"].1, "");
        assert_eq!(
            serde_json::from_str::<Cell>(&cells["L2"].1).unwrap(),
            s.result.rows[0].cells["invalid"]
        );
        assert_eq!(
            serde_json::from_str::<Cell>(&cells["M2"].1).unwrap(),
            s.result.rows[0].cells["unavailable"]
        );
        let exact: serde_json::Value =
            serde_json::from_str(&exact_item(&parts, "sheet:0")).unwrap();
        assert_eq!(exact, serde_json::to_value(s).unwrap());
        assert!(
            artifact
                .losses
                .iter()
                .any(|l| l.code == "integer_as_exact_text")
        );
        assert!(artifact.losses.iter().any(|l| l.code == "nested_structure"));
        for bytes in parts.values() {
            if let Ok(xml) = std::str::from_utf8(bytes) {
                assert!(!xml.contains("<f>HYPERLINK"));
            }
        }
    }
    #[test]
    fn long_text_has_an_explicit_display_report_and_complete_ordered_chunks() {
        let full = "正文🙂\n".repeat(8000);
        let s = input(vec![("long", text(&full))]);
        let artifact = export_xlsx(&XlsxRequest {
            sheets: vec![s.clone()],
            template: None,
        })
        .unwrap();
        assert!(
            artifact
                .losses
                .iter()
                .any(|l| l.code == "cell_display_shortened")
        );
        let parts = unpack(&artifact.bytes);
        assert!(sheet_cells(&parts, 0)["A2"].1.encode_utf16().count() < 32_767);
        let exact: serde_json::Value =
            serde_json::from_str(&exact_item(&parts, "sheet:0")).unwrap();
        assert_eq!(exact, serde_json::to_value(s).unwrap());
    }
    #[test]
    fn string_escape_spellings_and_controls_cannot_become_xml_or_formulas() {
        let s = input(vec![("escaped", text("_x000A_\u{1}\r<&=cmd"))]);
        let artifact = export_xlsx(&XlsxRequest {
            sheets: vec![s],
            template: None,
        })
        .unwrap();
        let parts = unpack(&artifact.bytes);
        let wb = Workbook::read(&parts).unwrap();
        let raw = std::str::from_utf8(part(&parts, &wb.sheets[0].path).unwrap()).unwrap();
        assert!(raw.contains("_x005F_x000A__x0001__x000D_&lt;&amp;=cmd"));
        assert!(!raw.contains('\u{1}'));
    }
    fn template_parts(sheet_body: &str, defined_names: &str) -> Parts {
        let mut parts = blank_package();
        let mut wb = Workbook::read(&parts).unwrap();
        wb.add_sheet(
            &mut parts,
            "报表",
            format!("<worksheet xmlns=\"{MAIN_NS}\" xmlns:r=\"{REL_NS}\">{sheet_body}</worksheet>")
                .into_bytes(),
        )
        .unwrap();
        let bytes = part(&parts, "xl/workbook.xml").unwrap().to_vec();
        let root = parse_xml(&bytes).unwrap();
        parts.insert(
            "xl/workbook.xml".into(),
            append_child(
                &bytes,
                &root,
                &format!("<definedNames>{defined_names}</definedNames>"),
            )
            .unwrap(),
        );
        parts
    }
    fn template_request(parts: Parts) -> XlsxRequest {
        XlsxRequest {
            sheets: vec![input(vec![
                ("名称", text("张三")),
                ("号码", text("0013800000000")),
            ])],
            template: Some(XlsxTemplate {
                bytes: write_package(parts).unwrap(),
                fields: vec![XlsxFieldSlot {
                    anchor: "ClientName".into(),
                    cell: text("客户甲"),
                }],
                queries: vec![XlsxQuerySlot {
                    anchor: "ReportRows".into(),
                    sheet: 0,
                }],
            }),
        }
    }
    fn standard_template() -> Parts {
        template_parts(
            "<dimension ref=\"A1:D6\"/><cols><col min=\"1\" max=\"2\" width=\"30\" customWidth=\"1\"/></cols><sheetData><row r=\"1\"><c r=\"D1\" t=\"inlineStr\"><is><t>客户</t></is></c></row><row r=\"3\"><c r=\"A3\" t=\"inlineStr\"><is><t>旧表头</t></is></c></row><row r=\"4\"><c r=\"A4\" t=\"inlineStr\"><is><t>旧行</t></is></c></row><row r=\"5\"><c r=\"A5\"><v>999</v></c></row><row r=\"6\"><c r=\"D6\"><f>SUM(A4:A5)</f><v>999</v></c></row></sheetData><pageMargins left=\"1\" right=\"1\" top=\"1\" bottom=\"1\" header=\"0.3\" footer=\"0.3\"/>",
            "<definedName name=\"ClientName\">'报表'!$D$1</definedName><definedName name=\"ReportRows\">'报表'!$A$3:$B$5</definedName>",
        )
    }
    #[test]
    fn template_standard_names_fill_fields_and_bounded_rows_without_rewriting_refs() {
        let mut parts = standard_template();
        let opaque = b"opaque unsupported drawing payload".to_vec();
        parts.insert("xl/media/preserved.bin".into(), opaque.clone());
        let request = template_request(parts);
        let inspection = inspect_xlsx_template(&request.template.as_ref().unwrap().bytes).unwrap();
        assert_eq!(inspection.anchors.len(), 2);
        assert!(inspection.anchors.iter().all(|a| a.supported));
        let artifact = export_xlsx(&request).unwrap();
        let parts = unpack(&artifact.bytes);
        let cells = sheet_cells(&parts, 0);
        assert_eq!(cells["D1"].1, "客户甲");
        assert_eq!(cells["A3"].1, "名称");
        assert_eq!(cells["A4"].1, "张三");
        assert_eq!(cells["B4"].1, "0013800000000");
        assert_eq!(cells["A5"].1, "");
        assert_eq!(parts["xl/media/preserved.bin"], opaque);
        let xml = std::str::from_utf8(part(&parts, "xl/worksheets/weftext1.xml").unwrap()).unwrap();
        assert!(
            xml.contains("<cols><col min=\"1\" max=\"2\" width=\"30\" customWidth=\"1\"/></cols>")
        );
        assert!(xml.contains("<f>SUM(A4:A5)</f>"));
        assert!(!xml.contains("<v>999</v>"));
        assert!(
            artifact
                .losses
                .iter()
                .any(|l| l.code == "template_formula_recalculation")
        );
        assert!(
            std::str::from_utf8(part(&parts, "xl/workbook.xml").unwrap())
                .unwrap()
                .contains("fullCalcOnLoad=\"1\"")
        );
    }
    #[test]
    fn template_bounds_overlap_and_omitted_sheets_are_refused() {
        let mut request = template_request(standard_template());
        request.sheets[0].result.rows = vec![request.sheets[0].result.rows[0].clone(); 3];
        assert!(
            export_xlsx(&request)
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("fixed bounds")
        );
        let mut request = template_request(standard_template());
        let duplicate = request.template.as_ref().unwrap().fields[0].clone();
        request.template.as_mut().unwrap().fields.push(duplicate);
        assert!(
            export_xlsx(&request)
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("overlap")
        );
        let mut request = template_request(standard_template());
        request.template.as_mut().unwrap().queries.clear();
        assert!(
            export_xlsx(&request)
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("Every frozen")
        );
    }
    #[test]
    fn unsupported_names_merges_and_formula_slots_have_honest_inspection() {
        let parts = template_parts(
            "<sheetData><row r=\"1\"><c r=\"A1\"><f>1+1</f><v>2</v></c></row></sheetData><mergeCells><mergeCell ref=\"C1:D1\"/></mergeCells>",
            "<definedName name=\"Dynamic\">OFFSET('报表'!$A$1,0,0,2,2)</definedName><definedName name=\"Relative\">'报表'!A1</definedName><definedName name=\"Formula\">'报表'!$A$1</definedName><definedName name=\"Merged\">'报表'!$C$1</definedName><definedName name=\"Scoped\" localSheetId=\"0\">'报表'!$E$1</definedName>",
        );
        let inspection = inspect_xlsx_template(&write_package(parts).unwrap()).unwrap();
        assert_eq!(inspection.anchors.len(), 5);
        assert!(
            inspection
                .anchors
                .iter()
                .all(|a| !a.supported && a.reason.is_some())
        );
    }
    #[test]
    fn standard_table_anchor_preserves_headers_bounds_and_table_definition() {
        let mut parts = template_parts(
            "<sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>名称</t></is></c><c r=\"B1\" t=\"inlineStr\"><is><t>号码</t></is></c></row></sheetData><tableParts count=\"1\"><tablePart r:id=\"table1\"/></tableParts>",
            "",
        );
        parts.insert("xl/worksheets/_rels/weftext1.xml.rels".into(),format!("<Relationships xmlns=\"{PKG_REL_NS}\"><Relationship Id=\"table1\" Type=\"{REL_NS}/table\" Target=\"../tables/table1.xml\"/></Relationships>").into_bytes());
        let table=format!("<table xmlns=\"{MAIN_NS}\" id=\"1\" name=\"People\" displayName=\"People\" ref=\"A1:B3\" totalsRowShown=\"0\"><autoFilter ref=\"A1:B3\"/><tableColumns count=\"2\"><tableColumn id=\"1\" name=\"名称\"/><tableColumn id=\"2\" name=\"号码\"/></tableColumns><tableStyleInfo name=\"TableStyleMedium2\" showRowStripes=\"1\"/></table>").into_bytes();
        parts.insert("xl/tables/table1.xml".into(), table.clone());
        let request = XlsxRequest {
            sheets: vec![input(vec![("名称", text("张三")), ("号码", text("00123"))])],
            template: Some(XlsxTemplate {
                bytes: write_package(parts).unwrap(),
                fields: vec![],
                queries: vec![XlsxQuerySlot {
                    anchor: "People".into(),
                    sheet: 0,
                }],
            }),
        };
        let info = inspect_xlsx_template(&request.template.as_ref().unwrap().bytes).unwrap();
        assert_eq!(info.anchors[0].capacity, 2);
        let output = export_xlsx(&request).unwrap();
        let parts = unpack(&output.bytes);
        assert_eq!(parts["xl/tables/table1.xml"], table);
        let cells = sheet_cells(&parts, 0);
        assert_eq!(cells["A1"].1, "名称");
        assert_eq!(cells["A2"].1, "张三");
        assert_eq!(cells["B2"].1, "00123");
        let mut wrong = request;
        wrong.sheets[0].columns.reverse();
        assert!(
            export_xlsx(&wrong)
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("column order")
        );
    }
    #[test]
    fn malicious_workbooks_cannot_fetch_external_parts_or_expand_entities() {
        let mut parts = standard_template();
        parts.insert("xl/vbaProject.bin".into(), b"macro".to_vec());
        assert!(
            inspect_xlsx_template(&write_package(parts).unwrap())
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("Macros")
        );
        let mut parts = standard_template();
        parts.insert("xl/_rels/workbook.xml.rels".into(),format!("<Relationships xmlns=\"{PKG_REL_NS}\"><Relationship Id=\"remote\" Type=\"{REL_NS}/worksheet\" TargetMode=\"External\" Target=\"https://example.test/workbook.xml\"/></Relationships>").into_bytes());
        assert!(
            inspect_xlsx_template(&write_package(parts).unwrap())
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("External")
        );
        let mut parts = standard_template();
        parts.insert("xl/worksheets/weftext1.xml".into(),b"<!DOCTYPE worksheet [<!ENTITY steal SYSTEM 'file:///C:/secret'>]><worksheet>&steal;</worksheet>".to_vec());
        assert!(
            inspect_xlsx_template(&write_package(parts).unwrap())
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("DTD")
        );
        let mut parts = standard_template();
        parts.insert("../evil.xml".into(), b"<x/>".to_vec());
        assert!(
            inspect_xlsx_template(&write_package(parts).unwrap())
                .unwrap_err()
                .description
                .explanation
                .render(weftext_messages::UiLocale::EnUs)
                .contains("Unsafe")
        );
    }
    #[test]
    fn frozen_query_columns_and_partial_state_never_disappear_silently() {
        let mut s = input(vec![("included", text("a")), ("omitted", text("b"))]);
        s.columns.pop();
        assert!(
            export_xlsx(&XlsxRequest {
                sheets: vec![s],
                template: None
            })
            .unwrap_err()
            .description
            .explanation
            .render(weftext_messages::UiLocale::EnUs)
            .contains("omits")
        );
        let mut s = input(vec![("total", v(NativeValue::Int("10".into())))]);
        s.result.limited = true;
        s.result.coverage = "partial".into();
        s.result.aggregate_exact = false;
        s.result.freshness = "stale".into();
        let artifact = export_xlsx(&XlsxRequest {
            sheets: vec![s],
            template: None,
        })
        .unwrap();
        assert!(artifact.losses.iter().any(|l| l.code == "query_incomplete"));
        assert!(artifact.losses.iter().any(|l| l.code == "query_freshness"));
    }

    #[test]
    fn error_cells_keep_all_parallel_diagnostics_and_exact_author_values_in_real_xlsx() {
        let diagnostics: Vec<_> = (0..40)
            .map(|index| {
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::common_operation(
                        weftext_messages::params::CommonOperation {
                            operation_id: format!("diagnostic-{index}"),
                        },
                    ),
                )
                .diagnostic(
                    weftext_messages::DiagnosticOrigin::Dependency,
                    format!("original-{index} {{value}} 😀\r\n"),
                )
            })
            .collect();
        let invalid = Cell::Invalid {
            diagnostics: diagnostics.clone(),
            value: Some(NativeValue::Decimal(
                "123456789012345678901234567890.000123000".into(),
            )),
        };
        let unavailable = Cell::Unavailable {
            kind: weftext_data::DataErrorKind::NativeBudget,
            reason: weftext_messages::UiMessage::error_export_data_qualification().into(),
            diagnostics,
        };
        let s = input(vec![
            ("invalid", invalid.clone()),
            ("unavailable", unavailable.clone()),
        ]);
        let artifact = export_xlsx(&XlsxRequest {
            sheets: vec![s.clone()],
            template: None,
        })
        .unwrap();
        let parts = unpack(&artifact.bytes);
        let cells = sheet_cells(&parts, 0);
        assert_eq!(cells["A2"].0, "inlineStr");
        assert_eq!(
            serde_json::from_str::<Cell>(&cells["A2"].1).unwrap(),
            invalid
        );
        assert_eq!(
            serde_json::from_str::<Cell>(&cells["B2"].1).unwrap(),
            unavailable
        );
        let exact: serde_json::Value =
            serde_json::from_str(&exact_item(&parts, "sheet:0")).unwrap();
        assert_eq!(exact, serde_json::to_value(&s).unwrap());
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&cells["A2"].1).unwrap()["diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            40
        );
    }
}
