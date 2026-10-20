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
#[test]
fn canonical_managed_heading_numbering_has_native_chinese_start_exclusions_and_level_limit() {
    let dir = TempDir::new().unwrap();
    let source = weftext_engine::edit_heading_numbering(
        "= 首章 <one>\n== 子节\n=== 不编号深层\n= 排除标题 <skip>\n= 后章\n",
        &weftext_engine::HeadingNumberingIntent {
            scheme: weftext_engine::HeadingNumberingScheme::Chinese,
            start: 3,
            max_level: 2,
            exclude_labels: vec!["skip".into()],
        },
    )
    .unwrap();
    let out = render(&source, &dir);
    let a = artifact(&out);
    let doc = document(&a);
    let numbering = xml::parse(&part(&a.bytes, "word/numbering.xml")).unwrap();
    assert!(
        find(&numbering, "w:numFmt")
            .iter()
            .any(|e| e.get("w:val") == Some("chineseCounting")),
        "{:?}",
        a.losses
    );
    assert!(
        find(&numbering, "w:startOverride")
            .iter()
            .any(|e| e.get("w:val") == Some("3")),
        "{:?}",
        a.losses
    );
    for title in ["不编号深层", "排除标题"] {
        let p = find(&doc, "w:p")
            .into_iter()
            .find(|p| p.text() == title)
            .unwrap();
        assert_eq!(
            p.child("w:pPr")
                .unwrap()
                .child("w:numPr")
                .unwrap()
                .child("w:numId")
                .unwrap()
                .get("w:val"),
            Some("0")
        );
    }
    assert!(
        !a.losses.iter().any(|l| l.code == "heading-numbering-loss"),
        "{:?}",
        a.losses
    );
    let uncertain = render(
        &(source + "\n#set heading(numbering: (..x) => [未知])\n= 自定义\n"),
        &dir,
    );
    let loss = artifact(&uncertain);
    assert!(
        loss.losses
            .iter()
            .any(|l| l.code == "heading-numbering-loss")
    );
}
#[test]
fn canonical_managed_heading_numbering_with_endnotes_requires_exact_pinned_module_bytes() {
    fn headings<'a>(node: &'a RenderNode, result: &mut Vec<&'a RenderNode>) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "heading")
        {
            result.push(node);
        }
        for child in &node.children {
            headings(child, result);
        }
    }
    let dir = TempDir::new().unwrap();
    let body = "= 首章 <one>\n== 子节\n=== 不编号深层\n= 排除标题 <skip>\n= 后章\n";
    let numbered = weftext_engine::edit_heading_numbering(
        body,
        &weftext_engine::HeadingNumberingIntent {
            scheme: weftext_engine::HeadingNumberingScheme::Chinese,
            start: 3,
            max_level: 2,
            exclude_labels: vec!["skip".into()],
        },
    )
    .unwrap();
    // Keep the canonical generated header; the actual import needs the module
    // bytes admitted by this render, rather than the source-only editing API.
    let header = numbered.strip_suffix(body).unwrap();
    let source = format!(
        "{header}#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n#set math.equation(numbering: \"(1)\")\n{body}\n$ x / x = 1 $\n正文#endnote(id: \"tail\")[尾注正文] 重复#endnote-ref(\"tail\")\n#endnotes()"
    );
    let module = dir.path().join("weftext-endnotes.typ");
    let pinned = include_bytes!("../../assets/weftext-endnotes.typ");
    std::fs::write(&module, pinned).unwrap();
    let out = render(&source, &dir);
    let mut rendered_headings = vec![];
    headings(out.tree.as_ref().unwrap(), &mut rendered_headings);
    assert_eq!(rendered_headings.len(), 5);
    assert_eq!(
        rendered_headings
            .iter()
            .filter(|node| {
                node.attrs
                    .get("data-weftext-managed-numbering-pattern")
                    .is_some_and(|pattern| pattern == "一.一")
            })
            .count(),
        3
    );
    let source_hash = weftext_engine::source_hash(&source);
    let function_headings: Vec<_> = rendered_headings
        .iter()
        .filter(|node| {
            node.attrs
                .get("data-weftext-numbering-kind")
                .is_some_and(|kind| kind == "function")
        })
        .collect();
    assert_eq!(function_headings.len(), 4);
    assert!(function_headings.iter().all(|node| {
        node.attrs.get("data-weftext-managed-numbering-source-hash") == Some(&source_hash)
    }));
    let excluded = rendered_headings
        .iter()
        .find(|node| {
            node.attrs
                .get("data-weftext-label")
                .is_some_and(|label| label == "skip")
        })
        .unwrap();
    assert_eq!(excluded.attrs["data-weftext-numbered"], "false");
    assert_eq!(excluded.attrs["data-weftext-numbering-kind"], "none");
    assert!(
        excluded
            .attrs
            .keys()
            .all(|name| !name.starts_with("data-weftext-managed-numbering-"))
    );
    assert_eq!(
        rendered_headings
            .iter()
            .filter(|node| {
                node.attrs
                    .get("data-weftext-managed-numbering-visible")
                    .is_some_and(|visible| visible == "false")
            })
            .count(),
        1
    );
    let a = artifact(&out);
    let numbering = xml::parse(&part(&a.bytes, "word/numbering.xml")).unwrap();
    assert!(
        find(&numbering, "w:numFmt")
            .iter()
            .any(|e| e.get("w:val") == Some("chineseCounting"))
    );
    assert!(
        find(&numbering, "w:startOverride")
            .iter()
            .any(|e| e.get("w:val") == Some("3"))
    );
    assert!(
        !a.losses.iter().any(|l| l.code == "heading-numbering-loss"),
        "{:?}",
        a.losses
    );
    let doc = document(&a);
    for title in ["不编号深层", "排除标题"] {
        let p = find(&doc, "w:p")
            .into_iter()
            .find(|p| p.text() == title)
            .unwrap();
        assert_eq!(
            p.child("w:pPr")
                .unwrap()
                .child("w:numPr")
                .unwrap()
                .child("w:numId")
                .unwrap()
                .get("w:val"),
            Some("0")
        );
    }
    assert_eq!(find(&doc, "w:endnoteReference").len(), 1);
    assert_eq!(find(&doc, "m:oMath").len(), 1);
    let endnotes = xml::parse(&part(&a.bytes, "word/endnotes.xml")).unwrap();
    assert!(endnotes.text().contains("尾注正文"));
    assert!(!doc.text().contains("尾注正文"));

    // A same-named, still-compilable module must not inherit the bundled
    // module's certificate, even when only an extra comment changes its bytes.
    let mut changed_module = pinned.to_vec();
    changed_module.extend_from_slice(b"\n// Untrusted extra module bytes.\n");
    std::fs::write(&module, changed_module).unwrap();
    let untrusted = render(&source, &dir);
    let mut untrusted_headings = vec![];
    headings(untrusted.tree.as_ref().unwrap(), &mut untrusted_headings);
    assert_eq!(untrusted_headings.len(), 5);
    assert!(untrusted_headings.iter().all(|node| {
        node.attrs
            .keys()
            .all(|name| !name.starts_with("data-weftext-managed-numbering-"))
    }));
    let loss = artifact(&untrusted);
    let numbering = xml::parse(&part(&loss.bytes, "word/numbering.xml")).unwrap();
    assert!(
        !find(&numbering, "w:numFmt")
            .iter()
            .any(|e| e.get("w:val") == Some("chineseCounting"))
    );
    assert!(
        loss.losses
            .iter()
            .any(|l| l.code == "heading-numbering-loss"),
        "{:?}",
        loss.losses
    );
}
#[test]
fn native_notes_export_real_parts_and_repeated_reference_fields() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("weftext-endnotes.typ"),
        include_bytes!("../../assets/weftext-endnotes.typ"),
    )
    .unwrap();
    let out = render(
        "#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n正文#footnote[中文 *强调*\n\n第二段] <one>，再见 @one。#endnote(id: \"n1\")[尾注正文] 重复#endnote-ref(\"n1\")\n#endnotes()",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    assert_eq!(find(&doc, "w:footnoteReference").len(), 1, "{:?}", a.losses);
    assert_eq!(find(&doc, "w:endnoteReference").len(), 1, "{:?}", a.losses);
    let fields = find(&doc, "w:fldSimple");
    assert_eq!(
        fields
            .iter()
            .filter(|e| e.get("w:instr").is_some_and(|s| s.contains("NOTEREF")))
            .count(),
        2,
        "{:?}",
        a.losses
    );
    let foot = xml::parse(&part(&a.bytes, "word/footnotes.xml")).unwrap();
    let end = xml::parse(&part(&a.bytes, "word/endnotes.xml")).unwrap();
    assert_eq!(
        find(&foot, "w:footnote")
            .iter()
            .filter(|n| n.get("w:type").is_none())
            .count(),
        1
    );
    assert!(foot.text().contains("中文 "));
    assert!(foot.text().contains("强调"));
    assert!(
        find(&foot, "w:r")
            .iter()
            .any(|r| r.text() == "强调" && r.child("w:rPr").and_then(|p| p.child("w:b")).is_some())
    );
    assert!(foot.text().contains("第二段"));
    assert_eq!(
        find(&end, "w:endnote")
            .iter()
            .filter(|n| n.get("w:type").is_none())
            .count(),
        1
    );
    assert!(end.text().contains("尾注正文"));
    assert!(!doc.text().contains("尾注正文"));
    assert!(!doc.text().contains("第二段"));
    assert!(
        find(&doc, "w:numId")
            .iter()
            .all(|e| e.get("w:val") == Some("0")),
        "派生注释列表不能重复输出为主文档编号列表"
    );
    let settings = xml::parse(&part(&a.bytes, "word/settings.xml")).unwrap();
    for kind in ["w:footnotePr", "w:endnotePr"] {
        let section = doc
            .child("w:body")
            .and_then(|body| body.child("w:sectPr"))
            .and_then(|section| section.child(kind))
            .unwrap();
        assert_eq!(
            section.child("w:numFmt").unwrap().get("w:val"),
            Some("decimal")
        );
        assert_eq!(section.child("w:numStart").unwrap().get("w:val"), Some("1"));
        let props = settings.child(kind).unwrap();
        assert_eq!(
            props.child("w:numFmt").unwrap().get("w:val"),
            Some("decimal")
        );
        assert_eq!(
            props.child("w:numRestart").unwrap().get("w:val"),
            Some("continuous")
        );
    }
}
#[test]
fn forward_native_note_reference_keeps_word_definition_order_and_one_body() {
    let dir = TempDir::new().unwrap();
    let out = render("前向 @b。#footnote[A] <a> #footnote[B] <b>", &dir);
    let a = artifact(&out);
    let doc = document(&a);
    let notes = xml::parse(&part(&a.bytes, "word/footnotes.xml")).unwrap();
    let markers = find(&doc, "w:footnoteReference");
    assert_eq!(markers.len(), 2, "{:?}", a.losses);
    for (marker, body) in markers.iter().zip(["A", "B"]) {
        let note = find(&notes, "w:footnote")
            .into_iter()
            .find(|n| n.get("w:id") == marker.get("w:id"))
            .unwrap();
        assert!(note.text().contains(body));
    }
    let xml = String::from_utf8(part(&a.bytes, "word/document.xml")).unwrap();
    assert!(xml.find("NOTEREF").unwrap() < xml.find("w:footnoteReference").unwrap());
    assert_eq!(
        find(&notes, "w:footnote")
            .iter()
            .filter(|n| n.get("w:type").is_none())
            .count(),
        2
    );
}
#[test]
fn native_caption_and_equation_numbers_are_updateable_seq_fields() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#figure(table(columns: 1, [内容]), caption:[结果]) <tab>\n见 @tab\n#set math.equation(numbering: \"(1)\")\n$ x^2 $ <eq>\n见 @eq",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let fields = find(&doc, "w:fldSimple");
    assert!(
        fields
            .iter()
            .any(|e| e.get("w:instr").is_some_and(|s| s.contains("SEQ Table"))),
        "{:?}",
        a.losses
    );
    assert!(
        fields
            .iter()
            .any(|e| e.get("w:instr").is_some_and(|s| s.contains("SEQ Equation"))),
        "{:?}",
        a.losses
    );
    assert!(
        fields
            .iter()
            .filter(|e| e.get("w:instr").is_some_and(|s| s.contains(" REF ")))
            .count()
            >= 2,
        "{:?}",
        a.losses
    );
    assert_eq!(find(&doc, "m:oMath").len(), 1);
}
#[test]
fn mathematical_reviews_preserve_repeated_symbol_identity_and_omit_unmapped_ranges() {
    let dir = TempDir::new().unwrap();
    let source = "$ frac(x,x)+x_1^2 $";
    let out = render(source, &dir);
    let a = source.find('x').unwrap();
    let b = source[a + 1..].find('x').unwrap() + a + 1;
    let reviews = vec![
        DocxReview {
            id: "numerator".into(),
            parent: None,
            author: "甲".into(),
            date: None,
            body: "分子".into(),
            resolved: false,
            range: Some((a, a + 1)),
            whole_document: false,
            replacement: None,
            downgrade_reason: None,
            empty_body_point: false,
        },
        DocxReview {
            id: "denominator".into(),
            parent: None,
            author: "乙".into(),
            date: None,
            body: "分母".into(),
            resolved: false,
            range: Some((b, b + 1)),
            whole_document: false,
            replacement: None,
            downgrade_reason: None,
            empty_body_point: false,
        },
    ];
    let a = export_reviewed_docx(
        DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default(),
        },
        &reviews,
    )
    .unwrap();
    let doc = document(&a);
    let numerator = find(&doc, "m:num")[0];
    let denominator = find(&doc, "m:den")[0];
    assert_eq!(
        find(numerator, "w:commentRangeStart").len(),
        1,
        "{:?}",
        a.losses
    );
    assert_eq!(
        find(denominator, "w:commentRangeStart").len(),
        1,
        "{:?}",
        a.losses
    );
    let imported = build_docx_native_package(
        &a.bytes,
        include_str!("../../../../docs/design/examples/schema.authoring.json"),
        "40fa8a18-d063-435a-a962-5be0b218d453",
    )
    .unwrap();
    let comments: serde_json::Value =
        serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
    let records = comments["annotations"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert!(
        records.iter().all(|r| r["target"]["kind"] == "sourceRange"),
        "{comments}\n{:?}",
        imported.manifest.losses
    );
    let ranges: Vec<_> = records
        .iter()
        .map(|r| {
            r["target"]["anchor"]["sourceRange"]["from"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_ne!(ranges[0], ranges[1]);
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

#[test]
fn native_languages_write_complex_script_slot_and_preserve_other_template_slot() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#text(lang: \"en\", region: none)[English] #text(lang: \"ar\", region: \"SA\")[عربي] #text(lang: \"he\", region: \"IL\")[עברית]",
        &dir,
    );
    let styles = TEMPLATE_STYLES.replace(
        "w:eastAsia=\"zh-CN\"",
        "w:eastAsia=\"zh-CN\" w:bidi=\"fa-IR\"",
    );
    let bytes = template(TEMPLATE_DOC, Some(&styles));
    let a = export_docx(DocxInput {
        render: &out,
        template: Some(&bytes),
        bindings: &BTreeMap::from([("body".into(), DocxBinding::Body)]),
        options: DocxOptions::default(),
    })
    .unwrap();
    let doc = document(&a);
    for (text, tag, bidi) in [
        ("English", "en", "fa-IR"),
        ("عربي", "ar-SA", "ar-SA"),
        ("עברית", "he-IL", "he-IL"),
    ] {
        let runs = find(&doc, "w:r");
        let run = runs
            .iter()
            .find(|r| r.text() == text)
            .unwrap_or_else(|| panic!("missing {text}: {doc:?}"));
        let lang = run.child("w:rPr").and_then(|p| p.child("w:lang")).unwrap();
        assert_eq!(lang.get("w:val"), Some(tag));
        assert_eq!(lang.get("w:bidi"), Some(bidi));
    }
}

#[test]
fn imported_word_scripts_do_not_add_visible_fences() {
    let dir = TempDir::new().unwrap();
    let equation = xml::parse(br#"<m:oMath xmlns:m="m"><m:sSubSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sub><m:r><m:t>1</m:t></m:r></m:sub><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSubSup></m:oMath>"#).unwrap();
    let imported = math::import_equation(&equation, 0).unwrap();
    let a = artifact(&render(&imported.source, &dir));
    let doc = document(&a);
    assert!(find(&doc, "m:d").is_empty(), "{doc:?}");
    assert_eq!(find(&doc, "m:sSubSup").len(), 1, "{doc:?}");
    assert!(
        a.losses.iter().all(|l| l.code != "math-structure-loss"),
        "{:?}",
        a.losses
    );
}

#[test]
fn actual_native_repeating_header_rows_roundtrip_and_nonrepeating_rows_do_not_repeat() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#table(columns:2, table.header(repeat:true,[Head A],[Head B]),[Body A],[Body B])\n\n#table(columns:2,table.header(repeat:false,[Static A],[Static B]),[Body C],[Body D])",
        &dir,
    );
    let a = artifact(&out);
    let doc = document(&a);
    let tables = find(&doc, "w:tbl");
    assert_eq!(find(tables[0], "w:tblHeader").len(), 1, "{doc:?}");
    assert!(find(tables[1], "w:tblHeader").is_empty(), "{doc:?}");
    let imported = build_docx_native_package(
        &a.bytes,
        include_str!("../../../../docs/design/examples/schema.authoring.json"),
        "40fa8a18-d063-435a-a962-5be0b218d453",
    )
    .unwrap();
    let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
    assert_eq!(
        source.matches("table.header(repeat: true,").count(),
        1,
        "{source}"
    );
    let again = artifact(&render(source, &dir));
    assert_eq!(find(&document(&again), "w:tblHeader").len(), 1);
    assert!(
        !a.losses
            .iter()
            .any(|l| l.code == "table-header-repeat-loss")
    );
}

#[test]
fn certified_literal_escapes_have_atomic_review_boundaries_and_keep_display_text() {
    let dir = TempDir::new().unwrap();
    let source = "#table(columns:1,[WORD\\_TABLE\\_CELL\\_EDIT\\u{20}尾部])";
    let out = render(source, &dir);
    let from = source.find("WORD").unwrap();
    let end = source.find("\\u{20}").unwrap();
    let escape = source.find("\\_").unwrap();
    let make = |id: &str, range: (usize, usize)| DocxReview {
        id: id.into(),
        parent: None,
        author: "reviewer".into(),
        date: None,
        body: id.into(),
        resolved: false,
        range: Some(range),
        whole_document: false,
        replacement: None,
        downgrade_reason: None,
        empty_body_point: false,
    };
    let a = export_reviewed_docx(
        DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default(),
        },
        &[
            make("whole", (from, end)),
            make("atomic", (escape, escape + 2)),
            make("inside-escape", (escape + 1, escape + 2)),
        ],
    )
    .unwrap();
    let doc = document(&a);
    assert_eq!(doc.text(), "WORD_TABLE_CELL_EDIT 尾部");
    let comments = xml::parse(&part(&a.bytes, "word/comments.xml")).unwrap();
    let records = find(&comments, "w:comment");
    assert_eq!(records.len(), 2, "{:?}", a.losses);
    let id = records
        .iter()
        .find(|r| r.text() == "atomic")
        .unwrap()
        .get("w:id")
        .unwrap();
    fn anchored(element: &Element, id: &str, active: &mut bool, text: &mut String) {
        if element.name == "w:commentRangeStart" && element.get("w:id") == Some(id) {
            *active = true;
        }
        if element.name == "w:commentRangeEnd" && element.get("w:id") == Some(id) {
            *active = false;
        }
        if element.name == "w:t" && *active {
            text.push_str(&element.text());
        }
        for child in element.children.iter().filter_map(Xml::element) {
            anchored(child, id, active, text);
        }
    }
    let mut selected = String::new();
    anchored(&doc, id, &mut false, &mut selected);
    assert_eq!(selected, "_");
    assert_eq!(
        a.losses
            .iter()
            .filter(|l| l.code == "word-review-unmapped")
            .count(),
        1
    );
    assert!(
        a.losses
            .iter()
            .any(|l| l.location.as_deref() == Some("inside-escape"))
    );
}

