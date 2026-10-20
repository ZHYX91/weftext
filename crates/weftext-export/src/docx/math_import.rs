//! Bounded Office Math -> native Typst math. This is a structural conversion,
//! never an evaluator of arbitrary Word fields, TeX, macros or embedded code.
use super::super::{
    DocxError,
    xml::{Element, Xml},
};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::docx) struct ImportedEquation {
    /// Includes native math delimiters. No independently editable second source.
    pub source: String,
    pub visible: String,
    /// Each original math scalar maps to its own generated source expression.
    pub positions: Vec<(usize, usize)>,
    /// Accurate source-byte boundaries, deliberately not glyph indices. For
    /// example the first x and the whole numerator have different boundaries.
    pub starts: Vec<(String, usize)>,
    pub ends: Vec<(String, usize)>,
    pub invalid_ranges: BTreeSet<String>,
}

pub(in crate::docx) fn import_equation(
    equation: &Element,
    source_offset: usize,
) -> Result<ImportedEquation, DocxError> {
    if !matches!(equation.name.as_str(), "m:oMath" | "m:oMathPara") {
        return Err(DocxError::new("公式入口不是 Office Math 对象"));
    }
    let display = equation.name == "m:oMathPara";
    let mut b = Builder {
        out: ImportedEquation {
            source: if display { "$ " } else { "$" }.into(),
            visible: String::new(),
            positions: vec![],
            starts: vec![],
            ends: vec![],
            invalid_ranges: BTreeSet::new(),
        },
        offset: source_offset,
        count: 0,
        open: BTreeMap::new(),
        closed: BTreeSet::new(),
        pending_starts: vec![],
        last_content_end: source_offset + if display { 2 } else { 1 },
    };
    b.node(equation, 0)?;
    b.out.source.push_str(if display { " $" } else { "$" });
    if b.out.positions.is_empty() {
        return Err(DocxError::new("空公式未作为可编辑公式导入；原件保留"));
    }
    // A valid Word comment may begin/end outside this mathematical object.
    // The document importer reconciles both sides; only local duplicates are
    // intrinsically invalid here.
    Ok(b.out)
}

