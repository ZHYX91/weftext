use super::*;
use std::{
    io::{Cursor, Read},
    sync::OnceLock,
};
use tempfile::TempDir;
use weftext_engine::Engine;

fn render(source: &str, dir: &TempDir) -> RenderOutput {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    let out = ENGINE
        .get_or_init(|| Engine::new_with_system_fonts(false).unwrap())
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(out.ok, "{source}\n{:?}", out.diagnostics);
    out
}
fn artifact(render: &RenderOutput) -> ExportArtifact {
    export_docx(DocxInput {
        render,
        template: None,
        bindings: &BTreeMap::new(),
        options: DocxOptions::default(),
    })
    .unwrap()
}
fn part(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut result = vec![];
    zip.by_name(name).unwrap().read_to_end(&mut result).unwrap();
    result
}
fn document(a: &ExportArtifact) -> Element {
    xml::parse(&part(&a.bytes, "word/document.xml")).unwrap()
}
fn find<'a>(e: &'a Element, name: &str) -> Vec<&'a Element> {
    let mut result = vec![];
    e.descendants(name, &mut result);
    result
}
fn pstyle(p: &Element) -> Option<&str> {
    p.child("w:pPr")
        .and_then(|p| p.child("w:pStyle"))
        .and_then(|p| p.get("w:val"))
}
fn run_in_recipe() -> &'static str {
    r#"#show <methods>: it => context {
  strong[
    #if it.numbering != none {
      counter(heading).display(it.numbering)
      text(" ")
    }
    #it.body
  ]
  text(" ")
}"#
}
fn template(doc: &str, styles: Option<&str>) -> Vec<u8> {
    let mut p = Package::load(None).unwrap();
    p.parts
        .insert("word/document.xml".into(), doc.as_bytes().to_vec());
    if let Some(s) = styles {
        p.parts
            .insert("word/styles.xml".into(), s.as_bytes().to_vec());
    }
    p.ensure_rel("styles", "styles.xml");
    p.content_type(
        "/word/styles.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
    );
    p.finish().unwrap()
}
const TEMPLATE_DOC: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>固定模板</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="project"/><w:alias w:val="项目"/></w:sdtPr><w:sdtContent><w:r><w:t>OLD FIELD</w:t></w:r></w:sdtContent></w:sdt></w:p><w:sdt><w:sdtPr><w:tag w:val="title"/></w:sdtPr><w:sdtContent><w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>OLD TITLE</w:t></w:r></w:p></w:sdtContent></w:sdt><w:sdt><w:sdtPr><w:tag w:val="body"/><w:dataBinding w:xpath="/old" w:storeItemID="old"/></w:sdtPr><w:sdtContent><w:p><w:pPr><w:pStyle w:val="Body"/></w:pPr><w:r><w:rPr><w:rStyle w:val="BodyChar"/><w:color w:val="123456"/></w:rPr><w:t>OLD BODY</w:t></w:r></w:p></w:sdtContent></w:sdt><w:sectPr/></w:body></w:document>"#;
const TEMPLATE_STYLES: &str = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:asciiTheme="minorHAnsi" w:hAnsiTheme="minorHAnsi" w:eastAsiaTheme="minorEastAsia"/><w:sz w:val="20"/><w:lang w:val="zh-CN" w:eastAsia="zh-CN"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="BodyBase"><w:basedOn w:val="Normal"/><w:rPr><w:sz w:val="24"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Body"><w:basedOn w:val="BodyBase"/></w:style><w:style w:type="character" w:styleId="BodyChar"><w:rPr><w:i w:val="0"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Heading Font" w:hAnsi="Heading Font" w:eastAsia="Heading CJK"/><w:b/><w:i/><w:sz w:val="48"/><w:color w:val="00FF00"/></w:rPr></w:style></w:styles>"#;
const THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="测试"><a:themeElements><a:fontScheme name="测试字体"><a:majorFont><a:latin typeface="Major Font"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Template Body Font"/><a:ea typeface=""/><a:cs typeface=""/><a:font script="Hans" typeface="模板中文字体"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#;

