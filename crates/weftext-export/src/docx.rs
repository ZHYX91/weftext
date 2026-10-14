//! Editable DOCX conversion of one frozen successful HTML consumer tree.
//! No Typst reparsing, Query evaluation, filesystem access, or resource fetch.
//! Templates use standard Word content controls, bound by their exact tag.
mod math;
mod package;
mod styles;
mod xml;

use crate::{ExportArtifact, ExportError, ExportLoss};
use base64::Engine as _;
use package::{Package, W};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use styles::{RunStyle, Styles};
use weftext_engine::{RenderNode, RenderOutput};
use xml::{Element, Xml, val};

#[derive(Debug)]
struct DocxError(String);
impl DocxError {
    fn new(message: impl ToString) -> Self {
        Self(message.to_string())
    }
}
impl From<DocxError> for ExportError {
    fn from(value: DocxError) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxOptions {
    /// False: template typography wins; true: map supported frozen CSS too.
    #[serde(default)]
    pub preserve_source_styles: bool,
    /// Declining reported losses makes generation fail before producing bytes.
    #[serde(default)]
    pub reject_losses: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DocxBinding {
    Body,
    Title,
    Text {
        value: String,
    },
    /// Already frozen field/Query output. A Query table is a semantic table tree.
    Nodes {
        nodes: Vec<RenderNode>,
    },
}
pub struct DocxInput<'a> {
    pub render: &'a RenderOutput,
    pub template: Option<&'a [u8]>,
    pub bindings: &'a BTreeMap<String, DocxBinding>,
    pub options: DocxOptions,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateControl {
    pub tag: String,
    pub alias: Option<String>,
    pub placement: String,
}

/// Inspect bounded DOCX bytes without executing Word or modifying the template.
pub fn list_template_controls(bytes: &[u8]) -> crate::Result<Vec<TemplateControl>> {
    let package = Package::load(Some(bytes))?;
    let doc = package
        .xml("word/document.xml")?
        .ok_or_else(|| ExportError("模板缺少主文档".into()))?;
    validate_document(&doc)?;
    let mut result = vec![];
    controls(&doc, "", &mut result);
    Ok(result)
}
fn controls(e: &Element, parent: &str, out: &mut Vec<TemplateControl>) {
    if e.name == "w:sdt"
        && let Some(pr) = e.child("w:sdtPr")
        && let Some(tag) = pr.child("w:tag").and_then(|t| t.get("w:val"))
    {
        out.push(TemplateControl {
            tag: tag.into(),
            alias: pr
                .child("w:alias")
                .and_then(|a| a.get("w:val"))
                .map(str::to_owned),
            placement: if matches!(parent, "w:p" | "w:hyperlink") {
                "inline"
            } else if matches!(parent, "w:body" | "w:tc" | "w:sdtContent") {
                "block"
            } else {
                "unsupported"
            }
            .into(),
        });
    }
    for child in e.children.iter().filter_map(Xml::element) {
        controls(child, &e.name, out);
    }
}

pub fn export_docx(input: DocxInput<'_>) -> crate::Result<ExportArtifact> {
    convert(input).map_err(Into::into)
}
fn convert(input: DocxInput<'_>) -> Result<ExportArtifact, DocxError> {
    if !input.render.ok {
        return Err(DocxError::new("DOCX 需要一次成功的冻结 HTML 编译"));
    }
    if input
        .render
        .products
        .as_ref()
        .is_none_or(|p| p.target != "html")
    {
        return Err(DocxError::new("DOCX 默认 profile 需要同次 HTML 编译的产品"));
    }
    let tree = input
        .render
        .tree
        .as_ref()
        .ok_or_else(|| DocxError::new("冻结编译缺少最终语义树"))?;
    check_tree(tree, 0, &mut 0usize)?;
    for binding in input.bindings.values() {
        if let DocxBinding::Text { value } = binding
            && !value.chars().all(valid_xml_char)
        {
            return Err(DocxError::new("字段文字包含 DOCX/XML 不能保留的控制字符"));
        }
        if let DocxBinding::Nodes { nodes } = binding {
            let mut count = 0;
            for n in nodes {
                check_tree(n, 0, &mut count)?;
            }
        }
    }
    let mut package = Package::load(input.template)?;
    let mut doc = package
        .xml("word/document.xml")?
        .ok_or_else(|| DocxError::new("缺少 DOCX 主文档"))?;
    validate_document(&doc)?;
    let style_xml = package
        .xml("word/styles.xml")?
        .unwrap_or(xml::parse(package::DEFAULT_STYLES.as_bytes())?);
    let mut styles = Styles::new(style_xml, package.xml("word/theme/theme1.xml")?)?;
    styles.ensure_export_styles();
    ensure_deep_styles(tree, &mut styles);
    for b in input.bindings.values() {
        if let DocxBinding::Nodes { nodes } = b {
            for n in nodes {
                ensure_deep_styles(n, &mut styles);
            }
        }
    }
    let normal = styles.effective(None, None)?;
    let mut patterns = vec![];
    heading_patterns(tree, &mut patterns);
    for b in input.bindings.values() {
        if let DocxBinding::Nodes { nodes } = b {
            for n in nodes {
                heading_patterns(n, &mut patterns);
            }
        }
    }
    let chosen_pattern = patterns
        .iter()
        .find_map(|(_, p)| p.as_ref().filter(|p| parse_number_pattern(p).is_some()))
        .cloned()
        .unwrap_or_else(|| "1.1".into());
    let (numbering, heading_num, bullet_num) =
        numbering(package.xml("word/numbering.xml")?, &chosen_pattern);
    let names = find_existing_bookmark_names(&doc);
    let mut ctx = Context {
        package: &mut package,
        styles: &styles,
        options: &input.options,
        losses: vec![],
        normal: normal.clone(),
        heading_num,
        heading_abstract: existing_max(&numbering, "w:abstractNum", "w:abstractNumId") - 1,
        heading_state: vec![],
        bullet_num,
        numbering_extra: vec![],
        num_next: bullet_num + 1,
        abstract_next: existing_max(&numbering, "w:abstractNum", "w:abstractNumId") + 1,
        bookmarks: BTreeMap::new(),
        numberable_targets: BTreeSet::new(),
        bookmark_names: names,
        bookmark_next: existing_max(&doc, "w:bookmarkStart", "w:id") + 1,
        reference_stack: vec![],
        drawing_next: existing_max(&doc, "wp:docPr", "id").max(existing_max(
            &doc,
            "pic:cNvPr",
            "id",
        )) + 1,
        body_inserted: false,
        skip_title: input.template.is_some()
            && input
                .bindings
                .values()
                .any(|b| matches!(b, DocxBinding::Title)),
    };
    for (key, pattern) in patterns {
        if pattern.as_ref() != Some(&chosen_pattern) {
            ctx.loss("heading-numbering-loss",format!("此标题使用函数、不支持或不同的编号 pattern；Word 共享原生编号采用 {chosen_pattern}"),Some(key));
        }
    }
    report_unassociated_native_headings(input.render, tree, &mut ctx);
    if input.template.is_none()
        || input
            .bindings
            .values()
            .any(|b| matches!(b, DocxBinding::Body))
    {
        ctx.scan_bookmarks(tree);
    }
    for b in input.bindings.values() {
        if let DocxBinding::Nodes { nodes } = b {
            for n in nodes {
                ctx.scan_bookmarks(n);
            }
        }
    }
    if input.template.is_some() {
        let mut used = BTreeSet::new();
        replace_controls(&mut doc, "", &input, &mut ctx, &mut used)?;
        for tag in input.bindings.keys() {
            if !used.contains(tag) {
                return Err(DocxError::new(format!("模板没有绑定内容控件 tag：{tag}")));
            }
        }
        if !input
            .bindings
            .values()
            .any(|b| matches!(b, DocxBinding::Body))
        {
            ctx.loss(
                "body-not-bound",
                "模板没有绑定正文；输出仅包含明确绑定的字段与原模板内容",
                None,
            );
        }
    } else {
        let body = doc
            .child_mut("w:body")
            .ok_or_else(|| DocxError::new("模板缺少 w:body"))?;
        let sect = body.child("w:sectPr").cloned();
        body.children = ctx
            .blocks(tree, &Layout::normal(&styles, &normal), 0)?
            .into_iter()
            .map(Xml::Element)
            .collect();
        if let Some(sect) = sect {
            body.push(sect);
        }
    }
    for (prefix, url) in [
        (
            "xmlns:r",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
        ),
        (
            "xmlns:m",
            "http://schemas.openxmlformats.org/officeDocument/2006/math",
        ),
        (
            "xmlns:wp",
            "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
        ),
        (
            "xmlns:a",
            "http://schemas.openxmlformats.org/drawingml/2006/main",
        ),
        (
            "xmlns:pic",
            "http://schemas.openxmlformats.org/drawingml/2006/picture",
        ),
    ] {
        doc.set_attr(prefix, url);
    }
    ctx.loss(
        "office-interoperability-unverified",
        "尚未在真实 Word/WPS/LibreOffice 验证字段更新、导航、编号、字体和保存再打开",
        None,
    );
    ctx.loss(
        "font-target-dependency",
        "输出保留可编辑 Unicode 和模板字体/语言；字体未嵌入，目标 Office 需要相应字体或会替换",
        None,
    );
    let mut numbering = numbering;
    for e in ctx.numbering_extra {
        numbering.push(e);
    }
    sort_elements(
        &mut numbering,
        &[
            "w:numPicBullet",
            "w:abstractNum",
            "w:num",
            "w:numIdMacAtCleanup",
        ],
    );
    let losses = ctx.losses;
    if input.options.reject_losses && !losses.is_empty() {
        return Err(DocxError::new(format!(
            "已拒绝 DOCX 损失：{}",
            losses
                .iter()
                .map(|l| l.code.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    package.set_xml("word/document.xml", &doc);
    package.set_xml("word/styles.xml", &styles.xml);
    package.ensure_rel("styles", "styles.xml");
    package.content_type(
        "/word/styles.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
    );
    package.set_xml("word/numbering.xml", &numbering);
    package.ensure_rel("numbering", "numbering.xml");
    package.content_type(
        "/word/numbering.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml",
    );
    let mut settings = package
        .xml("word/settings.xml")?
        .unwrap_or(Element::new("w:settings").attr("xmlns:w", W));
    settings.remove("w:updateFields");
    settings.push(val("w:updateFields", "true"));
    package.set_xml("word/settings.xml", &settings);
    package.ensure_rel("settings", "settings.xml");
    package.content_type(
        "/word/settings.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml",
    );
    // DOTX templates become an ordinary DOCX output, without other part edits.
    for e in package
        .types
        .children
        .iter_mut()
        .filter_map(Xml::element_mut)
    {
        if e.get("PartName") == Some("/word/document.xml") {
            e.set_attr(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
            );
        }
    }
    Ok(ExportArtifact {
        bytes: package.finish()?,
        content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            .into(),
        extension: "docx".into(),
        losses,
        summary: if input.template.is_some() {
            "已按 Word 内容控件绑定生成可编辑 DOCX"
        } else {
            "已按冻结 HTML 语义生成可编辑 DOCX"
        }
        .into(),
    })
}
fn check_tree(n: &RenderNode, depth: usize, count: &mut usize) -> Result<(), DocxError> {
    *count += 1;
    if depth > 128 || *count > 100_000 {
        return Err(DocxError::new("DOCX 语义树超过安全预算"));
    }
    if n.text.as_ref().is_some_and(|t| t.len() > 16 * 1024 * 1024) {
        return Err(DocxError::new("DOCX 文本超出预算"));
    }
    if n.text
        .as_ref()
        .is_some_and(|t| !t.chars().all(valid_xml_char))
    {
        return Err(DocxError::new("显示文字包含 DOCX/XML 不能保留的控制字符"));
    }
    for c in &n.children {
        check_tree(c, depth + 1, count)?;
    }
    Ok(())
}
fn ensure_deep_styles(n: &RenderNode, styles: &mut Styles) {
    if n.attrs
        .get("data-weftext-role")
        .is_some_and(|r| r == "heading")
        && let Some(level) = n
            .attrs
            .get("data-weftext-level")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 9)
    {
        styles.ensure_deep_heading(level);
    }
    for c in &n.children {
        ensure_deep_styles(c, styles);
    }
}
fn heading_patterns(n: &RenderNode, out: &mut Vec<(String, Option<String>)>) {
    if n.attrs
        .get("data-weftext-role")
        .is_some_and(|r| r == "heading")
        && n.attrs
            .get("data-weftext-numbered")
            .is_some_and(|v| v == "true")
    {
        out.push((
            n.key.clone(),
            n.attrs.get("data-weftext-numbering-pattern").cloned(),
        ));
    }
    for c in &n.children {
        heading_patterns(c, out);
    }
}
fn find_existing_bookmark_names(doc: &Element) -> BTreeSet<String> {
    let mut v = vec![];
    doc.descendants("w:bookmarkStart", &mut v);
    v.into_iter()
        .filter_map(|e| e.get("w:name").map(str::to_owned))
        .collect()
}
fn report_unassociated_native_headings(
    render: &RenderOutput,
    tree: &RenderNode,
    ctx: &mut Context<'_>,
) {
    type OriginKey = (Option<String>, usize, usize);
    fn gather(n: &RenderNode, out: &mut BTreeMap<OriginKey, usize>) {
        if n.attrs
            .get("data-weftext-role")
            .is_some_and(|r| r == "heading")
        {
            let key = n
                .origin
                .as_ref()
                .map(|o| (Some(o.path.clone()), o.from, o.to))
                .unwrap_or((None, 0, 0));
            *out.entry(key).or_default() += 1;
        }
        for c in &n.children {
            gather(c, out);
        }
    }
    let mut associated = BTreeMap::new();
    gather(tree, &mut associated);
    for h in &render.products.as_ref().unwrap().headings {
        let key = (h.path.clone(), h.from, h.to);
        if let Some(left) = associated.get_mut(&key).filter(|v| **v > 0) {
            *left -= 1;
        } else {
            ctx.loss("native-heading-role-unavailable","原生标题已隐藏或经自定义 show 改写，缺少可证明的最终标题角色；保留实际输出内容，未补建原生 Word 标题",h.path.as_ref().map(|p|format!("{p}:{}..{}",h.from,h.to)));
        }
    }
}
fn validate_document(doc: &Element) -> Result<(), DocxError> {
    if doc.name != "w:document" || doc.get("xmlns:w") != Some(W) || doc.child("w:body").is_none() {
        return Err(DocxError::new(
            "只支持标准 transitional OOXML 的 w:document 模板",
        ));
    }
    Ok(())
}
fn existing_max(doc: &Element, tag: &str, attr: &str) -> usize {
    let mut v = vec![];
    doc.descendants(tag, &mut v);
    v.iter()
        .filter_map(|e| e.get(attr).and_then(|s| s.parse().ok()))
        .max()
        .unwrap_or(0)
}

#[derive(Clone)]
struct Layout {
    ppr: Element,
    body: RunStyle,
}
impl Layout {
    fn normal(styles: &Styles, body: &RunStyle) -> Self {
        let mut ppr = Element::new("w:pPr");
        ppr.push(val("w:pStyle", styles.normal()));
        ppr.push(numpr(0, 0));
        Self {
            ppr,
            body: body.clone(),
        }
    }
    fn placeholder(
        content: &Element,
        styles: &Styles,
        normal: &RunStyle,
    ) -> Result<Self, DocxError> {
        let mut paragraphs = vec![];
        content.descendants("w:p", &mut paragraphs);
        let p = paragraphs.first().copied();
        let mut runs = vec![];
        if let Some(p) = p {
            p.descendants("w:r", &mut runs);
        } else {
            content.descendants("w:r", &mut runs);
        }
        let mut ppr = p
            .and_then(|p| p.child("w:pPr"))
            .cloned()
            .unwrap_or_else(|| Layout::normal(styles, normal).ppr);
        for n in ["w:sectPr", "w:numPr", "w:outlineLvl"] {
            ppr.remove(n);
        }
        ppr.push(numpr(0, 0));
        Ok(Self {
            ppr,
            body: styles.effective(p, runs.first().copied())?,
        })
    }
}
fn replace_controls(
    e: &mut Element,
    parent: &str,
    input: &DocxInput<'_>,
    ctx: &mut Context<'_>,
    used: &mut BTreeSet<String>,
) -> Result<(), DocxError> {
    if e.name == "w:sdt" {
        let tag = e
            .child("w:sdtPr")
            .and_then(|p| p.child("w:tag"))
            .and_then(|t| t.get("w:val"))
            .map(str::to_owned);
        if let Some(tag) = tag {
            if let Some(binding) = input.bindings.get(&tag) {
                if !matches!(
                    parent,
                    "w:p" | "w:hyperlink" | "w:body" | "w:tc" | "w:sdtContent"
                ) {
                    return Err(DocxError::new(format!("内容控件 {tag} 的结构位置暂不支持")));
                }
                let first = used.insert(tag.clone());
                if !first && matches!(binding, DocxBinding::Body | DocxBinding::Nodes { .. }) {
                    return Err(DocxError::new(
                        "同一结构绑定不能放入多个内容控件；请为每个独立输出位置冻结独立节点",
                    ));
                }
                let content = e
                    .child("w:sdtContent")
                    .ok_or_else(|| DocxError::new("内容控件缺少 sdtContent"))?;
                let layout = Layout::placeholder(content, ctx.styles, &ctx.normal)?;
                let inline = matches!(parent, "w:p" | "w:hyperlink");
                let title = || {
                    input
                        .render
                        .products
                        .as_ref()
                        .and_then(|p| p.title.as_deref())
                        .ok_or_else(|| DocxError::new("冻结产品没有唯一 title；不能为模板发明标题"))
                };
                let nodes = match binding {
                    DocxBinding::Body => {
                        if ctx.body_inserted {
                            return Err(DocxError::new(
                                "正文只能绑定一个块级内容控件；重复正文需要独立编译的出现位置与路由",
                            ));
                        }
                        ctx.body_inserted = true;
                        if inline {
                            return Err(DocxError::new("正文绑定需要块级内容控件"));
                        }
                        ctx.blocks(input.render.tree.as_ref().unwrap(), &layout, 0)?
                    }
                    DocxBinding::Nodes { nodes } => {
                        let mut out = vec![];
                        for n in nodes {
                            if inline {
                                if is_block(n) {
                                    return Err(DocxError::new(
                                        "表格/段落 Query 结果需要块级内容控件",
                                    ));
                                }
                                out.extend(ctx.inline(n, &RunStyle::default(), None)?);
                            } else {
                                out.extend(ctx.blocks(n, &layout, 0)?);
                            }
                        }
                        out
                    }
                    DocxBinding::Text { value } => {
                        let runs = vec![text_run(value, &layout.body.properties())];
                        if inline {
                            runs
                        } else {
                            vec![paragraph(layout.ppr.clone(), runs)]
                        }
                    }
                    DocxBinding::Title => {
                        let runs = vec![text_run(title()?, &layout.body.properties())];
                        if inline {
                            runs
                        } else {
                            vec![paragraph(layout.ppr.clone(), runs)]
                        }
                    }
                };
                let content = e.child_mut("w:sdtContent").unwrap();
                content.children = nodes.into_iter().map(Xml::Element).collect();
                if !inline && content.children.is_empty() {
                    content.push(paragraph(layout.ppr, vec![]));
                }
                if let Some(pr) = e.child_mut("w:sdtPr") {
                    for n in [
                        "w:dataBinding",
                        "w:showingPlcHdr",
                        "w:placeholder",
                        "w:lock",
                        "w:temporary",
                    ] {
                        pr.remove(n);
                    }
                    if !inline {
                        pr.remove("w:text");
                    }
                }
                return Ok(());
            } else {
                ctx.loss(
                    "template-control-unbound",
                    format!("模板内容控件 {tag} 未绑定；保留其原内容"),
                    Some(tag),
                );
            }
        }
    }
    let own = e.name.clone();
    for c in e.children.iter_mut().filter_map(Xml::element_mut) {
        replace_controls(c, &own, input, ctx, used)?;
    }
    Ok(())
}

struct Context<'a> {
    package: &'a mut Package,
    styles: &'a Styles,
    options: &'a DocxOptions,
    losses: Vec<ExportLoss>,
    normal: RunStyle,
    heading_num: usize,
    heading_abstract: usize,
    heading_state: Vec<u64>,
    bullet_num: usize,
    numbering_extra: Vec<Element>,
    num_next: usize,
    abstract_next: usize,
    bookmarks: BTreeMap<String, Option<String>>,
    numberable_targets: BTreeSet<String>,
    bookmark_names: BTreeSet<String>,
    bookmark_next: usize,
    reference_stack: Vec<Option<String>>,
    drawing_next: usize,
    body_inserted: bool,
    skip_title: bool,
}
impl Context<'_> {
    fn loss(&mut self, code: &str, message: impl ToString, location: Option<String>) {
        let message = message.to_string();
        if !self
            .losses
            .iter()
            .any(|l| l.code == code && l.message == message && l.location == location)
        {
            self.losses.push(ExportLoss {
                code: code.into(),
                message,
                location,
            });
        }
    }
    fn scan_bookmarks(&mut self, n: &RenderNode) {
        if self.skip_title
            && n.attrs
                .get("data-weftext-role")
                .is_some_and(|r| r == "title")
        {
            return;
        }
        if css(n).get("display").is_some_and(|v| v == "none") {
            return;
        }
        if let Some(id) = n.attrs.get("id") {
            if n.attrs
                .get("data-weftext-role")
                .is_some_and(|r| r == "heading")
                && n.attrs
                    .get("data-weftext-numbered")
                    .is_some_and(|r| r == "true")
                && n.attrs
                    .get("data-weftext-level")
                    .and_then(|v| v.parse::<usize>().ok())
                    .is_some_and(|v| (1..=9).contains(&v))
            {
                self.numberable_targets.insert(id.clone());
            }
            let name = loop {
                let name = format!("weftext_b{}", self.bookmark_next);
                self.bookmark_next += 1;
                if self.bookmark_names.insert(name.clone()) {
                    break name;
                }
            };
            self.bookmarks
                .entry(id.clone())
                .and_modify(|v| *v = None)
                .or_insert(Some(name));
        }
        for c in &n.children {
            self.scan_bookmarks(c);
        }
    }
    fn bookmark(&mut self, n: &RenderNode, runs: Vec<Element>) -> Vec<Element> {
        if let Some(name) = n
            .attrs
            .get("id")
            .and_then(|id| self.bookmarks.get(id))
            .and_then(Option::as_ref)
        {
            let id = self.bookmark_next;
            self.bookmark_next += 1;
            let mut out = vec![
                Element::new("w:bookmarkStart")
                    .attr("w:id", id)
                    .attr("w:name", name),
            ];
            out.extend(runs);
            out.push(Element::new("w:bookmarkEnd").attr("w:id", id));
            out
        } else {
            runs
        }
    }
    fn blocks(
        &mut self,
        n: &RenderNode,
        layout: &Layout,
        list_depth: usize,
    ) -> Result<Vec<Element>, DocxError> {
        if n.kind == "tag" {
            return Ok(vec![]);
        }
        if css(n).get("display").is_some_and(|v| v == "none") {
            return Ok(vec![]);
        }
        let tag = n.tag.as_deref().unwrap_or("");
        let role = n
            .attrs
            .get("data-weftext-role")
            .map(String::as_str)
            .unwrap_or("");
        if self.skip_title && role == "title" {
            return Ok(vec![]);
        }
        if n.kind == "frame"
            || n.attrs
                .get("class")
                .is_some_and(|v| v == "weftext-unsupported")
        {
            self.loss(
                "unsupported-layout",
                "原生分页区域或不支持的 HTML 结构只保留可编辑说明",
                Some(n.key.clone()),
            );
            return Ok(vec![paragraph(
                layout.ppr.clone(),
                vec![text_run(
                    n.text.as_deref().unwrap_or("[无法转换的排版区域]"),
                    &layout.body.properties(),
                )],
            )]);
        }
        if tag == "table" {
            return Ok(vec![self.table(n, layout, list_depth)?]);
        }
        if matches!(tag, "ul" | "ol") {
            return self.list(n, layout, list_depth);
        }
        if matches!(role, "title" | "subtitle" | "heading")
            || matches!(
                tag,
                "p" | "pre" | "blockquote" | "figcaption" | "dt" | "dd" | "li"
            )
        {
            return Ok(vec![self.semantic_paragraph(n, layout, None)?]);
        }
        if matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
            || n.attrs.get("role").is_some_and(|r| r == "heading")
        {
            self.loss(
                "unassociated-heading",
                "显示树中的 heading 缺少原生角色关联；保留其实际内容为普通段落，不猜作者层级",
                Some(n.key.clone()),
            );
            return Ok(vec![self.semantic_paragraph(n, layout, None)?]);
        }
        if matches!(
            tag,
            "html"
                | "body"
                | "main"
                | "article"
                | "section"
                | "div"
                | "figure"
                | "header"
                | "footer"
                | "nav"
                | ""
        ) && n.kind != "text"
        {
            let mut out = vec![];
            let mut inline = vec![];
            for c in &n.children {
                if c.kind == "tag" {
                    continue;
                }
                if is_block(c) {
                    if !inline.is_empty() {
                        out.push(paragraph(layout.ppr.clone(), std::mem::take(&mut inline)));
                    }
                    out.extend(self.blocks(c, layout, list_depth)?);
                } else {
                    inline.extend(self.inline(c, &layout.body, None)?);
                }
            }
            if !inline.is_empty() {
                out.push(paragraph(layout.ppr.clone(), inline));
            }
            return Ok(out);
        }
        Ok(vec![paragraph(
            layout.ppr.clone(),
            self.inline(n, &layout.body, None)?,
        )])
    }
    fn semantic_paragraph(
        &mut self,
        n: &RenderNode,
        layout: &Layout,
        num: Option<(usize, usize)>,
    ) -> Result<Element, DocxError> {
        let mut ppr = layout.ppr.clone();
        let role = n
            .attrs
            .get("data-weftext-role")
            .map(String::as_str)
            .unwrap_or("");
        let run_in = n.children.iter().position(|c| {
            c.attrs
                .get("data-weftext-role")
                .is_some_and(|r| r == "heading")
                && (c
                    .attrs
                    .get("data-weftext-run-in")
                    .is_some_and(|v| v == "true")
                    || css(c).get("display").is_some_and(|v| v == "run-in"))
        });
        let heading = if role == "heading" {
            Some(n)
        } else {
            run_in.map(|i| &n.children[i])
        };
        let mut heading_style = None;
        if let Some(h) = heading {
            let level = h
                .attrs
                .get("data-weftext-level")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            if (1..=9).contains(&level) {
                ppr.remove("w:pStyle");
                ppr.children
                    .insert(0, Xml::Element(val("w:pStyle", format!("Heading{level}"))));
                ppr.push(val("w:outlineLvl", level - 1));
                heading_style = Some(level);
                if h.attrs
                    .get("data-weftext-numbered")
                    .is_some_and(|v| v == "true")
                {
                    let num_id = self.heading_number(h, level)?;
                    set_numpr(&mut ppr, num_id, level - 1);
                } else {
                    set_numpr(&mut ppr, 0, 0);
                }
            } else {
                self.loss("heading-depth-unsupported",format!("作者标题深度 {level} 超出 Word Heading 1–9；保留文字与深度，无法提供等价原生编号/outline"),Some(h.key.clone()));
                ppr.remove("w:pStyle");
                ppr.children.insert(
                    0,
                    Xml::Element(val("w:pStyle", format!("WeftextHeading{level}"))),
                );
                ppr.push(val("w:outlineLvl", 9));
            }
            if run_in.is_some() {
                if let Some(index) = run_in
                    && h.attrs
                        .get("data-weftext-numbered")
                        .is_some_and(|v| v == "true")
                    && n.children[..index]
                        .iter()
                        .any(|c| !math::plain(c).trim().is_empty())
                {
                    self.loss("run-in-leading-body-loss","同段标题之前已有正文；Word 原生段落编号位于段首，不能保留编号在前文之后的位置。前文及后文保留正文字符格式，整段仍属于 Heading",Some(h.key.clone()));
                }
                if !layout.body.0.contains_key("w:sz") || !layout.body.0.contains_key("w:rFonts") {
                    return Err(DocxError::new(
                        "同段标题正文无法解析模板正文的有效字体/字号；请在模板正文样式或占位段落指定字体与字号",
                    ));
                }
                self.loss(
                    "run-in-toc-whole-paragraph",
                    "同段标题使用整个 Heading 段落；Word 普通目录和导航可能包含后面的正文",
                    Some(h.key.clone()),
                );
            }
        } else if matches!(role, "title" | "subtitle") {
            ppr.remove("w:pStyle");
            ppr.children.insert(
                0,
                Xml::Element(val(
                    "w:pStyle",
                    if role == "title" { "Title" } else { "Subtitle" },
                )),
            );
        }
        if let Some((id, level)) = num {
            set_numpr(&mut ppr, id, level);
        }
        if self.options.preserve_source_styles {
            let styles = css(n);
            if let Some(align) = styles.get("text-align")
                && matches!(align.as_str(), "left" | "right" | "center" | "justify")
            {
                ppr.remove("w:jc");
                ppr.push(val("w:jc", if align == "justify" { "both" } else { align }));
            }
        }
        let mut runs = vec![];
        if let Some(index) = run_in {
            for (i, c) in n.children.iter().enumerate() {
                let style = if i == index {
                    RunStyle::default()
                } else {
                    let mut s = RunStyle::default();
                    s.merge(Some(&layout.body.reset()), false);
                    s
                };
                runs.extend(self.inline(c, &style, heading_style)?);
            }
        } else {
            let base = if heading.is_some() || matches!(role, "title" | "subtitle") {
                RunStyle::default()
            } else {
                layout.body.clone()
            };
            if n.children.is_empty() {
                runs.extend(self.inline(n, &base, heading_style)?);
            } else {
                for c in &n.children {
                    runs.extend(self.inline(c, &base, heading_style)?);
                }
            }
        }
        runs = self.bookmark(n, runs);
        Ok(paragraph(ppr, runs))
    }
    fn inline(
        &mut self,
        n: &RenderNode,
        base: &RunStyle,
        heading_level: Option<usize>,
    ) -> Result<Vec<Element>, DocxError> {
        if n.kind == "tag" {
            return Ok(vec![]);
        }
        if css(n).get("display").is_some_and(|v| v == "none") {
            return Ok(vec![]);
        }
        if n.attrs
            .get("data-weftext-role")
            .is_some_and(|r| r == "heading-prefix")
            && heading_level.is_some()
        {
            return Ok(vec![]);
        }
        let tag = n.tag.as_deref().unwrap_or("");
        let mut style = base.clone();
        if tag == "strong" {
            style.0.insert("w:b".into(), val("w:b", "1"));
            style.0.insert("w:bCs".into(), val("w:bCs", "1"));
        }
        if tag == "em" {
            style.0.insert("w:i".into(), val("w:i", "1"));
            style.0.insert("w:iCs".into(), val("w:iCs", "1"));
        }
        if matches!(tag, "s" | "del") {
            style.0.insert("w:strike".into(), val("w:strike", "1"));
        }
        if tag == "u" {
            style.0.insert("w:u".into(), val("w:u", "single"));
        }
        if matches!(tag, "sub" | "sup") {
            style.0.insert(
                "w:vertAlign".into(),
                val(
                    "w:vertAlign",
                    if tag == "sub" {
                        "subscript"
                    } else {
                        "superscript"
                    },
                ),
            );
        }
        if let Some(lang) = n.attrs.get("lang") {
            style.0.insert(
                "w:lang".into(),
                Element::new("w:lang")
                    .attr("w:val", lang)
                    .attr("w:eastAsia", lang),
            );
        }
        if n.attrs.get("dir").is_some_and(|s| s == "rtl") {
            style.0.insert("w:rtl".into(), val("w:rtl", "1"));
        }
        if self.options.preserve_source_styles {
            self.source_style(n, &mut style);
        }
        if n.attrs
            .get("data-weftext-role")
            .is_some_and(|r| r == "reference")
        {
            let bookmark = find_href(n)
                .and_then(|href| href.strip_prefix('#'))
                .filter(|target| self.numberable_targets.contains(*target))
                .and_then(|target| self.bookmarks.get(target))
                .and_then(Clone::clone);
            self.reference_stack.push(bookmark);
            let mut out = vec![];
            for c in &n.children {
                out.extend(self.inline(c, &style, heading_level)?);
            }
            self.reference_stack.pop();
            return Ok(out);
        }
        if n.attrs
            .get("data-weftext-role")
            .is_some_and(|r| r == "reference-number")
        {
            if let Some(Some(bookmark)) = self.reference_stack.last() {
                let mut field = Element::new("w:fldSimple")
                    .attr("w:instr", format!(" REF {bookmark} \\w \\h "))
                    .attr("w:dirty", "true");
                field.push(text_run(&math::plain(n), &style.properties()));
                return Ok(vec![field]);
            } else {
                self.loss(
                    "number-reference-unavailable",
                    "原生编号引用没有唯一导出 bookmark；保留冻结文字，不能动态更新",
                    Some(n.key.clone()),
                );
            }
        }
        if n.kind == "text" {
            return Ok(vec![text_run(
                n.text.as_deref().unwrap_or(""),
                &style.properties(),
            )]);
        }
        if tag == "br" {
            let mut r = Element::new("w:r");
            r.push(style.properties());
            r.push(Element::new("w:br"));
            return Ok(vec![r]);
        }
        if tag == "img" {
            return self.image(n, &style);
        }
        if tag == "math" {
            return match math::convert(n) {
                Ok(m) => Ok(vec![m]),
                Err(reason) => {
                    self.loss("math-structure-loss", reason, Some(n.key.clone()));
                    Ok(vec![text_run(&math::plain(n), &style.properties())])
                }
            };
        }
        if tag == "a" {
            let mut runs = vec![];
            for c in &n.children {
                runs.extend(self.inline(c, &style, heading_level)?);
            }
            let href = n.attrs.get("href").map(String::as_str).unwrap_or("");
            let mut link = Element::new("w:hyperlink");
            if let Some(target) = href.strip_prefix('#') {
                if let Some(Some(bookmark)) = self.bookmarks.get(target) {
                    link.set_attr("w:anchor", bookmark);
                } else {
                    self.loss(
                        "link-target-unavailable",
                        format!("显示链接 {href} 没有唯一导出 bookmark"),
                        Some(n.key.clone()),
                    );
                    return Ok(runs);
                }
            } else if external_url(href) {
                let id = self.package.relationship("hyperlink", href, true);
                link.set_attr("r:id", &id);
            } else {
                self.loss(
                    "offline-link-unavailable",
                    format!("显示链接 {href} 缺少同次编译的离线路由；保留可编辑文字"),
                    Some(n.key.clone()),
                );
                return Ok(runs);
            }
            link.children = runs.into_iter().map(Xml::Element).collect();
            return Ok(vec![link]);
        }
        if n.kind == "frame" {
            self.loss(
                "unsupported-layout",
                "局部分页区域无法转换为等价可编辑 DOCX",
                Some(n.key.clone()),
            );
        }
        let mut out = vec![];
        if let Some(text) = &n.text {
            out.push(text_run(text, &style.properties()));
        }
        for c in &n.children {
            if is_block(c)
                && !matches!(
                    c.attrs.get("data-weftext-role").map(String::as_str),
                    Some("heading")
                )
            {
                self.loss(
                    "nested-block-in-paragraph",
                    "段落内嵌块结构降为可编辑文字；布局不等价",
                    Some(c.key.clone()),
                );
            }
            out.extend(self.inline(c, &style, heading_level)?);
        }
        Ok(self.bookmark(n, out))
    }
    fn source_style(&mut self, n: &RenderNode, style: &mut RunStyle) {
        for (key, value) in css(n) {
            match key.as_str() {
                "font-family" => {
                    let font = value
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches(['\'', '"']);
                    if !font.is_empty() {
                        style.0.insert(
                            "w:rFonts".into(),
                            Element::new("w:rFonts")
                                .attr("w:ascii", font)
                                .attr("w:hAnsi", font)
                                .attr("w:eastAsia", font)
                                .attr("w:cs", font),
                        );
                    }
                }
                "font-size" => {
                    if let Some(size) = css_points(&value) {
                        if (1.0..=800.0).contains(&size) {
                            style
                                .0
                                .insert("w:sz".into(), val("w:sz", (size * 2.).round() as usize));
                            style.0.insert(
                                "w:szCs".into(),
                                val("w:szCs", (size * 2.).round() as usize),
                            );
                        }
                    } else {
                        self.loss(
                            "source-style-unavailable",
                            format!("无法映射源字体大小 {value}"),
                            Some(n.key.clone()),
                        );
                    }
                }
                "font-weight" => {
                    let bold = value == "bold" || value.parse::<u16>().is_ok_and(|w| w >= 600);
                    style
                        .0
                        .insert("w:b".into(), val("w:b", if bold { "1" } else { "0" }));
                }
                "font-style" => {
                    style.0.insert(
                        "w:i".into(),
                        val(
                            "w:i",
                            if value == "italic" || value == "oblique" {
                                "1"
                            } else {
                                "0"
                            },
                        ),
                    );
                }
                "color" => {
                    if let Some(c) = hex_color(&value) {
                        style.0.insert("w:color".into(), val("w:color", c));
                    } else {
                        self.loss(
                            "source-style-unavailable",
                            format!("无法映射源颜色 {value}"),
                            Some(n.key.clone()),
                        );
                    }
                }
                "background-color" => {
                    if let Some(c) = hex_color(&value) {
                        style.0.insert(
                            "w:shd".into(),
                            Element::new("w:shd")
                                .attr("w:val", "clear")
                                .attr("w:fill", c),
                        );
                    }
                }
                "display" | "white-space" | "text-align" | "list-style-type"
                | "border-collapse" => {}
                _ => self.loss(
                    "source-style-loss",
                    format!("DOCX 暂未映射源样式 {key}: {value}"),
                    Some(n.key.clone()),
                ),
            }
        }
    }
    fn image(&mut self, n: &RenderNode, style: &RunStyle) -> Result<Vec<Element>, DocxError> {
        let src = n.attrs.get("src").map(String::as_str).unwrap_or("");
        let Some((mime, encoded)) = src
            .strip_prefix("data:")
            .and_then(|s| s.split_once(";base64,"))
        else {
            self.loss(
                "image-unavailable",
                "图片没有获权冻结的内联字节；保留说明",
                Some(n.key.clone()),
            );
            return Ok(vec![text_run(
                n.attrs
                    .get("alt")
                    .map(String::as_str)
                    .unwrap_or("[图片不可用]"),
                &style.properties(),
            )]);
        };
        if encoded.len() > 12 * 1024 * 1024 {
            return Err(DocxError::new("图片超过导出安全预算"));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| DocxError::new("图片 Base64 无效"))?;
        let Some((extension, width, height)) = image_dimensions(mime, &bytes) else {
            self.loss(
                "image-format-loss",
                format!("图片 {mime} 不支持或无法验证尺寸；保留可编辑说明"),
                Some(n.key.clone()),
            );
            return Ok(vec![text_run(
                n.attrs
                    .get("alt")
                    .map(String::as_str)
                    .unwrap_or("[图片格式不可用]"),
                &style.properties(),
            )]);
        };
        if width == 0 || height == 0 || width > 100_000 || height > 100_000 {
            return Err(DocxError::new("图片尺寸无效或超过预算"));
        }
        let rel = self.package.media(extension, mime, bytes);
        let id = self.drawing_next;
        self.drawing_next += 1;
        let scale = (5_486_400f64 / (f64::from(width) * 9525.)).min(1.0);
        let cx = (f64::from(width) * 9525. * scale).round() as u64;
        let cy = (f64::from(height) * 9525. * scale).round() as u64;
        let mut inline = Element::new("wp:inline")
            .attr("distT", "0")
            .attr("distB", "0")
            .attr("distL", "0")
            .attr("distR", "0");
        inline.push(Element::new("wp:extent").attr("cx", cx).attr("cy", cy));
        inline.push(
            Element::new("wp:docPr")
                .attr("id", id)
                .attr("name", format!("Weftext image {id}"))
                .attr(
                    "descr",
                    n.attrs.get("alt").map(String::as_str).unwrap_or(""),
                ),
        );
        let mut frame = Element::new("wp:cNvGraphicFramePr");
        frame.push(Element::new("a:graphicFrameLocks").attr("noChangeAspect", "1"));
        inline.push(frame);
        let mut graphic = Element::new("a:graphic");
        let mut data = Element::new("a:graphicData").attr(
            "uri",
            "http://schemas.openxmlformats.org/drawingml/2006/picture",
        );
        let mut pic = Element::new("pic:pic");
        let mut nv = Element::new("pic:nvPicPr");
        nv.push(
            Element::new("pic:cNvPr")
                .attr("id", id)
                .attr("name", format!("image{id}.{extension}")),
        );
        nv.push(Element::new("pic:cNvPicPr"));
        pic.push(nv);
        let mut fill = Element::new("pic:blipFill");
        fill.push(Element::new("a:blip").attr("r:embed", rel));
        let mut stretch = Element::new("a:stretch");
        stretch.push(Element::new("a:fillRect"));
        fill.push(stretch);
        pic.push(fill);
        let mut shape = Element::new("pic:spPr");
        let mut transform = Element::new("a:xfrm");
        transform.push(Element::new("a:off").attr("x", 0).attr("y", 0));
        transform.push(Element::new("a:ext").attr("cx", cx).attr("cy", cy));
        shape.push(transform);
        let mut geom = Element::new("a:prstGeom").attr("prst", "rect");
        geom.push(Element::new("a:avLst"));
        shape.push(geom);
        pic.push(shape);
        data.push(pic);
        graphic.push(data);
        inline.push(graphic);
        let mut drawing = Element::new("w:drawing");
        drawing.push(inline);
        let mut run = Element::new("w:r");
        run.push(drawing);
        Ok(vec![run])
    }
    fn ordered_definition(&mut self, depth: usize, start: usize) -> (usize, usize) {
        let aid = self.abstract_next;
        self.abstract_next += 1;
        let mut a = Element::new("w:abstractNum").attr("w:abstractNumId", aid);
        a.push(val("w:multiLevelType", "multilevel"));
        for level in 0..9 {
            let mut l = Element::new("w:lvl").attr("w:ilvl", level);
            l.push(val("w:start", 1));
            l.push(val("w:numFmt", "decimal"));
            l.push(val("w:lvlText", format!("%{}.", level + 1)));
            l.push(val("w:lvlJc", "left"));
            let mut p = Element::new("w:pPr");
            p.push(
                Element::new("w:ind")
                    .attr("w:left", 720 * (level + 1))
                    .attr("w:hanging", 360),
            );
            l.push(p);
            a.push(l);
        }
        self.numbering_extra.push(a);
        let num = self.number_instance(aid, depth, start);
        (aid, num)
    }
    fn heading_number(&mut self, n: &RenderNode, level: usize) -> Result<usize, DocxError> {
        self.heading_state.resize(level, 0);
        self.heading_state.truncate(level);
        self.heading_state[level - 1] = self.heading_state[level - 1]
            .checked_add(1)
            .ok_or_else(|| DocxError::new("标题计数溢出"))?;
        let state = n
            .attrs
            .get("data-weftext-counter-state")
            .and_then(|v| serde_json::from_str::<Vec<u64>>(v).ok())
            .filter(|v| v.len() == level && v.iter().all(|n| *n <= 2_147_483_647));
        if let Some(state) = state {
            if state != self.heading_state {
                let id = self.num_next;
                self.num_next += 1;
                let mut instance = Element::new("w:num").attr("w:numId", id);
                instance.push(val("w:abstractNumId", self.heading_abstract));
                for (i, count) in state.iter().enumerate() {
                    let mut ov = Element::new("w:lvlOverride").attr("w:ilvl", i);
                    ov.push(val("w:startOverride", count));
                    instance.push(ov);
                }
                self.numbering_extra.push(instance);
                self.heading_num = id;
                self.heading_state = state;
            }
        } else {
            self.loss("heading-counter-state-unavailable","冻结标题未提供准确原生计数状态；Word 动态编号采用标准层级，不能证明保留手工 counter 更新",Some(n.key.clone()));
        }
        Ok(self.heading_num)
    }
    fn number_instance(&mut self, aid: usize, depth: usize, start: usize) -> usize {
        let id = self.num_next;
        self.num_next += 1;
        let mut n = Element::new("w:num").attr("w:numId", id);
        n.push(val("w:abstractNumId", aid));
        let mut ov = Element::new("w:lvlOverride").attr("w:ilvl", depth);
        ov.push(val("w:startOverride", start));
        n.push(ov);
        self.numbering_extra.push(n);
        id
    }
    fn list(
        &mut self,
        n: &RenderNode,
        layout: &Layout,
        depth: usize,
    ) -> Result<Vec<Element>, DocxError> {
        let ordered = n.tag.as_deref() == Some("ol");
        let literal = css(n).get("list-style-type").is_some_and(|v| v == "none");
        let native_depth = depth.min(8);
        if depth > 8 {
            self.loss(
                "list-depth-loss",
                "Word 原生列表最多九级；更深列表保留文字并报告层级损失",
                Some(n.key.clone()),
            );
        }
        if literal {
            self.loss(
                "list-marker-literal",
                "此冻结列表有自定义可见标记；保留其文字，未声称标记可自动重新编号",
                Some(n.key.clone()),
            );
        }
        let start = n
            .attrs
            .get("start")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let (aid, mut id) = if ordered {
            self.ordered_definition(native_depth, start)
        } else {
            (0, self.bullet_num)
        };
        let mut out = vec![];
        for li in n.children.iter().filter(|c| c.tag.as_deref() == Some("li")) {
            if ordered && let Some(value) = li.attrs.get("value").and_then(|v| v.parse().ok()) {
                id = self.number_instance(aid, native_depth, value);
            }
            let mut inline = vec![];
            let mut first = true;
            for child in &li.children {
                if child.kind == "tag" {
                    continue;
                }
                if is_block(child) {
                    if !inline.is_empty() {
                        let mut ppr = layout.ppr.clone();
                        if first && !literal && depth <= 8 {
                            set_numpr(&mut ppr, id, native_depth);
                        }
                        out.push(paragraph(ppr, std::mem::take(&mut inline)));
                        first = false;
                    }
                    if matches!(child.tag.as_deref(), Some("ul" | "ol")) {
                        out.extend(self.list(child, layout, depth + 1)?);
                    } else {
                        let mut paragraphs = self.blocks(child, layout, depth)?;
                        if first
                            && !literal
                            && depth <= 8
                            && let Some(p) = paragraphs.first_mut()
                            && p.name == "w:p"
                        {
                            if let Some(pr) = p.child_mut("w:pPr") {
                                set_numpr(pr, id, native_depth);
                            }
                            first = false;
                        }
                        out.extend(paragraphs);
                    }
                } else {
                    inline.extend(self.inline(child, &layout.body, None)?);
                }
            }
            if !inline.is_empty() || first {
                let mut ppr = layout.ppr.clone();
                if first && !literal && depth <= 8 {
                    set_numpr(&mut ppr, id, native_depth);
                }
                out.push(paragraph(ppr, inline));
            }
        }
        Ok(out)
    }
    fn table(
        &mut self,
        n: &RenderNode,
        layout: &Layout,
        list_depth: usize,
    ) -> Result<Element, DocxError> {
        let mut rows = vec![];
        table_rows(n, &mut rows);
        if rows.len() > 5000 {
            return Err(DocxError::new("表格行数超过预算"));
        }
        let mut out = Element::new("w:tbl");
        let mut pr = Element::new("w:tblPr");
        pr.push(
            Element::new("w:tblW")
                .attr("w:w", "0")
                .attr("w:type", "auto"),
        );
        pr.push(val("w:tblLayout", "autofit"));
        out.push(pr);
        let mut generated = vec![];
        let mut active: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
        let mut max_columns = 0;
        for row in rows {
            let mut tr = Element::new("w:tr");
            let mut column = 0usize;
            let mut next = BTreeMap::new();
            for cell in row
                .children
                .iter()
                .filter(|c| matches!(c.tag.as_deref(), Some("td" | "th")))
            {
                while let Some((left, span)) = active.remove(&column) {
                    tr.push(continuation(span));
                    if left > 1 {
                        next.insert(column, (left - 1, span));
                    }
                    column += span;
                }
                let span = parse_span(cell, "colspan")?;
                let rowspan = parse_span(cell, "rowspan")?;
                if active.range(column..column + span).next().is_some() {
                    return Err(DocxError::new("表格跨列与跨行范围重叠"));
                }
                let mut tc = Element::new("w:tc");
                let mut props = Element::new("w:tcPr");
                if span > 1 {
                    props.push(val("w:gridSpan", span));
                }
                if rowspan > 1 {
                    props.push(val("w:vMerge", "restart"));
                    next.insert(column, (rowspan - 1, span));
                }
                let cell_css = css(cell);
                if let Some(align) = cell_css.get("vertical-align")
                    && matches!(align.as_str(), "top" | "center" | "bottom")
                {
                    props.push(val("w:vAlign", align));
                }
                tc.push(props);
                let mut cell_layout = layout.clone();
                if cell.tag.as_deref() == Some("th") {
                    cell_layout.body.0.insert("w:b".into(), val("w:b", "1"));
                }
                let mut body = cell.clone();
                body.tag = Some("div".into());
                for p in self.blocks(&body, &cell_layout, list_depth)? {
                    tc.push(p);
                }
                if tc
                    .children
                    .iter()
                    .filter_map(Xml::element)
                    .next_back()
                    .is_none_or(|e| e.name != "w:p")
                {
                    tc.push(paragraph(cell_layout.ppr, vec![]));
                }
                tr.push(tc);
                column += span;
                if column > 512 {
                    return Err(DocxError::new("表格列数超过预算"));
                }
            }
            while !active.is_empty() {
                if let Some((left, span)) = active.remove(&column) {
                    tr.push(continuation(span));
                    if left > 1 {
                        next.insert(column, (left - 1, span));
                    }
                    column += span;
                } else {
                    tr.push(empty_cell());
                    column += 1;
                }
                if column > 512 {
                    return Err(DocxError::new("表格列数超过预算"));
                }
            }
            max_columns = max_columns.max(column);
            active = next;
            generated.push(tr);
        }
        if !active.is_empty() {
            self.loss(
                "table-rowspan-loss",
                "表格跨行范围超过实际行数；保留已有单元格并报告损失",
                Some(n.key.clone()),
            );
        }
        let mut grid = Element::new("w:tblGrid");
        for _ in 0..max_columns {
            grid.push(Element::new("w:gridCol").attr("w:w", 1440));
        }
        out.push(grid);
        for row in generated {
            out.push(row);
        }
        Ok(out)
    }
}
fn image_dimensions(mime: &str, b: &[u8]) -> Option<(&'static str, u32, u32)> {
    match mime {
        "image/png"
            if b.len() >= 24
                && b.starts_with(b"\x89PNG\r\n\x1a\n")
                && b.get(12..16) == Some(b"IHDR") =>
        {
            Some((
                "png",
                u32::from_be_bytes(b[16..20].try_into().ok()?),
                u32::from_be_bytes(b[20..24].try_into().ok()?),
            ))
        }
        "image/gif" if b.len() >= 10 && (b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a")) => {
            Some((
                "gif",
                u16::from_le_bytes(b[6..8].try_into().ok()?) as u32,
                u16::from_le_bytes(b[8..10].try_into().ok()?) as u32,
            ))
        }
        "image/jpeg" if b.starts_with(&[0xff, 0xd8]) => {
            let mut at = 2;
            while at + 4 <= b.len() {
                if b[at] != 0xff {
                    return None;
                }
                while at < b.len() && b[at] == 0xff {
                    at += 1;
                }
                let marker = *b.get(at)?;
                at += 1;
                if matches!(marker, 0xd8 | 0xd9 | 0x01 | 0xd0..=0xd7) {
                    continue;
                }
                let len = u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize;
                if len < 2 || at + len > b.len() {
                    return None;
                }
                if matches!(marker,0xc0..=0xc3|0xc5..=0xc7|0xc9..=0xcb|0xcd..=0xcf) && len >= 7 {
                    let h = u16::from_be_bytes(b[at + 3..at + 5].try_into().ok()?) as u32;
                    let w = u16::from_be_bytes(b[at + 5..at + 7].try_into().ok()?) as u32;
                    return Some(("jpg", w, h));
                }
                at += len;
            }
            None
        }
        _ => None,
    }
}
fn table_rows<'a>(n: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    for c in &n.children {
        if c.tag.as_deref() == Some("tr") {
            out.push(c);
        } else if matches!(c.tag.as_deref(), Some("thead" | "tbody" | "tfoot")) {
            table_rows(c, out);
        }
    }
}
fn parse_span(n: &RenderNode, attr: &str) -> Result<usize, DocxError> {
    let span = n
        .attrs
        .get(attr)
        .map(|s| s.parse::<usize>())
        .transpose()
        .map_err(|_| DocxError::new("无效表格跨度"))?
        .unwrap_or(1);
    if !(1..=512).contains(&span) {
        return Err(DocxError::new("表格跨度超出预算"));
    }
    Ok(span)
}
fn empty_cell() -> Element {
    let mut c = Element::new("w:tc");
    c.push(Element::new("w:p"));
    c
}
fn continuation(span: usize) -> Element {
    let mut c = Element::new("w:tc");
    let mut pr = Element::new("w:tcPr");
    if span > 1 {
        pr.push(val("w:gridSpan", span));
    }
    pr.push(Element::new("w:vMerge"));
    c.push(pr);
    c.push(Element::new("w:p"));
    c
}
fn css(n: &RenderNode) -> BTreeMap<String, String> {
    n.attrs
        .get("style")
        .map(|s| {
            s.split(';')
                .filter_map(|p| p.split_once(':'))
                .map(|(k, v)| (k.trim().into(), v.trim().into()))
                .collect()
        })
        .unwrap_or_default()
}
fn css_points(v: &str) -> Option<f64> {
    if let Some(s) = v.strip_suffix("pt") {
        s.parse().ok()
    } else if let Some(s) = v.strip_suffix("px") {
        s.parse::<f64>().ok().map(|v| v * 0.75)
    } else {
        None
    }
}
fn hex_color(v: &str) -> Option<String> {
    let c = v.strip_prefix('#')?;
    if !c.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match c.len() {
        6 => Some(c.to_ascii_uppercase()),
        3 => Some(
            c.chars()
                .flat_map(|c| [c, c])
                .collect::<String>()
                .to_ascii_uppercase(),
        ),
        _ => None,
    }
}
fn is_block(n: &RenderNode) -> bool {
    matches!(
        n.tag.as_deref(),
        Some(
            "html"
                | "body"
                | "main"
                | "article"
                | "section"
                | "div"
                | "p"
                | "table"
                | "ul"
                | "ol"
                | "li"
                | "blockquote"
                | "pre"
                | "figure"
                | "figcaption"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "dt"
                | "dd"
        )
    ) || matches!(
        n.attrs.get("data-weftext-role").map(String::as_str),
        Some("title" | "subtitle" | "heading")
    ) || n.kind == "frame"
}
fn paragraph(mut ppr: Element, runs: Vec<Element>) -> Element {
    sort_elements(
        &mut ppr,
        &[
            "w:pStyle",
            "w:keepNext",
            "w:keepLines",
            "w:pageBreakBefore",
            "w:framePr",
            "w:widowControl",
            "w:numPr",
            "w:suppressLineNumbers",
            "w:pBdr",
            "w:shd",
            "w:tabs",
            "w:suppressAutoHyphens",
            "w:kinsoku",
            "w:wordWrap",
            "w:overflowPunct",
            "w:topLinePunct",
            "w:autoSpaceDE",
            "w:autoSpaceDN",
            "w:bidi",
            "w:adjustRightInd",
            "w:snapToGrid",
            "w:spacing",
            "w:ind",
            "w:contextualSpacing",
            "w:mirrorIndents",
            "w:suppressOverlap",
            "w:jc",
            "w:textDirection",
            "w:textAlignment",
            "w:textboxTightWrap",
            "w:outlineLvl",
            "w:divId",
            "w:cnfStyle",
            "w:rPr",
            "w:sectPr",
            "w:pPrChange",
        ],
    );
    let mut p = Element::new("w:p");
    p.push(ppr);
    p.children.extend(runs.into_iter().map(Xml::Element));
    p
}
fn text_run(text: &str, rpr: &Element) -> Element {
    let mut r = Element::new("w:r");
    if !rpr.children.is_empty() {
        r.push(rpr.clone());
    }
    let mut segment = String::new();
    for c in text.chars() {
        if c == '\n' || c == '\t' || c == '\r' {
            if !segment.is_empty() {
                let mut t = Element::new("w:t").attr("xml:space", "preserve");
                t.children.push(Xml::Text(std::mem::take(&mut segment)));
                r.push(t);
            }
            if c != '\r' {
                r.push(Element::new(if c == '\t' { "w:tab" } else { "w:br" }));
            }
        } else if valid_xml_char(c) {
            segment.push(c);
        }
    }
    if !segment.is_empty() {
        let mut t = Element::new("w:t").attr("xml:space", "preserve");
        t.children.push(Xml::Text(segment));
        r.push(t);
    }
    r
}
fn valid_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r')
        || ('\u{20}'..='\u{d7ff}').contains(&c)
        || ('\u{e000}'..='\u{fffd}').contains(&c)
        || ('\u{10000}'..='\u{10ffff}').contains(&c)
}
fn external_url(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    !v.chars().any(char::is_control)
        && ["https://", "http://", "mailto:", "tel:"]
            .iter()
            .any(|p| lower.starts_with(p))
}
fn find_href(n: &RenderNode) -> Option<&str> {
    if let Some(href) = n.attrs.get("href") {
        return Some(href);
    }
    n.children.iter().find_map(find_href)
}
fn numpr(id: usize, level: usize) -> Element {
    let mut e = Element::new("w:numPr");
    e.push(val("w:ilvl", level));
    e.push(val("w:numId", id));
    e
}
fn set_numpr(ppr: &mut Element, id: usize, level: usize) {
    ppr.remove("w:numPr");
    ppr.push(numpr(id, level));
}
fn sort_elements(e: &mut Element, order: &[&str]) {
    e.children.sort_by_key(|n| {
        n.element()
            .map(|e| {
                order
                    .iter()
                    .position(|name| *name == e.name)
                    .unwrap_or(order.len())
            })
            .unwrap_or(0)
    });
}
fn numbering(existing: Option<Element>, pattern: &str) -> (Element, usize, usize) {
    let mut root = existing.unwrap_or_else(|| Element::new("w:numbering").attr("xmlns:w", W));
    let abstract_id = existing_max(&root, "w:abstractNum", "w:abstractNumId") + 1;
    let heading_id = existing_max(&root, "w:num", "w:numId") + 1;
    let bullet_id = heading_id + 1;
    let mut abstract_num = Element::new("w:abstractNum").attr("w:abstractNumId", abstract_id);
    abstract_num.push(val("w:multiLevelType", "multilevel"));
    let (atoms, suffix) = parse_number_pattern(pattern).unwrap();
    for level in 0..9 {
        let mut l = Element::new("w:lvl").attr("w:ilvl", level);
        l.push(val("w:start", 1));
        l.push(val("w:numFmt", atoms[level.min(atoms.len() - 1)].1));
        l.push(val("w:pStyle", format!("Heading{}", level + 1)));
        let text = (0..=level)
            .map(|i| {
                let prefix = &atoms[i.min(atoms.len() - 1)].0;
                let prefix = if i >= atoms.len() && prefix.is_empty() {
                    &suffix
                } else {
                    prefix
                };
                format!("{prefix}%{}", i + 1)
            })
            .collect::<String>()
            + &suffix;
        l.push(val("w:lvlText", text));
        l.push(val("w:lvlJc", "left"));
        abstract_num.push(l);
    }
    root.push(abstract_num);
    let mut bullet = Element::new("w:abstractNum").attr("w:abstractNumId", abstract_id + 1);
    bullet.push(val("w:multiLevelType", "multilevel"));
    for level in 0..9 {
        let mut l = Element::new("w:lvl").attr("w:ilvl", level);
        l.push(val("w:start", 1));
        l.push(val("w:numFmt", "bullet"));
        l.push(val("w:lvlText", "•"));
        l.push(val("w:lvlJc", "left"));
        let mut p = Element::new("w:pPr");
        p.push(
            Element::new("w:ind")
                .attr("w:left", 720 * (level + 1))
                .attr("w:hanging", 360),
        );
        l.push(p);
        bullet.push(l);
    }
    root.push(bullet);
    for (id, aid) in [(heading_id, abstract_id), (bullet_id, abstract_id + 1)] {
        let mut n = Element::new("w:num").attr("w:numId", id);
        n.push(val("w:abstractNumId", aid));
        root.push(n);
    }
    (root, heading_id, bullet_id)
}
type NumberPattern = (Vec<(String, &'static str)>, String);
fn parse_number_pattern(pattern: &str) -> Option<NumberPattern> {
    if pattern.len() > 256 || pattern.contains('%') {
        return None;
    }
    let mut prefix = String::new();
    let mut atoms = vec![];
    for c in pattern.chars() {
        let fmt = match c {
            '1' => Some("decimal"),
            'a' => Some("lowerLetter"),
            'A' => Some("upperLetter"),
            'i' => Some("lowerRoman"),
            'I' => Some("upperRoman"),
            _ => None,
        };
        if let Some(fmt) = fmt {
            atoms.push((std::mem::take(&mut prefix), fmt));
        } else if "αΑ一壹あいアイא가ㄱ*١۱१১ক①⓵".contains(c) {
            return None;
        } else {
            prefix.push(c);
        }
    }
    if atoms.is_empty() {
        None
    } else {
        Some((atoms, prefix))
    }
}

#[cfg(test)]
mod tests;
