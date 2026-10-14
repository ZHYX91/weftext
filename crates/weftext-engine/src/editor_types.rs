use crate::SourceRange;
use serde::{Deserialize, Serialize};

/// These addresses describe one exact source snapshot, never persistent IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralRegion {
    pub range: SourceRange,
    pub text: String,
    pub role: String,
    /// Official paragraph-break ranges inside an ordinary body text run.
    #[serde(default)]
    pub paragraph_breaks: Vec<SourceRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureInfo {
    pub kind: String,
    pub range: SourceRange,
    pub body: Option<SourceRange>,
    pub level: Option<usize>,
    /// `level` is authored markup depth; compiled levels can differ under rules.
    #[serde(default)]
    pub level_writable: bool,
    #[serde(default)]
    pub level_reason: Option<String>,
    pub label: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    pub run_in: bool,
    pub table: Option<TableInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub columns: usize,
    pub rows: usize,
    pub cells: Vec<SourceRange>,
    pub grid_writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRole {
    pub status: String,
    pub text: Option<String>,
    pub range: Option<SourceRange>,
    pub construct: Option<SourceRange>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorMap {
    pub source_hash: String,
    pub syntax_valid: bool,
    pub literal_regions: Vec<LiteralRegion>,
    pub structures: Vec<StructureInfo>,
    pub title: DocumentRole,
    pub subtitle: DocumentRole,
    pub body_insertion: Option<SourceRange>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequest {
    pub source_hash: String,
    pub intent: EditIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EditIntent {
    ReplaceLiteral {
        range: SourceRange,
        text: String,
    },
    InsertBlock {
        at: usize,
        block: BlockInsert,
    },
    SetHeadingLevel {
        range: SourceRange,
        level: usize,
    },
    SetTitle {
        text: Option<String>,
    },
    SetSubtitle {
        text: Option<String>,
    },
    SetRunIn {
        range: SourceRange,
        enabled: bool,
    },
    SetInline {
        range: SourceRange,
        format: InlineFormat,
    },
    ReplaceMath {
        range: SourceRange,
        text: String,
        display: bool,
    },
    ReplaceRaw {
        range: SourceRange,
        text: String,
        language: Option<String>,
        block: bool,
    },
    SetTableCell {
        range: SourceRange,
        text: String,
    },
    TableEdit {
        range: SourceRange,
        action: TableAction,
    },
    ListEdit {
        range: SourceRange,
        action: ListAction,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InlineFormat {
    Strong,
    Emph,
    Strike,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BlockInsert {
    Paragraph {
        text: String,
    },
    Heading {
        text: String,
        level: usize,
    },
    BulletList {
        items: Vec<String>,
    },
    NumberedList {
        items: Vec<String>,
    },
    TermList {
        items: Vec<TermEntry>,
    },
    Math {
        text: String,
        display: bool,
    },
    Table {
        columns: usize,
        cells: Vec<String>,
    },
    Link {
        target: String,
        text: String,
    },
    Raw {
        text: String,
        language: Option<String>,
        block: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermEntry {
    pub term: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TableAction {
    InsertRow {
        index: usize,
    },
    RemoveRow {
        index: usize,
    },
    InsertColumn {
        index: usize,
    },
    RemoveColumn {
        index: usize,
    },
    MoveRow {
        from: usize,
        to: usize,
    },
    Paste {
        row: usize,
        column: usize,
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ListAction {
    Split { at: usize },
    Indent,
    Outdent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePatch {
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub expected: String,
    pub insert: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSelection {
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditOutput {
    pub source: String,
    pub source_hash: String,
    /// Patches are ordered ascending and address the original source.
    pub patches: Vec<SourcePatch>,
    /// UTF-16 source offsets in the resulting draft.
    pub selection: EditSelection,
    pub map: EditorMap,
}
