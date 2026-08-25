use std::io::{BufReader, Cursor};

use tempfile::tempdir;
use weftext_agent_mcp::ReadOnlyMcpServer;
use weftext_core::{create_child_node, create_workspace, read_node_document};

#[test]
fn mcp_lists_only_read_tools_and_reads_by_scoped_node_identity() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let root = create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let child_source = read_node_document(&child.path).unwrap().source;

    let input = format!(
        "{}\n{}\n{}\n{}\n{}\n",
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"workspace_inventory","arguments":{}}}),
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_document","arguments":{"nodeId":child.id.to_string()}}}),
    );
    let mut output = Vec::new();
    ReadOnlyMcpServer::new(&workspace)
        .unwrap()
        .serve(BufReader::new(Cursor::new(input)), &mut output)
        .unwrap();
    let responses = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(responses.len(), 4);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-06-18");
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 2);
    assert!(
        tools
            .iter()
            .all(|tool| tool["annotations"]["readOnlyHint"] == true)
    );
    assert!(tools.iter().all(|tool| {
        !tool["name"].as_str().unwrap().contains("write")
            && !tool["name"].as_str().unwrap().contains("commit")
    }));
    assert_eq!(responses[2]["result"]["structuredContent"]["nodeCount"], 2);
    assert_eq!(
        responses[3]["result"]["structuredContent"]["document"]["source"],
        child_source
    );
    assert_eq!(
        responses[3]["result"]["structuredContent"]["document"]["relativePath"],
        "Child"
    );
    assert_eq!(
        responses[3]["result"]["structuredContent"]["document"]["profile"]["profile"],
        "ascii_doc_v1"
    );
    assert!(
        !responses[3]
            .to_string()
            .contains(&workspace.display().to_string())
    );
    assert_ne!(root.id, child.id);
}

#[test]
fn mcp_refuses_calls_before_initialization_and_out_of_scope_ids() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let outside = create_workspace(temporary.path().join("Outside")).unwrap();

    let input = format!(
        "{}\n{}\n{}\n{}\n",
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_document","arguments":{"nodeId":outside.id.to_string()}}}),
    );
    let mut output = Vec::new();
    ReadOnlyMcpServer::new(&workspace)
        .unwrap()
        .serve(BufReader::new(Cursor::new(input)), &mut output)
        .unwrap();
    let responses = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(responses[0]["error"]["code"], -32002);
    assert_eq!(responses[2]["result"]["isError"], true);
    assert_eq!(
        responses[2]["result"]["content"][0]["text"],
        "node is outside the granted workspace or unavailable"
    );
}
