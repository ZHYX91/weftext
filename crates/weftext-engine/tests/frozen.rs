use std::{collections::BTreeMap, fs, sync::OnceLock};
use tempfile::TempDir;
use weftext_engine::*;
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::new_with_system_fonts(false).unwrap())
}
fn request(source: &str, root: &std::path::Path) -> FrozenRenderRequest {
    FrozenRenderRequest {
        source: source.into(),
        path: root.join("notes/note.typ"),
        root: root.into(),
        files: BTreeMap::new(),
        style: None,
    }
}

#[test]
fn frozen_world_has_no_ambient_file_fallback_and_keeps_real_relative_origins() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("notes")).unwrap();
    fs::write(dir.path().join("notes/inc.typ"), "磁盘新值").unwrap();
    let mut input = request("#include \"inc.typ\"\n\n正文", dir.path());
    let denied = engine().render_frozen(&input).unwrap();
    assert!(!denied.ok);
    assert!(denied.products.is_none());
    input
        .files
        .insert("notes/inc.typ".into(), "冻结原值".as_bytes().to_vec());
    let first = engine().render_frozen(&input).unwrap();
    assert!(first.ok, "{:?}", first.diagnostics);
    assert!(
        first
            .products
            .as_ref()
            .unwrap()
            .body
            .as_ref()
            .unwrap()
            .contains("冻结原值")
    );
    assert!(!first.html.contains("磁盘新值"));
    fs::write(dir.path().join("notes/inc.typ"), "再次变化").unwrap();
    let second = engine().render_frozen(&input).unwrap();
    assert_eq!(first.html, second.html);
    assert!(
        first
            .dependencies
            .iter()
            .any(|d| d.path == "notes/inc.typ" && d.status == "loaded")
    );
}

#[test]
fn frozen_lexical_inputs_reject_outside_aliases_and_mismatched_authorities() {
    let dir = TempDir::new().unwrap();
    for path in ["../outside.typ", "/absolute.typ", "C:/outside.typ"] {
        let mut input = request("正文", dir.path());
        input.files.insert(path.into(), vec![]);
        assert!(engine().render_frozen(&input).is_err(), "{path}");
    }
    let mut mismatch = request("主源", dir.path());
    mismatch
        .files
        .insert("notes/note.typ".into(), "另一个源".as_bytes().to_vec());
    assert!(engine().render_frozen(&mismatch).is_err());
    let mut package = request("#import \"@preview/example:0.1.0\"", dir.path());
    assert!(!engine().render_frozen(&package).unwrap().ok);
    package.source = "正文".into();
    package.root = std::path::PathBuf::from("relative-root");
    assert!(engine().render_frozen(&package).is_err());
}

#[test]
fn frozen_style_wrapper_uses_only_frozen_dependency_bytes() {
    let dir = TempDir::new().unwrap();
    let mut input = request("#include \"inc.typ\"\n\n#text(size:18pt)[局部]", dir.path());
    input
        .files
        .insert("notes/inc.typ".into(), "冻结中文".as_bytes().to_vec());
    input.style = Some(StyleModule {
        path: "styles/library-style.typ".into(),
        source: DEFAULT_LIBRARY_STYLE.into(),
        enabled: true,
        entry_function: "library-style".into(),
    });
    let out = engine().render_frozen(&input).unwrap();
    assert!(out.ok, "{:?}", out.diagnostics);
    assert!(out.html.contains("font-size: 18pt"));
    assert!(out.html.contains("冻结中文"));
    assert!(
        out.dependencies
            .iter()
            .any(|d| d.path == "styles/library-style.typ")
    );
    input.files.insert(
        "styles/library-style.typ".into(),
        "不同样式".as_bytes().to_vec(),
    );
    assert!(engine().render_frozen(&input).is_err());
}

#[test]
fn query_products_use_final_body_and_native_generated_heading_and_title() {
    let dir = TempDir::new().unwrap();
    let mut input = request(
        "#let generated = [生成标题]\n#title(generated)\n\n#if false [= 不存在标题]\n#heading(level:12)[实际深标题]\n#for n in (1,2) [循环#n]\n",
        dir.path(),
    );
    let out = engine().render_frozen(&input).unwrap();
    assert!(out.ok, "{:?}", out.diagnostics);
    let products = out.products.as_ref().unwrap();
    assert_eq!(products.target, "html");
    assert_eq!(products.title.as_deref(), Some("生成标题"));
    assert!(!products.body.as_ref().unwrap().contains("不存在标题"));
    assert_eq!(products.headings.len(), 1);
    assert_eq!(products.headings[0].level, 12);
    assert!(products.headings[0].generated);
    input.source =
        "#set document(title:[早期标题])\n#title()\n#set document(title:[后期元标题])\n正文".into();
    let out = engine().render_frozen(&input).unwrap();
    assert!(out.ok);
    let products = out.products.unwrap();
    assert_eq!(products.native_title_bodies, vec!["早期标题"]);
    assert_eq!(products.metadata_title.as_deref(), Some("后期元标题"));
    assert_eq!(products.title_status, "ambiguous");
    input.source = "#html.frame[无法承诺流式文字]".into();
    let out = engine().render_frozen(&input).unwrap();
    assert!(out.ok);
    assert_eq!(out.products.as_ref().unwrap().body_status, "unavailable");
    assert!(out.products.unwrap().body.is_none());
}
