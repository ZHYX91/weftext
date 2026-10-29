use std::collections::BTreeMap;
use tempfile::TempDir;
use weftext_engine::*;

fn available_for_image_and_link(source: &str) {
    for name in ["image", "link"] {
        assert!(standard_binding_available(source, name), "{name}: {source}");
    }
}

#[test]
fn explicit_imports_certify_only_the_names_they_actually_bind() {
    for import in [
        r#"#import "helpers.typ": helper"#,
        r#"#import "helpers.typ": image as picture, link as target-link"#,
        r#"#import "helpers.typ": image.picture, link.helper"#,
        r#"#import "helpers.typ": tools.image as picture, tools.link as target-link"#,
        r#"#import "helpers.typ" as helpers"#,
        r#"#import ("helpers" + ".typ") as helpers"#,
        r#"#import ("helpers" + ".typ"): helper"#,
        r#"#import "helpers.typ" as helpers: helper, image as picture"#,
    ] {
        available_for_image_and_link(&format!("{import}\n\n正文😀"));
    }
    assert!(standard_binding_available(
        r#"#import "helpers.typ": image"#,
        "link"
    ));
    assert!(standard_binding_available(
        r#"#import "helpers.typ" as image"#,
        "link"
    ));
}

#[test]
fn wildcard_bare_include_and_actual_target_bindings_stay_unqualified() {
    for name in ["image", "link"] {
        for import in [
            r#"#import "helpers.typ": *"#.to_string(),
            r#"#import "helpers.typ" as helpers: *"#.to_string(),
            r#"#import "helpers.typ""#.to_string(),
            r#"#import ("helpers" + ".typ")"#.to_string(),
            r#"#include "helpers.typ""#.to_string(),
            format!("#import \"helpers.typ\": {name}"),
            format!("#import \"helpers.typ\": helper as {name}"),
            format!("#import \"helpers.typ\": tools.{name}"),
            format!("#import \"helpers.typ\": tools.helper as {name}"),
            format!("#import \"helpers.typ\" as {name}"),
            format!("#import \"helpers.typ\" as {name}: helper"),
            format!("#import \"helpers.typ\" as helpers: {name}"),
        ] {
            assert!(
                !standard_binding_available(&import, name),
                "{name}: {import}"
            );
        }
    }
}

#[test]
fn explicit_imports_do_not_bypass_source_expression_or_nested_binding_checks() {
    for name in ["image", "link"] {
        for source in [
            format!("#import {{ {name} = text; \"helpers.typ\" }} as helpers"),
            format!("#import {{ let {name} = text; \"helpers.typ\" }}: helper"),
            "#import { import \"helpers.typ\": *; \"helpers.typ\" } as helpers".to_string(),
            format!("#import \"helpers.typ\": helper\n#let {name} = text"),
            format!("#import \"helpers.typ\" as helpers\n#let local({name}) = {name}"),
            format!("#import \"helpers.typ\": helper\n#{{ let {name} = text }}"),
            format!("#import \"helpers.typ\": helper\n#({name} = text)"),
        ] {
            assert!(
                typst_syntax::Source::detached(&source)
                    .root()
                    .errors_and_warnings()
                    .0
                    .is_empty(),
                "valid negative fixture: {source}"
            );
            assert!(
                !standard_binding_available(&source, name),
                "{name}: {source}"
            );
        }
        assert!(!standard_binding_available(
            "#import \"helpers.typ\": helper as",
            name
        ));
    }
}

fn frozen(source: &str, root: &std::path::Path) -> FrozenRenderRequest {
    FrozenRenderRequest {
        source: source.into(),
        path: root.join("note.typ"),
        root: root.into(),
        files: BTreeMap::from([
            (
                "helpers.typ".into(),
                b"#let helper = 1\n#let image = 2\n#let link = 3\n#import \"nested.typ\" as tools\n"
                    .to_vec(),
            ),
            (
                "nested.typ".into(),
                b"#let picture = 4\n#let image = 5\n#let link = 6\n".to_vec(),
            ),
        ]),
        style: None,
    }
}

#[test]
fn locked_typst_evaluation_preserves_builtins_for_certified_explicit_imports() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    for import in [
        r#"#import "helpers.typ": helper"#,
        r#"#import "helpers.typ": image as picture, link as target-link"#,
        r#"#import "helpers.typ": tools.picture"#,
        r#"#import "helpers.typ": tools.image as picture, tools.link as target-link"#,
        r#"#import "helpers.typ" as helpers"#,
        r#"#import ("helpers" + ".typ") as helpers"#,
        r#"#import ("helpers" + ".typ"): helper"#,
        r#"#import "helpers.typ" as helpers: helper, image as picture"#,
    ] {
        let source = format!(
            "#let native-image = image\n#let native-link = link\n{import}\n#assert(image == native-image)\n#assert(link == native-link)\n正文😀"
        );
        available_for_image_and_link(&source);
        let result = engine.render_frozen(&frozen(&source, dir.path())).unwrap();
        assert!(result.ok, "{import}: {:?}", result.diagnostics);
    }
}

#[test]
fn locked_typst_evaluation_confirms_alias_and_item_shadowing_are_independent() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    for name in ["image", "link"] {
        for import in [
            format!("#import \"helpers.typ\": {name}"),
            format!("#import \"helpers.typ\": helper as {name}"),
            format!("#import \"helpers.typ\": tools.{name}"),
            format!("#import \"helpers.typ\" as {name}: helper"),
            format!("#import \"helpers.typ\" as helpers: {name}"),
            r#"#import "helpers.typ" as helpers: *"#.into(),
        ] {
            let source = format!(
                "#let native-original = {name}\n{import}\n#assert({name} != native-original)\n正文"
            );
            assert!(!standard_binding_available(&source, name), "{source}");
            let result = engine.render_frozen(&frozen(&source, dir.path())).unwrap();
            assert!(result.ok, "{import}: {:?}", result.diagnostics);
        }
    }
}

#[test]
fn native_image_insertion_after_explicit_import_preserves_exact_author_prefix() {
    for import in [
        r#"#import "helpers.typ": helper"#,
        r#"#import "helpers.typ": image as picture"#,
        r#"#import "helpers.typ" as helpers: helper"#,
    ] {
        let source = format!("{import}\r\n\r\n正文😀");
        let output = edit(
            &source,
            &EditRequest {
                source_hash: source_hash(&source),
                intent: EditIntent::InsertBlock {
                    at: source.len(),
                    block: BlockInsert::Image {
                        path: "assets/中文\"#图.png".into(),
                        alt: "原图😀".into(),
                    },
                },
            },
        )
        .unwrap();
        assert_eq!(output.patches.len(), 1);
        assert_eq!(
            output.source,
            format!(
                "{source}\n\n#image(\"assets/中文\\\"#图.png\", format: auto, alt: \"原图😀\")"
            )
        );
    }
}
