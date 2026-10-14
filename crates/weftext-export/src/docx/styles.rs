use super::{
    DocxError,
    xml::{Element, Xml, val},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub(super) struct RunStyle(pub BTreeMap<String, Element>);
impl RunStyle {
    pub fn merge(&mut self, props: Option<&Element>, toggle: bool) {
        if let Some(props) = props {
            for e in props.children.iter().filter_map(Xml::element) {
                if e.name == "w:rStyle" {
                    continue;
                }
                if toggle && is_toggle(&e.name) {
                    if truth(e) {
                        let on = !self.0.get(&e.name).is_some_and(truth);
                        self.0
                            .insert(e.name.clone(), val(&e.name, if on { "1" } else { "0" }));
                    }
                } else if e.name == "w:rFonts" || e.name == "w:lang" {
                    let dest = self
                        .0
                        .entry(e.name.clone())
                        .or_insert_with(|| Element::new(&e.name));
                    // Font attributes cascade by script. A higher tier's
                    // explicit font supersedes an inherited theme selection;
                    // a theme selector present in this same rFonts still wins.
                    if e.name == "w:rFonts" {
                        for (explicit, theme) in [
                            ("w:ascii", "w:asciiTheme"),
                            ("w:hAnsi", "w:hAnsiTheme"),
                            ("w:eastAsia", "w:eastAsiaTheme"),
                            ("w:cs", "w:cstheme"),
                        ] {
                            if e.get(explicit).is_some() && e.get(theme).is_none() {
                                dest.attrs.retain(|(k, _)| k != theme);
                            }
                        }
                    }
                    for (k, v) in &e.attrs {
                        dest.set_attr(k, v);
                    }
                } else {
                    self.0.insert(e.name.clone(), e.clone());
                }
            }
        }
    }
    /// All body character properties are direct properties here, including
    /// explicit neutral values, so a Heading style cannot leak into body runs.
    pub fn reset(&self) -> Element {
        let mut out = self.clone();
        for n in [
            "w:b",
            "w:bCs",
            "w:i",
            "w:iCs",
            "w:caps",
            "w:smallCaps",
            "w:strike",
            "w:dstrike",
            "w:outline",
            "w:shadow",
            "w:emboss",
            "w:imprint",
            "w:vanish",
            "w:webHidden",
            "w:rtl",
        ] {
            out.0.entry(n.into()).or_insert_with(|| val(n, "0"));
        }
        for (n, v) in [
            ("w:u", "none"),
            ("w:color", "auto"),
            ("w:highlight", "none"),
            ("w:vertAlign", "baseline"),
            ("w:spacing", "0"),
            ("w:position", "0"),
            ("w:w", "100"),
            ("w:kern", "0"),
        ] {
            out.0.entry(n.into()).or_insert_with(|| val(n, v));
        }
        let mut e = Element::new("w:rPr");
        // ECMA CT_RPr child order, rather than alphabetic map order.
        for name in RPR_ORDER {
            if let Some(p) = out.0.remove(*name) {
                e.push(p);
            }
        }
        for p in out.0.into_values() {
            e.push(p);
        }
        e
    }
    pub fn properties(&self) -> Element {
        let mut e = Element::new("w:rPr");
        for name in RPR_ORDER {
            if let Some(p) = self.0.get(*name) {
                e.push(p.clone());
            }
        }
        e
    }
}
const RPR_ORDER: &[&str] = &[
    "w:rStyle",
    "w:rFonts",
    "w:b",
    "w:bCs",
    "w:i",
    "w:iCs",
    "w:caps",
    "w:smallCaps",
    "w:strike",
    "w:dstrike",
    "w:outline",
    "w:shadow",
    "w:emboss",
    "w:imprint",
    "w:noProof",
    "w:snapToGrid",
    "w:vanish",
    "w:webHidden",
    "w:color",
    "w:spacing",
    "w:w",
    "w:kern",
    "w:position",
    "w:sz",
    "w:szCs",
    "w:highlight",
    "w:u",
    "w:bdr",
    "w:shd",
    "w:fitText",
    "w:vertAlign",
    "w:rtl",
    "w:cs",
    "w:em",
    "w:lang",
    "w:eastAsianLayout",
    "w:specVanish",
    "w:oMath",
];
fn is_toggle(n: &str) -> bool {
    matches!(
        n,
        "w:b"
            | "w:bCs"
            | "w:i"
            | "w:iCs"
            | "w:caps"
            | "w:smallCaps"
            | "w:strike"
            | "w:outline"
            | "w:shadow"
            | "w:emboss"
            | "w:imprint"
            | "w:vanish"
            | "w:webHidden"
    )
}
fn truth(e: &Element) -> bool {
    !matches!(e.get("w:val"), Some("0" | "false" | "off"))
}

pub(super) struct Styles {
    pub xml: Element,
    defaults: RunStyle,
    normal: String,
    theme: Option<Element>,
}
impl Styles {
    pub fn new(xml: Element, theme: Option<Element>) -> Result<Self, DocxError> {
        if xml.name != "w:styles" {
            return Err(DocxError::new("模板样式须使用标准 w 命名空间前缀"));
        }
        let normal = xml
            .children
            .iter()
            .filter_map(Xml::element)
            .find(|e| {
                e.name == "w:style"
                    && e.get("w:type") == Some("paragraph")
                    && e.get("w:default")
                        .is_some_and(|v| matches!(v, "1" | "true" | "on"))
            })
            .and_then(|e| e.get("w:styleId"))
            .unwrap_or("Normal")
            .to_owned();
        let mut defaults = RunStyle::default();
        defaults.merge(
            xml.child("w:docDefaults")
                .and_then(|e| e.child("w:rPrDefault"))
                .and_then(|e| e.child("w:rPr")),
            false,
        );
        Ok(Self {
            xml,
            defaults,
            normal,
            theme,
        })
    }
    pub fn normal(&self) -> &str {
        &self.normal
    }
    fn style(&self, id: &str) -> Option<&Element> {
        self.xml
            .children
            .iter()
            .filter_map(Xml::element)
            .find(|e| e.name == "w:style" && e.get("w:styleId") == Some(id))
    }
    fn chain(
        &self,
        id: &str,
        result: &mut RunStyle,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), DocxError> {
        if !seen.insert(id.into()) {
            return Err(DocxError::new("模板样式继承存在循环"));
        }
        if seen.len() > 64 {
            return Err(DocxError::new("模板样式继承过深"));
        }
        if let Some(e) = self.style(id) {
            if let Some(base) = e.child("w:basedOn").and_then(|e| e.get("w:val")) {
                self.chain(base, result, seen)?;
            }
            result.merge(e.child("w:rPr"), true);
        }
        seen.remove(id);
        Ok(())
    }
    pub fn effective(
        &self,
        paragraph: Option<&Element>,
        run: Option<&Element>,
    ) -> Result<RunStyle, DocxError> {
        let mut result = self.defaults.clone();
        let ppr = paragraph.and_then(|p| p.child("w:pPr"));
        let pstyle = ppr
            .and_then(|p| p.child("w:pStyle"))
            .and_then(|e| e.get("w:val"))
            .unwrap_or(&self.normal);
        self.chain(pstyle, &mut result, &mut BTreeSet::new())?;
        let rpr = run.and_then(|r| r.child("w:rPr"));
        if let Some(cstyle) = rpr
            .and_then(|p| p.child("w:rStyle"))
            .and_then(|e| e.get("w:val"))
        {
            self.chain(cstyle, &mut result, &mut BTreeSet::new())?;
        }
        result.merge(rpr, false);
        self.resolve_theme(&mut result);
        Ok(result)
    }
    fn resolve_theme(&self, result: &mut RunStyle) {
        let Some(theme) = &self.theme else {
            return;
        };
        let scheme = theme
            .child("a:themeElements")
            .and_then(|e| e.child("a:fontScheme"));
        if let Some(fonts) = result.0.get_mut("w:rFonts") {
            for (theme_key, explicit, script) in [
                ("w:asciiTheme", "w:ascii", "a:latin"),
                ("w:hAnsiTheme", "w:hAnsi", "a:latin"),
                ("w:eastAsiaTheme", "w:eastAsia", "a:ea"),
                ("w:cstheme", "w:cs", "a:cs"),
            ] {
                if let Some(t) = fonts.get(theme_key).map(str::to_owned) {
                    let group = if t.starts_with("major") {
                        "a:majorFont"
                    } else {
                        "a:minorFont"
                    };
                    let font = scheme.and_then(|s| s.child(group));
                    let face = font
                        .and_then(|s| s.child(script))
                        .and_then(|e| e.get("typeface"))
                        .filter(|s| !s.is_empty())
                        .or_else(|| {
                            if script == "a:ea" {
                                font.and_then(|f| {
                                    f.children.iter().filter_map(Xml::element).find(|e| {
                                        e.name == "a:font" && e.get("script") == Some("Hans")
                                    })
                                })
                                .and_then(|e| e.get("typeface"))
                            } else {
                                None
                            }
                        });
                    if let Some(face) = face {
                        fonts.set_attr(explicit, face);
                        fonts.attrs.retain(|(k, _)| k != theme_key);
                    }
                }
            }
        }
    }
    pub fn ensure_export_styles(&mut self) {
        let normal = self.normal.clone();
        for (id, name) in [("Title", "Title"), ("Subtitle", "Subtitle")] {
            if self.style(id).is_none() {
                let mut s = Element::new("w:style")
                    .attr("w:type", "paragraph")
                    .attr("w:styleId", id);
                s.push(val("w:name", name));
                s.push(val("w:basedOn", &normal));
                self.xml.push(s);
            }
        }
        for level in 1..=9 {
            let id = format!("Heading{level}");
            if self.style(&id).is_none() {
                let mut s = Element::new("w:style")
                    .attr("w:type", "paragraph")
                    .attr("w:styleId", &id);
                s.push(val("w:name", format!("heading {level}")));
                s.push(val("w:basedOn", &normal));
                s.push(val("w:next", &normal));
                s.push(Element::new("w:qFormat"));
                let mut p = Element::new("w:pPr");
                p.push(Element::new("w:keepNext"));
                p.push(val("w:outlineLvl", level - 1));
                s.push(p);
                let mut r = Element::new("w:rPr");
                r.push(Element::new("w:b"));
                s.push(r);
                self.xml.push(s);
            }
        }
    }
    pub fn ensure_deep_heading(&mut self, level: usize) {
        let id = format!("WeftextHeading{level}");
        if self.style(&id).is_none() {
            let mut s = Element::new("w:style")
                .attr("w:type", "paragraph")
                .attr("w:styleId", id);
            s.push(val("w:name", format!("Native heading depth {level}")));
            s.push(val("w:basedOn", &self.normal));
            let mut p = Element::new("w:pPr");
            p.push(val("w:outlineLvl", 9));
            s.push(p);
            self.xml.push(s);
        }
    }
}
