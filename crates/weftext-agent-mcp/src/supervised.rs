use std::collections::VecDeque;
use std::fmt;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use weftext_agent::{
    AgentAuditConfig, AgentAuditError, AgentAuditEvent, AgentAuditIdentity, AgentAuditLog,
    AgentAuditRecovery, AgentAuditRecoveryState, AgentBrokerConfig, AgentCapability, AgentOrigin,
    AgentRuntimeController, AgentRuntimeEvent, CancellationMode, CapabilityGrant,
    capability_digest,
};
use weftext_core::{
    NodeId, WorkspaceIndex, WorkspaceReadScope, read_node_document, scan_workspace,
};

use crate::MCP_PROTOCOL_VERSION;

const SERVER_NAME: &str = "weftext-supervised";
const INVENTORY_TOOL: &str = "workspace_inventory";
const READ_DOCUMENT_TOOL: &str = "read_document";
const MUTATION_STATUS_TOOL: &str = "mutation_status";
const DEFAULT_EVENT_BUFFER: usize = 1_024;
const HARD_MAX_EVENT_BUFFER: usize = 65_536;

/// Trusted actor, delegated-session, and workspace-policy authority for one MCP session.
///
/// The harness never supplies this structure. Desktop or Server builds it from authenticated
/// control-plane state before the first tool call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupervisedMcpAuthority {
    pub human_actor_id: String,
    pub delegated_client_id: String,
    pub workspace_scope_id: String,
    pub origin: AgentOrigin,
    pub actor_capabilities: CapabilityGrant,
    pub delegated_session_capabilities: CapabilityGrant,
    pub workspace_policy_capabilities: CapabilityGrant,
}

impl SupervisedMcpAuthority {
    fn allows(&self, capability: AgentCapability) -> bool {
        self.actor_capabilities.allows(capability)
            && self.delegated_session_capabilities.allows(capability)
            && self.workspace_policy_capabilities.allows(capability)
    }
}

impl From<&SupervisedMcpAuthority> for AgentAuditIdentity {
    fn from(authority: &SupervisedMcpAuthority) -> Self {
        Self {
            human_actor_id: authority.human_actor_id.clone(),
            delegated_client_id: authority.delegated_client_id.clone(),
            harness: authority.origin.harness.clone(),
            adapter_version: authority.origin.adapter_version.clone(),
            session_id: authority.origin.session_id.clone(),
            workspace_scope_id: authority.workspace_scope_id.clone(),
        }
    }
}

/// Runtime facts displayed by the trusted controller for this exact session.
///
/// DSH wire `0.0.1` is represented with `resume_supported = false` and
/// [`CancellationMode::RuntimeTermination`]; the session never invents prompt
/// cancellation or reconnect support.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupervisedRuntimeContract {
    pub wire_version: String,
    pub resume_supported: bool,
    pub cancellation: CancellationMode,
}

/// Configuration for one co-located agent endpoint and trusted controller.
#[derive(Clone, Debug)]
pub struct SupervisedSessionConfig {
    pub broker: AgentBrokerConfig,
    pub audit: AgentAuditConfig,
    pub runtime: SupervisedRuntimeContract,
    pub max_buffered_events: usize,
}

impl SupervisedSessionConfig {
    /// Exact currently-tested DSH preview-wire contract.
    #[must_use]
    pub fn dsh_wire_0_0_1(broker: AgentBrokerConfig, audit: AgentAuditConfig) -> Self {
        Self {
            broker,
            audit,
            runtime: SupervisedRuntimeContract {
                wire_version: "0.0.1".to_owned(),
                resume_supported: false,
                cancellation: CancellationMode::RuntimeTermination,
            },
            max_buffered_events: DEFAULT_EVENT_BUFFER,
        }
    }
}

/// One typed event merged by the co-located controller.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum SupervisedSessionEventKind {
    Runtime {
        event: AgentRuntimeEvent,
    },
    CapabilitiesUpdated {
        capability_digest: String,
    },
    RestartRecovery {
        recovery: AgentAuditRecovery,
    },
    RuntimeTerminatedForCancellation {
        cancellation: CancellationMode,
        resume_supported: bool,
    },
    AdapterCrashed {
        error_code: String,
    },
}

/// Monotonic process-local envelope for one UI/controller event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupervisedSessionEvent {
    pub sequence: u64,
    pub event: SupervisedSessionEventKind,
}