#[test]
fn real_native_tree_exports_chinese_editable_roles_and_nine_level_numbering() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set document(title:[中文标题])\n#title()\n#block[副标题] <weftext-subtitle>\n#set heading(numbering:\"1.1\")\n= 7. 原题 <one>\n正文 *强调* _斜体_\n#heading(level:9)[九级]\n#heading(level:10)[十级]\n见 @one",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let ps = find(&doc, "w:p");
    assert!(
        ps.iter()
            .any(|p| pstyle(p) == Some("Title") && p.text().contains("中文标题"))
    );
    assert!(
        ps.iter()
            .any(|p| pstyle(p) == Some("Subtitle") && p.text().contains("副标题"))
    );
    let h = ps.iter().find(|p| pstyle(p) == Some("Heading1")).unwrap();
    assert_eq!(h.text(), "7. 原题");
    assert_eq!(
        h.child("w:pPr")
            .unwrap()
            .child("w:outlineLvl")
            .unwrap()
            .get("w:val"),
        Some("0")
    );
    assert!(h.child("w:pPr").unwrap().child("w:numPr").is_some());
    assert!(ps.iter().any(|p| pstyle(p) == Some("Heading9")));
    assert!(ps.iter().any(|p| pstyle(p) == Some("WeftextHeading10")));
    assert!(
        a.losses
            .iter()
            .any(|l| l.code == "heading-depth-unsupported")
    );
    let numbering = xml::parse(&part(&a.bytes, "word/numbering.xml")).unwrap();
    let heading = find(&numbering, "w:abstractNum")[0];
    assert_eq!(find(heading, "w:lvl").len(), 9);
    assert!(
        find(heading, "w:lvlText")
            .iter()
            .any(|e| e.get("w:val") == Some("%1.%2.%3.%4.%5.%6.%7.%8.%9"))
    );
    assert!(!find(&doc, "w:bookmarkStart").is_empty());
    let emphasis = find(&doc, "w:r")
        .into_iter()
        .find(|r| r.text() == "强调")
        .unwrap();
    assert_eq!(
        emphasis
            .child("w:rPr")
            .unwrap()
            .child("w:b")
            .unwrap()
            .get("w:val"),
        Some("1")
    );
}

#[test]
fn template_run_in_resets_effective_body_and_preserves_author_emphasis() {
    let dir = TempDir::new().unwrap();
    let source = format!(
        "#set document(title:[中文标题])\n#title()\n#set text(size:30pt, font:\"Noto Serif CJK SC\")\n#set heading(numbering:\"1.1\")\n{}\n== 方法 <methods>\n正文 *明确强调* _明确斜体_",
        run_in_recipe()
    );
    let out = render(&source, &dir);
    let bytes = template(TEMPLATE_DOC, Some(TEMPLATE_STYLES));
    let mut pkg = Package::load(Some(&bytes)).unwrap();
    pkg.parts
        .insert("word/theme/theme1.xml".into(), THEME.as_bytes().to_vec());
    pkg.parts.insert("word/footer1.xml".into(),"<w:ftr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:r><w:t>不可丢失的页脚</w:t></w:r></w:p></w:ftr>".as_bytes().to_vec());
    pkg.content_type(
        "/word/footer1.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml",
    );
    pkg.ensure_rel("theme", "theme/theme1.xml");
    let template = pkg.finish().unwrap();
    let bindings = BTreeMap::from([
        ("body".into(), DocxBinding::Body),
        ("title".into(), DocxBinding::Title),
        (
            "project".into(),
            DocxBinding::Text {
                value: "示例 & <项目>".into(),
            },
        ),
    ]);
    let a = export_docx(DocxInput {
        render: &out,
        template: Some(&template),
        bindings: &bindings,
        options: DocxOptions::default(),
    })
    .unwrap();
    let doc = document(&a);
    assert!(doc.text().contains("示例 & <项目>"));
    assert!(!doc.text().contains("OLD"));
    assert_eq!(doc.text().matches("中文标题").count(), 1);
    assert!(find(&doc, "w:dataBinding").is_empty());
    assert_eq!(
        part(&a.bytes, "word/footer1.xml"),
        part(&template, "word/footer1.xml")
    );
    assert_eq!(
        part(&a.bytes, "word/theme/theme1.xml"),
        part(&template, "word/theme/theme1.xml")
    );
    let ps = find(&doc, "w:p");
    let h = ps
        .iter()
        .find(|p| pstyle(p) == Some("Heading2"))
        .expect("run-in should be one native Heading2 paragraph");
    assert!(h.text().contains("方法"));
    assert!(h.text().contains("正文"));
    assert!(!h.text().contains("0.1"));
    let runs = find(h, "w:r");
    let body = runs.iter().find(|r| r.text().contains("正文")).unwrap();
    let props = body.child("w:rPr").unwrap();
    assert_eq!(
        props.child("w:rFonts").unwrap().get("w:ascii"),
        Some("Template Body Font")
    );
    assert_eq!(
        props.child("w:rFonts").unwrap().get("w:eastAsia"),
        Some("模板中文字体")
    );
    assert_eq!(props.child("w:sz").unwrap().get("w:val"), Some("24"));
    assert_eq!(props.child("w:b").unwrap().get("w:val"), Some("0"));
    assert_eq!(props.child("w:i").unwrap().get("w:val"), Some("0"));
    assert_eq!(props.child("w:color").unwrap().get("w:val"), Some("123456"));
    let strong = runs.iter().find(|r| r.text() == "明确强调").unwrap();
    assert_eq!(
        strong
            .child("w:rPr")
            .unwrap()
            .child("w:b")
            .unwrap()
            .get("w:val"),
        Some("1")
    );
    let emph = runs.iter().find(|r| r.text() == "明确斜体").unwrap();
    assert_eq!(
        emph.child("w:rPr")
            .unwrap()
            .child("w:i")
            .unwrap()
            .get("w:val"),
        Some("1")
    );
    assert!(
        a.losses
            .iter()
            .any(|l| l.code == "run-in-toc-whole-paragraph")
    );
    let styles = xml::parse(&part(&a.bytes, "word/styles.xml")).unwrap();
    let heading = find(&styles, "w:style")
        .into_iter()
        .find(|s| s.get("w:styleId") == Some("Heading2"))
        .unwrap();
    assert!(
        heading
            .child("w:pPr")
            .unwrap()
            .child("w:keepNext")
            .is_some()
    );
}

