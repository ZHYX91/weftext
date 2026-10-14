//! DOCX to an internal native proposal. Core alone admits identities/writes.
use super::*;
use crate::exchange::{
    NATIVE_FORMAT, NATIVE_VERSION, NativeEntry, NativeFileRole as Role, NativeLoss, NativeManifest,
    NativePackage, digest, validate_native_package,
};
use serde_json::{Value, json};

fn id(op: &str, role: &str) -> String {
    let h = digest(format!("{op}:docx:{role}").as_bytes());
    let mut b = u128::from_str_radix(&h[..32], 16).unwrap().to_be_bytes();
    b[6] = (b[6] & 15) | 0x40;
    b[8] = (b[8] & 63) | 0x80;
    uuid::Uuid::from_bytes(b).to_string()
}
fn quoted(s: &str) -> String {
    weftext_data::NativeValue::Text(s.into())
        .to_typst()
        .unwrap()
}
fn loss(out: &mut Vec<NativeLoss>, code: &str, detail: impl Into<String>) {
    out.push(NativeLoss {
        path: "word/document.xml".into(),
        code: code.into(),
        detail: detail.into(),
    });
}
fn text(e: &Element) -> String {
    let mut out = String::new();
    fn walk(e: &Element, out: &mut String) {
        match e.name.as_str() {
            "w:t" | "w:delText" => out.push_str(&e.text()),
            "w:tab" => out.push('\t'),
            "w:br" | "w:cr" => out.push('\n'),
            _ => {
                for c in e.children.iter().filter_map(Xml::element) {
                    walk(c, out)
                }
            }
        }
    }
    walk(e, &mut out);
    out
}
struct Revision {
    executable: bool,
    element: Element,
    start: usize,
    end: usize,
    point: usize,
    replacement: String,
}
struct Builder<'a> {
    package: &'a Package,
    source: String,
    visible: String,
    positions: Vec<(usize, usize)>,
    starts: BTreeMap<String, usize>,
    ends: BTreeMap<String, usize>,
    invalid_ranges: BTreeSet<String>,
    revision_depth: usize,
    revisions: Vec<Revision>,
    assets: BTreeMap<String, Vec<u8>>,
    losses: Vec<NativeLoss>,
}
impl Builder<'_> {
    fn literal(&mut self, s: &str) -> Result<(), DocxError> {
        for c in s.chars() {
            let start = self.source.len();
            let encoded = weftext_engine::encode_literal(&c.to_string())
                .map_err(|e| DocxError::new(e.to_string()))?;
            self.source.push_str(&encoded);
            self.positions.push((start, self.source.len()));
            self.visible.push(c);
        }
        Ok(())
    }
    fn paragraph(&mut self, p: &Element, in_cell: bool) -> Result<(), DocxError> {
        if !in_cell {
            self.source.push_str("\n\n");
        }
        if let Some(pr) = p.child("w:pPr") {
            let heading = pr
                .child("w:outlineLvl")
                .and_then(|v| v.get("w:val"))
                .and_then(|s| s.parse::<usize>().ok())
                .map(|n| n + 1)
                .or_else(|| {
                    pr.child("w:pStyle")
                        .and_then(|v| v.get("w:val"))
                        .and_then(|s| {
                            s.strip_prefix("Heading")
                                .or_else(|| s.strip_prefix("heading"))
                        })
                        .and_then(|n| n.parse::<usize>().ok())
                });
            if let Some(level) = heading.filter(|n| (1..=9).contains(n))
                && !in_cell
            {
                self.source.push_str(&"=".repeat(level));
                self.source.push(' ');
            } else if pr.child("w:numPr").is_some() && !in_cell {
                self.source.push_str("- ");
                loss(
                    &mut self.losses,
                    "word-list-numbering",
                    "列表内容保留为原生列表；原编号格式、层级及起始值未完整映射",
                );
            }
            if pr.child("w:pPrChange").is_some() {
                loss(
                    &mut self.losses,
                    "word-format-revision",
                    "段落格式修订保留在原件；本次未作为可执行文字建议导入",
                );
            }
        }
        for c in p
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|c| c.name != "w:pPr")
        {
            self.inline(c)?;
        }
        Ok(())
    }
    fn inline(&mut self, e: &Element) -> Result<(), DocxError> {
        match e.name.as_str() {
            "w:commentRangeStart" => {
                if let Some(id) = e.get("w:id")
                    && self
                        .starts
                        .insert(id.into(), self.positions.len())
                        .is_some()
                {
                    loss(
                        &mut self.losses,
                        "word-comment-range",
                        "重复批注起点无法唯一映射；保留原件",
                    );
                    self.invalid_ranges.insert(id.into());
                }
            }
            "w:commentRangeEnd" => {
                if let Some(id) = e.get("w:id")
                    && self.ends.insert(id.into(), self.positions.len()).is_some()
                {
                    self.invalid_ranges.insert(id.into());
                }
            }
            "w:commentReference" => {}
            "w:ins" | "w:del" => {
                let start = self.positions.len();
                let point = self.source.len();
                let mut nested = vec![];
                e.descendants("w:ins", &mut nested);
                e.descendants("w:del", &mut nested);
                let executable = nested.len() == 1 && self.revision_depth == 0;
                if !executable {
                    loss(
                        &mut self.losses,
                        "word-nested-revision",
                        "嵌套修订未获得自动应用资格，原件保留",
                    );
                }
                self.revision_depth += 1;
                if e.name == "w:del" {
                    for c in e.children.iter().filter_map(Xml::element) {
                        self.inline(c)?;
                    }
                }
                self.revision_depth -= 1;
                let replacement = if e.name == "w:ins" {
                    text(e)
                } else {
                    String::new()
                };
                self.revisions.push(Revision {
                    executable,
                    element: e.clone(),
                    start,
                    end: self.positions.len(),
                    point,
                    replacement,
                });
            }
            "w:r" => {
                let pr = e.child("w:rPr");
                let on = |name: &str| {
                    pr.and_then(|p| p.child(name))
                        .is_some_and(|v| !matches!(v.get("w:val"), Some("0" | "false" | "off")))
                };
                let has_text = !text(e).is_empty();
                let bold = has_text && on("w:b");
                let italic = has_text && on("w:i");
                if pr.and_then(|p| p.child("w:rPrChange")).is_some() {
                    loss(
                        &mut self.losses,
                        "word-format-revision",
                        "字符格式修订保留在原件，未转换为文字修改建议",
                    );
                }
                if bold {
                    self.source.push_str("#strong[");
                }
                if italic {
                    self.source.push_str("#emph[");
                }
                for c in e
                    .children
                    .iter()
                    .filter_map(Xml::element)
                    .filter(|c| c.name != "w:rPr")
                {
                    self.inline(c)?;
                }
                if italic {
                    self.source.push(']');
                }
                if bold {
                    self.source.push(']');
                }
            }
            "w:t" | "w:delText" => self.literal(&e.text())?,
            "w:tab" => self.literal("\t")?,
            "w:br" | "w:cr" => {
                self.source.push_str("#linebreak()\n");
            }
            "w:hyperlink" => {
                let relation = e
                    .get("r:id")
                    .and_then(|id| {
                        self.package
                            .rels
                            .children
                            .iter()
                            .filter_map(Xml::element)
                            .find(|r| r.get("Id") == Some(id))
                    })
                    .and_then(|r| r.get("Target"));
                if let Some(url) = relation.filter(|u| {
                    u.starts_with("https://")
                        || u.starts_with("http://")
                        || u.starts_with("mailto:")
                }) {
                    self.source.push_str(&format!("#link({})[", quoted(url)));
                    for c in e.children.iter().filter_map(Xml::element) {
                        self.inline(c)?;
                    }
                    self.source.push(']');
                } else {
                    loss(
                        &mut self.losses,
                        "word-link",
                        "链接目标无法安全映射，保留可见文字与原件",
                    );
                    for c in e.children.iter().filter_map(Xml::element) {
                        self.inline(c)?;
                    }
                }
            }
            "w:drawing" => {
                let mut blips = vec![];
                e.descendants("a:blip", &mut blips);
                for blip in blips {
                    let rel = blip.get("r:embed").and_then(|id| {
                        self.package
                            .rels
                            .children
                            .iter()
                            .filter_map(Xml::element)
                            .find(|r| r.get("Id") == Some(id))
                    });
                    let target = rel
                        .filter(|r| r.get("TargetMode") != Some("External"))
                        .and_then(|r| r.get("Target"));
                    let path = target.map(|t| {
                        if t.starts_with("/word/") {
                            t.trim_start_matches('/').to_owned()
                        } else {
                            format!("word/{t}")
                        }
                    });
                    if let Some((path, bytes)) = path
                        .as_ref()
                        .and_then(|p| self.package.parts.get(p).map(|b| (p, b)))
                    {
                        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "svg") {
                            loss(
                                &mut self.losses,
                                "word-image-format",
                                "图片格式不在首轮支持范围，原件保留",
                            );
                            continue;
                        }
                        let name = format!("image-{}.{}", self.assets.len() + 1, ext);
                        self.assets.insert(name.clone(), bytes.clone());
                        self.source.push_str(&format!("#image({})", quoted(&name)));
                        loss(
                            &mut self.losses,
                            "word-image-layout",
                            "图片资源保留，Word 浮动位置、裁切和尺寸未完整映射",
                        );
                    } else {
                        loss(
                            &mut self.losses,
                            "word-image-missing",
                            "图片关系不可读取，原件保留",
                        );
                    }
                }
            }
            "w:fldChar" | "w:instrText" => {
                loss(
                    &mut self.losses,
                    "word-field",
                    "保留域的可见结果；动态域和更新语义保留在原件",
                );
            }
            "w:bookmarkStart" | "w:bookmarkEnd" | "w:proofErr" | "w:lastRenderedPageBreak" => {}
            _ => {
                loss(
                    &mut self.losses,
                    "word-element",
                    format!("未完整支持 {}，保留可读取文字及原件", e.name),
                );
                for c in e.children.iter().filter_map(Xml::element) {
                    self.inline(c)?;
                }
            }
        }
        Ok(())
    }
    fn blocks(&mut self, e: &Element) -> Result<(), DocxError> {
        for c in e.children.iter().filter_map(Xml::element) {
            match c.name.as_str() {
                "w:p" => self.paragraph(c, false)?,
                "w:tbl" => {
                    let rows: Vec<_> = c
                        .children
                        .iter()
                        .filter_map(Xml::element)
                        .filter(|e| e.name == "w:tr")
                        .collect();
                    let columns = rows
                        .iter()
                        .map(|r| {
                            r.children
                                .iter()
                                .filter_map(Xml::element)
                                .filter(|e| e.name == "w:tc")
                                .count()
                        })
                        .max()
                        .unwrap_or(0);
                    if columns == 0 {
                        continue;
                    }
                    self.source
                        .push_str(&format!("\n\n#table(columns: {columns},\n"));
                    for row in rows {
                        let cells: Vec<_> = row
                            .children
                            .iter()
                            .filter_map(Xml::element)
                            .filter(|e| e.name == "w:tc")
                            .collect();
                        for cell in &cells {
                            self.source.push('[');
                            let mut first = true;
                            for p in cell
                                .children
                                .iter()
                                .filter_map(Xml::element)
                                .filter(|e| e.name == "w:p")
                            {
                                if !first {
                                    self.source.push_str("\n\n");
                                }
                                self.paragraph(p, true)?;
                                first = false;
                            }
                            self.source.push_str("],\n");
                        }
                        for _ in cells.len()..columns {
                            self.source.push_str("[],\n");
                        }
                    }
                    self.source.push_str(")\n");
                    loss(
                        &mut self.losses,
                        "word-table-layout",
                        "普通表格文字与单元格顺序保留；合并、列宽、边框及复杂嵌套结构未完整映射",
                    );
                }
                "w:sectPr" => {}
                _ => {
                    loss(
                        &mut self.losses,
                        "word-block",
                        format!("块 {} 未完整映射，保留可读取段落与原件", c.name),
                    );
                    self.blocks(c)?;
                }
            }
        }
        Ok(())
    }
    fn range(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        if start >= end {
            return None;
        }
        Some((self.positions.get(start)?.0, self.positions.get(end - 1)?.1))
    }
}

