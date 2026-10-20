use crate::{
    EditOutput, EngineError, SourceRange,
    editor::{finish, source_range},
    source_hash,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use typst_syntax::{
    LinkedNode, Source,
    ast::{self, AstNode},
};

/// Frozen ordinary Typst module supplied by the Core environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleModule {
    pub path: String,
    pub source: String,
    pub enabled: bool,
    #[serde(default = "default_entry")]
    pub entry_function: String,
}
fn default_entry() -> String {
    "library-style".into()
}
pub const DEFAULT_LIBRARY_STYLE: &str = include_str!("../resources/default-style.typ");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleControl {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub value: String,
    pub editable: bool,
    pub reason: Option<String>,
    pub range: Option<SourceRange>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleInspection {
    pub source_hash: String,
    pub syntax_valid: bool,
    pub controls: Vec<StyleControl>,
}
const SLOTS: [(&str, &str); 7] = [
    ("text.font", "正文字体（原生参数）"),
    ("text.size", "正文字号（pt）"),
    ("text.lang", "文档排版语言（原生 text.lang）"),
    ("text.region", "文档排版地区（原生 text.region）"),
    ("raw.font", "代码字体（原生参数）"),
    ("para.leading", "段落行距（em）"),
    ("table.stroke", "表格线宽（pt）"),
];

pub fn inspect_library_style(source: &str) -> StyleInspection {
    let parsed = Source::detached(source);
    let syntax_valid = parsed.root().errors_and_warnings().0.is_empty();
    let mut slots: BTreeMap<String, Vec<SourceRange>> = BTreeMap::new();
    let mut entries = 0;
    fn find_entry<'a>(
        node: &LinkedNode<'a>,
        parsed: &'a Source,
        source: &str,
        slots: &mut BTreeMap<String, Vec<SourceRange>>,
        entries: &mut usize,
    ) {
        if let Some(binding) = node.get().cast::<ast::LetBinding>()
            && binding
                .kind()
                .bindings()
                .iter()
                .any(|i| i.get() == "library-style")
        {
            *entries += 1;
            if let Some(closure) = binding
                .init()
                .and_then(|e| e.to_untyped().cast::<ast::Closure>())
                && let Some(block) = closure.body().to_untyped().cast::<ast::CodeBlock>()
                && let Some(body) = parsed.find(block.body().to_untyped().span())
            {
                for expression in body.children() {
                    collect(&expression, source, "", slots);
                }
            }
        }
    }
    fn collect(
        node: &LinkedNode<'_>,
        source: &str,
        selector: &str,
        slots: &mut BTreeMap<String, Vec<SourceRange>>,
    ) {
        if let Some(show) = node.get().cast::<ast::ShowRule>() {
            let Some(selector) = show
                .selector()
                .and_then(|e| e.to_untyped().cast::<ast::Ident>())
                .map(|i| i.get().to_string())
            else {
                return;
            };
            // Only an unconditional direct show-set is a panel slot.
            if show
                .transform()
                .to_untyped()
                .cast::<ast::SetRule>()
                .is_some()
            {
                for n in node.children() {
                    collect(&n, source, &selector, slots)
                }
            }
            return;
        }
        if let Some(set) = node.get().cast::<ast::SetRule>()
            && set.condition().is_none()
            && let Some(target) = set.target().to_untyped().cast::<ast::Ident>()
        {
            let target = target.get().as_str();
            for arg in set.args().items() {
                if let ast::Arg::Named(n) = arg {
                    let key = if selector == "raw" && target == "text" && n.name().get() == "font" {
                        "raw.font".into()
                    } else if selector.is_empty() {
                        format!(
                            "{}.{}",
                            if target == "par" { "para" } else { target },
                            n.name().get()
                        )
                    } else {
                        continue;
                    };
                    if SLOTS.iter().any(|(k, _)| *k == key)
                        && let Some(value) = node
                            .children()
                            .find(|n| n.get().cast::<ast::Args>().is_some())
                            .and_then(|args| find_span(&args, Some(n.expr().to_untyped().span())))
                    {
                        slots
                            .entry(key)
                            .or_default()
                            .push(source_range(source, value.range()));
                    }
                }
            }
        }
        // A nested block/function is its own native scope, never a default slot.
    }
    fn find_span<'a>(
        node: &LinkedNode<'a>,
        span: Option<typst_syntax::Span>,
    ) -> Option<LinkedNode<'a>> {
        if Some(node.get().span()) == span {
            return Some(node.clone());
        }
        node.children().find_map(|n| find_span(&n, span))
    }
    if syntax_valid {
        for child in LinkedNode::new(parsed.root()).children() {
            find_entry(&child, &parsed, source, &mut slots, &mut entries);
        }
    }
    let controls = SLOTS
        .iter()
        .map(|(key, label)| {
            let ranges = slots.get(*key);
            let range = ranges
                .filter(|r| r.len() == 1)
                .and_then(|r| r.first())
                .cloned();
            let binding = match *key {
                "raw.font" => {
                    crate::standard_binding_available(source, "raw")
                        && crate::standard_binding_available(source, "text")
                }
                "para.leading" => crate::standard_binding_available(source, "par"),
                "table.stroke" => crate::standard_binding_available(source, "table"),
                _ => crate::standard_binding_available(source, "text"),
            };
            let editable = syntax_valid
                && entries == 1
                && binding
                && range
                    .as_ref()
                    .is_some_and(|r| slot_value_valid(key, &r.raw));
            StyleControl {
                key: (*key).into(),
                label: (*label).into(),
                kind: "expression".into(),
                value: range.as_ref().map(|r| r.raw.clone()).unwrap_or_default(),
                editable,
                reason: (!editable).then(|| {
                    if !syntax_valid {
                        "syntax-invalid"
                    } else if entries != 1 {
                        "missing-or-ambiguous-entry"
                    } else if !binding {
                        "unverified-standard-binding"
                    } else if range.is_none() {
                        "missing-or-ambiguous-slot"
                    } else {
                        "computed-or-unsupported-value"
                    }
                    .into()
                }),
                range,
            }
        })
        .collect();
    StyleInspection {
        source_hash: source_hash(source),
        syntax_valid,
        controls,
    }
}

