use std::fs;
use std::time::Instant;

use tempfile::tempdir;
use weftext_core::{
    DocumentEdit, NodeId, SearchIndexError, commit_document_edit, create_child_node,
    create_workspace, plan_document_edit, read_node_document, rebuild_workspace_search_index,
    refresh_workspace_search_index, refresh_workspace_search_index_invalidating,
    search_workspace_index,
};

fn replace_source(node: &std::path::Path, transform: impl FnOnce(String) -> String) {
    let snapshot = read_node_document(node).expect("read source");
    let source = transform(snapshot.source.clone());
    let plan = plan_document_edit(
        node,
        &snapshot.revision,
        [DocumentEdit {
            start: 0,
            end: u64::try_from(snapshot.source.len()).expect("source length"),
            replacement: source,
        }],
    )
    .expect("plan source replacement");
    commit_document_edit(&plan).expect("commit source replacement");
}

#[test]
fn external_index_rebuild_refresh_and_delete_are_derived_and_incremental() {
    let temporary = tempdir().expect("temporary directory");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");
    let alpha = create_child_node(&workspace, "Alpha").expect("alpha");
    let beta = create_child_node(&workspace, "Beta").expect("beta");
    replace_source(&alpha.path, |source| {
        source.replacen("weftext:\n", "weftext:\n  icon: '😀'\n", 1)
            + "= Alpha\n:keywords: indexed-property\n\nVisible alpha body\n"
    });
    replace_source(&beta.path, |source| source + "Visible beta body\n");

    let index = temporary.path().join("derived").join("search-v1.json");
    let rebuilt = rebuild_workspace_search_index(&workspace, &index).expect("rebuild");
    assert_eq!(rebuilt.entries, 3);
    assert_eq!(rebuilt.reparsed_documents, 3);
    let alpha_results = search_workspace_index(&index, "indexed-property").expect("search");
    assert_eq!(alpha_results.len(), 1);
    assert_eq!(
        alpha_results[0]
            .icon
            .as_ref()
            .map(|icon| icon.glyph.as_str()),
        Some("😀")
    );
    assert!(
        search_workspace_index(&index, "weftext")
            .expect("hidden search")
            .is_empty()
    );

    let unchanged = refresh_workspace_search_index(&workspace, &index).expect("unchanged refresh");
    assert_eq!(unchanged.reparsed_documents, 0);
    assert_eq!(unchanged.reused_documents, 3);

    let forced = refresh_workspace_search_index_invalidating(&workspace, &index, [alpha.id])
        .expect("known commit invalidation");
    assert_eq!(forced.reparsed_documents, 1);
    assert_eq!(forced.reused_documents, 2);

    replace_source(&beta.path, |source| source + "Changed beta token\n");
    let changed = refresh_workspace_search_index(&workspace, &index).expect("delta refresh");
    assert_eq!(changed.reparsed_documents, 1);
    assert_eq!(changed.reused_documents, 2);
    assert_eq!(
        search_workspace_index(&index, "Changed beta token")
            .expect("changed search")
            .len(),
        1
    );

    fs::remove_dir_all(&beta.path).expect("delete beta authority");
    let deleted = refresh_workspace_search_index(&workspace, &index).expect("delete refresh");
    assert_eq!(deleted.entries, 2);
    assert!(
        search_workspace_index(&index, "Changed beta token")
            .expect("deleted search")
            .is_empty()
    );

    assert!(matches!(
        rebuild_workspace_search_index(&workspace, &workspace.join("forbidden-index.json")),
        Err(SearchIndexError::IndexInsideWorkspace)
    ));
}

#[test]
fn external_index_rejects_dot_dot_and_filesystem_aliases_into_workspace() {
    let temporary = tempdir().expect("temporary directory");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("workspace");

    let dot_dot = workspace
        .join("..")
        .join("Workspace")
        .join("not-created")
        .join("index.json");
    assert!(matches!(
        rebuild_workspace_search_index(&workspace, &dot_dot),
        Err(SearchIndexError::IndexInsideWorkspace)
    ));

    let alias = temporary.path().join("workspace-alias");
    create_directory_alias(&workspace, &alias);
    assert!(matches!(
        rebuild_workspace_search_index(&workspace, &alias.join("cache").join("index.json")),
        Err(SearchIndexError::IndexInsideWorkspace)
    ));
}

#[cfg(unix)]
fn create_directory_alias(target: &std::path::Path, alias: &std::path::Path) {
    std::os::unix::fs::symlink(target, alias).expect("workspace symlink");
}

#[cfg(windows)]
fn create_directory_alias(target: &std::path::Path, alias: &std::path::Path) {
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(alias)
        .arg(target)
        .status()
        .expect("launch mklink");
    assert!(status.success(), "create workspace junction");
}

#[test]
fn ten_thousand_node_real_filesystem_index_baseline() {
    const CHILDREN: usize = 10_000;
    let temporary = tempdir().expect("temporary directory");
    let workspace = temporary.path().join("LargeWorkspace");
    create_workspace(&workspace).expect("workspace");
    for index in 0..CHILDREN {
        let name = format!("Node-{index:05}");
        let directory = workspace.join(&name);
        fs::create_dir(&directory).expect("node directory");
        let id = NodeId::new_v4();
        fs::write(
            directory.join(format!("{name}.adoc")),
            format!(
                "---\nweftext:\n  id: \"{id}\"\n---\n= {name}\n\nRepresentative body {index:05}\n"
            ),
        )
        .expect("node document");
    }
    let index_path = temporary.path().join("cache").join("large-search.json");
    let started = Instant::now();
    let stats = rebuild_workspace_search_index(&workspace, &index_path).expect("large rebuild");
    let rebuild_elapsed = started.elapsed();
    assert_eq!(stats.entries, CHILDREN + 1);
    let query_started = Instant::now();
    let results =
        search_workspace_index(&index_path, "Representative body 09999").expect("large query");
    let query_elapsed = query_started.elapsed();
    assert_eq!(results.len(), 1);
    eprintln!(
        "10k search baseline: rebuild={rebuild_elapsed:?}, query={query_elapsed:?}, bytes={}",
        fs::metadata(index_path).expect("index metadata").len()
    );
}
