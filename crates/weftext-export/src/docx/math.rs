//! Presentation MathML from the frozen HTML tree -> editable Office Math.
use super::xml::{Element, Xml};
use unicode_normalization::char::decompose_compatible;
use weftext_engine::RenderNode;
use weftext_messages::{ErrorDescription, UiMessage};

#[path = "math_import.rs"]
mod omml;
pub(super) use omml::import_equation;

pub(super) fn convert(node: &RenderNode) -> Result<Element, ErrorDescription> {
    let mut math = Element::new("m:oMath");
    math.children = content(node)?;
    provenance(&mut math, node);
    Ok(math)
}
/// MathML's authored/generated width does not have an exact OMML space-run
/// counterpart in this bounded converter. Preserve editable math and disclose
/// the independent spacing loss instead of converting the equation to prose.
pub(super) fn spacing_losses(node: &RenderNode) -> Vec<ErrorDescription> {
    fn walk(node: &RenderNode, out: &mut Vec<ErrorDescription>) {
        if node.tag.as_deref() == Some("mspace") {
            let explanation = match node.attrs.get("width") {
                Some(width) => UiMessage::error_export_docx_math_spacing_width_approximate(
                    weftext_messages::params::ErrorExportDocxMathSpacingWidthApproximate {
                        width: width.clone(),
                    },
                ),
                None => UiMessage::error_export_docx_math_spacing_width_unspecified(),
            };
            out.push(explanation.into());
        }
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = vec![];
    walk(node, &mut out);
    out
}
/// Native producer provenance belongs to mathematical objects as well as text.
/// Consumers may attach only an exact, unique source range to this object; the
/// source bytes are not a string to be searched in the displayed math glyphs.
fn provenance(element: &mut Element, node: &RenderNode) {
    if let Some(origin) = &node.origin
        && origin.is_main
        && origin.from < origin.to
        && origin.to - origin.from == origin.raw.len()
    {
        element.source = Some((origin.from, origin.raw.clone()));
    }
}
fn wrap(name: &str, nodes: Vec<Xml>) -> Element {
    let mut e = Element::new(name);
    e.children = nodes;
    e
}
fn content(node: &RenderNode) -> Result<Vec<Xml>, ErrorDescription> {
    let tag = node.tag.as_deref().unwrap_or("");
    if node.kind == "tag" || matches!(tag, "annotation" | "annotation-xml") {
        return Ok(vec![]);
    }
    if node.kind == "text" {
        return tokens(node.text.as_deref().unwrap_or(""), None, false, "mi");
    }
    let children: Vec<_> = node
        .children
        .iter()
        .filter(|c| {
            c.kind != "tag"
                && c.tag
                    .as_deref()
                    .is_none_or(|t| !matches!(t, "annotation" | "annotation-xml"))
        })
        .collect();
    let argument = |index: usize| -> Result<Vec<Xml>, ErrorDescription> {
        children
            .get(index)
            .ok_or_else(|| {
                ErrorDescription::new(UiMessage::error_export_docx_math_argument_missing(
                    weftext_messages::params::ErrorExportDocxMathArgumentMissing {
                        element: tag.into(),
                    },
                ))
            })
            .and_then(|n| content(n))
    };
    let argument_element = |name: &str, index: usize| -> Result<Element, ErrorDescription> {
        let mut element = wrap(name, argument(index)?);
        if let Some(node) = children.get(index) {
            provenance(&mut element, node);
        }
        Ok(element)
    };
    // Native MathML Core represents stretchy delimiters using an mrow with
    // explicit fence operators. Retain that mathematical structure in OMML,
    // particularly a cases matrix's one-sided left brace.
    if tag == "mrow" && children.len() >= 2 {
        let native_fence = node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "math-fence");
        let fence = |node: &RenderNode| {
            node.tag.as_deref() == Some("mo")
                && (native_fence
                    || node
                        .attrs
                        .get("fence")
                        .is_some_and(|v| v.eq_ignore_ascii_case("true")))
        };
        let open = plain(children[0]);
        let last = children.len() - 1;
        let close = plain(children[last]);
        let paired = fence(children[0])
            && fence(children[last])
            && matches!(
                (open.as_str(), close.as_str()),
                ("(", ")")
                    | ("[", "]")
                    | ("{", "}")
                    | ("|", "|")
                    | ("‖", "‖")
                    | ("⌈", "⌉")
                    | ("⌊", "⌋")
                    | ("⟨", "⟩")
            );
        let cases = fence(children[0])
            && open == "{"
            && children.len() == 2
            && children[1].tag.as_deref() == Some("mtable");
        if paired || cases {
            let mut e = Element::new("m:d");
            let mut pr = Element::new("m:dPr");
            pr.push(Element::new("m:begChr").attr("m:val", &open));
            pr.push(
                Element::new("m:endChr").attr("m:val", if paired { close.as_str() } else { "" }),
            );
            e.push(pr);
            let mut body = vec![];
            for child in &children[1..if paired { last } else { children.len() }] {
                body.extend(content(child)?);
            }
            let mut argument = wrap("m:e", body);
            if children.len() == if paired { 3 } else { 2 } {
                provenance(&mut argument, children[1]);
            }
            e.push(argument);
            provenance(&mut e, node);
            return Ok(vec![Xml::Element(e)]);
        }
    }
    let result = match tag {
        "math" | "mrow" | "semantics" => {
            let mut out = vec![];
            for c in children {
                out.extend(content(c)?);
            }
            return Ok(out);
        }
        "mi" | "mn" | "mo" | "mtext" | "ms" => {
            let mut result = tokens(
                &plain(node),
                node.attrs.get("mathvariant").map(String::as_str),
                tag == "mtext" || tag == "ms",
                tag,
            )?;
            // A multi-style token can create several Office runs. Only a
            // single native producer is an exact element mapping; splitting a
            // quoted string or macro's source by output glyphs would be false.
            if result.len() == 1
                && let Some(element) = result[0].element_mut()
            {
                provenance(element, node);
            }
            return Ok(result);
        }
        "mspace" => return tokens(" ", Some("normal"), true, "mtext"),
        "mfrac" if children.len() == 2 => {
            let mut e = Element::new("m:f");
            if let Some(thickness) = node.attrs.get("linethickness") {
                if !matches!(thickness.as_str(), "0" | "0px" | "0em" | "0pt") {
                    return Err(
                        UiMessage::error_export_docx_math_fraction_line_width_unsupported().into(),
                    );
                }
                let mut pr = Element::new("m:fPr");
                pr.push(Element::new("m:type").attr("m:val", "noBar"));
                e.push(pr);
            }
            e.push(argument_element("m:num", 0)?);
            e.push(argument_element("m:den", 1)?);
            e
        }
        "msub" | "msup" if children.len() == 2 => {
            let mut e = Element::new(if tag == "msub" { "m:sSub" } else { "m:sSup" });
            e.push(argument_element("m:e", 0)?);
            e.push(argument_element(
                if tag == "msub" { "m:sub" } else { "m:sup" },
                1,
            )?);
            e
        }
        "msubsup" if children.len() == 3 => {
            let mut e = Element::new("m:sSubSup");
            e.push(argument_element("m:e", 0)?);
            e.push(argument_element("m:sub", 1)?);
            e.push(argument_element("m:sup", 2)?);
            e
        }
        "munder" | "mover" if children.len() == 2 => {
            if node
                .attrs
                .get(if tag == "munder" {
                    "accentunder"
                } else {
                    "accent"
                })
                .is_some_and(|v| v.eq_ignore_ascii_case("true"))
            {
                return Err(UiMessage::error_export_docx_math_accent_limit_unsupported().into());
            }
            let mut e = Element::new(if tag == "munder" {
                "m:limLow"
            } else {
                "m:limUpp"
            });
            e.push(argument_element("m:e", 0)?);
            e.push(argument_element("m:lim", 1)?);
            e
        }
        "munderover" if children.len() == 3 => {
            if ["accent", "accentunder"].iter().any(|attr| {
                node.attrs
                    .get(*attr)
                    .is_some_and(|v| v.eq_ignore_ascii_case("true"))
            }) {
                return Err(
                    UiMessage::error_export_docx_math_accent_lower_upper_unsupported().into(),
                );
            }
            let mut lower = Element::new("m:limLow");
            lower.push(argument_element("m:e", 0)?);
            lower.push(argument_element("m:lim", 1)?);
            let mut upper = Element::new("m:limUpp");
            upper.push(wrap("m:e", vec![Xml::Element(lower)]));
            upper.push(argument_element("m:lim", 2)?);
            upper
        }
        "msqrt" => {
            let mut e = Element::new("m:rad");
            let mut pr = Element::new("m:radPr");
            pr.push(Element::new("m:degHide").attr("m:val", "1"));
            e.push(pr);
            e.push(Element::new("m:deg"));
            let mut inner = vec![];
            for c in children {
                inner.extend(content(c)?);
            }
            e.push(wrap("m:e", inner));
            e
        }
        "mroot" if children.len() == 2 => {
            let mut e = Element::new("m:rad");
            e.push(argument_element("m:deg", 1)?);
            e.push(argument_element("m:e", 0)?);
            e
        }
        "mfenced" => {
            let mut e = Element::new("m:d");
            let mut pr = Element::new("m:dPr");
            pr.push(Element::new("m:begChr").attr(
                "m:val",
                node.attrs.get("open").map(String::as_str).unwrap_or("("),
            ));
            pr.push(Element::new("m:endChr").attr(
                "m:val",
                node.attrs.get("close").map(String::as_str).unwrap_or(")"),
            ));
            let separator = node
                .attrs
                .get("separators")
                .map(String::as_str)
                .unwrap_or(",");
            if separator.chars().count() > 1 {
                return Err(
                    UiMessage::error_export_docx_math_multiple_separators_unsupported().into(),
                );
            }
            pr.push(Element::new("m:sepChr").attr("m:val", separator));
            e.push(pr);
            for c in children {
                e.push(wrap("m:e", content(c)?));
            }
            e
        }
        "mtable" => {
            let mut e = Element::new("m:m");
            for row in children {
                if row.tag.as_deref() != Some("mtr") {
                    return Err(UiMessage::error_export_docx_math_matrix_row_unsupported().into());
                }
                let mut mr = Element::new("m:mr");
                for cell in &row.children {
                    if cell.tag.as_deref() != Some("mtd") {
                        return Err(
                            UiMessage::error_export_docx_math_matrix_cell_unsupported().into()
                        );
                    }
                    let mut nodes = vec![];
                    for c in &cell.children {
                        nodes.extend(content(c)?);
                    }
                    mr.push(wrap("m:e", nodes));
                }
                e.push(mr);
            }
            e
        }
        _ => {
            return Err(UiMessage::error_export_docx_math_structure_unsupported(
                weftext_messages::params::ErrorExportDocxMathStructureUnsupported {
                    element: tag.into(),
                },
            )
            .into());
        }
    };
    let mut result = result;
    provenance(&mut result, node);
    Ok(vec![Xml::Element(result)])
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct MathStyle {
    script: &'static str,
    style: &'static str,
}
fn variant(name: &str) -> Result<MathStyle, ErrorDescription> {
    let (script, style) = match name {
        "normal" => ("roman", "p"),
        "bold" => ("roman", "b"),
        "italic" => ("roman", "i"),
        "bold-italic" => ("roman", "bi"),
        "script" => ("script", "p"),
        "bold-script" => ("script", "b"),
        "fraktur" => ("fraktur", "p"),
        "bold-fraktur" => ("fraktur", "b"),
        "double-struck" => ("double-struck", "p"),
        "sans-serif" => ("sans-serif", "p"),
        "bold-sans-serif" => ("sans-serif", "b"),
        "sans-serif-italic" => ("sans-serif", "i"),
        "sans-serif-bold-italic" => ("sans-serif", "bi"),
        "monospace" => ("monospace", "p"),
        _ => {
            return Err(UiMessage::error_export_docx_math_variant_unsupported(
                weftext_messages::params::ErrorExportDocxMathVariantUnsupported {
                    variant: name.into(),
                },
            )
            .into());
        }
    };
    Ok(MathStyle { script, style })
}
/// Unicode mathematical alphabets carry semantic style distinctions. Convert
/// just those characters to base letters + OMML scr/sty, never normalize prose.
/// ECMA-376 22.1.2.91/94; Unicode 16 chapter 22.2.1. Greek symbol variants are
/// preserved explicitly because recursive compatibility decomposition would
/// otherwise fold e.g. theta-symbol into the distinct ordinary theta letter.
pub(super) fn mathematical_letter(c: char) -> Option<(char, MathStyle)> {
    let n = c as u32;
    let latin = [
        (0x1D400, "roman", "b"),
        (0x1D434, "roman", "i"),
        (0x1D468, "roman", "bi"),
        (0x1D49C, "script", "p"),
        (0x1D4D0, "script", "b"),
        (0x1D504, "fraktur", "p"),
        (0x1D538, "double-struck", "p"),
        (0x1D56C, "fraktur", "b"),
        (0x1D5A0, "sans-serif", "p"),
        (0x1D5D4, "sans-serif", "b"),
        (0x1D608, "sans-serif", "i"),
        (0x1D63C, "sans-serif", "bi"),
        (0x1D670, "monospace", "p"),
    ];
    let mut spec = latin
        .iter()
        .find(|(start, _, _)| n >= *start && n < *start + 52)
        .map(|(_, script, style)| (*script, *style));
    let mut greek = None;
    for (start, script, style) in [
        (0x1D6A8, "roman", "b"),
        (0x1D6E2, "roman", "i"),
        (0x1D71C, "roman", "bi"),
        (0x1D756, "sans-serif", "b"),
        (0x1D790, "sans-serif", "bi"),
    ] {
        if n >= start && n < start + 58 {
            spec = Some((script, style));
            greek = Some(n - start);
            break;
        }
    }
    for (start, script, style) in [
        (0x1D7CE, "roman", "b"),
        (0x1D7D8, "double-struck", "p"),
        (0x1D7E2, "sans-serif", "p"),
        (0x1D7EC, "sans-serif", "b"),
        (0x1D7F6, "monospace", "p"),
    ] {
        if n >= start && n < start + 10 {
            spec = Some((script, style));
            break;
        }
    }
    if matches!(n, 0x1D6A4 | 0x1D6A5) {
        spec = Some(("roman", "i"));
    }
    if matches!(n, 0x1D7CA | 0x1D7CB) {
        spec = Some(("roman", "b"));
    }
    let legacy = match c {
        'ℎ' => Some(('h', "roman", "i")),
        'ℬ' => Some(('B', "script", "p")),
        'ℰ' => Some(('E', "script", "p")),
        'ℱ' => Some(('F', "script", "p")),
        'ℋ' => Some(('H', "script", "p")),
        'ℐ' => Some(('I', "script", "p")),
        'ℒ' => Some(('L', "script", "p")),
        'ℳ' => Some(('M', "script", "p")),
        'ℛ' => Some(('R', "script", "p")),
        'ℯ' => Some(('e', "script", "p")),
        'ℊ' => Some(('g', "script", "p")),
        'ℴ' => Some(('o', "script", "p")),
        'ℭ' => Some(('C', "fraktur", "p")),
        'ℌ' => Some(('H', "fraktur", "p")),
        'ℑ' => Some(('I', "fraktur", "p")),
        'ℜ' => Some(('R', "fraktur", "p")),
        'ℨ' => Some(('Z', "fraktur", "p")),
        'ℂ' => Some(('C', "double-struck", "p")),
        'ℍ' => Some(('H', "double-struck", "p")),
        'ℕ' => Some(('N', "double-struck", "p")),
        'ℙ' => Some(('P', "double-struck", "p")),
        'ℚ' => Some(('Q', "double-struck", "p")),
        'ℝ' => Some(('R', "double-struck", "p")),
        'ℤ' => Some(('Z', "double-struck", "p")),
        _ => None,
    };
    if let Some((base, script, style)) = legacy {
        return Some((base, MathStyle { script, style }));
    }
    let (script, style) = spec?;
    let mut decomposed = vec![];
    decompose_compatible(c, |c| decomposed.push(c));
    if decomposed.len() != 1 || decomposed[0] == c {
        return None;
    }
    let base = match greek {
        Some(17) => 'ϴ',
        Some(52) => 'ϵ',
        Some(53) => 'ϑ',
        Some(54) => 'ϰ',
        Some(55) => 'ϕ',
        Some(56) => 'ϱ',
        Some(57) => 'ϖ',
        _ => decomposed[0],
    };
    Some((base, MathStyle { script, style }))
}
fn tokens(
    text: &str,
    mathvariant: Option<&str>,
    normal_text: bool,
    tag: &str,
) -> Result<Vec<Xml>, ErrorDescription> {
    let explicit = mathvariant.map(variant).transpose()?;
    let mut runs: Vec<(MathStyle, String)> = vec![];
    for c in text.chars() {
        let (base, encoded) = mathematical_letter(c)
            .map(|(c, s)| (c, Some(s)))
            .unwrap_or((c, None));
        let default = MathStyle {
            script: "roman",
            style: if !normal_text && tag == "mi" && c.is_alphabetic() {
                "i"
            } else {
                "p"
            },
        };
        let style = explicit.or(encoded).unwrap_or(default);
        if let Some((previous, group)) = runs.last_mut().filter(|(s, _)| *s == style) {
            let _ = previous;
            group.push(base);
        } else {
            runs.push((style, base.to_string()));
        }
    }
    Ok(runs
        .into_iter()
        .map(|(style, text)| Xml::Element(token(&text, style, normal_text)))
        .collect())
}
fn token(text: &str, style: MathStyle, normal: bool) -> Element {
    let mut r = Element::new("m:r");
    let mut pr = Element::new("m:rPr");
    if normal {
        pr.push(Element::new("m:nor"));
    }
    pr.push(Element::new("m:scr").attr("m:val", style.script));
    pr.push(Element::new("m:sty").attr("m:val", style.style));
    r.push(pr);
    let mut t = Element::new("m:t").attr("xml:space", "preserve");
    t.children.push(Xml::Text(text.into()));
    r.push(t);
    r
}
pub(super) fn plain(node: &RenderNode) -> String {
    if node.kind == "tag" || matches!(node.tag.as_deref(), Some("annotation" | "annotation-xml")) {
        return String::new();
    }
    let mut s = node.text.clone().unwrap_or_default();
    for c in &node.children {
        s.push_str(&plain(c));
    }
    s
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    fn rendered_math(source: &str) -> RenderNode {
        let root = std::env::current_dir().unwrap();
        let rendered = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("math-typed-loss.typ"), &root)
            .unwrap();
        assert!(rendered.ok, "{:?}", rendered.diagnostics);
        fn first(node: &RenderNode) -> Option<&RenderNode> {
            if node.tag.as_deref() == Some("math") {
                Some(node)
            } else {
                node.children.iter().find_map(first)
            }
        }
        first(rendered.tree.as_ref().unwrap()).unwrap().clone()
    }

    #[test]
    fn typed_math_losses_never_enter_plain_fallback_or_native_omml_roundtrip_source() {
        let original = rendered_math("$frac(x, y)$");
        let authored_plain = plain(&original);
        let omml = convert(&original).unwrap();
        let roundtrip = import_equation(&omml, 0).unwrap().source;
        let mut unsupported = original.clone();
        fn fraction(node: &mut RenderNode) -> Option<&mut RenderNode> {
            if node.tag.as_deref() == Some("mfrac") {
                Some(node)
            } else {
                node.children.iter_mut().find_map(fraction)
            }
        }
        fraction(&mut unsupported)
            .unwrap()
            .attrs
            .insert("linethickness".into(), "2pt".into());
        let error = convert(&unsupported).unwrap_err();
        assert_eq!(
            error.explanation,
            UiMessage::error_export_docx_math_fraction_line_width_unsupported()
        );
        assert!(error.diagnostics.is_empty());
        assert!(error.causes.as_slice().is_empty());
        let wire = serde_json::to_value(&error).unwrap();
        let _ = error.explanation.render(weftext_messages::UiLocale::ZhCn);
        let _ = error.explanation.render(weftext_messages::UiLocale::EnUs);
        assert_eq!(serde_json::to_value(&error).unwrap(), wire);
        assert_eq!(plain(&unsupported), authored_plain);
        assert_eq!(
            import_equation(&convert(&original).unwrap(), 0)
                .unwrap()
                .source,
            roundtrip
        );
    }

    #[test]
    fn spacing_reports_original_width_or_its_absence_and_preserves_parallel_losses() {
        let mut space = rendered_math("$x$");
        space.tag = Some("mspace".into());
        space.attrs.clear();
        space.children.clear();
        space.text = None;
        space.attrs.insert("width".into(), "0.75em".into());
        let known = spacing_losses(&space).remove(0);
        assert_eq!(
            known.explanation,
            UiMessage::error_export_docx_math_spacing_width_approximate(
                weftext_messages::params::ErrorExportDocxMathSpacingWidthApproximate {
                    width: "0.75em".into(),
                }
            )
        );
        let mut root = rendered_math("$x$");
        root.children = vec![space.clone(); 96];
        let parallel = spacing_losses(&root);
        assert_eq!(parallel.len(), 96);
        assert!(parallel.iter().all(|reason| reason == &known));
        space.attrs.remove("width");
        let missing = spacing_losses(&space).remove(0);
        assert_eq!(
            missing.explanation,
            UiMessage::error_export_docx_math_spacing_width_unspecified()
        );
        assert!(missing.diagnostics.is_empty());
        assert!(missing.causes.as_slice().is_empty());
        assert_eq!(
            serde_json::to_value(&missing).unwrap()["explanation"]["params"],
            serde_json::json!({})
        );
    }

    #[test]
    fn math_native_provenance_distinguishes_repeated_symbols_and_subexpressions() {
        let source = "$frac(x, x) + x_1^2$";
        let root = std::env::current_dir().unwrap();
        let render = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("math-provenance.typ"), &root)
            .unwrap();
        assert!(render.ok, "{:?}", render.diagnostics);
        fn first(n: &RenderNode) -> Option<&RenderNode> {
            if n.tag.as_deref() == Some("math") {
                Some(n)
            } else {
                n.children.iter().find_map(first)
            }
        }
        let math = convert(first(render.tree.as_ref().unwrap()).unwrap()).unwrap();
        assert_eq!(math.source.as_ref(), Some(&(0, source.to_owned())));
        let mut runs = vec![];
        math.descendants("m:r", &mut runs);
        let occurrences: Vec<_> = runs
            .into_iter()
            .filter_map(|r| r.source.as_ref())
            .filter(|(_, raw)| raw == "x")
            .map(|(from, raw)| (*from, from + raw.len()))
            .collect();
        assert_eq!(occurrences, [(6, 7), (9, 10), (14, 15)]);
        let mut fractions = vec![];
        math.descendants("m:f", &mut fractions);
        assert_eq!(fractions.len(), 1);
        let (from, raw) = fractions[0]
            .source
            .as_ref()
            .expect("native fraction source");
        assert_eq!(&source[*from..*from + raw.len()], raw);
        assert!(raw.contains("frac"));
    }

    #[test]
    fn math_native_provenance_covers_fraction_argument_subexpressions() {
        let source = "$frac(x + y, x - y)$";
        let root = std::env::current_dir().unwrap();
        let render = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("math-subexpression.typ"), &root)
            .unwrap();
        assert!(render.ok, "{:?}", render.diagnostics);
        fn first(n: &RenderNode) -> Option<&RenderNode> {
            if n.tag.as_deref() == Some("math") {
                Some(n)
            } else {
                n.children.iter().find_map(first)
            }
        }
        let math = convert(first(render.tree.as_ref().unwrap()).unwrap()).unwrap();
        let mut numerators = vec![];
        math.descendants("m:num", &mut numerators);
        assert_eq!(
            numerators[0].source.as_ref().map(|(_, raw)| raw.as_str()),
            Some("x + y")
        );
        let mut denominators = vec![];
        math.descendants("m:den", &mut denominators);
        assert_eq!(
            denominators[0].source.as_ref().map(|(_, raw)| raw.as_str()),
            Some("x - y")
        );
    }

    #[test]
    fn math_native_provenance_never_invents_group_ranges_for_reordered_arguments() {
        let source = "#let rev(a,b) = b + a\n$frac(rev(x, y), 1)$";
        let root = std::env::current_dir().unwrap();
        let render = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("math-reordered.typ"), &root)
            .unwrap();
        assert!(render.ok, "{:?}", render.diagnostics);
        fn first(n: &RenderNode) -> Option<&RenderNode> {
            if n.tag.as_deref() == Some("math") {
                Some(n)
            } else {
                n.children.iter().find_map(first)
            }
        }
        let math = convert(first(render.tree.as_ref().unwrap()).unwrap()).unwrap();
        let mut numerators = vec![];
        math.descendants("m:num", &mut numerators);
        assert!(
            numerators[0].source.is_none(),
            "generated/reordered numerator acquired a source: {:?}",
            numerators[0].source
        );
    }

    #[test]
    fn math_native_cases_remain_an_editable_one_sided_delimited_matrix() {
        let source = "$cases(x & x > 0, 0 & \"其它\")$";
        let root = std::env::current_dir().unwrap();
        let render = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("math-cases.typ"), &root)
            .unwrap();
        assert!(render.ok, "{:?}", render.diagnostics);
        fn first(n: &RenderNode) -> Option<&RenderNode> {
            if n.tag.as_deref() == Some("math") {
                Some(n)
            } else {
                n.children.iter().find_map(first)
            }
        }
        let math = convert(first(render.tree.as_ref().unwrap()).unwrap()).unwrap();
        let mut delimiters = vec![];
        math.descendants("m:d", &mut delimiters);
        assert_eq!(delimiters.len(), 1, "{math:?}");
        let imported = import_equation(&math, 0).unwrap();
        assert!(imported.source.contains("cases("), "{}", imported.source);
    }

    #[test]
    fn math_native_binomial_does_not_silently_gain_a_fraction_bar() {
        let source = "$binom(n, k)$";
        let root = std::env::current_dir().unwrap();
        let render = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &root.join("math-binomial.typ"), &root)
            .unwrap();
        assert!(render.ok, "{:?}", render.diagnostics);
        fn first(n: &RenderNode) -> Option<&RenderNode> {
            if n.tag.as_deref() == Some("math") {
                Some(n)
            } else {
                n.children.iter().find_map(first)
            }
        }
        let math = convert(first(render.tree.as_ref().unwrap()).unwrap()).unwrap();
        let mut kinds = vec![];
        math.descendants("m:type", &mut kinds);
        assert!(
            kinds.iter().any(|p| p.get("m:val") == Some("noBar")),
            "{math:?}"
        );
        // A no-line fraction has no supported native reverse representation;
        // retain the original rather than change it into a division fraction.
        assert!(import_equation(&math, 0).is_err());
    }
}
