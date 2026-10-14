//! Pure export conversions consume frozen values and bytes, never live files.
pub mod bibliography;
pub mod bibliography_import;
pub mod docx;
pub mod exchange;
pub mod recovery_backup;
pub mod xlsx;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportLoss {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportArtifact {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub extension: String,
    pub losses: Vec<ExportLoss>,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct ExportError(pub String);
impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ExportError {}
impl From<std::io::Error> for ExportError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<serde_json::Error> for ExportError {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<zip::result::ZipError> for ExportError {
    fn from(value: zip::result::ZipError) -> Self {
        Self(value.to_string())
    }
}
pub type Result<T> = std::result::Result<T, ExportError>;
