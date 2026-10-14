use crate::{inspect, types::*, world::LocalWorld};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};
use typst::{
    Feature, Library, LibraryExt, World,
    diag::{Severity, SourceDiagnostic},
    foundations::{NativeElement, StyleChain},
    introspection::{Introspector, MetadataElem, Tag},
    model::{Document, HeadingElem},
    text::{FontFamily, FontList, Lang, TextElem},
    utils::LazyHash,
};
use typst_html::{HtmlDocument, HtmlNode};
use typst_kit::fonts::{self, FontStore};
use typst_syntax::{DiagSpanKind, Span};

pub struct Engine {
    pub(crate) library: LazyHash<Library>,
    pub(crate) fonts: FontStore,
    pub(crate) families: Vec<String>,
}

impl Engine {
    pub fn new() -> Result<Self, EngineError> {
        Self::new_with_system_fonts(true)
    }

    pub fn new_with_system_fonts(system_fonts: bool) -> Result<Self, EngineError> {
        let mut fonts = FontStore::new();
        fonts.extend(fonts::embedded());
        fonts.extend(crate::fonts::cjk_fonts().flat_map(|bytes| {
            typst::text::Font::iter(typst::foundations::Bytes::new(bytes)).map(|font| {
                let info = font.info().clone();
                (font, info)
            })
        }));
        if system_fonts {
            fonts.extend(fonts::system());
        }
        let families: Vec<_> = fonts
            .book()
            .families()
            .map(|(name, _)| name.to_owned())
            .collect();
        let mut library = Library::builder()
            .with_features([Feature::Html].into_iter().collect())
            .build();
        let preferred = [
            "Noto Serif CJK SC",
            "Source Han Serif SC",
            "Microsoft YaHei",
            "SimSun",
            "New Computer Modern",
        ];
        let fallback: Vec<_> = preferred
            .into_iter()
            .filter(|name| fonts.book().contains_family(&name.to_lowercase()))
            .map(FontFamily::new)
            .collect();
        library.styles.set(TextElem::font, FontList(fallback));
        library.styles.set(TextElem::lang, Lang::CHINESE);
        crate::style_adapter::install(&mut library);
        Ok(Self {
            library: LazyHash::new(library),
            fonts,
            families,
        })
    }

    pub fn render(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
    ) -> Result<RenderOutput, EngineError> {
        let world = LocalWorld::new(&self.library, &self.fonts, source, path, root)?;
        self.render_world(source, world, crate::inspect_editor(source))
    }

    pub fn render_with_style(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
        style: &crate::StyleModule,
    ) -> Result<RenderOutput, EngineError> {
        let world = LocalWorld::with_style(&self.library, &self.fonts, source, path, root, style)?;
        self.render_world(
            source,
            world,
            crate::inspect_editor_with_style(source, style),
        )
    }

