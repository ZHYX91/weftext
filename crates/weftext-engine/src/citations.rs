//! Conservative native citation declarations and authored cite slots. Ref syntax
//! (`@key`) deliberately remains unclassified until an official compile resolves it.
use crate::{EngineError, SourceRange, certify_metadata_declaration, standard_binding_available};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use typst_syntax::{
    LinkedNode, Source, SyntaxKind, SyntaxNode,
    ast::{self, AstNode},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CitationEntry {
    pub key: String,
    pub work: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationDeclaration {
    pub entries: Vec<CitationEntry>,
    pub range: SourceRange,
    pub work_slots: Vec<CitationWorkSlot>,
}

/// Exact static native string slots are scoped to the inspected source hash.
/// Copy producers may rewrite these proved values using their fresh Node map;
/// citation keys and lookalike strings elsewhere are deliberately absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationWorkSlot {
    pub key: String,
    pub work: String,
    pub range: SourceRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCitationOccurrence {
    pub key: String,
    pub range: SourceRange,
}

pub fn valid_citation_key(key: &str) -> bool {
    key.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

/// None means the owner contains no citation-role label; a present but
/// unqualified role returns an explicit error, never a partial declaration.
pub fn inspect_citation_declaration(
    source: &str,
) -> Result<Option<CitationDeclaration>, EngineError> {
    if source.len() > 8 * 1024 * 1024 {
        return Err(EngineError(
            "citation-budget: owner exceeds citation parser budget".into(),
        ));
    }
    let parsed = Source::detached(source);
    fn contains(node: &SyntaxNode, parent: Option<SyntaxKind>) -> bool {
        (parent == Some(SyntaxKind::Markup)
            && node
                .cast::<ast::Label>()
                .is_some_and(|label| label.get() == "weftext-citations"))
            || node
                .children()
                .any(|child| contains(child, Some(node.kind())))
    }
    if !contains(parsed.root(), None) {
        return Ok(None);
    }
    let unavailable = || {
        EngineError(
            "citation-declaration: unique direct static standard metadata declaration required"
                .into(),
        )
    };
    let range =
        certify_metadata_declaration(source, "weftext-citations").ok_or_else(unavailable)?;
    // The role certificate proves the direct call's prefix. Later literal
    // imports with explicit, unrelated bound names cannot replace `metadata`.
    // This is a lexical binding proof, not trust in imported module contents.
    if !crate::notes::notes_binding_available(source, "metadata")
        || !loop_binding_available(source, "metadata")
    {
        return Err(unavailable());
    }
    let root = LinkedNode::new(parsed.root());
    let call = root
        .children()
        .find(|n| n.range().start == range.from)
        .and_then(|n| n.get().cast::<ast::FuncCall>())
        .ok_or_else(unavailable)?;
    let args: Vec<_> = call.args().items().collect();
    let [ast::Arg::Pos(expr)] = args.as_slice() else {
        return Err(unavailable());
    };
    let dict = expr
        .to_untyped()
        .cast::<ast::Dict>()
        .ok_or_else(unavailable)?;
    let fields = dict_fields(dict).ok_or_else(unavailable)?;
    if fields.len() != 2
        || fields
            .iter()
            .find(|(k, _)| k == "kind")
            .and_then(|(_, e)| e.to_untyped().cast::<ast::Str>())
            .is_none_or(|s| s.get() != "weftext.citations")
    {
        return Err(unavailable());
    }
    let array = fields
        .iter()
        .find(|(k, _)| k == "entries")
        .and_then(|(_, e)| e.to_untyped().cast::<ast::Array>())
        .ok_or_else(unavailable)?;
    let mut entries = Vec::new();
    let mut work_slots = Vec::new();
    let mut keys = BTreeSet::new();
    for item in array.items() {
        if entries.len() >= 4096 {
            return Err(EngineError(
                "citation-budget: too many declared entries".into(),
            ));
        }
        let ast::ArrayItem::Pos(expr) = item else {
            return Err(unavailable());
        };
        let fields = dict_fields(
            expr.to_untyped()
                .cast::<ast::Dict>()
                .ok_or_else(unavailable)?,
        )
        .ok_or_else(unavailable)?;
        if fields.len() != 2 {
            return Err(unavailable());
        }
        let literal = |name| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .and_then(|(_, e)| e.to_untyped().cast::<ast::Str>())
                .map(|s| s.get().to_string())
        };
        let key = literal("key").ok_or_else(unavailable)?;
        let work = literal("work").ok_or_else(unavailable)?;
        if !valid_citation_key(&key) || !keys.insert(key.clone()) {
            return Err(EngineError(
                "citation-key: invalid or duplicate exact citation key".into(),
            ));
        }
        if !valid_node_ref(&work) {
            return Err(EngineError(
                "citation-work: complete stable Work NodeRef required".into(),
            ));
        }
        let work_expr = fields
            .iter()
            .find(|(name, _)| name == "work")
            .ok_or_else(unavailable)?
            .1;
        let work_range = parsed
            .find(work_expr.to_untyped().span())
            .ok_or_else(unavailable)?
            .range();
        work_slots.push(CitationWorkSlot {
            key: key.clone(),
            work: work.clone(),
            range: crate::editor::source_range(source, work_range),
        });
        entries.push(CitationEntry { key, work });
    }
    Ok(Some(CitationDeclaration {
        entries,
        range,
        work_slots,
    }))
}

pub fn citation_work_slots(source: &str) -> Result<Vec<CitationWorkSlot>, EngineError> {
    Ok(inspect_citation_declaration(source)?
        .map(|role| role.work_slots)
        .unwrap_or_default())
}

fn dict_fields(dict: ast::Dict<'_>) -> Option<Vec<(String, ast::Expr<'_>)>> {
    let mut keys = BTreeSet::new();
    dict.items()
        .map(|item| {
            let (key, expr) = match item {
                ast::DictItem::Named(n) => (n.name().get().to_string(), n.expr()),
                ast::DictItem::Keyed(n) => (
                    n.key().to_untyped().cast::<ast::Str>()?.get().to_string(),
                    n.expr(),
                ),
                _ => return None,
            };
            keys.insert(key.clone()).then_some((key, expr))
        })
        .collect()
}

fn valid_node_ref(value: &str) -> bool {
    fn uuid(value: &str) -> bool {
        let bytes = value.as_bytes();
        bytes.len() == 36
            && bytes[14] == b'4'
            && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
            && bytes.iter().enumerate().all(|(i, b)| {
                if matches!(i, 8 | 13 | 18 | 23) {
                    *b == b'-'
                } else {
                    b.is_ascii_digit() || matches!(b, b'a'..=b'f')
                }
            })
    }
    value
        .strip_prefix("weftext-node://")
        .and_then(|v| v.split_once('/'))
        .is_some_and(|(library, node)| uuid(library) && uuid(node))
}

/// Only standard `cite` with a literal first Label yields an authored slot.
/// This is source evidence, not target-specific rendered occurrence evidence.
pub fn proven_static_cites(source: &str) -> Vec<NativeCitationOccurrence> {
    if source.len() > 8 * 1024 * 1024 || !citation_binding_available(source, "cite") {
        return vec![];
    }
    let parsed = Source::detached(source);
    fn walk(node: &LinkedNode<'_>, source: &str, out: &mut Vec<NativeCitationOccurrence>) {
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call
                .callee()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "cite")
        {
            let args: Vec<_> = call.args().items().collect();
            if let Some(ast::Arg::Pos(expr)) = args.first()
                && let Some(label) = expr.to_untyped().cast::<ast::Label>()
                && valid_citation_key(label.get())
                && args.iter().skip(1).all(|a| matches!(a, ast::Arg::Named(_)))
            {
                out.push(NativeCitationOccurrence {
                    key: label.get().to_string(),
                    range: crate::editor::source_range(source, node.range()),
                });
            }
        }
        for child in node.children() {
            walk(&child, source, out);
        }
    }
    let mut out = vec![];
    walk(&LinkedNode::new(parsed.root()), source, &mut out);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CitationForm {
    Normal,
    Prose,
    Author,
    Year,
    Full,
    None,
}

impl CitationForm {
    fn native(self) -> &'static str {
        match self {
            Self::Normal => "\"normal\"",
            Self::Prose => "\"prose\"",
            Self::Author => "\"author\"",
            Self::Year => "\"year\"",
            Self::Full => "\"full\"",
            Self::None => "none",
        }
    }
    fn read(expr: ast::Expr<'_>) -> Option<Self> {
        if expr.to_untyped().kind() == SyntaxKind::None {
            return Some(Self::None);
        }
        match expr.to_untyped().cast::<ast::Str>()?.get().as_str() {
            "normal" => Some(Self::Normal),
            "prose" => Some(Self::Prose),
            "author" => Some(Self::Author),
            "year" => Some(Self::Year),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCitationControl {
    pub key: String,
    /// Source binding proof is independent of whether parameter controls are safe.
    pub binding_proven: bool,
    pub range: SourceRange,
    /// None with form_explicit=false inherits; explicit complex expressions
    /// remain source expressions rather than being presented as inheritance.
    pub form: Option<CitationForm>,
    pub form_explicit: bool,
    pub supplement: Option<String>,
    pub supplement_explicit: bool,
    pub style: Option<String>,
    pub style_explicit: bool,
    pub editable: bool,
    pub reason: Option<String>,
}

/// Source labels are candidates, never proof that a target was executed or has
/// a referenceable number. The consumer must join a fresh official compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrossReferenceTarget {
    pub label: String,
    pub kind: String,
    pub range: SourceRange,
    pub label_range: SourceRange,
    pub unique: bool,
    pub source_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCrossReferenceControl {
    pub label: String,
    pub range: SourceRange,
    /// `unclassified` for @ syntax; only compilation can distinguish citation.
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCitationEditor {
    pub source_hash: String,
    pub bindings: Vec<CitationEntry>,
    pub citations: Vec<NativeCitationControl>,
    pub targets: Vec<NativeCrossReferenceTarget>,
    pub cross_references: Vec<NativeCrossReferenceControl>,
    pub rename_available: bool,
    pub rename_reason: Option<String>,
}

fn walk_nodes<'a>(node: &LinkedNode<'a>, visit: &mut impl FnMut(&LinkedNode<'a>)) {
    visit(node);
    for child in node.children() {
        walk_nodes(&child, visit);
    }
}

fn citation_binding_available(source: &str, name: &str) -> bool {
    if !standard_binding_available(source, name) {
        return false;
    }
    loop_binding_available(source, name)
}

fn loop_binding_available(source: &str, name: &str) -> bool {
    // The shared source certificate covers bindings and function parameters;
    // loop patterns are another official binding site that this adapter needs.
    let parsed = Source::detached(source);
    let mut available = true;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node
            .get()
            .cast::<ast::ForLoop>()
            .is_some_and(|loop_| loop_.pattern().bindings().iter().any(|i| i.get() == name))
        {
            available = false;
        }
    });
    available
}

fn call_name<'a>(call: ast::FuncCall<'a>) -> Option<&'a str> {
    call.callee()
        .to_untyped()
        .cast::<ast::Ident>()
        .map(|i| i.get().as_str())
}

fn plain_supplement(expr: ast::Expr<'_>) -> Option<String> {
    if let Some(value) = expr.to_untyped().cast::<ast::Str>() {
        return Some(value.get().to_string());
    }
    let block = expr.to_untyped().cast::<ast::ContentBlock>()?;
    let mut text = String::new();
    for node in block.body().to_untyped().children() {
        match node.kind() {
            SyntaxKind::Text | SyntaxKind::Space | SyntaxKind::Parbreak => {
                text.push_str(node.leaf_text().as_str())
            }
            SyntaxKind::Hash => {}
            SyntaxKind::Str => text.push_str(&node.cast::<ast::Str>()?.get()),
            SyntaxKind::Escape => text.push(node.cast::<ast::Escape>()?.get()),
            _ => return None,
        }
    }
    Some(text)
}

fn unique_authored_slot(source: &str, node: &LinkedNode<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if matches!(
            parent.kind(),
            SyntaxKind::ForLoop
                | SyntaxKind::WhileLoop
                | SyntaxKind::Conditional
                | SyntaxKind::Closure
                | SyntaxKind::Contextual
                | SyntaxKind::LetBinding
                | SyntaxKind::ShowRule
                | SyntaxKind::SetRule
        ) {
            return false;
        }
        if let Some(call) = parent.get().cast::<ast::FuncCall>() {
            if let Some(name) = call_name(call) {
                if !matches!(
                    name,
                    "strong"
                        | "emph"
                        | "strike"
                        | "text"
                        | "block"
                        | "box"
                        | "link"
                        | "figure"
                        | "table"
                        | "heading"
                        | "footnote"
                        | "equation"
                ) || !citation_binding_available(source, name)
                {
                    return false;
                }
            } else {
                let standard_table_cell = call
                    .callee()
                    .to_untyped()
                    .cast::<ast::FieldAccess>()
                    .is_some_and(|field| {
                        field
                            .target()
                            .to_untyped()
                            .cast::<ast::Ident>()
                            .is_some_and(|id| id.get() == "table")
                            && matches!(field.field().get().as_str(), "cell" | "header" | "footer")
                            && citation_binding_available(source, "table")
                    });
                if !standard_table_cell {
                    return false;
                }
            }
        }
        ancestor = parent.parent();
    }
    true
}

