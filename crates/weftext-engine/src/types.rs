use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

#[derive(Debug, Clone)]
pub struct EngineError(pub String);
impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for EngineError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRange {
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub raw: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    pub path: String,
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub raw: String,
    pub syntax_kind: String,
    pub is_main: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: String,
    pub message: String,
    pub origin: Option<Origin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderNode {
    pub key: String,
    pub kind: String,
    pub tag: Option<String>,
    pub text: Option<String>,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<RenderNode>,
    pub origin: Option<Origin>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub path: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderOutput {
    pub ok: bool,
    pub html: String,
    pub tree: Option<RenderNode>,
    pub diagnostics: Vec<Diagnostic>,
    pub source_info: SourceInfo,
    pub dependencies: Vec<Dependency>,
    pub font_families: Vec<String>,
    pub compiler_version: String,
    #[serde(default)]
    pub labels: Vec<CompiledLabel>,
    #[serde(default)]
    pub products: Option<crate::RenderProducts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledLabel {
    pub label: String,
    pub count: usize,
    pub origins: Vec<Origin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataField {
    pub key: String,
    pub raw: String,
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub value: Option<serde_json::Value>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataInfo {
    pub from: usize,
    pub to: usize,
    pub fields: Vec<MetadataField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxToken {
    pub kind: String,
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadingInfo {
    pub text: String,
    pub path: Option<String>,
    pub is_main: bool,
    pub level: usize,
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub generated: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInfo {
    pub target: String,
    pub kind: String,
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub syntax_valid: bool,
    pub identity_status: String,
    pub id: Option<String>,
    pub types: Vec<String>,
    pub metadata: Option<MetadataInfo>,
    pub title: Option<String>,
    pub headings: Vec<HeadingInfo>,
    pub links: Vec<LinkInfo>,
    pub syntax_tokens: Vec<SyntaxToken>,
    pub body_insertion: Option<SourceRange>,
    pub writer_status: String,
}

pub(crate) fn utf16(source: &str, at: usize) -> usize {
    source.get(..at).unwrap_or("").encode_utf16().count()
}