    pub(crate) fn render_world(
        &self,
        source: &str,
        world: LocalWorld<'_>,
        editor_map: crate::EditorMap,
    ) -> Result<RenderOutput, EngineError> {
        let scope = format!("{:x}", Sha256::digest(source.as_bytes()));
        let mut source_info = inspect(source);
        source_info.body_insertion = editor_map.body_insertion.clone();
        if let Some(reason) = &editor_map.unavailable_reason {
            source_info.body_insertion = None;
            source_info.writer_status = reason.clone();
        }
        let result = typst::compile::<HtmlDocument>(&world);
        let mut diagnostics: Vec<_> = result
            .warnings
            .iter()
            .map(|d| diagnostic(d, &world))
            .collect();
        let mut tree = None;
        let mut html = String::new();
        let mut labels = BTreeMap::<String, CompiledLabel>::new();
        let mut products = None;
        let ok = match result.output {
            Ok(document) => {
                for content in document.introspector().query_labelled() {
                    if let Some(label) = content.label() {
                        let label = label.resolve().as_str().to_string();
                        let entry = labels.entry(label.clone()).or_insert(CompiledLabel {
                            label,
                            count: 0,
                            origins: vec![],
                        });
                        entry.count += 1;
                        if let Some(origin) = origin(content.span(), &world) {
                            entry.origins.push(origin);
                        }
                    }
                }
                if let Some(title) = &document.info().title {
                    source_info.title = Some(title.to_string());
                }
                let mut counter = 0;
                let mut native_headings = vec![];
                let mut projected = project(
                    document.root_node(),
                    &world,
                    &mut counter,
                    &mut native_headings,
                    0,
                    &scope,
                )?;
                qualify_run_in_roles(&mut projected);
                qualify_subtitle_role(
                    &mut projected,
                    labels
                        .get("weftext-subtitle")
                        .map_or(0, |label| label.count),
                );
                let mut occurrences = HashMap::new();
                count_origins(&projected, &mut occurrences);
                qualify(&mut projected, &editor_map, &occurrences);
                // Query the successful target's real native introspector. This
                // includes headings in same-target frame islands and excludes
                // unevaluated author AST branches.
                native_headings = document
                    .introspector()
                    .query(&HeadingElem::ELEM.select())
                    .iter()
                    .filter_map(|content| {
                        let heading = content.to_packed::<HeadingElem>()?;
                        let origin = origin(content.span(), &world);
                        Some(HeadingInfo {
                            text: heading.body.plain_text().to_string(),
                            path: origin.as_ref().map(|o| o.path.clone()),
                            is_main: origin.as_ref().is_some_and(|o| o.is_main),
                            level: heading
                                .resolve_level(StyleChain::new(&world.library.styles))
                                .get(),
                            from: origin.as_ref().map_or(0, |o| o.from),
                            to: origin.as_ref().map_or(0, |o| o.to),
                            utf16_from: origin.as_ref().map_or(0, |o| o.utf16_from),
                            utf16_to: origin.as_ref().map_or(0, |o| o.utf16_to),
                            generated: origin
                                .as_ref()
                                .is_none_or(|o| !o.is_main || o.syntax_kind != "Heading"),
                            visible: false,
                        })
                    })
                    .collect();
                // Native final heading tags retain actual resolved levels, even
                // for generated headings. Their bodies are semantic metadata,
                // never inserted into the displayed consumer tree.
                if !native_headings.is_empty() {
                    for heading in &mut native_headings {
                        heading.visible = contains_range(
                            &projected,
                            heading.path.as_deref(),
                            heading.from,
                            heading.to,
                        );
                    }
                    source_info.headings = native_headings.clone();
                } else {
                    for heading in &mut source_info.headings {
                        heading.visible =
                            contains_range(&projected, None, heading.from, heading.to);
                    }
                }
                html = safe_html(&projected);
                products = Some(crate::products::native_products(
                    document.root_node(),
                    &projected,
                    document.info().title.as_ref().map(ToString::to_string),
                    native_headings,
                ));
                tree = Some(projected);
                true
            }
            Err(errors) => {
                diagnostics.extend(errors.iter().map(|d| diagnostic(d, &world)));
                false
            }
        };
        Ok(RenderOutput {
            ok,
            html,
            tree,
            diagnostics,
            source_info,
            dependencies: world.dependencies(),
            font_families: self.families.clone(),
            compiler_version: "0.15.1".into(),
            labels: labels.into_values().collect(),
            products,
        })
    }

    pub fn compile(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
    ) -> Result<RenderOutput, EngineError> {
        self.render(source, path, root)
    }
}

