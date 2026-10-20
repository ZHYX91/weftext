//! Author notes are independent OOXML parts, never review-comment JSON.
use super::*;
use crate::exchange::NativeLoss;
pub(super) fn canonical_note_id(id: &str) -> Option<String> {
    id.parse::<i32>()
        .ok()
        .filter(|v| *v > 0)
        .map(|v| v.to_string())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocxImportOptions {
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub caption_pairings: Vec<DocxCaptionPairing>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocxCaptionPairing {
    pub caption_block: usize,
    pub object_block: usize,
    pub kind: String,
    pub caption_text: String,
    pub position: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxCaptionCandidate {
    pub block_index: usize,
    pub sequence: String,
    pub text: String,
    pub suggested_text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxObjectCandidate {
    pub block_index: usize,
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxImportAnalysis {
    pub content_hash: String,
    pub captions: Vec<DocxCaptionCandidate>,
    pub objects: Vec<DocxObjectCandidate>,
}

pub(super) fn normalize_fields(e: &Element) -> Element {
    let mut out = e.clone();
    out.children.clear();
    let mut pending: Vec<Xml> = vec![];
    let mut instruction = String::new();
    let mut cache = vec![];
    let mut stage = 0usize;
    for child in &e.children {
        let Some(c) = child.element() else {
            if stage == 0 {
                out.children.push(child.clone());
            } else {
                pending.push(child.clone());
            }
            continue;
        };
        let field = c.child("w:fldChar").and_then(|f| f.get("w:fldCharType"));
        if field == Some("begin") && stage == 0 {
            stage = 1;
            pending.push(child.clone());
            continue;
        }
        if stage > 0 {
            pending.push(child.clone());
            if field == Some("begin") {
                stage = 3;
            }
            if let Some(instr) = c.child("w:instrText") {
                instruction.push_str(&instr.text());
            }
            if field == Some("separate") && stage == 1 {
                stage = 2;
                continue;
            }
            if field == Some("end") {
                if stage == 2 {
                    let mut f = Element::new("w:fldSimple").attr("w:instr", &instruction);
                    f.children = std::mem::take(&mut cache);
                    out.push(f);
                } else {
                    out.children.append(&mut pending);
                }
                pending.clear();
                cache.clear();
                instruction.clear();
                stage = 0;
                continue;
            }
            if stage == 2 {
                cache.push(Xml::Element(normalize_fields(c)));
            }
        } else {
            out.children.push(Xml::Element(normalize_fields(c)));
        }
    }
    out.children.append(&mut pending);
    out
}
/// Word adds MERGEFORMAT to ordinary fields when saving. It affects cached
/// formatting, not the identity or meaning of a reference. Other switches must
/// still be recognized explicitly rather than silently changing semantics.
pub(super) fn reference_switches(tokens: &[&str], allowed: &[&str]) -> bool {
    let mut tokens = tokens.iter();
    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case("\\*") {
            if !tokens
                .next()
                .is_some_and(|value| value.eq_ignore_ascii_case("MERGEFORMAT"))
            {
                return false;
            }
        } else if !allowed
            .iter()
            .any(|allowed| token.eq_ignore_ascii_case(allowed))
        {
            return false;
        }
    }
    true
}
fn word_text(e: &Element) -> String {
    if matches!(e.name.as_str(), "w:t" | "w:delText") {
        return e.text();
    }
    e.children
        .iter()
        .filter_map(Xml::element)
        .map(word_text)
        .collect()
}
pub(super) fn analyze(bytes: &[u8], doc: &Element) -> DocxImportAnalysis {
    let mut analysis = DocxImportAnalysis {
        content_hash: crate::exchange::digest(bytes),
        captions: vec![],
        objects: vec![],
    };
    let Some(body) = doc.child("w:body") else {
        return analysis;
    };
    for (block_index, block) in body.children.iter().filter_map(Xml::element).enumerate() {
        let mut fields = vec![];
        block.descendants("w:fldSimple", &mut fields);
        let sequences: Vec<_> = fields
            .iter()
            .filter_map(|e| {
                let mut t = e.get("w:instr")?.split_whitespace();
                t.next()?
                    .eq_ignore_ascii_case("SEQ")
                    .then(|| t.next().unwrap_or("").to_owned())
            })
            .collect();
        if sequences.len() == 1 {
            let mut seen = false;
            let mut body_text = String::new();
            for child in block.children.iter().filter_map(Xml::element) {
                if child.name == "w:fldSimple"
                    && child
                        .get("w:instr")
                        .is_some_and(|s| s.trim_start().to_ascii_uppercase().starts_with("SEQ "))
                {
                    seen = true;
                    continue;
                }
                if seen {
                    body_text.push_str(&word_text(child));
                }
            }
            analysis.captions.push(DocxCaptionCandidate {
                block_index,
                sequence: sequences[0].clone(),
                text: word_text(block),
                suggested_text: body_text
                    .trim_start_matches([' ', '\u{a0}', ':', '：', '.', '．'])
                    .to_owned(),
            });
        }
        let mut drawings = vec![];
        block.descendants("w:drawing", &mut drawings);
        let mut equations = vec![];
        block.descendants("m:oMath", &mut equations);
        let kind = if block.name == "w:tbl" {
            Some("table")
        } else if drawings.len() == 1 && equations.is_empty() && word_text(block).trim().is_empty()
        {
            Some("figure")
        } else if equations.len() == 1 && drawings.is_empty() && word_text(block).trim().is_empty()
        {
            Some("equation")
        } else {
            None
        };
        if let Some(kind) = kind {
            analysis.objects.push(DocxObjectCandidate {
                block_index,
                kind: kind.into(),
            });
        }
    }
    analysis
}
pub(super) fn paired_body(
    bytes: &[u8],
    doc: &Element,
    options: &DocxImportOptions,
) -> Result<Element, DocxError> {
    let analysis = analyze(bytes, doc);
    if (!options.content_hash.is_empty() || !options.caption_pairings.is_empty())
        && options.content_hash != analysis.content_hash
    {
        return Err(DocxError::new("题注配对预览原件已变化，请重新预览"));
    }
    let mut body = doc
        .child("w:body")
        .ok_or_else(|| DocxError::new("缺少Word body"))?
        .clone();
    let blocks: Vec<_> = body
        .children
        .iter()
        .filter_map(Xml::element)
        .cloned()
        .collect();
    let mut used = BTreeSet::new();
    let mut replacements = BTreeMap::new();
    let mut suppressed = BTreeSet::new();
    for p in &options.caption_pairings {
        if !used.insert(p.caption_block)
            || !used.insert(p.object_block)
            || !matches!(p.position.as_str(), "top" | "bottom")
            || !analysis
                .captions
                .iter()
                .any(|c| c.block_index == p.caption_block)
            || !analysis
                .objects
                .iter()
                .any(|o| o.block_index == p.object_block && o.kind == p.kind)
            || p.caption_text.len() > 64 * 1024
        {
            return Err(DocxError::new(
                "题注配对必须一一对应真实SEQ与所选同类对象，位置和文本须有效",
            ));
        }
        let mut wrapper = Element::new("weftext:pairedFigure")
            .attr("kind", &p.kind)
            .attr("position", &p.position)
            .attr("caption", &p.caption_text)
            .attr("label", format!("weftext-word-caption-{}", p.caption_block));
        let mut bookmarks = vec![];
        blocks[p.caption_block].descendants("w:bookmarkStart", &mut bookmarks);
        wrapper.set_attr(
            "bookmarks",
            &serde_json::to_string(
                &bookmarks
                    .iter()
                    .filter_map(|b| b.get("w:name"))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        );
        wrapper.push(blocks[p.object_block].clone());
        replacements.insert(p.object_block, wrapper);
        suppressed.insert(p.caption_block);
    }
    body.children = blocks
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            (!suppressed.contains(&i)).then(|| Xml::Element(replacements.remove(&i).unwrap_or(e)))
        })
        .collect();
    Ok(body)
}
pub(super) fn caption_bookmarks(body: &Element) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for wrapper in body
        .children
        .iter()
        .filter_map(Xml::element)
        .filter(|e| e.name == "weftext:pairedFigure")
    {
        let names: Vec<String> =
            serde_json::from_str(wrapper.get("bookmarks").unwrap_or("[]")).unwrap_or_default();
        for name in names {
            if out
                .insert(name.clone(), wrapper.get("label").unwrap_or("").into())
                .is_some()
            {
                duplicates.insert(name);
            }
        }
    }
    for name in duplicates {
        out.remove(&name);
    }
    out
}

pub(super) fn note_bookmarks(body: &Element) -> BTreeMap<String, (String, String)> {
    fn walk(
        e: &Element,
        active: &mut Vec<(String, String)>,
        out: &mut BTreeMap<String, Option<(String, String)>>,
    ) {
        if e.name == "w:ins" {
            return;
        }
        if e.name == "w:bookmarkStart" {
            if let (Some(id), Some(name)) = (e.get("w:id"), e.get("w:name")) {
                active.push((id.into(), name.into()));
            }
        } else if e.name == "w:bookmarkEnd" {
            if let Some(id) = e.get("w:id") {
                active.retain(|(key, _)| key != id);
            }
        } else if matches!(
            e.name.as_str(),
            "w:footnoteReference" | "w:endnoteReference"
        ) && let Some(id) = e.get("w:id")
        {
            let kind = if e.name == "w:footnoteReference" {
                "footnote"
            } else {
                "endnote"
            };
            let Some(id) = canonical_note_id(id) else {
                return;
            };
            for (_, name) in active.iter() {
                out.entry(name.clone())
                    .and_modify(|v| *v = None)
                    .or_insert(Some((kind.into(), id.clone())));
            }
        }
        for c in e.children.iter().filter_map(Xml::element) {
            walk(c, active, out);
        }
    }
    let mut out = BTreeMap::new();
    walk(body, &mut vec![], &mut out);
    out.into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect()
}

#[derive(Clone)]
pub(super) struct ImportedNote {
    pub body: Element,
    pub part: String,
    pub rels: Element,
}

pub(super) fn read(
    package: &Package,
    losses: &mut Vec<NativeLoss>,
) -> Result<BTreeMap<(String, String), ImportedNote>, DocxError> {
    let mut notes = BTreeMap::new();
    for path in ["word/document.xml", "word/settings.xml"] {
        if let Some(root) = package.xml(path)? {
            for tag in ["w:footnotePr", "w:endnotePr"] {
                let mut settings = vec![];
                root.descendants(tag, &mut settings);
                for setting in settings {
                    if setting
                        .child("w:numFmt")
                        .and_then(|e| e.get("w:val"))
                        .is_some_and(|v| v != "decimal")
                        || setting
                            .child("w:numStart")
                            .and_then(|e| e.get("w:val"))
                            .is_some_and(|v| v != "1")
                        || setting
                            .child("w:numRestart")
                            .and_then(|e| e.get("w:val"))
                            .is_some_and(|v| v != "continuous")
                        || setting
                            .child("w:pos")
                            .and_then(|e| e.get("w:val"))
                            .is_some_and(|v| !matches!(v, "pageBottom" | "docEnd"))
                    {
                        losses.push(NativeLoss{path:path.into(),code:"word-note-numbering-layout".into(),detail:format!("{tag} 自定义符号／起始／重启／分节位置未映射；新文档采用独立连续十进制编号，准确原件保留")});
                    }
                }
            }
        }
    }
    for kind in ["footnote", "endnote"] {
        let rel_type = format!("{}{kind}s", package::REL);
        if !package
            .rels
            .children
            .iter()
            .filter_map(Xml::element)
            .any(|e| e.get("Type") == Some(&rel_type))
        {
            continue;
        }
        let part = match package.related_part(&rel_type, "") {
            Ok(part) => part,
            Err(e) => {
                losses.push(NativeLoss {
                    path: "word/document.xml".into(),
                    code: "word-note-part".into(),
                    detail: e.0,
                });
                continue;
            }
        };
        let Some(root) = package.xml(&part)? else {
            continue;
        };
        let root = normalize_fields(&root);
        if root.name != format!("w:{kind}s") {
            losses.push(NativeLoss {
                path: part,
                code: "word-note-part".into(),
                detail: "注释部件根元素不匹配；正文未转换，原件保留".into(),
            });
            continue;
        }
        let rels = package.part_relationships(&part)?;
        let mut duplicate = BTreeSet::new();
        for body in root
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|e| e.name == format!("w:{kind}"))
        {
            // Separators and continuation notices are layout, not author notes.
            if body.get("w:type").is_some_and(|v| v != "normal") {
                continue;
            }
            let Some(id) = body.get("w:id").and_then(canonical_note_id) else {
                losses.push(NativeLoss {
                    path: part.clone(),
                    code: "word-note-id".into(),
                    detail: "普通注释缺少有效正整数 ID；正文未转换，原件保留".into(),
                });
                continue;
            };
            let key = (kind.into(), id);
            if notes
                .insert(
                    key.clone(),
                    ImportedNote {
                        body: body.clone(),
                        part: part.clone(),
                        rels: rels.clone(),
                    },
                )
                .is_some()
            {
                duplicate.insert(key);
            }
        }
        for key in duplicate {
            notes.remove(&key);
            losses.push(NativeLoss {
                path: part.clone(),
                code: "word-note-duplicate-id".into(),
                detail: format!("{} ID {} 重复；不猜正文归属，原件保留", key.0, key.1),
            });
        }
    }
    Ok(notes)
}

#[derive(Default)]
pub(super) struct Notes {
    bodies: BTreeMap<(String, String), Option<RenderNode>>,
    assigned: BTreeMap<(String, String), (usize, String)>,
    parts: BTreeMap<String, Element>,
    next: BTreeMap<String, usize>,
    definitions: BTreeMap<(String, String), usize>,
    emitted: BTreeSet<(String, String)>,
}
impl Notes {
    pub fn scan(&mut self, n: &RenderNode) {
        if n.attrs
            .get("data-weftext-note-definition")
            .is_some_and(|v| v == "true")
            && let Some(key) = self.reference(n)
        {
            *self.definitions.entry(key).or_default() += 1;
        }
        if let Some(role) = n.attrs.get("data-weftext-role")
            && let Some(kind) = role
                .strip_suffix("-body")
                .filter(|s| matches!(*s, "footnote" | "endnote"))
            && let Some(id) = n.attrs.get("data-weftext-note-id")
        {
            self.bodies
                .entry((kind.into(), id.clone()))
                .and_modify(|v| *v = None)
                .or_insert(Some(n.clone()));
        }
        for c in &n.children {
            self.scan(c);
        }
    }
    pub fn is_body(n: &RenderNode) -> bool {
        matches!(
            n.attrs.get("data-weftext-role").map(String::as_str),
            Some("footnote-body" | "endnote-body" | "endnote-list" | "footnote-list" | "endnotes")
        )
    }
    pub fn reference(&self, n: &RenderNode) -> Option<(String, String)> {
        let kind = n
            .attrs
            .get("data-weftext-role")?
            .strip_suffix("-reference")?;
        if !matches!(kind, "footnote" | "endnote") {
            return None;
        }
        Some((kind.into(), n.attrs.get("data-weftext-note-id")?.clone()))
    }
    pub fn body(&self, key: &(String, String)) -> Option<RenderNode> {
        self.bodies.get(key).and_then(Clone::clone)
    }
    pub fn assigned(&self, key: &(String, String)) -> Option<(usize, String)> {
        self.assigned.get(key).cloned()
    }
    pub fn has_definition(&self, key: &(String, String)) -> bool {
        self.definitions.get(key) == Some(&1)
    }
    pub fn emitted(&self, key: &(String, String)) -> bool {
        self.emitted.contains(key)
    }
    pub fn allocate(
        &mut self,
        key: &(String, String),
        package: &Package,
    ) -> Result<(usize, String), DocxError> {
        if let Some(value) = self.assigned(key) {
            return Ok(value);
        }
        let kind = &key.0;
        if !self.parts.contains_key(kind) {
            let part = package.related_part(
                &format!("{}{kind}s", package::REL),
                &format!("word/{kind}s.xml"),
            )?;
            let root = package.xml(&part)?.unwrap_or_else(|| {
                let mut root = Element::new(&format!("w:{kind}s")).attr("xmlns:w", W).attr(
                    "xmlns:r",
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                );
                for (id, typ, marker) in [
                    (-1, "separator", "w:separator"),
                    (0, "continuationSeparator", "w:continuationSeparator"),
                ] {
                    let mut note = Element::new(&format!("w:{kind}"))
                        .attr("w:id", id)
                        .attr("w:type", typ);
                    let mut p = Element::new("w:p");
                    let mut r = Element::new("w:r");
                    r.push(Element::new(marker));
                    p.push(r);
                    note.push(p);
                    root.push(note);
                }
                root.attr(
                    "xmlns:m",
                    "http://schemas.openxmlformats.org/officeDocument/2006/math",
                )
                .attr(
                    "xmlns:a",
                    "http://schemas.openxmlformats.org/drawingml/2006/main",
                )
                .attr(
                    "xmlns:wp",
                    "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
                )
                .attr(
                    "xmlns:pic",
                    "http://schemas.openxmlformats.org/drawingml/2006/picture",
                )
            });
            self.next.insert(
                kind.clone(),
                existing_max(&root, &format!("w:{kind}"), "w:id") + 1,
            );
            self.parts.insert(kind.clone(), root);
        }
        let next = self.next.get_mut(kind).unwrap();
        let value = (*next, format!("weftext_{kind}_{}", *next));
        *next += 1;
        self.assigned.insert(key.clone(), value.clone());
        Ok(value)
    }
    pub fn push(&mut self, key: &(String, String), id: usize, mut paragraphs: Vec<Element>) {
        self.emitted.insert(key.clone());
        let kind = &key.0;
        let mut body = Element::new(&format!("w:{kind}")).attr("w:id", id);
        if paragraphs.is_empty() {
            paragraphs.push(Element::new("w:p"));
        }
        let mut mark = Element::new("w:r");
        let mut style = Element::new("w:rPr");
        style.push(val("w:vertAlign", "superscript"));
        mark.push(style);
        mark.push(Element::new(&format!("w:{kind}Ref")));
        if let Some(first) = paragraphs.first_mut().filter(|e| e.name == "w:p") {
            let at = usize::from(first.child("w:pPr").is_some());
            first.children.insert(at, Xml::Element(mark));
        } else {
            let mut p = Element::new("w:p");
            p.push(mark);
            paragraphs.insert(0, p);
        }
        for p in paragraphs {
            body.push(p);
        }
        self.parts.get_mut(kind).unwrap().push(body);
    }
    pub fn finish(&self, package: &mut Package, document: &mut Element) -> Result<(), DocxError> {
        if !self.parts.is_empty() {
            let mut settings = package
                .xml("word/settings.xml")?
                .unwrap_or_else(|| Element::new("w:settings").attr("xmlns:w", W));
            for kind in ["footnote", "endnote"]
                .into_iter()
                .filter(|kind| self.parts.contains_key(*kind))
            {
                let tag = format!("w:{kind}Pr");
                if settings.child(&tag).is_none() {
                    let mut properties = Element::new(&tag);
                    properties.push(val(
                        "w:pos",
                        if kind == "endnote" {
                            "docEnd"
                        } else {
                            "pageBottom"
                        },
                    ));
                    properties.push(val("w:numFmt", "decimal"));
                    properties.push(val("w:numStart", 1));
                    properties.push(val("w:numRestart", "continuous"));
                    let at = settings
                        .children
                        .iter()
                        .position(|child| {
                            child.element().is_some_and(|e| {
                                matches!(
                                    e.name.as_str(),
                                    "w:compat"
                                        | "w:docVars"
                                        | "w:rsids"
                                        | "m:mathPr"
                                        | "w:themeFontLang"
                                        | "w:clrSchemeMapping"
                                        | "w:updateFields"
                                ) || (kind == "footnote" && e.name == "w:endnotePr")
                            })
                        })
                        .unwrap_or(settings.children.len());
                    settings.children.insert(at, Xml::Element(properties));
                    // Word initializes an absent section endnote format to
                    // lower-Roman even when doc settings request decimal. The
                    // section must carry the same explicit native defaults.
                    // Preserve every existing template-specific note setting.
                    if let Some(body) = document.child_mut("w:body")
                        && body.child("w:sectPr").is_none()
                    {
                        body.push(Element::new("w:sectPr"));
                    }
                    fn sections(element: &mut Element, kind: &str, tag: &str) {
                        if element.name == "w:sectPr" && element.child(tag).is_none() {
                            let mut properties = Element::new(tag);
                            properties.push(val(
                                "w:pos",
                                if kind == "endnote" {
                                    "docEnd"
                                } else {
                                    "pageBottom"
                                },
                            ));
                            properties.push(val("w:numFmt", "decimal"));
                            properties.push(val("w:numStart", 1));
                            properties.push(val("w:numRestart", "continuous"));
                            let at = element
                                .children
                                .iter()
                                .position(|child| {
                                    child.element().is_some_and(|e| {
                                        !matches!(
                                            e.name.as_str(),
                                            "w:headerReference" | "w:footerReference"
                                        ) && !(kind == "endnote" && e.name == "w:footnotePr")
                                    })
                                })
                                .unwrap_or(element.children.len());
                            element.children.insert(at, Xml::Element(properties));
                        }
                        for child in element.children.iter_mut().filter_map(Xml::element_mut) {
                            sections(child, kind, tag);
                        }
                    }
                    sections(document, kind, &tag);
                }
            }
            package.set_xml("word/settings.xml", &settings);
        }
        for (kind, root) in &self.parts {
            let part = package.related_part(
                &format!("{}{kind}s", package::REL),
                &format!("word/{kind}s.xml"),
            )?;
            package.set_xml(&part, root);
            package.ensure_rel(&format!("{kind}s"), &format!("/{part}"));
            package.content_type(
                &format!("/{part}"),
                &format!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}s+xml"
                ),
            );
            // Every note part resolves its own relationships. Merge generated
            // relationships without replacing any template relationship IDs.
            let mut rels = package.part_relationships(&part)?;
            for rel in package
                .rels
                .children
                .iter()
                .filter_map(Xml::element)
                .filter(|r| {
                    matches!(
                        r.get("Type").and_then(|v| v.strip_prefix(package::REL)),
                        Some("image" | "hyperlink")
                    )
                })
            {
                if !rels
                    .children
                    .iter()
                    .filter_map(Xml::element)
                    .any(|r| r.get("Id") == rel.get("Id"))
                {
                    let mut relation = rel.clone();
                    if relation.get("TargetMode") != Some("External")
                        && let Some(target) = relation
                            .get("Target")
                            .filter(|t| !t.starts_with('/'))
                            .map(str::to_owned)
                    {
                        relation.set_attr("Target", &format!("/word/{target}"));
                    }
                    rels.push(relation);
                }
            }
            package.set_part_relationships(&part, &rels)?;
        }
        Ok(())
    }
}
