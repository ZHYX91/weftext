use std::{fs, sync::OnceLock};
use tempfile::TempDir;
use weftext_engine::{Engine, RenderNode, RenderOutput};

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::new_with_system_fonts(false).unwrap())
}

fn render(source: &str, dir: &TempDir) -> RenderOutput {
    let out = engine()
        .render(source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(out.ok, "{source}\n{:?}", out.diagnostics);
    out
}

fn roles<'a>(node: &'a RenderNode, role: &str) -> Vec<&'a RenderNode> {
    let mut out = vec![];
    fn visit<'a>(node: &'a RenderNode, role: &str, out: &mut Vec<&'a RenderNode>) {
        if node.kind != "tag"
            && node
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
    if node.kind == "tag" {
        return String::new();
    }
    node.text.clone().unwrap_or_default() + &node.children.iter().map(text).collect::<String>()
}

fn run_in_recipe(label: &str) -> String {
    format!(
        "#show <{label}>: it => context {{\n  strong[\n    #if it.numbering != none {{\n      counter(heading).display(it.numbering)\n      text(\" \")\n    }}\n    #it.body\n  ]\n  text(\" \")\n}}"
    )
}

#[test]
fn native_heading_level_and_exact_prefix_do_not_come_from_html_or_title_text() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set heading(numbering: \"1.1\")\n= 7. 原题 <one>\n#heading(level:6)[深标题]\n#heading(level:12, numbering:none)[更深]",
        &dir,
    );
    let headings = roles(out.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings.len(), 3);
    assert_eq!(headings[0].tag.as_deref(), Some("h2"));
    assert_eq!(
        headings
            .iter()
            .map(|node| node.attrs["data-weftext-level"].as_str())
            .collect::<Vec<_>>(),
        ["1", "6", "12"]
    );
    assert_eq!(headings[0].attrs["data-weftext-label"], "one");
    assert_eq!(headings[0].attrs["data-weftext-numbered"], "true");
    assert_eq!(headings[0].attrs["data-weftext-numbering-kind"], "pattern");
    assert_eq!(headings[0].attrs["data-weftext-numbering-pattern"], "1.1");
    assert_eq!(headings[0].attrs["data-weftext-counter-state"], "[1]");
    assert_eq!(headings[2].attrs["data-weftext-numbered"], "false");
    let prefix = roles(headings[0], "heading-prefix");
    assert_eq!(prefix.len(), 1);
    assert_eq!(text(prefix[0]), "1 ");
    assert_eq!(text(headings[0]), "1 7. 原题");
    assert!(roles(headings[2], "heading-prefix").is_empty());
}

#[test]
fn displayed_native_title_and_unique_native_block_own_frontmatter_roles() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set document(title:[原生标题])\n#title()\n#block[副标题] <weftext-subtitle>\n= 章节",
        &dir,
    );
    let tree = out.tree.as_ref().unwrap();
    assert_eq!(text(roles(tree, "title")[0]), "原生标题");
    assert_eq!(text(roles(tree, "subtitle")[0]), "副标题");
    assert_eq!(roles(tree, "heading").len(), 1);
    for source in [
        "#block[孤立] <weftext-subtitle>",
        "#title[A]\n#title[B]\n#block[孤立] <weftext-subtitle>",
        "#title[A]\n#block[甲] <weftext-subtitle>\n#block[乙] <weftext-subtitle>",
        "#title[A]\n#block[甲] <weftext-subtitle>\n= 章节 <weftext-subtitle>",
        "#show title: none\n#title[A]\n#block[孤立] <weftext-subtitle>",
    ] {
        let out = render(source, &dir);
        assert!(
            roles(out.tree.as_ref().unwrap(), "subtitle").is_empty(),
            "{source}"
        );
    }
}

#[test]
fn deleted_custom_show_and_author_html_cannot_forge_native_heading_roles() {
    let dir = TempDir::new().unwrap();
    for source in [
        "#show heading: none\n= 消失",
        "#show heading: it => [替换]\n= 原题",
        "#show heading: it => html.elem(\"h2\")[#it.body]\n= 原题",
        "#html.elem(\"h1\", attrs:(\"data-weftext-role\":\"heading\", \"data-weftext-level\":\"1\", \"data-weftext-numbered\":\"true\"))[伪造]",
    ] {
        let out = render(source, &dir);
        assert!(
            roles(out.tree.as_ref().unwrap(), "heading").is_empty(),
            "{source}\n{:?}",
            out.tree
        );
    }
    let out = render("#show title: none\n#title[消失标题]", &dir);
    assert!(roles(out.tree.as_ref().unwrap(), "title").is_empty());
    assert!(out.products.unwrap().native_title_bodies.is_empty());
}

#[test]
fn repeated_include_and_repeated_show_preserve_each_actual_native_production() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("inc.typ"), "= 相同 <same>").unwrap();
    let out = render(
        "#set heading(numbering:\"1\")\n#include \"inc.typ\"\n#include \"inc.typ\"",
        &dir,
    );
    let headings = roles(out.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings.len(), 2);
    assert_eq!(text(roles(headings[0], "heading-prefix")[0]), "1 ");
    assert_eq!(text(roles(headings[1], "heading-prefix")[0]), "2 ");
    assert!(headings.iter().all(|node| {
        node.origin
            .as_ref()
            .is_some_and(|origin| origin.path == "inc.typ" && !origin.is_main)
    }));
    let out = render("#show heading: it => {it; it}\n= 双次", &dir);
    assert_eq!(roles(out.tree.as_ref().unwrap(), "heading").len(), 2);
}