pub(crate) fn diagnostic(d: &SourceDiagnostic, world: &LocalWorld<'_>) -> Diagnostic {
    let origin = d.span.id().and_then(|id| {
        let source = world.source(id).ok()?;
        let range = match d.span.get() {
            DiagSpanKind::Number { num, sub_range, .. } => source.range(num, sub_range)?,
            DiagSpanKind::Range { range, .. } => range,
            DiagSpanKind::Detached => return None,
        };
        Some(Origin {
            path: world.display_path(id),
            from: range.start,
            to: range.end,
            utf16_from: utf16(source.text(), range.start),
            utf16_to: utf16(source.text(), range.end),
            raw: source.text().get(range)?.into(),
            syntax_kind: "Diagnostic".into(),
            is_main: id == world.main_id,
        })
    });
    Diagnostic {
        severity: match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
        .into(),
        message: d.message.to_string(),
        origin,
    }
}

pub(crate) fn origin(span: Span, world: &LocalWorld<'_>) -> Option<Origin> {
    let id = span.id()?;
    let source = world.source(id).ok()?;
    let node = source.find(span)?;
    let range = node.range();
    Some(Origin {
        path: world.display_path(id),
        from: range.start,
        to: range.end,
        utf16_from: utf16(source.text(), range.start),
        utf16_to: utf16(source.text(), range.end),
        raw: source.text().get(range)?.into(),
        syntax_kind: format!("{:?}", node.kind()),
        is_main: id == world.main_id,
    })
}

