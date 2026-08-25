use serde_json::{Value, json};
use weftext_core::NodeId;
use weftext_export::{
    MarkdownMetadataPolicy, commit_markdown_export, preview_markdown_export,
    read_markdown_export_bundle, write_markdown_export_bundle,
};
use weftext_intake::rfc3339_utc_now;

pub(crate) fn run(arguments: &[String], schema: &str) -> Result<Value, String> {
    match arguments {
        [scope, command, workspace, node_id, destination, bundle]
            if scope == "export" && command == "markdown-preview" =>
        {
            preview(
                workspace,
                node_id,
                destination,
                bundle,
                MarkdownMetadataPolicy::PreserveWeftext,
                schema,
            )
        }
        [scope, command, workspace, node_id, destination, bundle, remove]
            if scope == "export"
                && command == "markdown-preview"
                && remove == "--remove-weftext-metadata" =>
        {
            preview(
                workspace,
                node_id,
                destination,
                bundle,
                MarkdownMetadataPolicy::RemoveWeftext,
                schema,
            )
        }
        [scope, command, workspace, bundle] if scope == "export" && command == "commit" => {
            commit(workspace, bundle, schema)
        }
        _ => Err(
            "usage: weftext export markdown-preview <workspace> <node-id> <destination.md> <bundle.json> [--remove-weftext-metadata] | weftext export commit <workspace> <bundle.json>"
                .to_owned(),
        ),
    }
}

fn preview(
    workspace: &str,
    node_id: &str,
    destination: &str,
    bundle_path: &str,
    metadata_policy: MarkdownMetadataPolicy,
    schema: &str,
) -> Result<Value, String> {
    let node_id = node_id
        .parse::<NodeId>()
        .map_err(|error| error.to_string())?;
    let plan = preview_markdown_export(workspace, node_id, destination, metadata_policy)
        .map_err(|error| error.to_string())?;
    write_markdown_export_bundle(workspace, bundle_path, &plan)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "export": {
            "stage": "preview",
            "format": "markdown_compatibility",
            "bundlePath": bundle_path,
            "plan": plan,
        }
    }))
}

fn commit(workspace: &str, bundle_path: &str, schema: &str) -> Result<Value, String> {
    let plan = read_markdown_export_bundle(bundle_path).map_err(|error| error.to_string())?;
    let receipt = commit_markdown_export(
        workspace,
        &plan,
        rfc3339_utc_now().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "export": {
            "stage": "committed",
            "format": "markdown_compatibility",
            "receipt": receipt,
        }
    }))
}