#[test]
fn real_tables_nested_lists_and_equations_remain_native() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#table(columns:2,table.header([甲],[乙]),table.cell(rowspan:2)[跨行], [#table(columns:1,[嵌套表])], [末行])\n\n+ 有序一\n+ 有序二\n  - 内层\n\n$ frac(1,2)+x^2+sqrt(3) $\n\n$ overbrace(1+2) $",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    assert!(find(&doc, "w:tbl").len() >= 2);
    assert!(!find(&doc, "w:vMerge").is_empty());
    assert!(doc.text().contains("嵌套表"));
    assert!(!find(&doc, "w:numPr").is_empty());
    assert!(!find(&doc, "m:f").is_empty());
    assert!(!find(&doc, "m:sSup").is_empty());
    assert!(!find(&doc, "m:rad").is_empty());
    assert!(a.losses.iter().any(|l| l.code == "math-structure-loss"));
}

#[test]
fn source_style_mode_is_optional_and_reports_unmapped_properties() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set text(size:18pt,fill:rgb(\"123abc\"))\n中文 *强调*",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let run = find(&doc, "w:r")
        .into_iter()
        .find(|r| r.text().contains("中文"))
        .unwrap();
    assert_eq!(
        run.child("w:rPr")
            .unwrap()
            .child("w:sz")
            .unwrap()
            .get("w:val"),
        Some("22")
    );
    let b = export_docx(DocxInput {
        render: &out,
        template: None,
        bindings: &BTreeMap::new(),
        options: DocxOptions {
            preserve_source_styles: true,
            reject_losses: false,
        },
    })
    .unwrap();
    let doc = document(&b);
    let run = find(&doc, "w:r")
        .into_iter()
        .find(|r| r.text().contains("中文"))
        .unwrap();
    assert_eq!(
        run.child("w:rPr")
            .unwrap()
            .child("w:sz")
            .unwrap()
            .get("w:val"),
        Some("36")
    );
    assert_eq!(
        run.child("w:rPr")
            .unwrap()
            .child("w:color")
            .unwrap()
            .get("w:val"),
        Some("123ABC")
    );
}

#[test]
fn hidden_heading_is_not_reinserted_and_custom_show_stays_actual_text() {
    let dir = TempDir::new().unwrap();
    let out = render("#show heading:none\n= 幽灵标题\n\n实际正文", &dir);
    let a = artifact(&out);
    let doc = document(&a);
    assert!(!doc.text().contains("幽灵标题"));
    assert!(doc.text().contains("实际正文"));
    assert!(
        find(&doc, "w:p")
            .iter()
            .all(|p| pstyle(p) != Some("Heading1"))
    );
    let out = render("#show heading:it=>[PREFIX #it.body SUFFIX]\n= 原题", &dir);
    let doc = document(&artifact(&out));
    assert!(doc.text().contains("PREFIX 原题 SUFFIX"));
    assert!(
        find(&doc, "w:p")
            .iter()
            .all(|p| pstyle(p) != Some("Heading1"))
    );
}

#[test]
fn standard_controls_are_inspected_and_invalid_mapping_is_rejected() {
    let dir = TempDir::new().unwrap();
    let out = render("正文", &dir);
    let t = template(TEMPLATE_DOC, None);
    let controls = list_template_controls(&t).unwrap();
    assert_eq!(
        controls
            .iter()
            .map(|c| (c.tag.as_str(), c.placement.as_str()))
            .collect::<Vec<_>>(),
        [("project", "inline"), ("title", "block"), ("body", "block")]
    );
    let bindings = BTreeMap::from([("missing".into(), DocxBinding::Body)]);
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: Some(&t),
            bindings: &bindings,
            options: DocxOptions::default()
        })
        .unwrap_err()
        .0
        .contains("missing")
    );
    let bindings = BTreeMap::from([("project".into(), DocxBinding::Body)]);
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: Some(&t),
            bindings: &bindings,
            options: DocxOptions::default()
        })
        .is_err()
    );
}

