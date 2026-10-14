use tempfile::TempDir;
use weftext_engine::{Engine, certify_metadata_declaration};

const METADATA: &str = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"paper\",), publication:(title:\"研究标题\"))) <weftext>";

#[test]
fn metadata_role_query_values_are_not_duplicate_declarations() {
    let source =
        format!("{METADATA}\n#context title(query(<weftext>).first().value.publication.title)");
    assert!(certify_metadata_declaration(&source, "weftext").is_some());
    let dir = TempDir::new().unwrap();
    let engine = Engine::new_with_system_fonts(false).unwrap();
    let out = engine
        .render(&source, &dir.path().join("note.typ"), dir.path())
        .unwrap();
    assert!(out.ok, "{:?}", out.diagnostics);
    assert_eq!(
        out.labels
            .iter()
            .find(|label| label.label == "weftext")
            .unwrap()
            .count,
        1
    );
    assert_eq!(out.products.unwrap().native_title_bodies, ["研究标题"]);
    for expression in [
        "#let role = <weftext>",
        "#show <weftext>: it=>it",
        "#context query(<weftext>)",
    ] {
        assert!(
            certify_metadata_declaration(&format!("{METADATA}\n{expression}"), "weftext").is_some()
        );
    }
}

#[test]
fn real_role_attachments_remain_ambiguous_and_generated_metadata_is_not_promoted() {
    for source in [
        format!("{METADATA}\n{METADATA}"),
        format!("{METADATA}\n= 标题 <weftext>"),
        format!("{METADATA}\n#if true [{METADATA}]"),
        format!("#if true [{METADATA}]"),
        format!("#let generated = [{METADATA}]\n#generated"),
    ] {
        assert!(
            certify_metadata_declaration(&source, "weftext").is_none(),
            "{source}"
        );
    }
}