fn slot_value_valid(key: &str, value: &str) -> bool {
    let source = format!("#let value = {value}");
    let parsed = Source::detached(&source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return false;
    }
    let Some(binding) = parsed
        .root()
        .children()
        .find_map(|n| n.cast::<ast::LetBinding>())
    else {
        return false;
    };
    let Some(expr) = binding.init() else {
        return false;
    };
    let Some(range) = parsed.find(expr.to_untyped().span()).map(|n| n.range()) else {
        return false;
    };
    if source[range] != *value {
        return false;
    }
    match key {
        "text.font"|"raw.font"=>expr.to_untyped().cast::<ast::Str>().is_some()||expr.to_untyped().cast::<ast::Array>().is_some_and(|a|a.items().all(|i|matches!(i,ast::ArrayItem::Pos(e) if e.to_untyped().cast::<ast::Str>().is_some()))),
        "text.lang"=>expr.to_untyped().cast::<ast::Str>().is_some(),
        "text.region"=>value=="none"||expr.to_untyped().cast::<ast::Str>().is_some(),
        "text.size"|"table.stroke"=>expr.to_untyped().cast::<ast::Numeric>().is_some_and(|n|{let(value,unit)=n.get();value.is_finite()&&value>0.&&unit==ast::Unit::Pt}),
        "para.leading"=>expr.to_untyped().cast::<ast::Numeric>().is_some_and(|n|{let(value,unit)=n.get();value.is_finite()&&value>=0.&&unit==ast::Unit::Em}),
        _=>false,
    }
}

pub fn apply_library_style_changes(
    source: &str,
    expected_hash: &str,
    changes: &BTreeMap<String, String>,
) -> Result<EditOutput, EngineError> {
    if source_hash(source) != expected_hash {
        return Err(EngineError("stale-style-source".into()));
    }
    let inspection = inspect_library_style(source);
    let mut patches = vec![];
    for (key, value) in changes {
        let control = inspection
            .controls
            .iter()
            .find(|c| &c.key == key && c.editable)
            .ok_or_else(|| EngineError(format!("样式参数不可无损编辑：{key}")))?;
        if !slot_value_valid(key, value) {
            return Err(EngineError(format!("非法原生样式参数：{key}")));
        }
        let range = control.range.as_ref().unwrap();
        patches.push(crate::SourcePatch {
            from: range.from,
            to: range.to,
            utf16_from: range.utf16_from,
            utf16_to: range.utf16_to,
            expected: range.raw.clone(),
            insert: value.clone(),
        });
    }
    finish(source, patches)
}

