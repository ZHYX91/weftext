use super::{
    DocxError,
    xml::{self, Element, Xml},
};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub(super) const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub(super) const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
pub(super) struct Package {
    pub parts: BTreeMap<String, Vec<u8>>,
    pub rels: Element,
    pub types: Element,
    next: usize,
}
impl Package {
    pub fn load(bytes: Option<&[u8]>) -> Result<Self, DocxError> {
        let mut parts = BTreeMap::new();
        if let Some(bytes) = bytes {
            if bytes.len() > 32 * 1024 * 1024 {
                return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_docx_package_archive_bytes_exceeded(),
                )));
            }
            let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| {
                DocxError::new(
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_archive(),
                    )
                    .diagnostic(
                        weftext_messages::DiagnosticOrigin::Dependency,
                        e.to_string(),
                    ),
                )
            })?;
            if zip.len() > 1024 {
                return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_docx_package_part_count_exceeded(),
                )));
            }
            let mut total = 0u64;
            for i in 0..zip.len() {
                let f = zip.by_index(i).map_err(|e| {
                    DocxError::new(
                        weftext_messages::ErrorDescription::new(
                            weftext_messages::UiMessage::error_export_archive(),
                        )
                        .diagnostic(
                            weftext_messages::DiagnosticOrigin::Dependency,
                            e.to_string(),
                        ),
                    )
                })?;
                let name = f.name().to_owned();
                if f.is_dir() {
                    continue;
                }
                if name.starts_with('/')
                    || name.contains('\\')
                    || name.split('/').any(|p| p == ".." || p.is_empty())
                    || name.contains(':')
                {
                    return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_package_part_path_unsafe(),
                    )));
                }
                if f.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
                    return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_package_symbolic_link_forbidden(),
                    )));
                }
                total = total.checked_add(f.size()).ok_or_else(|| {
                    DocxError::new(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_package_budget_overflow(),
                    ))
                })?;
                if f.size() > 16 * 1024 * 1024 || total > 64 * 1024 * 1024 {
                    return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_package_expanded_bytes_exceeded(),
                    )));
                }
                let lower = name.to_ascii_lowercase();
                if lower.contains("vba")
                    || lower.contains("activex")
                    || lower.starts_with("word/embeddings/")
                    || lower.starts_with("_xmlsignatures/")
                {
                    return Err(DocxError::new(
                        weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_docx_package_active_content_unsupported()),
                    ));
                }
                let mut data = vec![];
                f.take(16 * 1024 * 1024 + 1)
                    .read_to_end(&mut data)
                    .map_err(|e| {
                        DocxError::new(
                            weftext_messages::ErrorDescription::new(
                                weftext_messages::UiMessage::error_export_io(),
                            )
                            .diagnostic(
                                weftext_messages::DiagnosticOrigin::Dependency,
                                e.to_string(),
                            ),
                        )
                    })?;
                if data.len() as u64 > 16 * 1024 * 1024 {
                    return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_package_part_expansion_exceeded(),
                    )));
                }
                if parts.insert(name, data).is_some() {
                    return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_docx_package_part_path_duplicate(
                        ),
                    )));
                }
            }
        } else {
            parts.insert(
                "word/document.xml".into(),
                DEFAULT_DOCUMENT.as_bytes().to_vec(),
            );
            parts.insert("word/styles.xml".into(), DEFAULT_STYLES.as_bytes().to_vec());
            parts.insert(
                "[Content_Types].xml".into(),
                DEFAULT_TYPES.as_bytes().to_vec(),
            );
            parts.insert("_rels/.rels".into(),format!(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{REL}officeDocument" Target="word/document.xml"/></Relationships>"#).into_bytes());
        }
        for required in ["[Content_Types].xml", "word/document.xml", "_rels/.rels"] {
            if !parts.contains_key(required) {
                return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_docx_template_part_missing(
                        weftext_messages::params::ErrorExportDocxTemplatePartMissing {
                            part: (required).to_string(),
                        },
                    ),
                )));
            }
        }
        for (name, bytes) in &parts {
            if name.ends_with(".xml") || name.ends_with(".rels") {
                let root = xml::parse(bytes)?;
                if name.ends_with(".rels") {
                    validate_rels(&root)?;
                }
                if name.starts_with("word/") {
                    let mut fields = vec![];
                    root.descendants("w:instrText", &mut fields);
                    let mut simple = vec![];
                    root.descendants("w:fldSimple", &mut simple);
                    let all = fields.iter().map(|e| e.text()).collect::<Vec<_>>().join("");
                    if unsafe_field(&all)
                        || simple
                            .iter()
                            .any(|e| unsafe_field(e.get("w:instr").unwrap_or("")))
                    {
                        return Err(DocxError::new(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_docx_package_active_field_forbidden())));
                    }
                    let mut chunks = vec![];
                    root.descendants("w:altChunk", &mut chunks);
                    if !chunks.is_empty() {
                        return Err(DocxError::new(weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_export_docx_package_alt_chunk_unsupported())));
                    }
                }
            }
        }
        let rootrels = xml::parse(parts.get("_rels/.rels").unwrap())?;
        if !rootrels.children.iter().filter_map(Xml::element).any(|e| {
            e.get("Type") == Some(&format!("{REL}officeDocument"))
                && e.get("Target")
                    .is_some_and(|t| matches!(t, "word/document.xml" | "/word/document.xml"))
        }) {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_main_document_path_unsupported(),
            )));
        }
        let types = xml::parse(parts.get("[Content_Types].xml").unwrap())?;
        if types.name != "Types"
            || types.get("xmlns")
                != Some("http://schemas.openxmlformats.org/package/2006/content-types")
        {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_content_types_invalid(),
            )));
        }
        if types.children.iter().filter_map(Xml::element).any(|e| {
            e.get("ContentType")
                .is_some_and(|t| t.to_ascii_lowercase().contains("macroenabled"))
        }) {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_macro_enabled_unsupported(),
            )));
        }
        let rels = if let Some(b) = parts.get("word/_rels/document.xml.rels") {
            xml::parse(b)?
        } else {
            xml::parse(b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>")?
        };
        Ok(Self {
            parts,
            rels,
            types,
            next: 1,
        })
    }
    pub fn xml(&self, name: &str) -> Result<Option<Element>, DocxError> {
        self.parts.get(name).map(|b| xml::parse(b)).transpose()
    }
    /// Resolve a unique internal main-document relationship, never a guessed filename.
    pub fn related_part(&self, kind: &str, fallback: &str) -> Result<String, DocxError> {
        let relations: Vec<_> = self
            .rels
            .children
            .iter()
            .filter_map(Xml::element)
            .filter(|e| e.get("Type") == Some(kind))
            .collect();
        if relations.len() > 1 {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_comment_relationship_not_unique(),
            )));
        }
        let Some(rel) = relations.first() else {
            return Ok(fallback.into());
        };
        if rel.get("TargetMode") == Some("External") {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_external_comment_part_forbidden(),
            )));
        }
        let target = rel.get("Target").ok_or_else(|| {
            DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_comment_target_missing(),
            ))
        })?;
        let path = if target.starts_with('/') {
            target.trim_start_matches('/').to_owned()
        } else {
            format!("word/{target}")
        };
        if !self.parts.contains_key(&path) {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_comment_part_missing(),
            )));
        }
        Ok(path)
    }
    pub fn set_xml(&mut self, name: &str, e: &Element) {
        self.parts.insert(name.into(), xml::serialize(e));
    }
    pub fn part_relationships(&self, part: &str) -> Result<Element, DocxError> {
        let path = part_rels_path(part)?;
        self.xml(&path)?.map_or_else(
            || xml::parse(b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>"),
            Ok,
        )
    }
    pub fn set_part_relationships(&mut self, part: &str, rels: &Element) -> Result<(), DocxError> {
        let path = part_rels_path(part)?;
        self.set_xml(&path, rels);
        Ok(())
    }
    pub fn relationship(&mut self, kind: &str, target: &str, external: bool) -> String {
        loop {
            let id = format!("weftextRel{}", self.next);
            self.next += 1;
            if !self
                .rels
                .children
                .iter()
                .filter_map(Xml::element)
                .any(|e| e.get("Id") == Some(&id))
                && !self
                    .parts
                    .iter()
                    .filter(|(name, _)| name.ends_with(".rels"))
                    .any(|(_, bytes)| {
                        xml::parse(bytes).is_ok_and(|root| {
                            root.children
                                .iter()
                                .filter_map(Xml::element)
                                .any(|e| e.get("Id") == Some(&id))
                        })
                    })
            {
                let mut e = Element::new("Relationship")
                    .attr("Id", &id)
                    .attr("Type", format!("{REL}{kind}"))
                    .attr("Target", target);
                if external {
                    e.set_attr("TargetMode", "External");
                }
                self.rels.push(e);
                return id;
            }
        }
    }
    pub fn ensure_rel(&mut self, kind: &str, target: &str) {
        if !self
            .rels
            .children
            .iter()
            .filter_map(Xml::element)
            .any(|e| e.get("Type") == Some(&format!("{REL}{kind}")))
        {
            self.relationship(kind, target, false);
        }
    }
    pub fn content_type(&mut self, name: &str, kind: &str) {
        if !self
            .types
            .children
            .iter()
            .filter_map(Xml::element)
            .any(|e| e.get("PartName") == Some(name))
        {
            self.types.push(
                Element::new("Override")
                    .attr("PartName", name)
                    .attr("ContentType", kind),
            );
        }
    }
    pub fn media(&mut self, extension: &str, content_type: &str, data: Vec<u8>) -> String {
        let mut n = 1;
        let path = loop {
            let p = format!("word/media/weftext{n}.{extension}");
            if !self.parts.contains_key(&p) {
                break p;
            }
            n += 1;
        };
        self.content_type(&format!("/{path}"), content_type);
        self.parts.insert(path.clone(), data);
        self.relationship("image", path.strip_prefix("word/").unwrap(), false)
    }
    pub fn finish(mut self) -> Result<Vec<u8>, DocxError> {
        self.set_xml("word/_rels/document.xml.rels", &self.rels.clone());
        self.set_xml("[Content_Types].xml", &self.types.clone());
        let mut zip = ZipWriter::new(Cursor::new(vec![]));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in self.parts {
            zip.start_file(name, options).map_err(|e| {
                DocxError::new(
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_archive(),
                    )
                    .diagnostic(
                        weftext_messages::DiagnosticOrigin::Dependency,
                        e.to_string(),
                    ),
                )
            })?;
            zip.write_all(&data).map_err(|e| {
                DocxError::new(
                    weftext_messages::ErrorDescription::new(
                        weftext_messages::UiMessage::error_export_io(),
                    )
                    .diagnostic(
                        weftext_messages::DiagnosticOrigin::Dependency,
                        e.to_string(),
                    ),
                )
            })?;
        }
        zip.finish().map(|c| c.into_inner()).map_err(|e| {
            DocxError::new(
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_archive(),
                )
                .diagnostic(
                    weftext_messages::DiagnosticOrigin::Dependency,
                    e.to_string(),
                ),
            )
        })
    }
}
fn part_rels_path(part: &str) -> Result<String, DocxError> {
    if part.contains('\\')
        || part.contains(':')
        || part.split('/').any(|v| v.is_empty() || v == "..")
    {
        return Err(DocxError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_docx_package_part_path_invalid(),
        )));
    }
    let (dir, file) = part.rsplit_once('/').ok_or_else(|| {
        DocxError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_docx_package_part_directory_missing(),
        ))
    })?;
    Ok(format!("{dir}/_rels/{file}.rels"))
}
fn unsafe_field(s: &str) -> bool {
    let upper = s.to_ascii_uppercase();
    upper.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| {
        matches!(
            t,
            "DDE" | "DDEAUTO" | "INCLUDETEXT" | "INCLUDEPICTURE" | "LINK" | "DATABASE"
        )
    })
}
fn validate_rels(root: &Element) -> Result<(), DocxError> {
    if root.name != "Relationships"
        || root.get("xmlns") != Some("http://schemas.openxmlformats.org/package/2006/relationships")
    {
        return Err(DocxError::new(weftext_messages::ErrorDescription::new(
            weftext_messages::UiMessage::error_export_docx_package_relationship_namespace_required(
            ),
        )));
    }
    for e in root.children.iter().filter_map(Xml::element) {
        if e.get("TargetMode")
            .is_some_and(|v| v.eq_ignore_ascii_case("external"))
        {
            if !e
                .get("Type")
                .is_some_and(|v| v == format!("{REL}hyperlink"))
                || !e.get("Target").is_some_and(super::external_url)
            {
                return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_export_docx_package_external_resource_forbidden(),
                )));
            }
        } else if e
            .get("Target")
            .is_some_and(|v| v.contains('\\') || v.contains(':') || v.starts_with("//"))
        {
            return Err(DocxError::new(weftext_messages::ErrorDescription::new(
                weftext_messages::UiMessage::error_export_docx_package_resource_relationship_unsafe(
                ),
            )));
        }
    }
    Ok(())
}
pub(super) const DEFAULT_DOCUMENT: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#;
pub(super) const DEFAULT_STYLES: &str = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Aptos" w:hAnsi="Aptos" w:eastAsia="Noto Serif CJK SC" w:cs="Aptos"/><w:sz w:val="22"/><w:szCs w:val="22"/><w:lang w:val="zh-CN" w:eastAsia="zh-CN"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;
const DEFAULT_TYPES: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