fn project(
    node: &HtmlNode,
    world: &LocalWorld<'_>,
    counter: &mut usize,
    headings: &mut Vec<HeadingInfo>,
    depth: usize,
    scope: &str,
) -> Result<RenderNode, EngineError> {
    *counter += 1;
    if *counter > 100_000 || depth > 256 {
        return Err(EngineError("显示树超过安全预算；请使用源码入口".into()));
    }
    let mut out = RenderNode {
        key: format!("{scope}-n{counter}"),
        kind: String::new(),
        tag: None,
        text: None,
        attrs: BTreeMap::new(),
        children: vec![],
        origin: origin(node.span(), world),
        writable: false,
    };
    match node {
        HtmlNode::Text(text, _) => {
            out.kind = "text".into();
            out.text = Some(text.to_string());
        }
        HtmlNode::Element(element) => {
            let tag = element.tag.resolve().as_str().to_owned();
            out.kind = "element".into();
            if matches!(tag.as_str(), "head" | "meta" | "title" | "link" | "base") {
                // The compiler's document head is metadata, not a displayed
                // consumer subtree. Native Typst title body is a separate
                // ordinary content element and remains in the body.
                out.kind = "tag".into();
                return Ok(out);
            }
            if matches!(
                tag.as_str(),
                "script"
                    | "style"
                    | "iframe"
                    | "object"
                    | "embed"
                    | "form"
                    | "input"
                    | "button"
                    | "textarea"
                    | "select"
            ) {
                // Preserve the source and show no executable/resource-fetching
                // subtree. Ordinary Typst output uses the semantic allowlist.
                out.tag = Some("span".into());
                out.attrs
                    .insert("class".into(), "weftext-unsupported".into());
                out.text = Some(format!("[不支持的 HTML 元素：{tag}]"));
                return Ok(out);
            }
            out.tag = Some(if allowed_tag(&tag) {
                tag.clone()
            } else {
                "span".into()
            });
            for (key, value) in &element.attrs.0 {
                let name = key.resolve().as_str().to_owned();
                if let Some(value) = safe_attr(&name, value.as_str(), &tag) {
                    out.attrs.insert(name, value);
                }
            }
            // Semantic attributes come only from the native producer's typed
            // side information, never from author-controlled HTML attributes.
            if let Some(role) = element.weftext.role {
                if role == "strong" {
                    out.attrs
                        .insert("data-weftext-producer".into(), role.into());
                } else {
                    out.attrs.insert("data-weftext-role".into(), role.into());
                }
            }
            if let Some(level) = element.weftext.level {
                out.attrs
                    .insert("data-weftext-level".into(), level.to_string());
            }
            if let Some(numbered) = element.weftext.numbered {
                out.attrs
                    .insert("data-weftext-numbered".into(), numbered.to_string());
            }
            if let Some(kind) = element.weftext.numbering_kind {
                out.attrs
                    .insert("data-weftext-numbering-kind".into(), kind.into());
            }
            if let Some(pattern) = &element.weftext.numbering_pattern {
                out.attrs
                    .insert("data-weftext-numbering-pattern".into(), pattern.to_string());
            }
            if let Some(state) = &element.weftext.counter_state {
                out.attrs
                    .insert("data-weftext-counter-state".into(), state.to_string());
            }
            if let Some(reference_kind) = element.weftext.reference_kind {
                out.attrs
                    .insert("data-weftext-reference-kind".into(), reference_kind.into());
            }
            if let Some(label) = &element.weftext.label {
                out.attrs
                    .insert("data-weftext-label".into(), label.to_string());
            }
            let css = safe_css(&element.css);
            if !css.is_empty() {
                out.attrs.insert("style".into(), css);
            }
            if element.pre_span {
                out.attrs.insert("class".into(), "weftext-pre".into());
            }
            if tag == "img" && !out.attrs.contains_key("src") {
                out.tag = Some("span".into());
                out.text = Some(format!(
                    "[图片：{}]",
                    out.attrs
                        .get("alt")
                        .map(String::as_str)
                        .unwrap_or("资源预览暂不可用")
                ));
            }
            for child in &element.children {
                out.children
                    .push(project(child, world, counter, headings, depth + 1, scope)?);
            }
        }
        HtmlNode::Tag(Tag::Start(content, _)) => {
            out.kind = "tag".into();
            // Keep the native metadata tag at its actual final-tree position.
            // The UI can attach derived Query presentation here, but this tag
            // never becomes body text, an exported result, or a source writer.
            if content.to_packed::<MetadataElem>().is_some()
                && let Some(label) = content.label()
                && label.resolve().as_str().starts_with("weftext-query-")
            {
                out.attrs.insert(
                    "data-weftext-query-label".into(),
                    label.resolve().as_str().into(),
                );
                if let Some(location) = content.location() {
                    out.attrs.insert(
                        "data-weftext-query-occurrence".into(),
                        format!("{:032x}", location.hash()),
                    );
                }
                out.origin = origin(content.span(), world);
            }
            if let Some(heading) = content.to_packed::<HeadingElem>() {
                out.attrs.insert(
                    "data-weftext-native-heading-start".into(),
                    format!("{:032x}", content.location().unwrap().hash()),
                );
                out.attrs.insert(
                    "data-weftext-level".into(),
                    heading
                        .resolve_level(StyleChain::default())
                        .get()
                        .to_string(),
                );
                out.attrs.insert(
                    "data-weftext-numbered".into(),
                    heading
                        .numbering
                        .get_ref(StyleChain::default())
                        .is_some()
                        .to_string(),
                );
                let numbering = typst_html::WeftextHtmlSemantics::default()
                    .with_numbering(heading.numbering.get_ref(StyleChain::default()).as_ref());
                if let Some(kind) = numbering.numbering_kind {
                    out.attrs
                        .insert("data-weftext-numbering-kind".into(), kind.into());
                }
                if let Some(pattern) = numbering.numbering_pattern {
                    out.attrs
                        .insert("data-weftext-numbering-pattern".into(), pattern.to_string());
                }
                if let Some(label) = content.label() {
                    out.attrs
                        .insert("data-weftext-label".into(), label.resolve().as_str().into());
                }
                let origin = origin(content.span(), world);
                let (from, to, uf, ut, generated) = origin
                    .as_ref()
                    .map(|o| {
                        (
                            o.from,
                            o.to,
                            o.utf16_from,
                            o.utf16_to,
                            !o.is_main || o.syntax_kind != "Heading",
                        )
                    })
                    .unwrap_or((0, 0, 0, 0, true));
                headings.push(HeadingInfo {
                    path: origin.as_ref().map(|o| o.path.clone()),
                    is_main: origin.as_ref().is_some_and(|o| o.is_main),
                    text: heading.body.plain_text().to_string(),
                    level: heading
                        .resolve_level(StyleChain::new(&world.library.styles))
                        .get(),
                    from,
                    to,
                    utf16_from: uf,
                    utf16_to: ut,
                    generated,
                    visible: true,
                });
            }
        }
        HtmlNode::Tag(Tag::End(location, _, _)) => {
            out.kind = "tag".into();
            out.attrs.insert(
                "data-weftext-native-end".into(),
                format!("{:032x}", location.hash()),
            );
        }
        HtmlNode::Frame(_) => {
            out.kind = "frame".into();
            out.text = Some("[原生分页排版区域；使用源码查看内容]".into());
        }
    }
    Ok(out)
}

