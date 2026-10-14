//! Small bounded XML tree for targeted OOXML edits. Unchanged ZIP parts never
//! pass through this serializer. DTDs and non-declaration PIs are rejected.
use super::DocxError;
use quick_xml::{Reader, events::Event};

#[derive(Clone, Debug)]
pub(super) enum Xml {
    Element(Element),
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug)]
pub(super) struct Element {
    /// In-memory authored text provenance; never parsed from or written to XML.
    pub source: Option<(usize, String)>,
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Xml>,
}

impl Element {
    pub fn new(name: &str) -> Self {
        Self {
            source: None,
            name: name.into(),
            attrs: vec![],
            children: vec![],
        }
    }
    pub fn attr(mut self, name: &str, value: impl ToString) -> Self {
        self.set_attr(name, &value.to_string());
        self
    }
    pub fn set_attr(&mut self, name: &str, value: &str) {
        if let Some((_, v)) = self.attrs.iter_mut().find(|(n, _)| n == name) {
            *v = value.into();
        } else {
            self.attrs.push((name.into(), value.into()));
        }
    }
    pub fn get(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children
            .iter()
            .filter_map(Xml::element)
            .find(|e| e.name == name)
    }
    pub fn child_mut(&mut self, name: &str) -> Option<&mut Element> {
        self.children
            .iter_mut()
            .filter_map(Xml::element_mut)
            .find(|e| e.name == name)
    }
    pub fn push(&mut self, child: Element) {
        self.children.push(Xml::Element(child));
    }
    pub fn remove(&mut self, name: &str) {
        self.children
            .retain(|n| n.element().is_none_or(|e| e.name != name));
    }
    pub fn descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Element>) {
        if self.name == name {
            out.push(self);
        }
        for e in self.children.iter().filter_map(Xml::element) {
            e.descendants(name, out);
        }
    }
    pub fn text(&self) -> String {
        let mut s = String::new();
        for n in &self.children {
            match n {
                Xml::Text(t) => s.push_str(t),
                Xml::Element(e) => s.push_str(&e.text()),
                _ => {}
            }
        }
        s
    }
}
impl Xml {
    pub fn element(&self) -> Option<&Element> {
        if let Self::Element(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn element_mut(&mut self) -> Option<&mut Element> {
        if let Self::Element(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Element, DocxError> {
    let source = std::str::from_utf8(bytes).map_err(|_| DocxError::new("模板 XML 必须是 UTF-8"))?;
    let mut reader = Reader::from_str(source);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<Element> = vec![];
    let mut root = None;
    let mut count = 0usize;
    loop {
        count += 1;
        if count > 300_000 || stack.len() > 128 {
            return Err(DocxError::new("模板 XML 超过安全预算"));
        }
        let event = reader
            .read_event()
            .map_err(|e| DocxError::new(format!("无效模板 XML：{e}")))?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|_| DocxError::new("无效 XML 名称"))?
                    .to_owned();
                let mut element = Element::new(&name);
                for a in e.attributes() {
                    let a = a.map_err(|e| DocxError::new(e.to_string()))?;
                    let key = std::str::from_utf8(a.key.as_ref())
                        .map_err(|_| DocxError::new("无效 XML 属性"))?;
                    let value = a
                        .unescape_value()
                        .map_err(|e| DocxError::new(e.to_string()))?;
                    element.attrs.push((key.into(), value.into_owned()));
                }
                if empty {
                    attach(element, &mut stack, &mut root)?;
                } else {
                    stack.push(element);
                }
            }
            Event::End(_) => {
                let e = stack
                    .pop()
                    .ok_or_else(|| DocxError::new("XML 结束标记不匹配"))?;
                attach(e, &mut stack, &mut root)?;
            }
            Event::Text(e) => {
                let raw =
                    std::str::from_utf8(e.as_ref()).map_err(|_| DocxError::new("无效 XML 文本"))?;
                let t = quick_xml::escape::unescape(raw)
                    .map_err(|e| DocxError::new(e.to_string()))?
                    .into_owned();
                if let Some(p) = stack.last_mut() {
                    p.children.push(Xml::Text(t));
                } else if !t.trim().is_empty() {
                    return Err(DocxError::new("XML 根之外存在文本"));
                }
            }
            Event::GeneralRef(e) => {
                let raw =
                    std::str::from_utf8(e.as_ref()).map_err(|_| DocxError::new("无效 XML 实体"))?;
                let t = quick_xml::escape::unescape(&format!("&{raw};"))
                    .map_err(|e| DocxError::new(e.to_string()))?
                    .into_owned();
                stack
                    .last_mut()
                    .ok_or_else(|| DocxError::new("XML 根之外存在实体"))?
                    .children
                    .push(Xml::Text(t));
            }
            Event::CData(e) => {
                let t =
                    std::str::from_utf8(e.as_ref()).map_err(|_| DocxError::new("无效 CDATA"))?;
                if let Some(p) = stack.last_mut() {
                    p.children.push(Xml::Text(t.into()));
                }
            }
            Event::Comment(e) => {
                if let Some(p) = stack.last_mut() {
                    p.children
                        .push(Xml::Comment(String::from_utf8_lossy(e.as_ref()).into()));
                }
            }
            Event::Decl(d) => {
                if let Some(enc) = d.encoding()
                    && !enc
                        .map_err(|e| DocxError::new(e.to_string()))?
                        .as_ref()
                        .eq_ignore_ascii_case(b"utf-8")
                {
                    return Err(DocxError::new("模板 XML 必须是 UTF-8"));
                }
            }
            Event::DocType(_) | Event::PI(_) => {
                return Err(DocxError::new("模板不能包含 DTD 或处理指令"));
            }
            Event::Eof => break,
        }
    }
    if !stack.is_empty() {
        return Err(DocxError::new("模板 XML 未闭合"));
    }
    root.ok_or_else(|| DocxError::new("模板 XML 缺少根元素"))
}
fn attach(e: Element, stack: &mut [Element], root: &mut Option<Element>) -> Result<(), DocxError> {
    if let Some(p) = stack.last_mut() {
        p.push(e);
    } else if root.replace(e).is_some() {
        return Err(DocxError::new("XML 有多个根元素"));
    }
    Ok(())
}
pub(super) fn serialize(root: &Element) -> Vec<u8> {
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>");
    write_element(root, &mut s);
    s.into_bytes()
}
pub(super) fn escape(s: &str) -> String {
    quick_xml::escape::escape(s).into_owned()
}
pub(super) fn write_element(e: &Element, out: &mut String) {
    out.push('<');
    out.push_str(&e.name);
    for (k, v) in &e.attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(&escape(v));
        out.push('"');
    }
    if e.children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    for n in &e.children {
        match n {
            Xml::Element(e) => write_element(e, out),
            Xml::Text(t) => out.push_str(&escape(t)),
            Xml::Comment(t) => {
                out.push_str("<!--");
                out.push_str(t);
                out.push_str("-->");
            }
        }
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}
pub(super) fn val(name: &str, value: impl ToString) -> Element {
    Element::new(name).attr("w:val", value)
}