/// Bounded event page. `dropped_before` makes an overrun visible instead of
/// pretending the transient UI stream is durable; the audit log remains the
/// durable authority.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupervisedSessionEventBatch {
    pub events: Vec<SupervisedSessionEvent>,
    pub next_sequence: u64,
    pub dropped_before: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadDocumentArguments {
    node_id: NodeId,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MutationLookupArguments {
    request_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ToolCallParameters {
    name: String,
    #[serde(default)]
    arguments: Value,
}

/// Stable supervised-MCP failures returned to a trusted embedding host.
#[derive(Debug)]
pub enum SupervisedMcpError {
    InvalidWorkspace,
    InvalidScope,
    InvalidControlPlane,
    RuntimeContractMismatch,
    InvalidIntent(&'static str),
    NodeUnavailable,
    CapabilityUnavailable(AgentCapability),
    SessionPoisoned,
    Audit(AgentAuditError),
}

impl fmt::Display for SupervisedMcpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => formatter.write_str("workspace is not safe for agent access"),
            Self::InvalidScope => formatter.write_str("agent scope is empty or invalid"),
            Self::InvalidControlPlane => formatter.write_str(
                "agent control-plane state must be an absolute path outside the workspace",
            ),
            Self::RuntimeContractMismatch => formatter
                .write_str("agent runtime contract does not match the durable session authority"),
            Self::InvalidIntent(reason) => {
                write!(formatter, "invalid typed mutation intent: {reason}")
            }
            Self::NodeUnavailable => {
                formatter.write_str("node is outside the granted scope or unavailable")
            }
            Self::CapabilityUnavailable(capability) => {
                write!(formatter, "agent capability is unavailable: {capability:?}")
            }
            Self::SessionPoisoned => formatter
                .write_str("supervised session is unavailable after an internal or audit failure"),
            Self::Audit(error) => write!(
                formatter,
                "agent durable audit rejected the action: {error}"
            ),
        }
    }
}

impl std::error::Error for SupervisedMcpError {}

impl From<AgentAuditError> for SupervisedMcpError {
    fn from(error: AgentAuditError) -> Self {
        Self::Audit(error)
    }
}

/// MCP server with scoped, read-only workspace access.
///
/// Agent enhancement for content intake operates on Weftext-owned Import IR in the importer. This
/// workspace endpoint deliberately exposes no raw source edit, approval, or commit capability.
///
/// This low-level type retains the original in-memory constructor for contract
/// tests. Product embedding must use [`SupervisedMcpSession`], which requires a
/// workspace-external durable audit and splits the blocking protocol endpoint
/// from its concurrent trusted controller.
pub struct SupervisedMcpServer {
    workspace_root: PathBuf,
    scope: WorkspaceReadScope,
    authority: SupervisedMcpAuthority,
    audit: Option<AgentAuditLog>,
    runtime: Option<SupervisedRuntimeContract>,
    poisoned: bool,
    initialized: bool,
    ready: bool,
}

