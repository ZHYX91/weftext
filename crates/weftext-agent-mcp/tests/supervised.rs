use std::io::{BufReader, Cursor};

use tempfile::tempdir;
use weftext_agent::{AgentBrokerConfig, AgentCapability, AgentOrigin, CapabilityGrant};
use weftext_agent_mcp::{SupervisedMcpAuthority, SupervisedMcpServer};
use weftext_core::{
    WorkspaceNodeProjection, WorkspaceReadScope, create_child_node, create_workspace,
    read_node_document,
};

fn read_capabilities() -> CapabilityGrant {
    CapabilityGrant::new([
        AgentCapability::ReadWorkspace,
        AgentCapability::SearchWorkspace,
    ])
}

fn authority() -> SupervisedMcpAuthority {
    SupervisedMcpAuthority {
        human_actor_id: "local-human".to_owned(),
        delegated_client_id: "desktop-dsh".to_owned(),
        workspace_scope_id: "workspace:child".to_owned(),
        origin: AgentOrigin {
            harness: "dsh".to_owned(),
            adapter_version: "1.0.0".to_owned(),
            session_id: "session-1".to_owned(),
        },
        actor_capabilities: read_capabilities(),
        delegated_session_capabilities: read_capabilities(),
        workspace_policy_capabilities: read_capabilities(),
    }
}

fn server_for(workspace: &std::path::Path, child_id: weftext_core::NodeId) -> SupervisedMcpServer {
    let scope = WorkspaceReadScope::new([WorkspaceNodeProjection::new(child_id, None, "Child")])
        .expect("scope is valid");
    SupervisedMcpServer::new(workspace, scope, authority(), AgentBrokerConfig::default())
        .expect("supervised server starts")
}

fn exchange(
    server: &mut SupervisedMcpServer,
    messages: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let input = messages
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut output = Vec::new();
    server
        .serve(BufReader::new(Cursor::new(input)), &mut output)
        .expect("MCP exchange succeeds");
    String::from_utf8(output)
        .expect("MCP output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("response is JSON"))
        .collect()
}

fn initialize(server: &mut SupervisedMcpServer) -> serde_json::Value {
    let responses = exchange(
        server,
        &[
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        ],
    );
    assert_eq!(responses.len(), 2);
    responses[1].clone()
}

#[test]
fn supervised_catalog_is_read_only_and_raw_document_edit_names_are_absent() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let initial = read_node_document(&child.path).unwrap();
    let mut server = server_for(&workspace, child.id);

    let catalog = initialize(&mut server);
    let tools = catalog["result"]["tools"].as_array().unwrap();
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        ["workspace_inventory", "read_document", "mutation_status"]
    );
    assert!(
        tools
            .iter()
            .all(|tool| tool["annotations"]["readOnlyHint"] == true)
    );
    let encoded_catalog = catalog.to_string();
    for forbidden in [
        "propose_document_edits",
        "cancel_mutation",
        "commit_mutation",
        "replacement",
        "baseRevision",
    ] {
        assert!(!encoded_catalog.contains(forbidden));
    }

    for retired_name in [
        "propose_document_edits",
        "cancel_mutation",
        "commit_mutation",
        "write_document",
        "apply_patch",
    ] {
        let response = exchange(
            &mut server,
            &[serde_json::json!({
                "jsonrpc":"2.0",
                "id":10,
                "method":"tools/call",
                "params":{"name":retired_name,"arguments":{}}
            })],
        )
        .remove(0);
        assert_eq!(response["error"]["code"], -32602, "{response:#}");
    }
    assert_eq!(
        read_node_document(&child.path).unwrap().source,
        initial.source,
        "retired tool names must never write"
    );
}

#[test]
fn scoped_reads_remain_exact_and_revocation_fails_closed() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let outside = create_child_node(&workspace, "Outside").unwrap();
    let initial = read_node_document(&child.path).unwrap();
    let outside_snapshot = read_node_document(&outside.path).unwrap();
    let mut server = server_for(&workspace, child.id);
    initialize(&mut server);

    let read = exchange(
        &mut server,
        &[serde_json::json!({
            "jsonrpc":"2.0","id":20,"method":"tools/call",
            "params":{"name":"read_document","arguments":{"nodeId":child.id}}
        })],
    )
    .remove(0);
    assert_eq!(read["result"]["isError"], false);
    assert_eq!(
        read["result"]["structuredContent"]["document"]["source"],
        initial.source
    );
    assert!(!read.to_string().contains(&workspace.display().to_string()));

    let denied_outside = exchange(
        &mut server,
        &[serde_json::json!({
            "jsonrpc":"2.0","id":21,"method":"tools/call",
            "params":{"name":"read_document","arguments":{"nodeId":outside.id}}
        })],
    )
    .remove(0);
    assert_eq!(denied_outside["result"]["isError"], true);
    assert!(
        !denied_outside
            .to_string()
            .contains(&outside_snapshot.source)
    );

    server.update_capability_grants(
        CapabilityGrant::default(),
        CapabilityGrant::default(),
        CapabilityGrant::default(),
    );
    let denied_after_revocation = exchange(
        &mut server,
        &[serde_json::json!({
            "jsonrpc":"2.0","id":22,"method":"tools/call",
            "params":{"name":"read_document","arguments":{"nodeId":child.id}}
        })],
    )
    .remove(0);
    assert_eq!(denied_after_revocation["result"]["isError"], true);
    assert!(
        !denied_after_revocation
            .to_string()
            .contains(&initial.source)
    );
    assert_eq!(
        read_node_document(&child.path).unwrap().source,
        initial.source
    );
}

#[test]
fn status_is_read_only_recovery_evidence_and_cannot_create_work() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let initial = read_node_document(&child.path).unwrap();
    let mut server = server_for(&workspace, child.id);
    initialize(&mut server);

    let status = exchange(
        &mut server,
        &[serde_json::json!({
            "jsonrpc":"2.0","id":30,"method":"tools/call",
            "params":{"name":"mutation_status","arguments":{"requestId":"not-created"}}
        })],
    )
    .remove(0);
    assert_eq!(status["result"]["isError"], true);
    assert!(
        !status
            .to_string()
            .contains(&workspace.display().to_string())
    );
    assert_eq!(
        read_node_document(&child.path).unwrap().source,
        initial.source
    );
}