fn citation_control(source: &str, node: &LinkedNode<'_>, binding: bool) -> NativeCitationControl {
    let call = node.get().cast::<ast::FuncCall>().unwrap();
    let args: Vec<_> = call.args().items().collect();
    let key = args
        .first()
        .and_then(|a| match a {
            ast::Arg::Pos(expr) => expr.to_untyped().cast::<ast::Label>(),
            _ => None,
        })
        .map(|l| l.get().to_owned())
        .unwrap_or_default();
    let mut control = NativeCitationControl {
        key,
        binding_proven: binding,
        range: crate::editor::source_range(source, node.range()),
        form: None,
        form_explicit: false,
        supplement: None,
        supplement_explicit: false,
        style: None,
        style_explicit: false,
        editable: binding,
        reason: (!binding).then(|| "unverified-cite-binding".into()),
    };
    let mut names = BTreeSet::new();
    if !unique_authored_slot(source, node) {
        control.editable = false;
        control.reason = Some("generated-or-reused-citation-source".into());
    }
    if !valid_citation_key(&control.key) || args.is_empty() {
        control.editable = false;
    }
    for arg in args.iter().skip(1) {
        let ast::Arg::Named(named) = arg else {
            control.editable = false;
            continue;
        };
        if !names.insert(named.name().get().to_string()) {
            control.editable = false;
        }
        match named.name().get().as_str() {
            "form" => {
                control.form_explicit = true;
                control.form = CitationForm::read(named.expr());
                control.editable &= control.form.is_some();
            }
            "supplement" => {
                control.supplement_explicit = true;
                control.supplement = plain_supplement(named.expr());
                control.editable &= control.supplement.is_some();
            }
            "style" => {
                control.style_explicit = true;
                control.style = named
                    .expr()
                    .to_untyped()
                    .cast::<ast::Str>()
                    .map(|s| s.get().to_string());
                // A dynamic style is retained exactly when form/supplement change.
            }
            _ => {}
        }
    }
    if !control.editable && control.reason.is_none() {
        control.reason = Some("dynamic-or-nonliteral-citation-options".into());
    }
    control
}

/// A deliberately bounded *complete* source profile for key/label rename.
/// Unknown calls, bindings, generated content and external sources may hide
/// additional references; they keep rendering, but receive no rename promise.
fn complete_reference_source(source: &str) -> Result<(), EngineError> {
    let parsed = Source::detached(source);
    if source.len() > 8 * 1024 * 1024 || !parsed.root().errors_and_warnings().0.is_empty() {
        return Err(EngineError(
            "citation-rename: syntax or source budget unavailable".into(),
        ));
    }
    let allowed = [
        "metadata",
        "cite",
        "ref",
        "link",
        "bibliography",
        "heading",
        "figure",
        "table",
        "image",
        "equation",
        "footnote",
        "strong",
        "emph",
        "strike",
        "text",
        "block",
        "box",
        "title",
        "outline",
        "parbreak",
        "linebreak",
    ];
    let bindings: std::collections::BTreeMap<_, _> = allowed
        .iter()
        .map(|name| (*name, citation_binding_available(source, name)))
        .collect();
    let mut safe = true;
    let mut bibliography_count = 0usize;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if matches!(
            node.kind(),
            SyntaxKind::LetBinding
                | SyntaxKind::ModuleImport
                | SyntaxKind::ModuleInclude
                | SyntaxKind::Closure
                | SyntaxKind::Conditional
                | SyntaxKind::WhileLoop
                | SyntaxKind::ForLoop
                | SyntaxKind::Contextual
                | SyntaxKind::ShowRule
                | SyntaxKind::DestructAssignment
                | SyntaxKind::FieldAccess
        ) {
            safe = false;
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>() {
            let Some(name) = call_name(call) else {
                safe = false;
                return;
            };
            if !bindings.get(name).copied().unwrap_or(false) {
                safe = false;
            }
            let args: Vec<_> = call.args().items().collect();
            if args.iter().any(|a| matches!(a, ast::Arg::Spread(_))) {
                safe = false;
            }
            if matches!(name, "cite" | "ref" | "link")
                && !matches!(args.first(), Some(ast::Arg::Pos(e)) if e.to_untyped().cast::<ast::Label>().is_some())
            {
                // Ordinary string links cannot refer to a native label.
                if name != "link"
                    || !matches!(args.first(), Some(ast::Arg::Pos(e)) if e.to_untyped().cast::<ast::Str>().is_some())
                {
                    safe = false;
                }
            }
            if name == "bibliography"
                && !matches!(args.first(), Some(ast::Arg::Pos(e)) if e.to_untyped().cast::<ast::Str>().is_some_and(|s|s.get()=="weftext.citations.bib"))
            {
                safe = false;
            }
            if name == "bibliography" {
                bibliography_count += 1;
            }
        }
        if node.get().cast::<ast::Label>().is_some()
            && node.parent().is_none_or(|p| p.kind() != SyntaxKind::Markup)
        {
            let reference_slot=node.parent().and_then(|p|p.parent()).and_then(|p|p.get().cast::<ast::FuncCall>()).is_some_and(|call| {
                matches!(call_name(call),Some("cite"|"ref"|"link"))&&matches!(call.args().items().next(),Some(ast::Arg::Pos(e)) if e.to_untyped().span()==node.get().span())
            });
            if !reference_slot {
                safe = false;
            }
        }
        if let Some(ident) = node.get().cast::<ast::Ident>()
            && matches!(
                ident.get().as_str(),
                "cite" | "ref" | "link" | "label" | "bibliography" | "eval"
            )
        {
            let direct = node
                .parent()
                .and_then(|n| n.get().cast::<ast::FuncCall>())
                .is_some_and(|c| c.callee().to_untyped().span() == node.get().span());
            if !direct {
                safe = false;
            }
        }
    });
    let application_bibliography = bibliography_count == 0
        || inspect_citation_declaration(source)
            .ok()
            .flatten()
            .is_some();
    if safe && bibliography_count <= 1 && application_bibliography {
        Ok(())
    } else {
        Err(EngineError("citation-rename: dynamic, generated or external reference coverage is incomplete; source preserved".into()))
    }
}

/// This owner has no unknown calls, source imports/includes, dynamic bindings,
/// computed citation targets or external/ambiguous bibliography in the bounded
/// native profile. Consumers must separately certify the active style module,
/// the generated virtual resource and this same compilation's actual products.
pub fn native_citation_owner_complete(source: &str) -> bool {
    complete_reference_source(source).is_ok()
}