impl fmt::Debug for SupervisedMcpServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SupervisedMcpServer")
            .field("workspace_root", &self.workspace_root)
            .field("scope_size", &self.scope.node_ids().count())
            .field("initialized", &self.initialized)
            .field("ready", &self.ready)
            .field("durable_audit", &self.audit.is_some())
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl SupervisedMcpServer {
    /// Creates a server over one already-authorized logical Core scope.
    ///
    /// This constructor has no durable audit. It is intended for the
    /// harness-neutral contract suite; packaged callers use
    /// [`SupervisedMcpSession::open`].
    ///
    /// # Errors
    ///
    /// Fails closed unless the workspace is valid and every projected node currently belongs to
    /// its rebuilt Core inventory.
    pub fn new(
        workspace_root: impl AsRef<Path>,
        scope: WorkspaceReadScope,
        authority: SupervisedMcpAuthority,
        _broker_config: AgentBrokerConfig,
    ) -> Result<Self, SupervisedMcpError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        validate_authority(&authority)?;
        let inventory = valid_inventory(&workspace_root)?;
        let index = WorkspaceIndex::rebuild(&inventory)
            .map_err(|_| SupervisedMcpError::InvalidWorkspace)?;
        let scoped_ids = scope.node_ids().collect::<Vec<_>>();
        if scoped_ids.is_empty()
            || scoped_ids
                .iter()
                .any(|node_id| index.path_for(*node_id).is_none())
        {
            return Err(SupervisedMcpError::InvalidScope);
        }
        Ok(Self {
            workspace_root,
            scope,
            authority,
            audit: None,
            runtime: None,
            poisoned: false,
            initialized: false,
            ready: false,
        })
    }

    fn new_audited(
        workspace_root: impl AsRef<Path>,
        scope: WorkspaceReadScope,
        authority: SupervisedMcpAuthority,
        audit_root: impl AsRef<Path>,
        config: &SupervisedSessionConfig,
    ) -> Result<(Self, Vec<AgentAuditRecovery>), SupervisedMcpError> {
        if config.max_buffered_events == 0
            || config.max_buffered_events > HARD_MAX_EVENT_BUFFER
            || config.runtime.wire_version.is_empty()
            || config.runtime.wire_version.len() > 256
            || config.runtime.wire_version.chars().any(char::is_control)
        {
            return Err(SupervisedMcpError::InvalidControlPlane);
        }
        if authority.origin.harness == "dsh"
            && config.runtime.wire_version == "0.0.1"
            && (config.runtime.resume_supported
                || config.runtime.cancellation != CancellationMode::RuntimeTermination)
        {
            return Err(SupervisedMcpError::RuntimeContractMismatch);
        }
        let workspace_root = workspace_root.as_ref();
        let audit_root = audit_root.as_ref();
        validate_control_plane_location(workspace_root, audit_root)?;
        let audit_identity = AgentAuditIdentity::from(&authority);
        let mut server = Self::new(workspace_root, scope, authority, config.broker.clone())?;
        let mut audit = AgentAuditLog::open(audit_root, audit_identity, config.audit.clone())?;
        if let Some(first) = audit.records().first() {
            match &first.event {
                AgentAuditEvent::SessionOpened {
                    runtime_wire_version,
                    resume_supported,
                    cancellation,
                } if runtime_wire_version == &config.runtime.wire_version
                    && *resume_supported == config.runtime.resume_supported
                    && *cancellation == config.runtime.cancellation => {}
                _ => return Err(SupervisedMcpError::RuntimeContractMismatch),
            }
        }
        let recovery = audit.recovery_states();
        let event = if audit.records().is_empty() {
            AgentAuditEvent::SessionOpened {
                runtime_wire_version: config.runtime.wire_version.clone(),
                resume_supported: config.runtime.resume_supported,
                cancellation: config.runtime.cancellation,
            }
        } else {
            AgentAuditEvent::SessionReopened
        };
        let timestamp = unix_time_millis().max(audit.last_timestamp_millis().unwrap_or(0));
        audit.append(timestamp, event)?;
        server.audit = Some(audit);
        server.runtime = Some(config.runtime.clone());
        Ok((server, recovery))
    }

    /// Serves newline-delimited MCP requests until input closes.
    ///
    /// # Errors
    ///
    /// Returns an error only for transport failures. Tool and authorization failures are encoded
    /// as MCP tool results without leaking inaccessible node details.
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

    /// Replaces the three trusted capability grants without changing session identity.
    ///
    /// Desktop or Server calls this when actor rights, delegated-session grants, or workspace
    /// policy change. The broker re-evaluates the new intersection at the next approval or commit.
    pub fn update_capability_grants(
        &mut self,
        actor_capabilities: CapabilityGrant,
        delegated_session_capabilities: CapabilityGrant,
        workspace_policy_capabilities: CapabilityGrant,
    ) {
        let _ = self.update_capability_grants_checked(
            actor_capabilities,
            delegated_session_capabilities,
            workspace_policy_capabilities,
        );
    }

    fn update_capability_grants_checked(
        &mut self,
        actor_capabilities: CapabilityGrant,
        delegated_session_capabilities: CapabilityGrant,
        workspace_policy_capabilities: CapabilityGrant,
    ) -> Result<(), SupervisedMcpError> {
        self.require_healthy()?;
        let effective = actor_capabilities
            .intersection(&delegated_session_capabilities)
            .intersection(&workspace_policy_capabilities);
        self.append_audit(AgentAuditEvent::CapabilitiesUpdated {
            capability_digest: capability_digest(effective.iter()),
        })?;
        self.authority.actor_capabilities = actor_capabilities;
        self.authority.delegated_session_capabilities = delegated_session_capabilities;
        self.authority.workspace_policy_capabilities = workspace_policy_capabilities;
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
            if method == Some("notifications/initialized") && self.initialized {
                self.ready = true;
            }
            return None;
        }
        let id = id.unwrap_or(Value::Null);
        let Some(method) = method else {
            return Some(rpc_error(&id, -32600, "request method is required"));
        };
        if self.poisoned {
            return Some(rpc_error(
                &id,
                -32_003,
                "supervised session is unavailable after a control-plane failure",
            ));
        }
        match method {
            "initialize" => Some(self.initialize(&id, &params)),
            "ping" => Some(rpc_result(&id, &json!({}))),
            "tools/list" if self.ready => Some(rpc_result(&id, &tools_list())),
            "tools/call" if self.ready => Some(self.call_tool(&id, params)),
            "tools/list" | "tools/call" => {
                Some(rpc_error(&id, -32002, "MCP server is not initialized"))
            }
            _ => Some(rpc_error(&id, -32601, "method not found")),
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
        let protocol_version = match requested_version {
            "2025-06-18" | "2025-03-26" | "2024-11-05" => requested_version,
            _ => MCP_PROTOCOL_VERSION,
        };
        self.initialized = true;
        rpc_result(
            id,
            &json!({
                "protocolVersion": protocol_version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {
                    "name": SERVER_NAME,
                    "title": "Weftext supervised workspace tools",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "instructions": "Scoped workspace reads only. Optional content-intake enhancement uses typed Weftext Import IR patches through the importer, not this workspace endpoint.",
            }),
        )
    }

    fn call_tool(&mut self, id: &Value, params: Value) -> Value {
        let Ok(call) = serde_json::from_value::<ToolCallParameters>(params) else {
            return rpc_error(id, -32602, "invalid closed tool-call parameters");
        };
        let result = match call.name.as_str() {
            INVENTORY_TOOL => self.inventory_result(&call.arguments),
            READ_DOCUMENT_TOOL => self.document_result(call.arguments),
            MUTATION_STATUS_TOOL => self.mutation_status(call.arguments),
            _ => return rpc_error(id, -32602, "unknown Weftext tool"),
        };
        match result {
            Ok(result) => rpc_result(id, &tool_success(&result)),
            Err(error) => rpc_result(id, &tool_failure(&error.to_string())),
        }
    }

    fn inventory_result(&self, arguments: &Value) -> Result<Value, SupervisedMcpError> {
        self.require_capability(AgentCapability::ReadWorkspace)?;
        require_empty_arguments(arguments)?;
        let inventory = valid_inventory(&self.workspace_root)?;
        let index = WorkspaceIndex::rebuild(&inventory)
            .map_err(|_| SupervisedMcpError::InvalidWorkspace)?;
        let nodes = self
            .scope
            .node_ids()
            .map(|node_id| {
                let path = index
                    .path_for(node_id)
                    .ok_or(SupervisedMcpError::NodeUnavailable)?;
                Ok(json!({
                    "nodeId": node_id,
                    "name": path.file_name().and_then(|name| name.to_str()).unwrap_or("node"),
                    "relativePath": self.scope.locator(node_id).ok_or(SupervisedMcpError::InvalidScope)?,
                    "parentId": self.scope.parent_node_id(node_id),
                }))
            })
            .collect::<Result<Vec<_>, SupervisedMcpError>>()?;
        Ok(json!({
            "schema": "weftext.agent.read.v1",
            "scope": self.authority.workspace_scope_id,
            "nodeCount": nodes.len(),
            "nodes": nodes,
        }))
    }

    fn document_result(&self, arguments: Value) -> Result<Value, SupervisedMcpError> {
        self.require_capability(AgentCapability::ReadWorkspace)?;
        let arguments = serde_json::from_value::<ReadDocumentArguments>(arguments)
            .map_err(|_| SupervisedMcpError::InvalidIntent("read_document accepts only nodeId"))?;
        let node_path = self.node_path(arguments.node_id)?;
        let snapshot =
            read_node_document(&node_path).map_err(|_| SupervisedMcpError::NodeUnavailable)?;
        Ok(json!({
            "schema": "weftext.agent.read.v1",
            "document": {
                "nodeId": snapshot.node_id,
                "name": node_path.file_name().and_then(|name| name.to_str()).unwrap_or("node"),
                "relativePath": self.scope.locator(snapshot.node_id).ok_or(SupervisedMcpError::NodeUnavailable)?,
                "revision": snapshot.revision,
                "length": snapshot.source.len(),
                "profile": weftext_core::active_document_profile(),
                "source": snapshot.source,
            }
        }))
    }

    fn mutation_status(&self, arguments: Value) -> Result<Value, SupervisedMcpError> {
        let arguments =
            serde_json::from_value::<MutationLookupArguments>(arguments).map_err(|_| {
                SupervisedMcpError::InvalidIntent("mutation_status accepts only requestId")
            })?;
        self.recovered_mutation_status(&arguments.request_id)
    }

    fn record_adapter_crash(&mut self, error_code: String) -> Result<(), SupervisedMcpError> {
        self.require_healthy()?;
        self.append_audit(AgentAuditEvent::AdapterCrashed { error_code })
    }

    fn record_runtime_termination(&mut self) -> Result<(), SupervisedMcpError> {
        self.require_healthy()?;
        if self.runtime.as_ref().map(|runtime| runtime.cancellation)
            != Some(CancellationMode::RuntimeTermination)
        {
            return Err(SupervisedMcpError::RuntimeContractMismatch);
        }
        self.append_audit(AgentAuditEvent::RuntimeTerminatedForCancellation)
    }

    fn recovery_states(&self) -> Vec<AgentAuditRecovery> {
        self.audit
            .as_ref()
            .map_or_else(Vec::new, AgentAuditLog::recovery_states)
    }

    fn recovered_mutation_status(&self, request_id: &str) -> Result<Value, SupervisedMcpError> {
        let recovery = self
            .recovery_states()
            .into_iter()
            .find(|recovery| recovery.request_id == request_id)
            .ok_or(SupervisedMcpError::InvalidIntent(
                "mutation status is unavailable",
            ))?;
        Ok(json!({
            "schema": "weftext.agent.recovered-mutation-status.v1",
            "requestId": request_id,
            "recoveryState": recovery.state,
            "lastAuditSequence": recovery.last_sequence,
            "executablePlanAvailable": false,
            "commitOutcomeKnown": !matches!(
                recovery.state,
                AgentAuditRecoveryState::RequiresReproposal
                    | AgentAuditRecoveryState::ApprovedPlanUnavailable
                    | AgentAuditRecoveryState::CommitOutcomeIndeterminate
            ),
        }))
    }

    fn require_healthy(&self) -> Result<(), SupervisedMcpError> {
        if self.poisoned {
            Err(SupervisedMcpError::SessionPoisoned)
        } else {
            Ok(())
        }
    }

    fn append_audit(&mut self, event: AgentAuditEvent) -> Result<(), SupervisedMcpError> {
        let Some(audit) = self.audit.as_mut() else {
            return Ok(());
        };
        let timestamp = unix_time_millis().max(audit.last_timestamp_millis().unwrap_or(0));
        if let Err(error) = audit.append(timestamp, event) {
            self.poisoned = true;
            return Err(SupervisedMcpError::Audit(error));
        }
        Ok(())
    }

    fn require_capability(&self, capability: AgentCapability) -> Result<(), SupervisedMcpError> {
        if self.authority.allows(capability) {
            Ok(())
        } else {
            Err(SupervisedMcpError::CapabilityUnavailable(capability))
        }
    }

    fn node_path(&self, node_id: NodeId) -> Result<PathBuf, SupervisedMcpError> {
        if !self.scope.allows(node_id) {
            return Err(SupervisedMcpError::NodeUnavailable);
        }
        let inventory = valid_inventory(&self.workspace_root)?;
        let index = WorkspaceIndex::rebuild(&inventory)
            .map_err(|_| SupervisedMcpError::InvalidWorkspace)?;
        index
            .path_for(node_id)
            .map(Path::to_path_buf)
            .ok_or(SupervisedMcpError::NodeUnavailable)
    }
}