#[test]
fn word_undeclared_plain_insertions_and_unique_empty_body_keep_exact_points() {
    let dir = TempDir::new().unwrap();
    for (body, inserted, before, after, empty) in [
        (
            r#"<w:r><w:t>原文😀</w:t></w:r><w:ins w:id="1"><w:r><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:del w:id="2"><w:r><w:delText>删除</w:delText></w:r></w:del>"#,
            "新增",
            "原文😀后文删除",
            "原文😀新增后文删除",
            false,
        ),
        (
            r#"<w:r><w:t>前文</w:t></w:r><w:del w:id="1"><w:r><w:delText>旧字</w:delText></w:r></w:del><w:ins w:id="2"><w:r><w:t>新字</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r>"#,
            "新字",
            "前文旧字后文",
            "前文旧字新字后文",
            false,
        ),
        (
            r#"<w:ins w:id="1"><w:r><w:t>全新正文😀</w:t></w:r></w:ins>"#,
            "全新正文😀",
            "",
            "全新正文😀",
            true,
        ),
    ] {
        let bytes = undeclared_insertion_word(&format!("<w:p>{body}</w:p>"), None);
        let imported = build_docx_native_package(
            &bytes,
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap();
        assert_eq!(imported.files["document/original.docx"], bytes);
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        assert!(!source.contains("#text("), "{source}");
        let annotations: serde_json::Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let mut point = None;
        let reviews: Vec<_> = annotations["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| {
                assert_eq!(record["target"]["kind"], "sourceRange", "{annotations}");
                assert_eq!(record["suggestion"]["requiresReconfirmation"], false);
                let range = &record["target"]["anchor"]["sourceRange"];
                let from = range["from"].as_u64().unwrap() as usize;
                let to = range["to"].as_u64().unwrap() as usize;
                assert_eq!(range["raw"], &source[from..to]);
                assert_eq!(range["utf16From"], source[..from].encode_utf16().count());
                assert_eq!(range["utf16To"], source[..to].encode_utf16().count());
                let insertion = record["wordImport"]["revisionKind"] == "insert";
                if insertion {
                    assert_eq!(from, to);
                    point = Some(from);
                    if empty {
                        let slot = weftext_engine::inspect_editor(source)
                            .body_insertion
                            .unwrap();
                        assert!(slot.writable);
                        assert_eq!(from, source.len());
                        assert_eq!(slot.from, from);
                    } else {
                        assert_eq!(from, source.find("后文").unwrap());
                    }
                }
                DocxReview {
                    id: record["annotationId"].as_str().unwrap().into(),
                    parent: None,
                    author: "作者".into(),
                    date: None,
                    body: "待审文字".into(),
                    resolved: false,
                    range: Some((from, to)),
                    whole_document: false,
                    replacement: Some(record["suggestion"]["replacement"].as_str().unwrap().into()),
                    downgrade_reason: None,
                    empty_body_point: empty && insertion,
                }
            })
            .collect();
        assert!(
            !imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-insertion-boundary")
        );
        // Language remains unassigned; retaining this loss is not a claim to
        // have recovered a Word language from the plain source boundary.
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|l| l.code == "word-language-missing")
        );
        let output = render(source, &dir);
        assert_eq!(
            output
                .products
                .as_ref()
                .unwrap()
                .body
                .as_deref()
                .unwrap()
                .trim(),
            before
        );
        let exported = export_reviewed_docx(
            DocxInput {
                render: &output,
                template: None,
                bindings: &BTreeMap::new(),
                options: DocxOptions::default(),
            },
            &reviews,
        )
        .unwrap();
        let document = document(&exported);
        assert_eq!(
            find(&document, "w:ins")
                .iter()
                .map(|n| n.text())
                .collect::<String>(),
            inserted
        );
        assert!(
            !exported
                .losses
                .iter()
                .any(|l| l.code == "word-review-unmapped"),
            "{:?}",
            exported.losses
        );
        let mut applied = source.to_owned();
        applied.insert_str(point.unwrap(), inserted);
        let applied = render(&applied, &dir);
        assert_eq!(
            applied
                .products
                .as_ref()
                .unwrap()
                .body
                .as_deref()
                .unwrap()
                .trim(),
            after
        );
    }
}