#[test]
fn template_security_rejects_macros_dtd_external_images_and_active_fields() {
    let t = template(TEMPLATE_DOC, None);
    let mut pkg = Package::load(Some(&t)).unwrap();
    pkg.parts
        .insert("word/vbaProject.bin".into(), vec![1, 2, 3]);
    assert!(list_template_controls(&pkg.finish().unwrap()).is_err());
    for doc in [
        r#"<!DOCTYPE w:document [<!ENTITY x SYSTEM "file:///secret">]><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:instrText>DDEAUTO cmd /c evil</w:instrText></w:r></w:p></w:body></w:document>"#,
    ] {
        let mut p = Package::load(None).unwrap();
        p.parts
            .insert("word/document.xml".into(), doc.as_bytes().to_vec());
        assert!(list_template_controls(&p.finish().unwrap()).is_err());
    }
    let mut p = Package::load(Some(&t)).unwrap();
    p.relationship("image", "https://example.invalid/secret.png", true);
    assert!(list_template_controls(&p.finish().unwrap()).is_err());
}

#[test]
fn real_inline_resource_is_embedded_and_export_is_deterministic() {
    let dir = TempDir::new().unwrap();
    let png=base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==").unwrap();
    std::fs::write(dir.path().join("tiny.png"), &png).unwrap();
    let out = render("中文 #image(\"tiny.png\") 图片", &dir);
    let a = artifact(&out);
    let doc = document(&a);
    assert_eq!(find(&doc, "w:drawing").len(), 1);
    assert_eq!(part(&a.bytes, "word/media/weftext1.png"), png);
    assert_eq!(a.bytes, artifact(&out).bytes);
}

#[test]
fn query_table_binding_consumes_frozen_nodes_and_keeps_template_layout() {
    let dir = TempDir::new().unwrap();
    let out = render("原笔记", &dir);
    let query = render(
        "#table(columns:2,[姓名],[金额],[张三],[9007199254740993])",
        &dir,
    );
    let t = template(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>查询表在此</w:t></w:r></w:p><w:sdt><w:sdtPr><w:tag w:val="query"/></w:sdtPr><w:sdtContent><w:p/></w:sdtContent></w:sdt></w:body></w:document>"#,
        None,
    );
    let bindings = BTreeMap::from([(
        "query".into(),
        DocxBinding::Nodes {
            nodes: vec![query.tree.unwrap()],
        },
    )]);
    let a = export_docx(DocxInput {
        render: &out,
        template: Some(&t),
        bindings: &bindings,
        options: DocxOptions::default(),
    })
    .unwrap();
    let doc = document(&a);
    assert!(doc.text().contains("查询表在此"));
    assert!(doc.text().contains("9007199254740993"));
    assert_eq!(find(&doc, "w:tbl").len(), 1);
    assert!(!doc.text().contains("原笔记"));
}

