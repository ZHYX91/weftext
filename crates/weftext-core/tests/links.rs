use std::fs;

use tempfile::tempdir;
use weftext_core::{
    LinkMatchQuality, build_workspace_link_index, create_child_node, create_workspace,
};

fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    (temporary, workspace)
}

fn append_document(node: &std::path::Path, text: &str) {
    let name = node.file_name().unwrap().to_str().unwrap();
    let path = node.join(format!("{name}.adoc"));
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str(text);
    fs::write(path, source).unwrap();
}

#[test]
fn outgoing_uuid_links_keep_source_order_and_exclude_code() {
    let (_temporary, workspace) = setup();
    let alpha = create_child_node(&workspace, "Alpha").unwrap();
    let beta = create_child_node(&workspace, "Beta").unwrap();
    append_document(&beta.path, "\nAliases below are user metadata only.\n");
    let beta_document = beta.document_path;
    let source = fs::read_to_string(&beta_document).unwrap();
    let source = source.replacen("weftext:\n", "weftext:\n  aliases:\n    - Bee\n", 1);
    fs::write(beta_document, source).unwrap();
    append_document(
        &workspace,
        &format!(
            "\nnode:{}[Alpha] then node::{}#body[shown].\n\n========== Deep node:{}[Alpha]\n\n+node:{}[Beta]+\n\n[source]\n----\nnode:{}[Alpha]\n----\n",
            alpha.id, beta.id, alpha.id, beta.id, alpha.id
        ),
    );

    let index = build_workspace_link_index(&workspace).unwrap();
    let root_id = index
        .nodes
        .iter()
        .find(|node| node.name == "Notes")
        .unwrap()
        .id;
    let outgoing = index
        .outgoing
        .iter()
        .filter(|link| link.source_node_id == root_id)
        .collect::<Vec<_>>();
    assert_eq!(outgoing.len(), 3);
    assert!(outgoing[0].start < outgoing[1].start);
    assert_eq!(outgoing[0].target_node_ids, vec![alpha.id]);
    assert_eq!(outgoing[0].canonical_locator.as_deref(), Some("Alpha"));
    assert_eq!(outgoing[1].target_node_ids, vec![beta.id]);
    assert_eq!(outgoing[1].fragment.as_deref(), Some("body"));
    assert_eq!(outgoing[1].display_text.as_deref(), Some("shown"));
    assert_eq!(outgoing[2].target_node_ids, vec![alpha.id]);
}

#[test]
fn longest_overlapping_potential_mention_is_first_and_primary() {
    let (_temporary, workspace) = setup();
    let ab = create_child_node(&workspace, "AB").unwrap();
    let long = create_child_node(&workspace, "Long").unwrap();
    let long_source = fs::read_to_string(&long.document_path).unwrap();
    fs::write(
        &long.document_path,
        long_source.replacen("weftext:\n", "weftext:\n  aliases:\n    - ABC\n", 1),
    )
    .unwrap();
    append_document(&workspace, "\nA sentence contains ABC.\n");

    let index = build_workspace_link_index(&workspace).unwrap();
    let root_id = index
        .nodes
        .iter()
        .find(|node| node.name == "Notes")
        .unwrap()
        .id;
    let mentions = index
        .potential_mentions
        .iter()
        .filter(|mention| mention.source_node_id == root_id)
        .collect::<Vec<_>>();
    let abc = mentions
        .iter()
        .position(|mention| mention.matched_text == "ABC")
        .unwrap();
    let ab_position = mentions
        .iter()
        .position(|mention| mention.matched_text == "AB")
        .unwrap();
    assert!(abc < ab_position);
    assert!(mentions[abc].primary);
    assert!(!mentions[ab_position].primary);
    assert_eq!(mentions[abc].quality, LinkMatchQuality::ExactAlias);
    assert_eq!(mentions[abc].target_node_ids, vec![long.id]);
    assert_eq!(mentions[ab_position].target_node_ids, vec![ab.id]);
}

#[test]
fn ambiguous_aliases_remain_one_candidate_group() {
    let (_temporary, workspace) = setup();
    let first = create_child_node(&workspace, "First").unwrap();
    let second = create_child_node(&workspace, "Second").unwrap();
    for document in [&first.document_path, &second.document_path] {
        let source = fs::read_to_string(document).unwrap();
        fs::write(
            document,
            source.replacen("weftext:\n", "weftext:\n  aliases:\n    - Shared\n", 1),
        )
        .unwrap();
    }
    append_document(&workspace, "\nShared appears here.\n");
    let index = build_workspace_link_index(&workspace).unwrap();
    let root_id = index
        .nodes
        .iter()
        .find(|node| node.name == "Notes")
        .unwrap()
        .id;
    let shared = index
        .potential_mentions
        .iter()
        .find(|mention| mention.source_node_id == root_id && mention.matched_text == "Shared")
        .unwrap();
    assert_eq!(shared.target_node_ids.len(), 2);
    assert!(shared.target_node_ids.contains(&first.id));
    assert!(shared.target_node_ids.contains(&second.id));
}

#[test]
fn outgoing_node_link_uses_the_decoded_closed_label_codec() {
    let (_temporary, workspace) = setup();
    let target = create_child_node(&workspace, "Target").unwrap();
    let display = format!(
        "\\[]:,\" 中文 مرحبا 😀 node:{}[] xref:local[inner] image::pic.png[] https://example.test",
        target.id
    );
    let encoded = weftext_asciidoc::encode_node_link_label(&display).unwrap();
    append_document(&workspace, &format!("\nnode:{}[{encoded}]\n", target.id));

    let index = build_workspace_link_index(&workspace).unwrap();
    let root_id = index
        .nodes
        .iter()
        .find(|node| node.name == "Notes")
        .unwrap()
        .id;
    let outgoing = index
        .outgoing
        .iter()
        .filter(|link| link.source_node_id == root_id)
        .collect::<Vec<_>>();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].target_node_ids, [target.id]);
    assert_eq!(outgoing[0].display_text.as_deref(), Some(display.as_str()));
}