fn undeclared_insertion_word(body: &str, styles: Option<&str>) -> Vec<u8> {
    let mut package = Package::load(None).unwrap();
    package.parts.remove("word/styles.xml");
    package.rels.children.retain(|child| {
        !child.element().is_some_and(|e| {
            e.get("Type")
                == Some(
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles",
                )
        })
    });
    if let Some(styles) = styles {
        package
            .parts
            .insert("word/styles.xml".into(), styles.as_bytes().to_vec());
        package.ensure_rel("styles", "styles.xml");
    }
    package.parts.insert("word/document.xml".into(), format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr/></w:body></w:document>"#,
    ).into_bytes());
    package.finish().unwrap()
}

#[test]
fn word_insertions_do_not_invent_undeclared_context_or_empty_body_points() {
    let insertion = r#"<w:ins w:id="1"><w:r><w:t>新增</w:t></w:r></w:ins>"#;
    let next = r#"<w:r><w:t>后文</w:t></w:r>"#;
    let partial = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-US"/></w:rPr></w:rPrDefault></w:docDefaults></w:styles>"#;
    for (case, body, styles) in [
        ("declared-partial-script", format!("<w:p>{insertion}{next}</w:p>"), Some(partial)),
        ("missing-referenced-style", format!(r#"<w:p><w:pPr><w:pStyle w:val="Missing"/></w:pPr>{insertion}{next}</w:p>"#), None),
        ("different-bold", format!(r#"<w:p>{insertion}<w:r><w:rPr><w:b/></w:rPr><w:t>后文</w:t></w:r></w:p>"#), None),
        ("cross-paragraph", format!("<w:p>{insertion}</w:p><w:p>{next}</w:p>"), None),
        ("second-empty-paragraph", format!("<w:p/><w:p>{insertion}</w:p>"), None),
        ("two-insertions", format!("<w:p>{insertion}{}</w:p>", insertion.replace("id=\"1\"", "id=\"2\"")), None),
        ("empty-body-bold", r#"<w:p><w:ins w:id="1"><w:r><w:rPr><w:b/></w:rPr><w:t>新增</w:t></w:r></w:ins></w:p>"#.into(), None),
        ("empty-body-declared-language", r#"<w:p><w:ins w:id="1"><w:r><w:rPr><w:lang w:eastAsia="zh-CN"/></w:rPr><w:t>新增</w:t></w:r></w:ins></w:p>"#.into(), None),
        ("empty-body-latin-unused-declared-slot", r#"<w:p><w:ins w:id="1"><w:r><w:rPr><w:lang w:eastAsia="zh-CN"/></w:rPr><w:t>added</w:t></w:r></w:ins></w:p>"#.into(), None),
        ("empty-body-latin-unused-invalid-slot", r#"<w:p><w:ins w:id="1"><w:r><w:rPr><w:lang w:eastAsia="invalid_tag"/></w:rPr><w:t>added</w:t></w:r></w:ins></w:p>"#.into(), None),
        ("empty-cell", format!("<w:tbl><w:tr><w:tc><w:p>{insertion}</w:p></w:tc></w:tr></w:tbl>"), None),
    ] {
        let imported = build_docx_native_package(
            &undeclared_insertion_word(&body, styles),
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        ).unwrap();
        let annotations: serde_json::Value = serde_json::from_slice(
            &imported.files["document/weftext.annotations.json"],
        ).unwrap();
        for record in annotations["annotations"].as_array().unwrap() {
            assert_eq!(record["target"]["kind"], "wholeResource", "{case}: {annotations}");
            assert_eq!(record["suggestion"]["requiresReconfirmation"], true, "{case}");
        }
        assert!(imported.manifest.losses.iter().any(|l| l.code == "word-insertion-boundary"), "{case}");
    }
}

#[test]
fn word_insertions_use_adjacent_native_text_boundaries_and_reject_context_changes() {
    fn word(body: &str) -> Vec<u8> {
        word_with_styles(body, "")
    }
    fn word_with_styles(body: &str, extra_styles: &str) -> Vec<u8> {
        let doc = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p>{body}</w:p></w:body></w:document>"#
        );
        let styles = format!(
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-US" w:eastAsia="zh-CN"/></w:rPr></w:rPrDefault></w:docDefaults>{extra_styles}</w:styles>"#
        );
        let bytes = template(&doc, Some(&styles));
        let mut package = Package::load(Some(&bytes)).unwrap();
        package.rels.push(
            Element::new("Relationship")
                .attr("Id", "review-link")
                .attr("Type", format!("{}hyperlink", package::REL))
                .attr("Target", "https://example.invalid/review")
                .attr("TargetMode", "External"),
        );
        package.finish().unwrap()
    }
    let dir = TempDir::new().unwrap();
    let bytes = word(
        r#"<w:r><w:t>原文</w:t></w:r><w:r><w:t>😀</w:t></w:r><w:ins w:id="1" w:author="作者"><w:r><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:del w:id="2" w:author="作者"><w:r><w:delText>删除</w:delText></w:r></w:del>"#,
    );
    let imported = build_docx_native_package(
        &bytes,
        include_str!("../../../../docs/design/examples/schema.authoring.json"),
        "40fa8a18-d063-435a-a962-5be0b218d453",
    )
    .unwrap();
    let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
    let annotations: serde_json::Value =
        serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
    let records = annotations["annotations"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let insertion = records
        .iter()
        .find(|record| record["wordImport"]["revisionKind"] == "insert")
        .unwrap();
    let point = source.find("后文").unwrap();
    let insertion_range = &insertion["target"]["anchor"]["sourceRange"];
    assert_eq!(insertion["target"]["kind"], "sourceRange");
    assert_eq!(insertion_range["from"], point);
    assert_eq!(insertion_range["to"], point);
    assert_eq!(insertion_range["raw"], "");
    assert_eq!(
        insertion_range["utf16From"],
        source[..point].encode_utf16().count()
    );
    assert_eq!(insertion["suggestion"]["requiresReconfirmation"], false);
    assert!(
        !imported
            .manifest
            .losses
            .iter()
            .any(|loss| loss.code == "word-insertion-boundary"),
        "{:?}",
        imported.manifest.losses
    );
    let reviews: Vec<_> = records
        .iter()
        .map(|record| {
            assert_eq!(record["target"]["kind"], "sourceRange");
            assert_eq!(record["suggestion"]["requiresReconfirmation"], false);
            let range = &record["target"]["anchor"]["sourceRange"];
            let from = range["from"].as_u64().unwrap() as usize;
            let to = range["to"].as_u64().unwrap() as usize;
            if record["wordImport"]["revisionKind"] == "delete" {
                assert_eq!(&source[from..to], "删除");
            }
            DocxReview {
                id: record["annotationId"].as_str().unwrap().into(),
                parent: None,
                author: "作者".into(),
                date: None,
                body: "待审文字".into(),
                resolved: false,
                range: Some((from, to)),
                whole_document: false,
                replacement: Some(record["suggestion"]["replacement"].as_str().unwrap().into()),
                downgrade_reason: None,
                empty_body_point: false,
            }
        })
        .collect();
    let out = render(source, &dir);
    assert_eq!(
        out.products
            .as_ref()
            .unwrap()
            .body
            .as_deref()
            .unwrap()
            .trim(),
        "原文😀后文删除"
    );
    let a = export_reviewed_docx(
        DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default(),
        },
        &reviews,
    )
    .unwrap();
    let doc = document(&a);
    assert_eq!(find(&doc, "w:ins").len(), 1, "{:?}", a.losses);
    assert_eq!(find(&doc, "w:ins")[0].text(), "新增");
    assert_eq!(
        find(find(&doc, "w:ins")[0], "w:r")[0]
            .child("w:rPr")
            .and_then(|pr| pr.child("w:lang"))
            .and_then(|lang| lang.get("w:val")),
        Some("zh-CN")
    );
    assert_eq!(find(&doc, "w:del").len(), 1, "{:?}", a.losses);
    assert_eq!(find(&doc, "w:del")[0].text(), "删除");
    assert_eq!(find(&doc, "w:delText")[0].text(), "删除");
    assert!(
        !a.losses
            .iter()
            .any(|loss| loss.code == "word-review-unmapped"),
        "{:?}",
        a.losses
    );
    let mut applied = source.to_owned();
    applied.insert_str(point, "新增");
    assert_eq!(
        applied.matches("#text(lang:").count(),
        source.matches("#text(lang:").count()
    );
    let applied_out = render(&applied, &dir);
    assert_eq!(
        applied_out
            .products
            .as_ref()
            .unwrap()
            .body
            .as_deref()
            .unwrap()
            .trim(),
        "原文😀新增后文删除"
    );
    let accepted = artifact(&applied_out);
    let accepted_doc = document(&accepted);
    let inserted_run = find(&accepted_doc, "w:r")
        .into_iter()
        .find(|run| run.text().contains("新增"))
        .unwrap();
    assert_eq!(
        inserted_run
            .child("w:rPr")
            .and_then(|pr| pr.child("w:lang"))
            .and_then(|lang| lang.get("w:val")),
        Some("zh-CN")
    );

    for (case, body, extra_styles) in [
        (
            "different-language",
            r#"<w:r><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:t>added</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:r><w:t>added</w:t></w:r>"#,
            "",
        ),
        (
            "different-bold",
            r#"<w:r><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:rPr><w:b/></w:rPr><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>新增</w:t></w:r>"#,
            "",
        ),
        (
            "hyperlink-boundary",
            r#"<w:hyperlink r:id="review-link"><w:r><w:t>前文</w:t></w:r></w:hyperlink><w:ins w:id="1"><w:r><w:t>新增</w:t></w:r></w:ins><w:hyperlink r:id="review-link"><w:r><w:t>后文</w:t></w:r></w:hyperlink><w:r><w:t>新增</w:t></w:r>"#,
            "",
        ),
        (
            "invalid-language-must-not-collapse-to-none",
            r#"<w:r><w:rPr><w:lang w:eastAsia="invalid_tag"/></w:rPr><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:rPr><w:lang w:eastAsia="invalid_tag"/></w:rPr><w:t>新增</w:t></w:r></w:ins><w:r><w:rPr><w:lang w:eastAsia="invalid_tag"/></w:rPr><w:t>后文</w:t></w:r><w:r><w:t>新增</w:t></w:r>"#,
            "",
        ),
        (
            "different-script-subtags-must-not-collapse-to-region",
            r#"<w:r><w:rPr><w:lang w:eastAsia="zh-Hans-CN"/></w:rPr><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:rPr><w:lang w:eastAsia="zh-Hant-CN"/></w:rPr><w:t>新增</w:t></w:r></w:ins><w:r><w:rPr><w:lang w:eastAsia="zh-Hans-CN"/></w:rPr><w:t>后文</w:t></w:r><w:r><w:t>新增</w:t></w:r>"#,
            "",
        ),
        (
            "multiple-region-subtags-must-not-certify-a-language",
            r#"<w:r><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:rPr><w:lang w:eastAsia="zh-CN-CN"/></w:rPr><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:r><w:t>新增</w:t></w:r>"#,
            "",
        ),
        (
            "missing-paragraph-style-cannot-certify-fallback",
            r#"<w:pPr><w:pStyle w:val="Missing"/></w:pPr><w:r><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:r><w:t>新增</w:t></w:r>"#,
            "",
        ),
        (
            "missing-default-character-style-base-cannot-certify-fallback",
            r#"<w:r><w:t>前文</w:t></w:r><w:ins w:id="1"><w:r><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:r><w:t>新增</w:t></w:r>"#,
            r#"<w:style w:type="character" w:default="1" w:styleId="BrokenDefault"><w:basedOn w:val="MissingCharacterBase"/></w:style>"#,
        ),
    ] {
        let imported = build_docx_native_package(
            &word_with_styles(body, extra_styles),
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap();
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        render(source, &dir);
        let annotations: serde_json::Value =
            serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
        let records = annotations["annotations"].as_array().unwrap();
        assert_eq!(records.len(), 1, "{case}: {annotations}");
        assert_eq!(records[0]["target"]["kind"], "wholeResource", "{case}");
        assert_eq!(
            records[0]["suggestion"]["requiresReconfirmation"], true,
            "{case}"
        );
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|loss| loss.code == "word-insertion-boundary"),
            "{case}: {:?}",
            imported.manifest.losses
        );
    }
}

#[test]
fn word_body_level_math_comment_starts_keep_whole_fraction_and_symbol_identity() {
    fn word(duplicate_start: bool) -> Vec<u8> {
        let mut package = Package::load(None).unwrap();
        let duplicate = if duplicate_start {
            r#"<w:commentRangeStart w:id="1"/>"#
        } else {
            ""
        };
        let doc = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body>
<w:commentRangeStart w:id="1"/>{duplicate}<w:commentRangeStart w:id="2"/>
<w:p><m:oMath><m:f><m:num><w:commentRangeStart w:id="3"/><m:r><m:t>x</m:t></m:r><w:commentRangeEnd w:id="3"/><m:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="3"/></m:r></m:num><m:den><w:commentRangeStart w:id="4"/><m:r><m:t>x</m:t></m:r><w:commentRangeEnd w:id="4"/><m:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="4"/></m:r></m:den></m:f><w:commentRangeEnd w:id="2"/><m:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="2"/></m:r><m:r><m:t>+y</m:t></m:r></m:oMath><w:commentRangeEnd w:id="1"/><w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="1"/></w:r></w:p>
</w:body></w:document>"#
        );
        package.set_xml("word/document.xml", &xml::parse(doc.as_bytes()).unwrap());
        let mut comments = Element::new("w:comments").attr("xmlns:w", package::W);
        for (id, body) in [
            ("1", "whole-equation"),
            ("2", "fraction"),
            ("3", "numerator"),
            ("4", "denominator"),
        ] {
            let mut comment = Element::new("w:comment")
                .attr("w:id", id)
                .attr("w:author", "作者");
            let mut paragraph = Element::new("w:p");
            let mut run = Element::new("w:r");
            let mut text = Element::new("w:t");
            text.children.push(Xml::Text(body.into()));
            run.push(text);
            paragraph.push(run);
            comment.push(paragraph);
            comments.push(comment);
        }
        package.set_xml("word/comments.xml", &comments);
        package.ensure_rel("comments", "comments.xml");
        package.content_type(
            "/word/comments.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
        );
        package.finish().unwrap()
    }
    let dir = TempDir::new().unwrap();
    let bytes = word(false);
    let imported = build_docx_native_package(
        &bytes,
        include_str!("../../../../docs/design/examples/schema.authoring.json"),
        "40fa8a18-d063-435a-a962-5be0b218d453",
    )
    .unwrap();
    assert_eq!(imported.files["document/original.docx"], bytes);
    let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
    let annotations: serde_json::Value =
        serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
    let records = annotations["annotations"].as_array().unwrap();
    assert_eq!(records.len(), 4);
    let record = |body: &str| {
        records
            .iter()
            .find(|record| record["body"] == body)
            .unwrap()
    };
    let range = |body: &str| {
        let record = record(body);
        assert_eq!(record["target"]["kind"], "sourceRange", "{annotations}");
        let range = &record["target"]["anchor"]["sourceRange"];
        let from = range["from"].as_u64().unwrap() as usize;
        let to = range["to"].as_u64().unwrap() as usize;
        assert_eq!(range["raw"], &source[from..to]);
        (from, to)
    };
    let whole = range("whole-equation");
    let fraction = range("fraction");
    let numerator = range("numerator");
    let denominator = range("denominator");
    let whole_raw = &source[whole.0..whole.1];
    assert!(
        whole_raw.starts_with('$') && whole_raw.ends_with('$'),
        "{whole_raw}"
    );
    assert!(
        whole_raw
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .contains("+y"),
        "{whole_raw}"
    );
    let fraction_raw = &source[fraction.0..fraction.1];
    assert!(fraction_raw.starts_with("frac("), "{fraction_raw}");
    assert!(fraction_raw.ends_with(')'), "{fraction_raw}");
    assert!(
        !fraction_raw.contains('+') && !fraction_raw.contains('y'),
        "{fraction_raw}"
    );
    assert_eq!(&source[numerator.0..numerator.1], "x");
    assert_eq!(&source[denominator.0..denominator.1], "x");
    assert!(numerator.1 <= denominator.0);
    assert!(whole.0 < fraction.0 && fraction.1 < whole.1);
    assert!(fraction.0 < numerator.0 && denominator.1 < fraction.1);
    let reviews: Vec<_> = records
        .iter()
        .map(|record| {
            let body = record["body"].as_str().unwrap();
            DocxReview {
                id: record["annotationId"].as_str().unwrap().into(),
                parent: None,
                author: "作者".into(),
                date: None,
                body: body.into(),
                resolved: false,
                range: Some(range(body)),
                whole_document: false,
                replacement: None,
                downgrade_reason: None,
                empty_body_point: false,
            }
        })
        .collect();
    let out = render(source, &dir);
    let a = export_reviewed_docx(
        DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default(),
        },
        &reviews,
    )
    .unwrap();
    let doc = document(&a);
    assert_eq!(find(&doc, "m:f").len(), 1, "{:?}", a.losses);
    assert_eq!(find(&doc, "w:commentRangeStart").len(), 4, "{:?}", a.losses);
    assert_eq!(find(&doc, "w:commentRangeEnd").len(), 4, "{:?}", a.losses);
    let comments = xml::parse(&part(&a.bytes, "word/comments.xml")).unwrap();
    assert_eq!(find(&comments, "w:comment").len(), 4, "{:?}", a.losses);
    assert!(
        !a.losses
            .iter()
            .any(|loss| loss.code == "word-review-unmapped"),
        "{:?}",
        a.losses
    );

    let duplicate = word(true);
    let imported = build_docx_native_package(
        &duplicate,
        include_str!("../../../../docs/design/examples/schema.authoring.json"),
        "40fa8a18-d063-435a-a962-5be0b218d453",
    )
    .unwrap();
    assert_eq!(imported.files["document/original.docx"], duplicate);
    let annotations: serde_json::Value =
        serde_json::from_slice(&imported.files["document/weftext.annotations.json"]).unwrap();
    let records = annotations["annotations"].as_array().unwrap();
    assert_eq!(records.len(), 4);
    for record in records {
        assert_eq!(
            record["target"]["kind"],
            if record["body"] == "whole-equation" {
                "wholeResource"
            } else {
                "sourceRange"
            },
            "{annotations}"
        );
    }
    assert!(
        imported
            .manifest
            .losses
            .iter()
            .any(|loss| loss.code == "word-comment-unlocated"),
        "{:?}",
        imported.manifest.losses
    );
}

#[test]
fn word_local_heading_styles_use_outline_properties_and_reject_uncertain_chains() {
    fn word(body: &str, extra_styles: &str) -> Vec<u8> {
        word_with_default(body, extra_styles, "")
    }
    fn word_with_default(body: &str, extra_styles: &str, default_properties: &str) -> Vec<u8> {
        let doc = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
        );
        let styles = format!(
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-US" w:eastAsia="zh-CN"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="a"><w:name w:val="Normal"/>{default_properties}</w:style>{extra_styles}</w:styles>"#
        );
        template(&doc, Some(&styles))
    }
    fn headings<'a>(node: &'a RenderNode, result: &mut Vec<&'a RenderNode>) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|role| role == "heading")
        {
            result.push(node);
        }
        for child in &node.children {
            headings(child, result);
        }
    }
    let dir = TempDir::new().unwrap();
    let styles = r#"
<w:style w:type="paragraph" w:styleId="1"><w:name w:val="heading 1"/><w:basedOn w:val="a"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="2"><w:name w:val="heading 2"/><w:basedOn w:val="a"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="3"><w:name w:val="heading 3"/><w:basedOn w:val="a"/><w:pPr><w:outlineLvl w:val="2"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="child"><w:name w:val="Custom child"/><w:basedOn w:val="1"/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="a"/></w:style>"#;
    let body = r#"
<w:p><w:pPr><w:pStyle w:val="1"/><w:numPr><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>HeadingOne</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="2"/></w:pPr><w:r><w:t>HeadingTwo</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="3"/></w:pPr><w:r><w:t>HeadingThree</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="child"/></w:pPr><w:r><w:t>InheritedHeading</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="1"/><w:outlineLvl w:val="9"/></w:pPr><w:r><w:t>ExplicitBodyOne</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="2"/><w:outlineLvl w:val="9"/></w:pPr><w:r><w:t>ExplicitBodyTwo</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>NamedOnly</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading1"/><w:numPr><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>OrdinaryList</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading1"/><w:numPr><w:numId w:val="0"/></w:numPr></w:pPr><w:r><w:t>OrdinaryZero</w:t></w:r></w:p>"#;
    let bytes = word(body, styles);
    let imported = build_docx_native_package(
        &bytes,
        include_str!("../../../../docs/design/examples/schema.authoring.json"),
        "40fa8a18-d063-435a-a962-5be0b218d453",
    )
    .unwrap();
    assert_eq!(imported.files["document/original.docx"], bytes);
    let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
    let out = render(source, &dir);
    let mut actual = vec![];
    headings(out.tree.as_ref().unwrap(), &mut actual);
    assert_eq!(
        actual
            .iter()
            .map(|heading| heading.attrs["data-weftext-level"].as_str())
            .collect::<Vec<_>>(),
        ["1", "2", "3", "1"],
        "{source}"
    );
    assert!(
        actual
            .iter()
            .all(|heading| heading.attrs["data-weftext-numbered"] == "false")
    );
    for text in [
        "ExplicitBodyOne",
        "ExplicitBodyTwo",
        "NamedOnly",
        "OrdinaryZero",
    ] {
        let line = source.lines().find(|line| line.contains(text)).unwrap();
        assert!(!line.starts_with('=') && !line.starts_with("- "), "{line}");
    }
    assert!(
        source
            .lines()
            .any(|line| line.starts_with("- ") && line.contains("OrdinaryList"))
    );
    assert!(!source.contains("#set heading(numbering:"), "{source}");
    for code in ["word-heading-numbering", "word-list-numbering"] {
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|loss| loss.code == code),
            "{code}: {:?}",
            imported.manifest.losses
        );
    }

    for (case, extra_styles, paragraph_properties) in [
        (
            "cycle",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:basedOn w:val="Other"/></w:style><w:style w:type="paragraph" w:styleId="Other"><w:basedOn w:val="Unsafe"/></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/>"#,
        ),
        (
            "missing-base",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:basedOn w:val="Missing"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/>"#,
        ),
        (
            "wrong-style-type",
            r#"<w:style w:type="character" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/>"#,
        ),
        (
            "duplicate-style-id",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/>"#,
        ),
        (
            "invalid-style-outline",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="10"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/>"#,
        ),
        (
            "duplicate-style-outline",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="0"/><w:outlineLvl w:val="1"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/>"#,
        ),
        (
            "invalid-direct-outline-does-not-fall-back",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/><w:outlineLvl w:val="bad"/>"#,
        ),
        (
            "duplicate-direct-outline-does-not-fall-back",
            r#"<w:style w:type="paragraph" w:styleId="Unsafe"><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>"#,
            r#"<w:pStyle w:val="Unsafe"/><w:outlineLvl w:val="0"/><w:outlineLvl w:val="1"/>"#,
        ),
    ] {
        let body = format!(
            "<w:p><w:pPr>{paragraph_properties}</w:pPr><w:r><w:t>UnsafeHeading</w:t></w:r></w:p>"
        );
        let bytes = word(&body, extra_styles);
        let imported = build_docx_native_package(
            &bytes,
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap();
        assert_eq!(imported.files["document/original.docx"], bytes, "{case}");
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        let out = render(source, &dir);
        let mut actual = vec![];
        headings(out.tree.as_ref().unwrap(), &mut actual);
        assert!(actual.is_empty(), "{case}: {source}");
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|loss| loss.code == "word-heading-style"),
            "{case}: {:?}",
            imported.manifest.losses
        );
        assert!(
            out.products
                .as_ref()
                .unwrap()
                .body
                .as_ref()
                .unwrap()
                .contains("UnsafeHeading")
        );
    }
    for (case, paragraph_properties, default_properties) in [
        (
            "empty-style-value-must-not-fall-back-to-default-heading",
            r#"<w:pPr><w:pStyle w:val=""/></w:pPr>"#,
            r#"<w:pPr><w:outlineLvl w:val="0"/></w:pPr>"#,
        ),
        (
            "missing-style-value-must-not-fall-back-to-default-heading",
            r#"<w:pPr><w:pStyle/></w:pPr>"#,
            r#"<w:pPr><w:outlineLvl w:val="0"/></w:pPr>"#,
        ),
        (
            "duplicate-paragraph-properties-must-not-use-first-outline",
            r#"<w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:pPr><w:outlineLvl w:val="9"/></w:pPr>"#,
            "",
        ),
    ] {
        let body =
            format!("<w:p>{paragraph_properties}<w:r><w:t>UnsafeParagraph</w:t></w:r></w:p>");
        let bytes = word_with_default(&body, "", default_properties);
        let imported = build_docx_native_package(
            &bytes,
            include_str!("../../../../docs/design/examples/schema.authoring.json"),
            "40fa8a18-d063-435a-a962-5be0b218d453",
        )
        .unwrap();
        assert_eq!(imported.files["document/original.docx"], bytes, "{case}");
        let source = std::str::from_utf8(&imported.files["document/document.typ"]).unwrap();
        let out = render(source, &dir);
        let mut actual = vec![];
        headings(out.tree.as_ref().unwrap(), &mut actual);
        assert!(actual.is_empty(), "{case}: {source}");
        assert!(
            imported
                .manifest
                .losses
                .iter()
                .any(|loss| loss.code == "word-heading-style"),
            "{case}: {:?}",
            imported.manifest.losses
        );
        assert!(
            out.products
                .as_ref()
                .unwrap()
                .body
                .as_ref()
                .unwrap()
                .contains("UnsafeParagraph")
        );
    }
}