#[test]
fn failed_compile_wrong_target_and_declined_losses_cannot_generate_bytes() {
    let dir = TempDir::new().unwrap();
    let mut out = render("正文", &dir);
    out.ok = false;
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default()
        })
        .is_err()
    );
    out.ok = true;
    out.products.as_mut().unwrap().target = "paged".into();
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default()
        })
        .is_err()
    );
    out.products.as_mut().unwrap().target = "html".into();
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions {
                reject_losses: true,
                preserve_source_styles: false
            }
        })
        .is_err()
    );
}

#[test]
fn native_ref_replaces_only_number_and_roman_pattern_is_real_numbering() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set heading(numbering:\"I.a\")\n= 第一 <one>\n== 第二 <two>\n见 @two 和 #link(<two>)[定制链接]",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let fields = find(&doc, "w:fldSimple");
    assert_eq!(fields.len(), 1);
    assert!(fields[0].get("w:instr").unwrap().contains("\\w \\h"));
    let name = fields[0]
        .get("w:instr")
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    assert!(
        find(&doc, "w:bookmarkStart")
            .iter()
            .any(|e| e.get("w:name") == Some(name))
    );
    assert!(doc.text().contains("定制链接"));
    assert!(!fields[0].text().contains("节"));
    let numbering = xml::parse(&part(&a.bytes, "word/numbering.xml")).unwrap();
    let levels = find(find(&numbering, "w:abstractNum")[0], "w:lvl");
    assert_eq!(
        levels[0].child("w:numFmt").unwrap().get("w:val"),
        Some("upperRoman")
    );
    assert_eq!(
        levels[1].child("w:numFmt").unwrap().get("w:val"),
        Some("lowerLetter")
    );
    assert_eq!(
        levels[1].child("w:lvlText").unwrap().get("w:val"),
        Some("%1.%2")
    );
    let out = render("#set heading(numbering:(..n)=>[CUSTOM])\n= 文字", &dir);
    assert!(
        artifact(&out)
            .losses
            .iter()
            .any(|l| l.code == "heading-numbering-loss")
    );
}

#[test]
fn repeated_structural_placeholder_and_unresolvable_run_in_style_are_rejected() {
    let dir = TempDir::new().unwrap();
    let out = render("正文", &dir);
    let repeated = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:sdt><w:sdtPr><w:tag w:val="body"/></w:sdtPr><w:sdtContent><w:p/></w:sdtContent></w:sdt><w:sdt><w:sdtPr><w:tag w:val="body"/></w:sdtPr><w:sdtContent><w:p/></w:sdtContent></w:sdt></w:body></w:document>"#;
    let t = template(repeated, None);
    let bindings = BTreeMap::from([("body".into(), DocxBinding::Body)]);
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: Some(&t),
            bindings: &bindings,
            options: DocxOptions::default()
        })
        .is_err()
    );
    let source = format!("{}\n== 方法 <methods>\n正文", run_in_recipe());
    let out = render(&source, &dir);
    let t = template(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:sdt><w:sdtPr><w:tag w:val="body"/></w:sdtPr><w:sdtContent><w:p/></w:sdtContent></w:sdt></w:body></w:document>"#,
        Some(
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"/></w:styles>"#,
        ),
    );
    assert!(
        export_docx(DocxInput {
            render: &out,
            template: Some(&t),
            bindings: &bindings,
            options: DocxOptions::default()
        })
        .is_err()
    );
}

#[test]
fn split_external_fetch_field_is_rejected_without_opening_office() {
    let doc = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:instrText>INCLUDE</w:instrText></w:r><w:r><w:instrText>TEXT "https://example.invalid/secret"</w:instrText></w:r></w:p></w:body></w:document>"#;
    let mut p = Package::load(None).unwrap();
    p.parts
        .insert("word/document.xml".into(), doc.as_bytes().to_vec());
    assert!(list_template_controls(&p.finish().unwrap()).is_err());
}