/// Qualify only the bounded virtual bibliography input needed by a portable
/// native package. This does not establish complete reference coverage, absence
/// of hidden bibliographies in imported modules, or any Work authorization.
/// Literal explicit imports may bind unrelated names; full-owner and rename
/// qualification deliberately keep their stricter independent requirements.
pub fn native_citation_bibliography_input(source: &str) -> bool {
    if inspect_citation_declaration(source)
        .ok()
        .flatten()
        .is_none()
        || !crate::notes::notes_binding_available(source, "bibliography")
        || !loop_binding_available(source, "bibliography")
    {
        return false;
    }
    let parsed = Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return false;
    }
    let mut count = 0usize;
    let mut safe = true;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if let Some(ident) = node.get().cast::<ast::Ident>()
            && ident.get() == "bibliography"
            && !node
                .parent()
                .and_then(|parent| parent.get().cast::<ast::FuncCall>())
                .is_some_and(|call| call.callee().to_untyped().span() == node.get().span())
        {
            safe = false;
        }
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call_name(call) == Some("bibliography")
        {
            count += 1;
            let args = call.args().items().collect::<Vec<_>>();
            if !matches!(args.first(), Some(ast::Arg::Pos(expr)) if expr.to_untyped().cast::<ast::Str>().is_some_and(|path| path.get() == "weftext.citations.bib"))
            {
                safe = false;
            }
            let mut names = BTreeSet::new();
            for arg in args.iter().skip(1) {
                let ast::Arg::Named(named) = arg else {
                    safe = false;
                    continue;
                };
                if !names.insert(named.name().get())
                    || !matches!(
                        named.expr().to_untyped().kind(),
                        SyntaxKind::Str | SyntaxKind::Bool | SyntaxKind::None
                    )
                {
                    safe = false;
                }
            }
        }
    });
    safe && count == 1
}

pub fn inspect_native_citation_editor(source: &str) -> NativeCitationEditor {
    let declaration = inspect_citation_declaration(source);
    let (bindings, declaration_reason) = match declaration {
        Ok(value) => (value.map(|d| d.entries).unwrap_or_default(), None),
        Err(error) => (vec![], Some(error.0)),
    };
    let mut editor = NativeCitationEditor {
        source_hash: crate::source_hash(source),
        bindings,
        citations: vec![],
        targets: vec![],
        cross_references: vec![],
        rename_available: false,
        rename_reason: None,
    };
    if source.len() > 8 * 1024 * 1024 {
        editor.rename_reason = Some("citation-budget".into());
        return editor;
    }
    let parsed = Source::detached(source);
    let cite_available = citation_binding_available(source, "cite");
    let ref_available = citation_binding_available(source, "ref");
    let link_available = citation_binding_available(source, "link");
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if let Some(call) = node.get().cast::<ast::FuncCall>() {
            match call_name(call) {
                Some("cite") => {
                    editor
                        .citations
                        .push(citation_control(source, node, cite_available))
                }
                Some(name @ ("ref" | "link"))
                    if if name == "ref" {
                        ref_available
                    } else {
                        link_available
                    } =>
                {
                    if let Some(ast::Arg::Pos(expr)) = call.args().items().next()
                        && let Some(label) = expr.to_untyped().cast::<ast::Label>()
                    {
                        editor.cross_references.push(NativeCrossReferenceControl {
                            label: label.get().into(),
                            range: crate::editor::source_range(source, node.range()),
                            kind: name.into(),
                        });
                    }
                }
                _ => {}
            }
        }
        if let Some(reference) = node.get().cast::<ast::Ref>() {
            editor.cross_references.push(NativeCrossReferenceControl {
                label: reference.target().into(),
                range: crate::editor::source_range(source, node.range()),
                kind: "unclassified".into(),
            });
        }
        if let Some(label) = node.get().cast::<ast::Label>() {
            // Label-valued call arguments are uses, not target declarations.
            if node
                .parent()
                .is_none_or(|p| !matches!(p.kind(), SyntaxKind::Markup))
            {
                return;
            }
            let previous = node.prev_sibling().and_then(|mut p| {
                while p.kind().is_trivia() || p.kind() == SyntaxKind::Hash {
                    p = p.prev_sibling()?;
                }
                Some(p)
            });
            let parent_heading = node
                .parent()
                .and_then(|p| p.parent())
                .filter(|p| p.kind() == SyntaxKind::Heading);
            let target = parent_heading.or(previous.as_ref());
            let (kind, range) = if let Some(target) = target {
                let kind = if target.kind() == SyntaxKind::Heading {
                    "heading"
                } else if target.kind() == SyntaxKind::Equation {
                    "equation"
                } else if let Some(call) = target.get().cast::<ast::FuncCall>() {
                    match call_name(call) {
                        Some("figure") => "figure",
                        Some("table") => "table",
                        Some("footnote") => "footnote",
                        _ => "unknown",
                    }
                } else {
                    "unknown"
                };
                (
                    kind.to_string(),
                    crate::editor::source_range(source, target.range()),
                )
            } else {
                (
                    "unknown".into(),
                    crate::editor::source_range(source, node.range()),
                )
            };
            editor.targets.push(NativeCrossReferenceTarget {
                label: label.get().into(),
                kind,
                range,
                label_range: crate::editor::source_range(source, node.range()),
                unique: true,
                source_only: true,
            });
        }
    });
    let mut counts = std::collections::BTreeMap::new();
    for target in &editor.targets {
        *counts.entry(target.label.clone()).or_insert(0usize) += 1;
    }
    for target in &mut editor.targets {
        target.unique = counts[&target.label] == 1;
    }
    editor.rename_reason =
        declaration_reason.or_else(|| complete_reference_source(source).err().map(|e| e.0));
    editor.rename_available = editor.rename_reason.is_none();
    editor
}

fn check_citation_basis(source: &str, hash: &str) -> Result<(), EngineError> {
    if crate::source_hash(source) != hash {
        return Err(EngineError("stale-citation-source".into()));
    }
    if source.len() > 8 * 1024 * 1024
        || !Source::detached(source)
            .root()
            .errors_and_warnings()
            .0
            .is_empty()
    {
        return Err(EngineError(
            "citation-edit: valid source within budget required".into(),
        ));
    }
    Ok(())
}

fn patch(source: &str, range: std::ops::Range<usize>, insert: String) -> crate::SourcePatch {
    let range = crate::editor::source_range(source, range);
    crate::SourcePatch {
        from: range.from,
        to: range.to,
        utf16_from: range.utf16_from,
        utf16_to: range.utf16_to,
        expected: range.raw,
        insert,
    }
}

fn finish_citations(
    source: &str,
    mut patches: Vec<crate::SourcePatch>,
) -> Result<crate::EditOutput, EngineError> {
    // Binding prefix and body citation can intentionally share an empty source.
    let mut merged: Vec<crate::SourcePatch> = vec![];
    for next in patches.drain(..) {
        if next.from == next.to
            && let Some(existing) = merged
                .iter_mut()
                .find(|p| p.from == next.from && p.to == next.to)
        {
            existing.insert.push_str(&next.insert);
        } else {
            merged.push(next);
        }
    }
    crate::editor::finish(source, merged)
}

fn check_insertion(source: &str, at: usize) -> Result<(), EngineError> {
    if at > source.len() || !source.is_char_boundary(at) {
        return Err(EngineError(
            "citation-insertion: invalid byte boundary".into(),
        ));
    }
    let map = crate::inspect_editor(source);
    if map.body_insertion.as_ref().is_some_and(|r| r.from == at) {
        return Ok(());
    }
    let eligible = map
        .literal_regions
        .iter()
        .any(|r| r.range.writable && r.range.from <= at && at <= r.range.to);
    let parsed = Source::detached(source);
    let mut plain = false;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if matches!(node.kind(), SyntaxKind::Text | SyntaxKind::Space)
            && node.range().start <= at
            && at <= node.range().end
        {
            plain = true;
        }
    });
    if eligible && plain {
        Ok(())
    } else {
        Err(EngineError(
            "citation-insertion: unique plain markup writer required".into(),
        ))
    }
}

fn citation_argument_patches(
    source: &str,
    range: &SourceRange,
    values: &[(&str, String)],
) -> Result<Vec<crate::SourcePatch>, EngineError> {
    let parsed = Source::detached(source);
    let mut found = None;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node.range() == (range.from..range.to) && node.get().cast::<ast::FuncCall>().is_some() {
            found = Some(node.clone());
        }
    });
    let node = found
        .ok_or_else(|| EngineError("citation-target: stale or unavailable source slot".into()))?;
    let control = citation_control(source, &node, citation_binding_available(source, "cite"));
    if range.raw != control.range.raw
        || call_name(node.get().cast::<ast::FuncCall>().unwrap()) != Some("cite")
        || !control.editable
    {
        return Err(EngineError(
            "citation-target: static standard citation required".into(),
        ));
    }
    let call = node.get().cast::<ast::FuncCall>().unwrap();
    let mut patches = vec![];
    let mut missing = vec![];
    for (name, value) in values {
        if let Some(named) = call.args().items().find_map(|a| match a {
            ast::Arg::Named(n) if n.name().get() == *name => Some(n),
            _ => None,
        }) {
            let r = parsed
                .find(named.expr().to_untyped().span())
                .unwrap()
                .range();
            patches.push(patch(source, r, value.clone()));
        } else {
            missing.push(format!("{name}: {value}"));
        }
    }
    if !missing.is_empty() {
        let args_node = parsed.find(call.args().to_untyped().span()).unwrap();
        let args = args_node.range();
        if source.as_bytes().get(args.end - 1) != Some(&b')') {
            return Err(EngineError(
                "citation-target: trailing content arguments unavailable".into(),
            ));
        }
        let trailing_comma = args_node
            .children()
            .rfind(|n| !n.kind().is_trivia() && n.range().end < args.end)
            .is_some_and(|n| n.kind() == SyntaxKind::Comma);
        let separator = if trailing_comma { " " } else { ", " };
        patches.push(patch(
            source,
            args.end - 1..args.end - 1,
            format!("{separator}{}", missing.join(", ")),
        ));
    }
    Ok(patches)
}

