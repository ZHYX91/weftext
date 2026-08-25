use std::io::{BufReader, Cursor};

use tempfile::tempdir;
use weftext_agent::{
    AgentAuditConfig, AgentAuditError, AgentBrokerConfig, AgentCapability, AgentOrigin,
    AgentRuntimeEvent, AgentSessionStatus, CancellationMode, CapabilityGrant,
};
use weftext_agent_mcp::{
    SupervisedMcpAgentEndpoint, SupervisedMcpAuthority, SupervisedMcpError, SupervisedMcpSession,
    SupervisedSessionConfig, SupervisedSessionEventKind,
};
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

fn session_config() -> SupervisedSessionConfig {
    SupervisedSessionConfig::dsh_wire_0_0_1(
        AgentBrokerConfig::default(),
        AgentAuditConfig::default(),
    )
}

fn scope(node_id: weftext_core::NodeId) -> WorkspaceReadScope {
    WorkspaceReadScope::new([WorkspaceNodeProjection::new(node_id, None, "Child")]).unwrap()
}

fn exchange(
    endpoint: &SupervisedMcpAgentEndpoint,
    messages: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let input = messages
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut output = Vec::new();
    endpoint
        .serve(BufReader::new(Cursor::new(input)), &mut output)
        .unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
#[allow(clippy::too_many_lines)]
fn audited_session_keeps_workspace_access_read_only_and_runtime_events_typed() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let audit = temporary.path().join("agent-control/session-1");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let initial = read_node_document(&child.path).unwrap();

    let session = SupervisedMcpSession::open(
        &workspace,
        scope(child.id),
        authority(),
        &audit,
        session_config(),
    )
    .unwrap();
    let (endpoint, controller) = session.into_parts();
    let responses = exchange(
        &endpoint,
        &[
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_document","arguments":{"nodeId":child.id}}}),
            serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"propose_document_edits","arguments":{}}}),
        ],
    );
    assert_eq!(responses.len(), 4);
    let names = responses[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        ["workspace_inventory", "read_document", "mutation_status"]
    );
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(
        responses[2]["result"]["structuredContent"]["document"]["source"],
        initial.source
    );
    assert_eq!(responses[3]["error"]["code"], -32602);

    controller
        .update_capability_grants(
            read_capabilities(),
            read_capabilities(),
            read_capabilities(),
        )
        .unwrap();
    controller
        .ingest_runtime_event(AgentRuntimeEvent::SessionStatus {
            session_id: "session-1".to_owned(),
            status: AgentSessionStatus::Running,
        })
        .unwrap();
    assert!(matches!(
        controller.ingest_runtime_event(AgentRuntimeEvent::SessionStatus {
            session_id: "foreign-session".to_owned(),
            status: AgentSessionStatus::Idle,
        }),
        Err(SupervisedMcpError::InvalidScope)
    ));
    controller
        .record_adapter_crash("runtime_transport_closed".to_owned())
        .unwrap();
    controller
        .record_runtime_terminated_for_cancellation()
        .unwrap();
    let contract = controller.runtime_contract().unwrap();
    assert_eq!(contract.wire_version, "0.0.1");
    assert!(!contract.resume_supported);
    assert_eq!(contract.cancellation, CancellationMode::RuntimeTermination);

    let events = controller.events_after(0).unwrap();
    assert!(events.events.iter().any(|event| matches!(
        event.event,
        SupervisedSessionEventKind::CapabilitiesUpdated { .. }
    )));
    assert!(
        events
            .events
            .iter()
            .any(|event| matches!(event.event, SupervisedSessionEventKind::Runtime { .. }))
    );
    assert!(events.events.iter().any(|event| matches!(
        event.event,
        SupervisedSessionEventKind::AdapterCrashed { .. }
    )));
    assert!(events.events.iter().any(|event| matches!(
        event.event,
        SupervisedSessionEventKind::RuntimeTerminatedForCancellation {
            resume_supported: false,
            ..
        }
    )));
    assert!(controller.recovery_states().unwrap().is_empty());
    assert_eq!(
        read_node_document(&child.path).unwrap().source,
        initial.source
    );

    drop(endpoint);
    drop(controller);
    let audit_text = std::fs::read_dir(audit.join("records"))
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(!audit_text.contains(&initial.source));
    assert!(!audit_text.contains(workspace.to_string_lossy().as_ref()));

    let reopened = SupervisedMcpSession::open(
        &workspace,
        scope(child.id),
        authority(),
        &audit,
        session_config(),
    )
    .unwrap();
    let (_, reopened_controller) = reopened.into_parts();
    assert!(reopened_controller.recovery_states().unwrap().is_empty());
    drop(reopened_controller);

    let mut incompatible = session_config();
    incompatible.runtime.wire_version = "9.9.9".to_owned();
    assert!(matches!(
        SupervisedMcpSession::open(
            &workspace,
            scope(child.id),
            authority(),
            &audit,
            incompatible
        ),
        Err(SupervisedMcpError::RuntimeContractMismatch)
    ));
    let mut foreign = authority();
    foreign.origin.session_id = "foreign-session".to_owned();
    assert!(matches!(
        SupervisedMcpSession::open(
            &workspace,
            scope(child.id),
            foreign,
            &audit,
            session_config()
        ),
        Err(SupervisedMcpError::Audit(AgentAuditError::IdentityMismatch))
    ));
}

#[test]
fn controller_revocation_blocks_subsequent_reads_without_a_write() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    let audit = temporary.path().join("agent-control/revoked-session");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let initial = read_node_document(&child.path).unwrap();
    let session = SupervisedMcpSession::open(
        &workspace,
        scope(child.id),
        authority(),
        &audit,
        session_config(),
    )
    .unwrap();
    let (endpoint, controller) = session.into_parts();
    let initialized = exchange(
        &endpoint,
        &[
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        ],
    );
    assert_eq!(initialized.len(), 1);
    controller.revoke().unwrap();
    let denied = exchange(
        &endpoint,
        &[serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_document","arguments":{"nodeId":child.id}}})],
    )
    .remove(0);
    assert_eq!(denied["result"]["isError"], true);
    assert!(!denied.to_string().contains(&initial.source));
    assert_eq!(
        read_node_document(&child.path).unwrap().source,
        initial.source
    );
}

#[test]
fn control_plane_inside_workspace_is_refused_before_creation() {
    let temporary = tempdir().unwrap();
    let workspace = temporary.path().join("Notes");
    create_workspace(&workspace).unwrap();
    let child = create_child_node(&workspace, "Child").unwrap();
    let forbidden = workspace.join("agent-control");
    assert!(matches!(
        SupervisedMcpSession::open(
            &workspace,
            scope(child.id),
            authority(),
            &forbidden,
            session_config()
        ),
        Err(SupervisedMcpError::InvalidControlPlane)
    ));
    assert!(!forbidden.exists());
}