struct SharedSession {
    server: SupervisedMcpServer,
    runtime: SupervisedRuntimeContract,
    events: VecDeque<SupervisedSessionEvent>,
    next_event_sequence: u64,
    max_buffered_events: usize,
}

impl SharedSession {
    fn push_event(&mut self, event: SupervisedSessionEventKind) {
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        if self.events.len() == self.max_buffered_events {
            self.events.pop_front();
        }
        self.events.push_back(SupervisedSessionEvent {
            sequence: self.next_event_sequence,
            event,
        });
    }
}

/// Co-located supervised session that splits into an agent-only MCP endpoint
/// and a trusted host controller sharing the same broker and durable audit.
pub struct SupervisedMcpSession {
    endpoint: SupervisedMcpAgentEndpoint,
    controller: SupervisedMcpController,
}

impl SupervisedMcpSession {
    /// Opens one audited session. The audit path is mandatory, absolute, and
    /// must be outside portable workspace content.
    ///
    /// # Errors
    ///
    /// Fails closed for an invalid workspace, scope, runtime contract, audit
    /// path, identity, or existing audit chain.
    pub fn open(
        workspace_root: impl AsRef<Path>,
        scope: WorkspaceReadScope,
        authority: SupervisedMcpAuthority,
        audit_root: impl AsRef<Path>,
        config: SupervisedSessionConfig,
    ) -> Result<Self, SupervisedMcpError> {
        let (server, recovery) = SupervisedMcpServer::new_audited(
            workspace_root,
            scope,
            authority,
            audit_root,
            &config,
        )?;
        let shared = Arc::new(Mutex::new(SharedSession {
            server,
            runtime: config.runtime,
            events: VecDeque::new(),
            next_event_sequence: 0,
            max_buffered_events: config.max_buffered_events,
        }));
        {
            let mut state = lock_session(&shared)?;
            for recovered in recovery {
                state.push_event(SupervisedSessionEventKind::RestartRecovery {
                    recovery: recovered,
                });
            }
        }
        Ok(Self {
            endpoint: SupervisedMcpAgentEndpoint {
                shared: Arc::clone(&shared),
            },
            controller: SupervisedMcpController { shared },
        })
    }

