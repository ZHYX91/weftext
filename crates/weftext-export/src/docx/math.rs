//! Presentation MathML from the frozen HTML tree -> editable Office Math.
use super::xml::{Element, Xml};
use unicode_normalization::char::decompose_compatible;
use weftext_engine::RenderNode;

pub(super) fn convert(node: &RenderNode) -> Result<Element, String> {
    let mut math = Element::new("m:oMath");
    math.children = content(node)?;
    Ok(math)
}
fn wrap(name: &str, nodes: Vec<Xml>) -> Element {
    let mut e = Element::new(name);
    e.children = nodes;
    e
}
fn content(node: &RenderNode) -> Result<Vec<Xml>, String> {
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
    let argument = |index: usize| -> Result<Vec<Xml>, String> {
        children
            .get(index)
            .ok_or_else(|| format!("{tag} 的参数不足"))
            .and_then(|n| content(n))
    };
    let result = match tag {
        "math" | "mrow" | "semantics" => {
            let mut out = vec![];
            for c in children {
                out.extend(content(c)?);
            }
            return Ok(out);
        }
        "mi" | "mn" | "mo" | "mtext" | "ms" => {
            return tokens(
                &plain(node),
                node.attrs.get("mathvariant").map(String::as_str),
                tag == "mtext" || tag == "ms",
                tag,
            );
        }
        "mfrac" if children.len() == 2 => {
            let mut e = Element::new("m:f");
            e.push(wrap("m:num", argument(0)?));
            e.push(wrap("m:den", argument(1)?));
            e
        }
        "msub" | "msup" if children.len() == 2 => {
            let mut e = Element::new(if tag == "msub" { "m:sSub" } else { "m:sSup" });
            e.push(wrap("m:e", argument(0)?));
            e.push(wrap(
                if tag == "msub" { "m:sub" } else { "m:sup" },
                argument(1)?,
            ));
            e
        }
        "msubsup" if children.len() == 3 => {
            let mut e = Element::new("m:sSubSup");
            e.push(wrap("m:e", argument(0)?));
            e.push(wrap("m:sub", argument(1)?));
            e.push(wrap("m:sup", argument(2)?));
            e
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
            e.push(wrap("m:deg", argument(1)?));
            e.push(wrap("m:e", argument(0)?));
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
                return Err("多分隔符公式暂不支持".into());
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
                    return Err("公式矩阵行类型暂不支持".into());
                }
                let mut mr = Element::new("m:mr");
                for cell in &row.children {
                    if cell.tag.as_deref() != Some("mtd") {
                        return Err("公式矩阵单元格类型暂不支持".into());
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
        _ => return Err(format!("公式结构 {tag} 暂不能转换为可编辑 Office Math")),
    };
    Ok(vec![Xml::Element(result)])
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct MathStyle {
    script: &'static str,
    style: &'static str,
}
fn variant(name: &str) -> Result<MathStyle, String> {
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
        _ => return Err(format!("公式 mathvariant {name} 暂不支持")),
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
) -> Result<Vec<Xml>, String> {
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
