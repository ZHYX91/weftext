//! Pure export conversions consume frozen values and bytes, never live files.
//! Product reports keep typed explanations, original external diagnostics and
//! complete parallel product diagnostics. Native package v2 carries this exact
//! loss report; readers reject earlier package versions without rewriting them.
pub mod bibliography;
pub mod bibliography_import;
pub mod docx;
pub mod exchange;
pub mod recognition;
pub mod recovery_backup;
pub mod xlsx;
use serde::{Deserialize, Serialize};
use weftext_messages::{DiagnosticOrigin, ErrorDescription, UiMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportLoss {
    pub code: String,
    #[serde(flatten)]
    pub description: ErrorDescription,
    pub product_diagnostics: Vec<ErrorDescription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportArtifact {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub extension: String,
    pub losses: Vec<ExportLoss>,
    pub summary: UiMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportError {
    #[serde(flatten)]
    pub description: ErrorDescription,
    pub product_diagnostics: Vec<ErrorDescription>,
}
impl ExportError {
    pub fn new(description: impl Into<ErrorDescription>) -> Self {
        Self {
            description: description.into(),
            product_diagnostics: Vec::new(),
        }
    }
    pub fn with_diagnostics(mut self, diagnostics: Vec<ErrorDescription>) -> Self {
        self.product_diagnostics.extend(diagnostics);
        self
    }
    pub fn into_parts(self) -> (ErrorDescription, Vec<ErrorDescription>) {
        (self.description, self.product_diagnostics)
    }
    pub fn from_engine(error: weftext_engine::EngineError) -> Self {
        let (description, diagnostics) = error.into_parts();
        Self::new(description).with_diagnostics(diagnostics)
    }
    pub fn from_data(error: weftext_data::DataError) -> Self {
        let (kind, description, details) = error.into_parts();
        let code = match kind {
            weftext_data::DataErrorKind::Invalid => "dataInvalid",
            weftext_data::DataErrorKind::Unavailable => "dataUnavailable",
            weftext_data::DataErrorKind::Coverage => "dataCoverage",
            weftext_data::DataErrorKind::Budget => "dataBudget",
            weftext_data::DataErrorKind::NativeBudget => "dataNativeBudget",
            weftext_data::DataErrorKind::ClosedExpressionUnavailable => {
                "dataClosedExpressionUnavailable"
            }
        };
        Self::with_cause(
            UiMessage::error_export_data_qualification(),
            code,
            description,
            details
                .map(weftext_data::DataErrorDetails::into_diagnostics)
                .unwrap_or_default(),
        )
    }
    pub fn from_native(error: exchange::NativeError) -> Self {
        let code = error.code.clone();
        let (description, details) = error.into_parts();
        Self::with_cause(
            UiMessage::error_export_native_package_invalid(),
            &code,
            description,
            details,
        )
    }
    pub(crate) fn from_losses(explanation: UiMessage, losses: Vec<ExportLoss>) -> Self {
        let mut diagnostics = Vec::new();
        for loss in losses {
            let context = match loss.location {
                Some(location) => UiMessage::error_export_loss_at_location(
                    weftext_messages::params::ErrorExportLossAtLocation { location },
                ),
                None => UiMessage::error_export_loss_reported(),
            };
            let (description, related) = Self::with_cause(
                context,
                &loss.code,
                loss.description,
                loss.product_diagnostics,
            )
            .into_parts();
            diagnostics.push(description);
            diagnostics.extend(related);
        }
        Self::new(explanation).with_diagnostics(diagnostics)
    }
    pub(crate) fn with_cause(
        explanation: UiMessage,
        code: &str,
        cause: ErrorDescription,
        mut details: Vec<ErrorDescription>,
    ) -> Self {
        let description = match ErrorDescription::new(explanation).try_caused_by(code, cause) {
            Ok(description) => description,
            Err(boundary) => {
                // Keep the entire original full chain. The fresh shallow header
                // carries the source's machine code without truncating evidence.
                let header = ErrorDescription::new(boundary.original.explanation.clone());
                details.insert(0, boundary.original);
                boundary
                    .outer
                    .try_caused_by(boundary.code, header)
                    .expect("a fresh product header and one shallow cause fit the fixed budget")
            }
        };
        Self {
            description,
            product_diagnostics: details,
        }
    }
}
impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.description.fmt(f)?;
        for diagnostic in &self.product_diagnostics {
            write!(f, "\n{diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for ExportError {}
impl From<std::io::Error> for ExportError {
    fn from(value: std::io::Error) -> Self {
        if let Some(description) = value
            .get_ref()
            .and_then(|error| error.downcast_ref::<ErrorDescription>())
        {
            return Self::new(description.clone());
        }
        let origin = if value.raw_os_error().is_some() {
            DiagnosticOrigin::Os
        } else {
            DiagnosticOrigin::Dependency
        };
        Self::new(
            ErrorDescription::new(UiMessage::error_export_io())
                .diagnostic(origin, value.to_string()),
        )
    }
}
impl From<serde_json::Error> for ExportError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(
            ErrorDescription::new(UiMessage::error_export_json())
                .diagnostic(DiagnosticOrigin::Json, value.to_string()),
        )
    }
}
impl From<zip::result::ZipError> for ExportError {
    fn from(value: zip::result::ZipError) -> Self {
        if let zip::result::ZipError::Io(error) = value {
            return Self::from(error);
        }
        Self::new(
            ErrorDescription::new(UiMessage::error_export_archive())
                .diagnostic(DiagnosticOrigin::Dependency, value.to_string()),
        )
    }
}
pub type Result<T> = std::result::Result<T, ExportError>;