    /// Separates the public protocol endpoint from the trusted in-process
    /// controller handle. No trusted method is encoded on the MCP transport.
    #[must_use]
    pub fn into_parts(self) -> (SupervisedMcpAgentEndpoint, SupervisedMcpController) {
        (self.endpoint, self.controller)
    }
}

/// Agent-facing MCP endpoint. It deliberately has no approval, grant, or commit
/// method. The mutex is acquired only while handling one complete JSON-RPC
/// frame, so blocked stdin never blocks the trusted controller.
pub struct SupervisedMcpAgentEndpoint {
    shared: Arc<Mutex<SharedSession>>,
}

impl fmt::Debug for SupervisedMcpAgentEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SupervisedMcpAgentEndpoint")
            .finish_non_exhaustive()
    }
}

impl SupervisedMcpAgentEndpoint {
    /// Serves newline-delimited MCP until the agent closes its input.
    ///
    /// # Errors
    ///
    /// Returns a redacted transport or poisoned-session failure.
    pub fn serve(&self, input: impl BufRead, mut output: impl Write) -> Result<(), String> {
        for line in input.lines() {
            let line = line.map_err(|error| format!("MCP input failed: {error}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let response = {
                let mut shared = self
                    .shared
                    .lock()
                    .map_err(|_| "supervised session lock is poisoned".to_owned())?;
                shared.server.handle_line(&line)
            };
            if let Some(response) = response {
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
}

/// Trusted, co-located controller handle. It is intentionally a Rust API, not
/// a second stdio or network command surface.
#[derive(Clone)]
pub struct SupervisedMcpController {
    shared: Arc<Mutex<SharedSession>>,
}

impl fmt::Debug for SupervisedMcpController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SupervisedMcpController")
            .finish_non_exhaustive()
    }
}

impl SupervisedMcpController {
    /// Replaces current actor/session/policy grants and audits only the digest
    /// of their effective intersection.
    ///
    /// # Errors
    ///
    /// Fails when the session or durable audit is unavailable.
    pub fn update_capability_grants(
        &self,
        actor_capabilities: CapabilityGrant,
        delegated_session_capabilities: CapabilityGrant,
        workspace_policy_capabilities: CapabilityGrant,
    ) -> Result<(), SupervisedMcpError> {
        let mut shared = lock_session(&self.shared)?;
        let effective = actor_capabilities
            .intersection(&delegated_session_capabilities)
            .intersection(&workspace_policy_capabilities);
        let digest = capability_digest(effective.iter());
        shared.server.update_capability_grants_checked(
            actor_capabilities,
            delegated_session_capabilities,
            workspace_policy_capabilities,
        )?;
        shared.push_event(SupervisedSessionEventKind::CapabilitiesUpdated {
            capability_digest: digest,
        });
        Ok(())
    }

    /// Revokes the delegated session before its next read, proposal, approval,
    /// or commit.
    ///
    /// # Errors
    ///
    /// Fails when the session or durable audit is unavailable.
    pub fn revoke(&self) -> Result<(), SupervisedMcpError> {
        let mut shared = lock_session(&self.shared)?;
        let actor = shared.server.authority.actor_capabilities.clone();
        let policy = shared
            .server
            .authority
            .workspace_policy_capabilities
            .clone();
        shared.server.update_capability_grants_checked(
            actor,
            CapabilityGrant::default(),
            policy,
        )?;
        let digest = capability_digest(std::iter::empty());
        shared.push_event(SupervisedSessionEventKind::CapabilitiesUpdated {
            capability_digest: digest,
        });
        Ok(())
    }

    /// Merges one validated harness event into the typed transient stream.
    ///
    /// # Errors
    ///
    /// Rejects foreign session events or an unavailable session.
    pub fn ingest_runtime_event(&self, event: AgentRuntimeEvent) -> Result<(), SupervisedMcpError> {
        let mut shared = lock_session(&self.shared)?;
        let expected = &shared.server.authority.origin.session_id;
        if !runtime_event_belongs_to(&event, expected) {
            return Err(SupervisedMcpError::InvalidScope);
        }
        shared.push_event(SupervisedSessionEventKind::Runtime { event });
        Ok(())
    }

    /// Records an adapter crash without persisting stderr, transcript, or body
    /// bytes.
    ///
    /// # Errors
    ///
    /// Fails if the redacted code is invalid or durable audit is unavailable.
    pub fn record_adapter_crash(&self, error_code: String) -> Result<(), SupervisedMcpError> {
        let mut shared = lock_session(&self.shared)?;
        shared.server.record_adapter_crash(error_code.clone())?;
        shared.push_event(SupervisedSessionEventKind::AdapterCrashed { error_code });
        Ok(())
    }

    /// Records the only cancellation available in DSH wire `0.0.1`: whole
    /// runtime termination. This does not imply prompt undo or resumability.
    ///
    /// # Errors
    ///
    /// Fails when durable audit is unavailable.
    pub fn record_runtime_terminated_for_cancellation(&self) -> Result<(), SupervisedMcpError> {
        let mut shared = lock_session(&self.shared)?;
        shared.server.record_runtime_termination()?;
        let runtime = shared.runtime.clone();
        shared.push_event(
            SupervisedSessionEventKind::RuntimeTerminatedForCancellation {
                cancellation: runtime.cancellation,
                resume_supported: runtime.resume_supported,
            },
        );
        Ok(())
    }

    /// Returns the exact runtime contract displayed for this session.
    ///
    /// # Errors
    ///
    /// Fails only when the co-located session lock is poisoned.
    pub fn runtime_contract(&self) -> Result<SupervisedRuntimeContract, SupervisedMcpError> {
        Ok(lock_session(&self.shared)?.runtime.clone())
    }

    /// Returns verified restart state without reconstructing lost opaque plans.
    ///
    /// # Errors
    ///
    /// Fails only when the co-located session lock is poisoned.
    pub fn recovery_states(&self) -> Result<Vec<AgentAuditRecovery>, SupervisedMcpError> {
        Ok(lock_session(&self.shared)?.server.recovery_states())
    }

    /// Reads one bounded page of merged mutation/runtime events after `cursor`.
    ///
    /// # Errors
    ///
    /// Fails only when the co-located session lock is poisoned.
    pub fn events_after(
        &self,
        cursor: u64,
    ) -> Result<SupervisedSessionEventBatch, SupervisedMcpError> {
        let shared = lock_session(&self.shared)?;
        let oldest = shared.events.front().map(|event| event.sequence);
        let dropped_before = oldest.filter(|oldest| cursor.saturating_add(1) < *oldest);
        let events = shared
            .events
            .iter()
            .filter(|event| event.sequence > cursor)
            .cloned()
            .collect::<Vec<_>>();
        Ok(SupervisedSessionEventBatch {
            next_sequence: events
                .last()
                .map_or(cursor.max(shared.next_event_sequence), |event| {
                    event.sequence
                }),
            events,
            dropped_before,
        })
    }
}

impl AgentRuntimeController for SupervisedMcpController {
    fn ingest_runtime_event(&self, event: AgentRuntimeEvent) -> Result<(), String> {
        Self::ingest_runtime_event(self, event).map_err(|error| error.to_string())
    }

    fn record_adapter_crash(&self, error_code: &str) -> Result<(), String> {
        Self::record_adapter_crash(self, error_code.to_owned()).map_err(|error| error.to_string())
    }

    fn record_runtime_terminated_for_cancellation(&self) -> Result<(), String> {
        Self::record_runtime_terminated_for_cancellation(self).map_err(|error| error.to_string())
    }
}

fn lock_session(
    shared: &Arc<Mutex<SharedSession>>,
) -> Result<MutexGuard<'_, SharedSession>, SupervisedMcpError> {
    shared
        .lock()
        .map_err(|_| SupervisedMcpError::SessionPoisoned)
}

fn validate_control_plane_location(
    workspace_root: &Path,
    audit_root: &Path,
) -> Result<(), SupervisedMcpError> {
    if !audit_root.is_absolute()
        || audit_root
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(SupervisedMcpError::InvalidControlPlane);
    }
    let workspace = workspace_root
        .canonicalize()
        .map_err(|_| SupervisedMcpError::InvalidWorkspace)?;
    #[cfg(windows)]
    {
        let workspace = windows_path_key(&workspace);
        let audit = windows_path_key(audit_root);
        let prefix = format!("{}\\", workspace.trim_end_matches('\\'));
        if audit == workspace || audit.starts_with(&prefix) {
            return Err(SupervisedMcpError::InvalidControlPlane);
        }
    }
    #[cfg(not(windows))]
    if audit_root.starts_with(&workspace) {
        return Err(SupervisedMcpError::InvalidControlPlane);
    }
    Ok(())
}