struct Builder {
    out: ImportedEquation,
    offset: usize,
    count: usize,
    open: BTreeMap<String, usize>,
    closed: BTreeSet<String>,
    pending_starts: Vec<usize>,
    last_content_end: usize,
}
impl Builder {
    fn decoration(&mut self, value: &str) -> Result<(), DocxError> {
        if self.out.source.len() + value.len() > 1_048_576 {
            return Err(DocxError::new("单个公式超过 1 MiB 转换预算"));
        }
        self.out.source.push_str(value);
        Ok(())
    }
    fn append(&mut self, value: &str) -> Result<(), DocxError> {
        if self.out.source.len() + value.len() > 1_048_576 {
            return Err(DocxError::new("单个公式超过 1 MiB 转换预算"));
        }
        if !value.trim().is_empty() {
            let at = self.offset + self.out.source.len();
            for index in self.pending_starts.drain(..) {
                self.out.starts[index].1 = at;
            }
            self.last_content_end = at + value.len();
        }
        self.out.source.push_str(value);
        Ok(())
    }
    fn sequence(&mut self, e: &Element, depth: usize, ignored: &[&str]) -> Result<(), DocxError> {
        for child in &e.children {
            match child {
                Xml::Element(c) if ignored.contains(&c.name.as_str()) => {}
                Xml::Element(c) => {
                    self.node(c, depth + 1)?;
                    self.append(" ")?;
                }
                Xml::Text(s) if s.trim().is_empty() => {}
                Xml::Comment(_) => {}
                _ => return Err(DocxError::new("Office Math 含未归属数学 run 的文字")),
            }
        }
        Ok(())
    }
    fn required<'a>(&self, e: &'a Element, name: &str) -> Result<&'a Element, DocxError> {
        let matches: Vec<_> = e
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|c| c.name == name)
            .collect();
        if matches.len() == 1 {
            Ok(matches[0])
        } else {
            Err(DocxError::new(format!(
                "{} 的 {name} 参数缺失或重复",
                e.name
            )))
        }
    }
    fn check_children(&self, e: &Element, allowed: &[&str]) -> Result<(), DocxError> {
        for c in e.children.iter().filter_map(Xml::element) {
            if !allowed.contains(&c.name.as_str()) {
                return Err(DocxError::new(format!(
                    "数学结构 {} 的子项 {} 暂不支持",
                    e.name, c.name
                )));
            }
        }
        Ok(())
    }
    fn argument(&mut self, e: &Element, name: &str, depth: usize) -> Result<(), DocxError> {
        self.node(self.required(e, name)?, depth + 1)
    }
    fn property<'a>(e: &'a Element, pr: &str, name: &str) -> Option<&'a str> {
        e.child(pr)?.child(name)?.get("m:val")
    }
    fn node(&mut self, e: &Element, depth: usize) -> Result<(), DocxError> {
        self.count += 1;
        if depth > 64 || self.count > 20_000 {
            return Err(DocxError::new("公式结构超过深度／节点转换预算"));
        }
        match e.name.as_str() {
            "m:oMathPara" => {
                self.check_children(
                    e,
                    &[
                        "m:oMathParaPr",
                        "m:oMath",
                        "w:commentRangeStart",
                        "w:commentRangeEnd",
                    ],
                )?;
                let formula_count = e
                    .children
                    .iter()
                    .filter_map(Xml::element)
                    .filter(|c| c.name == "m:oMath")
                    .count();
                if formula_count != 1 {
                    return Err(DocxError::new("一个公式段中的多个独立数学对象暂不合并"));
                }
                self.sequence(e, depth, &["m:oMathParaPr"])
            }
            "m:oMath" | "m:e" | "m:num" | "m:den" | "m:sub" | "m:sup" | "m:deg" | "m:lim" => {
                self.sequence(e, depth, &[])
            }
            "w:commentRangeStart" => {
                let id = e
                    .get("w:id")
                    .ok_or_else(|| DocxError::new("数学批注起点缺少标识"))?
                    .to_owned();
                let at = self.offset + self.out.source.len();
                let index = self.out.starts.len();
                if self.open.insert(id.clone(), index).is_some() || self.closed.contains(&id) {
                    self.out.invalid_ranges.insert(id.clone());
                }
                self.out.starts.push((id, at));
                self.pending_starts.push(index);
                Ok(())
            }
            "w:commentRangeEnd" => {
                let id = e
                    .get("w:id")
                    .ok_or_else(|| DocxError::new("数学批注终点缺少标识"))?
                    .to_owned();
                let at = self.last_content_end;
                if let Some(index) = self.open.remove(&id)
                    && self.pending_starts.contains(&index)
                {
                    self.pending_starts.retain(|n| *n != index);
                    self.out.invalid_ranges.insert(id.clone());
                }
                if !self.closed.insert(id.clone()) {
                    self.out.invalid_ranges.insert(id.clone());
                }
                self.out.ends.push((id, at));
                Ok(())
            }
            "w:commentReference" => Ok(()),
            "m:ctrlPr" => {
                // Word adds control-character formatting to argument nodes
                // when it builds/saves an equation. It is not an additional
                // mathematical operand. Keep this boundary narrow: revision
                // containers, text and unknown properties must not disappear.
                self.check_children(e, &["w:rPr"])?;
                if e.children.iter().filter_map(Xml::element).count() > 1 {
                    return Err(DocxError::new("数学控制字形属性重复"));
                }
                if e.children
                    .iter()
                    .any(|c| matches!(c, Xml::Text(s) if !s.trim().is_empty()))
                {
                    return Err(DocxError::new("数学控制字形属性包含正文"));
                }
                if let Some(pr) = e.child("w:rPr") {
                    self.check_children(
                        pr,
                        &[
                            "w:rFonts",
                            "w:rStyle",
                            "w:i",
                            "w:iCs",
                            "w:b",
                            "w:bCs",
                            "w:sz",
                            "w:szCs",
                            "w:lang",
                            "w:color",
                            "w:noProof",
                            "w:rtl",
                            "w:cs",
                        ],
                    )?;
                    if pr.children.iter().any(|c| match c {
                        Xml::Element(p) => p.children.iter().any(|n| match n {
                            Xml::Element(_) => true,
                            Xml::Text(s) => !s.trim().is_empty(),
                            Xml::Comment(_) => false,
                        }),
                        Xml::Text(s) => !s.trim().is_empty(),
                        Xml::Comment(_) => false,
                    }) {
                        return Err(DocxError::new("数学控制字形属性包含未知嵌套内容"));
                    }
                }
                Ok(())
            }
            "w:r" => {
                // Word's comment-reference run is discussion metadata, not
                // ordinary prose to be folded into the mathematical source.
                self.check_children(e, &["w:rPr", "w:commentReference"])?;
                if e.child("w:commentReference").is_none()
                    || e.children
                        .iter()
                        .any(|c| matches!(c, Xml::Text(s) if !s.trim().is_empty()))
                {
                    return Err(DocxError::new(
                        "公式内的普通 Word 文字 run 未作为数学内容导入",
                    ));
                }
                Ok(())
            }
            "m:r" => self.run(e),
            "m:f" => {
                self.check_children(e, &["m:fPr", "m:num", "m:den"])?;
                if Self::property(e, "m:fPr", "m:type").is_some_and(|v| v != "bar") {
                    return Err(DocxError::new(
                        "无横线／线性／斜线分数版式暂不支持；原件保留",
                    ));
                }
                self.append("frac(")?;
                self.argument(e, "m:num", depth)?;
                self.append(", ")?;
                self.argument(e, "m:den", depth)?;
                self.append(")")
            }
            "m:sSub" | "m:sSup" | "m:sSubSup" => {
                self.check_children(
                    e,
                    &[
                        "m:sSubPr",
                        "m:sSupPr",
                        "m:sSubSupPr",
                        "m:e",
                        "m:sub",
                        "m:sup",
                    ],
                )?;
                // Parentheses in native math are visible delimiters. Use the
                // actual attachment function to group a base without adding
                // fences that were absent from the Word equation.
                self.append("attach(")?;
                self.argument(e, "m:e", depth)?;
                if e.name != "m:sSup" {
                    self.append(", br: ")?;
                    self.argument(e, "m:sub", depth)?;
                }
                if e.name != "m:sSub" {
                    self.append(", tr: ")?;
                    self.argument(e, "m:sup", depth)?;
                }
                self.append(")")
            }
            "m:rad" => {
                self.check_children(e, &["m:radPr", "m:deg", "m:e"])?;
                let hide = Self::property(e, "m:radPr", "m:degHide")
                    .is_some_and(|v| matches!(v, "1" | "true" | "on"));
                let degree = self.required(e, "m:deg")?;
                if hide || degree.children.is_empty() {
                    self.append("sqrt(")?;
                } else {
                    self.append("root(")?;
                    self.node(degree, depth + 1)?;
                    self.append(", ")?;
                }
                self.argument(e, "m:e", depth)?;
                self.append(")")
            }
            "m:m" => self.matrix(e, depth, false),
            "m:d" => self.delimiter(e, depth),
            "m:limLow" | "m:limUpp" => {
                self.check_children(e, &["m:limLowPr", "m:limUppPr", "m:e", "m:lim"])?;
                self.append("limits(")?;
                self.argument(e, "m:e", depth)?;
                self.append(if e.name == "m:limLow" { ")_(" } else { ")^(" })?;
                self.argument(e, "m:lim", depth)?;
                self.append(")")
            }
            "m:nary" => {
                self.check_children(e, &["m:naryPr", "m:sub", "m:sup", "m:e"])?;
                let ch = Self::property(e, "m:naryPr", "m:chr").unwrap_or("∫");
                if !matches!(ch, "∫" | "∬" | "∭" | "∮" | "∑" | "∏" | "⋃" | "⋂") {
                    return Err(DocxError::new(format!("n 元运算符 {ch} 暂不支持")));
                }
                self.scalar(ch, "", "")?;
                for (name, hide, marker) in
                    [("m:sub", "m:subHide", "_("), ("m:sup", "m:supHide", "^(")]
                {
                    if !Self::property(e, "m:naryPr", hide)
                        .is_some_and(|v| matches!(v, "1" | "true" | "on"))
                        && let Some(arg) = e.child(name)
                        && !arg.children.is_empty()
                    {
                        self.append(marker)?;
                        self.node(arg, depth + 1)?;
                        self.append(")")?;
                    }
                }
                self.append(" ")?;
                self.argument(e, "m:e", depth)
            }
            _ => Err(DocxError::new(format!(
                "数学结构 {} 尚不能转换为原生可编辑公式；原件保留",
                e.name
            ))),
        }
    }
    fn run(&mut self, e: &Element) -> Result<(), DocxError> {
        // Real Word stores a comment-reference glyph in either a w:r or m:r.
        // This glyph is discussion metadata, never visible mathematical text.
        self.check_children(e, &["m:rPr", "w:rPr", "m:t", "w:commentReference"])?;
        if let Some(pr) = e.child("m:rPr") {
            self.check_children(pr, &["m:sty", "m:scr", "m:nor", "m:lit", "m:brk", "m:aln"])?;
            if pr.child("m:brk").is_some() || pr.child("m:aln").is_some() {
                return Err(DocxError::new("数学 run 的手动换行／对齐控制暂不支持"));
            }
        }
        let normal = e
            .child("m:rPr")
            .and_then(|p| p.child("m:nor"))
            .is_some_and(|n| !matches!(n.get("m:val"), Some("0" | "false" | "off")));
        let script = Self::property(e, "m:rPr", "m:scr").unwrap_or("roman");
        let style = Self::property(e, "m:rPr", "m:sty").unwrap_or("i");
        let function = match script {
            "roman" => "",
            "script" => "cal",
            "fraktur" => "frak",
            "double-struck" => "bb",
            "sans-serif" => "sans",
            "monospace" => "mono",
            _ => return Err(DocxError::new(format!("数学字体族 {script} 暂不支持"))),
        };
        if !matches!(style, "p" | "b" | "i" | "bi") {
            return Err(DocxError::new(format!("数学字形 {style} 暂不支持")));
        }
        for t in e
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|c| c.name == "m:t")
        {
            if t.children.iter().any(|c| matches!(c, Xml::Element(_))) {
                return Err(DocxError::new("数学文字含未知嵌套结构"));
            }
            let value = t.text();
            let mut characters = value.chars().peekable();
            while let Some(c) = characters.next() {
                if c.is_control() {
                    return Err(DocxError::new("数学文字含不支持的控制字符"));
                }
                let decoration = if normal {
                    "upright"
                } else {
                    match style {
                        "b" => "bold",
                        "bi" => "bolditalic",
                        "p" => "upright",
                        _ => "",
                    }
                };
                let mut token = c.to_string();
                if c.is_ascii_digit() {
                    while characters.peek().is_some_and(char::is_ascii_digit) {
                        token.push(characters.next().unwrap());
                    }
                    if characters.peek() == Some(&'.')
                        && characters
                            .clone()
                            .nth(1)
                            .is_some_and(|c| c.is_ascii_digit())
                    {
                        token.push(characters.next().unwrap());
                        while characters.peek().is_some_and(char::is_ascii_digit) {
                            token.push(characters.next().unwrap());
                        }
                    }
                }
                self.scalar(&token, function, decoration)?;
                self.append(" ")?;
            }
        }
        Ok(())
    }
    fn scalar(&mut self, value: &str, family: &str, decoration: &str) -> Result<(), DocxError> {
        let c = value.chars().next().unwrap_or(' ');
        let number = !value.is_empty()
            && (value.chars().all(|c| c.is_ascii_digit())
                || value.split_once('.').is_some_and(|(a, b)| {
                    !a.is_empty()
                        && !b.is_empty()
                        && a.chars().all(|c| c.is_ascii_digit())
                        && b.chars().all(|c| c.is_ascii_digit())
                }));
        let bare = number
            || value.chars().count() == 1
                && (c.is_alphabetic()
                    || c.is_numeric()
                    || "+−×÷=≠≤≥<>∫∬∭∮∑∏⋃⋂∞∂∇±∓·⋅∈∉∋⊂⊃⊆⊇∪∩∧∨¬∀∃→←↔⇒⇐⇔≡≈∝∅∠⊥∥".contains(c));
        let encoded = if bare {
            value.into()
        } else {
            weftext_data::NativeValue::Text(value.into())
                .to_typst()
                .map_err(|e| DocxError::new(e.to_string()))?
        };
        if !family.is_empty() {
            self.decoration(family)?;
            self.decoration("(")?;
        }
        if decoration == "bolditalic" {
            self.decoration("bold(italic(")?;
        } else if !decoration.is_empty() {
            self.decoration(decoration)?;
            self.decoration("(")?;
        }
        let from = self.offset + self.out.source.len();
        self.append(&encoded)?;
        let to = self.offset + self.out.source.len();
        if decoration == "bolditalic" {
            self.decoration("))")?;
        } else if !decoration.is_empty() {
            self.decoration(")")?;
        }
        if !family.is_empty() {
            self.decoration(")")?;
        }
        if encoded == value {
            self.out.positions.extend(
                value
                    .char_indices()
                    .map(|(i, c)| (from + i, from + i + c.len_utf8())),
            );
        } else {
            self.out.positions.push((from, to));
        }
        self.out.visible.push_str(value);
        Ok(())
    }
    fn matrix(&mut self, e: &Element, depth: usize, cases: bool) -> Result<(), DocxError> {
        self.check_children(e, &["m:mPr", "m:mr"])?;
        let rows: Vec<_> = e
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|c| c.name == "m:mr")
            .collect();
        if rows.is_empty() || rows.len() > 100 {
            return Err(DocxError::new("矩阵行数不在 1–100 支持范围"));
        }
        let mut width = None;
        self.append(if cases {
            "cases("
        } else {
            "mat(delim: #none, "
        })?;
        for (r, row) in rows.iter().enumerate() {
            self.check_children(row, &["m:e"])?;
            let cells: Vec<_> = row.children.iter().filter_map(Xml::element).collect();
            if cells.is_empty() || cells.len() > 100 || width.is_some_and(|n| n != cells.len()) {
                return Err(DocxError::new("矩阵为空、列数过多或各行宽度不一致"));
            }
            width = Some(cells.len());
            if r > 0 {
                self.append(if cases { ", " } else { "; " })?;
            }
            for (i, cell) in cells.iter().enumerate() {
                if i > 0 {
                    self.append(if cases { " & " } else { ", " })?;
                }
                self.node(cell, depth + 1)?;
            }
        }
        self.append(")")
    }
    fn delimiter(&mut self, e: &Element, depth: usize) -> Result<(), DocxError> {
        self.check_children(e, &["m:dPr", "m:e"])?;
        let begin = Self::property(e, "m:dPr", "m:begChr").unwrap_or("(");
        let end = Self::property(e, "m:dPr", "m:endChr").unwrap_or(")");
        let separator = Self::property(e, "m:dPr", "m:sepChr").unwrap_or("|");
        let arguments: Vec<_> = e
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|c| c.name == "m:e")
            .collect();
        if begin == "{" && end.is_empty() && arguments.len() == 1 {
            let inner: Vec<_> = arguments[0]
                .children
                .iter()
                .filter_map(Xml::element)
                .collect();
            if inner.len() == 1 && inner[0].name == "m:m" {
                return self.matrix(inner[0], depth + 1, true);
            }
        }
        if !matches!(begin, "(" | "[" | "{" | "|" | "‖" | "⌈" | "⌊" | "⟨" | "")
            || !matches!(end, ")" | "]" | "}" | "|" | "‖" | "⌉" | "⌋" | "⟩" | "")
            || separator.chars().count() > 1
            || arguments.is_empty()
        {
            return Err(DocxError::new("分隔符公式含未支持的括号／多字符分隔符"));
        }
        self.append("lr(")?;
        self.append(&escaped_delimiter(begin))?;
        for (i, argument) in arguments.iter().enumerate() {
            if i > 0 {
                self.append(&quoted(separator))?;
                self.append(" ")?;
            }
            self.node(argument, depth + 1)?;
        }
        self.append(&escaped_delimiter(end))?;
        self.append(")")
    }
}
fn quoted(s: &str) -> String {
    weftext_data::NativeValue::Text(s.into())
        .to_typst()
        .expect("valid mathematical delimiter")
}
fn escaped_delimiter(s: &str) -> String {
    match s {
        "{" => "\\{".into(),
        "}" => "\\}".into(),
        "[" => "[".into(),
        "]" => "]".into(),
        _ => s.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::xml;
    use super::*;
    use std::path::Path;

    fn import(body: &str) -> ImportedEquation {
        let e =
            xml::parse(format!("<m:oMath xmlns:m=\"m\" xmlns:w=\"w\">{body}</m:oMath>").as_bytes())
                .unwrap();
        import_equation(&e, 40).unwrap()
    }
    fn run(s: &str) -> String {
        format!("<m:r><m:t>{s}</m:t></m:r>")
    }
    fn compile(source: &str) {
        let root = std::env::current_dir().unwrap();
        let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
        let result = engine
            .render(source, &root.join("math-import-test.typ"), Path::new(&root))
            .unwrap();
        assert!(result.ok, "{source}\n{:?}", result.diagnostics);
        fn math(n: &weftext_engine::RenderNode) -> bool {
            n.tag.as_deref() == Some("math") || n.children.iter().any(math)
        }
        assert!(math(result.tree.as_ref().unwrap()));
    }
    #[test]
    fn omml_fraction_scripts_roots_native_compile() {
        let fraction = format!(
            "<m:f><m:num>{}</m:num><m:den>{}</m:den></m:f>",
            run("x"),
            run("x")
        );
        let scripts = format!(
            "<m:sSubSup><m:e>{fraction}</m:e><m:sub>{}</m:sub><m:sup>{}</m:sup></m:sSubSup>",
            run("1"),
            run("2")
        );
        let equation = import(&format!(
            "<m:rad><m:deg>{}</m:deg><m:e>{scripts}</m:e></m:rad>",
            run("3")
        ));
        assert_eq!(equation.visible, "3xx12");
        assert_eq!(equation.positions.len(), 5);
        assert!(equation.positions.windows(2).all(|p| p[0].1 <= p[1].0));
        compile(&equation.source);
    }
    #[test]
    fn omml_matrix_cases_integral_and_cjk_compile() {
        let matrix = format!(
            "<m:m><m:mr><m:e>{}</m:e><m:e>{}</m:e></m:mr><m:mr><m:e>{}</m:e><m:e>{}</m:e></m:mr></m:m>",
            run("1"),
            run("x"),
            run("2"),
            run("中")
        );
        compile(&import(&matrix).source);
        compile(&import(&format!("<m:d><m:dPr><m:begChr m:val=\"{{\"/><m:endChr m:val=\"\"/></m:dPr><m:e>{matrix}</m:e></m:d>")).source);
        let integral = format!(
            "<m:nary><m:naryPr><m:chr m:val=\"∫\"/></m:naryPr><m:sub>{}</m:sub><m:sup>{}</m:sup><m:e>{}</m:e></m:nary>",
            run("0"),
            run("∞"),
            run("x")
        );
        compile(&import(&integral).source);
        let limits = format!(
            "<m:limUpp><m:e><m:limLow><m:e>{}</m:e><m:lim>{}</m:lim></m:limLow></m:e><m:lim>{}</m:lim></m:limUpp>",
            run("∑"),
            run("0"),
            run("∞")
        );
        compile(&import(&limits).source);
    }
    #[test]
    fn omml_comments_map_occurrences_and_subexpressions_without_text_search() {
        let equation = import(&format!(
            "<m:f><m:num><w:commentRangeStart w:id=\"symbol\"/>{}<w:commentRangeEnd w:id=\"symbol\"/></m:num><m:den><w:commentRangeStart w:id=\"den\"/>{}<w:commentRangeEnd w:id=\"den\"/></m:den></m:f>",
            run("x"),
            run("x")
        ));
        let start = |id: &str| equation.starts.iter().find(|(s, _)| s == id).unwrap().1;
        let end = |id: &str| equation.ends.iter().find(|(s, _)| s == id).unwrap().1;
        assert!(start("symbol") < end("symbol") && end("symbol") < start("den"));
        assert!(equation.invalid_ranges.is_empty());
        assert_eq!(
            &equation.source[start("symbol") - 40..end("symbol") - 40],
            "x"
        );
        assert_eq!(&equation.source[start("den") - 40..end("den") - 40], "x");
    }
    #[test]
    fn omml_unknown_structures_and_control_do_not_become_text() {
        for body in [
            "<m:phant><m:e><m:r><m:t>x</m:t></m:r></m:e></m:phant>",
            "<m:f><m:num/><m:num/><m:den/></m:f>",
            "<m:r><m:rPr><m:scr m:val=\"unknown\"/></m:rPr><m:t>x</m:t></m:r>",
        ] {
            let e = xml::parse(format!("<m:oMath>{body}</m:oMath>").as_bytes()).unwrap();
            assert!(import_equation(&e, 0).is_err());
        }
        let equation = import(&run("#[$]@`\\"));
        compile(&equation.source);
    }
    #[test]
    fn omml_repeated_comment_ids_are_invalid_and_unicode_boundaries_exact() {
        let equation = import(&format!(
            "<w:commentRangeStart w:id=\"1\"/>{}<w:commentRangeEnd w:id=\"1\"/><w:commentRangeStart w:id=\"1\"/>{}<w:commentRangeEnd w:id=\"1\"/>",
            run("中𝑥"),
            run("é")
        ));
        assert!(equation.invalid_ranges.contains("1"));
        assert_eq!(equation.positions.len(), 4);
        for (from, to) in equation.positions {
            assert!(
                equation.source.is_char_boundary(from - 40)
                    && equation.source.is_char_boundary(to - 40)
            );
        }
    }
    #[test]
    fn omml_comment_reference_word_runs_are_metadata_but_prose_is_rejected() {
        let equation = import(&format!(
            "<w:commentRangeStart w:id=\"1\"/>{}<w:commentRangeEnd w:id=\"1\"/><w:r><w:rPr/><w:commentReference w:id=\"1\"/></w:r>",
            run("x")
        ));
        assert_eq!(equation.visible, "x");
        assert!(equation.invalid_ranges.is_empty());
        let e = xml::parse(b"<m:oMath><w:r><w:t>x</w:t></w:r></m:oMath>").unwrap();
        assert!(import_equation(&e, 0).is_err());
    }

    #[test]
    fn omml_word_saved_math_comment_runs_and_argument_control_properties() {
        let imported = import(
            r#"<m:f><m:fPr><m:ctrlPr><w:rPr><w:rFonts w:ascii="Cambria Math"/><w:i/></w:rPr></m:ctrlPr></m:fPr>
            <m:num><w:commentRangeStart w:id="1"/><m:r><w:rPr><w:rFonts w:hint="eastAsia"/></w:rPr><m:t>x</m:t></m:r>
            <w:commentRangeEnd w:id="1"/><m:r><w:rPr><w:rStyle w:val="ae"/><w:sz w:val="22"/></w:rPr><w:commentReference w:id="1"/></m:r>
            <m:ctrlPr><w:rPr><w:rFonts w:ascii="Cambria Math"/><w:i/></w:rPr></m:ctrlPr></m:num>
            <m:den><w:commentRangeStart w:id="2"/><m:r><m:t>x</m:t></m:r><w:commentRangeEnd w:id="2"/>
            <m:r><w:rPr><w:i/><w:szCs w:val="24"/></w:rPr><w:commentReference w:id="2"/></m:r></m:den></m:f>"#,
        );
        assert_eq!(imported.visible, "xx");
        for id in ["1", "2"] {
            let from = imported.starts.iter().find(|(key, _)| key == id).unwrap().1 - 40;
            let to = imported.ends.iter().find(|(key, _)| key == id).unwrap().1 - 40;
            assert_eq!(&imported.source[from..to], "x");
        }
        assert!(imported.invalid_ranges.is_empty());
        compile(&imported.source);
        for body in [
            "<m:ctrlPr><w:t>must not disappear</w:t></m:ctrlPr>",
            "<m:ctrlPr><w:rPr><w:unknown/></w:rPr></m:ctrlPr>",
            "<m:ctrlPr><w:rPr><w:i><w:t>hidden</w:t></w:i></w:rPr></m:ctrlPr>",
        ] {
            let e = xml::parse(
                format!(
                    "<m:oMath xmlns:m=\"m\" xmlns:w=\"w\">{}{} </m:oMath>",
                    run("x"),
                    body
                )
                .as_bytes(),
            )
            .unwrap();
            assert!(import_equation(&e, 0).is_err(), "{body}");
        }
    }
    #[test]
    fn omml_numeric_runs_remain_numbers_and_styled_symbol_ranges_exclude_generated_wrappers() {
        let e = import(
            "<w:commentRangeStart w:id=\"number\"/><m:r><m:rPr><m:sty m:val=\"p\"/></m:rPr><m:t>123</m:t></m:r><w:commentRangeEnd w:id=\"number\"/>",
        );
        assert_eq!(e.visible, "123");
        assert_eq!(e.positions.len(), 3);
        let a = e.starts[0].1 - 40;
        let z = e.ends[0].1 - 40;
        assert_eq!(&e.source[a..z], "123");
        assert!(e.source.contains("upright(123)"));
        compile(&e.source);
        let decimal = import("<m:r><m:t>123.450</m:t></m:r>");
        assert_eq!(decimal.visible, "123.450");
        assert_eq!(decimal.positions.len(), 7);
        assert!(decimal.source.contains("123.450"));
        compile(&decimal.source);
    }
}