/// Associate only the certified controlled recipe's actual native Strong
/// producer with its enclosing final native heading tags. Arbitrary returned
/// plain text, custom HTML h tags, and deleted headings acquire no role.
fn qualify_run_in_roles(tree: &mut RenderNode) {
    #[derive(Clone)]
    struct Heading {
        location: String,
        label: Option<String>,
        level: String,
        numbered: String,
        numbering_kind: Option<String>,
        numbering_pattern: Option<String>,
    }
    fn clear_prefix_roles(node: &mut RenderNode) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "heading-prefix")
        {
            node.attrs.remove("data-weftext-role");
            node.attrs.remove("data-weftext-label");
        }
        for child in &mut node.children {
            clear_prefix_roles(child);
        }
    }
    fn visit(node: &mut RenderNode, headings: &mut Vec<Heading>) {
        if let Some(location) = node.attrs.get("data-weftext-native-heading-start") {
            headings.push(Heading {
                location: location.clone(),
                label: node.attrs.get("data-weftext-label").cloned(),
                level: node.attrs["data-weftext-level"].clone(),
                numbered: node.attrs["data-weftext-numbered"].clone(),
                numbering_kind: node.attrs.get("data-weftext-numbering-kind").cloned(),
                numbering_pattern: node.attrs.get("data-weftext-numbering-pattern").cloned(),
            });
        }
        if let Some(location) = node.attrs.get("data-weftext-native-end")
            && let Some(position) = headings
                .iter()
                .rposition(|heading| &heading.location == location)
        {
            headings.truncate(position);
        }
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "run-in-candidate")
        {
            if let Some(heading) = headings
                .last()
                .filter(|heading| heading.label.as_ref() == node.attrs.get("data-weftext-label"))
            {
                node.attrs
                    .insert("data-weftext-role".into(), "heading".into());
                node.attrs
                    .insert("data-weftext-run-in".into(), "true".into());
                node.attrs
                    .insert("data-weftext-level".into(), heading.level.clone());
                node.attrs
                    .insert("data-weftext-numbered".into(), heading.numbered.clone());
                if let Some(kind) = &heading.numbering_kind {
                    node.attrs
                        .insert("data-weftext-numbering-kind".into(), kind.clone());
                }
                if let Some(pattern) = &heading.numbering_pattern {
                    node.attrs
                        .insert("data-weftext-numbering-pattern".into(), pattern.clone());
                }
            } else {
                node.attrs.remove("data-weftext-role");
                node.attrs.remove("data-weftext-label");
                clear_prefix_roles(node);
            }
        }
        for child in &mut node.children {
            visit(child, headings);
        }
    }
    visit(tree, &mut vec![]);
}

