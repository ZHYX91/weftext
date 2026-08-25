//! Read-only Model Context Protocol tools over a scoped Weftext workspace.

mod supervised;

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::{Value, json};
use weftext_core::{
    NodeId, WorkspaceIndex, active_document_profile, read_node_document, scan_workspace,
};

pub use supervised::{
    SupervisedMcpAgentEndpoint, SupervisedMcpAuthority, SupervisedMcpController,
    SupervisedMcpError, SupervisedMcpServer, SupervisedMcpSession, SupervisedRuntimeContract,
    SupervisedSessionConfig, SupervisedSessionEvent, SupervisedSessionEventBatch,
    SupervisedSessionEventKind,
};

/// MCP revision tested with the first-party DSH client composition.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const MCP_PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const SERVER_NAME: &str = "weftext-readonly";
const INVENTORY_TOOL: &str = "workspace_inventory";
const READ_DOCUMENT_TOOL: &str = "read_document";

/// A process-local MCP server whose only authority is one explicit workspace root.
#[derive(Debug)]
pub struct ReadOnlyMcpServer {
    workspace_root: PathBuf,
    initialized: bool,
    ready: bool,
}

impl ReadOnlyMcpServer {
    /// Validates the granted workspace before accepting MCP requests.
    ///
    /// # Errors
    ///
    /// Returns an error unless `workspace_root` is currently a valid Weftext workspace.
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, String> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        require_valid_inventory(&workspace_root)?;
        Ok(Self {
            workspace_root,
            initialized: false,
            ready: false,
        })
    }

    /// Serves newline-delimited MCP JSON-RPC until standard input closes.
    ///
    /// # Errors
    ///
    /// Returns an error when protocol input or output cannot be read or written.
    pub fn serve(&mut self, input: impl BufRead, mut output: impl Write) -> Result<(), String> {
        for line in input.lines() {
            let line = line.map_err(|error| format!("MCP input failed: {error}"))?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(response) = self.handle_line(&line) {
                serde_json::to_writer(&mut output, &response)
                    .map_err(|error| format!("MCP output serialization failed: {error}"))?;
                output
                    .write_all(b"\n")
                    .and_then(|()| output.flush())
                    .map_err(|error| format!("MCP output failed: {error}"))?;
            }
        }
        Ok(())
    }

    fn handle_line(&mut self, line: &str) -> Option<Value> {
        let message = match serde_json::from_str::<Value>(line) {
            Ok(message) => message,
            Err(error) => {
                return Some(rpc_error(
                    &Value::Null,
                    -32700,
                    &format!("invalid JSON: {error}"),
                ));
            }
        };
        let Some(object) = message.as_object() else {
            return Some(rpc_error(&Value::Null, -32600, "invalid JSON-RPC request"));
        };
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(rpc_error(&Value::Null, -32600, "invalid JSON-RPC version"));
        }
        let method = object.get("method").and_then(Value::as_str);
        let id = object.get("id").cloned();
        let params = object.get("params").cloned().unwrap_or_else(|| json!({}));

        if id.is_none() {
            self.handle_notification(method, &params);
            return None;
        }
        let id = id.unwrap_or(Value::Null);
        let Some(method) = method else {
            return Some(rpc_error(&id, -32600, "request method is required"));
        };
        match method {
            "initialize" => Some(self.initialize(&id, &params)),
            "ping" => Some(rpc_result(&id, &json!({}))),
            "tools/list" if self.ready => Some(rpc_result(&id, &tools_list())),
            "tools/call" if self.ready => Some(self.call_tool(&id, &params)),
            "tools/list" | "tools/call" => {
                Some(rpc_error(&id, -32002, "MCP server is not initialized"))
            }
            _ => Some(rpc_error(&id, -32601, "method not found")),
        }
    }

    fn handle_notification(&mut self, method: Option<&str>, _params: &Value) {
        if method == Some("notifications/initialized") && self.initialized {
            self.ready = true;
        }
    }

    fn initialize(&mut self, id: &Value, params: &Value) -> Value {
        if self.initialized {
            return rpc_error(id, -32600, "MCP server is already initialized");
        }
        let Some(requested_version) = params.get("protocolVersion").and_then(Value::as_str) else {
            return rpc_error(id, -32602, "invalid initialize parameters");
        };
        if params
            .get("capabilities")
            .and_then(Value::as_object)
            .is_none()
            || params
                .get("clientInfo")
                .and_then(Value::as_object)
                .is_none()
        {
            return rpc_error(id, -32602, "invalid initialize parameters");
        }
        let protocol_version = if MCP_PROTOCOL_VERSIONS.contains(&requested_version) {
            requested_version
        } else {
            MCP_PROTOCOL_VERSION
        };
        self.initialized = true;
        rpc_result(
            id,
            &json!({
                "protocolVersion": protocol_version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {
                    "name": SERVER_NAME,
                    "title": "Weftext read-only workspace tools",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "instructions": "Read-only Weftext context. No mutation, shell, filesystem path, or external-egress tool is exposed.",
            }),
        )
    }

    fn call_tool(&self, id: &Value, params: &Value) -> Value {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return rpc_error(id, -32602, "tool name is required");
        };
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        match name {
            INVENTORY_TOOL => match self.inventory_result(&arguments) {
                Ok(result) => rpc_result(id, &tool_success(&result)),
                Err(error) => rpc_result(id, &tool_failure(&error)),
            },
            READ_DOCUMENT_TOOL => match self.document_result(&arguments) {
                Ok(result) => rpc_result(id, &tool_success(&result)),
                Err(error) => rpc_result(id, &tool_failure(&error)),
            },
            _ => rpc_error(id, -32602, "unknown Weftext tool"),
        }
    }

    fn inventory_result(&self, arguments: &Value) -> Result<Value, String> {
        require_empty_arguments(arguments)?;
        let inventory = require_valid_inventory(&self.workspace_root)?;
        let nodes = inventory
            .nodes
            .iter()
            .map(|node| {
                let id = node
                    .id
                    .ok_or_else(|| "workspace is not safe to read".to_owned())?;
                Ok(json!({
                    "nodeId": id,
                    "name": node.name,
                    "relativePath": relative_node_path(&self.workspace_root, &node.path)?,
                    "parentId": node.parent_id,
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(json!({
            "schema": "weftext.agent.read.v1",
            "scope": "workspace",
            "nodeCount": nodes.len(),
            "nodes": nodes,
        }))
    }

    fn document_result(&self, arguments: &Value) -> Result<Value, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "read_document arguments must be an object".to_owned())?;
        if object.len() != 1 {
            return Err("read_document accepts only nodeId".to_owned());
        }
        let node_id = object
            .get("nodeId")
            .and_then(Value::as_str)
            .ok_or_else(|| "read_document requires nodeId".to_owned())?;
        let node_id = NodeId::from_str(node_id).map_err(|error| error.to_string())?;
        let inventory = require_valid_inventory(&self.workspace_root)?;
        let index = WorkspaceIndex::rebuild(&inventory)
            .map_err(|_| "workspace is not safe to read".to_owned())?;
        let node_path = index
            .path_for(node_id)
            .ok_or_else(|| "node is outside the granted workspace or unavailable".to_owned())?;
        let snapshot = read_node_document(node_path).map_err(|error| error.to_string())?;
        Ok(json!({
            "schema": "weftext.agent.read.v1",
            "document": {
                "nodeId": snapshot.node_id,
                "name": node_path.file_name().and_then(|name| name.to_str()).unwrap_or("node"),
                "relativePath": relative_node_path(&self.workspace_root, node_path)?,
                "revision": snapshot.revision,
                "length": snapshot.source.len(),
                "profile": active_document_profile(),
                "source": snapshot.source,
            }
        }))
    }
}

/// Runs the scoped server over process standard input/output.
///
/// # Errors
///
/// Returns an error when the workspace is invalid or the protocol stream fails.
pub fn serve_stdio(workspace_root: impl AsRef<Path>) -> Result<(), String> {
    let mut server = ReadOnlyMcpServer::new(workspace_root)?;
    let input = std::io::stdin();
    let output = std::io::stdout();
    server.serve(input.lock(), output.lock())
}

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": INVENTORY_TOOL,
                "title": "List scoped Weftext nodes",
                "description": "List node identities, names, relative paths, and parent identities in the granted Weftext workspace. This tool is read-only and never returns absolute filesystem paths.",
                "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            },
            {
                "name": READ_DOCUMENT_TOOL,
                "title": "Read one scoped Weftext document",
                "description": "Read the exact UTF-8 managed-document source, active profile, and revision for one node identity from the granted workspace. The node must be selected by UUID, never by filesystem path. This tool cannot modify content.",
                "inputSchema": {
                    "type": "object",
                    "properties": {"nodeId": {"type": "string", "description": "Canonical lowercase UUIDv4 returned by workspace_inventory"}},
                    "required": ["nodeId"],
                    "additionalProperties": false,
                },
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            }
        ]
    })
}