#[cfg(windows)]
fn windows_path_key(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('/', "\\");
    normalized
        .strip_prefix("\\\\?\\")
        .unwrap_or(&normalized)
        .to_lowercase()
}

fn unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

fn runtime_event_belongs_to(event: &AgentRuntimeEvent, expected_session_id: &str) -> bool {
    match event {
        AgentRuntimeEvent::SessionEvent { session_id, .. }
        | AgentRuntimeEvent::SessionStatus { session_id, .. } => session_id == expected_session_id,
        AgentRuntimeEvent::SubagentStarted {
            parent_session_id, ..
        } => parent_session_id == expected_session_id,
        AgentRuntimeEvent::SubagentFinished { payload }
        | AgentRuntimeEvent::Unknown {
            params: payload, ..
        } => payload
            .get("sessionId")
            .or_else(|| payload.get("parentSessionId"))
            .and_then(Value::as_str)
            .is_none_or(|session_id| session_id == expected_session_id),
    }
}

fn validate_authority(authority: &SupervisedMcpAuthority) -> Result<(), SupervisedMcpError> {
    for value in [
        authority.human_actor_id.as_str(),
        authority.delegated_client_id.as_str(),
        authority.workspace_scope_id.as_str(),
        authority.origin.harness.as_str(),
        authority.origin.adapter_version.as_str(),
        authority.origin.session_id.as_str(),
    ] {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(SupervisedMcpError::InvalidScope);
        }
    }
    Ok(())
}