#[test]
fn actual_counter_restart_has_native_start_override_and_deep_ref_stays_static() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set heading(numbering:\"1.\")\n#counter(heading).update(5)\n= 第六节 <six>\n== 子节 <child>\n#heading(level:10)[深节] <deep>\n见 @deep",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let ps = find(&doc, "w:p");
    let h = ps.iter().find(|p| pstyle(p) == Some("Heading1")).unwrap();
    let numid = h
        .child("w:pPr")
        .unwrap()
        .child("w:numPr")
        .unwrap()
        .child("w:numId")
        .unwrap()
        .get("w:val")
        .unwrap();
    let numbering = xml::parse(&part(&a.bytes, "word/numbering.xml")).unwrap();
    let instance = find(&numbering, "w:num")
        .into_iter()
        .find(|n| n.get("w:numId") == Some(numid))
        .unwrap();
    assert!(
        find(instance, "w:startOverride")
            .iter()
            .any(|e| e.get("w:val") == Some("6"))
    );
    let levels = find(find(&numbering, "w:abstractNum")[0], "w:lvl");
    assert_eq!(
        levels[1].child("w:lvlText").unwrap().get("w:val"),
        Some("%1.%2.")
    );
    assert!(find(&doc, "w:fldSimple").is_empty());
    assert!(
        a.losses
            .iter()
            .any(|l| l.code == "number-reference-unavailable")
    );
    assert!(
        !a.losses
            .iter()
            .any(|l| l.code == "heading-counter-state-unavailable")
    );
}

#[test]
fn custom_show_has_an_explicit_semantic_loss_even_when_actual_text_is_plain() {
    let dir = TempDir::new().unwrap();
    let out = render("#show heading:it=>[PREFIX #it.body SUFFIX]\n= 原题", &dir);
    let a = artifact(&out);
    assert!(
        a.losses
            .iter()
            .any(|l| l.code == "native-heading-role-unavailable")
    );
    assert!(document(&a).text().contains("PREFIX 原题 SUFFIX"));
}

#[test]
fn direct_template_body_fonts_override_inherited_theme_fonts() {
    let dir = TempDir::new().unwrap();
    let source = format!(
        "#set heading(numbering:\"1.1\")\n{}\n== 方法 <methods>\n正文",
        run_in_recipe()
    );
    let out = render(&source, &dir);
    let t=template(&TEMPLATE_DOC.replace("<w:rStyle w:val=\"BodyChar\"/>","<w:rStyle w:val=\"BodyChar\"/><w:rFonts w:ascii=\"DirectBodyFont\" w:hAnsi=\"DirectBodyFont\" w:eastAsia=\"DirectCJK\"/>"),Some(TEMPLATE_STYLES));
    let mut pkg = Package::load(Some(&t)).unwrap();
    pkg.parts
        .insert("word/theme/theme1.xml".into(), THEME.as_bytes().to_vec());
    let t = pkg.finish().unwrap();
    let bindings = BTreeMap::from([("body".into(), DocxBinding::Body)]);
    let a = export_docx(DocxInput {
        render: &out,
        template: Some(&t),
        bindings: &bindings,
        options: DocxOptions::default(),
    })
    .unwrap();
    let doc = document(&a);
    let body = find(&doc, "w:r")
        .into_iter()
        .find(|r| r.text().contains("正文"))
        .unwrap();
    let fonts = body.child("w:rPr").unwrap().child("w:rFonts").unwrap();
    assert_eq!(fonts.get("w:ascii"), Some("DirectBodyFont"));
    assert_eq!(fonts.get("w:eastAsia"), Some("DirectCJK"));
    assert!(fonts.get("w:asciiTheme").is_none());
}

#[test]
fn cell_inline_emphasis_is_one_editable_paragraph() {
    let dir = TempDir::new().unwrap();
    let out = render("#table(columns:1,[前 *中* 后])", &dir);
    let doc = document(&artifact(&out));
    let cells = find(&doc, "w:tc");
    let paragraphs = find(cells[0], "w:p");
    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraphs[0].text(), "前 中 后");
    assert!(find(paragraphs[0], "w:r").iter().any(|r| {
        r.text() == "中"
            && r.child("w:rPr")
                .is_some_and(|p| p.child("w:b").is_some_and(|b| b.get("w:val") == Some("1")))
    }));
}

