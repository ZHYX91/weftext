use base64::Engine as _;
use tempfile::TempDir;
use weftext_engine::*;

fn roles(node: &RenderNode, role: &str) -> usize {
    usize::from(
        node.attrs
            .get("data-weftext-role")
            .is_some_and(|value| value == role),
    ) + node
        .children
        .iter()
        .map(|child| roles(child, role))
        .sum::<usize>()
}
fn managed_chinese(node: &RenderNode) -> usize {
    usize::from(
        node.attrs
            .get("data-weftext-managed-numbering-pattern")
            .is_some_and(|value| value == "一.一"),
    ) + node.children.iter().map(managed_chinese).sum::<usize>()
}

#[test]
fn certified_numbering_identity_citations_and_both_note_families_compile_together() {
    let dir = TempDir::new().unwrap();
    let body = r#"#metadata((id:"8f3fe32e-e7ce-47f7-a731-dda1eb15480f",types:())) <weftext>
#metadata((kind:"weftext.citations",entries:((key:"native2026",work:"weftext-node://a97accaf-60c9-41e7-807e-6248ec30cbee/b257f6be-e010-4ec4-beca-71af230af253"),))) <weftext-citations>
#import "weftext-endnotes.typ": endnote,endnote-ref,endnotes
#set text(lang: "zh", region: "CN")
#set math.equation(numbering: "(1)")
#title[Native composition]

= 章 <chapter>

正文#footnote[脚注正文] <foot> 再引#footnote(<foot>)。尾#box[#endnote(id:"end")[尾注正文]] 再引#box[#endnote-ref("end")]。
#text(lang:"en",region:"GB")[Colour] #text(lang:"en",region:"US")[Color] #text(lang:"ar",region:"SA")[نص عربي]。
行内 $x / x + y_1^2$ 与独立公式：

$ x / x + y_1^2 = 1 $ <equation>

#figure(table(columns:2, table.header([项目],[说明]), [收入],[相邻格收入]), kind:table,
  supplement:[表], numbering:"1", caption:figure.caption(position:top)[表格题注]) <tab>
#figure(image("image.png",width:12pt), kind:image, supplement:[图], numbering:"1",
  caption:figure.caption(position:bottom)[图片题注]) <fig>
引用 @chapter、@equation、@tab、@fig。正式文献@native2026 与#cite(<native2026>,form:"prose")，再次引用@native2026。
#link("https://example.com/reference")[作者链接]。

- 无序项
- 第二项

+ 有序项
+ 第二项

=== 超出编号层级 <deep>

= 排除 <excluded>

#bibliography("weftext.citations.bib",style:"ieee")
#endnotes(title:[尾注])
"#;
    let source = edit_heading_numbering("", &Default::default()).unwrap() + body;
    let source = source.as_str();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let mut input = FrozenRenderRequest {
        source: source.into(), path: dir.path().join("combined.typ"), root: dir.path().into(), style: None,
        files: std::collections::BTreeMap::from([
            ("weftext-endnotes.typ".into(), include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec()),
            ("weftext.citations.bib".into(), b"@article{native2026,title={Native Work Publication},author={Chen, Ming},year={2026},journal={Native Journal}}".to_vec()),
            ("image.png".into(), base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==").unwrap()),
        ]),
    };
    let current = engine.render_frozen(&input).unwrap();
    assert!(current.ok, "{:?}", current.diagnostics);
    let source = engine
        .edit_notes_frozen(
            &input,
            &EditRequest {
                source_hash: source_hash(source),
                intent: EditIntent::HeadingNumbering {
                    action: HeadingNumberingIntent {
                        scheme: HeadingNumberingScheme::Chinese,
                        start: 3,
                        max_level: 2,
                        exclude_labels: vec!["excluded".into()],
                    },
                },
            },
        )
        .unwrap()
        .source;
    assert_eq!(inspect(&source).identity_status, "valid");
    let declaration = inspect_citation_declaration(&source).unwrap().unwrap();
    assert_eq!(declaration.entries.len(), 1);
    assert_eq!(
        declaration.work_slots[0].range.raw,
        "\"weftext-node://a97accaf-60c9-41e7-807e-6248ec30cbee/b257f6be-e010-4ec4-beca-71af230af253\""
    );
    input.source = source;
    let output = engine.render_frozen(&input).unwrap();
    assert!(output.ok, "{:?}", output.diagnostics);
    let tree = output.tree.as_ref().unwrap();
    assert_eq!(roles(tree, "footnote-reference"), 2);
    assert_eq!(roles(tree, "footnote-body"), 1);
    assert_eq!(roles(tree, "endnote-reference"), 2);
    assert_eq!(roles(tree, "endnote-body"), 1);
    assert_eq!(managed_chinese(tree), 1);
    assert!(roles(tree, "citation") >= 3);
    assert!(roles(tree, "equation") >= 1);
    assert!(roles(tree, "math") >= 2);
    assert!(output.html.contains("Native Work Publication"));
    assert!(
        output
            .dependencies
            .iter()
            .any(|dependency| dependency.path == "weftext.citations.bib"
                && dependency.status == "loaded")
    );
}

#[test]
fn same_named_module_and_unread_pinned_copy_do_not_certify_managed_numbering() {
    let dir = TempDir::new().unwrap();
    let source = edit_heading_numbering(
        "= Heading",
        &HeadingNumberingIntent {
            scheme: HeadingNumberingScheme::Chinese,
            ..Default::default()
        },
    )
    .unwrap()
        + "\n#import \"weftext-endnotes.typ\": endnote, endnote-ref, endnotes\n#endnotes()";
    let input = FrozenRenderRequest {
        source, path: dir.path().join("notes/note.typ"), root: dir.path().into(), style: None,
        files: std::collections::BTreeMap::from([
            ("notes/weftext-endnotes.typ".into(), b"#let endnote(..args) = []\n#let endnote-ref(..args) = []\n#let endnotes() = context counter(heading).update(50)".to_vec()),
            ("weftext-endnotes.typ".into(), include_bytes!("../../weftext-export/assets/weftext-endnotes.typ").to_vec()),
        ]),
    };
    let output = Engine::new_with_system_fonts(false)
        .unwrap()
        .render_frozen(&input)
        .unwrap();
    assert!(output.ok, "{:?}", output.diagnostics);
    assert_eq!(managed_chinese(output.tree.as_ref().unwrap()), 0);
    assert!(
        output
            .dependencies
            .iter()
            .any(|dependency| dependency.path == "notes/weftext-endnotes.typ"
                && dependency.status == "loaded")
    );
    assert!(
        !output
            .dependencies
            .iter()
            .any(|dependency| dependency.path == "weftext-endnotes.typ")
    );
}