pub fn build_docx_native_package(
    bytes: &[u8],
    schema: &str,
    operation_id: &str,
) -> crate::Result<NativePackage> {
    build(bytes, schema, operation_id).map_err(Into::into)
}
fn build(bytes: &[u8], schema: &str, op: &str) -> Result<NativePackage, DocxError> {
    if !weftext_data::uuid_v4(op) {
        return Err(DocxError::new("Word 导入需要有效操作标识"));
    }
    weftext_data::Schema::from_author_json(schema).map_err(|e| DocxError::new(e.to_string()))?;
    let package = Package::load(Some(bytes))?;
    let doc = package
        .xml("word/document.xml")?
        .ok_or_else(|| DocxError::new("缺少 Word 正文"))?;
    let body = doc
        .child("w:body")
        .ok_or_else(|| DocxError::new("缺少 Word body"))?;
    let workspace = id(op, "workspace");
    let node = id(op, "node");
    let revision = id(op, "revision");
    let original = id(op, "original");
    let mut b = Builder {
        package: &package,
        source: format!("#metadata((id: {})) <weftext>\n", quoted(&node)),
        visible: String::new(),
        positions: vec![],
        starts: BTreeMap::new(),
        ends: BTreeMap::new(),
        invalid_ranges: BTreeSet::new(),
        revision_depth: 0,
        revisions: vec![],
        assets: BTreeMap::new(),
        losses: vec![],
    };
    b.blocks(body)?;
    if !weftext_engine::inspect_editor(&b.source).syntax_valid {
        return Err(DocxError::new(
            "转换结果未通过原生语法校验，保留上传原件且不创建笔记",
        ));
    }
    let comments_path =
        package.related_part(&format!("{}comments", package::REL), "word/comments.xml")?;
    let extended_path = package.related_part(
        "http://schemas.microsoft.com/office/2011/relationships/commentsExtended",
        "word/commentsExtended.xml",
    )?;
    let comments = package.xml(&comments_path)?;
    let extended = package.xml(&extended_path)?;
    let mut metadata = BTreeMap::new();
    let mut comment_ids = BTreeMap::new();
    if let Some(comments) = &comments {
        for c in comments
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|e| e.name == "w:comment")
        {
            let key = c
                .get("w:id")
                .ok_or_else(|| DocxError::new("批注缺少标识，不能准确映射"))?;
            if comment_ids
                .insert(key.to_owned(), id(op, &format!("comment:{key}")))
                .is_some()
            {
                return Err(DocxError::new("重复 Word 批注标识，不能猜测归属"));
            }
            if let Some(pid) = c
                .children
                .iter()
                .filter_map(Xml::element)
                .rfind(|p| p.name == "w:p")
                .and_then(|p| p.get("w14:paraId"))
                && metadata.insert(pid.to_owned(), key.to_owned()).is_some()
            {
                return Err(DocxError::new("重复 Word 批注段落标识，不能猜测回复归属"));
            }
        }
    }
    let node_ref = json!({"workspaceId":workspace,"nodeId":node});
    let target = |range: Option<(usize, usize)>, excerpt: &str| -> Value {
        if let Some((from, to)) = range {
            json!({"kind":"sourceRange","anchor":{"node":node_ref,"revisionId":revision,"sourceRange":{"from":from,"to":to,"utf16From":b.source[..from].encode_utf16().count(),"utf16To":b.source[..to].encode_utf16().count(),"raw":&b.source[from..to]},"visibleExcerpt":excerpt}})
        } else {
            json!({"kind":"wholeResource","resource":format!("weftext-resource://{workspace}/{original}"),"revisionId":id(op,"original-revision")})
        }
    };
    let mut annotations = vec![];
    if let Some(comments) = &comments {
        for c in comments
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|e| e.name == "w:comment")
        {
            let key = c.get("w:id").unwrap();
            let para = c
                .children
                .iter()
                .filter_map(Xml::element)
                .rfind(|e| e.name == "w:p")
                .and_then(|e| e.get("w14:paraId"));
            let ext = extended.as_ref().and_then(|e| {
                e.children
                    .iter()
                    .filter_map(Xml::element)
                    .find(|e| e.get("w15:paraId") == para && para.is_some())
            });
            let parent = ext
                .and_then(|e| e.get("w15:paraIdParent"))
                .and_then(|p| metadata.get(p));
            let range = b
                .starts
                .get(key)
                .zip(b.ends.get(key))
                .filter(|_| !b.invalid_ranges.contains(key))
                .and_then(|(a, z)| b.range(*a, *z));
            let content = c
                .children
                .iter()
                .filter_map(Xml::element)
                .filter(|e| e.name == "w:p")
                .map(text)
                .collect::<Vec<_>>()
                .join("\n");
            let excerpt = range
                .map(|(from, to)| b.source[from..to].to_owned())
                .unwrap_or_default();
            annotations.push(json!({"annotationId":comment_ids[key],"replyTo":parent.and_then(|p|comment_ids.get(p)),"attribution":c.get("w:author").unwrap_or("未知作者"),"createdAt":c.get("w:date").unwrap_or("unknown"),"purpose":"comment","body":content,"labels":["word-import"],"appearance":{},"reviewState":if ext.is_some_and(|e|e.get("w15:done")==Some("1")){"resolved"}else{"open"},"target":target(range,&excerpt),"suggestion":null,"targetHistory":[],"wordImport":{"part":"word/comments.xml","originalId":key,"originalResourcePath":"original.docx","unlocated":range.is_none() && parent.is_none()}}));
        }
    }
    for (n, r) in b.revisions.iter().enumerate() {
        let range = if !r.executable {
            None
        } else if r.element.name == "w:ins" {
            Some((r.point, r.point))
        } else {
            b.range(r.start, r.end)
        };
        let kind = if r.element.name == "w:ins" {
            "insert"
        } else {
            "delete"
        };
        annotations.push(json!({"annotationId":id(op,&format!("revision:{n}")),"replyTo":null,"attribution":r.element.get("w:author").unwrap_or("未知作者"),"createdAt":r.element.get("w:date").unwrap_or("unknown"),"purpose":"suggestion","body":format!("Word 待审文字修订：{kind}"),"labels":["word-import","revision"],"appearance":{},"reviewState":"open","target":target(range,&text(&r.element)),"suggestion":{"replacement":r.replacement,"state":"pending","confirmedRevision":revision,"requiresReconfirmation":range.is_none()},"targetHistory":[],"wordImport":{"part":"word/document.xml","originalId":r.element.get("w:id"),"originalResourcePath":"original.docx","revisionKind":kind}}));
    }
    // Replies share their actual parent's anchor, including unlocated originals.
    for _ in 0..annotations.len() {
        let targets: BTreeMap<_, _> = annotations
            .iter()
            .map(|a| {
                (
                    a["annotationId"].as_str().unwrap().to_owned(),
                    a["target"].clone(),
                )
            })
            .collect();
        for a in &mut annotations {
            if let Some(parent) = a["replyTo"].as_str().and_then(|p| targets.get(p)) {
                a["target"] = parent.clone();
            }
        }
    }
    let mut losses = b.losses;
    for a in &annotations {
        if a["wordImport"]["unlocated"] == true {
            loss(
                &mut losses,
                "word-comment-unlocated",
                format!(
                    "批注 {} 保留在原件资料上，未能准确定位到新正文",
                    a["wordImport"]["originalId"]
                ),
            );
        }
    }
    loss(
        &mut losses,
        "word-layout",
        "正文以原生可编辑结构导入；Word 页眉页脚、分页、主题字体及复杂排版保留在原件，不承诺版式等价",
    );
    let mut native = NativePackage {
        manifest: NativeManifest {
            format: NATIVE_FORMAT.into(),
            version: NATIVE_VERSION,
            package_id: id(op, "package"),
            workspace_id: workspace,
            scope: "library".into(),
            schema_path: ".weftext-schema.json".into(),
            entries: BTreeMap::new(),
            losses,
        },
        files: BTreeMap::new(),
    };
    let mut insert = |path: String,
                      bytes: Vec<u8>,
                      role: Role,
                      resource: Option<String>,
                      rev: Option<String>| {
        let mut entry = NativeEntry::new(role.clone(), &bytes);
        if role == Role::Source {
            entry.node_id = Some(node.clone());
        } else if role != Role::Schema {
            entry.owner_node_id = Some(node.clone());
        }
        entry.resource_id = resource;
        entry.revision = rev;
        native.manifest.entries.insert(path.clone(), entry);
        native.files.insert(path, bytes);
    };
    insert(
        ".weftext-schema.json".into(),
        schema.as_bytes().to_vec(),
        Role::Schema,
        None,
        None,
    );
    insert(
        "document/document.typ".into(),
        b.source.into_bytes(),
        Role::Source,
        None,
        Some(revision),
    );
    insert(
        "document/weftext.annotations.json".into(),
        serde_json::to_vec_pretty(&json!({"version":1,"annotations":annotations})).unwrap(),
        Role::Annotations,
        None,
        None,
    );
    insert(
        "document/original.docx".into(),
        bytes.to_vec(),
        Role::Resource,
        Some(original.clone()),
        Some(id(op, "original-revision")),
    );
    let mut resources = vec![json!({"resourceId":original,"path":"original.docx"})];
    for (name, data) in b.assets {
        let resource = id(op, &format!("asset:{name}"));
        resources.push(json!({"resourceId":resource,"path":name}));
        insert(
            format!("document/{name}"),
            data,
            Role::Resource,
            Some(resource),
            Some(id(op, &format!("asset-revision:{name}"))),
        );
    }
    insert(
        "document/weftext.resources.json".into(),
        serde_json::to_vec_pretty(&json!({"version":1,"resources":resources})).unwrap(),
        Role::ResourceManifest,
        None,
        None,
    );
    validate_native_package(&native).map_err(|e| DocxError::new(e.to_string()))?;
    Ok(native)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_ranges_and_nested_revisions_keep_original_without_write_authority() {
        let mut p = Package::load(None).unwrap();
        p.parts.insert("word/document.xml".into(),format!(r#"<w:document xmlns:w="{}"><w:body><w:p><w:r><w:rPr><w:b/><w:i/></w:rPr><w:commentReference w:id="0"/></w:r><w:commentRangeStart w:id="0"/><w:commentRangeStart w:id="0"/><w:commentRangeStart w:id="0"/><w:r><w:rPr><w:b/></w:rPr><w:t> A#[]😀 </w:t></w:r><w:commentRangeEnd w:id="0"/><w:del w:id="1"><w:r><w:delText>删除</w:delText></w:r><w:ins w:id="2"><w:r><w:t>嵌套</w:t></w:r></w:ins></w:del></w:p></w:body></w:document>"#,package::W).into_bytes());
        p.parts.insert("word/custom-comments.xml".into(),format!(r#"<w:comments xmlns:w="{}"><w:comment w:id="0" w:author="甲"><w:p><w:r><w:t>重复目标保留意见</w:t></w:r></w:p></w:comment></w:comments>"#,package::W).into_bytes());
        p.ensure_rel("comments", "custom-comments.xml");
        let bytes = p.finish().unwrap();
        let imported = build_docx_native_package(
            &bytes,
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap();
        assert_eq!(imported.files["document/original.docx"], bytes);
        let a: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let records = a["annotations"].as_array().unwrap();
        assert_eq!(records.len(), 3);
        assert!(
            records
                .iter()
                .all(|r| r["target"]["kind"] == "wholeResource")
        );
        assert!(
            records
                .iter()
                .filter(|r| !r["suggestion"].is_null())
                .all(|r| r["suggestion"]["requiresReconfirmation"] == true)
        );
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-nested-revision")
        );
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-comment-unlocated")
        );
    }
    #[test]
    fn imports_pending_text_and_comments_without_accepting_changes() {
        let mut p = Package::load(None).unwrap();
        let xml = format!(
            r#"<w:document xmlns:w="{}"><w:body><w:p><w:commentRangeStart w:id="0"/><w:r><w:t>原文😀</w:t></w:r><w:commentRangeEnd w:id="0"/><w:ins w:id="1" w:author="审阅人" w:date="2026-10-14T00:00:00Z"><w:r><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:del w:id="2" w:author="审阅人"><w:r><w:delText>删除</w:delText></w:r></w:del></w:p></w:body></w:document>"#,
            package::W
        );
        p.parts.insert("word/document.xml".into(), xml.into_bytes());
        p.parts.insert("word/comments.xml".into(),format!(r#"<w:comments xmlns:w="{}"><w:comment w:id="0" w:author="甲"><w:p><w:r><w:t>核对原文</w:t></w:r></w:p></w:comment></w:comments>"#,package::W).into_bytes());
        let original = p.finish().unwrap();
        let proposal = build_docx_native_package(
            &original,
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap();
        let source = std::str::from_utf8(&proposal.files["document/document.typ"]).unwrap();
        assert!(source.contains("原文😀后文删除"));
        assert!(!source.contains("新增"));
        assert_eq!(proposal.files["document/original.docx"], original);
        let a: Value =
            serde_json::from_slice(&proposal.files["document/weftext.annotations.json"]).unwrap();
        assert_eq!(a["annotations"].as_array().unwrap().len(), 3);
        assert_eq!(
            a["annotations"][0]["target"]["anchor"]["sourceRange"]["raw"],
            "原文😀"
        );
        assert_eq!(a["annotations"][1]["suggestion"]["replacement"], "新增");
        assert_eq!(
            a["annotations"][1]["target"]["anchor"]["sourceRange"]["from"],
            a["annotations"][1]["target"]["anchor"]["sourceRange"]["to"]
        );
        assert_eq!(
            a["annotations"][2]["target"]["anchor"]["sourceRange"]["raw"],
            "删除"
        );
    }
}
