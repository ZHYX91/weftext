use std::{fs, path::Path, sync::OnceLock};
use tempfile::TempDir;
use weftext_engine::{Engine, RenderNode, encode_literal, inspect};

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::new().unwrap())
}
fn render(source: &str, root: &Path) -> weftext_engine::RenderOutput {
    engine()
        .render(source, &root.join("note.typ"), root)
        .unwrap()
}
fn texts(node: &RenderNode) -> Vec<&RenderNode> {
    let mut result = vec![];
    if node.kind == "text" {
        result.push(node);
    }
    for child in &node.children {
        result.extend(texts(child));
    }
    result
}
const META: &str = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"task\",), status: \"todo\")) <weftext>\n";

#[test]
fn native_chinese_emphasis_has_exact_literal_origins() {
    let dir = TempDir::new().unwrap();
    let source = "= 中文标题\n你好 *强调*，_斜体_。 😀\n\n- 列表一\n- 列表二";
    let output = render(source, dir.path());
    assert!(output.ok, "{:?}", output.diagnostics);
    fn has_role(node: &RenderNode, tag: &str, text: &str) -> bool {
        (node.tag.as_deref() == Some(tag)
            && texts(node)
                .iter()
                .filter_map(|n| n.text.as_deref())
                .collect::<String>()
                == text)
            || node.children.iter().any(|n| has_role(n, tag, text))
    }
    assert!(has_role(output.tree.as_ref().unwrap(), "strong", "强调"));
    assert!(has_role(output.tree.as_ref().unwrap(), "em", "斜体"));
    let nodes = texts(output.tree.as_ref().unwrap());
    let strong = nodes
        .iter()
        .find(|n| n.text.as_deref() == Some("强调"))
        .unwrap();
    assert!(strong.writable, "{strong:?}");
    let origin = strong.origin.as_ref().unwrap();
    assert_eq!(&source[origin.from..origin.to], "强调");
    assert_eq!(
        source[..origin.from].encode_utf16().count(),
        origin.utf16_from
    );
    assert_eq!(output.source_info.headings[0].level, 1);
}

#[test]
fn broken_source_is_diagnosed_and_never_writable() {
    let dir = TempDir::new().unwrap();
    let output = render("#let x = (\n", dir.path());
    assert!(!output.ok);
    assert!(output.tree.is_none());
    assert!(!output.source_info.syntax_valid);
    assert!(!output.diagnostics.is_empty());
}

#[test]
fn include_repetition_and_generated_source_are_readonly() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("included.typ"), "重复来源").unwrap();
    let output = render(
        "#include \"included.typ\"\n\n#include \"included.typ\"",
        dir.path(),
    );
    assert!(output.ok, "{:?}", output.diagnostics);
    let nodes = texts(output.tree.as_ref().unwrap());
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n.text.as_deref() == Some("重复来源"))
            .count(),
        2
    );
    assert!(nodes.iter().all(|n| !n.writable));
    assert!(
        nodes
            .iter()
            .filter_map(|n| n.origin.as_ref())
            .any(|o| o.path == "included.typ" && !o.is_main)
    );
    let generated = render("#let s = [计算内容]\n#s #s\n旁边正文", dir.path());
    assert!(generated.ok);
    assert!(
        texts(generated.tree.as_ref().unwrap())
            .iter()
            .filter(|n| n.text.as_deref() == Some("计算内容"))
            .all(|n| !n.writable)
    );
    assert!(
        texts(generated.tree.as_ref().unwrap())
            .iter()
            .any(|n| n.text.as_deref() == Some("旁边正文") && n.writable)
    );
}

#[test]
fn metadata_is_official_static_and_exactly_unique() {
    let info = inspect(&format!("{META}正文 *强调*"));
    assert_eq!(info.identity_status, "valid", "{info:?}");
    assert_eq!(info.types, vec!["task"]);
    assert!(info.body_insertion.is_some());
    let status = info
        .metadata
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.key == "status")
        .unwrap();
    assert_eq!(status.value.unwrap(), "todo");
    assert_eq!(status.raw, "\"todo\"");
    let duplicate = inspect(&format!("{META}{META}"));
    assert_eq!(duplicate.identity_status, "ambiguous");
    assert!(duplicate.id.is_none());
    let duplicate_key = inspect(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", \"id\": \"same\")) <weftext>",
    );
    assert!(duplicate_key.id.is_none());
    let shadowed = inspect(&format!("#let metadata = x => x\n{META}"));
    assert!(shadowed.id.is_none());
    let dynamic = inspect("#metadata((id: str(42), types: ())) <weftext>");
    assert!(dynamic.id.is_none());
    assert!(dynamic.body_insertion.is_none());
}