pub fn update_native_citation(
    source: &str,
    expected_hash: &str,
    range: &SourceRange,
    form: CitationForm,
    supplement: &str,
) -> Result<crate::EditOutput, EngineError> {
    update_native_citation_options(source, expected_hash, range, Some(form), Some(supplement))
}

/// None keeps that occurrence's exact authored expression or native inheritance;
/// Some("") for the supplement is an intentional explicit empty override.
pub fn update_native_citation_options(
    source: &str,
    expected_hash: &str,
    range: &SourceRange,
    form: Option<CitationForm>,
    supplement: Option<&str>,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    let mut values = vec![];
    if let Some(form) = form {
        values.push(("form", form.native().into()));
    }
    if let Some(supplement) = supplement {
        values.push((
            "supplement",
            format!("[#{}]", crate::encode_string(supplement)?),
        ));
    }
    let output = finish_citations(source, citation_argument_patches(source, range, &values)?)?;
    // Official AST must read back the requested native semantics.
    let control = inspect_native_citation_editor(&output.source)
        .citations
        .into_iter()
        .find(|c| c.range.from == range.from)
        .ok_or_else(|| EngineError("citation-readback-failed".into()))?;
    if form.is_some_and(|form| control.form != Some(form))
        || supplement.is_some_and(|text| control.supplement.as_deref() != Some(text))
    {
        return Err(EngineError("citation-readback-failed".into()));
    }
    Ok(output)
}

pub fn update_native_citation_style(
    source: &str,
    expected_hash: &str,
    range: &SourceRange,
    style: Option<&str>,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    if let Some(style) = style {
        return finish_citations(
            source,
            citation_argument_patches(source, range, &[("style", crate::encode_string(style)?)])?,
        );
    }
    // Validate the same exact certified call before removing only the named
    // argument and one AST comma token; neighboring comments stay byte exact.
    citation_argument_patches(source, range, &[])?;
    let parsed = Source::detached(source);
    let mut patches = vec![];
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node.range() != (range.from..range.to) {
            return;
        }
        let Some(call) = node.get().cast::<ast::FuncCall>() else {
            return;
        };
        let Some(named) = call.args().items().find_map(|a| match a {
            ast::Arg::Named(n) if n.name().get() == "style" => Some(n),
            _ => None,
        }) else {
            return;
        };
        let named_range = parsed.find(named.to_untyped().span()).unwrap().range();
        let args = parsed.find(call.args().to_untyped().span()).unwrap();
        let comma = args
            .children()
            .rfind(|n| n.kind() == SyntaxKind::Comma && n.range().end <= named_range.start)
            .or_else(|| {
                args.children()
                    .find(|n| n.kind() == SyntaxKind::Comma && n.range().start >= named_range.end)
            });
        if let Some(comma) = comma {
            patches.push(patch(source, comma.range(), String::new()));
        }
        patches.push(patch(source, named_range, String::new()));
    });
    finish_citations(source, patches)
}

/// Same Work with one existing alias is reused regardless of a changed hint.
/// Multiple aliases require an explicit choice. Different Works never coalesce.
pub fn suggest_native_citation_key(
    source: &str,
    work: &str,
    suggested_key: &str,
    chosen_key: Option<&str>,
) -> Result<String, EngineError> {
    if !valid_node_ref(work) {
        return Err(EngineError(
            "citation-work: complete stable Work NodeRef required".into(),
        ));
    }
    inspect_citation_declaration(source)?;
    let editor = inspect_native_citation_editor(source);
    let aliases: Vec<_> = editor.bindings.iter().filter(|e| e.work == work).collect();
    if let Some(key) = chosen_key {
        if !valid_citation_key(key) {
            return Err(EngineError("citation-key: ASCII key required".into()));
        }
        if let Some(entry) = editor.bindings.iter().find(|e| e.key == key) {
            return if entry.work == work {
                if editor.targets.iter().any(|t| t.label == key) {
                    Err(EngineError(
                        "citation-key: existing Work alias conflicts with a document label".into(),
                    ))
                } else {
                    Ok(key.into())
                }
            } else {
                Err(EngineError(
                    "citation-key: key already binds another Work".into(),
                ))
            };
        }
        if !aliases.is_empty() {
            return Err(EngineError(
                "citation-key: choose one of the existing Work aliases".into(),
            ));
        }
        if editor.targets.iter().any(|t| t.label == key)
            || editor.citations.iter().any(|c| c.key == key)
            || editor.cross_references.iter().any(|r| r.label == key)
        {
            return Err(EngineError(
                "citation-key: conflicts with an existing unbound reference or document label"
                    .into(),
            ));
        }
        return Ok(key.into());
    }
    match aliases.as_slice() {
        [entry] => {
            return if editor.targets.iter().any(|t| t.label == entry.key) {
                Err(EngineError(
                    "citation-key: existing Work alias conflicts with a document label".into(),
                ))
            } else {
                Ok(entry.key.clone())
            };
        }
        [] => {}
        _ => {
            return Err(EngineError(
                "citation-key: multiple existing aliases require a choice".into(),
            ));
        }
    }
    if !valid_citation_key(suggested_key) {
        return Err(EngineError(
            "citation-key: invalid suggestion; choose an ASCII key explicitly".into(),
        ));
    }
    let occupied = |key: &str| {
        editor.bindings.iter().any(|e| e.key == key)
            || editor.targets.iter().any(|t| t.label == key)
            || editor.citations.iter().any(|c| c.key == key)
            || editor.cross_references.iter().any(|r| r.label == key)
    };
    if !occupied(suggested_key) {
        return Ok(suggested_key.into());
    }
    for suffix in 2..=4097 {
        let key = format!("{suggested_key}-{suffix}");
        if !occupied(&key) {
            return Ok(key);
        }
    }
    Err(EngineError(
        "citation-key: suggestion budget exhausted".into(),
    ))
}

#[allow(clippy::too_many_arguments)] // One authoring intent, bound to one exact source basis.
pub fn insert_native_citation(
    source: &str,
    expected_hash: &str,
    at: usize,
    work: &str,
    suggested_key: &str,
    chosen_key: Option<&str>,
    form: CitationForm,
    supplement: &str,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    check_insertion(source, at)?;
    for name in ["cite", "metadata", "bibliography"] {
        if !citation_binding_available(source, name) {
            return Err(EngineError(format!(
                "citation-insertion: unverified {name} binding"
            )));
        }
    }
    let declaration = inspect_citation_declaration(source)?;
    let key = suggest_native_citation_key(source, work, suggested_key, chosen_key)?;
    let mut patches = vec![];
    if declaration
        .as_ref()
        .is_none_or(|d| !d.entries.iter().any(|e| e.key == key && e.work == work))
    {
        let entry = format!(
            "(key: {}, work: {})",
            crate::encode_string(&key)?,
            crate::encode_string(work)?
        );
        if let Some(declaration) = declaration {
            let parsed = Source::detached(source);
            let mut array_range = None;
            walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
                if node.range().start >= declaration.range.from
                    && node.range().end <= declaration.range.to
                    && node.get().cast::<ast::Array>().is_some()
                {
                    array_range = Some(node.range());
                }
            });
            let array = array_range.ok_or_else(|| {
                EngineError("citation-declaration: entries array unavailable".into())
            })?;
            patches.push(patch(
                source,
                array.start + 1..array.start + 1,
                format!("{entry}, "),
            ));
        } else {
            let metadata = crate::inspect_metadata(source);
            let at = metadata
                .metadata
                .filter(|_| metadata.id.is_some())
                .map_or_else(
                    || usize::from(source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8(),
                    |metadata| metadata.to,
                );
            let separator = if at > 0 { "\n" } else { "" };
            patches.push(patch(source,at..at,format!("{separator}#metadata((kind: \"weftext.citations\", entries: ({entry},))) <weftext-citations>\n")));
        }
    }
    let parsed = Source::detached(source);
    let mut bibliographies = vec![];
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if let Some(call) = node.get().cast::<ast::FuncCall>()
            && call_name(call) == Some("bibliography")
        {
            bibliographies.push(node.parent().is_some_and(|p|p.kind()==SyntaxKind::Markup&&p.parent().is_none())&&matches!(call.args().items().next(),Some(ast::Arg::Pos(e)) if e.to_untyped().cast::<ast::Str>().is_some_and(|s|s.get()=="weftext.citations.bib")));
        }
    });
    if bibliographies.len() > 1 || bibliographies.first() == Some(&false) {
        return Err(EngineError(
            "citation-insertion: external or ambiguous bibliography ownership; source preserved"
                .into(),
        ));
    }
    let citation = format!(
        "#cite(<{key}>, form: {}, supplement: [#{}])",
        form.native(),
        crate::encode_string(supplement)?
    );
    let shift: isize = patches
        .iter()
        .filter(|p| p.from <= at)
        .map(|p| p.insert.len() as isize - (p.to - p.from) as isize)
        .sum();
    let citation_end = (at as isize + shift) as usize + citation.len();
    patches.push(patch(source, at..at, citation));
    if bibliographies.is_empty() {
        patches.push(patch(
            source,
            source.len()..source.len(),
            "\n\n#bibliography(\"weftext.citations.bib\")\n".into(),
        ));
    }
    let mut output = finish_citations(source, patches)?;
    if crate::inspect_metadata(&output.source).id != crate::inspect_metadata(source).id {
        return Err(EngineError(
            "citation-insertion: document identity would change; source preserved".into(),
        ));
    }
    let after = inspect_citation_declaration(&output.source)?
        .ok_or_else(|| EngineError("citation-readback-failed".into()))?;
    if !after.entries.iter().any(|e| e.key == key && e.work == work) {
        return Err(EngineError("citation-readback-failed".into()));
    }
    let caret = output.source[..citation_end].encode_utf16().count();
    output.selection = crate::EditSelection {
        from: caret,
        to: caret,
    };
    Ok(output)
}

