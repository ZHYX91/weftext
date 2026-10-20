use std::sync::OnceLock;
use tempfile::TempDir;
use weftext_engine::{Engine, RenderNode};

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::new_with_system_fonts(false).unwrap())
}
fn render(source: &str, dir: &TempDir) -> weftext_engine::RenderOutput {
    let output = engine()
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(output.ok, "{source}\n{:?}", output.diagnostics);
    output
}
fn roles<'a>(node: &'a RenderNode, role: &str) -> Vec<&'a RenderNode> {
    let mut out = vec![];
    fn visit<'a>(node: &'a RenderNode, role: &str, out: &mut Vec<&'a RenderNode>) {
        if node
            .attrs
            .get("data-weftext-role")
            .is_some_and(|value| value == role)
        {
            out.push(node);
        }
        for child in &node.children {
            visit(child, role, out);
        }
    }
    visit(node, role, &mut out);
    out
}
fn text(node: &RenderNode) -> String {
    node.text.clone().unwrap_or_default() + &node.children.iter().map(text).collect::<String>()
}

#[test]
fn native_footnote_identity_has_one_body_and_all_actual_references() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "正文#footnote[中文 *强调*\n\n第二段] <note>。再引 @note。#footnote(<note>)。独立#footnote[中文 *强调*\n\n第二段]。空#footnote[]。",
        &dir,
    );
    let tree = output.tree.as_ref().unwrap();
    let references = roles(tree, "footnote-reference");
    let bodies = roles(tree, "footnote-body");
    assert_eq!(references.len(), 5);
    assert_eq!(
        references
            .iter()
            .map(|reference| reference.attrs["data-weftext-note-definition"].as_str())
            .collect::<Vec<_>>(),
        ["true", "false", "false", "true", "true"]
    );
    assert_eq!(bodies.len(), 3);
    let first = &references[0].attrs["data-weftext-note-id"];
    assert!(
        references[..3]
            .iter()
            .all(|node| node.attrs["data-weftext-note-id"] == *first)
    );
    assert_ne!(references[3].attrs["data-weftext-note-id"], *first);
    assert_eq!(bodies[0].attrs["data-weftext-note-id"], *first);
    assert_eq!(text(bodies[0]), "中文 强调第二段");
    assert_eq!(text(bodies[2]), "");
    assert!(
        references[0]
            .origin
            .as_ref()
            .is_some_and(|origin| origin.raw.starts_with("footnote"))
    );
}

#[test]
fn caption_owner_category_counter_and_author_body_come_from_native_objects() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "#figure([对象甲], caption:[同题]) <a>\n#figure(table(columns:1)[单元],caption:[同题]) <b>\n#figure([对象乙],caption:figure.caption(position:top)[同题]) <c>\n@a @b @c",
        &dir,
    );
    let tree = output.tree.as_ref().unwrap();
    let figures = roles(tree, "figure");
    let captions = roles(tree, "caption");
    assert_eq!(figures.len(), 3);
    assert_eq!(captions.len(), 3);
    for (figure, caption) in figures.iter().zip(captions.iter()) {
        assert_eq!(
            figure.attrs["data-weftext-target-id"],
            caption.attrs["data-weftext-target-id"]
        );
        assert_eq!(text(roles(caption, "caption-body")[0]), "同题");
    }
    assert_eq!(captions[0].attrs["data-weftext-caption-kind"], "figure");
    assert_eq!(captions[1].attrs["data-weftext-caption-kind"], "table");
    assert_eq!(captions[2].attrs["data-weftext-caption-position"], "top");
    assert_eq!(captions[2].attrs["data-weftext-counter-state"], "[2]");
    assert_ne!(
        captions[0].attrs["data-weftext-target-id"],
        captions[2].attrs["data-weftext-target-id"]
    );
    for reference in roles(tree, "reference") {
        assert!(reference.attrs.contains_key("data-weftext-target-id"));
    }
}

#[test]
fn numbered_equation_and_descriptive_caption_do_not_invent_each_others_counter() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "#set math.equation(numbering:\"(1)\")\n$ x = 1 $ <eq>\n#figure($ y = 2 $,caption:[仅描述],numbering:none,kind:math.equation) <desc>\n@eq",
        &dir,
    );
    let tree = output.tree.as_ref().unwrap();
    let equations = roles(tree, "equation");
    assert_eq!(equations.len(), 2);
    assert_eq!(roles(equations[0], "equation-prefix").len(), 1);
    assert_eq!(equations[0].attrs["data-weftext-numbering-pattern"], "(1)");
    assert_eq!(
        roles(tree, "caption")[0].attrs["data-weftext-numbered"],
        "false"
    );
    assert!(roles(roles(tree, "caption")[0], "caption-prefix").is_empty());
}

#[test]
fn forged_html_and_shadowed_or_replaced_producers_gain_no_native_role() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "#html.elem(\"span\",attrs:(\"data-weftext-role\":\"footnote-reference\",\"data-weftext-note-id\":\"forged\"))[1]\n#html.elem(\"figcaption\",attrs:(\"data-weftext-role\":\"caption\"))[图 1]\n#let footnote(body)=super[假注]\n#footnote[原文]",
        &dir,
    );
    let tree = output.tree.as_ref().unwrap();
    assert!(roles(tree, "footnote-reference").is_empty());
    assert!(roles(tree, "caption").is_empty());
}