/// The subtitle convention is meaningful only with a unique displayed native
/// title and one finally consumed labelled native block. Source labels or a
/// title kept solely in the introspector do not establish that association.
fn qualify_subtitle_role(tree: &mut RenderNode, label_occurrences: usize) {
    fn count(node: &RenderNode, titles: &mut usize, subtitles: &mut usize) {
        match node.attrs.get("data-weftext-role").map(String::as_str) {
            Some("title") => *titles += 1,
            Some("subtitle-candidate") => *subtitles += 1,
            _ => {}
        }
        for child in &node.children {
            count(child, titles, subtitles);
        }
    }
    fn resolve(node: &mut RenderNode, associated: bool) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "subtitle-candidate")
        {
            if associated {
                node.attrs
                    .insert("data-weftext-role".into(), "subtitle".into());
            } else {
                node.attrs.remove("data-weftext-role");
                node.attrs.remove("data-weftext-label");
            }
        }
        for child in &mut node.children {
            resolve(child, associated);
        }
    }
    let (mut titles, mut subtitles) = (0, 0);
    count(tree, &mut titles, &mut subtitles);
    resolve(
        tree,
        titles == 1 && subtitles == 1 && label_occurrences == 1,
    );
}

fn count_origins(node: &RenderNode, counts: &mut HashMap<(String, usize, usize), usize>) {
    if node.kind == "text"
        && let Some(o) = &node.origin
    {
        *counts.entry((o.path.clone(), o.from, o.to)).or_default() += 1;
    }
    for child in &node.children {
        count_origins(child, counts);
    }
}

fn qualify(
    node: &mut RenderNode,
    info: &crate::EditorMap,
    counts: &HashMap<(String, usize, usize), usize>,
) {
    if info.unavailable_reason.is_none()
        && node.kind == "text"
        && let Some(o) = &node.origin
    {
        let expected = match o.syntax_kind.as_str() {
            "Text" => Some(o.raw.clone()),
            "Escape" | "Shorthand" => {
                let parsed = typst_syntax::Source::detached(&o.raw);
                parsed.root().children().next().and_then(|n| {
                    n.cast::<typst_syntax::ast::Escape>()
                        .map(|e| e.get().to_string())
                        .or_else(|| {
                            n.cast::<typst_syntax::ast::Shorthand>()
                                .map(|e| e.get().to_string())
                        })
                })
            }
            "Space" => Some(" ".into()),
            _ => None,
        };
        node.writable = o.is_main
            && expected.as_deref() == node.text.as_deref()
            && info
                .literal_regions
                .iter()
                .any(|r| o.from >= r.range.from && o.to <= r.range.to)
            && counts.get(&(o.path.clone(), o.from, o.to)) == Some(&1);
    }
    for child in &mut node.children {
        qualify(child, info, counts);
    }
}

fn contains_range(node: &RenderNode, path: Option<&str>, from: usize, to: usize) -> bool {
    (node.kind == "text"
        && node.origin.as_ref().is_some_and(|o| {
            path.map_or(o.is_main, |p| o.path == p) && o.from >= from && o.to <= to
        }))
        || node
            .children
            .iter()
            .any(|n| contains_range(n, path, from, to))
}

fn allowed_tag(tag: &str) -> bool {
    matches!(
        tag,
        "html"
            | "head"
            | "body"
            | "main"
            | "article"
            | "section"
            | "header"
            | "footer"
            | "nav"
            | "aside"
            | "div"
            | "span"
            | "p"
            | "br"
            | "hr"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "s"
            | "del"
            | "u"
            | "sub"
            | "sup"
            | "small"
            | "mark"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "blockquote"
            | "pre"
            | "code"
            | "a"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "td"
            | "th"
            | "caption"
            | "colgroup"
            | "col"
            | "figure"
            | "figcaption"
            | "img"
            | "title"
            | "math"
            | "semantics"
            | "annotation"
            | "mrow"
            | "mi"
            | "mo"
            | "mn"
            | "mtext"
            | "mspace"
            | "ms"
            | "mfrac"
            | "msqrt"
            | "mroot"
            | "mstyle"
            | "merror"
            | "mpadded"
            | "mphantom"
            | "mfenced"
            | "menclose"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "mmultiscripts"
            | "mprescripts"
            | "none"
            | "mtable"
            | "mlabeledtr"
            | "mtr"
            | "mtd"
    )
}