#[test]
fn readonly_native_literal_discussions_keep_escaped_and_wrapped_display_scopes() {
    fn texts<'a>(node: &'a RenderNode, result: &mut Vec<&'a RenderNode>) {
        if node.kind == "text" {
            result.push(node);
        }
        for child in &node.children {
            texts(child, result);
        }
    }
    fn review(id: &str, range: (usize, usize)) -> DocxReview {
        DocxReview {
            id: id.into(),
            parent: None,
            author: "reviewer".into(),
            date: None,
            body: id.into(),
            resolved: false,
            range: Some(range),
            whole_document: false,
            replacement: None,
            downgrade_reason: None,
            empty_body_point: false,
        }
    }
    fn anchored(node: &Element, id: &str, active: &mut bool, text: &mut String) {
        if node.name == "w:commentRangeStart" && node.get("w:id") == Some(id) {
            *active = true;
        }
        if node.name == "w:commentRangeEnd" && node.get("w:id") == Some(id) {
            *active = false;
        }
        if node.name == "w:t" && *active {
            text.push_str(&node.text());
        }
        for child in node.children.iter().filter_map(Xml::element) {
            anchored(child, id, active, text);
        }
    }
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("weftext-endnotes.typ"),
        include_bytes!("../../assets/weftext-endnotes.typ"),
    )
    .unwrap();
    let source = r#"#import "weftext-endnotes.typ": endnote, endnote-ref, endnotes
