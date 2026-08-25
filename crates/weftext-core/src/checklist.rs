use std::ops::Range;

use serde::Serialize;

pub use weftext_asciidoc::{
    ChecklistEvidence, ChecklistMarker, ChecklistParserOccurrence, ChecklistState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistDiagnosticCode {
    ParserFailure,
    IncompleteParserBranch,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistDiagnostic {
    pub code: ChecklistDiagnosticCode,
    pub range: Range<u64>,
    pub parser_ordinal_path: Option<Vec<u32>>,
    pub message: String,
}

/// Parser-owned native checklist occurrences and any evidence that blocks exact promotion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistAnalysis {
    pub semantic_model_version: u16,
    pub occurrences: Vec<ChecklistEvidence>,
    pub diagnostics: Vec<ChecklistDiagnostic>,
}

/// Projects native checklist authority without recognizing list syntax in Core.
#[must_use]
pub fn analyze_checklist_source(source: &str) -> ChecklistAnalysis {
    let analysis = weftext_asciidoc::analyze(source);
    analyze_checklist_analysis(&analysis)
}

/// Projects checklist evidence from an `AsciiDoc` analysis already produced by a workspace scan.
/// Keeping this helper crate-private prevents a second parser invocation while preserving the
/// public source-analysis boundary above.
pub(crate) fn analyze_checklist_analysis(
    analysis: &weftext_asciidoc::Analysis,
) -> ChecklistAnalysis {
    let mut diagnostics = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == weftext_asciidoc::DiagnosticCode::ParserError)
        .map(|diagnostic| ChecklistDiagnostic {
            code: ChecklistDiagnosticCode::ParserFailure,
            range: diagnostic.range.clone(),
            parser_ordinal_path: None,
            message: diagnostic.message.clone(),
        })
        .collect::<Vec<_>>();
    diagnostics.extend(
        analysis
            .checklists
            .iter()
            .filter(|evidence| !evidence.parser_occurrence.branch_complete)
            .map(|evidence| ChecklistDiagnostic {
                code: ChecklistDiagnosticCode::IncompleteParserBranch,
                range: evidence.item_range.clone(),
                parser_ordinal_path: Some(
                    evidence.parser_occurrence.parser_ordinal_path.clone(),
                ),
                message: "the AsciiDoc parser could not prove the complete attached checklist branch; promotion is unavailable"
                    .to_owned(),
            }),
    );
    ChecklistAnalysis {
        semantic_model_version: analysis.semantic_model_version,
        occurrences: analysis.checklists.clone(),
        diagnostics,
    }
}