/// A bounded inverse profile for the library wrapper. Unknown native programs
/// remain fully renderable, with source editing instead of guessed rich writes.
pub fn style_allows_rich_editing(style: &StyleModule) -> bool {
    if !style.enabled {
        return true;
    }
    let parsed = Source::detached(&style.source);
    if !parsed.root().errors_and_warnings().0.is_empty() {
        return false;
    }
    let bindings: Vec<_> = parsed
        .root()
        .children()
        .filter_map(|n| n.cast::<ast::LetBinding>())
        .collect();
    if bindings.len() != 1 {
        return false;
    }
    let binding = bindings[0];
    if !binding
        .kind()
        .bindings()
        .iter()
        .any(|i| i.get().as_str() == style.entry_function)
    {
        return false;
    }
    let Some(closure) = binding
        .init()
        .and_then(|e| e.to_untyped().cast::<ast::Closure>())
    else {
        return false;
    };
    let parameters: Vec<_> = closure.params().children().collect();
    let [ast::Param::Pos(ast::Pattern::Normal(ast::Expr::Ident(body_name)))] =
        parameters.as_slice()
    else {
        return false;
    };
    let Some(block) = closure.body().to_untyped().cast::<ast::CodeBlock>() else {
        return false;
    };
    let expressions: Vec<_> = block.body().exprs().collect();
    let Some((last, settings)) = expressions.split_last() else {
        return false;
    };
    if !last
        .to_untyped()
        .cast::<ast::Ident>()
        .is_some_and(|i| i.get() == body_name.get())
    {
        return false;
    }
    fn native_set(node: &typst_syntax::SyntaxNode, source: &str) -> bool {
        let Some(set) = node.cast::<ast::SetRule>() else {
            return false;
        };
        if set.condition().is_some() {
            return false;
        }
        let Some(target) = set.target().to_untyped().cast::<ast::Ident>() else {
            return false;
        };
        let name = target.get().as_str();
        if !matches!(
            name,
            "text" | "par" | "heading" | "table" | "list" | "enum" | "terms" | "raw"
        ) || !crate::standard_binding_available(source, name)
        {
            return false;
        }
        if name == "text"
            && set
                .args()
                .items()
                .any(|a| matches!(a,ast::Arg::Named(n) if n.name().get()=="case"))
        {
            return false;
        }
        if name=="heading"&&set.args().items().any(|a|matches!(a,ast::Arg::Named(n) if matches!(n.name().get().as_str(),"level"|"depth"|"offset"))){return false;}
        true
    }
    settings.iter().all(|e| {
        if e.to_untyped().cast::<ast::SetRule>().is_some() {
            return native_set(e.to_untyped(), &style.source);
        }
        if let Some(show) = e.to_untyped().cast::<ast::ShowRule>() {
            if let Some(field) = show
                .selector()
                .and_then(|e| e.to_untyped().cast::<ast::FieldAccess>())
            {
                return field
                    .target()
                    .to_untyped()
                    .cast::<ast::Ident>()
                    .is_some_and(|i| i.get() == "math")
                    && field.field().get() == "equation"
                    && crate::standard_binding_available(&style.source, "math")
                    && native_set(show.transform().to_untyped(), &style.source);
            }
            let Some(selector) = show
                .selector()
                .and_then(|e| e.to_untyped().cast::<ast::Ident>())
            else {
                return false;
            };
            let selector = selector.get().as_str();
            return matches!(
                selector,
                "text"
                    | "raw"
                    | "heading"
                    | "strong"
                    | "emph"
                    | "strike"
                    | "list"
                    | "enum"
                    | "terms"
                    | "table"
            ) && crate::standard_binding_available(&style.source, selector)
                && native_set(show.transform().to_untyped(), &style.source);
        }
        false
    })
}