pub fn rename_native_citation_key(
    source: &str,
    expected_hash: &str,
    old: &str,
    new: &str,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    complete_reference_source(source)?;
    if !valid_citation_key(new) || old == new {
        return Err(EngineError(
            "citation-key: distinct valid new key required".into(),
        ));
    }
    let declaration = inspect_citation_declaration(source)?
        .ok_or_else(|| EngineError("citation-key: binding declaration required".into()))?;
    if !declaration.entries.iter().any(|e| e.key == old)
        || declaration.entries.iter().any(|e| e.key == new)
    {
        return Err(EngineError(
            "citation-key: missing source key or occupied destination".into(),
        ));
    }
    let editor = inspect_native_citation_editor(source);
    if editor
        .targets
        .iter()
        .any(|t| t.label == old || t.label == new)
        || editor.citations.iter().any(|c| c.key == new)
        || editor.cross_references.iter().any(|r| r.label == new)
    {
        return Err(EngineError("citation-key: document-label collision".into()));
    }
    let parsed = Source::detached(source);
    let mut patches = vec![];
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node.range().start >= declaration.range.from
            && node.range().end <= declaration.range.to
            && let Some(named) = node.get().cast::<ast::Named>()
            && named.name().get() == "key"
            && named
                .expr()
                .to_untyped()
                .cast::<ast::Str>()
                .is_some_and(|s| s.get() == old)
        {
            let r = parsed
                .find(named.expr().to_untyped().span())
                .unwrap()
                .range();
            patches.push(patch(source, r, format!("\"{new}\"")));
        }
        // Keyed dictionary spelling is also certified by the declaration reader.
        if node.range().start >= declaration.range.from
            && node.range().end <= declaration.range.to
            && let Some(keyed) = node.get().cast::<ast::Keyed>()
            && keyed
                .key()
                .to_untyped()
                .cast::<ast::Str>()
                .is_some_and(|s| s.get() == "key")
            && keyed
                .expr()
                .to_untyped()
                .cast::<ast::Str>()
                .is_some_and(|s| s.get() == old)
        {
            let r = parsed
                .find(keyed.expr().to_untyped().span())
                .unwrap()
                .range();
            patches.push(patch(source, r, format!("\"{new}\"")));
        }
        if let Some(label) = node.get().cast::<ast::Label>()
            && label.get() == old
        {
            // Under the complete profile a label-valued argument is a static use.
            patches.push(patch(source, node.range(), format!("<{new}>")));
        }
        if let Some(reference) = node.get().cast::<ast::Ref>()
            && reference.target() == old
            && let Some(marker) = node.children().find(|n| n.kind() == SyntaxKind::RefMarker)
        {
            patches.push(patch(source, marker.range(), format!("@{new}")));
        }
    });
    let output = finish_citations(source, patches)?;
    let after = inspect_citation_declaration(&output.source)?.unwrap();
    if after.entries.iter().any(|e| e.key == old) || !after.entries.iter().any(|e| e.key == new) {
        return Err(EngineError("citation-rename-readback-failed".into()));
    }
    Ok(output)
}

pub fn insert_native_cross_reference(
    source: &str,
    expected_hash: &str,
    at: usize,
    label: &str,
    page: bool,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    check_insertion(source, at)?;
    if !citation_binding_available(source, "ref") {
        return Err(EngineError(
            "cross-reference: unverified ref binding".into(),
        ));
    }
    let editor = inspect_native_citation_editor(source);
    if !editor.targets.iter().any(|t| t.label == label && t.unique)
        || editor.bindings.iter().any(|e| e.key == label)
    {
        return Err(EngineError(
            "cross-reference: unique document label without a citation collision required".into(),
        ));
    }
    let insert = if page {
        format!("#ref(<{label}>, form: \"page\")")
    } else {
        format!("#ref(<{label}>)")
    };
    finish_citations(source, vec![patch(source, at..at, insert)])
}

pub fn rename_native_label(
    source: &str,
    expected_hash: &str,
    old: &str,
    new: &str,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    complete_reference_source(source)?;
    let label_source = format!("<{new}>");
    let label_parse = Source::detached(&label_source);
    if old == new
        || !label_parse.root().errors_and_warnings().0.is_empty()
        || label_parse.root().children().count() != 1
        || !label_parse
            .root()
            .children()
            .next()
            .is_some_and(|n| n.cast::<ast::Label>().is_some_and(|l| l.get() == new))
    {
        return Err(EngineError(
            "cross-reference: distinct native label required".into(),
        ));
    }
    let editor = inspect_native_citation_editor(source);
    let marker_source = format!("@{new}");
    let marker_parse = Source::detached(&marker_source);
    let exact_marker = marker_parse.root().errors_and_warnings().0.is_empty()
        && marker_parse.root().children().count() == 1
        && marker_parse
            .root()
            .children()
            .next()
            .is_some_and(|n| n.cast::<ast::Ref>().is_some_and(|r| r.target() == new));
    if !exact_marker
        && editor
            .cross_references
            .iter()
            .any(|r| r.kind == "unclassified" && r.label == old)
    {
        return Err(EngineError("cross-reference: new label cannot be represented by existing @ reference markers without changing semantics".into()));
    }
    if !editor.targets.iter().any(|t| t.label == old && t.unique)
        || editor.targets.iter().any(|t| t.label == new)
        || editor.bindings.iter().any(|e| e.key == old || e.key == new)
        || editor
            .citations
            .iter()
            .any(|c| c.key == old || c.key == new)
        || editor.cross_references.iter().any(|r| r.label == new)
    {
        return Err(EngineError(
            "cross-reference: missing, duplicate or colliding label".into(),
        ));
    }
    let parsed = Source::detached(source);
    let mut patches = vec![];
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node
            .get()
            .cast::<ast::Label>()
            .is_some_and(|l| l.get() == old)
        {
            patches.push(patch(source, node.range(), format!("<{new}>")));
        }
        if node
            .get()
            .cast::<ast::Ref>()
            .is_some_and(|r| r.target() == old)
            && let Some(marker) = node.children().find(|n| n.kind() == SyntaxKind::RefMarker)
        {
            patches.push(patch(source, marker.range(), format!("@{new}")));
        }
    });
    finish_citations(source, patches)
}

fn valid_native_label(label: &str) -> bool {
    let source = format!("<{label}>");
    let parsed = Source::detached(&source);
    parsed.root().errors_and_warnings().0.is_empty()
        && parsed.root().children().count() == 1
        && parsed
            .root()
            .children()
            .next()
            .is_some_and(|n| n.cast::<ast::Label>().is_some_and(|l| l.get() == label))
}

/// Add an explicit author label and its reference in one exact Draft edit.
/// The target range must additionally be matched to a fresh compiled object by
/// the host; this source adapter never infers numbering or page availability.
pub fn insert_native_cross_reference_with_label(
    source: &str,
    expected_hash: &str,
    at: usize,
    target: &SourceRange,
    label: &str,
    page: bool,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    check_insertion(source, at)?;
    if !valid_native_label(label) || !citation_binding_available(source, "ref") {
        return Err(EngineError(
            "cross-reference: valid label and standard ref binding required".into(),
        ));
    }
    let editor = inspect_native_citation_editor(source);
    if editor.targets.iter().any(|t| t.label == label)
        || editor.bindings.iter().any(|e| e.key == label)
        || editor.cross_references.iter().any(|r| r.label == label)
        || editor.citations.iter().any(|c| c.key == label)
    {
        return Err(EngineError(
            "cross-reference: destination label is already used".into(),
        ));
    }
    let parsed = Source::detached(source);
    let mut found = None;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node.range() == (target.from..target.to)
            && source.get(node.range()) == Some(target.raw.as_str())
        {
            found = Some(node.clone());
        }
    });
    let node =
        found.ok_or_else(|| EngineError("cross-reference: stale target source slot".into()))?;
    if node
        .parent()
        .is_none_or(|p| p.kind() != SyntaxKind::Markup || p.parent().is_some())
    {
        return Err(EngineError(
            "cross-reference: direct main-source target required".into(),
        ));
    }
    let supported = matches!(node.kind(), SyntaxKind::Heading | SyntaxKind::Equation)
        || node.get().cast::<ast::FuncCall>().is_some_and(|call| {
            matches!(call_name(call), Some("figure" | "table" | "equation"))
                && citation_binding_available(source, call_name(call).unwrap())
        });
    if !supported {
        return Err(EngineError(
            "cross-reference: supported heading, figure, table or equation source slot required"
                .into(),
        ));
    }
    let mut has_label = false;
    walk_nodes(&node, &mut |child| {
        if child.get().cast::<ast::Label>().is_some()
            && child.parent_kind() == Some(SyntaxKind::Markup)
        {
            has_label = true;
        }
    });
    let mut next = node.next_sibling();
    while next.as_ref().is_some_and(|n| n.kind().is_trivia()) {
        next = next.unwrap().next_sibling();
    }
    has_label |= next
        .as_ref()
        .is_some_and(|n| n.get().cast::<ast::Label>().is_some());
    if has_label {
        return Err(EngineError(
            "cross-reference: target already has an author label; reuse it explicitly".into(),
        ));
    }
    if at > target.from && at < target.to {
        return Err(EngineError(
            "cross-reference: reference insertion cannot rewrite its target body".into(),
        ));
    }
    let label_insert = format!(" <{label}>");
    let reference = if page {
        format!("#ref(<{label}>, form: \"page\")")
    } else {
        format!("#ref(<{label}>)")
    };
    let caret =
        at + if target.to <= at {
            label_insert.len()
        } else {
            0
        } + reference.len();
    let mut output = finish_citations(
        source,
        vec![
            patch(source, target.to..target.to, label_insert),
            patch(source, at..at, reference),
        ],
    )?;
    if !inspect_native_citation_editor(&output.source)
        .targets
        .iter()
        .any(|t| t.label == label && t.unique)
    {
        return Err(EngineError("cross-reference: label readback failed".into()));
    }
    let caret = output.source[..caret].encode_utf16().count();
    output.selection = crate::EditSelection {
        from: caret,
        to: caret,
    };
    Ok(output)
}

