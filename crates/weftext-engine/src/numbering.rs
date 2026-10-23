//! Bounded native heading numbering. Unknown author rules are preserved.
use crate::{EngineError, UiMessage};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use typst_syntax::{
    LinkedNode, Source,
    ast::{self, AstNode},
};

const BEGIN: &str = "// weftext:heading-numbering\n";
const END: &str = "// weftext:end-heading-numbering\n";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadingNumberingScheme {
    None,
    Decimal,
    Chinese,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeadingNumberingIntent {
    pub scheme: HeadingNumberingScheme,
    pub start: u32,
    pub max_level: u32,
    #[serde(default)]
    pub exclude_labels: Vec<String>,
}
impl Default for HeadingNumberingIntent {
    fn default() -> Self {
        Self {
            scheme: HeadingNumberingScheme::Decimal,
            start: 1,
            max_level: 6,
            exclude_labels: vec![],
        }
    }
}
impl HeadingNumberingIntent {
    pub fn requires_frozen_module(&self, source: &str) -> bool {
        crate::notes::endnotes_import(source)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadingNumberingInspection {
    pub source_hash: String,
    pub settings: Option<HeadingNumberingIntent>,
    pub editable: bool,
    /// True only when an exact frozen endnotes certificate is the sole blocker.
    /// Independently produced admission evidence, never inferred from UI text.
    pub frozen_endnotes_required: bool,
    pub reason: Option<weftext_messages::ErrorDescription>,
}
fn generate(settings: &HeadingNumberingIntent) -> Result<String, EngineError> {
    if settings.start == 0
        || settings.start > 1_000_000
        || !(1..=6).contains(&settings.max_level)
        || settings.exclude_labels.len() > 200
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_numbering_numbering_bounds(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for label in &settings.exclude_labels {
        let parsed = Source::detached(format!("<{label}>"));
        if label.is_empty()
            || label.len() > 256
            || !parsed.root().errors_and_warnings().0.is_empty()
            || !parsed
                .root()
                .children()
                .any(|n| n.cast::<ast::Label>().is_some_and(|l| l.get() == label))
            || !seen.insert(label)
        {
            return Err(EngineError::new(
                weftext_messages::UiMessage::error_engine_numbering_numbering_exclusion_invalid(),
            ));
        }
    }
    let numbering = match settings.scheme {
        HeadingNumberingScheme::None => "none".into(),
        HeadingNumberingScheme::Decimal => format!(
            "(..n) => if n.pos().len() <= {} {{ numbering(\"1.1\", ..n) }}",
            settings.max_level
        ),
        HeadingNumberingScheme::Chinese => format!(
            "(..n) => if n.pos().len() <= {} {{ numbering(\"一.一\", ..n) }}",
            settings.max_level
        ),
    };
    let mut header = format!(
        "{BEGIN}#set heading(numbering: {numbering})\n#counter(heading).update({})\n",
        settings.start - 1
    );
    for label in &settings.exclude_labels {
        header.push_str(&format!("#show <{label}>: set heading(numbering: none)\n"));
    }
    header.push_str(END);
    Ok(header)
}
fn parse_header(source: &str) -> Option<(HeadingNumberingIntent, Range<usize>)> {
    let rest = source.strip_prefix(BEGIN)?;
    let end = BEGIN.len() + rest.find(END)? + END.len();
    let mut lines = source[BEGIN.len()..end - END.len()].lines();
    let numbering = lines
        .next()?
        .strip_prefix("#set heading(numbering: ")?
        .strip_suffix(')')?;
    let (scheme, max_level) = if numbering == "none" {
        (HeadingNumberingScheme::None, 6)
    } else {
        let body = numbering.strip_prefix("(..n) => if n.pos().len() <= ")?;
        let (level, pattern) = body.split_once(" { numbering(\"")?;
        let pattern = pattern.strip_suffix("\", ..n) }")?;
        (
            match pattern {
                "1.1" => HeadingNumberingScheme::Decimal,
                "一.一" => HeadingNumberingScheme::Chinese,
                _ => return None,
            },
            level.parse().ok()?,
        )
    };
    let start = lines
        .next()?
        .strip_prefix("#counter(heading).update(")?
        .strip_suffix(')')?
        .parse::<u32>()
        .ok()?
        .checked_add(1)?;
    let exclude_labels = lines
        .map(|line| {
            line.strip_prefix("#show <")
                .and_then(|s| s.strip_suffix(">: set heading(numbering: none)"))
                .map(str::to_owned)
        })
        .collect::<Option<Vec<_>>>()?;
    let settings = HeadingNumberingIntent {
        scheme,
        start,
        max_level,
        exclude_labels,
    };
    if generate(&settings).ok()? != source[..end] {
        return None;
    }
    Some((settings, 0..end))
}
/// Editor uses this only to allow exact native show-set rules generated here.
pub(crate) fn verified_numbering_header_range(source: &str) -> Option<Range<usize>> {
    let (_, range) = parse_header(source)?;
    let parsed = Source::detached(source);
    if parsed.root().errors_and_warnings().0.is_empty()
        && ["heading", "numbering", "counter"]
            .iter()
            .all(|name| crate::notes::notes_binding_available(source, name))
    {
        Some(range)
    } else {
        None
    }
}
fn author_rules_unknown(source: &str, endnotes: bool) -> bool {
    fn walk(
        n: &LinkedNode<'_>,
        source: &str,
        bindings: &mut std::collections::BTreeMap<String, bool>,
        endnotes: bool,
    ) -> bool {
        // Import declarations do not consume the values they bind. All
        // executable uses below still require their original binding proof.
        if n.kind() == typst_syntax::SyntaxKind::ModuleImport {
            return false;
        }
        // Imported content can update a counter without a function call, for
        // example `#reset` where reset is contextual content from a module.
        if let Some(ident) = n.get().cast::<ast::Ident>()
            && !(endnotes && matches!(ident.get().as_str(), "endnote" | "endnote-ref" | "endnotes"))
            && !*bindings
                .entry(ident.get().to_string())
                .or_insert_with(|| crate::notes::notes_binding_available(source, ident.get()))
        {
            return true;
        }
        if n.get().cast::<ast::LetBinding>().is_some()
            || matches!(
                n.kind(),
                typst_syntax::SyntaxKind::ModuleInclude
                    | typst_syntax::SyntaxKind::ForLoop
                    | typst_syntax::SyntaxKind::WhileLoop
            )
        {
            return true;
        }
        // Only this exact native math member has a qualified-target proof.
        // Arbitrary qualified or computed targets can replace headings later.
        if let Some(set) = n.get().cast::<ast::SetRule>()
            && set.target().to_untyped().cast::<ast::Ident>().is_none()
            && !set
                .target()
                .to_untyped()
                .cast::<ast::FieldAccess>()
                .is_some_and(|field| {
                    field.field().get() == "equation"
                        && field
                            .target()
                            .to_untyped()
                            .cast::<ast::Ident>()
                            .is_some_and(|root| {
                                root.get() == "math"
                                    && *bindings.entry("math".into()).or_insert_with(|| {
                                        crate::notes::notes_binding_available(source, "math")
                                    })
                            })
                })
        {
            return true;
        }
        if let Some(set) = n.get().cast::<ast::SetRule>()
            && set
                .target()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "heading")
            && set.args().items().any(|a| {
                matches!(a, ast::Arg::Named(v) if v.name().get() == "numbering")
                    || matches!(a, ast::Arg::Spread(_))
            })
        {
            return true;
        }
        // A set rule evaluates its target too: an import can bind heading to
        // another native-looking name and override the managed scheme later.
        if let Some(target) = n
            .get()
            .cast::<ast::SetRule>()
            .and_then(|set| set.target().to_untyped().cast::<ast::Ident>())
            && !*bindings
                .entry(target.get().to_string())
                .or_insert_with(|| crate::notes::notes_binding_available(source, target.get()))
        {
            return true;
        }
        // A show function may alter heading counters or numbering indirectly.
        if n.get().cast::<ast::ShowRule>().is_some() {
            return true;
        }
        if let Some(call) = n.get().cast::<ast::FuncCall>() {
            let direct = call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .map(|i| i.get().to_string());
            let native_root = direct.clone().or_else(|| {
                call.callee()
                    .to_untyped()
                    .cast::<ast::FieldAccess>()
                    .and_then(|field| field.target().to_untyped().cast::<ast::Ident>())
                    .map(|i| i.get().to_string())
            });
            let allowed = native_root.as_deref().is_some_and(|name| {
                if matches!(name, "endnote" | "endnote-ref" | "endnotes") {
                    return endnotes;
                }
                if !matches!(
                    name,
                    "text"
                        | "document"
                        | "title"
                        | "heading"
                        | "strong"
                        | "emph"
                        | "strike"
                        | "table"
                        | "grid"
                        | "figure"
                        | "image"
                        | "raw"
                        | "link"
                        | "ref"
                        | "cite"
                        | "bibliography"
                        | "footnote"
                        | "metadata"
                        | "datetime"
                        | "label"
                        | "box"
                        | "block"
                        | "pagebreak"
                        | "linebreak"
                        | "parbreak"
                        | "place"
                        | "align"
                        | "pad"
                        | "h"
                        | "v"
                        | "math"
                        | "outline"
                ) {
                    return false;
                }
                *bindings
                    .entry(name.to_owned())
                    .or_insert_with(|| crate::notes::notes_binding_available(source, name))
            });
            if !allowed {
                return true;
            }
            if call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "counter")
            {
                return true;
            }
            if call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "heading")
                && call.args().items().any(|a| {
                    matches!(a, ast::Arg::Named(v) if v.name().get() == "numbering")
                        || matches!(a, ast::Arg::Spread(_))
                })
            {
                return true;
            }
        }
        n.children().any(|c| walk(&c, source, bindings, endnotes))
    }
    let parsed = Source::detached(source);
    !parsed.root().errors_and_warnings().0.is_empty()
        || walk(
            &LinkedNode::new(parsed.root()),
            source,
            &mut Default::default(),
            endnotes,
        )
}
pub fn inspect_heading_numbering(source: &str) -> HeadingNumberingInspection {
    inspect_heading_numbering_qualified(source, None)
}
pub(crate) fn inspect_heading_numbering_qualified(
    source: &str,
    pinned: Option<&crate::notes::PinnedEndnotes<'_>>,
) -> HeadingNumberingInspection {
    let parsed = parse_header(source);
    let tail = parsed
        .as_ref()
        .map(|(_, range)| &source[range.end..])
        .unwrap_or(source);
    let endnotes = crate::notes::endnotes_import(source);
    let (reason, frozen_endnotes_required) = if source.starts_with("// weftext:heading-numbering")
        && parsed.is_none()
    {
        (
            Some(weftext_messages::UiMessage::error_engine_reason_numbering_header_unverified()),
            false,
        )
    } else if !["heading", "numbering", "counter"]
        .iter()
        .all(|name| crate::notes::notes_binding_available(source, name))
    {
        (
            Some(weftext_messages::UiMessage::error_engine_reason_standard_binding_unverified()),
            false,
        )
    } else if author_rules_unknown(tail, endnotes) {
        (
            Some(weftext_messages::UiMessage::error_engine_reason_numbering_rule_custom()),
            false,
        )
    } else if endnotes && !pinned.is_some_and(|proof| proof.certifies(source)) {
        (
            Some(
                weftext_messages::UiMessage::error_engine_reason_endnotes_frozen_module_required(),
            ),
            true,
        )
    } else {
        (None, false)
    };
    HeadingNumberingInspection {
        source_hash: crate::source_hash(source),
        settings: parsed.map(|(settings, _)| settings),
        editable: reason.is_none(),
        frozen_endnotes_required,
        reason: reason.map(Into::into),
    }
}
pub fn edit_heading_numbering(
    source: &str,
    action: &HeadingNumberingIntent,
) -> Result<String, EngineError> {
    edit_heading_numbering_qualified(source, action, None)
}
pub(crate) fn edit_heading_numbering_qualified(
    source: &str,
    action: &HeadingNumberingIntent,
    pinned: Option<&crate::notes::PinnedEndnotes<'_>>,
) -> Result<String, EngineError> {
    let inspection = inspect_heading_numbering_qualified(source, pinned);
    if !inspection.editable {
        return Err(EngineError::new(inspection.reason.unwrap_or_else(|| {
            UiMessage::error_engine_reason_numbering_rule_custom().into()
        })));
    }
    let header = generate(action)?;
    let tail = parse_header(source)
        .map(|(_, range)| &source[range.end..])
        .unwrap_or(source);
    let parsed = Source::detached(tail);
    let mut labels = std::collections::BTreeMap::<String, usize>::new();
    fn collect(n: &LinkedNode<'_>, labels: &mut std::collections::BTreeMap<String, usize>) {
        if let Some(label) = n.get().cast::<ast::Label>()
            && n.parent()
                .is_some_and(|p| p.kind() == typst_syntax::SyntaxKind::Markup)
        {
            let previous = n.prev_sibling().and_then(|mut p| {
                while p.kind().is_trivia() || p.kind() == typst_syntax::SyntaxKind::Hash {
                    p = p.prev_sibling()?;
                }
                Some(p)
            });
            let parent_heading = n
                .parent()
                .and_then(|p| p.parent())
                .is_some_and(|p| p.kind() == typst_syntax::SyntaxKind::Heading);
            let heading = parent_heading
                || previous.is_some_and(|p| p.kind() == typst_syntax::SyntaxKind::Heading);
            let count = labels.entry(label.get().into()).or_default();
            if heading {
                *count += 1;
            } else {
                *count += 1000;
            }
        }
        for c in n.children() {
            collect(&c, labels);
        }
    }
    collect(&LinkedNode::new(parsed.root()), &mut labels);
    if action
        .exclude_labels
        .iter()
        .any(|label| labels.get(label) != Some(&1))
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_numbering_numbering_exclusion_unmatched(),
        ));
    }
    let result = format!("{header}{tail}");
    if !Source::detached(&result)
        .root()
        .errors_and_warnings()
        .0
        .is_empty()
    {
        return Err(EngineError::new(
            weftext_messages::UiMessage::error_engine_numbering_numbering_readback_failed(),
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_document_title_does_not_block_heading_numbering() {
        let source = "#metadata((id: \"note\", types: ())) <weftext>\n#set document(title: [文档标题])\n#title()\n#block[副标题] <weftext-subtitle>\n\n= 一、已有前缀 <one>\n\n正文#footnote[说明]\n";
        assert!(inspect_heading_numbering(source).editable);
        let edited = edit_heading_numbering(source, &Default::default()).unwrap();
        assert!(edited.ends_with(source));
        assert!(inspect_heading_numbering(&edited).editable);
        for name in ["title", "document"] {
            let shadowed = format!("#import \"custom.typ\": {name}\n{source}");
            assert!(!inspect_heading_numbering(&shadowed).editable, "{name}");
        }
    }
    #[test]
    fn source_stays_exact_and_unknown_rules_are_not_overwritten() {
        let source = "= 一、已有前缀 <first>\n\n正文\n\n== 子标题\n";
        let action = HeadingNumberingIntent {
            start: 3,
            exclude_labels: vec!["first".into()],
            ..Default::default()
        };
        let out = edit_heading_numbering(source, &action).unwrap();
        assert!(out.ends_with(source));
        assert_eq!(inspect_heading_numbering(&out).settings, Some(action));
        assert!(verified_numbering_header_range(&out).is_some());
        assert!(
            edit_heading_numbering("#set heading(numbering: custom)\n= A", &Default::default())
                .is_err()
        );
        assert!(
            edit_heading_numbering("#show heading: it => it\n= A", &Default::default()).is_err()
        );
        assert!(
            edit_heading_numbering(
                &out.replace("update(2)", "update(2 + 1)"),
                &Default::default()
            )
            .is_err()
        );
    }
    #[test]
    fn invalid_and_missing_exclusions_fail_without_creating_labels() {
        let action = HeadingNumberingIntent {
            exclude_labels: vec!["missing".into()],
            ..Default::default()
        };
        assert!(edit_heading_numbering("= A", &action).is_err());
        assert!(
            edit_heading_numbering(
                "= A",
                &HeadingNumberingIntent {
                    start: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn qualified_native_heading_set_is_preserved_without_a_false_applied_scheme() {
        let source = "#set std.heading(numbering: \"A\")\n= Heading";
        let dir = tempfile::TempDir::new().unwrap();
        let output = crate::Engine::new_with_system_fonts(false)
            .unwrap()
            .render_frozen(&crate::FrozenRenderRequest {
                source: source.into(),
                path: dir.path().join("qualified.typ"),
                root: dir.path().into(),
                style: None,
                files: Default::default(),
            })
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        assert!(!inspect_heading_numbering(source).editable);
        assert!(edit_heading_numbering(source, &Default::default()).is_err());
        assert!(
            edit_heading_numbering(
                source,
                &HeadingNumberingIntent {
                    scheme: HeadingNumberingScheme::None,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn imported_heading_aliased_to_another_set_target_is_preserved() {
        let source = "#import \"helper.typ\": h as text\n#set text(numbering: \"A\")\n= Heading";
        let dir = tempfile::TempDir::new().unwrap();
        let output = crate::Engine::new_with_system_fonts(false)
            .unwrap()
            .render_frozen(&crate::FrozenRenderRequest {
                source: source.into(),
                path: dir.path().join("aliased.typ"),
                root: dir.path().into(),
                style: None,
                files: std::collections::BTreeMap::from([(
                    "helper.typ".into(),
                    b"#let h = heading".to_vec(),
                )]),
            })
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        assert!(!inspect_heading_numbering(source).editable);
        assert!(edit_heading_numbering(source, &Default::default()).is_err());
        assert!(
            inspect_heading_numbering(
                "#set text(lang: \"en\")\n#set table(stroke: black)\n= Heading"
            )
            .editable
        );
    }
    #[test]
    fn exact_native_math_equation_setter_keeps_independent_heading_numbering() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        let source = "#set math.equation(numbering: \"(1)\")\n= Heading\n$ x / x + y_1^2 = 1 $";
        let input = |source: &str| crate::FrozenRenderRequest {
            source: source.into(),
            path: dir.path().join("math.typ"),
            root: dir.path().into(),
            style: None,
            files: Default::default(),
        };
        assert!(engine.render_frozen(&input(source)).unwrap().ok);
        assert!(inspect_heading_numbering(source).editable);
        let numbered = edit_heading_numbering(source, &Default::default()).unwrap();
        assert!(engine.render_frozen(&input(&numbered)).unwrap().ok);
        for source in [
            "#let math = (equation: heading)\n#set math.equation(numbering: \"A\")\n= Heading",
            "#import \"helper.typ\": h as math\n#set math.equation(numbering: \"A\")\n= Heading",
            "#set std.heading(numbering: \"A\")\n= Heading",
            "#set (heading)(numbering: \"A\")\n= Heading",
            "#set math.equation(numbering: context counter(heading).update(50))\n= Heading",
            "#set math.equation(numbering: eval(\"counter(heading).update(50)\"))\n= Heading",
            "#set math.equation(numbering: state(\"side-effect\", 1))\n= Heading",
        ] {
            if source.starts_with("#let math") || source.starts_with("#import") {
                let mut alias = input(source);
                alias.files.insert(
                    "helper.typ".into(),
                    b"#let h = (equation: heading)".to_vec(),
                );
                let compiled = engine.render_frozen(&alias).unwrap();
                assert!(compiled.ok, "{source}: {:?}", compiled.diagnostics);
            }
            assert!(!inspect_heading_numbering(source).editable, "{source}");
            assert!(
                edit_heading_numbering(source, &Default::default()).is_err(),
                "{source}"
            );
        }
    }
    #[test]
    fn imported_contextual_content_is_not_mistaken_for_inert_markup() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        for source in [
            "#import \"helper.typ\": reset\n#reset\n= Heading",
            "#import \"helper.typ\": reset\n#(reset)\n= Heading",
            "#import \"helper.typ\" as helper\n#helper.reset\n= Heading",
        ] {
            let output = engine
                .render_frozen(&crate::FrozenRenderRequest {
                    source: source.into(),
                    path: dir.path().join("contextual.typ"),
                    root: dir.path().into(),
                    style: None,
                    files: std::collections::BTreeMap::from([(
                        "helper.typ".into(),
                        b"#let reset = context counter(heading).update(50)".to_vec(),
                    )]),
                })
                .unwrap();
            assert!(output.ok, "{source}: {:?}", output.diagnostics);
            assert!(!inspect_heading_numbering(source).editable, "{source}");
            assert!(edit_heading_numbering(source, &Default::default()).is_err());
        }
    }
    #[test]
    fn static_nonshadowing_import_is_safe_but_unknown_calls_and_wildcards_are_not() {
        assert!(inspect_heading_numbering("#import \"helper.typ\": unrelated\n= A").editable);
        assert!(!inspect_heading_numbering("#import \"helper.typ\": *\n= A").editable);
        assert!(!inspect_heading_numbering("#import \"helper.typ\": heading\n= A").editable);
        assert!(
            !inspect_heading_numbering("#import \"helper.typ\": unrelated\n#unrelated()[= A]")
                .editable
        );
        let notes =
            "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n= A\n#endnotes()";
        assert_eq!(
            inspect_heading_numbering(notes)
                .reason
                .map(|reason| reason.explanation),
            Some(
                weftext_messages::UiMessage::error_engine_reason_endnotes_frozen_module_required()
            )
        );
        assert!(inspect_heading_numbering(notes).frozen_endnotes_required);
        let blocked = format!("#set heading(numbering: custom)\n{notes}");
        assert!(!inspect_heading_numbering(&blocked).frozen_endnotes_required);
        assert!(!inspect_heading_numbering("= Plain heading").frozen_endnotes_required);
        assert!(edit_heading_numbering(notes, &Default::default()).is_err());
    }
    #[test]
    fn same_named_endnotes_module_cannot_certify_heading_counter_behavior() {
        let dir = tempfile::TempDir::new().unwrap();
        let source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n= Heading\n#endnotes()";
        let input = crate::FrozenRenderRequest {
            source: source.into(), path: dir.path().join("note.typ"), root: dir.path().into(), style: None,
            files: std::collections::BTreeMap::from([("weftext-endnotes.typ".into(),
                b"#let endnote(..args) = []\n#let endnote-ref(..args) = []\n#let endnotes() = context counter(heading).update(50)".to_vec())]),
        };
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        let current = engine.render_frozen(&input).unwrap();
        assert!(current.ok, "{:?}", current.diagnostics);
        let request = crate::EditRequest {
            source_hash: crate::source_hash(source),
            intent: crate::EditIntent::HeadingNumbering {
                action: Default::default(),
            },
        };
        assert!(crate::edit(source, &request).is_err());
        assert!(engine.edit_notes_frozen(&input, &request).is_err());
        assert_eq!(input.source, source);
    }
    #[test]
    fn pinned_endnotes_numbering_uses_current_and_proposed_native_compilation() {
        let dir = tempfile::TempDir::new().unwrap();
        let source = "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n= Heading <chapter>\n尾#box[#endnote(id:\"tail\")[说明]]\n#endnotes()";
        let mut input = crate::FrozenRenderRequest {
            source: source.into(),
            path: dir.path().join("note.typ"),
            root: dir.path().into(),
            style: None,
            files: std::collections::BTreeMap::from([
                (
                    "weftext-endnotes.typ".into(),
                    include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec(),
                ),
                (
                    "unused.txt".into(),
                    b"not read by the native compiler".to_vec(),
                ),
                ("note.typ".into(), source.as_bytes().to_vec()),
            ]),
        };
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        let action = HeadingNumberingIntent {
            start: 3,
            ..Default::default()
        };
        let request = crate::EditRequest {
            source_hash: crate::source_hash(source),
            intent: crate::EditIntent::HeadingNumbering {
                action: action.clone(),
            },
        };
        let result = engine
            .edit_notes_frozen_with_dependencies(&input, &request)
            .unwrap();
        assert!(result.edit.source.ends_with(source));
        assert_eq!(result.edit.patches.len(), 1);
        assert!(result.edit.map.heading_numbering.editable);
        assert_eq!(result.edit.map.heading_numbering.settings, Some(action));
        assert!(!inspect_heading_numbering(&result.edit.source).editable);
        assert!(
            result
                .dependencies
                .iter()
                .any(|dependency| dependency.path == "weftext-endnotes.typ"
                    && dependency.status == "loaded")
        );
        assert!(
            !result
                .dependencies
                .iter()
                .any(|dependency| dependency.path == "unused.txt")
        );
        input.files.remove("weftext-endnotes.typ");
        assert!(engine.edit_notes_frozen(&input, &request).is_err());
        let ordinary = "= Ordinary heading";
        input.source = ordinary.into();
        input.files.clear();
        let ordinary_request = crate::EditRequest {
            source_hash: crate::source_hash(ordinary),
            intent: crate::EditIntent::HeadingNumbering {
                action: Default::default(),
            },
        };
        assert!(
            crate::edit(ordinary, &ordinary_request)
                .unwrap()
                .source
                .ends_with(ordinary)
        );
        assert!(
            engine
                .edit_notes_frozen_with_dependencies(&input, &ordinary_request)
                .unwrap()
                .dependencies
                .is_empty()
        );
    }
}
