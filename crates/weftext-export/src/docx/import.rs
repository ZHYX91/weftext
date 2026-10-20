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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalImportAsset {
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// Attach exact earlier conversion inputs; mutation is transactional and all
/// members enter the same Core creation plan as the editable document.
pub fn retain_docx_import_originals(
    package: &mut NativePackage,
    operation_id: &str,
    originals: &[OriginalImportAsset],
    conversion_losses: &[NativeLoss],
) -> crate::Result<()> {
    if !weftext_data::uuid_v4(operation_id) || originals.len() > 4 || conversion_losses.len() > 4096
    {
        return Err(ExportError("旧格式原件数量／操作标识／损失清单无效".into()));
    }
    let mut next = package.clone();
    let owner = next
        .manifest
        .entries
        .get("document/document.typ")
        .and_then(|e| e.node_id.clone())
        .ok_or_else(|| ExportError("导入包缺少文档owner".into()))?;
    let manifest_path = "document/weftext.resources.json";
    let mut resources: Value = serde_json::from_slice(
        next.files
            .get(manifest_path)
            .ok_or_else(|| ExportError("导入包缺少资源manifest".into()))?,
    )?;
    let entries = resources["resources"]
        .as_array_mut()
        .ok_or_else(|| ExportError("导入包资源manifest无效".into()))?;
    let mut total = 0usize;
    for (index, asset) in originals.iter().enumerate() {
        total = total
            .checked_add(asset.bytes.len())
            .ok_or_else(|| ExportError("旧格式原件预算溢出".into()))?;
        if asset.filename.is_empty()
            || asset.filename.len() > 255
            || asset
                .filename
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
            || asset.mime.is_empty()
            || asset.mime.len() > 128
            || asset.mime.chars().any(char::is_control)
            || asset.bytes.is_empty()
            || asset.bytes.len() > 16 * 1024 * 1024
            || total > 32 * 1024 * 1024
        {
            return Err(ExportError(
                "旧格式原件名称／媒体类型／大小超出安全范围".into(),
            ));
        }
        let ext = asset
            .filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "doc" | "wps" | "docx" | "rtf" | "odt") {
            return Err(ExportError("旧格式原件后缀不受支持".into()));
        }
        let name = format!("original-input-{}.{}", index + 1, ext);
        let path = format!("document/{name}");
        if next.files.contains_key(&path) {
            return Err(ExportError("旧格式原件资源槽已存在".into()));
        }
        let resource = id(
            operation_id,
            &format!("original-input:{index}:{}", digest(&asset.bytes)),
        );
        let mut entry = NativeEntry::new(Role::Resource, &asset.bytes);
        entry.owner_node_id = Some(owner.clone());
        entry.resource_id = Some(resource.clone());
        entry.revision = Some(id(
            operation_id,
            &format!("original-input-revision:{index}"),
        ));
        next.files.insert(path.clone(), asset.bytes.clone());
        next.manifest.entries.insert(path.clone(), entry);
        entries.push(json!({"resourceId":resource,"path":name}));
        next.manifest.losses.push(NativeLoss {
            path,
            code: "original-conversion-input-retained".into(),
            detail: format!(
                "原始输入 {} ({}) 的准确字节保留；不以中间DOCX代替原件",
                asset.filename, asset.mime
            ),
        });
    }
    if entries.len() > 64 {
        return Err(ExportError("导入资源数量超过64项".into()));
    }
    let bytes = serde_json::to_vec_pretty(&resources)?;
    let mut entry = NativeEntry::new(Role::ResourceManifest, &bytes);
    entry.owner_node_id = Some(owner);
    next.files.insert(manifest_path.into(), bytes);
    next.manifest.entries.insert(manifest_path.into(), entry);
    next.manifest.losses.extend_from_slice(conversion_losses);
    validate_native_package(&next).map_err(|e| ExportError(e.to_string()))?;
    *package = next;
    Ok(())
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
#[derive(Clone, Default)]
struct WordLanguage {
    values: BTreeMap<String, String>,
    unsupported: BTreeSet<String>,
}
impl WordLanguage {
    fn apply(&mut self, pr: Option<&Element>) {
        if let Some(lang) = pr.and_then(|p| p.child("w:lang")) {
            for key in ["w:val", "w:eastAsia", "w:bidi"] {
                if let Some(value) = lang.get(key) {
                    self.values.insert(key.into(), value.into());
                }
            }
            for (key, value) in &lang.attrs {
                if !matches!(key.as_str(), "w:val" | "w:eastAsia" | "w:bidi") {
                    self.unsupported.insert(format!("{key}={value}"));
                }
            }
        }
    }
    fn wrapper(&self, slot: &str, losses: &mut Vec<NativeLoss>) -> Option<String> {
        let tag = self.values.get(slot)?;
        let tokens: Vec<_> = tag.split('-').collect();
        if !(2..=8).contains(&tokens[0].len())
            || !tokens[0].bytes().all(|c| c.is_ascii_alphabetic())
        {
            loss(
                losses,
                "word-language-invalid",
                format!("Word语言槽 {slot}={tag} 不能安全映射；准确原件保留"),
            );
            return None;
        }
        let mut region = None;
        for token in tokens.iter().skip(1) {
            if (token.len() == 2 && token.bytes().all(|c| c.is_ascii_alphabetic()))
                || (token.len() == 3 && token.bytes().all(|c| c.is_ascii_digit()))
            {
                if region.is_some() {
                    loss(
                        losses,
                        "word-language-subtag",
                        format!("语言标记 {tag} 含多个地区子标记，不能证明唯一语言意图；原件保留"),
                    );
                }
                region = Some(token.to_ascii_uppercase());
            } else {
                loss(
                    losses,
                    "word-language-subtag",
                    format!("语言标记 {tag} 的文字体系／扩展子标记未映射；原件保留"),
                );
            }
        }
        Some(format!(
            "#text(lang: {}{})[",
            quoted(&tokens[0].to_ascii_lowercase()),
            region
                .map(|r| format!(", region: {}", quoted(&r)))
                .unwrap_or_else(|| ", region: none".into())
        ))
    }
    fn segments(&self, text: &str, losses: &mut Vec<NativeLoss>) -> Vec<(String, Option<String>)> {
        let mut result: Vec<(String, Option<String>)> = vec![];
        let mut used = BTreeSet::new();
        let mut unknown = BTreeSet::new();
        let mut missing = BTreeSet::new();
        let mut wrappers = BTreeMap::new();
        let mut previous = None;
        for c in text.chars() {
            // This is a bounded script mapping, not Word's font selection or an
            // inference from neighbouring runs. Common ASCII stays in val;
            // CJK punctuation has its own East Asian Unicode range. Inherited
            // combining marks only share a known base within this text node.
            let slot = if unicode_normalization::char::canonical_combining_class(c) != 0 {
                word_language_slot(c).or(previous)
            } else {
                word_language_slot(c)
            };
            let wrapper = if let Some(slot) = slot {
                used.insert(slot);
                if !self.values.contains_key(slot) && slot != "w:val" {
                    missing.insert(slot);
                }
                wrappers
                    .entry(slot)
                    .or_insert_with(|| self.wrapper(slot, losses))
                    .clone()
            } else {
                unknown.insert(c);
                None
            };
            if unicode_normalization::char::canonical_combining_class(c) == 0 {
                previous = slot;
            }
            if let Some((content, _)) = result.last_mut().filter(|(_, current)| *current == wrapper)
            {
                content.push(c);
            } else {
                result.push((c.to_string(), wrapper));
            }
        }
        if !unknown.is_empty() {
            loss(
                losses,
                "word-language-script",
                format!(
                    "字符 {} 的语言脚本槽尚未可靠映射；文字与准确原件保留，未猜测语言",
                    unknown
                        .iter()
                        .map(|c| format!("U+{:04X}", *c as u32))
                        .collect::<Vec<_>>()
                        .join("、")
                ),
            );
        }
        for slot in missing {
            loss(
                losses,
                "word-language-missing",
                format!("已确认脚本所需 {slot} 语言槽没有继承值；未借通用语言猜测，准确原件保留"),
            );
        }
        for property in &self.unsupported {
            loss(
                losses,
                "word-language-property",
                format!("Word语言属性 {property} 未映射；准确原件保留"),
            );
        }
        for slot in ["w:eastAsia", "w:bidi"] {
            if !used.contains(slot)
                && let Some(value) = self
                    .values
                    .get(slot)
                    .filter(|v| Some(*v) != self.values.get("w:val"))
            {
                loss(
                    losses,
                    "word-language-specialized",
                    format!(
                        "{slot}={value} 未被本run中已确认脚本的文字使用；该设置仅保留在准确原件"
                    ),
                );
            }
        }
        result
    }
}
// Only scripts whose Word language-slot meaning is established here are
// consumed. Other alphabetic scripts remain explicit losses, not guessed val.
fn word_language_slot(c: char) -> Option<&'static str> {
    match c as u32 {
        0x0590..=0x05ff
        | 0x0600..=0x06ff
        | 0x0750..=0x077f
        | 0x0870..=0x089f
        | 0x08a0..=0x08ff
        | 0xfb1d..=0xfb4f
        | 0xfb50..=0xfdff
        | 0xfe70..=0xfeff => Some("w:bidi"),
        0x1100..=0x11ff
        | 0x2e80..=0x2eff
        | 0x2f00..=0x2fdf
        | 0x2ff0..=0x31ff
        | 0x3200..=0x33ff
        | 0x3400..=0x4dbf
        | 0x4e00..=0x9fff
        | 0xa960..=0xa97f
        | 0xac00..=0xd7ff
        | 0xf900..=0xfaff
        | 0xfe10..=0xfe1f
        | 0xfe30..=0xfe4f
        | 0xff00..=0xffef
        | 0x1b000..=0x1b16f
        | 0x20000..=0x2a6df
        | 0x2a700..=0x2b73f
        | 0x2b740..=0x2b81f
        | 0x2b820..=0x2ceaf
        | 0x2ceb0..=0x2ee5f
        | 0x2f800..=0x2fa1f
        | 0x30000..=0x323af => Some("w:eastAsia"),
        0x0000..=0x024f | 0x0370..=0x052f | 0x1e00..=0x1fff | 0x2de0..=0x2dff | 0xa640..=0xa69f => {
            Some("w:val")
        }
        _ if unicode_normalization::char::canonical_combining_class(c) != 0
            || c.is_alphabetic() =>
        {
            None
        }
        _ => Some("w:val"),
    }
}
#[derive(Clone, Default)]
struct LanguageStyles {
    defaults: WordLanguage,
    styles: BTreeMap<String, Element>,
    duplicates: BTreeSet<String>,
    paragraph_default: Option<String>,
    character_default: Option<String>,
}
impl LanguageStyles {
    fn heading_level(
        &self,
        paragraph: &Element,
        style_id: Option<&str>,
        losses: &mut Vec<NativeLoss>,
    ) -> Option<usize> {
        fn only_child<'a>(parent: &'a Element, name: &str) -> Result<Option<&'a Element>, ()> {
            let mut children = parent
                .children
                .iter()
                .filter_map(Xml::element)
                .filter(|child| child.name == name);
            let first = children.next();
            if children.next().is_some() {
                Err(())
            } else {
                Ok(first)
            }
        }
        fn outline(properties: Option<&Element>) -> Result<Option<usize>, ()> {
            let Some(properties) = properties else {
                return Ok(None);
            };
            only_child(properties, "w:outlineLvl")?
                .map(|value| {
                    value
                        .get("w:val")
                        .and_then(|value| value.parse::<usize>().ok())
                        .filter(|level| *level <= 9)
                        .ok_or(())
                })
                .transpose()
        }
        let resolved = (|| {
            let properties = only_child(paragraph, "w:pPr")?;
            // Validate the complete bounded inheritance chain before accepting
            // a level. Neither localized names nor Word's rewritten style IDs
            // establish a heading role.
            if let Some(properties) = properties
                && let Some(style) = only_child(properties, "w:pStyle")?
            {
                style.get("w:val").filter(|id| !id.is_empty()).ok_or(())?;
            }
            let direct = outline(properties)?;
            let mut inherited = None;
            let mut visited = BTreeSet::new();
            let mut current = style_id;
            while let Some(id) = current {
                if visited.len() >= 32 || !visited.insert(id) || self.duplicates.contains(id) {
                    return Err(());
                }
                let style = self
                    .styles
                    .get(id)
                    .filter(|style| style.get("w:type") == Some("paragraph"))
                    .ok_or(())?;
                let properties = only_child(style, "w:pPr")?;
                let level = outline(properties)?;
                if inherited.is_none() {
                    inherited = level;
                }
                current = only_child(style, "w:basedOn")?
                    .map(|parent| parent.get("w:val").filter(|id| !id.is_empty()).ok_or(()))
                    .transpose()?;
            }
            Ok(direct
                .or(inherited)
                .filter(|level| *level < 9)
                .map(|level| level + 1))
        })();
        match resolved {
            Ok(level) => level,
            Err(()) => {
                loss(
                    losses,
                    "word-heading-style",
                    "标题层级属性或段落样式继承不唯一／无效／循环／缺失；正文保留，未猜测标题层级，准确原件保留",
                );
                None
            }
        }
    }
    fn read(package: &Package, losses: &mut Vec<NativeLoss>) -> Result<Self, DocxError> {
        let path = package.related_part(&format!("{}styles", package::REL), "word/styles.xml")?;
        let mut result = Self::default();
        let Some(root) = package.xml(&path)? else {
            return Ok(result);
        };
        result.defaults.apply(
            root.child("w:docDefaults")
                .and_then(|p| p.child("w:rPrDefault"))
                .and_then(|p| p.child("w:rPr")),
        );
        for style in root
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|e| e.name == "w:style")
        {
            let Some(id) = style.get("w:styleId") else {
                continue;
            };
            if result.styles.len() >= 4096 {
                return Err(DocxError::new("Word语言样式超过4096项预算"));
            }
            if result.styles.insert(id.into(), style.clone()).is_some() {
                result.duplicates.insert(id.into());
                loss(
                    losses,
                    "word-language-style",
                    format!("语言样式 {id} 标识重复；不能猜测继承，准确原件保留"),
                );
            }
            if matches!(style.get("w:default"), Some("1" | "true" | "on")) {
                let slot = match style.get("w:type") {
                    Some("paragraph") => &mut result.paragraph_default,
                    Some("character") => &mut result.character_default,
                    _ => continue,
                };
                if slot.is_some() {
                    loss(
                        losses,
                        "word-language-style",
                        "Word有多个默认样式；默认语言继承不唯一，准确原件保留",
                    );
                    *slot = Some(String::new());
                } else {
                    *slot = Some(id.into());
                }
            }
        }
        Ok(result)
    }
    fn apply(
        &self,
        id: Option<&str>,
        kind: &str,
        language: &mut WordLanguage,
        losses: &mut Vec<NativeLoss>,
    ) {
        let Some(mut current) = id else { return };
        let mut chain = vec![];
        let mut visited = BTreeSet::new();
        loop {
            if chain.len() >= 32 || !visited.insert(current) || self.duplicates.contains(current) {
                loss(
                    losses,
                    "word-language-style",
                    format!("语言样式 {current} 的继承循环／重复／过深；未猜测该链，准确原件保留"),
                );
                return;
            }
            let Some(style) = self
                .styles
                .get(current)
                .filter(|s| s.get("w:type") == Some(kind))
            else {
                loss(
                    losses,
                    "word-language-style",
                    format!("语言样式 {current} 缺失或类型不匹配；未猜测该链，准确原件保留"),
                );
                return;
            };
            chain.push(style);
            let Some(parent) = style.child("w:basedOn").and_then(|e| e.get("w:val")) else {
                break;
            };
            current = parent;
        }
        for style in chain.iter().rev() {
            language.apply(style.child("w:rPr"));
        }
    }
}
struct Revision {
    executable: bool,
    empty_body_candidate: bool,
    element: Element,
    start: usize,
    end: usize,
    point: usize,
    replacement: String,
}
fn plain_text_revision(e: &Element) -> bool {
    if e.name == "w:rPr" {
        return true;
    }
    matches!(
        e.name.as_str(),
        "w:ins"
            | "w:del"
            | "w:r"
            | "w:t"
            | "w:delText"
            | "w:tab"
            | "w:commentRangeStart"
            | "w:commentRangeEnd"
            | "w:commentReference"
    ) && e
        .children
        .iter()
        .filter_map(Xml::element)
        .all(plain_text_revision)
}
struct Builder<'a> {
    package: &'a Package,
    rels: Element,
    part: String,
    language_styles: LanguageStyles,
    paragraph_language: WordLanguage,
    run_language: Option<WordLanguage>,
    notes: BTreeMap<(String, String), notes::ImportedNote>,
    used_notes: BTreeSet<(String, String)>,
    note_bookmarks: BTreeMap<String, (String, String)>,
    caption_refs: BTreeMap<String, String>,
    mapping_barriers: BTreeSet<usize>,
    unmapped_ranges: Vec<(usize, usize)>,
    precise_starts: BTreeMap<String, usize>,
    precise_ends: BTreeMap<String, usize>,
    equations: Vec<(usize, usize, usize, usize)>,
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
    fn repeat_note(&mut self, kind: &str, id: &str) -> Result<(), DocxError> {
        self.mapping_gap();
        let Some(id) = notes::canonical_note_id(id) else {
            loss(
                &mut self.losses,
                "word-note-id",
                "重复注释引用没有有效ID；原件保留",
            );
            return Ok(());
        };
        if !self.notes.contains_key(&(kind.into(), id.clone())) {
            loss(
                &mut self.losses,
                "word-note-body-missing",
                "重复注释引用缺少唯一真实正文；正文未进入新文档，原件保留",
            );
            return Ok(());
        }
        let label = format!("weftext-word-{kind}-{id}");
        self.source.push_str(&if kind == "footnote" {
            format!("#ref(<{label}>)")
        } else {
            format!("#endnote-ref({})", quoted(&label))
        });
        Ok(())
    }
    fn mapping_gap(&mut self) {
        self.mapping_barriers.insert(self.positions.len());
        for id in self.starts.keys().chain(self.precise_starts.keys()) {
            if !self.ends.contains_key(id) && !self.precise_ends.contains_key(id) {
                self.invalid_ranges.insert(id.clone());
            }
        }
    }
    fn note(&mut self, kind: &str, id: &str) -> Result<(), DocxError> {
        let Some(id) = notes::canonical_note_id(id) else {
            self.mapping_gap();
            loss(
                &mut self.losses,
                "word-note-id",
                "正文注释标记没有有效正整数ID；注释正文未转换，准确原件保留",
            );
            return Ok(());
        };
        let key = (kind.to_owned(), id.to_owned());
        let label = format!("weftext-word-{kind}-{id}");
        self.mapping_gap();
        if self.used_notes.contains(&key) {
            self.source.push_str(&if kind == "footnote" {
                format!("#ref(<{label}>)")
            } else {
                format!("#endnote-ref({})", quoted(&label))
            });
            return Ok(());
        }
        let Some(note) = self.notes.get(&key).cloned() else {
            loss(
                &mut self.losses,
                "word-note-body-missing",
                format!("{kind} ID {id} 缺失或不唯一；注释正文未进入新文档，原件保留"),
            );
            return Ok(());
        };
        self.used_notes.insert(key);
        let mut child = Builder {
            package: self.package,
            rels: note.rels,
            part: note.part.clone(),
            language_styles: self.language_styles.clone(),
            paragraph_language: self.language_styles.defaults.clone(),
            run_language: None,
            notes: BTreeMap::new(),
            used_notes: BTreeSet::new(),
            note_bookmarks: BTreeMap::new(),
            caption_refs: BTreeMap::new(),
            mapping_barriers: BTreeSet::new(),
            unmapped_ranges: vec![],
            precise_starts: BTreeMap::new(),
            precise_ends: BTreeMap::new(),
            equations: vec![],
            source: String::new(),
            visible: String::new(),
            positions: vec![],
            starts: BTreeMap::new(),
            ends: BTreeMap::new(),
            invalid_ranges: BTreeSet::new(),
            revision_depth: 0,
            revisions: vec![],
            assets: std::mem::take(&mut self.assets),
            losses: vec![],
        };
        child.blocks(&note.body)?;
        if !child.starts.is_empty() || !child.ends.is_empty() || !child.revisions.is_empty() {
            loss(
                &mut child.losses,
                "word-note-review-unmapped",
                "注释部件中的批注／修订没有跨部件准确映射；保原件与讨论，未挂回主文档同文；待插入未接受、待删除保基线",
            );
        }
        for item in &mut child.losses {
            item.path = note.part.clone();
        }
        self.losses.extend(child.losses);
        self.assets = child.assets;
        if kind == "footnote" {
            self.source
                .push_str(&format!("#footnote[{}] <{label}>", child.source.trim()));
        } else {
            self.source.push_str(&format!(
                "#endnote(id: {})[{}]",
                quoted(&label),
                child.source.trim()
            ));
        }
        Ok(())
    }
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
    fn language_literal(&mut self, s: &str) -> Result<(), DocxError> {
        let Some(language) = self.run_language.clone() else {
            return self.literal(s);
        };
        for (content, wrapper) in language.segments(s, &mut self.losses) {
            if let Some(wrapper) = &wrapper {
                self.source.push_str(wrapper);
            }
            self.literal(&content)?;
            if wrapper.is_some() {
                self.source.push(']');
            }
        }
        Ok(())
    }
    fn language_for_run(&mut self, run: &Element) -> WordLanguage {
        let pr = run.child("w:rPr");
        let mut language = self.paragraph_language.clone();
        let style = pr
            .and_then(|p| p.child("w:rStyle"))
            .and_then(|s| s.get("w:val"))
            .or(self.language_styles.character_default.as_deref());
        self.language_styles
            .apply(style, "character", &mut language, &mut self.losses);
        language.apply(pr);
        language
    }
    fn split_language_revision(&mut self, revision: &Element) -> bool {
        let mut runs = vec![];
        revision.descendants("w:r", &mut runs);
        let mut wrappers = BTreeSet::new();
        for run in runs {
            let language = self.language_for_run(run);
            for (_, wrapper) in language.segments(&text(run), &mut self.losses) {
                wrappers.insert(wrapper);
            }
        }
        wrappers.len() > 1
    }
    /// The native wrappers actually emitted for a plain run/revision. Source
    /// insertion points may move into an adjacent wrapper only when this exact
    /// context agrees; an XML text boundary is not itself a writable source gap.
    fn plain_revision_context(
        &mut self,
        element: &Element,
    ) -> Option<(Option<String>, bool, bool)> {
        let runs: Vec<_> = match element.name.as_str() {
            "w:r" => vec![element],
            "w:ins" | "w:del" => {
                if element
                    .children
                    .iter()
                    .filter_map(Xml::element)
                    .any(|child| {
                        !matches!(
                            child.name.as_str(),
                            "w:r" | "w:commentRangeStart" | "w:commentRangeEnd"
                        )
                    })
                {
                    return None;
                }
                element
                    .children
                    .iter()
                    .filter_map(Xml::element)
                    .filter(|child| child.name == "w:r")
                    .collect()
            }
            _ => return None,
        };
        let mut contexts = BTreeSet::new();
        for run in runs {
            if run.children.iter().filter_map(Xml::element).any(|child| {
                !matches!(
                    child.name.as_str(),
                    "w:rPr" | "w:t" | "w:delText" | "w:tab" | "w:commentReference"
                )
            }) {
                return None;
            }
            if text(run).is_empty() {
                continue;
            }
            let pr = run.child("w:rPr");
            if pr.is_some_and(|pr| {
                pr.child("w:rStyle").is_some() || pr.child("w:rPrChange").is_some()
            }) {
                return None;
            }
            let on = |name: &str| {
                pr.and_then(|pr| pr.child(name))
                    .is_some_and(|v| !matches!(v.get("w:val"), Some("0" | "false" | "off")))
            };
            let losses_before = self.losses.len();
            let language = self.language_for_run(run);
            if self.losses[losses_before..]
                .iter()
                .any(|loss| loss.code == "word-language-style")
            {
                return None;
            }
            // No declared language is a known absence of native wrappers, not
            // a failed resolution of a declared language. Preserve its plain
            // source boundary without inventing any language or region value.
            if language.values.is_empty() && language.unsupported.is_empty() {
                contexts.insert((None, on("w:b"), on("w:i")));
                continue;
            }
            let mut ignored = vec![];
            let segments = language.segments(&text(run), &mut ignored);
            if ignored
                .iter()
                .any(|loss| loss.code != "word-language-specialized")
            {
                return None;
            }
            for (_, wrapper) in segments {
                // A declared but unused/missing script slot is not the known
                // absence certified above, even if both render no wrapper.
                let wrapper = wrapper?;
                contexts.insert((Some(wrapper), on("w:b"), on("w:i")));
            }
        }
        (contexts.len() == 1).then(|| contexts.into_iter().next().unwrap())
    }
    fn paragraph(&mut self, p: &Element, in_cell: bool) -> Result<(), DocxError> {
        let previous_language = self.paragraph_language.clone();
        self.paragraph_language = self.language_styles.defaults.clone();
        let language_losses_before = self.losses.len();
        let paragraph_style = p
            .child("w:pPr")
            .and_then(|pr| pr.child("w:pStyle"))
            .and_then(|s| s.get("w:val"))
            .or(self.language_styles.paragraph_default.as_deref());
        self.language_styles.apply(
            paragraph_style,
            "paragraph",
            &mut self.paragraph_language,
            &mut self.losses,
        );
        let paragraph_language_known = !self.losses[language_losses_before..]
            .iter()
            .any(|loss| loss.code == "word-language-style");
        if !in_cell {
            self.source.push_str("\n\n");
        }
        let heading = self
            .language_styles
            .heading_level(p, paragraph_style, &mut self.losses);
        if let Some(level) = heading
            && !in_cell
        {
            self.source.push_str(&"=".repeat(level));
            self.source.push(' ');
        }
        if let Some(pr) = p.child("w:pPr") {
            if heading.is_some() && !in_cell {
                if pr
                    .child("w:numPr")
                    .and_then(|p| p.child("w:numId"))
                    .and_then(|e| e.get("w:val"))
                    .is_some_and(|id| id != "0")
                {
                    loss(
                        &mut self.losses,
                        "word-heading-numbering",
                        "原生标题层级与正文保留；Word标题编号方案／起始／排除规则未完整反向映射，准确原件保留",
                    );
                }
            } else if pr.child("w:numPr").is_some_and(|numbering| {
                numbering.child("w:numId").and_then(|id| id.get("w:val")) != Some("0")
            }) && !in_cell
            {
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
        let children: Vec<_> = p
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|c| c.name != "w:pPr")
            .collect();
        let mut emitted = vec![];
        for c in &children {
            let start = self.positions.len();
            let revision_start = self.revisions.len();
            self.inline(c)?;
            emitted.push((
                start,
                self.positions.len(),
                revision_start,
                self.revisions.len(),
            ));
        }
        for (index, child) in children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.name == "w:ins")
        {
            let (_, _, first, last) = emitted[index];
            if last != first + 1 || !self.revisions[first].executable {
                continue;
            }
            let context = paragraph_language_known
                .then(|| self.plain_revision_context(child))
                .flatten();
            let visible_point = self.revisions[first].start;
            let mut mapped = None;
            // Prefer the following actual sibling, then the previous one. No
            // traversal through hyperlinks, equations, fields, or other syntax.
            for neighbour in [
                index.checked_add(1).filter(|n| *n < children.len()),
                index.checked_sub(1),
            ]
            .into_iter()
            .flatten()
            {
                let (start, end, _, _) = emitted[neighbour];
                if start == end
                    || context.is_none()
                    || self.plain_revision_context(children[neighbour]) != context
                {
                    continue;
                }
                if neighbour > index && start == visible_point {
                    mapped = self.positions.get(start).map(|position| position.0);
                } else if neighbour < index && end == visible_point {
                    mapped = self.positions.get(end - 1).map(|position| position.1);
                }
                if mapped.is_some() {
                    break;
                }
            }
            if let Some(point) = mapped {
                self.revisions[first].point = point;
            } else {
                self.revisions[first].executable = false;
                // Defer only a single unformatted, unwrapped insertion in a
                // plain paragraph. The complete document must independently
                // prove its exact empty trailing body slot below.
                self.revisions[first].empty_body_candidate = !in_cell
                    && self.part == "word/document.xml"
                    && p.child("w:pPr").is_none()
                    && children.len() == 1
                    && context == Some((None, false, false));
                if !self.revisions[first].empty_body_candidate {
                    loss(
                        &mut self.losses,
                        "word-insertion-boundary",
                        "待审插入点位于原生包装语法之间，且紧邻文字的语言／格式或结构不能证明等价；建议仅保留于原件并需重新确认",
                    );
                }
            }
        }
        self.paragraph_language = previous_language;
        Ok(())
    }
    fn inline(&mut self, e: &Element) -> Result<(), DocxError> {
        match e.name.as_str() {
            "w:commentRangeStart" => {
                if let Some(id) = e.get("w:id")
                    && (self.precise_starts.contains_key(id)
                        || self
                            .starts
                            .insert(id.into(), self.positions.len())
                            .is_some())
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
                    && (self.precise_ends.contains_key(id)
                        || self.ends.insert(id.into(), self.positions.len()).is_some())
                {
                    self.invalid_ranges.insert(id.into());
                }
            }
            "w:commentReference" => {}
            "w:footnoteRef" | "w:endnoteRef" => {}
            "w:footnoteReference" | "w:endnoteReference" => {
                if e.get("w:customMarkFollows").is_some() {
                    loss(
                        &mut self.losses,
                        "word-note-custom-mark",
                        "自定义注释标记未映射；采用独立连续编号，准确原件保留",
                    );
                }
                if let Some(id) = e.get("w:id") {
                    self.note(
                        if e.name == "w:footnoteReference" {
                            "footnote"
                        } else {
                            "endnote"
                        },
                        id,
                    )?;
                }
            }
            "w:fldSimple" => {
                let visible_start = self.positions.len();
                let instruction = e.get("w:instr").unwrap_or("");
                let tokens: Vec<_> = instruction.split_whitespace().collect();
                if tokens
                    .first()
                    .is_some_and(|s| s.eq_ignore_ascii_case("NOTEREF"))
                    && notes::reference_switches(
                        tokens.get(2..).unwrap_or_default(),
                        &["\\h", "\\f"],
                    )
                {
                    if let Some(key) = tokens
                        .get(1)
                        .and_then(|name| self.note_bookmarks.get(*name))
                        .cloned()
                    {
                        self.repeat_note(&key.0, &key.1)?;
                    } else {
                        self.mapping_gap();
                        loss(
                            &mut self.losses,
                            "word-note-reference-unmapped",
                            "NOTEREF 域没有唯一真实注释标记书签；不凭缓存数字猜归属，原件保留",
                        );
                        for c in e.children.iter().filter_map(Xml::element) {
                            self.inline(c)?;
                        }
                        self.unmapped_ranges
                            .push((visible_start, self.positions.len()));
                    }
                } else if tokens
                    .first()
                    .is_some_and(|s| s.eq_ignore_ascii_case("REF"))
                    && notes::reference_switches(tokens.get(2..).unwrap_or_default(), &["\\h"])
                    && let Some(label) = tokens.get(1).and_then(|name| self.caption_refs.get(*name))
                {
                    self.source.push_str(&format!("#ref(<{label}>)"));
                    self.mapping_barriers.insert(self.positions.len());
                } else {
                    self.mapping_gap();
                    loss(
                        &mut self.losses,
                        if tokens
                            .first()
                            .is_some_and(|s| s.eq_ignore_ascii_case("SEQ"))
                        {
                            "word-caption-unpaired"
                        } else {
                            "word-field"
                        },
                        if tokens
                            .first()
                            .is_some_and(|s| s.eq_ignore_ascii_case("SEQ"))
                        {
                            "SEQ 题注域存在，但尚无已确认对象配对；本次仅保可见内容，自动编号与对象绑定未进入新文档，原件保留"
                        } else {
                            "域仅保可见结果，未求值活动域或承诺自动更新"
                        },
                    );
                    for c in e.children.iter().filter_map(Xml::element) {
                        self.inline(c)?;
                    }
                    self.unmapped_ranges
                        .push((visible_start, self.positions.len()));
                }
            }
            "m:oMath" | "m:oMathPara" => {
                let offset = self.source.len();
                match math::import_equation(e, offset) {
                    Ok(eq) => {
                        let visible_from = self.positions.len();
                        self.source.push_str(&eq.source);
                        self.visible.push_str(&eq.visible);
                        self.positions.extend(eq.positions);
                        for (id, at) in eq.starts {
                            if self.starts.contains_key(&id)
                                || self.precise_starts.insert(id.clone(), at).is_some()
                            {
                                self.invalid_ranges.insert(id);
                            }
                        }
                        for (id, at) in eq.ends {
                            if self.ends.contains_key(&id)
                                || self.precise_ends.insert(id.clone(), at).is_some()
                            {
                                self.invalid_ranges.insert(id);
                            }
                        }
                        self.invalid_ranges.extend(eq.invalid_ranges);
                        self.equations.push((
                            visible_from,
                            self.positions.len(),
                            offset,
                            self.source.len(),
                        ));
                    }
                    Err(reason) => {
                        self.mapping_gap();
                        loss(
                            &mut self.losses,
                            "word-math-unmapped",
                            format!(
                                "{}；原公式仅保留于 original.docx，未进入新原生正文",
                                reason.0
                            ),
                        );
                    }
                }
            }
            "w:ins" | "w:del" => {
                let start = self.positions.len();
                let point = self.source.len();
                let mut nested = vec![];
                e.descendants("w:ins", &mut nested);
                e.descendants("w:del", &mut nested);
                let textual = plain_text_revision(e);
                let language_split = textual && self.split_language_revision(e);
                let executable =
                    nested.len() == 1 && self.revision_depth == 0 && textual && !language_split;
                if language_split {
                    loss(
                        &mut self.losses,
                        "word-structural-revision",
                        "待审文字修订横跨不同语言包装；普通文字与批注逐字位置保留，但源码替换的语法／语言意图未证明安全，修订保原件并需重新确认",
                    );
                }
                if !textual {
                    loss(
                        &mut self.losses,
                        "word-structural-revision",
                        "公式／图片／注释／域等结构修订未作为可执行文字建议导入；修订前基线与准确原件保留",
                    );
                }
                if nested.len() != 1 || self.revision_depth != 0 {
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
                    empty_body_candidate: false,
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
                let language = self.language_for_run(e);
                let segments = language.segments(&text(e), &mut self.losses);
                let split = segments.len() > 1;
                let language_wrapper = (!split)
                    .then(|| segments.first().and_then(|(_, wrapper)| wrapper.clone()))
                    .flatten();
                let previous_language = self.run_language.take();
                if split {
                    self.run_language = Some(language);
                }
                if let Some(wrapper) = &language_wrapper {
                    self.source.push_str(wrapper);
                }
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
                if language_wrapper.is_some() {
                    self.source.push(']');
                }
                self.run_language = previous_language;
            }
            "w:t" | "w:delText" => self.language_literal(&e.text())?,
            "w:tab" => self.language_literal("\t")?,
            "w:br" | "w:cr" => {
                self.source.push_str("#linebreak()\n");
            }
            "w:hyperlink" => {
                let relation = e
                    .get("r:id")
                    .and_then(|id| {
                        self.rels
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
                self.mapping_gap();
                let mut blips = vec![];
                e.descendants("a:blip", &mut blips);
                for blip in blips {
                    let rel = blip.get("r:embed").and_then(|id| {
                        self.rels
                            .children
                            .iter()
                            .filter_map(Xml::element)
                            .find(|r| r.get("Id") == Some(id))
                    });
                    let target = rel
                        .filter(|r| r.get("TargetMode") != Some("External"))
                        .and_then(|r| r.get("Target"));
                    let path = target.map(|t| {
                        if t.starts_with('/') {
                            t.trim_start_matches('/').to_owned()
                        } else {
                            format!(
                                "{}/{t}",
                                self.part.rsplit_once('/').map(|v| v.0).unwrap_or("word")
                            )
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
                self.mapping_gap();
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
                "weftext:pairedFigure" => {
                    let kind = match c.get("kind") {
                        Some("table") => "table",
                        Some("equation") => "math.equation",
                        _ => "image",
                    };
                    self.mapping_barriers.insert(self.positions.len());
                    self.source.push_str(&format!("\n\n#figure(kind: {kind}, numbering: \"1\", caption: figure.caption(position: {})[",c.get("position").unwrap_or("bottom")));
                    self.source.push_str(
                        &weftext_engine::encode_literal(c.get("caption").unwrap_or(""))
                            .map_err(|e| DocxError::new(e.to_string()))?,
                    );
                    self.source.push_str("], [");
                    self.blocks(c)?;
                    self.source.push_str(&format!(
                        "]) <{}>\n",
                        c.get("label").unwrap_or("weftext-word-caption")
                    ));
                    loss(
                        &mut self.losses,
                        "word-caption-confirmed",
                        "按冻结预览中的显式配对创建原生题注及编号；所选题注文字保留，原Word题注富格式和自定义SEQ选项未完整映射",
                    );
                }
                "w:p" => self.paragraph(c, false)?,
                // Word saves structural math/comment scopes with their start
                // markers directly in the body, before the containing w:p.
                // Keep their actual XML order in the same scalar mapping.
                "w:commentRangeStart" | "w:commentRangeEnd" => self.inline(c)?,
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
                    let mut leading = true;
                    let mut header_open = false;
                    for row in rows {
                        let cells: Vec<_> = row
                            .children
                            .iter()
                            .filter_map(Xml::element)
                            .filter(|e| e.name == "w:tc")
                            .collect();
                        let header = row
                            .child("w:trPr")
                            .and_then(|pr| pr.child("w:tblHeader"))
                            .is_some_and(|value| {
                                !matches!(value.get("w:val"), Some("0" | "false" | "off"))
                            });
                        let simple = cells.len() == columns
                            && cells.iter().all(|cell| {
                                cell.child("w:tcPr").is_none_or(|pr| {
                                    pr.child("w:vMerge").is_none()
                                        && pr
                                            .child("w:gridSpan")
                                            .is_none_or(|span| span.get("w:val") == Some("1"))
                                })
                            });
                        if leading && header && simple {
                            if !header_open {
                                self.source.push_str("table.header(repeat: true,\n");
                                header_open = true;
                            }
                        } else {
                            leading = false;
                            if header_open {
                                self.source.push_str("),\n");
                                header_open = false;
                            }
                            if header {
                                loss(
                                    &mut self.losses,
                                    "word-table-header",
                                    "非首部连续普通行的Word表头重复属性未转换；内容与准确原件保留",
                                );
                            }
                        }
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
                    if header_open {
                        self.source.push_str("),\n");
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
        if self
            .unmapped_ranges
            .iter()
            .any(|(a, b)| *a < end && *b > start)
        {
            return None;
        }
        if self
            .mapping_barriers
            .range((
                std::ops::Bound::Excluded(start),
                std::ops::Bound::Excluded(end),
            ))
            .next()
            .is_some()
        {
            return None;
        }
        if let Some((_, _, from, to)) = self
            .equations
            .iter()
            .find(|(a, b, _, _)| *a == start && *b == end)
        {
            return Some((*from, *to));
        }
        Some((self.positions.get(start)?.0, self.positions.get(end - 1)?.1))
    }
}

pub fn build_docx_native_package(
    bytes: &[u8],
    schema: &str,
    operation_id: &str,
) -> crate::Result<NativePackage> {
    build(bytes, schema, operation_id, &DocxImportOptions::default()).map_err(Into::into)
}
pub fn build_docx_native_package_with_options(
    bytes: &[u8],
    schema: &str,
    operation_id: &str,
    options: &DocxImportOptions,
) -> crate::Result<NativePackage> {
    build(bytes, schema, operation_id, options).map_err(Into::into)
}
pub fn analyze_docx_import(bytes: &[u8]) -> crate::Result<DocxImportAnalysis> {
    let package = Package::load(Some(bytes)).map_err(ExportError::from)?;
    let doc = package
        .xml("word/document.xml")
        .map_err(ExportError::from)?
        .ok_or_else(|| ExportError("缺少 Word 正文".into()))?;
    Ok(notes::analyze(bytes, &notes::normalize_fields(&doc)))
}
fn build(
    bytes: &[u8],
    schema: &str,
    op: &str,
    options: &DocxImportOptions,
) -> Result<NativePackage, DocxError> {
    if !weftext_data::uuid_v4(op) {
        return Err(DocxError::new("Word 导入需要有效操作标识"));
    }
    weftext_data::Schema::from_author_json(schema).map_err(|e| DocxError::new(e.to_string()))?;
    let package = Package::load(Some(bytes))?;
    let doc = package
        .xml("word/document.xml")?
        .ok_or_else(|| DocxError::new("缺少 Word 正文"))?;
    let doc = notes::normalize_fields(&doc);
    let body = notes::paired_body(bytes, &doc, options)?;
    let workspace = id(op, "workspace");
    let node = id(op, "node");
    let revision = id(op, "revision");
    let original = id(op, "original");
    let mut language_losses = vec![];
    let language_styles = LanguageStyles::read(&package, &mut language_losses)?;
    let mut b = Builder {
        package: &package,
        rels: package.rels.clone(),
        part: "word/document.xml".into(),
        paragraph_language: language_styles.defaults.clone(),
        run_language: None,
        language_styles,
        notes: BTreeMap::new(),
        used_notes: BTreeSet::new(),
        note_bookmarks: BTreeMap::new(),
        caption_refs: BTreeMap::new(),
        mapping_barriers: BTreeSet::new(),
        unmapped_ranges: vec![],
        precise_starts: BTreeMap::new(),
        precise_ends: BTreeMap::new(),
        equations: vec![],
        source: format!("#metadata((id: {})) <weftext>\n", quoted(&node)),
        visible: String::new(),
        positions: vec![],
        starts: BTreeMap::new(),
        ends: BTreeMap::new(),
        invalid_ranges: BTreeSet::new(),
        revision_depth: 0,
        revisions: vec![],
        assets: BTreeMap::new(),
        losses: language_losses,
    };
    b.notes = notes::read(&package, &mut b.losses)?;
    b.note_bookmarks = notes::note_bookmarks(&body);
    b.caption_refs = notes::caption_bookmarks(&body);
    let has_endnotes = b.notes.keys().any(|(kind, _)| kind == "endnote");
    if has_endnotes {
        b.source
            .push_str("#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n");
    }
    b.blocks(&body)?;
    for key in b.notes.keys().filter(|key| !b.used_notes.contains(*key)) {
        loss(
            &mut b.losses,
            "word-note-unreferenced",
            format!(
                "{} ID {} 没有正文引用；注释正文未进入新文档，原件保留",
                key.0, key.1
            ),
        );
    }
    if has_endnotes {
        b.source.push_str("\n\n#endnotes()\n");
    }
    let editor = weftext_engine::inspect_editor(&b.source);
    if !editor.syntax_valid {
        return Err(DocxError::new(
            "转换结果未通过原生语法校验，保留上传原件且不创建笔记",
        ));
    }
    if b.revisions.iter().any(|r| r.empty_body_candidate) {
        let blocks: Vec<_> = body
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|child| child.name != "w:sectPr")
            .collect();
        let metadata = weftext_engine::inspect_metadata(&b.source);
        let empty_body_point = (b.revisions.len() == 1
            && matches!(blocks.as_slice(), [paragraph] if paragraph.name == "w:p")
            && b.positions.is_empty()
            && b.mapping_barriers.is_empty()
            && metadata.id.as_deref() == Some(node.as_str())
            && metadata.metadata.as_ref().is_some_and(|declaration| {
                b.source[declaration.to..].chars().all(char::is_whitespace)
            })
            && b.source.ends_with("\n\n"))
        .then_some(editor.body_insertion)
        .flatten()
        .filter(|point| point.writable && point.from == b.source.len() && point.to == point.from);
        for revision in b.revisions.iter_mut().filter(|r| r.empty_body_candidate) {
            if let Some(point) = &empty_body_point {
                revision.point = point.from;
                revision.executable = true;
            } else {
                loss(
                    &mut b.losses,
                    "word-insertion-boundary",
                    "待审插入没有紧邻的等价文字，且完整文档不能独立证明唯一空正文插入点；建议保留于原件并需重新确认",
                );
            }
        }
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
    let mut review_content_losses = vec![];
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
            let range = if b.invalid_ranges.contains(key) {
                None
            } else {
                let mixed = b.precise_starts.contains_key(key) || b.precise_ends.contains_key(key);
                let from = b.precise_starts.get(key).copied().or_else(|| {
                    b.starts.get(key).and_then(|a| {
                        // A body-level start paired with an end inside the
                        // first math construct starts at that construct's
                        // syntax, not merely its first argument glyph.
                        if mixed {
                            b.equations.iter().find(|(start, _, _, _)| start == a).map(
                                |(_, _, from, to)| {
                                    let inner = &b.source[from + 1..to - 1];
                                    from + 1 + inner.len() - inner.trim_start().len()
                                },
                            )
                        } else {
                            None
                        }
                        .or_else(|| b.positions.get(*a).map(|p| p.0))
                    })
                });
                let to = b.precise_ends.get(key).copied().or_else(|| {
                    b.ends.get(key).and_then(|z| {
                        if mixed
                            && let Some((_, _, from, to)) =
                                b.equations.iter().find(|(_, end, _, _)| end == z)
                        {
                            return Some(from + 1 + b.source[from + 1..to - 1].trim_end().len());
                        }
                        z.checked_sub(1)
                            .and_then(|z| b.positions.get(z).map(|p| p.1))
                    })
                });
                if mixed {
                    from.zip(to).filter(|(a, z)| {
                        if a >= z {
                            return false;
                        }
                        let first = b.positions.iter().position(|(_, end)| end > a);
                        let last = b.positions.iter().rposition(|(start, _)| start < z);
                        first.zip(last).is_some_and(|(first, last)| {
                            first <= last
                                && !b
                                    .unmapped_ranges
                                    .iter()
                                    .any(|(a, z)| *a <= last && *z > first)
                                && b.mapping_barriers
                                    .range((
                                        std::ops::Bound::Excluded(first),
                                        std::ops::Bound::Included(last),
                                    ))
                                    .next()
                                    .is_none()
                        })
                    })
                } else {
                    b.starts
                        .get(key)
                        .zip(b.ends.get(key))
                        .and_then(|(a, z)| b.range(*a, *z))
                }
            };
            let content = c
                .children
                .iter()
                .filter_map(Xml::element)
                .filter(|e| e.name == "w:p")
                .map(text)
                .collect::<Vec<_>>()
                .join("\n");
            let mut rich = false;
            for name in [
                "w:rPr",
                "w:pPr",
                "w:drawing",
                "w:tbl",
                "w:hyperlink",
                "m:oMath",
                "w:fldSimple",
            ] {
                let mut items = vec![];
                c.descendants(name, &mut items);
                rich |= items.iter().any(|item| !item.children.is_empty());
            }
            if rich {
                review_content_losses.push(NativeLoss{path:comments_path.clone(),code:"word-comment-content-loss".into(),detail:format!("批注 {key} 的可读取段落文字保留；其富格式／列表／链接／图片／公式等内容没有全部进入批注正文，准确原件保留")});
            }
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
    losses.extend(review_content_losses);
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
    if has_endnotes {
        let name = "weftext-endnotes.typ";
        let resource = id(op, "endnotes-module");
        resources.push(json!({"resourceId":resource,"path":name}));
        insert(
            format!("document/{name}"),
            include_bytes!("../../assets/weftext-endnotes.typ").to_vec(),
            Role::Resource,
            Some(resource),
            Some(id(op, "endnotes-module-revision")),
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
    fn explicit_zero_numbering_is_an_ordinary_paragraph_in_real_office_shape() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml", &xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="0"/></w:numPr></w:pPr><w:r><w:t>REAL_HOSTED_DOC_BODY中文</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="3"/></w:numPr></w:pPr><w:r><w:t>ActualList</w:t></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert_eq!(
            source.lines().filter(|line| line.starts_with("- ")).count(),
            1,
            "{source}"
        );
        assert!(
            source
                .lines()
                .any(|line| line.starts_with("- ") && line.contains("ActualList")),
            "{source}"
        );
        assert!(
            !source
                .lines()
                .any(|line| line.starts_with("- ") && line.contains("REAL")),
            "{source}"
        );
    }
    #[test]
    fn word_saved_complex_noteref_preserves_repeats_and_rejects_semantic_switches() {
        for (switches, repeated) in [
            (r"\h \* MERGEFORMAT", true),
            (r"\f \* mergeformat", true),
            (r"\p \* MERGEFORMAT", false),
            (r"\*", false),
        ] {
            let mut p = Package::load(None).unwrap();
            p.set_xml("word/document.xml", &xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:bookmarkStart w:id="0" w:name="ncFootA"/><w:r><w:footnoteReference w:id="1"/></w:r><w:bookmarkEnd w:id="0"/><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> NOTEREF ncFootA {switches} </w:instrText></w:r><w:r><w:rPr/></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
            p.set_xml("word/footnotes.xml", &xml::parse(format!(r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="1"><w:p><w:r><w:t>独立正文</w:t></w:r></w:p></w:footnote></w:footnotes>"#).as_bytes()).unwrap());
            p.ensure_rel("footnotes", "footnotes.xml");
            let imported = proposal(p);
            let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
            assert_eq!(
                source.contains("#ref(<weftext-word-footnote-1>)"),
                repeated,
                "{switches}: {source}"
            );
            assert_eq!(
                source
                    .matches("#footnote[#text(lang: \"zh\", region: \"CN\")[独立正文]]")
                    .count(),
                1
            );
        }
    }
    #[test]
    fn forward_noteref_does_not_move_the_author_definition_or_renumber_independent_notes() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:fldSimple w:instr=" NOTEREF targetB "><w:r><w:t>2</w:t></w:r></w:fldSimple><w:r><w:footnoteReference w:id="1"/></w:r><w:bookmarkStart w:id="8" w:name="targetB"/><w:r><w:footnoteReference w:id="2"/></w:r><w:bookmarkEnd w:id="8"/></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/footnotes.xml",&xml::parse(format!(r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="1"><w:p><w:r><w:t>A</w:t></w:r></w:p></w:footnote><w:footnote w:id="2"><w:p><w:r><w:t>B</w:t></w:r></w:p></w:footnote></w:footnotes>"#).as_bytes()).unwrap());
        p.ensure_rel("footnotes", "footnotes.xml");
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert!(
            source.find("#ref(<weftext-word-footnote-2>)").unwrap()
                < source
                    .find("#footnote[#text(lang: \"zh\", region: \"CN\")[A]]")
                    .unwrap()
        );
        assert!(
            source
                .find("#footnote[#text(lang: \"zh\", region: \"CN\")[A]]")
                .unwrap()
                < source
                    .find("#footnote[#text(lang: \"zh\", region: \"CN\")[B]]")
                    .unwrap()
        );
        let dir = tempfile::TempDir::new().unwrap();
        let out = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &dir.path().join("note.typ"), dir.path())
            .unwrap();
        assert!(out.ok, "{source}\n{:?}", out.diagnostics);
    }
    #[test]
    fn note_ids_follow_ooxml_numeric_identity_without_merging_different_notes() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:r><w:footnoteReference w:id="0007"/></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/footnotes.xml",&xml::parse(format!(r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="+7"><w:p><w:r><w:t>正文</w:t></w:r></w:p></w:footnote></w:footnotes>"#).as_bytes()).unwrap());
        p.ensure_rel("footnotes", "footnotes.xml");
        let imported = proposal(p);
        assert!(
            std::str::from_utf8(&imported.files["document/document.typ"])
                .unwrap()
                .contains(
                    "#footnote[#text(lang: \"zh\", region: \"CN\")[正文]] <weftext-word-footnote-7>"
                )
        );
        let mut p = Package::load(Some(&imported.files["document/original.docx"])).unwrap();
        let mut root = p.xml("word/footnotes.xml").unwrap().unwrap();
        root.push(Element::new("w:footnote").attr("w:id", "7"));
        p.set_xml("word/footnotes.xml", &root);
        let imported = proposal(p);
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-note-duplicate-id")
        );
        assert!(
            !std::str::from_utf8(&imported.files["document/document.typ"])
                .unwrap()
                .contains("#footnote[")
        );
    }
    #[test]
    fn comments_on_unsupported_dynamic_field_caches_stay_on_the_original() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:commentRangeStart w:id="0"/><w:fldSimple w:instr=" DATE "><w:r><w:t>相同</w:t></w:r></w:fldSimple><w:commentRangeEnd w:id="0"/><w:r><w:t>相同</w:t></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/comments.xml",&xml::parse(format!(r#"<w:comments xmlns:w="{W}"><w:comment w:id="0"><w:p><w:r><w:t>检查生成值</w:t></w:r></w:p></w:comment></w:comments>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let annotations: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        assert_eq!(
            annotations["annotations"][0]["target"]["kind"],
            "wholeResource"
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
    fn unmapped_endpoint_objects_and_structural_revisions_never_acquire_text_write_authority() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><w:commentRangeStart w:id="0"/><w:r><w:t>正文</w:t></w:r><w:r><w:drawing/></w:r><w:commentRangeEnd w:id="0"/><w:del w:id="8"><w:r><w:delText>待删</w:delText></w:r><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:del></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/comments.xml",&xml::parse(format!(r#"<w:comments xmlns:w="{W}"><w:comment w:id="0"><w:p><w:r><w:t>包含末尾图片</w:t></w:r></w:p></w:comment></w:comments>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let annotations: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let records = annotations["annotations"].as_array().unwrap();
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .all(|r| r["target"]["kind"] == "wholeResource")
        );
        assert_eq!(records[1]["suggestion"]["requiresReconfirmation"], true);
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-structural-revision")
        );
    }
    #[test]
    fn extra_originals_are_exact_transactional_resources_and_language_is_local() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:r><w:rPr><w:lang w:val="en-GB" w:eastAsia="zh-CN"/></w:rPr><w:t>Colour</w:t></w:r><w:r><w:t>中文</w:t></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        let mut imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert!(source.contains(
            "#text(lang: \"en\", region: \"GB\")[Colour]#text(lang: \"zh\", region: \"CN\")[中文]"
        ));
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-language-specialized")
        );
        let original = OriginalImportAsset {
            filename: "原稿.doc".into(),
            mime: "application/msword".into(),
            bytes: vec![1, 2, 3, 4],
        };
        retain_docx_import_originals(
            &mut imported,
            "40fa8a18-d063-435a-a962-5be0b218d453",
            &[original],
            &[],
        )
        .unwrap();
        assert_eq!(
            imported.files["document/original-input-1.doc"],
            vec![1, 2, 3, 4]
        );
        let prior = imported.files.clone();
        let invalid = OriginalImportAsset {
            filename: "../秘密.doc".into(),
            mime: "application/msword".into(),
            bytes: vec![1],
        };
        assert!(
            retain_docx_import_originals(
                &mut imported,
                "40fa8a18-d063-435a-a962-5be0b218d453",
                &[invalid],
                &[]
            )
            .is_err()
        );
        assert_eq!(imported.files, prior);
    }
    fn proposal(p: Package) -> NativePackage {
        build_docx_native_package(
            &p.finish().unwrap(),
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap()
    }
    #[test]
    fn language_resolves_document_paragraph_character_styles_and_explicit_unknown_region() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/styles.xml", &xml::parse(format!(r#"<w:styles xmlns:w="{W}">
          <w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-GB" w:eastAsia="zh-CN" w:bidi="ar-SA"/></w:rPr></w:rPrDefault></w:docDefaults>
          <w:style w:type="paragraph" w:styleId="Base"><w:rPr><w:lang w:val="en-US"/></w:rPr></w:style>
          <w:style w:type="paragraph" w:styleId="Normal" w:default="1"><w:basedOn w:val="Base"/></w:style>
          <w:style w:type="paragraph" w:styleId="British"><w:basedOn w:val="Normal"/><w:rPr><w:lang w:val="en-GB"/></w:rPr></w:style>
          <w:style w:type="character" w:styleId="German"><w:rPr><w:lang w:val="de"/></w:rPr></w:style>
          <w:style w:type="character" w:styleId="GermanChild"><w:basedOn w:val="German"/><w:rPr><w:lang w:eastAsia="ja-JP"/></w:rPr></w:style>
          <w:style w:type="paragraph" w:styleId="CycleA"><w:basedOn w:val="CycleB"/><w:rPr><w:lang w:val="fr-FR"/></w:rPr></w:style>
          <w:style w:type="paragraph" w:styleId="CycleB"><w:basedOn w:val="CycleA"/></w:style>
        </w:styles>"#).as_bytes()).unwrap());
        p.set_xml("word/document.xml", &xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body>
          <w:p><w:r><w:t>DefaultAmerican</w:t></w:r></w:p>
          <w:p><w:pPr><w:pStyle w:val="British"/></w:pPr><w:r><w:t>InheritedBritish</w:t></w:r><w:r><w:rPr><w:rStyle w:val="GermanChild"/></w:rPr><w:t>GermanUnknownRegion</w:t></w:r><w:r><w:rPr><w:lang w:val="en"/></w:rPr><w:t>EnglishUnknownRegion</w:t></w:r></w:p>
          <w:p><w:pPr><w:pStyle w:val="CycleA"/></w:pPr><w:r><w:t>CycleFallback</w:t></w:r></w:p>
        </w:body></w:document>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        for expected in [
            "#text(lang: \"en\", region: \"US\")[DefaultAmerican]",
            "#text(lang: \"en\", region: \"GB\")[InheritedBritish]",
            "#text(lang: \"de\", region: none)[GermanUnknownRegion]",
            "#text(lang: \"en\", region: none)[EnglishUnknownRegion]",
            "#text(lang: \"en\", region: \"GB\")[CycleFallback]",
        ] {
            assert!(source.contains(expected), "{source}\nmissing {expected}");
        }
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-language-style")
        );
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-language-specialized" && l.detail.contains("ja-JP"))
        );
        let dir = tempfile::TempDir::new().unwrap();
        let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
        let output = engine
            .render(
                &format!("#set text(lang: \"zh\", region: \"CN\")\n{source}"),
                &dir.path().join("language.typ"),
                dir.path(),
            )
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
    }
    #[test]
    fn language_consumes_inherited_script_slots_in_mixed_runs_with_exact_comment_offsets() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/styles.xml", &xml::parse(format!(r#"<w:styles xmlns:w="{W}">
          <w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-US" w:eastAsia="zh-CN" w:bidi="he-IL"/></w:rPr></w:rPrDefault></w:docDefaults>
          <w:style w:type="paragraph" w:styleId="Normal" w:default="1"><w:rPr><w:lang w:val="en-GB"/></w:rPr></w:style>
        </w:styles>"#).as_bytes()).unwrap());
        p.set_xml("word/document.xml", &xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body>
          <w:p><w:r><w:rPr><w:rFonts w:hint="eastAsia"/></w:rPr><w:t>A#[]😀漢かな한ㄅאבְعَرَبِيّ</w:t></w:r><w:r><w:t>𠮷漢</w:t></w:r><w:commentRangeStart w:id="0"/><w:r><w:t>𠮷漢</w:t></w:r><w:commentRangeEnd w:id="0"/></w:p>
          <w:p><w:r><w:rPr><w:lang w:val="en" w:eastAsia="ja" w:bidi="ar"/></w:rPr><w:t>z漢ع</w:t></w:r></w:p>
        </w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/comments.xml", &xml::parse(format!(r#"<w:comments xmlns:w="{W}"><w:comment w:id="0"><w:p><w:r><w:t>第二处补充汉字</w:t></w:r></w:p></w:comment></w:comments>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        let encoded = weftext_engine::encode_literal("A#[]😀").unwrap();
        for expected in [
            format!("#text(lang: \"en\", region: \"GB\")[{encoded}]"),
            "#text(lang: \"zh\", region: \"CN\")[漢かな한ㄅ]".into(),
            "#text(lang: \"he\", region: \"IL\")[אבְعَرَبِيّ]".into(),
            "#text(lang: \"en\", region: none)[z]#text(lang: \"ja\", region: none)[漢]#text(lang: \"ar\", region: none)[ع]".into(),
        ] {
            assert!(source.contains(&expected), "{source}\nmissing {expected}");
        }
        let annotations: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let range = &annotations["annotations"][0]["target"]["anchor"]["sourceRange"];
        let from = range["from"].as_u64().unwrap() as usize;
        let to = range["to"].as_u64().unwrap() as usize;
        assert_eq!(range["raw"], "𠮷漢");
        assert_eq!(from, source.rfind("𠮷漢").unwrap());
        assert_eq!(&source[from..to], "𠮷漢");
        assert_eq!(range["utf16From"], source[..from].encode_utf16().count());
        assert_eq!(range["utf16To"], source[..to].encode_utf16().count());
        assert!(!imported.manifest.losses.iter().any(|l| matches!(
            l.code.as_str(),
            "word-language-script" | "word-language-missing"
        )));
        let dir = tempfile::TempDir::new().unwrap();
        let engine = weftext_engine::Engine::new_with_system_fonts(false).unwrap();
        let output = engine
            .render(source, &dir.path().join("mixed-languages.typ"), dir.path())
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
    }
    #[test]
    fn language_neutrals_unknown_scripts_and_properties_are_not_guessed_from_neighbours() {
        let language = WordLanguage {
            values: BTreeMap::from([
                ("w:val".into(), "en-GB".into()),
                ("w:eastAsia".into(), "zh-CN".into()),
                ("w:bidi".into(), "he-IL".into()),
            ]),
            unsupported: BTreeSet::from(["w:future=unknown".into()]),
        };
        let mut losses = vec![];
        let segments = language.segments("A(漢（漢） A\u{0301} קְ क", &mut losses);
        let source = segments
            .iter()
            .map(|(content, wrapper)| match wrapper {
                Some(wrapper) => format!("{wrapper}{content}]"),
                None => content.clone(),
            })
            .collect::<String>();
        assert!(source.contains(
            "#text(lang: \"en\", region: \"GB\")[A(]#text(lang: \"zh\", region: \"CN\")[漢（漢）]"
        ));
        assert!(source.contains("#text(lang: \"en\", region: \"GB\")[ A\u{0301} ]"));
        assert!(source.contains("#text(lang: \"he\", region: \"IL\")[קְ]"));
        assert!(source.ends_with("]क"), "{source}");
        assert!(
            losses
                .iter()
                .any(|l| l.code == "word-language-script" && l.detail.contains("U+0915"))
        );
        assert!(losses.iter().any(|l| l.code == "word-language-property"));
        let mut losses = vec![];
        let neutral = language.segments(" 123😀", &mut losses);
        assert_eq!(neutral.len(), 1);
        assert_eq!(
            neutral[0].1.as_deref(),
            Some("#text(lang: \"en\", region: \"GB\")[")
        );
        let language = WordLanguage {
            values: BTreeMap::from([("w:val".into(), "en".into())]),
            unsupported: BTreeSet::new(),
        };
        let mut losses = vec![];
        assert_eq!(language.segments("漢", &mut losses)[0].1, None);
        assert!(losses.iter().any(|l| l.code == "word-language-missing"));
    }
    #[test]
    fn language_split_keeps_comments_exact_but_multislot_revisions_require_confirmation() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/styles.xml", &xml::parse(format!(r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-GB" w:eastAsia="zh-CN"/></w:rPr></w:rPrDefault></w:docDefaults></w:styles>"#).as_bytes()).unwrap());
        p.set_xml(
            "word/document.xml",
            &xml::parse(
                format!(
                    r#"<w:document xmlns:w="{W}"><w:body><w:p>
          <w:commentRangeStart w:id="0"/><w:r><w:t>A漢</w:t></w:r><w:commentRangeEnd w:id="0"/>
          <w:del w:id="10"><w:r><w:delText>B字</w:delText></w:r></w:del>
          <w:ins w:id="11"><w:r><w:t>C文</w:t></w:r></w:ins>
          <w:del w:id="12"><w:r><w:delText>甲乙</w:delText></w:r></w:del>
          <w:ins w:id="13"><w:r><w:t>新增</w:t></w:r></w:ins>
        </w:p></w:body></w:document>"#
                )
                .as_bytes(),
            )
            .unwrap(),
        );
        p.set_xml("word/comments.xml", &xml::parse(format!(r#"<w:comments xmlns:w="{W}"><w:comment w:id="0"><w:p><w:r><w:t>跨语言</w:t></w:r></w:p></w:comment></w:comments>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        let annotations: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let records = annotations["annotations"].as_array().unwrap();
        let comment = records.iter().find(|a| a["purpose"] == "comment").unwrap();
        assert_eq!(comment["target"]["kind"], "sourceRange");
        let range = &comment["target"]["anchor"]["sourceRange"];
        let from = range["from"].as_u64().unwrap() as usize;
        let to = range["to"].as_u64().unwrap() as usize;
        assert_eq!(range["raw"], "A]#text(lang: \"zh\", region: \"CN\")[漢");
        assert_eq!(range["raw"], &source[from..to]);
        for id in ["10", "11", "12", "13"] {
            let revision = records
                .iter()
                .find(|a| a["purpose"] == "suggestion" && a["wordImport"]["originalId"] == id)
                .unwrap();
            let split = matches!(id, "10" | "11");
            assert_eq!(revision["suggestion"]["requiresReconfirmation"], split);
            assert_eq!(
                revision["target"]["kind"],
                if split {
                    "wholeResource"
                } else {
                    "sourceRange"
                }
            );
            if id == "12" {
                assert_eq!(revision["target"]["anchor"]["sourceRange"]["raw"], "甲乙");
            }
        }
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-structural-revision" && l.detail.contains("不同语言包装"))
        );
        assert!(!source.contains("C文"));
        assert!(!source.contains("新增"));
    }
    #[test]
    fn note_parts_are_author_source_and_use_their_own_relationships() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:p><w:bookmarkStart w:id="4" w:name="noteTarget"/><w:r><w:footnoteReference w:id="7"/></w:r><w:bookmarkEnd w:id="4"/><w:fldSimple w:instr=" NOTEREF noteTarget "><w:r><w:t>1</w:t></w:r></w:fldSimple><w:r><w:endnoteReference w:id="2"/></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/custom-foot.xml",&xml::parse(format!(r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="-1" w:type="separator"><w:p/></w:footnote><w:footnote w:id="7"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>中文注释😀</w:t></w:r><w:hyperlink r:id="local"><w:r><w:t>来源</w:t></w:r></w:hyperlink></w:p><w:p><w:r><w:t>第二段</w:t></w:r></w:p></w:footnote></w:footnotes>"#).as_bytes()).unwrap());
        p.ensure_rel("footnotes", "custom-foot.xml");
        let mut rels = Element::new("Relationships").attr(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/relationships",
        );
        rels.push(
            Element::new("Relationship")
                .attr("Id", "local")
                .attr("Type", format!("{}hyperlink", package::REL))
                .attr("Target", "https://example.com/note")
                .attr("TargetMode", "External"),
        );
        p.set_part_relationships("word/custom-foot.xml", &rels)
            .unwrap();
        p.set_xml("word/endnotes.xml",&xml::parse(format!(r#"<w:endnotes xmlns:w="{W}"><w:endnote w:id="2"><w:p/></w:endnote></w:endnotes>"#).as_bytes()).unwrap());
        p.ensure_rel("endnotes", "endnotes.xml");
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert_eq!(source.matches("#footnote[").count(), 1);
        assert!(source.contains("#ref(<weftext-word-footnote-7>)"));
        assert!(source.contains("#strong[中文注释😀]"));
        assert!(source.contains("https://example.com/note"));
        assert!(source.contains("第二段"));
        assert!(source.contains("#endnote(id: \"weftext-word-endnote-2\")[]"));
        assert!(imported.files.contains_key("document/weftext-endnotes.typ"));
        let annotations: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        assert!(annotations["annotations"].as_array().unwrap().is_empty());
    }
    #[test]
    fn duplicate_or_missing_note_bodies_never_guess_and_math_comments_are_exact() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><w:r><w:footnoteReference w:id="7"/></w:r><w:r><w:endnoteReference w:id="9"/></w:r><w:commentRangeStart w:id="0"/><m:oMath><m:f><m:num><m:r><m:t>x</m:t></m:r></m:num><m:den><m:r><m:t>x</m:t></m:r></m:den></m:f></m:oMath><w:commentRangeEnd w:id="0"/></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        p.set_xml("word/footnotes.xml",&xml::parse(format!(r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="7"><w:p><w:r><w:t>甲</w:t></w:r></w:p></w:footnote><w:footnote w:id="7"><w:p><w:r><w:t>乙</w:t></w:r></w:p></w:footnote></w:footnotes>"#).as_bytes()).unwrap());
        p.ensure_rel("footnotes", "footnotes.xml");
        p.set_xml("word/comments.xml",&xml::parse(format!(r#"<w:comments xmlns:w="{W}"><w:comment w:id="0"><w:p><w:r><w:t>整式</w:t></w:r></w:p></w:comment></w:comments>"#).as_bytes()).unwrap());
        let imported = proposal(p);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert!(!source.contains("#footnote["));
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-note-duplicate-id")
        );
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .filter(|l| l.code == "word-note-body-missing")
                .count()
                == 2
        );
        let annotations: Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let raw = annotations["annotations"][0]["target"]["anchor"]["sourceRange"]["raw"]
            .as_str()
            .unwrap();
        assert!(raw.starts_with('$') && raw.ends_with('$'));
        assert!(raw.contains("frac"));
    }
    #[test]
    fn caption_pairing_is_explicit_hash_bound_and_compiles_to_native_figure() {
        let mut p = Package::load(None).unwrap();
        p.set_xml("word/document.xml",&xml::parse(format!(r#"<w:document xmlns:w="{W}"><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>单元格</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>表 </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> SEQ Table </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>99</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t>：中文结果</w:t></w:r></w:p></w:body></w:document>"#).as_bytes()).unwrap());
        let bytes = p.finish().unwrap();
        let analysis = analyze_docx_import(&bytes).unwrap();
        assert_eq!(analysis.captions[0].suggested_text, "中文结果");
        assert_eq!(analysis.objects[0].kind, "table");
        let mut options = DocxImportOptions {
            content_hash: analysis.content_hash,
            caption_pairings: vec![DocxCaptionPairing {
                caption_block: 1,
                object_block: 0,
                kind: "table".into(),
                caption_text: "中文结果".into(),
                position: "top".into(),
            }],
        };
        let imported = build_docx_native_package_with_options(
            &bytes,
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
            &options,
        )
        .unwrap();
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert!(source.contains("#figure(kind: table"));
        assert!(!source.contains("99"));
        let dir = tempfile::TempDir::new().unwrap();
        let out = weftext_engine::Engine::new_with_system_fonts(false)
            .unwrap()
            .render(source, &dir.path().join("note.typ"), dir.path())
            .unwrap();
        assert!(out.ok, "{source}\n{:?}", out.diagnostics);
        options.content_hash = "stale".into();
        assert!(
            build_docx_native_package_with_options(
                &bytes,
                include_str!("../../../../docs/design/examples/schema.authoring.json"),
                "40fa8a18-d063-435a-a962-5be0b218d453",
                &options
            )
            .is_err()
        );
    }
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
        assert!(source.contains("#text(lang: \"zh\", region: \"CN\")[原文😀]"));
        assert!(source.contains("#text(lang: \"zh\", region: \"CN\")[后文]"));
        assert!(source.contains("#text(lang: \"zh\", region: \"CN\")[删除]"));
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