#[test]
fn leading_native_metadata_preserves_body_writer() {
    let dir = TempDir::new().unwrap();
    let output = render(&format!("{META}你好 *可写*"), dir.path());
    assert!(output.ok, "{:?}", output.diagnostics);
    assert!(
        texts(output.tree.as_ref().unwrap())
            .iter()
            .any(|n| n.text.as_deref() == Some("可写") && n.writable)
    );
    assert_eq!(output.source_info.identity_status, "valid");
}

#[test]
fn dependency_root_and_packages_are_restricted() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    assert!(
        engine()
            .render("正文", &outside.path().join("note.typ"), root.path())
            .is_err()
    );
    fs::write(outside.path().join("secret.typ"), "机密").unwrap();
    let output = render("#include \"../secret.typ\"", root.path());
    assert!(!output.ok);
    let package = render("#import \"@preview/example:0.1.0\"", root.path());
    assert!(!package.ok);
    assert!(
        package
            .diagnostics
            .iter()
            .any(|d| d.message.contains("packages are unavailable"))
    );
}

#[test]
fn unsafe_html_is_not_executed_or_exposed() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "#html.elem(\"script\")[alert(1)]\n#html.elem(\"a\", attrs: (href: \"javascript:alert(1)\", onclick: \"alert(2)\"))[链接]",
        dir.path(),
    );
    assert!(output.ok, "{:?}", output.diagnostics);
    assert!(!output.html.contains("<script"));
    assert!(!output.html.contains("javascript:"));
    assert!(!output.html.contains("onclick"));
}

#[test]
fn literal_encoder_roundtrips_native_punctuation() {
    let dir = TempDir::new().unwrap();
    let literal = "#let x = [危险] *普通* _文字_ @label $x$ //注释 😀";
    let encoded = encode_literal(literal).unwrap();
    let output = render(&encoded, dir.path());
    assert!(output.ok, "{} {:?}", encoded, output.diagnostics);
    let actual = texts(output.tree.as_ref().unwrap())
        .into_iter()
        .filter_map(|n| n.text.as_deref())
        .collect::<String>();
    assert_eq!(actual, literal);
}

#[test]
fn render_keys_expire_after_literal_escape_splits_a_leaf() {
    let dir = TempDir::new().unwrap();
    let before = render("abc", dir.path());
    let after = render(&encode_literal("a*b").unwrap(), dir.path());
    let old = texts(before.tree.as_ref().unwrap())[0];
    let split = texts(after.tree.as_ref().unwrap());
    assert!(split.len() > 1);
    assert!(split.iter().all(|n| n.key != old.key));
    assert_eq!(
        split
            .iter()
            .filter_map(|n| n.text.as_deref())
            .collect::<String>(),
        "a*b"
    );
}

#[test]
fn static_native_title_is_readable_and_has_unique_writers() {
    let dir = TempDir::new().unwrap();
    let source = format!("{META}#set document(title: [研究标题])\n#title()\n\n正文");
    let output = render(&source, dir.path());
    assert!(output.ok, "{:?}", output.diagnostics);
    assert_eq!(output.source_info.title.as_deref(), Some("研究标题"));
    assert!(
        texts(output.tree.as_ref().unwrap())
            .iter()
            .any(|n| n.writable && n.text.as_deref() == Some("正文"))
    );
    assert!(
        inspect("#let title = x => x\n#title[伪标题]")
            .title
            .is_none()
    );
}

#[test]
fn native_math_table_and_deep_generated_headings_are_preserved() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "$x^2 + y^2$\n\n#table(columns: 2, [甲], [乙], [丙], [丁])\n\n#heading(level: 9)[生成深标题]",
        dir.path(),
    );
    assert!(output.ok, "{:?}", output.diagnostics);
    assert!(output.html.contains("<math"), "{}", output.html);
    assert!(output.html.contains("<table"));
    assert!(
        output
            .source_info
            .headings
            .iter()
            .any(|h| h.level == 9 && h.generated && h.text == "生成深标题")
    );
    assert!(
        texts(output.tree.as_ref().unwrap())
            .iter()
            .any(|n| n.text.as_deref() == Some("甲") && n.writable)
    );
    assert!(
        texts(output.tree.as_ref().unwrap())
            .iter()
            .filter(|n| n.text.as_deref() == Some("生成深标题"))
            .all(|n| !n.writable)
    );
    let removed = render("#show heading: none\n= 被删除\n\n实际正文", dir.path());
    assert!(removed.ok);
    assert!(!removed.html.contains("被删除"));
    assert!(removed.source_info.headings.iter().all(|h| !h.visible));
}