#[test]
fn controlled_run_in_uses_actual_native_heading_tags_and_number_prefix() {
    let dir = TempDir::new().unwrap();
    let source = format!(
        "#set heading(numbering:\"1.1\")\n{}\n== 7. 方法 <methods>\n正文 *明确强调*",
        run_in_recipe("methods")
    );
    let out = render(&source, &dir);
    let headings = roles(out.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings.len(), 1, "{:?}", out.tree);
    assert_eq!(headings[0].tag.as_deref(), Some("strong"));
    assert_eq!(headings[0].attrs["data-weftext-run-in"], "true");
    assert_eq!(headings[0].attrs["data-weftext-level"], "2");
    assert_eq!(headings[0].attrs["data-weftext-numbered"], "true");
    assert_eq!(headings[0].attrs["data-weftext-counter-state"], "[0,1]");
    let prefix = roles(headings[0], "heading-prefix");
    assert_eq!(prefix.len(), 1, "{:?}", out.tree);
    assert_eq!(text(prefix[0]), "0.1  ");
    assert_eq!(text(headings[0]), "0.1  7. 方法");
    assert!(
        out.products
            .unwrap()
            .body
            .unwrap()
            .contains("正文 明确强调")
    );
    let out = render(
        "#show <methods>: it => strong(it.body)\n== 7. 方法 <methods>\n正文",
        &dir,
    );
    assert!(roles(out.tree.as_ref().unwrap(), "heading").is_empty());
    let out = render(
        &format!("{}\n== 方法 <methods>\n正文", run_in_recipe("methods")),
        &dir,
    );
    let headings = roles(out.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings.len(), 1);
    assert_eq!(headings[0].attrs["data-weftext-numbered"], "false");
    assert!(roles(headings[0], "heading-prefix").is_empty());
    let out = render(
        &format!(
            "#show strong: it=>it.body\n{}\n== 方法 <methods>\n正文",
            run_in_recipe("methods")
        ),
        &dir,
    );
    assert!(roles(out.tree.as_ref().unwrap(), "heading").is_empty());
}

#[test]
fn same_html_target_native_math_and_heading_reference_are_distinct_from_custom_links() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set heading(numbering:\"1\")\n= 章节 <one>\n见 @one 和 #link(<one>)[自定义文字]。$ frac(1,2) + x^2 $\n#context if target() == \"html\" { heading(level:3)[HTML 分支] } else { heading(level:9)[分页分支] }",
        &dir,
    );
    let tree = out.tree.as_ref().unwrap();
    let refs = roles(tree, "reference");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].attrs["data-weftext-reference-kind"], "number");
    assert_eq!(refs[0].attrs["data-weftext-label"], "one");
    let numbers = roles(refs[0], "reference-number");
    assert_eq!(numbers.len(), 1);
    assert_eq!(text(numbers[0]), "1");
    assert_eq!(roles(tree, "math").len(), 1);
    assert!(out.html.contains("<mfrac"));
    assert!(out.html.contains("<msup"));
    let headings = roles(tree, "heading");
    assert_eq!(headings.len(), 2);
    assert_eq!(headings[1].attrs["data-weftext-level"], "3");
    assert!(!text(tree).contains("分页分支"));
}

#[test]
fn effective_offsets_numbering_functions_and_reference_supplement_keep_native_facts() {
    let dir = TempDir::new().unwrap();
    let out = render(
        "#set heading(offset:2, numbering:(..nums)=>[§#nums.pos().last()])\n= 函数编号\n#heading(level:8, numbering:none)[局部无编号]",
        &dir,
    );
    let headings = roles(out.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings[0].attrs["data-weftext-level"], "3");
    assert_eq!(headings[0].attrs["data-weftext-numbering-kind"], "function");
    assert_eq!(headings[0].attrs["data-weftext-counter-state"], "[0,0,1]");
    assert!(
        !headings[0]
            .attrs
            .contains_key("data-weftext-numbering-pattern")
    );
    assert_eq!(text(roles(headings[0], "heading-prefix")[0]), "§1 ");
    assert_eq!(headings[1].attrs["data-weftext-level"], "8");
    assert_eq!(headings[1].attrs["data-weftext-numbering-kind"], "none");
    let out = render(
        "#set heading(numbering:\"I\")\n= 章节 <one>\n见 #ref(<one>, supplement:[自定义章]) 和 #link(<one>)[I 自定义链接]。",
        &dir,
    );
    let tree = out.tree.as_ref().unwrap();
    let refs = roles(tree, "reference");
    assert_eq!(refs.len(), 1);
    assert_eq!(text(refs[0]), "自定义章\u{a0}I");
    let numbers = roles(refs[0], "reference-number");
    assert_eq!(text(numbers[0]), "I");
    assert_eq!(numbers[0].attrs["data-weftext-numbering-pattern"], "I");
    let out = render(
        "#show ref: it=>[自定义引用]\n#set heading(numbering:\"1\")\n= 章节 <one>\n@one",
        &dir,
    );
    assert!(roles(out.tree.as_ref().unwrap(), "reference").is_empty());
    let out = render(
        "#set heading(numbering:\"1\")\n#counter(heading).update(5)\n= 第六节",
        &dir,
    );
    let headings = roles(out.tree.as_ref().unwrap(), "heading");
    assert_eq!(headings[0].attrs["data-weftext-counter-state"], "[6]");
    assert_eq!(text(roles(headings[0], "heading-prefix")[0]), "6 ");
}
