//! Pure structured-data and Query transformations. Persistence and authorization
//! belong to Core; this crate accepts frozen bytes and never opens files.
mod embedded_query;
mod native;
mod projection;
pub use embedded_query::*;
mod query;
mod query_native_products;
pub use query_native_products::native_query_products;
mod query_providers;
pub use query_providers::*;
pub mod query_advanced;
pub use query_advanced::*;
mod recurrence;
mod relations;
mod schema;
mod tag_forest;
mod task_reminder;
mod temporal;
pub use tag_forest::*;
mod tag_remap;
pub use native::*;
pub use projection::*;
pub use query::*;
pub use recurrence::*;
pub use relations::*;
pub use schema::*;
pub use tag_remap::*;
pub use task_reminder::*;
pub use temporal::*;

use serde::{Deserialize, Serialize};
use std::fmt;
pub use weftext_messages::{DiagnosticOrigin, ErrorDescription, RawDiagnostic, UiMessage};

/// Semantic admission facts. They never depend on rendered text or message keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataErrorKind {
    Invalid,
    Unavailable,
    Coverage,
    Budget,
    NativeBudget,
    ClosedExpressionUnavailable,
}
#[derive(Debug, Clone)]
pub struct DataError {
    kind: DataErrorKind,
    description: ErrorDescription,
    details: Option<DataErrorDetails>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum DataErrorDetails {
    Diagnostics { diagnostics: Vec<ErrorDescription> },
}
impl DataErrorDetails {
    pub fn into_diagnostics(self) -> Vec<ErrorDescription> {
        match self {
            Self::Diagnostics { diagnostics } => diagnostics,
        }
    }
}
impl DataError {
    pub fn new(message: impl Into<ErrorDescription>) -> Self {
        Self::with_kind(DataErrorKind::Invalid, message)
    }
    pub fn unavailable(message: impl Into<ErrorDescription>) -> Self {
        Self::with_kind(DataErrorKind::Unavailable, message)
    }
    pub fn coverage(message: impl Into<ErrorDescription>) -> Self {
        Self::with_kind(DataErrorKind::Coverage, message)
    }
    pub fn budget(message: impl Into<ErrorDescription>) -> Self {
        Self::with_kind(DataErrorKind::Budget, message)
    }
    pub fn native_budget(message: impl Into<ErrorDescription>) -> Self {
        Self::with_kind(DataErrorKind::NativeBudget, message)
    }
    pub fn closed_expression(message: impl Into<ErrorDescription>) -> Self {
        Self::with_kind(DataErrorKind::ClosedExpressionUnavailable, message)
    }
    pub fn with_kind(kind: DataErrorKind, message: impl Into<ErrorDescription>) -> Self {
        Self {
            kind,
            description: message.into(),
            details: None,
        }
    }
    pub fn kind(&self) -> DataErrorKind {
        self.kind
    }
    pub fn is_unavailable(&self) -> bool {
        self.kind != DataErrorKind::Invalid
    }
    pub fn diagnostic(mut self, origin: DiagnosticOrigin, text: impl Into<String>) -> Self {
        self.description = self.description.diagnostic(origin, text);
        self
    }
    pub fn description(&self) -> &ErrorDescription {
        &self.description
    }
    pub fn with_diagnostics(mut self, diagnostics: Vec<ErrorDescription>) -> Self {
        match &mut self.details {
            Some(DataErrorDetails::Diagnostics {
                diagnostics: existing,
            }) => existing.extend(diagnostics),
            None => self.details = Some(DataErrorDetails::Diagnostics { diagnostics }),
        }
        self
    }
    pub fn details(&self) -> Option<&DataErrorDetails> {
        self.details.as_ref()
    }
    pub fn into_parts(self) -> (DataErrorKind, ErrorDescription, Option<DataErrorDetails>) {
        (self.kind, self.description, self.details)
    }
    pub fn into_diagnostics(self) -> Vec<ErrorDescription> {
        let mut diagnostics = vec![self.description];
        if let Some(details) = self.details {
            diagnostics.extend(details.into_diagnostics());
        }
        diagnostics
    }
}
impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.description.fmt(f)?;
        if let Some(DataErrorDetails::Diagnostics { diagnostics }) = &self.details {
            for diagnostic in diagnostics {
                write!(f, "\n{diagnostic}")?;
            }
        }
        Ok(())
    }
}
impl std::error::Error for DataError {}
impl From<serde_json::Error> for DataError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(UiMessage::error_data_json_invalid())
            .diagnostic(DiagnosticOrigin::Json, e.to_string())
    }
}
impl From<weftext_engine::EngineError> for DataError {
    fn from(error: weftext_engine::EngineError) -> Self {
        let (description, diagnostics) = error.into_parts();
        Self::new(description).with_diagnostics(diagnostics)
    }
}
pub type Result<T> = std::result::Result<T, DataError>;

#[cfg(test)]
mod error_bridge_tests {
    use super::*;

    #[test]
    fn engine_bridge_preserves_parallel_product_evidence_beyond_cause_budget() {
        let parallel = (0..96)
            .map(|_| {
                ErrorDescription::new(UiMessage::error_engine_world_frozen_packages_unavailable())
            })
            .collect::<Vec<_>>();
        let engine = weftext_engine::EngineError::new(
            UiMessage::error_engine_notes_frozen_compilation_failed(),
        )
        .diagnostic(DiagnosticOrigin::Typst, "author original 原件😀")
        .with_product_diagnostics(parallel.clone());
        let error = DataError::from(engine);
        let (kind, description, details) = error.into_parts();
        assert_eq!(kind, DataErrorKind::Invalid);
        assert_eq!(description.diagnostics[0].text, "author original 原件😀");
        assert!(description.causes.as_slice().is_empty());
        assert_eq!(details.unwrap().into_diagnostics(), parallel);
    }
}