#[test]
fn mathematical_alphabets_become_editable_base_letters_with_exact_omml_styles() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "$ x^2 + bold(y)^2 + cal(A) + bb(R) + frak(B) + sans(C) + upright(d) + alpha + \"中文\" $",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let math = find(&doc, "m:oMath");
    assert_eq!(math.len(), 1);
    let runs = find(math[0], "m:r");
    let properties = |text: &str| {
        let r = runs
            .iter()
            .find(|r| r.child("m:t").is_some_and(|t| t.text() == text))
            .unwrap_or_else(|| panic!("missing math token {text}: {:?}", math[0]));
        let p = r.child("m:rPr").unwrap();
        (
            p.child("m:scr").unwrap().get("m:val").unwrap(),
            p.child("m:sty").unwrap().get("m:val").unwrap(),
        )
    };
    assert_eq!(properties("x"), ("roman", "i"));
    assert_eq!(properties("y"), ("roman", "bi"));
    assert_eq!(properties("A"), ("script", "p"));
    assert_eq!(properties("R"), ("double-struck", "p"));
    assert_eq!(properties("B"), ("fraktur", "p"));
    assert_eq!(properties("C"), ("sans-serif", "i"));
    assert_eq!(properties("d"), ("roman", "p"));
    assert!(math[0].text().contains("中文"));
    assert!(
        !math[0]
            .text()
            .chars()
            .any(|c| (0x1D400..=0x1D7FF).contains(&(c as u32)))
    );
    assert!(!a.losses.iter().any(|l| l.code == "math-structure-loss"));
}

#[test]
fn mathematical_greek_symbols_are_not_folded_into_different_letters() {
    let n = |c| math::mathematical_letter(c).unwrap().0;
    assert_eq!(n('𝛳'), 'ϴ');
    assert_eq!(n('𝜖'), 'ϵ');
    assert_eq!(n('𝜗'), 'ϑ');
    assert_eq!(n('𝜘'), 'ϰ');
    assert_eq!(n('𝜙'), 'ϕ');
    assert_eq!(n('𝜚'), 'ϱ');
    assert_eq!(n('𝜛'), 'ϖ');
    assert!(math::mathematical_letter('ℏ').is_none());
    assert!(math::mathematical_letter('ℓ').is_none());
    assert!(math::mathematical_letter('中').is_none());
}

#[test]
fn run_in_body_before_and_after_heading_resets_heading_character_style() {
    let dir = TempDir::new().unwrap();
    let source = format!(
        "#set heading(numbering:\"1.1\")\n{}\nBefore body.\n#counter(heading).update(4)\n= Title <methods>\nAfter body. *Author bold*",
        run_in_recipe()
    );
    let out = render(&source, &dir);
    let a = artifact(&out);
    assert!(
        a.losses
            .iter()
            .any(|l| l.code == "run-in-leading-body-loss")
    );
    let doc = document(&a);
    let paragraphs = find(&doc, "w:p");
    let heading = paragraphs
        .into_iter()
        .find(|p| pstyle(p) == Some("Heading1"))
        .unwrap();
    let runs = find(heading, "w:r");
    assert!(heading.text().contains("Before body."));
    assert!(heading.text().contains("After body."));
    for value in ["Before body.", "After body."] {
        let r = runs.iter().find(|r| r.text().contains(value)).unwrap();
        assert_eq!(
            r.child("w:rPr").unwrap().child("w:b").unwrap().get("w:val"),
            Some("0")
        );
    }
    let title = runs.iter().find(|r| r.text() == "Title").unwrap();
    assert_eq!(
        title
            .child("w:rPr")
            .unwrap()
            .child("w:b")
            .unwrap()
            .get("w:val"),
        Some("1")
    );
    let authored = runs.iter().find(|r| r.text() == "Author bold").unwrap();
    assert_eq!(
        authored
            .child("w:rPr")
            .unwrap()
            .child("w:b")
            .unwrap()
            .get("w:val"),
        Some("1")
    );
}
