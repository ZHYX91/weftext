use std::process::{Command, Output};

use weftext_core::{create_workspace, read_node_document, read_workspace_revision};

#[test]
fn markdown_export_cli_separates_read_only_preview_from_exact_external_commit() {
    let temporary = tempfile::tempdir().expect("CLI export fixture");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("canonical workspace");
    let snapshot = read_node_document(&workspace).expect("root snapshot");
    let workspace_revision = read_workspace_revision(&workspace).expect("workspace revision");
    let destination = temporary.path().join("Workspace export.md");
    let bundle = temporary.path().join("export-plan.json");

    let preview = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["export", "markdown-preview"])
        .arg(&workspace)
        .arg(snapshot.node_id.to_string())
        .arg(&destination)
        .arg(&bundle)
        .output()
        .expect("Markdown export preview");
    assert_success(&preview);
    assert!(!destination.exists());
    assert!(bundle.is_file());
    assert_eq!(
        read_workspace_revision(&workspace).expect("preview revision"),
        workspace_revision
    );
    let preview_json = output_json(&preview);
    assert_eq!(preview_json["export"]["stage"], "preview");
    assert_eq!(preview_json["export"]["format"], "markdown_compatibility");
    assert_eq!(
        preview_json["export"]["plan"]["metadataPolicy"],
        "preserve_weftext"
    );
    let exact = preview_json["export"]["plan"]["artifact"]
        .as_str()
        .expect("exact Markdown artifact")
        .to_owned();

    let commit = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["export", "commit"])
        .arg(&workspace)
        .arg(&bundle)
        .output()
        .expect("Markdown export commit");
    assert_success(&commit);
    assert_eq!(output_json(&commit)["export"]["stage"], "committed");
    assert_eq!(
        std::fs::read_to_string(&destination).expect("external Markdown artifact"),
        exact
    );
    assert_eq!(
        read_workspace_revision(&workspace).expect("commit revision"),
        workspace_revision
    );
}

#[test]
fn plain_markdown_cli_option_removes_weftext_metadata_only_after_explicit_request() {
    let temporary = tempfile::tempdir().expect("plain CLI export fixture");
    let workspace = temporary.path().join("Workspace");
    create_workspace(&workspace).expect("canonical workspace");
    let snapshot = read_node_document(&workspace).expect("root snapshot");
    let destination = temporary.path().join("plain.md");
    let bundle = temporary.path().join("plain-plan.json");

    let preview = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["export", "markdown-preview"])
        .arg(&workspace)
        .arg(snapshot.node_id.to_string())
        .arg(&destination)
        .arg(&bundle)
        .arg("--remove-weftext-metadata")
        .output()
        .expect("plain Markdown preview");
    assert_success(&preview);
    let preview = output_json(&preview);
    assert_eq!(
        preview["export"]["plan"]["metadataPolicy"],
        "remove_weftext"
    );
    assert!(
        !preview["export"]["plan"]["artifact"]
            .as_str()
            .expect("plain artifact")
            .starts_with("---\nweftext:")
    );
    assert!(
        preview["export"]["plan"]["diagnostics"]
            .as_array()
            .expect("compatibility diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "weftext_metadata_removed")
    );
    assert!(!destination.exists());
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("CLI JSON stdout")
}