pub fn insert_native_link(
    source: &str,
    expected_hash: &str,
    at: usize,
    target: &str,
    text: &str,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    check_insertion(source, at)?;
    if !citation_binding_available(source, "link") {
        return Err(EngineError(
            "native-link: standard link binding required".into(),
        ));
    }
    let insert = format!(
        "#link({})[#{}]",
        crate::encode_string(target)?,
        crate::encode_string(text)?
    );
    finish_citations(source, vec![patch(source, at..at, insert)])
}

pub fn insert_native_cross_link(
    source: &str,
    expected_hash: &str,
    at: usize,
    label: &str,
    text: &str,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    check_insertion(source, at)?;
    if !citation_binding_available(source, "link") {
        return Err(EngineError(
            "cross-reference: standard link binding required".into(),
        ));
    }
    let editor = inspect_native_citation_editor(source);
    if !editor.targets.iter().any(|t| t.label == label && t.unique)
        || editor.bindings.iter().any(|e| e.key == label)
    {
        return Err(EngineError(
            "cross-reference: unique document label without a citation collision required".into(),
        ));
    }
    let insert = format!("#link(<{label}>)[#{}]", crate::encode_string(text)?);
    finish_citations(source, vec![patch(source, at..at, insert)])
}

/// Replace only a proved plain link body. Targets, parameters and all bytes
/// outside that body expression survive exactly; formatted/dynamic bodies keep
/// their native source and receive no misleading flatten-to-text operation.
pub fn update_native_link_text(
    source: &str,
    expected_hash: &str,
    range: &SourceRange,
    text: &str,
) -> Result<crate::EditOutput, EngineError> {
    check_citation_basis(source, expected_hash)?;
    if !citation_binding_available(source, "link") {
        return Err(EngineError(
            "native-link: standard link binding required".into(),
        ));
    }
    let parsed = Source::detached(source);
    let mut found = None;
    walk_nodes(&LinkedNode::new(parsed.root()), &mut |node| {
        if node.range() == (range.from..range.to)
            && source.get(node.range()) == Some(range.raw.as_str())
        {
            found = Some(node.clone());
        }
    });
    let node = found.ok_or_else(|| EngineError("native-link: stale link source slot".into()))?;
    let call = node
        .get()
        .cast::<ast::FuncCall>()
        .filter(|c| call_name(*c) == Some("link"))
        .ok_or_else(|| EngineError("native-link: direct standard link required".into()))?;
    let args: Vec<_> = call.args().items().collect();
    let Some(ast::Arg::Pos(target)) = args.first() else {
        return Err(EngineError("native-link: static target required".into()));
    };
    if target.to_untyped().cast::<ast::Str>().is_none()
        && target.to_untyped().cast::<ast::Label>().is_none()
    {
        return Err(EngineError("native-link: static target required".into()));
    }
    let mut bodies = vec![];
    let mut positional = 0usize;
    for arg in &args {
        match arg {
            ast::Arg::Pos(expr) => {
                positional += 1;
                if positional > 1 {
                    bodies.push(*expr);
                }
            }
            ast::Arg::Named(named) if named.name().get() == "body" => bodies.push(named.expr()),
            ast::Arg::Named(_) => {}
            _ => {
                return Err(EngineError(
                    "native-link: spread arguments are unavailable".into(),
                ));
            }
        }
    }
    let [body] = bodies.as_slice() else {
        return Err(EngineError(
            "native-link: one explicit plain body required".into(),
        ));
    };
    if plain_supplement(*body).is_none() {
        return Err(EngineError(
            "native-link: formatted or generated body preserved; edit its source".into(),
        ));
    }
    if !unique_authored_slot(source, &node) {
        return Err(EngineError(
            "native-link: generated or reused writer unavailable".into(),
        ));
    }
    let range = parsed.find(body.to_untyped().span()).unwrap().range();
    finish_citations(
        source,
        vec![patch(
            source,
            range,
            format!("[#{}]", crate::encode_string(text)?),
        )],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const WORK: &str =
        "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/8aa10c08-3e7d-4d10-9488-92bc7f393050";
    #[test]
    fn inserting_and_reusing_citation_keeps_primary_metadata_qualified() {
        let id = "8aa10c08-3e7d-4d10-9488-92bc7f393050";
        let original = format!(
            "\u{feff}#metadata((id: \"{id}\", types: ())) <weftext>\n#set document(title: [标题])\n#title()\n正文\n"
        );
        let first = insert_native_citation(
            &original,
            &crate::source_hash(&original),
            original.find("正文").unwrap(),
            WORK,
            "reader2026",
            None,
            CitationForm::Prose,
            "页码",
        )
        .unwrap();
        assert_eq!(
            crate::inspect_metadata(&first.source).id.as_deref(),
            Some(id)
        );
        assert!(
            first
                .source
                .starts_with(original.split('\n').next().unwrap())
        );
        let second = insert_native_citation(
            &first.source,
            &crate::source_hash(&first.source),
            first.source.find("正文").unwrap(),
            WORK,
            "reader2026",
            None,
            CitationForm::None,
            "",
        )
        .unwrap();
        assert_eq!(
            crate::inspect_metadata(&second.source).id.as_deref(),
            Some(id)
        );
        assert_eq!(
            inspect_citation_declaration(&second.source)
                .unwrap()
                .unwrap()
                .entries
                .len(),
            1
        );
        assert_eq!(
            inspect_native_citation_editor(&second.source)
                .citations
                .len(),
            2
        );
    }
    fn source(entries: &str) -> String {
        format!(
            "#metadata((kind: \"weftext.citations\", entries: ({entries}))) <weftext-citations>\n= Heading <refs>\n@refs #cite(<chen2026>)"
        )
    }
    #[test]
    fn direct_stable_binding_does_not_promote_heading_refs() {
        let source = source(&format!("(key: \"chen2026\", work: \"{WORK}\"),"));
        let declaration = inspect_citation_declaration(&source).unwrap().unwrap();
        assert_eq!(declaration.entries[0].work, WORK);
        assert_eq!(declaration.work_slots[0].range.raw, format!("\"{WORK}\""));
        assert_eq!(
            &source[declaration.work_slots[0].range.from..declaration.work_slots[0].range.to],
            format!("\"{WORK}\"")
        );
        assert_eq!(
            proven_static_cites(&source)
                .iter()
                .map(|c| c.key.as_str())
                .collect::<Vec<_>>(),
            ["chen2026"]
        );
    }
    #[test]
    fn declaration_binding_accepts_only_nonshadowing_explicit_imports_after_the_role() {
        let declaration = source(&format!("(key: \"k\", work: \"{WORK}\"),"));
        for import in [
            "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes",
            "#import \"untrusted.typ\": other as isolated",
        ] {
            let combined = format!("{declaration}\n{import}");
            let role = inspect_citation_declaration(&combined).unwrap().unwrap();
            assert_eq!(role.entries[0].work, WORK);
            assert!(proven_static_cites(&combined).is_empty());
            assert!(!inspect_native_citation_editor(&combined).rename_available);
            assert!(inspect_citation_declaration(&format!("{import}\n{declaration}")).is_err());
        }
        for suffix in [
            "#import \"untrusted.typ\": *",
            "#import \"untrusted.typ\": metadata",
            "#import \"untrusted.typ\": other as metadata",
            "#import \"untrusted.typ\" as metadata",
            "#import (\"untrusted.typ\"): other",
            "#include \"untrusted.typ\"",
            "#let metadata(value) = value",
            "#for metadata in () []",
            "<weftext-citations>",
        ] {
            assert!(
                inspect_citation_declaration(&format!("{declaration}\n{suffix}")).is_err(),
                "{suffix}"
            );
        }
        assert!(inspect_citation_declaration(&format!("{declaration}\n{declaration}")).is_err());
    }

    #[test]
    fn portable_bibliography_input_is_independent_of_full_reference_coverage() {
        let declaration = source(&format!("(key: \"k\", work: \"{WORK}\"),"));
        let combined = format!(
            "{declaration}\n#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n\
             #endnote[Authored note]\n@k\n#bibliography(\"weftext.citations.bib\", style: \"ieee\", title: none)"
        );
        assert!(native_citation_bibliography_input(&combined));
        assert!(!native_citation_owner_complete(&combined));
        assert!(!inspect_native_citation_editor(&combined).rename_available);
        for suffix in [
            "#bibliography(\"external.bib\")",
            "#bibliography(\"weftext.citations.bib\")",
            "#import \"other.typ\": *",
            "#import \"other.typ\": bibliography",
            "#import \"other.typ\": other as bibliography",
            "#import \"other.typ\" as bibliography",
            "#import (\"other.typ\"): other",
            "#include \"other.typ\"",
            "#let bibliography(value) = value",
            "#let alias = bibliography",
            "#for bibliography in () []",
            "#let metadata(value) = value",
        ] {
            assert!(
                !native_citation_bibliography_input(&format!("{combined}\n{suffix}")),
                "{suffix}"
            );
        }
        for bibliography in [
            "#bibliography(path)",
            "#bibliography(\"external.bib\")",
            "#bibliography((\"weftext.citations.bib\", \"other.bib\"))",
            "#bibliography(\"weftext.citations.bib\", ..options)",
            "#bibliography(\"weftext.citations.bib\", style: selected-style)",
        ] {
            assert!(
                !native_citation_bibliography_input(&format!("{declaration}\n{bibliography}")),
                "{bibliography}"
            );
        }
    }

    #[test]
    fn duplicate_dynamic_and_shadowed_roles_reject() {
        let entry = format!("(key: \"k\", work: \"{WORK}\"),");
        assert!(inspect_citation_declaration(&source(&format!("{entry}{entry}"))).is_err());
        assert!(
            inspect_citation_declaration(&format!("{}\n<weftext-citations>", source(&entry)))
                .is_err()
        );
        assert!(
            inspect_citation_declaration(&format!("#let metadata(x) = x\n{}", source(&entry)))
                .is_err()
        );
        assert!(
            inspect_citation_declaration(
                "#metadata((kind: \"weftext.citations\", entries: data)) <weftext-citations>"
            )
            .is_err()
        );
        assert!(inspect_citation_declaration(&format!("#context [{}]", source(&entry))).is_err());
        assert!(inspect_citation_declaration("@refs").unwrap().is_none());
        assert!(
            inspect_citation_declaration("#cite(<weftext-citations>)")
                .unwrap()
                .is_none()
        );
        assert!(
            inspect_citation_declaration(&source(""))
                .unwrap()
                .unwrap()
                .entries
                .is_empty()
        );
    }

    fn bound_source(body: &str) -> String {
        format!(
            "#metadata((kind: \"weftext.citations\", entries: ((key: \"chen2026\", work: \"{WORK}\"),))) <weftext-citations>\r\n{body}\r\n#bibliography(\"weftext.citations.bib\")"
        )
    }

    fn assert_exact_patch_contract(before: &str, output: &crate::EditOutput) {
        let mut rebuilt = before.to_owned();
        for p in output.patches.iter().rev() {
            assert_eq!(&before[p.from..p.to], p.expected);
            assert_eq!(before[..p.from].encode_utf16().count(), p.utf16_from);
            assert_eq!(before[..p.to].encode_utf16().count(), p.utf16_to);
            rebuilt.replace_range(p.from..p.to, &p.insert);
        }
        assert_eq!(rebuilt, output.source);
        assert!(
            Source::detached(&output.source)
                .root()
                .errors_and_warnings()
                .0
                .is_empty()
        );
    }

    #[test]
    fn citation_forms_and_plain_supplements_round_trip_without_code_injection() {
        let before = bound_source(
            "中文😀 #cite(<chen2026>, /*form note*/ form: \"normal\", style: custom, supplement: [p. 3]) suffix",
        );
        let range = inspect_native_citation_editor(&before).citations[0]
            .range
            .clone();
        let supplement = "中文 ] #panic(\"injected\") \\ 😀\n页 2";
        for form in [
            CitationForm::Normal,
            CitationForm::Prose,
            CitationForm::Author,
            CitationForm::Year,
            CitationForm::Full,
            CitationForm::None,
        ] {
            let output = update_native_citation(
                &before,
                &crate::source_hash(&before),
                &range,
                form,
                supplement,
            )
            .unwrap();
            assert_exact_patch_contract(&before, &output);
            assert!(output.source.contains("/*form note*/ form:"));
            assert!(output.source.contains("style: custom"));
            assert_eq!(output.patches.len(), 2);
            let editor = inspect_native_citation_editor(&output.source);
            assert_eq!(editor.citations[0].form, Some(form));
            assert_eq!(editor.citations[0].supplement.as_deref(), Some(supplement));
        }
    }

    #[test]
    fn inherited_options_are_added_without_rewriting_other_arguments() {
        let before = bound_source(
            "#set cite(form: \"author\")\n#cite(<chen2026>, style: \"apa\", /*keep*/)",
        );
        let editor = inspect_native_citation_editor(&before);
        assert_eq!(editor.citations[0].form, None);
        let output = update_native_citation(
            &before,
            &editor.source_hash,
            &editor.citations[0].range,
            CitationForm::Year,
            "42",
        )
        .unwrap();
        assert_exact_patch_contract(&before, &output);
        assert!(output.source.contains("style: \"apa\", /*keep*/"));
        assert!(output.source.contains("#set cite(form: \"author\")"));
    }

    #[test]
    fn partial_option_updates_keep_other_authored_and_inherited_parameters_exact() {
        let before = bound_source(
            "#set cite(form: \"author\", supplement: [继承])\n#cite(<chen2026>, style: \"apa\")",
        );
        let editor = inspect_native_citation_editor(&before);
        let range = &editor.citations[0].range;
        let form_only = update_native_citation_options(
            &before,
            &editor.source_hash,
            range,
            Some(CitationForm::Year),
            None,
        )
        .unwrap();
        assert_exact_patch_contract(&before, &form_only);
        let after = inspect_native_citation_editor(&form_only.source);
        assert_eq!(after.citations[0].form, Some(CitationForm::Year));
        assert_eq!(after.citations[0].supplement, None);
        assert!(
            form_only
                .source
                .contains("#set cite(form: \"author\", supplement: [继承])")
        );
        let supplement_only = update_native_citation_options(
            &before,
            &editor.source_hash,
            range,
            None,
            Some("explicit"),
        )
        .unwrap();
        assert_exact_patch_contract(&before, &supplement_only);
        let after = inspect_native_citation_editor(&supplement_only.source);
        assert_eq!(after.citations[0].form, None);
        assert_eq!(after.citations[0].supplement.as_deref(), Some("explicit"));
        assert_eq!(after.citations[0].style.as_deref(), Some("apa"));
        let noop = update_native_citation_options(&before, &editor.source_hash, range, None, None)
            .unwrap();
        assert_eq!(noop.source, before);
        assert!(noop.patches.is_empty());
    }

    #[test]
    fn removing_local_style_preserves_trivia_and_restores_set_inheritance() {
        let before = bound_source(
            "#set cite(style: \"apa\")\n#cite(<chen2026>, /*before*/ style: \"ieee\" /*after*/, form: \"prose\")",
        );
        let editor = inspect_native_citation_editor(&before);
        let output = update_native_citation_style(
            &before,
            &editor.source_hash,
            &editor.citations[0].range,
            None,
        )
        .unwrap();
        assert_exact_patch_contract(&before, &output);
        assert!(
            output
                .source
                .contains("/*before*/  /*after*/, form: \"prose\"")
        );
        assert_eq!(
            inspect_native_citation_editor(&output.source).citations[0].style,
            None
        );
        assert!(output.source.contains("#set cite(style: \"apa\")"));
    }

    #[test]
    fn insert_binding_citation_and_bibliography_is_one_exact_edit() {
        let before = "前😀后\r\n";
        let at = "前😀".len();
        let output = insert_native_citation(
            before,
            &crate::source_hash(before),
            at,
            WORK,
            "chen2026",
            None,
            CitationForm::Prose,
            "页 7",
        )
        .unwrap();
        assert_exact_patch_contract(before, &output);
        let editor = inspect_native_citation_editor(&output.source);
        assert_eq!(editor.bindings.len(), 1);
        assert_eq!(editor.bindings[0].work, WORK);
        assert_eq!(editor.citations[0].form, Some(CitationForm::Prose));
        assert!(output.source.contains("后\r\n\n\n#bibliography"));
        let cite_end = editor.citations[0].range.utf16_to;
        assert_eq!(output.selection.to, cite_end);
        let empty = insert_native_citation(
            "",
            &crate::source_hash(""),
            0,
            WORK,
            "chen2026",
            None,
            CitationForm::Normal,
            "",
        )
        .unwrap();
        assert_exact_patch_contract("", &empty);
        assert_eq!(empty.patches.len(), 1);
    }

    #[test]
    fn work_reuse_suggestion_collisions_and_aliases_are_identity_scoped() {
        let before = bound_source("= Heading <hint>\n正文");
        assert_eq!(
            suggest_native_citation_key(&before, WORK, "changed", None).unwrap(),
            "chen2026"
        );
        let another = WORK.replace("8aa10c08", "9aa10c08");
        assert_eq!(
            suggest_native_citation_key(&before, &another, "chen2026", None).unwrap(),
            "chen2026-2"
        );
        assert_eq!(
            suggest_native_citation_key(&before, &another, "hint", None).unwrap(),
            "hint-2"
        );
        assert!(suggest_native_citation_key(&before, &another, "bad key", None).is_err());
        assert!(suggest_native_citation_key(&before, &another, "hint", Some("chen2026")).is_err());
        let aliases = before.replacen(
            "entries: (",
            &format!("entries: ((key: \"alias\", work: \"{WORK}\"),"),
            1,
        );
        assert!(suggest_native_citation_key(&aliases, WORK, "hint", None).is_err());
        assert_eq!(
            suggest_native_citation_key(&aliases, WORK, "hint", Some("alias")).unwrap(),
            "alias"
        );
        assert!(suggest_native_citation_key(&aliases, WORK, "hint", Some("newalias")).is_err());
        let conflicting = bound_source("= Heading <chen2026>\n正文");
        assert!(suggest_native_citation_key(&conflicting, WORK, "changed", None).is_err());
        assert!(
            suggest_native_citation_key(&conflicting, WORK, "changed", Some("chen2026")).is_err()
        );
    }

    #[test]
    fn static_rename_changes_only_certified_key_slots() {
        let before = bound_source(
            "= Heading <heading>\n@heading @chen2026[保留补充] #cite(<chen2026>, supplement: [chen2026])\n`chen2026` chen2026 #link(\"https://example.com/chen2026\")[chen2026]",
        );
        let output =
            rename_native_citation_key(&before, &crate::source_hash(&before), "chen2026", "li2027")
                .unwrap();
        assert_exact_patch_contract(&before, &output);
        assert!(
            output
                .source
                .contains("@li2027[保留补充] #cite(<li2027>, supplement: [chen2026])")
        );
        assert!(
            output
                .source
                .contains("`chen2026` chen2026 #link(\"https://example.com/chen2026\")[chen2026]")
        );
        assert_eq!(output.patches.len(), 3);
        let keyed = before.replacen("key: \"chen2026\"", "\"key\": \"chen2026\"", 1);
        assert!(
            rename_native_citation_key(&keyed, &crate::source_hash(&keyed), "chen2026", "li2027")
                .is_ok()
        );
    }

    #[test]
    fn unsafe_or_incomplete_rename_retains_every_source_byte() {
        let before = bound_source("#cite(<chen2026>)");
        for suffix in [
            "#include \"other.typ\"",
            "#let alias = cite",
            "#if true [@chen2026]",
            "#ref(label(\"chen2026\"))",
            "#f()",
            "#bibliography(\"other.bib\")",
            "#metadata(<chen2026>)",
            "#cite(..args)",
        ] {
            let source = format!("{before}\n{suffix}");
            assert!(
                rename_native_citation_key(
                    &source,
                    &crate::source_hash(&source),
                    "chen2026",
                    "next"
                )
                .is_err(),
                "{suffix}"
            );
        }
        let collision = bound_source("= Header <chen2026>\n#cite(<chen2026>)");
        assert!(
            rename_native_citation_key(
                &collision,
                &crate::source_hash(&collision),
                "chen2026",
                "next"
            )
            .is_err()
        );
    }

    #[test]
    fn unclassified_at_refs_and_source_targets_never_pretend_compile_evidence() {
        let before = bound_source(
            "= 章节 <章>\n@章 @chen2026 #cite(<chen2026>, form: none)\n#ref(<章>, form: \"page\")",
        );
        let editor = inspect_native_citation_editor(&before);
        assert_eq!(editor.citations.len(), 1);
        assert_eq!(editor.citations[0].form, Some(CitationForm::None));
        assert_eq!(
            editor
                .cross_references
                .iter()
                .filter(|r| r.kind == "unclassified")
                .count(),
            2
        );
        let target = editor.targets.iter().find(|t| t.label == "章").unwrap();
        assert_eq!(target.kind, "heading");
        assert!(target.unique && target.source_only);
        assert!(!editor.targets.iter().any(|t| t.label == "chen2026"));
    }

    #[test]
    fn cross_reference_insert_and_unicode_label_rename_preserve_supplement() {
        let before = "= 章节 <章>\n参见 @章[补充] #ref(<章>) #link(<章>)[显示文字]\n";
        let output =
            rename_native_label(before, &crate::source_hash(before), "章", "新章").unwrap();
        assert_exact_patch_contract(before, &output);
        assert!(
            output
                .source
                .contains("@新章[补充] #ref(<新章>) #link(<新章>)[显示文字]")
        );
        assert_eq!(output.patches.len(), 4);
        let inserted = insert_native_cross_reference(
            before,
            &crate::source_hash(before),
            before.len(),
            "章",
            true,
        )
        .unwrap();
        assert!(inserted.source.ends_with("#ref(<章>, form: \"page\")"));
        let duplicate = format!("{before}= Again <章>\n");
        assert!(
            rename_native_label(&duplicate, &crate::source_hash(&duplicate), "章", "新章").is_err()
        );
        for invalid_marker in ["new.", "new:"] {
            assert!(
                rename_native_label(before, &crate::source_hash(before), "章", invalid_marker)
                    .is_err()
            );
        }
        let external = "= H <old>\n#cite(<old>)\n#bibliography(\"weftext.citations.bib\")";
        assert!(
            rename_native_label(external, &crate::source_hash(external), "old", "new").is_err()
        );
    }

    #[test]
    fn stale_invalid_dynamic_and_shadowed_citation_writers_reject() {
        let before = bound_source("#cite(<chen2026>)");
        let editor = inspect_native_citation_editor(&before);
        assert!(
            update_native_citation(
                &before,
                "old hash",
                &editor.citations[0].range,
                CitationForm::Author,
                "2"
            )
            .is_err()
        );
        let dynamic = bound_source("#cite(key, supplement: f())");
        assert!(!inspect_native_citation_editor(&dynamic).citations[0].editable);
        let loop_source = "#for cite in (x,) [#cite(<chen2026>)]";
        assert!(proven_static_cites(loop_source).is_empty());
        assert!(!inspect_native_citation_editor(loop_source).citations[0].editable);
        for source in [
            "#show \"x\": cite(<chen2026>)\nx x",
            "#set heading(supplement: [#cite(<chen2026>)])\n= Head",
            "#let twice(x)=x+x\n#twice([#cite(<chen2026>)])",
        ] {
            assert!(
                !inspect_native_citation_editor(source).citations[0].editable,
                "{source}"
            );
        }
        let escaped = "前\\#后";
        assert!(
            insert_native_citation(
                escaped,
                &crate::source_hash(escaped),
                "前\\".len(),
                WORK,
                "key",
                None,
                CitationForm::Normal,
                ""
            )
            .is_err()
        );
        let external = "正文\n#bibliography(\"external.bib\")";
        assert!(
            insert_native_citation(
                external,
                &crate::source_hash(external),
                external.len(),
                WORK,
                "key",
                None,
                CitationForm::Normal,
                ""
            )
            .is_err()
        );
    }

    #[test]
    fn all_forms_with_encoded_supplements_compile_on_official_html_and_paged_targets() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("weftext.citations.bib"),"@article{chen2026, author={Chen, Lin}, title={Native Citation Test}, year={2026}, journal={Example Journal}}\n").unwrap();
        let engine = crate::Engine::new_with_system_fonts(false).unwrap();
        let mut body = String::new();
        for form in [
            CitationForm::Normal,
            CitationForm::Prose,
            CitationForm::Author,
            CitationForm::Year,
            CitationForm::Full,
            CitationForm::None,
        ] {
            body.push_str(&format!(
                "#cite(<chen2026>, form: {}, supplement: [#{}])\n\n",
                form.native(),
                crate::encode_string("中文 ] #panic(\"injected\") 页2").unwrap()
            ));
        }
        let source = bound_source(&body);
        let path = dir.path().join("citation.typ");
        let html = engine.render(&source, &path, dir.path()).unwrap();
        assert!(html.ok, "{:?}", html.diagnostics);
        assert!(html.html.contains("panic"));
        let paged = engine.render_paged(&source, &path, dir.path()).unwrap();
        assert!(paged.ok, "{:?}", paged.diagnostics);
        assert!(paged.pdf_base64.is_some());
    }

    #[test]
    fn new_label_and_reference_are_one_draft_edit_for_a_direct_target() {
        let before = "= 章节\n\n正文😀\n";
        let target = crate::inspect_editor(before)
            .structures
            .into_iter()
            .find(|s| s.kind == "heading")
            .unwrap()
            .range;
        let output = insert_native_cross_reference_with_label(
            before,
            &crate::source_hash(before),
            before.len(),
            &target,
            "章节标签",
            false,
        )
        .unwrap();
        assert_exact_patch_contract(before, &output);
        assert_eq!(output.patches.len(), 2);
        assert!(output.source.starts_with("= 章节 <章节标签>\n\n正文😀\n"));
        assert!(output.source.ends_with("#ref(<章节标签>)"));
        let mut stale = target.clone();
        stale.raw.push('!');
        assert!(
            insert_native_cross_reference_with_label(
                before,
                &crate::source_hash(before),
                before.len(),
                &stale,
                "new",
                false
            )
            .is_err()
        );
        let already = "= 章节 <old>\n正文\n";
        let range = crate::inspect_editor(already).structures[0].range.clone();
        assert!(
            insert_native_cross_reference_with_label(
                already,
                &crate::source_hash(already),
                already.len(),
                &range,
                "new",
                false
            )
            .is_err()
        );
    }

    #[test]
    fn plain_native_link_insert_and_text_update_preserve_the_target_and_author_bytes() {
        let before = "前😀后";
        let output = insert_native_link(
            before,
            &crate::source_hash(before),
            "前😀".len(),
            "weftext-node://library/node#片段",
            "文字 ] #panic(\"x\")",
        )
        .unwrap();
        assert_exact_patch_contract(before, &output);
        let info = crate::inspect(&output.source);
        let link = info
            .links
            .iter()
            .find(|l| l.kind == "unverified-call")
            .unwrap();
        let range = crate::editor::source_range(&output.source, link.from..link.to);
        let updated =
            update_native_link_text(&output.source, &output.source_hash, &range, "新名字😀")
                .unwrap();
        assert_exact_patch_contract(&output.source, &updated);
        assert!(
            updated
                .source
                .contains("link(\"weftext-node://library/node#片段\")[#\"新名字😀\"]")
        );
        assert_eq!(updated.patches.len(), 1);
        let complex = "#link(\"https://example.com\")[*格式文字*]";
        let info = crate::inspect(complex);
        let link = &info.links[0];
        let range = crate::editor::source_range(complex, link.from..link.to);
        assert!(
            update_native_link_text(complex, &crate::source_hash(complex), &range, "flat").is_err()
        );
        let comments = "#link(\"https://example.com\", /* target note */ [plain]) suffix";
        let info = crate::inspect(comments);
        let link = &info.links[0];
        let range = crate::editor::source_range(comments, link.from..link.to);
        let updated =
            update_native_link_text(comments, &crate::source_hash(comments), &range, "new")
                .unwrap();
        assert!(updated.source.contains("/* target note */ [#\"new\"]"));
        let labeled = "= Heading <section>\n正文\n";
        let cross = insert_native_cross_link(
            labeled,
            &crate::source_hash(labeled),
            labeled.len(),
            "section",
            "显示 ] #panic(\"x\")",
        )
        .unwrap();
        assert_exact_patch_contract(labeled, &cross);
        assert!(
            cross
                .source
                .contains("#link(<section>)[#\"显示 ] #panic(\\\"x\\\")\"]")
        );
        assert!(
            insert_native_cross_link(
                labeled,
                &crate::source_hash(labeled),
                labeled.len(),
                "missing",
                "display"
            )
            .is_err()
        );
    }
}