fn require_valid_inventory(
    workspace_root: &Path,
) -> Result<weftext_core::WorkspaceInventory, String> {
    let inventory = scan_workspace(workspace_root);
    if inventory.is_valid() {
        Ok(inventory)
    } else {
        let issue = inventory
            .issues
            .first()
            .map_or("empty workspace".to_owned(), |issue| {
                format!("{:?}", issue.code)
            });
        Err(format!("workspace is not safe to read: {issue}"))
    }
}

fn require_empty_arguments(arguments: &Value) -> Result<(), String> {
    match arguments.as_object() {
        Some(object) if object.is_empty() => Ok(()),
        _ => Err("workspace_inventory accepts no arguments".to_owned()),
    }
}

fn relative_node_path(root: &Path, node: &Path) -> Result<String, String> {
    let relative = node
        .strip_prefix(root)
        .map_err(|_| "node is outside the granted workspace".to_owned())?;
    let text = relative.to_string_lossy().replace('\\', "/");
    Ok(if text.is_empty() {
        ".".to_owned()
    } else {
        text
    })
}

fn tool_success(structured: &Value) -> Value {
    let text = serde_json::to_string_pretty(&structured)
        .unwrap_or_else(|_| "Weftext read result could not be rendered".to_owned());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false,
    })
}

fn tool_failure(message: &str) -> Value {
    json!({
        "content": [{"type": "text", "text": message}],
        "isError": true,
    })
}

fn rpc_result(id: &Value, result: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn rpc_error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