fn safe_css(properties: &typst_html::css::Properties) -> String {
    properties
        .iter()
        .filter(|p| {
            matches!(
                p.name,
                "font-family"
                    | "font-size"
                    | "font-weight"
                    | "font-style"
                    | "color"
                    | "background-color"
                    | "border-top"
                    | "border-right"
                    | "border-bottom"
                    | "border-left"
                    | "border-collapse"
                    | "text-align"
                    | "vertical-align"
                    | "white-space"
                    | "display"
                    | "line-height"
                    | "margin-inline-end"
                    | "list-style-type"
            )
        })
        .filter(|p| {
            p.value.len() < 4096
                && !p
                    .value
                    .chars()
                    .any(|c| c.is_control() || ";{}<>".contains(c))
                && !p.value.to_lowercase().contains("url(")
                && !p.value.to_lowercase().contains("expression(")
        })
        .map(|p| format!("{}: {}", p.name, p.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn safe_attr(name: &str, value: &str, tag: &str) -> Option<String> {
    let limit = if name == "src" && tag == "img" {
        8 * 1024 * 1024
    } else {
        4096
    };
    if value.len() > limit || value.chars().any(|c| c == '\0') {
        return None;
    }
    match name {
        "href" if tag == "a" => safe_url(value).then(|| value.into()),
        // Native image resource transport is deliberately not an ambient URL
        // loader; only inline raster bytes can be displayed in this MVP.
        "src" if tag == "img" => (value.starts_with("data:image/png;base64,")
            || value.starts_with("data:image/jpeg;base64,")
            || value.starts_with("data:image/gif;base64,")
            || value.starts_with("data:image/webp;base64,"))
        .then(|| value.into()),
        "id" | "title" | "alt" | "lang" | "dir" | "role" | "aria-level" | "aria-label"
        | "colspan" | "rowspan" | "scope" | "start" | "value" | "display" | "mathvariant"
        | "stretchy" | "fence" | "separator" | "accent" | "accentunder" | "lspace" | "rspace"
        | "encoding" | "xmlns" => Some(value.into()),
        _ => None,
    }
}

pub fn safe_url(value: &str) -> bool {
    let value = value.trim();
    if value.chars().any(|c| c.is_control()) {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    let local_file = lower.strip_prefix("file:///").is_some_and(|path| {
        !path.is_empty()
            && !path.starts_with(['/', '\\'])
            && !path.starts_with("%2f")
            && !path.starts_with("%5c")
    });
    lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("weftext-node://")
        || lower.starts_with("weftext-resource://")
        || lower.starts_with("weftext-workspace://")
        || local_file
        || value.starts_with('#')
        || (!value.contains(':') && !value.starts_with("//") && !value.starts_with('\\'))
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn safe_html(node: &RenderNode) -> String {
    if node.kind == "tag" {
        return String::new();
    }
    if node.kind == "text" {
        return escape(node.text.as_deref().unwrap_or(""));
    }
    if node.kind == "frame" {
        return format!(
            "<span class=\"weftext-frame\">{}</span>",
            escape(node.text.as_deref().unwrap_or(""))
        );
    }
    let tag = node.tag.as_deref().unwrap_or("span");
    let children = escape(node.text.as_deref().unwrap_or(""))
        + &node.children.iter().map(safe_html).collect::<String>();
    if matches!(tag, "html" | "head" | "body" | "title") {
        return if matches!(tag, "head" | "title") {
            String::new()
        } else {
            children
        };
    }
    let attrs = node
        .attrs
        .iter()
        .map(|(k, v)| format!(" {k}=\"{}\"", escape(v)))
        .collect::<String>();
    if matches!(tag, "br" | "hr" | "img" | "col") {
        format!("<{tag}{attrs}>")
    } else {
        format!("<{tag}{attrs}>{children}</{tag}>")
    }
}
