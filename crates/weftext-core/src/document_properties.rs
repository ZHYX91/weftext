use std::fmt;
use std::ops::Range;

use serde::Serialize;
use weftext_asciidoc::{
    DocumentHeaderAttributeKind, DocumentHeaderIssueCode, DocumentHeaderPatchError,
    analyze_document_header, patch_document_header_attribute,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentPropertyKind {
    Descriptive,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentProperty {
    pub name: String,
    pub value: String,
    pub kind: DocumentPropertyKind,
    pub range: Range<u64>,
    pub name_range: Range<u64>,
    pub value_range: Range<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentPropertyDiagnosticCode {
    ParserFailure,
    UnclosedEnvelope,
    InvalidName,
    DuplicateName,
    UnsupportedUnset,
    ContinuedValue,
    ValueTooLarge,
    PropertyLimitExceeded,
    ProcessorControl,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPropertyDiagnostic {
    pub code: DocumentPropertyDiagnosticCode,
    pub message: String,
    pub range: Range<u64>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPropertyAnalysis {
    pub properties: Vec<DocumentProperty>,
    pub diagnostics: Vec<DocumentPropertyDiagnostic>,
    pub header_range: Range<u64>,
}

/// Enumerates bounded literal `AsciiDoc` document-header attributes.
///
/// Processor-control attributes remain exact source but are excluded from the stable Properties
/// projection. Later body redefinitions are outside `header_range` and are never returned.
#[must_use]
pub fn analyze_document_header_properties(source: &str) -> DocumentPropertyAnalysis {
    let header = analyze_document_header(source);
    let properties = header
        .attributes
        .iter()
        .filter(|attribute| attribute.projected && !is_reserved_task_profile_name(&attribute.name))
        .map(|attribute| DocumentProperty {
            name: attribute.name.clone(),
            value: attribute.literal_value.clone().unwrap_or_default(),
            kind: map_property_kind(attribute.kind),
            range: attribute.range.clone(),
            name_range: attribute.name_range.clone(),
            value_range: attribute.value_range.clone(),
        })
        .collect();
    let diagnostics = header
        .issues
        .into_iter()
        .map(|issue| DocumentPropertyDiagnostic {
            code: map_diagnostic_code(issue.code),
            message: issue.message,
            range: issue.range,
            name: issue.name,
        })
        .collect();
    DocumentPropertyAnalysis {
        properties,
        diagnostics,
        header_range: header.range,
    }
}

/// Narrowly sets or removes one literal `AsciiDoc` document-header property.
///
/// # Errors
///
/// Rejects invalid or processor-owned names, duplicate targets, multiline/continued values, and
/// unclosed YAML envelopes. Diagnostics on unrelated properties do not authorize rewriting them.
pub fn patch_document_header_property(
    source: &str,
    name: &str,
    value: Option<&str>,
) -> Result<String, DocumentPropertyPatchError> {
    if is_reserved_task_profile_name(name) {
        return Err(DocumentPropertyPatchError::InvalidName);
    }
    patch_document_header_attribute(source, name, value).map_err(map_patch_error)
}

fn is_reserved_task_profile_name(name: &str) -> bool {
    name == "weftext-task" || name.starts_with("weftext-task-")
}

fn map_property_kind(kind: DocumentHeaderAttributeKind) -> DocumentPropertyKind {
    match kind {
        DocumentHeaderAttributeKind::Descriptive => DocumentPropertyKind::Descriptive,
        DocumentHeaderAttributeKind::Custom | DocumentHeaderAttributeKind::ProcessorControl => {
            DocumentPropertyKind::Custom
        }
    }
}

fn map_diagnostic_code(code: DocumentHeaderIssueCode) -> DocumentPropertyDiagnosticCode {
    match code {
        DocumentHeaderIssueCode::ParserFailure => DocumentPropertyDiagnosticCode::ParserFailure,
        DocumentHeaderIssueCode::UnclosedEnvelope => {
            DocumentPropertyDiagnosticCode::UnclosedEnvelope
        }
        DocumentHeaderIssueCode::InvalidName => DocumentPropertyDiagnosticCode::InvalidName,
        DocumentHeaderIssueCode::DuplicateName => DocumentPropertyDiagnosticCode::DuplicateName,
        DocumentHeaderIssueCode::UnsupportedUnset => {
            DocumentPropertyDiagnosticCode::UnsupportedUnset
        }
        DocumentHeaderIssueCode::ContinuedValue => DocumentPropertyDiagnosticCode::ContinuedValue,
        DocumentHeaderIssueCode::ValueTooLarge => DocumentPropertyDiagnosticCode::ValueTooLarge,
        DocumentHeaderIssueCode::AttributeLimitExceeded => {
            DocumentPropertyDiagnosticCode::PropertyLimitExceeded
        }
        DocumentHeaderIssueCode::ProcessorControl => {
            DocumentPropertyDiagnosticCode::ProcessorControl
        }
    }
}

fn map_patch_error(error: DocumentHeaderPatchError) -> DocumentPropertyPatchError {
    match error {
        DocumentHeaderPatchError::InvalidName => DocumentPropertyPatchError::InvalidName,
        DocumentHeaderPatchError::InvalidValue => DocumentPropertyPatchError::InvalidValue,
        DocumentHeaderPatchError::DuplicateName => DocumentPropertyPatchError::DuplicateName,
        DocumentHeaderPatchError::UnclosedEnvelope => DocumentPropertyPatchError::UnclosedEnvelope,
        DocumentHeaderPatchError::UnsupportedHeader => {
            DocumentPropertyPatchError::UnsupportedHeader
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPropertyPatchError {
    InvalidName,
    InvalidValue,
    DuplicateName,
    UnclosedEnvelope,
    UnsupportedHeader,
}

impl fmt::Display for DocumentPropertyPatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidName => "document property name is invalid or processor-owned",
            Self::InvalidValue => "document property value must be one bounded literal line",
            Self::DuplicateName => "document property is duplicated",
            Self::UnclosedEnvelope => "document YAML envelope is not closed",
            Self::UnsupportedHeader => {
                "document property cannot be patched through unsupported header syntax"
            }
        })
    }
}

impl std::error::Error for DocumentPropertyPatchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_only_literal_header_properties_with_exact_ranges() {
        let source = concat!(
            "---\r\nweftext:\r\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n---\r\n",
            "= 标题\r\n作者 <author@example.com>\r\nv1.0, 2026-08-24\r\n",
            ":lang: zh-CN\r\n:status: in progress\r\n:toc: left\r\n\r\n",
            "正文\r\n:status: body-only\r\n",
        );
        let analysis = analyze_document_header_properties(source);
        assert_eq!(analysis.diagnostics.len(), 1);
        assert_eq!(
            analysis.diagnostics[0].code,
            DocumentPropertyDiagnosticCode::ProcessorControl
        );
        assert_eq!(analysis.diagnostics[0].name.as_deref(), Some("toc"));
        assert_eq!(
            analysis
                .properties
                .iter()
                .map(|property| (property.name.as_str(), property.value.as_str()))
                .collect::<Vec<_>>(),
            [("lang", "zh-CN"), ("status", "in progress")]
        );
        let status = &analysis.properties[1];
        assert_eq!(
            &source[usize::try_from(status.value_range.start).unwrap()
                ..usize::try_from(status.value_range.end).unwrap()],
            "in progress"
        );
        assert!(
            !analysis
                .properties
                .iter()
                .any(|property| property.name == "toc")
        );
        assert!(
            !analysis
                .properties
                .iter()
                .any(|property| property.value == "body-only")
        );
    }

    #[test]
    fn diagnoses_one_bad_property_without_hiding_safe_neighbors() {
        let source = "= Title\n:good: 保留\n:Bad: no\n:good: duplicate\n:long: continued \\\nnext line\n:after: 安全\n\nBody\n";
        let analysis = analyze_document_header_properties(source);
        assert_eq!(analysis.properties.len(), 2);
        assert_eq!(analysis.properties[0].value, "保留");
        assert_eq!(analysis.properties[1].value, "安全");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DocumentPropertyDiagnosticCode::InvalidName)
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DocumentPropertyDiagnosticCode::DuplicateName)
        );
        assert!(analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DocumentPropertyDiagnosticCode::ContinuedValue));
    }

    #[test]
    fn patches_only_target_value_and_inserts_before_the_header_boundary() {
        let source = "---\r\nweftext:\r\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n---\r\n= Title\r\n:status: old\r\n\r\nBody\r\n:status: body\r\n";
        let patched = patch_document_header_property(source, "status", Some("新值"))
            .expect("replace property");
        assert_eq!(patched, source.replacen(":status: old", ":status: 新值", 1));
        let inserted = patch_document_header_property(&patched, "project", Some("Weftext"))
            .expect("insert property");
        assert!(inserted.contains(":status: 新值\r\n:project: Weftext\r\n\r\nBody"));
        let removed =
            patch_document_header_property(&inserted, "status", None).expect("remove property");
        assert!(!removed.contains(":status: 新值"));
        assert!(removed.contains(":status: body"));
    }

    #[test]
    fn rejects_processor_names_duplicate_targets_and_multiline_values() {
        assert_eq!(
            patch_document_header_property("= T\n\n", "toc", Some("left")),
            Err(DocumentPropertyPatchError::InvalidName)
        );
        assert_eq!(
            patch_document_header_property(
                "= T\n:status: one\n:status: two\n\n",
                "status",
                Some("three"),
            ),
            Err(DocumentPropertyPatchError::DuplicateName)
        );
        assert_eq!(
            patch_document_header_property("= T\n\n", "status", Some("two\nlines")),
            Err(DocumentPropertyPatchError::InvalidValue)
        );
    }
}