fn valid_inventory(root: &Path) -> Result<weftext_core::WorkspaceInventory, SupervisedMcpError> {
    let inventory = scan_workspace(root);
    if inventory.is_valid() {
        Ok(inventory)
    } else {
        Err(SupervisedMcpError::InvalidWorkspace)
    }
}

fn require_empty_arguments(arguments: &Value) -> Result<(), SupervisedMcpError> {
    match arguments.as_object() {
        Some(object) if object.is_empty() => Ok(()),
        _ => Err(SupervisedMcpError::InvalidIntent(
            "workspace_inventory accepts no arguments",
        )),
    }
}

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": INVENTORY_TOOL,
                "title": "List scoped Weftext nodes",
                "description": "List only nodes in the trusted logical session scope.",
                "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            },
            {
                "name": READ_DOCUMENT_TOOL,
                "title": "Read one scoped Weftext document",
                "description": "Read one authorized document by UUID. Filesystem paths are not accepted.",
                "inputSchema": {
                    "type": "object",
                    "properties": {"nodeId": {"type": "string", "format": "uuid"}},
                    "required": ["nodeId"],
                    "additionalProperties": false,
                },
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            },
            {
                "name": MUTATION_STATUS_TOOL,
                "title": "Read durable operation recovery status",
                "description": "Inspect non-executable recovery evidence for a previously reviewed operation in this exact session.",
                "inputSchema": {
                    "type": "object",
                    "properties": {"requestId": {"type": "string", "minLength": 1, "maxLength": 256}},
                    "required": ["requestId"],
                    "additionalProperties": false,
                },
                "annotations": {"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
            }
        ]
    })
}

fn tool_success(structured: &Value) -> Value {
    let text = serde_json::to_string_pretty(structured)
        .unwrap_or_else(|_| "Weftext tool result could not be rendered".to_owned());
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