#table(columns: 1, [#text(lang: "en", region: "US")[WORD\_TABLE\_CELL\_EDIT\u{20}tail]])

#link("https://example.invalid/readonly-review")[#text(lang: "en", region: "US")[LINK\_TEXT\u{20}]#text(lang: "zh", region: "CN")[链接尾部]]

- #text(lang: "en", region: "US")[LIST\_TEXT\u{20}]#text(lang: "zh", region: "CN")[列表尾部]

+ #text(lang: "en", region: "US")[NUMBERED\_TEXT\u{20}]#text(lang: "zh", region: "CN")[有序尾部]
"#;
    let out = render(source, &dir);
    assert!(out.dependencies.iter().any(|dependency| {
        dependency.status == "loaded" && dependency.path.ends_with("weftext-endnotes.typ")
    }));
    let mut nodes = vec![];
    texts(out.tree.as_ref().unwrap(), &mut nodes);
    let escape = source.find("\\_").unwrap();
    let actual_escape = nodes
        .iter()
        .find(|node| {
            node.origin
                .as_ref()
                .is_some_and(|origin| origin.from == escape && origin.to == escape + 2)
        })
        .unwrap();
    let origin = actual_escape.origin.as_ref().unwrap();
    assert!(!actual_escape.writable, "{actual_escape:?}");
    assert!(origin.is_main);
    assert_eq!(origin.syntax_kind, "Escape");
    assert_eq!(origin.raw, "\\_");
    assert_eq!(&source[origin.from..origin.to], origin.raw);
    assert_eq!(actual_escape.text.as_deref(), Some("_"));
    assert_eq!(
        nodes
            .iter()
            .filter(|node| {
                node.origin.as_ref().is_some_and(|candidate| {
                    candidate.path == origin.path
                        && candidate.from == origin.from
                        && candidate.to == origin.to
                })
            })
            .count(),
        1
    );
    let targets = [
        ("table-word", "WORD", "EDIT", "WORD_TABLE_CELL_EDIT"),
        ("hyperlink", "LINK", "链接尾部", "LINK_TEXT 链接尾部"),
        ("list", "LIST", "列表尾部", "LIST_TEXT 列表尾部"),
        (
            "numbered-list",
            "NUMBERED",
            "有序尾部",
            "NUMBERED_TEXT 有序尾部",
        ),
    ];
    let mut reviews: Vec<_> = targets
        .iter()
        .map(|(id, first, last, _)| {
            let from = source.find(*first).unwrap();
            let to = from + source[from..].find(*last).unwrap() + last.len();
            review(id, (from, to))
        })
        .collect();
    reviews.push(review("atomic-escape", (escape, escape + 2)));
    reviews.push(review("inside-escape", (escape + 1, escape + 2)));
    let a = export_reviewed_docx(
        DocxInput {
            render: &out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default(),
        },
        &reviews,
    )
    .unwrap();
    let doc = document(&a);
    assert_eq!(find(&doc, "w:tbl").len(), 1);
    assert_eq!(find(&doc, "w:hyperlink").len(), 1);
    let comments = xml::parse(&part(&a.bytes, "word/comments.xml")).unwrap();
    let records = find(&comments, "w:comment");
    assert_eq!(records.len(), 5, "{:?}", a.losses);
    for (body, expected) in targets
        .iter()
        .map(|(id, _, _, expected)| (*id, *expected))
        .chain([("atomic-escape", "_")])
    {
        let id = records
            .iter()
            .find(|record| record.text() == body)
            .unwrap()
            .get("w:id")
            .unwrap();
        let mut selected = String::new();
        anchored(&doc, id, &mut false, &mut selected);
        assert_eq!(selected, expected, "{body}: {:?}", a.losses);
    }
    let unmapped: Vec<_> = a
        .losses
        .iter()
        .filter(|loss| loss.code == "word-review-unmapped")
        .collect();
    assert_eq!(unmapped.len(), 1, "{:?}", a.losses);
    assert_eq!(unmapped[0].location.as_deref(), Some("inside-escape"));

    let repeated = r#"#import "weftext-endnotes.typ": endnote, endnote-ref, endnotes
#for i in range(2) [REPEAT\_SOURCE]
"#;
    let repeated_out = render(repeated, &dir);
    let from = repeated.find("REPEAT").unwrap();
    let to = from + "REPEAT\\_SOURCE".len();
    let repeated_a = export_reviewed_docx(
        DocxInput {
            render: &repeated_out,
            template: None,
            bindings: &BTreeMap::new(),
            options: DocxOptions::default(),
        },
        &[review("repeated-source", (from, to))],
    )
    .unwrap();
    let repeated_doc = document(&repeated_a);
    assert_eq!(repeated_doc.text().matches("REPEAT_SOURCE").count(), 2);
    assert!(find(&repeated_doc, "w:commentRangeStart").is_empty());
    assert!(repeated_a.losses.iter().any(|loss| {
        loss.code == "word-review-unmapped" && loss.location.as_deref() == Some("repeated-source")
    }));
}