#[test]
fn eof_insertion_is_a_real_markup_boundary() {
    assert!(inspect("").body_insertion.is_some());
    assert!(inspect(META).body_insertion.is_some());
    assert!(inspect("// 注释末尾").body_insertion.is_none());
    assert!(inspect("// 注释末尾\n").body_insertion.is_some());
    assert!(inspect("#let x = [代码里的空白 ]").body_insertion.is_none());
}

#[test]
fn document_head_metadata_does_not_enter_display_text() {
    let dir = TempDir::new().unwrap();
    let ordinary = render("= 普通标题\n\n普通正文", dir.path());
    assert!(ordinary.ok);
    let actual = texts(ordinary.tree.as_ref().unwrap())
        .iter()
        .filter_map(|n| n.text.as_deref())
        .collect::<String>();
    assert!(!actual.contains("不支持的 HTML"), "{actual}");
    assert!(actual.contains("普通标题") && actual.contains("普通正文"));
    let native = render(
        "#set document(title: [真正标题])\n#title()\n\n正文",
        dir.path(),
    );
    assert!(native.ok, "{:?}", native.diagnostics);
    let text_nodes = texts(native.tree.as_ref().unwrap());
    assert_eq!(
        text_nodes
            .iter()
            .filter(|n| n.text.as_deref() == Some("真正标题"))
            .count(),
        1
    );
    assert!(native.html.contains("真正标题"));
}

#[test]
fn bare_link_call_slots_are_unverified_until_final_native_output() {
    let dir = TempDir::new().unwrap();
    let shadowed = render(
        "#let link(x) = [假的]\n#link(\"https://example.org\")",
        dir.path(),
    );
    assert!(shadowed.ok);
    assert_eq!(shadowed.source_info.links[0].kind, "unverified-call");
    assert!(!shadowed.html.contains("href="));
    let builtin = render("#link(\"https://example.org\")[真链]", dir.path());
    assert!(builtin.ok);
    assert_eq!(builtin.source_info.links[0].kind, "unverified-call");
    assert!(builtin.html.contains("href=\"https://example.org\""));
}

#[test]
fn projected_file_links_require_a_local_authority_free_uri() {
    let dir = TempDir::new().unwrap();
    let output = render(
        "#link(\"file:///C:/Notes/note.typ\")[本机]\n#link(\"file://server/share/note.typ\")[远程]\n#link(\"file:////server/share/note.typ\")[网络路径]\n#link(\"javascript:alert(1)\")[脚本]",
        dir.path(),
    );
    assert!(output.ok, "{:?}", output.diagnostics);
    assert!(output.html.contains("href=\"file:///C:/Notes/note.typ\""));
    assert!(!output.html.contains("file://server"));
    assert!(!output.html.contains("file:////server"));
    assert!(!output.html.contains("javascript:"));
}

#[test]
fn projected_resource_links_preserve_exact_reference_and_fragment() {
    let dir = TempDir::new().unwrap();
    let reference = "weftext-resource://d502838a-3359-4a2d-979f-eb1d8b34556a/354ed3b0-e49e-40ae-87fb-cdd7bb64fc17#page=2";
    let output = render(
        &format!("#link(\"{reference}\")[资源第二页]\n#link(\"javascript:alert(1)\")[拒绝脚本]"),
        dir.path(),
    );
    assert!(output.ok, "{:?}", output.diagnostics);
    assert!(output.html.contains(&format!("href=\"{reference}\"")));
    assert!(!output.html.contains("javascript:"));
    fn retained(node: &RenderNode, reference: &str) -> bool {
        (node.tag.as_deref() == Some("a") && node.attrs.get("href").is_some_and(|v| v == reference))
            || node.children.iter().any(|child| retained(child, reference))
    }
    assert!(retained(output.tree.as_ref().unwrap(), reference));
}

#[cfg(windows)]
#[test]
fn windows_junctions_cannot_escape_the_dependency_root() {
    use std::process::Command;
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("secret.typ"), "外部机密").unwrap();
    let junction = root.path().join("jump");
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let included = render("#include \"jump/secret.typ\"", root.path());
    assert!(!included.ok);
    assert!(!included.html.contains("外部机密"));
    assert!(
        engine()
            .render("草稿", &junction.join("secret.typ"), root.path())
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_the_dependency_root() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("secret.typ"), "外部机密").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("jump")).unwrap();
    assert!(!render("#include \"jump/secret.typ\"", root.path()).ok);
    assert!(
        engine()
            .render("草稿", &root.path().join("jump/secret.typ"), root.path())
            .is_err()
    );
}
