use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};
pub use weftext_messages::{DiagnosticOrigin, ErrorDescription, RawDiagnostic, UiMessage};

#[derive(Debug, Clone)]
pub enum EngineError {
    Product {
        description: ErrorDescription,
        product_diagnostics: Vec<ErrorDescription>,
    },
    DisplayTreeBudget {
        product_diagnostics: Vec<ErrorDescription>,
    },
}
impl EngineError {
    pub fn new(message: impl Into<ErrorDescription>) -> Self {
        Self::Product {
            description: message.into(),
            product_diagnostics: vec![],
        }
    }
    pub fn diagnostic(self, origin: DiagnosticOrigin, text: impl Into<String>) -> Self {
        match self {
            Self::Product {
                description,
                product_diagnostics,
            } => Self::Product {
                description: description.diagnostic(origin, text),
                product_diagnostics,
            },
            Self::DisplayTreeBudget {
                mut product_diagnostics,
            } => {
                product_diagnostics.push(
                    ErrorDescription::new(UiMessage::engine_render_display_tree_budget())
                        .diagnostic(origin, text),
                );
                Self::DisplayTreeBudget {
                    product_diagnostics,
                }
            }
        }
    }
    pub fn with_product_diagnostics(self, diagnostics: Vec<ErrorDescription>) -> Self {
        match self {
            Self::Product {
                description,
                mut product_diagnostics,
            } => {
                product_diagnostics.extend(diagnostics);
                Self::Product {
                    description,
                    product_diagnostics,
                }
            }
            Self::DisplayTreeBudget {
                mut product_diagnostics,
            } => {
                product_diagnostics.extend(diagnostics);
                Self::DisplayTreeBudget {
                    product_diagnostics,
                }
            }
        }
    }
    pub fn description(&self) -> ErrorDescription {
        match self {
            Self::Product { description, .. } => description.clone(),
            Self::DisplayTreeBudget { .. } => UiMessage::engine_render_display_tree_budget().into(),
        }
    }
    pub fn into_parts(self) -> (ErrorDescription, Vec<ErrorDescription>) {
        match self {
            Self::Product {
                description,
                product_diagnostics,
            } => (description, product_diagnostics),
            Self::DisplayTreeBudget {
                product_diagnostics,
            } => (
                UiMessage::engine_render_display_tree_budget().into(),
                product_diagnostics,
            ),
        }
    }
}
impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.description().fmt(f)?;
        let product_diagnostics = match self {
            Self::Product {
                product_diagnostics,
                ..
            }
            | Self::DisplayTreeBudget {
                product_diagnostics,
            } => product_diagnostics,
        };
        for diagnostic in product_diagnostics {
            write!(f, "\n{diagnostic}")?;
        }
        Ok(())
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Diagnostic {
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<UiMessage>,
    pub diagnostics: Vec<RawDiagnostic>,
    pub origin: Option<Origin>,
}

#[cfg(test)]
mod error_contract_tests {
    use super::*;

    #[test]
    fn display_tree_budget_is_a_typed_failure_with_bilingual_explanation() {
        let error = EngineError::DisplayTreeBudget {
            product_diagnostics: vec![],
        };
        assert!(matches!(error, EngineError::DisplayTreeBudget { .. }));
        let (description, diagnostics) = error.into_parts();
        assert!(diagnostics.is_empty());
        assert_eq!(
            description.explanation.key(),
            "engine.render.displayTreeBudget"
        );
        assert!(description.diagnostics.is_empty());
        let wire = serde_json::to_value(&description).unwrap();
        assert_eq!(wire["explanation"]["params"], serde_json::json!({}));
        assert_ne!(
            description
                .explanation
                .render(weftext_messages::UiLocale::ZhCn),
            description
                .explanation
                .render(weftext_messages::UiLocale::EnUs),
        );
    }

    #[test]
    fn original_typst_diagnostic_and_author_range_round_trip_without_product_relabeling() {
        let wire = serde_json::json!({
            "severity": "error",
            "diagnostics": [{"origin":"typst", "text":"unknown variable: 作者原名 <x>"}],
            "origin": {
                "path":"author.typ", "from":3,"to":9,"utf16From":3,"utf16To":5,
                "raw":"作者", "syntaxKind":"Diagnostic", "isMain":true
            }
        });
        let diagnostic: Diagnostic = serde_json::from_value(wire.clone()).unwrap();
        assert!(diagnostic.explanation.is_none());
        assert_eq!(serde_json::to_value(diagnostic).unwrap(), wire);
        let mut missing = wire.clone();
        missing.as_object_mut().unwrap().remove("diagnostics");
        assert!(serde_json::from_value::<Diagnostic>(missing).is_err());
        let mut wrong_origin = wire.clone();
        wrong_origin["diagnostics"][0]["origin"] = "product".into();
        assert!(serde_json::from_value::<Diagnostic>(wrong_origin).is_err());
        let mut unknown_field = wire;
        unknown_field["message"] = "old product text".into();
        assert!(serde_json::from_value::<Diagnostic>(unknown_field).is_err());
    }
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
    pub writer_reason: Option<ErrorDescription>,
    #[serde(default)]
    pub notes: crate::NotesSourceMap,
}

pub(crate) fn utf16(source: &str, at: usize) -> usize {
    source.get(..at).unwrap_or("").encode_utf16().count()
}