#[test]
fn final_text_language_tracks_native_overrides_and_absent_region() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "#set text(lang:\"en\",region:\"GB\")\nEnglish #text(lang:\"zh\",region:\"CN\")[中文] #text(lang:\"zh\",region:none)[无地区]",
        &dir,
    );
    let tree = output.tree.as_ref().unwrap();
    fn collect(node: &RenderNode, inherited: &str, out: &mut Vec<(String, String)>) {
        let language = node
            .attrs
            .get("lang")
            .map(String::as_str)
            .unwrap_or(inherited);
        if let Some(text) = &node.text
            && !text.trim().is_empty()
        {
            out.push((text.clone(), language.into()));
        }
        for child in &node.children {
            collect(child, language, out);
        }
    }
    let mut actual = vec![];
    collect(tree, "", &mut actual);
    assert!(
        actual
            .iter()
            .any(|(text, lang)| text.contains("English") && lang == "en-GB"),
        "{actual:?}"
    );
    assert!(
        actual
            .iter()
            .any(|(text, lang)| text.contains("中文") && lang == "zh-CN"),
        "{actual:?}"
    );
    assert!(
        actual
            .iter()
            .any(|(text, lang)| text.contains("无地区") && lang == "zh"),
        "{actual:?}"
    );
}

#[test]
fn managed_numbering_requires_the_actual_effective_native_function_from_this_header() {
    use weftext_engine::{HeadingNumberingIntent, HeadingNumberingScheme, edit_heading_numbering};
    let dir = TempDir::new().unwrap();
    let source = edit_heading_numbering(
        "= 章 <one>\n== 节 <two>\n=== 深 <deep>\n= 不编号 <skip>",
        &HeadingNumberingIntent {
            scheme: HeadingNumberingScheme::Chinese,
            start: 3,
            max_level: 2,
            exclude_labels: vec!["skip".into()],
        },
    )
    .unwrap();
    let output = render(&source, &dir);
    let headings = roles(output.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings[0].attrs["data-weftext-numbering-kind"], "function");
    assert_eq!(
        headings[0]
            .attrs
            .get("data-weftext-managed-numbering-pattern")
            .map(String::as_str),
        Some("一.一"),
        "{:?}",
        headings[0].attrs
    );
    assert_eq!(
        headings[2].attrs["data-weftext-managed-numbering-visible"],
        "false"
    );
    assert_eq!(headings[3].attrs["data-weftext-numbered"], "false");
    let output = render(
        "#set heading(numbering:(..n)=>numbering(\"1.1\",..n))\n= 原生自定义",
        &dir,
    );
    assert!(
        !roles(output.tree.as_ref().unwrap(), "heading")[0]
            .attrs
            .contains_key("data-weftext-managed-numbering-pattern")
    );
}

#[test]
fn certified_numbering_header_preserves_the_managed_identity_and_control_binding() {
    use weftext_engine::{
        HeadingNumberingIntent, certify_metadata_declaration, edit_heading_numbering, inspect,
    };
    let source =
        "#metadata((id:\"8f3fe32e-e7ce-47f7-a731-dda1eb15480f\",types:())) <weftext>\n= 标题 <one>";
    let numbered = edit_heading_numbering(source, &HeadingNumberingIntent::default()).unwrap();
    assert_eq!(inspect(&numbered).identity_status, "valid");
    assert!(certify_metadata_declaration(&numbered, "weftext").is_some());
    let forged = numbered.replace(
        "numbering(\"1.1\", ..n)",
        "numbering(\"1.1\", ..n); eval(\"\")",
    );
    assert_ne!(inspect(&forged).identity_status, "valid");
    assert!(certify_metadata_declaration(&forged, "weftext").is_none());
}

#[test]
fn forward_note_references_are_never_promoted_to_definitions() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("weftext-endnotes.typ"),
        include_bytes!("../../weftext-export/assets/weftext-endnotes.typ"),
    )
    .unwrap();
    let output = render(
        "#import \"weftext-endnotes.typ\": endnote,endnote-ref,endnotes\n#endnote-ref(\"b\") #endnote(id:\"a\")[甲] #endnote(id:\"b\")[乙]\n#endnotes()\n前引#footnote(<b>) 甲#footnote[甲] <a> 乙#footnote[乙] <b>",
        &dir,
    );
    let tree = output.tree.as_ref().unwrap();
    for family in ["endnote-reference", "footnote-reference"] {
        let references = roles(tree, family);
        assert_eq!(references.len(), 3, "{family}");
        assert_eq!(
            references
                .iter()
                .map(|reference| reference
                    .attrs
                    .get("data-weftext-note-definition")
                    .map(String::as_str))
                .collect::<Vec<_>>(),
            [Some("false"), Some("true"), Some("true")],
            "{family} {references:?}"
        );
        assert_eq!(
            references[0].attrs["data-weftext-note-id"],
            references[2].attrs["data-weftext-note-id"]
        );
    }
}
